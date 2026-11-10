//! Normalized native planner identity/provenance. No worker/task impersonation.
#![cfg(target_os = "linux")]
use crate::{agent_launch::query, AgentLaunchError as E};
use rusqlite::{Connection, Transaction};
use serde::{Deserialize, Serialize};

/// Lifecycle protocol, not an operator timeout or concurrency budget. Unresolved
/// ownership remains live for exclusion; no terminal business outcome is inferred.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Phase {
    Claimed,
    Registered,
    Permitted,
    Applied,
    Refused,
    Unresolved,
}

/// Private datastore projection. Normalized SQL columns are authoritative;
/// transient JSON parameters/read projections are never an operational BLOB.
/// Public requests cannot construct this actual owner/child/permission record.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Run {
    pub claim_id: String,
    pub work_id: String,
    pub scope_id: String,
    pub holder: String,
    pub generation: i64,
    pub scope_revision: i64,
    pub prepared_revision: i64,
    pub settings_revision: i64,
    pub owner_pid: i64,
    pub owner_stamp: String,
    pub owner_host: String,
    pub child_pid: Option<i64>,
    pub parent_pid: Option<i64>,
    pub group_id: Option<i64>,
    pub child_stamp: Option<String>,
    pub phase: Phase,
    pub permission: i64,
    pub host_scope: String,
    pub preflight_until: String,
    pub context_sha256: String,
    pub workflow_prompt_sha256: String,
    pub prompt_sha256: String,
    pub provider_digest: String,
    pub directory_device: String,
    pub directory_inode: String,
    pub decision_event: String,
    pub prepared_at: String,
    pub receipt_sha256: Option<String>,
    pub applied_revision: Option<i64>,
    pub refusal_code: Option<String>,
}
impl std::fmt::Debug for Run {
    /// Never render local host/stamp/context metadata via an incidental Debug.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativePlanningRun { redacted }")
    }
}
const TABLE: &str = "native_planning_runs";
const DDL: &str = "CREATE TABLE native_planning_runs(claim_id TEXT PRIMARY KEY REFERENCES native_planning_claims(claim_id),work_id TEXT NOT NULL REFERENCES works(id),scope_id TEXT NOT NULL,holder TEXT NOT NULL,generation INTEGER NOT NULL CHECK(generation>0),scope_revision INTEGER NOT NULL CHECK(scope_revision>0),prepared_revision INTEGER NOT NULL CHECK(prepared_revision>0),settings_revision INTEGER NOT NULL CHECK(settings_revision>0),owner_pid INTEGER NOT NULL CHECK(owner_pid>0),owner_stamp TEXT NOT NULL,owner_host TEXT NOT NULL,child_pid INTEGER,parent_pid INTEGER,group_id INTEGER,child_stamp TEXT,phase TEXT NOT NULL CHECK(phase IN ('claimed','registered','permitted','applied','refused','unresolved')),permission INTEGER NOT NULL CHECK(permission IN (0,1)),host_scope TEXT NOT NULL,preflight_until TEXT NOT NULL,context_sha256 TEXT NOT NULL,workflow_prompt_sha256 TEXT NOT NULL,prompt_sha256 TEXT NOT NULL,provider_digest TEXT NOT NULL,directory_device TEXT NOT NULL,directory_inode TEXT NOT NULL,decision_event TEXT NOT NULL UNIQUE,prepared_at TEXT NOT NULL,receipt_sha256 TEXT,applied_revision INTEGER,refusal_code TEXT)";
const INDEX: &str = "CREATE UNIQUE INDEX native_planning_live_scope ON native_planning_runs(work_id,scope_id) WHERE phase IN ('claimed','registered','permitted','unresolved')";
const COLUMNS: &[&str] = &[
    "claim_id",
    "work_id",
    "scope_id",
    "holder",
    "generation",
    "scope_revision",
    "prepared_revision",
    "settings_revision",
    "owner_pid",
    "owner_stamp",
    "owner_host",
    "child_pid",
    "parent_pid",
    "group_id",
    "child_stamp",
    "phase",
    "permission",
    "host_scope",
    "preflight_until",
    "context_sha256",
    "workflow_prompt_sha256",
    "prompt_sha256",
    "provider_digest",
    "directory_device",
    "directory_inode",
    "decision_event",
    "prepared_at",
    "receipt_sha256",
    "applied_revision",
    "refusal_code",
];
const MUTABLE: &[&str] = &[
    "child_pid",
    "parent_pid",
    "group_id",
    "child_stamp",
    "phase",
    "permission",
    "receipt_sha256",
    "applied_revision",
    "refusal_code",
];

