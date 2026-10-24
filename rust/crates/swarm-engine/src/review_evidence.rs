//! Current review data and native original-chain verification, never identity or authority.
use crate::{ContextPolicyLimits, Work};
use std::{error::Error, fmt, path::Path};
/// Internal evidence purpose, never deserialized from a launch/decision DTO.
/// A failed criterion permits only production correction; a known protocol
/// failure permits only reviewer correction. Neither satisfies acceptance.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurrentReviewUse { Acceptance, ProductionRetry, ReviewerRetry }
impl CurrentReviewUse {
    /// Purpose-specific state discriminator, never identity, verdict or permit.
    pub(crate) fn state(self) -> &'static str {
        match self { Self::Acceptance => "passed", Self::ProductionRetry => "changes_requested", Self::ReviewerRetry => "error" }
    }
}
/// Verified original failure category, not deserialized policy or permission.
/// Runtime classification still requires the private original and current proof.
#[derive(Clone, Copy)]
pub(crate) enum NativeReviewFailure { Protocol, Interrupted }
impl NativeReviewFailure {
    /// Stable routing code for observed originals; no IO or execution authority.
    pub(crate) fn code(self) -> &'static str {
        match self { Self::Protocol => "review_fragment_protocol", Self::Interrupted => "review_fragment_interrupted" }
    }
    /// Role responsible for correction, not an authenticated actor identity.
    pub(crate) fn owner(self) -> &'static str {
        match self { Self::Protocol => "reviewer_protocol", Self::Interrupted => "reviewer_runtime" }
    }
}
#[derive(Debug, serde::Serialize)]
pub struct ReviewEvidenceObservation {
    pub review_id: String,
    pub task_id: String,
    pub attempt_id: String,
    pub contract: String,
    pub report: String,
    pub report_sha256: String,
}
pub enum ReviewEvidenceError {
    Refused(&'static str),
    Context(crate::ProjectContextError),
    Contract(crate::WorkError),
    Artifact(crate::GateEvaluationError),
    ContextFormat(crate::WorkError),
    ReceiptFormat(crate::WorkError),
    Settings(crate::EngineSettingsError),
    #[cfg(target_os = "linux")]
    /// Native Root/SQL/receipt verification failure; original typed cause retained.
    Native(Box<crate::RootStoreError>),
    #[cfg(target_os = "linux")]
    Git(crate::GitReportError),
}
impl ReviewEvidenceError {
    pub fn code(&self) -> &'static str {
        "review-evidence-invalid"
    }
    /// Return a static safe failure category for binding, policy, artifact or
    /// codec refusal. Receipt decoding is distinct from review-context decoding;
    /// raw payloads are excluded, while Error::source retains the original cause.
    /// This performs no IO and neither rejects nor authorizes a separate action.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Refused(s) => s,
            Self::Context(e) => e.reason(),
            Self::Contract(_) => "review-contract-encoding",
            Self::Artifact(e) => e.reason(),
            Self::ContextFormat(_) => "managed-review-context-format",
            Self::ReceiptFormat(_) => "managed-receipt-format",
            Self::Settings(e) => e.reason(),
            #[cfg(target_os = "linux")]
            Self::Native(e) => e.code(),
            #[cfg(target_os = "linux")]
            Self::Git(e) => e.reason(),
        }
    }
}
impl fmt::Display for ReviewEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for ReviewEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for ReviewEvidenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Refused(_) => None,
            Self::Context(e) => Some(e),
            Self::Contract(e) => Some(e),
            Self::Artifact(e) => Some(e),
            Self::ContextFormat(e) => Some(e),
            Self::ReceiptFormat(e) => Some(e),
            Self::Settings(e) => Some(e),
            #[cfg(target_os = "linux")]
            Self::Native(e) => Some(e.as_ref()),
            #[cfg(target_os = "linux")]
            Self::Git(e) => Some(e),
        }
    }
}
/// Reserved engine certificate namespace selects the native wire protocol only.
/// It grants no authority: malformed names still reach authoritative verification
/// and refuse, rather than falling back to an ordinary supplied review object.
/// Versions1/2 share this prefix; exact addressing/content is checked by Root.
pub(crate) fn native_receipt_reference(receipt: &str) -> bool {
    receipt.starts_with(".swarm/native-review-public-")
}
/// Inspect supplied Work data and current ordinary files. A Root caller must
/// supply its actual row/settings; a successful observation grants no authority.
pub fn check_review_evidence(
    root: &Path,
    work: &Work,
    task_id: &str,
    limits: Option<&ContextPolicyLimits>,
) -> Result<Option<ReviewEvidenceObservation>, ReviewEvidenceError> {
    check_review_evidence_policy(root, work, task_id, limits, None, CurrentReviewUse::Acceptance)
}
/// Observe current ordinary, native or simple managed review data with explicit settings.
/// Native certificates additionally require the actual current Root SQL row,
/// original framed receipt and exact public certificate; missing evidence refuses.
/// Namespace selection is not authority and cannot fall back to ordinary data.
/// A Root caller supplies its stored Work and settings under its own reservation.
/// Common context/attempt/contract checks precede managed candidate/files/baseline
/// and actual Git report checks. Batches/fragments remain explicitly refused.
/// No source, settings or Work is mutated. Success authenticates no reviewer,
/// proves no independent decision and grants neither acceptance nor a later lease.
///
/// # Errors
/// Preserves settings/context/codec/artifact/Git and checked-close causes. A
/// missing/stale review or unsupported protocol refuses without empty success.
pub fn check_current_review_evidence(
    root: &Path,
    work: &Work,
    task_id: &str,
    settings: &crate::EngineSettingsDocument,
) -> Result<Option<ReviewEvidenceObservation>, ReviewEvidenceError> {
    settings.validate().map_err(ReviewEvidenceError::Settings)?;
    let limits = if work
        .planning
        .as_deref()
        .is_some_and(|p| p.reviewer.is_some())
    {
        Some(
            settings
                .values
                .context_policy_limits()
                .map_err(ReviewEvidenceError::Settings)?,
        )
    } else {
        None
    };
    check_review_evidence_policy(root, work, task_id, limits.as_ref(), Some(settings), CurrentReviewUse::Acceptance)
}
/// Verify a current native correction verdict and its complete original chain.
/// Ordinary/managed, stale, protocol failures and absent settings cannot become
/// production retry evidence. Original control/report/context/contract errors
/// retain causes. No attempt, reservation, permission or Work mutation occurs;
/// a true persisted retry signal additionally requires its canonical decision.
pub(crate) fn check_current_native_change_request(root: &Path, work: &Work, task_id: &str, settings: &crate::EngineSettingsDocument) -> Result<Option<ReviewEvidenceObservation>, ReviewEvidenceError> {
    settings.validate().map_err(ReviewEvidenceError::Settings)?;
    let task = work.tasks.iter().flatten().find(|t| t.id == task_id).ok_or(ReviewEvidenceError::Refused("review-task-not-found"))?;
    if task.independent_review.as_deref().is_none_or(|r| !native_receipt_reference(&r.receipt)) {
        return Err(ReviewEvidenceError::Refused("native-change-request-required"));
    }
    let limits = settings.values.context_policy_limits().map_err(ReviewEvidenceError::Settings)?;
    check_review_evidence_policy(root, work, task_id, Some(&limits), Some(settings), CurrentReviewUse::ProductionRetry)
}
/// Verify a known current native protocol failure or terminal interruption for
/// reviewer-only correction; neither is an applicable quality verdict.
/// Actual original typed cause/wait/capture are checked independently of the
/// public error text. Same current controls/report/contract/project are mandatory;
/// a non-verdict remains inapplicable for acceptance or production. This creates
/// no call, attempt, signal, reservation or mutation; all failures retain causes.
pub(crate) fn check_current_native_reviewer_failure(root: &Path, work: &Work, task_id: &str, settings: &crate::EngineSettingsDocument) -> Result<Option<ReviewEvidenceObservation>, ReviewEvidenceError> {
    settings.validate().map_err(ReviewEvidenceError::Settings)?;
    let task = work.tasks.iter().flatten().find(|t| t.id == task_id).ok_or(ReviewEvidenceError::Refused("review-task-not-found"))?;
    if task.independent_review.as_deref().is_none_or(|r| !native_receipt_reference(&r.receipt)) {
        return Err(ReviewEvidenceError::Refused("native-protocol-review-required"));
    }
    let limits = settings.values.context_policy_limits().map_err(ReviewEvidenceError::Settings)?;
    check_review_evidence_policy(root, work, task_id, Some(&limits), Some(settings), CurrentReviewUse::ReviewerRetry)
}
/// Shared binding check: ordinary callers cannot invent a managed policy fallback.
/// Reads current project/artifact/Git observations, returning bound data or typed
/// causes without mutation or authority. All supplied policies need a Root owner.
fn check_review_evidence_policy(
    root: &Path,
    work: &Work,
    task_id: &str,
    limits: Option<&ContextPolicyLimits>,
    settings: Option<&crate::EngineSettingsDocument>,
    purpose: CurrentReviewUse,
) -> Result<Option<ReviewEvidenceObservation>, ReviewEvidenceError> {
    use ReviewEvidenceError as E;
    let mut tasks = work.tasks.iter().flatten().filter(|t| t.id == task_id);
    let task = tasks.next().ok_or(E::Refused("review-task-not-found"))?;
    if tasks.next().is_some() {
        return Err(E::Refused("review-task-ambiguous"));
    }
    let Some(planning) = work.planning.as_deref() else {
        return Ok(None);
    };
    if planning.reviewer.is_none() {
        return Err(E::Refused("reviewer-required"));
    }
    let review = task
        .independent_review
        .as_deref()
        .filter(|r| r.state == purpose.state())
        .ok_or(E::Refused("review-required"))?;
    if native_receipt_reference(&review.receipt) {
        let _settings = settings.ok_or(E::Refused("native-review-current-root-required"))?;
        #[cfg(target_os = "linux")]
        crate::RootStore::verify_current_native_review(root, work, task_id, purpose)
            .map_err(|e| E::Native(Box::new(e)))?;
        #[cfg(not(target_os = "linux"))]
        return Err(E::Refused("native-review-platform-unported"));
    }
    let limits = limits.ok_or(E::Refused("review-context-policy-required"))?;
    let previous = review
        .workflow
        .as_deref()
        .and_then(|w| w.project.as_deref());
    crate::check_project_context_freshness(root, previous, "reviewer", limits)
        .map_err(E::Context)?;
    let latest = task
        .attempts
        .as_ref()
        .and_then(|v| v.last())
        .ok_or(E::Refused("review-attempt-or-contract-stale"))?;
    let contract = crate::review_contract(task).map_err(E::Contract)?;
    if review.attempt != latest.id || review.contract != contract {
        return Err(E::Refused("review-attempt-or-contract-stale"));
    }
    let digest = if planning.repository.is_some() {
        let settings = settings.ok_or(E::Refused("managed-review-not-ported"))?;
        #[cfg(target_os = "linux")]
        {
            crate::inspect_managed_review_git(root, work, task_id, settings)?
                .anchor
                .report_sha256
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = settings;
            return Err(E::Refused("managed-review-platform-not-ported"));
        }
    } else {
        for (name, expected) in review.report_artifacts.iter().flatten() {
            let (_, actual) =
                crate::gate_evaluation::artifact_hash(root, name).map_err(E::Artifact)?;
            if &actual != expected {
                return Err(E::Refused("review-artifact-stale"));
            }
        }
        let (_, digest) =
            crate::gate_evaluation::artifact_hash(root, &review.report).map_err(E::Artifact)?;
        if digest != review.sha256 {
            return Err(E::Refused("review-report-stale"));
        }
        digest
    };
    Ok(Some(ReviewEvidenceObservation {
        review_id: review.id.clone(),
        task_id: task.id.clone(),
        attempt_id: latest.id.clone(),
        contract,
        report: review.report.clone(),
        report_sha256: digest,
    }))
}
