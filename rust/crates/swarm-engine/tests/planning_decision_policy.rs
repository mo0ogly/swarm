//! Decision policy remains explicit persisted data, not execution authority.
use swarm_engine::{
    EngineSettingsCatalog, EngineSettingsDocument, EngineSettingsValues, SettingCap, SettingCount,
};
/// Every nullable cap supports zero/unlimited; negative/partial values and
/// incompatible worker defaults refuse. Genuine v24 data stays readable and
/// cannot obtain v25 values through an implicit fallback.
#[test]
fn planning_decision_policy_caps_defaults_partial_and_historical_are_explicit() {
    let values = EngineSettingsValues::installation_defaults().unwrap();
    let policy = values.planning_decision_policy().unwrap();
    assert_eq!(policy.max_inputs, Some(100));
    assert_eq!(policy.max_operations, Some(20));
    assert_eq!(policy.worker_default_attempts, 2);
    let fields: Vec<_> = EngineSettingsCatalog::installation()
        .unwrap()
        .fields
        .into_iter()
        .filter(|f| f.introduced_schema_version == 25 && f.nullable)
        .map(|f| f.name)
        .collect();
    assert_eq!(fields.len(), 7);
    for field in fields {
        for value in [
            serde_json::Value::Null,
            serde_json::Value::String("0".into()),
        ] {
            let mut raw = serde_json::to_value(&values).unwrap();
            raw[&field] = value;
            let decoded = EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
            decoded.planning_decision_policy().unwrap();
        }
        let mut raw = serde_json::to_value(&values).unwrap();
        raw[&field] = serde_json::json!("-1");
        assert!(EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_err());
        raw.as_object_mut().unwrap().remove(&field);
        assert!(EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_err());
    }
    let mut changed = values;
    changed.planning_worker_default_attempts = Some(SettingCount(0));
    assert!(changed.validate().is_err());
    changed.planning_worker_default_attempts = Some(SettingCount(4));
    assert!(changed.validate().is_err());
    changed.task_max_attempts = None;
    changed.planning_worker_default_attempts = Some(SettingCount(i64::MAX));
    changed.validate().unwrap();
    changed.planning_decision_max_operations = Some(SettingCap(None));
    assert_eq!(
        changed.planning_decision_policy().unwrap().max_operations,
        None
    );
    let legacy =
        EngineSettingsDocument::decode(include_bytes!("fixtures/legacy-v24-document.json"))
            .unwrap();
    assert_eq!(legacy.schema_version, 24);
    assert!(legacy.values.planning_decision_policy().is_err());
    legacy.values.planning_action_policy().unwrap();
}