/// Require the exact table, uniqueness indexes and absence of extra triggers or
/// indexes before consuming identity data. Missing legacy schema is explicit;
/// creation occurs only in the caller's transaction after claim preconditions.
/// The live-scope key serializes exclusive scope ownership including uncertainty,
/// not global planner concurrency; different scopes retain independent capacity.
pub(crate) fn schema(db: &Connection, create: bool) -> Result<bool, E> {
    let existing: Option<(String, String)> = query(
        db,
        "SELECT type,sql FROM sqlite_schema WHERE name=?",
        [TABLE],
        |rows| {
            Ok(rows
                .next()?
                .map(|r| Ok::<_, rusqlite::Error>((r.get(0)?, r.get(1)?)))
                .transpose()?)
        },
    )?;
    match existing {
        None if !create => return Ok(false),
        None => {
            if db.is_autocommit() {
                return Err(E::Refused("planning-run-transaction-required"));
            }
            db.execute_batch(DDL)?;
            db.execute_batch(INDEX)?;
        }
        Some((kind, sql)) if kind == "table" && sql == DDL => {}
        _ => return Err(E::Refused("planning-run-schema-invalid")),
    }
    let extras: Vec<(String, String, Option<String>)> = query(
        db,
        "SELECT type,name,sql FROM sqlite_schema WHERE tbl_name=? AND name!=? ORDER BY name",
        [TABLE, TABLE],
        |rows| {
            let mut out = Vec::new();
            while let Some(r) = rows.next()? {
                out.push((r.get(0)?, r.get(1)?, r.get(2)?));
            }
            Ok(out)
        },
    )?;
    let expected = vec![
        (
            "index".into(),
            "native_planning_live_scope".into(),
            Some(INDEX.into()),
        ),
        (
            "index".into(),
            "sqlite_autoindex_native_planning_runs_1".into(),
            None,
        ),
        (
            "index".into(),
            "sqlite_autoindex_native_planning_runs_2".into(),
            None,
        ),
    ];
    if extras != expected {
        return Err(E::Refused("planning-run-schema-invalid"));
    }
    Ok(true)
}

/// Validate wire/representation invariants, never manufacture observed identity.
/// Actual PID/starttime/parent/group checks belong to registration/gate callers.
fn validate(row: &Run) -> Result<(), E> {
    let hash = |s: &str| {
        s.len() == 64
            && s.bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    };
    let stamp = |s: &str| s.parse::<u64>().is_ok_and(|n| n.to_string() == s);
    if [
        &row.claim_id,
        &row.work_id,
        &row.scope_id,
        &row.holder,
        &row.decision_event,
    ]
    .iter()
    .any(|s| !swarm_identity_contract::name(s.as_bytes()))
        || [
            row.generation,
            row.scope_revision,
            row.prepared_revision,
            row.settings_revision,
            row.owner_pid,
        ]
        .iter()
        .any(|n| *n < 1)
        || !stamp(&row.owner_stamp)
        || row.owner_host.is_empty()
        || ![0, 1].contains(&row.permission)
        || [
            &row.host_scope,
            &row.context_sha256,
            &row.workflow_prompt_sha256,
            &row.prompt_sha256,
            &row.provider_digest,
        ]
        .iter()
        .any(|s| !hash(s))
        || [&row.directory_device, &row.directory_inode]
            .iter()
            .any(|s| !stamp(s))
        || row.receipt_sha256.as_deref().is_some_and(|s| !hash(s))
        || row.applied_revision.is_some_and(|r| r < 1)
    {
        return Err(E::Refused("planning-run-record-invalid"));
    }
    for at in [&row.preflight_until, &row.prepared_at] {
        swarm_wire_json::go_rfc3339::parse(at)
            .map_err(|e| E::caused("planning-run-record-clock", e))?;
    }
    let any = row.child_pid.is_some()
        || row.parent_pid.is_some()
        || row.group_id.is_some()
        || row.child_stamp.is_some();
    let complete = matches!((row.child_pid,row.parent_pid,row.group_id,row.child_stamp.as_deref()),(Some(pid),Some(parent),Some(group),Some(start)) if pid>0&&pid!=row.owner_pid&&parent==row.owner_pid&&group==pid&&stamp(start));
    if (any || row.permission == 1) && !complete
        || matches!(row.phase, Phase::Claimed) && any
        || matches!(row.phase, Phase::Registered) && (!complete || row.permission != 0)
        || matches!(row.phase, Phase::Permitted | Phase::Applied) && row.permission != 1
        || matches!(row.phase, Phase::Applied) && row.applied_revision.is_none()
    {
        return Err(E::Refused("planning-run-record-invalid"));
    }
    Ok(())
}

