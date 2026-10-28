use swarm_engine::{decode_event, decode_task, decode_work_row, encode_model};

#[test]
fn work_rows_bind_sql_identity_revision_schema_and_refuse_invalid_utf8() {
    let raw = br#"{"schema_version":1,"id":"work-a","revision":1,"tasks":[{"id":"task-a"}],"legacy_unknown":{"kept_elsewhere":true}}"#;
    let w = decode_work_row("work-a", 1, raw).unwrap();
    assert_eq!(w.id, "work-a");
    assert_eq!(w.tasks.unwrap()[0].id, "task-a");
    for (id, revision, data) in [
        ("../work-a", 1, raw.as_slice()),
        ("other", 1, raw),
        ("work-a", 2, raw),
        ("work-a", 0, raw),
        ("work-a", 1, b"null"),
        (
            "work-a",
            1,
            b"{\"id\":\"private-secret\",\"revision\":false}",
        ),
        ("work-a", 1, b"{\"id\":\"\xff\"}"),
    ] {
        let error = decode_work_row(id, revision, data).expect_err("invalid row");
        assert!(!format!("{error} {error:?}").contains("private-secret"));
    }
    assert!(decode_work_row("work-a", 1, raw).is_ok());
}

#[test]
fn model_pointers_raw_json_and_go_float_format_survive_roundtrip() {
    let task = decode_task(br#"{"instruction_base":"first","instruction_base":null,"instruction_operator_base":"","independent_review":{"usage":{"provider_reported_cost_usd":1e-7}},"gate":{"document": {"same":1,"same":2,"huge":1e9999,"escape":"\ud800<"}}}"#).unwrap();
    assert!(task.instruction_base.is_none());
    assert_eq!(
        task.instruction_operator_base
            .as_deref()
            .map(String::as_str),
        Some("")
    );
    let out = String::from_utf8(encode_model(&task).unwrap()).unwrap();
    assert!(out.contains("1e-7"), "{out}");
    assert!(
        out.contains(r#""document":{"same":1,"same":2,"huge":1e9999,"escape":"\ud800\u003c"}"#),
        "{out}"
    );
    let event = decode_event(br#"{"payload":{"x":1,"x":2}}"#).unwrap();
    assert!(String::from_utf8(encode_model(&event).unwrap())
        .unwrap()
        .contains(r#""payload":{"x":1,"x":2}"#));
}

#[test]
fn persisted_model_integer_null_and_duplicate_backing_rules_are_go_compatible() {
    let task = decode_task(br#"{"criteria":["old","keep"],"criteria":["new"],"criteria":[null,null],"launch_profile":{"timeout_seconds":-0},"instruction_base":"old","instruction_base":null}"#).unwrap();
    assert_eq!(task.criteria.unwrap(), ["new", "keep"]);
    assert!(task.instruction_base.is_none());
    for raw in [
        br#"{"criteria":[1]}"#.as_slice(),
        br#"{"launch_profile":{"timeout_seconds":-0.0}}"#,
        br#"{"memory_selections":[{"version":9223372036854775808}]}"#,
    ] {
        assert!(decode_task(raw).is_err());
    }
}

#[cfg(feature = "test-fixtures")]
#[test]
fn recursive_history_preserves_go_depth_and_clone_drop_without_a_new_limit() {
    let raw = format!(
        "{}{{}}{}",
        "{\"replanned_from\":".repeat(9999),
        "}".repeat(9999)
    );
    let result =
        swarm_engine::fixture_model_roundtrip("ManagedFragmentJournalAnchor", raw.as_bytes())
            .unwrap();
    assert!(result.len() > raw.len());
    // The public Task path also protects deep deserialize, clone, equality,
    // serialization, debug and destruction, below Go's whole-document depth.
    let anchor = format!(
        "{}{{}}{}",
        "{\"replanned_from\":".repeat(9000),
        "}".repeat(9000)
    );
    let task = format!("{{\"independent_review\":{{\"fragment_journal\":{anchor}}}}}");
    let value = decode_task(task.as_bytes()).unwrap();
    let cloned = value.clone();
    assert_eq!(value, cloned);
    assert!(!format!("{value:?}").is_empty());
    assert!(!encode_model(&cloned).unwrap().is_empty());
    drop(value);
    drop(cloned);
    let excessive = format!("{{\"replanned_from\":{raw}}}");
    assert!(swarm_engine::fixture_model_roundtrip(
        "ManagedFragmentJournalAnchor",
        excessive.as_bytes()
    )
    .is_err());
}

#[test]
fn model_failures_preserve_private_causes_and_reject_nonfinite_constructed_floats() {
    use std::error::Error;
    let e = decode_task(br#"{"gate":{"evaluation":{"quality":"private-cost"}}}"#).unwrap_err();
    assert!(e.source().is_some());
    assert!(!format!("{e} {e:?}").contains("private-cost"));
    for f in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(encode_model(&swarm_engine::GoFloat(f)).is_err());
    }
    let raw = format!(
        "{{\"payload\":{}null{}}}",
        "[".repeat(9999),
        "]".repeat(9999)
    );
    let event = decode_event(raw.as_bytes()).unwrap();
    assert_eq!(
        encode_model(&event)
            .unwrap()
            .iter()
            .filter(|&&b| b == b'[')
            .count(),
        9999
    );
    let excessive = format!(
        "{{\"payload\":{}null{}}}",
        "[".repeat(10000),
        "]".repeat(10000)
    );
    assert!(decode_event(excessive.as_bytes()).is_err());
}
