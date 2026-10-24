//! Separate provider-telemetry store `.swarm/telemetry.db` (LIVE-UX-03, D2a).
//!
//! The mission database (schema 37, shared with Go) is never touched: this
//! file has its own versioned schema, so it adds no migration to the shared
//! schema, survives a rollback of the single Go→Rust switch and can be deleted
//! without affecting mission state. It holds:
//! - ingest tokens per agent attempt: only the sha256 digest of the secret is
//!   stored; the secret is returned once by [`TelemetryStore::issue_token`];
//! - whitelisted records from [`crate::telemetry_normalize`], keyed by an
//!   agent-scoped digest for idempotent exporter retries (TEL-R7);
//! - unknown-name and rejected-item counters (TEL-R6/R12).
//!
//! A token only authorizes writing telemetry for its agent. Whether that agent
//! is still active (TEL-R13) is decided by the caller against the mission
//! store; this module does not read it. Values remain provider declarations.

use crate::agent_live_read::LIVE_PAGE_ROWS;
use crate::local_path::{canonical_project_root, LocalPathError};
use crate::telemetry_normalize::{hex, NormalizedBatch};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
#[cfg(test)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Schema version stored in `PRAGMA user_version`.
pub const TELEMETRY_SCHEMA_VERSION: i64 = 7;
/// File name under the Root's `.swarm` directory.
pub const TELEMETRY_DB: &str = "telemetry.db";
/// Token length in hex characters (32 random bytes).
pub const TELEMETRY_TOKEN_HEX: usize = 64;

const SCHEMA: &str = "
CREATE TABLE tokens(digest TEXT PRIMARY KEY, agent_id TEXT NOT NULL,
  issued_at_ms INTEGER NOT NULL, revoked_at_ms INTEGER);
CREATE TABLE records(digest TEXT PRIMARY KEY, agent_id TEXT NOT NULL,
  kind TEXT NOT NULL, name TEXT NOT NULL, at_ms INTEGER NOT NULL,
  received_ms INTEGER NOT NULL, value REAL, temporality TEXT, fields TEXT NOT NULL);
CREATE INDEX records_agent ON records(agent_id, kind, name, at_ms);
CREATE TABLE unknown_names(agent_id TEXT NOT NULL, name TEXT NOT NULL,
  count INTEGER NOT NULL, PRIMARY KEY(agent_id, name));
CREATE TABLE rejections(agent_id TEXT PRIMARY KEY, count INTEGER NOT NULL);
";
/// v2 (LangWatch L5): supervisor alert history; at most one open row per
/// (rule, subject). Applied to fresh stores and as the v1 → v2 step.
const ALERTS_V2: &str = "
CREATE TABLE alerts(id INTEGER PRIMARY KEY AUTOINCREMENT, rule TEXT NOT NULL,
  level TEXT NOT NULL, subject TEXT NOT NULL, work_id TEXT NOT NULL,
  fired_ms INTEGER NOT NULL, occurred TEXT NOT NULL, resolved_ms INTEGER);
CREATE UNIQUE INDEX alerts_open ON alerts(rule, subject) WHERE resolved_ms IS NULL;
";
/// v3: why an alert closed — `cleared` (condition gone, subject still
/// present) or `gone` (agent or task no longer recorded). Empty for rows
/// resolved under v2 (reason unknown) and for open rows.
const ALERTS_V3: &str = "ALTER TABLE alerts ADD COLUMN resolution TEXT NOT NULL DEFAULT '';";
/// v4 (settings v33 retention): index for age purges and one cumulative,
/// content-free purge summary row (observable, never an identifier).
const RETENTION_V4: &str = "
CREATE INDEX records_received ON records(received_ms);
CREATE TABLE purges(id INTEGER PRIMARY KEY CHECK (id = 1), at_ms INTEGER NOT NULL,
  cutoff_ms INTEGER NOT NULL, records INTEGER NOT NULL, alerts INTEGER NOT NULL, tokens INTEGER NOT NULL);
";

/// v5 (LangWatch L10): one operator annotation per agent attempt — verdict
/// and free note written by the operator (never provider content), with an
/// optimistic revision. Not touched by the telemetry retention purge.
const ANNOTATIONS_V5: &str = "
CREATE TABLE annotations(agent_id TEXT PRIMARY KEY, work_id TEXT NOT NULL,
  verdict TEXT NOT NULL CHECK (verdict IN ('good', 'bad', 'unsure')), note TEXT NOT NULL,
  revision INTEGER NOT NULL CHECK (revision > 0), updated_ms INTEGER NOT NULL);
CREATE INDEX annotations_work ON annotations(work_id);
";

/// v6 (LangWatch L4): deterministic evaluations of finished attempts, one
/// row per (attempt, evaluator), written once by the alert evaluation. Not
/// touched by the telemetry retention purge (summaries outlive raw points).
const EVALUATIONS_V6: &str = "
CREATE TABLE evaluations(agent_id TEXT NOT NULL, work_id TEXT NOT NULL, evaluator TEXT NOT NULL,
  score REAL, passed INTEGER, detail TEXT NOT NULL, at_ms INTEGER NOT NULL,
  PRIMARY KEY(agent_id, evaluator));
";

/// v7 (LangWatch L6): one operator scenario per task (expectations JSON,
/// see [`crate::ScenarioSpec`]), with an optimistic revision. Results are
/// computed on read from recorded facts, never stored.
const SCENARIOS_V7: &str = "
CREATE TABLE scenarios(work_id TEXT NOT NULL, task_id TEXT NOT NULL, spec TEXT NOT NULL,
  revision INTEGER NOT NULL CHECK (revision > 0), updated_ms INTEGER NOT NULL, PRIMARY KEY(work_id, task_id));
";

/// Telemetry store failure. `code()` is the stable public family; causes are
/// kept for diagnosis and never contain a token or record content.
#[derive(Debug)]
pub enum TelemetryStoreError {
    Path(LocalPathError),
    Io(io::Error),
    Sql(rusqlite::Error),
    Random(getrandom::Error),
    /// `telemetry-store-busy` (another writer holds the lock; transient),
    /// `telemetry-store-missing` (no token was ever issued in this Root),
    /// `telemetry-store-unsafe` (not a private regular file of this user, or
    /// `.swarm` is not a directory), `telemetry-store-version-unsupported`,
    /// `telemetry-agent-invalid`.
    Refused(&'static str),
    /// Descriptor-relative storage failure, including uncertain publication.
    Storage(crate::LocalStorageError),
    /// Original Root failure retained through a later telemetry stage.
    Root(Box<crate::RootStoreError>),
    /// Primary and mandatory cleanup failures; neither branch is discarded.
    Cleanup {
        primary: Option<Box<Self>>,
        cleanup: Box<Self>,
    },
    /// A token INSERT succeeded before a later failure. No secret is retained.
    TokenIssued {
        primary: Box<Self>,
    },
}
impl TelemetryStoreError {
    /// Classify this failure without rendering private data or authorizing retry.
    /// TokenIssued returns `telemetry-token-issued`: SQLite reported INSERT1
    /// before a later failure, not necessarily a currently retained token row.
    /// Cleanup selects its primary code, or the cleanup code when no primary
    /// exists; consumers must separately traverse both typed branches. Root
    /// delegates its original family. Rejected ownership/permissions, symlink or
    /// non-directory storage returns `telemetry-store-unsafe`; SQLite busy is
    /// `telemetry-store-busy`, other storage/IO/SQL faults are unavailable.
    /// This code alone does not establish rollback, absence of effects or all
    /// details of a nested original; retain the error tree alongside it.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Refused(code) => code,
            Self::TokenIssued { .. } => "telemetry-token-issued",
            Self::Cleanup { primary, cleanup } => primary.as_deref().unwrap_or(cleanup).code(),
            Self::Root(error) => error.code(),
            Self::Storage(error) if error.source().and_then(|cause|cause.downcast_ref::<io::Error>()).is_some_and(|error|error.kind()==io::ErrorKind::PermissionDenied)
                => "telemetry-store-unsafe",
            Self::Storage(error) if error.source().and_then(|cause|cause.downcast_ref::<rustix::io::Errno>()).is_some_and(|error|matches!(*error,rustix::io::Errno::LOOP|rustix::io::Errno::NOTDIR))
                => "telemetry-store-unsafe",
            Self::Storage(_) => "telemetry-store-unavailable",
            Self::Sql(e) if e.sqlite_error_code() == Some(rusqlite::ErrorCode::DatabaseBusy) => {
                "telemetry-store-busy"
            }
            _ => "telemetry-store-unavailable",
        }
    }
}
/// Preserve a result and mandatory cleanup without losing either cause.
/// A cleanup failure after success remains failure, never proof of rollback.
pub(crate) fn telemetry_finished<T>(
    result: Result<T, TelemetryStoreError>,
    cleanup: Result<(), TelemetryStoreError>,
) -> Result<T, TelemetryStoreError> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (result, Err(cleanup)) => Err(TelemetryStoreError::Cleanup {
            primary: result.err().map(Box::new),
            cleanup: Box::new(cleanup),
        }),
    }
}
/// Lift the identity observer's secondary closes into the shared telemetry tree.
fn identity_error(
    mut error: crate::connected_database_identity::ConnectedDatabaseIdentityError,
) -> TelemetryStoreError {
    let cleanup = error.cleanup.take();
    let primary = TelemetryStoreError::Io(io::Error::other(error));
    match cleanup {
        Some(cleanup) => TelemetryStoreError::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(identity_error(*cleanup)),
        },
        None => primary,
    }
}
/// Close a SQLite connection explicitly; its cause survives the returned handle.
fn close_database(db: Connection) -> Result<(), TelemetryStoreError> {
    db.close()
        .map_err(|(_, error)| TelemetryStoreError::Sql(error))
}
impl TelemetryStoreError {
    /// Observe a known successful token INSERT through mandatory cleanup layers.
    /// This never reveals the token or authorizes execution, delivery or retry.
    pub fn token_issued(&self) -> bool {
        match self {
            Self::TokenIssued { .. } => true,
            Self::Cleanup { primary, cleanup } => primary.as_deref().is_some_and(Self::token_issued) || cleanup.token_issued(),
            _ => false,
        }
    }
}
impl fmt::Display for TelemetryStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for TelemetryStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Path(e) => Some(e),
            Self::Io(e) => Some(e),
            Self::Sql(e) => Some(e),
            Self::Random(e) => Some(e),
            Self::Refused(_) => None,
            Self::Storage(error) => Some(error),
            Self::Root(error) => Some(error.as_ref()),
            Self::Cleanup { primary, cleanup } => Some(primary.as_deref().unwrap_or(cleanup)),
            Self::TokenIssued { primary } => Some(primary.as_ref()),
        }
    }
}
impl From<rusqlite::Error> for TelemetryStoreError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}

/// Result of recording one normalized batch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecordOutcome {
    pub inserted: u64,
    pub duplicates: u64,
    /// New records refused because the attempt reached its cap (also added
    /// to the agent's rejection counter).
    pub capped: u64,
    /// Rows removed by the retention purge run in the same transaction.
    pub purged: PurgeReport,
}

/// Saved bounds for the console store (engine settings v33), or none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StoreLimits {
    /// Keep rows this many days; `None` keeps without limit.
    pub retention_days: Option<i64>,
    /// Records kept per agent attempt; `None` = no operator cap.
    pub max_records_per_agent: Option<i64>,
}

/// Rows removed by one retention purge.
#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
pub struct PurgeReport {
    pub records: u64,
    pub alerts: u64,
    pub tokens: u64,
}

/// Cumulative purge summary (store v4): last purge time and cutoff, and the
/// totals removed since the summary exists.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct PurgeState {
    pub at_ms: i64,
    pub cutoff_ms: i64,
    pub removed: PurgeReport,
}

const DAY_MS: i64 = 86_400_000;

