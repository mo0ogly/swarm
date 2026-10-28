#![cfg(unix)]
use rusqlite::{types::Value, Connection};
use std::{collections::BTreeMap, fs, os::unix::fs::DirBuilderExt, path::PathBuf};
use swarm_engine::{encode_model, Agent, RootStore, SystemEnvironment};

struct Fixture(PathBuf);
impl Fixture {
    /// Real native schema and one unstarted manual attempt, all fixture-owned.
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(swarm_engine::generate_identifier("agent-cancel-").unwrap());
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        RootStore::initialize(&path, &mut SystemEnvironment)
            .unwrap()
            .close()
            .unwrap();
        let result = Self(path);
        let db = result.db();
        let work=br#"{"schema_version":1,"id":"work","revision":1,"criteria":["Proof"],"tasks":[{"id":"task","title":"Task","deliverable":"Report","criteria":["Proof"],"status":"running","attempts":[{"id":"attempt","status":"recorded","started":"2026-10-19T00:00:00Z","ended":""}]}]}"#;
        db.execute(
            "INSERT INTO works(id,revision,body) VALUES('work',1,?)",
            [work.as_slice()],
        )
        .unwrap();
        let mut agent = Agent::default();
        agent.id = "session".into();
        agent.work_id = "work".into();
        agent.task_id = "task".into();
        agent.attempt_id = "attempt".into();
        agent.workspace = "/isolated/fixture".into();
        agent.status = "queued".into();
        agent.host = "other-host".into();
        db.execute("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES('session','work','task','/isolated/fixture','queued',?,?)",rusqlite::params![encode_model(&agent).unwrap(),b"{}".as_slice()]).unwrap();
        db.execute(
            "INSERT INTO reservations VALUES('session','work',8.75,'reserved')",
            [],
        )
        .unwrap();
        db.close().unwrap();
        result
    }
    /// Fixture-only connection with the real projection UDF; never live state.
    fn db(&self) -> Connection {
        let db = Connection::open(self.0.join(".swarm/state.db")).unwrap();
        swarm_engine::register_identity_plan_projection(&db).unwrap();
        db
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}

