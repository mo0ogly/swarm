//! Saved v33 telemetry store bounds (LIVE-UX L1): retention days and
//! per-attempt cap — complete group, positive or explicitly unlimited,
//! dependency on v30; no store or network effect is exercised here.
use swarm_engine::{EngineSettingsValues, SettingCap, SettingCount};

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
fn telemetry_bounds_default_validate_and_refuse_partial_zero_or_orphan() {
    let defaults = EngineSettingsValues::installation_defaults().unwrap();
    assert_eq!(defaults.schema_version(), current_version());
    // v34 usage-gap tolerance: installation default 10 %, null turns the rule off.
    let mut v34 = truncated(&defaults, 34);
    assert_eq!((v34.schema_version(), v34.usage_mismatch_percent()), (34, Some(10)));
    v34.agent_usage_mismatch_percent = Some(SettingCap(None));
    v34.validate().unwrap();
    assert_eq!(v34.usage_mismatch_percent(), None);
    v34.agent_telemetry_providers = None;
    v34.agent_cache_miss_ratio_percent = None;
    v34.agent_cache_min_input_tokens = None;
    v34.agent_telemetry_retention_days = None;
    v34.agent_telemetry_max_events_per_attempt = None;
    assert_eq!(v34.validate().unwrap_err().reason(), "settings-usage-mismatch-requires-telemetry-policy");
    let mut values = truncated(&defaults, 33);
    assert_eq!(values.schema_version(), 33);
    assert_eq!(values.telemetry_retention_policy().unwrap(), Some((Some(30), None)), "30 days, no cap");
    let encoded = swarm_engine::encode_model(&values).unwrap();
    assert_eq!(EngineSettingsValues::decode(&encoded).unwrap(), values, "explicit null survives a round trip");
    values.agent_telemetry_retention_days = Some(SettingCap(None));
    values.agent_telemetry_max_events_per_attempt = Some(SettingCap(Some(SettingCount(500))));
    values.validate().unwrap();
    assert_eq!(values.telemetry_retention_policy().unwrap(), Some((None, Some(500))));
    for (days, cap, code) in [
        (Some(SettingCap(Some(SettingCount(0)))), Some(SettingCap(None)), "settings-telemetry-retention-positive-or-unlimited"),
        (Some(SettingCap(None)), Some(SettingCap(Some(SettingCount(0)))), "settings-telemetry-cap-positive-or-unlimited"),
        (Some(SettingCap(None)), None, "settings-telemetry-retention-incomplete"),
    ] {
        values.agent_telemetry_retention_days = days;
        values.agent_telemetry_max_events_per_attempt = cap;
        assert_eq!(values.validate().unwrap_err().reason(), code);
    }
    // Historical absence: no purge, no cap (never an implicit default).
    let older = truncated(&defaults, 32);
    assert_eq!(older.telemetry_retention_policy().unwrap(), None);
    // v33 without the v30 telemetry list is a partial history and refuses.
    let mut orphan = truncated(&defaults, 33);
    orphan.agent_telemetry_providers = None;
    orphan.agent_cache_miss_ratio_percent = None;
    orphan.agent_cache_min_input_tokens = None;
    assert_eq!(orphan.validate().unwrap_err().reason(), "settings-telemetry-retention-requires-telemetry-policy");
}

#[test]
fn alert_policy_v35_defaults_validate_and_refuse_partial_zero_range_or_orphan() {
    let defaults = EngineSettingsValues::installation_defaults().unwrap();
    let v35 = truncated(&defaults, 35);
    assert_eq!((v35.schema_version(), v35.alert_evaluation_seconds(), v35.budget_warning_percent()), (35, Some(60), Some(80)));
    v35.validate().unwrap();
    let mut off = v35.clone();
    off.agent_alert_evaluation_seconds = Some(SettingCap(None));
    off.work_budget_warning_percent = Some(SettingCap(None));
    off.validate().unwrap();
    assert_eq!((off.alert_evaluation_seconds(), off.budget_warning_percent(), off.schema_version()), (None, None, 35), "explicit null is v35, off");
    let set = |seconds: Option<SettingCap>, percent: Option<SettingCap>| {
        let mut v = v35.clone();
        v.agent_alert_evaluation_seconds = seconds;
        v.work_budget_warning_percent = percent;
        v.validate().map_err(|e| e.reason())
    };
    let n = |v: i64| Some(SettingCap(Some(SettingCount(v))));
    assert_eq!(set(n(1), n(100)), Ok(()), "bounds accepted");
    for (seconds, percent, code) in [
        (n(0), n(80), "settings-alert-evaluation-positive-or-off"),
        (n(60), n(0), "settings-budget-warning-percent-range"),
        (n(60), n(101), "settings-budget-warning-percent-range"),
        (n(60), None, "settings-alert-policy-incomplete"),
        (None, n(80), "settings-alert-policy-incomplete"),
    ] {
        assert_eq!(set(seconds, percent), Err(code));
    }
    let mut orphan = v35.clone();
    orphan.agent_usage_mismatch_percent = None;
    assert_eq!(orphan.validate().unwrap_err().reason(), "settings-alert-policy-requires-usage-mismatch-policy");
    // Historical absence: no server loop, no warning (never an implicit default).
    let older = truncated(&defaults, 34);
    assert_eq!((older.alert_evaluation_seconds(), older.budget_warning_percent()), (None, None));
}

#[test]
fn annotation_note_bound_v36_defaults_validate_and_refuse_zero_or_orphan() {
    let defaults = EngineSettingsValues::installation_defaults().unwrap();
    let v36 = truncated(&defaults, 36);
    assert_eq!((v36.schema_version(), v36.annotation_note_limit()), (36, Some(Some(2000))));
    let mut unlimited = v36.clone();
    unlimited.annotation_note_max_chars = Some(SettingCap(None));
    unlimited.validate().unwrap();
    assert_eq!(unlimited.annotation_note_limit(), Some(None));
    let mut zero = v36.clone();
    zero.annotation_note_max_chars = Some(SettingCap(Some(SettingCount(0))));
    assert_eq!(zero.validate().unwrap_err().reason(), "settings-annotation-note-positive-or-unlimited");
    let mut orphan = v36.clone();
    orphan.agent_alert_evaluation_seconds = None;
    orphan.work_budget_warning_percent = None;
    assert_eq!(orphan.validate().unwrap_err().reason(), "settings-annotation-requires-alert-policy");
    assert_eq!(truncated(&defaults, 35).annotation_note_limit(), None, "absent before v36");
}

#[test]
fn evaluation_sample_v37_defaults_validate_and_refuse_range_or_orphan() {
    let defaults = EngineSettingsValues::installation_defaults().unwrap();
    let v37 = truncated(&defaults, 37);
    assert_eq!((v37.schema_version(), v37.evaluation_sample_percent()), (37, Some(100)));
    let mut off = v37.clone();
    off.agent_evaluation_sample_percent = Some(SettingCap(None));
    off.validate().unwrap();
    assert_eq!(off.evaluation_sample_percent(), None);
    for bad in [0, 101] {
        let mut v = v37.clone();
        v.agent_evaluation_sample_percent = Some(SettingCap(Some(SettingCount(bad))));
        assert_eq!(v.validate().unwrap_err().reason(), "settings-evaluation-sample-percent-range", "{bad}");
    }
    let mut orphan = v37.clone();
    orphan.annotation_note_max_chars = None;
    assert_eq!(orphan.validate().unwrap_err().reason(), "settings-evaluation-requires-annotation-policy");
    assert_eq!(truncated(&defaults, 36).evaluation_sample_percent(), None, "absent before v37");
}
