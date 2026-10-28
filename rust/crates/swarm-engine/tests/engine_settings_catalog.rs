use swarm_engine::{
    EngineSettingsCatalog, EngineSettingsChange, EngineSettingsDocument, SettingCount, SettingUnit,
};
/// Catalog v5 covers address/null/effect/transport metadata and all document
/// policies through schema42, including saved preparation editing policy.
/// Metadata covers actual fields/nullability without adopting policy or granting
/// authority; a preview preserves exact decimal counters.
#[test]
fn catalog_covers_all_actual_values_and_nullable_fields_match_authoritative_validation() {
    let catalog = EngineSettingsCatalog::installation().unwrap();
    assert_eq!(catalog.schema_version, 5);
    assert_eq!(catalog.document_schema_version, 43);
    assert!(
        catalog
            .supported_document_versions
            .contains(&catalog.document_schema_version),
        "The actual current Admin document must be declared readable"
    );
    for field in &catalog.fields {
        assert!(
            catalog
                .supported_document_versions
                .contains(&field.introduced_schema_version),
            "Admin must understand the schema introducing {}",
            field.name
        );
    }
    assert_eq!(catalog.model_policy_format.version, 1);
    assert_eq!(
        catalog.model_policy_format.model_name_max_bytes,
        SettingCount(160)
    );
    assert!(catalog.model_policy_format.grammar.contains("ASCII"));
    assert_eq!(
        catalog.input_transport.max_bytes,
        SettingCount(16 * 1024 * 1024)
    );
    assert!(
        catalog.input_transport.named_files_must_be_regular
            && catalog.input_transport.stdin_streams_supported
    );
    let doc = EngineSettingsDocument::initial(
        catalog.defaults.clone(),
        "fixture/uid:1".into(),
        "2026-10-18T00:00:00Z".into(),
    )
    .unwrap();
    let original = serde_json::to_value(&doc.values).unwrap();
    assert_eq!(catalog.fields.len(), original.as_object().unwrap().len());
    for field in &catalog.fields {
        assert!(!field.label.is_empty() && !field.help.is_empty());
        let mut values = original.clone();
        values[&field.name] = serde_json::Value::Null;
        let result =
            swarm_engine::EngineSettingsValues::decode(&serde_json::to_vec(&values).unwrap());
        assert_eq!(
            result.is_ok(),
            field.nullable,
            "{} nullable metadata mismatch",
            field.name
        );
        if field.unit == SettingUnit::Programs {
            assert!(!field.nullable);
        }
    }
    let mut values = catalog.defaults;
    values.task_max_attempts = Some(SettingCount(9007199254740993));
    let request = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "catalog-preview".into(),
        expected_revision: doc.revision,
        values,
        reason: "Configure exact integer".into(),
        preview_token: String::new(),
    };
    let preview = serde_json::to_value(doc.preview(&request).unwrap()).unwrap();
    assert_eq!(preview["values"]["task_max_attempts"], "9007199254740993");
}

/// A complete schema32 fixture excludes every later policy using authoritative
/// catalogue introduction versions. Removing only the v32 cost policy then
/// yields v31; explicit null/count adoption restores v32 and preserves exact
/// counts. Invalid signs/fractions refuse without introducing a fallback.
#[test]
fn reported_cost_policy_preserves_legacy_absence_null_and_exact_counts() {
    use swarm_engine::{EngineSettingsValues, SettingCap};
    let values = EngineSettingsValues::installation_defaults().unwrap();
    assert_eq!(values.schema_version(), 43);
    assert_eq!(
        values.mission_dispatch_cost_reserve_percent,
        Some(SettingCap(Some(SettingCount(200))))
    );
    let mut raw = serde_json::to_value(&values).unwrap();
    let catalog = EngineSettingsCatalog::installation().unwrap();
    for field in catalog
        .fields
        .iter()
        .filter(|field| field.introduced_schema_version > 32)
    {
        assert!(
            raw.as_object_mut().unwrap().remove(&field.name).is_some(),
            "Current defaults must contain later policy {}",
            field.name
        );
    }
    let schema32 = EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(schema32.schema_version(), 32);
    assert_eq!(
        schema32.mission_dispatch_cost_reserve_percent,
        values.mission_dispatch_cost_reserve_percent
    );
    raw.as_object_mut()
        .unwrap()
        .remove("mission_dispatch_cost_reserve_percent");
    let legacy = EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(legacy.schema_version(), 31);
    assert_eq!(legacy.mission_dispatch_cost_reserve_percent, None);
    for cap in [
        serde_json::Value::Null,
        serde_json::json!("0"),
        serde_json::json!("9007199254740993"),
        serde_json::json!("9223372036854775807"),
    ] {
        raw["mission_dispatch_cost_reserve_percent"] = cap.clone();
        let decoded = EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
        assert_eq!(decoded.schema_version(), 32);
        assert_eq!(
            serde_json::to_value(decoded).unwrap()["mission_dispatch_cost_reserve_percent"],
            cap
        );
    }
    for cap in [
        serde_json::json!("-1"),
        serde_json::json!("1.5"),
        serde_json::json!("9223372036854775808"),
    ] {
        raw["mission_dispatch_cost_reserve_percent"] = cap;
        assert!(EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_err());
    }
}
