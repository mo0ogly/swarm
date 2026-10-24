//! Durable native terminal observations. Private local storage is a trust boundary,
//! not a cryptographic identity: a same-UID writer can alter files and the database.
#![cfg(target_os = "linux")]
use crate::{
    agent_execution_store as store, local_storage::LocalDirectory, Agent, AgentLaunchError, GoRaw,
    RootStoreError,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, path::Path};

/// Serializable observation projection built from the owned provider stream.
/// Replay preserves canonical absence and opaque history; no requested route,
/// authenticated model, invoice or new authorization is inferred.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompletionFacts {
    usage: Option<Box<crate::Usage>>,
    reported_model: Option<Box<crate::ReportedModel>>,
    raw: GoRaw,
}
impl CompletionFacts {
    /// Freeze actual stream facts before final persistence. Encoding failures
    /// retain their cause; this does not deserialize public Agent assertions.
    pub(crate) fn observe(
        facts: &crate::agent_provider_facts::ProviderFactsSnapshot,
    ) -> Result<Self, AgentLaunchError> {
        Ok(Self {
            usage: facts.usage.clone().map(Box::new),
            reported_model: facts.reported_model.clone().map(Box::new),
            raw: GoRaw(
                serde_json::value::to_raw_value(facts)
                    .map_err(|e| AgentLaunchError::caused("agent-provider-facts-encoding", e))?,
            ),
        })
    }
    /// Restore the frozen provider projection, including absent canonical facts.
    /// Caller already owns or validates the durable receipt, never a public body.
    pub(crate) fn apply(&self, agent: &mut Agent) {
        agent.usage = self.usage.clone();
        agent.reported_model = self.reported_model.clone();
        agent.native_provider_facts = Some(self.raw.clone());
    }
}
/// Version1 is a file-format invariant, not retention or execution policy.
/// Receipts are create-only and retained after success for idempotent inspection.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: i64,
    directory: (u64, u64),
    expected: Agent,
    incomplete_output: bool,
    exit_code: Option<i64>,
    failed: bool,
    interrupted: bool,
    stop_kind: Option<String>,
    reserved: store::ReservationReceipt,
    may_have_started: bool,
    progress: crate::AgentProgress,
    details: GoRaw,
    public_reply: String,
    facts: CompletionFacts,
}
/// Hash only the identifier for a fixed safe filename. This binds naming, not
/// authenticity; root inode and full Agent snapshot are independently checked.
fn name(id: &str) -> String {
    format!("native-completion-{:x}.json", Sha256::digest(id.as_bytes()))
}
/// Publish reaped-process observations before any final SQL write. Atomic private
/// create-only publication synchronizes bytes and directory; existing receipts
/// never get overwritten. IO/codec/durability uncertainty stays explicit. If both
/// this storage and SQL fail, terminal durability cannot be guaranteed.
pub(crate) fn persist(
    directory: &LocalDirectory,
    expected: &Agent,
    receipt: &store::Completion<'_>,
    incomplete_output: bool,
) -> Result<(), RootStoreError> {
    let journal = Journal {
        schema_version: 1,
        directory: directory.identity().map_err(RootStoreError::Storage)?,
        expected: expected.clone(),
        incomplete_output,
        exit_code: receipt.exit_code,
        failed: receipt.failed,
        interrupted: receipt.interrupted,
        stop_kind: receipt.stop_kind.map(str::to_owned),
        reserved: receipt.reserved.clone(),
        may_have_started: receipt.may_have_started,
        progress: receipt.progress.clone(),
        details: receipt.details.clone(),
        public_reply: receipt.public_reply.into(),
        facts: CompletionFacts {
            usage: receipt.facts.usage.clone(),
            reported_model: receipt.facts.reported_model.clone(),
            raw: receipt.facts.raw.clone(),
        },
    };
    let bytes = serde_json::to_vec(&journal).map_err(|e| {
        RootStoreError::Launch(Box::new(AgentLaunchError::caused(
            "agent-completion-journal-encoding",
            e,
        )))
    })?;
    directory
        .publish_new_with_fault(
            &name(&expected.id),
            &bytes,
            crate::local_storage::AtomicStorageFault::None,
        )
        .map_err(RootStoreError::Storage)
}
/// Establish original supervisor absence on the same host/boot/PID namespace.
/// PID reuse is checked by start stamp. Missing/zombie is absence; permission,
/// malformed procfs or unknown host refuses. Observation grants no signal rights.
fn departed(expected: &Agent) -> Result<(), AgentLaunchError> {
    let current = crate::SupervisorIdentity::current()
        .map_err(|e| AgentLaunchError::caused("agent-completion-supervisor-observation", e))?;
    if expected.host != current.host() || expected.supervisor_identity.is_empty() {
        return Err(AgentLaunchError::Refused(
            "agent-completion-supervisor-host-mismatch",
        ));
    }
    let pid = u32::try_from(expected.supervisor_pid)
        .map_err(|e| AgentLaunchError::caused("agent-completion-supervisor-pid-invalid", e))?;
    match crate::observe_linux_process(pid) {
        Ok(live) if live.process_stamp() != expected.supervisor_identity => Ok(()),
        Ok(_) => Err(AgentLaunchError::Refused(
            "agent-completion-supervisor-still-live",
        )),
        Err(error) if error.confirms_absence() => Ok(()),
        Err(error) => Err(AgentLaunchError::caused(
            "agent-completion-supervisor-observation",
            error,
        )),
    }
}
/// Transaction-local ownership proof for journal replay. Compare every persisted
/// Agent field except an allowed stop intention; newer attempts/state refuse.
/// This permits settlement only after original owner departure, never by dead PID
/// alone. Called exclusively after receipt/root binding in recover().
pub(crate) fn recovery_owner(
    tx: &rusqlite::Transaction<'_>,
    expected: &Agent,
) -> Result<(Agent, Vec<u8>), AgentLaunchError> {
    let epoch: i64 = tx.query_row("PRAGMA main.user_version", [], |row| row.get(0))?;
    if epoch != crate::RECORDED_COMMAND_SCHEMA_VERSION {
        return Err(AgentLaunchError::Refused("storage_version_conflict"));
    }
    departed(expected)?;
    let current =
        crate::read_agent(tx, &expected.id).map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    crate::agent_store::verify_agent_ownership(tx, &current)
        .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    let mut comparable = current.clone();
    comparable.desired = expected.desired.clone();
    if comparable != *expected
        || !matches!(current.status.as_str(), "starting" | "running")
        || !matches!(current.desired.as_str(), "" | "stop")
        || current.native_execution_result.is_some()
    {
        return Err(AgentLaunchError::Refused(
            "agent-completion-current-state-changed",
        ));
    }
    let raw = crate::agent_launch::query(
        tx,
        "SELECT CAST(body AS BLOB) FROM main.agents WHERE id=?",
        [&expected.id],
        |rows| {
            Ok(rows
                .next()?
                .ok_or(AgentLaunchError::Refused("agent-execution-missing"))?
                .get(0)?)
        },
    )?;
    Ok((current, raw))
}
/// Read the exact retained v1 original and Root binding, preserving missing,
/// codec/IO/identity causes. No recovery, signal, budget or state change occurs.
fn read_journal(directory: &LocalDirectory, id: &str) -> Result<(Journal, Vec<u8>), RootStoreError> {
    let bytes = directory.read(&name(id)).map_err(|error| {
        let missing = error.cleanup.is_none()
            && error
                .source()
                .and_then(|e| e.downcast_ref::<rustix::io::Errno>())
                .is_some_and(|e| *e == rustix::io::Errno::NOENT);
        RootStoreError::Launch(Box::new(AgentLaunchError::caused(
            if missing {
                "agent-completion-journal-missing"
            } else {
                "agent-completion-journal-read"
            },
            error,
        )))
    })?;
    let journal: Journal = swarm_wire_json::decode(&bytes).map_err(|e| {
        RootStoreError::Launch(Box::new(AgentLaunchError::caused(
            "agent-completion-journal-invalid",
            e,
        )))
    })?;
    if journal.schema_version != 1
        || journal.expected.id != id
        || journal.directory != directory.identity().map_err(RootStoreError::Storage)?
    {
        return Err(RootStoreError::Refused(
            "agent-completion-journal-binding-invalid",
        ));
    }
    Ok((journal, bytes))
}
/// Shared exact terminal Agent/reservation/FIFO projection used by recovery and
/// readonly worker evidence. Caller checked the original ID/Root and owns its SQL
/// snapshot. After matching the original process receipt and reservation, v3 also
/// requires the exact released turn history; missing proof never becomes successful
/// terminal replay or planner evidence. Legacy/manual semantics remain unchanged.
/// All causes survive, no current consent is required and no write, new ownership,
/// expiry, process start or task acceptance follows from this observation.
fn verify_terminal(db: &rusqlite::Connection, journal: &Journal, current: &Agent) -> Result<(), RootStoreError> {
    if !matches!(current.status.as_str(), "completed" | "failed" | "interrupted") {
        return Err(RootStoreError::Refused("agent-completion-terminal-state-required"));
    }
    let mut expected = journal.expected.clone();
    expected.desired = current.desired.clone();
    expected.status = if journal.interrupted || current.desired == "stop" {
        "interrupted"
    } else if journal.failed {
        "failed"
    } else {
        "completed"
    }
    .into();
    expected.exit_code = journal.exit_code.map(Box::new);
    expected.ended = current.ended.clone();
    expected.heartbeat = current.ended.clone();
    expected.progress = journal.progress.clone();
    expected.reply = journal.public_reply.clone();
    expected.native_execution_result = Some(journal.details.clone());
    journal.facts.apply(&mut expected);
    expected.activity = "Processus récolté ; tâche non acceptée, validation requise".into();
    if expected.status == "interrupted" {
        expected.stop_kind = if current.desired == "stop" {
            "operateur".into()
        } else {
            journal.stop_kind.clone().ok_or(RootStoreError::Refused(
                "agent-execution-stop-cause-missing",
            ))?
        };
    }
    if current != &expected || current.ended.is_empty() {
        return Err(RootStoreError::Refused(
            "agent-completion-current-state-changed",
        ));
    }
    crate::agent_store::verify_agent_ownership(db, current)
        .map_err(RootStoreError::Agent)?;
    let mut reservation = journal.reserved.clone();
    if let Some((_, _, state)) = reservation.as_mut() {
        if state == "reserved" {
            *state = if journal.may_have_started {
                "estimated"
            } else {
                "released"
            }
            .into();
        }
    }
    if store::reservation(db, &current.id).map_err(|e| RootStoreError::Launch(Box::new(e)))?
        != reservation
    {
        return Err(RootStoreError::Refused(
            "agent-completion-reservation-changed",
        ));
    }
    crate::root_store::mission_agent::verify_terminal_workspace(db, current)
        .map_err(|cause| RootStoreError::Launch(Box::new(cause)))?;
    Ok(())
}
/// Private controller facts; missing legacy observations stay unknown. Additional
/// diagnostic fields are ignored by this projection, never promoted to a grant.
#[derive(Deserialize)]
struct WaitFacts {
    schema_version: i64,
    kernel_reaped: Option<bool>,
    workspace_release_confirmed: Option<bool>,
    recovery_required: Option<bool>,
    output_complete: Option<bool>,
    gate_release_attempted: Option<bool>,
    provider_may_have_started: Option<bool>,
    private_gate_eof: Option<bool>,
    private_gate_invalid: Option<bool>,
    pre_exec_failure: Option<GoRaw>,
}
/// Readonly original coordinates. Full Agent remains internal; safe projections
/// may expose IDs/outcome/hash, never raw prompt, capture, host or PID stamps.
pub(crate) struct TerminalObservation {
    pub agent: Agent,
    pub receipt_sha256: String,
    pub output_complete: bool,
    pub provider_may_have_started: bool,
}
/// Validate original kernel reap, workspace release, private gate and native
/// provenance independently of the current SQL terminal projection. Recovery uses
/// the same proof BEFORE any settlement; observation additionally checks terminal
/// rows. Malformed/unknown facts refuse, even if the original owner is still live.
fn verify_original_facts(journal: &Journal) -> Result<WaitFacts, RootStoreError> {
    let facts: WaitFacts = swarm_wire_json::decode(journal.details.0.get().as_bytes()).map_err(|e|
        RootStoreError::Launch(Box::new(AgentLaunchError::caused("agent-completion-wait-format", e))))?;
    if facts.schema_version != 1 || facts.kernel_reaped != Some(true) || facts.workspace_release_confirmed != Some(true)
        || facts.recovery_required != Some(false) || journal.incomplete_output || facts.output_complete.is_none() {
        return Err(RootStoreError::Refused("agent-completion-wait-not-confirmed"));
    }
    if facts.private_gate_eof != Some(true) || facts.private_gate_invalid != Some(false)
        || facts.provider_may_have_started != Some(journal.may_have_started)
        || if journal.may_have_started { facts.gate_release_attempted != Some(true) || facts.pre_exec_failure.is_some() }
           else { facts.pre_exec_failure.is_none() } {
        return Err(RootStoreError::Refused("agent-completion-gate-not-confirmed"));
    }
    let original = &journal.expected;
    if original.role != "worker" || original.child_pid < 1 || original.supervisor_pid < 1
        || original.child_identity.is_empty() || original.supervisor_identity.is_empty() || original.host.is_empty() {
        return Err(RootStoreError::Refused("agent-completion-physical-origin-required"));
    }
    let snapshot = original.native_execution.as_ref().ok_or(RootStoreError::Refused("agent-completion-native-origin-required"))?;
    let snapshot = crate::decode_agent_execution_snapshot(snapshot.0.get().as_bytes()).map_err(|e|
        RootStoreError::Launch(Box::new(AgentLaunchError::caused("agent-completion-native-origin", e))))?;
    if snapshot.context_sha256 != crate::provider_configuration::configuration_digest(original.prompt.as_bytes())
        || journal.may_have_started && snapshot.phase != crate::AgentExecutionPhase::PermissionRecorded {
        return Err(RootStoreError::Refused("agent-completion-native-origin-changed"));
    }
    Ok(facts)
}
/// Verify actual terminal native observations in the caller transaction without
/// invoking recovery or mutating history. Requires known reap/release and gate
/// disposition, matching full Agent/ownership/reservation and native snapshot.
/// Explicit logical output failure can be observed for correction, never quality
/// acceptance; unknown wait/drain/owner remains refusal regardless of PID/lease.
/// Reads the complete original under the existing store contract; no aggregate
/// memory bound or hostile same-UID writer protection is established.
pub(crate) fn observe_terminal(db: &rusqlite::Connection, directory: &LocalDirectory, id: &str) -> Result<TerminalObservation, RootStoreError> {
    if db.is_autocommit() { return Err(RootStoreError::Refused("agent-completion-observation-transaction-required")); }
    let (journal, bytes) = read_journal(directory, id)?;
    let facts = verify_original_facts(&journal)?;
    let current = crate::read_agent(db, id).map_err(RootStoreError::Agent)?;
    verify_terminal(db, &journal, &current)?;
    Ok(TerminalObservation { agent: current, receipt_sha256: format!("{:x}", Sha256::digest(&bytes)),
        output_complete: facts.output_complete.ok_or(RootStoreError::Refused("agent-completion-output-observation-required"))?,
        provider_may_have_started: journal.may_have_started })
}
/// Replay an owned durable complete-output receipt under current saved SQLite
/// settings. Missing/foreign/stale/incomplete receipts refuse, with no relaunch,
/// refund or task acceptance. Original causes remain available. Already recorded
/// matching terminal facts return from an explicit coherent read transaction, so
/// output failure can be retried. Read/finalize/rollback and settings-release
/// errors remain distinct; the storage reservation alone is not a SQL snapshot.
pub(crate) fn recover(
    db: &mut rusqlite::Connection,
    root: &Path,
    directory: &LocalDirectory,
    id: &str,
) -> Result<Agent, RootStoreError> {
    let (journal, _) = read_journal(directory, id)?;
    recover_original(db, root, directory, id, journal, None)
}

