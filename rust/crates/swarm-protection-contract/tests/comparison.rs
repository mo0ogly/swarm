//! Pending compilation: comparison regressions use the existing frozen fixtures.
use serde_json::{json, Value};
use swarm_protection_contract::compare_documents;

/// Build document bytes from a frozen vector without accepting its claimed digest.
fn document(index: usize) -> Vec<u8> {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea/contract-v1.json"
    ))
    .expect("fixture JSON");
    let vector = &fixture["vectors"][index];
    serde_json::to_vec(&json!({"proposal": vector["proposal"], "context": vector["context"]}))
        .expect("fixture serialization")
}

#[test]
fn compares_both_domains_without_authorizing() {
    for (before, after, path, intent) in [
        (0, 2, "/action/amount_minor", false),
        (0, 3, "/action/beneficiary_account_id", false),
        (0, 6, "/context/project_id", true),
        (8, 10, "/action/candidate_oid", false),
        (8, 11, "/action/expected_old_oid", true),
    ] {
        let report = compare_documents(&document(before), &document(after)).expect("valid pair");
        assert_eq!(report.changed_paths, vec![path.to_owned()]);
        assert!(report.canonical_content_changed);
        assert_eq!(report.intent_changed, intent);
        assert_eq!(report.authority, "not-established");
        assert_eq!(report.effect, "not-established");
        assert_eq!(report.approval_validity, "not-established");
    }
}

#[test]
fn transport_and_label_order_preserve_content_but_sources_do_not() {
    let before = document(0);
    let transport = compare_documents(&before, &document(1)).expect("new request");
    assert!(transport.transport_request_changed);
    assert!(!transport.canonical_content_changed);
    assert!(transport.changed_paths.is_empty());
    let mut after: Value = serde_json::from_slice(&before).expect("fixture");
    after["context"]["sources"][0]["confidentiality_labels"] = json!(["internal", "restricted"]);
    let labels = compare_documents(&before, &serde_json::to_vec(&after).unwrap()).unwrap();
    assert!(!labels.canonical_content_changed);
    let mut source = after["context"]["sources"][0].clone();
    source["source_id"] = json!("document-2");
    after["context"]["sources"].as_array_mut().unwrap().push(source);
    let first = serde_json::to_vec(&after).unwrap();
    after["context"]["sources"].as_array_mut().unwrap().reverse();
    let report = compare_documents(&first, &serde_json::to_vec(&after).unwrap()).unwrap();
    assert_eq!(report.changed_paths, vec!["/context/sources"]);
    assert!(report.canonical_content_changed);
    assert!(!report.intent_changed);
}

#[test]
fn invalid_input_on_either_side_refuses_then_recovers() {
    let valid = document(0);
    let mut overflow: Value = serde_json::from_slice(&valid).unwrap();
    overflow["proposal"]["action"]["amount_minor"] = json!("9223372036854775808");
    let overflow = serde_json::to_vec(&overflow).unwrap();
    for invalid in [b"{\"private-CANARY\":".as_slice(), overflow.as_slice()] {
        assert!(compare_documents(&valid, &valid).is_ok());
        for (before, after) in [(invalid, valid.as_slice()), (valid.as_slice(), invalid)] {
            let error = compare_documents(before, after).unwrap_err();
            assert!(!error.to_string().contains("CANARY"));
        }
        assert!(compare_documents(&valid, &valid).is_ok());
    }
}
