//! Wire DTOs for trust ledger evidence, effect proofs, and identity context.
//!
//! These structs mirror the canonical Go source exactly (field order is load-bearing
//! for canonical hashes). All data carried here is untrusted wire data: presence of a
//! value proves nothing about its provenance or authority — verification happens
//! elsewhere. This module performs no hashing, IO, crypto, or policy evaluation.
//!
//! The supplied Go field list is authoritative; no fields are added or reordered.

use serde::{Deserialize, Serialize};

use super::json::CompactJson;
use crate::trust_types::TrustPolicy;

/// Omitempty helper for Go's `omitempty` on bools (zero value = false).
fn is_false(b: &bool) -> bool {
    !*b
}

/// Ledger evidence entry. Untrusted data; authenticity is not implied by presence.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Evidence {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_context: Option<ProofPolicyContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_receipt: Option<EffectProof>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_context: Option<IdentityContext>,
    pub sequence: i64,
    pub previous_hash: String,
    pub hash: String,
    pub kind: String,
    pub actor: String,
    pub integrity: i64,
    pub object_id: String,
    pub content_digest: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub original_digest: String,
    pub result: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    pub at: String,
}

/// Policy context embedded in proofs. Untrusted; not an authorization decision.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProofPolicyContext {
    pub schema_version: i64,
    pub revision: i64,
    pub policy: TrustPolicy,
    /// Raw canonical JSON; a missing field is refused rather than invented.
    pub controls: CompactJson,
}

/// Effect proof receipt. Untrusted wire data; no validity is asserted here.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EffectProof {
    pub schema_version: i64,
    pub connector_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<EffectIntent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<EffectResult>,
    /// Raw canonical JSON; a missing field is refused rather than invented.
    pub native_result: CompactJson,
    #[serde(default, skip_serializing_if = "is_false")]
    pub legacy_receipt: bool,
}

/// Intent record for a connector effect. Untrusted data only.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EffectIntent {
    pub schema_version: i64,
    pub intent_id: String,
    pub action_digest: String,
    pub mandate_digest: String,
    pub approval_digests: Vec<String>,
    pub connector_id: String,
    pub connector_version: i64,
    pub resource: String,
    /// Raw canonical JSON; a missing field is refused rather than invented.
    pub canonical_parameters: CompactJson,
    pub idempotency_key: String,
    pub deadline: String,
}

/// Result record for a connector effect. Untrusted wire data.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EffectResult {
    pub schema_version: i64,
    pub intent_id: String,
    pub connector_receipt: String,
    pub state: String,
    pub committed_at: String,
    pub observed_at: String,
    pub result_digest: String,
    pub reconciliation_required: bool,
    pub diagnostic_code: String,
}

/// Identity context. Untrusted claims; provenance is not authentication.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IdentityContext {
    pub schema_version: i64,
    pub configuration_revision: i64,
    pub principal_id: String,
    pub principal_kind: String,
    /// Go pointer: serializes as explicit `null` when absent (not omitempty).
    pub issuer: Option<String>,
    pub subject: Option<String>,
    pub assurance: IdentityAssurance,
    pub attributes: IdentityAttributes,
    pub delegation_id: Option<String>,
    pub authenticated_at: Option<String>,
    pub expires_at: Option<String>,
    pub token_digest: String,
}

/// Assurance claims. Untrusted; not verified here.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IdentityAssurance {
    pub method: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub acr: String,
    pub provider_authenticated_at: Option<String>,
}

/// Identity attribute claims. Untrusted; not verified here.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IdentityAttributes {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role_mapping_digest: String,
    pub roles: Vec<String>,
    pub integrity: i64,
}

/// Ledger entry for committed actions. Untrusted wire data.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LedgerEntry {
    pub sequence: i64,
    pub action_id: String,
    pub content_digest: String,
    pub idempotency_key: String,
    pub amount_minor: i64,
    pub currency: String,
    pub beneficiary: String,
    pub resource: String,
    pub committed_at: String,
    pub simulation: bool,
}

/// Connector selection. Untrusted; selection is not authorization.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConnectorSelection {
    pub id: String,
    pub version: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<AccessParameters>,
}

/// Access parameters. Untrusted claims; no authority is implied.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AccessParameters {
    pub subject: String,
    pub role: String,
}

/// Access ledger entry. Untrusted wire data.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AccessEntry {
    pub sequence: i64,
    pub action_id: String,
    pub content_digest: String,
    pub idempotency_key: String,
    pub resource: String,
    pub access: AccessParameters,
    pub committed_at: String,
    pub simulation: bool,
}

/// Stable native receipt; raw parameters keep their canonical property order.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Receipt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<EffectIntent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<EffectResult>,
    pub content_digest: String,
    pub action_id: String,
    pub state: String,
}