/// Delete rows older than `days` (records by arrival, resolved alerts by
/// resolution, revoked tokens by revocation; open alerts and valid tokens are
/// never purged) inside `tx`, and fold the counts into the summary row.
/// Counts first: nothing to delete writes nothing.
fn purge_in(tx: &rusqlite::Transaction<'_>, days: i64, now_ms: i64) -> Result<PurgeReport, TelemetryStoreError> {
    let cutoff = now_ms.saturating_sub(days.saturating_mul(DAY_MS));
    let count = |sql: &str| -> Result<u64, TelemetryStoreError> { Ok(tx.query_row(sql, [cutoff], |r| r.get::<_, i64>(0))? as u64) };
    let report = PurgeReport {
        records: count("SELECT count(*) FROM records WHERE received_ms < ?1")?,
        alerts: count("SELECT count(*) FROM alerts WHERE resolved_ms IS NOT NULL AND resolved_ms < ?1")?,
        tokens: count("SELECT count(*) FROM tokens WHERE revoked_at_ms IS NOT NULL AND revoked_at_ms < ?1")?,
    };
    if report == PurgeReport::default() {
        return Ok(report);
    }
    tx.execute("DELETE FROM records WHERE received_ms < ?1", [cutoff])?;
    tx.execute("DELETE FROM alerts WHERE resolved_ms IS NOT NULL AND resolved_ms < ?1", [cutoff])?;
    tx.execute("DELETE FROM tokens WHERE revoked_at_ms IS NOT NULL AND revoked_at_ms < ?1", [cutoff])?;
    tx.execute(
        "INSERT INTO purges(id, at_ms, cutoff_ms, records, alerts, tokens) VALUES (1, ?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET at_ms = excluded.at_ms, cutoff_ms = excluded.cutoff_ms,
           records = records + excluded.records, alerts = alerts + excluded.alerts, tokens = tokens + excluded.tokens",
        params![now_ms, cutoff, report.records as i64, report.alerts as i64, report.tokens as i64],
    )?;
    Ok(report)
}

/// Per-call row from an `api_request` event (one event = one model call).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct TelemetryCall {
    pub at_ms: i64,
    pub fields: BTreeMap<String, Value>,
}
/// Aggregate of `tool_result` events for one tool name.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct TelemetryTool {
    pub name: String,
    pub count: u64,
    pub failures: u64,
    pub total_ms: f64,
    pub max_ms: f64,
}
/// One `tool_result` event, for the harness waterfall. `duration_ms` and
/// `success` are None when the provider did not declare them.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct TelemetryToolCall {
    pub at_ms: i64,
    pub tool: String,
    pub duration_ms: Option<f64>,
    pub success: Option<bool>,
}
/// Total of one metric (by name and `type` attribute). `value` is None when the
/// provider declared both delta and cumulative points for it (never mixed).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct TelemetryTotal {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub temporality: String,
    pub value: Option<f64>,
}
/// Latency summary of one histogram name and model (milliseconds), merged
/// over the agent's points: delta points add up, cumulative points keep the
/// newest per series. Quantiles are linear interpolations inside the merged
/// buckets (`None` when bounds differ between points or no bucket exists).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct TelemetryLatency {
    pub name: String,
    pub model: Option<String>,
    pub count: u64,
    pub mean_ms: Option<f64>,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub max_ms: Option<f64>,
}

/// One kept span (Codex trace): start time and duration in ms, identity
/// (`trace_id`, `span_id`, `parent_id`) and allowlisted attributes.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct TelemetrySpan {
    pub name: String,
    pub start_ms: i64,
    pub duration_ms: f64,
    pub fields: BTreeMap<String, Value>,
}

/// Newest cumulative histogram point of one series: (time, fields, sum).
type NewestPoint = (i64, BTreeMap<String, Value>, f64);

/// Merged histogram state for one (name, model, token_type) group.
#[derive(Default)]
struct HistogramMerge {
    count: u64,
    sum: f64,
    max: Option<f64>,
    bounds: Option<Vec<f64>>,
    buckets: Vec<u64>,
    comparable: bool,
}
impl HistogramMerge {
    fn add(&mut self, f: &BTreeMap<String, Value>, sum: f64) {
        let first = self.count == 0 && self.bounds.is_none();
        self.count += f.get("count").and_then(Value::as_u64).unwrap_or(0);
        self.sum += sum;
        if let Some(m) = f.get("max").and_then(Value::as_f64) {
            self.max = Some(self.max.map_or(m, |x| x.max(m)));
        }
        let bounds: Option<Vec<f64>> = f.get("bounds").and_then(Value::as_array).map(|b| b.iter().filter_map(Value::as_f64).collect());
        let buckets: Vec<u64> = f.get("buckets").and_then(Value::as_array).map_or_else(Vec::new, |b| b.iter().filter_map(Value::as_u64).collect());
        if first {
            self.comparable = bounds.is_some();
            self.bounds = bounds;
            self.buckets = buckets;
        } else if self.comparable && bounds == self.bounds && buckets.len() == self.buckets.len() {
            self.buckets.iter_mut().zip(buckets).for_each(|(a, b)| *a += b);
        } else {
            self.comparable = false;
        }
    }
    /// Value at quantile `q` by linear interpolation inside the bucket that
    /// holds it; the open last bucket uses the recorded max when known.
    fn quantile(&self, q: f64) -> Option<f64> {
        let bounds = self.bounds.as_ref().filter(|_| self.comparable && self.count > 0)?;
        let target = q * self.count as f64;
        let mut seen = 0.0;
        for (i, &n) in self.buckets.iter().enumerate() {
            let next = seen + n as f64;
            if n > 0 && next >= target {
                let low = if i == 0 { 0.0 } else { bounds[i - 1] };
                let high = bounds.get(i).copied().or(self.max)?;
                return Some(low + (high - low) * ((target - seen) / n as f64));
            }
            seen = next;
        }
        None
    }
}

/// Read model for one agent, for the live console.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct AgentTelemetry {
    /// `no-token` (none issued), `waiting` (token, nothing received), `received`.
    pub state: &'static str,
    /// Always `provider-declared-unauthenticated`.
    pub provenance: &'static str,
    pub last_received_ms: Option<i64>,
    /// Newest first, at most [`LIVE_PAGE_ROWS`]; `calls_more` marks truncation.
    pub calls: Vec<TelemetryCall>,
    pub calls_more: bool,
    pub tools: Vec<TelemetryTool>,
    /// Newest first, at most [`LIVE_PAGE_ROWS`]; `tool_calls_more` marks truncation.
    pub tool_calls: Vec<TelemetryToolCall>,
    pub tool_calls_more: bool,
    /// `tool_decision` counts by decision value.
    pub decisions: BTreeMap<String, u64>,
    /// Totals of sum/gauge metrics, plus Codex `turn.token_usage` histogram
    /// sums by token type.
    pub totals: Vec<TelemetryTotal>,
    /// Latency histograms (`*.duration_ms`) by name and model.
    pub latencies: Vec<TelemetryLatency>,
    /// Kept spans of the newest trace, in start order, at most
    /// [`LIVE_PAGE_ROWS`] (`spans_more` marks the cut).
    pub spans: Vec<TelemetrySpan>,
    pub spans_more: bool,
    pub unknown: BTreeMap<String, u64>,
    pub rejected: u64,
}

/// One alert row (open when `resolved_ms` is None).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AlertRow {
    pub id: i64,
    pub rule: String,
    pub level: String,
    pub subject: String,
    pub work_id: String,
    /// When the console first observed the condition.
    pub fired_ms: i64,
    /// Engine time of the event when known (e.g. agent end), else empty.
    pub occurred: String,
    pub resolved_ms: Option<i64>,
    /// `cleared`, `gone`, or empty (open, or resolved before v3).
    pub resolution: String,
}
/// Transitions applied by one [`TelemetryStore::sync_alerts`].
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct AlertSync {
    pub fired: Vec<AlertRow>,
    pub resolved: Vec<AlertRow>,
}
/// Open alerts (oldest first) and the newest resolved ones, each at most
/// [`LIVE_PAGE_ROWS`] (`open_more` / `history_more` report the cut).
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct AlertView {
    pub open: Vec<AlertRow>,
    pub open_more: bool,
    pub history: Vec<AlertRow>,
    pub history_more: bool,
}

fn alert_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<AlertRow> {
    Ok(AlertRow {
        id: r.get(0)?,
        rule: r.get(1)?,
        level: r.get(2)?,
        subject: r.get(3)?,
        work_id: r.get(4)?,
        fired_ms: r.get(5)?,
        occurred: r.get(6)?,
        resolved_ms: r.get(7)?,
        resolution: r.get(8)?,
    })
}
const ALERT_COLUMNS: &str = "id, rule, level, subject, work_id, fired_ms, occurred, resolved_ms, resolution";

/// Open handle on `.swarm/telemetry.db`. `version` is the schema actually
/// present: an older supported schema opened without `create` is read as is
/// (alerts absent before v2), never migrated by a read.
pub struct TelemetryStore {
    db: Connection,
    version: i64,
    directory: crate::local_storage::LocalDirectory,
    identity: crate::connected_database_identity::ConnectedDatabaseIdentity,
}

fn path(root: &Path) -> Result<PathBuf, TelemetryStoreError> {
    let root = canonical_project_root(root).map_err(TelemetryStoreError::Path)?;
    let dir = root.join(".swarm");
    match fs::symlink_metadata(&dir) {
        Ok(m) if m.is_dir() => Ok(dir.join(TELEMETRY_DB)),
        Ok(_) => Err(TelemetryStoreError::Refused("telemetry-store-unsafe")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            Err(TelemetryStoreError::Refused("telemetry-store-missing"))
        }
        Err(e) => Err(TelemetryStoreError::Io(e)),
    }
}

/// Preserve the public unsafe-file refusal before connection acquisition.
/// This is a compatibility rejection only; the actual descriptor-relative inode
/// checks remain authoritative after open and never grant based on this stat.
fn private_regular(path:&Path) -> Result<(),TelemetryStoreError> {
    let metadata=fs::symlink_metadata(path).map_err(TelemetryStoreError::Io)?;
    if !metadata.file_type().is_file() || metadata.uid()!=rustix::process::getuid().as_raw() || metadata.mode()&0o077!=0 {
        return Err(TelemetryStoreError::Refused("telemetry-store-unsafe"));
    }
    Ok(())
}

impl TelemetryStore {
    /// Open the Root's telemetry store. With `create`, a missing file is
    /// created private (0600) with the current schema; without it, a missing
    /// file is `telemetry-store-missing`. The Root's
    /// `.swarm` directory must already exist (the mission Root is never
    /// initialized here). Existing files must be private regular files of this
    /// user and carry a supported schema version (1 to current). With
    /// `create`, an older schema (or an empty file left by an interrupted
    /// creation) is migrated under the write lock, re-reading the version in
    /// the transaction (busy: `telemetry-store-busy`); without it, application
    /// code does not migrate an older schema and an empty file is
    /// `telemetry-store-missing`. Immediately after connecting, this store sets
    /// its existing zero SQLite busy timeout before the initial identity SELECT,
    /// version/schema operations and token writes; it does not inherit the
    /// constructor handler during diagnostic SQL.
    /// Root's separate connection loads its persisted timeout after bootstrap;
    /// the telemetry timeout is not Root's effective persisted policy.
    /// Retains the owned directory and actual SQLite O_PATH inode witness.
    /// FILESTAT support is mandatory. Files must also be single-link; standalone
    /// owned directory permissions remain compatible (0755 allowed). No pathname
    /// fallback or ordinary SQLite witness descriptor is used. SQLite's own
    /// hot-journal recovery during connection/diagnostic is not a zero-effect
    /// guarantee; descriptor association precedes application schema migration.
    pub fn open(root: &Path, create: bool) -> Result<Self, TelemetryStoreError> {
        let path=path(root)?;
        match fs::symlink_metadata(&path) {
            Ok(_) => private_regular(&path)?,
            Err(error) if error.kind()==io::ErrorKind::NotFound && create => {},
            Err(error) if error.kind()==io::ErrorKind::NotFound => return Err(TelemetryStoreError::Refused("telemetry-store-missing")),
            Err(error) => return Err(TelemetryStoreError::Io(error)),
        }
        let directory=crate::local_storage::LocalDirectory::open_owned(path.parent().ok_or(TelemetryStoreError::Refused("telemetry-store-directory-missing"))?)
            .map_err(TelemetryStoreError::Storage)?;
        match Self::connect_bound(&path,&directory,create) {
            Ok((db,identity,version)) => Ok(Self {db,identity,version,directory}),
            Err(error) => telemetry_finished(Err(error),directory.close().map_err(TelemetryStoreError::Storage)),
        }
    }

