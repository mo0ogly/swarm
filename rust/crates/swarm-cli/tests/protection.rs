//! Public offline CLI contract tests; no project/control/effect authority.
use serde_json::{json, Value};
use std::{
    fs,
    process::{Command, Output},
};

fn fixture(index: usize) -> Value {
    let all: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tests/fixtures/paea/contract-v1.json"
    )))
    .unwrap();
    json!({"proposal":all["vectors"][index]["proposal"],"context":all["vectors"][index]["context"]})
}
fn cli(root: &std::path::Path, input: &std::path::Path, language: &str, operation: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .current_dir(root)
        .args([
            "protection",
            operation,
            "--lang",
            language,
            "--input",
            input.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap()
}

#[test]
fn protection_both_domains_are_offline_and_context_never_becomes_authority() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("document.json");
    for language in ["fr", "en"] {
        for index in [0, 8] {
            fs::write(&input, serde_json::to_vec(&fixture(index)).unwrap()).unwrap();
            let output = cli(dir.path(), &input, language, "canonicalize");
            assert!(output.status.success(), "{:?}", output.stderr);
            assert!(output.stderr.is_empty());
            let v: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(v["authority"], "not-established");
            assert_eq!(v["scope"], "offline-data-only");
            assert_eq!(v["action_digest"].as_str().unwrap().len(), 64);
            assert!(!dir.path().join(".swarm").exists());
        }
    }
}

#[test]
fn protection_refusal_then_recovery_preserves_safe_causes_and_empty_root() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("document.json");
    let raw = serde_json::to_string(&fixture(0)).unwrap();
    fs::write(
        &input,
        raw.replacen(
            "\"request_id\":\"request-1\"",
            "\"request_id\":\"request-1\",\"private-value-never-echoed\":true",
            1,
        ),
    )
    .unwrap();
    let denied = cli(dir.path(), &input, "en", "canonicalize");
    assert_eq!(denied.status.code(), Some(2));
    assert!(denied.stdout.is_empty());
    let text = String::from_utf8(denied.stderr).unwrap();
    assert!(!text.contains("private-value-never-echoed"));
    assert!(text.contains("protection-document-invalid"));
    fs::write(&input, raw).unwrap();
    assert!(cli(dir.path(), &input, "en", "canonicalize")
        .status
        .success());
    assert!(!dir.path().join(".swarm").exists());
}

