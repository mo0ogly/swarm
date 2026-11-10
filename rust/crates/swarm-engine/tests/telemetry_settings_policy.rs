//! Saved v30 telemetry providers (LIVE-UX-03): opt-in list, closed provider set,
//! dependency on v29; no launch, token or network effect is exercised here.
use swarm_engine::EngineSettingsValues;

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
fn telemetry_providers_default_off_accept_claude_and_refuse_others() {
    let defaults = EngineSettingsValues::installation_defaults().unwrap();
    assert_eq!(defaults.schema_version(), current_version());
    // This test is about v30: start from a v30 document (later fields removed).
    let mut values = truncated(&defaults, 30);
    assert_eq!(values.schema_version(), 30);
    assert!(values.telemetry_providers().unwrap().is_empty(), "installed default disables telemetry");
    values.agent_telemetry_providers = Some(vec!["claude".into()]);
    values.validate().unwrap();
    assert_eq!(values.telemetry_providers().unwrap(), ["claude".to_owned()]);
    values.agent_telemetry_providers = Some(vec!["claude".into(), "codex".into()]);
    values.validate().unwrap();
    assert_eq!(values.telemetry_providers().unwrap(), ["claude".to_owned(), "codex".to_owned()], "Codex supported since 2026-10-22");
    let encoded = swarm_engine::encode_model(&values).unwrap();
    assert_eq!(EngineSettingsValues::decode(&encoded).unwrap(), values);
    for (list, code) in [
        (vec!["aider"], "settings-telemetry-provider-unsupported"),
        (vec!["Claude"], "settings-telemetry-provider-unsupported"),
        (vec!["claude", "claude"], "settings-telemetry-provider-duplicate"),
    ] {
        values.agent_telemetry_providers = Some(list.into_iter().map(str::to_owned).collect());
        assert_eq!(values.validate().unwrap_err().reason(), code);
    }
    // Historical absence: v29 document, telemetry never opted in.
    values.agent_telemetry_providers = None;
    values.validate().unwrap();
    assert_eq!(values.schema_version(), 29);
    assert!(values.telemetry_providers().unwrap().is_empty());
    // v30 without v29 is a partial history and refuses.
    values.agent_telemetry_providers = Some(vec![]);
    values.review_response_max_bytes = None;
    assert_eq!(values.validate().unwrap_err().reason(), "settings-telemetry-requires-review-response-policy");
}
