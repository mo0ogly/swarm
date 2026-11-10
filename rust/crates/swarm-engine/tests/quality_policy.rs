use swarm_engine::{
    EngineSettingsChange, EngineSettingsDocument, EngineSettingsValues, SettingCount,
};
fn initial() -> EngineSettingsDocument {
    EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "operator".into(),
        "2026-10-18T00:00:00Z".into(),
    )
    .unwrap()
}
fn apply(
    d: &EngineSettingsDocument,
    event: &str,
    values: EngineSettingsValues,
) -> EngineSettingsDocument {
    let mut c = EngineSettingsChange {
        schema_version: values.schema_version(),
        event_id: event.into(),
        expected_revision: d.revision,
        values,
        reason: "Explicit operator policy".into(),
        preview_token: String::new(),
    };
    c.preview_token = d.preview(&c).unwrap().preview_token;
    d.apply(&c, "operator".into(), "2026-10-18T00:00:01Z".into())
        .unwrap()
        .0
}
#[test]
fn policy_generation_prevents_reverting_thresholds_from_reviving_an_old_receipt() {
    let original = initial();
    let first = original.quality_policy_snapshot().unwrap();
    assert_eq!(first.generation, SettingCount(0));
    assert!(first.legacy_method2_thresholds_compatible);
    let mut values = original.values.clone();
    values.storage_busy_retry_delay_ms = SettingCount(26);
    let unrelated = apply(&original, "unrelated", values);
    assert_eq!(
        unrelated.quality_policy_snapshot().unwrap().digest,
        first.digest
    );
    let mut values = unrelated.values.clone();
    values.quality_delivery_min_percent = Some(SettingCount(60));
    let changed = apply(&unrelated, "threshold", values);
    let middle = changed.quality_policy_snapshot().unwrap();
    assert_eq!(middle.generation, SettingCount(1));
    assert_ne!(middle.digest, first.digest);
    assert!(!middle.legacy_method2_thresholds_compatible);
    let mut values = changed.values.clone();
    values.quality_delivery_min_percent = Some(SettingCount(70));
    let reverted = apply(&changed, "revert", values);
    let last = reverted.quality_policy_snapshot().unwrap();
    assert_eq!(last.values, first.values);
    assert_eq!(last.generation, SettingCount(2));
    assert_ne!(last.digest, first.digest);
    assert!(!last.legacy_method2_thresholds_compatible);
    let reopened =
        EngineSettingsDocument::decode(&swarm_wire_json::encode(&reverted).unwrap()).unwrap();
    assert_eq!(reopened.quality_policy_snapshot().unwrap(), last);
}
#[test]
fn actual_v1_v2_documents_replay_exactly_and_require_explicit_complete_v3_adoption() {
    for (raw, request) in [
        (
            include_bytes!("fixtures/legacy-v1-document.json").as_slice(),
            include_bytes!("fixtures/legacy-v1-request.json").as_slice(),
        ),
        (
            include_bytes!("fixtures/legacy-v2-document.json").as_slice(),
            include_bytes!("fixtures/legacy-v2-request.json").as_slice(),
        ),
    ] {
        let old = EngineSettingsDocument::decode(raw).unwrap();
        assert_eq!(swarm_wire_json::encode(&old).unwrap(), raw);
        assert!(old.quality_policy_snapshot().is_err());
        let replay: EngineSettingsChange = serde_json::from_slice(request).unwrap();
        assert!(old.replay(&replay).unwrap());
        let defaults = EngineSettingsValues::installation_defaults().unwrap();
        let mut values = old.values.clone();
        values.validation_max_output_bytes = defaults.validation_max_output_bytes;
        values.quality_delivery_min_percent = defaults.quality_delivery_min_percent;
        values.quality_domain_min_percent = defaults.quality_domain_min_percent;
        values.quality_band_ok_min_percent = defaults.quality_band_ok_min_percent;
        values.quality_band_warn_min_percent = defaults.quality_band_warn_min_percent;
        values.quality_band_blocked_min_percent = defaults.quality_band_blocked_min_percent;
        let new = apply(&old, "adopt-quality", values);
        assert_eq!(new.schema_version, 3);
        assert_eq!(&new.history[..old.history.len()], &old.history);
        assert!(new.replay(&replay).unwrap());
        assert_eq!(swarm_wire_json::encode(&old).unwrap(), raw);
    }
}
#[test]
fn percent_representation_and_profile_consistency_refuse_without_silent_defaults() {
    let original = initial();
    let before = swarm_wire_json::encode(&original).unwrap();
    for bad in 0..5 {
        let mut values = original.values.clone();
        match bad {
            0 => values.quality_delivery_min_percent = Some(SettingCount(101)),
            1 => values.quality_domain_min_percent = Some(SettingCount(-1)),
            2 => values.quality_band_ok_min_percent = Some(SettingCount(40)),
            3 => values.quality_band_warn_min_percent = None,
            _ => values.validation_max_output_bytes = None,
        };
        assert!(values.validate().is_err());
        assert_eq!(swarm_wire_json::encode(&original).unwrap(), before);
    }
    let mut values = original.values.clone();
    values.quality_delivery_min_percent = Some(SettingCount(0));
    values.quality_domain_min_percent = Some(SettingCount(100));
    values.validate().unwrap();
    let mut raw = serde_json::to_value(&values).unwrap();
    raw["quality_delivery_min_percent"] = serde_json::Value::Null;
    assert!(EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_err());
}