/// Observe an original completion for a registered automatic worker, then recover
/// only after actual owner departure and a current transaction-local mission fence.
/// Missing original or a still-live owner is explicit waiting with its typed cause,
/// never a terminal/permission claim. Invalid originals and all other errors refuse.
/// No queued cancellation, provider invocation, process signal or new attempt occurs.
pub(crate) fn recover_under_mission(
    db: &mut rusqlite::Connection, root: &Path, directory: &LocalDirectory,
    id: &str, fence: &dyn crate::work_engine::MutationFence,
) -> Result<crate::MissionWorkerRecovery, RootStoreError> {
    let journal=match read_journal(directory,id) {
        Ok((journal,_))=>journal,
        Err(error) if matches!(&error,RootStoreError::Launch(cause)
            if matches!(cause.as_ref(),AgentLaunchError::Cause{code:"agent-completion-journal-missing",..})) => {
                return Ok(crate::MissionWorkerRecovery::Waiting { reason:"completion-original-pending",cause:error });
            }
        Err(error)=>return Err(error),
    };
    verify_original_facts(&journal)?;
    match departed(&journal.expected) {
        Ok(())=>{},
        Err(error @ AgentLaunchError::Refused("agent-completion-supervisor-still-live"))=>{
            return Ok(crate::MissionWorkerRecovery::Waiting { reason:"completion-owner-still-live",cause:RootStoreError::Launch(Box::new(error)) });
        }
        Err(error)=>return Err(RootStoreError::Launch(Box::new(error))),
    }
    recover_original(db,root,directory,id,journal,Some(fence))
        .map(|agent|crate::MissionWorkerRecovery::Finalized { agent: Box::new(agent) })
}

