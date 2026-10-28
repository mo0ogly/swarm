//! Digest-bound machine receipt data; it establishes no producer authenticity.
use crate::{
    model_wire::{Fields, Kind},
    AutomaticValidation, EngineSettingsDocument, ReviewEvidenceError, ValidationControlResult,
    Work,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};
#[derive(Default, Deserialize)]
#[serde(default)]
struct Receipt {
    candidate_commit: String,
    attempt_id: String,
    agent_id: String,
    controls: Option<BTreeMap<String, Option<Vec<ValidationControlResult>>>>,
}
/// Describe the actual Go projection DTO: unknown receipt fields are compatible,
/// while known fields use Go merge/null/casefold semantics. No policy is encoded.
fn shape() -> Fields {
    &[
        ("candidate_commit", Kind::String),
        ("attempt_id", Kind::String),
        ("agent_id", Kind::String),
        (
            "controls",
            Kind::Map(&Kind::Array(&Kind::Object(
                crate::models::shape_validation_control_result,
            ))),
        ),
    ]
}
/// Read supplied Work's unique managed task receipt under explicit saved policy.
/// No review or no declared receipt returns None; partial declaration, unsupported
/// batch/fragment protocol, invalid binding or missing controls refuses. Complete
/// bytes must fit the cap and expected SHA before compatible JSON decoding. The
/// candidate, attempt and producer must match the review's declared values.
/// Settings/path/read/allocation/close/codec causes are retained; safe messages
/// never include receipt content. No files, journal or Work change. This is data
/// consistency, not current review freshness, authenticated identity or acceptance.
/// Root callers must supply their observed actual Work and settings reservation.
pub fn read_managed_receipt(
    root: &Path,
    work: &Work,
    task_id: &str,
    settings: &EngineSettingsDocument,
) -> Result<Option<AutomaticValidation>, ReviewEvidenceError> {
    use ReviewEvidenceError as E;
    settings.validate().map_err(E::Settings)?;
    let limit = settings
        .values
        .managed_receipt_limit()
        .map_err(E::Settings)?;
    if work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_ref())
        .is_none()
    {
        return Err(E::Refused("managed-receipt-managed-work-required"));
    }
    let mut tasks = work.tasks.iter().flatten().filter(|t| t.id == task_id);
    let task = tasks.next().ok_or(E::Refused("review-task-not-found"))?;
    if tasks.next().is_some() {
        return Err(E::Refused("review-task-ambiguous"));
    }
    let Some(review) = task.independent_review.as_deref() else {
        return Ok(None);
    };
    if review.fragment_journal.is_some() {
        return Err(E::Refused("managed-review-fragment-protocol-not-ported"));
    }
    if review.batches.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err(E::Refused("managed-review-batch-protocol-not-ported"));
    }
    if review.receipt.is_empty() && review.receipt_sha256.is_empty() {
        return Ok(None);
    }
    if review.receipt.is_empty()
        || review.receipt_sha256.is_empty()
        || review.candidate_commit.is_empty()
        || review.attempt.is_empty()
        || review.producer.is_empty()
    {
        return Err(E::Refused("managed-receipt-binding-required"));
    }
    let (_, raw) = crate::gate_evaluation::artifact_bytes_limited(root, &review.receipt, limit)
        .map_err(E::Artifact)?;
    if format!("{:x}", Sha256::digest(&raw)) != review.receipt_sha256 {
        return Err(E::Refused("managed-receipt-digest-stale"));
    }
    let mut receipt: Receipt = crate::work::decode(&raw, shape()).map_err(E::ReceiptFormat)?;
    if receipt.candidate_commit != review.candidate_commit
        || receipt.attempt_id != review.attempt
        || receipt.agent_id != review.producer
    {
        return Err(E::Refused("managed-receipt-binding-stale"));
    }
    let controls = receipt
        .controls
        .as_mut()
        .and_then(|c| c.remove(task_id))
        .flatten()
        .filter(|c| !c.is_empty())
        .ok_or(E::Refused("managed-receipt-controls-required"))?;
    Ok(Some(AutomaticValidation {
        attempt_id: review.attempt.clone(),
        candidate_sha: review.candidate_commit.clone(),
        producer_agent_id: review.producer.clone(),
        controls: Some(controls),
        at: review.started.clone(),
        ..AutomaticValidation::default()
    }))
}
