//! Terminal engine decision for an expired claim with no execution permission.
//! Decision events are not provider originals and cannot establish a verdict.
use super::{guard, journal, runtime::TaskReviewOutcome, store::{self, Job, Phase}};
use crate::{
    root_store::{native_transaction::{self, Change}, RootStore, RootStoreError as E},
    AgentLaunchError as A,
};
use rusqlite::{params, Transaction};
use serde::{Deserialize, Serialize};

const KIND: &str = "review.claim-retired";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Decision {
    schema_version: i64,
    request: super::super::task_validation::Request,
    original: Job,
    archived: crate::IndependentReview,
    instance: crate::root_store::RootInstanceIdentity,
}

/// Preserve static context and the original error, including IO/SQL cleanup.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, error)))
}

/// Require a clean prelaunch lifecycle. An absent child is not a known wait.
pub(super) fn unstarted(job: &Job) -> Result<(), E> {
    if job.phase != Phase::Claimed || job.permission != 0
        || job.child_pid.is_some() || job.parent_pid.is_some()
        || job.group_id.is_some() || job.child_stamp.is_some()
        || job.receipt_sha256.is_some() || job.published_revision.is_some()
    {
        return Err(E::Refused("review-unstarted-claim-required"));
    }
    Ok(())
}

/// Invert the existing freshness guard without renewing the original expiry.
pub(super) fn expired(job: &Job) -> Result<(), E> {
    match guard::original_fresh(&job.preflight_until) {
        Err(E::Refused("review-guard-original-preflight-expired")) => Ok(()),
        Err(error) => Err(error),
        Ok(()) => Err(E::Refused("review-unstarted-not-expired")),
    }
}

/// Require actual absence relative to the held directory. Symlinks and all
/// non-NotFound IO failures refuse; no provider journal is created or read.
fn no_original(root: &RootStore, job: &Job) -> Result<(), E> {
    let path = root.directory.process_path().join(journal::name(&job.id));
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(caused("review-unstarted-original-observation", error)),
        Ok(_) => Err(E::Refused("review-unstarted-original-present")),
    }
}

/// Compare the connected datastore to the claim's physical directory. Actual
/// connection/path association is checked by instance(), not by a pathname stat.
pub(super) fn physical(root: &RootStore, job: &Job) -> Result<crate::root_store::RootInstanceIdentity, E> {
    let instance = root.instance()?;
    if job.directory_device != instance.directory.0.to_string()
        || job.directory_inode != instance.directory.1.to_string()
    {
        return Err(E::Refused("review-unstarted-root-changed"));
    }
    Ok(instance)
}

/// Reconstruct the one terminal ledger projection from its engine event bytes.
fn terminal(original: &Job, hash: String, revision: i64) -> Job {
    let mut done = original.clone();
    done.phase = Phase::Refused;
    // This hash binds KIND's event, never a fabricated provider receipt.
    done.receipt_sha256 = Some(hash);
    done.published_revision = Some(revision);
    done
}

/// Acknowledgement describes only this historical non-verdict. Returned Work
/// is current and may already contain a newer claim; it is never modified here.
fn outcome(job: &Job, work: crate::Work, replayed: bool) -> TaskReviewOutcome {
    TaskReviewOutcome { schema_version: 1, review_id: job.id.clone(),
        review_recorded: false, verdict_applicable: false,
        task_accepted: false, replayed, work }
}

