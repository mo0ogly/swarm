//! Explicit non-verdict retirement from a genuine waited successful original.
use super::{
    guard, journal,
    publication::{self, Ready},
    store::{self, Phase},
};
use crate::{
    root_store::{task_validation, RootStore, RootStoreError as E},
    AgentLaunchError as A,
};
use rusqlite::Transaction;
/// Keep original read/codec/clock failures under safe static diagnostics.
fn caused(code: &'static str, e: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, e)))
}
/// Observe a concrete changed dependency from typed state or successfully read
/// current bytes. An IO/SQL/invalid policy/preparation error is never caught as
/// staleness. Same settings/SQL snapshot belongs to caller; this is no file lease.
/// Settings generation is the existing strict verdict contract, not a new TTL.
fn changed(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: &crate::EngineSettingsDocument,
    job: &store::Job,
    work: &crate::Work,
    request: &task_validation::Request,
    review: &crate::IndependentReview,
) -> Result<&'static str, E> {
    if settings.revision.0 != job.settings_revision {
        return Ok("settings-changed");
    }
    if let Some(planning) = work.planning.as_deref() {
        if planning.paused || !planning.failure.is_empty() {
            return Ok("planning-inactive");
        }
    }
    let policy = settings
        .values
        .context_policy_limits()
        .map_err(E::Settings)?;
    let report = crate::read_review_delivery(&root.root, &review.report, &policy)
        .map_err(|e| caused("review-stale-report", e))?;
    if crate::provider_configuration::configuration_digest(report.as_bytes()) != review.sha256 {
        return Ok("report-changed");
    }
    let source = task_validation::prepare_claimed_review_preview(
        tx,
        &root.root,
        settings,
        &job.work_id,
        request,
        review,
    )
    .map_err(|e| E::Launch(Box::new(e)))?;
    if super::mission_authority::source(tx,job,&source.source_sha256).map_err(|e|E::Launch(Box::new(e)))? != job.source_sha256 {
        return Ok("validation-source-changed");
    }
    let (_, inputs) = guard::result_dependencies(root, tx, settings.clone(), job)?;
    if !guard::bound_inputs_match(tx,&inputs, job)? {
        return Ok("review-inputs-changed");
    }
    Err(E::Refused("review-stale-original-still-applicable"))
}
/// Prepare only a known successful receipt2 with committed permission, exact
/// immutable job/funding/Root/intent/current running review and known wait/absent
/// child under the selected owner boundary. Require an actual dependency change;
/// uncertainty and unchanged originals refuse without mutation. The caller must
/// repeat after artifact IO before atomic non-verdict application. Retain task
/// state, original reply/digest/time and held estimate; no provider or settlement.
/// Historical result integrity is distinct from authenticated applying identity.
pub(super) fn prepare(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: &crate::EngineSettingsDocument,
    id: &str,
    owner: publication::OwnerBoundary,
) -> Result<Ready, E> {
    let (receipt, hash) = journal::read(root, id)?;
    let job = store::job(tx, id)
        .map_err(|e| E::Launch(Box::new(e)))?
        .ok_or(E::Refused("review-stale-job-required"))?;
    if job != receipt.job
        || job.phase != Phase::Permitted
        || job.permission != 1
        || receipt.schema_version != 2
        || receipt.failed
        || !receipt.output_complete
        || !receipt.process_attempted
        || !receipt.grant_attempted
        || !receipt.channel_closed
        || !receipt.waited
        || receipt.exit_code != Some(0)
        || receipt.original_reply.is_none()
        || receipt.raw_capture.as_ref().is_none_or(|r| r.failed)
    {
        return Err(E::Refused("review-stale-original-not-applicable"));
    }
    publication::quiescent(&job, owner)?;
    let (work, request) = guard::retirement_frame(root, tx, &job)?;
    let mut review = work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == job.task_id)
        .and_then(|t| t.independent_review.as_deref())
        .cloned()
        .ok_or(E::Refused("review-stale-record-required"))?;
    if review.usage.is_some()
        || review.managed_tasks.is_some()
        || review.fragment_journal.is_some()
        || review.batches.is_some()
    {
        return Err(E::Refused("review-stale-record-unported"));
    }
    let task = work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == job.task_id)
        .ok_or(E::Refused("review-stale-task-required"))?;
    let submitted =
        task_validation::submission(tx, &work, &job.task_id).map_err(|e| E::Launch(Box::new(e)))?;
    if !task
        .attempts
        .iter()
        .flatten()
        .any(|a| a.id == job.attempt_id)
        || review.report != submitted.report_path
        || review.sha256 != submitted.report_sha256
    {
        return Err(E::Refused("review-stale-submission-replaced"));
    }
    let reason = changed(root, tx, settings, &job, &work, &request, &review)?;
    let parse = |s: &str| {
        swarm_wire_json::go_rfc3339::parse(s).map_err(|e| caused("review-stale-clock", e))
    };
    if parse(&receipt.finished_at)? < parse(&review.started)? {
        return Err(E::Refused("review-stale-clock-order"));
    }
    review.state = "stale".into();
    review.reason = format!("native-review-original-stale: {reason}");
    review.criteria = Default::default();
    review.finished = receipt.finished_at.clone();
    review.result_event = format!("{}-result", job.id);
    review.reply_sha256 = crate::provider_configuration::configuration_digest(
        receipt
            .original_reply
            .as_deref()
            .ok_or(E::Refused("review-stale-reply-required"))?,
    );
    Ok(Ready {
        receipt,
        hash,
        work,
        review,
    })
}
