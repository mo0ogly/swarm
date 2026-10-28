use rusqlite::Connection;
use std::{io, path::Path};
use swarm_engine::{
    encode_model, read_recorded_work, EngineMutationLimits, GitObservation, GitState, RuntimeError,
    RuntimeOperation, TaskDefinitionLimits, ValidationPolicyLimits, WorkEngine, WorkEnvironment,
};
#[derive(Default)]
struct Environment {
    id_suffix: Option<&'static str>,
    calls: Vec<&'static str>,
    fail: Option<&'static str>,
}
impl Environment {
    fn call(&mut self, name: &'static str) -> Result<(), RuntimeError> {
        self.calls.push(name);
        if self.fail == Some(name) {
            Err(RuntimeError::new(
                RuntimeOperation::Clock,
                io::Error::other("PRIVATE environment failure"),
            ))
        } else {
            Ok(())
        }
    }
}
impl WorkEnvironment for Environment {
    fn identifier(&mut self, prefix: &str) -> Result<String, RuntimeError> {
        self.call("id")?;
        Ok(format!("{prefix}{}", self.id_suffix.unwrap_or("fixture")))
    }
    fn timestamp(&mut self) -> Result<String, RuntimeError> {
        self.call("clock")?;
        Ok("2026-10-18T00:00:00Z".into())
    }
    fn operator(&mut self) -> Result<String, RuntimeError> {
        self.call("operator")?;
        Ok("fixture/uid:1".into())
    }
    fn git(&mut self, _: &Path) -> Result<GitObservation, RuntimeError> {
        self.call("git")?;
        Ok(GitObservation {
            state: GitState {
                head: "fixture-head".into(),
                branch: "branch".into(),
                changes: "?? proof".into(),
            },
            issues: vec![],
        })
    }
}
fn limits() -> EngineMutationLimits {
    EngineMutationLimits {
        task: TaskDefinitionLimits::legacy_go(),
        policy: ValidationPolicyLimits::legacy_go(),
    }
}
fn fixture() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("PRAGMA user_version=37;PRAGMA foreign_keys=ON;CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,body BLOB NOT NULL);CREATE TABLE events(id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id),revision INTEGER NOT NULL,kind TEXT NOT NULL,at TEXT NOT NULL,payload BLOB NOT NULL,request BLOB NOT NULL,UNIQUE(work_id,revision));CREATE TABLE agents(work_id TEXT,task_id TEXT,status TEXT);CREATE TABLE cockpit_controls(work_id TEXT,paused);CREATE TABLE preparation_launch_locks(work_id TEXT,task_id TEXT,released INTEGER);").unwrap();
    c
}
fn execute(
    c: &mut Connection,
    e: &mut Environment,
    id: &str,
    kind: &str,
    r: serde_json::Value,
) -> Result<swarm_engine::WorkMutationOutcome, swarm_engine::WorkEngineError> {
    WorkEngine::new(c, Path::new("."), e, &limits()).execute_external(
        id,
        kind,
        &serde_json::to_vec(&r).unwrap(),
    )
}
fn creation() -> serde_json::Value {
    serde_json::json!({"schema_version":1,"event_id":"create","expected_revision":0,"title":"Title","objective":"Objective","scope":"Scope","criteria":["first","second"],"requirement_prerequisites":{"req-1":["req-2"]},"origin":"engine"})
}
fn create(c: &mut Connection, e: &mut Environment) {
    assert_eq!(
        execute(c, e, "", "work.create", creation())
            .unwrap()
            .work
            .revision,
        1
    );
}
#[test]
fn complete_creation_update_task_metadata_and_ooda_commit_and_reopen() {
    let mut c = fixture();
    let mut e = Environment::default();
    create(&mut c, &mut e);
    let requests = [
        ("work.update", serde_json::json!({"title":"Changed"})),
        (
            "task.add",
            serde_json::json!({"id":"task","title":"Task","deliverable":"report","criteria":["proof"]}),
        ),
        (
            "task.update",
            serde_json::json!({"id":"task","title":"New","criteria":["one","two"],"owner":"owner","next":"next"}),
        ),
        (
            "ooda",
            serde_json::json!({"observation":"obs","orientation":"orient","decision":"decide","owner":"owner","next":"next","result":"result"}),
        ),
    ];
    for (i, (kind, mut r)) in requests.into_iter().enumerate() {
        r["schema_version"] = 1.into();
        r["event_id"] = format!("event{i}").into();
        r["expected_revision"] = (i + 1).into();
        let result = execute(&mut c, &mut e, "w-fixture", kind, r).unwrap();
        assert!(!result.replayed);
        assert_eq!(
            encode_model(&result.work).unwrap(),
            encode_model(&read_recorded_work(&c, "w-fixture").unwrap()).unwrap()
        );
    }
    let w = read_recorded_work(&c, "w-fixture").unwrap();
    assert_eq!(w.revision, 5);
    assert_eq!(w.summary, "result");
    assert_eq!(w.git.head, "fixture-head");
    assert_eq!(w.tasks.as_ref().unwrap()[0].owner, "owner");
    let raw: Vec<u8> = c
        .query_row("SELECT request FROM events WHERE id='create'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(serde_json::from_slice::<serde_json::Value>(&raw)
        .unwrap()
        .get("origin")
        .is_none());
}
#[test]
fn replay_precedes_revision_runtime_and_business_rules_but_preserves_typed_intent() {
    let mut c = fixture();
    let mut e = Environment::default();
    create(&mut c, &mut e);
    let r = serde_json::json!({"schema_version":1,"event_id":"edit","expected_revision":1,"title":"Changed"});
    execute(&mut c, &mut e, "w-fixture", "work.update", r.clone()).unwrap();
    e.calls.clear();
    e.fail = Some("clock");
    assert!(
        execute(&mut c, &mut e, "w-fixture", "work.update", r.clone())
            .unwrap()
            .replayed
    );
    assert!(e.calls.is_empty());
    for key in ["criteria", "requirement_prerequisites"] {
        let mut changed = r.clone();
        changed[key] = if key == "criteria" {
            serde_json::json!([])
        } else {
            serde_json::json!({})
        };
        assert_eq!(
            execute(&mut c, &mut e, "w-fixture", "work.update", changed)
                .unwrap_err()
                .code(),
            "event_conflict"
        );
        assert!(e.calls.is_empty());
    }
    assert_eq!(read_recorded_work(&c, "w-fixture").unwrap().revision, 2);
}
#[test]
fn refusal_and_environment_failure_roll_back_then_same_event_recovers() {
    let mut c = fixture();
    let mut e = Environment::default();
    create(&mut c, &mut e);
    e.calls.clear();
    let invalid = serde_json::json!({"schema_version":1,"event_id":"repair","expected_revision":1,"title":" "});
    assert!(execute(&mut c, &mut e, "w-fixture", "work.update", invalid).is_err());
    assert!(e.calls.is_empty());
    let good = serde_json::json!({"schema_version":1,"event_id":"repair","expected_revision":1,"title":"Changed"});
    e.fail = Some("clock");
    let err = execute(&mut c, &mut e, "w-fixture", "work.update", good.clone()).unwrap_err();
    assert_eq!(err.code(), "runtime-clock");
    assert!(!format!("{err:?}").contains("PRIVATE"));
    assert!(!err.requires_reconciliation());
    assert_eq!(read_recorded_work(&c, "w-fixture").unwrap().revision, 1);
    e.fail = None;
    assert_eq!(
        execute(&mut c, &mut e, "w-fixture", "work.update", good)
            .unwrap()
            .work
            .revision,
        2
    );
}
#[test]
fn invalid_creation_and_checkpoint_do_not_run_git_and_unknown_schema_has_no_effect() {
    let mut c = fixture();
    let mut e = Environment::default();
    let mut r = creation();
    r["title"] = " ".into();
    assert!(execute(&mut c, &mut e, "", "work.create", r).is_err());
    assert!(!e.calls.contains(&"git"));
    create(&mut c, &mut e);
    e.calls.clear();
    let r = serde_json::json!({"schema_version":1,"event_id":"checkpoint","expected_revision":1,"summary":"summary","memory":["/outside"]});
    assert!(execute(&mut c, &mut e, "w-fixture", "checkpoint", r).is_err());
    assert!(e.calls.is_empty());
    assert_eq!(
        execute(
            &mut c,
            &mut e,
            "w-fixture",
            "work.update",
            serde_json::json!({"schema_version":2})
        )
        .unwrap_err()
        .code(),
        "invalid_schema"
    );
    assert!(e.calls.is_empty());
}
#[test]
fn guards_precede_changes_and_missing_requirement_cannot_start_task() {
    let mut c = fixture();
    let mut e = Environment::default();
    create(&mut c, &mut e);
    execute(&mut c,&mut e,"w-fixture","task.add",serde_json::json!({"schema_version":1,"event_id":"add","expected_revision":1,"id":"task","title":"Task","deliverable":"report","criteria":["proof"]})).unwrap();
    e.calls.clear();
    let r = serde_json::json!({"schema_version":1,"event_id":"running","expected_revision":2,"id":"task","status":"running"});
    c.execute(
        "INSERT INTO agents VALUES('w-fixture','task','running')",
        [],
    )
    .unwrap();
    assert_eq!(
        execute(&mut c, &mut e, "w-fixture", "task.update", r.clone())
            .unwrap_err()
            .code(),
        "active_agent"
    );
    c.execute("DELETE FROM agents", []).unwrap();
    assert_eq!(
        execute(&mut c, &mut e, "w-fixture", "task.update", r)
            .unwrap_err()
            .code(),
        "requirement_prerequisite"
    );
    assert!(e.calls.is_empty());
    assert_eq!(read_recorded_work(&c, "w-fixture").unwrap().revision, 2);
}
#[test]
fn deferred_commit_failure_is_explicitly_rolled_back_and_recovers_without_duplicate_event() {
    let mut c = fixture();
    let mut e = Environment::default();
    create(&mut c, &mut e);
    c.execute_batch("CREATE TABLE parent(id INTEGER PRIMARY KEY);CREATE TABLE deferred(value INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);CREATE TRIGGER defer_bad AFTER INSERT ON events WHEN NEW.id='commit-refusal' BEGIN INSERT INTO deferred VALUES(1);END").unwrap();
    let r = serde_json::json!({"schema_version":1,"event_id":"commit-refusal","expected_revision":1,"title":"Changed"});
    let err = execute(&mut c, &mut e, "w-fixture", "work.update", r.clone()).unwrap_err();
    assert_eq!(err.code(), "command-commit-refused");
    assert!(!err.requires_reconciliation());
    assert!(c.is_autocommit());
    assert_eq!(read_recorded_work(&c, "w-fixture").unwrap().revision, 1);
    assert_eq!(
        c.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    c.execute_batch("DROP TRIGGER defer_bad").unwrap();
    execute(&mut c, &mut e, "w-fixture", "work.update", r).unwrap();
}

/// Each operation has a valid public witness, atomic refusal and same-event
/// recovery. This uses a private SQLite fixture and executes no task/provider.
#[test]
fn operation_fields_refuse_ignored_values_and_recover_without_history_changes() {
    for (kind, valid, field, value) in [
        (
            "work.create",
            serde_json::json!({"title":"Title","objective":"Goal","scope":"Scope","criteria":["Proof"]}),
            "max_attempts",
            serde_json::json!(1),
        ),
        (
            "work.update",
            serde_json::json!({"title":"Changed"}),
            "owner",
            serde_json::json!("PRIVATE"),
        ),
        (
            "task.add",
            serde_json::json!({"id":"t","title":"Task","deliverable":"Report","criteria":["Proof"]}),
            "max_tool_calls",
            serde_json::json!(1),
        ),
        (
            "task.update",
            serde_json::json!({"id":"t","owner":"human"}),
            "objective",
            serde_json::json!("PRIVATE"),
        ),
        (
            "checkpoint",
            serde_json::json!({"summary":"Saved"}),
            "owner",
            serde_json::json!("PRIVATE"),
        ),
        (
            "ooda",
            serde_json::json!({"observation":"Fact","orientation":"Context","decision":"Proceed","owner":"human","next":"Continue"}),
            "summary",
            serde_json::json!("PRIVATE"),
        ),
    ] {
        let mut c = fixture();
        let mut e = Environment::default();
        let target = if kind == "work.create" {
            ""
        } else {
            create(&mut c, &mut e);
            "w-fixture"
        };
        if kind == "task.update" {
            execute(&mut c,&mut e,target,"task.add",serde_json::json!({"schema_version":1,"event_id":"task","expected_revision":1,"id":"t","title":"Task","deliverable":"Report","criteria":["Proof"]})).unwrap();
        }
        let rev = if target.is_empty() {
            0
        } else {
            read_recorded_work(&c, target).unwrap().revision
        };
        let mut good = valid.clone();
        good["schema_version"] = 1.into();
        good["event_id"] = "witness".into();
        good["expected_revision"] = rev.into();
        execute(&mut c, &mut e, target, kind, good).unwrap();
        let actual = read_recorded_work(&c, "w-fixture").unwrap();
        let count: i64 = c
            .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
            .unwrap();
        let mut repaired = valid;
        repaired["schema_version"] = 1.into();
        repaired["event_id"] = "repair".into();
        repaired["expected_revision"] = if target.is_empty() {
            0.into()
        } else {
            actual.revision.into()
        };
        if kind == "task.add" {
            repaired["id"] = "second".into();
        }
        let mut bad = repaired.clone();
        bad[field] = value;
        e.calls.clear();
        let err = execute(&mut c, &mut e, target, kind, bad).unwrap_err();
        assert_eq!(err.code(), "unsupported_parameter", "{kind}");
        assert!(!err.to_string().contains("PRIVATE"));
        assert!(e.calls.is_empty());
        assert_eq!(actual, read_recorded_work(&c, "w-fixture").unwrap());
        assert_eq!(
            count,
            c.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
                .unwrap()
        );
        if kind == "work.create" {
            // Allocate a distinct fixture ID without clearing any recorded receipt.
            e.id_suffix = Some("recovered");
        }
        execute(&mut c, &mut e, target, kind, repaired).unwrap();
    }
}

/// Actual pre-fix native CLI bytes preserve replay compatibility, without
/// retroactive policy application. Only this isolated fixture DB is populated.
#[test]
fn operation_fields_legacy_native_receipt_replays_but_new_ignored_policy_refuses() {
    let fixtures: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../internal/engine/testdata/legacy-ignored-request-55.json"
    ))
    .unwrap();
    let f = &fixtures["native-cli"];
    let w = &f["work"];
    let ev = &f["event"];
    let mut c = fixture();
    let mut e = Environment::default();
    c.execute(
        "INSERT INTO works(id,revision,body) VALUES(?,?,?)",
        rusqlite::params![
            w["id"].as_str().unwrap(),
            w["revision"].as_i64().unwrap(),
            w["body"].as_str().unwrap().as_bytes()
        ],
    )
    .unwrap();
    c.execute(
        "INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?,?,?,?,?,?,?)",
        rusqlite::params![
            ev["id"].as_str().unwrap(),
            ev["work_id"].as_str().unwrap(),
            ev["revision"].as_i64().unwrap(),
            ev["kind"].as_str().unwrap(),
            ev["at"].as_str().unwrap(),
            ev["payload"].as_str().unwrap().as_bytes(),
            ev["request"].as_str().unwrap().as_bytes()
        ],
    )
    .unwrap();
    let mut r: serde_json::Value = serde_json::from_str(ev["request"].as_str().unwrap()).unwrap();
    let id = w["id"].as_str().unwrap();
    let replay = execute(&mut c, &mut e, id, "task.add", r.clone()).unwrap();
    assert!(replay.replayed);
    assert!(e.calls.is_empty());
    r["event_id"] = "new-correctable-event".into();
    r["id"] = "new-task".into();
    r["expected_revision"] = w["revision"].clone();
    assert_eq!(
        execute(&mut c, &mut e, id, "task.add", r.clone())
            .unwrap_err()
            .code(),
        "unsupported_parameter"
    );
    assert_eq!(read_recorded_work(&c, id).unwrap(), replay.work);
    assert!(e.calls.is_empty());
    assert_eq!(
        c.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    r.as_object_mut().unwrap().remove("max_tool_calls");
    execute(&mut c, &mut e, id, "task.add", r).unwrap();
}

