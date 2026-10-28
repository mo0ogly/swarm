#![cfg(unix)]
use std::{error::Error, fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{encode_model, register_identity_plan_projection, RootStore, SystemEnvironment};
struct Fixture(PathBuf);
impl Fixture {
    /// Private isolated root; the actual Work is created/started through supported operations.
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(swarm_engine::generate_identifier("history-root-").unwrap());
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap()
        }
    }
}
/// Keep real triggers/UDFs/foreign keys; this injects declared historical data, never an execution proof.
#[test]
fn root_history_uses_actual_stored_attempts_and_refuses_corruption_without_mutation_then_recovers()
{
    let f = Fixture::new();
    let mut store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    let w=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"History","objective":"Data only","scope":"Fixture","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap().result.work;
    let request = |event: &str, revision: i64, fields: serde_json::Value| {
        let mut v = fields;
        v["schema_version"] = 1.into();
        v["event_id"] = event.into();
        v["expected_revision"] = revision.into();
        serde_json::to_vec(&v).unwrap()
    };
    let w=store.execute_external(&w.id,"task.add",&request("task",w.revision,serde_json::json!({"id":"task","title":"Task","deliverable":"report.md","criteria":["Proof"]})),&mut SystemEnvironment).unwrap().result.work;
    assert!(store
        .inspect_control_history(&w.id, "task")
        .unwrap()
        .history
        .is_empty());
    let mut w = store
        .execute_external(
            &w.id,
            "task.update",
            &request(
                "start",
                w.revision,
                serde_json::json!({"id":"task","status":"running"}),
            ),
            &mut SystemEnvironment,
        )
        .unwrap()
        .result
        .work;
    let attempt = w.tasks.as_ref().unwrap()[0].attempts.as_ref().unwrap()[0]
        .id
        .clone();
    let raw=serde_json::to_vec(&serde_json::json!({"attempt_id":attempt,"revision":w.revision,"receipt":"historical","controls":[{"id":"check","executed":true,"passed":true,"exit_code":0}]})).unwrap();
    store.close().unwrap();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    register_identity_plan_projection(&db).unwrap();
    w.revision += 1;
    db.execute(
        "UPDATE works SET revision=?1,body=?2 WHERE id=?3",
        rusqlite::params![
            w.revision,
            String::from_utf8(encode_model(&w).unwrap()).unwrap(),
            w.id
        ],
    )
    .unwrap();
    db.execute("INSERT INTO events VALUES('history',?1,?2,'task.auto-validation','2026-10-18T00:00:00Z',?3,?3)",rusqlite::params![w.id,w.revision,raw]).unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    let before = encode_model(&store.read_work(&w.id).unwrap()).unwrap();
    let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    let events = store.events(&w.id).unwrap().len();
    let observation = store.inspect_control_history(&w.id, "task").unwrap();
    assert_eq!(observation.history.len(), 1);
    assert_eq!(observation.history[0].attempt_id, attempt);
    assert_eq!(observation.work_revision, w.revision);
    assert_eq!(
        store
            .inspect_control_history(&w.id, "missing")
            .err()
            .unwrap()
            .code(),
        "control-history-task-absent"
    );
    store.inspect_control_history(&w.id, "task").unwrap();
    db.execute(
        "UPDATE events SET payload=?1 WHERE id='history'",
        [b"{private-invalid-history".as_slice()],
    )
    .unwrap();
    let error = store.inspect_control_history(&w.id, "task").err().unwrap();
    assert_eq!(error.code(), "control-history-format");
    assert!(error.source().is_some());
    assert!(!format!("{error:?}").contains("private-invalid"));
    assert_eq!(
        encode_model(&store.read_work(&w.id).unwrap()).unwrap(),
        before
    );
    assert_eq!(store.events(&w.id).unwrap().len(), events);
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        settings
    );
    let stored: Vec<u8> = db
        .query_row("SELECT payload FROM events WHERE id='history'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(stored, b"{private-invalid-history");
    db.execute("UPDATE events SET payload=?1 WHERE id='history'", [raw])
        .unwrap();
    assert_eq!(
        store
            .inspect_control_history(&w.id, "task")
            .unwrap()
            .history
            .len(),
        1
    );
    store.close().unwrap();
    db.close().unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    assert_eq!(
        store
            .inspect_control_history(&w.id, "task")
            .unwrap()
            .history
            .len(),
        1
    );
    store.close().unwrap();
}
