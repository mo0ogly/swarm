//! Offline receipt CLI regressions; pending compilation under the user's restriction.
use serde_json::{json, Value};
use std::{
    error::Error as _,
    fs,
    io::{self, Write},
    path::Path,
    process::{Command, Output},
};

/// Load independent frozen action/receipt data without creating any effect service.
fn inputs(index: usize, outcome: &str) -> (Value, Value) {
    let vectors: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tests/fixtures/paea/contract-v1.json"
    )))
    .unwrap();
    let vector = &vectors["vectors"][index];
    (
        json!({"proposal": vector["proposal"], "context": vector["context"]}),
        json!({
            "schema_version": 1, "action_digest": vector["action_digest"],
            "intent_id": vector["intent_id"], "sink_id": "sink-1", "outcome": outcome,
            "observation_digest": "d".repeat(64)
        }),
    )
}

/// Build absolute file arguments so in-process output tests never change global cwd.
fn args(root: &Path, language: &str, sink: &str, previous: &str) -> Vec<String> {
    vec![
        "protection".into(),
        "check-receipt".into(),
        root.join("action.json").to_str().unwrap().into(),
        root.join("receipt.json").to_str().unwrap().into(),
        sink.into(),
        previous.into(),
        "--json".into(),
        "--lang".into(),
        language.into(),
    ]
}

/// Exercise the actual CLI binary on an owned temporary directory, with no timeout policy.
fn call(root: &Path, language: &str, sink: &str, previous: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .current_dir(root)
        .args(args(root, language, sink, previous))
        .args(extra)
        .output()
        .unwrap()
}

/// Refusals publish no successful stdout and match the producer's exact
/// failure.code envelope, never private input or a root-level code assumption.
fn refused(output: Output, code: &str) {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let diagnostic = String::from_utf8(output.stderr).unwrap();
    let failure: Value = serde_json::from_str(&diagnostic).unwrap();
    assert_eq!(failure["failure"]["code"], code, "{diagnostic}");
    assert!(!diagnostic.contains("CANARY"));
}

/// Matching unsigned fixtures cover both domains, every permitted outcome and
/// both help languages; reports grant nothing and all input bytes remain intact.
#[test]
fn protection_receipt_both_domains_and_reconciliation_are_offline() {
    let dir = tempfile::tempdir().unwrap();
    for language in ["fr", "en"] {
        let help = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .current_dir(dir.path())
            .args(["--help", "--lang", language])
            .output()
            .unwrap();
        assert!(help.status.success());
        assert!(String::from_utf8(help.stdout)
            .unwrap()
            .contains("protection check-receipt"));
        for index in [0, 8] {
            for (previous, outcome) in [
                ("COMMIT_REQUESTED", "COMMITTED"),
                ("COMMIT_REQUESTED", "UNKNOWN"),
                ("PROPOSED", "DENIED"),
                ("ADMITTED", "DENIED"),
                ("APPROVAL_PENDING", "DENIED"),
                ("AUTHORIZED", "DENIED"),
                ("COMMIT_REQUESTED", "DENIED"),
                ("UNKNOWN", "RECONCILED_COMMITTED"),
                ("UNKNOWN", "RECONCILED_NOT_COMMITTED"),
            ] {
                let (action, receipt) = inputs(index, outcome);
                let action_bytes = serde_json::to_vec(&action).unwrap();
                let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
                fs::write(dir.path().join("action.json"), &action_bytes).unwrap();
                fs::write(dir.path().join("receipt.json"), &receipt_bytes).unwrap();
                let output = call(dir.path(), language, "sink-1", previous, &[]);
                assert!(output.status.success(), "{:?}", output.stderr);
                assert!(output.stderr.is_empty());
                let report: Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(report["schema_version"], 1);
                assert_eq!(report["scope"], "offline-data-only");
                assert_eq!(report["action_digest"], receipt["action_digest"]);
                assert_eq!(report["intent_id"], receipt["intent_id"]);
                assert_eq!(report["previous_state"], previous);
                assert_eq!(report["proposed_state"], outcome);
                for marker in ["authority", "receipt_authenticity", "effect"] {
                    assert_eq!(report[marker], "not-established");
                }
                // Fixture observations are intentionally unsigned and unverified.
                assert!(report.get("approved").is_none());
                assert!(report.get("verified").is_none());
                assert_eq!(
                    fs::read(dir.path().join("action.json")).unwrap(),
                    action_bytes
                );
                assert_eq!(
                    fs::read(dir.path().join("receipt.json")).unwrap(),
                    receipt_bytes
                );
                assert!(!dir.path().join(".swarm").exists());
            }
        }
    }
}

