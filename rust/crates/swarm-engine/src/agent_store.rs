//! Agent observations and durable stop intent, not launch or process termination.
use crate::{decode_agent_row, Agent, WorkError};
use rusqlite::{types::ValueRef, Connection, Params, Rows, Transaction};
use std::{error::Error, fmt};

pub enum AgentStoreError {
    Sql(rusqlite::Error),
    Body(WorkError),
    Encode(WorkError),
    Runtime(crate::RuntimeError),
    Work(Box<crate::RecordedWorkError>),
    Transition(Box<crate::TaskLifecycleError>),
    ScopeSignal(Box<crate::AgentLaunchError>),
    /// Original mission provenance failure, without minting new execution rights.
    MissionProvenance(Box<crate::AgentLaunchError>),
    Change(Box<crate::RecordedChangeError>),
    #[cfg(target_os = "linux")]
    Supervisor(crate::SupervisorIdentityError),
    Clock(swarm_wire_json::go_rfc3339::GoTimestampError),
    Missing,
    Refused(&'static str),
    Finalize {
        primary: Box<Self>,
        cleanup: rusqlite::Error,
    },
    Rollback {
        primary: Box<Self>,
        cleanup: rusqlite::Error,
    },
    Commit {
        cause: rusqlite::Error,
        rolled_back: bool,
        cleanup: Option<Box<rusqlite::Error>>,
    },
}
impl AgentStoreError {
    /// Stable public classification; SQL/body details remain in typed causes.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Sql(_) => "agent-storage",
            Self::Body(_) => "agent-document-invalid",
            Self::Encode(_) => "agent-document-encoding",
            Self::Runtime(e) => e.code(),
            Self::Work(e) => e.code(),
            Self::Transition(e) => e.code(),
            Self::ScopeSignal(e) | Self::MissionProvenance(e) => e.code(),
            Self::Change(e) => e.code(),
            #[cfg(target_os = "linux")]
            Self::Supervisor(e) => e.code(),
            Self::Clock(_) => "agent-clock-invalid",
            Self::Missing => "agent-missing",
            Self::Refused(code) => code,
            Self::Finalize { .. } => "agent-storage-cleanup",
            Self::Rollback { .. } => "agent-rollback-failed",
            Self::Commit {
                rolled_back: true, ..
            } => "agent-commit-refused",
            Self::Commit { .. } => "agent-commit-uncertain",
        }
    }
}
impl fmt::Display for AgentStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transition(error) => fmt::Display::fmt(error, f),
            _ => f.write_str(self.code()),
        }
    }
}
impl fmt::Debug for AgentStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for AgentStoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Sql(e) => Some(e),
            Self::Body(e) | Self::Encode(e) => Some(e),
            Self::Runtime(e) => Some(e),
            Self::Work(e) => Some(e.as_ref()),
            Self::Transition(e) => Some(e.as_ref()),
            Self::ScopeSignal(e) | Self::MissionProvenance(e) => Some(e.as_ref()),
            Self::Change(e) => Some(e.as_ref()),
            #[cfg(target_os = "linux")]
            Self::Supervisor(e) => Some(e),
            Self::Clock(e) => Some(e),
            Self::Finalize { primary, .. } | Self::Rollback { primary, .. } => {
                Some(primary.as_ref())
            }
            Self::Commit { cause, .. } => Some(cause),
            Self::Missing | Self::Refused(_) => None,
        }
    }
}
impl From<rusqlite::Error> for AgentStoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}
impl From<WorkError> for AgentStoreError {
    fn from(error: WorkError) -> Self {
        Self::Body(error)
    }
}

/// Execute one parameterized read and explicitly finalize its statement.
/// Read and cleanup causes survive together; no transaction, retry or policy.
fn query<T>(
    db: &Connection,
    sql: &str,
    parameters: impl Params,
    read: impl FnOnce(&mut Rows<'_>) -> Result<T, AgentStoreError>,
) -> Result<T, AgentStoreError> {
    let mut statement = db.prepare(sql)?;
    let result = match statement.query(parameters) {
        Ok(mut rows) => read(&mut rows),
        Err(e) => Err(e.into()),
    };
    match (result, statement.finalize()) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error.into()),
        (Err(primary), Err(cleanup)) => Err(AgentStoreError::Finalize {
            primary: Box::new(primary),
            cleanup,
        }),
    }
}

