#![cfg(target_os = "linux")]
use std::{error::Error, fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{prepare_assistant_provider, Provider};
/// Owned inert executables are inspected only; no CLI/provider/model is run.
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("readonly-provider-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
    fn provider(&self, name: &str) -> Provider {
        let p = self.0.join(name);
        fs::write(&p, b"#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Provider {
            command: p.to_str().unwrap().into(),
            args: Some(vec![
                "--dangerously-skip-permissions".into(),
                "--resume".into(),
                "PRIVATE-SESSION".into(),
                "--model".into(),
                "chosen-model".into(),
            ]),
            ..Default::default()
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap()
        }
    }
}
/// Known adapters request fresh tool-free/read-only sessions and retain only the
/// last explicit model. Model availability and effective tool isolation unproven.
#[test]
fn assistant_provider_rebuilds_known_cli_argv_without_coding_flags() {
    let f = Fixture::new();
    for name in ["claude", "skynet_harness", "codex"] {
        let p = f.provider(name);
        let before = swarm_engine::encode_model(&p).unwrap();
        let prepared = prepare_assistant_provider(&p).unwrap();
        let args = prepared.args.unwrap();
        assert!(!args.iter().any(|v| v == "PRIVATE-SESSION"
            || v == "--resume"
            || v == "--dangerously-skip-permissions"));
        assert!(args.windows(2).any(|v| v == ["--model", "chosen-model"]));
        if name == "codex" {
            assert!(args.windows(2).any(|v| v == ["--sandbox", "read-only"]));
            assert!(args.windows(2).any(|v| v == ["--disable", "multi_agent"]));
            assert_eq!(args.last().unwrap(), "-");
        } else {
            assert!(args.windows(2).any(|v| v == ["--tools", ""]));
            assert!(args.iter().any(|v| v == "--no-session-persistence"));
        }
        assert_eq!(swarm_engine::encode_model(&p).unwrap(), before);
    }
}
/// No model fallback is invented; API bridge arguments remain declared data,
/// and actual connection/role authorization belongs to the Root consumer.
#[test]
fn assistant_provider_preserves_explicit_model_and_api_bridge_contracts() {
    let f = Fixture::new();
    let mut p = f.provider("codex");
    p.args = Some(vec![
        "--model".into(),
        "first".into(),
        "-m".into(),
        "last".into(),
    ]);
    assert!(prepare_assistant_provider(&p)
        .unwrap()
        .args
        .unwrap()
        .windows(2)
        .any(|v| v == ["--model", "last"]));
    p.args = None;
    assert!(!prepare_assistant_provider(&p)
        .unwrap()
        .args
        .unwrap()
        .iter()
        .any(|v| v == "--model"));
    let mut api = f.provider("bridge");
    api.api_connection_id = "declared-connection".into();
    let prepared = prepare_assistant_provider(&api).unwrap();
    assert_eq!(
        swarm_engine::encode_model(&prepared).unwrap(),
        swarm_engine::encode_model(&api).unwrap()
    );
}
/// A valid configured witness precedes unknown/nonexecutable/missing cases.
/// Refusals keep IO causes without exposing command paths or private args.
#[test]
fn assistant_provider_refuses_unknown_or_unavailable_executables_then_recovers() {
    let f = Fixture::new();
    let p = f.provider("claude");
    prepare_assistant_provider(&p).unwrap();
    let unknown = f.provider("unknown");
    assert_eq!(
        prepare_assistant_provider(&unknown).unwrap_err().code(),
        "assistant-provider-adapter-unported"
    );
    fs::set_permissions(&p.command, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        prepare_assistant_provider(&p).unwrap_err().code(),
        "assistant-provider-executable-unavailable"
    );
    fs::set_permissions(&p.command, fs::Permissions::from_mode(0o700)).unwrap();
    prepare_assistant_provider(&p).unwrap();
    fs::remove_file(&p.command).unwrap();
    let e = prepare_assistant_provider(&p).unwrap_err();
    assert!(e
        .source()
        .unwrap()
        .downcast_ref::<std::io::Error>()
        .is_some());
    assert!(!format!("{e:?} {e}").contains(f.0.to_str().unwrap()));
    let restored = f.provider("claude");
    prepare_assistant_provider(&restored).unwrap();
    let mut relative = restored;
    relative.command = "claude".into();
    assert_eq!(
        prepare_assistant_provider(&relative).unwrap_err().code(),
        "assistant-provider-absolute-command-required"
    );
}
