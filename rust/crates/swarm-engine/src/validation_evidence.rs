//! Current receipt consistency only. Stored claims are not actor authentication,
//! execution provenance, acceptance or a lease on future filesystem state.
use crate::{AutomaticValidation, QualityPolicySnapshot, Task, ValidationPolicy};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt, path::Path};

/// Checked supplied receipt and attributed artifacts; output contains private
/// data, so Debug omits it. Only complete policy-shared output becomes a quote source.
#[derive(Clone, PartialEq, Eq, serde::Serialize)]
pub struct ValidationEvidenceObservation {
    pub receipt_json: String,
    pub artifacts: BTreeMap<String, String>,
    pub observations: Vec<String>,
}
impl fmt::Debug for ValidationEvidenceObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ValidationEvidenceObservation { data: redacted }")
    }
}

pub enum ValidationEvidenceError {
    Refused(&'static str),
    Encode(crate::WorkError),
    Gate(crate::GateContractError),
    Artifact(crate::GateEvaluationError),
}
impl ValidationEvidenceError {
    /// Stable public family; private paths, commands and observations stay out of diagnostics.
    pub fn code(&self) -> &'static str {
        "validation-evidence-invalid"
    }
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Refused(reason) => reason,
            Self::Encode(_) => "receipt-encoding",
            Self::Gate(_) => "current-gate-required",
            Self::Artifact(_) => "current-artifact-required",
        }
    }
}
impl fmt::Display for ValidationEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for ValidationEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for ValidationEvidenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Refused(_) => None,
            Self::Encode(e) => Some(e),
            Self::Gate(e) => Some(e),
            Self::Artifact(e) => Some(e),
        }
    }
}

/// SHA-256 of Go-compatible policy JSON with attribution fields omitted, as in
/// validationPolicyDigest. No normalization, authorization or mutation is performed.
/// Codec failure is preserved; caller-supplied policy remains unauthenticated data.
pub fn validation_policy_digest(policy: &ValidationPolicy) -> Result<String, crate::WorkError> {
    let mut policy = policy.clone();
    policy.authorized_at.clear();
    policy.actor.clear();
    Ok(format!(
        "{:x}",
        Sha256::digest(crate::encode_model(&policy)?)
    ))
}

/// Inspect current validation claims and real ordinary artifact bytes without
/// executing a control. Root callers must provide their stored task/settings.
/// No configured controls returns None; otherwise latest attempt, controller URI,
/// policy digest, ordered commands/results, receipt membership and every artifact
/// hash must agree. Automatic mode also rechecks the actual gate using the supplied
/// quality profile; no missing profile is silently replaced by defaults.
///
/// The explicit output cap is the persisted validation-output setting (None is
/// unlimited). Review output is only a quote source when opt-in, nonempty and
/// complete; a consistent truncated prefix is retained as data but never promoted.
/// UTF-8 sizes are bytes. Missing/unreadable/changed artifacts, inconsistent data,
/// codec and checked artifact-close failures return errors preserving their cause.
/// No identity, engine-execution provenance, independent approval or acceptance
/// follows from success. Returned facts must be rechecked before later use.
pub fn check_validation_evidence(
    root: &Path,
    task: &Task,
    quality: Option<&QualityPolicySnapshot>,
    max_review_output_bytes: Option<usize>,
) -> Result<Option<ValidationEvidenceObservation>, ValidationEvidenceError> {
    use ValidationEvidenceError as E;
    let Some(policy) = task
        .validation_policy
        .as_deref()
        .filter(|p| p.controls.as_ref().is_some_and(|v| !v.is_empty()))
    else {
        return Ok(None);
    };
    let a: &AutomaticValidation = task
        .automatic_validation
        .as_deref()
        .ok_or(E::Refused("current-validation-required"))?;
    let attempt = task
        .attempts
        .as_ref()
        .and_then(|v| v.last())
        .map(|a| a.id.as_str())
        .filter(|v| !crate::text::trim_space(v).is_empty())
        .unwrap_or("unknown");
    if a.attempt_id != attempt
        || a.controller != "controller://swarm-validation"
        || a.policy_digest != validation_policy_digest(policy).map_err(E::Encode)?
    {
        return Err(E::Refused("current-validation-required"));
    }
    if policy.mode == "automatic" {
        crate::check_gate_evidence(
            task,
            root,
            quality.ok_or(E::Refused("quality-policy-required"))?,
        )
        .map_err(E::Gate)?;
    }
    if !matches!(
        a.state.as_str(),
        "pending_review" | "accepted" | "pending_human"
    ) {
        return Err(E::Refused("successful-controls-required"));
    }
    let controls = a.controls.as_deref().unwrap_or_default();
    let configured = policy.controls.as_deref().unwrap_or_default();
    let artifacts = a
        .artifacts
        .as_ref()
        .ok_or(E::Refused("complete-receipt-required"))?;
    if controls.len() != configured.len()
        || a.receipt.is_empty()
        || artifacts.get(&a.receipt).is_none_or(|s| s.is_empty())
    {
        return Err(E::Refused("complete-receipt-required"));
    }
    for (result, control) in controls.iter().zip(configured) {
        if result.id != control.id
            || result.command != control.command
            || !result.executed
            || !result.passed
            || result.exit_code != 0
            || result.started_at.is_empty()
            || result.finished_at.is_empty()
        {
            return Err(E::Refused("control-not-demonstrated"));
        }
    }
    let mut observations = Vec::new();
    for (result, control) in controls.iter().zip(configured) {
        if !control.review_output {
            if !result.review_output.is_empty()
                || result.output_bytes != 0
                || result.review_output_truncated
            {
                return Err(E::Refused("control-output-not-authorized"));
            }
            continue;
        }
        let retained = result.review_output.len();
        let total = usize::try_from(result.output_bytes)
            .map_err(|_| E::Refused("control-output-incoherent"))?;
        if max_review_output_bytes.is_some_and(|cap| retained > cap)
            || total < retained
            || result.review_output_truncated != (total > retained)
        {
            return Err(E::Refused("control-output-incoherent"));
        }
        if !result.review_output_truncated && retained != 0 {
            observations.push(result.review_output.clone());
        }
    }
    for (name, expected) in artifacts {
        let (_, actual) = crate::gate_evaluation::artifact_hash(root, name).map_err(E::Artifact)?;
        if &actual != expected {
            return Err(E::Refused("validation-artifact-stale"));
        }
    }
    let receipt_json = String::from_utf8(crate::encode_model(a).map_err(E::Encode)?)
        .expect("JSON serializer emits UTF-8");
    Ok(Some(ValidationEvidenceObservation {
        receipt_json,
        artifacts: artifacts.clone(),
        observations,
    }))
}
