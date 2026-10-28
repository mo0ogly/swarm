#![cfg(target_os = "linux")]
use rusqlite::{params, Connection};
use std::{
    error::Error,
    fs,
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    sync::{Arc, Barrier},
};
use swarm_engine::{
    claim_agent_supervisor, decode_agent, encode_model, parse_process_start_stamp, Agent,
    RootStore, SupervisorIdentity, SystemEnvironment,
};

/// Complete authoritative agent columns and a preserved financial reservation.
/// This isolated memory database proves registration only, not launch admission.
fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE agents(id TEXT PRIMARY KEY,work_id TEXT NOT NULL,task_id TEXT NOT NULL,cwd TEXT NOT NULL,status TEXT NOT NULL,desired TEXT NOT NULL DEFAULT '',body BLOB NOT NULL,request BLOB NOT NULL); CREATE TABLE agent_logs(seq INTEGER PRIMARY KEY AUTOINCREMENT,agent_id TEXT NOT NULL,at TEXT NOT NULL,kind TEXT NOT NULL,message TEXT NOT NULL); CREATE TABLE reservations(agent_id TEXT PRIMARY KEY,amount REAL,state TEXT NOT NULL); PRAGMA user_version=37;").unwrap();
    seed(&db);
    db.execute(
        "INSERT INTO reservations VALUES('session',9.0,'reserved')",
        [],
    )
    .unwrap();
    db
}

/// Synthetic queued body uses the actual current host tuple and no provider.
fn queued() -> Agent {
    let mut agent = Agent::default();
    agent.id = "session".into();
    agent.work_id = "work".into();
    agent.task_id = "task".into();
    agent.workspace = "/isolated/fixture".into();
    agent.status = "queued".into();
    agent.host = SupervisorIdentity::current().unwrap().host().into();
    agent.prompt = "PRIVATE fixture".into();
    agent
}

/// Seed only an owned test database; never a public runtime mutation path.
fn seed(db: &Connection) {
    let body = encode_model(&queued()).unwrap();
    db.execute("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES('session','work','task','/isolated/fixture','queued',?,?)",params![body,b"{\"fixture\":true}".as_slice()]).unwrap();
}

/// Snapshot every session column and reservation to detect a refused mutation.
fn state(db: &Connection) -> (String, String, Vec<u8>, Vec<u8>, String, String, String) {
    db.query_row(
        "SELECT status,desired,body,request,work_id,task_id,cwd FROM agents WHERE id='session'",
        [],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        },
    )
    .unwrap()
}

/// Kernel ABI parsing handles parentheses/spaces, zombie and malformed fields;
/// the live registration comes from this actual process, never a supplied PID.
#[test]
fn supervisor_identity_observes_current_kernel_process_without_empty_fallback() {
    let raw = |status: &str, stamp: &str| {
        format!(
            "17 (nested ) name) {status} {} {stamp}",
            ["0"; 18].join(" ")
        )
    };
    assert_eq!(
        parse_process_start_stamp(&raw("S", "12345")).unwrap(),
        Some("12345".into())
    );
    assert_eq!(parse_process_start_stamp(&raw("Z", "12345")).unwrap(), None);
    for invalid in [
        "PRIVATE".into(),
        "17 (name) S 1".into(),
        raw("S", "PRIVATE"),
        raw("BAD", "10"),
    ] {
        let error = parse_process_start_stamp(&invalid).unwrap_err();
        assert!(error.source().is_some());
        assert!(!format!("{error:?}").contains("PRIVATE"));
    }
    let observed = SupervisorIdentity::current().unwrap();
    assert_eq!(observed.pid(), i64::from(std::process::id()));
    assert!(!observed.process_stamp().is_empty());
    assert!(observed.host().contains(":pid:["));
}

