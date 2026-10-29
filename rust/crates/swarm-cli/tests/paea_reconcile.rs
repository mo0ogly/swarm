#![cfg(target_os = "linux")]
//! Public CLI contract for the read-only PA-09 effect-journal reconciliation
//! command: explicit uninitialized state, separation refusal, runtime-option
//! gate, and no mutation of either root. Real public binary, private roots,
//! no provider or server.
use std::{fs, path::Path, process::Command};
use swarm_engine::{RootStore, SystemEnvironment};

/// Exercise the real public binary against two private roots.
fn call(service: &Path, ledger: &Path, language: &str, extra: &[&str]) -> std::process::Output {
    let mut args = vec![
        "paea".to_owned(),
        "reconcile-effects".to_owned(),
        ledger.to_str().unwrap().to_owned(),
        "--json".to_owned(),
        "--lang".to_owned(),
        language.to_owned(),
        "--root".to_owned(),
        service.to_str().unwrap().to_owned(),
    ];
    args.extend(extra.iter().map(|s| s.to_string()));
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(&args)
        .output()
        .unwrap()
}

/// Uninitialized journals are an explicit per-domain state, never an empty
/// success, and the read mutates neither the service root nor the ledger.
#[test]
fn paea_reconcile_effects_reports_explicit_uninitialized_without_mutation() {
    let service_root = tempfile::tempdir().unwrap();
    let ledger_root = tempfile::tempdir().unwrap();
    RootStore::initialize(service_root.path(), &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    RootStore::initialize(ledger_root.path(), &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    let database = service_root.path().join(".swarm/state.db");
    let ledger_database = ledger_root.path().join(".swarm/state.db");
    let before = fs::read(&database).unwrap();
    let ledger_before = fs::read(&ledger_database).unwrap();
    for language in ["fr", "en"] {
        let result = call(service_root.path(), ledger_root.path(), language, &[]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.stderr.is_empty());
        let report: serde_json::Value =
            serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["schema_version"], 1);
        assert_eq!(report["settlement"]["state"], "uninitialized");
        assert_eq!(report["git"]["state"], "uninitialized");
    }
    assert_eq!(fs::read(&database).unwrap(), before);
    assert_eq!(fs::read(&ledger_database).unwrap(), ledger_before);
}

/// The same root passed as both service and ledger is refused with the exact
/// separation code: one compromised store must not fabricate both sides of
/// the cross-check.
#[test]
fn paea_reconcile_effects_refuses_shared_service_and_ledger_root() {
    let root = tempfile::tempdir().unwrap();
    RootStore::initialize(root.path(), &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    let result = call(root.path(), root.path(), "fr", &[]);
    assert!(!result.status.success());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(
        error.contains("paea-effect-reconcile-separate-ledger-required"),
        "unexpected diagnostic: {error}"
    );
}

/// Incompatible runtime options (server, task, output, input) refuse with
/// usage before any root is opened; no report is emitted.
#[test]
fn paea_reconcile_effects_rejects_runtime_options_without_opening_roots() {
    let service_root = tempfile::tempdir().unwrap();
    let ledger_root = tempfile::tempdir().unwrap();
    for extra in [
        &["--server"][..],
        &["--task", "x"],
        &["--output", "out.json"],
        &["--input", "in.json"],
    ] {
        let result = call(service_root.path(), ledger_root.path(), "fr", extra);
        assert!(!result.status.success(), "accepted: {extra:?}");
        assert!(result.stdout.is_empty(), "report for refused call: {extra:?}");
    }
    // Missing ledger positional is also a usage refusal.
    let short = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(["paea", "reconcile-effects", "--json", "--root"])
        .arg(service_root.path())
        .output()
        .unwrap();
    assert!(!short.status.success());
    assert!(short.stdout.is_empty());
}
