//! Response limits must be explicit persisted policy, never capture truncation.
use swarm_engine::{EngineSettingsDocument, EngineSettingsValues, SettingCap, SettingCount};
/// Actual pre-v26 initialized document; no strip of a current policy is used.
fn legacy() -> EngineSettingsDocument {
    EngineSettingsDocument::decode(include_bytes!("fixtures/legacy-v25-document.json")).unwrap()
}
#[test]
fn planning_response_historical_absence_requires_explicit_adoption() {
    let doc = legacy();
    assert_eq!(doc.schema_version, 25);
    assert!(doc.values.validate().is_ok());
    assert_eq!(
        doc.values.planning_response_limit().err().unwrap().reason(),
        "settings-planning-response-not-configured"
    );
    let current = EngineSettingsValues::installation_defaults().unwrap();
    assert_eq!(current.schema_version(), 27);
    assert_eq!(current.planning_response_limit().unwrap(), Some(65536));
}
#[test]
fn planning_response_zero_null_and_negative_are_distinct() {
    let mut v = EngineSettingsValues::installation_defaults().unwrap();
    v.planning_response_max_bytes = Some(SettingCap(Some(SettingCount(0))));
    assert_eq!(v.planning_response_limit().unwrap(), Some(0));
    assert!(v.validate().is_ok());
    v.planning_response_max_bytes = Some(SettingCap(None));
    assert_eq!(v.planning_response_limit().unwrap(), None);
    assert!(v.validate().is_ok());
    v.planning_response_max_bytes = Some(SettingCap(Some(SettingCount(-1))));
    assert!(v.validate().is_err());
}
#[test]
fn planning_response_policy_requires_prior_decision_group() {
    let mut v = EngineSettingsValues::installation_defaults().unwrap();
    assert!(v.validate().is_ok());
    v.planning_decision_max_inputs = None;
    assert!(v.validate().is_err());
}

/// Real v25 history remains intact through explicit preview/apply, serialized
/// reopening and replay. A label alone never supplies the missing policy.
#[test]
fn planning_response_v25_adoption_preserves_history_and_replays() {
    let old = legacy();
    let original = swarm_wire_json::encode(&old).unwrap();
    let mut r = swarm_engine::EngineSettingsChange {
        schema_version: 26,
        event_id: "response-adoption".into(),
        expected_revision: old.revision,
        values: old.values.clone(),
        reason: "Explicit response bound".into(),
        preview_token: String::new(),
    };
    assert!(old.preview(&r).is_err());
    r.values.planning_response_max_bytes = Some(SettingCap(Some(SettingCount(17))));
    let preview = old.preview(&r).unwrap();
    assert_eq!(swarm_wire_json::encode(&old).unwrap(), original);
    assert!(old
        .apply(&r, "operator".into(), "2026-10-20T13:00:00Z".into())
        .is_err());
    r.preview_token = preview.preview_token;
    let (new, replay) = old
        .apply(&r, "operator".into(), "2026-10-20T13:00:00Z".into())
        .unwrap();
    assert!(!replay);
    assert_eq!(new.schema_version, 26);
    assert_eq!(&new.history[..old.history.len()], &old.history);
    let reopened = EngineSettingsDocument::decode(&swarm_wire_json::encode(&new).unwrap()).unwrap();
    assert_eq!(reopened.values.planning_response_limit().unwrap(), Some(17));
    assert_eq!(
        reopened.apply(&r, String::new(), "invalid".into()).unwrap(),
        (new, true)
    );
}
