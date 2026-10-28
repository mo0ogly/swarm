//! Saved-policy filtered historical log pages for one Work, matching the Go
//! filtered HTTP page contract with the page bound supplied by the observed saved
//! log-read policy instead of a hard-coded number. Recorded rows are display
//! observations of past CLI output, never agent authentication, liveness or
//! permission.

use super::agent_log_history::HistoricalAgentLog;
use super::{finished, material_path, RootStore, RootStoreError as E};
use rusqlite::{params, Connection, Row};

/// Decode SQL TEXT/BLOB as Go string bytes with per-invalid-byte UTF-8
/// replacement; NULL/numeric cells keep their typed column error. Unreadable
/// stored history never becomes an empty success.
fn string(row: &Row<'_>, index: usize) -> Result<String, E> {
    match row.get_ref(index).map_err(E::Sql)? {
        rusqlite::types::ValueRef::Text(raw) | rusqlite::types::ValueRef::Blob(raw) => {
            Ok(crate::text::go_utf8(raw))
        }
        v => Err(E::Sql(rusqlite::Error::InvalidColumnType(
            index,
            "agent log text".into(),
            v.data_type(),
        ))),
    }
}

/// One decoded snapshot page before the saved-policy observation is attached.
struct RawLogPage {
    entries: Vec<HistoricalAgentLog>,
    next_cursor: i64,
    retained_from: i64,
    retention_gap: bool,
    more: bool,
}

/// Read the retention anchor and one filtered page inside one caller-provided
/// SQL snapshot. `policy_rows` is the validated saved page bound passed in by
/// the caller: None is SQL -1 (unlimited) and Some(n) becomes n+1 so the extra
/// sentinel row decides `more`. The sentinel is decoded before `more` is
/// decided, as Go does, so a malformed boundary row fails the page instead of
/// silently truncating it. The anchor is min(seq) over ALL of the Agent's rows
/// before the message/kind filter; the gap copies Go's signed `first-1`
/// comparison exactly, including the technically-overflowing i64::MIN wrap
/// (`wrapping_sub`, never a saturating rewrite). `after` is bound as given,
/// negative included. No count-based preallocation, default or cap lives
/// here.
fn read_page(
    db: &Connection,
    agent: &str,
    query: &str,
    kind: &str,
    after: i64,
    policy_rows: Option<i64>,
) -> Result<RawLogPage, E> {
    let first: Option<i64> = db
        .query_row(
            "SELECT min(seq) FROM agent_logs WHERE agent_id=?1",
            params![agent],
            |row| row.get(0),
        )
        .map_err(E::Sql)?;
    let (retained_from, retention_gap) = match first {
        Some(min_seq) => (min_seq, after > 0 && after < min_seq.wrapping_sub(1)),
        None => (0, false),
    };
    let take = match policy_rows {
        None => -1,
        Some(bound) => bound
            .checked_add(1)
            .ok_or(E::Refused("log-query-page-bound-representation"))?,
    };
    let sql = "SELECT seq,agent_id,at,kind,message FROM agent_logs WHERE agent_id=?1 AND seq>?2 AND instr(lower(message),lower(?3))>0 AND (?4='' OR kind=?5) ORDER BY seq LIMIT ?6";
    let mut statement = db.prepare(sql).map_err(E::Sql)?;
    let result = (|| {
        let mut rows = statement
            .query(params![agent, after, query, kind, kind, take])
            .map_err(E::Sql)?;
        let mut entries = Vec::new();
        let mut next_cursor = after;
        let mut more = false;
        while let Some(row) = rows.next().map_err(E::Sql)? {
            // Decode before deciding More, as Go does.
            let entry = HistoricalAgentLog {
                seq: row.get(0).map_err(E::Sql)?,
                agent_id: string(row, 1)?,
                at: string(row, 2)?,
                kind: string(row, 3)?,
                message: string(row, 4)?,
            };
            if policy_rows.is_some_and(|bound| i64::try_from(entries.len()).ok() == Some(bound)) {
                more = true;
                break;
            }
            next_cursor = entry.seq;
            entries.push(entry);
        }
        Ok(RawLogPage {
            entries,
            next_cursor,
            retained_from,
            retention_gap,
            more,
        })
    })();
    finished(result, statement.finalize().map_err(E::Sql))
}

