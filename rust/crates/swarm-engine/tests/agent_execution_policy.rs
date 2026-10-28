//! Saved supervision policies are data, never provider-execution authority.
use swarm_engine::EngineSettingsValues;
/// Derive exactly historical v18 inputs by excluding later catalogue members;
/// current installation policy is never silently relabelled as an old version.
fn v18_values() -> serde_json::Value {
    let mut value =
        serde_json::to_value(EngineSettingsValues::installation_defaults().unwrap()).unwrap();
    for field in swarm_engine::EngineSettingsCatalog::installation()
        .unwrap()
        .fields
    {
        if field.introduced_schema_version > 18 {
            value.as_object_mut().unwrap().remove(&field.name);
        }
    }
    value
}
/// A current valid document is the control before adding the explicit v18 group.
/// New fields must be retained and validated, not ignored or given Rust fallbacks.
#[test]
fn supervision_settings_group_is_explicit_complete_and_versioned() {
    let mut raw = v18_values();
    assert!(EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_ok());
    for (name, value) in [
        ("agent_supervision_poll_interval_ms", "1000"),
        ("agent_supervision_heartbeat_stale_ms", "10000"),
        ("agent_supervision_term_grace_ms", "3000"),
        ("agent_supervision_output_drain_ms", "2000"),
        ("agent_provider_event_max_bytes", "1048576"),
        ("agent_capture_max_bytes", "1048576"),
        ("agent_capture_max_lines", "1000"),
        ("agent_public_message_max_bytes", "1048576"),
        ("agent_log_message_max_bytes", "2048"),
    ] {
        raw[name] = value.into();
    }
    let adopted = EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap())
        .expect("explicit supervision policy must be supported");
    assert_eq!(adopted.schema_version(), 18);
    assert_eq!(serde_json::to_value(adopted).unwrap(), raw);
}