/// Decode SQL lifecycle columns over JSON body; malformed rows fail the read.
/// SQL text/BLOB follows existing database/sql scalar scanning. Metadata remains
/// untrusted observation, not a process/host identity or signalling capability.
fn row(row: &rusqlite::Row<'_>) -> Result<Agent, AgentStoreError> {
    let raw = match row.get_ref(0)? {
        ValueRef::Blob(bytes) | ValueRef::Text(bytes) => bytes,
        value => {
            return Err(rusqlite::Error::InvalidColumnType(
                0,
                "agent body".into(),
                value.data_type(),
            )
            .into())
        }
    };
    let status = crate::sqlite_value::string(row.get_ref(1)?, 1)?;
    let desired = crate::sqlite_value::string(row.get_ref(2)?, 2)?;
    let status = std::str::from_utf8(&status).map_err(WorkError::Utf8)?;
    let desired = std::str::from_utf8(&desired).map_err(WorkError::Utf8)?;
    Ok(decode_agent_row(raw, status, desired)?)
}

/// Require the decoded identity to match the authoritative selected SQL key.
/// This detects storage incoherence, never authenticates the actor/process. No
/// mutation/fallback occurs; callers retain the original typed refusal.
fn checked_identity(agent: Agent, expected: &[u8]) -> Result<Agent, AgentStoreError> {
    if agent.id.as_bytes() != expected {
        return Err(AgentStoreError::Refused("agent-row-identity-mismatch"));
    }
    Ok(agent)
}

/// Read one SQL-selected agent without mutation or live-process inference.
/// Require body ID to match the selected SQL key; an alias row cannot select a
/// different agent via its opaque body. This is consistency, not authentication.
/// Missing, invalid JSON/types and SQL/finalization failures are explicit errors;
/// no empty successful agent is fabricated. Connection policy belongs to caller.
pub fn read_agent(db: &Connection, id: &str) -> Result<Agent, AgentStoreError> {
    query(
        db,
        "SELECT body,status,desired FROM main.agents WHERE id=?",
        [id],
        |rows| {
            checked_identity(
                row(rows.next()?.ok_or(AgentStoreError::Missing)?)?,
                id.as_bytes(),
            )
        },
    )
}

/// Read all retained attempts newest first, including terminal history.
/// Every decoded ID must match its SQL key; one incoherent row refuses the list.
/// Intentionally independent of Go's recent UI200 window: this complete reader
/// is suitable for accounting/recovery, with no hidden count policy. Caller owes
/// any explicit UI pagination contract. Any malformed row refuses the full read.
pub fn list_agents(db: &Connection, work: &str) -> Result<Vec<Agent>, AgentStoreError> {
    query(
        db,
        "SELECT body,status,desired,id FROM main.agents WHERE work_id=? ORDER BY rowid DESC",
        [work],
        |rows| {
            let mut agents = Vec::new();
            while let Some(next) = rows.next()? {
                let id = crate::sqlite_value::string(next.get_ref(3)?, 3)?;
                agents.push(checked_identity(row(next)?, &id)?);
            }
            Ok(agents)
        },
    )
}

/// Recheck all authoritative ownership columns against the observed body.
/// Used after staged lifecycle writes to detect a later trigger changing a row's
/// workspace/work/task while its JSON still appears valid; refusal is rolled back
/// by the caller. Missing/invalid SQL data remains an error, never a match.
pub(crate) fn verify_agent_ownership(
    db: &Connection,
    agent: &Agent,
) -> Result<(), AgentStoreError> {
    query(
        db,
        "SELECT work_id,task_id,cwd FROM main.agents WHERE id=?",
        [&agent.id],
        |rows| {
            let row = rows.next()?.ok_or(AgentStoreError::Missing)?;
            for (index, expected) in [&agent.work_id, &agent.task_id, &agent.workspace]
                .into_iter()
                .enumerate()
            {
                if crate::sqlite_value::string(row.get_ref(index)?, index)? != expected.as_bytes() {
                    return Err(AgentStoreError::Refused("agent-row-identity-mismatch"));
                }
            }
            Ok(())
        },
    )
}