/// Read and validate the distinct retirement event before any journal path.
/// Missing, ambiguous, mismatched, foreign or mutated events fail closed.
fn replay_in(root: &RootStore, tx: &Transaction<'_>, job: &Job) -> Result<(TaskReviewOutcome, Decision), E> {
    let row: Option<(String, String, Vec<u8>)> = crate::agent_launch::query(
        tx, "SELECT id,kind,request FROM events WHERE revision=? AND work_id=?",
        params![job.published_revision.ok_or(E::Refused("review-unstarted-terminal-required"))?, job.work_id], |rows| {
            let value = rows.next()?.map(|r| -> rusqlite::Result<(String, String, Vec<u8>)> {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            }).transpose()?;
            if rows.next()?.is_some() { return Err(A::Refused("review-unstarted-event-ambiguous")); }
            Ok(value)
        }).map_err(|e| E::Launch(Box::new(e)))?;
    let (event_id, kind, raw) = row.ok_or(E::Refused("review-unstarted-event-required"))?;
    let decision: Decision = serde_json::from_slice(&raw)
        .map_err(|e| caused("review-unstarted-event-decode", e))?;
    unstarted(&decision.original)?;
    let revision = job.published_revision.ok_or(E::Refused("review-unstarted-terminal-required"))?;
    let hash = crate::provider_configuration::configuration_digest(&raw);
    if kind != KIND || decision.schema_version != 1
        || terminal(&decision.original, hash, revision) != *job
        || decision.instance != physical(root, job)?
        || decision.request.expected_revision.checked_add(1) != Some(revision)
        || decision.archived.id != job.id
        || decision.archived.state != "stale"
        || decision.archived.result_event != event_id
        || decision.request.event_id != event_id
        || decision.request.task_id != job.task_id
    {
        return Err(E::Refused("review-unstarted-event-conflict"));
    }
    no_original(root, job)?;
    let work = crate::read_recorded_work(tx, &job.work_id).map_err(E::Work)?;
    let task = work.tasks.iter().flatten().find(|t| t.id == job.task_id)
        .ok_or(E::Refused("review-unstarted-task-required"))?;
    if work.revision < revision
        || !task.previous_reviews.iter().flatten().any(|r| r == &decision.archived)
    {
        return Err(E::Refused("review-unstarted-history-changed"));
    }
    Ok((outcome(job, work, true), decision))
}

/// Historical replay for a childless refused job. No owner/clock freshness
/// requirement is reapplied and no permission, budget or new claim is touched.
pub(super) fn replay(root: &RootStore, job: &Job) -> Result<TaskReviewOutcome, E> {
    crate::agent_storage_retry::execute_borrowed(&root.db, &root.directory, false, |_, _| {
        native_transaction::apply(root, |tx| {
            replay_in(root, tx, job).map(|(value, _)| Change::Observe(value))
        })
    })
}

