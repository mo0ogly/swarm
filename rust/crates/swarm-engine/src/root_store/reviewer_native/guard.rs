//! Reobserve a committed candidate claim. No exec permit or process is created.
use super::{
    material::{self, Material},
    store::{self, Job, Phase},
};
use crate::{
    root_store::{task_validation, RootStore, RootStoreError as E},
    AgentLaunchError as A, EngineSettingsDocument,
};
use rusqlite::Transaction;

/// Preserve a safe static boundary and original typed cause, never private text.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, error)))
}
/// Reject expiry of the originally recorded host observation. Rebuilding host
/// metadata may not renew its TTL. No extra timeout policy or timer is introduced.
pub(super) fn original_fresh(until: &str) -> Result<(), E> {
    let now = crate::system_timestamp().map_err(E::Runtime)?;
    let parse = |text: &str| {
        swarm_wire_json::go_rfc3339::parse(text)
            .map_err(|error| caused("review-guard-clock", error))
    };
    if parse(&now)? >= parse(until)? {
        return Err(E::Refused("review-guard-original-preflight-expired"));
    }
    Ok(())
}
/// Require the recorded owner to be this current OS process, including host and
/// start stamp. Intended for foreground owner checks; a future child gate must
/// independently verify actual socket peer, parent, group and child tuple.
/// No PID supplied by public JSON is trusted and no signal or mutation occurs.
pub(super) fn owner(job: &Job) -> Result<(), E> {
    let actual = crate::SupervisorIdentity::current()
        .map_err(|error| caused("review-guard-owner-observation", error))?;
    if actual.pid() != job.owner_pid
        || actual.process_stamp() != job.owner_stamp
        || actual.host() != job.owner_host
    {
        return Err(E::Refused("review-guard-owner-changed"));
    }
    Ok(())
}
/// Confirm the original owner no longer lives in the same host/boot/PID
/// namespace. PID reuse is distinguished by kernel start stamp, not signalled.
/// Unknown/foreign observations refuse; absence grants no wait, exec or identity.
/// Caller separately requires an original known wait and absent child before any
/// result application. This is local Root recovery, not agent authentication.
pub(super) fn departed(job: &Job) -> Result<(), E> {
    let current = crate::SupervisorIdentity::current()
        .map_err(|e| caused("review-guard-departed-host", e))?;
    if current.host() != job.owner_host {
        return Err(E::Refused("review-guard-owner-host-changed"));
    }
    let pid = u32::try_from(job.owner_pid).map_err(|e| caused("review-guard-departed-pid", e))?;
    match crate::observe_linux_process(pid) {
        Ok(process) if process.process_stamp() != job.owner_stamp => Ok(()),
        Ok(_) => Err(E::Refused("review-guard-owner-still-present")),
        Err(e) if e.confirms_absence() => Ok(()),
        Err(e) => Err(caused("review-guard-owner-presence-unknown", e)),
    }
}
/// Read the exact job, financial reserve, directory, monotonic Work and settings generation
/// in the caller snapshot. This is not admission: original preflight TTL belongs
/// separately to fresh() before any new grant. No process, debit, workspace probe
/// or future lease is created here; IO/SQL/codec refusal remains explicit.
fn frame(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: &EngineSettingsDocument,
    job: &Job,
) -> Result<(crate::Work, task_validation::Request), E> {
    frame_mode(
        root,
        tx,
        Some(settings.revision.0),
        job,
        FundingBoundary::Prospective,
    )
}
/// Non-verdict-only immutable job/funding/Root/intent/current-review frame.
/// Current settings may differ because this creates no verdict or execution.
/// All other original bindings and monotonic Work still apply. No mutation,
/// file read, permission or source applicability is established by this alone.
pub(super) fn retirement_frame(
    root: &RootStore,
    tx: &Transaction<'_>,
    job: &Job,
) -> Result<(crate::Work, task_validation::Request), E> {
    frame_mode(root, tx, None, job, FundingBoundary::KnownResult)
}
/// Lifecycle contract, never an operator setting: prospective start/source
/// observation versus application of a separately verified known original.
#[derive(Clone, Copy)]
enum FundingBoundary {
    Prospective,
    KnownResult,
}
/// Require current source/settings but preserve the existing stored reservation
/// independently of a later future-start tariff/limit. Caller owes original
/// receipt/wait/permission checks; this grants no process or result by itself.
fn result_frame(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: &EngineSettingsDocument,
    job: &Job,
) -> Result<(crate::Work, task_validation::Request), E> {
    frame_mode(
        root,
        tx,
        Some(settings.revision.0),
        job,
        FundingBoundary::KnownResult,
    )
}
/// Common immutable frame. Some revision retains exact existing caller guards;
/// None is only for explicit non-verdict retirement, never grant/publication.
/// SQL/IO/funding/shape errors retain cause and cannot prove a stale dependency.
fn frame_mode(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings_revision: Option<i64>,
    job: &Job,
    funding: FundingBoundary,
) -> Result<(crate::Work, task_validation::Request), E> {
    if store::job(tx, &job.id)
        .map_err(|error| E::Launch(Box::new(error)))?
        .as_ref()
        != Some(job)
        || !matches!(
            job.phase,
            Phase::Claimed | Phase::Registered | Phase::Permitted
        )
    {
        return Err(E::Refused("review-guard-job-changed"));
    }
    match funding {
        FundingBoundary::Prospective => {
            crate::financial_budget::check_reserved_planning_start_budget(
                tx,
                &job.work_id,
                "reviewer",
                &job.id,
            )
        }
        FundingBoundary::KnownResult => {
            crate::financial_budget::check_reserved_planning_result_budget(
                tx,
                &job.work_id,
                "reviewer",
                &job.id,
            )
        }
    }
    .map_err(|error| caused("review-guard-funding", error))?;
    let identity = root.directory.identity().map_err(E::Storage)?;
    if identity.0.to_string() != job.directory_device
        || identity.1.to_string() != job.directory_inode
    {
        return Err(E::Refused("review-guard-root-changed"));
    }
    let work = crate::read_recorded_work(tx, &job.work_id).map_err(E::Work)?;
    // Mission bookkeeping can advance independently. Never rebase the saved
    // claim/intent or accept a snapshot older than it; complete task/control/
    // prompt dependencies are reassembled below by every caller of this frame.
    if work.revision < job.work_revision
        || settings_revision.is_some_and(|r| r != job.settings_revision)
    {
        return Err(E::Refused("review-guard-generation-changed"));
    }
    let review = work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == job.task_id)
        .and_then(|t| t.independent_review.as_deref())
        .ok_or(E::Refused("review-guard-record-required"))?;
    if review.id != job.id
        || review.state != "running"
        || review.attempt != job.attempt_id
        || review.producer != job.producer_id
        || review.contract != job.contract_sha256
    {
        return Err(E::Refused("review-guard-record-changed"));
    }
    let mut request = task_validation::Request {
        schema_version: 1,
        event_id: job.event_id.clone(),
        expected_revision: job
            .work_revision
            .checked_sub(1)
            .ok_or(E::Refused("review-guard-revision-invalid"))?,
        task_id: job.task_id.clone(),
    };
    let raw = crate::encode_model(&request).map_err(E::Encode)?;
    if crate::provider_configuration::configuration_digest(&raw) != job.request_sha256 {
        return Err(E::Refused("review-guard-intent-changed"));
    }
    request.expected_revision = work.revision;
    Ok((work, request))
}
/// Compare complete prompt and fresh admission authority with the immutable claim.
/// Original controls retain their independent receipt binding on reviewer retry;
/// their historical source hash cannot substitute for a new admission grant.
/// This compares only supplied data; it performs no IO or permission decision.
pub(super) fn inputs_match(inputs: &crate::TaskReviewInputs, job: &Job) -> bool {
    inputs_match_source(inputs,job,&job.source_sha256)
}
/// Compare an explicitly checked source projection without altering the actual
/// Job. Only the bound-source adapter supplies a separately verified base value.
fn inputs_match_source(inputs:&crate::TaskReviewInputs,job:&Job,source:&str)->bool{
    let preview = &inputs.preview;
    !(preview.producer_agent_id != job.producer_id
        || preview.attempt_id != job.attempt_id
        || inputs.admission_source_sha256 != source
        || preview.validation.receipt_sha256 != job.validation_receipt_sha256
        || preview.review_contract_sha256 != job.contract_sha256
        || preview.review_provider_digest != job.provider_sha256
        || preview.workflow_prompt_sha256 != job.workflow_sha256
        || inputs.prompt_sha256 != job.prompt_sha256)
}
/// Bind complete current materials to the normalized automatic source domain.
/// Manual jobs use the original pure comparator; missing/changed automatic
/// linkage refuses the hash comparison. This reads data, never grants a lease.
pub(super) fn bound_inputs_match(db:&rusqlite::Connection,inputs:&crate::TaskReviewInputs,job:&Job)->Result<bool,E>{
    let binding=super::mission_authority::read(db,&job.id).map_err(|e|E::Launch(Box::new(e)))?;
    if binding.is_none(){return Ok(inputs_match(inputs,job));}
    let observed=super::mission_authority::digest(&inputs.admission_source_sha256,binding.as_ref()).map_err(|e|E::Launch(Box::new(e)))?;
    Ok(observed==job.source_sha256 && inputs_match_source(inputs,job,&inputs.admission_source_sha256))
}
/// Reobserve current task originals after admission, without renewing or treating
/// the admission TTL as a provider duration. Caller must separately own the live
/// process and retain its committed permission; success grants no new exec.
/// Same Root/SQL/settings generation, reserve, canonical controls/workflow and
/// original prompt must agree. This reads bounded materials and mutates no Work,
/// ledger or workspace probe. Failure is revocation data for the owned controller,
/// never automatic acceptance, retry or refund. Legacy wait adoption still refuses.
#[cfg(test)]
pub(super) fn observe_sources(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: EngineSettingsDocument,
    job: &Job,
) -> Result<(), E> {
    super::mission_authority::observe(root,tx,job,&settings).map_err(|e|E::Launch(Box::new(e)))?;
    publication_inputs(root, tx, settings, job).map(|_| ())
}
/// Rebuild current Work and complete original materials after admission. Exact
/// ledger/reserve/settings/source/prompt bindings still apply; no original TTL
/// renewal, host probe, new grant, Work mutation or business publication occurs.
/// This is an observation, not a filesystem lease. Caller owns SQL/settings
/// snapshots and must recheck immediately before staging any publication.
#[cfg(test)]
pub(super) fn publication_inputs(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: EngineSettingsDocument,
    job: &Job,
) -> Result<(crate::Work, crate::TaskReviewInputs), E> {
    let (work, inputs) = publication_dependencies(root, tx, settings, job)?;
    if !bound_inputs_match(tx,&inputs, job)? {
        return Err(E::Refused("review-guard-preparation-changed"));
    }
    Ok((work, inputs))
}
/// Observe complete current dependencies with the strict immutable frame but
/// without declaring equality to the old source/prompt. Used by non-verdict
/// preparation to compare actual typed data, never to catch an arbitrary error.
/// Assembly errors retain cause. No new permission, IO mutation or publication.
#[cfg(test)]
pub(super) fn publication_dependencies(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: EngineSettingsDocument,
    job: &Job,
) -> Result<(crate::Work, crate::TaskReviewInputs), E> {
    dependencies(root, tx, settings, job, FundingBoundary::Prospective,None)
}
/// Observe complete current dependencies for application of a known original.
/// Current sources/settings remain exact; future-start reserve/limit does not
/// reprice the stored reservation. Caller must separately require known wait and
/// permission; no grant, source lease, file or business result is created here.
pub(super) fn result_dependencies(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: EngineSettingsDocument,
    job: &Job,
) -> Result<(crate::Work, crate::TaskReviewInputs), E> {
    dependencies(root, tx, settings, job, FundingBoundary::KnownResult,None)
}
/// Require the full unchanged result dependencies plus its authoritative stored
/// reservation. This is result-only observation, not authorization of any new exec.
/// Source/IO/shape/financial errors retain causes; no ledger/Work mutation occurs.
#[cfg(test)]
pub(super) fn result_inputs(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: EngineSettingsDocument,
    job: &Job,
) -> Result<(crate::Work, crate::TaskReviewInputs), E> {
    let (work, inputs) = result_dependencies(root, tx, settings, job)?;
    if !bound_inputs_match(tx,&inputs, job)? {
        return Err(E::Refused("review-guard-preparation-changed"));
    }
    Ok((work, inputs))
}
/// Shared complete assembly under explicit internal lifecycle funding semantics.
/// No caller-supplied mode, fallback or new permit; propagate all read failures.
fn dependencies(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: EngineSettingsDocument,
    job: &Job,
    funding: FundingBoundary,
    source:Option<&task_validation::ReviewCandidate>,
) -> Result<(crate::Work, crate::TaskReviewInputs), E> {
    material::require_native_wait(root)?;
    let (work, request) = match funding {
        FundingBoundary::Prospective => frame(root, tx, &settings, job)?,
        FundingBoundary::KnownResult => result_frame(root, tx, &settings, job)?,
    };
    let review = work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == job.task_id)
        .and_then(|t| t.independent_review.as_deref())
        .ok_or(E::Refused("review-guard-record-required"))?;
    let prepared =
        root.prepare_review_snapshot_using(tx, settings, &job.work_id, &request, Some(review),source)?;
    let inputs=root.assemble_review_inputs_using(tx,prepared,source,true)?;
    Ok((work, inputs))
}
/// Rebuild complete material from a checked current ledger/Work/settings snapshot,
/// allowing only the exact running review already attached by the atomic claim.
/// Independent Work revisions may advance; complete original dependencies and
/// provider/executable/environment/prompt/source and saved generation must still match.
/// Caller owns the SQL/settings snapshot and phase-specific owner/child checks.
/// This observation grants no permission, file lease, process or result. Host
/// preparation writes/removes its workspace compatibility probe; it mutates no
/// Work/ledger and preserves probe errors.
/// Typed failures and original expiry refuse; no new budget/call is reserved.
#[cfg(test)]
pub(super) fn fresh(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: EngineSettingsDocument,
    job: &Job,
) -> Result<Material, E> {
    fresh_using(root,tx,settings,job,None)
}
/// Source-aware native observation: borrow an external holder while preserving
/// exact frame, consent, funding, original TTL and provider/host guards.
pub(super) fn fresh_using(root:&RootStore,tx:&Transaction<'_>,settings:EngineSettingsDocument,job:&Job,source:Option<&task_validation::ReviewCandidate>)->Result<Material,E>{
    let (work, request) = frame(root, tx, &settings, job)?;
    super::mission_authority::observe(root,tx,job,&settings).map_err(|e|E::Launch(Box::new(e)))?;
    original_fresh(&job.preflight_until)?;
    let review = work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == job.task_id)
        .and_then(|t| t.independent_review.as_deref())
        .ok_or(E::Refused("review-guard-record-required"))?;
    let material =
        material::prepare_for_job_using(root, tx, settings, &job.work_id, &request, Some(review),source)?;
    if !bound_inputs_match(tx,&material.inputs, job)? {
        return Err(E::Refused("review-guard-preparation-changed"));
    }
    if material.host.executable_digest != job.executable_sha256
        || material.host.preflight.scope != job.host_scope
    {
        return Err(E::Refused("review-guard-preparation-changed"));
    }
    material
        .fresh(&root.root)
        .map_err(|error| E::Launch(Box::new(error)))?;
    original_fresh(&job.preflight_until)?;
    Ok(material)
}

