//! LIVE-UX telemetry ingest subjects: an agent id, `review:<job id>` or
//! `planning:<claim id>` is accepted only while its run is live. Job tables
//! are fixture stand-ins (key + `phase`, names from the engine DDL); no
//! runtime path writes them this way.
use rusqlite::Connection;
use std::{fs, path::PathBuf};
use swarm_engine::{RootStore, SystemEnvironment};

struct Root(PathBuf);
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn subjects_are_active_only_while_their_run_is_live() {
    let path = std::env::temp_dir().join(swarm_engine::generate_identifier("telemetry-subjects-").unwrap());
    fs::create_dir(&path).unwrap();
    let _owned = Root(path.clone());
    RootStore::initialize(&path, &mut SystemEnvironment).unwrap().close().unwrap();
    let store = RootStore::open(&path, false).unwrap();
    assert!(!store.telemetry_subject_active("nobody").unwrap(), "unknown agent");
    assert!(!store.telemetry_subject_active("review:none").unwrap(), "no such review job");
    assert!(!store.telemetry_subject_active("planning:none").unwrap(), "no such planning run");
    store.close().unwrap();
    // Job tables are created by a Root's first review/planning run; here a
    // minimal table with the real key and `phase` columns (names from the
    // engine DDL) stands for them.
    let db = Connection::open(path.join(".swarm/state.db")).unwrap();
    for (table, key) in [("native_review_jobs", "id"), ("native_planning_runs", "claim_id")] {
        db.execute_batch(&format!("CREATE TABLE {table}({key} TEXT PRIMARY KEY, phase TEXT NOT NULL)")).unwrap();
        for (id, phase) in [("live", "permitted"), ("unresolved", "unresolved"), ("done", "succeeded")] {
            db.execute(&format!("INSERT INTO {table}({key}, phase) VALUES (?1, ?2)"), [id, phase]).unwrap();
        }
    }
    drop(db);
    let store = RootStore::open(&path, false).unwrap();
    for prefix in ["review", "planning"] {
        let active = |id: &str| store.telemetry_subject_active(&format!("{prefix}:{id}")).unwrap();
        assert!(active("live") && active("unresolved"), "{prefix}: live phases accept");
        assert!(!active("done") && !active("other"), "{prefix}: finished or unknown run refuses");
    }
    assert!(!store.telemetry_subject_active("live").unwrap(), "an unprefixed id is an agent id");
    store.close().unwrap();
}

#[test]
fn runs_of_a_work_list_both_kinds_and_tolerate_missing_tables() {
    let path = std::env::temp_dir().join(swarm_engine::generate_identifier("telemetry-runs-").unwrap());
    fs::create_dir(&path).unwrap();
    let _owned = Root(path.clone());
    RootStore::initialize(&path, &mut SystemEnvironment).unwrap().close().unwrap();
    let store = RootStore::open(&path, false).unwrap();
    assert!(store.telemetry_runs("w1", None).unwrap().is_empty(), "no job table: empty, not an error");
    store.close().unwrap();
    // Minimal stand-ins with the engine column names read by the projection.
    let db = Connection::open(path.join(".swarm/state.db")).unwrap();
    db.execute_batch(
        "CREATE TABLE native_review_jobs(id TEXT PRIMARY KEY, work_id TEXT NOT NULL, task_id TEXT NOT NULL, phase TEXT NOT NULL);
         INSERT INTO native_review_jobs VALUES('r-old','w1','t1','recorded'),('r-new','w1','t2','permitted'),('r-other','w2','t1','permitted');",
    )
    .unwrap();
    drop(db);
    let store = RootStore::open(&path, false).unwrap();
    let runs = store.telemetry_runs("w1", None).unwrap();
    let seen: Vec<_> = runs.iter().map(|r| (r.subject.as_str(), r.target.as_str(), r.active)).collect();
    assert_eq!(seen, [("review:r-new", "t2", true), ("review:r-old", "t1", false)], "planning table absent, other Work excluded, newest first");
    store.close().unwrap();
    let db = Connection::open(path.join(".swarm/state.db")).unwrap();
    db.execute_batch(
        "CREATE TABLE native_planning_runs(claim_id TEXT PRIMARY KEY, work_id TEXT NOT NULL, scope_id TEXT NOT NULL, phase TEXT NOT NULL, prepared_at TEXT NOT NULL);
         INSERT INTO native_planning_runs VALUES('c1','w1','root','applied','2026-10-22T10:00:00Z'),('c2','w1','root','claimed','2026-10-22T11:00:00Z');",
    )
    .unwrap();
    drop(db);
    let store = RootStore::open(&path, false).unwrap();
    let runs = store.telemetry_runs("w1", None).unwrap();
    let subjects: Vec<_> = runs.iter().map(|r| r.subject.as_str()).collect();
    assert_eq!(subjects, ["review:r-new", "review:r-old", "planning:c2", "planning:c1"]);
    assert_eq!((runs[2].kind, runs[2].active, runs[2].prepared_at.as_deref()), ("planning", true, Some("2026-10-22T11:00:00Z")));
    let one = store.telemetry_runs("w1", Some("planning:c1")).unwrap();
    assert_eq!((one.len(), one[0].phase.as_str(), one[0].active), (1, "applied", false));
    assert!(store.telemetry_runs("w2", Some("planning:c1")).unwrap().is_empty(), "a subject of another Work is not returned");
    store.close().unwrap();
    // A malformed table is a read failure with its cause, never an empty list.
    let db = Connection::open(path.join(".swarm/state.db")).unwrap();
    db.execute_batch("DROP TABLE native_planning_runs; CREATE TABLE native_planning_runs(claim_id TEXT PRIMARY KEY);").unwrap();
    drop(db);
    let store = RootStore::open(&path, false).unwrap();
    let error = store.telemetry_runs("w1", None).unwrap_err();
    assert!(format!("{error:?}").contains("telemetry-runs-read"), "{error:?}");
    store.close().unwrap();
}
