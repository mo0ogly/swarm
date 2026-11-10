use swarm_engine::{decode_managed_review_context, encode_model};

/// The strict context codec keeps actual Go null/duplicate/raw semantics without authorizing a review.
#[test]
fn context_codec_keeps_go_alias_null_and_opaque_receipt_semantics() {
    let raw=br#"{"candidate_commit":"candidate","CANDIDATE_COMMIT":null,"tasks":[{"task":"task","title":"Original","TITLE":null,"criteria":[]}],"receipt":{"opaque":1e999,"n":-0},"sources":[]}"#;
    let c = decode_managed_review_context(raw, true).unwrap();
    assert_eq!(c.candidate_commit, "candidate");
    assert_eq!(c.tasks.as_ref().unwrap()[0].title, "Original");
    let text = String::from_utf8(encode_model(&c).unwrap()).unwrap();
    assert!(text.contains("\"receipt\":{\"opaque\":1e999,\"n\":-0}"));
    assert!(!text.contains("\"sources\""));
    assert!(text.contains("\"criteria\":[]"));
    let nil = decode_managed_review_context(b"null", true).unwrap();
    assert!(nil.tasks.is_none());
    assert_eq!(nil.citation_contract_version, 0);
    let replaced =
        decode_managed_review_context(br#"{"tasks":[{"title":"A"}],"TASKS":[{"task":"B"}]}"#, true)
            .unwrap();
    let t = &replaced.tasks.as_ref().unwrap()[0];
    assert_eq!(t.task, "B");
    assert_eq!(t.title, "A");
}
/// Unknown structured fields fail in strict mode, while opaque receipts remain unconstrained data.
#[test]
fn context_codec_refuses_unknown_or_wrong_types_without_weakening_work_mode() {
    let witness=br#"{"tasks":[{"delivery":{"criteria":[{"index":1,"status":"pass"}]}}],"accepted_baseline":{"accepted_reviews":[{"state":"passed"}]}}"#;
    assert!(decode_managed_review_context(witness, true).is_ok());
    for raw in [
        br#"{"unknown":1}"#.as_slice(),
        br#"{"tasks":[{"unknown":1}]}"#,
        br#"{"tasks":[{"delivery":{"unknown":1}}]}"#,
        br#"{"tasks":[{"delivery":{"criteria":[{"unknown":1}]}}]}"#,
        br#"{"accepted_baseline":{"unknown":1}}"#,
        br#"{"accepted_baseline":{"accepted_reviews":[{"unknown":1}]}}"#,
    ] {
        assert!(decode_managed_review_context(witness, true).is_ok());
        assert!(decode_managed_review_context(raw, true).is_err());
        assert!(decode_managed_review_context(raw, false).is_ok());
    }
    for raw in [
        br#"{"citation_contract_version":1.0}"#.as_slice(),
        br#"{"tasks":{}}"#,
        br#"{"tasks":[{"delivery":{"criteria":[{"index":"1"}]}}]}"#,
        b"{}{}",
        b"[]",
        b"{",
    ] {
        assert!(decode_managed_review_context(witness, true).is_ok());
        assert!(decode_managed_review_context(raw, true).is_err());
    }
    assert!(decode_managed_review_context(br#"{"receipt":{"unknown":[1e999]}}"#, true).is_ok());
    assert!(swarm_engine::decode_work(br#"{"unknown":1,"tasks":[{"unknown":2}]}"#).is_ok());
}
