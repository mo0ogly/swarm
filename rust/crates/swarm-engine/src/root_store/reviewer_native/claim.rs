//! Atomic reviewer claim used by normal Root/CLI execution and owned SDK checks.
//! A staged record is never an exec permit, process observation or acceptance.
use super::{
    material::{self, Material},
    store::{self, Job, Phase},
};
use crate::{
    root_store::{task_validation, RootStore, RootStoreError as E},
    AgentLaunchError as A, IndependentReview, Work,
};
use rusqlite::Transaction;

/// Staged SQL state plus frozen observations. The caller must commit successfully
/// before process setup, and obtain a separately verified private gate permission.
/// This private type has no Debug output or public deserialization surface.
pub(super) struct Staged {
    pub job: Job,
    pub work: Work,
    pub material: Material,
}
/// Preserve original typed causes behind a static private claim family.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, error)))
}
/// Stage record/call count, normalized provenance and monetary reserve atomically
/// in caller-owned actual SQL/settings snapshots. Exact existing request keys
/// refuse for the outer replay/reconciliation reader; conflicting keys refuse.
/// Active jobs cannot be replaced. A same-attempt error requires Root-verified
/// canonical recovery; it is archived only after every source/funding check.
/// New admission source and original control receipt are bound separately. Historical
/// prior-attempt review records are preserved. Errors require caller rollback;
/// no commit, durable input/output file, grant, provider call, refund or task
/// acceptance occurs. Host preparation creates/syncs/removes a workspace probe;
/// its IO/cleanup failures refuse and remain observable even after SQL rollback.
#[cfg(test)]
pub(super) fn stage(
    root_store: &RootStore,
    tx: &mut Transaction<'_>,
    settings: crate::EngineSettingsDocument,
    work: &str,
    request: &task_validation::Request,
) -> Result<Staged, E> {
    stage_using(root_store,tx,settings,work,request,None,None)
}
/// Engine-owned mission variant. An actual borrowed lease and current mission
/// consent are checked inside this writer before host preparation. Binding, Job,
/// call debit and reserve are staged atomically; no request field grants rights.
fn stage_using(root_store:&RootStore,tx:&mut Transaction<'_>,settings:crate::EngineSettingsDocument,work:&str,request:&task_validation::Request,lease:Option<&crate::MissionLease>,source:Option<&task_validation::ReviewCandidate>)->Result<Staged,E>{
    if let Some(lease)=lease {
        use crate::work_engine::MutationFence;
        lease.check(tx,work,&crate::system_timestamp().map_err(|e|caused("review-mission-clock",e))?)
            .map_err(|e|caused("review-mission-lease",e))?;
        super::mission_authority::enabled(tx,work).map_err(|e|E::Launch(Box::new(e)))?;
    }
    if !root_store.writable {
        return Err(E::Refused("root-read-only"));
    }
    let intent = crate::encode_model(request).map_err(E::Encode)?;
    let request_sha256 = crate::provider_configuration::configuration_digest(&intent);
    if let Some(old) = store::event(tx, &request.event_id).map_err(|e| E::Launch(Box::new(e)))? {
        if old.work_id != work
            || old.task_id != request.task_id
            || old.request_sha256 != request_sha256
        {
            return Err(E::Refused("review-event-conflict"));
        }
        return Err(E::Refused("review-claim-already-recorded"));
    }
    if store::active(tx, work, &request.task_id)
        .map_err(|e| E::Launch(Box::new(e)))?
        .is_some()
    {
        return Err(E::Refused("review-job-active"));
    }
    let material = material::prepare_for_job_using(root_store,tx,settings,work,request,None,source)?;
    let preview = &material.inputs.preview;
    let mut candidate = material.work.clone();
    let task = candidate
        .tasks
        .as_ref()
        .and_then(|tasks| tasks.iter().find(|task| task.id == preview.task_id))
        .ok_or(E::Refused("review-task-missing"))?;
    if let Some(old) = task.independent_review.as_deref() {
        if old.state == "running" {
            return Err(E::Refused("review-running-state-requires-reconciliation"));
        }
        if old.attempt == preview.attempt_id && material.recovery.as_ref().is_none_or(|r| &r.previous != old) {
            return Err(E::Refused("review-attempt-already-reviewed"));
        }
    }
    let review_id =
        crate::generate_identifier("review-").map_err(|e| caused("review-identifier", e))?;
    let at = crate::system_timestamp().map_err(|e| caused("review-claim-clock", e))?;
    let old_revision = candidate.revision;
    candidate.revision = old_revision
        .checked_add(1)
        .ok_or(E::Refused("review-revision-overflow"))?;
    candidate.updated = at.clone();
    let reviewer = candidate
        .planning
        .as_mut()
        .and_then(|p| p.reviewer.as_mut())
        .ok_or(E::Refused("reviewer_required"))?;
    if reviewer.calls < 0 || reviewer.calls >= reviewer.max_calls {
        return Err(E::Refused("review-call-budget-exhausted"));
    }
    reviewer.calls = reviewer
        .calls
        .checked_add(1)
        .ok_or(E::Refused("review-call-count-overflow"))?;
    let record = IndependentReview {
        id: review_id.clone(),
        attempt: preview.attempt_id.clone(),
        producer: preview.producer_agent_id.clone(),
        reviewer: format!("reviewer://{}", material.config.provider),
        report: material.submitted.report_path.clone(),
        sha256: material.submitted.report_sha256.clone(),
        contract: preview.review_contract_sha256.clone(),
        state: "running".into(),
        reason: "review-in-progress".into(),
        started: at,
        timeout_seconds: material.config.timeout_seconds,
        workflow: Some(Box::new(material.workflow.clone())),
        model_route: material.config.model_route.clone(),
        report_artifacts: Some(material.artifacts.clone()),
        ..Default::default()
    };
    let target = candidate
        .tasks
        .as_mut()
        .and_then(|tasks| tasks.iter_mut().find(|task| task.id == preview.task_id))
        .ok_or(E::Refused("review-task-missing"))?;
    if let Some(old) = target.independent_review.take() {
        let history = target.previous_reviews.get_or_insert_with(Vec::new);
        if !history.iter().any(|r| r.id == old.id) {
            history.push(*old);
        }
    }
    target.independent_review = Some(Box::new(record));
    let owner =
        crate::SupervisorIdentity::current().map_err(|e| caused("review-owner-observation", e))?;
    // Bind the private SQL/journal directory separately from the workspace
    // identity already included in the immutable host_scope observation.
    let identity = root_store.directory.identity().map_err(E::Storage)?;
    let mut job = Job {
        id: review_id.clone(),
        work_id: work.into(),
        task_id: preview.task_id.clone(),
        event_id: request.event_id.clone(),
        request_sha256,
        producer_id: preview.producer_agent_id.clone(),
        attempt_id: preview.attempt_id.clone(),
        source_sha256: material.inputs.admission_source_sha256.clone(),
        validation_receipt_sha256: preview.validation.receipt_sha256.clone(),
        contract_sha256: preview.review_contract_sha256.clone(),
        provider_sha256: preview.review_provider_digest.clone(),
        executable_sha256: material.host.executable_digest.clone(),
        host_scope: material.host.preflight.scope.clone(),
        workflow_sha256: preview.workflow_prompt_sha256.clone(),
        prompt_sha256: material.inputs.prompt_sha256.clone(),
        work_revision: candidate.revision,
        settings_revision: material.settings.revision.0,
        owner_pid: owner.pid(),
        owner_stamp: owner.process_stamp().into(),
        owner_host: owner.host().into(),
        directory_device: identity.0.to_string(),
        directory_inode: identity.1.to_string(),
        preflight_until: material.host.preflight.valid_until.clone(),
        child_pid: None,
        parent_pid: None,
        group_id: None,
        child_stamp: None,
        permission: 0,
        phase: Phase::Claimed,
        receipt_sha256: None,
        published_revision: None,
    };
    let binding=lease.map(|lease|super::mission_authority::claim(root_store,tx,lease,&job,&material.settings)).transpose().map_err(|e|E::Launch(Box::new(e)))?;
    job.source_sha256=super::mission_authority::digest(&job.source_sha256,binding.as_ref()).map_err(|e|E::Launch(Box::new(e)))?;
    crate::financial_budget::reserve_planning_call(
        tx,
        work,
        "reviewer",
        &review_id,
        &candidate.updated,
    )
    .map_err(|e| caused("review-budget-reservation", e))?;
    store::claim(tx, &job).map_err(|e| E::Launch(Box::new(e)))?;
    if let Some(binding)=binding.as_ref(){super::mission_authority::stage(tx,binding).map_err(|e|E::Launch(Box::new(e)))?;}
    let result = crate::stage_recorded_change(
        tx,
        crate::RecordedChange {
            work: &candidate,
            expected_revision: old_revision,
            event: &request.event_id,
            kind: "review.claim",
            request: &intent,
        },
    )
    .map_err(|e| caused("review-claim-publication", e))?;
    if !matches!(result, crate::RecordedChangeOutcome::Staged) {
        return Err(E::Refused("review-unexpected-claim-replay"));
    }
    Ok(Staged {
        job,
        work: candidate,
        material,
    })
}

