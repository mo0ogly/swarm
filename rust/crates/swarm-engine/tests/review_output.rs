//! Review-domain stream data only, with no provider, identity or acceptance.
use std::io::Write;
use swarm_engine::{
    EngineSettingsValues, ReviewOutputStream, ReviewQuoteOriginal, ReviewReplyState, SettingCap,
    SettingCount,
};
const ORIGINAL: &str = "Original exact evidence from the supplied report";
const REPLY: &str = r#"{"reason":"Valid rationale","criteria":[{"index":1,"verdict":"pass","evidence":"Original exact evidence"}]}"#;
/// Freeze real installation policy, not an adapter fallback.
fn values() -> EngineSettingsValues {
    EngineSettingsValues::installation_defaults().unwrap()
}
/// Supply attributed fixture text, without claiming its origin is authenticated.
fn originals() -> [ReviewQuoteOriginal<'static>; 1] {
    [ReviewQuoteOriginal {
        name: "report.md",
        kind: "source",
        content: ORIGINAL,
    }]
}
/// Encode a complete Codex reply while preserving inner JSON bytes verbatim.
fn codex(reply: &str) -> Vec<u8> {
    format!("{}\n{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":7,\"output_tokens\":8}}}}\n",serde_json::json!({"type":"item.completed","item":{"type":"agent_message","id":"reply","text":reply}})).into_bytes()
}
/// Positive control with the same settings/originals before each independent fault.
fn healthy(v: &EngineSettingsValues) {
    let originals = originals();
    let mut stream = ReviewOutputStream::new(v, Some(8192), 1, &originals).unwrap();
    stream.feed(&codex(REPLY)).unwrap();
    let output = stream.finish().unwrap();
    assert_eq!(output.review.state, ReviewReplyState::Passed);
    assert_eq!(output.transport.response, REPLY.as_bytes());
}
/// Both provider envelopes bind exact decoded reply bytes, criteria and source quotes.
#[test]
fn review_stream_claude_codex_chunks_preserve_reply_and_unverified_facts() {
    let v = values();
    let originals = originals();
    let mut claude=serde_json::to_vec(&serde_json::json!({"type":"result","structured_output":serde_json::from_str::<serde_json::Value>(REPLY).unwrap()})).unwrap();
    claude.push(b'\n');
    for raw in [codex(REPLY), claude] {
        for chunk in [1, 7, raw.len()] {
            healthy(&v);
            let mut stream = ReviewOutputStream::new(&v, Some(8192), 1, &originals).unwrap();
            for bytes in raw.chunks(chunk) {
                stream.write_all(bytes).unwrap();
            }
            let output = stream.finish().unwrap();
            assert_eq!(output.review.state, ReviewReplyState::Passed);
            assert_eq!(output.review.criteria.unwrap()[0].index, 1);
            let parsed: serde_json::Value =
                serde_json::from_slice(&output.transport.response).unwrap();
            assert_eq!(
                parsed,
                serde_json::from_str::<serde_json::Value>(REPLY).unwrap()
            );
            assert!(stream.finish().is_err());
        }
    }
    let mut stream = ReviewOutputStream::new(&v, Some(8192), 1, &originals).unwrap();
    stream.feed(&codex(REPLY)).unwrap();
    let output = stream.finish().unwrap();
    assert_eq!(output.transport.usage.unwrap().input_tokens, 7);
    assert_eq!(output.transport.response, REPLY.as_bytes());
}
/// Inner duplicates, absent citations and wrong criterion counts never produce a review.
#[test]
fn review_stream_domain_refusal_retains_safe_cause_and_prevents_reuse() {
    let v = values();
    let originals = originals();
    let historical = REPLY.replace(
        "Valid rationale",
        "Valid rationale\",\"reason\":\"PRIVATE_INPUT",
    );
    let legacy = swarm_engine::parse_review_reply(
        historical.as_bytes(),
        1,
        &originals,
        v.review_reply_limits().unwrap(),
    )
    .unwrap();
    assert_eq!(legacy.reason, "PRIVATE_INPUT");
    for reply in [
        REPLY.replace(
            "Valid rationale",
            "Valid rationale\",\"reason\":\"PRIVATE_INPUT",
        ),
        REPLY.replace("Original exact evidence", "PRIVATE_INPUT is absent"),
        REPLY.replace("\"index\":1", "\"index\":2"),
        REPLY.replace(
            "\"verdict\":\"pass\"",
            "\"verdict\":\"pass\",\"verdict\":\"pass\"",
        ),
        REPLY.replace("\"reason\"", "\"Reason\""),
    ] {
        healthy(&v);
        let mut stream = ReviewOutputStream::new(&v, Some(8192), 1, &originals).unwrap();
        stream.feed(&codex(&reply)).unwrap();
        let error = stream.finish().err().unwrap();
        assert_eq!(error.code(), "review_fragment_protocol");
        assert!(!error.to_string().contains("PRIVATE_INPUT"));
        assert!(std::error::Error::source(&error).is_some());
        let facts = serde_json::to_value(stream.provider_facts().unwrap()).unwrap();
        assert_eq!(facts["usage_state"], "observed");
        assert_eq!(facts["usage"]["input_tokens"], 7);
        assert!(stream.finish().is_err());
        assert!(stream.feed(&codex(REPLY)).is_err());
    }
    healthy(&v);
    let mut stream = ReviewOutputStream::new(&v, Some(8192), 2, &originals).unwrap();
    stream.feed(&codex(REPLY)).unwrap();
    assert!(stream.finish().is_err());
}
/// Caller response bounds and persisted review-text limits stay separate and effective.
#[test]
fn review_stream_explicit_response_cap_and_saved_text_policy_have_no_fallback() {
    let mut v = values();
    let originals = originals();
    healthy(&v);
    for cap in [Some(0), Some(REPLY.len() - 1)] {
        let mut stream = ReviewOutputStream::new(&v, cap, 1, &originals).unwrap();
        assert_eq!(
            stream.feed(&codex(REPLY)).unwrap_err().code(),
            "planning-response-byte-limit-exceeded"
        );
        assert!(stream.finish().is_err());
    }
    let mut stream = ReviewOutputStream::new(&v, None, 1, &originals).unwrap();
    stream.feed(&codex(REPLY)).unwrap();
    assert!(stream.finish().is_ok());
    v.review_reply_min_text_bytes = Some(SettingCount(32));
    v.review_reply_max_text_bytes = Some(SettingCap(Some(SettingCount(64))));
    let mut stream = ReviewOutputStream::new(&v, Some(8192), 1, &originals).unwrap();
    stream.feed(&codex(REPLY)).unwrap();
    assert_eq!(
        stream.finish().err().unwrap().code(),
        "review_fragment_protocol"
    );
    v = values();
    healthy(&v);
    v.review_reply_min_text_bytes = None;
    v.review_reply_max_text_bytes = None;
    v.review_quote_min_normalized_bytes = None;
    assert!(ReviewOutputStream::new(&v, Some(8192), 1, &originals).is_err());
}
/// Explicit changes-requested data is preserved without inventing acceptance or an empty success.
#[test]
fn review_stream_unknown_criterion_is_changes_requested_and_missing_reply_refuses() {
    let v = values();
    let originals = originals();
    healthy(&v);
    for verdict in ["unknown", "fail"] {
        let reply = REPLY.replace("\"pass\"", &format!("\"{verdict}\""));
        let mut stream = ReviewOutputStream::new(&v, Some(8192), 1, &originals).unwrap();
        stream.feed(&codex(&reply)).unwrap();
        assert_eq!(
            stream.finish().unwrap().review.state,
            ReviewReplyState::ChangesRequested
        );
    }
    healthy(&v);
    let mut stream = ReviewOutputStream::new(&v, Some(8192), 1, &originals).unwrap();
    stream.feed(b"{\"type\":\"turn.completed\"}\n").unwrap();
    assert!(stream.finish().is_err());
}
/// Tool declarations, missing terminal and owner-observed IO loss cannot yield a verdict.
#[test]
fn review_stream_tool_and_incomplete_transport_refuse_before_domain_success() {
    let v = values();
    let originals = originals();
    for prefix in [
        b"{\"type\":\"item.started\",\"item\":{\"type\":\"command_execution\"}}\n".as_slice(),
        b"{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\"}]}}\n",
    ] {
        healthy(&v);
        let mut stream = ReviewOutputStream::new(&v, Some(8192), 1, &originals).unwrap();
        assert_eq!(
            stream.feed(prefix).unwrap_err().code(),
            "planning-output-tool-event-forbidden"
        );
        assert!(stream.feed(&codex(REPLY)).is_err());
        assert!(stream.finish().is_err());
    }
    healthy(&v);
    let mut stream = ReviewOutputStream::new(&v, Some(8192), 1, &originals).unwrap();
    stream.feed(&codex(REPLY)).unwrap();
    stream.incomplete_transport();
    let facts = serde_json::to_value(stream.provider_facts().unwrap()).unwrap();
    assert_eq!(facts["usage_state"], "incomplete");
    assert!(facts["usage"].is_null());
    assert_eq!(facts["last_valid_usage"]["input_tokens"], 7);
    assert_eq!(
        stream.finish().err().unwrap().code(),
        "planning-output-stream-failed"
    );
    healthy(&v);
    let mut stream = ReviewOutputStream::new(&v, Some(8192), 1, &originals).unwrap();
    let partial = serde_json::to_vec(
        &serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":REPLY}}),
    )
    .unwrap();
    stream.feed(&partial).unwrap();
    assert_eq!(
        stream.finish().err().unwrap().code(),
        "planning-output-terminal-required"
    );
}
