//! Untrusted model JSON is data only; no SQL, process, actor or lease is accepted.
use swarm_engine::{decode_planning_proposal, EngineSettingsValues, PlanningDecisionPolicy};

/// Use actual persisted installation-policy validation rather than a test-only
/// policy default. Every fault is preceded by a valid observation under it.
fn policy() -> PlanningDecisionPolicy {
    EngineSettingsValues::installation_defaults()
        .unwrap()
        .planning_decision_policy()
        .unwrap()
}
const VALID: &[u8] = br#"{"input_events":["event-1"],"reason":"Observe","operations":[]}"#;
/// Require safe refusal without serializing the model's private fields/text.
fn refused(raw: &[u8], code: &str, p: &PlanningDecisionPolicy) {
    assert!(decode_planning_proposal(VALID, p).is_ok());
    let e = match decode_planning_proposal(raw, p) {
        Ok(_) => panic!("untrusted proposal accepted"),
        Err(e) => e,
    };
    assert_eq!(e.code(), code);
    assert!(!e.to_string().contains("PRIVATE"));
}
#[test]
fn proposal_complete_observation_preserves_data_without_authority() {
    let p = decode_planning_proposal(VALID, &policy()).unwrap();
    assert_eq!(p.input_events, ["event-1"]);
    assert_eq!(p.reason, "Observe");
    assert!(p.operations.is_empty());
    let raw=br#"{"input_events":["event-1"],"reason":"Propose","operations":[{"kind":"task","id":"new-task","title":"New task","requirements":["req-1"],"criteria":["Evidence"],"depends":[] }]}"#;
    let p = decode_planning_proposal(raw, &policy()).unwrap();
    assert_eq!(p.operations[0].kind, "task");
    assert_eq!(p.operations[0].id, "new-task");
}
#[test]
fn proposal_model_cannot_supply_engine_authority_fields() {
    for key in [
        "event_id",
        "schema_version",
        "expected_revision",
        "scope",
        "scope_revision",
        "holder",
        "generation",
        "actor",
        "at",
        "lease_seconds",
        "unknown",
    ] {
        let mut v: serde_json::Value = serde_json::from_slice(VALID).unwrap();
        v[key] = "PRIVATE".into();
        refused(
            &serde_json::to_vec(&v).unwrap(),
            "planning-proposal-format",
            &policy(),
        );
    }
}
#[test]
fn proposal_duplicate_keys_trailing_and_non_json_refuse() {
    for raw in [br#"{"input_events":["event-1"],"input_events":[],"reason":"PRIVATE","operations":[]}"#.as_slice(),br#"{"input_events":["event-1"],"reason":"Observe","operations":[],"reas\u006fn":"PRIVATE"}"#,br#"{"input_events":["event-1"],"reason":"Observe","operations":[]} {}"#,b"```json\nPRIVATE\n```",b"\xff"] { refused(raw,"planning-proposal-format",&policy()); }
}
#[test]
fn proposal_missing_null_and_wrong_types_refuse() {
    for key in ["input_events", "reason", "operations"] {
        let mut v: serde_json::Value = serde_json::from_slice(VALID).unwrap();
        v.as_object_mut().unwrap().remove(key);
        refused(
            &serde_json::to_vec(&v).unwrap(),
            "planning-proposal-format",
            &policy(),
        );
        v = serde_json::from_slice(VALID).unwrap();
        v[key] = serde_json::Value::Null;
        refused(
            &serde_json::to_vec(&v).unwrap(),
            "planning-proposal-format",
            &policy(),
        );
        v = serde_json::from_slice(VALID).unwrap();
        v[key] = 7.into();
        refused(
            &serde_json::to_vec(&v).unwrap(),
            "planning-proposal-format",
            &policy(),
        );
    }
}
#[test]
fn proposal_nested_operation_unknown_and_null_fields_refuse() {
    for op in [
        r#"{"kind":"task","id":"new-task","PRIVATE":"value"}"#,
        r#"{"kind":"task","kind":"PRIVATE"}"#,
        r#"{"kind":null}"#,
        r#"{"kind":"task","title":null}"#,
        r#"{"kind":"task","max_tasks":1.5}"#,
    ] {
        let raw =
            format!(r#"{{"input_events":["event-1"],"reason":"Observe","operations":[{op}]}}"#);
        refused(raw.as_bytes(), "planning-proposal-format", &policy());
    }
}
#[test]
fn proposal_event_names_duplicates_and_empty_reason_refuse() {
    for (raw, code) in [
        (
            br#"{"input_events":[],"reason":"Observe","operations":[]}"#.as_slice(),
            "planning-proposal-inputs-required",
        ),
        (
            br#"{"input_events":["event-1","event-1"],"reason":"Observe","operations":[]}"#,
            "planning-proposal-input-invalid",
        ),
        (
            br#"{"input_events":["../PRIVATE"],"reason":"Observe","operations":[]}"#,
            "planning-proposal-input-invalid",
        ),
        (
            br#"{"input_events":["event-1"],"reason":" \n\t ","operations":[]}"#,
            "planning-proposal-reason-required",
        ),
    ] {
        refused(raw, code, &policy());
    }
}
#[test]
fn proposal_saved_limits_count_reason_bytes_and_operations() {
    let mut p = policy();
    p.max_inputs = Some(1);
    refused(
        br#"{"input_events":["event-1","event-2"],"reason":"Observe","operations":[]}"#,
        "planning-decision-input-limit",
        &p,
    );
    p = policy();
    p.max_reason_bytes = Some(7);
    refused(
        "{\"input_events\":[\"event-1\"],\"reason\":\"éééé\",\"operations\":[]}".as_bytes(),
        "planning-decision-reason-limit",
        &p,
    );
    p = policy();
    p.max_operations = Some(0);
    refused(
        br#"{"input_events":["event-1"],"reason":"Observe","operations":[{"kind":"task"}]}"#,
        "planning-decision-operation-limit",
        &p,
    );
}
#[test]
fn proposal_unlimited_removes_only_configured_caps() {
    let mut p = policy();
    p.max_reason_bytes = None;
    p.max_inputs = None;
    p.max_operations = None;
    let mut value: serde_json::Value = serde_json::from_slice(VALID).unwrap();
    value["reason"] = "é".repeat(5000).into();
    assert!(decode_planning_proposal(&serde_json::to_vec(&value).unwrap(), &p).is_ok());
    refused(
        br#"{"input_events":[],"reason":"PRIVATE","operations":[]}"#,
        "planning-proposal-inputs-required",
        &p,
    );
}

/// Zero input/reason ceilings make every structurally valid response impossible;
/// witness uses the original saved policy before the explicit policy change.
#[test]
fn proposal_zero_input_or_reason_cap_never_becomes_unlimited() {
    let mut p = policy();
    assert!(decode_planning_proposal(VALID, &p).is_ok());
    p.max_inputs = Some(0);
    let e = decode_planning_proposal(VALID, &p).err().unwrap();
    assert_eq!(e.code(), "planning-decision-input-limit");
    p = policy();
    assert!(decode_planning_proposal(VALID, &p).is_ok());
    p.max_reason_bytes = Some(0);
    let e = decode_planning_proposal(VALID, &p).err().unwrap();
    assert_eq!(e.code(), "planning-decision-reason-limit");
    p.max_reason_bytes = None;
    assert!(decode_planning_proposal(VALID, &p).is_ok());
}