    /// Open telemetry under an actual retained directory. Compare the pathname's
    /// directory before opening SQLite, then associate the actual connected main
    /// FD with the expected private file before migration or token issuance.
    /// Creation publishes a closed private empty temporary atomically, avoiding
    /// an ordinary data-file close after other SQLite connections can open it.
    /// This binds coherent substitutions, not hostile ABA or arbitrary SQLite
    /// hot-journal recovery during connection/diagnostic preparation.
    pub(crate) fn open_bound(
        root: &Path,
        expected: &crate::local_storage::LocalDirectory,
        create: bool,
    ) -> Result<Self, TelemetryStoreError> {
        let path = path(root)?;
        let current = crate::local_storage::LocalDirectory::open_owned(
            path.parent().ok_or(TelemetryStoreError::Refused("telemetry-store-directory-missing"))?,
        )
        .map_err(TelemetryStoreError::Storage)?;
        let check = (|| {
            if current
                .owned_identity()
                .map_err(TelemetryStoreError::Storage)?
                != expected
                    .owned_identity()
                    .map_err(TelemetryStoreError::Storage)?
            {
                return Err(TelemetryStoreError::Refused(
                    "telemetry-store-instance-changed",
                ));
            }
            Ok(())
        })();
        telemetry_finished(check, current.close().map_err(TelemetryStoreError::Storage))?;
        let directory = expected.duplicate().map_err(TelemetryStoreError::Storage)?;
        match Self::connect_bound(&path, &directory, create) {
            Ok((db, identity, version)) => Ok(Self {
                db,
                identity,
                version,
                directory,
            }),
            Err(error) => telemetry_finished(
                Err(error),
                directory.close().map_err(TelemetryStoreError::Storage),
            ),
        }
    }

    /// Set bootstrap busy policy before the first diagnostic SQL, then associate
    /// the real SQLite handle before any application migration.
    /// Every partial connection/witness failure closes both with causes retained.
    fn connect_bound(
        path: &Path,
        directory: &crate::local_storage::LocalDirectory,
        create: bool,
    ) -> Result<
        (
            Connection,
            crate::connected_database_identity::ConnectedDatabaseIdentity,
            i64,
        ),
        TelemetryStoreError,
    > {
        let expected = match directory.regular_identity(TELEMETRY_DB) {
            Ok(identity) => identity,
            Err(error)
                if error
                    .source()
                    .and_then(|cause| cause.downcast_ref::<rustix::io::Errno>())
                    == Some(&rustix::io::Errno::NOENT) =>
            {
                if !create {
                    return Err(TelemetryStoreError::Refused("telemetry-store-missing"));
                }
                directory
                    .publish_new_with_fault(
                        TELEMETRY_DB,
                        b"",
                        crate::local_storage::AtomicStorageFault::None,
                    )
                    .map_err(TelemetryStoreError::Storage)?;
                directory
                    .regular_identity(TELEMETRY_DB)
                    .map_err(TelemetryStoreError::Storage)?
            }
            Err(error) => return Err(TelemetryStoreError::Storage(error)),
        };
        let mut db = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NOFOLLOW
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        if let Err(error) = db.busy_timeout(Duration::ZERO) {
            return telemetry_finished(Err(TelemetryStoreError::Sql(error)), close_database(db));
        }
        let identity =
            match crate::connected_database_identity::ConnectedDatabaseIdentity::capture(&db) {
                Ok(identity) => identity,
                Err(error) => {
                    return telemetry_finished(Err(identity_error(error)), close_database(db))
                }
            };
        let result = (|| {
            if identity.identity() != expected
                || directory
                    .regular_identity(TELEMETRY_DB)
                    .map_err(TelemetryStoreError::Storage)?
                    != expected
            {
                return Err(TelemetryStoreError::Refused(
                    "telemetry-store-instance-changed",
                ));
            }
            let version: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
            match version {
                TELEMETRY_SCHEMA_VERSION => Ok(version),
                1..=6 if !create => Ok(version),
                0 if !create => Err(TelemetryStoreError::Refused("telemetry-store-missing")),
                0..=6 => Self::migrate(&mut db),
                _ => Err(TelemetryStoreError::Refused(
                    "telemetry-store-version-unsupported",
                )),
            }
        })();
        match result {
            Ok(version) => Ok((db, identity, version)),
            Err(error) => telemetry_finished(
                telemetry_finished(Err(error), close_database(db)),
                identity.close().map_err(identity_error),
            ),
        }
    }

    /// Refuse a replaced telemetry file before a token write; the retained
    /// witness pins the original connection's inode, never a caller's tuple.
    fn check_binding(&self) -> Result<(), TelemetryStoreError> {
        if self
            .directory
            .regular_identity(TELEMETRY_DB)
            .map_err(TelemetryStoreError::Storage)?
            != self.identity.identity()
        {
            return Err(TelemetryStoreError::Refused(
                "telemetry-store-instance-changed",
            ));
        }
        Ok(())
    }