/// Persist an idempotent operator stop intention in the caller-owned transaction.
/// Reserves the writer before observing lifecycle; terminal/missing/corrupt state
/// refuses. Repeated active stop adds no log. Only desired and a command log change;
/// never signals a PID, releases reservations or claims confirmed termination.
/// Caller must commit/rollback and supply an engine-observed timestamp. Identity
/// and authority are not established by the supplied id or retained JSON body.
pub fn request_agent_stop(
    tx: &Transaction<'_>,
    id: &str,
    at: &str,
) -> Result<Agent, AgentStoreError> {
    if tx.is_autocommit() {
        return Err(AgentStoreError::Refused("agent-transaction-inactive"));
    }
    swarm_wire_json::go_rfc3339::parse(at).map_err(AgentStoreError::Clock)?;
    // The write reservation avoids a stale read snapshot upgrade; this no-op
    // is rolled back with every refused request by the transaction owner.
    let mut statement = tx.prepare("UPDATE main.agents SET desired=desired WHERE id=?")?;
    let result = statement.execute([id]);
    match (result, statement.finalize()) {
        (Ok(_), Ok(())) => {}
        (Err(e), Ok(())) | (Ok(_), Err(e)) => return Err(e.into()),
        (Err(primary), Err(cleanup)) => {
            return Err(AgentStoreError::Finalize {
                primary: Box::new(primary.into()),
                cleanup,
            })
        }
    }
    let epoch = query(tx, "PRAGMA main.user_version", [], |rows| {
        Ok(crate::sqlite_value::integer(
            rows.next()?.ok_or(AgentStoreError::Missing)?.get_ref(0)?,
            0,
        )?)
    })?;
    if epoch != crate::RECORDED_COMMAND_SCHEMA_VERSION {
        return Err(AgentStoreError::Refused("storage_version_conflict"));
    }
    let agent = read_agent(tx, id)?;
    if !agent.is_active() {
        return Err(AgentStoreError::Refused("agent-already-terminal"));
    }
    // Mutating consumers bind authoritative row key to body identity. A mismatched
    // imported document is never allowed to direct another session's operation.
    if agent.id != id {
        return Err(AgentStoreError::Refused("agent-row-identity-mismatch"));
    }
    if agent.desired == "stop" {
        return Ok(agent);
    }
    let changed = tx.execute("UPDATE main.agents SET desired='stop' WHERE id=?", [id])?;
    if changed != 1 {
        return Err(AgentStoreError::Refused("agent-stop-not-recorded"));
    }
    let message = "Arrêt demandé ; attendre confirmation du superviseur";
    let sequence:i64=query(tx, "INSERT INTO main.agent_logs(agent_id,at,kind,message) VALUES(?,?,'command',?) RETURNING seq", rusqlite::params![id,at,message], |rows| {
        Ok(rows.next()?.ok_or(AgentStoreError::Refused("agent-stop-log-not-recorded"))?.get(0)?)
    })?;
    let log_valid = query(
        tx,
        "SELECT at,kind,message FROM main.agent_logs WHERE seq=? AND agent_id=?",
        rusqlite::params![sequence, id],
        |rows| {
            let Some(row) = rows.next()? else {
                return Ok(false);
            };
            Ok(row.get::<_, String>(0)? == at
                && row.get::<_, String>(1)? == "command"
                && row.get::<_, String>(2)? == message)
        },
    )?;
    if !log_valid {
        return Err(AgentStoreError::Refused("agent-stop-log-not-recorded"));
    }
    // A successful SQLite call alone does not prove the effective state when
    // triggers silently ignore/revert a write. Refusal rolls back log and intent.
    let stored = read_agent(tx, id)?;
    if stored.id != id || stored.desired != "stop" || stored.status != agent.status {
        return Err(AgentStoreError::Refused("agent-stop-state-not-recorded"));
    }
    Ok(stored)
}

/// Own the short stop transaction and explicitly handle rollback/commit failure.
/// SQL refusal rolls back desired and log together. A commit failure preserves
/// its cause and reports whether rollback was established; read back before any
/// retry when uncertain. No implicit retry, process signal or task acceptance.
pub fn execute_agent_stop(
    db: &mut Connection,
    id: &str,
    at: &str,
) -> Result<Agent, AgentStoreError> {
    transaction(db, rusqlite::TransactionBehavior::Deferred, |tx| {
        request_agent_stop(tx, id, at)
    })
}

