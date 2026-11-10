//! Read-only operator audit of an existing transaction, never effect authority.
use super::{encoded, proof, state::State, TrustError as E};
use serde::Deserialize;
use serde_json::{json, Value};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    action_id: String,
}
/// Verify the full recorded chain and correlate one action with its admitted
/// mandate, exact approval and native receipt. Returns minimized event metadata,
/// explicit missing links and completeness; no raw originals, tokens or writes.
/// The storage boundary owns operator/I3 authority. Local hash validity proves
/// neither collection completeness, external freshness nor immutable storage.
pub(crate) fn transaction(st: &State, raw: &[u8]) -> Result<Vec<u8>, E> {
    let input: Input = swarm_wire_json::decode(raw)?;
    if !swarm_identity_contract::name(input.action_id.as_bytes()) {
        return Err(E::Invalid("trust-audit-action-id"));
    }
    let a = st
        .actions
        .get(&input.action_id)
        .ok_or(E::Invalid("action absente"))?;
    let text = |key: &str| {
        a.get(key)
            .and_then(Value::as_str)
            .ok_or(E::Integrity("trust-audit-action-shape"))
    };
    let content = text("content_digest")?;
    let mandate_id = text("mandate_id")?;
    let state = text("state")?;
    let export = proof::export(st)?;
    proof::verify_chain(&export)?;
    super::operations::bound_action(st, a)?;
    let events: Vec<_> = st
        .evidence
        .iter()
        .filter(|e| {
            e.object_id == input.action_id
                && !e.kind.starts_with("mandate-")
                && e.kind != "audit-access"
                || e.object_id == mandate_id && e.kind.starts_with("mandate-")
        })
        .collect();
    let mut missing = Vec::new();
    let mandate = st.mandates.get(mandate_id);
    if !events.iter().any(|e| {
        e.kind == "mandate-create"
            && e.result == "admitted"
            && e.object_id == mandate_id
            && mandate.and_then(|m| m["original_digest"].as_str()) == Some(e.original_digest.as_str())
    }) {
        missing.push("mandate-admission");
    }
    if !events.iter().any(|e| {
        e.kind == "request"
            && e.result == "admitted"
            && e.object_id == input.action_id
            && e.content_digest == content
    }) {
        missing.push("request-admission");
    }
    let approved = events.iter().any(|e| {
        e.kind == "approve"
            && e.result == "approved"
            && e.object_id == input.action_id
            && e.content_digest == content
            && a["approver"].as_str() == Some(e.actor.as_str())
    });
    if matches!(state, "approved" | "committed" | "unknown" | "reconciled") && !approved {
        missing.push("exact-approval");
    }
    let receipt = st.connector_receipts.get(text("idempotency_key")?);
    let has_effect = matches!(state, "committed" | "unknown" | "reconciled");
    if has_effect
        && !receipt.is_some_and(|r| r.action_id == input.action_id && r.content_digest == content)
    {
        missing.push("native-receipt");
    }
    if has_effect
        && !events.iter().any(|e| {
            matches!(e.kind.as_str(), "commit" | "reconcile")
                && e.object_id == input.action_id
                && e.content_digest == content
                && e.effect_receipt.is_some()
        })
    {
        missing.push("effect-proof");
    }
    let last = events.iter().rev().find(|e| {
        e.object_id == input.action_id && e.content_digest == content && e.result != "denied"
    });
    let expected = last.map(|e| {
        if e.kind == "request" && e.result == "admitted" {
            "pending_approval"
        } else {
            e.result.as_str()
        }
    });
    if expected != Some(state) {
        missing.push("action-state-link");
    }
    if let Some(r) = receipt {
        let last = events.iter().rev().find_map(|e| e.effect_receipt.as_ref());
        let linked = if let Some(p) = last {
            serde_json::to_value(&p.intent)? == serde_json::to_value(&r.intent)?
                && serde_json::to_value(&p.result)? == serde_json::to_value(&r.result)?
        } else {
            false
        };
        if !linked {
            missing.push("receipt-state-link");
        }
    }
    let records:Vec<_>=events.iter().map(|e|json!({"sequence":e.sequence.to_string(),"kind":e.kind,"actor":e.actor,"integrity":e.integrity,"object_id":e.object_id,"content_digest":e.content_digest,"original_digest":e.original_digest,"hash":e.hash,"previous_hash":e.previous_hash,"result":e.result,"reason":e.reason,"at":e.at,"policy_revision":e.policy_context.as_ref().map(|p|p.revision),"role_separation":e.policy_context.as_ref().and_then(|p|p.role_separation.as_ref())})).collect();
    encoded(
        &json!({"schema_version":1,"kind":"trust-transaction-audit-v1","action_id":input.action_id,"mandate_id":mandate_id,"state":state,"content_digest":content,"chain_verified":true,"report_prefix_sequence":st.evidence.last().map(|e|e.sequence.to_string()),"transaction_verified":missing.is_empty(),"complete":matches!(state,"committed"|"reconciled")&&missing.is_empty(),"missing_evidence":missing,"events":records,"effect_receipt":receipt,"simulation":true,"external_checkpoint_verified":false,"physical_immutability_verified":false,"independent_acceptance":false}),
    )
}
