//! Actual current policy attribution and fail-closed compatibility checks.
use super::{PaeaSettlementError, Result, RootStore, R};
use swarm_enrollment_authority::paea_action::{Identity, Operation};
use swarm_protection_contract::AdmissionContext;
/// Read the actual protected configuration under a caller-held Trust reservation.
/// Preserve codec/validation causes and refuse unsupported active controls.
fn read(project: &RootStore) -> Result<crate::trust::state::State> {
    let raw = project
        .directory
        .read_bounded(
            super::super::trust_store::STATE,
            super::super::trust_store::MAX_STATE,
        )
        .map_err(|e| PaeaSettlementError::caused("paea-settlement-trust-read", e))?;
    let state = crate::trust::state::State::decode(&raw)
        .map_err(|e| PaeaSettlementError::caused("paea-settlement-trust-decode", e))?;
    state
        .validate()
        .map_err(|e| PaeaSettlementError::caused("paea-settlement-trust-validate", e))?;
    state
        .require_supported()
        .map_err(|e| PaeaSettlementError::caused("paea-settlement-trust-unsupported", e))?;
    Ok(state)
}
/// Domain-separated digest of the real base policy, optional role policy and
/// control blocks. Configuration revision independently fences identity changes.
/// This is a pinned policy projection, not an independent signature or JCS claim.
fn fingerprint(state: &crate::trust::state::State) -> Result<String> {
    let controls = state
        .controls()
        .map_err(|e| PaeaSettlementError::caused("paea-settlement-policy-controls", e))?;
    crate::trust::digest(&serde_json::json!({"domain":"paea-trust-policy-v1","policy":state.policy,"role_separation":state.role_separation,"controls":controls})).map_err(|e|PaeaSettlementError::caused("paea-settlement-policy-digest",e))
}
/// Produce a separate context with policy fields from opened current storage.
/// Validate supplied representations first; preserve all other declared fields
/// and original input. A close error refuses preparation without a business effect.
pub(super) fn attribute(
    project: &RootStore,
    declared: &AdmissionContext,
) -> Result<AdmissionContext> {
    declared
        .validate()
        .map_err(|e| PaeaSettlementError::caused("paea-settlement-context", e))?;
    let reservation = project
        .directory
        .reserve_trust(false)
        .map_err(|e| PaeaSettlementError::caused("paea-settlement-policy-reservation", e))?;
    let result = (|| {
        let state = read(project)?;
        let mut context = declared.clone();
        context.policy_revision = state.revision;
        context.policy_digest = fingerprint(&state)?;
        Ok(context)
    })();
    match reservation.close() {
        Ok(()) => result,
        Err(cause) => match result {
            Ok(_) => Err(PaeaSettlementError::caused(
                "paea-settlement-policy-release",
                cause,
            )),
            Err(mut e) => {
                e.cleanup.push(R::Storage(cause));
                Err(e)
            }
        },
    }
}
/// Re-read the actual configured Trust revision and canonical effect role while
/// its shared reservation is held. PAEA permission never substitutes for a
/// disabled/missing role mapping or a newer configuration. The existing bounded
/// state reader and shared Biba invariant are reused without a new policy.
pub(super) fn current_actor(
    project: &RootStore,
    identity: &Identity<'_>,
    context: &AdmissionContext,
) -> Result<()> {
    let state = read(project)?;
    if identity.trust_revision() != state.revision {
        return Err(PaeaSettlementError::refused(
            "paea-settlement-trust-revision",
        ));
    }
    let actor = state
        .identities
        .iter()
        .find(|i| {
            !i.disabled
                && i.enrollment_principal.as_ref() == Some(identity.principal())
                && i.roles.iter().any(|r| r == "effect")
        })
        .ok_or_else(|| PaeaSettlementError::refused("paea-settlement-effect-role"))?;
    let level = match actor.integrity {
        0 => crate::biba::Integrity::I0,
        1 => crate::biba::Integrity::I1,
        2 => crate::biba::Integrity::I2,
        3 => crate::biba::Integrity::I3,
        _ => {
            return Err(PaeaSettlementError::refused(
                "paea-settlement-role-integrity",
            ))
        }
    };
    crate::biba::check_biba(
        level,
        crate::biba::Integrity::I2,
        crate::biba::BibaAccess::Invoke,
    )
    .map_err(|e| PaeaSettlementError::caused("paea-settlement-effect-integrity", e))?;
    if identity.request().operation() == Operation::Commit {
        if state.policy.require_separation
            || state
                .role_separation
                .as_ref()
                .is_some_and(|p| p.enabled && !p.incompatible_roles.is_empty())
        {
            return Err(PaeaSettlementError::refused(
                "paea-settlement-attributed-admission-required",
            ));
        }
        if context.policy_revision != state.revision
            || context.policy_digest != fingerprint(&state)?
        {
            return Err(PaeaSettlementError::refused("paea-settlement-policy-stale"));
        }
    }
    Ok(())
}
