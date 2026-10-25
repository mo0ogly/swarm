//! Explicit native managed-candidate preparation, not control admission.
//! Root observations precede and follow Git outside SQL/settings locks. The
//! explicit stage starts no job or command. Actual controls use the separate
//! managed executor; final managed review remains guarded.
use super::{Observation, RepositoryPurpose, Request, ReviewSource};
use crate::root_store::{finished, RootStore, RootStoreError as E};
use serde::Serialize;

/// Public candidate-stage result. Hashes identify an actual retained checkpoint
/// and cohort but grant no command permission or task acceptance. Host paths,
/// commands, kernel identity and private report bytes are deliberately absent.
#[derive(Serialize)]
pub struct TaskValidationCandidate {
    pub schema_version: i64,
    pub work_id: String,
    pub work_revision: i64,
    pub task_id: String,
    pub producer_agent_id: String,
    pub attempt_id: String,
    pub candidate_commit: String,
    pub candidate_binding_sha256: String,
    pub cohort_task_count: usize,
    pub control_count: usize,
    pub job_created: bool,
    pub controls_started: bool,
    pub task_accepted: bool,
}
/// Private fresh Root observation. Whole saved settings, Work and real connected
/// Root identity bind this finite preparation interval; the digest is not a lease.
struct Snapshot {
    observation: Observation,
    settings: crate::EngineSettingsDocument,
    digest: String,
}
/// Observe an unclaimed actual submission under a short read transaction and
/// saved-settings reservation; close both before returning plain data. Existing
/// validation events/active jobs refuse before any Git or stage creation. The
/// actual cumulative plan is validated with saved limits before filesystem writes.
/// Errors and explicit rollback/release failures retain their typed causes. No
/// busy retry, mutation, process, permission or lease is added by this observer.
fn snapshot(store: &RootStore, work: &str, request: &Request) -> Result<Snapshot, E> {
    let launch = |e| E::Launch(Box::new(e));
    let reservation = store.directory.reserve(false, false).map_err(E::Storage)?;
    let result = (|| {
        let settings = store.read_settings()?;
        let tx = store.db.unchecked_transaction().map_err(E::Sql)?;
        let result = (|| {
            if super::store::event(&tx, &request.event_id)
                .map_err(launch)?
                .is_some()
            {
                return Err(E::Refused("task-validation-candidate-job-already-claimed"));
            }
            super::require_validation_idle(&tx, work, &request.task_id).map_err(launch)?;
            let observation = super::observe_submission(
                &tx,
                &store.root,
                &settings,
                work,
                request,
                ReviewSource::Fresh,
                RepositoryPurpose::Candidate,
            )
            .map_err(launch)?;
            let limits = settings.values.mutation_limits().map_err(E::Settings)?;
            crate::managed_repository::managed_control_plan(
                &observation.work,
                &request.task_id,
                &limits.policy,
            )
            .map_err(launch)?;
            let identity = crate::root_store::observe_instance(&tx, &store.directory)?;
            let digest = super::digest(&(
                "native-managed-candidate-preparation/1",
                &observation,
                &settings,
                identity,
            ))
            .map_err(launch)?;
            Ok(Snapshot {
                observation,
                settings,
                digest,
            })
        })();
        finished(result, tx.rollback().map_err(E::Sql))
    })();
    finished(result, reservation.close().map_err(E::Storage))
}
impl RootStore {
    /// Prepare and retain one managed single-parent candidate from the current
    /// canonical submission and actual completed native producer. Uses saved
    /// per-task cumulative policies, no aggregate limit. Root observations and
    /// checked releases surround Git preparation/pinning outside Root locks.
    /// Repeating an unchanged request reuses the immutable checkpoint; no job,
    /// event, command, permission, review, candidate-pointer or Work mutation is
    /// performed. Existing jobs refuse instead of recreating their candidate.
    /// Returns public hashes/counts only. Source/settings/identity changes and
    /// cleanup errors refuse; Git may already have retained files/refs on error,
    /// so nonzero never implies rollback. Owned descriptors are explicitly closed
    /// on every post-pin outcome. Same-UID confinement, atomic filesystem
    /// snapshots and subsequent native execution/acceptance remain unproved.
    pub fn prepare_task_validation_candidate(
        &self,
        work: &str,
        raw: &[u8],
    ) -> Result<TaskValidationCandidate, E> {
        let launch = |e| E::Launch(Box::new(e));
        let request = super::decode(raw).map_err(launch)?;
        let before = snapshot(self, work, &request)?;
        let observation = &before.observation;
        // No Root transaction/reservation is held by these Git operations.
        let retained = crate::managed_repository::verify_result(
            &self.root,
            &observation.work,
            &observation.producer,
            &before.settings,
        )
        .map_err(launch)?;
        if retained.report_path != observation.submitted.report_path
            || retained.report_sha256 != observation.submitted.report_sha256
        {
            return Err(E::Refused("task-validation-candidate-report-mismatch"));
        }
        let candidate = crate::managed_repository::prepare_candidate(
            &self.root,
            &observation.work,
            &observation.producer,
            &before.settings,
        )
        .map_err(launch)?;
        let witness = crate::managed_repository::pin_candidate(
            &self.root,
            &observation.work,
            &observation.producer,
            &before.settings,
        )
        .map_err(launch)?;
        // Keep ownership outside generic cleanup wrappers, including Root reads.
        let result = (|| {
            witness.verify_retained_identities().map_err(launch)?;
            if witness.record() != &candidate {
                return Err(E::Refused("task-validation-candidate-preparation-changed"));
            }
            let after = snapshot(self, work, &request)?;
            if after.digest != before.digest {
                return Err(E::Refused("task-validation-candidate-preparation-changed"));
            }
            let plan = candidate
                .control_plan
                .as_ref()
                .ok_or(E::Refused("task-validation-candidate-controls-required"))?;
            Ok(TaskValidationCandidate {
                schema_version: 1,
                work_id: work.into(),
                work_revision: observation.work.revision,
                task_id: request.task_id,
                producer_agent_id: observation.producer.id.clone(),
                attempt_id: observation.producer.attempt_id.clone(),
                candidate_commit: candidate.commit.clone(),
                candidate_binding_sha256: candidate.binding_digest().map_err(launch)?,
                cohort_task_count: plan.policies.len(),
                control_count: plan.slots.len(),
                job_created: false,
                controls_started: false,
                task_accepted: false,
            })
        })();
        finished(result, witness.close().map(|_| ()).map_err(launch))
    }
}