/// Rehydrate a pre-fix native receipt; new validation cannot erase history or
/// permit another invalid creation. New requests recover with the same event.
#[test]
fn work_creation_criteria_preserves_actual_legacy_receipt_and_refuses_new_blank() {
    let fixtures: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../internal/engine/testdata/legacy-blank-work-56.json"
    ))
    .unwrap();
    let f = &fixtures["native"];
    let w = &f["work"];
    let ev = &f["event"];
    let mut c = fixture();
    let mut e = Environment::default();
    c.execute(
        "INSERT INTO works(id,revision,body) VALUES(?,?,?)",
        rusqlite::params![
            w["id"].as_str().unwrap(),
            w["revision"].as_i64().unwrap(),
            w["body"].as_str().unwrap().as_bytes()
        ],
    )
    .unwrap();
    c.execute(
        "INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?,?,?,?,?,?,?)",
        rusqlite::params![
            ev["id"].as_str().unwrap(),
            ev["work_id"].as_str().unwrap(),
            ev["revision"].as_i64().unwrap(),
            ev["kind"].as_str().unwrap(),
            ev["at"].as_str().unwrap(),
            ev["payload"].as_str().unwrap().as_bytes(),
            ev["request"].as_str().unwrap().as_bytes()
        ],
    )
    .unwrap();
    let mut r: serde_json::Value = serde_json::from_str(ev["request"].as_str().unwrap()).unwrap();
    let replay = execute(&mut c, &mut e, "", "work.create", r.clone()).unwrap();
    assert!(replay.replayed);
    assert!(e.calls.is_empty());
    r["event_id"] = "new-correctable-event".into();
    assert_eq!(
        execute(&mut c, &mut e, "", "work.create", r.clone())
            .unwrap_err()
            .code(),
        "format"
    );
    assert!(!e.calls.contains(&"git"));
    assert_eq!(
        read_recorded_work(&c, w["id"].as_str().unwrap()).unwrap(),
        replay.work
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM works", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    r["criteria"] = serde_json::json!(["Meaningful proof"]);
    let repaired = execute(&mut c, &mut e, "", "work.create", r).unwrap();
    assert!(!repaired.replayed);
    assert_ne!(repaired.work.id, replay.work.id);
}

