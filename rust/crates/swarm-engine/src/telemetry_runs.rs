//! LIVE-UX telemetry subjects that are not worker agents: native review jobs
//! (`review:<job id>`) and planning runs (`planning:<claim id>`). Read-only
//! projection of the engine tables for the live console; it never writes,
//! never creates the tables and carries no prompt, receipt or process data.
use rusqlite::Connection;
use serde::Serialize;

/// Job phases during which a review job or planning run may still export
/// telemetry (its process can be alive). Every other phase is terminal.
pub(crate) const ACTIVE_RUN_PHASES: [&str; 4] = ["claimed", "registered", "permitted", "unresolved"];

/// One review job or planning run of a Work, as shown by the live console.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TelemetryRun {
    /// Telemetry ingest subject: `review:<id>` or `planning:<claim id>`.
    pub subject: String,
    /// `review` or `planning`.
    pub kind: &'static str,
    /// Job id (review) or claim id (planning).
    pub id: String,
    /// Reviewed task id (review) or planned scope id (planning).
    pub target: String,
    /// Engine phase as recorded (e.g. `permitted`, `recorded`, `applied`).
    pub phase: String,
    /// Whether `phase` is an active job phase (claimed, registered,
    /// permitted, unresolved): the run may still export telemetry.
    pub active: bool,
    /// Planning preparation time as recorded; `None` for review jobs, whose
    /// table keeps no timestamp.
    pub prepared_at: Option<String>,
}

/// Read the review jobs then planning runs of `work` (every Work when
/// `None`, for the live change marker), newest first within each kind
/// (review: insertion order; planning: `prepared_at`). A table
/// absent from an older Root yields no row of that kind. `only` restricts
/// the read to one subject (used by the run view). Other SQLite failures
/// are returned unchanged. Read only.
pub(crate) fn read_telemetry_runs(
    db: &Connection,
    work: Option<&str>,
    only: Option<&str>,
) -> rusqlite::Result<Vec<TelemetryRun>> {
    let mut runs = Vec::new();
    for (kind, sql) in [
        ("review", "SELECT id, task_id, phase, NULL FROM native_review_jobs WHERE ?1 IS NULL OR work_id = ?1 ORDER BY rowid DESC"),
        ("planning", "SELECT claim_id, scope_id, phase, prepared_at FROM native_planning_runs WHERE ?1 IS NULL OR work_id = ?1 ORDER BY prepared_at DESC, claim_id"),
    ] {
        let mut stmt = match db.prepare(sql) {
            Ok(stmt) => stmt,
            Err(rusqlite::Error::SqliteFailure(_, Some(message))) if message.starts_with("no such table") => continue,
            Err(error) => return Err(error),
        };
        let rows = stmt.query_map([work], |r| {
            let (id, phase): (String, String) = (r.get(0)?, r.get(2)?);
            Ok(TelemetryRun {
                subject: format!("{kind}:{id}"),
                kind,
                active: ACTIVE_RUN_PHASES.contains(&phase.as_str()),
                id,
                target: r.get(1)?,
                phase,
                prepared_at: r.get(3)?,
            })
        })?;
        for row in rows {
            let row = row?;
            if only.is_none_or(|s| s == row.subject) {
                runs.push(row);
            }
        }
    }
    Ok(runs)
}
