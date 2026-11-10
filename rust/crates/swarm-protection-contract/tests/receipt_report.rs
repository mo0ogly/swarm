//! Exact-input receipt report regressions, not executed under the compilation ban.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use swarm_protection_contract::{check_receipt_documents, State};

/// Load frozen expected identities and construct unsigned receipt-shaped fixture data.
fn inputs(index: usize) -> (Vec<u8>, Vec<u8>) {
    let all: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea/contract-v1.json"
    ))
    .unwrap();
    let v = &all["vectors"][index];
    (
        serde_json::to_vec(&json!({"proposal":v["proposal"],"context":v["context"]})).unwrap(),
        serde_json::to_vec(
            &json!({"schema_version":1,"action_digest":v["action_digest"],
            "intent_id":v["intent_id"],"sink_id":"sink-1","outcome":"COMMITTED",
            "observation_digest":"d".repeat(64)}),
        )
        .unwrap(),
    )
}

/// Change one fixture JSON value without touching the original byte slice.
fn changed(raw: &[u8], key: &str, value: Value) -> Vec<u8> {
    let mut object: Value = serde_json::from_slice(raw).unwrap();
    object[key] = value;
    serde_json::to_vec(&object).unwrap()
}

/// Different observation bytes must change the report's raw-receipt identity;
/// forged matching data still establish no authentic observation or protected effect.
#[test]
fn receipt_report_binds_observation_and_raw_bytes_without_authorizing() {
    for index in [0, 8] {
        let (document, receipt) = inputs(index);
        let first =
            check_receipt_documents(&document, &receipt, "sink-1", State::CommitRequested).unwrap();
        let other = changed(&receipt, "observation_digest", json!("e".repeat(64)));
        let second =
            check_receipt_documents(&document, &other, "sink-1", State::CommitRequested).unwrap();
        assert_eq!(first.schema_version, 2);
        assert_eq!(first.scope, "offline-data-only");
        assert_eq!(
            first.document.document_sha256,
            format!("{:x}", Sha256::digest(&document))
        );
        assert_eq!(
            first.receipt_sha256,
            format!("{:x}", Sha256::digest(&receipt))
        );
        assert_eq!(
            second.receipt_sha256,
            format!("{:x}", Sha256::digest(&other))
        );
        assert_ne!(first.receipt_sha256, second.receipt_sha256);
        assert_eq!(first.document, second.document);
        assert_eq!(first.previous_state, State::CommitRequested);
        assert_eq!(first.proposed_state, State::Committed);
        for report in [&first, &second] {
            assert_eq!(report.authority, "not-established");
            assert_eq!(report.receipt_authenticity, "not-established");
            assert_eq!(report.effect, "not-established");
        }
        assert!(
            check_receipt_documents(&document, &receipt, "sink-1", State::CommitRequested).is_ok()
        );
    }
}

/// Raw whitespace and transport aliases are distinguishable while canonical
/// action/intent stay identical; receipt formatting affects only its raw hash.
#[test]
fn receipt_report_transport_and_whitespace_do_not_change_business_identity() {
    for index in [0, 8] {
        let (document, receipt) = inputs(index);
        let original =
            check_receipt_documents(&document, &receipt, "sink-1", State::CommitRequested).unwrap();
        let (transport, _) = inputs(index + 1);
        let parsed: Value = serde_json::from_slice(&document).unwrap();
        let pretty = serde_json::to_vec_pretty(&parsed).unwrap();
        for raw in [&transport, &pretty] {
            let next =
                check_receipt_documents(raw, &receipt, "sink-1", State::CommitRequested).unwrap();
            assert_ne!(
                next.document.document_sha256,
                original.document.document_sha256
            );
            assert_eq!(next.document.action_digest, original.document.action_digest);
            assert_eq!(next.document.intent_id, original.document.intent_id);
            assert_eq!(next.receipt_sha256, original.receipt_sha256);
        }
        let pretty_receipt =
            serde_json::to_vec_pretty(&serde_json::from_slice::<Value>(&receipt).unwrap()).unwrap();
        let next =
            check_receipt_documents(&document, &pretty_receipt, "sink-1", State::CommitRequested)
                .unwrap();
        assert_eq!(next.document, original.document);
        assert_ne!(next.receipt_sha256, original.receipt_sha256);
    }
}

