use std::time::Duration;
use swarm_engine::{EngineSettingsDocument, EngineSettingsValues, PlanningWaitStop, SettingCount};

/// Actual v26 document produced by retained normal125 before adding fields27.
fn legacy() -> EngineSettingsValues {
    EngineSettingsDocument::decode(include_bytes!("fixtures/legacy-v26-document.json"))
        .unwrap()
        .values
}

#[test]
fn planning_wait_legacy_absence_does_not_acquire_defaults() {
    let values = legacy();
    assert_eq!(values.schema_version(), 26);
    assert_eq!(
        values.planning_wait_policy().unwrap_err().reason(),
        "settings-planning-wait-not-configured"
    );
    assert!(!serde_json::to_value(values)
        .unwrap()
        .as_object()
        .unwrap()
        .contains_key("planning_provider_silence_seconds"));
}

#[test]
fn planning_wait_defaults_preserve_go_values_and_explicit_renewal() {
    let values = EngineSettingsValues::installation_defaults().unwrap();
    assert_eq!(values.schema_version(), 27);
    let policy = values.planning_wait_policy().unwrap();
    assert_eq!(policy.silence_seconds.0, 300);
    assert_eq!(policy.max_duration_seconds.0, 0);
    assert_eq!(policy.default_lease_seconds.0, 120);
    assert_eq!(policy.renew_after_percent.0, 50);
    assert_eq!(
        policy.renewal_delay(5).unwrap(),
        Some(Duration::from_millis(2500))
    );
}

#[test]
fn planning_wait_requires_complete_group_and_current_lease_bounds() {
    let mut values = legacy();
    values.planning_provider_silence_seconds = Some(SettingCount(0));
    assert_eq!(
        values.validate().unwrap_err().reason(),
        "settings-planning-wait-not-configured"
    );
    let mut values = EngineSettingsValues::installation_defaults().unwrap();
    values.planning_default_lease_seconds = Some(SettingCount(0));
    assert!(values.validate().is_err());
    values.planning_default_lease_seconds = Some(SettingCount(4));
    assert_eq!(
        values.validate().unwrap_err().reason(),
        "planning-lease-outside-policy"
    );
    values.planning_default_lease_seconds = Some(SettingCount(300));
    values.validate().unwrap();
    values.planning_default_lease_seconds = Some(SettingCount(301));
    assert!(values.validate().is_err());
}

#[test]
fn planning_wait_disabled_timers_are_not_an_inferred_duration_limit() {
    let mut values = EngineSettingsValues::installation_defaults().unwrap();
    values.planning_provider_silence_seconds = Some(SettingCount(0));
    let policy = values.planning_wait_policy().unwrap();
    assert_eq!(
        policy.stop_reason(Duration::MAX, Duration::MAX).unwrap(),
        None
    );
    values.planning_provider_max_duration_seconds = Some(SettingCount(3));
    let policy = values.planning_wait_policy().unwrap();
    assert_eq!(
        policy
            .stop_reason(Duration::from_secs(2), Duration::MAX)
            .unwrap(),
        None
    );
    assert_eq!(
        policy
            .stop_reason(Duration::from_secs(3), Duration::ZERO)
            .unwrap(),
        Some(PlanningWaitStop::MaximumDuration)
    );
    values.planning_provider_max_duration_seconds = Some(SettingCount(0));
    values.planning_provider_silence_seconds = Some(SettingCount(2));
    let policy = values.planning_wait_policy().unwrap();
    assert_eq!(
        policy
            .stop_reason(Duration::from_secs(100), Duration::from_secs(1))
            .unwrap(),
        None
    );
    assert_eq!(
        policy
            .stop_reason(Duration::from_secs(100), Duration::from_secs(2))
            .unwrap(),
        Some(PlanningWaitStop::Silence)
    );
}

#[test]
fn planning_wait_renewal_is_saved_and_rejects_invalid_percentages() {
    let mut values = EngineSettingsValues::installation_defaults().unwrap();
    values.planning_lease_renew_after_percent = Some(SettingCount(25));
    assert_eq!(
        values
            .planning_wait_policy()
            .unwrap()
            .renewal_delay(5)
            .unwrap(),
        Some(Duration::from_millis(1250))
    );
    values.planning_lease_renew_after_percent = Some(SettingCount(0));
    assert_eq!(
        values
            .planning_wait_policy()
            .unwrap()
            .renewal_delay(5)
            .unwrap(),
        None
    );
    for n in [-1, 100] {
        values.planning_lease_renew_after_percent = Some(SettingCount(n));
        assert!(values.validate().is_err());
    }
    values.planning_lease_renew_after_percent = Some(SettingCount(50));
    values.planning_provider_silence_seconds = Some(SettingCount(-1));
    assert!(values.validate().is_err());
}
