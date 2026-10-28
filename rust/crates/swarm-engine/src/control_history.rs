//! Historical control projection only: retained receipts grant no current verdict or authority.
use crate::{AutomaticValidation, Task, WorkError};
use rusqlite::Connection;
use serde::Serialize;
use std::{collections::BTreeSet, error::Error, fmt};

/// CLI/web control facts. Missing measurements remain null/unknown, never invented.
/// Historical entries keep their original order and may contain private command data;
/// Debug intentionally omits that data. This DTO proves no current execution or rights.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceControl {
    pub id: String,
    pub attempt_id: String,
    pub execution: String,
    pub revision: String,
    pub candidate_sha: String,
    pub command: Option<Vec<String>>,
    pub exit_code: Option<i64>,
    pub started_at: String,
    pub finished_at: String,
    pub wall_duration_ms: Option<i64>,
    pub cpu_duration_ms: Option<i64>,
    pub cost: String,
    pub tokens: String,
    pub freshness: String,
    pub result: String,
    pub limits: Vec<String>,
}
impl fmt::Debug for EvidenceControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvidenceControl")
            .field("freshness", &self.freshness)
            .finish_non_exhaustive()
    }
}

/// Checked read/codec/finalization failure, distinct from an empty history.
pub enum ControlHistoryError {
    Sql(rusqlite::Error),
    Format(WorkError),
    Finalize {
        primary: Box<Self>,
        cleanup: rusqlite::Error,
    },
}
impl ControlHistoryError {
    /// Stable safe diagnostic family, excluding receipt and command contents.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Sql(_) => "control-history-storage",
            Self::Format(_) => "control-history-format",
            Self::Finalize { .. } => "control-history-cleanup",
        }
    }
}
impl fmt::Display for ControlHistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for ControlHistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for ControlHistoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Sql(e) => Some(e),
            Self::Format(e) => Some(e),
            Self::Finalize { primary, .. } => Some(primary.as_ref()),
        }
    }
}
impl From<rusqlite::Error> for ControlHistoryError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}

/// Keep attributed nonempty text verbatim; Go whitespace-only values become unknown.
fn known(value: &str) -> String {
    if crate::text::trim_space(value).is_empty() {
        "unknown".into()
    } else {
        value.into()
    }
}

/// Read all retained controls for attempts owned by the supplied task, newest receipt first.
///
/// The connection/Work/task are caller-owned data, not authenticated actors. The
/// Root consumer supplies its actual stored row; this function opens/migrates no
/// root and mutates no state. Current receipt ID and repeated receipt IDs are
/// excluded after ownership/codec checks. Empty command slices become null as
/// Go's append(nil, values...) does. Historical exit codes remain present even
/// when execution is unknown. No top-N, truncation or retention policy is added.
/// An attempt-free task needs no history query and returns an empty vector.
///
/// # Errors
/// SQL, scan, malformed receipt and statement-finalization errors refuse the
/// whole observation, retaining the cause; they never become an empty success.
/// Malformed candidate rows refuse even if their ownership cannot be determined.
/// This deliberately strengthens the legacy Go reader that skips such failures.
pub fn read_validation_control_history(
    db: &Connection,
    work: &str,
    task: &Task,
    current: Option<&AutomaticValidation>,
) -> Result<Vec<EvidenceControl>, ControlHistoryError> {
    let attempts: BTreeSet<_> = task
        .attempts
        .iter()
        .flatten()
        .map(|a| a.id.as_str())
        .collect();
    if attempts.is_empty() {
        return Ok(Vec::new());
    }
    let mut statement = db.prepare("SELECT payload FROM main.events WHERE work_id=? AND kind='task.auto-validation' ORDER BY revision DESC")?;
    let result = (|| {
        let mut rows = statement.query([work])?;
        let mut history = Vec::new();
        let mut seen = BTreeSet::new();
        while let Some(row) = rows.next()? {
            let raw = crate::sqlite_value::string(row.get_ref(0)?, 0)?;
            let receipt: AutomaticValidation =
                crate::work::decode(&raw, crate::models::shape_automatic_validation())
                    .map_err(ControlHistoryError::Format)?;
            if !attempts.contains(receipt.attempt_id.as_str())
                || current.is_some_and(|c| c.receipt == receipt.receipt)
                || !seen.insert(receipt.receipt.clone())
            {
                continue;
            }
            for control in receipt.controls.iter().flatten() {
                let (state, execution) = if control.executed {
                    (if control.passed { "passed" } else { "failed" }, "executed")
                } else if control.started_at.is_empty() {
                    ("unknown", "unknown")
                } else {
                    ("failed", "not_executed")
                };
                history.push(EvidenceControl {
                    id: control.id.clone(),
                    attempt_id: known(&receipt.attempt_id),
                    execution: execution.into(),
                    revision: receipt.revision.to_string(),
                    candidate_sha: known(&receipt.candidate_sha),
                    command: control.command.as_ref().filter(|v| !v.is_empty()).cloned(),
                    exit_code: Some(control.exit_code),
                    started_at: known(&control.started_at),
                    finished_at: known(&control.finished_at),
                    wall_duration_ms: control.wall_duration_ms.as_deref().copied(),
                    cpu_duration_ms: control.cpu_duration_ms.as_deref().copied(),
                    cost: "unknown".into(),
                    tokens: "unknown".into(),
                    freshness: "historical".into(),
                    result: state.into(),
                    limits: vec![
                        "Reçu historique conservé ; il ne constitue pas le verdict actuel.".into(),
                    ],
                });
            }
        }
        Ok(history)
    })();
    match (result, statement.finalize()) {
        (Ok(history), Ok(())) => Ok(history),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(ControlHistoryError::Sql(error)),
        (Err(primary), Err(cleanup)) => Err(ControlHistoryError::Finalize {
            primary: Box::new(primary),
            cleanup,
        }),
    }
}
