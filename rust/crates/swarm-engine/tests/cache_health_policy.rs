//! Saved v31 cache-health thresholds (LIVE-UX L3): complete group, bounded
//! percent, dependency on v30; display policy only, no consumer gate here.
use swarm_engine::{EngineSettingsValues, SettingCount};

/// Current settings schema = newest `introduced_schema_version` in the catalog,
/// so this pin follows later settings instead of breaking on each addition.
fn catalog() -> serde_json::Value {
    serde_json::from_str(include_str!("../config/engine-settings-catalog.json")).unwrap()
}
fn current_version() -> i64 {
    catalog()["fields"].as_array().unwrap().iter().filter_map(|f| f["introduced_schema_version"].as_i64()).max().unwrap()
}
/// The same document with every field introduced after `version` removed.
fn truncated(values: &EngineSettingsValues, version: i64) -> EngineSettingsValues {
    let mut doc: serde_json::Value = serde_json::from_slice(&swarm_engine::encode_model(values).unwrap()).unwrap();
    for f in catalog()["fields"].as_array().unwrap() {
        if f["introduced_schema_version"].as_i64().unwrap_or(0) > version {
            doc.as_object_mut().unwrap().remove(f["name"].as_str().unwrap());
        }
    }
    EngineSettingsValues::decode(&serde_json::to_vec(&doc).unwrap()).unwrap()
}

#[test]
fn cache_health_thresholds_default_validate_and_refuse_partial_or_out_of_range() {
    let defaults = EngineSettingsValues::installation_defaults().unwrap();
    assert_eq!(defaults.schema_version(), current_version());
    // This test is about v31: later fields removed.
    let mut values = truncated(&defaults, 31);
    assert_eq!(values.schema_version(), 31);
    assert_eq!(values.cache_health_policy().unwrap(), Some((50, 1000)), "LangWatch defaults");
    let encoded = swarm_engine::encode_model(&values).unwrap();
    assert_eq!(EngineSettingsValues::decode(&encoded).unwrap(), values);
    values.agent_cache_miss_ratio_percent = Some(SettingCount(101));
    assert_eq!(values.validate().unwrap_err().reason(), "settings-cache-health-ratio-invalid");
    values.agent_cache_miss_ratio_percent = Some(SettingCount(0));
    values.agent_cache_min_input_tokens = Some(SettingCount(0));
    values.validate().unwrap();
    values.agent_cache_min_input_tokens = None;
    assert_eq!(values.validate().unwrap_err().reason(), "settings-cache-health-incomplete");
    // Historical absence: a v30 document, console shows "not configured".
    values.agent_cache_miss_ratio_percent = None;
    values.validate().unwrap();
    assert_eq!(values.schema_version(), 30);
    assert_eq!(values.cache_health_policy().unwrap(), None);
    // v31 without v30 is a partial history.
    values.agent_cache_miss_ratio_percent = Some(SettingCount(50));
    values.agent_cache_min_input_tokens = Some(SettingCount(1000));
    values.agent_telemetry_providers = None;
    assert_eq!(values.validate().unwrap_err().reason(), "settings-cache-health-requires-telemetry-policy");
}
