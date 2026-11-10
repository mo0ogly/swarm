use swarm_engine::{decode_gate_request, encode_gate_request_intent};
#[test]
fn gate_request_preserves_opaque_numbers_strict_keys_and_go_null_duplicate_semantics() {
    let raw=br#"{"schema_version":1,"SCHEMA_VERSION":null,"event_id":"gate","expected_revision":9007199254740993,"task_id":"task","phase":"delivery","name":"first","NAME":"Proof","document":{"method_version":"2","extra":[1.0,-0,1e+09,1e999]}}"#;
    let request = decode_gate_request(raw).unwrap();
    assert_eq!(request.expected_revision, 9007199254740993);
    assert_eq!(request.name, "Proof");
    assert_eq!(request.schema_version, 1);
    let encoded = encode_gate_request_intent(&request).unwrap();
    let text = std::str::from_utf8(&encoded).unwrap();
    assert!(text.contains("[1.0,-0,1e+09,1e999]"));
    assert_eq!(decode_gate_request(&encoded).unwrap(), request);
    for bad in [
        r#"{"schema_version":1,"task_id":"task","policy":{"generation":0}}"#,
        r#"{"schema_version":1,"task_id":"task","origin":"trusted"}"#,
        r#"{"schema_version":2,"task_id":"task"}"#,
        r#"{"schema_version":1,"task_id":""}"#,
        r#"{"schema_version":true,"SCHEMA_VERSION":1,"task_id":"task"}"#,
        r#"{"schema_version":1,"task_id":"task"}{}"#,
    ] {
        assert!(decode_gate_request(raw).is_ok());
        assert!(decode_gate_request(bad.as_bytes()).is_err());
        assert!(decode_gate_request(raw).is_ok());
    }
}