/// Observed saved log-read policy for one page. It grants no execution,
/// identity or business authority.
#[derive(serde::Serialize)]
pub struct LogQueryPolicyObservation {
    /// Saved policy revision observed at read time.
    pub revision: i64,
    /// Static source label of the observed saved policy.
    pub source: &'static str,
    /// Digest of the observed saved policy content.
    pub digest: String,
    /// Saved page bound rendered as text; None renders the saved unlimited
    /// bound rather than any hidden default.
    pub page_rows: Option<String>,
}

/// Exact native decimal cursor extension: preserve i64 values through browser
/// JSON parsing without imposing a hidden safe-integer cutoff on stored history.
#[derive(serde::Serialize)]
pub struct LogQueryExact {
    pub next_cursor: String,
    pub retained_from: String,
    pub sequences: Vec<String>,
}

/// Go filtered page keys plus the explicit observed saved log-read policy. An empty
/// filtered page never proves that no matching older rows exist; `more` is
/// page truncation visibility, not retention or a liveness claim.
#[derive(serde::Serialize)]
pub struct LogQueryView {
    /// Recorded historical rows in ascending seq order; display observations
    /// only, never agent authentication, liveness or permission, and consumers
    /// must escape their text for the rendering context.
    pub entries: Vec<HistoricalAgentLog>,
    /// Cursor for the next page: the last returned seq, or the requested
    /// `after` unchanged when the page is empty.
    pub next_cursor: i64,
    /// Smallest seq currently retained for the Agent, or 0 when nothing is
    /// retained; an observation of stored state, not a completeness promise.
    pub retained_from: i64,
    /// True when a positive cursor lies before retained history, reproducing
    /// Go's signed comparison including its technical i64::MIN wrap.
    pub retention_gap: bool,
    /// True when the saved bound truncated the filtered page; never a claim
    /// about future rows or liveness.
    pub more: bool,
    /// Observed saved policy that bounded this page.
    pub read_policy: LogQueryPolicyObservation,
    /// Exact decimal extension; numeric Go fields remain unchanged.
    pub exact: LogQueryExact,
}

impl RootStore {
    /// Read one Go-compatible filtered historical log page for an existing,
    /// valid stored Agent of the given Work, without mutating or launching
    /// anything. `query` is a literal substring match and `kind` an exact kind
    /// match with empty selecting every kind; `after` may be negative, zero or
    /// positive and is bound as given: only rows strictly after it qualify.
    /// This does not imply that zero includes explicitly negative SQL sequences.
    /// The page bound comes only from the saved log-read policy: None is
    /// unlimited with `more` always false, Some(n) returns at most n entries
    /// and sets `more` when one further row exists. Work and Agent existence
    /// errors propagate; an Agent stored under a different Work is refused
    /// with "log-query-work-mismatch". The Work read, Agent read, min(seq)
    /// anchor and page run while one read SQL snapshot is open on the store
    /// connection under the material identity wrapper; the saved-policy file
    /// read and that snapshot are not jointly atomic and the observed
    /// revision/source/digest is reported exactly as read. Codec, statement
    /// finalization and rollback causes are preserved; the Root is not closed here
    /// and Root closing remains the caller's responsibility. Rows are
    /// recorded display observations, never agent authentication, liveness or
    /// permission, and this is not an atomic filesystem snapshot or
    /// later-action authority.
    pub fn query_logs(
        &self,
        work: &str,
        agent: &str,
        query: &str,
        kind: &str,
        after: i64,
    ) -> Result<LogQueryView, E> {
        material_path::with_material_read(self, || {
            let policy = self.log_read_policy()?;
            let policy_rows = policy.config.values.search_page_rows.rows();
            let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            let result = (|| {
                crate::read_recorded_work(&tx, work).map_err(E::Work)?;
                let stored = self.read_agent(agent)?;
                if stored.work_id != work {
                    return Err(E::Refused("log-query-work-mismatch"));
                }
                let page = read_page(&tx, agent, query, kind, after, policy_rows)?;
                let exact = LogQueryExact {
                    next_cursor: page.next_cursor.to_string(),
                    retained_from: page.retained_from.to_string(),
                    sequences: page.entries.iter().map(|e| e.seq.to_string()).collect(),
                };
                Ok(LogQueryView {
                    exact,
                    entries: page.entries,
                    next_cursor: page.next_cursor,
                    retained_from: page.retained_from,
                    retention_gap: page.retention_gap,
                    more: page.more,
                    read_policy: LogQueryPolicyObservation {
                        revision: policy.config.revision,
                        source: policy.source,
                        digest: policy.digest,
                        page_rows: policy_rows.map(|bound| bound.to_string()),
                    },
                })
            })();
            finished(result, tx.rollback().map_err(E::Sql))
        })
    }
}
