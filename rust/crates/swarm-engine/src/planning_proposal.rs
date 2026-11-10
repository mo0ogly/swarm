//! Strict model proposal data. Decoding never constructs execution authority.
use crate::{AgentLaunchError as E, PlanningDecisionPolicy as P, PlanningOperation};
use serde::Deserialize;
use std::collections::BTreeSet;

/// Model-supplied suggestions, with no actor, clock, lease or event authority.
/// Required top-level fields reject null/missing values. Operations remain
/// untrusted Go-compatible data: decoding does not prove support, ownership,
/// requirements, delivery membership, permissions or acceptance.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningProposal {
    pub input_events: Vec<String>,
    pub reason: String,
    pub operations: Vec<PlanningOperation>,
}

/// Decode one complete UTF8 JSON proposal using explicit current saved caps.
/// Reject unknown/duplicate fields, trailing data and malformed nested operations;
/// require nonempty distinct protocol event names and a nonblank reason. Reason
/// limits count UTF8 bytes, never characters. Null caps remove only their policy
/// ceiling, not the structural checks. No fallback, truncation, retry or IO occurs.
/// Original parser causes remain behind static safe diagnostics without rendering
/// model content. Allocation is proportional to input: a future provider reader
/// must enforce its own persisted transport policy before collecting bytes.
/// Callers must construct engine-owned request metadata and rerun authoritative
/// current-state decision/lease checks; this result grants no apply/start right.
pub fn decode_planning_proposal(raw: &[u8], policy: &P) -> Result<PlanningProposal, E> {
    let proposal: PlanningProposal =
        serde_json::from_slice(raw).map_err(|e| E::caused("planning-proposal-format", e))?;
    if proposal.input_events.is_empty() {
        return Err(E::Refused("planning-proposal-inputs-required"));
    }
    if !crate::text::nonempty(&proposal.reason) {
        return Err(E::Refused("planning-proposal-reason-required"));
    }
    P::check(
        policy.max_inputs,
        proposal.input_events.len(),
        "planning-decision-input-limit",
    )?;
    P::check(
        policy.max_operations,
        proposal.operations.len(),
        "planning-decision-operation-limit",
    )?;
    P::check(
        policy.max_reason_bytes,
        proposal.reason.len(),
        "planning-decision-reason-limit",
    )?;
    let mut seen = BTreeSet::new();
    for id in &proposal.input_events {
        if !swarm_identity_contract::name(id.as_bytes()) || !seen.insert(id) {
            return Err(E::Refused("planning-proposal-input-invalid"));
        }
    }
    Ok(proposal)
}
