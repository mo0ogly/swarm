//! Current accepted native baseline, verified from canonical publication originals.
use super::{finished, material_path, task_validation, RootStore, RootStoreError as E};
use crate::{EngineSettingsDocument, Work};
use serde::Serialize;
use std::collections::BTreeSet;

/// One accepted task's actual publication and immutable original coordinates.
/// Digests identify retained bytes, not an authenticated reviewer or a new verdict.
/// Private body text, host paths and execution details are not exposed.
#[derive(Serialize)]
pub struct ManagedBaselineTask {
    pub task_id: String,
    pub publication_event: String,
    pub publication_sha256: String,
    pub review_id: String,
    pub attempt_id: String,
    pub producer_agent_id: String,
    pub original_sha256: String,
    pub validation_receipt_sha256: String,
    pub candidate_binding_sha256: String,
    pub accepted_task_sha256: String,
    pub policy_sha256: String,
}
/// Read-only observation of all currently accepted tasks on the published OID.
/// Empty initial baselines have present/verified false. A verified historical
/// baseline never authorizes publication, accepts the target or proves a verdict
/// on a future candidate; that candidate needs new whole-cohort checks and review.
#[derive(Serialize)]
pub struct ManagedBaselinePreview {
    pub schema_version: u32,
    pub work_id: String,
    pub work_revision: i64,
    pub settings_revision: crate::SettingCount,
    pub target_task_id: String,
    pub base_commit: String,
    pub candidate_commit: String,
    pub baseline_present: bool,
    pub baseline_verified: bool,
    pub tasks: Vec<ManagedBaselineTask>,
    pub publication_authorized: bool,
    pub provider_started: bool,
    pub task_accepted: bool,
}

/// Verify accepted publication custody, native originals, actual report blobs,
/// current saved gate/review/prerequisite policy and unique task identities.
/// Caller supplies its actual Root snapshot and settings; no caller-supplied
/// proof, event, job or grant is created. Cumulative readers verify canonical
/// lineage and the latest whole-cohort originals while retaining earlier tasks.
/// Missing/corrupt/cross-bound originals,
/// IO/Git/codec/SQL failures propagate, without an empty successful fallback.
/// Reads may run Git under the saved command policy; this is not a source lease.
pub(in crate::root_store) fn inspect(root: &RootStore, db: &rusqlite::Connection,
    work: &Work, target: &str, settings: &EngineSettingsDocument) -> Result<ManagedBaselinePreview,E> {
    let repo=work.planning.as_deref().and_then(|p|p.repository.as_deref())
        .ok_or(E::Refused("managed-baseline-repository-required"))?;
    let mut ids=BTreeSet::new();
    for task in work.tasks.iter().flatten() {
        if !ids.insert(task.id.clone()) { return Err(E::Refused("managed-baseline-task-ambiguous")); }
    }
    if !ids.contains(target) { return Err(E::Refused("managed-baseline-target-required")); }
    let limits=settings.values.mutation_limits().map_err(E::Settings)?;
    let quality=settings.quality_policy_snapshot().map_err(E::Settings)?;
    let context=crate::TaskLifecycleContext { root:&root.root,quality_policy:Some(&quality),task_limits:&limits.task,validation_limits:&limits.policy };
    let mut tasks=Vec::new();
    for task in work.tasks.iter().flatten().filter(|t|t.status=="accepted") {
        let record=super::managed_publication::verify_published_current(root,db,work,task,settings)?;
        crate::task_lifecycle::check_scoped_accepted_freshness(work,&task.id,&context,settings)
            .map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("managed-baseline-current-evidence",e))))?;
        let policy=task.validation_policy.as_deref().ok_or(E::Refused("managed-baseline-policy-required"))?;
        tasks.push(ManagedBaselineTask { task_id:task.id.clone(),
            publication_sha256:crate::managed_publication_record::digest(&record).map_err(|e|E::Launch(Box::new(e)))?,
            publication_event:record.event_id,review_id:record.review_id,attempt_id:record.attempt,
            producer_agent_id:record.producer,original_sha256:record.original_sha256,
            validation_receipt_sha256:record.validation_sha256,candidate_binding_sha256:record.binding_sha256,
            accepted_task_sha256:record.accepted_task_sha256,
            policy_sha256:crate::managed_publication_record::digest(policy).map_err(|e|E::Launch(Box::new(e)))? });
    }
    let present=!tasks.is_empty();
    if present == (repo.candidate_commit==repo.base_commit) {
        return Err(E::Refused("managed-baseline-candidate-inconsistent"));
    }
    Ok(ManagedBaselinePreview { schema_version:1,work_id:work.id.clone(),work_revision:work.revision,
        settings_revision:settings.revision,target_task_id:target.into(),base_commit:repo.base_commit.clone(),
        candidate_commit:repo.candidate_commit.clone(),baseline_present:present,baseline_verified:present,tasks,
        publication_authorized:false,provider_started:false,task_accepted:false })
}

