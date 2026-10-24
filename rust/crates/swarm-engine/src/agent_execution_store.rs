//! Caller-owned native startup transitions. No gate byte or provider is executed.
#![cfg(target_os = "linux")]
use crate::{
    agent_execution_preflight::{inspect, ExecutionPreparation, ExecutionStage},
    agent_launch::query,
    agent_launch_error::finish,
    local_storage::LocalDirectory,
};
use crate::{
    Agent, AgentExecutionPhase, AgentExecutionSnapshot, AgentLaunchError, EngineSettingsDocument,
};
use rusqlite::{Connection, Transaction};
use std::path::Path;
/// Exact existing reservation receipt, including explicit absence. This is an
/// internal observation; no caller-provided amount creates a budget allocation.
pub(crate) type ReservationReceipt = Option<(String, f64, String)>;
/// Read and finalize one existing estimate. Missing can be legitimate only when
/// the verified launch had no positive envelope; consumers compare that receipt.
pub(crate) fn reservation(
    db: &Connection,
    id: &str,
) -> Result<ReservationReceipt, AgentLaunchError> {
    query(
        db,
        "SELECT work_id,amount,state FROM main.reservations WHERE agent_id=?",
        [id],
        |rows| {
            Ok(rows
                .next()?
                .map(|r| {
                    Ok::<_, rusqlite::Error>((
                        r.get::<_, String>(0)?,
                        r.get::<_, f64>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })
                .transpose()?)
        },
    )
}
/// Internal log family is an event invariant, never an operator stop policy.
#[derive(Clone, Copy)]
enum LogFamily {
    Lifecycle,
    Incident,
}
/// Apply the frozen UTF-8 detail cap, sanitize terminal text and verify the exact
/// inserted log after RETURNING. Zero omits detail but leaves the incident/state
/// in Agent metadata. SQL/codec/close failures retain causes and refuse the tx.
fn append_log(
    tx: &Transaction<'_>,
    agent: &Agent,
    family: LogFamily,
    message: &str,
) -> Result<(), AgentLaunchError> {
    let cap = crate::decode_agent_execution_snapshot(
        agent
            .native_execution
            .as_ref()
            .ok_or(AgentLaunchError::Refused(
                "agent-execution-snapshot-missing",
            ))?
            .0
            .get()
            .as_bytes(),
    )
    .map_err(|e| AgentLaunchError::caused(e.code(), e))?
    .policy
    .log_message_max_bytes;
    let mut detail = crate::agent_terminal_text::terminal_text(message);
    let mut truncated = false;
    if let Some(cap) = cap.0 {
        let cap = usize::try_from(cap.0)
            .map_err(|e| AgentLaunchError::caused("agent-execution-log-representation", e))?;
        if detail.len() > cap {
            let mut end = cap;
            while !detail.is_char_boundary(end) {
                end -= 1;
            }
            detail.truncate(end);
            truncated = true;
        }
    }
    let kind = match (family, truncated) {
        (LogFamily::Lifecycle, false) => "lifecycle",
        (LogFamily::Lifecycle, true) => "lifecycle-truncated",
        (LogFamily::Incident, false) => "incident",
        (LogFamily::Incident, true) => "incident-truncated",
    };
    let sequence: i64 = query(
        tx,
        "INSERT INTO main.agent_logs(agent_id,at,kind,message) VALUES(?,?,?,?) RETURNING seq",
        rusqlite::params![agent.id, agent.heartbeat, kind, detail],
        |rows| {
            Ok(rows
                .next()?
                .ok_or(AgentLaunchError::Refused(
                    "agent-execution-log-not-recorded",
                ))?
                .get(0)?)
        },
    )?;
    let valid = query(
        tx,
        "SELECT at,kind,message FROM main.agent_logs WHERE seq=? AND agent_id=?",
        rusqlite::params![sequence, agent.id],
        |rows| match rows.next()? {
            Some(row) => Ok(row.get::<_, String>(0)? == agent.heartbeat
                && row.get::<_, String>(1)? == kind
                && row.get::<_, String>(2)? == detail),
            None => Ok(false),
        },
    )?;
    if !valid {
        return Err(AgentLaunchError::Refused(
            "agent-execution-log-not-preserved",
        ));
    }
    Ok(())
}
/// Own a short immediate lifecycle transaction and preserve every rollback or
/// uncertain-commit cause. No commit outcome is retried by this function. The
/// caller must use saved storage policy and release its settings reservation.
pub(crate) fn transaction<T>(
    db: &mut Connection,
    operation: impl FnOnce(&mut Transaction<'_>) -> Result<T, AgentLaunchError>,
) -> Result<T, AgentLaunchError> {
    let mut tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let result = operation(&mut tx);
    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
    match result {
        Err(primary) => finish(
            Err(primary),
            tx.rollback()
                .map_err(|e| AgentLaunchError::caused("agent-execution-rollback", e)),
        ),
        Ok(value) => match tx.commit() {
            Ok(()) => Ok(value),
            Err(cause) => {
                let active = !db.is_autocommit();
                let cleanup = if active {
                    db.execute_batch("ROLLBACK")
                        .err()
                        .map(|e| Box::new(AgentLaunchError::caused("agent-execution-rollback", e)))
                } else {
                    None
                };
                Err(AgentLaunchError::Commit {
                    cause: Box::new(AgentLaunchError::caused("agent-execution-commit", cause)),
                    rollback_confirmed: active && cleanup.is_none() && db.is_autocommit(),
                    cleanup,
                })
            }
        },
    }
}
/// Persist complete Agent bytes by exact prior-row CAS, not a supplied identity.
/// The final read after log insertion detects ignored/reverted writes. Work must
/// remain byte-contract equivalent and the reserved budget must remain valid.
/// No new attempt, budget allocation, gate release or task acceptance is created.
fn save(
    tx: &Transaction<'_>,
    prepared: &ExecutionPreparation,
    agent: &Agent,
    message: &str,
) -> Result<(), AgentLaunchError> {
    let encoded = crate::encode_model(agent)
        .map_err(|e| AgentLaunchError::caused("agent-execution-encoding", e))?;
    let changed=tx.execute("UPDATE main.agents SET status=?,desired=?,body=? WHERE id=? AND status=? AND desired=? AND CAST(body AS BLOB)=?",rusqlite::params![agent.status,agent.desired,encoded,agent.id,prepared.agent.status,prepared.agent.desired,prepared.body])?;
    if changed != 1 {
        return Err(AgentLaunchError::Refused(
            "agent-execution-state-not-recorded",
        ));
    }
    append_log(tx, agent, LogFamily::Lifecycle, message)?;
    let (actual, _, raw) = crate::agent_execution_preflight::retained(tx, &agent.id)?;
    if actual != *agent
        || raw != encoded
        || crate::read_recorded_work(tx, &agent.work_id)
            .map_err(|e| AgentLaunchError::caused("agent-execution-work-readback", e))?
            != prepared.work
    {
        return Err(AgentLaunchError::Refused(
            "agent-execution-effects-not-preserved",
        ));
    }
    crate::check_reserved_agent_budget(tx, &agent.work_id, &agent.id)
        .map_err(|e| AgentLaunchError::caused(e.code(), e))
}
/// Bind an actual held child PID to this current registered supervisor. Kernel
/// parent/group/starttime must agree before recording awaiting_permission and
/// the selected v18 settings revision. The helper must still be blocked; this
/// operation neither releases its input nor claims the provider has started.
/// All metadata/log/Work/budget refusals roll back; commit uncertainty stays open.
pub(crate) fn register_child(
    db: &mut Connection,
    root: &Path,
    dir: &LocalDirectory,
    settings: &EngineSettingsDocument,
    expected: &Agent,
    pid: u32,
    reserved: &ReservationReceipt,
) -> Result<Agent, AgentLaunchError> {
    let id = &expected.id;
    transaction(db, |tx| {
        let prepared = inspect(tx, root, dir, settings, id, ExecutionStage::Supervisor)?;
        if reservation(tx, id)? != *reserved {
            return Err(AgentLaunchError::Refused(
                "agent-execution-reservation-changed",
            ));
        }
        if prepared.agent != *expected {
            return Err(AgentLaunchError::Refused("agent-execution-claim-changed"));
        }
        let child = crate::observe_linux_process(pid)
            .map_err(|e| AgentLaunchError::caused("agent-execution-child-identity", e))?;
        if child.parent_pid() != std::process::id() || child.process_group_id() != pid {
            return Err(AgentLaunchError::Refused("agent-execution-child-not-owned"));
        }
        let policy = settings
            .values
            .agent_execution_policy()
            .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
        let context = prepared
            .agent
            .context
            .as_ref()
            .ok_or(AgentLaunchError::Refused("agent-execution-context-missing"))?;
        let snapshot = AgentExecutionSnapshot {
            schema_version: 1,
            settings_revision: settings.revision,
            work_revision: prepared.work.revision,
            context_sha256: context.sha256.clone(),
            phase: AgentExecutionPhase::AwaitingPermission,
            policy,
        };
        snapshot
            .validate()
            .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
        let mut agent = prepared.agent.clone();
        agent.child_pid = i64::from(pid);
        agent.child_identity = child.process_stamp().into();
        agent.native_execution = Some(crate::GoRaw(
            serde_json::value::to_raw_value(&snapshot)
                .map_err(|e| AgentLaunchError::caused("agent-execution-snapshot-encoding", e))?,
        ));
        agent.heartbeat = crate::system_timestamp()
            .map_err(|e| AgentLaunchError::caused("agent-execution-clock", e))?;
        agent.activity = "Enfant moteur enregistré ; fournisseur non libéré".into();
        save(tx, &prepared, &agent, &agent.activity)?;
        Ok(agent)
    })
}
/// Record startup permission only after fresh parent/child/context/provider/
/// budget/stop checks and matching saved policy revision. This is the intended
/// linearization point, not proof of actual exec. The owned caller may release
/// its private input gate only after confirmed commit; uncertainty never grants.
/// Capture immutable directory/connected-DB provenance in this same writer,
/// consumed by the child gate and later manual-origin automatic review.
pub(crate) fn record_permission(
    db: &mut Connection,
    root: &Path,
    dir: &LocalDirectory,
    settings: &EngineSettingsDocument,
    expected: &Agent,
    reserved: &ReservationReceipt,
) -> Result<Agent, AgentLaunchError> {
    let id = &expected.id;
    transaction(db, |tx| {
        let prepared = inspect(tx, root, dir, settings, id, ExecutionStage::Registered)?;
        if reservation(tx, id)? != *reserved {
            return Err(AgentLaunchError::Refused(
                "agent-execution-reservation-changed",
            ));
        }
        if prepared.agent != *expected {
            return Err(AgentLaunchError::Refused(
                "agent-execution-registration-changed",
            ));
        }
        let mut agent = prepared.agent.clone();
        let mut snapshot = crate::decode_agent_execution_snapshot(
            agent
                .native_execution
                .as_ref()
                .ok_or(AgentLaunchError::Refused(
                    "agent-execution-snapshot-missing",
                ))?
                .0
                .get()
                .as_bytes(),
        )
        .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
        if snapshot.settings_revision != settings.revision
            || snapshot.policy
                != settings
                    .values
                    .agent_execution_policy()
                    .map_err(|e| AgentLaunchError::caused(e.reason(), e))?
        {
            return Err(AgentLaunchError::Refused(
                "agent-execution-settings-changed",
            ));
        }
        crate::root_store::agent_execution_origin::stage(tx,dir,id)?;
        snapshot.phase = AgentExecutionPhase::PermissionRecorded;
        agent.native_execution = Some(crate::GoRaw(
            serde_json::value::to_raw_value(&snapshot)
                .map_err(|e| AgentLaunchError::caused("agent-execution-snapshot-encoding", e))?,
        ));
        agent.heartbeat = crate::system_timestamp()
            .map_err(|e| AgentLaunchError::caused("agent-execution-clock", e))?;
        agent.activity = "Départ autorisé ; exécution fournisseur à confirmer".into();
        save(tx, &prepared, &agent, &agent.activity)?;
        Ok(agent)
    })
}

/// Read the current owned lifecycle and raw CAS bytes in a coherent transaction.
/// Only a separately persisted stop intention may differ from our last receipt.
/// Kernel parent identity and schema epoch are checked; numeric child metadata
/// is not reinterpreted as a new signal capability. Caller holds the owned child.
fn owned(tx: &Transaction<'_>, expected: &Agent) -> Result<(Agent, Vec<u8>), AgentLaunchError> {
    let epoch: i64 = query(tx, "PRAGMA main.user_version", [], |rows| {
        Ok(rows
            .next()?
            .ok_or(AgentLaunchError::Refused("agent-execution-schema-missing"))?
            .get(0)?)
    })?;
    if epoch != crate::RECORDED_COMMAND_SCHEMA_VERSION {
        return Err(AgentLaunchError::Refused("storage_version_conflict"));
    }
    let current =
        crate::read_agent(tx, &expected.id).map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    crate::agent_store::verify_agent_ownership(tx, &current)
        .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    let identity = crate::SupervisorIdentity::current()
        .map_err(|e| AgentLaunchError::caused("agent-execution-supervisor-identity", e))?;
    let mut comparable = current.clone();
    comparable.desired = expected.desired.clone();
    if comparable != *expected
        || !matches!(current.desired.as_str(), "" | "stop")
        || !matches!(current.status.as_str(), "starting" | "running")
        || current.supervisor_pid != identity.pid()
        || current.supervisor_identity != identity.process_stamp()
        || current.host != identity.host()
        || current.native_execution_result.is_some()
    {
        return Err(AgentLaunchError::Refused(
            "agent-execution-owned-state-changed",
        ));
    }
    let raw = query(
        tx,
        "SELECT CAST(body AS BLOB) FROM main.agents WHERE id=?",
        [&current.id],
        |rows| {
            Ok(rows
                .next()?
                .ok_or(AgentLaunchError::Refused("agent-execution-missing"))?
                .get::<_, Vec<u8>>(0)?)
        },
    )?;
    Ok((current, raw))
}
/// Observe the current owned Agent and stop intention without taking a writer
/// transaction. Reuses exact schema/body/ownership/process checks in one DEFERRED
/// snapshot, explicitly rolls back, and retains primary plus rollback errors.
/// No heartbeat, progress, lifecycle log, permission or budget is changed. Used
/// while the helper opens Root with the deliberate zero-wait bootstrap policy.
/// This neither proves provider startup nor authenticates an Agent.
pub(crate) fn pending_gate_owner(
    db: &mut Connection,
    expected: &Agent,
) -> Result<Agent, AgentLaunchError> {
    let mut tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)?;
    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
    let result = owned(&tx, expected).map(|(agent, _raw)| agent);
    finish(result, tx.rollback().map_err(|error| AgentLaunchError::caused("agent-execution-read-rollback", error)))
}
/// Replace only the verified owned row, preserving stop races in the transaction.
/// Exact body/status/desired readback detects trigger no-ops and rewrites. This
/// helper does not settle a task/reservation or create a lifecycle log by itself.
fn replace(
    tx: &Transaction<'_>,
    before: &Agent,
    raw: &[u8],
    after: &Agent,
) -> Result<(), AgentLaunchError> {
    let bytes = crate::encode_model(after)
        .map_err(|e| AgentLaunchError::caused("agent-execution-encoding", e))?;
    let count=tx.execute("UPDATE main.agents SET status=?,desired=?,body=? WHERE id=? AND status=? AND desired=? AND CAST(body AS BLOB)=?",rusqlite::params![after.status,after.desired,bytes,after.id,before.status,before.desired,raw])?;
    let actual =
        crate::read_agent(tx, &after.id).map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    let actual_raw = query(
        tx,
        "SELECT CAST(body AS BLOB) FROM main.agents WHERE id=?",
        [&after.id],
        |rows| {
            Ok(rows
                .next()?
                .ok_or(AgentLaunchError::Refused("agent-execution-missing"))?
                .get::<_, Vec<u8>>(0)?)
        },
    )?;
    if count != 1 || actual != *after || actual_raw != bytes {
        return Err(AgentLaunchError::Refused(
            "agent-execution-state-not-preserved",
        ));
    }
    Ok(())
}
/// Apply only an engine-owned observation projection, preserving codec causes.
/// Missing/invalid/incomplete channels clear canonical fields; diagnostic history
/// stays explicit in opaque metadata. This never prices, authenticates or grants.
fn provider_facts(
    agent: &mut Agent,
    facts: &crate::agent_provider_facts::ProviderFactsSnapshot,
) -> Result<(), AgentLaunchError> {
    let raw = serde_json::value::to_raw_value(facts)
        .map_err(|e| AgentLaunchError::caused("agent-provider-facts-encoding", e))?;
    agent.usage = facts.usage.clone().map(Box::new);
    agent.reported_model = facts.reported_model.clone().map(Box::new);
    agent.native_provider_facts = Some(crate::GoRaw(raw));
    Ok(())
}
/// Persist cumulative progress, declared metadata and the real supervisor heartbeat. Provider
/// vitality stays unknown; this does not assert successful exec or acceptance.
/// A current stop intention is retained, never overwritten by stale memory.
pub(crate) fn pulse(
    db: &mut Connection,
    expected: &Agent,
    progress: &crate::AgentProgress,
    facts: &crate::agent_provider_facts::ProviderFactsSnapshot,
) -> Result<Agent, AgentLaunchError> {
    transaction(db, |tx| {
        let (before, raw) = owned(tx, expected)?;
        let mut a = before.clone();
        a.progress = progress.clone();
        provider_facts(&mut a, facts)?;
        a.status = "running".into();
        a.heartbeat = crate::system_timestamp()
            .map_err(|e| AgentLaunchError::caused("agent-execution-clock", e))?;
        a.activity = if a.desired == "stop" {
            "Arrêt opérateur demandé ; terminaison à confirmer"
        } else {
            "Supervision native active ; vitalité fournisseur non établie"
        }
        .into();
        replace(tx, &before, &raw, &a)?;
        Ok(a)
    })
}
/// Engine-owned wait/stream facts, never deserialized from a public request.
/// Recorded permission conservatively consumes the reservation estimate unless
/// the owned private channel explicitly reported a validated pre-exec refusal.
pub(crate) struct Completion<'a> {
    pub exit_code: Option<i64>,
    pub failed: bool,
    pub interrupted: bool,
    pub stop_kind: Option<&'a str>,
    pub reserved: &'a ReservationReceipt,
    pub may_have_started: bool,
    pub progress: &'a crate::AgentProgress,
    pub details: &'a crate::GoRaw,
    pub public_reply: &'a str,
    pub facts: &'a crate::agent_completion_journal::CompletionFacts,
}
/// Settle an actually reaped manual process and only its matching task attempt.
/// Task remains blocked for handoff/review; newer attempts remain untouched.
/// Agent, Work event, lifecycle log and existing reservation change atomically.
/// Scoped attempts also deliver a terminal signal to their owning scope; a closed
/// scope retains its state. No supervisor decision or task acceptance occurs.
/// No measured price/zero usage/refund is invented. Original SQL, environment,
/// transition, rollback and uncertain-commit causes survive. Caller must retain
/// unresolved owned processes and must not call this on unknown termination.
pub(crate) fn complete(
    db: &mut Connection,
    root: &Path,
    settings: &EngineSettingsDocument,
    expected: &Agent,
    receipt: &Completion<'_>,
) -> Result<Agent, AgentLaunchError> {
    complete_with_owner(db, root, settings, expected, receipt, false)
}
/// Replay only a durable receipt whose original owner is no longer live. The
/// caller validates journal binding; transaction rechecks the current Agent and
/// reservation. No provider process, OS signal or acceptance is produced;
/// scoped terminal notification uses the same atomic settlement as the live owner.
pub(crate) fn complete_recovered(
    db: &mut Connection,
    root: &Path,
    settings: &EngineSettingsDocument,
    expected: &Agent,
    receipt: &Completion<'_>,
) -> Result<Agent, AgentLaunchError> {
    complete_with_owner(db, root, settings, expected, receipt, true)
}
/// Share identical atomic settlement semantics across live owner and durable
/// replay. Recovery changes only the owner proof, never task/budget validation.
/// Automatic rows additionally require their historical canonical mission proof;
/// failure rolls back settlement with its typed cause. Current consent is not
/// required after grant, and no completion implies task acceptance.
/// Binding3 releases its exact owned FIFO generation in this same settlement;
/// a journal failure rolls back SQL, not already observed provider effects.
/// Uncertain/quarantined execution uses its separate retained-ownership path.
fn complete_with_owner(
    db: &mut Connection,
    root: &Path,
    settings: &EngineSettingsDocument,
    expected: &Agent,
    receipt: &Completion<'_>,
    recovering: bool,
) -> Result<Agent, AgentLaunchError> {
    transaction(db, |tx| {
        let (before, raw) = if recovering {
            crate::agent_completion_journal::recovery_owner(tx, expected)?
        } else {
            owned(tx, expected)?
        };
        let provenance=crate::root_store::mission_agent::inspect(tx,&before,settings,crate::root_store::mission_agent::Check::Original)?;
        let mut a = before.clone();
        let interrupted = receipt.interrupted || a.desired == "stop";
        let outcome = if interrupted {
            "interrupted"
        } else if receipt.failed {
            "failed"
        } else {
            "completed"
        };
        let mut work = crate::read_recorded_work(tx, &a.work_id)
            .map_err(|e| AgentLaunchError::caused("agent-execution-work-read", e))?;
        if work
            .planning
            .as_ref()
            .is_some_and(|p| p.repository.is_some())
        {
            return Err(AgentLaunchError::Refused(
                "agent-execution-managed-settlement-unported",
            ));
        }
        let index = work
            .tasks
            .as_ref()
            .and_then(|v| v.iter().position(|t| t.id == a.task_id))
            .ok_or(AgentLaunchError::Refused("agent-execution-task-missing"))?;
        let task = &work
            .tasks
            .as_ref()
            .ok_or(AgentLaunchError::Refused("agent-execution-task-missing"))?[index];
        let settle = task.status == "running"
            && task
                .attempts
                .as_ref()
                .and_then(|v| v.last())
                .is_some_and(|t| t.id == a.attempt_id);
        let at = crate::system_timestamp()
            .map_err(|e| AgentLaunchError::caused("agent-execution-clock", e))?;
        if settle {
            let limits = settings
                .values
                .mutation_limits()
                .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
            let context = crate::TaskLifecycleContext {
                root,
                quality_policy: None,
                task_limits: &limits.task,
                validation_limits: &limits.policy,
            };
            let event = crate::generate_identifier("finish-")
                .map_err(|e| AgentLaunchError::caused("agent-execution-event-id", e))?;
            let request = crate::Request {
                schema_version: 1,
                event_id: event.clone(),
                expected_revision: work.revision,
                id: a.task_id.clone(),
                status: "blocked".into(),
                origin: if provenance.is_some() {"conducteur"} else {"operateur"}.into(),
                outcome: outcome.into(),
                blocker: "Processus terminé ; handoff, preuves et validation requis".into(),
                next: "Examiner les preuves puis reprendre explicitement".into(),
                ..Default::default()
            };
            let next = if work.planning.is_some() {
                crate::transition_scoped_worker_task(
                    &work,
                    &request,
                    &context,
                    settings,
                    &mut crate::SystemEnvironment,
                )
            } else {
                crate::transition_manual_task(
                    &work,
                    &request,
                    &context,
                    &mut crate::SystemEnvironment,
                )
            }
            .map_err(|e| AgentLaunchError::caused("agent-execution-task-transition", e))?;
            work.tasks
                .as_mut()
                .ok_or(AgentLaunchError::Refused("agent-execution-task-missing"))?[index] = next;
            work.revision = work
                .revision
                .checked_add(1)
                .ok_or(AgentLaunchError::Refused(
                    "agent-execution-work-revision-overflow",
                ))?;
            work.updated = at.clone();
            crate::agent_scope_signals::ended(&mut work, &a.task_id, &a.attempt_id, outcome, &at)?;
            let request_bytes = crate::encode_model(&request)
                .map_err(|e| AgentLaunchError::caused("agent-execution-event-encoding", e))?;
            let staged = crate::stage_recorded_change(
                tx,
                crate::RecordedChange {
                    work: &work,
                    expected_revision: request.expected_revision,
                    event: &event,
                    kind: "task.update",
                    request: &request_bytes,
                },
            )
            .map_err(|e| AgentLaunchError::caused("agent-execution-event-recording", e))?;
            if !matches!(staged, crate::RecordedChangeOutcome::Staged) {
                return Err(AgentLaunchError::Refused("agent-execution-event-conflict"));
            }
        }
        let reservation = query(
            tx,
            "SELECT work_id,amount,state FROM main.reservations WHERE agent_id=?",
            [&a.id],
            |rows| {
                Ok(rows
                    .next()?
                    .map(|r| {
                        Ok::<_, rusqlite::Error>((
                            r.get::<_, String>(0)?,
                            r.get::<_, f64>(1)?,
                            r.get::<_, String>(2)?,
                        ))
                    })
                    .transpose()?)
            },
        )?;
        if reservation != *receipt.reserved {
            return Err(AgentLaunchError::Refused(
                "agent-execution-reservation-changed",
            ));
        }
        let mut expected_reservation = reservation.clone();
        if let Some((scope, amount, state)) = expected_reservation.as_mut() {
            if *scope != a.work_id || !amount.is_finite() || *amount < 0.0 {
                return Err(AgentLaunchError::Refused(
                    "agent-execution-reservation-invalid",
                ));
            }
            if state == "reserved" {
                *state = if receipt.may_have_started {
                    "estimated"
                } else {
                    "released"
                }
                .into();
                let changed=tx.execute("UPDATE main.reservations SET state=? WHERE agent_id=? AND work_id=? AND state='reserved' AND amount=?",rusqlite::params![state.as_str(),a.id,scope.as_str(),*amount])?;
                if changed != 1 {
                    return Err(AgentLaunchError::Refused(
                        "agent-execution-reservation-not-recorded",
                    ));
                }
            }
        }
        a.status = outcome.into();
        a.exit_code = receipt.exit_code.map(Box::new);
        a.ended = at.clone();
        a.heartbeat = at.clone();
        a.progress = receipt.progress.clone();
        a.reply = receipt.public_reply.into();
        a.native_execution_result = Some(receipt.details.clone());
        receipt.facts.apply(&mut a);
        a.activity = "Processus récolté ; tâche non acceptée, validation requise".into();
        if interrupted {
            a.stop_kind = if before.desired == "stop" {
                "operateur"
            } else {
                receipt.stop_kind.ok_or(AgentLaunchError::Refused(
                    "agent-execution-stop-cause-missing",
                ))?
            }
            .into();
        }
        crate::root_store::mission_agent::release_workspace(tx, &a, provenance.as_ref(), &at)?;
        replace(tx, &before, &raw, &a)?;
        append_log(tx, &a, LogFamily::Lifecycle, &a.activity)?;
        let final_res = query(
            tx,
            "SELECT work_id,amount,state FROM main.reservations WHERE agent_id=?",
            [&a.id],
            |rows| {
                Ok(rows
                    .next()?
                    .map(|r| {
                        Ok::<_, rusqlite::Error>((
                            r.get::<_, String>(0)?,
                            r.get::<_, f64>(1)?,
                            r.get::<_, String>(2)?,
                        ))
                    })
                    .transpose()?)
            },
        )?;
        let final_raw = query(
            tx,
            "SELECT CAST(body AS BLOB) FROM main.agents WHERE id=?",
            [&a.id],
            |rows| {
                Ok(rows
                    .next()?
                    .ok_or(AgentLaunchError::Refused("agent-execution-missing"))?
                    .get::<_, Vec<u8>>(0)?)
            },
        )?;
        let expected_raw = crate::encode_model(&a)
            .map_err(|e| AgentLaunchError::caused("agent-execution-encoding", e))?;
        let final_agent =
            crate::read_agent(tx, &a.id).map_err(|e| AgentLaunchError::caused(e.code(), e))?;
        if final_raw != expected_raw
            || final_agent != a
            || final_res != expected_reservation
            || crate::read_recorded_work(tx, &a.work_id)
                .map_err(|e| AgentLaunchError::caused("agent-execution-work-readback", e))?
                != work
        {
            return Err(AgentLaunchError::Refused(
                "agent-execution-settlement-not-preserved",
            ));
        }
        crate::agent_store::verify_agent_ownership(tx, &a)
            .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
        Ok(a)
    })
}

/// Preserve an owned process failure with incomplete output as an active incident.
/// The leader may be reaped while a descendant still holds a stream; that is not
/// permission to release workspace or budget. No task/event/outcome/reservation
/// changes occur. Receipt is engine-owned, not a public cleanup assertion. All
/// body/log/Work/reservation effects are read back after the last write; failure
/// rolls back or retains commit uncertainty. Recovery is a separate proof flow.
pub(crate) fn quarantine(
    db: &mut Connection,
    expected: &Agent,
    receipt: &Completion<'_>,
) -> Result<Agent, AgentLaunchError> {
    transaction(db, |tx| {
        let (before, raw) = owned(tx, expected)?;
        let work = crate::read_recorded_work(tx, &before.work_id)
            .map_err(|e| AgentLaunchError::caused("agent-execution-work-read", e))?;
        if reservation(tx, &before.id)? != *receipt.reserved {
            return Err(AgentLaunchError::Refused(
                "agent-execution-reservation-changed",
            ));
        }
        let mut a = before.clone();
        a.status = "stopping".into();
        a.heartbeat = crate::system_timestamp()
            .map_err(|e| AgentLaunchError::caused("agent-execution-clock", e))?;
        a.activity =
            "Sortie incomplète ; workspace et réservation bloqués, preuve de reprise requise"
                .into();
        a.progress = receipt.progress.clone();
        a.native_execution_result = Some(receipt.details.clone());
        receipt.facts.apply(&mut a);
        replace(tx, &before, &raw, &a)?;
        append_log(tx, &a, LogFamily::Incident, &a.activity)?;
        let actual =
            crate::read_agent(tx, &a.id).map_err(|e| AgentLaunchError::caused(e.code(), e))?;
        let actual_raw = query(
            tx,
            "SELECT CAST(body AS BLOB) FROM main.agents WHERE id=?",
            [&a.id],
            |rows| {
                Ok(rows
                    .next()?
                    .ok_or(AgentLaunchError::Refused("agent-execution-missing"))?
                    .get::<_, Vec<u8>>(0)?)
            },
        )?;
        if actual != a
            || actual_raw
                != crate::encode_model(&a)
                    .map_err(|e| AgentLaunchError::caused("agent-execution-encoding", e))?
            || reservation(tx, &a.id)? != *receipt.reserved
            || crate::read_recorded_work(tx, &a.work_id)
                .map_err(|e| AgentLaunchError::caused("agent-execution-work-readback", e))?
                != work
        {
            return Err(AgentLaunchError::Refused(
                "agent-execution-quarantine-not-preserved",
            ));
        }
        crate::agent_store::verify_agent_ownership(tx, &a)
            .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
        Ok(a)
    })
}
