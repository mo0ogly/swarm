//! Historical command preimage. Claimed actor/integrity are data, never authority.
use super::{
    json::CompactJson,
    proof::{Anchor, ProofExport},
    proof_types::ConnectorSelection,
    TrustError as E,
};
use crate::trust_types::{TrustIdentityInput, TrustPolicy};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
fn zero_policy() -> TrustPolicy {
    TrustPolicy {
        mandate_ttl_seconds: 0,
        unknown_reconcile_seconds: 0,
        evidence_retention_days: 0,
        require_separation: false,
    }
}
fn zero(v: &i64) -> bool {
    *v == 0
}
fn no(v: &bool) -> bool {
    !*v
}
/// Business input, including ignored historical claims. Order/omitempty matches
/// the Go event-digest producer. Only the historical all-zero policy placeholder
/// is accepted; it grants no configuration authority. Other configuration refuses.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Command {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<ConnectorSelection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_context: Option<CompactJson>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub parent_mandate_id: String,
    #[serde(default)]
    pub schema_version: i64,
    #[serde(default)]
    pub event_id: String,
    #[serde(default, skip_serializing_if = "zero")]
    pub expected_revision: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub actor: String,
    #[serde(default, skip_serializing_if = "zero")]
    pub integrity: i64,
    #[serde(default = "zero_policy")]
    pub policy: TrustPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identities: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mandate_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub action_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content_digest: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub subject: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resource: String,
    #[serde(default, skip_serializing_if = "zero")]
    pub max_amount_minor: i64,
    #[serde(default, skip_serializing_if = "zero")]
    pub amount_minor: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub currency: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub beneficiary: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub starts_at: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub expires_at: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub idempotency_key: String,
    #[serde(default, skip_serializing_if = "no")]
    pub lose_response: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export: Option<ProofExport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub verification_key: String,
}
/// Explicit operator configuration. Extensions are retained, but known active
/// unported policies must refuse; an unknown key never becomes configuration.
#[derive(Deserialize)]
pub(crate) struct ConfigChange {
    #[serde(default,deserialize_with="super::separation::present")]
    pub role_separation:Option<super::separation::RoleSeparation>,
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub policy: TrustPolicy,
    pub identities: Vec<TrustIdentityInput>,
    pub reason: String,
    #[serde(flatten, skip_deserializing)]
    pub extensions: BTreeMap<String, CompactJson>,
}
/// Validate original business bytes before any state change. Supplied actor/
/// integrity are removed only from the event fingerprint, never from originals.
pub(crate) fn decode(raw: &[u8]) -> Result<Command, E> {
    let cmd: Command = swarm_wire_json::decode(raw)?;
    if cmd.schema_version != 1 || !swarm_identity_contract::name(cmd.event_id.as_bytes()) {
        return Err(E::Invalid("schema_version=1 et event_id requis"));
    }
    if !cmd.identities.is_empty()
        || cmd.policy.mandate_ttl_seconds != 0
        || cmd.policy.unknown_reconcile_seconds != 0
        || cmd.policy.evidence_retention_days != 0
        || cmd.policy.require_separation
        || cmd.export.is_some()
        || cmd.anchor.is_some()
        || !cmd.verification_key.is_empty()
    {
        return Err(E::Invalid(
            "champs de configuration ou de vérification interdits dans une commande métier",
        ));
    }
    Ok(cmd)
}
/// Canonical command fingerprint excluding unauthenticated claims, matching the
/// historical operation/command envelope. This does not grant replay authority.
pub(crate) fn fingerprint(operation: &str, cmd: &mut Command) -> Result<String, E> {
    #[derive(Serialize)]
    struct Envelope<'a> {
        operation: &'a str,
        command: &'a Command,
    }
    let actor = std::mem::take(&mut cmd.actor);
    let integrity = std::mem::replace(&mut cmd.integrity, 0);
    let result = super::digest(&Envelope {
        operation,
        command: cmd,
    });
    cmd.actor = actor;
    cmd.integrity = integrity;
    result
}
/// Resource grammar is a historical identifier path, not a filesystem path.
pub(crate) fn resource(raw: &str) -> bool {
    !raw.is_empty()
        && raw.len() <= 512
        && raw
            .split('/')
            .all(|p| swarm_identity_contract::name(p.as_bytes()))
}

impl ConfigChange {
    /// Decode known fields and exact raw extension values without serde flatten's
    /// lossy intermediate buffer. Unknown extension admission remains in config.
    pub fn decode(raw: &[u8]) -> Result<Self, E> {
        let mut c: Self = swarm_wire_json::decode(raw)?;
        c.extensions = super::json::extensions(
            raw,
            &[
                "schema_version",
                "event_id",
                "expected_revision",
                "policy",
                "identities",
                "reason",
                "role_separation",
            ],
        )?;
        Ok(c)
    }
}
