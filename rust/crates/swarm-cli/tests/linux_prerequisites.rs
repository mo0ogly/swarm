#![cfg(target_os = "linux")]
//! Actual CLI binary in an empty owned cwd; no Root/host mutation or launch.
use serde_json::Value;
use std::{
    io::{self, Write},
    process::Command,
};
/// Real default-feature command output, without a source Python dependency.
fn valid(root: &std::path::Path, lang: &str) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .current_dir(root)
        .args(["identity", "linux-prerequisites", "--json", "--lang", lang])
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["schema_version"], 2);
    assert_eq!(v["read_only"], true);
    for key in [
        "launch_verified",
        "separate_host_uid_verified",
        "confinement_verified",
        "human_verified",
    ] {
        assert_eq!(v[key], false)
    }
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
    v
}
/// Both languages preserve JSON contract; forbidden and duplicate flags refuse
/// before source selection, never creating Root/output or accepting a fake path.
#[test]
fn native_prerequisites_cli_valid_refusal_and_recovery_without_root() {
    let dir = tempfile::tempdir().unwrap();
    let no_json = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .current_dir(dir.path())
        .args(["identity", "linux-prerequisites"])
        .output()
        .unwrap();
    assert!(!no_json.status.success());
    assert!(no_json.stdout.is_empty());

    for lang in ["fr", "en"] {
        valid(dir.path(), lang);
        for options in [
            vec!["--root", "."],
            vec!["--input", "PRIVATE_CANARY"],
            vec!["--input", ""],
            vec!["--output", "PRIVATE_CANARY"],
            vec!["--phase", "delivery"],
            vec!["--json"],
            vec!["--unknown"],
            vec!["extra"],
        ] {
            let output = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
                .current_dir(dir.path())
                .args(["identity", "linux-prerequisites", "--json", "--lang", lang])
                .args(options)
                .output()
                .unwrap();
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE_CANARY"));
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
            valid(dir.path(), lang);
        }
    }
}
/// Owned failing sink isolates write/flush errors; original cause stays in typed
/// outcome, and a new sink recovers without interpreting output refusal as success.
#[test]
fn native_prerequisites_cli_output_write_and_flush_preserve_original() {
    struct Sink {
        flush_fail: bool,
        bytes: Vec<u8>,
    }
    impl Write for Sink {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if !self.flush_fail {
                return Err(io::Error::other("private-prerequisite-write"));
            }
            self.bytes.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("private-prerequisite-flush"))
        }
    }
    let args = ["identity", "linux-prerequisites", "--json"].map(str::to_owned);
    for flush_fail in [false, true] {
        let mut valid = Vec::new();
        let result = swarm_cli::run_detailed(&args, "en", &mut valid, &mut Vec::new());
        assert_eq!(result.code, 0);
        assert!(result.error.is_none());
        let mut sink = Sink {
            flush_fail,
            bytes: Vec::new(),
        };
        let mut stderr = Vec::new();
        let result = swarm_cli::run_detailed(&args, "en", &mut sink, &mut stderr);
        assert_ne!(result.code, 0);
        let error = result.error.unwrap();
        assert_eq!(error.code(), "command_failed");
        let source = std::error::Error::source(&error).unwrap();
        assert!(source.to_string().contains(if flush_fail {
            "private-prerequisite-flush"
        } else {
            "private-prerequisite-write"
        }));
        assert!(!String::from_utf8_lossy(&stderr).contains("private-prerequisite"));
        assert_eq!(sink.bytes.is_empty(), !flush_fail);
        let mut recovered = Vec::new();
        assert_eq!(
            swarm_cli::run(&args, "en", &mut recovered, &mut Vec::new()),
            0
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&recovered).unwrap()["read_only"],
            true
        );
    }
}
