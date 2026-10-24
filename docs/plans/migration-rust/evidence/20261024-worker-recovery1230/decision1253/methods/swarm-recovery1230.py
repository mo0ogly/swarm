"""Add a lease-fenced terminal journal recovery path without queued cancellation."""
from pathlib import Path
import sys,json
sys.path.insert(0,'.cache/rust-git-port');from editlib import atomic,edit
b=Path('docs/plans/migration-rust/evidence/20261024-worker-recovery1230');b.mkdir(mode=0o700)
paths=['rust/crates/swarm-engine/src/agent_completion_journal.rs','rust/crates/swarm-engine/src/agent_execution_store.rs','rust/crates/swarm-engine/src/mission_supervision_store.rs','rust/crates/swarm-engine/src/root_store.rs','rust/crates/swarm-engine/src/lib.rs','rust/crates/swarm-cli/src/mission_watch.rs','version_history.json']
for name in paths:
 p=b/'originals'/name;p.parent.mkdir(parents=True,exist_ok=True);atomic(p,Path(name).read_bytes())
p=Path('docs/plans/migration-rust/PLAN.md');atomic(p,p.read_text()+'''\n###1230 — finalisation native enregistrée sous bail\nReplan/APEX : finalisation de reçu non appliqué SQL distincte de l'intégration\nrepository managed (encore refusée dans complete_with_owner). Ne pas l'annoncer\ncomme migration du règlement Git. Reconcile générique rejeté : peut annuler queued.\nChoix : journal/finalisation partagés, fence mission dans la même transaction.\nRacine réserve agent_completion_journal,agent_execution_store, nouveau module\nmission_worker_recovery, exports mission_supervision_store/root_store/lib,\nmission_watch, recette et docs FR/EN/version ; pas de LIVE ni index Go modifié.\nParcours : worker réel→refus SQL fixture→ancien helper parti→serveur redémarré\nfinalise même Agent, sans provider supplémentaire ; reçus absents/invalides,\nsuperviseur actif, mauvais Work/bail refusent ou attendent sans mutation.\nGardes ciblées, puis profil normal+recette publique ; aucune installation.\n''')
p='rust/crates/swarm-engine/src/agent_execution_store.rs'
edit(p,'complete_with_owner(db, root, settings, expected, receipt, false)','complete_with_owner(db, root, settings, expected, receipt, false, None)')
edit(p,'complete_with_owner(db, root, settings, expected, receipt, true)','complete_with_owner(db, root, settings, expected, receipt, true, None)')
edit(p,'/// Share identical atomic settlement semantics across live owner and durable','''/// Recover the same durable completion under an additional current mission fence.
/// The fence is checked inside the existing settlement writer transaction before
/// owner/task/reservation changes. It adds no launch, consent or cancellation;
/// original and independent transaction/cleanup errors remain retained.
pub(crate) fn complete_recovered_fenced(
    db: &mut Connection, root: &Path, settings: &EngineSettingsDocument,
    expected: &Agent, receipt: &Completion<'_>, fence: &dyn crate::work_engine::MutationFence,
) -> Result<Agent, AgentLaunchError> {
    complete_with_owner(db, root, settings, expected, receipt, true, Some(fence))
}
/// Share identical atomic settlement semantics across live owner and durable''')
edit(p,'    recovering: bool,\n) -> Result<Agent, AgentLaunchError> {\n    transaction(db, |tx| {','''    recovering: bool,
    fence: Option<&dyn crate::work_engine::MutationFence>,
) -> Result<Agent, AgentLaunchError> {
    transaction(db, |tx| {
        if let Some(fence)=fence {
            let now=crate::system_timestamp().map_err(|e|AgentLaunchError::caused("agent-recovery-clock",e))?;
            fence.check(tx,&expected.work_id,&now)
                .map_err(|e|AgentLaunchError::caused("agent-recovery-mission-fence",e))?;
        }''')
p='rust/crates/swarm-engine/src/agent_completion_journal.rs'
edit(p,'''    let (journal, _) = read_journal(directory, id)?;
    if journal.incomplete_output {''','''    let (journal, _) = read_journal(directory, id)?;
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
    match departed(&journal.expected) {
        Ok(())=>{},
        Err(error @ AgentLaunchError::Refused("agent-completion-supervisor-still-live"))=>{
            return Ok(crate::MissionWorkerRecovery::Waiting { reason:"completion-owner-still-live",cause:RootStoreError::Launch(Box::new(error)) });
        }
        Err(error)=>return Err(RootStoreError::Launch(Box::new(error))),
    }
    recover_original(db,root,directory,id,journal,Some(fence))
        .map(|agent|crate::MissionWorkerRecovery::Finalized { agent })
}

/// Share journal decoding/settlement across explicit manual and mission recovery.
/// The optional live fence applies inside each SQL snapshot/settlement, not merely
/// before callback admission. Original terminal replay stays idempotent; every
/// failure retains its cause and any uncertain/recorded effect through retry/close.
fn recover_original(
    db: &mut rusqlite::Connection, root: &Path, directory: &LocalDirectory,
    id: &str, journal: Journal, fence: Option<&dyn crate::work_engine::MutationFence>,
) -> Result<Agent, RootStoreError> {
    if journal.incomplete_output {''')
