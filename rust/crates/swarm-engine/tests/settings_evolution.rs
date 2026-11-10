use std::io::Write;
use swarm_engine::{
    EngineSettingsChange, EngineSettingsDocument, SettingCap, SettingCount, ValidationOutput,
};
const OLD: &[u8] = include_bytes!("fixtures/legacy-v1-document.json");
const REPLAY: &[u8] = include_bytes!("fixtures/legacy-v1-request.json");

#[test]
fn v1_exact_history_and_replay_survive_explicit_upgrade_without_policy_fallback() {
    let old = EngineSettingsDocument::decode(OLD).unwrap();
    assert_eq!(swarm_wire_json::encode(&old).unwrap(), OLD);
    assert_eq!(old.schema_version, 1);
    assert_eq!(
        old.values.validation_output_limit().unwrap_err().reason(),
        "settings-output-policy-not-configured"
    );
    assert!(ValidationOutput::from_settings(&old.values).is_err());
    let replay: EngineSettingsChange = serde_json::from_slice(REPLAY).unwrap();
    assert!(old.replay(&replay).unwrap());
    let mut upgrade = EngineSettingsChange {
        schema_version: 2,
        event_id: "upgrade-retention".into(),
        expected_revision: old.revision,
        values: old.values.clone(),
        reason: "Operator adopts explicit output retention".into(),
        preview_token: String::new(),
    };
    // A version label does not invent the missing field.
    assert!(old.preview(&upgrade).is_err());
    upgrade.values.validation_max_output_bytes = Some(SettingCap(Some(SettingCount(7))));
    let original = swarm_wire_json::encode(&old).unwrap();
    let preview = old.preview(&upgrade).unwrap();
    assert_eq!(swarm_wire_json::encode(&old).unwrap(), original);
    assert!(old
        .apply(&upgrade, "operator".into(), "2026-10-18T01:00:00Z".into())
        .is_err());
    upgrade.preview_token = preview.preview_token;
    let (new, was_replay) = old
        .apply(&upgrade, "operator".into(), "2026-10-18T01:00:00Z".into())
        .unwrap();
    assert!(!was_replay);
    assert_eq!(new.schema_version, 2);
    assert_eq!(&new.history[..old.history.len()], &old.history);
    let reopened = EngineSettingsDocument::decode(&swarm_wire_json::encode(&new).unwrap()).unwrap();
    assert_eq!(reopened, new);
    assert_eq!(reopened.values.validation_output_limit().unwrap(), Some(7));
    assert_eq!(
        reopened
            .apply(&replay, "".into(), "invalid".into())
            .unwrap(),
        (new.clone(), true)
    );
    assert_eq!(
        new.apply(&upgrade, "".into(), "invalid".into()).unwrap(),
        (new.clone(), true)
    );
    let mut downgrade = replay.clone();
    downgrade.event_id = "downgrade".into();
    downgrade.expected_revision = new.revision;
    assert_eq!(
        new.preview(&downgrade).unwrap_err().reason(),
        "settings-schema-downgrade"
    );
    let mut wrong = new.clone();
    wrong.schema_version = 1;
    assert!(wrong.validate().is_err());
    let mut invalid = upgrade.clone();
    invalid.schema_version = 1;
    assert!(old.preview(&invalid).is_err());
    assert_eq!(swarm_wire_json::encode(&old).unwrap(), OLD);
}

#[test]
fn explicit_null_zero_exact_bytes_and_chunking_change_the_retention_consumer() {
    let old = EngineSettingsDocument::decode(OLD).unwrap();
    let mut values = old.values;
    let input = b"abc\xF0\x9F\x92\xA1\xFFxyz";
    for (cap, expected) in [
        (Some(5), 5),
        (Some(0), 0),
        (None, input.len()),
        (Some(99), input.len()),
    ] {
        values.validation_max_output_bytes = Some(SettingCap(cap.map(SettingCount)));
        let decoded =
            swarm_engine::EngineSettingsValues::decode(&swarm_wire_json::encode(&values).unwrap())
                .unwrap();
        assert_eq!(decoded, values);
        let mut sink = ValidationOutput::from_settings(&decoded).unwrap();
        for chunk in input.chunks(3) {
            sink.write_all(chunk).unwrap();
        }
        sink.flush().unwrap();
        assert_eq!(sink.retained(), &input[..expected]);
        assert_eq!(sink.total(), input.len() as u64);
        assert_eq!(sink.truncated(), expected < input.len());
    }
    values.validation_max_output_bytes = Some(SettingCap(Some(SettingCount(-1))));
    assert!(values.validate().is_err());
    assert!(ValidationOutput::from_settings(&values).is_err());
}
