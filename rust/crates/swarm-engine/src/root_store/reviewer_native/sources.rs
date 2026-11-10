//! Candidate custody outside Root settings/SQL transactions for native reviews.
use super::store::{self, Job, Phase};
use crate::root_store::{finished, task_validation::{self, ReviewCandidate, Request}, RootStore, RootStoreError as E};

/// Own actual descriptors outside every transaction/result wrapper, pass only a
/// borrowed witness, then explicitly close even after callback failure. Source
/// acquisition/refresh precedes the callback; callers still reassemble exact
/// sources in their transaction. A successful writer can change task state, so
/// no post-write new admission is inferred or attempted. No process is granted.
fn held<T>(root:&RootStore, candidate:Option<ReviewCandidate>, consume:impl FnOnce(Option<&ReviewCandidate>)->Result<T,E>)->Result<T,E> {
    let result=(|| {
        if let Some(candidate)=&candidate {candidate.refresh(root)?;}
        consume(candidate.as_ref())
    })();
    finished(result,candidate.map_or(Ok(()),ReviewCandidate::close))
}
/// Pin the current native managed candidate before a new claim; exact old event
/// replay skips new admission and delegates intent/ledger checks to the claim.
/// Actual source/settings/SQL/read/close errors retain causes, never empty success.
pub(super) fn request<T>(root:&RootStore,work:&str,request:&Request,consume:impl FnOnce(Option<&ReviewCandidate>)->Result<T,E>)->Result<T,E> {
    let existing=store::event(&root.db,&request.event_id).map_err(|e|E::Launch(Box::new(e)))?;
    let candidate=if existing.is_some(){None}else{task_validation::acquire_review_candidate(root,work,request)?};
    held(root,candidate,consume)
}
/// Reacquire the actual current candidate for this existing nonterminal job,
/// outside Root locks. Current revision is an internal observation, not rebasing
/// its recorded intent. The later guard still checks every immutable job/frame.
/// Terminal replay reads historical originals without requiring current files.
pub(super) fn job<T>(root:&RootStore,job:&Job,consume:impl FnOnce(Option<&ReviewCandidate>)->Result<T,E>)->Result<T,E> {
    let candidate=if matches!(job.phase,Phase::Recorded|Phase::Refused){None}else{
        let work=root.read_work(&job.work_id)?;
        let request=Request{schema_version:1,event_id:job.event_id.clone(),expected_revision:work.revision,task_id:job.task_id.clone()};
        let review=work.tasks.iter().flatten().find(|t|t.id==job.task_id).and_then(|t|t.independent_review.as_deref())
            .ok_or(E::Refused("review-guard-record-required"))?;
        if review.id!=job.id || review.state!="running"{return Err(E::Refused("review-guard-record-changed"));}
        task_validation::acquire_claimed_review_candidate(root,&job.work_id,&request,Some(review))?
    };
    held(root,candidate,consume)
}
