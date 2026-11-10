//! Private schema3 managed-receipt binding for native task validation.
//!
//! [`bound`] classifies one private [`Receipt`]. Ordinary native v1/v2
//! originals stay unmanaged and keep their exact serialization and replay
//! behavior: they yield `Ok(None)` and must not carry a managed candidate. A
//! schema3 original must carry a managed candidate plus an origin and no
//! separate effective policy, and yields a [`Binding`] only after the complete
//! stored custody chain verifies. Unknown versions refuse. There is no
//! fallback, and this is not managed-execution enablement: the actual
//! `prepare_source` managed-unported guard remains in force.
//!
//! Every check is pure data/shape/preimage validation over the receipt and
//! its stored candidate, plan and embedded origin claim; this module performs
//! no IO, SQL, process or mutation. It can never establish actual Work cohort
//! completeness, saved-limit normalization, permission, owned candidate
//! witnesses, a departed native owner or task acceptance. Parent original
//! readers verify the actual SQL/Job source, the per-slot StepReceipt/result
//! mapping and public projection, and a later Root claim must reproduce the
//! plan from actual Work/settings. Stored normalization is never recomputed
//! here and no aggregate policy, cap, retry or timeout is invented: the whole
//! stored plan is bound to the source origin by `Origin::verify_controls`, so
//! replay cannot reconfigure limits. For schema3 the origin's controls
//! preimage is exactly this stored plan and its `candidate_binding_sha256` is
//! the candidate's full private `binding_digest`; parent claim and readback
//! must hash exactly these documents.

use super::runtime::Receipt;
use super::source_origin::Origin;
use crate::managed_repository::{ManagedCandidate, ManagedControlPlan};
use crate::{AgentLaunchError as A, ValidationPolicy};

/// Preserve typed codec causes under one safe static managed-receipt family.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> A {
    A::caused(code, error)
}

/// Borrowed managed custody extracted from a verified schema3 receipt.
///
/// `candidate` and `plan` borrow the private receipt data and stay immutable;
/// `candidate_sha256` is the full recomputed private binding digest that
/// equaled both the candidate's recorded `binding_sha256` (via
/// `verify_binding`) and the origin's recorded `candidate_binding_sha256`
/// (via `verify_candidate_binding`). Holding a `Binding` grants nothing: no
/// permission, SQL backing, witness, owner departure or task acceptance is
/// established by these borrowed values.
pub(super) struct Binding<'a> {
    pub candidate: &'a ManagedCandidate,
    pub plan: &'a ManagedControlPlan,
    pub candidate_sha256: String,
}

/// Classify one private receipt and fully verify its managed binding.
///
/// Returns `Ok(None)` for ordinary native v1/v2 originals, which must not
/// carry a managed candidate (their encoding and replay behavior stay
/// unchanged), and `Ok(Some(Binding))` for a schema3 original whose custody
/// verifies completely. Refusals are safe static codes; codec failures keep
/// their typed causes. There is no fallback: a schema3 receipt without a
/// candidate, any candidate on v1/v2, or an unknown version refuses.
///
/// Verified, in order: the strict version/presence matrix (schema3 requires a
/// candidate and an origin and no separate effective policy; v1/v2 refuse any
/// candidate; unknown versions refuse), the full candidate `verify_binding`
/// (recorded digest equals the recomputed preimage), candidate
/// work/task/agent/attempt equality with the outer job, plan presence,
/// plan/candidate contract equality with valid SHA-256 shape, origin
/// candidate-binding equality for the recomputed digest, origin controls
/// preimage equality over the entire stored plan, stored cohort shape (valid
/// unique task ids, trimmed-automatic originals with nonblank actor and
/// authorized_at, automatic normalized policies with cleared authority
/// fields, nonempty original and normalized controls, valid unique normalized
/// control ids per task), the selected task occurring exactly once with its
/// original policy equal to `r.policy`, exact slot flattening of the
/// normalized policies in stored order with global ordinals, task ids,
/// unchanged stored control bodies and unique canonical transport SHA-256
/// digests over `encode_model((&task_id, &control.id))`, and a total slot
/// count equal to both `r.job.control_count` and `r.steps.len()`.
///
/// Limits: pure validation with no IO, SQL, process or mutation. It cannot
/// establish actual Work cohort completeness, saved-limit normalization,
/// permission, owned candidate witnesses, native owner departure or task
/// acceptance; the parent readers verify the actual origin row and Job
/// source, and per-slot result mapping plus public projection remain parent
/// duties.
pub(super) fn bound(r: &Receipt) -> Result<Option<Binding<'_>>, A> {
    match r.schema_version {
        // Historical originals never carry custody; requiring absence keeps
        // their exact bytes and replay path untouched.
        1 | 2 => {
            if r.managed_candidate.is_some() {
                return Err(A::Refused("task-validation-managed-receipt-legacy-candidate"));
            }
            Ok(None)
        }
        3 => {
            let candidate = r
                .managed_candidate
                .as_ref()
                .ok_or(A::Refused("task-validation-managed-receipt-candidate-required"))?;
            if r.effective_policy.is_some() {
                return Err(A::Refused("task-validation-managed-receipt-policy-present"));
            }
            let origin = r
                .source_origin
                .as_ref()
                .ok_or(A::Refused("task-validation-managed-receipt-origin-required"))?;
            Ok(Some(verify(candidate, origin, r)?))
        }
        _ => Err(A::Refused("task-validation-managed-receipt-version")),
    }
}

