//! Creation defaults are persisted choices, not active review enforcement.
use swarm_engine::{EngineSettingsValues, SettingCount};
/// Exact installed defaults are declared in JSON, never adopted by this builder.
/// Changed values/zero reach the configuration projection; negative/partial or
/// historical absence refuses without mutating the supplied settings.
#[test]
fn reviewer_creation_defaults_require_complete_explicit_values_and_preserve_zero() {
    let mut values = EngineSettingsValues::installation_defaults().unwrap();
    let initial = values.reviewer_creation_defaults().unwrap();
    assert_eq!(initial.delivery_files, SettingCount(100));
    assert_eq!(initial.context_files, SettingCount(24));
    assert_eq!(initial.source_bytes, SettingCount(98304));
    assert_eq!(initial.context_bytes, SettingCount(131072));
    values.reviewer_default_delivery_files = Some(SettingCount(7));
    values.reviewer_default_context_files = Some(SettingCount(0));
    values.reviewer_default_source_bytes = Some(SettingCount(3072));
    values.reviewer_default_context_bytes = Some(SettingCount(8192));
    let changed = values.reviewer_creation_defaults().unwrap();
    assert_eq!(changed.delivery_files, SettingCount(7));
    assert_eq!(changed.context_files, SettingCount(0));
    assert_eq!(changed.source_bytes, SettingCount(3072));
    assert_eq!(changed.context_bytes, SettingCount(8192));
    let valid = values.clone();
    values.reviewer_default_source_bytes = Some(SettingCount(-1));
    assert!(values.reviewer_creation_defaults().is_err());
    assert!(values.validate().is_err());
    values = valid.clone();
    values.reviewer_default_source_bytes = None;
    assert!(values.reviewer_creation_defaults().is_err());
    assert!(values.validate().is_err());
    values = swarm_engine::EngineSettingsDocument::decode(include_bytes!(
        "fixtures/legacy-v23-document.json"
    ))
    .unwrap()
    .values;
    values.planning_scope_lease_min_seconds = None;
    values.planning_scope_lease_max_seconds = None;
    values.planning_context_max_events = None;
    values.planning_context_max_bytes = None;
    values.reviewer_default_delivery_files = None;
    values.reviewer_default_context_files = None;
    values.reviewer_default_source_bytes = None;
    values.reviewer_default_context_bytes = None;
    values.validate().unwrap();
    assert_eq!(values.schema_version(), 22);
    assert!(values.reviewer_creation_defaults().is_err());
}

/// Explicit v22->v23 adoption changes only the new document/history entry;
/// the old receipt and its bytes survive, with a required current preview token.
#[test]
fn reviewer_creation_history_requires_explicit_v23_adoption() {
    use swarm_engine::{EngineSettingsChange, EngineSettingsDocument};
    let raw = include_bytes!("fixtures/legacy-v22-document.json");
    let doc = EngineSettingsDocument::decode(raw).unwrap();
    assert_eq!(doc.schema_version, 22);
    assert!(doc.values.reviewer_creation_defaults().is_err());
    let original = doc.clone();
    // Keep this historical22->23 test exact when the current profile advances.
    let mut values =
        EngineSettingsDocument::decode(include_bytes!("fixtures/legacy-v23-document.json"))
            .unwrap()
            .values;
    values.reviewer_default_context_files = Some(SettingCount(0));
    let mut request = EngineSettingsChange {
        schema_version: 23,
        event_id: "adopt-reviewer-defaults".into(),
        expected_revision: doc.revision,
        values,
        reason: "Explicit reviewer creation defaults".into(),
        preview_token: String::new(),
    };
    assert!(doc
        .apply(&request, "fixture".into(), "2026-10-20T00:00:00Z".into())
        .is_err());
    request.preview_token = doc.preview(&request).unwrap().preview_token;
    let (next, replayed) = doc
        .apply(&request, "fixture".into(), "2026-10-20T00:00:00Z".into())
        .unwrap();
    assert!(!replayed);
    assert_eq!(next.schema_version, 23);
    assert_eq!(next.history[0], doc.history[0]);
    assert_eq!(
        next.values
            .reviewer_creation_defaults()
            .unwrap()
            .context_files,
        SettingCount(0)
    );
    assert_eq!(doc, original);
    assert_eq!(
        next.apply(&request, "ignored".into(), "invalid".into())
            .unwrap(),
        (next.clone(), true)
    );
}
