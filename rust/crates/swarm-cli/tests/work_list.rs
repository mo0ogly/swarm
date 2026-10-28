#![cfg(target_os = "linux")]
use std::{fs, io, path::Path, process::Command};
use swarm_engine::{RootStore, SystemEnvironment};

/// Exercise the real public binary, with a private root and no provider or server.
fn call(root: &Path, language: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(["work", "list", "--json", "--lang", language, "--root"])
        .arg(root)
        .output()
        .unwrap()
}

/// Add through the supported native mutation path, not through fixture SQL.
fn create(store: &mut RootStore, event: &str, title: &str) -> swarm_engine::Work {
    let raw = serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":event,
        "expected_revision":0,"title":title,"objective":"Objective","scope":"Fixture",
        "criteria":["Proof"]}))
    .unwrap();
    store
        .execute_external("", "work.create", &raw, &mut SystemEnvironment)
        .unwrap()
        .result
        .work
}

/// Empty and populated lists preserve stored objects, reverse creation order,
/// FR/EN JSON and separator escaping. Reading changes no DB/settings bytes.
#[test]
fn public_work_list_reads_actual_stored_objects_in_reverse_order_without_mutation() {
    let root = tempfile::tempdir().unwrap();
    let mut store = RootStore::initialize(root.path(), &mut SystemEnvironment).unwrap();
    store.close().unwrap();
    let empty = call(root.path(), "fr");
    assert!(
        empty.status.success(),
        "{}",
        String::from_utf8_lossy(&empty.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&empty.stdout).unwrap(),
        serde_json::json!([])
    );
    store = RootStore::open(root.path(), true).unwrap();
    let first = create(&mut store, "first", "First <&> \u{2028} end");
    let second = create(&mut store, "second", "Deuxième \"projet\"");
    store.close().unwrap();
    let expected = serde_json::to_value(vec![second, first]).unwrap();
    let database = root.path().join(".swarm/state.db");
    let settings = root.path().join(".swarm/engine-settings.json");
    let before = fs::read(&database).unwrap();
    let policy = fs::read(&settings).unwrap();
    for language in ["fr", "en"] {
        let result = call(root.path(), language);
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
            expected
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("<&> \\u2028 end"));
        assert_eq!(fs::read(&database).unwrap(), before);
        assert_eq!(fs::read(&settings).unwrap(), policy);
    }
}

/// A damaged owned fixture DB refuses without empty success; original data and
/// failed bytes survive and restoration recovers the same actual Work.
#[test]
fn public_work_list_refuses_unreadable_database_and_recovers_without_losing_original() {
    let root = tempfile::tempdir().unwrap();
    let mut store = RootStore::initialize(root.path(), &mut SystemEnvironment).unwrap();
    let work = create(&mut store, "create", "Preserved Work");
    store.close().unwrap();
    let valid = call(root.path(), "fr");
    assert!(valid.status.success());
    let database = root.path().join(".swarm/state.db");
    let original = fs::read(&database).unwrap();
    let retained = root.path().join("original.db");
    fs::rename(&database, &retained).unwrap();
    fs::write(&database, b"private-damaged-fixture").unwrap();
    let failure = call(root.path(), "en");
    assert_eq!(failure.status.code(), Some(2));
    assert!(failure.stdout.is_empty());
    let diagnostic: serde_json::Value = serde_json::from_slice(&failure.stderr).unwrap();
    assert!(diagnostic["failure"]["code"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));
    assert!(!String::from_utf8_lossy(&failure.stderr).contains("private-damaged"));
    assert_eq!(fs::read(&database).unwrap(), b"private-damaged-fixture");
    assert_eq!(fs::read(&retained).unwrap(), original);
    fs::rename(&database, root.path().join("failed.db")).unwrap();
    fs::rename(&retained, &database).unwrap();
    let recovery = call(root.path(), "en");
    assert!(recovery.status.success());
    let observed: serde_json::Value = serde_json::from_slice(&recovery.stdout).unwrap();
    assert_eq!(observed[0]["id"], work.id);
    assert_eq!(recovery.stdout, valid.stdout);
    assert_eq!(fs::read(&database).unwrap(), original);
}

/// Validate syntax before opening a root; failed output must keep its original
/// IO cause and return failure. Invalid syntax never opens a missing root.
#[test]
fn public_work_list_rejects_unsupported_arguments_missing_roots_and_failed_output() {
    let absent = tempfile::tempdir().unwrap().path().join("absent");
    for args in [
        vec!["work", "list", "--json", "--input", "ignored"],
        vec!["work", "show"],
        vec!["work", "list", "extra", "--json"],
    ] {
        let mut args: Vec<String> = args.into_iter().map(String::from).collect();
        args.extend(["--root".into(), absent.to_str().unwrap().into()]);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(swarm_cli::run(&args, "fr", &mut out, &mut err), 2);
        assert!(out.is_empty());
        assert!(!absent.exists());
    }
    let failure = call(&absent, "fr");
    assert_eq!(failure.status.code(), Some(2));
    assert!(failure.stdout.is_empty());
    assert!(!absent.exists());
    let root = tempfile::tempdir().unwrap();
    RootStore::initialize(root.path(), &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    assert!(call(root.path(), "fr").status.success());
    struct Denied;
    impl io::Write for Denied {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("private-output-cause"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let args = [
        "work",
        "list",
        "--json",
        "--root",
        root.path().to_str().unwrap(),
    ]
    .map(String::from);
    let mut err = Vec::new();
    let result = swarm_cli::run_detailed(&args, "en", &mut Denied, &mut err);
    assert_eq!(result.code, 2);
    assert!(matches!(result.error, Some(swarm_cli::CliError::Output(_))));
    assert!(!String::from_utf8_lossy(&err).contains("private-output"));
    assert!(call(root.path(), "en").status.success());
}