/// Capture actual current Work/settings in a short material-bound read snapshot.
/// No validation job, event or Git mutation is admitted; rollback/close errors
/// retain their causes. This observation is rechecked after original verification.
fn snapshot(root: &RootStore, work: &str, request: &task_validation::Request) -> Result<(Work,EngineSettingsDocument),E> {
    material_path::with_material_read(root,|| {
        let settings=root.read_settings()?;
        let tx=root.db.unchecked_transaction().map_err(E::Sql)?;
        let result=(|| {
            let current=crate::read_recorded_work(&tx,work).map_err(E::Work)?;
            if current.revision!=request.expected_revision { return Err(E::Refused("managed-baseline-revision-conflict")); }
            Ok((current,settings))
        })();
        finished(result,tx.rollback().map_err(E::Sql))
    })
}
impl RootStore {
    /// Observe the complete currently accepted native baseline for a real task.
    /// Takes the strict native task/revision envelope; event_id is syntax only,
    /// never persisted. Checks canonical publication, private/public originals,
    /// current Git report/artifacts and saved policy under actual Root custody,
    /// then rechecks Work/settings. Returns hashes and explicit non-authority
    /// flags; no provider, job, debit, Work or event mutation occurs. Empty initial
    /// baseline is distinct from corrupt/missing evidence, which refuses with its
    /// typed cause. The returned observation grants no future freshness lease,
    /// cumulative publication or implicit task/global completion.
    pub fn preview_managed_baseline(&self, work: &str, raw: &[u8]) -> Result<ManagedBaselinePreview,E> {
        let request=task_validation::decode(raw).map_err(|e|E::Launch(Box::new(e)))?;
        let before=snapshot(self,work,&request)?;
        let result=material_path::with_material_read(self,|| {
            let tx=self.db.unchecked_transaction().map_err(E::Sql)?;
            let checked=(|| {
                let current=crate::read_recorded_work(&tx,work).map_err(E::Work)?;
                let settings=self.read_settings()?;
                if current!=before.0 || crate::managed_publication_record::digest(&settings).map_err(|e|E::Launch(Box::new(e)))?
                    != crate::managed_publication_record::digest(&before.1).map_err(|e|E::Launch(Box::new(e)))? {
                    return Err(E::Refused("managed-baseline-inputs-changed"));
                }
                inspect(self,&tx,&current,&request.task_id,&settings)
            })();
            finished(checked,tx.rollback().map_err(E::Sql))
        })?;
        let after=snapshot(self,work,&request)?;
        if before.0!=after.0 || crate::managed_publication_record::digest(&before.1).map_err(|e|E::Launch(Box::new(e)))?
            != crate::managed_publication_record::digest(&after.1).map_err(|e|E::Launch(Box::new(e)))? {
            return Err(E::Refused("managed-baseline-inputs-changed"));
        }
        Ok(result)
    }
}
