use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::{Command, Output},
};
/// Public CLI on an owned root; no provider or native mission is launched.
fn call(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}
/// Known SHA256(valid) event is untrusted input data, not actor authentication.
fn input(root: &Path) -> std::path::PathBuf {
    fs::write(root.join("report.md"), b"valid").unwrap();
    let p = root.join("event.json");
    fs::write(&p,br#"{"id":"event","scope":"root","kind":"handoff","handoff":{"path":"report.md","sha256":"ec654fac9599f62e79e2706abef23dfb7c07c08185aa86db4d8695f0b718d1b3"}}"#).unwrap();
    p
}
/// Observe real report text and readonly state, changed-byte refusal/recovery,
/// strict request fields and legitimate absent-reference null as distinct outcomes.
#[test]
fn public_report_read_is_complete_readonly_and_refusals_do_not_become_null() {
    let root = tempfile::tempdir().unwrap();
    assert!(call(root.path(), &["init", "--json"]).status.success());
    let p = input(root.path());
    let args = ["planning-report", "--input", p.to_str().unwrap(), "--json"];
    let db = fs::read(root.path().join(".swarm/state.db")).unwrap();
    let settings = fs::read(root.path().join(".swarm/engine-settings.json")).unwrap();
    let valid = call(root.path(), &args);
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(v["text"], "valid");
    assert_eq!(v["event_id"], "event");
    fs::write(root.path().join("report.md"), b"changed").unwrap();
    let refused = call(root.path(), &args);
    assert_eq!(refused.status.code(), Some(2));
    assert!(refused.stdout.is_empty());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("report-digest-changed"));
    input(root.path());
    assert!(call(root.path(), &args).status.success());
    fs::write(&p, br#"{"PRIVATE_FIELD":true}"#).unwrap();
    let bad = call(root.path(), &args);
    assert_eq!(bad.status.code(), Some(2));
    assert!(!String::from_utf8_lossy(&bad.stderr).contains("PRIVATE_FIELD"));
    fs::write(&p, b"{}").unwrap();
    assert_eq!(call(root.path(), &args).stdout, b"null\n");
    assert_eq!(fs::read(root.path().join(".swarm/state.db")).unwrap(), db);
    assert_eq!(
        fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),
        settings
    );
}
/// Real public dispatch preserves separate write/flush causes, safe diagnostics,
/// original state and recovery after a valid report witness before each failure.
#[test]
fn public_report_write_and_flush_errors_keep_cause_and_allow_recovery() {
    struct Broken(bool);
    impl Write for Broken {
        fn write(&mut self, v: &[u8]) -> io::Result<usize> {
            if self.0 {
                Ok(v.len())
            } else {
                Err(io::Error::other("PRIVATE_WRITE"))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("PRIVATE_FLUSH"))
        }
    }
    let root = tempfile::tempdir().unwrap();
    assert!(call(root.path(), &["init", "--json"]).status.success());
    let p = input(root.path());
    let db = fs::read(root.path().join(".swarm/state.db")).unwrap();
    let settings = fs::read(root.path().join(".swarm/engine-settings.json")).unwrap();
    let args = [
        "planning-report",
        "--root",
        root.path().to_str().unwrap(),
        "--input",
        p.to_str().unwrap(),
        "--json",
    ]
    .map(String::from);
    for flush in [false, true] {
        assert!(call(
            root.path(),
            &["planning-report", "--input", p.to_str().unwrap(), "--json"]
        )
        .status
        .success());
        let mut err = Vec::new();
        let result = swarm_cli::run_detailed(&args, "en", &mut Broken(flush), &mut err);
        assert_eq!(result.code, 2);
        let Some(swarm_cli::CliError::Output(cause)) = result.error else {
            panic!("output cause required")
        };
        assert!(cause.to_string().contains("PRIVATE_"));
        assert!(!String::from_utf8_lossy(&err).contains("PRIVATE_"));
        assert_eq!(fs::read(root.path().join(".swarm/state.db")).unwrap(), db);
        assert_eq!(
            fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
        assert!(call(
            root.path(),
            &["planning-report", "--input", p.to_str().unwrap(), "--json"]
        )
        .status
        .success());
    }
}
