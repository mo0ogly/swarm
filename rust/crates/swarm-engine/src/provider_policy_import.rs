//! Portable saved model policies; no implicit model, account or execution grant.
use crate::{
    model_wire::{Fields, Kind},
    ModelPolicy, ProviderConfigurationError as E,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
/// Go PolicyChange plus a native public preview binding. Scalar zero/null/case/
/// merge behavior follows the inherited decoder. Private fields keep consumers
/// on validation; a policy is operator data, never model availability or identity.
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct ProviderPolicyChange {
    pub(crate) version: i64,
    pub(crate) expected_digest: String,
    pub(crate) policies: Option<BTreeMap<String, Option<Box<ModelPolicy>>>>,
    pub(crate) preview: bool,
    pub(crate) preview_token: String,
}
/// Inherited fields and explicit native token extension; no commands, env, keys
/// or endpoints can enter a policy-only import. ModelPolicy shares its Go shape.
fn shape() -> Fields {
    &[
        ("version", Kind::Int64),
        ("expected_digest", Kind::String),
        (
            "policies",
            Kind::Map(&Kind::Pointer(&Kind::Object(
                crate::agent_models::shape_model_policy,
            ))),
        ),
        ("preview", Kind::Bool),
        ("preview_token", Kind::String),
    ]
}
/// Decode one bounded-by-caller policy-only request using actual Go scalar/null/
/// case/duplicate merge semantics. Require version1, a source digest and nonempty
/// nonnull policies; unknown/type/multiple JSON and invalid names refuse safely.
/// No IO, fallback defaults, model/account verification or execution occurs.
pub fn decode_provider_policy_change(raw: &[u8]) -> Result<ProviderPolicyChange, E> {
    let r: ProviderPolicyChange =
        crate::work::decode_mode(raw, shape(), true).map_err(E::Format)?;
    if r.version != 1
        || !crate::provider_configuration::configuration_digest_valid(&r.expected_digest)
        || r.policies.as_ref().is_none_or(|p| {
            p.is_empty()
                || p.iter()
                    .any(|(name, p)| !swarm_identity_contract::name(name.as_bytes()) || p.is_none())
        })
    {
        return Err(E::Refused("provider-policy-request-invalid"));
    }
    Ok(r)
}
/// Exactly the portable Go export format: saved policies only, no secrets,
/// commands, fallback model names or assertion of availability. The strict Root
/// exporter refuses unresolved known-provider defaults. An Admin inventory can
/// carry this portable shape with only actually saved policies, including an
/// explicitly empty snapshot; that never approves an empty import.
#[derive(Serialize)]
pub struct ProviderPolicyExport {
    pub version: i64,
    pub policies: BTreeMap<String, ModelPolicy>,
}
/// Policy-only mutation receipt. Public token binds this Root/source/settings;
/// it is neither identity nor approval. No command/env/API key is returned and
/// all model/catalogue information remains unverified local configuration data.
#[derive(Serialize)]
pub struct ProviderPolicyMutation {
    pub version: i64,
    pub preview: bool,
    pub applied: bool,
    pub changed: bool,
    pub before_digest: String,
    pub after_digest: String,
    pub preview_token: String,
    pub backup_reference: Option<String>,
    pub policies: BTreeMap<String, ModelPolicy>,
    pub provider_execution_started: bool,
    pub scope: &'static str,
}
/// Source-bound saved policies for an authenticated read surface. The digest
/// and policies come from one checked provider-file read; no command, environment,
/// key, availability or execution authority is exposed. CLI portable export keeps
/// its original version/policies-only format.
#[derive(Serialize)]
pub struct ProviderPolicyView {
    pub version: i64,
    pub digest: String,
    pub policies: BTreeMap<String, ModelPolicy>,
}
impl ProviderPolicyChange {
    /// Return the flag after strict inherited null/case/duplicate decoding.
    /// This chooses preview versus apply, not authentication or write authority;
    /// the Root publisher still requires current digest/token and writable access.
    pub fn preview_requested(&self) -> bool {
        self.preview
    }

    /// Clone only validated nonnull policies. This never supplies defaults or
    /// treats a missing value as a cleared/approved provider declaration.
    pub(crate) fn entries(&self) -> Result<BTreeMap<String, ModelPolicy>, E> {
        self.policies
            .as_ref()
            .ok_or(E::Refused("provider-policy-request-invalid"))?
            .iter()
            .map(|(name, p)| {
                p.as_ref()
                    .map(|p| (name.clone(), (**p).clone()))
                    .ok_or(E::Refused("provider-policy-request-invalid"))
            })
            .collect()
    }
}
impl ProviderPolicyMutation {
    /// Preserve actual file-mutation facts without leaking unrelated provider
    /// command/env data in the policy response. Does not authenticate the author.
    pub(crate) fn observed(
        m: crate::ProviderConfigurationMutation,
        policies: BTreeMap<String, ModelPolicy>,
    ) -> Self {
        Self {
            version: 1,
            preview: m.preview,
            applied: m.applied,
            changed: m.changed,
            before_digest: m.before_digest,
            after_digest: m.after_digest,
            preview_token: m.preview_token,
            backup_reference: m.backup_reference,
            policies,
            provider_execution_started: false,
            scope: "saved_policy_metadata_only_no_admission",
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Nonempty valid Go policy data is a control, never an availability witness.
    fn request() -> serde_json::Value {
        serde_json::json!({"version":1,"expected_digest":"a".repeat(64),"policies":{"fixture":{"version":1,"billing":"external","work_level":"standard","page_level":"simple","levels":{"simple":{"model":"same","effort":""},"standard":{"model":"same","effort":""},"exigeant":{"model":"same","effort":""}}}},"preview":true})
    }
    /// Preserve Go null/case/merge behavior without accepting unrelated secrets.
    #[test]
    fn provider_policy_request_go_projection_and_native_token() {
        let mut r = request();
        r["preview_token"] = serde_json::Value::Null;
        assert!(
            decode_provider_policy_change(&serde_json::to_vec(&r).unwrap())
                .unwrap()
                .preview_token
                .is_empty()
        );
        let raw = serde_json::to_string(&request())
            .unwrap()
            .replace("\"version\":1", "\"VERSION\":1");
        let parsed = decode_provider_policy_change(raw.as_bytes()).unwrap();
        assert_eq!(parsed.entries().unwrap().len(), 1);
        assert!(parsed.preview);
        let raw = serde_json::to_string(&request()).unwrap().replacen(
            "\"preview\":true",
            "\"preview\":false,\"PREVIEW\":true",
            1,
        );
        assert!(
            decode_provider_policy_change(raw.as_bytes())
                .unwrap()
                .preview
        );
    }
    /// Empty/null/unknown/wrong-type input never becomes a successful empty import.
    #[test]
    fn provider_policy_request_refusals_keep_explicit_nonempty_contract() {
        assert!(decode_provider_policy_change(&serde_json::to_vec(&request()).unwrap()).is_ok());
        for patch in [
            serde_json::json!({"policies":null}),
            serde_json::json!({"policies":{}}),
            serde_json::json!({"policies":{"fixture":null}}),
            serde_json::json!({"version":2}),
            serde_json::json!({"expected_digest":"bad"}),
            serde_json::json!({"key":"PRIVATE"}),
            serde_json::json!({"preview":"true"}),
        ] {
            let mut r = request();
            r.as_object_mut()
                .unwrap()
                .extend(patch.as_object().unwrap().clone());
            assert!(decode_provider_policy_change(&serde_json::to_vec(&r).unwrap()).is_err());
        }
        let mut raw = serde_json::to_vec(&request()).unwrap();
        raw.extend_from_slice(b" {}");
        assert!(decode_provider_policy_change(&raw).is_err());
    }
}
