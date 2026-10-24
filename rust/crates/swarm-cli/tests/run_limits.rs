#![cfg(unix)]
use std::{
    fs,
    io::{self, Write},
    process::Command,
};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCount, SystemEnvironment};
/// Owned current-schema native root and explicit diagnostic input, no provider.
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    RootStore::initialize(root.path(), &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    fs::write(root.path().join("limits.json"), br#"{"schema_version":1}"#).unwrap();
    root
}
/// Public production-profile argv, never a directly called policy substitute.
fn args(root: &std::path::Path) -> Vec<String> {
    vec![
        "agent".into(),
        "limits".into(),
        "work".into(),
        "worker".into(),
        "task".into(),
        "--input".into(),
        root.join("limits.json").to_string_lossy().into_owned(),
        "--root".into(),
        root.to_string_lossy().into_owned(),
        "--json".into(),
    ]
}
/// Save/reopen through the supported settings API then observe the real CLI
/// consumer. Strict invalid inputs refuse without touching configuration/state.
#[test]
fn runtime_limits_cli_uses_saved_policy_and_refuses_invalid_preview() {
    let root = fixture();
    let store = RootStore::open(root.path(), true).unwrap();
    let doc = store.settings().unwrap();
    let mut values = doc.values.clone();
    values.runtime_max_tool_calls_default = Some(SettingCount(77));
    let mut request = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "runtime-cli".into(),
        expected_revision: doc.revision,
        values,
        reason: "CLI consumer proof".into(),
        preview_token: String::new(),
    };
    request.preview_token = store.preview_settings(&request).unwrap().preview_token;
    store
        .apply_settings(&request, &mut SystemEnvironment)
        .unwrap();
    store.close().unwrap();
    let file = root.path().join(".swarm/engine-settings.json");
    let before = fs::read(&file).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(args(root.path()))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["limits"]["max_tool_calls"], 77);
    assert_eq!(v["launch_authorized"], false);
    assert_eq!(v["preview_only"], true);
    for raw in [
        r#"{"schema_version":2}"#,
        r#"{"schema_version":1,"mission":{"observation_mode":1}}"#,
        r#"{"schema_version":1,"mission":{"max_tool_calls":78}}"#,
        r#"{"schema_version":1,"PRIVATE":true}"#,
    ] {
        fs::write(root.path().join("limits.json"), raw).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .args(args(root.path()))
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("PRIVATE"));
        assert_eq!(fs::read(&file).unwrap(), before);
    }
}
struct Denied {
    flush: bool,
}
impl Write for Denied {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.flush {
            Ok(bytes.len())
        } else {
            Err(io::Error::other("PRIVATE write"))
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.flush {
            Err(io::Error::other("PRIVATE flush"))
        } else {
            Ok(())
        }
    }
}
/// Read-only preview preserves state even if output or flush fails; a following
/// real CLI succeeds, proving cleanup released all held resources without retries.
#[test]
fn runtime_limits_cli_output_failures_are_nonzero_and_recover() {
    for flush in [false, true] {
        let root = fixture();
        let before = fs::read(root.path().join(".swarm/engine-settings.json")).unwrap();
        let mut stderr = Vec::new();
        let result =
            swarm_cli::run_detailed(&args(root.path()), "en", &mut Denied { flush }, &mut stderr);
        assert_eq!(result.code, 2);
        assert!(!String::from_utf8_lossy(&stderr).contains("PRIVATE"));
        assert_eq!(
            fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),
            before
        );
        assert!(Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .args(args(root.path()))
            .output()
            .unwrap()
            .status
            .success());
    }
}
