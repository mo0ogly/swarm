//! Structured output framing is separate from execution or decision authority.
use std::io::Write;
use swarm_engine::{EngineSettingsValues, PlanningOutputStream, SettingCap, SettingCount};
const PROPOSAL: &str = r#"{"input_events":["event-1"],"reason":"Observe","operations":[]}"#;
/// Real validated current defaults; no private stream policy fallback.
fn values() -> EngineSettingsValues {
    EngineSettingsValues::installation_defaults().unwrap()
}
/// Encode only fixture transport, preserving the proposal's original JSON text.
fn codex(text: &str) -> Vec<u8> {
    format!("{}\n{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":7,\"output_tokens\":8}}}}\n",serde_json::json!({"type":"item.completed","item":{"type":"agent_message","id":"answer","text":text}})).into_bytes()
}
/// Positive control before malformed framing under the same actual policy.
fn healthy(v: &EngineSettingsValues) {
    let mut s = PlanningOutputStream::new(v).unwrap();
    s.write_all(&codex(PROPOSAL)).unwrap();
    assert_eq!(s.finish().unwrap().proposal.reason, "Observe");
}
#[test]
fn planning_output_codex_chunks_and_missing_newline_preserve_complete_proposal() {
    let v = values();
    let raw = codex(PROPOSAL);
    for size in [1, 7, raw.len()] {
        healthy(&v);
        let mut s = PlanningOutputStream::new(&v).unwrap();
        for chunk in raw.trim_ascii_end().chunks(size) {
            s.write_all(chunk).unwrap();
        }
        let out = s.finish().unwrap();
        assert_eq!(out.proposal.input_events, ["event-1"]);
        assert_eq!(out.usage.unwrap().input_tokens, 7);
        assert!(s.finish().is_err());
    }
}
#[test]
fn planning_output_claude_raw_structured_proposal_keeps_duplicate_refusal() {
    let v = values();
    healthy(&v);
    let mut s = PlanningOutputStream::new(&v).unwrap();
    s.write_all(
        format!(r#"{{"type":"result","subtype":"success","structured_output":{PROPOSAL}}}"#)
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(s.finish().unwrap().proposal.reason, "Observe");
    healthy(&v);
    let mut s = PlanningOutputStream::new(&v).unwrap();
    s.write_all(br#"{"type":"result","structured_output":{"input_events":["event-1"],"reason":"Observe","reason":"PRIVATE","operations":[]}}
"#).unwrap();
    let e = s.finish().err().unwrap();
    assert_eq!(e.code(), "planning-proposal-format");
    assert!(!e.to_string().contains("PRIVATE"));
}
#[test]
fn planning_output_eof_without_terminal_never_becomes_success() {
    let v = values();
    healthy(&v);
    let raw = codex(PROPOSAL);
    let line = raw.split(|b| *b == b'\n').next().unwrap();
    let mut s = PlanningOutputStream::new(&v).unwrap();
    s.write_all(line).unwrap();
    assert_eq!(
        s.finish().err().unwrap().code(),
        "planning-output-terminal-required"
    );
}
#[test]
fn planning_output_failed_terminal_and_trailing_events_refuse() {
    let v = values();
    for raw in [
        b"{\"type\":\"turn.failed\",\"error\":{\"message\":\"PRIVATE\"}}\n".as_slice(),
        b"{\"type\":\"result\",\"is_error\":true}\n",
        b"{\"type\":\"result\",\"subtype\":\"error_max_turns\"}\n",
    ] {
        healthy(&v);
        let mut s = PlanningOutputStream::new(&v).unwrap();
        let e = s.write_all(raw).err().unwrap();
        assert!(!e.to_string().contains("PRIVATE"));
        assert!(s.finish().is_err());
    }
    healthy(&v);
    let mut s = PlanningOutputStream::new(&v).unwrap();
    s.write_all(&codex(PROPOSAL)).unwrap();
    assert!(s.write_all(b"{\"type\":\"turn.completed\"}\n").is_err());
    assert!(s.finish().is_err());
}
#[test]
fn planning_output_malformed_unknown_and_duplicate_wire_data_do_not_grant_reply() {
    let v = values();
    for raw in [
        b"PRIVATE not json\n".as_slice(),
        b"{\"type\":\"result\",\"type\":\"turn.completed\"}\n",
        b"{\"type\":7}\n",
        b"{\"type\":\"assistant\",\"message\":null}\n",
        b"{\"type\":\"result\",\"structured_output\":\"PRIVATE\"}\n",
    ] {
        healthy(&v);
        let mut s = PlanningOutputStream::new(&v).unwrap();
        let failure = s.write_all(raw).err();
        let final_result = s.finish();
        assert!(failure.is_some() || final_result.is_err());
        if let Some(e) = failure {
            assert!(!e.to_string().contains("PRIVATE"));
        }
    }
    healthy(&v);
    let mut s = PlanningOutputStream::new(&v).unwrap();
    s.write_all(b"{\"type\":\"unknown\",\"structured_output\":{}}\n")
        .unwrap();
    assert!(s.finish().is_err());
}
#[test]
fn planning_output_saved_reply_caps_refuse_without_truncated_proposal() {
    let mut v = values();
    healthy(&v);
    v.planning_response_max_bytes = Some(SettingCap(Some(SettingCount(0))));
    let mut s = PlanningOutputStream::new(&v).unwrap();
    assert!(s.write_all(&codex(PROPOSAL)).is_err());
    assert!(s.finish().is_err());
    v.planning_response_max_bytes = Some(SettingCap(Some(SettingCount(PROPOSAL.len() as i64))));
    healthy(&v);
    v.planning_response_max_bytes = Some(SettingCap(Some(SettingCount(PROPOSAL.len() as i64 - 1))));
    let mut s = PlanningOutputStream::new(&v).unwrap();
    assert!(s.write_all(&codex(PROPOSAL)).is_err());
    assert!(s.finish().is_err());
    v.planning_response_max_bytes = Some(SettingCap(None));
    healthy(&v);
}
#[test]
fn planning_output_event_cap_and_partial_json_fail_closed() {
    let mut v = values();
    healthy(&v);
    v.agent_provider_event_max_bytes = Some(SettingCap(Some(SettingCount(8))));
    let mut s = PlanningOutputStream::new(&v).unwrap();
    assert!(s.write_all(&codex(PROPOSAL)).is_err());
    assert!(s.finish().is_err());
    v = values();
    healthy(&v);
    let mut s = PlanningOutputStream::new(&v).unwrap();
    s.write_all(b"{\"type\":\"result\"").unwrap();
    assert!(s.finish().is_err());
}
#[test]
fn planning_output_missing_usage_stays_unknown_and_legacy_policy_is_not_adopted() {
    let v = values();
    healthy(&v);
    let mut s = PlanningOutputStream::new(&v).unwrap();
    s.write_all(format!(r#"{{"type":"result","structured_output":{PROPOSAL}}}"#).as_bytes())
        .unwrap();
    let out = s.finish().unwrap();
    assert!(out.usage.is_none());
    assert!(out
        .provider_facts
        .0
        .get()
        .contains("provider_declared_unverified"));
    let old = swarm_engine::EngineSettingsDocument::decode(include_bytes!(
        "fixtures/legacy-v25-document.json"
    ))
    .unwrap();
    assert_eq!(
        PlanningOutputStream::new(&old.values).err().unwrap().code(),
        "settings-planning-response-not-configured"
    );
}

/// Wire compatibility must not replace invalid UTF8 inside a quoted response,
/// turning a malformed model protocol into a valid inner JSON proposal.
#[test]
fn planning_output_invalid_utf8_refuses_instead_of_replacing_bytes() {
    let v = values();
    healthy(&v);
    let text = PROPOSAL.replace("Observe", "bad X");
    let mut raw = serde_json::to_vec(&serde_json::json!({"type":"result","result":text})).unwrap();
    raw.push(b'\n');
    let mut s = PlanningOutputStream::new(&v).unwrap();
    s.feed(&raw).unwrap();
    assert_eq!(s.finish().unwrap().proposal.reason, "bad X");
    let pos = raw.windows(5).position(|v| v == b"bad X").unwrap();
    raw[pos + 4] = 0xff;
    let mut s = PlanningOutputStream::new(&v).unwrap();
    let e = s.feed(&raw).err().unwrap();
    assert_eq!(e.code(), "planning-output-utf8");
    assert!(s.finish().is_err());
    let raw = codex(&PROPOSAL.replace("Observe", "é"));
    let mut s = PlanningOutputStream::new(&v).unwrap();
    for chunk in raw.chunks(1) {
        s.feed(chunk).unwrap();
    }
    assert_eq!(s.finish().unwrap().proposal.reason, "é");
}
/// A terminal, syntactically valid provider reply is not a PlanningProposal.
/// Shared framing retains exact bytes and unverified metadata for the caller.
#[test]
fn provider_response_nonplanning_domain_preserves_exact_terminal_bytes() {
    let v = values();
    healthy(&v);
    let reply = r#"{"expected_revision":"9007199254740993","explanation":"Fixture advice","changes":{"task_max_attempts":"4"}}"#;
    let mut stream = swarm_engine::ProviderResponseStream::new(&v, Some(reply.len())).unwrap();
    for chunk in codex(reply).chunks(7) {
        stream.feed(chunk).unwrap()
    }
    let output = stream.finish().unwrap();
    assert_eq!(output.response, reply.as_bytes());
    assert_eq!(output.usage.unwrap().input_tokens, 7);
    assert!(stream.finish().is_err());
    let mut planning = PlanningOutputStream::new(&v).unwrap();
    planning.feed(&codex(reply)).unwrap();
    assert!(planning.finish().is_err());
}
/// Response and event caps are separate actual consumers. A domain cap zero
/// does not silently clamp another caller's explicitly supplied response limit.
#[test]
fn provider_response_explicit_cap_and_saved_event_policy_are_distinct() {
    let mut v = values();
    v.planning_response_max_bytes = Some(SettingCap(Some(SettingCount(0))));
    let reply = r#"{"fixture":"complete"}"#;
    let mut stream = swarm_engine::ProviderResponseStream::new(&v, Some(reply.len())).unwrap();
    stream.feed(&codex(reply)).unwrap();
    assert_eq!(stream.finish().unwrap().response, reply.as_bytes());
    let mut too_small =
        swarm_engine::ProviderResponseStream::new(&v, Some(reply.len() - 1)).unwrap();
    assert!(too_small.feed(&codex(reply)).is_err());
    assert!(too_small.finish().is_err());
    let mut unlimited = swarm_engine::ProviderResponseStream::new(&v, None).unwrap();
    unlimited.feed(&codex(reply)).unwrap();
    assert_eq!(unlimited.finish().unwrap().response, reply.as_bytes());
    v.agent_provider_event_max_bytes = Some(SettingCap(Some(SettingCount(1))));
    let mut event_limited = swarm_engine::ProviderResponseStream::new(&v, None).unwrap();
    assert!(event_limited.feed(&codex(reply)).is_err());
    assert!(event_limited.finish().is_err());
}
/// Valid terminal witness precedes partial/failing/ambiguous transport. A raw
/// response does not validate domain JSON, grant exec or invent actual billing.
#[test]
fn provider_response_terminal_failures_and_nonjson_text_remain_explicit() {
    let v = values();
    let raw = codex("not domain JSON");
    let mut valid = swarm_engine::ProviderResponseStream::new(&v, None).unwrap();
    valid.feed(&raw).unwrap();
    assert_eq!(valid.finish().unwrap().response, b"not domain JSON");
    for raw in [b"{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"partial\"}}\n".to_vec(),b"{\"type\":\"turn.failed\"}\n".to_vec(),[codex("complete"),b"{\"type\":\"turn.completed\"}\n".to_vec()].concat()]{let mut stream=swarm_engine::ProviderResponseStream::new(&v,None).unwrap();let feed=stream.feed(&raw);assert!(feed.is_err()||stream.finish().is_err());}
}

/// A tool-free role refuses observed tool declarations before any valid reply.
/// Generic transport still accepts them; fixture declarations run no tool. Each
/// fault follows a healthy control with identical caps, including byte chunks.
#[test]
fn planning_output_tool_events_refuse_and_latch_without_restricting_generic_transport() {
    let v = values();
    let mut events = Vec::new();
    for event in ["item.started", "item.completed"] {
        for kind in [
            "command_execution",
            "file_change",
            "web_search",
            "mcp_tool_call",
        ] {
            events.push(serde_json::json!({"type":event,"item":{"type":kind,"id":"tool","command":"PRIVATE_TOOL_INPUT"}}));
        }
    }
    for (role, kind) in [
        ("assistant", "tool_use"),
        ("assistant", "tool_result"),
        ("user", "tool_result"),
    ] {
        events.push(serde_json::json!({"type":role,"message":{"content":[{"type":kind,"input":{"secret":"PRIVATE_TOOL_INPUT"}}]}}));
    }
    for event in events {
        let mut raw = serde_json::to_vec(&event).unwrap();
        raw.push(b'\n');
        raw.extend(codex(PROPOSAL));
        let mut generic = swarm_engine::ProviderResponseStream::new(&v, None).unwrap();
        generic.feed(&raw).unwrap();
        assert_eq!(generic.finish().unwrap().response, PROPOSAL.as_bytes());
        for chunk in [1, 7, raw.len()] {
            healthy(&v);
            let mut stream = PlanningOutputStream::new(&v).unwrap();
            let error = raw
                .chunks(chunk)
                .find_map(|bytes| stream.feed(bytes).err())
                .expect("tool event must refuse");
            assert_eq!(error.code(), "planning-output-tool-event-forbidden");
            assert!(!error.to_string().contains("PRIVATE_TOOL_INPUT"));
            assert!(stream.feed(&codex(PROPOSAL)).is_err());
            assert_eq!(
                stream.finish().err().unwrap().code(),
                "planning-output-stream-failed"
            );
        }
    }
}
