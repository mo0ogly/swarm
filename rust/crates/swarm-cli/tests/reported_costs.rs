//! Public native cost reads and output failures on owned Roots, no provider.
#![cfg(target_os = "linux")]
use std::io::{self, Write};
use swarm_engine::{RootStore, SystemEnvironment};
/// Initialize only owned data through public operations; never alter live state.
fn fixture() -> (tempfile::TempDir, String) {
    let root = tempfile::tempdir().unwrap();
    let mut store = RootStore::initialize(root.path(), &mut SystemEnvironment).unwrap();
    let work=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Costs","objective":"Observe","scope":"Owned","criteria":["Partial remains partial"]}"#,&mut SystemEnvironment).unwrap().result.work.id;
    store.close().unwrap();
    (root, work)
}
/// Execute the same public parser/dispatch/output path; arguments carry no grant.
fn args(root: &std::path::Path, work: &str) -> Vec<String> {
    vec![
        "costs".into(),
        "show".into(),
        work.into(),
        "--root".into(),
        root.to_str().unwrap().into(),
        "--json".into(),
    ]
}
/// The actual normal binary publishes a valid empty projection, refuses unknown
/// work and invalid arguments, then recovers on the same read-only fixture.
#[test]
fn reported_costs_cli_real_binary_reads_and_refuses_without_mutation() {
    let (root, work) = fixture();
    let before = std::fs::read(root.path().join(".swarm/state.db")).unwrap();
    let call = |args: &[String]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .args(args)
            .output()
            .unwrap()
    };
    let output = call(&args(root.path(), &work));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["scope"], "retained-worker-attempts");
    assert_eq!(summary["attempts_with_cost"], 0);
    assert_eq!(summary["attempts_without_cost"], 0);
    assert_eq!(summary["by_task"], serde_json::json!({}));
    let missing = call(&args(root.path(), "missing-work"));
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    let mut bad = args(root.path(), &work);
    bad.extend(["--input".into(), "unused".into()]);
    assert!(!call(&bad).status.success());
    assert!(call(&args(root.path(), &work)).status.success());
    assert_eq!(
        std::fs::read(root.path().join(".swarm/state.db")).unwrap(),
        before
    );
}
/// Output injection: write or flush may fail; neither failure is success and
/// private inner IO text must not be echoed. Successful witness precedes each.
#[test]
fn reported_costs_cli_write_and_flush_failures_preserve_causes() {
    struct Sink {
        flush: bool,
        bytes: Vec<u8>,
    }
    impl Write for Sink {
        fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
            if self.flush {
                self.bytes.extend_from_slice(raw);
                Ok(raw.len())
            } else {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "PRIVATE_COST_IO"))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "PRIVATE_COST_IO"))
        }
    }
    let (root, work) = fixture();
    let argv = args(root.path(), &work);
    for flush in [false, true] {
        let mut good = Vec::new();
        let mut err = Vec::new();
        assert_eq!(swarm_cli::run(&argv, "fr", &mut good, &mut err), 0);
        let mut sink = Sink {
            flush,
            bytes: Vec::new(),
        };
        let mut err = Vec::new();
        let result = swarm_cli::run_detailed(&argv, "fr", &mut sink, &mut err);
        assert_ne!(result.code, 0);
        assert!(result.error.is_some());
        assert!(!String::from_utf8(err).unwrap().contains("PRIVATE_COST_IO"));
        if flush {
            assert!(!sink.bytes.is_empty())
        } else {
            assert!(sink.bytes.is_empty())
        }
    }
}
