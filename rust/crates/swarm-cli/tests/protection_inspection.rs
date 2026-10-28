//! Public data-only inspection; no Root, approval or effect service is opened.
use serde_json::{json, Value};
use std::{fs, process::{Command, Output}};

/// Invoke the actual integration-test binary in an isolated caller directory.
/// Retain real output/exit; file reads and safe failures use normal CLI paths.
fn call(root: &std::path::Path, file: &std::path::Path, operation: &str, lang: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust")).current_dir(root)
        .args(["protection",operation,"--input",file.to_str().unwrap(),"--json","--lang",lang])
        .output().unwrap()
}

/// Require exact shared report bytes and unchanged old canonicalize output on
/// both domains/FR/EN. Maximum integer text survives; no project store is created.
#[test]
fn inspection_cli_matches_pure_report_and_existing_canonicalize() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("document.json");
    let vectors: Value = serde_json::from_str(include_str!("../../../../tests/fixtures/paea/contract-v1.json")).unwrap();
    for index in [0, 8] {
        let mut input = json!({"proposal":vectors["vectors"][index]["proposal"],"context":vectors["vectors"][index]["context"]});
        input["context"]["mandate_revision"] = json!(i64::MAX);
        input["context"]["policy_revision"] = json!(i64::MAX);
        let raw = serde_json::to_vec(&input).unwrap();
        fs::write(&file, &raw).unwrap();
        let expected = swarm_protection_contract::inspect_document(&raw).unwrap();
        for lang in ["fr", "en"] {
            let output = call(root.path(), &file, "inspect", lang);
            assert!(output.status.success());
            assert!(output.stderr.is_empty());
            assert_eq!(output.stdout, serde_json::to_vec(&expected).unwrap());
            let original = call(root.path(), &file, "canonicalize", lang);
            assert!(original.status.success());
            let previous: Value = serde_json::from_slice(&original.stdout).unwrap();
            assert_eq!(previous.as_object().unwrap().len(), 7);
            assert_eq!(previous["canonical"], expected.canonical);
            assert_eq!(previous["action_digest"], expected.document.action_digest);
            assert_eq!(previous["intent_id"], expected.document.intent_id);
            assert!(!root.path().join(".swarm").exists());
        }
    }
}

/// A valid control precedes malformed data and incompatible root selection.
/// Refuse with safe diagnostics/no stdout, then recover exact output and retain
/// the fictional input; no configuration, ledger or authority is fabricated.
#[test]
fn inspection_cli_refusal_and_recovery_keep_output_and_root_safe() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("document.json");
    let valid = include_bytes!("../../../../web/paea-examples/v1/payment-document.json");
    fs::write(&file, valid).unwrap();
    let control = call(root.path(), &file, "inspect", "en");
    assert!(control.status.success());
    fs::write(&file, b"INSPECTION_PRIVATE_CANARY").unwrap();
    let denied = call(root.path(), &file, "inspect", "en");
    assert_eq!(denied.status.code(), Some(2));
    assert!(denied.stdout.is_empty());
    let diagnostic = String::from_utf8(denied.stderr).unwrap();
    assert!(diagnostic.contains("protection-document-invalid"));
    assert!(!diagnostic.contains("INSPECTION_PRIVATE_CANARY"));
    fs::write(&file, valid).unwrap();
    assert_eq!(call(root.path(), &file, "inspect", "en").stdout, control.stdout);
    let refused = Command::new(env!("CARGO_BIN_EXE_swarm-rust")).current_dir(root.path())
        .args(["--root","forbidden","protection","inspect","--input",file.to_str().unwrap(),"--json"])
        .output().unwrap();
    assert_eq!(refused.status.code(), Some(2));
    assert!(refused.stdout.is_empty());
    assert!(!root.path().join("forbidden").exists());
    assert!(!root.path().join(".swarm").exists());
    assert_eq!(call(root.path(), &file, "inspect", "en").stdout, control.stdout);
}
