//! Strict declared-event replay; no provider, OS tools, identity or acceptance.
use swarm_engine::{decode_agent, decode_agent_guard_trace, replay_agent_guard, AgentGuardStop};
/// Explicit retained fixture limits/snapshot, never installation/default policy.
fn agent() -> swarm_engine::Agent {
    decode_agent(br#"{"limits":{"silence_seconds":1,"tool_seconds":1,"max_tool_calls":20,"max_repeated_calls":4,"max_consecutive_errors":1},"native_tool_guard_policy":{"schema_version":1,"settings_revision":"7","lookback_multiplier":"4"}}"#).unwrap()
}
/// Error wording is untrusted data: ordinary and 'technical' failures count the
/// same, and safe output never leaks arguments/messages or claims execution.
#[test]
fn guard_trace_uses_frozen_values_and_counts_technical_wording() {
    for message in ["ordinary failure", "PRIVATE permission denied"] {
        let raw = format!(
            r#"{{"schema_version":1,"events":[{{"call":{{"at_ms":0,"id":"A","name":"Read","input":{{"PRIVATE":true}}}}}},{{"result":{{"at_ms":1,"id":"A","failed":true,"message":"{message}"}}}}],"final_at_ms":1}}"#
        );
        let trace = decode_agent_guard_trace(raw.as_bytes()).unwrap();
        let report = replay_agent_guard(&agent(), &trace).unwrap();
        assert_eq!(report.observation.total_errors, 1);
        assert_eq!(
            report.observation.stop_reason,
            Some(AgentGuardStop::ConsecutiveErrors)
        );
        assert_eq!(report.settings_revision, "7");
        assert!(report.preview_only);
        assert!(!report.process_started);
        let output = String::from_utf8(serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(!output.contains("PRIVATE"));
        assert!(!output.contains(message));
        assert_eq!(report.event_source, "caller_declared_unverified");
    }
}
/// Valid control precedes missing/unknown/wrong-type/duplicate field injections.
/// A result without a failed flag cannot silently become a successful result.
#[test]
fn guard_trace_requires_complete_unambiguous_events() {
    assert!(
        decode_agent_guard_trace(br#"{"schema_version":1,"events":[],"final_at_ms":0}"#).is_ok()
    );
    for raw in [
        r#"{"schema_version":1,"events":[]}"#,
        r#"{"schema_version":1,"events":[],"final_at_ms":0,"unknown":1}"#,
        r#"{"schema_version":1,"events":[],"final_at_ms":0,"final_at_ms":1}"#,
        r#"{"schema_version":1,"events":[{"output":{}}],"final_at_ms":0}"#,
        r#"{"schema_version":1,"events":[{"output":{"at_ms":0,"secret":1}}],"final_at_ms":0}"#,
        r#"{"schema_version":1,"events":[{"result":{"at_ms":0,"id":"A"}}],"final_at_ms":0}"#,
        r#"{"schema_version":1,"events":[{"call":{"at_ms":0,"id":"A","name":"Read"}}],"final_at_ms":0}"#,
        r#"{"schema_version":1,"events":[{"output":{"at_ms":"0"}}],"final_at_ms":0}"#,
    ] {
        assert_eq!(
            decode_agent_guard_trace(raw.as_bytes())
                .err()
                .expect(raw)
                .code(),
            "agent-guard-trace-invalid"
        );
    }
}
/// Native opaque input remains safe beyond recursive Value depth. Consumption
/// rejects attribution/clock/schema/legacy absence without partial report.
#[test]
fn guard_trace_deep_input_and_invalid_replay_refuse_safely() {
    let input = format!("{}null{}", "[".repeat(512), "]".repeat(512));
    let raw = format!(
        r#"{{"schema_version":1,"events":[{{"call":{{"at_ms":0,"id":"A","name":"Read","input":{input}}}}}],"final_at_ms":1001}}"#
    );
    let trace = decode_agent_guard_trace(raw.as_bytes()).unwrap();
    let report = replay_agent_guard(&agent(), &trace).unwrap();
    assert_eq!(report.observation.pending, 1);
    assert!(report.observation.tool_time_threshold_exceeded);
    assert_eq!(report.observation.stop_reason, None);
    for raw in [
        r#"{"schema_version":2,"events":[],"final_at_ms":0}"#,
        r#"{"schema_version":1,"events":[],"final_at_ms":-1}"#,
        r#"{"schema_version":1,"events":[{"output":{"at_ms":1}}],"final_at_ms":0}"#,
        r#"{"schema_version":1,"events":[{"result":{"at_ms":0,"id":"PRIVATE","failed":false}}],"final_at_ms":0}"#,
    ] {
        let trace = decode_agent_guard_trace(raw.as_bytes()).unwrap();
        assert!(replay_agent_guard(&agent(), &trace).is_err(), "{raw}");
    }
    let mut legacy = agent();
    legacy.native_tool_guard_policy = None;
    assert_eq!(
        replay_agent_guard(&legacy, &trace).err().unwrap().code(),
        "agent-guard-snapshot-missing"
    );
}
