//! Native frozen policy decoding must preserve explicit absence and constraints.
use swarm_engine::{decode_agent, encode_model, EngineSettingsValues};
/// A valid native snapshot is the control before missing/unknown metadata cases.
/// Neither legacy absence nor projection defaults may become an unlimited policy.
#[test]
fn native_guard_snapshot_requires_explicit_window() {
    let legacy = decode_agent(b"{}").unwrap();
    assert!(legacy.native_tool_guard_policy.is_none());
    let valid=br#"{"native_tool_guard_policy":{"schema_version":1,"settings_revision":"1","lookback_multiplier":"4"}}"#;
    assert!(decode_agent(valid)
        .unwrap()
        .native_tool_guard_policy
        .unwrap()
        .policy()
        .is_ok());
    let missing = br#"{"native_tool_guard_policy":{"schema_version":1,"settings_revision":"1"}}"#;
    match decode_agent(missing) {
        Err(_) => {}
        Ok(agent) => assert!(
            agent.native_tool_guard_policy.unwrap().policy().is_err(),
            "missing factor became an unlimited policy"
        ),
    }
}
/// Unknown native constraints survive observation/roundtrip but cannot be consumed
/// as a known policy. A valid known snapshot precedes this separate injection.
#[test]
fn native_guard_snapshot_preserves_unknown_constraints_without_grant() {
    let valid=br#"{"native_tool_guard_policy":{"schema_version":1,"settings_revision":"1","lookback_multiplier":"4"}}"#;
    assert!(decode_agent(valid)
        .unwrap()
        .native_tool_guard_policy
        .unwrap()
        .policy()
        .is_ok());
    let future=br#"{"native_tool_guard_policy":{"schema_version":1,"settings_revision":"1","lookback_multiplier":"4","future_constraint":{"must_validate":true}}}"#;
    let agent = decode_agent(future).unwrap();
    assert!(
        agent
            .native_tool_guard_policy
            .as_ref()
            .unwrap()
            .policy()
            .is_err(),
        "unknown constraint silently ignored"
    );
    let value: serde_json::Value = serde_json::from_slice(&encode_model(&agent).unwrap()).unwrap();
    assert_eq!(
        value["native_tool_guard_policy"]["future_constraint"]["must_validate"],
        true
    );
}
/// Saved multiplier is explicit, nullable or positive; there is no fallback for
/// old configuration and zero cannot silently disable interleaved detection.
#[test]
fn native_guard_settings_validate_positive_nullable_and_absent() {
    let values = EngineSettingsValues::installation_defaults().unwrap();
    assert!(values.tool_guard_policy().is_ok());
    let mut raw = serde_json::to_value(values).unwrap();
    raw["agent_interleaved_failure_lookback_multiplier"] = serde_json::Value::Null;
    assert!(
        EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap())
            .unwrap()
            .tool_guard_policy()
            .is_ok()
    );
    raw["agent_interleaved_failure_lookback_multiplier"] = "0".into();
    assert!(EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_err());
    for field in swarm_engine::EngineSettingsCatalog::installation()
        .unwrap()
        .fields
    {
        if field.introduced_schema_version > 16 {
            raw.as_object_mut().unwrap().remove(&field.name);
        }
    }
    let old = EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(old.schema_version(), 16);
    assert!(old.tool_guard_policy().is_err());
}

/// Opaque extension values can exceed serde Value's recursive tree depth;
/// observation retains them safely, while an unconsumed constraint refuses use.
#[test]
fn native_guard_snapshot_deep_extensions_preserve_without_recursive_value_drop() {
    let nested = format!("{}null{}", "[".repeat(512), "]".repeat(512));
    let raw = format!(
        r#"{{"native_tool_guard_policy":{{"schema_version":1,"settings_revision":"1","lookback_multiplier":null,"future":{nested}}}}}"#
    );
    let agent = decode_agent(raw.as_bytes()).unwrap();
    let snapshot = agent.native_tool_guard_policy.as_ref().unwrap();
    assert!(snapshot.policy().is_err());
    assert!(encode_model(&agent).is_ok());
}