/// Starting state/PID/starttime/heartbeat publish together once. Durable stop
/// survives registration; no provider, task acceptance or reservation release.
#[test]
fn supervisor_claim_registers_once_and_preserves_stop_and_reservation() {
    let mut db = fixture();
    swarm_engine::execute_agent_stop(&mut db, "session", "2026-10-19T00:00:00Z").unwrap();
    let before = state(&db);
    let claim = claim_agent_supervisor(&mut db, "session").unwrap();
    let agent = claim.agent();
    assert_eq!(agent.status, "starting");
    assert_eq!(agent.desired, "stop");
    assert_eq!(agent.child_pid, 0);
    assert_eq!(agent.supervisor_pid, i64::from(std::process::id()));
    assert_eq!(agent.supervisor_identity, claim.identity().process_stamp());
    assert!(!agent.heartbeat.is_empty());
    let saved = state(&db);
    let body = decode_agent(&saved.2).unwrap();
    assert_eq!(body.status, "starting");
    assert_eq!(body.supervisor_pid, agent.supervisor_pid);
    assert_eq!(body.heartbeat, agent.heartbeat);
    assert_eq!(saved.3, before.3);
    assert_eq!(
        db.query_row(
            "SELECT state FROM reservations WHERE agent_id='session'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "reserved"
    );
    assert_eq!(
        claim_agent_supervisor(&mut db, "session")
            .unwrap_err()
            .code(),
        "agent-already-claimed"
    );
    assert_eq!(state(&db), saved);
    assert_eq!(
        db.query_row("SELECT count(*) FROM agent_logs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

/// Same valid fixture is witnessed before each malformed host/key/registration,
/// epoch and JSON variant; refusal preserves all stored session bytes.
#[test]
fn supervisor_claim_refuses_inconsistent_registration_without_mutation() {
    for fault in [
        "host",
        "id",
        "work",
        "task",
        "workspace",
        "body-status",
        "pid",
        "heartbeat",
        "epoch",
        "json",
    ] {
        let mut db = fixture();
        let mut witness = fixture();
        assert_eq!(state(&db), state(&witness));
        claim_agent_supervisor(&mut witness, "session").unwrap();
        if fault == "epoch" {
            db.execute_batch("PRAGMA user_version=38;").unwrap();
        } else if fault == "json" {
            db.execute("UPDATE agents SET body=?", [b"{PRIVATE".as_slice()])
                .unwrap();
        } else {
            let mut a = queued();
            match fault {
                "host" => a.host = "PRIVATE other host".into(),
                "id" => a.id = "other".into(),
                "work" => a.work_id = "other".into(),
                "task" => a.task_id = "other".into(),
                "workspace" => a.workspace = "/other".into(),
                "body-status" => a.status = "running".into(),
                "pid" => a.supervisor_pid = 99,
                "heartbeat" => a.heartbeat = "PRIVATE".into(),
                _ => unreachable!(),
            }
            db.execute("UPDATE agents SET body=?", [encode_model(&a).unwrap()])
                .unwrap();
        }
        let before = state(&db);
        let error = claim_agent_supervisor(&mut db, "session").unwrap_err();
        assert!(!format!("{error:?}").contains("PRIVATE"));
        assert_eq!(state(&db), before, "{fault}");
        assert!(db.is_autocommit());
    }
}

/// Silent update/body/control reversals and SQL errors cannot yield ownership.
/// A byte-identical valid witness precedes each fault, with recovery afterwards.
#[test]
fn supervisor_claim_refuses_silent_sql_noops_then_recovers() {
    for fault in [
        "CREATE TRIGGER fault BEFORE UPDATE OF status ON agents BEGIN SELECT RAISE(IGNORE); END;",
        "CREATE TRIGGER fault AFTER UPDATE OF status ON agents BEGIN UPDATE agents SET cwd='/changed' WHERE id=new.id; END;",
        "CREATE TRIGGER fault AFTER UPDATE OF status ON agents BEGIN UPDATE agents SET status='queued' WHERE id=new.id; END;",
        "CREATE TRIGGER fault AFTER UPDATE OF body ON agents BEGIN UPDATE agents SET body=x'7b7d' WHERE id=new.id; END;",
        "CREATE TRIGGER fault AFTER UPDATE OF status ON agents BEGIN UPDATE agents SET desired='stop' WHERE id=new.id; END;",
        "CREATE TRIGGER fault BEFORE UPDATE OF status ON agents BEGIN SELECT RAISE(FAIL,'PRIVATE SQL'); END;",
    ] {
        let mut db=fixture();let mut witness=fixture();assert_eq!(state(&db),state(&witness));claim_agent_supervisor(&mut witness,"session").unwrap();
        let before=state(&db);db.execute_batch(fault).unwrap();
        let error=claim_agent_supervisor(&mut db,"session").unwrap_err();assert!(!format!("{error:?}").contains("PRIVATE"));assert_eq!(state(&db),before);
        db.execute_batch("DROP TRIGGER fault;").unwrap();assert_eq!(claim_agent_supervisor(&mut db,"session").unwrap().agent().status,"starting");
    }
}

struct RootFixture(PathBuf);
impl RootFixture {
    /// Actual private native schema; only historical fixture rows are seeded.
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(swarm_engine::generate_identifier("native-claim-").unwrap());
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        RootStore::initialize(&path, &mut SystemEnvironment)
            .unwrap()
            .close()
            .unwrap();
        let db = Connection::open(path.join(".swarm/state.db")).unwrap();
        swarm_engine::register_identity_plan_projection(&db).unwrap();
        db.execute(
            "INSERT INTO works(id,revision,body) VALUES('work',1,?)",
            [r#"{"schema_version":1,"id":"work","revision":1,"criteria":["Proof"]}"#],
        )
        .unwrap();
        seed(&db);
        db.close().unwrap();
        Self(path)
    }
}
impl Drop for RootFixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}

/// Two actual Root connections compete once; one registration wins. Read-only
/// refuses unchanged. No test deadline, signal, provider or native orchestration.
#[test]
fn root_supervisor_claim_single_winner_and_readonly_refusal() {
    let fixture = RootFixture::new();
    let mut readonly = RootStore::open(&fixture.0, false).unwrap();
    assert_eq!(
        readonly.claim_agent("session").unwrap_err().code(),
        "root-read-only"
    );
    readonly.close().unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let mut threads = Vec::new();
    for _ in 0..2 {
        let path = fixture.0.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            let mut store = RootStore::open(&path, true).unwrap();
            barrier.wait();
            let result = store
                .claim_agent("session")
                .map(|claim| claim.agent().supervisor_pid)
                .map_err(|e| e.code().to_owned());
            store.close().unwrap();
            result
        }));
    }
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    for error in results.iter().filter_map(|r| r.as_ref().err()) {
        assert!(
            matches!(error.as_str(), "agent-already-claimed" | "agent-storage"),
            "{error}"
        );
    }
    let store = RootStore::open(&fixture.0, false).unwrap();
    let a = store.read_agent("session").unwrap();
    assert_eq!(a.status, "starting");
    assert_eq!(a.supervisor_pid, i64::from(std::process::id()));
    assert_eq!(a.child_pid, 0);
    store.close().unwrap();
}

