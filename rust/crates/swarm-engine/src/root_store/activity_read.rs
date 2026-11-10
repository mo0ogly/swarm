//! Work-scoped activity read under a SQL snapshot and an observed display policy.
use super::{finished, RootStore, RootStoreError as E};
use crate::{ActivityEntry, ActivityPage, ActivityQuery};
use rusqlite::{Connection, Row};
/// Actual selected display policy. It grants no execution, identity or business
/// completion. File policy read and SQL snapshot are not jointly atomic; reads
/// retain the observed revision rather than refreezing on a later save.
#[derive(serde::Serialize)]
pub struct ActivityPolicyObservation {
    pub revision: i64,
    pub source: &'static str,
    pub digest: String,
    pub effective_page_rows: String,
    pub source_max_rows: Option<String>,
    pub origin_evidence: &'static str,
}
/// Compatible Go page fields plus explicit native policy observation. Truncation
/// must remain visible; a filtered empty page never proves no older decisions.
#[derive(serde::Serialize)]
pub struct ActivityReadView {
    #[serde(flatten)]
    pub page: ActivityPage,
    pub read_policy: ActivityPolicyObservation,
}
/// Decode SQL TEXT/BLOB as Go string bytes; NULL/numeric incompatible fields
/// retain column/type causes. Never treat unreadable data as empty success.
fn string(row: &Row<'_>, index: usize) -> Result<String, E> {
    match row.get_ref(index).map_err(E::Sql)? {
        rusqlite::types::ValueRef::Text(raw) | rusqlite::types::ValueRef::Blob(raw) => {
            Ok(crate::activity_entry::text(raw))
        }
        v => Err(E::Sql(rusqlite::Error::InvalidColumnType(
            index,
            "activity text".into(),
            v.data_type(),
        ))),
    }
}
/// Read one source's newest rows by its write sequence before global time sort,
/// as Go. One extra sentinel only detects truncation; no payload beyond the
/// saved source cap is decoded or returned. SQL -1 is unlimited representation.
/// Statement/finalization and codec causes propagate; no hidden row ceiling.
fn source(
    db: &Connection,
    work: &str,
    cockpit: bool,
    cap: Option<i64>,
) -> Result<(Vec<ActivityEntry>, bool), E> {
    let take = cap
        .map(|n| {
            n.checked_add(1)
                .ok_or(E::Refused("activity-source-bound-representation"))
        })
        .transpose()?
        .unwrap_or(-1);
    let sql = if cockpit {
        "SELECT seq,at,kind,message FROM cockpit_events WHERE work_id=?1 ORDER BY seq DESC LIMIT ?2"
    } else {
        "SELECT revision,at,kind,payload FROM events WHERE work_id=?1 ORDER BY revision DESC LIMIT ?2"
    };
    let mut statement = db.prepare(sql).map_err(E::Sql)?;
    let result = (|| {
        let mut rows = statement
            .query(rusqlite::params![work, take])
            .map_err(E::Sql)?;
        let mut entries = Vec::new();
        let mut truncated = false;
        while let Some(row) = rows.next().map_err(E::Sql)? {
            if cap.is_some_and(|n| i64::try_from(entries.len()).ok() == Some(n)) {
                truncated = true;
                break;
            }
            let sequence = row.get(0).map_err(E::Sql)?;
            let at = string(row, 1)?;
            let kind = string(row, 2)?;
            let entry = if cockpit {
                crate::activity_entry::control(sequence, at, kind, string(row, 3)?)
            } else {
                let stored = row.get_ref(3).map_err(E::Sql)?;
                match stored {
                    rusqlite::types::ValueRef::Text(raw) | rusqlite::types::ValueRef::Blob(raw) => {
                        crate::activity_entry::event(sequence, at, kind, raw)
                    }
                    v => {
                        return Err(E::Sql(rusqlite::Error::InvalidColumnType(
                            3,
                            "activity payload".into(),
                            v.data_type(),
                        )))
                    }
                }
            }
            .map_err(|e| E::Launch(Box::new(e)))?;
            entries.push(entry)
        }
        Ok((entries, truncated))
    })();
    finished(result, statement.finalize().map_err(E::Sql))
}
impl RootStore {
    /// Read a Go-compatible filtered activity page for one existing Work. Policy
    /// comes from visible versioned defaults or its explicit saved ledger; requests
    /// select saved default with absent/0 limit, and excessive pages clamp only to
    /// the saved operator maximum. No literal50/200/500 or business stop here.
    /// One readonly transaction binds Work/source rows; known codec/source errors,
    /// finalization/rollback and caller Root close/output remain visible. No visit,
    /// event, budget, provider, identity or supervisor decision is changed.
    pub fn activity(
        &self,
        work: &str,
        before: &str,
        requested: Option<i64>,
        decisions_only: bool,
    ) -> Result<ActivityReadView, E> {
        if requested.is_some_and(|n| n < 0) {
            return Err(E::Refused("activity-query-invalid"));
        }
        let policy = self.activity_read_policy()?;
        let v = policy.config.values;
        let mut limit = requested.filter(|n| *n > 0).unwrap_or(v.page_default_rows);
        if let Some(max) = v.page_max_rows.rows() {
            limit = limit.min(max)
        }
        let query = ActivityQuery {
            before: before.into(),
            limit: usize::try_from(limit).map_err(|e| {
                E::Launch(Box::new(crate::AgentLaunchError::caused(
                    "activity-page-representation",
                    e,
                )))
            })?,
            decisions_only,
        };
        let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        let result = (|| {
            crate::read_recorded_work(&tx, work).map_err(E::Work)?;
            let (mut entries, a) = source(&tx, work, true, v.source_max_rows.rows())?;
            let (other, b) = source(&tx, work, false, v.source_max_rows.rows())?;
            entries.extend(other);
            let page = crate::paginate_activity(entries, &query, a || b).map_err(|e| {
                E::Launch(Box::new(crate::AgentLaunchError::caused(
                    "activity-page-invalid",
                    e,
                )))
            })?;
            Ok(ActivityReadView {
                page,
                read_policy: ActivityPolicyObservation {
                    revision: policy.config.revision,
                    source: policy.source,
                    digest: policy.digest,
                    effective_page_rows: limit.to_string(),
                    source_max_rows: v.source_max_rows.rows().map(|n| n.to_string()),
                    origin_evidence: "inferred-recorded-kind-and-payload-not-authentication",
                },
            })
        })();
        finished(result, tx.rollback().map_err(E::Sql))
    }
}