/// Verify the complete schema3 managed custody chain and return the borrowed
/// binding: candidate self-binding, outer job identity, plan/contract, the
/// recomputed candidate digest against the origin claim, the origin controls
/// preimage over the whole plan, cohort shape, then slot flattening. Pure
/// preimage/shape work with the limits documented on [`bound`].
fn verify<'a>(
    candidate: &'a ManagedCandidate,
    origin: &Origin,
    r: &Receipt,
) -> Result<Binding<'a>, A> {
    // Full candidate self-binding: schema, nonempty plan, recorded digest.
    candidate.verify_binding()?;
    if candidate.work != r.job.work_id
        || candidate.task != r.job.task_id
        || candidate.agent != r.producer_agent_id
        || candidate.attempt != r.attempt_id
    {
        return Err(A::Refused("task-validation-managed-receipt-candidate-mismatch"));
    }
    let plan = candidate
        .control_plan
        .as_ref()
        .ok_or(A::Refused("task-validation-managed-receipt-plan-required"))?;
    if !crate::provider_configuration::configuration_digest_valid(&candidate.contract) {
        return Err(A::Refused("task-validation-managed-receipt-contract-invalid"));
    }
    if plan.contract != candidate.contract {
        return Err(A::Refused("task-validation-managed-receipt-contract-mismatch"));
    }
    let candidate_sha256 = candidate
        .binding_digest()
        .map_err(|e| caused("task-validation-managed-candidate-digest", e))?;
    origin.verify_candidate_binding(Some(candidate_sha256.as_str()))?;
    origin.verify_controls(&r.job.id, plan)?;
    cohort(plan, &r.policy, &r.job.task_id)?;
    slots(plan, r)?;
    Ok(Binding {
        candidate,
        plan,
        candidate_sha256,
    })
}

/// Check stored cohort policy shape without re-normalizing anything.
///
/// Every participating task id must be valid and unique; each original policy
/// must be trimmed-automatic with nonblank actor and authorized_at; each
/// normalized policy must be automatic with cleared authority fields, nonempty
/// original and normalized controls, and valid unique normalized control ids
/// per task (equal names across different tasks remain valid). The selected
/// task must occur exactly once and its original policy must equal the
/// receipt's original policy value. Nothing here checks task statuses,
/// criteria coverage or Work membership against actual data, and no limits
/// are loaded or invented; normalization was bound to the origin elsewhere.
fn cohort(
    plan: &ManagedControlPlan,
    receipt_policy: &ValidationPolicy,
    selected: &str,
) -> Result<(), A> {
    let mut seen = std::collections::BTreeSet::new();
    let mut matched: Option<&ValidationPolicy> = None;
    for policy in &plan.policies {
        if !swarm_identity_contract::name(policy.task_id.as_bytes())
            || !seen.insert(policy.task_id.as_str())
        {
            return Err(A::Refused("task-validation-managed-receipt-cohort-invalid"));
        }
        if crate::text::trim_space(&policy.original.mode) != "automatic"
            || crate::text::trim_space(&policy.original.actor).is_empty()
            || crate::text::trim_space(&policy.original.authorized_at).is_empty()
        {
            return Err(A::Refused("task-validation-managed-receipt-policy-unauthorized"));
        }
        if policy.normalized.mode != "automatic"
            || !policy.normalized.actor.is_empty()
            || !policy.normalized.authorized_at.is_empty()
        {
            return Err(A::Refused("task-validation-managed-receipt-normalization-invalid"));
        }
        normalization(&policy.original, &policy.normalized)?;
        if policy
            .original
            .controls
            .as_deref()
            .is_none_or(|c| c.is_empty())
            || policy
                .normalized
                .controls
                .as_deref()
                .is_none_or(|c| c.is_empty())
        {
            return Err(A::Refused("task-validation-managed-receipt-controls-required"));
        }
        let mut ids = std::collections::BTreeSet::new();
        for control in policy.normalized.controls.iter().flatten() {
            if !swarm_identity_contract::name(control.id.as_bytes())
                || !ids.insert(control.id.as_str())
            {
                return Err(A::Refused("task-validation-managed-receipt-control-id-invalid"));
            }
        }
        if policy.task_id == selected {
            if matched.is_some() {
                return Err(A::Refused("task-validation-managed-receipt-selection-mismatch"));
            }
            matched = Some(&policy.original);
        }
    }
    if matched != Some(receipt_policy) {
        return Err(A::Refused("task-validation-managed-receipt-selection-mismatch"));
    }
    Ok(())
}

