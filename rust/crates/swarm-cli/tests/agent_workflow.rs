use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
/// Invoke the actual public CLI on an initialized owned root, without provider.
fn call(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}
/// Public inspection returns exact frozen framing, does not load writable method
/// overrides or change DB/settings, and refuses unknown roles without private echo.
#[test]
fn public_workflow_observation_is_readonly_and_bound_to_compiled_sources() {
    let root = tempfile::tempdir().unwrap();
    assert!(call(root.path(), &["init", "--json"]).status.success());
    fs::create_dir_all(root.path().join(".claude/skills/apex")).unwrap();
    fs::write(
        root.path().join(".claude/skills/apex/SKILL.md"),
        b"UNTRUSTED_OVERRIDE",
    )
    .unwrap();
    let db = fs::read(root.path().join(".swarm/state.db")).unwrap();
    let settings = fs::read(root.path().join(".swarm/engine-settings.json")).unwrap();
    for role in ["planner", "subplanner", "worker", "reviewer"] {
        let result = call(root.path(), &["agent-workflow", role, "--json"]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let observed: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(observed["workflow"]["role"], role);
        assert!(!observed["prompt"]
            .as_str()
            .unwrap()
            .contains("UNTRUSTED_OVERRIDE"));
        assert_eq!(fs::read(root.path().join(".swarm/state.db")).unwrap(), db);
        assert_eq!(
            fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
    }
    let refused = call(
        root.path(),
        &["agent-workflow", "PRIVATE_UNKNOWN", "--json"],
    );
    assert_eq!(refused.status.code(), Some(2));
    assert!(refused.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&refused.stderr).contains("PRIVATE_UNKNOWN"));
    assert_eq!(fs::read(root.path().join(".swarm/state.db")).unwrap(), db);
}
/// Output and flush failures remain failures after a read and checked close;
/// the stored Work/settings remain unchanged and a fresh public read recovers.
#[test]
fn workflow_output_failure_preserves_readonly_state() {
    use std::io::{self, Write};
    struct Broken {
        flush_only: bool,
    }
    impl Write for Broken {
        fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
            if self.flush_only {
                Ok(raw.len())
            } else {
                Err(io::Error::other("PRIVATE_OUTPUT_CAUSE"))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.flush_only {
                Err(io::Error::other("PRIVATE_FLUSH_CAUSE"))
            } else {
                Ok(())
            }
        }
    }
    let root = tempfile::tempdir().unwrap();
    assert!(call(root.path(), &["init", "--json"]).status.success());
    let before = fs::read(root.path().join(".swarm/state.db")).unwrap();
    let settings = fs::read(root.path().join(".swarm/engine-settings.json")).unwrap();
    let args = [
        "agent-workflow",
        "worker",
        "--root",
        root.path().to_str().unwrap(),
        "--json",
    ]
    .map(String::from);
    for flush_only in [false, true] {
        assert!(call(root.path(), &["agent-workflow", "worker", "--json"])
            .status
            .success());
        let mut diagnostics = Vec::new();
        let outcome =
            swarm_cli::run_detailed(&args, "en", &mut Broken { flush_only }, &mut diagnostics);
        assert_eq!(outcome.code, 2);
        let Some(swarm_cli::CliError::Output(cause)) = outcome.error else {
            panic!("original output cause required")
        };
        assert!(cause.to_string().contains("PRIVATE_"));
        assert!(!String::from_utf8_lossy(&diagnostics).contains("PRIVATE_"));
        assert_eq!(
            fs::read(root.path().join(".swarm/state.db")).unwrap(),
            before
        );
        assert_eq!(
            fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
        assert!(call(root.path(), &["agent-workflow", "worker", "--json"])
            .status
            .success());
    }
}