    /// Bring a v0 (empty) to v6 store to the current schema under the
    /// write lock, re-reading the version inside the transaction so a
    /// concurrent opener that migrated first is not migrated twice. A v0 file
    /// that already holds tables is refused (`telemetry-store-version-unsupported`).
    /// Busy → `telemetry-store-busy`; on any error nothing is changed.
    fn migrate(db: &mut Connection) -> Result<i64, TelemetryStoreError> {
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let version: i64 = tx.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        match version {
            0 => {
                let tables: i64 = tx.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0))?;
                if tables != 0 {
                    return Err(TelemetryStoreError::Refused("telemetry-store-version-unsupported"));
                }
                tx.execute_batch(&format!("{SCHEMA} {ALERTS_V2} {ALERTS_V3} {RETENTION_V4} {ANNOTATIONS_V5} {EVALUATIONS_V6} {SCENARIOS_V7}"))?;
            }
            1 => tx.execute_batch(&format!("{ALERTS_V2} {ALERTS_V3} {RETENTION_V4} {ANNOTATIONS_V5} {EVALUATIONS_V6} {SCENARIOS_V7}"))?,
            2 => tx.execute_batch(&format!("{ALERTS_V3} {RETENTION_V4} {ANNOTATIONS_V5} {EVALUATIONS_V6} {SCENARIOS_V7}"))?,
            3 => tx.execute_batch(&format!("{RETENTION_V4} {ANNOTATIONS_V5} {EVALUATIONS_V6} {SCENARIOS_V7}"))?,
            4 => tx.execute_batch(&format!("{ANNOTATIONS_V5} {EVALUATIONS_V6} {SCENARIOS_V7}"))?,
            5 => tx.execute_batch(&format!("{EVALUATIONS_V6} {SCENARIOS_V7}"))?,
            6 => tx.execute_batch(SCENARIOS_V7)?,
            TELEMETRY_SCHEMA_VERSION => {}
            _ => return Err(TelemetryStoreError::Refused("telemetry-store-version-unsupported")),
        }
        tx.execute_batch(&format!("PRAGMA user_version = {TELEMETRY_SCHEMA_VERSION};"))?;
        tx.commit()?;
        Ok(TELEMETRY_SCHEMA_VERSION)
    }

    /// Migrate an older store opened without `create` to the current schema
    /// (write paths only, e.g. ingest once its token is authenticated). Same
    /// errors as [`Self::open`] with `create`.
    pub fn ensure_current(&mut self) -> Result<(), TelemetryStoreError> {
        if self.version != TELEMETRY_SCHEMA_VERSION {
            self.version = Self::migrate(&mut self.db)?;
        }
        Ok(())
    }

    /// Retention purge outside an ingest (e.g. alert evaluation): rows older
    /// than `days` before `now_ms`, see [`StoreLimits::retention_days`]. Takes
    /// the write lock only when something is due. Requires the current schema.
    pub fn apply_retention(&mut self, days: i64, now_ms: i64) -> Result<PurgeReport, TelemetryStoreError> {
        if self.version != TELEMETRY_SCHEMA_VERSION {
            return Err(TelemetryStoreError::Refused("telemetry-store-version-unsupported"));
        }
        let cutoff = now_ms.saturating_sub(days.saturating_mul(DAY_MS));
        let due: i64 = self.db.query_row(
            "SELECT (SELECT count(*) FROM records WHERE received_ms < ?1)
                  + (SELECT count(*) FROM alerts WHERE resolved_ms IS NOT NULL AND resolved_ms < ?1)
                  + (SELECT count(*) FROM tokens WHERE revoked_at_ms IS NOT NULL AND revoked_at_ms < ?1)",
            [cutoff],
            |r| r.get(0),
        )?;
        if due == 0 {
            return Ok(PurgeReport::default());
        }
        let tx = self.db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let report = purge_in(&tx, days, now_ms)?;
        tx.commit()?;
        Ok(report)
    }

    /// Cumulative purge summary, `None` before the first purge or on a store
    /// older than v4 (read as is). Read only.
    pub fn retention_state(&self) -> Result<Option<PurgeState>, TelemetryStoreError> {
        if self.version < 4 {
            return Ok(None);
        }
        Ok(self
            .db
            .query_row("SELECT at_ms, cutoff_ms, records, alerts, tokens FROM purges WHERE id = 1", [], |r| {
                Ok(PurgeState {
                    at_ms: r.get(0)?,
                    cutoff_ms: r.get(1)?,
                    removed: PurgeReport { records: r.get::<_, i64>(2)? as u64, alerts: r.get::<_, i64>(3)? as u64, tokens: r.get::<_, i64>(4)? as u64 },
                })
            })
            .optional()?)
    }

    /// Close SQLite, its O_PATH witness and the owned directory explicitly,
    /// preserving every primary/cleanup error. No token is revoked by closing.
    pub fn close(self) -> Result<(), TelemetryStoreError> {
        telemetry_finished(
            telemetry_finished(
                close_database(self.db),
                self.identity.close().map_err(identity_error),
            ),
            self.directory.close().map_err(TelemetryStoreError::Storage),
        )
    }

    /// Issue a fresh random token for `agent_id` and persist only its digest.
    /// The returned secret must be handed to the provider process and never
    /// logged or persisted by the caller. Several tokens per agent may coexist
    /// (one per launch); each is revoked by [`Self::revoke_agent`]. The retained
    /// telemetry filename must still match the actual connection's pinned inode
    /// before INSERT. Empty agent IDs refuse before the write. Success requires
    /// exactly one reported INSERT change and matching digest/agent/issue-time/
    /// null-revocation readback, with explicit statement finalization before the
    /// secret is returned. INSERT0 refuses without a secret. After INSERT1,
    /// readback mismatch, SQL or finalization failure retains TokenIssued and
    /// both primary/cleanup causes: it records that write stage, not a promise
    /// that a trigger, concurrent writer or later operation kept the row alive.
    /// SQL errors and affected-row failures are not a general rollback or
    /// zero-effect assertion. No token text is stored in the error or Debug.
    pub fn issue_token(&self, agent_id: &str, now_ms: i64) -> Result<String, TelemetryStoreError> {
        self.check_binding()?;
        if agent_id.is_empty() {
            return Err(TelemetryStoreError::Refused("telemetry-agent-invalid"));
        }
        let mut raw = [0u8; TELEMETRY_TOKEN_HEX / 2];
        getrandom::getrandom(&mut raw).map_err(TelemetryStoreError::Random)?;
        let secret = hex(&raw);
        let digest=token_digest(&secret);
        let changed=self.db.execute(
            "INSERT INTO tokens(digest, agent_id, issued_at_ms) VALUES (?1, ?2, ?3)",
            params![&digest, agent_id, now_ms],
        )?;
        if changed!=1 {
            return Err(TelemetryStoreError::Refused("telemetry-token-insert-not-recorded"));
        }
        // INSERT1 is already reported by SQLite's autocommit operation. Any
        // subsequent readback/finalize failure retains that write observation.
        let verified=(|| {
            let mut statement=self.db.prepare("SELECT agent_id,issued_at_ms,revoked_at_ms FROM tokens WHERE digest=?1")?;
            let result=statement.query_row([&digest],|row|Ok((row.get::<_,String>(0)?,row.get::<_,i64>(1)?,row.get::<_,Option<i64>>(2)?)))
                .optional().map_err(TelemetryStoreError::Sql);
            let row=telemetry_finished(result,statement.finalize().map_err(TelemetryStoreError::Sql))?;
            if row!=Some((agent_id.to_owned(),now_ms,None)) {
                return Err(TelemetryStoreError::Refused("telemetry-token-persistence-conflict"));
            }
            Ok(())
        })();
        verified.map_err(|error|TelemetryStoreError::TokenIssued {primary:Box::new(error)})?;
        Ok(secret)
    }

    /// Revoke every token of `agent_id`; returns how many were still valid.
    pub fn revoke_agent(&self, agent_id: &str, now_ms: i64) -> Result<u64, TelemetryStoreError> {
        let n = self.db.execute(
            "UPDATE tokens SET revoked_at_ms = ?2 WHERE agent_id = ?1 AND revoked_at_ms IS NULL",
            params![agent_id, now_ms],
        )?;
        Ok(n as u64)
    }

    /// Agent of a non-revoked token, or None for an unknown/revoked/malformed
    /// secret (callers answer the same refusal for all three).
    pub fn token_agent(&self, secret: &str) -> Result<Option<String>, TelemetryStoreError> {
        if !valid_token_syntax(secret) {
            return Ok(None);
        }
        Ok(self
            .db
            .query_row(
                "SELECT agent_id FROM tokens WHERE digest = ?1 AND revoked_at_ms IS NULL",
                [token_digest(secret)],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Store a normalized batch for `agent_id` in one transaction: records by
    /// agent-scoped digest (replays counted as duplicates), unknown names and
    /// rejected items added to their counters. With `limits`: new records
    /// beyond `max_records_per_agent` are refused and counted (`capped`, and
    /// in the rejection counter), and the retention purge runs in the same
    /// transaction. Limits require the current schema
    /// (`telemetry-store-version-unsupported`: call [`Self::ensure_current`]).
    /// On error nothing is kept, so the exporter may retry the whole request
    /// safely; busy is `telemetry-store-busy`.
    pub fn record(&mut self, agent_id: &str, batch: &NormalizedBatch, received_ms: i64, limits: StoreLimits) -> Result<RecordOutcome, TelemetryStoreError> {
        if limits != StoreLimits::default() && self.version != TELEMETRY_SCHEMA_VERSION {
            return Err(TelemetryStoreError::Refused("telemetry-store-version-unsupported"));
        }
        let tx = self.db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut outcome = RecordOutcome::default();
        let mut stored: i64 = match limits.max_records_per_agent {
            Some(_) => tx.query_row("SELECT count(*) FROM records WHERE agent_id = ?1", [agent_id], |r| r.get(0))?,
            None => 0,
        };
        {
            let mut exists = tx.prepare("SELECT 1 FROM records WHERE digest = ?1")?;
            let mut insert = tx.prepare(
                "INSERT OR IGNORE INTO records(digest, agent_id, kind, name, at_ms, received_ms, value, temporality, fields)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;
            for r in &batch.records {
                let fields = serde_json::to_string(&r.fields).map_err(|e| TelemetryStoreError::Io(e.into()))?;
                let digest = hex(&Sha256::digest(format!("{agent_id}\n{}", r.digest).as_bytes()));
                if limits.max_records_per_agent.is_some_and(|cap| stored >= cap) {
                    if exists.exists([&digest])? {
                        outcome.duplicates += 1;
                    } else {
                        outcome.capped += 1;
                    }
                    continue;
                }
                let n = insert.execute(params![
                    digest,
                    agent_id,
                    r.kind.as_str(),
                    r.name,
                    r.at_ms,
                    received_ms,
                    r.value,
                    r.temporality.map(|t| t.as_str()),
                    fields
                ])?;
                if n == 1 {
                    outcome.inserted += 1;
                    stored += 1;
                } else {
                    outcome.duplicates += 1;
                }
            }
            let mut unknown = tx.prepare(
                "INSERT INTO unknown_names(agent_id, name, count) VALUES (?1, ?2, ?3)
                 ON CONFLICT(agent_id, name) DO UPDATE SET count = count + excluded.count",
            )?;
            for (name, count) in &batch.unknown {
                unknown.execute(params![agent_id, name, *count as i64])?;
            }
            let rejected = batch.rejected + outcome.capped;
            if rejected > 0 {
                tx.execute(
                    "INSERT INTO rejections(agent_id, count) VALUES (?1, ?2)
                     ON CONFLICT(agent_id) DO UPDATE SET count = count + excluded.count",
                    params![agent_id, rejected as i64],
                )?;
            }
        }
        if let Some(days) = limits.retention_days {
            outcome.purged = purge_in(&tx, days, received_ms)?;
        }
        tx.commit()?;
        Ok(outcome)
    }


    /// Reconcile the stored open alerts with the conditions true now.
    ///
    /// A condition without an open row fires (row opened at `now_ms`); an open
    /// row whose condition is gone resolves at `now_ms`, with `resolution`
    /// `cleared` when its subject is still in `present`, `gone` otherwise.
    /// Duplicate conditions count once. Idempotent: the same input twice
    /// changes nothing. When nothing changes, no write lock is taken (a
    /// steady console does not contend with OTLP ingest); otherwise the
    /// difference is recomputed under an IMMEDIATE transaction. Errors:
    /// `telemetry-store-busy` when another writer holds the lock,
    /// `telemetry-store-version-unsupported` on a store not opened for
    /// writing at the current schema, `telemetry-store-unavailable` for other
    /// faults; on any error nothing is kept.
    pub fn sync_alerts(
        &mut self,
        current: &[crate::AlertCondition],
        present: &BTreeSet<String>,
        now_ms: i64,
    ) -> Result<AlertSync, TelemetryStoreError> {
        if self.version != TELEMETRY_SCHEMA_VERSION {
            return Err(TelemetryStoreError::Refused("telemetry-store-version-unsupported"));
        }
        let mut seen = BTreeSet::new();
        let current: Vec<&crate::AlertCondition> = current.iter().filter(|c| seen.insert((c.rule, c.subject.as_str()))).collect();
        let open_rows = |db: &Connection| -> Result<Vec<AlertRow>, TelemetryStoreError> {
            let mut stmt = db.prepare(&format!("SELECT {ALERT_COLUMNS} FROM alerts WHERE resolved_ms IS NULL"))?;
            let rows = stmt.query_map([], alert_row)?.collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        };
        let changed = |open: &[AlertRow]| {
            open.len() != current.len()
                || current.iter().any(|c| !open.iter().any(|o| o.rule == c.rule && o.subject == c.subject))
        };
        if !changed(&open_rows(&self.db)?) {
            return Ok(AlertSync::default());
        }
        let tx = self.db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut sync = AlertSync::default();
        let open = open_rows(&tx)?;
        for c in &current {
            if open.iter().any(|o| o.rule == c.rule && o.subject == c.subject) {
                continue;
            }
            tx.execute(
                "INSERT INTO alerts(rule, level, subject, work_id, fired_ms, occurred) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![c.rule, c.level, c.subject, c.work_id, now_ms, c.occurred],
            )?;
            sync.fired.push(AlertRow {
                id: tx.last_insert_rowid(),
                rule: c.rule.into(),
                level: c.level.into(),
                subject: c.subject.clone(),
                work_id: c.work_id.clone(),
                fired_ms: now_ms,
                occurred: c.occurred.clone(),
                resolved_ms: None,
                resolution: String::new(),
            });
        }
        for mut o in open {
            if current.iter().any(|c| c.rule == o.rule && c.subject == o.subject) {
                continue;
            }
            let resolution = if present.contains(&o.subject) { "cleared" } else { "gone" };
            tx.execute("UPDATE alerts SET resolved_ms = ?2, resolution = ?3 WHERE id = ?1", params![o.id, now_ms, resolution])?;
            o.resolved_ms = Some(now_ms);
            o.resolution = resolution.into();
            sync.resolved.push(o);
        }
        tx.commit()?;
        Ok(sync)
    }

    /// Open alerts (oldest first) and resolved history (newest first), each
    /// bounded by [`LIVE_PAGE_ROWS`]. Read only: a store older than v2 has no
    /// alerts (empty view); a v2 store opened for reading reports an empty
    /// `resolution`.
    pub fn read_alerts(&self) -> Result<AlertView, TelemetryStoreError> {
        if self.version < 2 {
            return Ok(AlertView::default());
        }
        let columns = if self.version < 3 { ALERT_COLUMNS.replace("resolution", "'' AS resolution") } else { ALERT_COLUMNS.to_owned() };
        let page = |filter: &str, order: &str| -> Result<(Vec<AlertRow>, bool), TelemetryStoreError> {
            let mut stmt = self.db.prepare(&format!("SELECT {columns} FROM alerts WHERE {filter} ORDER BY {order} LIMIT ?1"))?;
            let mut rows = stmt.query_map([LIVE_PAGE_ROWS as i64 + 1], alert_row)?.collect::<Result<Vec<_>, _>>()?;
            let more = rows.len() > LIVE_PAGE_ROWS;
            rows.truncate(LIVE_PAGE_ROWS);
            Ok((rows, more))
        };
        let (open, open_more) = page("resolved_ms IS NULL", "fired_ms, id")?;
        let (history, history_more) = page("resolved_ms IS NOT NULL", "resolved_ms DESC, id DESC")?;
        Ok(AlertView { open, open_more, history, history_more })
    }

    /// Opaque change marker over the whole store (record count and latest
    /// arrival, unknown-name and rejection counters, valid tokens, and — from
    /// v2 — open-alert count and newest alert id), mixed into the live
    /// fingerprint so a telemetry arrival or an alert transition wakes pushed
    /// live views. Not an identifier; content-free.
    pub fn change_marker(&self) -> Result<String, TelemetryStoreError> {
        let alerts = match self.version {
            7.. => "|| ':' || (SELECT count(*) FROM alerts WHERE resolved_ms IS NULL) || '/' || (SELECT coalesce(max(id), 0) FROM alerts)
                    || ':' || (SELECT count(*) FROM annotations) || '/' || (SELECT coalesce(max(updated_ms), 0) FROM annotations)
                    || ':' || (SELECT count(*) FROM evaluations) || ':' || (SELECT coalesce(max(updated_ms), 0) FROM scenarios)",
            6 => "|| ':' || (SELECT count(*) FROM alerts WHERE resolved_ms IS NULL) || '/' || (SELECT coalesce(max(id), 0) FROM alerts)
                    || ':' || (SELECT count(*) FROM annotations) || '/' || (SELECT coalesce(max(updated_ms), 0) FROM annotations)
                    || ':' || (SELECT count(*) FROM evaluations)",
            5 => "|| ':' || (SELECT count(*) FROM alerts WHERE resolved_ms IS NULL) || '/' || (SELECT coalesce(max(id), 0) FROM alerts)
                    || ':' || (SELECT count(*) FROM annotations) || '/' || (SELECT coalesce(max(updated_ms), 0) FROM annotations)",
            2.. => "|| ':' || (SELECT count(*) FROM alerts WHERE resolved_ms IS NULL) || '/' || (SELECT coalesce(max(id), 0) FROM alerts)",
            _ => "",
        };
        Ok(self.db.query_row(
            &format!(
                "SELECT (SELECT count(*) FROM records) || ':' || (SELECT coalesce(max(received_ms), 0) FROM records)
              || ':' || (SELECT coalesce(sum(count), 0) FROM unknown_names) || ':' || (SELECT coalesce(sum(count), 0) FROM rejections)
              || ':' || (SELECT count(*) FROM tokens WHERE revoked_at_ms IS NULL) {alerts}"
            ),
            [],
            |r| r.get(0),
        )?)
    }

    /// Sum of declared output tokens over each agent's model calls
    /// (`api_request` events, Claude or mapped Codex), for the
    /// `agent-usage-mismatch` rule. Agents without any call are absent; a
    /// call without a decimal `output_tokens` counts as zero. Read only.
    pub fn output_tokens_by_agent(&self) -> Result<BTreeMap<String, u64>, TelemetryStoreError> {
        let mut stmt = self.db.prepare(
            "SELECT agent_id, sum(CAST(coalesce(json_extract(fields, '$.output_tokens'), 0) AS INTEGER))
             FROM records WHERE kind = 'event' AND name = 'api_request' GROUP BY agent_id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        let mut out = BTreeMap::new();
        for row in rows {
            let (agent, sum) = row?;
            out.insert(agent, u64::try_from(sum).unwrap_or(0));
        }
        Ok(out)
    }

    /// Aggregated read model for one agent (see [`AgentTelemetry`]).
    pub fn read_agent(&self, agent_id: &str) -> Result<AgentTelemetry, TelemetryStoreError> {
        let tokens: i64 = self
            .db
            .query_row("SELECT count(*) FROM tokens WHERE agent_id = ?1", [agent_id], |r| r.get(0))?;
        let last: Option<i64> = self
            .db
            .query_row("SELECT max(received_ms) FROM records WHERE agent_id = ?1", [agent_id], |r| r.get(0))?;

        let mut calls = Vec::new();
        let mut stmt = self.db.prepare(
            "SELECT at_ms, fields FROM records WHERE agent_id = ?1 AND kind = 'event' AND name = 'api_request'
             ORDER BY at_ms DESC, digest LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![agent_id, LIVE_PAGE_ROWS as i64 + 1], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (at_ms, raw) = row?;
            calls.push(TelemetryCall { at_ms, fields: fields(&raw)? });
        }
        let calls_more = calls.len() > LIVE_PAGE_ROWS;
        calls.truncate(LIVE_PAGE_ROWS);

        let mut tool_calls = Vec::new();
        let mut stmt = self.db.prepare(
            "SELECT at_ms, fields FROM records WHERE agent_id = ?1 AND kind = 'event' AND name = 'tool_result'
             ORDER BY at_ms DESC, digest LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![agent_id, LIVE_PAGE_ROWS as i64 + 1], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (at_ms, raw) = row?;
            let f = fields(&raw)?;
            let success = match f.get("success") {
                Some(Value::Bool(b)) => Some(*b),
                Some(Value::String(s)) if s == "true" => Some(true),
                Some(Value::String(s)) if s == "false" => Some(false),
                _ => None,
            };
            tool_calls.push(TelemetryToolCall {
                at_ms,
                tool: f.get("tool_name").and_then(Value::as_str).unwrap_or("?").to_owned(),
                duration_ms: f.get("duration_ms").and_then(number),
                success,
            });
        }
        let tool_calls_more = tool_calls.len() > LIVE_PAGE_ROWS;
        tool_calls.truncate(LIVE_PAGE_ROWS);

        let mut tools: BTreeMap<String, TelemetryTool> = BTreeMap::new();
        let mut decisions: BTreeMap<String, u64> = BTreeMap::new();
        let mut stmt = self.db.prepare(
            "SELECT name, fields FROM records WHERE agent_id = ?1 AND kind = 'event' AND name IN ('tool_result', 'tool_decision')",
        )?;
        let rows = stmt.query_map([agent_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (name, raw) = row?;
            let f = fields(&raw)?;
            let tool = f.get("tool_name").and_then(Value::as_str).unwrap_or("?").to_owned();
            if name == "tool_decision" {
                let decision = f.get("decision").and_then(Value::as_str).unwrap_or("?").to_owned();
                *decisions.entry(decision).or_default() += 1;
                continue;
            }
            let entry = tools.entry(tool.clone()).or_insert_with(|| TelemetryTool { name: tool, ..Default::default() });
            entry.count += 1;
            if matches!(f.get("success"), Some(Value::Bool(false))) || f.get("success").and_then(Value::as_str) == Some("false") {
                entry.failures += 1;
            }
            if let Some(ms) = f.get("duration_ms").and_then(number) {
                entry.total_ms += ms;
                entry.max_ms = entry.max_ms.max(ms);
            }
        }

        // Delta points sum; cumulative points keep the newest value per series
        // (name + all kept attributes), then sum over series; gauges keep the
        // newest value per series. A name+type with two temporalities is null.
        let mut series: BTreeMap<SeriesKey, (i64, f64)> = BTreeMap::new();
        let mut stmt = self.db.prepare(
            "SELECT name, fields, temporality, at_ms, value FROM records WHERE agent_id = ?1 AND kind = 'point'",
        )?;
        let rows = stmt.query_map([agent_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?, r.get::<_, f64>(4)?))
        })?;
        for row in rows {
            let (name, raw, temporality, at_ms, value) = row?;
            let kind = fields(&raw)?.get("type").and_then(Value::as_str).map(str::to_owned);
            let key = (name, kind, temporality.clone(), if temporality == "delta" { String::new() } else { raw });
            let slot = series.entry(key).or_insert((i64::MIN, 0.0));
            if temporality == "delta" {
                slot.1 += value;
            } else if at_ms >= slot.0 {
                *slot = (at_ms, value);
            }
        }
        let mut grouped: BTreeMap<(String, Option<String>), (String, f64, bool)> = BTreeMap::new();
        for ((name, kind, temporality, _), (_, value)) in series {
            let slot = grouped.entry((name, kind)).or_insert((temporality.clone(), 0.0, false));
            if slot.0 != temporality {
                slot.2 = true;
            }
            slot.1 += value;
        }
        // Histograms: delta points merge; cumulative keep the newest per
        // series (attributes without the histogram payload).
        let mut newest: BTreeMap<(String, String), NewestPoint> = BTreeMap::new();
        let mut merged: BTreeMap<(String, Option<String>, Option<String>), HistogramMerge> = BTreeMap::new();
        let mut stmt = self.db.prepare(
            "SELECT name, fields, temporality, at_ms, value FROM records WHERE agent_id = ?1 AND kind = 'histogram'",
        )?;
        let rows = stmt.query_map([agent_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?, r.get::<_, f64>(4)?))
        })?;
        let group = |name: &str, f: &BTreeMap<String, Value>| {
            let text = |k: &str| f.get(k).and_then(Value::as_str).map(str::to_owned);
            (name.to_owned(), text("model"), text("token_type"))
        };
        for row in rows {
            let (name, raw, temporality, at_ms, value) = row?;
            let f = fields(&raw)?;
            if temporality == "delta" {
                merged.entry(group(&name, &f)).or_default().add(&f, value);
            } else {
                let mut key_fields = f.clone();
                for k in ["count", "min", "max", "buckets", "bounds"] {
                    key_fields.remove(k);
                }
                let key = (name, serde_json::to_string(&key_fields).map_err(|e| TelemetryStoreError::Io(e.into()))?);
                if newest.get(&key).is_none_or(|(t, _, _)| at_ms >= *t) {
                    newest.insert(key, (at_ms, f, value));
                }
            }
        }
        for ((name, _), (_, f, value)) in newest {
            merged.entry(group(&name, &f)).or_default().add(&f, value);
        }
        let mut latencies = Vec::new();
        for ((name, model, token_type), m) in merged {
            if name == "turn.token_usage" {
                grouped.insert((name, token_type), ("histogram".into(), m.sum, false));
            } else if name.ends_with("duration_ms") {
                latencies.push(TelemetryLatency {
                    mean_ms: (m.count > 0).then(|| m.sum / m.count as f64),
                    p50_ms: m.quantile(0.5),
                    p95_ms: m.quantile(0.95),
                    max_ms: m.max,
                    count: m.count,
                    name,
                    model,
                });
            }
        }

        let totals = grouped
            .into_iter()
            .map(|((name, kind), (temporality, value, mixed))| TelemetryTotal {
                name,
                kind,
                temporality: if mixed { "mixed".into() } else { temporality },
                value: (!mixed).then_some(value),
            })
            .collect();

        // Spans of the newest trace (by latest start), oldest first.
        let newest_trace: Option<String> = self
            .db
            .query_row(
                "SELECT json_extract(fields, '$.trace_id') FROM records WHERE agent_id = ?1 AND kind = 'span' ORDER BY at_ms DESC LIMIT 1",
                [agent_id],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        let mut spans = Vec::new();
        if let Some(trace) = newest_trace {
            let mut stmt = self.db.prepare(
                "SELECT name, at_ms, value, fields FROM records WHERE agent_id = ?1 AND kind = 'span'
                 AND json_extract(fields, '$.trace_id') = ?2 ORDER BY at_ms, digest LIMIT ?3",
            )?;
            let rows = stmt.query_map(params![agent_id, trace, LIVE_PAGE_ROWS as i64 + 1], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, Option<f64>>(2)?, r.get::<_, String>(3)?))
            })?;
            for row in rows {
                let (name, start_ms, value, raw) = row?;
                spans.push(TelemetrySpan { name, start_ms, duration_ms: value.unwrap_or(0.0), fields: fields(&raw)? });
            }
        }
        let spans_more = spans.len() > LIVE_PAGE_ROWS;
        spans.truncate(LIVE_PAGE_ROWS);

        let mut unknown = BTreeMap::new();
        let mut stmt = self.db.prepare("SELECT name, count FROM unknown_names WHERE agent_id = ?1")?;
        for row in stmt.query_map([agent_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
            let (name, count) = row?;
            unknown.insert(name, count as u64);
        }
        let rejected: i64 = self
            .db
            .query_row("SELECT count FROM rejections WHERE agent_id = ?1", [agent_id], |r| r.get(0))
            .optional()?
            .unwrap_or(0);
        Ok(AgentTelemetry {
            state: if last.is_some() { "received" } else if tokens > 0 { "waiting" } else { "no-token" },
            provenance: "provider-declared-unauthenticated",
            last_received_ms: last,
            calls,
            calls_more,
            tools: tools.into_values().collect(),
            tool_calls,
            tool_calls_more,
            decisions,
            totals,
            latencies,
            spans,
            spans_more,
            unknown,
            rejected: rejected as u64,
        })
    }
}

