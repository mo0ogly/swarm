//! Explicit archival of a verified failed managed review without another call.
use super::{journal, publication, store, transaction};
use crate::{root_store::{RootStore, RootStoreError as E}, IndependentReview, Work};
use rusqlite::{OptionalExtension, Transaction};

const EVENT_KIND: &str = "review.archive-failure";

/// Return the exact terminal managed error and its receipt binding. A refusal
/// with a reply, ambiguous wait, revoked source or uncertain capture cannot
/// release the current error through this operation.
fn original(
    root: &RootStore, tx: &Transaction<'_>, job: &store::Job,
) -> Result<(IndependentReview, String), E> {
    if job.phase != store::Phase::Refused {
        return Err(E::Refused("review-archive-terminal-failure-required"));
    }
    let (receipt, hash) = journal::read(root, &job.id)?;
    if receipt.original_reply.is_some() || receipt.output_complete
        || receipt.managed_review.is_none() || !receipt.failed
        || receipt.gate.is_some() || receipt.source_interrupted
        || receipt.duration_expired || receipt.drain_expired
        || !receipt.exit_code.is_some_and(|code| code != 0)
        || receipt.raw_capture.as_ref().is_none_or(|capture| capture.failed)
    {
        return Err(E::Refused("review-archive-managed-process-failure-required"));
    }
    let checked = root.recorded_native_review(tx, &job.work_id, &job.id)?;
    if checked.task != job.task_id || checked.verdict_applicable
        || checked.review.state != "error" || checked.original_receipt_sha256 != hash {
        return Err(E::Refused("review-archive-original-changed"));
    }
    Ok((checked.review, hash))
}

/// Verify the existing archive event against the exact failure and current
/// history. Absence means this operation never released that review. The
/// marker does not confer a new budget reservation or source custody.
pub(super) fn verified(
    root: &RootStore, tx: &Transaction<'_>, work: &Work, task: &str, review: &IndependentReview,
) -> Result<bool, E> {
    let event_id = format!("{}-archive-failed", review.id);
    let marker: Option<(String, String, i64, Vec<u8>)> = tx.query_row(
        "SELECT work_id,kind,revision,request FROM events WHERE id=?",
        [&event_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))
    ).optional().map_err(E::Sql)?;
    let Some((marker_work, kind, revision, request)) = marker else { return Ok(false); };
    let job = store::job(tx, &review.id).map_err(|e| E::Launch(Box::new(e)))?
        .ok_or(E::Refused("review-archive-job-required"))?;
    let (saved, hash) = original(root, tx, &job)?;
    if marker_work != work.id || job.work_id != work.id || job.task_id != task
        || kind != EVENT_KIND || revision > work.revision
        || job.published_revision.is_none_or(|published| published >= revision)
        || request != crate::encode_model(&hash).map_err(E::Encode)? || &saved != review {
        return Err(E::Refused("review-archive-marker-changed"));
    }
    Ok(true)
}

/// Archive the exact current terminal failure and preserve the original,
/// inbox, financial ledger and candidate. Exact replay checks the historical
/// marker. The transaction creates no verdict, new call or approval.
pub(super) fn record(root: &RootStore, id: &str) -> Result<publication::Outcome, E> {
    if !root.writable { return Err(E::Refused("root-read-only")); }
    if !swarm_identity_contract::name(id.as_bytes()) {
        return Err(E::Refused("review-recovery-id-invalid"));
    }
    crate::agent_storage_retry::execute_borrowed(&root.db, &root.directory, false, |_, _| {
        transaction::apply(root, |tx| {
            let job = store::job(tx, id).map_err(|e| E::Launch(Box::new(e)))?
                .ok_or(E::Refused("review-archive-job-required"))?;
            let (saved, hash) = original(root, tx, &job)?;
            let mut work = crate::read_recorded_work(tx, &job.work_id).map_err(E::Work)?;
            let task = work.tasks.iter_mut().flatten().find(|task| task.id == job.task_id)
                .ok_or(E::Refused("review-archive-task-required"))?;
            let replayed = task.previous_reviews.iter().flatten().any(|r| r == &saved);
            if replayed {
                if !verified(root, tx, &work, &job.task_id, &saved)? {
                    return Err(E::Refused("review-archive-marker-required"));
                }
            } else {
                if task.status != "submitted" || task.independent_review.as_deref() != Some(&saved)
                    || task.attempts.iter().flatten().last().is_none_or(|attempt| {
                        attempt.id != saved.attempt || attempt.status != "completed"
                    })
                    || store::active(tx, &job.work_id, &job.task_id)
                        .map_err(|e| E::Launch(Box::new(e)))?.is_some()
                {
                    return Err(E::Refused("review-archive-current-failure-required"));
                }
                task.previous_reviews.get_or_insert_default().push(saved);
                task.independent_review = None;
                let previous_revision = work.revision;
                work.revision = previous_revision.checked_add(1)
                    .ok_or(E::Refused("review-archive-revision-overflow"))?;
                work.updated = crate::system_timestamp().map_err(E::Runtime)?;
                crate::validate_planning_state(&work)
                    .map_err(|e| E::Launch(Box::new(crate::AgentLaunchError::caused("review-archive-planning", e))))?;
                let event = format!("{id}-archive-failed");
                let request = crate::encode_model(&hash).map_err(E::Encode)?;
                let staged = crate::stage_recorded_change(tx, crate::RecordedChange {
                    work: &work, expected_revision: previous_revision, event: &event,
                    kind: EVENT_KIND, request: &request,
                }).map_err(|e| E::Launch(Box::new(crate::AgentLaunchError::caused("review-archive-stage", e))))?;
                if !matches!(staged, crate::RecordedChangeOutcome::Staged) {
                    return Err(E::Refused("review-archive-unexpected-replay"));
                }
            }
            let output = publication::Outcome {
                schema_version: 1, review_id: id.into(), review_recorded: true,
                verdict_applicable: false, task_accepted: false, replayed, work,
            };
            Ok(if replayed { transaction::Change::Observe(output) } else { transaction::Change::Mutate(output) })
        })
    })
}
