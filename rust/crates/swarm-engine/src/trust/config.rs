//! Operator configuration and exact historical fingerprint. Raw credentials are
//! hashed only in memory and never become a stored configuration original.
use super::{command::ConfigChange, encoded, json::CompactJson, state::State, TrustError as E};
use crate::trust_types::{prepare_identities, TrustIdentityInput};
use serde::{ser::SerializeMap, Serialize, Serializer};
use serde_json::json;
use time::OffsetDateTime;
/// Borrowed credential material for the historical digest only. Never exposed
/// by a public result or debug representation; disabled omission matches Go.
#[derive(Serialize)]
struct Material<'a> {
    subject: &'a str,
    token: &'a str,
    integrity: i64,
    roles: &'a Vec<String>,
    #[serde(skip_serializing_if = "is_false")]
    disabled: bool,
}
fn is_false(v: &bool) -> bool {
    !*v
}
/// Historical Go field order, including explicitly supplied policy extensions.
struct ConfigDigest<'a>(&'a ConfigChange);
impl Serialize for ConfigDigest<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let c = self.0;
        let mut map = s.serialize_map(None)?;
        for key in [
            "kernel_observation",
            "kernel_check",
            "laboratory",
            "evidence_signing",
            "process_policy",
            "workload_introspection",
            "secret_broker",
            "sealed_execution",
            "data_policy",
            "group_mapping",
            "oidc_logout",
            "step_up",
            "delegation",
            "workloads",
            "oidc",
        ] {
            if let Some(v) = c.extensions.get(key) {
                if v.raw() != "null" {
                    map.serialize_entry(key, v)?;
                }
            }
        }
        map.serialize_entry("schema_version", &c.schema_version)?;
        map.serialize_entry("event_id", &c.event_id)?;
        map.serialize_entry("expected_revision", &c.expected_revision)?;
        map.serialize_entry("policy", &c.policy)?;
        let identities: Vec<_> = c
            .identities
            .iter()
            .map(|i| Material {
                subject: &i.subject,
                token: &i.token,
                integrity: i.integrity,
                roles: &i.roles,
                disabled: i.disabled,
            })
            .collect();
        map.serialize_entry("identities", &identities)?;
        map.serialize_entry("reason", &c.reason)?;
        map.end()
    }
}
/// Validate operator proposal and construct a preview or durable next state.
/// The caller owns Root/session authority and serialization; preview never
/// publishes material. Replays return retained snapshots, not current state.
pub(crate) fn change(
    st: &mut State,
    raw: &[u8],
    preview: bool,
    now: OffsetDateTime,
    operator: &str,
) -> Result<Vec<u8>, E> {
    let c = ConfigChange::decode(raw)?;
    if c.schema_version != 1
        || !swarm_identity_contract::name(c.event_id.as_bytes())
        || c.expected_revision < 0
        || c.reason.trim().len() < 8
        || c.reason.len() > 2000
    {
        return Err(E::Invalid("configuration Actions de confiance invalide"));
    }
    c.policy.validate().map_err(E::Invalid)?;
    let fingerprint = super::digest(&ConfigDigest(&c))?;
    if let Some(prior) = st.event_digests.get(&c.event_id) {
        if prior != &fingerprint {
            return Err(E::Conflict("event_conflict"));
        }
        return st
            .event_replies
            .get(&c.event_id)
            .map(|v| v.raw().as_bytes().to_vec())
            .ok_or(E::Integrity("trust-reply-missing"));
    }
    if c.expected_revision != st.revision {
        return Err(E::Conflict("revision_conflict"));
    }
    let inputs: Vec<TrustIdentityInput> = c.identities;
    let identities = prepare_identities(inputs).map_err(E::Invalid)?;
    let mut proposed = st.clone();
    proposed.policy = c.policy;
    proposed.identities = identities;
    let controls = proposed.controls()?;
    for (name, value) in c.extensions {
        if !controls.contains_key(&name) && !matches!(name.as_str(), "oidc" | "workloads") {
            return Err(E::Invalid("champ de configuration inconnu"));
        }
        if value.raw() != "null" {
            proposed.extensions.insert(name, value);
        }
    }
    proposed.require_supported()?;
    let next = st
        .revision
        .checked_add(1)
        .ok_or(E::Integrity("trust-revision-representation"))?;
    if preview {
        let public = proposed.public()?;
        return encoded(
            &json!({"schema_version":1,"current_revision":st.revision,"proposed_revision":next,"policy":proposed.policy,"identities":public["identities"],"identity_count":proposed.identities.len(),"applied":false}),
        );
    }
    proposed.revision = next;
    let at = now
        .to_offset(time::UtcOffset::UTC)
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| E::Invalid("horloge non représentable"))?;
    proposed.history.push(json!({"revision":next,"policy":proposed.policy,"actor":operator,"reason":c.reason,"event_id":c.event_id,"at":at}));
    for field in ["oidc_pending", "oidc_sessions", "workload_sessions"] {
        proposed.extensions.remove(field);
    }
    let bytes = encoded(&proposed.public()?)?;
    let reply = CompactJson::new(
        std::str::from_utf8(&bytes).map_err(|_| E::Integrity("trust-reply-encoding"))?,
    )?;
    proposed
        .event_digests
        .insert(c.event_id.clone(), fingerprint);
    proposed.event_replies.insert(c.event_id, reply);
    *st = proposed;
    Ok(bytes)
}