/// Malformed/unknown/Unicode-duplicate input on either side yields no report,
/// preserves the original bytes and recovers between valid controls.
#[test]
fn receipt_report_rejects_bad_documents_and_receipts_then_recovers() {
    for index in [0, 8] {
        let (document, receipt) = inputs(index);
        let duplicate = String::from_utf8(receipt.clone()).unwrap().replacen(
            "\"sink_id\":\"sink-1\"",
            "\"sink_id\":\"sink-1\",\"sink_\\u0069d\":\"sink-1\"",
            1,
        );
        assert_ne!(duplicate.as_bytes(), receipt.as_slice());
        let unknown_document = changed(&document, "PRIVATE_CANARY", json!(true));
        for (left, right) in [
            (b"{".as_slice(), receipt.as_slice()),
            (document.as_slice(), b"{".as_slice()),
            (document.as_slice(), duplicate.as_bytes()),
            (unknown_document.as_slice(), receipt.as_slice()),
        ] {
            assert!(
                check_receipt_documents(&document, &receipt, "sink-1", State::CommitRequested)
                    .is_ok()
            );
            let before = (left.to_vec(), right.to_vec());
            assert_eq!(
                check_receipt_documents(left, right, "sink-1", State::CommitRequested)
                    .unwrap_err()
                    .code(),
                "protection-document-invalid"
            );
            assert_eq!(left, before.0.as_slice());
            assert_eq!(right, before.1.as_slice());
            assert!(
                check_receipt_documents(&document, &receipt, "sink-1", State::CommitRequested)
                    .is_ok()
            );
        }
        let unknown = changed(&receipt, "PRIVATE_CANARY", json!(true));
        assert!(
            check_receipt_documents(&document, &receipt, "sink-1", State::CommitRequested).is_ok()
        );
        let error = check_receipt_documents(&document, &unknown, "sink-1", State::CommitRequested)
            .unwrap_err();
        assert_eq!(error.code(), "protection-document-invalid");
        assert!(!error.to_string().contains("PRIVATE_CANARY"));
        assert!(
            check_receipt_documents(&document, &receipt, "sink-1", State::CommitRequested).is_ok()
        );
    }
}

/// Exact binding and lifecycle remain authoritative before report construction;
/// metadata hashing cannot turn a mismatch, UNKNOWN relabel or terminal replay into success.
#[test]
fn receipt_report_keeps_binding_and_lifecycle_refusals() {
    for index in [0, 8] {
        let (document, receipt) = inputs(index);
        for key in ["action_digest", "intent_id", "sink_id"] {
            let invalid = changed(
                &receipt,
                key,
                if key == "sink_id" {
                    json!("sink-2")
                } else {
                    json!("e".repeat(64))
                },
            );
            assert!(
                check_receipt_documents(&document, &receipt, "sink-1", State::CommitRequested)
                    .is_ok()
            );
            assert_eq!(
                check_receipt_documents(&document, &invalid, "sink-1", State::CommitRequested)
                    .unwrap_err()
                    .code(),
                "protection-receipt-binding"
            );
            assert!(
                check_receipt_documents(&document, &receipt, "sink-1", State::CommitRequested)
                    .is_ok()
            );
        }
        for previous in [
            State::Unknown,
            State::Committed,
            State::Denied,
            State::ReconciledCommitted,
            State::ReconciledNotCommitted,
        ] {
            assert!(
                check_receipt_documents(&document, &receipt, "sink-1", State::CommitRequested)
                    .is_ok()
            );
            assert_eq!(
                check_receipt_documents(&document, &receipt, "sink-1", previous)
                    .unwrap_err()
                    .code(),
                "protection-transition-invalid"
            );
            assert!(
                check_receipt_documents(&document, &receipt, "sink-1", State::CommitRequested)
                    .is_ok()
            );
        }
        let reconciled = changed(&receipt, "outcome", json!("RECONCILED_NOT_COMMITTED"));
        let report =
            check_receipt_documents(&document, &reconciled, "sink-1", State::Unknown).unwrap();
        assert_eq!(report.proposed_state, State::ReconciledNotCommitted);
    }
}
