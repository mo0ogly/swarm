use rusqlite::{params, Connection};
use swarm_engine::{
    decode_work, encode_model, inspect_recorded_mutation, task_review_contract, MutationGuardError,
    MutationInspection,
};

fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("PRAGMA user_version=37;CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER,body BLOB);CREATE TABLE events(id TEXT PRIMARY KEY,work_id TEXT,kind TEXT,request BLOB);CREATE TABLE agents(work_id TEXT,task_id TEXT,status TEXT);CREATE TABLE cockpit_controls(work_id TEXT,paused);CREATE TABLE preparation_launch_locks(work_id TEXT,task_id TEXT,released INTEGER);").unwrap();
    db.execute(
        "INSERT INTO works VALUES('work',1,?)",
        params![br#"{"schema_version":1,"id":"work","revision":1,"tasks":[]}"#.as_slice()],
    )
    .unwrap();
    db
}

#[test]
fn guard_returns_existing_without_ending_transaction_and_replay_precedes_revision() {
    let mut db = fixture();
    let tx = db.transaction().unwrap();
    assert!(matches!(
        inspect_recorded_mutation(&tx, "work", "work.update", "new", 1, b"{}").unwrap(),
        MutationInspection::Existing(_)
    ));
    assert!(!tx.is_autocommit());
    tx.execute(
        "INSERT INTO events VALUES('recorded','work','task.update',x'ff')",
        [],
    )
    .unwrap();
    assert!(matches!(
        inspect_recorded_mutation(&tx, "work", "task.update", "recorded", -9, b"\xff").unwrap(),
        MutationInspection::Replayed(_)
    ));
    tx.rollback().unwrap();
}

fn inspect(
    db: &mut Connection,
    kind: &str,
    raw: &[u8],
) -> Result<MutationInspection, MutationGuardError> {
    let tx = db.transaction().unwrap();
    let result = inspect_recorded_mutation(&tx, "work", kind, "fresh", 1, raw);
    assert!(!tx.is_autocommit());
    tx.rollback().unwrap();
    result
}
fn hierarchy(db: &Connection) -> Vec<u8> {
    let raw=br#"{"schema_version":1,"id":"work","revision":1,"tasks":[{"id":"task","status":"blocked","scope_id":"root","title":"T","deliverable":"D","criteria":["old"],"depends":[]}],"planning":{"scopes":[{"id":"root","state":"ready"}]}}"#;
    db.execute("UPDATE works SET body=?", params![raw.as_slice()])
        .unwrap();
    db.execute("INSERT INTO cockpit_controls VALUES('work',1)", [])
        .unwrap();
    let w = decode_work(raw).unwrap();
    serde_json::to_vec(&serde_json::json!({"id":"task","criteria":["new"],"confirm_contract_revision":true,"contract_revision_reason":"Repair the blocked criterion", "expected_contract":task_review_contract(&w.tasks.as_ref().unwrap()[0]).unwrap()})).unwrap()
}

#[test]
fn revision_and_creation_checks_follow_replay_without_parsing_opaque_requests() {
    let mut db = fixture();
    let tx = db.transaction().unwrap();
    assert!(matches!(
        inspect_recorded_mutation(&tx, "work", "work.update", "fresh", 0, b"bad"),
        Err(MutationGuardError::Revision {
            observed: Some(1),
            ..
        })
    ));
    assert!(matches!(
        inspect_recorded_mutation(&tx, "", "work.create", "fresh", 1, b"bad"),
        Err(MutationGuardError::Revision { observed: None, .. })
    ));
    assert!(matches!(
        inspect_recorded_mutation(&tx, "", "work.create", "fresh", 0, b"bad").unwrap(),
        MutationInspection::Create
    ));
    tx.rollback().unwrap();
}

