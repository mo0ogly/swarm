//! Normalized validator ownership, never command text or an operational BLOB.
//! This internal ledger does not itself authorize execution or prove a reap.
use crate::{agent_launch::query, AgentLaunchError as E};
use rusqlite::{params, Connection};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Protocol phases retain exclusion until an explicit terminal publication.
/// Lease expiry, missing PID or an absent receipt never manufacture completion.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum JobPhase {
    Active,
    Unresolved,
    Recorded,
    Refused,
}
/// Each control has its own immutable child tuple. Completed means a receipt
/// was bound by the caller, not that the program passed or all descendants died.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum StepPhase {
    Prepared,
    Registered,
    Permitted,
    Completed,
    Refused,
    Unresolved,
}
/// Engine-observed source and controller identity. Public request IDs are only
/// replay keys; they cannot supply PID/starttime, root identity or permissions.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Job {
    pub id: String,
    pub work_id: String,
    pub task_id: String,
    pub event_id: String,
    pub request_sha256: String,
    pub source_sha256: String,
    pub work_revision: i64,
    pub settings_revision: i64,
    pub owner_pid: i64,
    pub owner_stamp: String,
    pub owner_host: String,
    pub directory_device: String,
    pub directory_inode: String,
    pub control_count: i64,
    pub phase: JobPhase,
    pub receipt_sha256: Option<String>,
    pub published_revision: Option<i64>,
}
/// One planned control slot. Runtime registration supplies the actual child,
/// then permission is committed before a possible grant write. Neither a DTO nor
/// this row asserts no fork, actual exec, a wait result or financial authority.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Step {
    pub id: String,
    pub job_id: String,
    pub ordinal: i64,
    pub control_id: String,
    pub phase: StepPhase,
    pub permission: i64,
    pub child_pid: Option<i64>,
    pub parent_pid: Option<i64>,
    pub group_id: Option<i64>,
    pub child_stamp: Option<String>,
    pub receipt_sha256: Option<String>,
}
const JOB: &str = "native_validation_jobs";
const STEP: &str = "native_validation_steps";
const JOB_DDL:&str="CREATE TABLE native_validation_jobs(id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id),task_id TEXT NOT NULL,event_id TEXT NOT NULL UNIQUE,request_sha256 TEXT NOT NULL,source_sha256 TEXT NOT NULL,work_revision INTEGER NOT NULL CHECK(work_revision>0),settings_revision INTEGER NOT NULL CHECK(settings_revision>0),owner_pid INTEGER NOT NULL CHECK(owner_pid>0),owner_stamp TEXT NOT NULL,owner_host TEXT NOT NULL,directory_device TEXT NOT NULL,directory_inode TEXT NOT NULL,control_count INTEGER NOT NULL CHECK(control_count>0),phase TEXT NOT NULL CHECK(phase IN ('active','unresolved','recorded','refused')),receipt_sha256 TEXT,published_revision INTEGER)";
const LIVE_INDEX:&str="CREATE UNIQUE INDEX native_validation_live_task ON native_validation_jobs(work_id,task_id) WHERE phase IN ('active','unresolved')";
const STEP_DDL:&str="CREATE TABLE native_validation_steps(id TEXT PRIMARY KEY,job_id TEXT NOT NULL REFERENCES native_validation_jobs(id),ordinal INTEGER NOT NULL CHECK(ordinal>=0),control_id TEXT NOT NULL,phase TEXT NOT NULL CHECK(phase IN ('prepared','registered','permitted','completed','refused','unresolved')),permission INTEGER NOT NULL CHECK(permission IN (0,1)),child_pid INTEGER,parent_pid INTEGER,group_id INTEGER,child_stamp TEXT,receipt_sha256 TEXT,UNIQUE(job_id,ordinal),UNIQUE(job_id,control_id))";
const JOB_COLUMNS: &[&str] = &[
    "id",
    "work_id",
    "task_id",
    "event_id",
    "request_sha256",
    "source_sha256",
    "work_revision",
    "settings_revision",
    "owner_pid",
    "owner_stamp",
    "owner_host",
    "directory_device",
    "directory_inode",
    "control_count",
    "phase",
    "receipt_sha256",
    "published_revision",
];
const STEP_COLUMNS: &[&str] = &[
    "id",
    "job_id",
    "ordinal",
    "control_id",
    "phase",
    "permission",
    "child_pid",
    "parent_pid",
    "group_id",
    "child_stamp",
    "receipt_sha256",
];

