//! Saved model-rule checks through the actual default-profile CLI, no provider.
#![cfg(unix)]
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    process::{Command, Output},
};

struct Fixture {
    root: PathBuf,
}
impl Fixture {
    /// Keep isolated fixture inputs/outputs for replay, including on assertion
    /// failure. No cleanup touches live roots or an unrelated retained fixture.
    fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap().keep(),
        };
        fixture.ok(&["init"], None);
        fixture
    }
    /// Spawn the real CLI with an explicit owned root and strict input file.
    /// Each request and original output is retained under a fresh name; file/IO
    /// failures fail the test. No shell, provider or global watchdog is used.
    fn call(&self, args: &[&str], raw: Option<&[u8]>) -> Output {
        let name = swarm_engine::generate_identifier("model-check-").unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_swarm-rust"));
        command
            .args(args)
            .arg("--root")
            .arg(&self.root)
            .arg("--json");
        if let Some(raw) = raw {
            let path = self.root.join(format!("{name}.input.json"));
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .unwrap();
            file.write_all(raw).unwrap();
            file.sync_all().unwrap();
            drop(file);
            command.arg("--input").arg(path);
        }
        let output = command.output().unwrap();
        for (suffix, raw) in [("stdout", &output.stdout), ("stderr", &output.stderr)] {
            let path = self.root.join(format!("{name}.{suffix}"));
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
                .unwrap();
            file.write_all(raw).unwrap();
            file.sync_all().unwrap();
        }
        output
    }
    /// Require meaningful successful JSON and no diagnostic on the success path.
    fn ok(&self, args: &[&str], value: Option<&Value>) -> Value {
        let raw = value.map(|v| serde_json::to_vec(v).unwrap());
        let out = self.call(args, raw.as_deref());
        assert!(
            out.status.success(),
            "root={:?}: {}",
            self.root,
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stderr.is_empty());
        serde_json::from_slice(&out.stdout).unwrap()
    }
    /// Read exact persisted settings, detecting unintended publication on a check.
    fn settings(&self) -> Vec<u8> {
        fs::read(self.root.join(".swarm/engine-settings.json")).unwrap()
    }
}

/// Public preview is pure; application/reopen changes the check consumer at the
/// reported revision. Denial never becomes empty success or process permission.
#[test]
fn public_model_check_observes_saved_policy_without_mutation_or_execution() {
    let f = Fixture::new();
    let initial = f.settings();
    let base = json!({"schema_version":1,"level":"standard","model":"ordinary","automatic":true});
    let check = f.ok(&["engine-settings", "model-check"], Some(&base));
    assert_eq!(check["status"], "allowed_by_saved_rules");
    assert_eq!(check["scope"], "candidate_rules_only");
    assert_eq!(check["settings_revision"], "1");
    assert_eq!(f.settings(), initial);
    let mut denied = base.clone();
    denied["level"] = "exigeant".into();
    let out = f.call(
        &["engine-settings", "model-check"],
        Some(&serde_json::to_vec(&denied).unwrap()),
    );
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    let error: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(
        error["failure"]["code"],
        "settings-model-automatic-level-denied"
    );
    assert_eq!(f.settings(), initial);
    let doc = f.ok(&["engine-settings", "show"], None);
    let mut values = doc["values"].clone();
    values["agent_model_automatic_levels"] = json!(["exigeant"]);
    let mut change = json!({"schema_version":doc["schema_version"],"event_id":"model-check-save","expected_revision":doc["revision"],"values":values,"reason":"Explicit isolated fixture automatic level","preview_token":""});
    let preview = f.ok(&["engine-settings", "preview"], Some(&change));
    assert_eq!(f.settings(), initial);
    change["preview_token"] = preview["preview_token"].clone();
    f.ok(&["engine-settings", "apply"], Some(&change));
    let changed = f.settings();
    assert_ne!(changed, initial);
    let actual = f.ok(&["engine-settings", "model-check"], Some(&denied));
    assert_eq!(actual["settings_revision"], "2");
    assert_eq!(f.settings(), changed);
    let reopened = f.ok(&["engine-settings", "show"], None);
    assert_eq!(reopened["history"][0], doc["history"][0]);
    let db = rusqlite::Connection::open_with_flags(
        f.root.join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM main.agents", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    db.close().unwrap();
}

/// Valid controls surround each malformed request. Automatic intent is required,
/// duplicate/unknown fields refuse, and supplied private text never enters errors.
#[test]
fn public_model_check_invalid_input_refuses_safely_and_recovers() {
    let f = Fixture::new();
    let before = f.settings();
    let valid = json!({"schema_version":1,"level":"standard","model":"ordinary","automatic":false});
    for bad in [
        br#"{"schema_version":1,"level":"standard","model":"private-payload"}"#.as_slice(),
        br#"{"schema_version":1,"level":"standard","model":"ordinary","automatic":false,"automatic":true}"#,
        br#"{"schema_version":1,"level":"standard","model":"ordinary","automatic":false,"private-payload":true}"#,
        br#"{"schema_version":1,"level":"standard","model":"ordinary","automatic":"false"}"#,
        br#"{"schema_version":2,"level":"standard","model":"ordinary","automatic":false}"#,
    ] {
        f.ok(&["engine-settings","model-check"], Some(&valid));
        let out = f.call(&["engine-settings","model-check"], Some(bad));
        assert!(!out.status.success());assert!(out.stdout.is_empty());
        let error: Value = serde_json::from_slice(&out.stderr).unwrap();assert!(error["failure"]["code"].is_string());
        assert!(!String::from_utf8_lossy(&out.stderr).contains("private-payload"));
        assert_eq!(f.settings(), before);f.ok(&["engine-settings","model-check"], Some(&valid));
    }
}