#[test]
fn all_active_states_block_work_and_definition_but_transition_targets_its_task() {
    let mut db = fixture();
    for status in ["queued", "starting", "running", "stopping"] {
        assert!(inspect(&mut db, "work.update", b"{}").is_ok());
        db.execute("INSERT INTO agents VALUES('work','other',?)", [status])
            .unwrap();
        assert!(matches!(
            inspect(&mut db, "work.update", b"{}"),
            Err(MutationGuardError::WorkActive)
        ));
        for request in [
            br#"{"id":"task","depends":[]}"#.as_slice(),
            br#"{"id":"task","criteria":[]}"#,
            br#"{"id":"task","max_tool_calls":-1}"#,
            br#"{"id":"task","validation_policy":{}}"#,
        ] {
            assert!(matches!(
                inspect(&mut db, "task.update", request),
                Err(MutationGuardError::ActiveAgent)
            ));
        }
        assert!(inspect(
            &mut db,
            "task.update",
            br#"{"id":"task","status":"blocked"}"#
        )
        .is_ok());
        assert!(inspect(
            &mut db,
            "task.update",
            br#"{"id":"task","owner":"next","depends":null}"#
        )
        .is_ok());
        db.execute("UPDATE agents SET task_id='task'", []).unwrap();
        assert!(matches!(
            inspect(
                &mut db,
                "task.submit",
                br#"{"id":"task","status":"submitted"}"#
            ),
            Err(MutationGuardError::ActiveAgent)
        ));
        db.execute("DELETE FROM agents", []).unwrap();
    }
    for status in ["completed", "failed", "interrupted", "", "RUNNING"] {
        db.execute("INSERT INTO agents VALUES('work','task',?)", [status])
            .unwrap();
        assert!(inspect(&mut db, "work.update", b"{}").is_ok());
        db.execute("DELETE FROM agents", []).unwrap();
    }
}

#[test]
fn hierarchy_is_immutable_and_revision_requires_public_pause_not_planning_flag() {
    let mut db = fixture();
    let raw = hierarchy(&db);
    for kind in ["work.update", "task.add"] {
        assert!(matches!(
            inspect(&mut db, kind, b"{}"),
            Err(MutationGuardError::Hierarchical)
        ));
    }
    assert!(inspect(&mut db, "task.update", &raw).is_ok());
    assert!(matches!(
        inspect(&mut db, "task.submit", &raw),
        Err(MutationGuardError::ContractRevision(_))
    ));
    db.execute("UPDATE cockpit_controls SET paused=0", [])
        .unwrap();
    assert!(matches!(
        inspect(&mut db, "task.update", &raw),
        Err(MutationGuardError::PauseRequired)
    ));
    db.execute("DELETE FROM cockpit_controls", []).unwrap();
    assert!(matches!(
        inspect(&mut db, "task.update", &raw),
        Err(MutationGuardError::PauseRequired)
    ));
}

#[test]
fn boolean_scan_matches_go_and_corrupt_pause_never_becomes_permission() {
    use rusqlite::types::Value;
    let mut db = fixture();
    let raw = hierarchy(&db);
    for v in [
        Value::Integer(1),
        Value::Text("True".into()),
        Value::Blob(b"TRUE".to_vec()),
    ] {
        db.execute("UPDATE cockpit_controls SET paused=?", [v])
            .unwrap();
        assert!(inspect(&mut db, "task.update", &raw).is_ok());
    }
    for v in [
        Value::Integer(2),
        Value::Real(1.0),
        Value::Null,
        Value::Text(" true".into()),
        Value::Blob(vec![255]),
    ] {
        db.execute("UPDATE cockpit_controls SET paused=1", [])
            .unwrap();
        assert!(inspect(&mut db, "task.update", &raw).is_ok());
        db.execute("UPDATE cockpit_controls SET paused=?", [v])
            .unwrap();
        let e = inspect(&mut db, "task.update", &raw).unwrap_err();
        assert!(matches!(e, MutationGuardError::Sql(_)));
        assert!(std::error::Error::source(&e).is_some());
    }
    db.execute("UPDATE cockpit_controls SET paused=1", [])
        .unwrap();
    assert!(inspect(&mut db, "task.update", &raw).is_ok());
    db.execute_batch("ALTER TABLE cockpit_controls RENAME TO saved_controls")
        .unwrap();
    assert!(matches!(
        inspect(&mut db, "task.update", &raw),
        Err(MutationGuardError::Sql(_))
    ));
    db.execute_batch("ALTER TABLE saved_controls RENAME TO cockpit_controls")
        .unwrap();
    assert!(inspect(&mut db, "task.update", &raw).is_ok());
}

