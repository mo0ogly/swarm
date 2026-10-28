use swarm_engine::EngineSettingsValues;
/// A valid installation witness precedes the new explicit cap proposal. A cap
/// is receipt policy, separate from reviewer text; this performs no IO or adoption.
#[test]
fn receipt_policy_has_explicit_nullable_installation_value() {
    let current = EngineSettingsValues::installation_defaults().unwrap();
    current.validate().unwrap();
    let mut raw = serde_json::to_value(&current).unwrap();
    raw["managed_receipt_max_bytes"] = serde_json::json!("1048576");
    let proposed = EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(proposed.managed_receipt_limit().unwrap(), Some(1048576));
    raw["managed_receipt_max_bytes"] = serde_json::Value::Null;
    let unlimited = EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(unlimited.managed_receipt_limit().unwrap(), None);
}

/// Historical v9 remains readable and replayable; only the new consumer requires
/// explicit adoption. Preview is pure and apply retains the exact history prefix.
#[test]
fn legacy_v9_policy_requires_adoption_and_preserves_original_history() {
    use swarm_engine::{EngineSettingsChange, EngineSettingsDocument, SettingCap, SettingCount};
    let old_raw = include_bytes!("fixtures/legacy-v9-document.json");
    let old = EngineSettingsDocument::decode(old_raw).unwrap();
    assert_eq!(old.schema_version, 9);
    assert_eq!(
        old.values.managed_receipt_limit().unwrap_err().reason(),
        "settings-managed-receipt-policy-not-configured"
    );
    assert!(old.replay(&old.history.last().unwrap().change).unwrap());
    let history = serde_json::to_value(&old.history).unwrap();
    let mut values = old.values.clone();
    values.managed_receipt_max_bytes = Some(SettingCap(Some(SettingCount(0))));
    let mut request = EngineSettingsChange {
        schema_version: 10,
        event_id: "receipt-adoption".into(),
        expected_revision: old.revision,
        values,
        reason: "Explicit machine receipt policy".into(),
        preview_token: String::new(),
    };
    request.preview_token = old.preview(&request).unwrap().preview_token;
    assert_eq!(serde_json::to_value(&old.history).unwrap(), history);
    let (current, replayed) = old
        .apply(
            &request,
            "fixture-operator".into(),
            "2026-10-19T00:00:00Z".into(),
        )
        .unwrap();
    assert!(!replayed);
    assert_eq!(current.schema_version, 10);
    assert_eq!(current.values.managed_receipt_limit().unwrap(), Some(0));
    assert_eq!(
        serde_json::to_value(&current.history[..old.history.len()]).unwrap(),
        history
    );
    assert!(current.replay(&request).unwrap());
    let mut negative = current.values.clone();
    negative.managed_receipt_max_bytes = Some(SettingCap(Some(SettingCount(-1))));
    assert!(negative.validate().is_err());
    negative = current.values.clone();
    negative.task_manual_required_gate_phases = None;
    assert!(negative.validate().is_err());
}
