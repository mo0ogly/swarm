//! Bind stored review candidate/files to a Git blob; identity/protocol acceptance remains separate.
use crate::{
    EngineSettingsDocument, GitReportObservation, ManagedReviewAnchorObservation,
    ReviewEvidenceError, Work,
};
use std::path::Path;

/// Inspect through the shared private State/Work alias binding and read lane.
/// The callback runs while both canonical and stored directory descriptors are
/// retained. Its original review failure and any independent binding/lock/close
/// failures survive together in the established Root cleanup wrapper. Nothing
/// is created or rewritten; the caller still owns Root/material/settings guards.
/// Only standard managed locators can pass this alias branch. No identity,
/// acceptance, snapshot or same-UID confinement follows from the observation.
fn with_legacy_alias<T>(
    root: &Path,
    work: &Work,
    run: impl FnOnce() -> Result<T, ReviewEvidenceError>,
) -> Result<T, ReviewEvidenceError> {
    use crate::RootStoreError as R;
    use ReviewEvidenceError as E;
    let mut observed = None;
    let binding = crate::managed_repository::with_bound_storage(root, work, false, |_| {
        observed = Some(run());
        Ok(())
    });
    match (observed, binding) {
        (Some(result), Ok(())) => result,
        (None, Ok(())) => Err(E::Native(Box::new(R::Refused(
            "managed-review-alias-observation-missing",
        )))),
        (Some(Err(primary)), Err(cleanup)) => Err(E::Native(Box::new(R::Cleanup {
            primary: Some(Box::new(R::Review(primary))),
            cleanup: Box::new(R::Launch(Box::new(cleanup))),
        }))),
        (_, Err(error)) => Err(E::Native(Box::new(R::Launch(Box::new(error))))),
    }
}

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
/// must be local and resolve inside the root. An absolute historical root alias
/// uses the shared standard managed-storage descriptor binding and existing read
/// lane; the stored private State/Work must match the canonical ones. Git still
/// reads the canonical contained path, without rewriting stored locators.
/// Settings must explicitly adopt v5.
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
    let inspect = |relative: &Path| {
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
    };
    if storage.is_absolute() {
        match storage.strip_prefix(&canonical) {
            Ok(relative) => inspect(relative),
            Err(_) => {
                let relative = Path::new(".swarm").join("managed").join(&work.id);
                with_legacy_alias(&canonical, work, || inspect(&relative))
            }
        }
    } else {
        inspect(storage)
    }
}