/// Own a short lifecycle transaction with explicit rollback/commit diagnostics.
/// The operation stages data only; a failed commit returns no ownership receipt.
/// Never retry implicitly or hide cleanup through Transaction's Drop behavior.
fn transaction<T>(
    db: &mut Connection,
    behavior: rusqlite::TransactionBehavior,
    operation: impl FnOnce(&mut Transaction<'_>) -> Result<T, AgentStoreError>,
) -> Result<T, AgentStoreError> {
    let mut tx = db.transaction_with_behavior(behavior)?;
    let result = operation(&mut tx);
    // No implicit Drop rollback may hide its cleanup cause after failed commit.
    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
    match result {
        Ok(agent) => match tx.commit() {
            Ok(()) => Ok(agent),
            Err(cause) => {
                let active = !db.is_autocommit();
                let cleanup = if active {
                    db.execute_batch("ROLLBACK").err().map(Box::new)
                } else {
                    None
                };
                Err(AgentStoreError::Commit {
                    cause,
                    rolled_back: active && cleanup.is_none() && db.is_autocommit(),
                    cleanup,
                })
            }
        },
        Err(primary) => match tx.rollback() {
            Ok(()) => Err(primary),
            Err(cleanup) => Err(AgentStoreError::Rollback {
                primary: Box::new(primary),
                cleanup,
            }),
        },
    }
}

/// Durable local supervisor registration, not permission to start a provider.
/// Private fields prevent constructing a receipt from supplied JSON/PID data.
/// Preparation, identity/permissions, context freshness and admission still need
/// verification before any launch. No process signal or budget settlement occurs.
#[cfg(target_os = "linux")]
pub struct SupervisorClaim {
    agent: Agent,
    identity: crate::SupervisorIdentity,
}
#[cfg(target_os = "linux")]
impl SupervisorClaim {
    /// Observe the registered agent; cloning its data grants no ownership/rights.
    pub fn agent(&self) -> &Agent {
        &self.agent
    }
    /// Kernel observation used for registration; not authenticated agent identity.
    pub fn identity(&self) -> &crate::SupervisorIdentity {
        &self.identity
    }
}
#[cfg(target_os = "linux")]
impl fmt::Debug for SupervisorClaim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SupervisorClaim { registration: redacted }")
    }
}

/// Register this real process as the only supervisor of a queued agent.
/// Immediate writer reservation precedes reads; SQL/body status, row keys, host,
/// queued registration and storage epoch must agree. Starting state, actual PID/
/// starttime and heartbeat publish together. Readback detects ignored/reverted
/// writes; all refusal rolls back. An existing desired=stop remains stop and must
/// be handled before provider start. Failed commit yields no successful receipt.
/// Connection/busy policy belongs to the caller. This is storage ownership only:
/// no provider, context/permission grant, financial reservation or task acceptance.
/// This Connection-only entry accepts manual agents only; native intentions need
/// the internal Root-derived path and cannot acquire authority from JSON labels.
#[cfg(target_os = "linux")]
pub fn claim_agent_supervisor(
    db: &mut Connection,
    id: &str,
) -> Result<SupervisorClaim, AgentStoreError> {
    claim_agent_supervisor_mode(db, id, ClaimAuthority::Manual)
}

/// Register using the actual opened Root directory and the caller's reserved
/// settings snapshot. Native provenance and physical instance are checked inside
/// the registration transaction before any starting/PID write. A client tuple
/// cannot construct this route; errors roll back with typed causes retained.
#[cfg(target_os = "linux")]
pub(crate) fn claim_agent_supervisor_bound(
    db: &mut Connection,
    id: &str,
    directory: &crate::local_storage::LocalDirectory,
    settings: &crate::EngineSettingsDocument,
) -> Result<SupervisorClaim, AgentStoreError> {
    claim_agent_supervisor_mode(db, id, ClaimAuthority::Root { directory, settings })
}

/// Private routing only: the public Connection-only API is manual-only. Native
/// rows cannot opt out of a Root-derived observation using a caller-supplied flag.
#[cfg(target_os = "linux")]
enum ClaimAuthority<'a> {
    Manual,
    Root {
        directory: &'a crate::local_storage::LocalDirectory,
        settings: &'a crate::EngineSettingsDocument,
    },
}

