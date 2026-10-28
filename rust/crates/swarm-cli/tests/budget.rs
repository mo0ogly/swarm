#![cfg(unix)]
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use swarm_engine::{RootStore, SystemEnvironment};
/// Actual public CLI on private root, never the live server or a provider.
fn call(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}
/// Initialize and create Work through supported public native operations.
fn fixture() -> (tempfile::TempDir, String) {
    let root = tempfile::tempdir().unwrap();
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .arg(root.path())
        .status()
        .unwrap()
        .success());
    let mut store = RootStore::initialize(root.path(), &mut SystemEnvironment).unwrap();
    let work=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Budget","objective":"Proof","scope":"Private","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap().result.work;
    store.close().unwrap();
    (root, work.id)
}
/// Persist one bounded external request; caller may later change metadata to test replay.
fn request(root: &Path) -> std::path::PathBuf {
    let file = root.join("budget.json");
    fs::write(&file,br#"{"schema_version":1,"event_id":"budget","expected_revision":1,"budget":{"limit_usd":5,"reserve_per_launch_usd":1,"estimate_source":"Fixture","reference_date":"2026-10-19","actor":"FORGED","updated":"FORGED"}}"#).unwrap();
    file
}
/// CLI commits real policy with engine metadata, reopens and replays unchanged;
/// unported show/preview explicitly refuse without altering the actual database.
#[test]
fn budget_cli_apply_persists_replays_and_does_not_fake_unported_views() {
    let (root, id) = fixture();
    let file = request(root.path());
    let args = [
        "budget",
        "apply",
        &id,
        "--input",
        file.to_str().unwrap(),
        "--json",
    ];
    let first = call(root.path(), &args);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let work: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(work["revision"], 2);
    let db = rusqlite::Connection::open(root.path().join(".swarm/state.db")).unwrap();
    let raw: Vec<u8> = db
        .query_row("SELECT body FROM budgets WHERE work_id=?", [&id], |r| {
            r.get(0)
        })
        .unwrap();
    let budget: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(budget["limit_usd"], 5);
    assert_ne!(budget["actor"], "FORGED");
    assert_ne!(budget["updated"], "FORGED");
    db.close().unwrap();
    let before = fs::read(root.path().join(".swarm/state.db")).unwrap();
    let replay = call(root.path(), &args);
    assert!(replay.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&replay.stdout).unwrap()["revision"],
        2
    );
    for action in ["show", "preview"] {
        let refused = call(
            root.path(),
            &[
                "budget",
                action,
                &id,
                "--input",
                file.to_str().unwrap(),
                "--json",
            ],
        );
        assert_eq!(refused.status.code(), Some(2));
        assert!(refused.stdout.is_empty());
    }
    assert_eq!(
        fs::read(root.path().join(".swarm/state.db")).unwrap(),
        before
    );
}
/// Valid witness precedes invalid policy; refusal is nonzero without state changes,
/// and a new valid intent recovers through the same public path.
#[test]
fn budget_cli_invalid_policy_refuses_without_mutation_then_recovers() {
    let (root, id) = fixture();
    let file = request(root.path());
    let valid = fs::read(&file).unwrap();
    assert!(call(
        root.path(),
        &[
            "budget",
            "apply",
            &id,
            "--input",
            file.to_str().unwrap(),
            "--json"
        ]
    )
    .status
    .success());
    let before = fs::read(root.path().join(".swarm/state.db")).unwrap();
    let mut change: serde_json::Value = serde_json::from_slice(&valid).unwrap();
    change["event_id"] = "next".into();
    change["expected_revision"] = 2.into();
    change["budget"]["reference_date"] = "2023-02-29".into();
    fs::write(&file, serde_json::to_vec(&change).unwrap()).unwrap();
    let denied = call(
        root.path(),
        &[
            "budget",
            "apply",
            &id,
            "--input",
            file.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(denied.status.code(), Some(2));
    assert!(denied.stdout.is_empty());
    assert_eq!(
        fs::read(root.path().join(".swarm/state.db")).unwrap(),
        before
    );
    change["budget"]["reference_date"] = "2024-02-29".into();
    fs::write(&file, serde_json::to_vec(&change).unwrap()).unwrap();
    let recovered = call(
        root.path(),
        &[
            "budget",
            "apply",
            &id,
            "--input",
            file.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(recovered.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&recovered.stdout).unwrap()["revision"],
        3
    );
}
/// A real committed apply followed by write/flush failure stays an error and
/// reports committed state; replay recovers without adding another event.
#[test]
fn budget_cli_output_failure_retains_committed_state_and_replays() {
    struct Denied {
        flush: bool,
    }
    impl std::io::Write for Denied {
        fn write(&mut self, raw: &[u8]) -> std::io::Result<usize> {
            if self.flush {
                Ok(raw.len())
            } else {
                Err(std::io::Error::other("PRIVATE writer failure"))
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            if self.flush {
                Err(std::io::Error::other("PRIVATE flush failure"))
            } else {
                Ok(())
            }
        }
    }
    for flush in [false, true] {
        let (root, id) = fixture();
        let file = request(root.path());
        let args: Vec<String> = vec![
            "budget",
            "apply",
            &id,
            "--input",
            file.to_str().unwrap(),
            "--json",
            "--root",
            root.path().to_str().unwrap(),
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let mut stderr = Vec::new();
        let outcome = swarm_cli::run_detailed(&args, "fr", &mut Denied { flush }, &mut stderr);
        assert_eq!(outcome.code, 2);
        assert!(outcome.error.is_some());
        assert!(!format!("{outcome:?}").contains("PRIVATE"));
        let store = RootStore::open(root.path(), false).unwrap();
        assert_eq!(store.read_work(&id).unwrap().revision, 2);
        store.close().unwrap();
        let replay = call(
            root.path(),
            &[
                "budget",
                "apply",
                &id,
                "--input",
                file.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(replay.status.success());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&replay.stdout).unwrap()["revision"],
            2
        );
    }
}
