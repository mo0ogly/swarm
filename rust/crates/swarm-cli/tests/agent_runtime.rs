#![cfg(unix)]
use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::{Command, Output},
};
use swarm_engine::{RootStore, SystemEnvironment};

/// Real default-profile CLI, pointed exclusively at this test's owned root.
fn call(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}

/// Initialize real native schema then seed one historical agent fixture.
/// No live mission, provider or process is touched; retained PIDs are passive.
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    RootStore::initialize(root.path(), &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    let db = rusqlite::Connection::open(root.path().join(".swarm/state.db")).unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    db.execute(
        "INSERT INTO works(id,revision,body) VALUES('work',1,?)",
        [r#"{"schema_version":1,"id":"work","revision":1,"criteria":["Proof"]}"#],
    )
    .unwrap();
    db.execute("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES('session','work','task','/fixture/work','running',?,?)",rusqlite::params![br#"{"id":"session","work_id":"work","task_id":"task","status":"queued","child_pid":123,"host":"other-host","prompt":"PRIVATE FIXTURE"}"#.as_slice(),b"{}".as_slice()]).unwrap();
    db.close().unwrap();
    root
}

/// Historical queued manual row in an owned schema; preparation/provider launch
/// are deliberately not simulated or claimed by this public cancellation recipe.
fn queued_fixture() -> tempfile::TempDir {
    let root = fixture();
    let db = rusqlite::Connection::open(root.path().join(".swarm/state.db")).unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    let work=br#"{"schema_version":1,"id":"work","revision":1,"criteria":["Proof"],"tasks":[{"id":"task","title":"Task","deliverable":"Report","criteria":["Proof"],"status":"running","attempts":[{"id":"attempt","status":"recorded","started":"2026-10-19T00:00:00Z","ended":""}]}]}"#;
    db.execute("UPDATE works SET body=? WHERE id='work'", [work.as_slice()])
        .unwrap();
    let mut agent = swarm_engine::Agent::default();
    agent.id = "session".into();
    agent.work_id = "work".into();
    agent.task_id = "task".into();
    agent.attempt_id = "attempt".into();
    agent.workspace = "/fixture/work".into();
    agent.status = "queued".into();
    agent.host = "other-host".into();
    db.execute(
        "UPDATE agents SET status='queued',body=? WHERE id='session'",
        [swarm_engine::encode_model(&agent).unwrap()],
    )
    .unwrap();
    db.execute(
        "INSERT INTO reservations VALUES('session','work',8.75,'reserved')",
        [],
    )
    .unwrap();
    db.close().unwrap();
    root
}

/// Actual CLI cancellation settles all records and replays without duplicates;
/// a registered supervisor refuses unchanged and no process is signalled.
#[test]
fn agent_cli_reconcile_cancels_unregistered_launch_once_and_refuses_registered() {
    let root = queued_fixture();
    for _ in 0..2 {
        let output = call(
            root.path(),
            &["agent", "reconcile", "session", "--json", "--lang", "en"],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["agent"]["status"], "interrupted");
        assert_eq!(value["cancelled_before_start"], true);
        assert_eq!(value["accepted_by_cancellation"], false);
    }
    let store = RootStore::open(root.path(), false).unwrap();
    let work = store.read_work("work").unwrap();
    assert_eq!(work.revision, 2);
    let task = &work.tasks.as_ref().unwrap()[0];
    assert_eq!(task.status, "blocked");
    assert_eq!(task.attempts.as_ref().unwrap()[0].status, "interrupted");
    store.close().unwrap();
    let db = rusqlite::Connection::open(root.path().join(".swarm/state.db")).unwrap();
    assert_eq!(
        db.query_row("SELECT state FROM reservations", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "released"
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM agent_logs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    db.close().unwrap();
    let registered = queued_fixture();
    let file = registered.path().join(".swarm/state.db");
    let db = rusqlite::Connection::open(&file).unwrap();
    db.execute("UPDATE agents SET status='starting',body=json_set(CAST(body AS TEXT),'$.status','starting','$.supervisor_pid',123)",[]).unwrap();
    db.close().unwrap();
    let before = fs::read(&file).unwrap();
    let output = call(
        registered.path(),
        &["agent", "reconcile", "session", "--json"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(file).unwrap(), before);
}

/// Checked write/flush failures after cancellation are nonzero, preserve the
/// outcome and produce safe diagnostics; readback and replay prove no duplicate.
#[test]
fn agent_cli_cancel_output_failures_preserve_settlement_and_replay() {
    struct Denied {
        flush: bool,
    }
    impl Write for Denied {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if self.flush {
                Ok(b.len())
            } else {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "PRIVATE OUTPUT"))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "PRIVATE FLUSH"))
        }
    }
    for flush in [false, true] {
        let root = queued_fixture();
        let args = vec![
            "agent".into(),
            "reconcile".into(),
            "session".into(),
            "--root".into(),
            root.path().to_string_lossy().into_owned(),
            "--json".into(),
            "--lang".into(),
            "en".into(),
        ];
        let mut error = Vec::new();
        let outcome = swarm_cli::run_detailed(&args, "en", &mut Denied { flush }, &mut error);
        assert_eq!(outcome.code, 2);
        assert!(matches!(
            outcome.error,
            Some(swarm_cli::CliError::AgentCancellationPreserved { .. })
        ));
        let message = String::from_utf8(error).unwrap();
        assert!(message.contains("Cancellation is recorded"));
        assert!(!message.contains("PRIVATE"));
        let store = RootStore::open(root.path(), false).unwrap();
        assert_eq!(store.read_agent("session").unwrap().status, "interrupted");
        assert_eq!(store.read_work("work").unwrap().revision, 2);
        store.close().unwrap();
        assert!(call(root.path(), &["agent", "reconcile", "session"])
            .status
            .success());
    }
}

/// Public show reports SQL state without claiming a live supervisor; stop only
/// records intention, survives reopen and adds no duplicate log. Invalid usage
/// and unsupported launch refuse without output or state mutation.
#[test]
fn agent_cli_show_and_stop_preserve_intent_without_claiming_termination() {
    let root = fixture();
    let file = root.path().join(".swarm/state.db");
    let before = fs::read(&file).unwrap();
    let shown = call(root.path(), &["agent", "show", "session", "--json"]);
    assert!(
        shown.status.success(),
        "{}",
        String::from_utf8_lossy(&shown.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(value["agent"]["status"], "running");
    assert_eq!(
        value["observed_status"],
        "unknown/native-supervision-unported"
    );
    assert_eq!(fs::read(&file).unwrap(), before);
    for args in [
        vec!["agent", "start", "work"],
        vec!["agent", "stop", "session", "--input", "ignored.json"],
    ] {
        let output = call(root.path(), &args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(fs::read(&file).unwrap(), before);
    }
    let stopped = call(
        root.path(),
        &["agent", "stop", "session", "--json", "--lang", "en"],
    );
    assert!(
        stopped.status.success(),
        "{}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&stopped.stdout).unwrap();
    assert_eq!(value["desired"], "stop");
    assert_eq!(value["termination_confirmed"], false);
    assert!(call(root.path(), &["agent", "stop", "session"])
        .status
        .success());
    let store = RootStore::open(root.path(), false).unwrap();
    let agent = store.read_agent("session").unwrap();
    assert_eq!(agent.status, "running");
    assert_eq!(agent.desired, "stop");
    store.close().unwrap();
    let db = rusqlite::Connection::open(file).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM agent_logs WHERE agent_id='session'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    db.close().unwrap();
}

/// Output failure after committed intention remains nonzero and explicitly
/// preserved, including flush failure; reopening proves intent, not process end.
#[test]
fn agent_cli_output_failures_preserve_recorded_stop_and_private_diagnostics() {
    struct Broken {
        flush: bool,
    }
    impl Write for Broken {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.flush {
                Ok(bytes.len())
            } else {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "PRIVATE OUTPUT"))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "PRIVATE FLUSH"))
        }
    }
    for flush in [false, true] {
        let root = fixture();
        let mut out = Broken { flush };
        let mut err = Vec::new();
        let args = vec![
            "agent".into(),
            "stop".into(),
            "session".into(),
            "--root".into(),
            root.path().to_string_lossy().into_owned(),
            "--json".into(),
            "--lang".into(),
            "en".into(),
        ];
        let result = swarm_cli::run_detailed(&args, "en", &mut out, &mut err);
        assert_eq!(result.code, 2);
        assert!(matches!(
            result.error,
            Some(swarm_cli::CliError::AgentStopPreserved { .. })
        ));
        let message = String::from_utf8(err).unwrap();
        assert!(message.contains("Stop intention is recorded"), "{message}");
        assert!(!message.contains("PRIVATE"));
        let store = RootStore::open(root.path(), false).unwrap();
        let agent = store.read_agent("session").unwrap();
        assert_eq!(agent.desired, "stop");
        assert_eq!(agent.status, "running");
        store.close().unwrap();
    }
}