/// A partial v18 group refuses; removing the whole group preserves historical
/// v17 readability but never supplies a supervision policy through a fallback.
/// Each fault has a valid complete document under the same preconditions.
#[test]
fn supervision_missing_members_and_legacy_absence_never_adopt_defaults() {
    let original = v18_values();
    let fields: Vec<_> = swarm_engine::EngineSettingsCatalog::installation()
        .unwrap()
        .fields
        .into_iter()
        .filter(|f| f.introduced_schema_version == 18)
        .collect();
    assert_eq!(fields.len(), 9);
    for field in &fields {
        assert!(
            EngineSettingsValues::decode(&serde_json::to_vec(&original).unwrap())
                .unwrap()
                .agent_execution_policy()
                .is_ok()
        );
        let mut changed = original.clone();
        changed.as_object_mut().unwrap().remove(&field.name);
        assert!(
            EngineSettingsValues::decode(&serde_json::to_vec(&changed).unwrap()).is_err(),
            "missing {} accepted",
            field.name
        );
    }
    let mut legacy = original;
    for field in fields {
        legacy.as_object_mut().unwrap().remove(&field.name);
    }
    let old = EngineSettingsValues::decode(&serde_json::to_vec(&legacy).unwrap()).unwrap();
    assert_eq!(old.schema_version(), 17);
    assert!(old.agent_execution_policy().is_err());
    assert_eq!(serde_json::to_value(old).unwrap(), legacy);
}
/// The nine operator contracts distinguish positive intervals, immediate grace,
/// zero retention and explicit unlimited caps. Frozen policy parsing also requires
/// nullable members and refuses unknown constraints instead of ignoring them.
#[test]
fn supervision_value_boundaries_and_frozen_policy_are_strict() {
    use swarm_engine::{AgentExecutionPolicy, SettingCap, SettingCount};
    let values = EngineSettingsValues::installation_defaults().unwrap();
    let raw = serde_json::to_value(&values).unwrap();
    for name in [
        "agent_supervision_poll_interval_ms",
        "agent_supervision_heartbeat_stale_ms",
    ] {
        for bad in [
            serde_json::Value::String("0".into()),
            serde_json::Value::Null,
            serde_json::Value::String("-1".into()),
            serde_json::json!(1),
        ] {
            assert!(EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_ok());
            let mut changed = raw.clone();
            changed[name] = bad;
            assert!(
                EngineSettingsValues::decode(&serde_json::to_vec(&changed).unwrap()).is_err(),
                "invalid {name}"
            );
        }
    }
    let mut immediate = raw.clone();
    immediate["agent_supervision_term_grace_ms"] = "0".into();
    assert_eq!(
        EngineSettingsValues::decode(&serde_json::to_vec(&immediate).unwrap())
            .unwrap()
            .agent_execution_policy()
            .unwrap()
            .term_grace_ms
            .0,
        0
    );
    let policy = values.agent_execution_policy().unwrap();
    let frozen = serde_json::to_value(&policy).unwrap();
    for (saved, member) in [
        ("agent_supervision_output_drain_ms", "output_drain_ms"),
        ("agent_provider_event_max_bytes", "event_max_bytes"),
        ("agent_capture_max_bytes", "capture_max_bytes"),
        ("agent_capture_max_lines", "capture_max_lines"),
        ("agent_public_message_max_bytes", "public_message_max_bytes"),
        ("agent_log_message_max_bytes", "log_message_max_bytes"),
    ] {
        for choice in [
            serde_json::Value::Null,
            serde_json::Value::String("0".into()),
        ] {
            let mut changed = raw.clone();
            changed[saved] = choice.clone();
            let got = EngineSettingsValues::decode(&serde_json::to_vec(&changed).unwrap())
                .unwrap()
                .agent_execution_policy()
                .unwrap();
            assert_eq!(serde_json::to_value(got).unwrap()[member], choice);
        }
        assert!(
            serde_json::from_value::<AgentExecutionPolicy>(frozen.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        let mut missing = frozen.clone();
        missing.as_object_mut().unwrap().remove(member);
        assert!(
            serde_json::from_value::<AgentExecutionPolicy>(missing).is_err(),
            "missing nullable {member}"
        );
    }
    let mut future = frozen;
    future["unverified_future_constraint"] = true.into();
    assert!(serde_json::from_value::<AgentExecutionPolicy>(future).is_err());
    let mut invalid = policy;
    invalid.capture_max_bytes = SettingCap(Some(SettingCount(-1)));
    assert!(invalid.validate().is_err());
}
/// Public Root preview/apply/reopen adopts v18 explicitly and preserves all v17
/// history and selected guard values. This verifies saved policy consumption only,
/// not process scheduling, an Admin browser or provider execution.
#[cfg(unix)]
#[test]
fn supervision_public_adoption_preview_reopen_preserves_history() {
    use std::{fs, os::unix::fs::DirBuilderExt};
    use swarm_engine::{
        EngineSettingsCatalog, EngineSettingsChange, EngineSettingsDocument, RootStore,
        SystemEnvironment,
    };
    let root =
        std::env::temp_dir().join(swarm_engine::generate_identifier("supervision78-").unwrap());
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    let store = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
    store.close().unwrap();
    let all = v18_values();
    let mut legacy = all.clone();
    let fields: Vec<_> = EngineSettingsCatalog::installation()
        .unwrap()
        .fields
        .into_iter()
        .filter(|f| f.introduced_schema_version == 18)
        .collect();
    for field in &fields {
        legacy.as_object_mut().unwrap().remove(&field.name);
    }
    legacy["agent_interleaved_failure_lookback_multiplier"] = "7".into();
    let old = EngineSettingsDocument::initial(
        EngineSettingsValues::decode(&serde_json::to_vec(&legacy).unwrap()).unwrap(),
        "fixture".into(),
        "2026-10-19T00:00:00Z".into(),
    )
    .unwrap();
    assert_eq!(old.schema_version, 17);
    // Private historical fixture only; a running mission is never patched.
    let settings = root.join(".swarm/engine-settings.json");
    fs::write(&settings, serde_json::to_vec(&old).unwrap()).unwrap();
    let store = RootStore::open(&root, true).unwrap();
    let current = store.settings().unwrap();
    assert_eq!(current, old);
    assert!(current.values.agent_execution_policy().is_err());
    for field in fields {
        legacy[&field.name] = all[&field.name].clone();
    }
    legacy["agent_supervision_poll_interval_ms"] = "17".into();
    legacy["agent_supervision_term_grace_ms"] = "0".into();
    legacy["agent_supervision_output_drain_ms"] = serde_json::Value::Null;
    let values = EngineSettingsValues::decode(&serde_json::to_vec(&legacy).unwrap()).unwrap();
    let mut change = EngineSettingsChange {
        schema_version: 18,
        event_id: "adopt-supervision".into(),
        expected_revision: current.revision,
        values,
        reason: "Explicit private fixture supervision adoption".into(),
        preview_token: String::new(),
    };
    let before = fs::read(&settings).unwrap();
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    assert_eq!(fs::read(&settings).unwrap(), before);
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    store.close().unwrap();
    let reopened = RootStore::open(&root, false).unwrap();
    let actual = reopened.settings().unwrap();
    assert_eq!(actual.schema_version, 18);
    assert_eq!(actual.history[0], old.history[0]);
    assert_eq!(
        actual.values.agent_interleaved_failure_lookback_multiplier,
        old.values.agent_interleaved_failure_lookback_multiplier
    );
    let policy = actual.values.agent_execution_policy().unwrap();
    assert_eq!(policy.poll_interval_ms.0, 17);
    assert_eq!(policy.term_grace_ms.0, 0);
    assert_eq!(policy.output_drain_ms.0, None);
    assert_eq!(actual.values, change.values);
    reopened.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}
