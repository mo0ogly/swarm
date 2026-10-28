//! Gate evidence contract only. Supplied policy/time are observations, not authority.
//! Task acceptance also requires actual identity, review, dependency and Store guards.
use crate::{
    gate_evaluation::records, gate_json::Document, text, Evaluation, GateEvaluationError,
    GateRecord, GoRaw, QualityPolicySnapshot, Task,
};
use serde_json::value::RawValue;
use std::{error::Error, fmt, path::Path};
pub struct GateSubmission<'a> {
    pub name: &'a str,
    pub phase: &'a str,
    pub document: &'a [u8],
    pub at: &'a str,
}
pub enum GateContractError {
    Refused(&'static str),
    Evaluation(GateEvaluationError),
    Policy(crate::EngineSettingsError),
    Json(serde_json::Error),
    Utf8(std::str::Utf8Error),
}
impl GateContractError {
    pub fn code(&self) -> &'static str {
        "gate-evidence-invalid"
    }
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Refused(r) => r,
            Self::Evaluation(_) => "evaluation",
            Self::Policy(_) => "policy-binding",
            Self::Json(_) => "document-json",
            Self::Utf8(_) => "document-utf8-required",
        }
    }
}
impl fmt::Display for GateContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for GateContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for GateContractError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Refused(_) => None,
            Self::Evaluation(e) => Some(e),
            Self::Policy(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Utf8(e) => Some(e),
        }
    }
}
type Result<T> = std::result::Result<T, GateContractError>;
fn fail(reason: &'static str) -> GateContractError {
    GateContractError::Refused(reason)
}
/// Match the exact keys and record validation used by score calculation.
pub fn validate_required_plan_checks(task: &Task, raw: &[u8]) -> Result<()> {
    let Some(required) = task.plan_checks.as_ref().filter(|m| !m.is_empty()) else {
        return Ok(());
    };
    let d = Document::parse(raw).map_err(|e| GateContractError::Evaluation(e.into()))?;
    let checks = records(&d, d.get(0, "checks")).map_err(GateContractError::Evaluation)?;
    for (id, phase) in required {
        if !checks.iter().any(|&c| {
            d.get(c, "id").and_then(|i| d.text(i)).as_deref() == Some(id)
                && d.get(c, "gate").and_then(|i| d.text(i)).as_deref() == Some(phase)
                && d.get(c, "mandatory").and_then(|i| d.boolean(i)) == Some(true)
        }) {
            return Err(fail("required-plan-check-missing-or-changed"));
        }
    }
    Ok(())
}
fn revalidation(task: &Task, evaluation: &Evaluation) -> Result<()> {
    let Some(r) = &task.revalidation else {
        return Ok(());
    };
    if r.report.is_empty()
        || evaluation.artifacts.as_ref().and_then(|m| m.get(&r.report)) != Some(&r.report_hash)
    {
        return Err(fail("revalidation-report-required"));
    }
    if evaluation.config_digest != r.config_digest {
        return Err(fail("revalidation-config-changed"));
    }
    Ok(())
}
pub(crate) fn evaluate_observed(
    raw: &[u8],
    root: &Path,
    phase: &str,
    policy: &QualityPolicySnapshot,
) -> Result<Evaluation> {
    policy
        .validate_binding()
        .map_err(GateContractError::Policy)?;
    match crate::gate_document_method(raw)
        .map_err(GateContractError::Evaluation)?
        .as_str()
    {
        "3" => crate::evaluate_gate_with_policy(raw, root, phase, policy)
            .map(|bound| bound.evaluation)
            .map_err(GateContractError::Evaluation),
        "2" if policy.legacy_method2_thresholds_compatible => {
            crate::evaluate_gate_method2(raw, root, phase).map_err(GateContractError::Evaluation)
        }
        "2" => Err(fail("historical-policy-no-longer-compatible")),
        _ => Err(fail("gate-method-unsupported")),
    }
}
/// Prepare a copy. The caller still owns the transaction, current-policy lock,
/// authorization and commit. A refused calculation never changes the input task.
pub fn prepare_gate_record(
    task: &Task,
    submission: &GateSubmission<'_>,
    root: &Path,
    policy: &QualityPolicySnapshot,
) -> Result<Task> {
    if !text::nonempty(submission.at) {
        return Err(fail("gate-observation-time-required"));
    }
    let mut next = prepare_gate_document(
        task,
        submission.name,
        submission.phase,
        submission.document,
        root,
        policy,
    )?;
    next.gate
        .as_mut()
        .ok_or_else(|| fail("gate-preparation-incomplete"))?
        .at = submission.at.into();
    Ok(next)
}
// Internal unstamped candidate: only the transaction owner stamps it after all
// business checks, before persistence. Never a persisted receipt by itself.
pub(crate) fn prepare_gate_document(
    task: &Task,
    name: &str,
    phase: &str,
    document: &[u8],
    root: &Path,
    policy: &QualityPolicySnapshot,
) -> Result<Task> {
    let name = text::trim_space(name);
    if name.is_empty() {
        return Err(fail("gate-name-required"));
    }
    if !matches!(
        task.status.as_str(),
        "todo" | "running" | "blocked" | "submitted"
    ) {
        return Err(fail("gate-task-must-be-open"));
    }
    validate_required_plan_checks(task, document)?;
    let evaluation = evaluate_observed(document, root, phase, policy)?;
    if task.revalidation.is_some() && task.status != "submitted" {
        return Err(fail("revalidation-submit-report-first"));
    }
    revalidation(task, &evaluation)?;
    if evaluation.scope_id != task.id {
        return Err(fail("gate-task-scope-mismatch"));
    }
    if task
        .gate
        .as_ref()
        .is_some_and(|g| g.evaluation.config_digest != evaluation.config_digest)
    {
        return Err(fail("gate-config-changed-reopen-required"));
    }
    let document = std::str::from_utf8(document).map_err(GateContractError::Utf8)?;
    let document =
        GoRaw(RawValue::from_string(document.to_owned()).map_err(GateContractError::Json)?);
    let mut next = task.clone();
    next.gate = Some(Box::new(GateRecord {
        name: name.into(),
        document,
        evaluation,
        at: String::new(),
    }));
    next.evidence_stale_reason.clear();
    Ok(next)
}
/// Recheck only gate evidence against the supplied observed profile. This does
/// not verify independent review, current actor authority or task dependencies.
pub fn check_gate_evidence(
    task: &Task,
    root: &Path,
    policy: &QualityPolicySnapshot,
) -> Result<Evaluation> {
    let gate = task.gate.as_ref().ok_or_else(|| fail("gate-required"))?;
    if !task.evidence_stale_reason.is_empty() {
        return Err(fail("gate-known-stale"));
    }
    if !gate.evaluation.ship_allowed {
        return Err(fail("gate-shipping-result-required"));
    }
    let raw = gate.document.0.get().as_bytes();
    validate_required_plan_checks(task, raw)?;
    let evaluation = evaluate_observed(raw, root, "delivery", policy)?;
    if !evaluation.ship_allowed || evaluation.scope_id != task.id {
        return Err(fail("gate-no-longer-valid"));
    }
    revalidation(task, &evaluation)?;
    if evaluation != gate.evaluation {
        return Err(fail("gate-receipt-does-not-match-current-calculation"));
    }
    Ok(evaluation)
}
