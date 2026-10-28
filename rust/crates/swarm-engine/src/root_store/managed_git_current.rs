//! Latest cumulative native publication observation, never an effect permit.
use super::{finished, managed_publication, material_path, RootStore, RootStoreError as E};
use crate::managed_publication_record;
use rusqlite::{Transaction, TransactionBehavior};
use serde::Serialize;

/// Exact latest publication verified against its originals and whole current
/// cohort. No private paths, serialized authority, terminal Git effect or lease.
/// repository_request_digest is actual planning data, NOT a service repository_id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CurrentManagedGitPublication {
    pub work_id: String,
    pub work_revision: i64,
    pub settings_revision: crate::SettingCount,
    pub publication_event: String,
    pub task_id: String,
    pub review_id: String,
    pub candidate_commit: String,
    pub previous_candidate: String,
    pub candidate_binding_sha256: String,
    pub context_sha256: String,
    pub validation_sha256: String,
    pub cohort_sha256: String,
    pub repository_request_digest: String,
}
/// Explicitly release a read snapshot, suppressing implicit Drop retries. Errors
/// preserve the actual rollback cause; no storage mutation was requested here.
fn release(mut tx: Transaction<'_>) -> Result<(), E> {
    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
    if tx.is_autocommit() {
        Ok(())
    } else {
        tx.execute_batch("ROLLBACK").map_err(E::Sql)
    }
}
impl RootStore {
    /// Read the LAST canonical native publication for this recorded work. Under
    /// a SQL snapshot and saved-settings/material association checks, verify real
    /// original review/validation/worker result, current controls and complete
    /// cohort via the existing authoritative reader. There is no caller task or
    /// candidate selector, fallback, positive SQL write or provider invocation.
    /// Missing/changed originals, non-managed work or absent chain refuse with
    /// original typed causes; independent snapshot/material/close errors survive.
    /// This is a traversal-local observation, not admission, current authority,
    /// held repository custody, an effect receipt or permission for later Git.
    /// Effect consumers must recheck under their own barrier and independently
    /// bind repository_id; the planning request digest does not create that bind.
    pub fn current_managed_git_publication(
        &self,
        work_id: &str,
    ) -> Result<CurrentManagedGitPublication, E> {
        material_path::with_material_read(self, || {
            let settings = self.read_settings()?;
            let tx = Transaction::new_unchecked(&self.db, TransactionBehavior::Deferred)
                .map_err(E::Sql)?;
            let result = self.read_managed_git_publication(work_id, &settings);
            finished(result, release(tx))
        })
    }
    /// Require the actual main-database WRITE state on THIS opened connection.
    /// Deferred/read snapshots and caller booleans cannot substitute for a writer.
    /// No transaction, settings, policy or business permission is created here.
    pub(super) fn require_native_git_writer(&self) -> Result<(), E> {
        if self.db.transaction_state(Some("main")).map_err(E::Sql)?
            != rusqlite::TransactionState::Write
        {
            return Err(E::Refused("managed-current-publication-writer-required"));
        }
        Ok(())
    }
    /// Read the same authoritative candidate under a caller-held actual writer,
    /// without nested BEGIN. Retains material/settings guards only for this read;
    /// the effect owner must keep its own settings/Trust/plan/authority barrier
    /// through commitment. This remains observation, never an approval or effect.
    pub(super) fn current_managed_git_publication_for_effect(
        &self,
        work_id: &str,
    ) -> Result<CurrentManagedGitPublication, E> {
        self.require_native_git_writer()?;
        material_path::with_material_read(self, || {
            let settings = self.read_settings()?;
            self.read_managed_git_publication(work_id, &settings)
        })
    }
    /// Shared native chain/original/cohort body over the Root's own connection.
    /// Caller owns a real SQL snapshot and saved-settings/material guards. No
    /// alternate connection, DTO acceptance flag, transaction or mutation occurs.
    fn read_managed_git_publication(
        &self,
        work_id: &str,
        settings: &crate::EngineSettingsDocument,
    ) -> Result<CurrentManagedGitPublication, E> {
        let work = crate::read_recorded_work(&self.db, work_id).map_err(E::Work)?;
        let repo = work
            .planning
            .as_deref()
            .and_then(|p| p.repository.as_deref())
            .ok_or(E::Refused(
                "managed-current-publication-repository-required",
            ))?;
        let records = managed_publication_record::chain(&self.db, &work)
            .map_err(|e| E::Launch(Box::new(e)))?;
        let latest = records
            .last()
            .ok_or(E::Refused("managed-current-publication-required"))?;
        let mut tasks = work
            .tasks
            .iter()
            .flatten()
            .filter(|t| t.id == latest.task_id);
        let task = tasks
            .next()
            .ok_or(E::Refused("managed-current-publication-task-required"))?;
        if tasks.next().is_some() {
            return Err(E::Refused("managed-current-publication-task-ambiguous"));
        }
        let record =
            managed_publication::verify_published_current(self, &self.db, &work, task, settings)?;
        if record.event_id != latest.event_id
            || record.candidate_commit != repo.candidate_commit
            || !swarm_identity_contract::digest(repo.request_digest.as_bytes())
        {
            return Err(E::Refused("managed-current-publication-binding"));
        }
        Ok(CurrentManagedGitPublication {
            work_id: work.id.clone(),
            work_revision: work.revision,
            settings_revision: settings.revision,
            publication_event: record.event_id,
            task_id: record.task_id,
            review_id: record.review_id,
            candidate_commit: record.candidate_commit,
            previous_candidate: record.previous_candidate,
            candidate_binding_sha256: record.binding_sha256,
            context_sha256: record.context_sha256,
            validation_sha256: record.validation_sha256,
            cohort_sha256: record.cohort_sha256,
            repository_request_digest: repo.request_digest.clone(),
        })
    }
}
