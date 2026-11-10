//! Approval correspondence is data-only, never a decision or authenticated accord.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use swarm_protection_contract::check_approval_documents;

/// Return complete frozen documents and matching unsigned approval data.
fn inputs() -> Vec<(Value, Value)> {
    let fixture: Value = serde_json::from_str(include_str!("../../../../tests/fixtures/paea/contract-v1.json")).unwrap();
    fixture["vectors"].as_array().unwrap().iter().map(|v| (
        json!({"proposal":v["proposal"],"context":v["context"]}),
        json!({"schema_version":1,"action_digest":v["action_digest"],"policy_digest":v["context"]["policy_digest"],"approver_id":"fictional-approver"}),
    )).collect()
}

/// All frozen canonical bindings match; both raw hashes remain exact and every
/// authority/authenticity/validity/effect claim remains explicitly unestablished.
#[test]
fn approval_all_vectors_match_exact_inputs_without_establishing_authority() {
    for (document, approval) in inputs() {
        let raw = serde_json::to_vec(&document).unwrap();
        let signed = serde_json::to_vec_pretty(&approval).unwrap();
        let report = check_approval_documents(&raw, &signed).unwrap();
        assert!(report.binding_matches);
        assert!(report.mismatched_fields.is_empty());
        assert_eq!(report.document.document_sha256, format!("{:x}", Sha256::digest(&raw)));
        assert_eq!(report.approval_sha256, format!("{:x}", Sha256::digest(&signed)));
        assert_eq!(report.document.action_digest, approval["action_digest"]);
        assert_eq!(report.authority, "not-established");
        assert_eq!(report.approval_authenticity, "not-established");
        assert_eq!(report.approval_validity, "not-established");
        assert_eq!(report.effect, "not-established");
    }
}

/// Request/format/declared approver changes do not alter content compatibility.
/// Changed terms/context/policy are distinguishable data, not parse failures;
/// even a matching forged approver name establishes no authenticated identity.
#[test]
fn approval_distinguishes_content_policy_attempt_and_untrusted_approver() {
    for index in [0, 8] {
        let (document, approval) = inputs()[index].clone();
        let bytes = serde_json::to_vec(&document).unwrap();
        let raw = serde_json::to_vec(&approval).unwrap();
        let control = check_approval_documents(&bytes, &raw).unwrap();
        let mut changed = document.clone();
        changed["proposal"]["request_id"] = json!("new-attempt");
        let request = check_approval_documents(&serde_json::to_vec_pretty(&changed).unwrap(), &raw).unwrap();
        assert!(request.binding_matches);
        assert_eq!(control.document.action_digest, request.document.action_digest);
        assert_ne!(control.document.document_sha256, request.document.document_sha256);
        let mut forged = approval.clone(); forged["approver_id"] = json!("another-unverified-identity");
        let report = check_approval_documents(&bytes, &serde_json::to_vec(&forged).unwrap()).unwrap();
        assert!(report.binding_matches);
        assert_ne!(control.approval_sha256, report.approval_sha256);
        assert_eq!(report.approval_authenticity, "not-established");
        for (field, paths) in [("action_digest", vec!["action_digest"]), ("policy_digest", vec!["policy_digest"])] {
            let mut wrong = approval.clone(); wrong[field] = json!("f".repeat(64));
            let mismatch = check_approval_documents(&bytes, &serde_json::to_vec(&wrong).unwrap()).unwrap();
            assert!(!mismatch.binding_matches); assert_eq!(mismatch.mismatched_fields, paths);
        }
        for variant in ["terms", "context", "policy", "source"] {
            let mut wrong = document.clone();
            match variant {
                "terms" if index == 0 => wrong["proposal"]["action"]["amount_minor"] = json!("420001"),
                "terms" => wrong["proposal"]["action"]["candidate_oid"] = json!("e".repeat(40)),
                "context" => wrong["context"]["mandate_revision"] = json!(i64::MAX),
                "policy" => wrong["context"]["policy_digest"] = json!("f".repeat(64)),
                _ => wrong["context"]["sources"] = json!([]),
            }
            let mismatch = check_approval_documents(&serde_json::to_vec(&wrong).unwrap(), &raw).unwrap();
            assert!(!mismatch.binding_matches, "{variant}");
            assert_eq!(mismatch.mismatched_fields, if variant == "policy" {vec!["action_digest", "policy_digest"]} else {vec!["action_digest"]});
        }
    }
}

/// Strict approval/document faults follow matching controls and recover exactly.
/// Public diagnostics omit input data; JSON causes survive and no partial report
/// can turn malformed/duplicate/unknown/wrong-version input into a match.
#[test]
fn approval_strict_refusals_retain_cause_and_recover_without_partial_report() {
    use std::error::Error;
    let (document, approval) = inputs()[0].clone();
    let raw = serde_json::to_vec(&document).unwrap(); let signed = serde_json::to_vec(&approval).unwrap();
    let control = check_approval_documents(&raw, &signed).unwrap();
    let mut version = approval.clone(); version["schema_version"] = json!(2);
    let mut unknown = approval.clone(); unknown["APPROVAL_PRIVATE_CANARY"] = json!(true);
    let mut name = approval.clone(); name["approver_id"] = json!("");
    let duplicate = format!("{{\"\\u0073chema_version\":1,{}", &std::str::from_utf8(&signed).unwrap()[1..]).into_bytes();
    for invalid in [b"APPROVAL_PRIVATE_CANARY".to_vec(), vec![0xff], [b"\xef\xbb\xbf".as_slice(), signed.as_slice()].concat(), duplicate, serde_json::to_vec(&version).unwrap(), serde_json::to_vec(&unknown).unwrap(), serde_json::to_vec(&name).unwrap()] {
        assert_eq!(check_approval_documents(&raw, &signed).unwrap(), control);
        let error = check_approval_documents(&raw, &invalid).unwrap_err();
        assert!(!error.to_string().contains("APPROVAL_PRIVATE_CANARY"));
        if matches!(error, swarm_protection_contract::Error::Json(_)) { assert!(error.source().is_some()); }
        assert_eq!(check_approval_documents(&raw, &signed).unwrap(), control);
    }
    assert_eq!(check_approval_documents(&raw, &signed).unwrap(), control);
    assert!(check_approval_documents(b"APPROVAL_PRIVATE_CANARY", &signed).is_err());
    assert_eq!(check_approval_documents(&raw, &signed).unwrap(), control);
}
