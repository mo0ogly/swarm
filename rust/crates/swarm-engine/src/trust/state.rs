//! Historical trust file schema; extensions retained verbatim and never enabled
//! by a missing Rust consumer. Typed core maps remain the business authority.
use super::{json::CompactJson, proof_types::*, TrustError};
use crate::trust_types::{TrustIdentity, TrustPolicy};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
/// Go nil arrays decode as empty, with public producers deliberately emitting [].
fn null_vec<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Vec<T>, D::Error> {
    Ok(Option::<Vec<T>>::deserialize(d)?.unwrap_or_default())
}
/// Persisted version1. This is not a credential dump or an authorization token.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct State {
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub public_evidence_registry:Option<super::public_evidence::Registry>,
    #[serde(default,skip_serializing_if="Option::is_none",deserialize_with="super::separation::present")]
    pub role_separation:Option<super::separation::RoleSeparation>,
    pub schema_version: i64,
    pub revision: i64,
    pub policy: TrustPolicy,
    #[serde(default, deserialize_with = "null_vec")]
    pub identities: Vec<TrustIdentity>,
    #[serde(default, deserialize_with = "null_vec")]
    pub history: Vec<Value>,
    pub mandates: BTreeMap<String, Value>,
    pub actions: BTreeMap<String, Value>,
    pub connector_receipts: BTreeMap<String, Receipt>,
    #[serde(default, deserialize_with = "null_vec")]
    pub ledger: Vec<LedgerEntry>,
    #[serde(default, deserialize_with = "null_vec")]
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub originals: BTreeMap<String, String>,
    pub event_replies: BTreeMap<String, CompactJson>,
    pub event_digests: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub event_authorities: BTreeMap<String, String>,
    #[serde(
        default,
        deserialize_with = "null_vec",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub access_ledger: Vec<AccessEntry>,
    #[serde(flatten, skip_deserializing)]
    pub extensions: BTreeMap<String, CompactJson>,
}
impl State {
    /// Decode core fields directly from raw JSON and retain extension bytes.
    /// serde flatten buffers cannot deserialize RawValue; no floating-point or
    /// property-order round trip is allowed for retained extension originals.
    pub fn decode(raw: &[u8]) -> Result<Self, TrustError> {
        let mut state: Self = swarm_wire_json::decode(raw)?;
        state.extensions = super::json::extensions(
            raw,
            &[
                "schema_version",
                "role_separation",
                "public_evidence_registry",
                "revision",
                "policy",
                "identities",
                "history",
                "mandates",
                "actions",
                "connector_receipts",
                "ledger",
                "evidence",
                "originals",
                "event_replies",
                "event_digests",
                "event_authorities",
                "access_ledger",
            ],
        )?;
        Ok(state)
    }
    /// Empty installation projection only; configuring requires explicit policy
    /// and private identities. This performs no IO and grants no operation.
    pub fn initial() -> Self {
        Self {
            role_separation:None,
            public_evidence_registry:None,
            schema_version: 1,
            revision: 0,
            policy: TrustPolicy::defaults(),
            identities: vec![],
            history: vec![],
            mandates: BTreeMap::new(),
            actions: BTreeMap::new(),
            connector_receipts: BTreeMap::new(),
            ledger: vec![],
            evidence: vec![],
            originals: BTreeMap::new(),
            event_replies: BTreeMap::new(),
            event_digests: BTreeMap::new(),
            event_authorities: BTreeMap::new(),
            access_ledger: vec![],
            extensions: BTreeMap::new(),
        }
    }
    /// Validate core storage consistency before using it. A bad history/schema or
    /// policy is never replaced by installation defaults or an empty success.
    pub fn validate(&self) -> Result<(), TrustError> {
        if self.schema_version != 1
            || self.revision < 0
            || self.revision as usize != self.history.len()
        {
            return Err(TrustError::Integrity("trust-state-revision"));
        }
        self.policy.validate().map_err(TrustError::Invalid)?;
        // A mapping never carries an alternative bearer credential. Duplicate
        // issuer/subject identities would bypass canonical-person separation.
        for (index, id) in self.identities.iter().enumerate() {
            if let Some(p) = &id.enrollment_principal {
                if !p.valid() || p.kind != "human" || !id.token_hash.is_empty()
                    || self.identities[..index].iter().filter_map(|i| i.enrollment_principal.as_ref())
                        .any(|q| q.id == p.id || q.issuer == p.issuer && q.subject == p.subject) {
                    return Err(TrustError::Integrity("trust-enrollment-mapping"));
                }
            }
        }
        if let Some(policy)=&self.role_separation{policy.validate(&self.identities)?;}
        let decisions:Vec<_>=self.evidence.iter().filter(|e|matches!(e.kind.as_str(),"evidence-key-prepare"|"evidence-key-activate"|"evidence-key-revoke")).collect();
        if let Some(registry)=&self.public_evidence_registry{
            registry.validate()?;
            let digest=super::digest(registry)?;
            if u64::try_from(decisions.len()).ok()!=Some(registry.revision)||decisions.last().is_some_and(|e|digest!=e.content_digest){return Err(TrustError::Integrity("trust-public-registry-proof"));}
        }else if !decisions.is_empty(){return Err(TrustError::Integrity("trust-public-registry-missing"));}
        for (id, m) in &self.mandates {
            if m.get("id").and_then(Value::as_str) != Some(id)
                || m.get("integrity").and_then(Value::as_i64) != Some(2)
            {
                return Err(TrustError::Integrity("trust-mandate-identity"));
            }
        }
        for (id, a) in &self.actions {
            if a.get("id").and_then(Value::as_str) != Some(id) {
                return Err(TrustError::Integrity("trust-action-identity"));
            }
        }
        Ok(())
    }
    /// Effective historical controls. Missing extensions mean only the explicit
    /// inactive compatibility projection; registered extensions remain verbatim.
    pub fn controls(&self) -> Result<BTreeMap<String, CompactJson>, TrustError> {
        let mut controls: BTreeMap<String, CompactJson> =
            serde_json::from_str(include_str!("../../config/trust-controls-defaults.json"))?;
        for (name, value) in &mut controls {
            if let Some(stored) = self.extensions.get(name) {
                if stored.raw() != "null" {
                    *value = stored.clone();
                }
            }
        }
        Ok(controls)
    }
    /// Refuse mutation while an unported authority/admission policy is active.
    /// This closes static-identity fallbacks and never disables saved policies.
    /// Retained null federation blocks mean absent, with no mapped authority.
    pub fn require_supported(&self) -> Result<(), TrustError> {
        for (name, raw) in self.controls()? {
            if name=="evidence_signing"{
                let policy:super::public_evidence::SigningPolicy=swarm_wire_json::decode(raw.raw().as_bytes())?;
                policy.validate()?;continue;
            }
            let value = super::extensions::validate(&name, &raw)?;
            if !value.is_object() || value.get("enabled").is_some_and(|v| !v.is_boolean()) {
                return Err(TrustError::Integrity("trust-policy-shape"));
            }
            if name == "delegation" && value.get("max_depth").and_then(Value::as_i64).is_none() {
                return Err(TrustError::Integrity("trust-delegation-shape"));
            }
            if value.get("enabled").and_then(Value::as_bool) == Some(true)
                || name == "delegation" && value.get("max_depth").and_then(Value::as_i64) != Some(0)
            {
                return Err(TrustError::Unsupported("trust-active-extension-not-ported"));
            }
        }
        for name in ["oidc", "workloads"] {
            if let Some(raw) = self.extensions.get(name) {
                if raw.raw() == "null" {
                    continue;
                }
                let v = super::extensions::validate(name, raw)?;
                if !v.is_object() || v.get("enabled").is_some_and(|v| !v.is_boolean()) {
                    return Err(TrustError::Integrity("trust-federated-policy-shape"));
                }
                if v.get("mappings")
                    .is_some_and(|m| !m.is_null() && !m.as_array().is_some_and(|a| a.is_empty()))
                {
                    return Err(TrustError::Unsupported("trust-mapped-identity-not-ported"));
                }
                if v.get("enabled").and_then(Value::as_bool) == Some(true)
                    || v.get("clients")
                        .and_then(Value::as_array)
                        .is_some_and(|v| !v.is_empty())
                {
                    return Err(TrustError::Unsupported(
                        "trust-federated-authority-not-ported",
                    ));
                }
            }
        }
        Ok(())
    }
    /// Operator audit projection: no raw token, hash, sessions or original bytes.
    /// Read access is local operator / authenticated native web control, not an
    /// agent-selected integrity level or direct read of an official object.
    pub fn public(&self) -> Result<Value, TrustError> {
        let identities:Vec<Value>=self.identities.iter().map(|id| {
            let mut value=serde_json::json!({"subject":id.subject,"integrity":id.integrity,"roles":id.roles,"disabled":id.disabled});
            if let Some(p)=&id.enrollment_principal {value["enrollment_principal"]=serde_json::json!({"principal_id":p.id,"principal_kind":p.kind,"issuer":p.issuer,"subject":p.subject});}
            value
        }).collect();
        let mut view = serde_json::json!({"schema_version":1,"revision":self.revision,"policy":self.policy,"identities":identities,"mandates":self.mandates,"actions":self.actions,"ledger":self.ledger,"access_ledger":self.access_ledger,"effect_receipts":self.connector_receipts,"evidence_count":self.evidence.len(),"simulation":true,"originals_missing":self.evidence.iter().filter(|e|self.originals.get(&e.original_digest).is_none_or(|v|v.is_empty())).count(),"limits":["aucun paiement réel","identités locales authentifiées","politiques avancées actives non portées : refus fermé"],"biba":{"model":"strict-with-named-endorsement-v1","scope":"trust-actions-only","os_confinement":false},"amount_minor_text":self.actions.iter().map(|(id,a)|(id.clone(),Value::String(a.get("amount_minor").and_then(Value::as_i64).unwrap_or(0).to_string()))).collect::<BTreeMap<_,_>>()});
        view["identity_authentication"]=serde_json::json!({"local_bearer":true,"held_enrollment":{"root_api_available":cfg!(target_os="linux"),"http_available":false,"reason":"trusted-enrollment-startup-not-bound"}});
        for (name, raw) in self.controls()? {
            view[&name] = serde_json::from_str(raw.raw())?;
        }
        view["policy_bounds"] = serde_json::json!({"mandate_ttl_seconds":{"min":0,"max":9223372036_i64},"unknown_reconcile_seconds":{"min":0,"max":9223372036_i64},"evidence_retention_days":{"min":1,"max":9007199254740991_i64},"state_max_bytes":8388608,"command_max_bytes":crate::COMMAND_INPUT_MAX_BYTES,"proof_key_bytes":32});
        if let Some(policy)=&self.role_separation{view["role_separation"]=serde_json::to_value(policy)?;}
        if let Some(registry)=&self.public_evidence_registry{view["public_evidence_registry"]=serde_json::to_value(registry)?;}
        Ok(view)
    }
}
