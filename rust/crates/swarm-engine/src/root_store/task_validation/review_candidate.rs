//! Readonly managed review custody, acquired outside Root SQL/settings locks.
use super::{journal, managed_execution, publication, runtime::Receipt, store, Request, ReviewSource, TaskValidationPreview};
use crate::{root_store::{finished, review_recovery::Recovery, RootStore, RootStoreError as E}, AgentLaunchError as A};
use std::path::PathBuf;

/// Private owned purpose plus the exact retained reviewer record for one
/// readonly custody holder. Not Copy/Clone: project it as the parent
/// ReviewSource borrow every time instead of moving it. Claimed and
/// RecordedOpinion are copied only from the single legacy mapping;
/// ReviewerRetry is stored only through acquire_for_retry and is never
/// inferred from a record's state. The retained record is an observation that
/// grants no permission; every consumer must re-verify it transactionally.
enum ReviewPurpose {
    Fresh,
    Claimed(crate::IndependentReview),
    RecordedOpinion(crate::IndependentReview),
    ReviewerRetry(crate::IndependentReview),
}
impl ReviewPurpose {
    /// Borrow the exact retained record, compared with every caller-supplied
    /// record; a digest or DTO alone never substitutes for it.
    fn record(&self)->Option<&crate::IndependentReview>{
        match self{Self::Fresh=>None,Self::Claimed(r)|Self::RecordedOpinion(r)|Self::ReviewerRetry(r)=>Some(r)}
    }
    /// Project the same parent ReviewSource variant this purpose was stored
    /// with, borrowing the owned record; the record state is never re-derived
    /// here, so a stored ReviewerRetry survives refresh unchanged.
    fn source(&self)->ReviewSource<'_>{
        match self{
            Self::Fresh=>ReviewSource::Fresh,
            Self::Claimed(r)=>ReviewSource::Claimed(r),
            Self::RecordedOpinion(r)=>ReviewSource::RecordedOpinion(r),
            Self::ReviewerRetry(r)=>ReviewSource::ReviewerRetry(r),
        }
    }
    /// Retain the existing legacy mapping without admitting an explicit retry.
    /// Copies only observed record data; an unexpected retry variant refuses
    /// before custody is acquired rather than silently authorizing that purpose.
    fn from_review(review:Option<&crate::IndependentReview>)->Result<Self,A>{
        Ok(match managed_execution::review_source(review){
            ReviewSource::Fresh=>Self::Fresh,
            ReviewSource::Claimed(r)=>Self::Claimed(r.clone()),
            ReviewSource::RecordedOpinion(r)=>Self::RecordedOpinion(r.clone()),
            ReviewSource::ReviewerRetry(_)=>return Err(A::Refused("review-retry-explicit-purpose-required")),
        })
    }
}
/// A readonly inspector owns one existing native candidate and its actual
/// completed control job. Keep outside generic result/cleanup wrappers; no
/// provider grant, future freshness lease or task acceptance is represented.
/// The holder is custody only: it re-verifies the retained purpose/record and
/// inputs on preview/refresh and performs no post-claim refresh, admission or
/// directive delivery; execution sources::held already refreshes before its
/// callback and closes after, and running gates acquire their own Claimed
/// record separately. A ReviewerRetry holder is stored only through
/// acquire_for_retry, which public acquire selects only after a fresh private
/// managed_review_recovery observation; the consuming snapshot still re-verifies
/// the original and canonical decision before any claim. Explicit close errors
/// propagate to the caller.
pub(in crate::root_store) struct ReviewCandidate {
    session: managed_execution::Session,
    job: store::Job,
    metadata: serde_json::Value,
    purpose: ReviewPurpose,
}
impl ReviewCandidate {
    /// Recompute current Root source and bind its initial input inventory to the
    /// original control job. Drift refuses before reviewer/publication, without
    /// Git inside SQL, mutation, fallback or a transferable freshness lease.
    /// An explicit retry holder refuses here before recomputation; it requires
    /// preview_for_retry and current Root-verified recovery observations. Existing
    /// legacy source/read/close refusals are unchanged; no invocation is granted.
    pub(in crate::root_store) fn preview(&self, root: &RootStore, tx: &rusqlite::Connection,
        settings: &crate::EngineSettingsDocument, work: &str, request: &Request,review:Option<&crate::IndependentReview>) -> Result<TaskValidationPreview, A> {
        if matches!(&self.purpose,ReviewPurpose::ReviewerRetry(_)){return Err(A::Refused("review-retry-explicit-purpose-required"));}
        if review!=self.purpose.record(){return Err(A::Refused("review-managed-held-record-changed"));}
        let prepared=managed_execution::prepare_for_review(tx,&root.directory,&root.root,settings,work,request,&self.session.data,review)?;
        let source=managed_execution::bound_source(tx,&self.job,&prepared.preview.source_sha256,Some(self.candidate()))?;
        if source!=self.job.source_sha256 {return Err(A::Refused("managed-review-inputs-stale"));}
        Ok(prepared.preview)
    }
    /// Retry-only source recomputation for a holder created by acquire_for_retry.
    /// Requires the stored purpose to be ReviewerRetry and the recovery's
    /// previous record to equal the retained record exactly, then recomputes the
    /// same complete source with an explicit ReviewerRetry allowance and applies
    /// the identical bound_source equality against the pinned job. Recovery data
    /// are cloneable observations granting no permission: the caller must still
    /// verify the original and canonical decision again transactionally before
    /// any claim, and the shared assembler re-checks the original artifact map
    /// against actually assembled observations. Reachable from the shared
    /// prepared-review snapshot only for a held ReviewerRetry purpose with a
    /// current managed recovery observation; no claim, directive or acceptance
    /// follows and no post-claim lifecycle is represented here.
    pub(in crate::root_store) fn preview_for_retry(&self, root: &RootStore, tx: &rusqlite::Connection,
        settings: &crate::EngineSettingsDocument, work: &str, request: &Request, recovery:&Recovery) -> Result<TaskValidationPreview, A> {
        let stored=match &self.purpose{
            ReviewPurpose::ReviewerRetry(stored)=>stored,
            _=>return Err(A::Refused("review-retry-purpose-required")),
        };
        if recovery.previous!=*stored{return Err(A::Refused("review-retry-previous-changed"));}
        let prepared=managed_execution::prepare_using_source(tx,&root.directory,&root.root,settings,work,request,&self.session.data,ReviewSource::ReviewerRetry(stored))?;
        let source=managed_execution::bound_source(tx,&self.job,&prepared.preview.source_sha256,Some(self.candidate()))?;
        if source!=self.job.source_sha256 {return Err(A::Refused("managed-review-inputs-stale"));}
        Ok(prepared.preview)
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
    /// The retained explicit purpose, including a retry-only ReviewerRetry, is
    /// reused as-is and never re-derived from record state. This source read
    /// grants no post-write admission: execution owners close after writing,
    /// while each running-job gate separately reacquires its exact record.
    pub(in crate::root_store) fn refresh(&self,root:&RootStore)->Result<(),E>{
        let fresh=managed_execution::acquire_using_source(root,&self.job,self.candidate(),self.purpose.source())?;
        let result=if fresh.data==self.session.data{Ok(())}else{Err(E::Refused("task-validation-managed-inputs-changed"))};
        finished(result,fresh.close())
    }
    /// Release every held candidate descriptor and preserve explicit close errors.
    pub(in crate::root_store) fn close(self)->Result<(),E>{self.session.close()}
}

/// Select actual ordinary vs managed Work under short read snapshots. Managed
/// mode requires a published native v3 job and complete successful originals;
/// only then pin its existing candidate after checked snapshot/lock release.
/// Managed Work first obtains a fresh private managed_review_recovery
/// observation: Some selects explicit retry custody through acquire_for_retry,
/// None (ordinary, fresh or no qualified original) keeps the legacy
/// acquire_using(None) mapping, and a real observer error propagates without
/// fallback. No preparation, file/SQL write, reviewer call or capacity
/// reservation occurs.
pub(in crate::root_store) fn acquire(root:&RootStore,work:&str,request:&Request)->Result<Option<ReviewCandidate>,E>{
    match observed_managed_retry_recovery(root,work,request)?{
        Some(recovery)=>acquire_for_retry(root,work,request,&recovery),
        None=>acquire_using(root,work,request,None),
    }
}
/// Internal retained-review variant. Native job callers still require their
/// exact running record; publication inspection may select a completed opinion
/// only after verifying its actual original. Source equality grants no permission.
/// The retained owned purpose copies the single legacy review_source mapping;
/// ReviewerRetry is never produced on this path.
pub(in crate::root_store) fn acquire_using(root:&RootStore,work:&str,request:&Request,review:Option<&crate::IndependentReview>)->Result<Option<ReviewCandidate>,E>{
    let purpose=ReviewPurpose::from_review(review).map_err(launch)?;
    observed_managed_original(root,work,request)?.map(|(job,candidate,metadata)|{
        let session=managed_execution::acquire_for_review(root,&job,&candidate,review)?;
        Ok(ReviewCandidate{session,job,metadata,purpose})
    }).transpose()
}
/// Retry-only custody selection for one exact previously failed review. Takes a
/// Root-verified Recovery observation whose cloneable data grant no permission:
/// the future caller must still verify the original and canonical decision
/// again inside its own transaction before any claim. ReviewerRetry is stored
/// only through this named hook and never inferred from record state, after
/// refusing a previous record that is not in the failed state; the shared
/// transactional guard must also re-observe that exact failed record as the
/// current one. acquire selects this hook only after its own fresh private
/// managed_review_recovery observation of stored current Work; ordinary Work
/// still yields None. No claim, directive, admission or post-claim refresh is
/// performed here.
pub(in crate::root_store) fn acquire_for_retry(root:&RootStore,work:&str,request:&Request,recovery:&Recovery)->Result<Option<ReviewCandidate>,E>{
    if recovery.previous.state!="error"{return Err(E::Refused("review-retry-previous-not-failed"));}
    observed_managed_original(root,work,request)?.map(|(job,candidate,metadata)|{
        let session=managed_execution::acquire_using_source(root,&job,&candidate,ReviewSource::ReviewerRetry(&recovery.previous))?;
        Ok(ReviewCandidate{session,job,metadata,purpose:ReviewPurpose::ReviewerRetry(recovery.previous.clone())})
    }).transpose()
}
/// Private short-read observer deciding whether public acquisition selects
/// explicit retry custody. Preconditions: a short directory reservation and a
/// non-autocommit read transaction over the actual stored Work in the current
/// SQL/settings snapshot; ordinary repository absent returns Ok(None) and the
/// request's expected revision must equal the actual revision. Calls the actual
/// Root managed_review_recovery core with claimed=None and returns only its
/// plain cloneable observation; None means no qualified original, never a
/// relaxed fallback. The read creates no mutation, custody, authority, grant or
/// admission and IO/SQL/codec causes propagate unchanged. The transaction is
/// rolled back and the reservation closed with both causes preserved before any
/// candidate/Git/descriptor custody is acquired.
fn observed_managed_retry_recovery(root:&RootStore,work:&str,request:&Request)->Result<Option<Recovery>,E>{
    let reservation=root.directory.reserve(false,false).map_err(E::Storage)?;
    let observed=(||{
        let tx=root.db.unchecked_transaction().map_err(E::Sql)?;
        let result=(||{
            let current=crate::read_recorded_work(&tx,work).map_err(E::Work)?;
            if !current.planning.as_deref().is_some_and(|p|p.repository.is_some()){return Ok(None);}
            if current.revision!=request.expected_revision{return Err(E::Refused("review-revision-conflict"));}
            let settings=root.read_settings()?;
            root.managed_review_recovery(&tx,&current,&request.task_id,None,&settings)
        })();
        finished(result,tx.rollback().map_err(E::Sql))
    })();
    finished(observed,reservation.close().map_err(E::Storage))
}
/// Snapshot the actual completed native managed original for one work/task
/// request under a short reservation and read transaction, refusing before any
/// custody is taken. None means ordinary Work without a managed repository,
/// never a failed read; every refusal keeps its original typed code and the
/// transaction/rollback and reservation close stay explicitly paired. No
/// preparation, file/SQL write, reviewer call or capacity reservation occurs.
fn observed_managed_original(root:&RootStore,work:&str,request:&Request)->Result<Option<(store::Job,crate::managed_repository::ManagedCandidate,serde_json::Value)>,E>{
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
    finished(observed,reservation.close().map_err(E::Storage))
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