/// Mutate one valid input/binding/state at a time, retain refusal diagnostics and
/// prove recovery to the same correspondence without creating a project.
#[test]
fn protection_receipt_invalid_inputs_and_bindings_refuse_then_recover() {
    let dir = tempfile::tempdir().unwrap();
    for index in [0, 8] {
        let (action, receipt) = inputs(index, "COMMITTED");
        let good_action = serde_json::to_vec(&action).unwrap();
        let good_receipt = serde_json::to_vec(&receipt).unwrap();
        fs::write(dir.path().join("action.json"), &good_action).unwrap();
        fs::write(dir.path().join("receipt.json"), &good_receipt).unwrap();
        let control = || {
            assert!(call(dir.path(), "en", "sink-1", "COMMIT_REQUESTED", &[])
                .status
                .success());
        };
        let mut mutations = Vec::new();
        for file in ["action.json", "receipt.json"] {
            mutations.push((
                file,
                b"{\"private-CANARY\":".to_vec(),
                "protection-document-invalid",
            ));
            let mut unknown = if file == "action.json" {
                action.clone()
            } else {
                receipt.clone()
            };
            unknown["private-CANARY"] = json!(true);
            mutations.push((
                file,
                serde_json::to_vec(&unknown).unwrap(),
                "protection-document-invalid",
            ));
        }
        let duplicate = String::from_utf8(good_receipt.clone()).unwrap().replacen(
            "\"sink_id\":\"sink-1\"",
            "\"sink_id\":\"sink-1\",\"sink_\\u0069d\":\"sink-CANARY\"",
            1,
        );
        assert_ne!(duplicate.as_bytes(), good_receipt.as_slice());
        mutations.push((
            "receipt.json",
            duplicate.into_bytes(),
            "protection-document-invalid",
        ));
        for field in ["action_digest", "intent_id", "sink_id"] {
            let mut bad = receipt.clone();
            bad[field] = if field == "sink_id" {
                json!("sink-CANARY")
            } else {
                json!("e".repeat(64))
            };
            mutations.push((
                "receipt.json",
                serde_json::to_vec(&bad).unwrap(),
                "protection-receipt-binding",
            ));
        }
        let mut wrong_outcome = receipt.clone();
        wrong_outcome["outcome"] = json!("AUTHORIZED");
        mutations.push((
            "receipt.json",
            serde_json::to_vec(&wrong_outcome).unwrap(),
            "protection-effect-receipt",
        ));
        let (changed, _) = inputs(if index == 0 { 2 } else { 10 }, "COMMITTED");
        mutations.push((
            "action.json",
            serde_json::to_vec(&changed).unwrap(),
            "protection-receipt-binding",
        ));
        for (file, invalid, code) in mutations {
            control();
            fs::write(dir.path().join(file), &invalid).unwrap();
            refused(
                call(dir.path(), "en", "sink-1", "COMMIT_REQUESTED", &[]),
                code,
            );
            assert_eq!(fs::read(dir.path().join(file)).unwrap(), invalid);
            fs::write(
                dir.path().join(file),
                if file == "action.json" {
                    &good_action
                } else {
                    &good_receipt
                },
            )
            .unwrap();
            control();
        }
        for (sink, previous, code) in [
            ("sink-1", "state-CANARY", "protection-document-invalid"),
            ("sink-1", "commit_requested", "protection-document-invalid"),
            ("", "COMMIT_REQUESTED", "protection-receipt-expected-sink"),
            (
                "sink-1\n",
                "COMMIT_REQUESTED",
                "protection-receipt-expected-sink",
            ),
            ("SINK-1", "COMMIT_REQUESTED", "protection-receipt-binding"),
            ("sink-1", "UNKNOWN", "protection-transition-invalid"),
            ("sink-1", "COMMITTED", "protection-transition-invalid"),
            ("sink-1", "DENIED", "protection-transition-invalid"),
            (
                "sink-1",
                "RECONCILED_COMMITTED",
                "protection-transition-invalid",
            ),
            (
                "sink-1",
                "RECONCILED_NOT_COMMITTED",
                "protection-transition-invalid",
            ),
        ] {
            control();
            refused(call(dir.path(), "en", sink, previous, &[]), code);
            control();
        }
        for file in ["action.json", "receipt.json"] {
            control();
            fs::remove_file(dir.path().join(file)).unwrap();
            refused(
                call(dir.path(), "en", "sink-1", "COMMIT_REQUESTED", &[]),
                "input-document-io",
            );
            fs::write(
                dir.path().join(file),
                if file == "action.json" {
                    &good_action
                } else {
                    &good_receipt
                },
            )
            .unwrap();
            control();
        }
        assert!(!dir.path().join(".swarm").exists());
    }
}