impl RootStore {
    /// Retire an expired, never-started reviewer claim after confirmed owner
    /// departure on its original physical Root. Input is the strict native
    /// task/event/revision envelope; REVIEW_ID selects the engine job.
    /// Atomically archive the running record as a non-verdict and end exclusion.
    /// Calls, reserves, attempts, controls and old opinions are retained. Fresh
    /// review still requires normal input, budget, configuration and host guards.
    /// Reject live/unknown owner, nonexpiry, stale revision, any child/permission/
    /// original, changed Root or malformed input. Exact replay observes only the
    /// old decision, including when a newer claim exists. Late errors report
    /// already recorded Work and retain causes. No provider receipt is produced.
    pub fn retire_unstarted_task_review(&self, id: &str, raw: &[u8]) -> Result<TaskReviewOutcome, E> {
        if !self.writable { return Err(E::Refused("root-read-only")); }
        if !swarm_identity_contract::name(id.as_bytes()) {
            return Err(E::Refused("review-unstarted-id-invalid"));
        }
        let request = super::super::task_validation::decode(raw)
            .map_err(|e| E::Launch(Box::new(e)))?;
        let mut known = None;
        let result = crate::agent_storage_retry::execute_borrowed(
            &self.db, &self.directory, false, |_, _| {
            native_transaction::apply(self, |tx| {
                let job = store::job(tx, id).map_err(|e| E::Launch(Box::new(e)))?
                    .ok_or(E::Refused("review-unstarted-job-required"))?;
                if job.task_id != request.task_id {
                    return Err(E::Refused("review-unstarted-task-conflict"));
                }
                if job.phase == Phase::Refused {
                    let (value, decision) = replay_in(self, tx, &job)?;
                    if serde_json::to_value(&request).map_err(|e| caused("review-unstarted-request", e))?
                        != serde_json::to_value(&decision.request).map_err(|e| caused("review-unstarted-request", e))?
                    {
                        return Err(E::Refused("review-unstarted-request-conflict"));
                    }
                    return Ok(Change::Observe(value));
                }
                unstarted(&job)?;
                let instance = physical(self, &job)?;
                guard::departed(&job)?;
                expired(&job)?;
                crate::financial_budget::check_reserved_planning_result_budget(
                    tx, &job.work_id, "reviewer", &job.id,
                ).map_err(|error| caused("review-unstarted-funding", error))?;
                no_original(self, &job)?;
                let mut work = crate::read_recorded_work(tx, &job.work_id).map_err(E::Work)?;
                if work.revision != request.expected_revision || work.revision < job.work_revision {
                    return Err(E::Refused("review-unstarted-revision-conflict"));
                }
                super::super::task_validation::require_validation_idle(tx, &job.work_id, &job.task_id)
                    .map_err(|e| E::Launch(Box::new(e)))?;
                let task = work.tasks.as_mut().and_then(|tasks| tasks.iter_mut().find(|t| t.id == job.task_id))
                    .ok_or(E::Refused("review-unstarted-task-required"))?;
                let review = task.independent_review.as_deref()
                    .ok_or(E::Refused("review-unstarted-current-record-required"))?;
                if task.status != "submitted" || review.id != job.id || review.state != "running"
                    || review.attempt != job.attempt_id || review.producer != job.producer_id
                    || review.contract != job.contract_sha256
                    || !review.receipt.is_empty() || !review.receipt_sha256.is_empty()
                    || !review.result_event.is_empty()
                    || task.previous_reviews.iter().flatten().any(|r| r.id == job.id)
                {
                    return Err(E::Refused("review-unstarted-current-record-changed"));
                }
                let mut archived = review.clone();
                archived.state = "stale".into();
                archived.reason = "review-claim-expired-before-start".into();
                archived.criteria = Default::default();
                archived.finished = crate::system_timestamp().map_err(E::Runtime)?;
                archived.result_event = request.event_id.clone();
                task.previous_reviews.get_or_insert_default().push(archived.clone());
                task.independent_review = None;
                // Controls are preserved. Normal admission rechecks their applicability.
                let revision = work.revision;
                work.revision = revision.checked_add(1).ok_or(E::Refused("review-unstarted-revision-overflow"))?;
                work.updated = archived.finished.clone();
                let decision = Decision { schema_version: 1, request: request.clone(), original: job.clone(), archived, instance };
                let body = crate::encode_model(&decision).map_err(E::Encode)?;
                let hash = crate::provider_configuration::configuration_digest(&body);
                let done = terminal(&job, hash, work.revision);
                // Reobserve owner, absence and actual connected Root immediately
                // before SQL publication; no helper ever received a permission.
                guard::departed(&job)?;
                if physical(self, &job)? != decision.instance {
                    return Err(E::Refused("review-unstarted-root-changed"));
                }
                no_original(self, &job)?;
                let staged = crate::stage_recorded_change_with_hook(tx, crate::RecordedChange {
                    work: &work, expected_revision: revision, event: &request.event_id, kind: KIND, request: &body,
                }, |db, _| store::change(db, &job, &done)
                    .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>))
                    .map_err(|error| caused("review-unstarted-stage", error))?;
                if !matches!(staged, crate::RecordedChangeOutcome::Staged) {
                    return Err(E::Refused("review-unstarted-stage-conflict"));
                }
                Ok(Change::Mutate(outcome(&job, work, false)))
            }).map(|value| {
                known = Some((value.work.id.clone(), value.work.revision));
                value
            })
        });
        result.map_err(|primary| match known {
            Some((work, revision)) => E::WorkRecorded { primary: Box::new(primary), work, revision },
            None => primary,
        })
    }
}
