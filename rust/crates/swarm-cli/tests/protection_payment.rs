//! Public offline payment resolution; no Root, protected registry or authority.
use serde_json::{json, Value};
use std::{
    error::Error as _,
    fs,
    io::{self, Write},
    path::Path,
    process::{Command, Output, Stdio},
};

/// Create only two data files in an owned temporary directory.
fn fixture(root: &Path) {
    fs::write(
        root.join("registry.json"),
        include_bytes!("../../../../tests/fixtures/paea/payment-registry-v1.json"),
    )
    .unwrap();
    fs::write(
        root.join("request.json"),
        include_bytes!("../../../../tests/fixtures/paea/payment-request-v1.json"),
    )
    .unwrap();
}

/// Absolute named files allow in-process tests without changing global cwd.
fn args(root: &Path, language: &str) -> Vec<String> {
    vec![
        "protection".into(),
        "resolve-payment".into(),
        root.join("registry.json").to_str().unwrap().into(),
        root.join("request.json").to_str().unwrap().into(),
        "--json".into(),
        "--lang".into(),
        language.into(),
    ]
}

/// Run the real binary with EOF stdin and no global timeout or runtime state.
fn call(root: &Path, language: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .current_dir(root)
        .args(args(root, language))
        .args(extra)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// Decode only successful reports and require the explicit non-authority markers.
fn accepted(output: Output) -> Value {
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["scope"], "offline-data-only");
    for field in ["authority", "registry_authenticity", "effect"] {
        assert_eq!(value[field], "not-established");
    }
    value
}

/// Refusal has no success output or unsafe input echo; verify the actual envelope.
fn refused(output: Output, code: &str) {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["failure"]["code"], code);
    assert!(!String::from_utf8(output.stderr).unwrap().contains("CANARY"));
}

/// Both languages expose complete proposals without creating Root state; aliases
/// differ only in transport ID and raw request hash, never resolved debt terms.
#[test]
fn payment_resolution_public_aliases_and_both_languages() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    for lang in ["fr", "en"] {
        let first = accepted(call(dir.path(), lang, &[]));
        let mut request: Value =
            serde_json::from_slice(&fs::read(dir.path().join("request.json")).unwrap()).unwrap();
        request["invoice_reference"] = json!("legacy-12");
        request["request_id"] = json!("attempt-2");
        fs::write(
            dir.path().join("request.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        let alias = accepted(call(dir.path(), lang, &[]));
        assert_eq!(first["proposal"]["action"], alias["proposal"]["action"]);
        assert_eq!(alias["proposal"]["action"]["invoice_id"], "debt-17");
        fixture(dir.path());
    }
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

/// Invalid inputs, missing files and incompatible runtime/framing refuse then
/// recover, preserving both input files and avoiding any output/Root creation.
#[test]
fn payment_resolution_public_refusal_and_recovery() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    for file in ["registry.json", "request.json"] {
        accepted(call(dir.path(), "en", &[]));
        fs::write(dir.path().join(file), b"{\"CANARY\":true}").unwrap();
        refused(call(dir.path(), "en", &[]), "protection-document-invalid");
        fixture(dir.path());
        accepted(call(dir.path(), "en", &[]));
        fs::remove_file(dir.path().join(file)).unwrap();
        refused(call(dir.path(), "fr", &[]), "input-document-io");
        fixture(dir.path());
        accepted(call(dir.path(), "fr", &[]));
    }
    for extra in [
        vec!["--root", "absent-root"],
        vec!["--input", "unused"],
        vec!["--server", "unused"],
        vec!["--output", "absent-output"],
        vec!["--task", "unused"],
        vec!["--phase", "audit"],
        vec!["extra"],
    ] {
        accepted(call(dir.path(), "fr", &[]));
        refused(call(dir.path(), "fr", &extra), "command_failed");
        accepted(call(dir.path(), "fr", &[]));
    }
    for paths in [["-", "request.json"], ["registry.json", "-"]] {
        accepted(call(dir.path(), "fr", &[]));
        let output = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .current_dir(dir.path())
            .args([
                "protection",
                "resolve-payment",
                paths[0],
                paths[1],
                "--json",
            ])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        refused(output, "command_failed");
        accepted(call(dir.path(), "fr", &[]));
    }
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

/// Write and flush errors retain their IO cause; publication may be partial or
/// complete but never success. Recovery returns identical data with no mutation.
#[test]
fn payment_resolution_write_and_flush_causes_and_recovery() {
    struct Broken {
        flush_only: bool,
        bytes: Vec<u8>,
    }
    impl Write for Broken {
        /// Preserve bytes only in the flush-failure variant.
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if !self.flush_only {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "IO_CANARY"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        /// Inject a final publication failure after any accepted writes.
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "IO_CANARY"))
        }
    }
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    for flush_only in [false, true] {
        let argv = args(dir.path(), "en");
        let mut good = Vec::new();
        assert_eq!(swarm_cli::run(&argv, "en", &mut good, &mut Vec::new()), 0);
        let mut broken = Broken {
            flush_only,
            bytes: Vec::new(),
        };
        let mut diagnostics = Vec::new();
        let result = swarm_cli::run_detailed(&argv, "en", &mut broken, &mut diagnostics);
        assert_eq!(result.code, 2);
        let error = result.error.unwrap();
        let cause = error.source().unwrap().downcast_ref::<io::Error>().unwrap();
        assert_eq!(cause.kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(cause.to_string(), "IO_CANARY");
        assert!(result.diagnostic_error.is_none());
        assert!(!String::from_utf8(diagnostics).unwrap().contains("CANARY"));
        assert_eq!(
            broken.bytes,
            if flush_only { good.clone() } else { Vec::new() }
        );
        let mut recovered = Vec::new();
        assert_eq!(
            swarm_cli::run(&argv, "en", &mut recovered, &mut Vec::new()),
            0
        );
        assert_eq!(good, recovered);
    }
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}