#[test]
fn hierarchical_revision_rejects_each_contract_change_and_in_progress_review() {
    let mut db = fixture();
    let raw = hierarchy(&db);
    let valid: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    for (key, value) in [
        ("confirm_contract_revision", serde_json::json!(false)),
        ("contract_revision_reason", serde_json::json!("short")),
        ("expected_contract", serde_json::json!("stale")),
        ("criteria", serde_json::json!([])),
        ("depends", serde_json::json!([])),
        ("status", serde_json::json!("blocked")),
        ("title", serde_json::json!("changed")),
        ("deliverable", serde_json::json!("changed")),
        ("max_attempts", serde_json::json!(1)),
        ("max_tool_calls", serde_json::json!(1)),
        ("validation_policy", serde_json::json!({})),
    ] {
        assert!(inspect(&mut db, "task.update", &raw).is_ok());
        let mut invalid = valid.clone();
        invalid[key] = value;
        assert!(
            matches!(
                inspect(
                    &mut db,
                    "task.update",
                    &serde_json::to_vec(&invalid).unwrap()
                ),
                Err(MutationGuardError::ContractRevision(_))
            ),
            "{key}"
        );
    }
    let body: Vec<u8> = db
        .query_row("SELECT body FROM works", [], |r| r.get(0))
        .unwrap();
    let w = decode_work(&body).unwrap();
    for index in 0..5 {
        let mut invalid = w.clone();
        match index {
            0 => invalid.planning.as_mut().unwrap().repository = Some(Default::default()),
            1 => invalid.tasks.as_mut().unwrap()[0].status = "todo".into(),
            2 => invalid.tasks.as_mut().unwrap()[0].scope_id.clear(),
            3 => {
                invalid.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].state =
                    "closed".into()
            }
            _ => {
                invalid.tasks.as_mut().unwrap()[0].independent_review =
                    Some(Box::new(swarm_engine::IndependentReview {
                        state: "running".into(),
                        ..Default::default()
                    }))
            }
        };
        db.execute(
            "UPDATE works SET body=?",
            params![encode_model(&invalid).unwrap()],
        )
        .unwrap();
        assert!(matches!(
            inspect(&mut db, "task.update", &raw),
            Err(MutationGuardError::ContractRevision(_))
        ));
        db.execute("UPDATE works SET body=?", params![body])
            .unwrap();
        assert!(inspect(&mut db, "task.update", &raw).is_ok());
    }
}

#[test]
fn preparation_lock_precedes_agent_guard_and_released_lock_recovers() {
    let mut db = fixture();
    let raw = br#"{"id":"task","status":"running"}"#;
    assert!(inspect(&mut db, "task.update", raw).is_ok());
    db.execute(
        "INSERT INTO preparation_launch_locks VALUES('work','task',0)",
        [],
    )
    .unwrap();
    db.execute("INSERT INTO agents VALUES('work','task','running')", [])
        .unwrap();
    assert!(matches!(
        inspect(&mut db, "task.update", raw),
        Err(MutationGuardError::LaunchLocked)
    ));
    db.execute("UPDATE preparation_launch_locks SET released=1", [])
        .unwrap();
    assert!(matches!(
        inspect(&mut db, "task.update", raw),
        Err(MutationGuardError::ActiveAgent)
    ));
    db.execute("DELETE FROM agents", []).unwrap();
    assert!(inspect(&mut db, "task.update", raw).is_ok());
}

#[test]
fn metadata_uses_no_unneeded_sql_and_internal_decode_is_permissive_but_typed() {
    let mut db = fixture();
    db.execute_batch(
        "DROP TABLE agents;DROP TABLE cockpit_controls;DROP TABLE preparation_launch_locks",
    )
    .unwrap();
    for raw in [
        br#"{"id":"task","unknown":true,"owner":"next"}"#.as_slice(),
        b"null",
        br#"{"TITLE":null,"depends":null}"#,
    ] {
        assert!(inspect(&mut db, "task.update", raw).is_ok());
    }
    for raw in [
        b"[]".as_slice(),
        b"",
        br#"{"title":true}"#,
        br#"{"criteria":[1]}"#,
    ] {
        assert!(matches!(
            inspect(&mut db, "task.update", raw),
            Err(MutationGuardError::Request(_))
        ));
    }
    assert!(matches!(
        inspect(&mut db, "task.update", br#"{"depends":[]}"#),
        Err(MutationGuardError::Sql(_))
    ));
}

#[test]
fn main_guards_resist_temp_shadow_and_stale_transaction() {
    let mut db = fixture();
    let raw = hierarchy(&db);
    assert!(inspect(&mut db, "task.update", &raw).is_ok());
    db.execute_batch("UPDATE main.cockpit_controls SET paused=0;CREATE TEMP TABLE cockpit_controls(work_id TEXT,paused);INSERT INTO temp.cockpit_controls VALUES('work',1)").unwrap();
    assert!(matches!(
        inspect(&mut db, "task.update", &raw),
        Err(MutationGuardError::PauseRequired)
    ));
    let tx = db.transaction().unwrap();
    tx.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        inspect_recorded_mutation(&tx, "work", "task.update", "fresh", 1, &raw)
            .unwrap_err()
            .code(),
        "command-transaction-inactive"
    );
    drop(tx);
    assert!(db.is_autocommit());
}