/// Share journal decoding/settlement across explicit manual and mission recovery.
/// The optional live fence applies inside each SQL snapshot/settlement, not merely
/// before callback admission. Original terminal replay stays idempotent; every
/// failure retains its cause and any uncertain/recorded effect through retry/close.
fn recover_original(
    db: &mut rusqlite::Connection, root: &Path, directory: &LocalDirectory,
    id: &str, journal: Journal, fence: Option<&dyn crate::work_engine::MutationFence>,
) -> Result<Agent, RootStoreError> {
    if journal.incomplete_output {
        return Err(RootStoreError::Refused(
            "agent-completion-output-recovery-required",
        ));
    }
    let receipt = store::Completion {
        exit_code: journal.exit_code,
        failed: journal.failed,
        interrupted: journal.interrupted,
        stop_kind: journal.stop_kind.as_deref(),
        reserved: &journal.reserved,
        may_have_started: journal.may_have_started,
        progress: &journal.progress,
        details: &journal.details,
        public_reply: &journal.public_reply,
        facts: &journal.facts,
    };
    crate::agent_storage_retry::execute(db, directory, true, |db, settings| {
        let current = crate::read_agent(db, id).map_err(RootStoreError::Agent)?;
        if matches!(
            current.status.as_str(),
            "completed" | "failed" | "interrupted"
        ) {
            // The settings reservation does not open a SQL transaction. Reread
            // Agent and all terminal/FIFO proof in one explicit read snapshot.
            let behavior=if fence.is_some(){rusqlite::TransactionBehavior::Immediate}else{rusqlite::TransactionBehavior::Deferred};
            let mut tx = db.transaction_with_behavior(behavior).map_err(RootStoreError::Sql)?;
            let observed = (|| {
                if let Some(fence)=fence {
                    let now=crate::system_timestamp().map_err(|e|RootStoreError::Runtime(crate::RuntimeError::new(crate::RuntimeOperation::Platform,e)))?;
                    fence.check(&tx,&journal.expected.work_id,&now).map_err(RootStoreError::Mutation)?;
                }
                let current = crate::read_agent(&tx, id).map_err(RootStoreError::Agent)?;
                verify_terminal(&tx, &journal, &current)?;
                Ok(current)
            })();
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            return crate::root_store::finished(observed, tx.rollback().map_err(RootStoreError::Sql));
        }
        match fence {
            Some(fence)=>store::complete_recovered_fenced(db,root,settings,&journal.expected,&receipt,fence),
            None=>store::complete_recovered(db, root, settings, &journal.expected, &receipt),
        }.map_err(|e| RootStoreError::Launch(Box::new(e)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A genuine current Linux supervisor is not a recoverable dead owner.
    /// Host mismatch fails closed; no process is spawned, signalled or stored.
    #[test]
    fn durable_recovery_refuses_live_or_foreign_supervisor() {
        let identity = crate::SupervisorIdentity::current().unwrap();
        let mut agent = Agent {
            supervisor_pid: identity.pid(),
            supervisor_identity: identity.process_stamp().into(),
            host: identity.host().into(),
            ..Default::default()
        };
        assert_eq!(
            departed(&agent).unwrap_err().code(),
            "agent-completion-supervisor-still-live"
        );
        agent.host = "foreign".into();
        assert_eq!(
            departed(&agent).unwrap_err().code(),
            "agent-completion-supervisor-host-mismatch"
        );
        assert_ne!(name("a/b"), name("a-b"));
        assert!(!name("a/b").contains('/'));
    }
}
