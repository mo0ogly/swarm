use swarm_engine::{
    parse_review_reply, EngineSettingsChange, EngineSettingsDocument, EngineSettingsValues,
    ReviewQuoteOriginal, ReviewReplyLimits, ReviewReplyState, SettingCap, SettingCount,
};

/// Use the explicit verified Go defaults only as test data, not a production fallback.
fn limits() -> ReviewReplyLimits {
    ReviewReplyLimits {
        min_text_bytes: 8,
        max_text_bytes: Some(4000),
        min_normalized_quote_bytes: 8,
    }
}
/// Build a supplied JSON reply without running providers or claiming independent judgment.
fn reply(reason: &str, evidence: &str, verdict: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"reason":reason,"criteria":[{"index":1,"verdict":verdict,"evidence":evidence}]})).unwrap()
}
/// Current policy and Go text semantics admit exact/prose quotes and changes requests.
#[test]
fn reply_parsing_preserves_verdicts_order_and_literal_or_markdown_evidence() {
    let originals = [ReviewQuoteOriginal {
        name: "proof.md",
        kind: "source",
        content: "Original `exact`\nevidence",
    }];
    let parsed = parse_review_reply(
        &reply("Valid rationale", "Original exact evidence", "pass"),
        1,
        &originals,
        limits(),
    )
    .unwrap();
    assert_eq!(parsed.state, ReviewReplyState::Passed);
    assert!(!format!("{parsed:?}").contains("Original"));
    for verdict in ["fail", "unknown"] {
        let parsed = parse_review_reply(
            &reply("Valid rationale", "Missing evidence", verdict),
            1,
            &[],
            limits(),
        )
        .unwrap();
        assert_eq!(parsed.state, ReviewReplyState::ChangesRequested);
    }
    for raw in [
        br#"{"reason":"Valid rationale","criteria":null}"#.as_slice(),
        br#"{"reason":"Valid rationale","criteria":[]}"#,
    ] {
        assert_eq!(
            parse_review_reply(raw, 0, &[], limits()).unwrap().state,
            ReviewReplyState::Passed
        );
    }
}
/// A valid control precedes every malformed/index/verdict/quote fault; no partial result escapes.
#[test]
fn invalid_replies_refuse_and_recover_without_private_diagnostics() {
    let originals = [ReviewQuoteOriginal {
        name: "proof.go",
        kind: "source",
        content: "Original exact evidence",
    }];
    let control = reply("Valid rationale", "Original exact evidence", "pass");
    for bad in [
        "{}",
        "null",
        "[]",
        "{}{}",
        r#"{"reason":"Valid rationale","unknown":true}"#,
        r#"{"reason":"Valid rationale","criteria":[{"index":0,"verdict":"pass","evidence":"Original exact evidence"}]}"#,
        r#"{"reason":"Valid rationale","criteria":[{"index":1,"verdict":"PASS","evidence":"Original exact evidence"}]}"#,
        r#"{"reason":"Valid rationale","criteria":[{"index":1,"verdict":"pass","evidence":"private absent text"}]}"#,
        r#"{"reason":"Valid rationale","criteria":[{"index":1,"verdict":"pass","evidence":"Original exact evidence","unknown":true}]}"#,
    ] {
        parse_review_reply(&control, 1, &originals, limits()).unwrap();
        let error = parse_review_reply(bad.as_bytes(), 1, &originals, limits()).unwrap_err();
        assert_eq!(error.code(), "review_fragment_protocol");
        assert!(!format!("{error:?}").contains("private absent"));
        parse_review_reply(&control, 1, &originals, limits()).unwrap();
    }
}
/// Runtime limits measure trimmed minimum/raw maximum UTF-8 bytes, not characters.
#[test]
fn reply_byte_limits_and_explicit_normalized_minimum_have_observable_effects() {
    for text in ["éééé".to_owned(), "x".repeat(4000)] {
        let originals = [ReviewQuoteOriginal {
            name: "proof.rs",
            kind: "source",
            content: &text,
        }];
        parse_review_reply(&reply(&text, &text, "pass"), 1, &originals, limits()).unwrap();
    }
    for text in ["x".repeat(7), "x".repeat(4001), "       x".to_owned()] {
        let originals = [ReviewQuoteOriginal {
            name: "proof.rs",
            kind: "source",
            content: &text,
        }];
        assert!(parse_review_reply(&reply(&text, &text, "pass"), 1, &originals, limits()).is_err());
    }
    let text = "x".repeat(4001);
    let originals = [ReviewQuoteOriginal {
        name: "proof.rs",
        kind: "source",
        content: &text,
    }];
    parse_review_reply(
        &reply(&text, &text, "pass"),
        1,
        &originals,
        ReviewReplyLimits {
            max_text_bytes: None,
            ..limits()
        },
    )
    .unwrap();
    let originals = [ReviewQuoteOriginal {
        name: "proof.md",
        kind: "source",
        content: "Original `exact` evidence",
    }];
    let raw = reply("Valid rationale", "Original exact evidence", "pass");
    parse_review_reply(&raw, 1, &originals, limits()).unwrap();
    assert!(parse_review_reply(
        &raw,
        1,
        &originals,
        ReviewReplyLimits {
            min_normalized_quote_bytes: 24,
            ..limits()
        }
    )
    .is_err());
}
/// Legacy bytes/replay remain exact; v7 is an explicit complete group with no hidden defaults.
#[test]
fn v6_journal_replays_and_v7_adoption_preserves_history_and_refuses_partial_groups() {
    let raw = include_bytes!("fixtures/legacy-v6-document.json");
    let old = EngineSettingsDocument::decode(raw).unwrap();
    assert_eq!(swarm_engine::encode_model(&old).unwrap(), raw);
    let replay: EngineSettingsChange =
        serde_json::from_slice(include_bytes!("fixtures/legacy-v6-request.json")).unwrap();
    assert!(old.replay(&replay).unwrap());
    assert_eq!(
        old.values.review_reply_limits().unwrap_err().reason(),
        "settings-review-reply-policy-not-configured"
    );
    let defaults = EngineSettingsValues::installation_defaults().unwrap();
    assert_eq!(defaults.review_reply_limits().unwrap(), limits());
    let mut values = serde_json::to_value(&defaults).unwrap();
    for key in [
        "review_reply_min_text_bytes",
        "review_reply_max_text_bytes",
        "review_quote_min_normalized_bytes",
    ] {
        let previous = values.as_object_mut().unwrap().remove(key).unwrap();
        assert!(EngineSettingsValues::decode(&serde_json::to_vec(&values).unwrap()).is_err());
        values[key] = previous;
    }
    // Captured v7 values preserve this adoption across future installation schemas.
    let defaults =
        EngineSettingsDocument::decode(include_bytes!("fixtures/legacy-v7-document.json"))
            .unwrap()
            .values;
    let mut change = EngineSettingsChange {
        schema_version: 7,
        event_id: "adopt-reply".into(),
        expected_revision: old.revision,
        values: defaults,
        reason: "Explicit reply policy".into(),
        preview_token: String::new(),
    };
    change.preview_token = old.preview(&change).unwrap().preview_token;
    let new = old
        .apply(&change, "operator".into(), "2026-10-18T00:00:00Z".into())
        .unwrap()
        .0;
    assert_eq!(&new.history[..old.history.len()], &old.history);
    assert!(new.replay(&replay).unwrap() && new.replay(&change).unwrap());
    let mut zero = new.values.clone();
    zero.review_reply_min_text_bytes = Some(SettingCount(0));
    zero.review_reply_max_text_bytes = Some(SettingCap(Some(SettingCount(0))));
    zero.review_quote_min_normalized_bytes = Some(SettingCount(0));
    zero.validate().unwrap();
    zero.review_reply_min_text_bytes = Some(SettingCount(1));
    assert!(zero.validate().is_err());
}
