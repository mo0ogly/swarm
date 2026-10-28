//! Exact offline approval correspondence through the real CLI, no project authority.
use serde_json::{json, Value};
use std::{fs, process::{Command, Output}};

/// Invoke two named regular files from an isolated caller using actual CLI code.
fn call(root: &std::path::Path, document: &std::path::Path, approval: &std::path::Path, lang: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust")).current_dir(root)
        .args(["protection", "check-approval", document.to_str().unwrap(), approval.to_str().unwrap(), "--json", "--lang", lang])
        .output().unwrap()
}

/// Both domains/languages emit exactly the shared report on match and mismatch;
/// success of a negative data check grants no authority and opens no Root.
#[test]
fn approval_cli_matches_shared_bytes_for_match_and_mismatch_without_root() {
    let root = tempfile::tempdir().unwrap();
    let document = root.path().join("document.json"); let approval = root.path().join("approval.json");
    for prefix in ["payment", "git"] {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../web/paea-examples/v1");
        let bytes = fs::read(dir.join(format!("{prefix}-document.json"))).unwrap();
        let value: Value = serde_json::from_slice(&fs::read(dir.join(format!("{prefix}-approval.json"))).unwrap()).unwrap();
        fs::write(&document, &bytes).unwrap();
        for mismatch in [false, true] {
            let mut changed = value.clone();
            if mismatch { changed["policy_digest"] = json!("f".repeat(64)); }
            let raw = serde_json::to_vec_pretty(&changed).unwrap(); fs::write(&approval, &raw).unwrap();
            let expected = swarm_protection_contract::check_approval_documents(&bytes, &raw).unwrap();
            assert_eq!(expected.binding_matches, !mismatch);
            for lang in ["fr", "en"] {
                let output = call(root.path(), &document, &approval, lang);
                assert!(output.status.success()); assert!(output.stderr.is_empty());
                assert_eq!(output.stdout, serde_json::to_vec(&expected).unwrap());
                assert_eq!(expected.approval_validity, "not-established");
                assert!(!root.path().join(".swarm").exists());
            }
        }
    }
}

/// Matching controls precede invalid approval, stdin framing, missing file and
/// incompatible root faults; safe code2/no stdout and exact recovery preserve
/// caller files and cause-bearing diagnostics without exposing input canaries.
#[test]
fn approval_cli_refuses_invalid_inputs_and_options_then_recovers_exactly() {
    let root = tempfile::tempdir().unwrap(); let document = root.path().join("document.json"); let approval = root.path().join("approval.json");
    fs::write(&document, include_bytes!("../../../../web/paea-examples/v1/payment-document.json")).unwrap();
    let bytes = include_bytes!("../../../../web/paea-examples/v1/payment-approval.json"); fs::write(&approval, bytes).unwrap();
    let control = call(root.path(), &document, &approval, "en"); assert!(control.status.success());
    for fault in ["json", "stdin", "missing", "root"] {
        assert_eq!(call(root.path(), &document, &approval, "en").stdout, control.stdout);
        let output = match fault {
            "json" => {fs::write(&approval, b"APPROVAL_PRIVATE_CANARY").unwrap(); call(root.path(), &document, &approval, "en")},
            "stdin" => call(root.path(), &document, std::path::Path::new("-"), "en"),
            "missing" => call(root.path(), &document, &root.path().join("missing.json"), "en"),
            _ => Command::new(env!("CARGO_BIN_EXE_swarm-rust")).current_dir(root.path())
                .args(["--root", "forbidden", "protection", "check-approval", document.to_str().unwrap(), approval.to_str().unwrap(), "--json"]).output().unwrap(),
        };
        assert_eq!(output.status.code(), Some(2), "{fault}"); assert!(output.stdout.is_empty());
        assert!(!String::from_utf8(output.stderr).unwrap().contains("APPROVAL_PRIVATE_CANARY"));
        fs::write(&approval, bytes).unwrap();
        assert_eq!(call(root.path(), &document, &approval, "en").stdout, control.stdout);
        assert!(!root.path().join("forbidden").exists()); assert!(!root.path().join(".swarm").exists());
    }
}