edit(p,'''            let mut tx = db.transaction().map_err(RootStoreError::Sql)?;
            let observed = (|| {
                let current = crate::read_agent(&tx, id).map_err(RootStoreError::Agent)?;''','''            let behavior=if fence.is_some(){rusqlite::TransactionBehavior::Immediate}else{rusqlite::TransactionBehavior::Deferred};
            let mut tx = db.transaction_with_behavior(behavior).map_err(RootStoreError::Sql)?;
            let observed = (|| {
                if let Some(fence)=fence {
                    let now=crate::system_timestamp().map_err(|e|RootStoreError::Runtime(crate::RuntimeError::new(crate::RuntimeOperation::Platform,e)))?;
                    fence.check(&tx,&journal.expected.work_id,&now).map_err(RootStoreError::Mutation)?;
                }
                let current = crate::read_agent(&tx, id).map_err(RootStoreError::Agent)?;''')
edit(p,'''        store::complete_recovered(db, root, settings, &journal.expected, &receipt)
            .map_err(|e| RootStoreError::Launch(Box::new(e)))''','''        match fence {
            Some(fence)=>store::complete_recovered_fenced(db,root,settings,&journal.expected,&receipt,fence),
            None=>store::complete_recovered(db, root, settings, &journal.expected, &receipt),
        }.map_err(|e| RootStoreError::Launch(Box::new(e)))''')
p=Path('rust/crates/swarm-engine/src/mission_worker_recovery.rs')
atomic(p,'''//! Finalize an existing registered automatic worker from its original journal.
//! This does not integrate managed repositories or grant another worker attempt.
use super::{bound, owned_heartbeat, read_transaction, root, MissionLease, RootStore, E};
use crate::{Agent, RootStoreError};

/// A terminal projection after guarded settlement/replay, or a legitimate pending
/// original/owner observation with its exact typed cause. Neither variant grants
/// launch, process signalling, new budget, task acceptance or a replacement ID.
pub enum MissionWorkerRecovery {
    /// Complete terminal proof, possibly an idempotent replay; not a new-commit claim.
    Finalized { agent: Agent },
    /// No recovery is applied. Cause distinguishes missing original and live owner;
    /// malformed/unreadable originals are errors, never this waiting observation.
    Waiting { reason: &'static str, cause: RootStoreError },
}
impl RootStore {
    /// Recover one already registered automatic worker in this lease's Work.
    /// Requires writable bound Root, current owned lease, worker origin/provenance,
    /// durable original and departed supervisor. Rechecks the lease in the actual
    /// completion transaction; original task/reservation/FIFO proof is unchanged.
    /// Missing original/live owner returns Waiting; malformed proof, stale fence,
    /// cleanup/SQL failure and uncertain outcomes stay errors. It never calls the
    /// generic queued-cancellation branch. No current consent is needed to settle
    /// an already granted worker; this grants no new execution. Caller owes lease
    /// and Root close; repository-managed integration remains explicitly unported.
    pub fn recover_mission_worker(&mut self,lease:&MissionLease,id:&str)
        ->Result<MissionWorkerRecovery,RootStoreError> {
        if !self.writable { return Err(RootStoreError::Refused("root-read-only")); }
        bound(self,lease).map_err(|e|RootStoreError::Mutation(crate::WorkEngineError::Mission(Box::new(e))))?;
        read_transaction(&self.db,||{
            owned_heartbeat(&self.db,lease)?;
            Ok(())
        }).map_err(|e|RootStoreError::Mutation(crate::WorkEngineError::Mission(Box::new(e))))?;
        let agent=self.read_agent(id)?;
        if agent.work_id!=lease.work || agent.origin!="conducteur" || agent.role!="worker"
            || agent.supervisor_pid<1 {
            return Err(RootStoreError::Refused("mission-recovery-registered-worker-required"));
        }
        crate::agent_completion_journal::recover_under_mission(&mut self.db,&self.root,&self.directory,id,lease)
    }
}
''')
p='rust/crates/swarm-engine/src/mission_supervision_store.rs';edit(p,'mod worker_adoption;','mod worker_adoption;\n#[path = "mission_worker_recovery.rs"]\nmod worker_recovery;\npub use worker_recovery::MissionWorkerRecovery;')
p='rust/crates/swarm-engine/src/root_store.rs';edit(p,'pub use mission_supervision_store::{MissionQueueRegistration,','pub use mission_supervision_store::MissionWorkerRecovery;\npub use mission_supervision_store::{MissionQueueRegistration,')
p='rust/crates/swarm-engine/src/lib.rs';edit(p,'    MissionWorkerAdoptionRequest, MissionWorkerAdoptionOutcome,','    MissionWorkerAdoptionRequest, MissionWorkerAdoptionOutcome, MissionWorkerRecovery,')
print('1230 guarded journal entry prepared; not compiled/tested yet')