/// Metric series: name, `type` attribute, temporality, kept attributes
/// (empty for delta points, which are summed regardless of other attributes).
type SeriesKey = (String, Option<String>, String, String);

/// Decode a stored field map. Rows are written by [`TelemetryStore::record`]
/// only, so a decode failure is store corruption and is reported as
/// `telemetry-store-unavailable` (never read as an empty map).
fn fields(raw: &str) -> Result<BTreeMap<String, Value>, TelemetryStoreError> {
    serde_json::from_str(raw).map_err(|e| TelemetryStoreError::Io(e.into()))
}

/// Numeric value of a field given as number or numeric string (Claude Code
/// sends most attributes as strings).
fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Accepted token spelling: exactly [`TELEMETRY_TOKEN_HEX`] lowercase hex.
pub fn valid_token_syntax(secret: &str) -> bool {
    secret.len() == TELEMETRY_TOKEN_HEX && secret.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn token_digest(secret: &str) -> String {
    hex(&Sha256::digest(secret.as_bytes()))
}

/// One operator annotation of an agent attempt (store v5, LangWatch L10).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Annotation {
    pub agent_id: String,
    pub work_id: String,
    /// `good`, `bad` or `unsure`.
    pub verdict: String,
    /// Operator text, as written (bounded by settings v36).
    pub note: String,
    /// 1 on creation, +1 per accepted change.
    pub revision: i64,
    pub updated_ms: i64,
}

/// One stored evaluation row (store v6), see [`crate::AttemptEvaluation`].
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StoredEvaluation {
    pub evaluator: String,
    pub score: Option<f64>,
    pub passed: Option<bool>,
    pub detail: String,
    pub at_ms: i64,
}

/// Annotation verdicts accepted by [`TelemetryStore::annotate`].
pub const ANNOTATION_VERDICTS: [&str; 3] = ["good", "bad", "unsure"];

