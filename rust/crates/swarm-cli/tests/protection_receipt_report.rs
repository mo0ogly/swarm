//! Public v2 receipt report sources; compilation and execution remain deferred.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
};

/// Build two owned v1 input files from frozen data; receipts are unsigned fixtures.
fn fixture(root: &Path, index: usize) -> (Vec<u8>, Vec<u8>, Value) {
    let all: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tests/fixtures/paea/contract-v1.json"
    )))
    .unwrap();
    let vector = all["vectors"][index].clone();
    let action =
        serde_json::to_vec(&json!({"proposal":vector["proposal"],"context":vector["context"]}))
            .unwrap();
    let receipt = serde_json::to_vec(
        &json!({"schema_version":1,"action_digest":vector["action_digest"],
        "intent_id":vector["intent_id"],"sink_id":"sink-1","outcome":"COMMITTED",
        "observation_digest":"d".repeat(64)}),
    )
    .unwrap();
    fs::write(root.join("action.json"), &action).unwrap();
    fs::write(root.join("receipt.json"), &receipt).unwrap();
    (action, receipt, vector)
}

/// Invoke only the named fixture route with EOF stdin and captured streams.
fn call(root: &Path, operation: &str, language: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .current_dir(root)
        .args([
            "protection",
            operation,
            "action.json",
            "receipt.json",
            "sink-1",
            "COMMIT_REQUESTED",
            "--json",
            "--lang",
            language,
        ])
        .args(extra)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// Decode successful output only after checking process outcome and empty diagnostics.
fn accepted(output: Output) -> Value {
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Require the consumed CLI failure envelope and no successful or private output.
fn refused(output: Output, code: &str) {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let text = String::from_utf8(output.stderr).unwrap();
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["failure"]["code"], code);
    assert!(!text.contains("PRIVATE_CANARY"));
}

/// Distinguish exact unsigned receipt bytes in FR/EN and both domains while
/// preserving the legacy report's exact shape and its non-authorizing result.
#[test]
fn protection_receipt_report_exact_files_and_legacy_shape_in_both_languages() {
    let dir = tempfile::tempdir().unwrap();
    for language in ["fr", "en"] {
        for index in [0, 8] {
            let (action, receipt, vector) = fixture(dir.path(), index);
            let old = accepted(call(dir.path(), "check-receipt", language, &[]));
            assert_eq!(old.as_object().unwrap().len(), 9);
            assert_eq!(old["schema_version"], 1);
            assert!(old.get("receipt_sha256").is_none());
            let report = accepted(call(dir.path(), "receipt-report", language, &[]));
            assert_eq!(
                report,
                json!({
                    "schema_version":2,"scope":"offline-data-only",
                    "document":{"document_sha256":format!("{:x}",Sha256::digest(&action)),
                        "action_digest":vector["action_digest"],"intent_id":vector["intent_id"]},
                    "receipt_sha256":format!("{:x}",Sha256::digest(&receipt)),
                    "previous_state":"COMMIT_REQUESTED","proposed_state":"COMMITTED",
                    "authority":"not-established","receipt_authenticity":"not-established",
                    "effect":"not-established"
                })
            );
            let mut altered: Value = serde_json::from_slice(&receipt).unwrap();
            altered["observation_digest"] = json!("e".repeat(64));
            let raw = serde_json::to_vec(&altered).unwrap();
            fs::write(dir.path().join("receipt.json"), &raw).unwrap();
            let next = accepted(call(dir.path(), "receipt-report", language, &[]));
            assert_ne!(next["receipt_sha256"], report["receipt_sha256"]);
            assert_eq!(
                next["receipt_sha256"],
                format!("{:x}", Sha256::digest(&raw))
            );
            assert_eq!(next["document"], report["document"]);
            assert_eq!(
                accepted(call(dir.path(), "check-receipt", language, &[])),
                old
            );
            assert_eq!(fs::read(dir.path().join("action.json")).unwrap(), action);
            assert_eq!(fs::read(dir.path().join("receipt.json")).unwrap(), raw);
            assert!(!dir.path().join(".swarm").exists());
        }
    }
}

/// Invalid content and runtime/framing options retain existing codes, return
/// no report, preserve fixture state and recover between valid controls.
#[test]
fn protection_receipt_report_refuses_invalid_inputs_and_runtime_then_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let (action, receipt, _) = fixture(dir.path(), 0);
    for name in ["action.json", "receipt.json"] {
        accepted(call(dir.path(), "receipt-report", "en", &[]));
        fs::write(dir.path().join(name), b"{\"PRIVATE_CANARY\":").unwrap();
        refused(
            call(dir.path(), "receipt-report", "en", &[]),
            "protection-document-invalid",
        );
        fs::write(
            dir.path().join(name),
            if name == "action.json" {
                &action
            } else {
                &receipt
            },
        )
        .unwrap();
        accepted(call(dir.path(), "receipt-report", "en", &[]));
    }
    let mut wrong: Value = serde_json::from_slice(&receipt).unwrap();
    wrong["intent_id"] = json!("e".repeat(64));
    accepted(call(dir.path(), "receipt-report", "en", &[]));
    fs::write(
        dir.path().join("receipt.json"),
        serde_json::to_vec(&wrong).unwrap(),
    )
    .unwrap();
    refused(
        call(dir.path(), "receipt-report", "en", &[]),
        "protection-receipt-binding",
    );
    fs::write(dir.path().join("receipt.json"), &receipt).unwrap();
    accepted(call(dir.path(), "receipt-report", "en", &[]));
    for extra in [
        vec!["--root", "absent-root"],
        vec!["--input", "unused"],
        vec!["--server", "unused"],
        vec!["--output", "absent-output"],
        vec!["--task", "unused"],
        vec!["--phase", "audit"],
        vec!["extra-argument"],
    ] {
        accepted(call(dir.path(), "receipt-report", "fr", &[]));
        refused(
            call(dir.path(), "receipt-report", "fr", &extra),
            "command_failed",
        );
        accepted(call(dir.path(), "receipt-report", "fr", &[]));
    }
    for (left, right, suffix) in [
        ("-", "receipt.json", vec!["sink-1", "COMMIT_REQUESTED"]),
        ("action.json", "-", vec!["sink-1", "COMMIT_REQUESTED"]),
        ("action.json", "receipt.json", vec!["sink-1"]),
    ] {
        accepted(call(dir.path(), "receipt-report", "en", &[]));
        let output = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .current_dir(dir.path())
            .args(["protection", "receipt-report", left, right])
            .args(suffix)
            .arg("--json")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        refused(output, "command_failed");
        accepted(call(dir.path(), "receipt-report", "en", &[]));
    }
    assert_eq!(fs::read(dir.path().join("action.json")).unwrap(), action);
    assert_eq!(fs::read(dir.path().join("receipt.json")).unwrap(), receipt);
    assert!(!dir.path().join("absent-root").exists());
    assert!(!dir.path().join("absent-output").exists());
    assert!(!dir.path().join(".swarm").exists());
}

/// Public output failure remains nonzero without changing either input; this
/// /dev/full case does not replace the existing injected Rust flush regression.
#[cfg(target_os = "linux")]
#[test]
fn protection_receipt_report_writer_failure_then_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let (action, receipt, _) = fixture(dir.path(), 8);
    accepted(call(dir.path(), "receipt-report", "en", &[]));
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .current_dir(dir.path())
        .args([
            "protection",
            "receipt-report",
            "action.json",
            "receipt.json",
            "sink-1",
            "COMMIT_REQUESTED",
            "--json",
        ])
        .stdin(Stdio::null())
        .stdout(full)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["failure"]["code"], "storage_unavailable");
    accepted(call(dir.path(), "receipt-report", "en", &[]));
    assert_eq!(fs::read(dir.path().join("action.json")).unwrap(), action);
    assert_eq!(fs::read(dir.path().join("receipt.json")).unwrap(), receipt);
    assert!(!dir.path().join(".swarm").exists());
}
