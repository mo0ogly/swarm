//! Exact original native control evidence for a new read, never a process grant.
use super::{
    journal, publication,
    runtime::Receipt,
    store::{self, JobPhase, StepPhase},
    TaskValidationPreview,
};
use crate::{
    agent_launch::query,
    root_store::{RootStore, RootStoreError as E},
};
use serde::Serialize;
/// Engine-read evidence coordinates, without private PID/host or control text.
/// Hashes bind observed data; they authenticate neither a human nor an account.
#[derive(Clone, Serialize)]
pub struct NativeValidationEvidence {
    pub job_id: String,
    pub receipt_sha256: String,
    pub published_revision: i64,
    pub source_sha256: String,
}
/// Bind current task data to a terminal registered native job and original slots.
/// Caller owns settings/SQL snapshot and has prepared the actual source. Missing,
/// changed or uncertain evidence refuses with original causes; no ownership is
/// transferred and a server that published may remain alive. Reads only, no retry.
pub(in crate::root_store) fn recorded_evidence(
    root: &RootStore,
    work: &crate::Work,
    source: &TaskValidationPreview,
) -> Result<NativeValidationEvidence, E> {
    recorded(root, work, &source.task_id, Origin::CurrentSource(&source.source_sha256))
}
/// Select exact input binding, never a relaxed error fallback or execution mode.
/// Current source is prepared by the engine; reviewed receipt SHA comes only from
/// the already verified original reviewer job. Neither value grants a new call.
#[derive(Clone, Copy)]
enum Origin<'a> {
    CurrentSource(&'a str),
    ReviewedReceipt(&'a str),
    PriorReview { sha256: &'a str, attempt: &'a str, producer: &'a str },
}
/// Re-read the original controls actually bound into a verified reviewer result.
/// This checks private job/step frames, scalar rows, result event and exact public
/// control data, including its match to current task data. It does not recompute
/// a new execution grant/source snapshot or require a submitted task merely to
/// observe an accepted dependency. Caller still owes current report, contract,
/// project, gate and policy checks; IO uncertainty refuses without any retry.
pub(in crate::root_store) fn recorded_evidence_for_review_result(
    root: &RootStore,
    work: &crate::Work,
    task_id: &str,
    receipt_sha256: &str,
) -> Result<NativeValidationEvidence, E> {
    recorded(root, work, task_id, Origin::ReviewedReceipt(receipt_sha256))
}
/// Re-read original control evidence after a real retry has archived its review.
/// Only the independently verified prior native review supplies body hash and
/// old producer/attempt; SQL locates the exact recorded job, never a fake Task or
/// preview. Current-source/current-review readers retain their original binding.
/// This historical check grants no new execution itself; the caller must bind
/// current task/decision/report/project, actual agent and current permissions.
pub(in crate::root_store) fn recorded_evidence_for_prior_review(root: &RootStore, work: &crate::Work, task_id: &str, review: &crate::IndependentReview, sha256: &str) -> Result<NativeValidationEvidence, E> {
    recorded(root, work, task_id, Origin::PriorReview { sha256, attempt: &review.attempt, producer: &review.producer })
}
/// Shared original/ledger/public binding with purpose-specific exact input.
/// No provider, source mutation or fallback; all original IO/SQL causes survive.
fn recorded(
    root: &RootStore,
    work: &crate::Work,
    task_id: &str,
    origin: Origin<'_>,
) -> Result<NativeValidationEvidence, E> {
    let launch = |e| E::Launch(Box::new(e));
    let task = work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == task_id)
        .ok_or(E::Refused("review-task-missing"))?;
    let actual = match origin {
        Origin::PriorReview { .. } => None,
        _ => Some(task.automatic_validation.as_deref().ok_or(E::Refused("review-native-controls-required"))?),
    };
    let id = if let Origin::PriorReview { sha256, .. } = origin {
        query(&root.db, "SELECT id FROM native_validation_jobs WHERE work_id=? AND task_id=? AND phase='recorded' AND receipt_sha256=? ORDER BY id", rusqlite::params![work.id, task.id, sha256], |rows| {
            let selected = rows.next()?.map(|r| r.get::<_, String>(0)).transpose()?;
            if rows.next()?.is_some() { return Err(crate::AgentLaunchError::Refused("review-native-controls-ambiguous")); }
            selected.ok_or(crate::AgentLaunchError::Refused("review-native-controls-required"))
        }).map_err(launch)?
    } else {
        store::recorded_id_for_receipt(&root.db, &work.id, &task.id, &actual.ok_or(E::Refused("review-native-controls-required"))?.receipt)
            .map_err(launch)?.ok_or(E::Refused("review-native-controls-required"))?
    };
    let (receipt, hash): (Receipt, String) = journal::read(root, &journal::name("job", &id))?;
    let job = publication::recorded_original(root, &receipt, &hash)?;
    let revision = job
        .published_revision
        .ok_or(E::Refused("review-native-publication-required"))?;
    if job.phase != JobPhase::Recorded
        || job.work_id != work.id
        || job.task_id != task.id
        || match origin {
            Origin::CurrentSource(source) => job.source_sha256 != super::managed_execution::bound_source(&root.db,&job,source,receipt.managed_candidate.as_ref()).map_err(launch)?,
            Origin::ReviewedReceipt(expected) => hash != expected,
            Origin::PriorReview { sha256, attempt, producer } => hash != sha256 || receipt.attempt_id != attempt || receipt.producer_agent_id != producer,
        }
        || job.receipt_sha256.as_deref() != Some(hash.as_str())
        || revision > work.revision
        || !publication::all_passed(&receipt)
    {
        return Err(E::Refused("review-native-controls-stale"));
    }
    for (step, original_hash) in &receipt.steps {
        let (parent, current) = store::step(&root.db, &step.step.id).map_err(launch)?;
        let mut expected = step.step.clone();
        expected.phase = StepPhase::Completed;
        expected.receipt_sha256 = Some(original_hash.clone());
        if parent != job || current != expected {
            return Err(E::Refused("review-native-slot-changed"));
        }
    }
    let (mut expected, _, bytes) = publication::public(&receipt).map_err(launch)?;
    let path = expected.receipt.clone();
    let digest = crate::provider_configuration::configuration_digest(&bytes);
    expected
        .artifacts
        .as_mut()
        .ok_or(E::Refused("review-native-public-artifacts-required"))?
        .insert(path.clone(), digest.clone());
    let encoded = crate::encode_model(&expected).map_err(E::Encode)?;
    if let Some(actual) = actual {
        if encoded != crate::encode_model(actual).map_err(E::Encode)? {
            return Err(E::Refused("review-native-receipt-changed"));
        }
    }
    let (_, observed) = crate::gate_evaluation::artifact_hash(&root.root, &path).map_err(|e| {
        launch(crate::AgentLaunchError::caused(
            "review-native-public-artifact",
            e,
        ))
    })?;
    if observed != digest {
        return Err(E::Refused("review-native-public-receipt-changed"));
    }
    let valid = query(
        &root.db,
        "SELECT work_id,revision,kind,request FROM main.events WHERE id=?",
        [&receipt.result_event],
        |rows| {
            let row = rows.next()?.ok_or(crate::AgentLaunchError::Refused(
                "review-native-publication-event-missing",
            ))?;
            let owner: String = row.get(0)?;
            let rev: i64 = row.get(1)?;
            let kind: String = row.get(2)?;
            let body: Vec<u8> = row.get(3)?;
            let matches = owner == work.id
                && rev == revision
                && kind == "task.auto-validation"
                && body == encoded;
            if rows.next()?.is_some() {
                return Err(crate::AgentLaunchError::Refused(
                    "review-native-publication-event-duplicate",
                ));
            }
            Ok(matches)
        },
    )
    .map_err(launch)?;
    if !valid {
        return Err(E::Refused("review-native-publication-event-changed"));
    }
    Ok(NativeValidationEvidence {
        job_id: id,
        receipt_sha256: hash,
        published_revision: revision,
        source_sha256: job.source_sha256.clone(),
    })
}