impl TelemetryStore {
    /// Create or change the operator annotation of `agent` under optimistic
    /// concurrency: `expected_revision` 0 creates (refused when one exists),
    /// N changes revision N (refused when the stored revision differs) —
    /// `annotation-revision-conflict` either way, nothing written. Unknown
    /// verdict: `annotation-verdict-invalid`. The caller validates the agent,
    /// its Work and the note bounds. Requires the current schema; one
    /// IMMEDIATE transaction (busy: `telemetry-store-busy`).
    pub fn annotate(
        &mut self,
        agent: &str,
        work: &str,
        verdict: &str,
        note: &str,
        expected_revision: i64,
        now_ms: i64,
    ) -> Result<Annotation, TelemetryStoreError> {
        if self.version != TELEMETRY_SCHEMA_VERSION {
            return Err(TelemetryStoreError::Refused("telemetry-store-version-unsupported"));
        }
        if !ANNOTATION_VERDICTS.contains(&verdict) {
            return Err(TelemetryStoreError::Refused("annotation-verdict-invalid"));
        }
        let tx = self.db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let stored: Option<(i64, String)> = match tx.query_row(
            "SELECT revision, work_id FROM annotations WHERE agent_id = ?1",
            [agent],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ) {
            Ok(row) => Some(row),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(e.into()),
        };
        let revision = match stored {
            None if expected_revision == 0 => 1,
            Some((current, ref owner)) if current == expected_revision && owner == work => current + 1,
            _ => return Err(TelemetryStoreError::Refused("annotation-revision-conflict")),
        };
        tx.execute(
            "INSERT INTO annotations(agent_id, work_id, verdict, note, revision, updated_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(agent_id) DO UPDATE SET verdict = excluded.verdict, note = excluded.note,
               revision = excluded.revision, updated_ms = excluded.updated_ms",
            rusqlite::params![agent, work, verdict, note, revision, now_ms],
        )?;
        tx.commit()?;
        Ok(Annotation { agent_id: agent.to_owned(), work_id: work.to_owned(), verdict: verdict.to_owned(), note: note.to_owned(), revision, updated_ms: now_ms })
    }

    /// Create or replace the scenario of `work`/`task` under optimistic
    /// concurrency (`expected_revision` 0 creates; N replaces revision N;
    /// otherwise `scenario-revision-conflict`, nothing written). The caller
    /// validates the specification ([`crate::validate_scenario`]) and that
    /// the task exists. Requires the current schema; busy: `telemetry-store-busy`.
    pub fn save_scenario(
        &mut self,
        work: &str,
        task: &str,
        spec: &crate::ScenarioSpec,
        expected_revision: i64,
        now_ms: i64,
    ) -> Result<i64, TelemetryStoreError> {
        if self.version != TELEMETRY_SCHEMA_VERSION {
            return Err(TelemetryStoreError::Refused("telemetry-store-version-unsupported"));
        }
        let raw = serde_json::to_string(spec).map_err(|e| TelemetryStoreError::Io(io::Error::other(e)))?;
        let tx = self.db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let current: Option<i64> = tx
            .query_row("SELECT revision FROM scenarios WHERE work_id = ?1 AND task_id = ?2", [work, task], |r| r.get(0))
            .optional()?;
        let revision = match current {
            None if expected_revision == 0 => 1,
            Some(n) if n == expected_revision => n + 1,
            _ => return Err(TelemetryStoreError::Refused("scenario-revision-conflict")),
        };
        tx.execute(
            "INSERT INTO scenarios(work_id, task_id, spec, revision, updated_ms) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(work_id, task_id) DO UPDATE SET spec = excluded.spec, revision = excluded.revision, updated_ms = excluded.updated_ms",
            params![work, task, raw, revision, now_ms],
        )?;
        tx.commit()?;
        Ok(revision)
    }

