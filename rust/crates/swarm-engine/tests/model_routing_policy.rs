//! Routing restrictions are saved operator choices, never execution permission.
use swarm_engine::{EngineSettingsCatalog, EngineSettingsValues};

/// Decode complete supplied values through the authoritative public boundary.
fn decode(
    raw: &serde_json::Value,
) -> Result<EngineSettingsValues, swarm_engine::EngineSettingsError> {
    EngineSettingsValues::decode(&serde_json::to_vec(raw).unwrap())
}
/// Derive a historical v18 shape by excluding later catalogue members only.
fn legacy_values() -> serde_json::Value {
    values_for_version(18)
}
/// Retain a historical policy generation without implicitly adopting future groups.
/// The authoritative decoder still verifies that the derived shape matches it.
fn values_for_version(version: i64) -> serde_json::Value {
    let mut value =
        serde_json::to_value(EngineSettingsValues::installation_defaults().unwrap()).unwrap();
    for field in EngineSettingsCatalog::installation().unwrap().fields {
        if field.introduced_schema_version > version {
            value.as_object_mut().unwrap().remove(&field.name);
        }
    }
    value
}
/// The three literal operators and explicit allowed sets preserve case and do
/// not turn a name match into a provider capability or actual model observation.
#[test]
fn saved_model_policy_matches_only_explicit_rules() {
    let mut raw =
        serde_json::to_value(EngineSettingsValues::installation_defaults().unwrap()).unwrap();
    raw["agent_model_automatic_levels"] = serde_json::json!(["standard"]);
    raw["agent_model_restricted_levels"] = serde_json::json!(["exigeant"]);
    raw["agent_model_restriction_rules"] = serde_json::json!([
        "exact:fixture-exact",
        "prefix:fixture-prefix/",
        "contains:fixture-fragment"
    ]);
    let values = decode(&raw).unwrap();
    let policy = values.agent_model_routing_policy().unwrap();
    for name in [
        "fixture-exact",
        "fixture-prefix/child",
        "x-fixture-fragment-y",
    ] {
        assert!(policy.check_selection("exigeant", name, false).is_ok());
        assert_eq!(
            policy
                .check_selection("standard", name, false)
                .unwrap_err()
                .reason(),
            "settings-model-restricted-level-denied"
        );
    }
    for name in [
        "fixture-exact-suffix",
        "Fixture-exact",
        "x-fixture-prefix/child",
        "unflagged",
    ] {
        assert!(policy.check_selection("standard", name, true).is_ok());
    }
    assert!(policy
        .check_selection("exigeant", "unflagged", false)
        .is_ok());
    assert_eq!(
        policy
            .check_selection("exigeant", "unflagged", true)
            .unwrap_err()
            .reason(),
        "settings-model-automatic-level-denied"
    );
    for (level, name) in [
        ("unknown", "fixture"),
        ("simple", ""),
        ("standard", "private\u{202e}text"),
    ] {
        assert!(policy
            .check_selection("standard", "unflagged", true)
            .is_ok());
        assert_eq!(
            policy
                .check_selection(level, name, false)
                .unwrap_err()
                .reason(),
            "settings-model-selection-invalid"
        );
    }
}
/// Empty configured collections retain their distinct deny/no-rule meaning;
/// reopening a value never manufactures a default level or classification.
#[test]
fn saved_model_policy_empty_sets_have_no_fallback() {
    let mut raw =
        serde_json::to_value(EngineSettingsValues::installation_defaults().unwrap()).unwrap();
    raw["agent_model_automatic_levels"] = serde_json::json!([]);
    raw["agent_model_restricted_levels"] = serde_json::json!([]);
    raw["agent_model_restriction_rules"] = serde_json::json!(["exact:fixture-sensitive"]);
    let values = decode(&raw).unwrap();
    let policy = values.agent_model_routing_policy().unwrap();
    assert!(policy.check_selection("simple", "ordinary", false).is_ok());
    for level in ["simple", "standard", "exigeant"] {
        assert!(policy.check_selection(level, "ordinary", true).is_err());
        assert!(policy
            .check_selection(level, "fixture-sensitive", false)
            .is_err());
    }
    raw["agent_model_restriction_rules"] = serde_json::json!([]);
    let policy = decode(&raw).unwrap().agent_model_routing_policy().unwrap();
    assert!(policy
        .check_selection("simple", "fixture-sensitive", false)
        .is_ok());
    assert!(policy
        .check_selection("simple", "fixture-sensitive", true)
        .is_err());
}
/// Every invalid document follows a valid control and leaves that input intact;
/// null, duplicates, unknown syntax and partial groups never become empty success.
#[test]
fn saved_model_policy_malformed_and_partial_groups_refuse_then_recover() {
    let original =
        serde_json::to_value(EngineSettingsValues::installation_defaults().unwrap()).unwrap();
    for field in [
        "agent_model_automatic_levels",
        "agent_model_restricted_levels",
        "agent_model_restriction_rules",
    ] {
        for bad in [
            serde_json::Value::Null,
            serde_json::json!("invalid"),
            serde_json::json!([true]),
        ] {
            assert!(decode(&original).is_ok());
            let mut changed = original.clone();
            changed[field] = bad;
            assert!(decode(&changed).is_err(), "{field}");
            assert!(decode(&original).is_ok());
        }
        assert!(decode(&original).is_ok());
        let mut changed = original.clone();
        changed.as_object_mut().unwrap().remove(field);
        assert!(decode(&changed).is_err());
    }
    for (field, bad) in [
        (
            "agent_model_automatic_levels",
            serde_json::json!(["standard", "standard"]),
        ),
        (
            "agent_model_restricted_levels",
            serde_json::json!(["invalid"]),
        ),
        (
            "agent_model_restriction_rules",
            serde_json::json!(["contains:"]),
        ),
        (
            "agent_model_restriction_rules",
            serde_json::json!(["regex:private-payload"]),
        ),
        (
            "agent_model_restriction_rules",
            serde_json::json!(["exact:private\nmodel"]),
        ),
        (
            "agent_model_restriction_rules",
            serde_json::json!(["exact:a", "exact:a"]),
        ),
    ] {
        assert!(decode(&original).is_ok());
        let mut changed = original.clone();
        changed[field] = bad;
        let error = decode(&changed).unwrap_err();
        assert!(!error.to_string().contains("private"));
        assert!(decode(&original).is_ok());
    }
}
/// Historical documents remain exactly readable; absence supplies no restrictions
/// and requires an explicit new generation before a model-selection consumer.
#[test]
fn saved_model_policy_legacy_absence_requires_explicit_adoption() {
    let raw = legacy_values();
    let old = decode(&raw).unwrap();
    assert_eq!(old.schema_version(), 18);
    assert_eq!(
        old.agent_model_routing_policy().unwrap_err().reason(),
        "settings-model-routing-not-configured"
    );
    assert_eq!(serde_json::to_value(old).unwrap(), raw);
    let catalog = EngineSettingsCatalog::installation().unwrap();
    assert_eq!(catalog.schema_version, 4);
    assert_eq!(catalog.document_schema_version, 27);
    let fields: Vec<_> = catalog
        .fields
        .iter()
        .filter(|f| f.introduced_schema_version == 19)
        .collect();
    assert_eq!(fields.len(), 3);
    assert!(fields
        .iter()
        .all(|f| f.unit == swarm_engine::SettingUnit::Strings && !f.nullable));
}
/// Publish historical fixture bytes atomically after retaining the original.
/// Uses an owned, inactive temporary root only. Creation/write/sync/rename errors
/// fail the test and leave retained data for diagnosis; no live store is patched.
#[cfg(unix)]
fn publish_fixture_settings(root: &std::path::Path, raw: &[u8]) {
    use std::{fs, io::Write, os::unix::fs::OpenOptionsExt};
    let directory = root.join(".swarm");
    let current = directory.join("engine-settings.json");
    let original = fs::read(&current).unwrap();
    let mut retained = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(directory.join("fixture-original19.json"))
        .unwrap();
    retained.write_all(&original).unwrap();
    retained.sync_all().unwrap();
    drop(retained);
    let temporary = directory.join("fixture-historical18.tmp");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .unwrap();
    file.write_all(raw).unwrap();
    file.sync_all().unwrap();
    drop(file);
    assert_eq!(fs::read(&temporary).unwrap(), raw);
    fs::rename(&temporary, &current).unwrap();
    fs::File::open(&directory).unwrap().sync_all().unwrap();
}
/// Public preview/apply and reopen change the actual policy consumer while
/// preserving old history. All disk writes are isolated fixture setup or public
/// RootStore operations; no live mission, provider or server is modified.
#[cfg(unix)]
#[test]
fn saved_model_policy_public_apply_reopen_preserves_history_and_effect() {
    use std::{fs, os::unix::fs::DirBuilderExt};
    use swarm_engine::{
        EngineSettingsChange, EngineSettingsDocument, RootStore, SystemEnvironment,
    };
    let root =
        std::env::temp_dir().join(swarm_engine::generate_identifier("model-policy81-").unwrap());
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    RootStore::initialize(&root, &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    let old = EngineSettingsDocument::initial(
        decode(&legacy_values()).unwrap(),
        "fixture".into(),
        "2026-10-19T00:00:00Z".into(),
    )
    .unwrap();
    publish_fixture_settings(&root, &swarm_engine::encode_model(&old).unwrap());
    let store = RootStore::open(&root, true).unwrap();
    assert_eq!(store.settings().unwrap(), old);
    let mut values = decode(&values_for_version(19)).unwrap();
    assert!(values
        .agent_model_routing_policy()
        .unwrap()
        .check_selection("exigeant", "ordinary", true)
        .is_err());
    values.agent_model_automatic_levels = Some(vec!["exigeant".into()]);
    values.agent_model_restricted_levels = Some(vec!["simple".into()]);
    values.agent_model_restriction_rules = Some(vec!["exact:fixture-sensitive".into()]);
    let mut change = EngineSettingsChange {
        schema_version: 19,
        event_id: "model-policy-adopt".into(),
        expected_revision: old.revision,
        values,
        reason: "Explicit isolated model policy adoption".into(),
        preview_token: String::new(),
    };
    let before = fs::read(root.join(".swarm/engine-settings.json")).unwrap();
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    assert_eq!(
        fs::read(root.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    store.close().unwrap();
    let reopened = RootStore::open(&root, false).unwrap();
    let actual = reopened.settings().unwrap();
    assert_eq!(actual.schema_version, 19);
    assert_eq!(actual.history[0], old.history[0]);
    assert_eq!(actual.values, change.values);
    let policy = actual.values.agent_model_routing_policy().unwrap();
    assert!(policy.check_selection("exigeant", "ordinary", true).is_ok());
    assert!(policy
        .check_selection("simple", "fixture-sensitive", false)
        .is_ok());
    assert!(policy
        .check_selection("standard", "fixture-sensitive", false)
        .is_err());
    reopened.close().unwrap();
    // Retain fixture history and data for replay/diagnosis instead of unchecked cleanup.
}