/// Build a transient JSON row projection from static engine-owned SQL names.
/// No user field enters SQL source; required serde fields prevent missing-column
/// or type errors from becoming empty successful records. Schema is checked first.
pub(crate) fn load(db: &Connection, id: &str) -> Result<Run, E> {
    if !schema(db, false)? {
        return Err(E::Refused("planning-run-not-recorded"));
    }
    let fields = COLUMNS
        .iter()
        .map(|c| format!("'{c}',{c}"))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("SELECT json_object({fields}) FROM native_planning_runs WHERE claim_id=?");
    let raw: String = query(db, &sql, [id], |rows| {
        Ok(rows
            .next()?
            .ok_or(E::Refused("planning-run-not-recorded"))?
            .get(0)?)
    })?;
    let row: Run =
        serde_json::from_str(&raw).map_err(|e| E::caused("planning-run-record-decode", e))?;
    validate(&row)?;
    Ok(row)
}

/// Read all normalized call rows for this exact work/scope in the caller TX.
/// Missing auxiliary schema means no native calls, never a fabricated process
/// exit. Every returned row is schema/type/identity validated; SQL/allocation and
/// statement cleanup failures propagate. There is no history scan policy cap.
pub(crate) fn scope_runs(db: &Connection, work: &str, scope: &str) -> Result<Vec<Run>, E> {
    if db.is_autocommit() {
        return Err(E::Refused("planning-run-transaction-required"));
    }
    if !schema(db, false)? {
        return Ok(Vec::new());
    }
    let ids=query(db,"SELECT claim_id FROM native_planning_runs WHERE work_id=? AND scope_id=? ORDER BY prepared_revision,claim_id",rusqlite::params![work,scope],|rows|{
        let mut ids=Vec::new();
        while let Some(row)=rows.next()?{
            ids.try_reserve(1).map_err(|e|E::caused("planning-run-history-allocation",e))?;
            ids.push(row.get::<_,String>(0)?);
        }
        Ok(ids)
    })?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(ids.len())
        .map_err(|e| E::caused("planning-run-history-allocation", e))?;
    for id in ids {
        result.push(load(db, &id)?);
    }
    Ok(result)
}
impl Phase {
    /// Execution ownership is retained until an actual terminal disposition.
    /// Lease expiry alone does not release it; this is the registry protocol,
    /// not an operator parallelism limit or proof that a process is still alive.
    pub(crate) fn retains_ownership(&self) -> bool {
        matches!(
            self,
            Self::Claimed | Self::Registered | Self::Permitted | Self::Unresolved
        )
    }
}
/// Refuse a NEW claim before activation/estimate spend while a previous native
/// call retains ownership. Requires the same Work transaction as admission and
/// is invoked after exact event replay has returned, so replay stays harmless.
/// No stale PID is signalled or unknown call refunded/treated as terminated.
pub(crate) fn require_claim_available(db: &Connection, work: &str, scope: &str) -> Result<(), E> {
    if scope_runs(db, work, scope)?
        .iter()
        .any(|r| r.phase.retains_ownership())
    {
        return Err(E::Refused("planning-claim-call-active"));
    }
    Ok(())
}
/// Record exactly one actual supervisor per immutable claim. No insert-or-replace
/// or takeover exists. Refusal/DDL/index/SQL/readback failures must roll back the
/// caller transaction, preserving the claim and its original financial estimate.
pub(crate) fn insert(tx: &Transaction<'_>, row: &Run) -> Result<(), E> {
    validate(row)?;
    schema(tx, true)?;
    if row.phase != Phase::Claimed || row.permission != 0 {
        return Err(E::Refused("planning-run-initial-phase-invalid"));
    }
    let active:i64=query(tx,"SELECT count(*) FROM native_planning_runs WHERE claim_id=? OR (work_id=? AND scope_id=? AND phase IN ('claimed','registered','permitted','unresolved'))",rusqlite::params![row.claim_id,row.work_id,row.scope_id],|rows|Ok(rows.next()?.ok_or(E::Refused("planning-run-registration-observation"))?.get(0)?))?;
    if active != 0 {
        return Err(E::Refused("planning-run-existing-owner"));
    }
    let raw = serde_json::to_string(row).map_err(|e| E::caused("planning-run-record-encode", e))?;
    let values = COLUMNS
        .iter()
        .map(|c| format!("json_extract(?1,'$.{c}')"))
        .collect::<Vec<_>>()
        .join(",");
    let changed = tx.execute(
        &format!(
            "INSERT INTO native_planning_runs({}) VALUES({values})",
            COLUMNS.join(",")
        ),
        [raw],
    )?;
    if changed != 1 || load(tx, &row.claim_id)? != *row {
        return Err(E::Refused("planning-run-registration-not-recorded"));
    }
    Ok(())
}

