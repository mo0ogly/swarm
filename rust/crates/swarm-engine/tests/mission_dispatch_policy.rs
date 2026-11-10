//! Configured slot selection is neither consent nor a launch permit.
use swarm_engine::{
    EngineSettingsCatalog, EngineSettingsChange, EngineSettingsDocument, EngineSettingsValues,
    SettingCount,
};
/// Full configured witness precedes absent/partial/invalid variants. No hidden
/// cap16 survives an explicit larger ceiling or null operator ceiling.
#[test]
fn mission_dispatch_explicit_defaults_boundaries_unlimited_and_no_clamp() {
    let mut values = EngineSettingsValues::installation_defaults().unwrap();
    let p = values.mission_dispatch_policy().unwrap();
    assert_eq!(p.slots(None).unwrap(), SettingCount(2));
    assert_eq!(p.slots(Some(16)).unwrap(), SettingCount(16));
    assert!(p.slots(Some(17)).is_err());
    assert!(p.slots(Some(0)).is_err());
    values.mission_dispatch_default_slots = Some(SettingCount(17));
    assert!(values.validate().is_err());
    values.mission_dispatch_max_slots = Some(swarm_engine::SettingCap(Some(SettingCount(33))));
    values.validate().unwrap();
    assert_eq!(
        values
            .mission_dispatch_policy()
            .unwrap()
            .slots(None)
            .unwrap(),
        SettingCount(17)
    );
    assert_eq!(
        values
            .mission_dispatch_policy()
            .unwrap()
            .slots(Some(33))
            .unwrap(),
        SettingCount(33)
    );
    values.mission_dispatch_max_slots = Some(swarm_engine::SettingCap(None));
    assert_eq!(
        values
            .mission_dispatch_policy()
            .unwrap()
            .slots(Some(i64::MAX))
            .unwrap(),
        SettingCount(i64::MAX)
    );
    for bad in [0, -1] {
        values.mission_dispatch_default_slots = Some(SettingCount(bad));
        assert!(values.validate().is_err());
    }
}
/// Preserve actual v21 history, require explicit adoption/token, reject partial,
/// null default and invalid cap; no policy fallback is manufactured on decode.
#[test]
fn mission_dispatch_history_explicit_adoption_and_invalid_groups() {
    let defaults =
        EngineSettingsDocument::decode(include_bytes!("fixtures/legacy-v22-document.json"))
            .unwrap()
            .values;
    let old = EngineSettingsDocument::decode(include_bytes!("fixtures/legacy-v21-document.json"))
        .unwrap()
        .values;
    old.validate().unwrap();
    assert_eq!(old.schema_version(), 21);
    assert!(old.mission_dispatch_policy().is_err());
    let doc = EngineSettingsDocument::initial(old, "fixture".into(), "2026-10-20T00:00:00Z".into())
        .unwrap();
    let raw = swarm_wire_json::encode(&doc).unwrap();
    assert_eq!(
        swarm_wire_json::encode(&EngineSettingsDocument::decode(&raw).unwrap()).unwrap(),
        raw
    );
    let mut change = EngineSettingsChange {
        schema_version: 22,
        event_id: "adopt-slots".into(),
        expected_revision: doc.revision,
        values: defaults.clone(),
        reason: "Explicit slot policy adoption".into(),
        preview_token: String::new(),
    };
    assert!(doc
        .apply(&change, "fixture".into(), "2026-10-20T00:00:01Z".into())
        .is_err());
    change.preview_token = doc.preview(&change).unwrap().preview_token;
    let (next, replay) = doc
        .apply(&change, "fixture".into(), "2026-10-20T00:00:01Z".into())
        .unwrap();
    assert!(!replay);
    assert_eq!(next.history[0], doc.history[0]);
    assert_eq!(next.schema_version, 22);
    assert_eq!(
        next.apply(&change, "ignored".into(), "invalid".into())
            .unwrap(),
        (next.clone(), true)
    );
    for name in [
        "mission_dispatch_default_slots",
        "mission_dispatch_max_slots",
    ] {
        let mut value = serde_json::to_value(&defaults).unwrap();
        value.as_object_mut().unwrap().remove(name);
        assert!(EngineSettingsValues::decode(&serde_json::to_vec(&value).unwrap()).is_err());
        for bad in [serde_json::json!("0"), serde_json::json!("-1")] {
            let mut value = serde_json::to_value(&defaults).unwrap();
            value[name] = bad;
            assert!(EngineSettingsValues::decode(&serde_json::to_vec(&value).unwrap()).is_err());
        }
    }
    let mut value = serde_json::to_value(&defaults).unwrap();
    value["mission_dispatch_default_slots"] = serde_json::Value::Null;
    assert!(EngineSettingsValues::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    let c = EngineSettingsCatalog::installation().unwrap();
    assert_eq!(c.document_schema_version, 27);
    assert!(c
        .fields
        .iter()
        .any(|f| f.name == "mission_dispatch_max_slots" && f.nullable));
}