    /// Scenarios of one Work keyed by task id, with revision and update
    /// time; empty before v7. A stored specification that no longer decodes
    /// is refused (`scenario-stored-invalid`) rather than skipped. Read only.
    pub fn read_scenarios(&self, work: &str) -> Result<BTreeMap<String, (crate::ScenarioSpec, i64, i64)>, TelemetryStoreError> {
        if self.version < 7 {
            return Ok(BTreeMap::new());
        }
        let mut stmt = self.db.prepare("SELECT task_id, spec, revision, updated_ms FROM scenarios WHERE work_id = ?1")?;
        let rows = stmt.query_map([work], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?)))?;
        let mut out = BTreeMap::new();
        for row in rows {
            let (task, raw, revision, at) = row?;
            let spec = serde_json::from_str(&raw).map_err(|_| TelemetryStoreError::Refused("scenario-stored-invalid"))?;
            out.insert(task, (spec, revision, at));
        }
        Ok(out)
    }

    /// Attempts that already have stored evaluations (any evaluator), so
    /// each finished attempt is evaluated once. Empty before v6. Read only.
    pub fn evaluated_agents(&self) -> Result<std::collections::BTreeSet<String>, TelemetryStoreError> {
        if self.version < 6 {
            return Ok(Default::default());
        }
        let mut stmt = self.db.prepare("SELECT DISTINCT agent_id FROM evaluations")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<Result<_, _>>().map_err(Into::into)
    }

    /// Store the evaluations of finished attempts in one IMMEDIATE
    /// transaction; an attempt already evaluated keeps its rows (first
    /// write wins: evaluations are not recomputed when telemetry is later
    /// purged). Requires the current schema; busy: `telemetry-store-busy`.
    /// Returns the number of attempts newly recorded.
    pub fn record_evaluations(
        &mut self,
        batch: &[(String, String, Vec<crate::AttemptEvaluation>)],
        now_ms: i64,
    ) -> Result<usize, TelemetryStoreError> {
        if self.version != TELEMETRY_SCHEMA_VERSION {
            return Err(TelemetryStoreError::Refused("telemetry-store-version-unsupported"));
        }
        if batch.is_empty() {
            return Ok(0);
        }
        let tx = self.db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut recorded = 0;
        for (agent, work, evaluations) in batch {
            let mut inserted = 0;
            for e in evaluations {
                inserted += tx.execute(
                    "INSERT OR IGNORE INTO evaluations(agent_id, work_id, evaluator, score, passed, detail, at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![agent, work, e.evaluator, e.score, e.passed, e.detail, now_ms],
                )?;
            }
            recorded += usize::from(inserted > 0);
        }
        tx.commit()?;
        Ok(recorded)
    }

    /// Stored evaluations of one attempt, by evaluator name; empty before
    /// v6 or when not evaluated (yet, or not sampled). Read only.
    pub fn read_evaluations(&self, agent: &str) -> Result<Vec<StoredEvaluation>, TelemetryStoreError> {
        if self.version < 6 {
            return Ok(Vec::new());
        }
        let mut stmt = self.db.prepare("SELECT evaluator, score, passed, detail, at_ms FROM evaluations WHERE agent_id = ?1 ORDER BY evaluator")?;
        let rows = stmt.query_map([agent], |r| {
            Ok(StoredEvaluation { evaluator: r.get(0)?, score: r.get(1)?, passed: r.get(2)?, detail: r.get(3)?, at_ms: r.get(4)? })
        })?;
        rows.collect::<Result<_, _>>().map_err(Into::into)
    }

    /// Verdict of every annotated attempt keyed by agent id (all Works),
    /// for the console queue; empty before v5. Read only.
    pub fn annotation_verdicts(&self) -> Result<BTreeMap<String, String>, TelemetryStoreError> {
        if self.version < 5 {
            return Ok(BTreeMap::new());
        }
        let mut stmt = self.db.prepare("SELECT agent_id, verdict FROM annotations")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        rows.collect::<Result<_, _>>().map_err(Into::into)
    }

    /// Annotations of one Work keyed by agent id; empty on a store older
    /// than v5 (read as is, never migrated by a read). Read only.
    pub fn read_annotations(&self, work: &str) -> Result<BTreeMap<String, Annotation>, TelemetryStoreError> {
        if self.version < 5 {
            return Ok(BTreeMap::new());
        }
        let mut stmt = self.db.prepare(
            "SELECT agent_id, work_id, verdict, note, revision, updated_ms FROM annotations WHERE work_id = ?1",
        )?;
        let rows = stmt.query_map([work], |r| {
            Ok(Annotation { agent_id: r.get(0)?, work_id: r.get(1)?, verdict: r.get(2)?, note: r.get(3)?, revision: r.get(4)?, updated_ms: r.get(5)? })
        })?;
        let mut out = BTreeMap::new();
        for row in rows {
            let row = row?;
            out.insert(row.agent_id.clone(), row);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn scenarios_save_with_revisions_and_read_per_work() {
        let dir = std::env::temp_dir().join(crate::generate_identifier("scenarios-").unwrap());
        fs::create_dir_all(dir.join(".swarm")).unwrap();
        let mut store = TelemetryStore::open(&dir, true).unwrap();
        let spec: crate::ScenarioSpec = serde_json::from_str(r#"{"schema_version":1,"expectations":[{"kind":"usage-observed"}]}"#).unwrap();
        assert_eq!(store.save_scenario("w", "t1", &spec, 0, 1).unwrap(), 1);
        assert_eq!(store.save_scenario("w", "t1", &spec, 0, 2).unwrap_err().code(), "scenario-revision-conflict");
        assert_eq!(store.save_scenario("w", "t1", &spec, 1, 3).unwrap(), 2);
        store.save_scenario("other", "t1", &spec, 0, 4).unwrap();
        let read = store.read_scenarios("w").unwrap();
        assert_eq!((read.len(), read["t1"].1, read["t1"].2), (1, 2, 3));
        Connection::open(dir.join(".swarm").join(TELEMETRY_DB)).unwrap().execute("UPDATE scenarios SET spec = '{}' WHERE work_id = 'w'", []).unwrap();
        assert_eq!(store.read_scenarios("w").unwrap_err().code(), "scenario-stored-invalid");
        store.close().unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn evaluations_are_recorded_once_per_attempt() {
        let dir = std::env::temp_dir().join(crate::generate_identifier("evaluations-").unwrap());
        fs::create_dir_all(dir.join(".swarm")).unwrap();
        let mut store = TelemetryStore::open(&dir, true).unwrap();
        let eval = |passed| crate::AttemptEvaluation { evaluator: "outcome", score: None, passed: Some(passed), detail: String::new() };
        assert_eq!(store.record_evaluations(&[("a".into(), "w".into(), vec![eval(true)])], 1).unwrap(), 1);
        assert_eq!(store.record_evaluations(&[("a".into(), "w".into(), vec![eval(false)]), ("b".into(), "w".into(), vec![eval(false)])], 2).unwrap(), 1, "a kept, b new");
        assert_eq!(store.read_evaluations("a").unwrap()[0].passed, Some(true), "first write wins");
        assert_eq!(store.evaluated_agents().unwrap().len(), 2);
        assert_eq!(store.record_evaluations(&[], 3).unwrap(), 0);
        store.close().unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn annotations_create_change_conflict_and_stay_per_work() {
        let dir = std::env::temp_dir().join(crate::generate_identifier("annotations-").unwrap());
        fs::create_dir_all(dir.join(".swarm")).unwrap();
        let mut store = TelemetryStore::open(&dir, true).unwrap();
        let first = store.annotate("a", "w", "bad", "wrong file edited", 0, 10).unwrap();
        assert_eq!((first.revision, first.updated_ms), (1, 10));
        let code = |r: Result<Annotation, TelemetryStoreError>| r.unwrap_err().code();
        assert_eq!(code(store.annotate("a", "w", "good", "", 0, 11)), "annotation-revision-conflict", "create over an existing one");
        assert_eq!(code(store.annotate("a", "w", "good", "", 2, 11)), "annotation-revision-conflict", "stale revision");
        assert_eq!(code(store.annotate("a", "w2", "good", "", 1, 11)), "annotation-revision-conflict", "other Work");
        assert_eq!(code(store.annotate("a", "w", "great", "", 1, 11)), "annotation-verdict-invalid");
        let second = store.annotate("a", "w", "good", "fixed by retry", 1, 12).unwrap();
        assert_eq!((second.revision, second.verdict.as_str()), (2, "good"));
        store.annotate("b", "other", "unsure", "", 0, 13).unwrap();
        let read = store.read_annotations("w").unwrap();
        assert_eq!((read.len(), read["a"].note.as_str()), (1, "fixed by retry"));
        let before = store.change_marker().unwrap();
        store.annotate("a", "w", "bad", "", 2, 14).unwrap();
        assert_ne!(before, store.change_marker().unwrap(), "an annotation wakes the console");
        store.close().unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }

    use super::*;
    use crate::telemetry_normalize::{normalize_logs, normalize_metrics};
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;

    /// Private scratch Root removed on drop (no live Root is ever used).
    struct Scratch(PathBuf);
    impl Scratch {
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn plain() -> Scratch {
        let path = std::env::temp_dir().join(crate::generate_identifier("telemetry-").unwrap());
        fs::create_dir(&path).unwrap();
        Scratch(path)
    }
    fn root() -> Scratch {
        let dir = plain();
        fs::create_dir(dir.path().join(".swarm")).unwrap();
        dir
    }

    fn event(name: &str, ns: u64, attrs: Value) -> Value {
        json!({"timeUnixNano": ns.to_string(), "body": {"stringValue": name}, "attributes": attrs})
    }

    fn logs(records: Vec<Value>) -> Vec<u8> {
        json!({"resourceLogs": [{"scopeLogs": [{"logRecords": records}]}]}).to_string().into_bytes()
    }

    fn a(key: &str, value: &str) -> Value {
        json!({"key": key, "value": {"stringValue": value}})
    }

    #[test]
    fn tokens_are_digest_only_and_revocable() {
        let dir = root();
        assert!(matches!(TelemetryStore::open(dir.path(), false), Err(TelemetryStoreError::Refused("telemetry-store-missing"))));
        let store = TelemetryStore::open(dir.path(), true).unwrap();
        let path = dir.path().join(".swarm").join(TELEMETRY_DB);
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        let secret = store.issue_token("agent-a", 1).unwrap();
        assert!(valid_token_syntax(&secret));
        assert_eq!(store.token_agent(&secret).unwrap().as_deref(), Some("agent-a"));
        assert_eq!(store.token_agent(&"0".repeat(64)).unwrap(), None);
        assert_eq!(store.token_agent("short").unwrap(), None);
        assert_eq!(store.revoke_agent("agent-a", 2).unwrap(), 1);
        assert_eq!(store.token_agent(&secret).unwrap(), None);
        assert!(matches!(store.issue_token("", 1), Err(TelemetryStoreError::Refused("telemetry-agent-invalid"))));
        store.close().unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(!bytes.windows(secret.len()).any(|w| w == secret.as_bytes()), "secret persisted");
    }

    #[test]
    fn unsafe_or_foreign_files_are_refused() {
        let dir = root();
        let path = dir.path().join(".swarm").join(TELEMETRY_DB);
        fs::write(&path, b"").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(TelemetryStore::open(dir.path(), true), Err(TelemetryStoreError::Refused("telemetry-store-unsafe"))));
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink("/etc/hostname", &path).unwrap();
        assert!(matches!(TelemetryStore::open(dir.path(), true), Err(TelemetryStoreError::Refused("telemetry-store-unsafe"))));
        fs::remove_file(&path).unwrap();
        // Newer schema written by a future version: refused, never rewritten.
        TelemetryStore::open(dir.path(), true).unwrap().close().unwrap();
        Connection::open(&path).unwrap().execute_batch("PRAGMA user_version = 9").unwrap();
        assert!(matches!(TelemetryStore::open(dir.path(), false), Err(TelemetryStoreError::Refused("telemetry-store-version-unsupported"))));
        let plain = plain();
        assert!(matches!(TelemetryStore::open(plain.path(), true), Err(TelemetryStoreError::Refused("telemetry-store-missing"))));
    }

    #[test]
    fn replay_is_idempotent_and_read_model_aggregates() {
        let dir = root();
        let mut store = TelemetryStore::open(dir.path(), true).unwrap();
        store.issue_token("a", 1).unwrap();
        assert_eq!(store.read_agent("a").unwrap().state, "waiting");
        assert_eq!(store.read_agent("b").unwrap().state, "no-token");
        let body = logs(vec![
            event("claude_code.api_request", 1_000_000_000, json!([a("model", "m1"), a("input_tokens", "10")])),
            event("claude_code.api_request", 2_000_000_000, json!([a("model", "m1"), a("input_tokens", "20")])),
            event("claude_code.tool_result", 3_000_000_000, json!([a("tool_name", "Bash"), a("success", "false"), a("duration_ms", "40")])),
            event("claude_code.tool_result", 4_000_000_000, json!([a("tool_name", "Bash"), a("success", "true"), a("duration_ms", "60")])),
            event("claude_code.tool_decision", 5_000_000_000, json!([a("tool_name", "Edit"), a("decision", "reject")])),
            event("claude_code.new_thing", 6_000_000_000, json!([])),
            json!(7),
        ]);
        let batch = normalize_logs(&body, 10).unwrap();
        assert_eq!(store.record("a", &batch, 10, StoreLimits::default()).unwrap(), RecordOutcome { inserted: 5, ..Default::default() });
        assert_eq!(store.record("a", &batch, 11, StoreLimits::default()).unwrap(), RecordOutcome { duplicates: 5, ..Default::default() });
        // Same records for another agent are distinct (agent-scoped digest).
        assert_eq!(store.record("b", &batch, 12, StoreLimits::default()).unwrap().inserted, 5);
        let view = store.read_agent("a").unwrap();
        assert_eq!(view.state, "received");
        assert_eq!(view.last_received_ms, Some(10));
        assert_eq!(view.calls.len(), 2);
        assert_eq!(view.calls[0].at_ms, 2000, "newest first");
        assert_eq!(view.tools, vec![TelemetryTool { name: "Bash".into(), count: 2, failures: 1, total_ms: 100.0, max_ms: 60.0 }]);
        assert_eq!(view.tool_calls.len(), 2);
        assert_eq!(view.tool_calls[0], TelemetryToolCall { at_ms: 4000, tool: "Bash".into(), duration_ms: Some(60.0), success: Some(true) });
        assert_eq!(view.tool_calls[1].success, Some(false), "newest first");
        assert_eq!(view.decisions.get("reject"), Some(&1));
        assert_eq!(view.unknown.get("new_thing"), Some(&2), "counted on each delivery");
        assert_eq!(view.rejected, 2);
        assert_eq!(view.provenance, "provider-declared-unauthenticated");
    }

    #[test]
    fn totals_respect_temporality_and_never_mix() {
        let dir = root();
        let mut store = TelemetryStore::open(dir.path(), true).unwrap();
        let point = |ns: u64, v: f64, t: &str, s: &str| json!({"timeUnixNano": ns.to_string(), "asDouble": v, "attributes": [a("type", t), a("session.id", s)]});
        let body = json!({"resourceMetrics": [{"scopeMetrics": [{"metrics": [
            {"name": "claude_code.token.usage", "sum": {"aggregationTemporality": 1, "dataPoints": [point(1, 5.0, "input", "s"), point(2, 7.0, "input", "s")]}},
            {"name": "claude_code.cost.usage", "sum": {"aggregationTemporality": 2, "dataPoints": [point(1, 0.5, "x", "s1"), point(2, 0.9, "x", "s1"), point(1, 0.1, "x", "s2")]}},
            {"name": "claude_code.commit.count", "sum": {"aggregationTemporality": 1, "dataPoints": [point(1, 1.0, "c", "s")]}},
            {"name": "claude_code.commit.count", "sum": {"aggregationTemporality": 2, "dataPoints": [point(2, 3.0, "c", "s")]}}
        ]}]}]}).to_string();
        store.record("a", &normalize_metrics(body.as_bytes(), 1).unwrap(), 1, StoreLimits::default()).unwrap();
        let totals = store.read_agent("a").unwrap().totals;
        let get = |name: &str| totals.iter().find(|t| t.name == name).unwrap().clone();
        assert_eq!(get("token.usage").value, Some(12.0), "delta summed");
        assert!((get("cost.usage").value.unwrap() - 1.0).abs() < 1e-9, "cumulative newest per session, summed");
        assert_eq!(get("commit.count").value, None, "mixed temporality not summed");
        assert_eq!(get("commit.count").temporality, "mixed");
    }

    #[test]
    fn alerts_fire_once_resolve_and_migrate_from_v1() {
        let dir = root();
        let path = dir.path().join(".swarm").join(TELEMETRY_DB);
        let version = |p: &Path| -> i64 { Connection::open(p).unwrap().query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap() };
        // A v1 store written before alerts existed: a read leaves it untouched.
        {
            fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path).unwrap();
            let db = Connection::open(&path).unwrap();
            db.execute_batch(&format!("{SCHEMA} PRAGMA user_version = 1;")).unwrap();
        }
        let mut reader = TelemetryStore::open(dir.path(), false).unwrap();
        assert_eq!(reader.read_alerts().unwrap(), AlertView::default());
        assert!(!reader.change_marker().unwrap().contains('/'), "no alert component before v2");
        assert_eq!(reader.sync_alerts(&[], &BTreeSet::new(), 1).unwrap_err().code(), "telemetry-store-version-unsupported");
        reader.close().unwrap();
        assert_eq!(version(&path), 1, "a read never migrates");
        // Busy while migrating: refused as busy, nothing changed, then it migrates.
        let other = Connection::open(&path).unwrap();
        other.execute_batch("BEGIN IMMEDIATE").unwrap();
        assert_eq!(TelemetryStore::open(dir.path(), true).err().unwrap().code(), "telemetry-store-busy");
        other.execute_batch("ROLLBACK").unwrap();
        assert_eq!(version(&path), 1);
        let mut store = TelemetryStore::open(dir.path(), true).unwrap();
        assert_eq!(version(&path), TELEMETRY_SCHEMA_VERSION);
        let c = |rule: &'static str, subject: &str| crate::AlertCondition { rule, level: "critical", subject: subject.into(), work_id: "w".into(), occurred: String::new() };
        let present: BTreeSet<String> = ["a".to_owned(), "w/t".to_owned()].into();
        // Duplicates count once instead of aborting on the unique index.
        let first = store.sync_alerts(&[c("agent-failed", "a"), c("task-blocked", "w/t"), c("agent-failed", "a")], &present, 10).unwrap();
        assert_eq!((first.fired.len(), first.resolved.len()), (2, 0));
        let marker = store.change_marker().unwrap();
        // Unchanged conditions need no write lock: they succeed while another writer holds it.
        other.execute_batch("BEGIN IMMEDIATE").unwrap();
        let again = store.sync_alerts(&[c("agent-failed", "a"), c("task-blocked", "w/t")], &present, 11).unwrap();
        assert_eq!(again, AlertSync::default(), "idempotent");
        assert_eq!(store.sync_alerts(&[c("agent-failed", "a")], &present, 12).unwrap_err().code(), "telemetry-store-busy");
        other.execute_batch("ROLLBACK").unwrap();
        assert_eq!(store.read_alerts().unwrap().open.len(), 2, "busy refusal kept nothing");
        let later = store.sync_alerts(&[c("agent-failed", "a")], &present, 20).unwrap();
        assert_eq!((later.fired.len(), later.resolved.len(), later.resolved[0].resolved_ms), (0, 1, Some(20)));
        assert_eq!(later.resolved[0].resolution, "cleared");
        assert_ne!(store.change_marker().unwrap(), marker, "a transition changes the marker");
        // Recurrence after resolution opens a new row; a vanished subject resolves as gone.
        assert_eq!(store.sync_alerts(&[c("agent-failed", "a"), c("task-blocked", "w/t")], &present, 30).unwrap().fired.len(), 1);
        let gone = store.sync_alerts(&[c("task-blocked", "w/t")], &["w/t".to_owned()].into(), 40).unwrap();
        assert_eq!((gone.resolved.len(), gone.resolved[0].resolution.as_str()), (1, "gone"));
        let view = store.read_alerts().unwrap();
        assert_eq!((view.open.len(), view.history.len(), view.open_more), (1, 2, false));
        assert_eq!((view.history[0].subject.as_str(), view.history[1].subject.as_str()), ("a", "w/t"));
    }

    #[test]
    fn codex_histograms_give_latency_quantiles_and_token_sums() {
        let dir = root();
        let mut store = TelemetryStore::open(dir.path(), true).unwrap();
        let point = |ns: u64, attrs: Value, buckets: Value, count: &str, sum: f64| json!({"timeUnixNano": ns.to_string(), "attributes": attrs,
            "count": count, "sum": sum, "max": 3000.0, "bucketCounts": buckets, "explicitBounds": [100.0, 1000.0]});
        let model = json!([{"key": "model", "value": {"stringValue": "gpt-x"}}]);
        let body = json!({"resourceMetrics": [{"scopeMetrics": [{"metrics": [
            {"name": "codex.api_request.duration_ms", "histogram": {"aggregationTemporality": 1, "dataPoints": [
                point(1, model.clone(), json!(["1", "2", "1"]), "4", 2200.0), point(2, model.clone(), json!(["1", "2", "1"]), "4", 2200.0)]}},
            {"name": "codex.turn.token_usage", "histogram": {"aggregationTemporality": 1, "dataPoints": [{"timeUnixNano": "3", "count": "1", "sum": 10.0,
                "attributes": [{"key": "token_type", "value": {"stringValue": "input"}}]}]}},
            {"name": "codex.sqlite.init.duration_ms", "histogram": {"aggregationTemporality": 1, "dataPoints": [{"count": "1", "sum": 1.0}]}},
            {"name": "codex.turn.e2e_duration_ms", "histogram": {"aggregationTemporality": 1, "dataPoints": [{"count": "2", "sum": 1.0, "bucketCounts": ["1"], "explicitBounds": [5.0]}]}}
        ]}]}]}).to_string();
        let batch = normalize_metrics(body.as_bytes(), 5).unwrap();
        assert_eq!(batch.records.len(), 3, "two latency points and one token point");
        assert_eq!((batch.rejected, batch.unknown.get("sqlite.init.duration_ms")), (1, Some(&1)), "malformed point rejected; unknown histogram counted, not rejected");
        store.record("a", &batch, 5, StoreLimits::default()).unwrap();
        let t = store.read_agent("a").unwrap();
        let lat = &t.latencies[0];
        assert_eq!((lat.name.as_str(), lat.model.as_deref(), lat.count), ("api_request.duration_ms", Some("gpt-x"), 8));
        assert_eq!((lat.mean_ms, lat.p50_ms, lat.max_ms), (Some(550.0), Some(550.0), Some(3000.0)));
        assert!((lat.p95_ms.unwrap() - 2600.0).abs() < 1e-6, "p95 inside the open last bucket, up to max");
        let tokens = t.totals.iter().find(|x| x.name == "turn.token_usage").unwrap();
        assert_eq!((tokens.kind.as_deref(), tokens.value, tokens.temporality.as_str()), (Some("input"), Some(10.0), "histogram"));
        assert_eq!(t.rejected, 1);
        // Output-token sums per agent come from call events only.
        let calls = normalize_logs(&logs(vec![
            event("claude_code.api_request", 1, json!([{"key": "output_tokens", "value": {"stringValue": "30"}}])),
            event("claude_code.api_request", 2, json!([{"key": "output_tokens", "value": {"stringValue": "12"}}])),
            event("claude_code.api_request", 3, json!([])),
        ]), 5).unwrap();
        store.record("b", &calls, 6, StoreLimits::default()).unwrap();
        let sums = store.output_tokens_by_agent().unwrap();
        assert_eq!((sums.get("b"), sums.get("a")), (Some(&42), None), "a has no call event");
    }

    #[test]
    fn codex_spans_keep_the_run_tree_and_drop_internal_noise() {
        let dir = root();
        let mut store = TelemetryStore::open(dir.path(), true).unwrap();
        let span = |trace: &str, id: &str, parent: &str, name: &str, start: u64, end: u64, attrs: Value| {
            json!({"traceId": trace, "spanId": id, "parentSpanId": parent, "name": name,
                   "startTimeUnixNano": start.to_string(), "endTimeUnixNano": end.to_string(), "attributes": attrs})
        };
        let code = json!([{"key": "code.file.path", "value": {"stringValue": "/PRIVATE/src/lib.rs"}},
                          {"key": "tool_name", "value": {"stringValue": "exec_command"}}]);
        let ms = 1_000_000u64;
        let body = json!({"resourceSpans": [{"scopeSpans": [{"spans": [
            span("aa", "01", "", "codex.exec", 1000 * ms, 6000 * ms, json!([])),
            span("aa", "02", "01", "session_task.turn", 1100 * ms, 5800 * ms, json!([])),
            span("aa", "03", "02", "exec_command", 2000 * ms, 2192 * ms, code),
            span("aa", "04", "02", "fs.read_file", 1200 * ms, 1200 * ms, json!([])),
            span("aa", "zz", "02", "run_sampling_request", 3000 * ms, 2000 * ms, json!([])), // ends before start
            span("0d", "09", "", "codex.exec", 10 * ms, 20 * ms, json!([])) // older trace
        ]}]}]}).to_string();
        let batch = crate::telemetry_normalize::normalize_traces(body.as_bytes(), 5).unwrap();
        assert_eq!((batch.records.len(), batch.rejected, batch.unknown.get("span (other)")), (4, 1, Some(&1)));
        store.record("a", &batch, 5, StoreLimits::default()).unwrap();
        let t = store.read_agent("a").unwrap();
        let tree: Vec<_> = t.spans.iter().map(|s| (s.name.as_str(), s.start_ms, s.duration_ms, s.fields.get("parent_id").and_then(Value::as_str))).collect();
        assert_eq!(tree, [("codex.exec", 1000, 5000.0, None), ("session_task.turn", 1100, 4700.0, Some("01")), ("exec_command", 2000, 192.0, Some("02"))], "newest trace only");
        assert_eq!(t.spans[2].fields.get("tool_name"), Some(&json!("exec_command")));
        assert!(!serde_json::to_string(&t.spans).unwrap().contains("PRIVATE"), "code locations dropped");
    }

    #[test]
    fn cap_refuses_and_counts_and_retention_purges_in_the_same_transaction() {
        let dir = root();
        let mut store = TelemetryStore::open(dir.path(), true).unwrap();
        let batch = |n: u64| normalize_logs(&logs((0..n).map(|i| event("claude_code.api_request", i + 1, json!([]))).collect()), 5).unwrap();
        let capped = StoreLimits { retention_days: None, max_records_per_agent: Some(3) };
        let first = store.record("a", &batch(5), 10, capped).unwrap();
        assert_eq!((first.inserted, first.capped, first.duplicates), (3, 2, 0));
        // Replay of stored points stays a duplicate at the cap, never "capped".
        let again = store.record("a", &batch(5), 11, capped).unwrap();
        assert_eq!((again.inserted, again.capped, again.duplicates), (0, 2, 3));
        assert_eq!(store.read_agent("a").unwrap().rejected, 4, "capped points are counted, not hidden");
        assert_eq!(store.record("b", &batch(2), 12, capped).unwrap().inserted, 2, "per attempt");
        // Retention in the ingest transaction: old rows go, the summary accumulates.
        store.issue_token("a", 0).unwrap();
        store.revoke_agent("a", 1).unwrap();
        let kept = StoreLimits { retention_days: Some(1), max_records_per_agent: None };
        let later = store.record("c", &batch(1), 12 + DAY_MS, kept).unwrap();
        assert_eq!(later.purged, PurgeReport { records: 3, alerts: 0, tokens: 1 }, "a (10 ms) goes, b (12 ms) is not older than the cutoff");
        let state = store.retention_state().unwrap().unwrap();
        assert_eq!((state.at_ms, state.cutoff_ms, state.removed.records), (12 + DAY_MS, 12, 3));
        assert_eq!(store.read_agent("c").unwrap().calls.len(), 1, "fresh record kept");
        // Limits need the current schema; a read-only older store refuses them.
        let path = dir.path().join(".swarm").join(TELEMETRY_DB);
        Connection::open(&path).unwrap().execute_batch("DROP TABLE scenarios; DROP TABLE evaluations; DROP TABLE annotations; DROP TABLE purges; DROP INDEX records_received; PRAGMA user_version = 3;").unwrap();
        let mut old = TelemetryStore::open(dir.path(), false).unwrap();
        assert_eq!(old.retention_state().unwrap(), None);
        assert_eq!(old.record("d", &batch(1), 20, kept).unwrap_err().code(), "telemetry-store-version-unsupported");
        old.ensure_current().unwrap();
        assert_eq!(old.record("d", &batch(1), 20, kept).unwrap().inserted, 1);
    }

    #[test]
    fn failed_transaction_keeps_nothing_and_purge_is_observable() {
        let dir = root();
        let mut store = TelemetryStore::open(dir.path(), true).unwrap();
        let batch = normalize_logs(&logs(vec![event("claude_code.api_request", 1, json!([]))]), 5).unwrap();
        // Fault injection: a writer lock held elsewhere makes the write fail at once.
        let other = Connection::open(dir.path().join(".swarm").join(TELEMETRY_DB)).unwrap();
        other.execute_batch("BEGIN EXCLUSIVE").unwrap();
        let error = store.record("a", &batch, 5, StoreLimits::default()).unwrap_err();
        assert_eq!(error.code(), "telemetry-store-busy");
        other.execute_batch("ROLLBACK").unwrap();
        assert_eq!(store.read_agent("a").unwrap().calls.len(), 0);
        assert_eq!(store.record("a", &batch, 5, StoreLimits::default()).unwrap().inserted, 1, "retry succeeds");
        // Corrupt stored fields are refused, never shown as an empty record.
        other.execute("UPDATE records SET fields = 'not json'", []).unwrap();
        assert_eq!(store.read_agent("a").unwrap_err().code(), "telemetry-store-unavailable");
        other.execute("UPDATE records SET fields = '{}'", []).unwrap();
        // Retention: a record received at 5 ms survives a 1-day policy until a day has passed.
        assert_eq!(store.apply_retention(1, 5 + DAY_MS).unwrap(), PurgeReport::default());
        assert_eq!(store.apply_retention(1, 6 + DAY_MS).unwrap().records, 1);
        assert_eq!(store.read_agent("a").unwrap().calls.len(), 0);
    }

    #[test]
    fn telemetry_retained_connection_refuses_same_directory_file_replacement_before_issue() {
        let original=root();
        let replacement=root();
        let store=TelemetryStore::open(original.path(),true).unwrap();
        store.issue_token("control",1).unwrap();
        TelemetryStore::open(replacement.path(),true).unwrap().close().unwrap();
        let target=original.path().join(".swarm/telemetry.db");
        let saved=original.path().join("retained-telemetry.db");
        let foreign=replacement.path().join(".swarm/telemetry.db");
        fs::rename(&target,&saved).unwrap();
        fs::rename(&foreign,&target).unwrap();
        let error=store.issue_token("refused",2).unwrap_err();
        assert_eq!(error.code(),"telemetry-store-instance-changed");
        let connection=Connection::open_with_flags(&target,OpenFlags::SQLITE_OPEN_READ_ONLY|OpenFlags::SQLITE_OPEN_NOFOLLOW).unwrap();
        let count:i64=connection.query_row("SELECT count(*) FROM tokens",[],|row|row.get(0)).unwrap();
        assert_eq!(count,0);
        connection.close().unwrap();
        fs::rename(&target,&foreign).unwrap();
        fs::rename(&saved,&target).unwrap();
        store.issue_token("recovered",3).unwrap();
        let count:i64=store.db.query_row("SELECT count(*) FROM tokens",[],|row|row.get(0)).unwrap();
        assert_eq!(count,2);
        store.close().unwrap();
    }


    #[test]
    fn telemetry_token_ignored_insert_is_refused_and_readback_failure_keeps_write_marker() {
        let dir=root();
        let store=TelemetryStore::open(dir.path(),true).unwrap();
        store.issue_token("control",1).unwrap();
        store.db.execute_batch("CREATE TRIGGER fixture_ignore BEFORE INSERT ON tokens BEGIN SELECT RAISE(IGNORE); END;").unwrap();
        let error=store.issue_token("ignored",2).unwrap_err();
        assert_eq!(error.code(),"telemetry-token-insert-not-recorded");
        let count:i64=store.db.query_row("SELECT count(*) FROM tokens",[],|row|row.get(0)).unwrap();
        assert_eq!(count,1);
        store.db.execute_batch("DROP TRIGGER fixture_ignore; CREATE TRIGGER fixture_delete AFTER INSERT ON tokens BEGIN DELETE FROM tokens WHERE digest=NEW.digest; END;").unwrap();
        let error=store.issue_token("deleted",3).unwrap_err();
        assert!(error.token_issued(),"INSERT reported1 before readback refusal must survive");
        assert_eq!(error.source().unwrap().to_string(),"telemetry-token-persistence-conflict");
        store.db.execute_batch("DROP TRIGGER fixture_delete;").unwrap();
        store.issue_token("recovered",4).unwrap();
        let count:i64=store.db.query_row("SELECT count(*) FROM tokens",[],|row|row.get(0)).unwrap();
        assert_eq!(count,2);
        store.close().unwrap();
    }

}
