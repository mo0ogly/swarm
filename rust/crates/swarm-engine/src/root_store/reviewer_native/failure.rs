//! Original failed-call preparation; never a criterion verdict or new permit.
use super::{
    journal,
    publication::{self, Ready},
    store::{self, Phase},
};
use crate::{
    root_store::{RootStore, RootStoreError as E},
    AgentLaunchError as A,
};
use rusqlite::Transaction;
/// Preserve original IO/codec/clock causes behind a safe static failure family.
fn caused(code: &'static str, e: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, e)))
}
/// Prepare a non-applicable error from the exact original receipt2 and current
/// matching running review. Require the selected owner boundary, committed permission,
/// known wait and actual child absence. An explicit confirmed-departed boundary
/// may replace the current-owner check; it never adopts or guesses process end.
/// Source changes may explain the failure and do not make it a fresh verdict.
/// Current task status is preserved. Missing/malformed originals, replaced review,
/// legacy/unknown wait, clock or OS uncertainty refuse with no mutation. Spending
/// remains held and private causes/facts remain original, never fabricated cost0.
/// Caller owns settings/SQL snapshots and must repeat this after artifact IO.
pub(super) fn prepare(
    root: &RootStore,
    tx: &Transaction<'_>,
    id: &str,
    owner: publication::OwnerBoundary,
) -> Result<Ready, E> {
    let (receipt, hash) = journal::read(root, id)?;
    let job = store::job(tx, id)
        .map_err(|e| E::Launch(Box::new(e)))?
        .ok_or(E::Refused("review-failure-job-required"))?;
    if job != receipt.job
        || job.phase != Phase::Permitted
        || job.permission != 1
        || receipt.schema_version != 2
        || !receipt.failed
        || !receipt.process_attempted
        || !receipt.grant_attempted
        || !receipt.channel_closed
        || !receipt.waited
    {
        return Err(E::Refused("review-failure-original-not-applicable"));
    }
    publication::quiescent(&job, owner)?;
    let work = crate::read_recorded_work(tx, &job.work_id).map_err(E::Work)?;
    if work.revision < job.work_revision {
        return Err(E::Refused("review-failure-work-before-claim"));
    }
    let task = work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == job.task_id)
        .ok_or(E::Refused("review-failure-task-required"))?;
    let mut review = task
        .independent_review
        .as_deref()
        .cloned()
        .ok_or(E::Refused("review-failure-record-required"))?;
    if review.id != job.id
        || review.state != "running"
        || review.attempt != job.attempt_id
        || review.producer != job.producer_id
        || review.contract != job.contract_sha256
        || !task
            .attempts
            .iter()
            .flatten()
            .any(|a| a.id == job.attempt_id)
        || review.usage.is_some()
        || review.managed_tasks.is_some()
        || review.fragment_journal.is_some()
        || review.batches.is_some()
    {
        return Err(E::Refused("review-failure-record-replaced-or-unported"));
    }
    let parse = |s: &str| {
        swarm_wire_json::go_rfc3339::parse(s).map_err(|e| caused("review-failure-clock", e))
    };
    if parse(&receipt.finished_at)? < parse(&review.started)? {
        return Err(E::Refused("review-failure-clock-order"));
    }
    // Static diagnostic vocabulary, never provider text, private paths or a
    // configurable retry/stop policy. The complete causes remain in the journal.
    let mut reasons = Vec::new();
    if receipt.duration_expired {
        reasons.push("duration-expired");
    }
    if receipt.drain_expired {
        reasons.push("drain-expired");
    }
    if receipt.source_interrupted {
        reasons.push(if receipt.causes.iter().any(|cause|cause.labels.iter().any(|label|label=="review-execution-interrupted")){"execution-interrupted"}else{"source-revoked"});
    }
    if receipt.raw_capture.as_ref().is_some_and(|r| r.failed) {
        reasons.push("capture-failed");
    }
    if receipt.signal.is_some() || receipt.exit_code.is_some_and(|c| c != 0) {
        reasons.push("process-failed");
    }
    if !receipt.output_complete {
        reasons.push("response-incomplete-or-invalid");
    }
    review.state = "error".into();
    review.reason = if reasons.is_empty() {
        "native-review-call-failed".into()
    } else {
        format!("native-review-call-failed: {}", reasons.join(", "))
    };
    review.criteria = Default::default();
    review.finished = receipt.finished_at.clone();
    review.result_event = format!("{}-result", job.id);
    review.reply_sha256 = receipt
        .original_reply
        .as_deref()
        .map(crate::provider_configuration::configuration_digest)
        .unwrap_or_default();
    Ok(Ready {
        receipt,
        hash,
        work,
        review,
    })
}
