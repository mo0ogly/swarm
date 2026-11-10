//! Exact inspection is data-only; these tests establish no protected assurance.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use swarm_protection_contract::{canonicalize, decode_document, inspect_document};

/// Retain complete named fictional vector documents; no claimed authority is used.
fn vectors() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!("../../../../tests/fixtures/paea/contract-v1.json"))
        .unwrap()["vectors"].as_array().unwrap().clone()
}

/// Compare every frozen identity and exact canonical byte sequence to the existing
/// producer; maximum revisions stay integers in canonical text, not float values.
#[test]
fn inspection_preserves_all_vectors_and_exact_i64_content() {
    for vector in vectors() {
        let mut document = json!({"proposal":vector["proposal"],"context":vector["context"]});
        for maximum in [false, true] {
            if maximum {
                document["context"]["mandate_revision"] = json!(i64::MAX);
                document["context"]["policy_revision"] = json!(i64::MAX);
            }
            let raw = serde_json::to_vec(&document).unwrap();
            let report = inspect_document(&raw).unwrap();
            let decoded = decode_document(&raw).unwrap();
            let expected = canonicalize(&decoded.proposal, &decoded.context).unwrap();
            assert_eq!(report.canonical.as_bytes(), expected.bytes());
            assert_eq!(report.document.action_digest, expected.digest());
            assert_eq!(report.document.intent_id, expected.intent_id());
            assert_eq!(report.document.document_sha256, format!("{:x}", Sha256::digest(&raw)));
            assert_eq!(report.authority, "not-established");
            assert_eq!(report.effect, "not-established");
            assert_eq!(report.approval_validity, "not-established");
            if maximum {
                assert!(report.canonical.contains("\"mandate_revision\":9223372036854775807"));
                assert!(report.canonical.contains("\"policy_revision\":9223372036854775807"));
            } else {
                assert_eq!(report.document.action_digest, vector["action_digest"]);
                assert_eq!(report.document.intent_id, vector["intent_id"]);
            }
        }
    }
}

/// Original formatting/request keys change raw identity only; ordered source
/// mutations change canonical content. Empty provenance stays explicit data.
#[test]
fn inspection_distinguishes_original_bytes_canonical_terms_and_sources() {
    let vector = &vectors()[0];
    let mut value = json!({"proposal":vector["proposal"],"context":vector["context"]});
    let original = inspect_document(&serde_json::to_vec(&value).unwrap()).unwrap();
    value["proposal"]["request_id"] = json!("inspection-second-attempt");
    let reformatted = inspect_document(&serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    assert_ne!(original.document.document_sha256, reformatted.document.document_sha256);
    assert_eq!(original.canonical, reformatted.canonical);
    let mut second = value["context"]["sources"][0].clone();
    second["source_id"] = json!("inspection-second-source");
    value["context"]["sources"].as_array_mut().unwrap().push(second);
    let first = inspect_document(&serde_json::to_vec(&value).unwrap()).unwrap();
    value["context"]["sources"].as_array_mut().unwrap().reverse();
    let reversed = inspect_document(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert_ne!(first.canonical, reversed.canonical);
    assert_eq!(first.document.intent_id, reversed.document.intent_id);
    value["context"]["sources"] = json!([]);
    assert!(inspect_document(&serde_json::to_vec(&value).unwrap()).unwrap().canonical.contains("\"sources\":[]"));
}

/// Each strict JSON/semantic refusal follows a valid control and recovers to
/// exact original data. Safe public diagnostics never turn malformed input into
/// successful empty output; the original JSON cause remains available.
#[test]
fn inspection_invalid_inputs_refuse_then_recover_without_partial_report() {
    use std::error::Error;
    let vector = &vectors()[0];
    let value = json!({"proposal":vector["proposal"],"context":vector["context"]});
    let raw = serde_json::to_vec(&value).unwrap();
    let expected = inspect_document(&raw).unwrap();
    let mut overflow = value.clone();
    overflow["context"]["mandate_revision"] = json!(u64::MAX);
    let mut floating = value.clone();
    floating["context"]["policy_revision"] = json!(1.5);
    for changed in [b"INSPECTION_PRIVATE_CANARY".to_vec(),
        [b"\xef\xbb\xbf".as_slice(), raw.as_slice()].concat(),
        format!("{{\"proposal\":null,{}", &String::from_utf8(raw.clone()).unwrap()[1..]).into_bytes(),
        serde_json::to_vec(&overflow).unwrap(), serde_json::to_vec(&floating).unwrap()] {
        assert_eq!(inspect_document(&raw).unwrap(), expected);
        let error = inspect_document(&changed).unwrap_err();
        assert!(!error.to_string().contains("INSPECTION_PRIVATE_CANARY"));
        if matches!(error, swarm_protection_contract::Error::Json(_)) {
            assert!(error.source().is_some());
        }
        assert_eq!(inspect_document(&raw).unwrap(), expected);
    }
}