/// Incompatible options, missing arguments and stdin markers refuse as invocation
/// errors, including with absent paths; valid calls bracket each mutation.
#[test]
fn protection_receipt_runtime_options_and_framing_refuse_before_reads() {
    let dir = tempfile::tempdir().unwrap();
    let (action, receipt) = inputs(8, "COMMITTED");
    fs::write(
        dir.path().join("action.json"),
        serde_json::to_vec(&action).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.path().join("receipt.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    let control = || {
        assert!(call(dir.path(), "fr", "sink-1", "COMMIT_REQUESTED", &[])
            .status
            .success())
    };
    for extra in [
        vec!["--root", "forbidden-root"],
        vec!["--input", "extra.json"],
        vec!["--server", "http://unused.invalid"],
        vec!["--output", "forbidden-output"],
        vec!["--task", "task-1"],
        vec!["--phase", "audit"],
        vec!["extra-argument"],
    ] {
        control();
        refused(
            call(dir.path(), "fr", "sink-1", "COMMIT_REQUESTED", &extra),
            "command_failed",
        );
        // Missing paths distinguish syntax refusal from a document-read refusal.
        let mut invalid = args(dir.path(), "fr", "sink-1", "COMMIT_REQUESTED");
        invalid[2] = "missing-action.json".into();
        invalid[3] = "missing-receipt.json".into();
        let output = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .current_dir(dir.path())
            .args(invalid)
            .args(extra)
            .output()
            .unwrap();
        refused(output, "command_failed");
        control();
    }
    for position in [2, 3] {
        control();
        let mut invalid = args(dir.path(), "fr", "sink-1", "COMMIT_REQUESTED");
        invalid[position] = "-".into();
        refused(
            Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
                .current_dir(dir.path())
                .args(invalid)
                .output()
                .unwrap(),
            "command_failed",
        );
        control();
    }
    control();
    let mut missing_state = args(dir.path(), "fr", "sink-1", "COMMIT_REQUESTED");
    missing_state.remove(5);
    refused(
        Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .current_dir(dir.path())
            .args(missing_state)
            .output()
            .unwrap(),
        "command_failed",
    );
    control();
    assert!(!dir.path().join("forbidden-root").exists());
    assert!(!dir.path().join("forbidden-output").exists());
    assert!(!dir.path().join(".swarm").exists());
}

/// Inject write and flush errors in an isolated test subprocess; retain the IO
/// cause and failure.code, distinguish emitted bytes and recover without mutating project state.
#[test]
fn protection_receipt_write_and_flush_failures_keep_cause_and_recover() {
    // run_detailed uses the process cwd for its default Root. Keep even an
    // accidental future Root-opening regression away from the calling checkout.
    const CHILD_ROOT: &str = "SWARM_PAEA_RECEIPT_IO_CHILD_ROOT";
    if std::env::var_os(CHILD_ROOT).is_none() {
        let isolated = tempfile::tempdir().unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .current_dir(isolated.path())
            .env(CHILD_ROOT, isolated.path())
            .args([
                "--exact",
                "protection_receipt_write_and_flush_failures_keep_cause_and_recover",
                "--nocapture",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{:?} {:?}",
            output.stdout,
            output.stderr
        );
        assert!(!isolated.path().join(".swarm").exists());
        return;
    }
    let current = std::env::current_dir().unwrap();
    assert_eq!(current, Path::new(&std::env::var_os(CHILD_ROOT).unwrap()));
    assert!(!current.join(".swarm").exists());
    /// Fail before publication or after complete JSON; fixture error text stays private.
    struct BrokenOutput {
        flush_only: bool,
        bytes: Vec<u8>,
    }
    impl Write for BrokenOutput {
        /// Keep successfully written bytes to distinguish flush from write failure.
        fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
            if !self.flush_only {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "IO_CANARY"));
            }
            self.bytes.extend_from_slice(raw);
            Ok(raw.len())
        }
        /// Always fail flushing; the preceding write decides how many bytes escaped.
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "IO_CANARY"))
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let (action, receipt) = inputs(0, "COMMITTED");
    let action_bytes = serde_json::to_vec(&action).unwrap();
    let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
    fs::write(dir.path().join("action.json"), &action_bytes).unwrap();
    fs::write(dir.path().join("receipt.json"), &receipt_bytes).unwrap();
    let argv = args(dir.path(), "en", "sink-1", "COMMIT_REQUESTED");
    for flush_only in [false, true] {
        let mut good = Vec::new();
        let mut diagnostic = Vec::new();
        assert_eq!(swarm_cli::run(&argv, "en", &mut good, &mut diagnostic), 0);
        assert!(diagnostic.is_empty());
        let mut broken = BrokenOutput {
            flush_only,
            bytes: Vec::new(),
        };
        let failure = swarm_cli::run_detailed(&argv, "en", &mut broken, &mut diagnostic);
        assert_eq!(failure.code, 2);
        let error = failure.error.unwrap();
        let cause = error.source().unwrap().downcast_ref::<io::Error>().unwrap();
        assert_eq!(cause.kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(cause.to_string(), "IO_CANARY");
        assert!(failure.diagnostic_error.is_none());
        let text = String::from_utf8(diagnostic).unwrap();
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["failure"]["code"], "command_failed");
        assert!(!text.contains("CANARY"));
        assert_eq!(broken.bytes, if flush_only { good } else { Vec::new() });
        let mut recovered = Vec::new();
        assert_eq!(
            swarm_cli::run(&argv, "en", &mut recovered, &mut Vec::new()),
            0
        );
        assert_eq!(
            fs::read(dir.path().join("action.json")).unwrap(),
            action_bytes
        );
        assert_eq!(
            fs::read(dir.path().join("receipt.json")).unwrap(),
            receipt_bytes
        );
        assert!(!dir.path().join(".swarm").exists());
        assert!(!current.join(".swarm").exists());
    }
}