#[test]
fn rejected_guard_preserves_prior_caller_changes_then_fresh_guard_and_storage_work() {
    let mut db = fixture();
    db.execute_batch(
        "CREATE TABLE notes(value TEXT);INSERT INTO agents VALUES('work','task','running')",
    )
    .unwrap();
    let tx = db.transaction().unwrap();
    tx.execute("INSERT INTO notes VALUES('prior')", []).unwrap();
    assert!(matches!(
        inspect_recorded_mutation(&tx, "work", "work.update", "new", 1, b"{}"),
        Err(MutationGuardError::WorkActive)
    ));
    assert_eq!(
        tx.query_row("SELECT count(*) FROM notes", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        tx.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    tx.rollback().unwrap();
    db.execute("DELETE FROM agents", []).unwrap();
    // Use the actual event schema required by storage after inspection.
    db.execute_batch("DROP TABLE events;CREATE TABLE events(id TEXT PRIMARY KEY,work_id TEXT,revision INTEGER,kind TEXT,at TEXT,payload BLOB,request BLOB)").unwrap();
    let mut tx = db.transaction().unwrap();
    let MutationInspection::Existing(mut candidate) =
        inspect_recorded_mutation(&tx, "work", "work.update", "new", 1, b"{}").unwrap()
    else {
        panic!("existing Work required")
    };
    candidate.revision = 2;
    candidate.title = "guard and storage".into();
    assert!(matches!(
        swarm_engine::stage_recorded_change(
            &mut tx,
            swarm_engine::RecordedChange {
                work: &candidate,
                expected_revision: 1,
                event: "new",
                kind: "work.update",
                request: b"{}"
            }
        )
        .unwrap(),
        swarm_engine::RecordedChangeOutcome::Staged
    ));
    tx.commit().unwrap();
    assert_eq!(
        swarm_engine::read_recorded_work(&db, "work")
            .unwrap()
            .revision,
        2
    );
}

#[test]
fn guard_refusals_keep_actionable_diagnostics_without_private_input_echo() {
    let mut db = fixture();
    let raw = hierarchy(&db);
    assert!(inspect(&mut db, "task.update", &raw).is_ok());
    let valid: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    for (key, value, reason) in [
        (
            "confirm_contract_revision",
            serde_json::json!(false),
            "contract-confirmation-required",
        ),
        (
            "contract_revision_reason",
            serde_json::json!("short"),
            "contract-reason-invalid",
        ),
        (
            "expected_contract",
            serde_json::json!("private submitted digest"),
            "contract-digest-stale",
        ),
        (
            "status",
            serde_json::json!("running"),
            "contract-separate-status-change-required",
        ),
        (
            "criteria",
            serde_json::json!([]),
            "contract-criterion-count-must-match",
        ),
        (
            "title",
            serde_json::json!("private submitted title"),
            "contract-other-fields-must-stay-unchanged",
        ),
    ] {
        let mut invalid = valid.clone();
        invalid["contract_revision_reason"] = serde_json::json!("private diagnostic fixture");
        invalid[key] = value;
        let e = inspect(
            &mut db,
            "task.update",
            &serde_json::to_vec(&invalid).unwrap(),
        )
        .unwrap_err();
        assert_eq!(e.code(), "format");
        assert_eq!(e.reason(), reason);
        assert!(e.to_string().contains(reason));
        assert!(!format!("{e:?}").contains("private"));
    }
}
