//! Guards preceding Go's mutation closure, under the caller's transaction.
//! Inspection is data, not a permit: retain this transaction through mutation,
//! planning validation, hooks and explicit commit. No clock, ID or retry policy.
use crate::{
    encode_model, read_recorded_work, reserve_recorded_command, Error as RequestError,
    RecordedCommandError, RecordedWorkError, Request, Task, Work, WorkError,
};
use rusqlite::{Connection, Statement, Transaction};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
use swarm_wire_json::projection;

pub enum MutationInspection {
    Replayed(Box<Work>),
    /// Initial creation still needs a fresh ID, clock, request application and validation.
    Create,
    Existing(Box<Work>),
}
impl fmt::Debug for MutationInspection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Replayed(_) => "Replayed (data redacted)",
            Self::Create => "Create (mutation pending)",
            Self::Existing(_) => "Existing (mutation pending)",
        })
    }
}
/// Safe contract refusal detail. No submitted reason, digest or task data is echoed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractRevisionRefusal {
    ExplicitUpdateRequired,
    ConfirmationRequired,
    ReasonInvalid,
    ManagedRepository,
    TaskMissing,
    TaskBlockedRequired,
    SeparateStatusRequired,
    TaskScopeRequired,
    StaleContract,
    CriteriaCount,
    FieldsUnchanged,
    ScopeMissing,
    ScopeClosed,
    ReviewRunning,
}
impl ContractRevisionRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ExplicitUpdateRequired => "contract-explicit-update-required",
            Self::ConfirmationRequired => "contract-confirmation-required",
            Self::ReasonInvalid => "contract-reason-invalid",
            Self::ManagedRepository => "contract-managed-repository-unsupported",
            Self::TaskMissing => "contract-task-missing",
            Self::TaskBlockedRequired => "contract-blocked-task-required",
            Self::SeparateStatusRequired => "contract-separate-status-change-required",
            Self::TaskScopeRequired => "contract-task-scope-required",
            Self::StaleContract => "contract-digest-stale",
            Self::CriteriaCount => "contract-criterion-count-must-match",
            Self::FieldsUnchanged => "contract-other-fields-must-stay-unchanged",
            Self::ScopeMissing => "contract-scope-missing",
            Self::ScopeClosed => "contract-scope-closed",
            Self::ReviewRunning => "contract-finish-independent-reviews",
        }
    }
}
pub enum MutationGuardError {
    Command(RecordedCommandError),
    Work(RecordedWorkError),
    Request(RequestError),
    Encode(WorkError),
    Sql(rusqlite::Error),
    Revision {
        expected: i64,
        observed: Option<i64>,
    },
    Hierarchical,
    WorkActive,
    PauseRequired,
    ContractRevision(ContractRevisionRefusal),
    LaunchLocked,
    ActiveAgent,
    Finalize {
        primary: Box<Self>,
        cleanup: rusqlite::Error,
    },
}
impl MutationGuardError {
    /// Public Go-compatible business classification; diagnostics remain in source().
    pub fn code(&self) -> &'static str {
        match self {
            Self::Command(e) => e.code(),
            Self::Work(e) => e.code(),
            Self::Request(_) | Self::Encode(_) => "format",
            Self::Sql(_) => "mutation-guard-storage",
            Self::Finalize { .. } => "mutation-guard-finalize",
            Self::Revision {
                observed: Some(_), ..
            } => "revision_conflict",
            Self::ActiveAgent => "active_agent",
            _ => "format",
        }
    }
    /// Stable, actionable detail for future adapters; grants no retry or authority.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::ContractRevision(reason) => reason.reason(),
            Self::Hierarchical => "hierarchical-plan-immutable",
            Self::WorkActive => "work-active-agents",
            Self::PauseRequired => "contract-public-pause-required",
            Self::LaunchLocked => "preparation-launch-locked",
            Self::ActiveAgent => "task-active-agents",
            Self::Revision { observed: None, .. } => "creation-expects-zero-revision",
            _ => self.code(),
        }
    }
}
impl fmt::Display for MutationGuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())?;
        if self.reason() != self.code() {
            write!(f, ": {}", self.reason())?;
        }
        Ok(())
    }
}
impl fmt::Debug for MutationGuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for MutationGuardError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Command(e) => Some(e),
            Self::Work(e) => Some(e),
            Self::Request(e) => Some(e),
            Self::Encode(e) => Some(e),
            Self::Sql(e) => Some(e),
            Self::Finalize { primary, .. } => Some(primary),
            _ => None,
        }
    }
}
impl From<rusqlite::Error> for MutationGuardError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
impl From<RecordedCommandError> for MutationGuardError {
    fn from(e: RecordedCommandError) -> Self {
        Self::Command(e)
    }
}
impl From<RecordedWorkError> for MutationGuardError {
    fn from(e: RecordedWorkError) -> Self {
        Self::Work(e)
    }
}
fn statement<T>(
    db: &Connection,
    sql: &str,
    read: impl FnOnce(&mut Statement<'_>) -> Result<T, MutationGuardError>,
) -> Result<T, MutationGuardError> {
    let mut stmt = db.prepare(sql)?;
    let result = read(&mut stmt);
    match (result, stmt.finalize()) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e.into()),
        (Err(primary), Err(cleanup)) => Err(MutationGuardError::Finalize {
            primary: Box::new(primary),
            cleanup,
        }),
    }
}
/// Exact Go reviewContract hash, including nil/empty lists and ordered memory references.
/// A matching digest alone authenticates no reviewer or current authority.
pub fn task_review_contract(task: &Task) -> Result<String, WorkError> {
    fn digest(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }
    let bytes = encode_model(&(
        &task.title,
        &task.deliverable,
        &task.criteria,
        &task.depends,
        &task.plan_brief_hash,
    ))?;
    let first = digest(&bytes);
    if task.memory_selections.as_ref().is_none_or(Vec::is_empty) {
        return Ok(first);
    }
    Ok(digest(&encode_model(&(first, &task.memory_selections))?))
}
fn decode_stored_request(bytes: &[u8]) -> Result<Request, MutationGuardError> {
    // This is Go json.Unmarshal inside the mutator, not the strict public DTO decoder.
    let value = projection::project(bytes, crate::request::FIELDS_REQUEST, false).map_err(|e| {
        MutationGuardError::Request(match e {
            projection::Error::Json(e) => RequestError::Frame(e),
            e => RequestError::Projection(e),
        })
    })?;
    serde_json::from_value(value).map_err(|e| MutationGuardError::Request(RequestError::Frame(e)))
}
pub(crate) fn edits_definition(r: &Request) -> bool {
    !r.title.is_empty()
        || !r.deliverable.is_empty()
        || r.criteria.is_some()
        || r.depends.is_some()
        || r.max_attempts != 0
        || r.max_tool_calls != 0
        || r.validation_policy.is_some()
}
fn paused(db: &Connection, work: &str) -> Result<bool, MutationGuardError> {
    statement(
        db,
        "SELECT paused FROM main.cockpit_controls WHERE work_id=?",
        |stmt| {
            let mut rows = stmt.query([work])?;
            match rows.next()? {
                None => Ok(false),
                Some(row) => Ok(crate::sqlite_value::boolean(row.get_ref(0)?, 0)?),
            }
        },
    )
}
fn contract_revision(w: &Work, r: &Request) -> Result<(), MutationGuardError> {
    let reject = MutationGuardError::ContractRevision;
    use ContractRevisionRefusal as Refusal;
    let reason = crate::text::trim_space(&r.contract_revision_reason);
    if !r.confirm_contract_revision {
        return Err(reject(Refusal::ConfirmationRequired));
    }
    if reason.len() < 16 || r.contract_revision_reason.len() > 2000 {
        return Err(reject(Refusal::ReasonInvalid));
    }
    let p = w
        .planning
        .as_ref()
        .ok_or_else(|| reject(Refusal::ScopeMissing))?;
    if p.repository.is_some() {
        return Err(reject(Refusal::ManagedRepository));
    }
    let tasks = w.tasks.as_deref().unwrap_or_default();
    let t = tasks
        .iter()
        .find(|t| t.id == r.id)
        .ok_or_else(|| reject(Refusal::TaskMissing))?;
    if t.status != "blocked" {
        return Err(reject(Refusal::TaskBlockedRequired));
    }
    if !r.status.is_empty() {
        return Err(reject(Refusal::SeparateStatusRequired));
    }
    if t.scope_id.is_empty() {
        return Err(reject(Refusal::TaskScopeRequired));
    }
    if r.expected_contract != task_review_contract(t).map_err(MutationGuardError::Encode)? {
        return Err(reject(Refusal::StaleContract));
    }
    if r.criteria
        .as_ref()
        .is_none_or(|v| v.len() != t.criteria.as_ref().map_or(0, Vec::len))
    {
        return Err(reject(Refusal::CriteriaCount));
    }
    if !r.title.is_empty()
        || !r.deliverable.is_empty()
        || r.depends.is_some()
        || r.max_attempts != 0
        || r.max_tool_calls != 0
        || r.validation_policy.is_some()
    {
        return Err(reject(Refusal::FieldsUnchanged));
    }
    let scope = p
        .scopes
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|s| s.id == t.scope_id)
        .ok_or_else(|| reject(Refusal::ScopeMissing))?;
    if scope.state == "closed" {
        return Err(reject(Refusal::ScopeClosed));
    }
    if tasks.iter().any(|t| {
        t.independent_review
            .as_ref()
            .is_some_and(|r| r.state == "running")
    }) {
        return Err(reject(Refusal::ReviewRunning));
    }
    Ok(())
}
/// Reserve, replay, read revision, then inspect the guards preceding fn(&mut Work).
/// Unknown kinds retain Go's behavior here; the future dispatcher must validate them.
/// Errors do not end the caller's transaction. Reservation triggers are its responsibility.
pub fn inspect_recorded_mutation(
    tx: &Transaction<'_>,
    work: &str,
    kind: &str,
    event: &str,
    expected: i64,
    request: &[u8],
) -> Result<MutationInspection, MutationGuardError> {
    if let Some(w) = reserve_recorded_command(tx, work, kind, event, request)? {
        return Ok(MutationInspection::Replayed(Box::new(w)));
    }
    if kind == "work.create" {
        if expected != 0 {
            return Err(MutationGuardError::Revision {
                expected,
                observed: None,
            });
        }
        return Ok(MutationInspection::Create);
    }
    let w = read_recorded_work(tx, work)?;
    if w.revision != expected {
        return Err(MutationGuardError::Revision {
            expected,
            observed: Some(w.revision),
        });
    }
    if w.planning.is_some() && matches!(kind, "work.update" | "task.add") {
        return Err(MutationGuardError::Hierarchical);
    }
    if kind == "work.update" {
        let count=statement(tx,"SELECT count(*) FROM main.agents WHERE work_id=? AND status IN ('queued','starting','running','stopping')",|s|Ok(s.query_row([work],|r|r.get::<_,i64>(0))?))?;
        if count > 0 {
            return Err(MutationGuardError::WorkActive);
        }
    }
    if matches!(kind, "task.update" | "task.submit") {
        let r = decode_stored_request(request)?;
        let definition = edits_definition(&r);
        if w.planning.is_some() && definition {
            if kind != "task.update" {
                return Err(MutationGuardError::ContractRevision(
                    ContractRevisionRefusal::ExplicitUpdateRequired,
                ));
            }
            if !paused(tx, work)? {
                return Err(MutationGuardError::PauseRequired);
            }
            contract_revision(&w, &r)?;
        }
        if r.status == "running" {
            let count=statement(tx,"SELECT count(*) FROM main.preparation_launch_locks WHERE work_id=? AND task_id=? AND released=0",|s|Ok(s.query_row((work,&r.id),|r|r.get::<_,i64>(0))?))?;
            if count > 0 {
                return Err(MutationGuardError::LaunchLocked);
            }
        }
        if !r.status.is_empty() || definition {
            let count=statement(tx,"SELECT count(*) FROM main.agents WHERE work_id=? AND (task_id=? OR ?) AND status IN ('queued','starting','running','stopping')",|s|Ok(s.query_row((work,&r.id,definition),|r|r.get::<_,i64>(0))?))?;
            if count > 0 {
                return Err(MutationGuardError::ActiveAgent);
            }
        }
    }
    Ok(MutationInspection::Existing(Box::new(w)))
}