/// Require the exact pair of auxiliary tables and indexes, including no extra
/// triggers. Creation is atomic only in the caller's transaction; partial or
/// incompatible existing schemas refuse rather than being silently repaired.
/// False means no ledger exists, never evidence that a process did not start.
pub(super) fn schema(db: &Connection, create: bool) -> Result<bool, E> {
    let mut found = Vec::new();
    for table in [JOB, STEP] {
        let row = query(
            db,
            "SELECT type,sql FROM main.sqlite_schema WHERE name=?",
            [table],
            |rows| {
                Ok(rows
                    .next()?
                    .map(|r| {
                        Ok::<_, rusqlite::Error>((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                    })
                    .transpose()?)
            },
        )?;
        found.push(row);
    }
    if found.iter().all(Option::is_none) {
        if !create {
            return Ok(false);
        }
        active_transaction(db)?;
        db.execute_batch(JOB_DDL)?;
        db.execute_batch(LIVE_INDEX)?;
        db.execute_batch(STEP_DDL)?;
    } else if found[0].as_ref() != Some(&("table".into(), JOB_DDL.into()))
        || found[1].as_ref() != Some(&("table".into(), STEP_DDL.into()))
    {
        return Err(E::Refused("task-validation-ledger-schema-invalid"));
    }
    let actual=query(db,"SELECT type,name,sql FROM main.sqlite_schema WHERE tbl_name IN (?,?) AND name NOT IN (?,?) ORDER BY name",params![JOB,STEP,JOB,STEP],|rows|{
        let mut v=Vec::new();while let Some(r)=rows.next()?{v.push((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?))}Ok(v)
    })?;
    let mut expected = vec![(
        "index".into(),
        "native_validation_live_task".into(),
        Some(LIVE_INDEX.into()),
    )];
    for (table, count) in [(JOB, 2), (STEP, 3)] {
        for i in 1..=count {
            expected.push((
                "index".into(),
                format!("sqlite_autoindex_{table}_{i}"),
                None,
            ))
        }
    }
    expected.sort_by(|a, b| a.1.cmp(&b.1));
    if actual != expected {
        return Err(E::Refused("task-validation-ledger-schema-invalid"));
    }
    Ok(true)
}
/// SQL mutators require caller-owned rollback/commit. No implicit transaction,
/// retry, timeout, statement swallowing or permission inference is introduced.
fn active_transaction(db: &Connection) -> Result<(), E> {
    if db.is_autocommit() {
        Err(E::Refused("task-validation-ledger-transaction-required"))
    } else {
        Ok(())
    }
}
fn name(s: &str) -> bool {
    swarm_identity_contract::name(s.as_bytes())
}
fn hash(s: &str) -> bool {
    crate::provider_configuration::configuration_digest_valid(s)
}
fn decimal(s: &str) -> bool {
    s.parse::<u64>().is_ok_and(|v| v.to_string() == s)
}
/// Validate representation and terminal-receipt invariants, not observed kernel
/// ownership or truth of receipt bytes. The runtime must establish those guards.
fn valid_job(j: &Job) -> Result<(), E> {
    if [&j.id, &j.work_id, &j.task_id, &j.event_id]
        .iter()
        .any(|v| !name(v))
        || !hash(&j.request_sha256)
        || !hash(&j.source_sha256)
        || [
            j.work_revision,
            j.settings_revision,
            j.owner_pid,
            j.control_count,
        ]
        .iter()
        .any(|v| *v < 1)
        || !decimal(&j.owner_stamp)
        || j.owner_host.is_empty()
        || !decimal(&j.directory_device)
        || !decimal(&j.directory_inode)
        || j.receipt_sha256.as_deref().is_some_and(|v| !hash(v))
        || j.published_revision.is_some_and(|v| v <= j.work_revision)
        || (j.phase == JobPhase::Recorded
            && (j.receipt_sha256.is_none() || j.published_revision.is_none()))
        || (!matches!(j.phase, JobPhase::Recorded | JobPhase::Refused)
            && j.published_revision.is_some())
        || (j.phase == JobPhase::Refused && j.receipt_sha256.is_none())
    {
        return Err(E::Refused("task-validation-job-invalid"));
    }
    Ok(())
}
/// Reject partial/replaced child identity, invalid slot, permission without a
/// registered tuple, and terminal observations without a receipt. Unknown phase
/// keeps exclusion; absence of a tuple is never evidence of no fork or no grant.
fn valid_step(s: &Step, j: &Job) -> Result<(), E> {
    let tuple = match (s.child_pid, s.parent_pid, s.group_id, &s.child_stamp) {
        (None, None, None, None) => false,
        (Some(pid), Some(parent), Some(group), Some(stamp))
            if pid > 0 && parent == j.owner_pid && group == pid && decimal(stamp) =>
        {
            true
        }
        _ => return Err(E::Refused("task-validation-child-tuple-invalid")),
    };
    if !name(&s.id)
        || s.job_id != j.id
        || !name(&s.control_id)
        || s.ordinal < 0
        || s.ordinal >= j.control_count
        || ![0, 1].contains(&s.permission)
        || s.receipt_sha256.as_deref().is_some_and(|v| !hash(v))
        || (s.permission == 1 && !tuple)
        || (s.phase == StepPhase::Prepared && (tuple || s.permission != 0))
        || (s.phase == StepPhase::Registered && (!tuple || s.permission != 0))
        || (s.phase == StepPhase::Permitted && (!tuple || s.permission != 1))
        || (s.phase == StepPhase::Completed && (s.permission != 1 || s.receipt_sha256.is_none()))
        || (s.phase == StepPhase::Refused && s.receipt_sha256.is_none())
    {
        return Err(E::Refused("task-validation-step-invalid"));
    }
    Ok(())
}
/// Reconstruct typed scalar columns through transient JSON, never storing JSON
/// as operational state. Malformed types/unknown phases and SQL/finalize errors
/// refuse; missing rows are explicit None and cannot imply completed execution.
fn read<T: DeserializeOwned>(
    db: &Connection,
    table: &str,
    columns: &[&str],
    key: &str,
    value: &str,
) -> Result<Option<T>, E> {
    let fields = columns
        .iter()
        .map(|v| format!("'{v}',{v}"))
        .collect::<Vec<_>>()
        .join(",");
    query(
        db,
        &format!("SELECT json_object({fields}) FROM {table} WHERE {key}=?"),
        [value],
        |rows| {
            let v = rows.next()?.map(|r| r.get::<_, String>(0)).transpose()?;
            if rows.next()?.is_some() {
                return Err(E::Refused("task-validation-ledger-duplicate"));
            }
            v.map(|v| {
                serde_json::from_str(&v).map_err(|e| E::caused("task-validation-ledger-decode", e))
            })
            .transpose()
        },
    )
}
/// Read exact checked job by engine ID. An absent auxiliary schema or row means
/// unknown/missing ownership, never a replacement owner or a finished process.
pub(super) fn job(db: &Connection, id: &str) -> Result<Option<Job>, E> {
    if !schema(db, false)? {
        return Ok(None);
    }
    let j: Option<Job> = read(db, JOB, JOB_COLUMNS, "id", id)?;
    if let Some(j) = &j {
        valid_job(j)?
    }
    Ok(j)
}
/// Resolve a command replay key without executing or accepting anything. The
/// runtime must compare work/task and canonical request hash before replay.
pub(super) fn event(db: &Connection, id: &str) -> Result<Option<Job>, E> {
    if !schema(db, false)? {
        return Ok(None);
    }
    let j: Option<Job> = read(db, JOB, JOB_COLUMNS, "event_id", id)?;
    if let Some(j) = &j {
        valid_job(j)?
    }
    Ok(j)
}
/// Select one published ID by its canonical public receipt without collecting
/// task history. No implicit count ceiling or absent-schema success is added;
/// None means missing evidence. The caller must validate the complete current
/// job, original material and slots in the same snapshot. Query/schema/close
/// failures retain causes; hash collisions/duplicate matches refuse, no writes.
pub(super) fn recorded_id_for_receipt(
    db: &Connection,
    work: &str,
    task: &str,
    receipt: &str,
) -> Result<Option<String>, E> {
    if !schema(db, false)? {
        return Ok(None);
    }
    query(db, "SELECT id FROM native_validation_jobs WHERE work_id=? AND task_id=? AND phase='recorded' ORDER BY id", params![work, task], |rows| {
        let mut selected = None;
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let name = format!(".swarm/native-validation-public-{}.json", crate::provider_configuration::configuration_digest(id.as_bytes()));
            if name == receipt && selected.replace(id).is_some() {
                return Err(E::Refused("review-native-controls-ambiguous"));
            }
        }
        Ok(selected)
    })
}
/// Read the unique live controller for this task, including unresolved owners.
/// A missing ledger means no native claim, not a process completion. No stale
/// PID is signalled or owner replaced; schema/query/finalization errors refuse.
pub(super) fn active(db: &Connection, work: &str, task: &str) -> Result<Option<Job>, E> {
    if !schema(db, false)? {
        return Ok(None);
    }
    let id = query(db,"SELECT id FROM native_validation_jobs WHERE work_id=? AND task_id=? AND phase IN ('active','unresolved')",params![work,task],|rows| {
        let id = rows.next()?.map(|r|r.get::<_,String>(0)).transpose()?;
        if rows.next()?.is_some(){return Err(E::Refused("task-validation-live-owner-duplicate"));} Ok(id)
    })?;
    id.map(|id| job(db, &id)?.ok_or(E::Refused("task-validation-job-missing")))
        .transpose()
}
/// Read a step and its exact current job before validating tuple/permission.
/// Missing parent/child records refuse and never become successful empty state.
pub(super) fn step(db: &Connection, id: &str) -> Result<(Job, Step), E> {
    if !schema(db, false)? {
        return Err(E::Refused("task-validation-ledger-missing"));
    }
    let s: Step = read(db, STEP, STEP_COLUMNS, "id", id)?
        .ok_or(E::Refused("task-validation-step-missing"))?;
    let j = job(db, &s.job_id)?.ok_or(E::Refused("task-validation-job-missing"))?;
    valid_step(&s, &j)?;
    Ok((j, s))
}
/// Read every actual slot in ordinal order for an unchanged registered job.
/// No receipt, permit or absence-of-process is inferred; count/order/tuple errors
/// refuse. Schema/query/finalization causes are preserved, with no hidden cap.
pub(super) fn ordered_steps(db: &Connection, job: &Job) -> Result<Vec<Step>, E> {
    let ids = query(db, "SELECT id FROM native_validation_steps WHERE job_id=? ORDER BY ordinal", params![job.id], |rows| {
        let mut ids = Vec::new();
        while let Some(row) = rows.next()? { ids.push(row.get::<_, String>(0)?); }
        Ok(ids)
    })?;
    let mut values = Vec::new();
    for (ordinal,id) in ids.iter().enumerate() {
        let (actual,slot) = step(db,id)?;
        if actual != *job || usize::try_from(slot.ordinal).ok() != Some(ordinal) {
            return Err(E::Refused("task-validation-reconstruction-slot-changed"));
        }
        values.push(slot);
    }
    if usize::try_from(job.control_count).ok() != Some(values.len()) || values.is_empty() {
        return Err(E::Refused("task-validation-slot-set-incomplete"));
    }
    Ok(values)
}
/// Encode one checked row into separate scalar columns. SQL, serialization and
/// affected-row errors remain explicit; caller owns identity and transaction.
fn insert(db: &Connection, table: &str, columns: &[&str], v: &impl Serialize) -> Result<(), E> {
    let raw =
        serde_json::to_string(v).map_err(|e| E::caused("task-validation-ledger-encode", e))?;
    let values = columns
        .iter()
        .map(|v| format!("json_extract(?1,'$.{v}')"))
        .collect::<Vec<_>>()
        .join(",");
    if db.execute(
        &format!(
            "INSERT INTO {table}({}) VALUES({values})",
            columns.join(",")
        ),
        [raw],
    )? != 1
    {
        return Err(E::Refused("task-validation-ledger-insert-not-recorded"));
    }
    Ok(())
}
/// Claim a fresh controller and every ordered control slot in the caller Work
/// transaction. One live owner per task is an integrity exclusion, not an
/// operator concurrency ceiling. No replacement, launch, grant or refund occurs.
/// A failure requires rollback of all rows/DDL and the coupled Work candidate.
pub(super) fn claim(db: &Connection, j: &Job, steps: &[Step]) -> Result<(), E> {
    active_transaction(db)?;
    valid_job(j)?;
    if j.phase != JobPhase::Active
        || j.receipt_sha256.is_some()
        || j.published_revision.is_some()
        || usize::try_from(j.control_count).ok() != Some(steps.len())
    {
        return Err(E::Refused("task-validation-initial-job-invalid"));
    }
    schema(db, true)?;
    insert(db, JOB, JOB_COLUMNS, j)?;
    for (i, s) in steps.iter().enumerate() {
        valid_step(s, j)?;
        if s.ordinal != i as i64 || s.phase != StepPhase::Prepared || s.receipt_sha256.is_some() {
            return Err(E::Refused("task-validation-initial-step-invalid"));
        }
        insert(db, STEP, STEP_COLUMNS, s)?;
        if step(db, &s.id)?.1 != *s {
            return Err(E::Refused("task-validation-step-readback"));
        }
    }
    if job(db, &j.id)?.as_ref() != Some(j) {
        return Err(E::Refused("task-validation-job-readback"));
    }
    Ok(())
}
/// Update only named lifecycle columns of an already validated row and require
/// one affected row. Immutable data and complete before/after readbacks belong
/// to the typed transition callers; no compare-and-swap outside a TX is allowed.
fn update(
    db: &Connection,
    table: &str,
    id: &str,
    columns: &[&str],
    v: &impl Serialize,
) -> Result<(), E> {
    let raw =
        serde_json::to_string(v).map_err(|e| E::caused("task-validation-ledger-encode", e))?;
    let values = columns
        .iter()
        .map(|v| format!("{v}=json_extract(?1,'$.{v}')"))
        .collect::<Vec<_>>()
        .join(",");
    if db.execute(
        &format!("UPDATE {table} SET {values} WHERE id=?2"),
        params![raw, id],
    )? != 1
    {
        return Err(E::Refused("task-validation-ledger-update-not-recorded"));
    }
    Ok(())
}
/// Preserve controller/source metadata and prior receipt; only active/unknown
/// jobs may reach a checked terminal publication. No retry/takeover is granted.
/// Requires fresh complete old row and caller-owned atomic publication guards.
pub(super) fn change_job(db: &Connection, old: &Job, new: &Job) -> Result<(), E> {
    active_transaction(db)?;
    valid_job(new)?;
    if job(db, &old.id)?.as_ref() != Some(old) {
        return Err(E::Refused("task-validation-job-stale"));
    }
    let mut immutable = new.clone();
    immutable.phase = old.phase;
    immutable.receipt_sha256.clone_from(&old.receipt_sha256);
    immutable.published_revision = old.published_revision;
    if immutable != *old
        || old.receipt_sha256.is_some() && old.receipt_sha256 != new.receipt_sha256
        || !matches!(old.phase, JobPhase::Active | JobPhase::Unresolved)
    {
        return Err(E::Refused("task-validation-job-evidence-regression"));
    }
    if old.phase == JobPhase::Unresolved && new.phase == JobPhase::Active {
        return Err(E::Refused("task-validation-job-phase-invalid"));
    }
    if matches!(new.phase, JobPhase::Recorded | JobPhase::Refused) {
        let ids = query(
            db,
            "SELECT id FROM native_validation_steps WHERE job_id=? ORDER BY ordinal",
            [&new.id],
            |rows| {
                let mut ids = Vec::new();
                while let Some(r) = rows.next()? {
                    ids.push(r.get::<_, String>(0)?)
                }
                Ok(ids)
            },
        )?;
        if usize::try_from(new.control_count).ok() != Some(ids.len()) {
            return Err(E::Refused("task-validation-control-set-incomplete"));
        }
        for (ordinal, id) in ids.iter().enumerate() {
            let (_, s) = step(db, id)?;
            if s.ordinal != ordinal as i64
                || !matches!(s.phase, StepPhase::Completed | StepPhase::Refused)
                || s.receipt_sha256.is_none()
            {
                return Err(E::Refused("task-validation-control-set-incomplete"));
            }
        }
    }
    update(
        db,
        JOB,
        &old.id,
        &["phase", "receipt_sha256", "published_revision"],
        new,
    )?;
    if job(db, &old.id)?.as_ref() != Some(new) {
        return Err(E::Refused("task-validation-job-readback"));
    }
    Ok(())
}
/// Advance a single slot monotonically; an observed tuple, grant permission and
/// receipt can never be replaced or erased. Unresolved ownership can become a
/// terminal receipt only through the runtime's checked reconciliation. Caller
/// owns actual process observations, source/parent checks and commit before IO.
pub(super) fn change_step(db: &Connection, old: &Step, new: &Step) -> Result<(), E> {
    active_transaction(db)?;
    let (j, actual) = step(db, &old.id)?;
    valid_step(new, &j)?;
    if actual != *old || !matches!(j.phase, JobPhase::Active | JobPhase::Unresolved) {
        return Err(E::Refused("task-validation-step-stale"));
    }
    if old.id != new.id
        || old.job_id != new.job_id
        || old.ordinal != new.ordinal
        || old.control_id != new.control_id
        || old.permission > new.permission
        || old.child_pid.is_some()
            && (
                old.child_pid,
                old.parent_pid,
                old.group_id,
                &old.child_stamp,
            ) != (
                new.child_pid,
                new.parent_pid,
                new.group_id,
                &new.child_stamp,
            )
        || old.receipt_sha256.is_some() && old.receipt_sha256 != new.receipt_sha256
    {
        return Err(E::Refused("task-validation-step-evidence-regression"));
    }
    let allowed = matches!(
        (old.phase, new.phase),
        (StepPhase::Prepared, StepPhase::Registered)
            | (StepPhase::Registered, StepPhase::Permitted)
            | (StepPhase::Permitted, StepPhase::Completed)
    ) || matches!(
        old.phase,
        StepPhase::Prepared | StepPhase::Registered | StepPhase::Permitted | StepPhase::Unresolved
    ) && matches!(new.phase, StepPhase::Refused | StepPhase::Unresolved)
        || old.phase == StepPhase::Unresolved && new.phase == StepPhase::Completed;
    if !allowed {
        return Err(E::Refused("task-validation-step-phase-invalid"));
    }
    update(
        db,
        STEP,
        &old.id,
        &[
            "phase",
            "permission",
            "child_pid",
            "parent_pid",
            "group_id",
            "child_stamp",
            "receipt_sha256",
        ],
        new,
    )?;
    if step(db, &old.id)?.1 != *new {
        return Err(E::Refused("task-validation-step-readback"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Isolated scalar-storage double only. PIDs/stamps are explicit test inputs,
    /// not observations of actual processes or evidence of native exec/reaping.
    fn fixture() -> (Connection, Job, Vec<Step>) {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE works(id TEXT PRIMARY KEY); INSERT INTO works VALUES('work')").unwrap();
        let j = Job {
            id: "controller".into(),
            work_id: "work".into(),
            task_id: "task".into(),
            event_id: "event".into(),
            request_sha256: "a".repeat(64),
            source_sha256: "b".repeat(64),
            work_revision: 4,
            settings_revision: 1,
            owner_pid: 1,
            owner_stamp: "2".into(),
            owner_host: "fixture-host".into(),
            directory_device: "3".into(),
            directory_inode: "4".into(),
            control_count: 2,
            phase: JobPhase::Active,
            receipt_sha256: None,
            published_revision: None,
        };
        let steps = (0..2)
            .map(|i| Step {
                id: format!("step-{i}"),
                job_id: j.id.clone(),
                ordinal: i,
                control_id: format!("control-{i}"),
                phase: StepPhase::Prepared,
                permission: 0,
                child_pid: None,
                parent_pid: None,
                group_id: None,
                child_stamp: None,
                receipt_sha256: None,
            })
            .collect();
        (db, j, steps)
    }
    /// Advance an actual stored fixture slot through each checked protocol edge;
    /// no real process, gate byte, business acceptance or OS identity is implied.
    fn permit(db: &Connection, old: &Step, pid: i64) -> Step {
        let mut registered = old.clone();
        registered.phase = StepPhase::Registered;
        registered.child_pid = Some(pid);
        registered.parent_pid = Some(1);
        registered.group_id = Some(pid);
        registered.child_stamp = Some(pid.to_string());
        change_step(db, old, &registered).unwrap();
        let mut permitted = registered.clone();
        permitted.phase = StepPhase::Permitted;
        permitted.permission = 1;
        change_step(db, &registered, &permitted).unwrap();
        permitted
    }
    /// Every mutation first has a valid witness, exact readback and unchanged-row
    /// assertions after invalid tuple/source/permission/phase transitions. Each
    /// control remains distinct; terminal publication needs every bound receipt.
    #[test]
    fn normalized_slots_preserve_child_identity_permission_and_all_receipts() {
        let (mut db, j, steps) = fixture();
        let tx = db.transaction().unwrap();
        claim(&tx, &j, &steps).unwrap();
        assert!(job(&tx, &j.id).unwrap().as_ref() == Some(&j));
        assert!(event(&tx, &j.event_id).unwrap().as_ref() == Some(&j));
        let types:(String,String,String)=tx.query_row("SELECT typeof(owner_pid),typeof(source_sha256),typeof(receipt_sha256) FROM native_validation_jobs",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert_eq!(types, ("integer".into(), "text".into(), "null".into()));
        let first = permit(&tx, &steps[0], 41);
        let second = permit(&tx, &steps[1], 42);
        assert!(step(&tx, &first.id).unwrap().1 == first);
        let mut changed = first.clone();
        changed.child_pid = Some(43);
        changed.group_id = Some(43);
        changed.child_stamp = Some("43".into());
        assert!(change_step(&tx, &first, &changed).is_err());
        changed = first.clone();
        changed.permission = 0;
        changed.phase = StepPhase::Registered;
        assert!(change_step(&tx, &first, &changed).is_err());
        changed = first.clone();
        changed.parent_pid = Some(99);
        assert!(change_step(&tx, &first, &changed).is_err());
        changed = first.clone();
        changed.control_id = "substituted".into();
        assert!(change_step(&tx, &first, &changed).is_err());
        changed = first.clone();
        changed.phase = StepPhase::Completed;
        assert!(change_step(&tx, &first, &changed).is_err());
        assert!(step(&tx, &first.id).unwrap().1 == first);
        let mut recorded = j.clone();
        recorded.phase = JobPhase::Recorded;
        recorded.receipt_sha256 = Some("c".repeat(64));
        recorded.published_revision = Some(6);
        assert!(change_job(&tx, &j, &recorded).is_err());
        for s in [first, second] {
            let mut done = s.clone();
            done.phase = StepPhase::Completed;
            done.receipt_sha256 = Some("d".repeat(64));
            change_step(&tx, &s, &done).unwrap();
            assert!(step(&tx, &s.id).unwrap().1 == done);
        }
        change_job(&tx, &j, &recorded).unwrap();
        assert!(job(&tx, &j.id).unwrap().as_ref() == Some(&recorded));
        assert!(change_job(&tx, &recorded, &j).is_err());
        tx.commit().unwrap();
        assert!(job(&db, &j.id).unwrap().as_ref() == Some(&recorded));
        db.close().unwrap();
    }
    /// Duplicate live ownership refuses, unresolved stays exclusive and cannot
    /// return to active. Failed SQL retains the exact prior row and the same
    /// transition succeeds only after its query-only precondition is restored.
    #[test]
    fn unknown_owner_stays_exclusive_and_sql_refusal_recovers_without_reset() {
        let (mut db, j, steps) = fixture();
        let tx = db.transaction().unwrap();
        claim(&tx, &j, &steps).unwrap();
        let mut unknown = j.clone();
        unknown.phase = JobPhase::Unresolved;
        change_job(&tx, &j, &unknown).unwrap();
        assert!(change_job(&tx, &unknown, &j).is_err());
        let mut other = j.clone();
        other.id = "other-controller".into();
        other.event_id = "other-event".into();
        assert!(claim(&tx, &other, &steps).is_err());
        assert!(job(&tx, &other.id).unwrap().is_none());
        assert!(job(&tx, &j.id).unwrap().as_ref() == Some(&unknown));
        let mut refused = unknown.clone();
        refused.phase = JobPhase::Refused;
        assert!(change_job(&tx, &unknown, &refused).is_err());
        refused.receipt_sha256 = Some("e".repeat(64));
        for slot in &steps {
            let mut stopped = slot.clone();
            stopped.phase = StepPhase::Refused;
            stopped.receipt_sha256 = Some("f".repeat(64));
            change_step(&tx, slot, &stopped).unwrap();
        }

        tx.execute_batch("PRAGMA query_only=ON").unwrap();
        assert!(change_job(&tx, &unknown, &refused).is_err());
        assert!(job(&tx, &j.id).unwrap().as_ref() == Some(&unknown));
        tx.execute_batch("PRAGMA query_only=OFF").unwrap();
        change_job(&tx, &unknown, &refused).unwrap();
        tx.commit().unwrap();
        assert!(job(&db, &j.id).unwrap().as_ref() == Some(&refused));
        db.close().unwrap();
    }
    /// Valid schema/claim witnesses precede isolated schema and partial-insert
    /// faults. Explicit rollback removes all new DDL/rows; incompatible objects
    /// and extra triggers are rejected, never overwritten or silently adopted.
    #[test]
    fn partial_claim_rollback_and_schema_tampering_never_become_missing_success() {
        let (mut witness, j, steps) = fixture();
        let tx = witness.transaction().unwrap();
        claim(&tx, &j, &steps).unwrap();
        tx.commit().unwrap();
        assert!(schema(&witness, false).unwrap());
        witness.close().unwrap();
        let (mut db, j, mut steps) = fixture();
        assert!(!schema(&db, false).unwrap());
        assert!(claim(&db, &j, &steps).is_err());
        {
            let tx = db.transaction().unwrap();
            let duplicate = steps[0].control_id.clone();
            steps[1].control_id = duplicate;
            assert!(claim(&tx, &j, &steps).is_err());
            tx.rollback().unwrap();
        }
        assert!(!schema(&db, false).unwrap());
        db.execute_batch(JOB_DDL).unwrap();
        assert!(schema(&db, false).is_err());
        db.close().unwrap();
        let (mut db, j, steps) = fixture();
        let tx = db.transaction().unwrap();
        claim(&tx, &j, &steps).unwrap();
        tx.commit().unwrap();
        db.execute_batch("CREATE TRIGGER fixture_fault AFTER UPDATE ON native_validation_steps BEGIN SELECT 1; END").unwrap();
        assert!(schema(&db, false).is_err());
        assert!(job(&db, &j.id).is_err());
        db.execute_batch("DROP TRIGGER fixture_fault").unwrap();
        assert!(schema(&db, false).unwrap());
        assert!(step(&db, &steps[0].id).unwrap().1 == steps[0]);
        db.close().unwrap();
    }
    /// Explicit fixture rows, not runtime PID evidence. Retirement may publish
    /// its audit revision without promoting observations to a recorded PASS.
    /// Invalid revision refuses without changing rows; terminal receipt removes
    /// exclusivity and preserves immutable original sources on readback.
    #[test]
    fn retired_job_retains_audit_revision_without_recorded_success() {
        let (mut db, j, steps) = fixture();
        let tx = db.transaction().unwrap();
        claim(&tx, &j, &steps).unwrap();
        for s in &steps {
            let mut done = s.clone();
            done.phase = StepPhase::Refused;
            done.receipt_sha256 = Some("c".repeat(64));
            change_step(&tx, s, &done).unwrap();
        }
        let mut retired = j.clone();
        retired.phase = JobPhase::Refused;
        retired.receipt_sha256 = Some("d".repeat(64));
        retired.published_revision = Some(j.work_revision + 1);
        let mut invalid = retired.clone();
        invalid.published_revision = Some(j.work_revision);
        assert!(change_job(&tx, &j, &invalid).is_err());
        assert!(job(&tx, &j.id).unwrap().as_ref() == Some(&j));
        change_job(&tx, &j, &retired).unwrap();
        assert!(active(&tx, &j.work_id, &j.task_id).unwrap().is_none());
        assert!(job(&tx, &j.id).unwrap().as_ref() == Some(&retired));
        tx.commit().unwrap();
        assert!(job(&db, &j.id).unwrap().as_ref() == Some(&retired));
    }
}