/// New committed state or exact previously recorded job. Neither is a process
/// permission or result: replay must inspect original receipt/owner, never rerun.
pub(super) enum Recorded {
    New(Box<Staged>),
    Replay(Box<Job>),
}
/// Finalize one actual Root transaction without hiding rollback or commit doubt.
/// BEGIN IMMEDIATE runtime-checks exclusive SQL ownership on the same connection;
/// direct BUSY can follow the saved storage policy, while commit/cleanup failures
/// never retry. A failed commit with autocommit already true is unknown, not PASS.
/// No provider or grant is invoked inside this callback.
fn commit(
    root: &RootStore,
    settings: crate::EngineSettingsDocument,
    work: &str,
    request: &task_validation::Request,
    lease: Option<&crate::MissionLease>,
    source:Option<&task_validation::ReviewCandidate>,
) -> Result<Recorded, E> {
    super::transaction::apply(root, |tx| {
        let intent = crate::encode_model(request).map_err(E::Encode)?;
        let digest = crate::provider_configuration::configuration_digest(&intent);
        if let Some(old) =
            store::event(tx, &request.event_id).map_err(|error| E::Launch(Box::new(error)))?
        {
            if old.work_id != work || old.task_id != request.task_id || old.request_sha256 != digest
            {
                return Err(E::Refused("review-event-conflict"));
            }
            return Ok(super::transaction::Change::Observe(Recorded::Replay(
                Box::new(old),
            )));
        }
        let staged=stage_using(root,tx,settings,work,request,lease,source);
        staged.map(|new| super::transaction::Change::Mutate(Recorded::New(Box::new(new))))
    })
}
/// Record the atomic claim through the common saved storage policy, using this
/// same Root connection and fresh settings for each eligible SQL attempt. Exact
/// replay returns the original job without reservation/source re-execution.
/// Caller must reconcile/read its result, never start another process from Replay.
/// A successful commit is marked before lock release, so a later close failure
/// retains known Work identity/revision. Commit uncertainty stays non-retryable;
/// no false rollback/refund, provider invocation, result or acceptance is created.
#[cfg(test)]
pub(super) fn record(
    root: &RootStore,
    work: &str,
    request: &task_validation::Request,
) -> Result<Recorded, E> {
    record_using(root,work,request,None)
}
/// Preserve manual record/replay semantics while forwarding actual engine-owned
/// mission authority only into a new admission. Historical replay never grants a
/// provider invocation; original budget and known Work errors remain intact.
#[cfg(test)]
pub(super) fn record_using(root:&RootStore,work:&str,request:&task_validation::Request,lease:Option<&crate::MissionLease>)->Result<Recorded,E>{
    record_using_candidate(root,work,request,lease,None)
}
/// Claim from borrowed externally held managed sources. The caller owns their
/// lifetime through execution, and explicit close after all recorded outcomes.
/// Same CAS, event replay, mission consent, call debit and reserve contracts.
pub(super) fn record_using_candidate(root:&RootStore,work:&str,request:&task_validation::Request,lease:Option<&crate::MissionLease>,source:Option<&task_validation::ReviewCandidate>)->Result<Recorded,E>{
    if !root.writable {
        return Err(E::Refused("root-read-only"));
    }
    let mut known = None;
    let result = crate::agent_storage_retry::execute_borrowed(
        &root.db,
        &root.directory,
        false,
        |_, settings| {
            let result = commit(root, settings.clone(), work, request, lease, source)?;
            match &result {
                Recorded::New(new) => known = Some((new.work.id.clone(), new.work.revision)),
                Recorded::Replay(job) => {
                    let current = root.read_work(&job.work_id)?;
                    known = Some((current.id, current.revision));
                }
            }
            Ok(result)
        },
    );
    result.map_err(|primary| match known {
        Some((work, revision)) => E::WorkRecorded {
            primary: Box::new(primary),
            work,
            revision,
        },
        None => primary,
    })
}