/// Snapshot all actual table values, including projection/history/sequence data.
/// Static owned fixture names are quoted; no production query or mutation occurs.
fn snapshot(db: &Connection) -> BTreeMap<String, Vec<Vec<Value>>> {
    let mut names = db
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
        .unwrap();
    let names: Vec<String> = names
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut result = BTreeMap::new();
    for name in names {
        let mut statement = db
            .prepare(&format!("SELECT * FROM \"{}\"", name.replace('"', "\"\"")))
            .unwrap();
        let width = statement.column_count();
        let rows = statement
            .query_map([], |r| {
                (0..width)
                    .map(|index| r.get(index))
                    .collect::<rusqlite::Result<Vec<Value>>>()
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        result.insert(name, rows);
    }
    result
}

/// Cancellation closes only the matching attempt, blocks for review, releases
/// its estimate and persists one event/log together. It never accepts the task.
#[test]
fn queued_cancellation_atomically_settles_task_agent_and_reservation() {
    let fixture = Fixture::new();
    let mut store = RootStore::open(&fixture.0, true).unwrap();
    let agent = store
        .cancel_queued_agent("session", &mut SystemEnvironment)
        .unwrap();
    assert_eq!(agent.status, "interrupted");
    assert_eq!(agent.desired, "stop");
    assert_eq!(agent.supervisor_pid, 0);
    assert_eq!(agent.child_pid, 0);
    assert!(!agent.ended.is_empty());
    let work = store.read_work("work").unwrap();
    assert_eq!(work.revision, 2);
    let task = &work.tasks.as_ref().unwrap()[0];
    assert_eq!(task.status, "blocked");
    let attempt = &task.attempts.as_ref().unwrap()[0];
    assert_eq!(attempt.status, "interrupted");
    assert!(!attempt.ended.is_empty());
    store.close().unwrap();
    let db = fixture.db();
    assert_eq!(
        db.query_row("SELECT amount FROM reservations", [], |r| r
            .get::<_, f64>(0))
            .unwrap(),
        8.75
    );
    assert_eq!(
        db.query_row("SELECT state FROM reservations", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "released"
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM events WHERE kind='task.update'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM agent_logs WHERE kind='lifecycle'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    db.close().unwrap();
    let mut reopened = RootStore::open(&fixture.0, true).unwrap();
    assert_eq!(
        reopened.read_agent("session").unwrap().status,
        "interrupted"
    );
    let db = fixture.db();
    let before = snapshot(&db);
    assert_eq!(
        reopened
            .cancel_queued_agent("session", &mut SystemEnvironment)
            .unwrap()
            .status,
        "interrupted"
    );
    assert_eq!(snapshot(&db), before);
    db.close().unwrap();
    assert_eq!(reopened.read_work("work").unwrap().revision, 2);
    reopened.close().unwrap();
}

/// Each fault has an actual same-policy witness first; all table values survive
/// refusal, including a stage already applied before later SQL failure. Removing
/// the faulty trigger enables the original cancellation, with no budget reset.
#[test]
fn queued_cancellation_refuses_silent_sql_failures_and_recovers_all_state() {
    for fault in [
        "CREATE TRIGGER fault BEFORE UPDATE OF status ON agents BEGIN SELECT RAISE(IGNORE); END;",
        "CREATE TRIGGER fault AFTER UPDATE OF status ON agents BEGIN UPDATE agents SET cwd='/changed' WHERE id=new.id; END;",
        "CREATE TRIGGER fault BEFORE UPDATE OF state ON reservations BEGIN SELECT RAISE(IGNORE); END;",
        "CREATE TRIGGER fault BEFORE UPDATE OF state ON reservations BEGIN DELETE FROM reservations WHERE agent_id=old.agent_id; SELECT RAISE(IGNORE); END;",
        "CREATE TRIGGER fault BEFORE INSERT ON agent_logs BEGIN SELECT RAISE(IGNORE); END;",
        "CREATE TRIGGER fault BEFORE INSERT ON agent_logs BEGIN SELECT RAISE(FAIL,'PRIVATE SQL'); END;",
        "CREATE TRIGGER fault AFTER UPDATE OF status ON agents BEGIN UPDATE works SET body=x'7b7d' WHERE id=new.work_id; END;",
    ]{
        let witness=Fixture::new();let mut valid=RootStore::open(&witness.0,true).unwrap();valid.cancel_queued_agent("session",&mut SystemEnvironment).unwrap();valid.close().unwrap();
        let fixture=Fixture::new();let mut store=RootStore::open(&fixture.0,true).unwrap();let db=fixture.db();db.execute_batch(fault).unwrap();let before=snapshot(&db);
        let result=store.cancel_queued_agent("session",&mut SystemEnvironment);
        assert!(result.is_err(),"silent cancellation failure became success: {fault}");let error=result.unwrap_err();assert!(!error.to_string().contains("PRIVATE"));assert_eq!(snapshot(&db),before,"{fault}");
        db.execute_batch("DROP TRIGGER fault;").unwrap();assert_eq!(store.cancel_queued_agent("session",&mut SystemEnvironment).unwrap().status,"interrupted");store.close().unwrap();db.close().unwrap();
    }
}

/// A newer attempt and its running state are never ended by an older queued
/// agent. Cancellation still releases only the older unstarted reservation.
#[test]
fn queued_cancellation_preserves_newer_task_attempt() {
    let fixture = Fixture::new();
    let db = fixture.db();
    db.execute(
        "UPDATE works SET body=json_set(body,'$.tasks[0].attempts[0].id','newer-attempt')",
        [],
    )
    .unwrap();
    let mut store = RootStore::open(&fixture.0, true).unwrap();
    let before = store.read_work("work").unwrap();
    store
        .cancel_queued_agent("session", &mut SystemEnvironment)
        .unwrap();
    assert_eq!(store.read_work("work").unwrap(), before);
    store.close().unwrap();
    db.close().unwrap();
}

/// Registered or unproven legacy starting rows retain reservations. Only the
/// documented SQL-starting/body-queued gap plus durable stop can cancel.
#[test]
fn queued_cancellation_preserves_registered_agents_and_handles_legacy_stop_gap() {
    let fixture = Fixture::new();
    let db = fixture.db();
    db.execute("UPDATE agents SET status='starting'", [])
        .unwrap();
    let mut store = RootStore::open(&fixture.0, true).unwrap();
    let before = snapshot(&db);
    assert!(store
        .cancel_queued_agent("session", &mut SystemEnvironment)
        .is_err());
    assert_eq!(snapshot(&db), before);
    store.stop_agent("session", &mut SystemEnvironment).unwrap();
    assert_eq!(
        store
            .cancel_queued_agent("session", &mut SystemEnvironment)
            .unwrap()
            .status,
        "interrupted"
    );
    store.close().unwrap();
    db.close().unwrap();
    let registered = Fixture::new();
    let db = registered.db();
    db.execute(
        "UPDATE agents SET body=json_set(CAST(body AS TEXT),'$.supervisor_pid',123)",
        [],
    )
    .unwrap();
    let before = snapshot(&db);
    let mut store = RootStore::open(&registered.0, true).unwrap();
    assert!(store
        .cancel_queued_agent("session", &mut SystemEnvironment)
        .is_err());
    assert_eq!(snapshot(&db), before);
    store.close().unwrap();
    db.close().unwrap();
}

/// Reconcile repairs the historical window where the agent was cancelled before
/// task/budget/log settlement, preserving its original terminal timestamp/body.
#[test]
fn queued_cancellation_repairs_partial_historical_settlement() {
    let fixture = Fixture::new();
    let db = fixture.db();
    db.execute("UPDATE agents SET status='interrupted',desired='stop',body=json_set(CAST(body AS TEXT),'$.status','interrupted','$.desired','stop','$.stop_kind','operateur','$.ended','2026-10-18T00:00:00Z','$.activity','Historical cancellation')", []).unwrap();
    let original: Vec<u8> = db
        .query_row("SELECT CAST(body AS BLOB) FROM agents", [], |r| r.get(0))
        .unwrap();
    let mut store = RootStore::open(&fixture.0, true).unwrap();
    let agent = store
        .cancel_queued_agent("session", &mut SystemEnvironment)
        .unwrap();
    assert_eq!(agent.ended, "2026-10-18T00:00:00Z");
    assert_eq!(store.read_work("work").unwrap().revision, 2);
    assert_eq!(
        db.query_row("SELECT CAST(body AS BLOB) FROM agents", [], |r| r
            .get::<_, Vec<u8>>(0))
            .unwrap(),
        original
    );
    assert_eq!(
        db.query_row("SELECT state FROM reservations", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "released"
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM agent_logs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let before = snapshot(&db);
    store
        .cancel_queued_agent("session", &mut SystemEnvironment)
        .unwrap();
    assert_eq!(snapshot(&db), before);
    store.close().unwrap();
    db.close().unwrap();
}

/// Actual competing SQLite connections serialize claim versus cancellation.
/// Exactly one wins; the losing path cannot release a claimed reservation or
/// register a cancelled agent. No provider process, signal or timer is involved.
#[cfg(target_os = "linux")]
#[test]
fn queued_cancellation_and_supervisor_claim_have_one_consistent_winner() {
    use std::sync::{Arc, Barrier};
    let fixture = Fixture::new();
    let db = fixture.db();
    let identity = swarm_engine::SupervisorIdentity::current().unwrap();
    db.execute(
        "UPDATE agents SET body=json_set(CAST(body AS TEXT),'$.host',?)",
        [identity.host()],
    )
    .unwrap();
    db.close().unwrap();
    let mut first = RootStore::open(&fixture.0, true).unwrap();
    let mut second = RootStore::open(&fixture.0, true).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let other = Arc::clone(&barrier);
    let claim = std::thread::spawn(move || {
        other.wait();
        let result = first.claim_agent("session").is_ok();
        first.close().unwrap();
        result
    });
    barrier.wait();
    let cancelled = second
        .cancel_queued_agent("session", &mut SystemEnvironment)
        .is_ok();
    let claimed = claim.join().unwrap();
    assert_ne!(claimed, cancelled);
    let agent = second.read_agent("session").unwrap();
    let work = second.read_work("work").unwrap();
    let db = fixture.db();
    let state: String = db
        .query_row("SELECT state FROM reservations", [], |r| r.get(0))
        .unwrap();
    if cancelled {
        assert_eq!(agent.status, "interrupted");
        assert_eq!(agent.supervisor_pid, 0);
        assert_eq!(work.revision, 2);
        assert_eq!(state, "released");
    } else {
        assert_eq!(agent.status, "starting");
        assert_eq!(agent.supervisor_pid, std::process::id() as i64);
        assert_eq!(work.revision, 1);
        assert_eq!(state, "reserved");
    }
    second.close().unwrap();
    db.close().unwrap();
}