/// Compare complete prior bytes under the caller transaction, then modify only
/// lifecycle fields. Owner/source/claim metadata never changes. Permission cannot
/// be erased and an observed child tuple cannot be replaced. Readback catches
/// silent/no-op writes; no successful empty state or replayed launch is produced.
pub(crate) fn transition(db: &Connection, old: &Run, new: &Run) -> Result<(), E> {
    if db.is_autocommit() {
        return Err(E::Refused("planning-run-transaction-required"));
    }
    validate(new)?;
    if load(db, &old.claim_id)? != *old {
        return Err(E::Refused("planning-run-stale-owner"));
    }
    if old.permission == 1 && new.permission != 1
        || old.child_pid.is_some()
            && (old.child_pid != new.child_pid
                || old.parent_pid != new.parent_pid
                || old.group_id != new.group_id
                || old.child_stamp != new.child_stamp)
    {
        return Err(E::Refused("planning-run-evidence-regression"));
    }
    let a = serde_json::to_value(old).map_err(|e| E::caused("planning-run-record-encode", e))?;
    let b = serde_json::to_value(new).map_err(|e| E::caused("planning-run-record-encode", e))?;
    if COLUMNS
        .iter()
        .filter(|c| !MUTABLE.contains(c))
        .any(|c| a.get(*c) != b.get(*c))
    {
        return Err(E::Refused("planning-run-immutable-source-changed"));
    }
    let allowed = match (&old.phase, &new.phase) {
        (Phase::Claimed, Phase::Registered)
        | (Phase::Registered, Phase::Permitted)
        | (Phase::Permitted, Phase::Applied) => true,
        (
            Phase::Claimed | Phase::Registered | Phase::Permitted | Phase::Unresolved,
            Phase::Refused | Phase::Unresolved,
        ) => true,
        (Phase::Unresolved, Phase::Applied) => true,
        _ => old == new,
    };
    if !allowed {
        return Err(E::Refused("planning-run-phase-invalid"));
    }
    let raw = serde_json::to_string(new).map_err(|e| E::caused("planning-run-record-encode", e))?;
    let fields = MUTABLE
        .iter()
        .map(|c| format!("{c}=json_extract(?1,'$.{c}')"))
        .collect::<Vec<_>>()
        .join(",");
    if db.execute(
        &format!("UPDATE native_planning_runs SET {fields} WHERE claim_id=?2"),
        rusqlite::params![raw, old.claim_id],
    )? != 1
        || load(db, &old.claim_id)? != *new
    {
        return Err(E::Refused("planning-run-transition-not-recorded"));
    }
    Ok(())
}
