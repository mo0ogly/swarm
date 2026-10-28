//! Agent runtime prerequisites on isolated SQLite fixtures. No provider/process.
use rusqlite::{params, Connection};
use std::error::Error;
use swarm_engine::{
    decode_agent, decode_agent_row, decode_launch, decode_providers, encode_model,
    execute_agent_stop, list_agents, read_agent,
};

/// Small actual-column fixture for readers/control only, not full Root schema.
fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE agents(id TEXT PRIMARY KEY, work_id TEXT NOT NULL, status TEXT NOT NULL, desired TEXT NOT NULL DEFAULT '', body BLOB NOT NULL); CREATE TABLE agent_logs(seq INTEGER PRIMARY KEY AUTOINCREMENT,agent_id TEXT NOT NULL,at TEXT NOT NULL,kind TEXT NOT NULL,message TEXT NOT NULL); PRAGMA user_version=37;").unwrap();
    db
}

/// Insert synthetic retained JSON with independent authoritative SQL lifecycle.
fn insert(db: &Connection, id: &str, status: &str, body: &[u8]) {
    db.execute(
        "INSERT INTO agents(id,work_id,status,body) VALUES(?,'work',?,?)",
        params![id, status, body],
    )
    .unwrap();
}

/// Value-struct omitempty remains present; nil/empty/pointers/strict input and
/// legacy unregistered claim must not fabricate identity or a live process.
#[test]
fn agent_contract_preserves_struct_nulls_and_authoritative_lifecycle() {
    let agent = decode_agent(br#"{}"#).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&encode_model(&agent).unwrap()).unwrap();
    assert!(value.get("diagnostic").unwrap().is_object());
    assert!(value["diagnostic"]["items"].is_null());
    assert!(value["args"].is_null());
    assert!(value.get("unregistered_claim").is_none());
    assert!(format!("{agent:?}").contains("redacted"));
    let row =
        decode_agent_row(br#"{"id":"session","status":"queued"}"#, "starting", "stop").unwrap();
    assert_eq!(row.status, "starting");
    assert_eq!(row.desired, "stop");
    assert!(row.has_unregistered_claim());
    assert!(row.is_active());
    let registered =
        decode_agent_row(br#"{"status":"queued","supervisor_pid":7}"#, "starting", "").unwrap();
    assert!(!registered.has_unregistered_claim());
    assert!(!decode_agent_row(b"{}", "completed", "")
        .unwrap()
        .is_active());
    assert!(decode_agent(br#"{"future":true}"#).is_ok());
    assert!(decode_launch(br#"{"future":true}"#).is_err());
    assert!(decode_providers(br#"{"future":true}"#).is_err());
    assert!(decode_launch(br#"{"timeout_seconds":1.5}"#).is_err());
    assert!(decode_agent(br#"{"child_pid":"PRIVATE"}"#).is_err());
    let providers = decode_providers(br#"{"schema_version":1,"providers":{"fake":{"command":"/fixture","args":[],"env_allow":null}}}"#).unwrap();
    let provider = &providers.providers.as_ref().unwrap()["fake"];
    assert_eq!(provider.args.as_ref().unwrap().len(), 0);
    assert!(provider.env_allow.is_none());
}

/// Complete history is independent of the UI200 window; one malformed row must
/// fail rather than hide missing consumption, then recover without rewriting it.
#[test]
fn agent_history_keeps_all_attempts_and_refuses_corruption_then_recovers() {
    let db = fixture();
    for index in 0..201 {
        let id = format!("session-{index}");
        let body = format!(
            r#"{{"id":"{id}","status":"queued","usage":{{"provider_reported_cost_usd":1}}}}"#
        );
        insert(&db, &id, "completed", body.as_bytes());
    }
    let agents = list_agents(&db, "work").unwrap();
    assert_eq!(agents.len(), 201);
    assert_eq!(agents.first().unwrap().id, "session-200");
    assert!(agents.iter().all(|agent| agent.status == "completed"));
    assert_eq!(
        agents
            .iter()
            .map(|a| a
                .usage
                .as_ref()
                .unwrap()
                .provider_reported_cost_usd
                .as_deref()
                .unwrap()
                .0)
            .sum::<f64>(),
        201.0
    );
    let original: Vec<u8> = db
        .query_row("SELECT body FROM agents WHERE id='session-17'", [], |r| {
            r.get(0)
        })
        .unwrap();
    db.execute(
        "UPDATE agents SET body=? WHERE id='session-17'",
        [b"{PRIVATE".as_slice()],
    )
    .unwrap();
    let error = list_agents(&db, "work").unwrap_err();
    assert!(error.source().is_some());
    assert!(!format!("{error:?}").contains("PRIVATE"));
    assert_eq!(
        db.query_row("SELECT count(*) FROM agents", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        201
    );
    db.execute(
        "UPDATE agents SET body=? WHERE id='session-17'",
        [&original],
    )
    .unwrap();
    assert_eq!(list_agents(&db, "work").unwrap().len(), 201);
    assert!(list_agents(&db, "absent-work").unwrap().is_empty());
    assert_eq!(
        read_agent(&db, "absent-session").unwrap_err().code(),
        "agent-missing"
    );
}

/// Durable stop is idempotent, never terminal. SQL-log failure rolls desired
/// back; fixed precondition then succeeds. Terminal/corrupt/foreign body refuse.
#[test]
fn agent_stop_is_idempotent_atomic_and_recovers_without_termination_claim() {
    let mut db = fixture();
    insert(
        &db,
        "first",
        "running",
        br#"{"id":"first","status":"queued","child_pid":123}"#,
    );
    let at = "2026-10-19T00:00:00Z";
    let stopped = execute_agent_stop(&mut db, "first", at).unwrap();
    assert_eq!(stopped.desired, "stop");
    assert_eq!(stopped.status, "running");
    assert_eq!(stopped.child_pid, 123);
    execute_agent_stop(&mut db, "first", at).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM agent_logs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    insert(
        &db,
        "second",
        "queued",
        br#"{"id":"second","status":"queued"}"#,
    );
    db.execute_batch("CREATE TRIGGER refuse_log BEFORE INSERT ON agent_logs BEGIN SELECT RAISE(FAIL,'PRIVATE LOG FAILURE'); END;").unwrap();
    let error = execute_agent_stop(&mut db, "second", at).unwrap_err();
    assert!(error.source().is_some());
    assert!(!error.to_string().contains("PRIVATE"));
    assert_eq!(read_agent(&db, "second").unwrap().desired, "");
    assert!(db.is_autocommit());
    db.execute_batch("DROP TRIGGER refuse_log;").unwrap();
    assert_eq!(
        execute_agent_stop(&mut db, "second", at).unwrap().desired,
        "stop"
    );
    db.execute("UPDATE agents SET status='completed' WHERE id='first'", [])
        .unwrap();
    assert_eq!(
        execute_agent_stop(&mut db, "first", at).unwrap_err().code(),
        "agent-already-terminal"
    );
    insert(&db, "mismatch", "queued", br#"{"id":"second"}"#);
    assert_eq!(
        execute_agent_stop(&mut db, "mismatch", at)
            .unwrap_err()
            .code(),
        "agent-row-identity-mismatch"
    );
    assert_eq!(
        read_agent(&db, "mismatch").unwrap_err().code(),
        "agent-row-identity-mismatch"
    );
    assert_eq!(
        db.query_row("SELECT desired FROM agents WHERE id='mismatch'", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap(),
        ""
    );
    assert_eq!(
        execute_agent_stop(&mut db, "mismatch", "PRIVATE")
            .unwrap_err()
            .code(),
        "agent-clock-invalid"
    );
    db.execute_batch("PRAGMA user_version=38;").unwrap();
    assert_eq!(
        execute_agent_stop(&mut db, "mismatch", at)
            .unwrap_err()
            .code(),
        "storage_version_conflict"
    );
    assert_eq!(
        read_agent(&db, "mismatch").unwrap_err().code(),
        "agent-row-identity-mismatch"
    );
    assert_eq!(
        db.query_row("SELECT desired FROM agents WHERE id='mismatch'", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap(),
        ""
    );
    db.execute_batch("PRAGMA user_version=37;").unwrap();
    db.execute(
        "UPDATE agents SET body=? WHERE id='mismatch'",
        [br#"{"id":"mismatch"}"#.as_slice()],
    )
    .unwrap();
    assert_eq!(
        execute_agent_stop(&mut db, "mismatch", at).unwrap().desired,
        "stop"
    );
}

/// Silent SQL no-ops must not turn an unrecorded intention or missing command
/// event into success. Each fault follows a valid same-input transaction rolled
/// back, then refuses without mutation and succeeds after the trigger is removed.
#[test]
fn agent_stop_refuses_silent_sql_noops_and_recovers() {
    for fault in [
        "CREATE TRIGGER fault BEFORE UPDATE OF desired ON agents WHEN new.desired='stop' BEGIN SELECT RAISE(IGNORE); END;",
        "CREATE TRIGGER fault BEFORE INSERT ON agent_logs BEGIN SELECT RAISE(IGNORE); END;",
        "CREATE TRIGGER fault AFTER INSERT ON agent_logs BEGIN DELETE FROM agent_logs WHERE seq=new.seq; END;",
        "CREATE TRIGGER fault AFTER UPDATE OF desired ON agents WHEN new.desired='stop' BEGIN UPDATE agents SET desired='' WHERE id=new.id; END;",
    ] {
        let mut db=fixture();
        insert(&db,"session","queued",br#"{"id":"session","status":"queued"}"#);
        let at="2026-10-19T00:00:00Z";
        let witness=db.transaction().unwrap();
        assert_eq!(swarm_engine::request_agent_stop(&witness,"session",at).unwrap().desired,"stop");
        witness.rollback().unwrap();
        assert_eq!(read_agent(&db,"session").unwrap().desired,"");
        db.execute_batch(fault).unwrap();
        assert!(execute_agent_stop(&mut db,"session",at).is_err(),"silent SQL refusal became success: {fault}");
        assert_eq!(read_agent(&db,"session").unwrap().desired,"");
        assert_eq!(db.query_row("SELECT count(*) FROM agent_logs",[],|r|r.get::<_,i64>(0)).unwrap(),0);
        db.execute_batch("DROP TRIGGER fault;").unwrap();
        assert_eq!(execute_agent_stop(&mut db,"session",at).unwrap().desired,"stop");
    }
}

/// Valid direct/list witnesses precede an alias body fault; reads refuse without
/// mutation, then recover after replacing only the owned malformed body.
#[test]
fn agent_readers_bind_body_identity_to_selected_sql_key_and_recover() {
    let db = fixture();
    insert(&db, "session", "completed", br#"{"id":"session"}"#);
    assert_eq!(read_agent(&db, "session").unwrap().id, "session");
    assert_eq!(list_agents(&db, "work").unwrap().len(), 1);
    insert(&db, "alias", "completed", br#"{"id":"session"}"#);
    let bytes: Vec<u8> = db
        .query_row("SELECT body FROM agents WHERE id='alias'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        read_agent(&db, "alias").unwrap_err().code(),
        "agent-row-identity-mismatch"
    );
    assert_eq!(
        list_agents(&db, "work").unwrap_err().code(),
        "agent-row-identity-mismatch"
    );
    assert_eq!(
        db.query_row("SELECT body FROM agents WHERE id='alias'", [], |r| r
            .get::<_, Vec<u8>>(0))
            .unwrap(),
        bytes
    );
    db.execute(
        "UPDATE agents SET body=? WHERE id='alias'",
        [br#"{"id":"alias"}"#.as_slice()],
    )
    .unwrap();
    assert_eq!(read_agent(&db, "alias").unwrap().id, "alias");
    assert_eq!(list_agents(&db, "work").unwrap().len(), 2);
}
