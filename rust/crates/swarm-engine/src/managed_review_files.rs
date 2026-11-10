//! Iterative historical file proofs, distinct from review identity and acceptance.
use crate::{EngineSettingsDocument, IndependentReview, ManagedReviewContext, ReviewEvidenceError};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};

/// Files and context observed during one traversal; no future lease or acceptance receipt.
#[derive(Debug)]
pub struct ManagedReviewFilesObservation {
    pub context: ManagedReviewContext,
    pub reviews_checked: usize,
    pub max_depth: usize,
}
/// Verify each logical file obligation, then parse its own digest-bound context buffer.
/// Unsupported batch/fragment protocols refuse at every level. A shared context
/// never suppresses a different review's receipt, report or protocol obligations.
fn read_context(
    root: &Path,
    review: &IndependentReview,
    strict: bool,
) -> Result<ManagedReviewContext, ReviewEvidenceError> {
    use ReviewEvidenceError as E;
    if review.fragment_journal.is_some() {
        return Err(E::Refused("managed-review-fragment-protocol-not-ported"));
    }
    if review.batches.as_ref().is_some_and(|b| !b.is_empty()) {
        return Err(E::Refused("managed-review-batch-protocol-not-ported"));
    }
    let mut context = None;
    for (path, expected, is_context) in [
        (&review.receipt, &review.receipt_sha256, false),
        (&review.context, &review.context_sha256, true),
        (&review.report, &review.sha256, false),
    ] {
        if expected.is_empty() {
            return Err(E::Refused("managed-review-file-stale"));
        }
        let actual = if is_context {
            let (_, raw) =
                crate::gate_evaluation::artifact_bytes(root, path).map_err(E::Artifact)?;
            let digest = format!("{:x}", Sha256::digest(&raw));
            context = Some(raw);
            digest
        } else {
            crate::gate_evaluation::artifact_hash(root, path)
                .map_err(E::Artifact)?
                .1
        };
        if &actual != expected {
            return Err(E::Refused("managed-review-file-stale"));
        }
    }
    let raw = context.ok_or(E::Refused("managed-review-context-required"))?;
    crate::decode_managed_review_context(&raw, strict).map_err(E::ContextFormat)
}
/// Validate baseline relationships and return children in original order.
/// A baseline is historical data, never a verdict on the current candidate.
fn children(
    context: &mut ManagedReviewContext,
    review: &IndependentReview,
) -> Result<Vec<IndependentReview>, ReviewEvidenceError> {
    use ReviewEvidenceError as E;
    let Some(baseline) = context.accepted_baseline.as_mut() else {
        return Ok(Vec::new());
    };
    if !matches!(baseline.version, 0..=2) {
        return Err(E::Refused("managed-review-baseline-version-unsupported"));
    }
    if baseline.candidate_commit != context.previous_candidate
        || context.previous_candidate == context.candidate_commit
        || baseline.accepted_reviews.as_ref().is_none_or(Vec::is_empty)
    {
        return Err(E::Refused("managed-review-baseline-incoherent"));
    }
    for previous in baseline.accepted_reviews.iter().flatten() {
        if previous.state != "passed"
            || previous.candidate_commit != context.previous_candidate
            || previous.context == review.context
            || previous.context_sha256 == review.context_sha256
        {
            return Err(E::Refused("managed-review-baseline-review-invalid"));
        }
    }
    Ok(baseline.accepted_reviews.take().unwrap_or_default())
}
/// Verify root and historical review files with an explicit persisted depth policy.
///
/// Every review's three logical file/hash obligations and supported protocol are
/// checked before memoizing context-baseline descent. Each context is hashed and
/// decoded from the same buffer. `strict_root` selects Go strict decoding for the
/// current full guard or compatible json.Unmarshal for file-only inspection;
/// historical descendants use the actual Go file-guard compatible mode.
///
/// Traversal is iterative. The root has depth0. If any baseline exists, explicit
/// v6 adoption is required: configured zero forbids ancestors, null removes this
/// depth ceiling. No fallback128, retry ceiling or Rust recursion bound is added.
/// Baseline versions0/1/2, candidate relationships and passed historical records
/// must agree. Batches/fragments remain unported and refuse even under a cached
/// context. Settings/files/Work are never mutated. This authenticates no reviewer,
/// verifies no independent verdict and grants no task acceptance or future lease.
///
/// # Errors
/// Preserves invalid settings, depth representation/limit, protocol, artifact,
/// digest, codec and baseline-consistency failures, including checked-close causes.
pub fn check_managed_review_files(
    root: &Path,
    review: &IndependentReview,
    settings: &EngineSettingsDocument,
    strict_root: bool,
) -> Result<ManagedReviewFilesObservation, ReviewEvidenceError> {
    use ReviewEvidenceError as E;
    settings.validate().map_err(E::Settings)?;
    let mut context = read_context(root, review, strict_root)?;
    let has_baseline = context.accepted_baseline.is_some();
    let limit = if has_baseline {
        settings
            .values
            .managed_review_baseline_depth()
            .map_err(E::Settings)?
    } else {
        None
    };
    let mut descent = ManagedReviewContext::default();
    std::mem::swap(
        &mut descent.accepted_baseline,
        &mut context.accepted_baseline,
    );
    descent.candidate_commit = context.candidate_commit.clone();
    descent.previous_candidate = context.previous_candidate.clone();
    // Keep the root context available to the caller; only this child list is cloned.
    let root_children = if let Some(b) = descent.accepted_baseline.as_ref() {
        b.accepted_reviews.clone()
    } else {
        None
    };
    let mut queue: Vec<_> = children(&mut descent, review)?
        .into_iter()
        .rev()
        .map(|r| (r, 1usize))
        .collect();
    if let Some(b) = descent.accepted_baseline.as_mut() {
        b.accepted_reviews = root_children;
    }
    context.accepted_baseline = descent.accepted_baseline;
    let mut seen = BTreeSet::from([(
        review.context.clone(),
        review.context_sha256.clone(),
        0usize,
    )]);
    let mut reviews_checked = 1usize;
    let mut max_depth = 0usize;
    while let Some((previous, depth)) = queue.pop() {
        if limit.is_some_and(|limit| depth > limit) {
            return Err(E::Refused("managed-review-baseline-depth-exceeded"));
        }
        let mut child_context = read_context(root, &previous, false)?;
        reviews_checked = reviews_checked
            .checked_add(1)
            .ok_or(E::Refused("managed-review-count-representation"))?;
        max_depth = max_depth.max(depth);
        // Remaining depth is an input to bounded descent. A proof first seen
        // near the root cannot suppress its deeper occurrence under this cap.
        let cache_depth = if limit.is_some() { depth } else { 0 };
        if !seen.insert((
            previous.context.clone(),
            previous.context_sha256.clone(),
            cache_depth,
        )) {
            continue;
        }
        let next = children(&mut child_context, &previous)?;
        if !next.is_empty() {
            let next_depth = depth
                .checked_add(1)
                .ok_or(E::Refused("managed-review-depth-representation"))?;
            queue.extend(next.into_iter().rev().map(|r| (r, next_depth)));
        }
    }
    Ok(ManagedReviewFilesObservation {
        context,
        reviews_checked,
        max_depth,
    })
}