/// Rebuild complete currently held inputs for source polling/publication without
/// reacquiring Git custody under SQL/settings locks. No grant or mutation.
pub(super) fn held_inputs(root:&RootStore,tx:&Transaction<'_>,settings:EngineSettingsDocument,job:&Job,source:Option<&task_validation::ReviewCandidate>,result:bool)->Result<(crate::Work,crate::TaskReviewInputs),E>{
    let boundary=if result{FundingBoundary::KnownResult}else{FundingBoundary::Prospective};
    let (work,inputs)=dependencies(root,tx,settings,job,boundary,source)?;
    if !bound_inputs_match(tx,&inputs,job)?{return Err(E::Refused("review-guard-preparation-changed"));}
    Ok((work,inputs))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Protocol timestamp data, not verification deadlines or operator policy.
    /// The original observation cannot be renewed by a newly computed host TTL.
    #[test]
    fn reviewer_guard_requires_original_fresh_timestamp() {
        original_fresh("9999-12-31T23:59:59Z").unwrap();
        assert!(matches!(
            original_fresh("1970-01-01T00:00:00Z"),
            Err(E::Refused("review-guard-original-preflight-expired"))
        ));
        original_fresh("9999-12-31T23:59:59Z").unwrap();
        assert!(original_fresh("invalid timestamp").is_err());
    }
}