/// Verify the stored slots are the exact flattening of the stored normalized
/// policies: stored policy order and per-policy control order, global
/// zero-based ordinals, task ids, the stored control bodies unchanged, and
/// canonical transport SHA-256 digests over `encode_model((&task_id,
/// &control.id))` unique across the whole plan. Equal control names in
/// different tasks stay valid and distinct because the task id is part of the
/// preimage. The total slot count must equal both the original
/// `job.control_count` and `r.steps.len()`; per-slot StepReceipt/result
/// validation is a parent duty. Codec failures keep their typed causes;
/// everything else refuses statically. No normalization is recomputed.
fn slots(plan: &ManagedControlPlan, r: &Receipt) -> Result<(), A> {
    let mut transports = std::collections::BTreeSet::new();
    let mut index = 0usize;
    for policy in &plan.policies {
        for control in policy.normalized.controls.iter().flatten() {
            let slot = plan
                .slots
                .get(index)
                .ok_or(A::Refused("task-validation-managed-receipt-slot-changed"))?;
            let raw = crate::encode_model(&(&policy.task_id, &control.id))
                .map_err(|e| caused("task-validation-managed-receipt-slot-encode", e))?;
            if slot.ordinal != index
                || slot.task_id != policy.task_id
                || slot.control != *control
                || slot.transport_id != crate::provider_configuration::configuration_digest(&raw)
            {
                return Err(A::Refused("task-validation-managed-receipt-slot-changed"));
            }
            if !transports.insert(slot.transport_id.as_str()) {
                return Err(A::Refused("task-validation-managed-receipt-slot-collision"));
            }
            index += 1;
        }
    }
    if index != plan.slots.len() {
        return Err(A::Refused("task-validation-managed-receipt-slot-changed"));
    }
    if usize::try_from(r.job.control_count).ok() != Some(index) || r.steps.len() != index {
        return Err(A::Refused("task-validation-managed-receipt-slot-count"));
    }
    Ok(())
}

/// Safe data-normalization refusal; no policy limit or authority implied.
fn normalization_invalid() -> A {
    A::Refused("task-validation-managed-receipt-normalization-invalid")
}
/// Check exact policy normalization shape without guessing historical limits.
/// Only trimming, authority removal and a positive resolved default timeout may
/// differ; supplied stored defaults are observations, not current-policy grants.
/// Pure comparison; errors refuse rather than silently using original commands.
fn normalization(original: &crate::ValidationPolicy, normalized: &crate::ValidationPolicy) -> Result<(), A> {
    let mut expected = original.clone();
    expected.mode = crate::text::trim_space(&expected.mode).into();
    expected.actor.clear(); expected.authorized_at.clear();
    let actual = normalized.controls.as_deref().filter(|c| !c.is_empty()).ok_or_else(normalization_invalid)?;
    let controls = expected.controls.as_mut().filter(|c| c.len()==actual.len()).ok_or_else(normalization_invalid)?;
    for (control, resolved) in controls.iter_mut().zip(actual) {
        control.id = crate::text::trim_space(&control.id).into();
        control.justification = crate::text::trim_space(&control.justification).into();
        if resolved.timeout_seconds < 1 { return Err(normalization_invalid()); }
        if control.timeout_seconds == 0 { control.timeout_seconds = resolved.timeout_seconds; }
    }
    if expected != *normalized { return Err(normalization_invalid()); }
    Ok(())
}
