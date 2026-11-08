//! Exact payment agreement from current Trust state held by the caller.
//! This records a local approval, never human presence or OS isolation.
use super::{PaeaSettlementError as E, PaeaSettlementRequest, Result};
use crate::trust::{self, state::State};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

/// Observed durable approval for all canonical payment/context bytes. This
/// serializable record is audit data, not a capability or independent signature.
/// Valid only under the caller-held Trust reservation and before expires_at.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaeaPaymentAgreement {
    pub action_id: String,
    pub mandate_id: String,
    pub requester: String,
    pub approver: String,
    pub action_digest: String,
    pub evidence_hash: String,
    pub original_digest: String,
    pub policy_revision: i64,
    pub expires_at: String,
}

/// Protocol resource binding full canonical bytes, including context and policy.
/// It grants nothing; the existing Trust mandate/request/approve producers own
/// admission. Amount, currency and beneficiary are checked independently too.
pub(super) fn resource(request: &PaeaSettlementRequest) -> String {
    format!("paea-payment/{}", request.canonical().digest())
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

/// Find the newest current separated approval under an already-held Trust read
/// reservation. Verify retained originals and chain, exact resource/terms,
/// authenticated request/approval actors, current role mappings and mandate.
/// Missing/stale approval returns None; corruption preserves its cause as Err.
/// No IO, grant, mutation, fallback, nested lock or clock policy is introduced.
pub(super) fn current(
    state: &State,
    request: &PaeaSettlementRequest,
) -> Result<Option<PaeaPaymentAgreement>> {
    let proof =
        trust::proof::export(state).map_err(|e| E::caused("paea-settlement-agreement-proof", e))?;
    trust::proof::verify_chain(&proof)
        .map_err(|e| E::caused("paea-settlement-agreement-proof", e))?;
    let (_, _, amount, currency, beneficiary) = super::terms(&request.proposal)?;
    let resource = resource(request);
    for evidence in state.evidence.iter().rev() {
        if evidence.kind != "approve"
            || evidence.result != "approved"
            || evidence
                .policy_context
                .as_ref()
                .is_none_or(|p| p.revision != state.revision || !p.policy.require_separation)
        {
            continue;
        }
        let Some(action) = state.actions.get(&evidence.object_id) else {
            continue;
        };
        let requester = text(action, "requester");
        let approver = text(action, "approver");
        if text(action, "state") != "approved"
            || approver != evidence.actor
            || requester.is_empty()
            || requester == approver
            || text(action, "resource") != resource
            || action.get("amount_minor").and_then(Value::as_i64) != Some(amount)
            || text(action, "currency") != currency
            || text(action, "beneficiary") != beneficiary
            || action.get("connector").is_some_and(|v| !v.is_null())
            || text(action, "content_digest") != evidence.content_digest
        {
            continue;
        }
        let current_role = |subject: &str, role: &str, minimum: i64| {
            state.identities.iter().any(|i| {
                i.subject == subject
                    && !i.disabled
                    && i.integrity >= minimum
                    && i.roles.iter().any(|r| r == role)
            })
        };
        if !current_role(requester, "requester", 1) || !current_role(approver, "approver", 2) {
            continue;
        }
        if !state.evidence.iter().any(|e| {
            e.kind == "request"
                && e.result == "admitted"
                && e.object_id == evidence.object_id
                && e.actor == requester
                && e.content_digest == evidence.content_digest
                && e.policy_context
                    .as_ref()
                    .is_some_and(|p| p.revision == state.revision)
        }) {
            continue;
        }
        let original = state
            .originals
            .get(&evidence.original_digest)
            .ok_or_else(|| E::refused("paea-settlement-agreement-original-missing"))?;
        let original = STANDARD
            .decode(original)
            .map_err(|e| E::caused("paea-settlement-agreement-original-encoding", e))?;
        if trust::hash(&original) != evidence.original_digest {
            return Err(E::refused("paea-settlement-agreement-original-altered"));
        }
        let original: trust::command::Command = swarm_wire_json::decode(&original)
            .map_err(|e| E::caused("paea-settlement-agreement-original-command", e))?;
        if original.action_id != evidence.object_id
            || original.mandate_id != text(action, "mandate_id")
            || original.content_digest != evidence.content_digest
            || original.resource != resource
            || original.amount_minor != amount
            || original.currency != currency
            || original.beneficiary != beneficiary
            || original.connector.is_some()
        {
            return Err(E::refused(
                "paea-settlement-agreement-original-incompatible",
            ));
        }
        let mandate_id = text(action, "mandate_id");
        if !trust::operations::active(state, mandate_id, OffsetDateTime::now_utc())
            .map_err(|e| E::caused("paea-settlement-agreement-mandate", e))?
            .is_empty()
        {
            continue;
        }
        let mandate = state
            .mandates
            .get(mandate_id)
            .ok_or_else(|| E::refused("paea-settlement-agreement-mandate-missing"))?;
        return Ok(Some(PaeaPaymentAgreement {
            action_id: evidence.object_id.clone(),
            mandate_id: mandate_id.into(),
            requester: requester.into(),
            approver: approver.into(),
            action_digest: request.canonical().digest().into(),
            evidence_hash: evidence.hash.clone(),
            original_digest: evidence.original_digest.clone(),
            policy_revision: state.revision,
            expires_at: text(mandate, "expires_at").into(),
        }));
    }
    Ok(None)
}

/// Recheck the observed mandate deadline immediately before effect commitment.
/// An expired/malformed deadline refuses; this neither revokes a published
/// effect nor guarantees real-time scheduling between this check and SQLite.
pub(super) fn timely(agreement: &Option<PaeaPaymentAgreement>) -> Result<()> {
    if let Some(agreement) = agreement {
        let expiry = swarm_wire_json::go_rfc3339::parse(&agreement.expires_at)
            .map_err(|e| E::caused("paea-settlement-agreement-expiry", e))?;
        if OffsetDateTime::now_utc() >= expiry {
            return Err(E::refused("paea-settlement-attributed-admission-required"));
        }
    }
    Ok(())
}

/// Validate a receipt's optional local agreement. Legacy receipts stay readable
/// but establish no separated approval. Reject malformed names/digests, wrong
/// action/revision or commitment requested at/after the mandate deadline.
/// This checks record consistency, never external authenticity or human presence.
pub(super) fn receipt(receipt: &super::PaeaSettlementReceipt) -> Result<()> {
    if let Some(agreement) = &receipt.agreement {
        let expiry = swarm_wire_json::go_rfc3339::parse(&agreement.expires_at)
            .map_err(|e| E::caused("paea-settlement-receipt-agreement", e))?;
        let requested = swarm_wire_json::go_rfc3339::parse(&receipt.commit_requested_at)
            .map_err(|e| E::caused("paea-settlement-receipt-agreement", e))?;
        if ![
            &agreement.action_id,
            &agreement.mandate_id,
            &agreement.requester,
            &agreement.approver,
        ]
        .iter()
        .all(|s| swarm_identity_contract::name(s.as_bytes()))
            || agreement.requester == agreement.approver
            || ![
                &agreement.action_digest,
                &agreement.evidence_hash,
                &agreement.original_digest,
            ]
            .iter()
            .all(|s| swarm_identity_contract::digest(s.as_bytes()))
            || agreement.action_digest != receipt.action_digest
            || agreement.policy_revision != receipt.trust_revision
            || requested >= expiry
        {
            return Err(E::refused("paea-settlement-receipt-agreement"));
        }
    }
    Ok(())
}