#[test]
fn protection_live_operations_are_not_advertised_as_implemented() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("document.json");
    fs::write(&input, serde_json::to_vec(&fixture(0)).unwrap()).unwrap();
    for op in ["commit", "approve", "config-apply", "mandate-create"] {
        let output = cli(dir.path(), &input, "fr", op);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    let output = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .current_dir(dir.path())
        .args([
            "protection",
            "canonicalize",
            "--input",
            input.to_str().unwrap(),
            "--root",
            "missing-root",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!dir.path().join("missing-root").exists());
    assert!(!dir.path().join(".swarm").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn protection_output_failure_is_nonzero_without_project_side_effects() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("document.json");
    fs::write(&input, serde_json::to_vec(&fixture(8)).unwrap()).unwrap();
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .current_dir(dir.path())
        .args([
            "protection",
            "canonicalize",
            "--input",
            input.to_str().unwrap(),
            "--json",
        ])
        .stdout(full)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!dir.path().join(".swarm").exists());
}

/// Invoke the new two-file route without opening a project or accepting stdin.
/// Tests alone own these fixture files; child IO failures fail the test.
fn compare_cli(root: &std::path::Path, language: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .current_dir(root)
        .args(["protection", "compare", "before.json", "after.json", "--json", "--lang", language])
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn protection_comparison_both_domains_preserves_values_and_authority_boundary() {
    let dir = tempfile::tempdir().unwrap();
    for language in ["fr", "en"] {
        for (before, after, path) in [(0, 2, "/action/amount_minor"), (8, 10, "/action/candidate_oid")] {
            let old = serde_json::to_vec(&fixture(before)).unwrap();
            let new = serde_json::to_vec(&fixture(after)).unwrap();
            fs::write(dir.path().join("before.json"), &old).unwrap();
            fs::write(dir.path().join("after.json"), &new).unwrap();
            let output = compare_cli(dir.path(), language, &[]);
            assert!(output.status.success(), "{:?}", output.stderr);
            assert!(output.stderr.is_empty());
            let report: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["scope"], "offline-data-only");
            assert_eq!(report["comparison"]["changed_paths"], json!([path]));
            assert_eq!(report["comparison"]["canonical_content_changed"], true);
            assert_eq!(report["comparison"]["intent_changed"], false);
            for marker in ["authority", "effect", "approval_validity"] {
                assert_eq!(report["comparison"][marker], "not-established");
            }
            assert_eq!(fs::read(dir.path().join("before.json")).unwrap(), old);
            assert_eq!(fs::read(dir.path().join("after.json")).unwrap(), new);
            assert!(!dir.path().join(".swarm").exists());
        }
    }
}

#[test]
fn protection_comparison_fault_on_either_side_refuses_then_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let valid = serde_json::to_vec(&fixture(0)).unwrap();
    for file in ["before.json", "after.json"] {
        fs::write(dir.path().join("before.json"), &valid).unwrap();
        fs::write(dir.path().join("after.json"), &valid).unwrap();
        let mut unknown = fixture(0);
        unknown["private-CANARY"] = json!(true);
        for bad in [b"{\"private-CANARY\":".to_vec(), serde_json::to_vec(&unknown).unwrap()] {
            assert!(compare_cli(dir.path(), "en", &[]).status.success());
            fs::write(dir.path().join(file), bad).unwrap();
            let failure = compare_cli(dir.path(), "en", &[]);
            assert_eq!(failure.status.code(), Some(2));
            assert!(failure.stdout.is_empty());
            let diagnostic = String::from_utf8(failure.stderr).unwrap();
            assert!(diagnostic.contains("protection-document-invalid"));
            assert!(!diagnostic.contains("CANARY"));
            fs::write(dir.path().join(file), &valid).unwrap();
            assert!(compare_cli(dir.path(), "en", &[]).status.success());
        }
        fs::remove_file(dir.path().join(file)).unwrap();
        let missing = compare_cli(dir.path(), "en", &[]);
        assert_eq!(missing.status.code(), Some(2));
        assert!(missing.stdout.is_empty());
        assert!(String::from_utf8(missing.stderr).unwrap().contains("input-document-io"));
        fs::write(dir.path().join(file), &valid).unwrap();
        assert!(compare_cli(dir.path(), "en", &[]).status.success());
    }
    assert!(!dir.path().join(".swarm").exists());
}

#[test]
fn protection_comparison_rejects_runtime_options_without_project_effects() {
    let dir = tempfile::tempdir().unwrap();
    let valid = serde_json::to_vec(&fixture(8)).unwrap();
    fs::write(dir.path().join("before.json"), &valid).unwrap();
    fs::write(dir.path().join("after.json"), &valid).unwrap();
    for extra in [
        vec!["--root", "forbidden-root"], vec!["--input", "extra.json"],
        vec!["--server", "http://unused.invalid"], vec!["--output", "forbidden-output"],
        vec!["--task", "task-1"], vec!["--phase", "audit"],
    ] {
        assert!(compare_cli(dir.path(), "fr", &[]).status.success());
        let failure = compare_cli(dir.path(), "fr", &extra);
        assert_eq!(failure.status.code(), Some(2));
        assert!(failure.stdout.is_empty());
        assert!(compare_cli(dir.path(), "fr", &[]).status.success());
    }
    for paths in [vec!["-", "after.json"], vec!["before.json", "-"], vec!["before.json", "after.json", "extra.json"]] {
        let failure = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .current_dir(dir.path())
            .args(["protection", "compare"])
            .args(paths)
            .arg("--json")
            .output()
            .unwrap();
        assert_eq!(failure.status.code(), Some(2));
        assert!(failure.stdout.is_empty());
        assert!(compare_cli(dir.path(), "fr", &[]).status.success());
    }
    assert!(!dir.path().join("forbidden-root").exists());
    assert!(!dir.path().join("forbidden-output").exists());
    assert!(!dir.path().join(".swarm").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn protection_comparison_output_failure_then_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let valid = serde_json::to_vec(&fixture(0)).unwrap();
    fs::write(dir.path().join("before.json"), &valid).unwrap();
    fs::write(dir.path().join("after.json"), &valid).unwrap();
    assert!(compare_cli(dir.path(), "en", &[]).status.success());
    let full = fs::OpenOptions::new().write(true).open("/dev/full").unwrap();
    let failed = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .current_dir(dir.path())
        .args(["protection", "compare", "before.json", "after.json", "--json"])
        .stdout(full)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8(failed.stderr).unwrap().contains("storage_unavailable"));
    assert!(compare_cli(dir.path(), "en", &[]).status.success());
    assert!(!dir.path().join(".swarm").exists());
}
