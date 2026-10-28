//! Readonly managed review custody, acquired outside Root SQL/settings locks.
use super::{journal, managed_execution, publication, runtime::Receipt, store, Request, TaskValidationPreview};
use crate::{root_store::{finished, RootStore, RootStoreError as E}, AgentLaunchError as A};
use std::path::PathBuf;

/// A readonly inspector owns one existing native candidate and its actual
/// completed control job. Keep outside generic result/cleanup wrappers; no
/// provider grant, future freshness lease or task acceptance is represented.
pub(in crate::root_store) struct ReviewCandidate {
    session: managed_execution::Session,
    job: store::Job,
    metadata: serde_json::Value,
    review:Option<crate::IndependentReview>,
}
impl ReviewCandidate {
    /// Recompute current Root source using held observations, without Git inside
    /// the caller's snapshot. Exact native evidence is checked by its consumer.
    pub(in crate::root_store) fn preview(&self, root: &RootStore, tx: &rusqlite::Connection,
        settings: &crate::EngineSettingsDocument, work: &str, request: &Request,review:Option<&crate::IndependentReview>) -> Result<TaskValidationPreview, A> {
        if review!=self.review.as_ref(){return Err(A::Refused("review-managed-held-record-changed"));}
        managed_execution::prepare_for_review(tx,&root.directory,&root.root,settings,work,request,&self.session.data,review).map(|p|p.preview)
    }
    /// Borrow the complete checkpoint; a digest or DTO alone grants no access.
    pub(in crate::root_store) fn candidate(&self) -> &crate::managed_repository::ManagedCandidate { &self.session.data.candidate }
    /// Holder-anchored complete repository path, valid only while this owner lives.
    /// It must not be persisted or returned as a reusable public source reference.
    pub(in crate::root_store) fn repository(&self) -> PathBuf { self.session.repository() }
    /// Retained public-shaped control observations, without kernel/host/path data.
    /// Full original mapping is separately checked before readonly context output.
    pub(in crate::root_store) fn metadata(&self) -> &serde_json::Value { &self.metadata }
    /// Read only a tracked symlink's literal target, never a linked file.
    pub(in crate::root_store) fn link(&self, name:&str) -> Result<String,E> { self.session.review_link(name) }
    /// Reinspect complete raw tracked sources/checkpoint outside Root locks after
    /// materialization, refusing drift. This runs no provider and grants no lease.
    pub(in crate::root_store) fn refresh(&self,root:&RootStore)->Result<(),E>{
        let fresh=managed_execution::acquire_for_review(root,&self.job,self.candidate(),self.review.as_ref())?;
        let result=if fresh.data==self.session.data{Ok(())}else{Err(E::Refused("task-validation-managed-inputs-changed"))};
        finished(result,fresh.close())
    }
    /// Release every held candidate descriptor and preserve explicit close errors.
    pub(in crate::root_store) fn close(self)->Result<(),E>{self.session.close()}
}

/// Select actual ordinary vs managed Work under a short read snapshot. Managed
/// mode requires a published native v3 job and complete successful originals;
/// only then pin its existing candidate after checked snapshot/lock release.
/// No preparation, file/SQL write, reviewer call or capacity reservation occurs.
pub(in crate::root_store) fn acquire(root:&RootStore,work:&str,request:&Request)->Result<Option<ReviewCandidate>,E>{
    acquire_using(root,work,request,None)
}
/// Internal retained-review variant. Native job callers still require their
/// exact running record; publication inspection may select a completed opinion
/// only after verifying its actual original. Source equality grants no permission.
pub(in crate::root_store) fn acquire_using(root:&RootStore,work:&str,request:&Request,review:Option<&crate::IndependentReview>)->Result<Option<ReviewCandidate>,E>{
    let reservation=root.directory.reserve(false,false).map_err(E::Storage)?;
    let observed=(||{
        let tx=root.db.unchecked_transaction().map_err(E::Sql)?;
        let result=(||{
            let current=crate::read_recorded_work(&tx,work).map_err(E::Work)?;
            if !current.planning.as_deref().is_some_and(|p|p.repository.is_some()){return Ok(None);}
            if current.revision!=request.expected_revision{return Err(E::Refused("review-revision-conflict"));}
            let task=current.tasks.iter().flatten().find(|t|t.id==request.task_id).ok_or(E::Refused("review-task-missing"))?;
            let validation=task.automatic_validation.as_deref().ok_or(E::Refused("review-native-controls-required"))?;
            let id=store::recorded_id_for_receipt(&tx,work,&task.id,&validation.receipt).map_err(launch)?
                .ok_or(E::Refused("review-native-controls-required"))?;
            let (receipt,hash):(Receipt,String)=journal::read(root,&journal::name("job",&id))?;
            let job=publication::recorded_original(root,&receipt,&hash)?;
            if receipt.schema_version!=3 || !publication::all_passed(&receipt){return Err(E::Refused("review-managed-controls-required"));}
            let candidate=receipt.managed_candidate.as_ref().ok_or(E::Refused("review-managed-candidate-required"))?;
            if job.work_id!=work || job.task_id!=request.task_id || candidate.work!=work || candidate.task!=request.task_id {
                return Err(E::Refused("review-managed-candidate-mismatch"));
            }
            let metadata=recorded_metadata(&receipt,&job,&hash)?;
            Ok(Some((job,candidate.clone(),metadata)))
        })();
        finished(result,tx.rollback().map_err(E::Sql))
    })();
    let observed=finished(observed,reservation.close().map_err(E::Storage))?;
    observed.map(|(job,candidate,metadata)|{
        let session=managed_execution::acquire_for_review(root,&job,&candidate,review)?;
        Ok(ReviewCandidate{session,job,metadata,review:review.cloned()})
    }).transpose()
}
/// Rebuild the exact public-shaped current review metadata from verified native
/// control originals. Caller must establish job/step custody and complete success
/// first. No caller-supplied hashes or observations are promoted into authority;
/// missing managed plans/preimages and codec failures refuse without mutation.
pub(super) fn recorded_metadata(receipt:&Receipt, job:&store::Job, hash:&str)->Result<serde_json::Value,E>{
    let candidate=receipt.managed_candidate.as_ref().ok_or(E::Refused("review-managed-candidate-required"))?;
    candidate.verify_binding().map_err(launch)?;
    let plan=candidate.control_plan.as_ref().ok_or(E::Refused("review-managed-plan-required"))?;
    let observations:Vec<_>=plan.slots.iter().zip(&receipt.steps).map(|(slot,(original,_))|serde_json::json!({
        "task_id":slot.task_id,"ordinal":slot.ordinal,"transport_id":slot.transport_id,
        "control":slot.control,"result":original.result})).collect();
    Ok(serde_json::json!({"schema_version":1,"inspection_only":true,
        "candidate_commit":candidate.commit,"previous_candidate":candidate.previous,
        "candidate_binding_sha256":candidate.binding_digest().map_err(launch)?,
        "cohort_contract_sha256":candidate.contract,"control_job_id":job.id,
        "control_receipt_sha256":hash,"policies":plan.policies,"control_observations":observations}))
}
/// Retain a typed launch cause behind the actual Root error family.
fn launch(error:A)->E{E::Launch(Box::new(error))}
