//! Opaque retained constraints must not become an implicit execution permit.
use swarm_engine::{
    decode_agent, decode_agent_execution_snapshot, encode_model, AgentExecutionPhase,
    AgentExecutionSnapshot, EngineSettingsValues, SettingCount,
};
/// A complete local data witness, with no identity/permission or kernel proof.
fn valid() -> AgentExecutionSnapshot {
    AgentExecutionSnapshot {
        schema_version: 1,
        settings_revision: SettingCount(2),
        work_revision: 3,
        context_sha256: "a".repeat(64),
        phase: AgentExecutionPhase::AwaitingPermission,
        policy: EngineSettingsValues::installation_defaults()
            .unwrap()
            .agent_execution_policy()
            .unwrap(),
    }
}
/// Opaque Agent storage retains future constraints through observation/encoding;
/// the current execution consumer refuses them. Legacy absence stays absent.
#[test]
fn execution_metadata_is_opaque_preserved_and_unknown_constraints_refuse() {
    assert!(decode_agent(b"{}").unwrap().native_execution.is_none());
    let witness = serde_json::to_value(valid()).unwrap();
    assert!(decode_agent_execution_snapshot(&serde_json::to_vec(&witness).unwrap()).is_ok());
    let mut future = witness.clone();
    future["future_execution_constraint"] = serde_json::json!({"must_validate":true});
    let agent =
        decode_agent(&serde_json::to_vec(&serde_json::json!({"native_execution":future})).unwrap())
            .unwrap();
    let raw = agent.native_execution.as_ref().unwrap().0.get();
    assert!(decode_agent_execution_snapshot(raw.as_bytes()).is_err());
    let round = decode_agent(&encode_model(&agent).unwrap()).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(round.native_execution.unwrap().0.get()).unwrap(),
        future
    );
    assert!(decode_agent_execution_snapshot(b"{}").is_err());
}
/// Schema, exact revisions/hash, phase and every nested nullable member are
/// required. Valid data precedes each malformed case and recovers afterwards;
/// errors render safe static families without echoing supplied private content.
#[test]
fn execution_metadata_requires_complete_canonical_supported_data() {
    let witness = serde_json::to_value(valid()).unwrap();
    let mut cases = vec![];
    for key in [
        "schema_version",
        "settings_revision",
        "work_revision",
        "context_sha256",
        "phase",
        "policy",
    ] {
        let mut missing = witness.clone();
        missing.as_object_mut().unwrap().remove(key);
        cases.push(missing);
    }
    for (key, bad) in [
        ("schema_version", serde_json::json!(2)),
        ("settings_revision", serde_json::json!("0")),
        ("work_revision", serde_json::json!(0)),
        ("context_sha256", serde_json::json!("PRIVATE")),
        ("phase", serde_json::json!("PRIVATE")),
        ("policy", serde_json::Value::Null),
    ] {
        let mut changed = witness.clone();
        changed[key] = bad;
        cases.push(changed);
    }
    let mut nested = witness.clone();
    nested["policy"]
        .as_object_mut()
        .unwrap()
        .remove("output_drain_ms");
    cases.push(nested);
    for bad in cases {
        assert!(decode_agent_execution_snapshot(&serde_json::to_vec(&witness).unwrap()).is_ok());
        let error =
            decode_agent_execution_snapshot(&serde_json::to_vec(&bad).unwrap()).unwrap_err();
        assert!(!format!("{error:?} {error}").contains("PRIVATE"));
        assert!(decode_agent_execution_snapshot(&serde_json::to_vec(&witness).unwrap()).is_ok());
    }
    let raw = serde_json::to_string(&valid()).unwrap();
    let duplicate = raw.replacen("{", "{\"schema_version\":1,", 1);
    assert!(decode_agent_execution_snapshot(duplicate.as_bytes()).is_err());
}
/// Future constraints remain flat RawValue spans rather than recursive Value
/// trees. Observation can retain them, while execution parsing refuses safely.
#[test]
fn execution_metadata_deep_future_values_remain_opaque_without_activation() {
    let base = serde_json::to_string(&valid()).unwrap();
    let nested = format!("{}null{}", "[".repeat(512), "]".repeat(512));
    let future = format!("{},\"future\":{nested}}}", &base[..base.len() - 1]);
    assert!(decode_agent_execution_snapshot(base.as_bytes()).is_ok());
    let raw = format!("{{\"native_execution\":{future}}}");
    let agent = decode_agent(raw.as_bytes()).unwrap();
    assert!(decode_agent_execution_snapshot(
        agent.native_execution.as_ref().unwrap().0.get().as_bytes()
    )
    .is_err());
    let retained = decode_agent(&encode_model(&agent).unwrap()).unwrap();
    assert_eq!(retained.native_execution.unwrap().0.get(), future);
}
