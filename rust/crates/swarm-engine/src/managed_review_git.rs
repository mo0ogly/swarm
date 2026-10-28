//! Bind stored review candidate/files to a Git blob; identity/protocol acceptance remains separate.
use crate::{
    EngineSettingsDocument, GitReportObservation, ManagedReviewAnchorObservation,
    ReviewEvidenceError, Work,
};
use std::path::Path;

/// Candidate/file/Git consistency only. Protocol, current identity and acceptance remain unproven.
#[derive(Debug, serde::Serialize)]
pub struct ManagedReviewGitObservation {
    pub scope: &'static str,
    pub anchor: ManagedReviewAnchorObservation,
    pub git: GitReportObservation,
    pub baseline_reviews_checked: usize,
    pub baseline_max_depth: usize,
    pub protocol_verified: bool,
    pub authority_verified: bool,
}
/// Observe a supplied Work's unique review against current files and its committed Git report.
///
/// All candidate/file bindings must pass first. The exact context buffer is hashed
/// against its own declared digest, then decoded strictly without rereading it.
/// That verified context selects the Git citation semantics. Its candidate must
/// also match the review. The report blob digest must equal the observed file
/// digest. Repository storage may be relative or absolute beneath `root`; it
/// must be local and resolve inside the root. Settings must explicitly adopt v5.
///
/// Historical baseline files follow their configured iterative guard. Batches
/// and fragment journals still refuse until their full protocols are ported. It neither authenticates reviewer nor
/// checks all task-contract/criterion freshness and never enables acceptance.
/// Root callers should use [`crate::RootStore::inspect_managed_review_git`] for
/// actual stored Work/settings. No file, journal or Work is changed; observations
/// supply no future lease. Errors preserve codec/artifact/Git/cleanup causes.
pub fn inspect_managed_review_git(
    root: &Path,
    work: &Work,
    task_id: &str,
    settings: &EngineSettingsDocument,
) -> Result<ManagedReviewGitObservation, ReviewEvidenceError> {
    use ReviewEvidenceError as E;
    let anchor = crate::inspect_managed_review_anchor(root, work, task_id)?;
    let task = work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == task_id)
        .ok_or(E::Refused("review-task-not-found"))?;
    let review = task
        .independent_review
        .as_deref()
        .ok_or(E::Refused("managed-review-candidate-stale"))?;
    if review.fragment_journal.is_some() {
        return Err(E::Refused("managed-review-fragment-protocol-not-ported"));
    }
    if review.batches.as_ref().is_some_and(|b| !b.is_empty()) {
        return Err(E::Refused("managed-review-batch-protocol-not-ported"));
    }
    let files = crate::check_managed_review_files(root, review, settings, true)?;
    let context = &files.context;
    if context.candidate_commit != anchor.candidate_commit {
        return Err(E::Refused("managed-review-context-candidate-stale"));
    }
    let repository = work
        .planning
        .as_deref()
        .and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-review-repository-required"))?;
    if repository.storage.is_empty() {
        return Err(E::Refused("managed-review-storage-required"));
    }
    let canonical =
        crate::canonical_project_root(root).map_err(|e| E::Git(crate::GitReportError::Path(e)))?;
    let storage = Path::new(&repository.storage);
    let relative = if storage.is_absolute() {
        storage
            .strip_prefix(&canonical)
            .map_err(|_| E::Refused("managed-review-storage-outside-root"))?
    } else {
        storage
    };
    let git_dir = relative.join("repository.git");
    let git_dir = git_dir
        .to_str()
        .ok_or(E::Refused("managed-review-storage-encoding"))?;
    let git = crate::hash_git_report(
        root,
        git_dir,
        &anchor.candidate_commit,
        &review.git_report,
        context.citation_contract_version,
        settings,
    )
    .map_err(E::Git)?;
    if git.sha256 != anchor.report_sha256 {
        return Err(E::Refused("managed-review-git-report-stale"));
    }
    Ok(ManagedReviewGitObservation {
        scope: "candidate-files-and-git-only",
        anchor,
        git,
        baseline_reviews_checked: files.reviews_checked,
        baseline_max_depth: files.max_depth,
        protocol_verified: false,
        authority_verified: false,
    })
}
