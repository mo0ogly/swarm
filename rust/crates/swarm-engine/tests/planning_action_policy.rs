//! Scope bounds are persisted configuration, never supervisor authority.
use swarm_engine::{EngineSettingsDocument, EngineSettingsValues, SettingCap, SettingCount};

/// Explicit minimum, finite/unlimited maximum and zero projection caps reach the
/// consumer without clamping. Invalid values/partial/historical groups refuse.
#[test]
fn scope_lease_policy_boundaries_projection_caps_and_legacy_are_explicit() {
    let mut values = EngineSettingsValues::installation_defaults().unwrap();
    let policy = values.planning_action_policy().unwrap();
    policy.check_lease(5).unwrap();
    policy.check_lease(300).unwrap();
    assert!(policy.check_lease(4).is_err());
    assert!(policy.check_lease(301).is_err());
    values.planning_scope_lease_min_seconds = Some(SettingCount(1));
    values.planning_scope_lease_max_seconds = Some(SettingCap(None));
    values.planning_context_max_events = Some(SettingCap(Some(SettingCount(0))));
    values.planning_context_max_bytes = Some(SettingCap(None));
    let changed = values.planning_action_policy().unwrap();
    changed.check_lease(i64::MAX).unwrap();
    assert_eq!(changed.context_max_events, Some(0));
    assert_eq!(changed.context_max_bytes, None);
    let control = values.clone();
    values.planning_scope_lease_max_seconds = Some(SettingCap(Some(SettingCount(0))));
    assert!(values.validate().is_err());
    values = control.clone();
    values.planning_context_max_bytes = Some(SettingCap(Some(SettingCount(-1))));
    assert!(values.validate().is_err());
    values = control;
    values.planning_context_max_events = None;
    assert!(values.validate().is_err());
    let old = EngineSettingsDocument::decode(include_bytes!("fixtures/legacy-v23-document.json"))
        .unwrap();
    assert_eq!(old.schema_version, 23);
    assert!(old.values.planning_action_policy().is_err());
}