/// Own the immediate registration transaction for both manual and Root routes.
/// Read canonical row/status first; native instance/provenance failures precede
/// mutation. Commit/readback/rollback behavior is shared with the original API.
#[cfg(target_os = "linux")]
fn claim_agent_supervisor_mode(
    db: &mut Connection,
    id: &str,
    authority: ClaimAuthority<'_>,
) -> Result<SupervisorClaim, AgentStoreError> {
    let identity = crate::SupervisorIdentity::current().map_err(AgentStoreError::Supervisor)?;
    let at = crate::system_timestamp().map_err(AgentStoreError::Runtime)?;
    let agent = transaction(db, rusqlite::TransactionBehavior::Immediate, |tx| {
        let epoch = query(tx, "PRAGMA main.user_version", [], |rows| {
            Ok(crate::sqlite_value::integer(
                rows.next()?.ok_or(AgentStoreError::Missing)?.get_ref(0)?,
                0,
            )?)
        })?;
        if epoch != crate::RECORDED_COMMAND_SCHEMA_VERSION {
            return Err(AgentStoreError::Refused("storage_version_conflict"));
        }
        let (before, status, desired, work, task, cwd) = query(
            tx,
            "SELECT body,status,desired,work_id,task_id,cwd FROM main.agents WHERE id=?",
            [id],
            |rows| {
                let row = rows.next()?.ok_or(AgentStoreError::Missing)?;
                let raw = match row.get_ref(0)? {
                    ValueRef::Blob(raw) | ValueRef::Text(raw) => raw.to_vec(),
                    other => {
                        return Err(rusqlite::Error::InvalidColumnType(
                            0,
                            "agent body".into(),
                            other.data_type(),
                        )
                        .into())
                    }
                };
                let mut values = Vec::new();
                for column in 1..=5 {
                    let raw = crate::sqlite_value::string(row.get_ref(column)?, column)?;
                    values.push(
                        String::from_utf8(raw)
                            .map_err(|e| AgentStoreError::Body(WorkError::Utf8(e.utf8_error())))?,
                    );
                }
                let mut values = values.into_iter();
                // Exactly five authoritative columns were read above.
                let mut next = || {
                    values
                        .next()
                        .ok_or(AgentStoreError::Refused("agent-row-incomplete"))
                };
                Ok((raw, next()?, next()?, next()?, next()?, next()?))
            },
        )?;
        let mut agent = crate::decode_agent(&before)?;
        if agent.id != id
            || agent.work_id != work
            || agent.task_id != task
            || agent.workspace != cwd
        {
            return Err(AgentStoreError::Refused("agent-row-identity-mismatch"));
        }
        if status != "queued" || agent.status != "queued" {
            return Err(AgentStoreError::Refused("agent-already-claimed"));
        }
        if agent.supervisor_pid != 0
            || agent.child_pid != 0
            || !agent.heartbeat.is_empty()
            || !agent.supervisor_identity.is_empty()
            || !agent.child_identity.is_empty()
        {
            return Err(AgentStoreError::Refused("agent-registration-inconsistent"));
        }
        if agent.host != identity.host {
            return Err(AgentStoreError::Refused("agent-other-host"));
        }
        if agent.native_mission.is_some() || agent.origin == "conducteur" {
            match &authority {
                ClaimAuthority::Manual => return Err(AgentStoreError::Refused("mission-agent-root-required")),
                ClaimAuthority::Root { directory, settings } => {
                    crate::root_store::mission_agent::inspect(
                        tx, &agent, settings,
                        crate::root_store::mission_agent::Check::BeforeClaim(directory),
                    ).map_err(|error| AgentStoreError::MissionProvenance(Box::new(error)))?
                        .ok_or(AgentStoreError::Refused("mission-agent-binding-required"))?;
                },
            }
        }
        agent.status = "starting".into();
        agent.desired = desired.clone();
        agent.supervisor_pid = identity.pid;
        agent.supervisor_identity = identity.stamp.clone();
        agent.heartbeat = at.clone();
        let encoded = crate::encode_model(&agent).map_err(AgentStoreError::Encode)?;
        let changed=tx.execute(
            "UPDATE main.agents SET status='starting',body=? WHERE id=? AND status='queued' AND desired=? AND CAST(body AS BLOB)=? AND work_id=? AND task_id=? AND cwd=?",
            rusqlite::params![encoded,id,desired,before,work,task,cwd],
        )?;
        if changed != 1 {
            return Err(AgentStoreError::Refused("agent-claim-not-recorded"));
        }
        let stored = read_agent(tx, id)?;
        let body: Vec<u8> = query(
            tx,
            "SELECT CAST(body AS BLOB) FROM main.agents WHERE id=?",
            [id],
            |rows| Ok(rows.next()?.ok_or(AgentStoreError::Missing)?.get(0)?),
        )?;
        if body != encoded || stored.status != "starting" || stored.desired != desired {
            return Err(AgentStoreError::Refused("agent-claim-state-not-recorded"));
        }
        verify_agent_ownership(tx, &stored)?;
        Ok(stored)
    })?;
    Ok(SupervisorClaim { agent, identity })
}

#[derive(PartialEq)]
struct ReservationSnapshot {
    work: String,
    amount: f64,
    state: String,
}

/// Observe the optional original reservation without inferring a zero estimate.
/// Malformed/nonfinite/negative data refuses. Retain scope, amount and state so
/// release can verify history was preserved rather than merely disappearing.
fn reservation_snapshot(
    db: &Connection,
    id: &str,
) -> Result<Option<ReservationSnapshot>, AgentStoreError> {
    query(
        db,
        "SELECT work_id,amount,state FROM main.reservations WHERE agent_id=?",
        [id],
        |rows| {
            let Some(row) = rows.next()? else {
                return Ok(None);
            };
            let snapshot = ReservationSnapshot {
                work: row.get(0)?,
                amount: row.get(1)?,
                state: row.get(2)?,
            };
            if !snapshot.amount.is_finite() || snapshot.amount < 0.0 {
                return Err(AgentStoreError::Refused("agent-reservation-invalid"));
            }
            Ok(Some(snapshot))
        },
    )
}

