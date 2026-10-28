//! Closed structural admission for retained, unported policy blocks.
use super::{
    json::{CompactJson, Fields},
    TrustError as E,
};
use serde_json::Value;
use std::collections::BTreeMap;

/// Validate known field spelling and supplied types without rewriting originals.
/// This establishes only an inactive compatibility shape, not URL/provider health
/// or complete validation for an unported consumer. Active policy refuses later.
/// Missing enabled/max_depth, null scalar fields, duplicates and unknown keys
/// refuse. Historical null federation lists mean no mapped/client authority;
/// editing an unported block still refuses at configuration admission.
pub(crate) fn validate(name: &str, raw: &CompactJson) -> Result<Value, E> {
    let mut defaults: BTreeMap<String, Value> =
        serde_json::from_str(include_str!("../../config/trust-controls-defaults.json"))?;
    // Legacy federation shapes are compatibility schemas, not runtime defaults.
    // The shared file owns the other thirteen blocks; moving these names there
    // requires removing these overrides in the same schema change.
    defaults.insert("oidc".into(),serde_json::json!({"enabled":false,"issuer":"","client_id":"","redirect_uri":"","request_timeout_seconds":0,"login_ttl_seconds":0,"session_ttl_seconds":0,"mappings":[]}));
    defaults.insert("workloads".into(),serde_json::json!({"enabled":false,"issuer":"","audience":"","request_timeout_seconds":0,"assertion_ttl_seconds":0,"session_ttl_seconds":0,"clients":[]}));
    let schema = defaults
        .get(name)
        .and_then(Value::as_object)
        .ok_or(E::Invalid("champ de configuration inconnu"))?;
    let Fields(fields) = serde_json::from_str(raw.raw())?;
    let flag = if name == "delegation" {
        "max_depth"
    } else {
        "enabled"
    };
    if !fields.contains_key(flag) {
        return Err(E::Invalid("champ de politique explicite requis"));
    }
    for (key, raw) in &fields {
        let expected = schema
            .get(key)
            .ok_or(E::Invalid("champ de politique inconnu"))?;
        let v: Value = serde_json::from_str(raw.get())?;
        if v.is_null()
            && matches!(
                (name, key.as_str()),
                ("oidc", "mappings") | ("workloads", "clients")
            )
        {
            continue;
        }
        let valid = match expected {
            Value::Bool(_) => v.is_boolean(),
            Value::String(_) => v.is_string(),
            Value::Number(n) => v
                .as_i64()
                .is_some_and(|x| x >= 0 && (n.as_i64() == Some(0) || x > 0)),
            Value::Array(_) => v.as_array().is_some_and(|a| {
                a.iter().all(|x| {
                    x.is_string()
                        || matches!(key.as_str(), "resources" | "rules" | "mappings" | "clients")
                            && x.is_object()
                })
            }),
            _ => false,
        };
        if !valid {
            return Err(E::Invalid("type ou valeur de politique invalide"));
        }
    }
    serde_json::from_str(raw.raw()).map_err(E::from)
}
