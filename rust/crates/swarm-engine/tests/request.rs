use swarm_engine::{decode_external_request, decode_request, encode_request};
#[test]
fn requests_preserve_nil_empty_defaults_and_omit_only_go_empty_values() {
    let r = decode_request(b"{}").unwrap();
    assert_eq!(
        encode_request(&r).unwrap(),
        br#"{"schema_version":0,"event_id":"","expected_revision":0}"#
    );
    let r=decode_request(br#"{"schema_version":-0,"title":"first","title":null,"criteria":[null,"ok"],"validation_policy":{}}"#).unwrap();
    assert_eq!(r.title, "first");
    assert_eq!(r.criteria, Some(vec!["".into(), "ok".into()]));
    let value: serde_json::Value = serde_json::from_slice(&encode_request(&r).unwrap()).unwrap();
    assert_eq!(
        value["validation_policy"]["controls"],
        serde_json::Value::Null
    );
    let r =
        decode_request(br#"{"criteria":[],"memory":null,"requirement_prerequisites":{}}"#).unwrap();
    assert_eq!(
        encode_request(&r).unwrap(),
        br#"{"schema_version":0,"event_id":"","expected_revision":0}"#
    );
}
#[test]
fn external_request_clears_untrusted_origin_but_creates_no_authenticated_actor() {
    let raw =
        br#"{"schema_version":1,"event_id":"event","expected_revision":0,"origin":"conducteur"}"#;
    assert_eq!(decode_request(raw).unwrap().origin, "conducteur");
    assert_eq!(decode_external_request(raw).unwrap().origin, "");
    assert!(decode_external_request(b"{}").is_err());
}
#[test]
fn strict_schema_refuses_unknown_nested_fields_wrong_types_and_overflow_without_private_echo() {
    for raw in [
        br#"{"unknown":true}"#.as_slice(),
        br#"{"validation_policy":{"unknown":true}}"#,
        br#"{"validation_policy":{"controls":[{"unknown":true}]}}"#,
        br#"{"expected_revision":9223372036854775808}"#,
        br#"{"expected_revision":1.0}"#,
        br#"{"criteria":[false]}"#,
        br#"{"validation_policy":{"controls":[{"executed":true}]}}"#,
        br#"{"title":false,"title":"later"}"#,
    ] {
        assert_eq!(decode_request(raw).unwrap_err().code(), "format");
    }
    let e = decode_request(br#"{"schema_version":"private-number-value"}"#).unwrap_err();
    assert!(!format!("{e} {e:?}").contains("private-number-value"));
    assert!(std::error::Error::source(&e)
        .unwrap()
        .to_string()
        .contains("private-number-value"));
    assert_eq!(
        decode_request("{\"ſchema_version\":1,\"EXPECTED_REVISION\":-0}".as_bytes())
            .unwrap()
            .schema_version,
        1
    );
}
#[test]
fn map_array_and_policy_duplicates_preserve_go_replacement_and_merge_rules() {
    let r=decode_request(br#"{"requirement_prerequisites":{"a":["old"],"b":["keep"]},"requirement_prerequisites":{"a":null,"c":[]},"validation_policy":{"mode":"first","controls":[{"command":[null,"echo"],"criteria":[null,1]}]},"validation_policy":{"mode":null,"actor":"last"}}"#).unwrap();
    let map = r.requirement_prerequisites.unwrap();
    assert_eq!(map["a"], None);
    assert_eq!(map["b"], Some(vec!["keep".into()]));
    assert_eq!(map["c"], Some(vec![]));
    let policy = r.validation_policy.unwrap();
    assert_eq!(policy.mode, "first");
    assert_eq!(policy.actor, "last");
    let c = &policy.controls.unwrap()[0];
    assert_eq!(c.command, Some(vec!["".into(), "echo".into()]));
    assert_eq!(c.criteria, Some(vec![0, 1]));
    let r = decode_request(br#"{"validation_policy":{"mode":"old"},"validation_policy":null}"#)
        .unwrap();
    assert!(r.validation_policy.is_none());
    let r = decode_request(br#"{"depends":["old"],"depends":["new"]}"#).unwrap();
    assert_eq!(r.depends, Some(vec!["new".into()]));
}
#[test]
fn model_encoding_keeps_i64_limits_unicode_and_null_control_entries() {
    let r=decode_request(br#"{"expected_revision":9223372036854775807,"max_attempts":-9223372036854775808,"validation_policy":{"controls":[null]}}"#).unwrap();
    assert_eq!(r.expected_revision, i64::MAX);
    assert_eq!(r.max_attempts, i64::MIN);
    let value: serde_json::Value = serde_json::from_slice(&encode_request(&r).unwrap()).unwrap();
    assert_eq!(
        value["validation_policy"]["controls"][0]["command"],
        serde_json::Value::Null
    );
    let r =
        decode_request("{\"title\":\"<x>\u{2028}&\",\"criteria\":[\"value\"]}".as_bytes()).unwrap();
    let raw = String::from_utf8(encode_request(&r).unwrap()).unwrap();
    assert!(raw.contains("\\u003cx\\u003e\\u2028\\u0026"));
}
#[test]
fn empty_dependencies_remain_explicit_at_the_end_of_legacy_request_encoding() {
    let r = decode_request(
        br#"{"schema_version":1,"depends":[],"origin":"conducteur","status":"todo"}"#,
    )
    .unwrap();
    assert_eq!(encode_request(&r).unwrap(),br#"{"schema_version":1,"event_id":"","expected_revision":0,"status":"todo","origin":"conducteur","depends":[]}"#);
    assert_eq!(
        swarm_wire_json::encode(&r).unwrap(),
        encode_request(&r).unwrap()
    );
    let r = decode_request(
        br#"{"criteria":["first","old"],"criteria":["new"],"criteria":[null,null]}"#,
    )
    .unwrap();
    assert_eq!(r.criteria, Some(vec!["new".into(), "old".into()]));
}
