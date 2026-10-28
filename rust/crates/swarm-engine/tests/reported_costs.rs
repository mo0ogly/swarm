//! Stored-data observation fixtures, not fabricated runtime/consent/commit proof.
#![cfg(target_os = "linux")]
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{Agent, GoFloat, RootStore, SystemEnvironment, Usage};
struct Fixture(PathBuf);
impl Fixture {
    /// Publicly initialize/create a Work. SQL rows below model retained historical
    /// declarations only; they do not start providers or grant execution rights.
    fn new() -> (Self, RootStore, String) {
        let path =
            std::env::temp_dir().join(swarm_engine::generate_identifier("costs186-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let mut store = RootStore::initialize(&path, &mut SystemEnvironment).unwrap();
        let work=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Costs","objective":"Observe","scope":"Owned","criteria":["Partial remains partial"]}"#,&mut SystemEnvironment).unwrap().result.work.id;
        (Self(path), store, work)
    }
    /// Open only this fixture for retained-data seeding and negative corruption.
    fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.0.join(".swarm/state.db")).unwrap()
    }
    /// Seed declared historical data, explicitly without a canonical execution receipt.
    fn insert(&self, work: &str, id: &str, task: &str, provider: &str, cost: Option<f64>) {
        let mut agent = Agent::default();
        agent.id = id.into();
        agent.work_id = work.into();
        agent.task_id = task.into();
        agent.provider = provider.into();
        agent.workspace = self.0.to_str().unwrap().into();
        agent.status = "completed".into();
        agent.usage = cost.map(|v| {
            Box::new(Usage {
                provider_reported_cost_usd: Some(Box::new(GoFloat(v))),
                ..Default::default()
            })
        });
        let db = self.db();
        db.execute("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,'completed',?,?)",rusqlite::params![id,work,task,agent.workspace,swarm_engine::encode_model(&agent).unwrap(),b"{}".as_slice()]).unwrap();
        db.close().unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Valid Work with no attempts differs from unknown Work; reported zero differs
/// from no declaration. Groups and retained Work remain consistent and unchanged.
#[test]
fn reported_costs_empty_zero_unknown_and_groups_are_distinct() {
    let (fixture, store, work) = Fixture::new();
    let before = store.read_work(&work).unwrap();
    let empty = store.reported_costs(&work).unwrap();
    assert_eq!(empty.total.attempts_with_cost, 0);
    assert_eq!(empty.total.attempts_without_cost, 0);
    assert!(empty.by_task.is_empty());
    assert!(store.reported_costs("missing-work").is_err());
    fixture.insert(&work, "zero", "a", "one", Some(0.0));
    fixture.insert(&work, "priced", "a", "one", Some(2.5));
    fixture.insert(&work, "silent", "b", "two", None);
    let summary = store.reported_costs(&work).unwrap();
    assert_eq!(summary.total.reported_usd.0, 2.5);
    assert_eq!(summary.total.attempts_with_cost, 2);
    assert_eq!(summary.total.attempts_without_cost, 1);
    assert_eq!(summary.by_task["a"].attempts_with_cost, 2);
    assert_eq!(summary.by_task["b"].attempts_without_cost, 1);
    assert_eq!(summary.by_provider["one"].reported_usd.0, 2.5);
    assert_eq!(summary.by_provider["two"].attempts_with_cost, 0);
    assert_eq!(store.read_work(&work).unwrap(), before);
    store.close().unwrap();
    let reopened = RootStore::open(&fixture.0, false).unwrap();
    assert_eq!(reopened.reported_costs(&work).unwrap(), summary);
    reopened.close().unwrap();
}
/// More than the old recent-UI window must count; 205 is test input, no new cap.
#[test]
fn reported_costs_include_complete_history_beyond_recent_window() {
    let (fixture, store, work) = Fixture::new();
    for i in 0..205 {
        fixture.insert(&work, &format!("history-{i}"), "a", "one", Some(1.0));
    }
    let summary = store.reported_costs(&work).unwrap();
    assert_eq!(summary.total.reported_usd.0, 205.0);
    assert_eq!(summary.total.attempts_with_cost, 205);
    assert_eq!(summary.by_task["a"], summary.total);
    store.close().unwrap();
}
/// Each corruption follows a valid same-Root witness; no read failure becomes
/// empty success. Restore only owned injected data/schema and observe recovery.
#[test]
fn reported_costs_refuse_negative_corrupt_foreign_rows_and_sql_then_recover() {
    let (fixture, store, work) = Fixture::new();
    fixture.insert(&work, "good", "a", "one", Some(2.5));
    let witness = store.reported_costs(&work).unwrap();
    let before = store.read_work(&work).unwrap();
    fixture.insert(&work, "negative", "a", "one", Some(-1.0));
    assert!(store.reported_costs(&work).is_err());
    let db = fixture.db();
    db.execute("DELETE FROM agents WHERE id='negative'", [])
        .unwrap();
    assert_eq!(store.reported_costs(&work).unwrap(), witness);
    db.execute("UPDATE agents SET task_id='foreign' WHERE id='good'", [])
        .unwrap();
    assert!(store.reported_costs(&work).is_err());
    db.execute("UPDATE agents SET task_id='a' WHERE id='good'", [])
        .unwrap();
    assert_eq!(store.reported_costs(&work).unwrap(), witness);
    let raw: Vec<u8> = db
        .query_row(
            "SELECT CAST(body AS BLOB) FROM agents WHERE id='good'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    db.execute(
        "UPDATE agents SET body='PRIVATE_INVALID_COST_BODY' WHERE id='good'",
        [],
    )
    .unwrap();
    let error = store.reported_costs(&work).unwrap_err();
    assert!(!format!("{error}").contains("PRIVATE_INVALID_COST_BODY"));
    db.execute("UPDATE agents SET body=? WHERE id='good'", [raw])
        .unwrap();
    assert_eq!(store.reported_costs(&work).unwrap(), witness);
    db.execute_batch("ALTER TABLE agents RENAME TO owned_agents_fault")
        .unwrap();
    assert!(store.reported_costs(&work).is_err());
    db.execute_batch("ALTER TABLE owned_agents_fault RENAME TO agents")
        .unwrap();
    assert_eq!(store.reported_costs(&work).unwrap(), witness);
    assert_eq!(store.read_work(&work).unwrap(), before);
    db.close().unwrap();
    store.close().unwrap();
}
