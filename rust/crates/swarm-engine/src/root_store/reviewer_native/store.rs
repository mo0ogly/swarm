//! Native reviewer ownership ledger. Integration is in progress; this module is
//! not wired into production Root. Candidate tests compile this exact source.
//! Scalar SQL columns, never an operational JSON BLOB.
use crate::{agent_launch::query, AgentLaunchError as E};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Internal lifecycle vocabulary, not a timeout/retry/concurrency policy. Every
/// nonterminal phase retains exclusion; an absent PID never manufactures a reap.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Claimed,
    Registered,
    Permitted,
    Unresolved,
    Recorded,
    Refused,
}

/// Engine-observed identities and immutable source bindings. Owner/child fields
/// are never accepted from a public request. This private value has no Debug
/// implementation and grants neither exec, authenticity nor task acceptance.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Job {
    pub id: String,
    pub work_id: String,
    pub task_id: String,
    pub event_id: String,
    pub request_sha256: String,
    pub producer_id: String,
    pub attempt_id: String,
    pub source_sha256: String,
    pub validation_receipt_sha256: String,
    pub contract_sha256: String,
    pub provider_sha256: String,
    pub executable_sha256: String,
    pub host_scope: String,
    pub workflow_sha256: String,
    pub prompt_sha256: String,
    pub work_revision: i64,
    pub settings_revision: i64,
    pub owner_pid: i64,
    pub owner_stamp: String,
    pub owner_host: String,
    pub directory_device: String,
    pub directory_inode: String,
    pub preflight_until: String,
    pub child_pid: Option<i64>,
    pub parent_pid: Option<i64>,
    pub group_id: Option<i64>,
    pub child_stamp: Option<String>,
    pub permission: i64,
    pub phase: Phase,
    pub receipt_sha256: Option<String>,
    pub published_revision: Option<i64>,
}
const TABLE: &str = "native_review_jobs";
const DDL: &str = "CREATE TABLE native_review_jobs(id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id),task_id TEXT NOT NULL,event_id TEXT NOT NULL UNIQUE,request_sha256 TEXT NOT NULL,producer_id TEXT NOT NULL,attempt_id TEXT NOT NULL,source_sha256 TEXT NOT NULL,validation_receipt_sha256 TEXT NOT NULL,contract_sha256 TEXT NOT NULL,provider_sha256 TEXT NOT NULL,executable_sha256 TEXT NOT NULL,host_scope TEXT NOT NULL,workflow_sha256 TEXT NOT NULL,prompt_sha256 TEXT NOT NULL,work_revision INTEGER NOT NULL CHECK(work_revision>0),settings_revision INTEGER NOT NULL CHECK(settings_revision>0),owner_pid INTEGER NOT NULL CHECK(owner_pid>0),owner_stamp TEXT NOT NULL,owner_host TEXT NOT NULL,directory_device TEXT NOT NULL,directory_inode TEXT NOT NULL,preflight_until TEXT NOT NULL,child_pid INTEGER,parent_pid INTEGER,group_id INTEGER,child_stamp TEXT,permission INTEGER NOT NULL CHECK(permission IN (0,1)),phase TEXT NOT NULL CHECK(phase IN ('claimed','registered','permitted','unresolved','recorded','refused')),receipt_sha256 TEXT,published_revision INTEGER)";
const LIVE: &str = "CREATE UNIQUE INDEX native_review_live_task ON native_review_jobs(work_id,task_id) WHERE phase IN ('claimed','registered','permitted','unresolved')";
const COLUMNS: &[&str] = &[
    "id",
    "work_id",
    "task_id",
    "event_id",
    "request_sha256",
    "producer_id",
    "attempt_id",
    "source_sha256",
    "validation_receipt_sha256",
    "contract_sha256",
    "provider_sha256",
    "executable_sha256",
    "host_scope",
    "workflow_sha256",
    "prompt_sha256",
    "work_revision",
    "settings_revision",
    "owner_pid",
    "owner_stamp",
    "owner_host",
    "directory_device",
    "directory_inode",
    "preflight_until",
    "child_pid",
    "parent_pid",
    "group_id",
    "child_stamp",
    "permission",
    "phase",
    "receipt_sha256",
    "published_revision",
];
/// Mutation requires the caller's Work/budget transaction; no standalone claim.
fn transaction(db: &Connection) -> Result<(), E> {
    if db.is_autocommit() {
        return Err(E::Refused("review-ledger-transaction-required"));
    }
    Ok(())
}
/// Recognize exact normalized schema/indices. Historical absence is explicit;
/// creation occurs only inside caller TX. Extra triggers or alternate definitions
/// for this table refuse; unrelated tables are outside this ledger contract.
/// SQLite/statement finalization causes survive; no process or Work is mutated.
pub(super) fn schema(db: &Connection, create: bool) -> Result<bool, E> {
    let found: Option<(String, String)> = query(
        db,
        "SELECT type,sql FROM main.sqlite_schema WHERE name=?",
        [TABLE],
        |rows| {
            let row = rows
                .next()?
                .map(|r| Ok::<_, rusqlite::Error>((r.get(0)?, r.get(1)?)))
                .transpose()?;
            if rows.next()?.is_some() {
                return Err(E::Refused("review-ledger-schema-duplicate"));
            }
            Ok(row)
        },
    )?;
    if found.is_none() {
        if !create {
            return Ok(false);
        }
        transaction(db)?;
        db.execute_batch(DDL)?;
        db.execute_batch(LIVE)?;
    } else if found != Some(("table".into(), DDL.into())) {
        return Err(E::Refused("review-ledger-schema-invalid"));
    }
    let mut actual: Vec<(String, String, Option<String>)> = query(
        db,
        "SELECT type,name,sql FROM main.sqlite_schema WHERE tbl_name=? AND name<>? ORDER BY name",
        params![TABLE, TABLE],
        |rows| {
            let mut entries = Vec::new();
            while let Some(row) = rows.next()? {
                entries.push((row.get(0)?, row.get(1)?, row.get(2)?));
            }
            Ok(entries)
        },
    )?;
    let mut expected = vec![(
        "index".into(),
        "native_review_live_task".into(),
        Some(LIVE.into()),
    )];
    for index in [1, 2] {
        expected.push((
            "index".into(),
            format!("sqlite_autoindex_{TABLE}_{index}"),
            None,
        ));
    }
    actual.sort();
    expected.sort();
    if actual != expected {
        return Err(E::Refused("review-ledger-index-invalid"));
    }
    let columns: Vec<String> = query(
        db,
        "SELECT name FROM pragma_table_info(?) ORDER BY cid",
        [TABLE],
        |rows| {
            let mut values = Vec::new();
            while let Some(row) = rows.next()? {
                values.push(row.get(0)?);
            }
            Ok(values)
        },
    )?;
    if columns != COLUMNS {
        return Err(E::Refused("review-ledger-columns-invalid"));
    }
    Ok(true)
}
/// SHA spelling is protocol data, not authentication or an input-size policy.
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
/// Validate complete scalar shape and lifecycle coherence before every use.
/// A registered tuple records identity only; committed permission still does not
/// prove a grant write, successful exec, complete wait or independent verdict.
fn valid(job: &Job) -> Result<(), E> {
    if [
        &job.id,
        &job.work_id,
        &job.task_id,
        &job.event_id,
        &job.producer_id,
        &job.attempt_id,
    ]
    .iter()
    .any(|v| !swarm_identity_contract::name(v.as_bytes()))
        || [
            &job.request_sha256,
            &job.source_sha256,
            &job.validation_receipt_sha256,
            &job.contract_sha256,
            &job.provider_sha256,
            &job.executable_sha256,
            &job.host_scope,
            &job.workflow_sha256,
            &job.prompt_sha256,
        ]
        .iter()
        .any(|v| !digest(v))
        || job.work_revision < 1
        || job.settings_revision < 1
        || job.owner_pid < 1
        || u32::try_from(job.owner_pid).is_err()
        || [
            &job.owner_stamp,
            &job.owner_host,
            &job.directory_device,
            &job.directory_inode,
        ]
        .iter()
        .any(|v| v.is_empty())
        || ![0, 1].contains(&job.permission)
        || job.receipt_sha256.as_deref().is_some_and(|v| !digest(v))
        || job
            .published_revision
            .is_some_and(|r| r <= job.work_revision)
    {
        return Err(E::Refused("review-ledger-job-invalid"));
    }
    swarm_wire_json::go_rfc3339::parse(&job.preflight_until)
        .map_err(|e| E::caused("review-ledger-clock", e))?;
    let tuple = match (
        job.child_pid,
        job.parent_pid,
        job.group_id,
        job.child_stamp.as_deref(),
    ) {
        (None, None, None, None) => false,
        (Some(pid), Some(parent), Some(group), Some(stamp))
            if pid > 0
                && u32::try_from(pid).is_ok()
                && parent == job.owner_pid
                && group == pid
                && !stamp.is_empty() =>
        {
            true
        }
        _ => return Err(E::Refused("review-ledger-child-invalid")),
    };
    if (job.permission == 1 && !tuple)
        || (job.phase == Phase::Claimed && (tuple || job.permission != 0))
        || (job.phase == Phase::Registered && (!tuple || job.permission != 0))
        || (job.phase == Phase::Permitted && (!tuple || job.permission != 1))
        || (matches!(job.phase, Phase::Recorded | Phase::Refused)
            && (job.receipt_sha256.is_none() || job.published_revision.is_none()))
        || (!matches!(job.phase, Phase::Recorded | Phase::Refused)
            && job.published_revision.is_some())
    {
        return Err(E::Refused("review-ledger-phase-invalid"));
    }
    Ok(())
}
/// Read checked scalar columns through transient JSON projection, without an
/// operational BLOB. Key selector is private static vocabulary, never input SQL.
/// Missing schema/row means unknown, not safe absence of any process or grant.
fn find(db: &Connection, key: &str, value: &str) -> Result<Option<Job>, E> {
    if !schema(db, false)? {
        return Ok(None);
    }
    let fields = COLUMNS
        .iter()
        .map(|c| format!("'{c}',{c}"))
        .collect::<Vec<_>>()
        .join(",");
    let result = query(
        db,
        &format!("SELECT json_object({fields}) FROM {TABLE} WHERE {key}=?"),
        [value],
        |rows| {
            let raw = rows.next()?.map(|r| r.get::<_, String>(0)).transpose()?;
            if rows.next()?.is_some() {
                return Err(E::Refused("review-ledger-duplicate"));
            }
            raw.map(|v| {
                serde_json::from_str::<Job>(&v).map_err(|e| E::caused("review-ledger-decode", e))
            })
            .transpose()
        },
    )?;
    if let Some(job) = &result {
        valid(job)?;
    }
    Ok(result)
}
/// Read the engine-created job ID without adopting an owner or granting exec.
pub(super) fn job(db: &Connection, id: &str) -> Result<Option<Job>, E> {
    find(db, "id", id)
}
/// Read an opaque public replay key; caller must check canonical request/work.
pub(super) fn event(db: &Connection, id: &str) -> Result<Option<Job>, E> {
    find(db, "event_id", id)
}
/// Resolve current exclusion. Terminal rows remain history and never disappear.
pub(super) fn active(db: &Connection, work: &str, task: &str) -> Result<Option<Job>, E> {
    if !schema(db, false)? {
        return Ok(None);
    }
    let id:Option<String>=query(db,"SELECT id FROM native_review_jobs WHERE work_id=? AND task_id=? AND phase IN ('claimed','registered','permitted','unresolved')",params![work,task],|rows|{
        let value=rows.next()?.map(|r|r.get(0)).transpose()?;
        if rows.next()?.is_some(){return Err(E::Refused("review-ledger-multiple-owners"));}Ok(value)
    })?;
    id.map(|id| job(db, &id)?.ok_or(E::Refused("review-ledger-owner-missing")))
        .transpose()
}
/// Stage one fresh job in the caller's atomic Work/call/budget transaction.
/// No exec, replacement, quota reset or implicit retry follows from this row.
pub(super) fn claim(db: &Connection, job: &Job) -> Result<(), E> {
    transaction(db)?;
    valid(job)?;
    if job.phase != Phase::Claimed
        || job.receipt_sha256.is_some()
        || job.published_revision.is_some()
    {
        return Err(E::Refused("review-ledger-initial-job-invalid"));
    }
    schema(db, true)?;
    let raw = crate::encode_model(job).map_err(|e| E::caused("review-ledger-encode", e))?;
    let raw = std::str::from_utf8(&raw).map_err(|e| E::caused("review-ledger-utf8", e))?;
    let expressions = COLUMNS
        .iter()
        .map(|c| format!("json_extract(?1,'$.{c}')"))
        .collect::<Vec<_>>()
        .join(",");
    let changed = db.execute(
        &format!(
            "INSERT INTO {TABLE}({}) SELECT {expressions}",
            COLUMNS.join(",")
        ),
        [raw],
    )?;
    if changed != 1 || self::job(db, &job.id)?.as_ref() != Some(job) {
        return Err(E::Refused("review-ledger-insert-readback"));
    }
    Ok(())
}
/// Atomically advance one current job, preserving all immutable source/owner
/// fields and registered child tuples. Permission is monotonic. Recovery never
/// returns a failed/uncertain job to a phase capable of a new grant.
/// Every error requires caller rollback, including readback/finalization errors.
pub(super) fn change(db: &Connection, old: &Job, new: &Job) -> Result<(), E> {
    transaction(db)?;
    valid(old)?;
    valid(new)?;
    if job(db, &old.id)?.as_ref() != Some(old) {
        return Err(E::Refused("review-ledger-state-changed"));
    }
    let mut immutable = old.clone();
    immutable.child_pid = new.child_pid;
    immutable.parent_pid = new.parent_pid;
    immutable.group_id = new.group_id;
    immutable.child_stamp.clone_from(&new.child_stamp);
    immutable.permission = new.permission;
    immutable.phase = new.phase;
    immutable.receipt_sha256.clone_from(&new.receipt_sha256);
    immutable.published_revision = new.published_revision;
    if immutable != *new
        || old.permission > new.permission
        || (old.permission != new.permission
            && !(old.phase == Phase::Registered && new.phase == Phase::Permitted))
        || (old.child_pid.is_some()
            && (old.child_pid != new.child_pid
                || old.parent_pid != new.parent_pid
                || old.group_id != new.group_id
                || old.child_stamp != new.child_stamp))
        || (old.receipt_sha256.is_some() && old.receipt_sha256 != new.receipt_sha256)
        || !matches!(
            (old.phase, new.phase),
            (
                Phase::Claimed,
                Phase::Registered | Phase::Unresolved | Phase::Refused
            ) | (
                Phase::Registered,
                Phase::Permitted | Phase::Unresolved | Phase::Refused
            ) | (
                Phase::Permitted,
                Phase::Unresolved | Phase::Recorded | Phase::Refused
            ) | (Phase::Unresolved, Phase::Recorded | Phase::Refused)
        )
    {
        return Err(E::Refused("review-ledger-transition-invalid"));
    }
    let raw = crate::encode_model(new).map_err(|e| E::caused("review-ledger-encode", e))?;
    let raw = std::str::from_utf8(&raw).map_err(|e| E::caused("review-ledger-utf8", e))?;
    let mutable = [
        "child_pid",
        "parent_pid",
        "group_id",
        "child_stamp",
        "permission",
        "phase",
        "receipt_sha256",
        "published_revision",
    ];
    let set = mutable
        .iter()
        .map(|c| format!("{c}=json_extract(?1,'$.{c}')"))
        .collect::<Vec<_>>()
        .join(",");
    if db.execute(
        &format!("UPDATE {TABLE} SET {set} WHERE id=?2 AND permission=?3"),
        params![raw, old.id, old.permission],
    )? != 1
        || job(db, &old.id)?.as_ref() != Some(new)
    {
        return Err(E::Refused("review-ledger-update-readback"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Supplied scalar data only. These PID/hash values are not OS observations,
    /// actor authentication, an execution grant or a positive review receipt.
    fn fixture() -> (Connection, Job) {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE works(id TEXT PRIMARY KEY); INSERT INTO works(id) VALUES('work');").unwrap();
        let hash = "a".repeat(64);
        let job = Job {
            id: "review-1".into(),
            work_id: "work".into(),
            task_id: "task".into(),
            event_id: "request-1".into(),
            request_sha256: hash.clone(),
            producer_id: "producer".into(),
            attempt_id: "attempt".into(),
            source_sha256: hash.clone(),
            validation_receipt_sha256: hash.clone(),
            contract_sha256: hash.clone(),
            provider_sha256: hash.clone(),
            executable_sha256: hash.clone(),
            host_scope: hash.clone(),
            workflow_sha256: hash.clone(),
            prompt_sha256: hash,
            work_revision: 2,
            settings_revision: 1,
            owner_pid: 10,
            owner_stamp: "supplied-owner".into(),
            owner_host: "supplied-host".into(),
            directory_device: "1".into(),
            directory_inode: "2".into(),
            preflight_until: "2026-10-21T00:00:00Z".into(),
            child_pid: None,
            parent_pid: None,
            group_id: None,
            child_stamp: None,
            permission: 0,
            phase: Phase::Claimed,
            receipt_sha256: None,
            published_revision: None,
        };
        (db, job)
    }
    /// Stage the actual candidate ledger, not a seeded fake positive execution.
    fn claimed(db: &Connection, value: &Job) {
        let tx = db.unchecked_transaction().unwrap();
        claim(&tx, value).unwrap();
        tx.commit().unwrap();
    }
    /// Registration describes supplied data; permission still requires its own
    /// legal transition. This helper starts no process and checks no real PID.
    fn registered(value: &Job) -> Job {
        let mut next = value.clone();
        next.phase = Phase::Registered;
        next.child_pid = Some(20);
        next.parent_pid = Some(value.owner_pid);
        next.group_id = Some(20);
        next.child_stamp = Some("supplied-child".into());
        next
    }

    /// Establish the valid next transition in a savepoint, then restore the same
    /// old scalar state before injecting a fault. This proves data admissibility,
    /// never real publication, OS identity, execution or task acceptance.
    fn witness(db: &Connection, old: &Job, next: &Job) {
        db.execute_batch("SAVEPOINT review_healthy_witness")
            .unwrap();
        change(db, old, next).unwrap();
        db.execute_batch("ROLLBACK TO review_healthy_witness; RELEASE review_healthy_witness")
            .unwrap();
        assert!(job(db, &old.id).unwrap().as_ref() == Some(old));
    }
    /// Exact scalar readback and rollback guard schema creation and exclusion;
    /// one active task owner is an integrity rule, not an operator count ceiling.
    #[test]
    fn reviewer_ledger_claim_is_scalar_atomic_and_excludes_duplicate_owner() {
        let (db, value) = fixture();
        assert!(!schema(&db, false).unwrap());
        assert!(claim(&db, &value).is_err());
        assert!(!schema(&db, false).unwrap());
        let tx = db.unchecked_transaction().unwrap();
        claim(&tx, &value).unwrap();
        assert!(job(&tx, &value.id).unwrap().as_ref() == Some(&value));
        tx.rollback().unwrap();
        assert!(!schema(&db, false).unwrap());
        claimed(&db, &value);
        assert!(event(&db, &value.event_id).unwrap().as_ref() == Some(&value));
        assert!(
            active(&db, &value.work_id, &value.task_id)
                .unwrap()
                .as_ref()
                == Some(&value)
        );
        let mut other = value.clone();
        other.id = "review-2".into();
        other.event_id = "request-2".into();
        let tx = db.unchecked_transaction().unwrap();
        assert!(claim(&tx, &other).is_err());
        tx.rollback().unwrap();
        assert!(job(&db, &other.id).unwrap().is_none());
        assert!(job(&db, &value.id).unwrap().as_ref() == Some(&value));
        let blobs: i64 = db
            .query_row(
                "SELECT count(*) FROM pragma_table_info('native_review_jobs') WHERE type='BLOB'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(blobs, 0);
    }
    /// A valid registration precedes attempts to manufacture grant permission
    /// while unresolved or to move an uncertain call back into runnable state.
    #[test]
    fn reviewer_ledger_permission_requires_registered_to_permitted_transition() {
        let (db, value) = fixture();
        claimed(&db, &value);
        let tx = db.unchecked_transaction().unwrap();
        let registration = registered(&value);
        change(&tx, &value, &registration).unwrap();
        let mut healthy = registration.clone();
        healthy.phase = Phase::Permitted;
        healthy.permission = 1;
        witness(&tx, &registration, &healthy);
        let mut forged = registration.clone();
        forged.phase = Phase::Unresolved;
        forged.permission = 1;
        assert!(change(&tx, &registration, &forged).is_err());
        assert!(job(&tx, &value.id).unwrap().as_ref() == Some(&registration));
        let mut permitted = registration.clone();
        permitted.phase = Phase::Permitted;
        permitted.permission = 1;
        change(&tx, &registration, &permitted).unwrap();
        let mut unresolved = permitted.clone();
        unresolved.phase = Phase::Unresolved;
        change(&tx, &permitted, &unresolved).unwrap();
        let mut terminal = unresolved.clone();
        terminal.phase = Phase::Recorded;
        terminal.receipt_sha256 = Some("b".repeat(64));
        terminal.published_revision = Some(3);
        witness(&tx, &unresolved, &terminal);
        assert!(change(&tx, &unresolved, &permitted).is_err());
        assert!(job(&tx, &value.id).unwrap().as_ref() == Some(&unresolved));
        tx.rollback().unwrap();
        assert!(job(&db, &value.id).unwrap().as_ref() == Some(&value));
    }
    /// Known scalar origins/child tuples cannot be replaced during publication;
    /// terminal history releases exclusion but does not accept a task.
    #[test]
    fn reviewer_ledger_publication_preserves_origin_and_terminal_history() {
        let (db, value) = fixture();
        claimed(&db, &value);
        let tx = db.unchecked_transaction().unwrap();
        let registration = registered(&value);
        change(&tx, &value, &registration).unwrap();
        let mut permitted = registration.clone();
        permitted.permission = 1;
        permitted.phase = Phase::Permitted;
        change(&tx, &registration, &permitted).unwrap();
        let mut recorded = permitted.clone();
        recorded.phase = Phase::Recorded;
        recorded.receipt_sha256 = Some("b".repeat(64));
        recorded.published_revision = Some(3);
        witness(&tx, &permitted, &recorded);
        let mut stale = recorded.clone();
        stale.published_revision = Some(permitted.work_revision);
        assert!(change(&tx, &permitted, &stale).is_err());
        witness(&tx, &permitted, &recorded);
        let mut altered = recorded.clone();
        altered.source_sha256 = "c".repeat(64);
        assert!(change(&tx, &permitted, &altered).is_err());
        assert!(job(&tx, &value.id).unwrap().as_ref() == Some(&permitted));
        witness(&tx, &permitted, &recorded);
        let mut replaced_program = recorded.clone();
        replaced_program.executable_sha256 = "c".repeat(64);
        assert!(change(&tx, &permitted, &replaced_program).is_err());
        witness(&tx, &permitted, &recorded);
        let mut changed_environment = recorded.clone();
        changed_environment.host_scope = "c".repeat(64);
        assert!(change(&tx, &permitted, &changed_environment).is_err());
        witness(&tx, &permitted, &recorded);
        let mut altered = recorded.clone();
        altered.child_stamp = "another-child".to_owned().into();
        assert!(change(&tx, &permitted, &altered).is_err());
        change(&tx, &permitted, &recorded).unwrap();
        tx.commit().unwrap();
        assert!(active(&db, &value.work_id, &value.task_id)
            .unwrap()
            .is_none());
        assert!(job(&db, &value.id).unwrap().as_ref() == Some(&recorded));
    }
    /// Schema refusal starts from a valid actual ledger and restores the exact
    /// fixture schema afterwards; no foreign/live database or row is modified.
    #[test]
    fn reviewer_ledger_rejects_extra_trigger_then_recovers() {
        let (db, value) = fixture();
        claimed(&db, &value);
        assert!(schema(&db, false).unwrap());
        db.execute_batch("CREATE TRIGGER unexpected_review_write AFTER UPDATE ON native_review_jobs BEGIN SELECT 1; END;").unwrap();
        assert!(schema(&db, false).is_err());
        db.execute_batch("DROP TRIGGER unexpected_review_write;")
            .unwrap();
        assert!(schema(&db, false).unwrap());
        assert!(job(&db, &value.id).unwrap().as_ref() == Some(&value));
    }
}
