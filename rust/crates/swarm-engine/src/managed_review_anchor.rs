//! Candidate/file bindings only. Git contents, protocol, identity and authority
//! must still be verified before any managed review can enable acceptance.
use crate::{ReviewEvidenceError, Work};
use std::path::Path;
/// Observed candidate and file-digest bindings, never an acceptance receipt.
///
/// Paths and digests refer to the three individually read files. The scope is
/// `candidate-and-file-bindings-only`; Git/protocol/authority flags are false.
/// Files may change after inspection, so this value supplies no future lease.
#[derive(Debug, serde::Serialize)]
pub struct ManagedReviewAnchorObservation {
    pub scope: &'static str,
    pub task_id: String,
    pub review_id: String,
    pub candidate_commit: String,
    pub receipt: String,
    pub receipt_sha256: String,
    pub context: String,
    pub context_sha256: String,
    pub report: String,
    pub report_sha256: String,
    pub git_report_verified: bool,
    pub protocol_verified: bool,
    pub authority_verified: bool,
}
/// Observe one task's candidate and three current file bindings under `root`.
///
/// `work` is supplied data: callers needing stored state must use
/// [`crate::RootStore::inspect_managed_review_anchor`]. The unique task must have
/// a managed repository, review and automatic validation with the same nonempty
/// candidate. Each receipt/context/report path must resolve to a regular local
/// file contained by the root and match its own nonempty SHA256, including
/// when paths coincide. No file or Work mutation occurs.
///
/// This neither parses context/receipt JSON nor verifies Git, review protocols,
/// general freshness, identities or authority. A successful observation is not
/// a verdict or future-action lease; unsupported review remains refused.
///
/// # Errors
/// Returns [`ReviewEvidenceError`] for missing/ambiguous task, missing or stale
/// candidate bindings, empty/mismatching digests, or file access/containment/
/// regular-file/read/close failure. Artifact failures retain their cause.
pub fn inspect_managed_review_anchor(
    root: &Path,
    work: &Work,
    task_id: &str,
) -> Result<ManagedReviewAnchorObservation, ReviewEvidenceError> {
    use ReviewEvidenceError as E;
    let mut tasks = work.tasks.iter().flatten().filter(|t| t.id == task_id);
    let task = tasks.next().ok_or(E::Refused("review-task-not-found"))?;
    if tasks.next().is_some() {
        return Err(E::Refused("review-task-ambiguous"));
    }
    let repository = work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-review-repository-required"))?;
    let review = task
        .independent_review
        .as_deref()
        .ok_or(E::Refused("managed-review-candidate-stale"))?;
    let automatic = task
        .automatic_validation
        .as_deref()
        .ok_or(E::Refused("managed-review-candidate-stale"))?;
    if review.candidate_commit.is_empty()
        || review.candidate_commit != repository.candidate_commit
        || review.candidate_commit != automatic.candidate_sha
    {
        return Err(E::Refused("managed-review-candidate-stale"));
    }
    // Check each logical binding separately. Equal paths with contradictory
    // digests must never collapse through a map and silently lose a binding.
    for (name, expected) in [
        (&review.receipt, &review.receipt_sha256),
        (&review.context, &review.context_sha256),
        (&review.report, &review.sha256),
    ] {
        if expected.is_empty() {
            return Err(E::Refused("managed-review-file-stale"));
        }
        let (_, actual) = crate::gate_evaluation::artifact_hash(root, name).map_err(E::Artifact)?;
        if &actual != expected {
            return Err(E::Refused("managed-review-file-stale"));
        }
    }
    Ok(ManagedReviewAnchorObservation {
        scope: "candidate-and-file-bindings-only",
        task_id: task.id.clone(),
        review_id: review.id.clone(),
        candidate_commit: review.candidate_commit.clone(),
        receipt: review.receipt.clone(),
        receipt_sha256: review.receipt_sha256.clone(),
        context: review.context.clone(),
        context_sha256: review.context_sha256.clone(),
        report: review.report.clone(),
        report_sha256: review.sha256.clone(),
        git_report_verified: false,
        protocol_verified: false,
        authority_verified: false,
    })
}