/// Actual transaction failure after candidate write rolls back Work/event;
/// same request recovers after removing only a TEMP fixture trigger. Local
/// inherit needs no provider and never rewrites attempts or reservations.
#[test]
fn task_model_sql_failure_rollback_replay_and_idle() {
    let mut c = fixture();
    let mut e = Environment::default();
    create(&mut c, &mut e);
    execute(&mut c,&mut e,"w-fixture","task.add",serde_json::json!({"schema_version":1,"event_id":"add","expected_revision":1,"id":"task","title":"Task","deliverable":"Report","criteria":["Proof"]})).unwrap();
    let settings = swarm_engine::EngineSettingsDocument::initial(
        swarm_engine::EngineSettingsValues::installation_defaults().unwrap(),
        "fixture".into(),
        "2026-10-19T00:00:00Z".into(),
    )
    .unwrap();
    let limits = limits();
    let run = |c: &mut Connection, e: &mut Environment, event: &str, rev: i64| {
        WorkEngine::new(c,Path::new("."),e,&limits).with_planning_settings(&settings).execute_external("w-fixture","task.model",&serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":event,"expected_revision":rev,"task_id":"task","inherit":true})).unwrap())
    };
    run(&mut c, &mut e, "witness", 2).unwrap();
    let before = read_recorded_work(&c, "w-fixture").unwrap();
    let count = c
        .query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
        .unwrap();
    c.execute_batch("CREATE TEMP TRIGGER task_model_fixture_refusal BEFORE INSERT ON main.events WHEN NEW.kind='task.model' BEGIN SELECT RAISE(ABORT,'PRIVATE event fault'); END;").unwrap();
    let error = run(&mut c, &mut e, "recover", 3).unwrap_err();
    assert!(!error.to_string().contains("PRIVATE"));
    assert!(c.is_autocommit());
    assert_eq!(read_recorded_work(&c, "w-fixture").unwrap(), before);
    assert_eq!(
        c.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        count
    );
    c.execute_batch("DROP TRIGGER task_model_fixture_refusal;")
        .unwrap();
    let changed = run(&mut c, &mut e, "recover", 3).unwrap();
    assert_eq!(changed.work.revision, 4);
    assert!(run(&mut c, &mut e, "recover", 3).unwrap().replayed);
    c.execute(
        "INSERT INTO agents(work_id,task_id,status) VALUES(?,?,?)",
        ["w-fixture", "task", "queued"],
    )
    .unwrap();
    assert_eq!(
        run(&mut c, &mut e, "active", 4).unwrap_err().code(),
        "task-model-active-attempt"
    );
    assert_eq!(read_recorded_work(&c, "w-fixture").unwrap(), changed.work);
    c.execute(
        "DELETE FROM agents WHERE work_id=? AND task_id=?",
        ["w-fixture", "task"],
    )
    .unwrap();
    assert_eq!(run(&mut c, &mut e, "active", 4).unwrap().work.revision, 5);
}

/// Saved policy and the normal transaction own quota history. A valid witness
/// precedes injected event failure; rollback/recovery/replay keep usage intact.
#[test]
fn quotas_transaction_rollback_replay_and_policy() {
    for configure_reviewer in [false,true] {
    println!("quota rollback fixture reviewer={configure_reviewer}");
    let mut c = fixture();
    let mut e = Environment::default();
    create(&mut c, &mut e);
    let mut work = read_recorded_work(&c, "w-fixture").unwrap();
    work.planning = swarm_engine::decode_work(br#"{"planning":{"version":1,"max_tasks":10,"max_decisions":10,"max_activations":20,"activations":2,"decisions":1,"scopes":[{"id":"root","revision":1,"activations":2,"requirements":["req-1","req-2"]}]}}"#).unwrap().planning;
    if configure_reviewer {
        work.planning.as_mut().unwrap().reviewer=Some(Box::new(swarm_engine::ReviewerConfig {
            provider:"fixture".into(),authorized:"supplied unit data".into(),max_calls:3,..Default::default()
        }));
    }
    swarm_engine::validate_planning_state(&work).unwrap();
    c.execute("UPDATE works SET body=? WHERE id=?", rusqlite::params![encode_model(&work).unwrap(),work.id]).unwrap();
    let settings = swarm_engine::EngineSettingsDocument::initial(
        swarm_engine::EngineSettingsValues::installation_defaults().unwrap(),
        "fixture".into(), "2026-10-21T00:00:00Z".into()).unwrap();
    let limits = limits();
    let run = |c: &mut Connection,e: &mut Environment,event: &str,rev: i64,reason: &str| {
        let mut change=swarm_engine::QuotaChange {
            schema_version:1,event_id:event.into(),expected_revision:rev,
            limits:swarm_engine::quota_view(&work).unwrap().limits,
            // Distinct fixture durations also participate in SQL rollback/replay.
            review_timeout_seconds:configure_reviewer.then(|| Box::new(rev)),
            reason:reason.into(),..Default::default()
        };
        change.limits.planning_activations=30;
        WorkEngine::new(c,Path::new("."),e,&limits).with_planning_settings(&settings)
            .execute_external("w-fixture","quotas.configure",&serde_json::to_vec(&change).unwrap())
    };
    let witness=run(&mut c,&mut e,"quota-witness",1,"Valid quota witness").unwrap();
    assert_eq!(witness.work.revision,2);
    assert_eq!(swarm_engine::quota_view(&witness.work).unwrap().review_timeout_seconds,configure_reviewer.then_some(1));
    assert_eq!(witness.work.planning.as_ref().unwrap().activations,2);
    let before=encode_model(&witness.work).unwrap();
    c.execute_batch("CREATE TEMP TRIGGER quota_fixture_refusal BEFORE INSERT ON main.events WHEN NEW.kind='quotas.configure' BEGIN SELECT RAISE(ABORT,'PRIVATE quota event fault'); END;").unwrap();
    let failed=run(&mut c,&mut e,"quota-recover",2,"Valid quota recovery").unwrap_err();
    assert!(!failed.to_string().contains("PRIVATE"));
    assert!(c.is_autocommit());
    assert_eq!(encode_model(&read_recorded_work(&c,"w-fixture").unwrap()).unwrap(),before);
    assert_eq!(c.query_row("SELECT count(*) FROM events WHERE kind='quotas.configure'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
    c.execute_batch("DROP TRIGGER quota_fixture_refusal;").unwrap();
    let recovered=run(&mut c,&mut e,"quota-recover",2,"Valid quota recovery").unwrap();
    assert_eq!(recovered.work.revision,3);
    assert_eq!(swarm_engine::quota_view(&recovered.work).unwrap().review_timeout_seconds,configure_reviewer.then_some(2));
    let calls=e.calls.len();
    assert!(run(&mut c,&mut e,"quota-recover",2,"Valid quota recovery").unwrap().replayed);
    assert_eq!(e.calls.len(),calls);
    assert!(run(&mut c,&mut e,"quota-stale",2,"Valid quota change").is_err());
    let denied=run(&mut c,&mut e,"quota-short",3,"abc").unwrap_err();
    assert!(denied.to_string().contains("quota-reason-outside-policy"));
    assert_eq!(encode_model(&read_recorded_work(&c,"w-fixture").unwrap()).unwrap(),encode_model(&recovered.work).unwrap());
    let auth=recovered.work.planning.as_ref().unwrap().quota_authorization.as_ref().unwrap();
    assert_eq!(auth.actor,"fixture/uid:1");
    assert_eq!(auth.reason,"Valid quota recovery");
    }
}