/// Cancel a queued, unregistered launch and settle its matching manual attempt
/// in one caller-policy transaction. The legacy starting/body-queued gap requires
/// durable desired=stop. Registered agents, unattributed managed work and corrupt
/// row ownership refuse without mutation; no PID is signalled. Reserved estimates
/// become released, never spent costs; a newer task attempt remains unchanged.
/// Replaying a prior unregistered cancellation preserves terminal bytes and repairs
/// missing task/budget/log settlement without duplicating completed effects.
/// Work/event, agent/log and reservation changes all roll back on refusal, including
/// silent SQL no-ops. Environment observations are metadata, not authenticated actor
/// identity. Caller supplies current task/validation policy and owns root authority.
pub fn cancel_queued_agent<E: crate::WorkEnvironment>(
    db: &mut Connection,
    id: &str,
    context: &crate::TaskLifecycleContext<'_>,
    environment: &mut E,
) -> Result<Agent, AgentStoreError> {
    cancel_queued_agent_with_settings(db, id, context, environment, None)
}
/// Root-only scoped cancellation verifies automatic intentions through historical
/// canonical provenance; revoked consent does not block cancelling an unstarted
/// proven intent. Registered/unknown physical ownership remains ineligible.
/// It uses the actual saved settings for lifecycle
/// review/dependency contracts. None preserves the public manual-only adapter.
/// No registered process can enter; atomic task/event/scope signal/reservation
/// and Agent/log readbacks remain mandatory. Original causes survive rollback.
/// Binding3 releases its exact FIFO generation in the same transaction; journal
/// failure preserves cancellation/reservation/turn state through rollback. A
/// terminal historical replay never releases a newer task generation.
pub(crate) fn cancel_queued_agent_with_settings<E: crate::WorkEnvironment>(
    db: &mut Connection,
    id: &str,
    context: &crate::TaskLifecycleContext<'_>,
    environment: &mut E,
    settings: Option<&crate::EngineSettingsDocument>,
) -> Result<Agent, AgentStoreError> {
    transaction(db, rusqlite::TransactionBehavior::Immediate, |tx| {
        let epoch = query(tx, "PRAGMA main.user_version", [], |rows| {
            Ok(rows
                .next()?
                .ok_or(AgentStoreError::Missing)?
                .get::<_, i64>(0)?)
        })?;
        if epoch != crate::RECORDED_COMMAND_SCHEMA_VERSION {
            return Err(AgentStoreError::Refused("storage_version_conflict"));
        }
        let (mut agent, before) = query(
            tx,
            "SELECT body,status,desired,work_id,task_id,cwd FROM main.agents WHERE id=?",
            [id],
            |rows| {
                let r = rows.next()?.ok_or(AgentStoreError::Missing)?;
                let agent = row(r)?;
                for (column, expected) in [
                    (3, agent.work_id.as_str()),
                    (4, agent.task_id.as_str()),
                    (5, agent.workspace.as_str()),
                ] {
                    if crate::sqlite_value::string(r.get_ref(column)?, column)?
                        != expected.as_bytes()
                    {
                        return Err(AgentStoreError::Refused("agent-row-identity-mismatch"));
                    }
                }
                if agent.id != id {
                    return Err(AgentStoreError::Refused("agent-row-identity-mismatch"));
                }
                let before = match r.get_ref(0)? {
                    ValueRef::Blob(raw) | ValueRef::Text(raw) => raw.to_vec(),
                    other => {
                        return Err(rusqlite::Error::InvalidColumnType(
                            0,
                            "agent body".into(),
                            other.data_type(),
                        )
                        .into())
                    }
                };
                Ok((agent, before))
            },
        )?;
        let already_cancelled = agent.status == "interrupted"
            && agent.stop_kind == "operateur"
            && agent.desired == "stop"
            && !agent.ended.is_empty();
        let cancellable = agent.status == "queued"
            || already_cancelled
            || (agent.has_unregistered_claim() && agent.desired == "stop");
        if !cancellable
            || agent.supervisor_pid != 0
            || agent.child_pid != 0
            || !agent.heartbeat.is_empty()
            || !agent.supervisor_identity.is_empty()
            || !agent.child_identity.is_empty()
        {
            return Err(AgentStoreError::Refused(
                "agent-reconcile-registered-unported",
            ));
        }
        let mut work = crate::read_recorded_work(tx, &agent.work_id)
            .map_err(|e| AgentStoreError::Work(Box::new(e)))?;
        #[cfg(target_os="linux")]
        let verified_binding = if agent.origin=="conducteur" || agent.native_mission.is_some() {
            let current=settings.ok_or(AgentStoreError::Refused("agent-mission-cancellation-settings-required"))?;
            crate::root_store::mission_agent::inspect(tx,&agent,current,crate::root_store::mission_agent::Check::Original)
                .map_err(|e|AgentStoreError::MissionProvenance(Box::new(e)))?
        } else {None};
        #[cfg(target_os="linux")]
        let verified_mission = verified_binding.is_some();
        #[cfg(not(target_os="linux"))]
        let verified_mission=false;
        if (work.planning.is_some() && settings.is_none())
            || (agent.origin == "conducteur" && !verified_mission)
            || agent.delivery_version != 0
        {
            return Err(AgentStoreError::Refused(
                "agent-managed-cancellation-unported",
            ));
        }
        if work.planning.as_deref().is_some_and(|p|p.repository.is_some()) {
            #[cfg(target_os="linux")]
            crate::managed_repository::verify_worker(context.root,tx,&work,&agent,settings.ok_or(AgentStoreError::Refused("agent-managed-cancellation-settings-required"))?,crate::managed_repository::WorkerCheck::Cancel)
                .map_err(|e|AgentStoreError::ScopeSignal(Box::new(e)))?;
            #[cfg(not(target_os="linux"))]
            return Err(AgentStoreError::Refused("agent-managed-cancellation-unported"));
        }
        let index = work
            .tasks
            .as_ref()
            .and_then(|tasks| tasks.iter().position(|t| t.id == agent.task_id))
            .ok_or(AgentStoreError::Refused("agent-task-missing"))?;
        let task = &work
            .tasks
            .as_ref()
            .ok_or(AgentStoreError::Refused("agent-task-missing"))?[index];
        let settle = task.status == "running"
            && task
                .attempts
                .as_ref()
                .and_then(|a| a.last())
                .is_some_and(|a| a.id == agent.attempt_id);
        let mut expected_reservation = reservation_snapshot(tx, id)?;
        if let Some(reservation) = expected_reservation.as_mut() {
            if reservation.work != agent.work_id {
                return Err(AgentStoreError::Refused("agent-reservation-scope-mismatch"));
            }
            if reservation.state == "reserved" {
                reservation.state = "released".into();
            }
        }
        let at = environment.timestamp().map_err(AgentStoreError::Runtime)?;
        swarm_wire_json::go_rfc3339::parse(&at).map_err(AgentStoreError::Clock)?;
        if settle {
            let event = environment
                .identifier("finish-")
                .map_err(AgentStoreError::Runtime)?;
            let request = crate::Request {
                schema_version: 1,
                event_id: event.clone(),
                expected_revision: work.revision,
                id: agent.task_id.clone(),
                status: "blocked".into(),
                origin: "operateur".into(),
                outcome: "interrupted".into(),
                blocker: "Démarrage annulé avant prise en charge ; handoff et validation requis"
                    .into(),
                next: "Examiner les preuves puis reprendre explicitement".into(),
                ..Default::default()
            };
            let next = if work.planning.is_some() {
                crate::transition_scoped_worker_task(
                    &work,
                    &request,
                    context,
                    settings.ok_or(AgentStoreError::Refused(
                        "agent-cancellation-settings-required",
                    ))?,
                    environment,
                )
            } else {
                crate::transition_manual_task(&work, &request, context, environment)
            }
            .map_err(|e| AgentStoreError::Transition(Box::new(e)))?;
            work.tasks
                .as_mut()
                .ok_or(AgentStoreError::Refused("agent-task-missing"))?[index] = next;
            work.revision = work
                .revision
                .checked_add(1)
                .ok_or(AgentStoreError::Refused("agent-work-revision-overflow"))?;
            work.updated = at.clone();
            crate::agent_scope_signals::ended(
                &mut work,
                &agent.task_id,
                &agent.attempt_id,
                "interrupted",
                &at,
            )
            .map_err(|e| AgentStoreError::ScopeSignal(Box::new(e)))?;
            let intent = crate::encode_model(&request).map_err(AgentStoreError::Encode)?;
            let staged = crate::stage_recorded_change(
                tx,
                crate::RecordedChange {
                    work: &work,
                    expected_revision: request.expected_revision,
                    event: &event,
                    kind: "task.update",
                    request: &intent,
                },
            )
            .map_err(|e| AgentStoreError::Change(Box::new(e)))?;
            if !matches!(staged, crate::RecordedChangeOutcome::Staged) {
                return Err(AgentStoreError::Refused(
                    "agent-cancellation-event-conflict",
                ));
            }
        }
        let previous_status = agent.status.clone();
        let desired = agent.desired.clone();
        let encoded = if already_cancelled {
            before.clone()
        } else {
            agent.status = "interrupted".into();
            agent.stop_kind = "operateur".into();
            agent.desired = "stop".into();
            agent.activity = "Démarrage annulé avant prise en charge — reprise possible".into();
            agent.ended = at.clone();
            let encoded = crate::encode_model(&agent).map_err(AgentStoreError::Encode)?;
            let changed=tx.execute("UPDATE main.agents SET status='interrupted',body=?,desired='stop' WHERE id=? AND status=? AND desired=? AND CAST(body AS BLOB)=?",rusqlite::params![encoded,id,previous_status,desired,before])?;
            if changed != 1 {
                return Err(AgentStoreError::Refused("agent-cancellation-not-recorded"));
            }
            encoded
        };
        #[cfg(target_os="linux")]
        crate::root_store::mission_agent::release_workspace(tx, &agent, verified_binding.as_ref(), &at)
            .map_err(|cause| AgentStoreError::MissionProvenance(Box::new(cause)))?;
        tx.execute(
            "UPDATE main.reservations SET state='released' WHERE agent_id=? AND state='reserved'",
            [id],
        )?;
        if reservation_snapshot(tx, id)? != expected_reservation {
            return Err(AgentStoreError::Refused(
                "agent-reservation-release-not-preserved",
            ));
        }
        let prior_log=query(tx,"SELECT seq FROM main.agent_logs WHERE agent_id=? AND kind='lifecycle' AND message=? ORDER BY seq",rusqlite::params![id,agent.activity],|rows|Ok(rows.next()?.map(|r|r.get::<_,i64>(0)).transpose()?))?;
        if !already_cancelled || prior_log.is_none() {
            let sequence:i64 = query(tx,
            "INSERT INTO main.agent_logs(agent_id,at,kind,message) VALUES(?,?,'lifecycle',?) RETURNING seq",
            rusqlite::params![id, at, agent.activity],|rows|Ok(rows.next()?.ok_or(AgentStoreError::Refused("agent-cancellation-log-not-recorded"))?.get(0)?))?;
            let log_valid = query(
                tx,
                "SELECT at,kind,message FROM main.agent_logs WHERE seq=? AND agent_id=?",
                rusqlite::params![sequence, id],
                |rows| {
                    let Some(row) = rows.next()? else {
                        return Ok(false);
                    };
                    Ok(row.get::<_, String>(0)? == at
                        && row.get::<_, String>(1)? == "lifecycle"
                        && row.get::<_, String>(2)? == agent.activity)
                },
            )?;
            if !log_valid {
                return Err(AgentStoreError::Refused(
                    "agent-cancellation-log-not-recorded",
                ));
            }
        }
        let stored = read_agent(tx, id)?;
        let raw: Vec<u8> = query(
            tx,
            "SELECT CAST(body AS BLOB) FROM main.agents WHERE id=?",
            [id],
            |rows| Ok(rows.next()?.ok_or(AgentStoreError::Missing)?.get(0)?),
        )?;
        if raw != encoded || stored.status != "interrupted" || stored.desired != "stop" {
            return Err(AgentStoreError::Refused(
                "agent-cancellation-state-not-recorded",
            ));
        }
        let observed_work = crate::read_recorded_work(tx, &agent.work_id)
            .map_err(|e| AgentStoreError::Work(Box::new(e)))?;
        if observed_work != work {
            return Err(AgentStoreError::Refused(
                "agent-cancellation-work-not-preserved",
            ));
        }
        verify_agent_ownership(tx, &stored)?;
        if work.planning.as_deref().is_some_and(|p|p.repository.is_some()) {
            #[cfg(target_os="linux")]
            crate::managed_repository::verify_worker(context.root,tx,&work,&stored,settings.ok_or(AgentStoreError::Refused("agent-managed-cancellation-settings-required"))?,crate::managed_repository::WorkerCheck::Cancel)
                .map_err(|e|AgentStoreError::ScopeSignal(Box::new(e)))?;
        }
        Ok(stored)
    })
}
