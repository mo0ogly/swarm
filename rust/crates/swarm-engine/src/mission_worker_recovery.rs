//! Finalize an existing registered automatic worker from its original journal.
//! This does not integrate managed repositories or grant another worker attempt.
use super::{bound, caused, owned_heartbeat, read_transaction, MissionLease, RootStore, E};
use crate::{Agent, RootStoreError};

/// A terminal projection after guarded settlement/replay, or a legitimate pending
/// original/owner observation with its exact typed cause. Neither variant grants
/// launch, process signalling, new budget, task acceptance or a replacement ID.
pub enum MissionWorkerRecovery {
    /// Complete terminal proof, possibly an idempotent replay; not a new-commit claim.
    Finalized { agent: Box<Agent> },
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
            let heartbeat=owned_heartbeat(&self.db,lease)?;
            let now=crate::system_timestamp().map_err(|e|caused("mission-recovery-clock",e))?;
            if lease.policy.observe(&heartbeat,&now)?!=crate::MissionLeaseClock::Present {
                return Err(E::Refused("mission-lease-not-present"));
            }
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
