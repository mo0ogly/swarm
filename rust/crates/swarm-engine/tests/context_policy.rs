use swarm_engine::{
    EngineSettingsChange, EngineSettingsDocument, EngineSettingsValues, SettingCap, SettingCount,
};
/// Preserve original v3 bytes/replay and explicitly adopt only the historical
/// v4 context group. Current installation values are checked separately; none
/// are inferred into retained journals or treated as operator authorization.
#[test]
fn v3_journal_replay_remains_exact_and_context_limits_require_explicit_complete_adoption() {
    let raw = include_bytes!("fixtures/legacy-v3-document.json");
    let old = EngineSettingsDocument::decode(raw).unwrap();
    assert_eq!(old.schema_version, 3);
    assert_eq!(swarm_engine::encode_model(&old).unwrap(), raw);
    let replay: EngineSettingsChange =
        serde_json::from_slice(include_bytes!("fixtures/legacy-v3-request.json")).unwrap();
    assert!(old.replay(&replay).unwrap());
    assert!(old.values.context_policy_limits().is_err());
    let defaults = EngineSettingsValues::installation_defaults().unwrap();
    let limits = defaults.context_policy_limits().unwrap();
    assert_eq!(limits.profile_document_bytes, Some(16000));
    assert_eq!(limits.context_bytes, Some(16000));
    assert_eq!(limits.worker_context_bytes, Some(32000));
    assert_eq!(limits.review_delivery_bytes, Some(48000));
    let mut values = old.values.clone();
    values.project_profile_max_name_bytes = defaults.project_profile_max_name_bytes;
    assert!(values.validate().is_err());
    values.project_profile_max_sources = defaults.project_profile_max_sources;
    values.project_profile_max_path_bytes = defaults.project_profile_max_path_bytes;
    values.project_profile_max_document_bytes = defaults.project_profile_max_document_bytes;
    values.project_context_max_bytes = defaults.project_context_max_bytes;
    values.project_worker_context_max_bytes = defaults.project_worker_context_max_bytes;
    values.review_delivery_max_bytes = defaults.review_delivery_max_bytes;
    let mut change = EngineSettingsChange {
        schema_version: 4,
        event_id: "adopt-context".into(),
        expected_revision: old.revision,
        values,
        reason: "Explicit context policy adoption".into(),
        preview_token: String::new(),
    };
    change.preview_token = old.preview(&change).unwrap().preview_token;
    let new = old
        .apply(&change, "operator".into(), "2026-10-18T00:00:00Z".into())
        .unwrap()
        .0;
    assert_eq!(new.schema_version, 4);
    assert!(new.replay(&replay).unwrap());
    assert_eq!(
        new.quality_policy_snapshot().unwrap().digest,
        old.quality_policy_snapshot().unwrap().digest
    );
    assert_eq!(&new.history[..old.history.len()], &old.history);
    assert_eq!(swarm_engine::encode_model(&old).unwrap(), raw);
    let mut values = new.values.clone();
    values.project_context_max_bytes = Some(SettingCap(None));
    values.project_profile_max_sources = Some(SettingCap(Some(SettingCount(0))));
    let limits = values.context_policy_limits().unwrap();
    assert_eq!(limits.context_bytes, None);
    assert_eq!(limits.profile_sources, Some(0));
}