/// The public Connection-only entry witnesses a healthy manual registration,
/// then refuses native origin or metadata before any registration/log/reservation
/// mutation. Synthetic native data here is a negative input, never authority.
#[test]
fn connection_only_claim_refuses_native_authority_without_mutation() {
    for metadata in [false, true] {
        let mut witness = fixture();
        assert_eq!(claim_agent_supervisor(&mut witness, "session").unwrap().agent().status, "starting");
        let mut db = fixture();
        let mut agent = queued();
        if metadata {
            agent.native_mission = Some(swarm_engine::GoRaw(serde_json::value::to_raw_value(&serde_json::json!({"schema_version":4})).unwrap()));
        } else { agent.origin = "conducteur".into(); }
        db.execute("UPDATE agents SET body=? WHERE id='session'", [encode_model(&agent).unwrap()]).unwrap();
        let before = state(&db);
        let changes = db.total_changes();
        let error = claim_agent_supervisor(&mut db, "session").unwrap_err();
        assert_eq!(error.code(), "mission-agent-root-required");
        assert_eq!(state(&db), before);
        assert_eq!(db.total_changes(), changes);
        assert_eq!(db.query_row("SELECT count(*) FROM agent_logs", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
        assert_eq!(db.query_row("SELECT state FROM reservations WHERE agent_id='session'", [], |r| r.get::<_, String>(0)).unwrap(), "reserved");
    }
}
