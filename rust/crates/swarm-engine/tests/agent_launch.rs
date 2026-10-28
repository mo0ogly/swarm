//! Manual admission through the public Root API; isolated roots, no provider child.
#![cfg(target_os = "linux")]
use rusqlite::{types::Value, Connection};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
};
use swarm_engine::{decode_launch, Launch, RootStore, SystemEnvironment};
struct Fixture {
    root: PathBuf,
    work: String,
}
impl Fixture {
    /// Build actual schema, Work and task through public commands. A harmless
    /// local executable is observed only; it must never be started by admission.
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(swarm_engine::generate_identifier("launch75-").unwrap());
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let mut store = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
        let created=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Title","objective":"Goal","scope":"Scope","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap();
        let work = created.result.work.id;
        store.execute_external(&work,"task.add",br#"{"schema_version":1,"event_id":"add","expected_revision":1,"id":"task","title":"Task","deliverable":"Report","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap();
        store.close().unwrap();
        fs::write(
            root.join(".swarm/providers.json"),
            br#"{"schema_version":1,"providers":{"fixture":{"command":"/bin/cat","args":[]}}}"#,
        )
        .unwrap();
        fs::set_permissions(
            root.join(".swarm/providers.json"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        let f = Self { root, work };
        // Fixture-only financial envelope, not a live mutation or launch permission.
        let db = f.db();
        db.execute("INSERT INTO budgets(work_id,body) VALUES(?,?)",rusqlite::params![f.work,br#"{"limit_usd":10,"reserve_per_launch_usd":1,"estimate_source":"fixture estimate","reference_date":"2026-10-19"}"#.as_slice()]).unwrap();
        db.close().unwrap();
        f
    }
    /// Open only this owned fixture and register actual schema projection helpers.
    fn db(&self) -> Connection {
        let db = Connection::open(self.root.join(".swarm/state.db")).unwrap();
        swarm_engine::register_identity_plan_projection(&db).unwrap();
        db
    }
    /// Canonical raw launch remains distinct from normalized saved defaults.
    fn request(&self) -> Launch {
        decode_launch(br#"{"schema_version":1,"event_id":"session","expected_revision":2,"task_id":"task","provider":"fixture"}"#).unwrap()
    }
    /// Exercise the same factory preview and real admission both use publicly.
    fn run(
        &self,
        preview: bool,
    ) -> Result<swarm_engine::AgentLaunchOutcome, swarm_engine::RootStoreError> {
        let mut store = RootStore::open(&self.root, true).unwrap();
        let result = store.prepare_agent_launch(
            &self.work,
            &self.request(),
            preview,
            &mut SystemEnvironment,
        );
        store.close().unwrap();
        result
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
}
/// Snapshot all actual SQL values, including counters and projection tables.
fn snapshot(db: &Connection) -> BTreeMap<String, Vec<Vec<Value>>> {
    let names: Vec<String> = db
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    names
        .into_iter()
        .map(|name| {
            let mut stmt = db
                .prepare(&format!(
                    "SELECT * FROM \"{}\" ORDER BY rowid",
                    name.replace('"', "\"\"")
                ))
                .unwrap();
            let width = stmt.column_count();
            let rows = stmt
                .query_map([], |r| {
                    (0..width)
                        .map(|i| r.get(i))
                        .collect::<Result<Vec<Value>, _>>()
                })
                .unwrap()
                .map(Result::unwrap)
                .collect();
            (name, rows)
        })
        .collect()
}
/// Preview preserves every table; admission persists queued intent, reservation,
/// Work attempt and event once, then exact replay works after provider removal.
#[test]
fn manual_launch_preview_admission_replay_and_actual_workspace() {
    let f = Fixture::new();
    let before = snapshot(&f.db());
    let preview = f.run(true).unwrap();
    assert!(preview.preview_only);
    assert!(!preview.created);
    assert_eq!(snapshot(&f.db()), before);
    let launch = f.run(false).unwrap();
    assert!(launch.created);
    assert!(!launch.preview_only);
    assert_eq!(launch.agent.status, "queued");
    assert_eq!(launch.agent.child_pid, 0);
    assert!(launch
        .agent
        .prompt
        .contains(&format!("Racine de cette tentative: {}", f.root.display())));
    let db = f.db();
    assert_eq!(
        db.query_row(
            "SELECT amount FROM reservations WHERE agent_id='session'",
            [],
            |r| r.get::<_, f64>(0)
        )
        .unwrap(),
        1.0
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM events WHERE kind='agent.start'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    let recorded = snapshot(&db);
    db.close().unwrap();
    fs::remove_file(f.root.join(".swarm/providers.json")).unwrap();
    let replay = f.run(false).unwrap();
    assert!(!replay.created);
    assert_eq!(replay.agent.id, launch.agent.id);
    assert_eq!(snapshot(&f.db()), recorded);
}
/// A valid control precedes each late SQL fault. Final log triggers must not
/// erase the event or estimate and still return success; refusal rolls all back.
#[test]
fn manual_launch_late_sql_faults_refuse_without_mutation_and_recover() {
    for action in [
        "DELETE FROM reservations WHERE agent_id=NEW.agent_id;",
        "DELETE FROM events WHERE kind='agent.start';",
        "UPDATE reservations SET amount=9 WHERE agent_id=NEW.agent_id;",
    ] {
        let f = Fixture::new();
        assert!(f.run(true).is_ok());
        let db = f.db();
        let before = snapshot(&db);
        let mut store = RootStore::open(&f.root, true).unwrap();
        db.execute_batch(&format!(
            "CREATE TRIGGER fault AFTER INSERT ON agent_logs BEGIN {action} END;"
        ))
        .unwrap();
        let result =
            store.prepare_agent_launch(&f.work, &f.request(), false, &mut SystemEnvironment);
        assert!(result.is_err(), "late corruption accepted: {action}");
        assert_eq!(snapshot(&db), before);
        db.execute_batch("DROP TRIGGER fault").unwrap();
        store.close().unwrap();
        db.close().unwrap();
        assert!(f.run(false).unwrap().created);
    }
}

/// Invalid request revisions, hierarchy and provider scope never mutate. Pause
/// refusal is followed by explicit fixture unpause and successful admission.
#[test]
fn manual_launch_refuses_stale_unsupported_outside_and_paused_then_recovers() {
    let f = Fixture::new();
    assert!(f.run(true).is_ok());
    let before = snapshot(&f.db());
    for case in 0..5 {
        let mut r = f.request();
        match case {
            0 => r.expected_revision = 1,
            1 => r.parent = "foreign".into(),
            2 => r.workspace = "/tmp".into(),
            3 => r.provider_digest = "invalid".into(),
            _ => r.timeout_seconds = -1,
        }
        let mut store = RootStore::open(&f.root, true).unwrap();
        assert!(store
            .prepare_agent_launch(&f.work, &r, false, &mut SystemEnvironment)
            .is_err());
        store.close().unwrap();
        assert_eq!(snapshot(&f.db()), before);
    }
    let db = f.db();
    db.execute(
        "INSERT INTO cockpit_controls(work_id,paused) VALUES(?,1)",
        [&f.work],
    )
    .unwrap();
    let paused = snapshot(&db);
    assert!(f.run(false).is_err());
    assert_eq!(snapshot(&db), paused);
    db.execute(
        "UPDATE cockpit_controls SET paused=0 WHERE work_id=?",
        [&f.work],
    )
    .unwrap();
    db.close().unwrap();
    assert!(f.run(false).unwrap().created);
    let mut other = f.request();
    other.instruction = "changed request".into();
    let mut store = RootStore::open(&f.root, true).unwrap();
    let retained = snapshot(&f.db());
    assert!(store
        .prepare_agent_launch(&f.work, &other, false, &mut SystemEnvironment)
        .is_err());
    store.close().unwrap();
    assert_eq!(snapshot(&f.db()), retained);
}
/// Saved values reach the actual admission consumer after closing/reopening.
/// Compatibility0 refuses unverified providers; restoring1 permits the same path.
#[test]
fn manual_launch_saved_policy_reopen_and_consumer_recovery() {
    use swarm_engine::{EngineSettingsChange, SettingCap, SettingCount};
    let f = Fixture::new();
    assert!(f.run(true).is_ok());
    for compatibility in [0, 1] {
        let store = RootStore::open(&f.root, true).unwrap();
        let doc = store.settings().unwrap();
        let mut values = doc.values;
        values.agent_allow_unverified_provider_context = Some(SettingCount(compatibility));
        values.agent_launch_timeout_seconds_default = Some(SettingCount(17));
        values.agent_launch_timeout_seconds_ceiling = Some(SettingCap(None));
        values.agent_environment_allow = Some(vec![]);
        let mut c = EngineSettingsChange {
            schema_version: doc.schema_version,
            event_id: format!("policy-{compatibility}"),
            expected_revision: doc.revision,
            values,
            reason: "Fixture operator policy".into(),
            preview_token: String::new(),
        };
        c.preview_token = store.preview_settings(&c).unwrap().preview_token;
        store.apply_settings(&c, &mut SystemEnvironment).unwrap();
        store.close().unwrap();
        let before = snapshot(&f.db());
        let outcome = f.run(true);
        if compatibility == 0 {
            assert!(outcome.is_err());
        } else {
            let result = outcome.unwrap();
            assert_eq!(result.agent.timeout_seconds, 17);
            assert_eq!(result.agent.env_allow, Some(vec![]));
        }
        assert_eq!(snapshot(&f.db()), before);
    }
    assert!(f.run(false).unwrap().created);
}
/// Concurrent factories create one intent or exhaust the saved finite SQLite
/// policy. Once the peer commits, explicit replay returns that one intention;
/// no timing assumption, extra retry budget or duplicate estimate is invented.
#[test]
fn manual_launch_concurrent_same_intention_records_once() {
    use std::sync::{Arc, Barrier};
    let f = Fixture::new();
    let barrier = Arc::new(Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let root = f.root.clone();
            let work = f.work.clone();
            let request = f.request();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut store = RootStore::open(&root, true).unwrap();
                barrier.wait();
                let result =
                    store.prepare_agent_launch(&work, &request, false, &mut SystemEnvironment);
                store.close().unwrap();
                result.map(|v| v.created)
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|v| matches!(v, Ok(true))).count(), 1);
    for result in &results {
        if let Err(error) = result {
            assert_eq!(error.code(), "agent-launch-storage-busy");
            assert_eq!(sqlite_failure(error).map(|v| v & 255), Some(5));
        }
    }
    // A committed peer is now a changed, observable precondition; no hidden
    // retry deadline or assumption that a finite wait always succeeds.
    assert!(!f.run(false).unwrap().created);
    let db = f.db();
    for table in ["agents", "reservations", "agent_logs"] {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM events WHERE kind='agent.start'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}

/// A catalogue appearing during an engine callback changes the combined source
/// fingerprint even when the new map is valid/empty. The earlier valid preview
/// controls the refusal: no SQL mutation, then removing only that file recovers.
#[test]
fn manual_launch_rechecks_new_catalogue_after_environment_callback() {
    use swarm_engine::{GitObservation, RuntimeError, WorkEnvironment};
    struct Changed {
        root: PathBuf,
    }
    impl WorkEnvironment for Changed {
        fn identifier(&mut self, prefix: &str) -> Result<String, RuntimeError> {
            if prefix == "launch-" {
                let path = self.root.join(".swarm/ai-connections.json");
                fs::write(&path, b"{}").unwrap();
                fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
            }
            WorkEnvironment::identifier(&mut SystemEnvironment, prefix)
        }
        fn timestamp(&mut self) -> Result<String, RuntimeError> {
            WorkEnvironment::timestamp(&mut SystemEnvironment)
        }
        fn operator(&mut self) -> Result<String, RuntimeError> {
            WorkEnvironment::operator(&mut SystemEnvironment)
        }
        fn git(&mut self, root: &std::path::Path) -> Result<GitObservation, RuntimeError> {
            WorkEnvironment::git(&mut SystemEnvironment, root)
        }
    }
    let f = Fixture::new();
    assert!(f.run(true).is_ok());
    let before = snapshot(&f.db());
    let mut store = RootStore::open(&f.root, true).unwrap();
    let result = store.prepare_agent_launch(
        &f.work,
        &f.request(),
        false,
        &mut Changed {
            root: f.root.clone(),
        },
    );
    store.close().unwrap();
    assert_eq!(
        result.err().expect("source change must refuse").code(),
        "agent-provider-catalogue-changed"
    );
    assert_eq!(snapshot(&f.db()), before);
    fs::remove_file(f.root.join(".swarm/ai-connections.json")).unwrap();
    assert!(f.run(false).unwrap().created);
}

/// Actual retained hold/archive, occupied workspace, exhausted estimate and an
/// unreadable pause each reject unchanged; restoring only the faulty fixture
/// precondition permits this same launch. Every injection has its valid control.
#[test]
fn manual_launch_retained_guards_refuse_without_consumption_and_recover() {
    for case in 0..5 {
        let f = Fixture::new();
        assert!(f.run(true).is_ok());
        let db = f.db();
        let original: Vec<u8> = db
            .query_row("SELECT body FROM works WHERE id=?", [&f.work], |r| r.get(0))
            .unwrap();
        match case {
            0 => {
                let mut w = swarm_engine::decode_work(&original).unwrap();
                w.tasks.as_mut().unwrap()[0].launch_held = true;
                db.execute(
                    "UPDATE works SET body=? WHERE id=?",
                    rusqlite::params![swarm_engine::encode_model(&w).unwrap(), f.work],
                )
                .unwrap();
            }
            1 => {
                db.execute("INSERT INTO mission_lifecycle VALUES(?,'2026-10-19T00:00:00Z','fixture','fixture')",[&f.work]).unwrap();
            }
            2 => {
                let mut agent = swarm_engine::Agent::default();
                agent.id = "busy".into();
                agent.work_id = f.work.clone();
                agent.task_id = "other".into();
                agent.workspace = f.root.to_str().unwrap().into();
                agent.status = "running".into();
                db.execute("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES('busy',?,'other',?,'running',?,?)",rusqlite::params![f.work,agent.workspace,swarm_engine::encode_model(&agent).unwrap(),b"{}".as_slice()]).unwrap();
            }
            3 => {
                db.execute("UPDATE budgets SET body=? WHERE work_id=?",rusqlite::params![br#"{"limit_usd":0.5,"reserve_per_launch_usd":1,"estimate_source":"fixture","reference_date":"2026-10-19"}"#.as_slice(),f.work]).unwrap();
            }
            _ => {
                db.execute(
                    "INSERT INTO cockpit_controls(work_id,paused) VALUES(?,'PRIVATE')",
                    [&f.work],
                )
                .unwrap();
            }
        }
        let before = snapshot(&db);
        let result = f.run(false);
        assert!(result.is_err(), "guard {case} accepted");
        assert!(!format!("{:?}", result.err().unwrap()).contains("PRIVATE"));
        assert_eq!(snapshot(&db), before);
        match case {
            0 => {
                db.execute(
                    "UPDATE works SET body=? WHERE id=?",
                    rusqlite::params![original, f.work],
                )
                .unwrap();
            }
            1 => {
                db.execute("DELETE FROM mission_lifecycle WHERE work_id=?", [&f.work])
                    .unwrap();
            }
            2 => {
                db.execute("DELETE FROM agents WHERE id='busy'", [])
                    .unwrap();
            }
            3 => {
                db.execute("UPDATE budgets SET body=? WHERE work_id=?",rusqlite::params![br#"{"limit_usd":10,"reserve_per_launch_usd":1,"estimate_source":"fixture","reference_date":"2026-10-19"}"#.as_slice(),f.work]).unwrap();
            }
            _ => {
                db.execute("DELETE FROM cockpit_controls WHERE work_id=?", [&f.work])
                    .unwrap();
            }
        }
        db.close().unwrap();
        assert!(f.run(false).unwrap().created);
    }
}
/// Malformed/oversized-scope provider data and unported field-guide context are
/// never silently ignored. A restored catalogue recovers with no extra attempt.
#[test]
fn manual_launch_provider_and_unported_context_refuse_then_recover() {
    let f = Fixture::new();
    assert!(f.run(true).is_ok());
    let before = snapshot(&f.db());
    let path = f.root.join(".swarm/providers.json");
    let original = fs::read(&path).unwrap();
    for raw in [b"{PRIVATE".as_slice(),br#"{"schema_version":1,"providers":{"fixture":{"command":"/bin/cat","preflight_ttl_seconds":301}}}"#,br#"{"schema_version":1,"providers":{"fixture":{"command":"/bin/cat","env_allow":["BAD=NAME"]}}}"#,br#"{"schema_version":1,"providers":{"fixture":{"command":"/bin/cat","preflight_required":true}}}"#] {
        assert!(f.run(true).is_ok());fs::write(&path,raw).unwrap();assert!(f.run(false).is_err());assert_eq!(snapshot(&f.db()),before);fs::write(&path,&original).unwrap();
    }
    let guide = f.root.join(".claude/field-guide");
    fs::create_dir_all(&guide).unwrap();
    fs::write(guide.join("index.md"), b"Must not be omitted").unwrap();
    assert!(f.run(false).is_err());
    assert_eq!(snapshot(&f.db()), before);
    fs::remove_file(guide.join("index.md")).unwrap();
    assert!(f.run(false).unwrap().created);
}

/// Retained preparation and overlapping integration leases refuse at their
/// exact guard, without consuming an attempt. Actual foreign keys stay enabled;
/// releasing only that fixture lease permits the original request to proceed.
#[test]
fn manual_launch_preparation_and_integration_leases_refuse_then_release() {
    for integration in [false, true] {
        let f = Fixture::new();
        assert!(f.run(true).is_ok());
        let db = f.db();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        if integration {
            let mut agent = swarm_engine::Agent::default();
            agent.id = "retained".into();
            agent.work_id = f.work.clone();
            agent.task_id = "historical".into();
            agent.workspace = f.root.to_str().unwrap().into();
            agent.status = "completed".into();
            db.execute("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES('retained',?,'historical',?,'completed',?,?)",rusqlite::params![f.work,agent.workspace,swarm_engine::encode_model(&agent).unwrap(),b"{}".as_slice()]).unwrap();
            db.execute("INSERT INTO agent_exchanges(id,request_hash,work_id,kind,source_agent_id,source_task_id,source_attempt_id,recipient_task_id,recipient_role,state,artifacts,created_at) VALUES('exchange','fixture',?,'handoff','retained','historical','old','task','worker','received',?,'2026-10-19T00:00:00Z')",rusqlite::params![f.work,b"[]".as_slice()]).unwrap();
            let child = f.root.join("integration");
            fs::create_dir(&child).unwrap();
            db.execute("INSERT INTO workspace_integrations(id,request_hash,work_id,exchange_id,canonical_workspace,state,requested_at,detail) VALUES('lease','fixture',?,'exchange',?,'running','2026-10-19T00:00:00Z',?)",rusqlite::params![f.work,child.to_str().unwrap(),b"{}".as_slice()]).unwrap();
        } else {
            db.execute(
                "INSERT INTO preparations(id,work_id,revision,body) VALUES('preparation',?,1,?)",
                rusqlite::params![f.work, b"{}".as_slice()],
            )
            .unwrap();
            db.execute("INSERT INTO preparation_launch_locks(work_id,task_id,preparation_id,released) VALUES(?,'task','preparation',0)",[&f.work]).unwrap();
        }
        assert_eq!(
            db.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
        let before = snapshot(&db);
        let result = f.run(false);
        assert_eq!(
            result.err().unwrap().code(),
            if integration {
                "agent-launch-integration-busy"
            } else {
                "agent-launch-preparation-not-authorized"
            }
        );
        assert_eq!(snapshot(&db), before);
        if integration {
            db.execute(
                "UPDATE workspace_integrations SET state='integrated' WHERE id='lease'",
                [],
            )
            .unwrap();
        } else {
            db.execute(
                "UPDATE preparation_launch_locks SET released=1 WHERE work_id=?",
                [&f.work],
            )
            .unwrap();
        }
        db.close().unwrap();
        assert!(f.run(false).unwrap().created);
    }
}

/// Public preview must identify the newly constructed attempt and candidate
/// revision, rather than letting a historical profile stand for current identity.
#[test]
fn worker_context_current_identity_candidate_revision_and_replay_hash() {
    let f = Fixture::new();
    let before = snapshot(&f.db());
    let preview = f.run(true).unwrap();
    for text in [
        format!(
            "mission {} ; tâche task ; agent session ; tentative {}",
            f.work, preview.agent.attempt_id
        ),
        "révision du travail 3".into(),
        "Les identités, limites et conclusions des rapports antérieurs sont historiques".into(),
    ] {
        assert!(
            preview.agent.prompt.contains(&text),
            "missing current identity clause: {text}"
        );
    }
    assert_eq!(snapshot(&f.db()), before);
    let mut r = f.request();
    r.context_hash = preview.agent.context.as_ref().unwrap().sha256.clone();
    let mut store = RootStore::open(&f.root, true).unwrap();
    let actual = store
        .prepare_agent_launch(&f.work, &r, false, &mut SystemEnvironment)
        .unwrap();
    assert_eq!(actual.agent.prompt, preview.agent.prompt);
    assert_eq!(actual.agent.context, preview.agent.context);
    assert_eq!(store.read_work(&f.work).unwrap().revision, 3);
    store.close().unwrap();
}
/// The latest own plan contract is required, even when Next is a generic UI
/// status. A sibling contract/obsolete plan must never enter this worker context.
#[test]
fn worker_context_uses_latest_own_contract_without_sibling_or_obsolete_scope() {
    use sha2::{Digest, Sha256};
    let f = Fixture::new();
    assert!(f.run(true).is_ok());
    let db = f.db();
    let raw: Vec<u8> = db
        .query_row("SELECT body FROM works WHERE id=?", [&f.work], |r| r.get(0))
        .unwrap();
    let mut w = swarm_engine::decode_work(&raw).unwrap();
    let source = "fixture-plan";
    let id = format!(
        "plan-{}-T1",
        &format!("{:x}", Sha256::digest(source.as_bytes()))[..10]
    );
    w.tasks.as_mut().unwrap()[0].id = id.clone();
    w.tasks.as_mut().unwrap()[0].next = "Generic UI status".into();
    let plan = |scope: &str| swarm_engine::ApprovedPlan {
        source: source.into(),
        brief_hash: scope.into(),
        response_hash: "response".into(),
        spec: swarm_engine::ActionPlan {
            version: 1,
            tasks: Some(vec![
                swarm_engine::PlanMission {
                    id: "T1".into(),
                    role: "worker".into(),
                    scope: scope.into(),
                    proof: "CURRENT-OWN-PROOF".into(),
                    ..Default::default()
                },
                swarm_engine::PlanMission {
                    id: "T2".into(),
                    role: "worker".into(),
                    scope: "SIBLING-PRIVATE-SCOPE".into(),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        },
        ..Default::default()
    };
    w.plans = Some(vec![plan("OBSOLETE-SCOPE"), plan("CURRENT-OWN-SCOPE")]);
    db.execute(
        "UPDATE works SET body=? WHERE id=?",
        rusqlite::params![swarm_engine::encode_model(&w).unwrap(), f.work],
    )
    .unwrap();
    let before = snapshot(&db);
    db.close().unwrap();
    let mut request = f.request();
    request.task_id = id;
    let mut store = RootStore::open(&f.root, true).unwrap();
    let preview = store
        .prepare_agent_launch(&f.work, &request, true, &mut SystemEnvironment)
        .unwrap();
    store.close().unwrap();
    assert!(preview.agent.prompt.contains("CURRENT-OWN-SCOPE"));
    assert!(preview.agent.prompt.contains("CURRENT-OWN-PROOF"));
    assert!(!preview.agent.prompt.contains("OBSOLETE-SCOPE"));
    assert!(!preview.agent.prompt.contains("SIBLING-PRIVATE-SCOPE"));
    assert_eq!(snapshot(&f.db()), before);
}
/// A fresh second attempt gets its own identity and current budget count; an
/// interrupted first attempt cannot become an implicit successful handoff.
#[test]
fn worker_context_retry_has_new_attempt_identity_and_explicit_recovery_guidance() {
    let f = Fixture::new();
    let first = f.run(false).unwrap().agent;
    let mut store = RootStore::open(&f.root, true).unwrap();
    store
        .cancel_queued_agent("session", &mut SystemEnvironment)
        .unwrap();
    let mut r = f.request();
    r.event_id = "retry".into();
    r.expected_revision = store.read_work(&f.work).unwrap().revision;
    let retry = store
        .prepare_agent_launch(&f.work, &r, true, &mut SystemEnvironment)
        .unwrap();
    assert_ne!(retry.agent.attempt_id, first.attempt_id);
    assert!(retry.agent.prompt.contains(&format!(
        "agent retry ; tentative {}",
        retry.agent.attempt_id
    )));
    assert!(retry
        .agent
        .prompt
        .contains("REPRISE : comparer le contrat courant"));
    assert!(!retry
        .agent
        .prompt
        .contains(&format!("tentative {} ;", first.attempt_id)));
    store.close().unwrap();
}

/// A retained latest plan that explicitly claims this task cannot silently fall
/// back to an obsolete contract. Duplicate mapping claims are equally ambiguous;
/// each injected fault follows a valid preview and preserves every SQL table.
#[test]
fn worker_context_incoherent_current_plan_mapping_refuses_without_fallback() {
    use sha2::{Digest, Sha256};
    for case in 0..6 {
        let f = Fixture::new();
        let db = f.db();
        let raw: Vec<u8> = db
            .query_row("SELECT body FROM works WHERE id=?", [&f.work], |r| r.get(0))
            .unwrap();
        let mut w = swarm_engine::decode_work(&raw).unwrap();
        let source = "fixture-mapping";
        let id = format!(
            "plan-{}-T1",
            &format!("{:x}", Sha256::digest(source.as_bytes()))[..10]
        );
        w.tasks.as_mut().unwrap()[0].id = id.clone();
        let current = swarm_engine::ApprovedPlan {
            source: source.into(),
            brief_hash: "current".into(),
            spec: swarm_engine::ActionPlan {
                version: 1,
                tasks: Some(vec![swarm_engine::PlanMission {
                    id: "T1".into(),
                    role: "worker".into(),
                    scope: "CURRENT".into(),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut old = current.clone();
        old.brief_hash = "obsolete".into();
        w.plans = Some(vec![old, current]);
        db.execute(
            "UPDATE works SET body=? WHERE id=?",
            rusqlite::params![swarm_engine::encode_model(&w).unwrap(), f.work],
        )
        .unwrap();
        let mut r = f.request();
        r.task_id = id.clone();
        let mut store = RootStore::open(&f.root, true).unwrap();
        assert!(store
            .prepare_agent_launch(&f.work, &r, true, &mut SystemEnvironment)
            .is_ok());
        store.close().unwrap();
        let correct = w.clone();
        let plan = w.plans.as_mut().unwrap().last_mut().unwrap();
        match case {
            0 => {
                plan.task_map = Some(BTreeMap::from([("T1".into(), "foreign".into())]));
            }
            1 => {
                plan.task_map = Some(BTreeMap::from([
                    ("T1".into(), id.clone()),
                    ("T2".into(), id.clone()),
                ]));
            }
            2 => {
                plan.spec.tasks = Some(vec![]);
                plan.task_map = Some(BTreeMap::from([("T1".into(), id.clone())]));
            }
            3 => {
                plan.spec.tasks = Some(vec![]);
                plan.task_ids = Some(vec![id.clone()]);
            }
            _ => {}
        }
        if case == 4 {
            w.tasks.as_mut().unwrap()[0].plan_role = "worker".into();
            w.plans = None;
        }
        if case == 5 {
            w.tasks.as_mut().unwrap()[0].scope_id = "child".into();
        }
        db.execute(
            "UPDATE works SET body=? WHERE id=?",
            rusqlite::params![swarm_engine::encode_model(&w).unwrap(), f.work],
        )
        .unwrap();
        let before = snapshot(&db);
        let mut store = RootStore::open(&f.root, true).unwrap();
        let result = store.prepare_agent_launch(&f.work, &r, false, &mut SystemEnvironment);
        store.close().unwrap();
        assert_eq!(
            result.err().expect("incoherent mapping accepted").code(),
            if case == 5 {
                "agent-current-context-branch-unported"
            } else {
                "agent-current-plan-mapping-invalid"
            }
        );
        assert_eq!(snapshot(&db), before);
        w = correct;
        db.execute(
            "UPDATE works SET body=? WHERE id=?",
            rusqlite::params![swarm_engine::encode_model(&w).unwrap(), f.work],
        )
        .unwrap();
        let mut store = RootStore::open(&f.root, true).unwrap();
        assert!(store
            .prepare_agent_launch(&f.work, &r, true, &mut SystemEnvironment)
            .is_ok());
        store.close().unwrap();
        db.close().unwrap();
    }
}
/// A context reviewed before an authorized Work update is stale even if the
/// launch revision is refreshed. Rejection preserves that legitimate update;
/// reviewing a fresh preview then committing its exact digest recovers.
#[test]
fn worker_context_stale_preview_refuses_then_fresh_digest_commits() {
    let f = Fixture::new();
    let preview = f.run(true).unwrap();
    let mut r = f.request();
    r.context_hash = preview.agent.context.unwrap().sha256;
    let mut store = RootStore::open(&f.root, true).unwrap();
    store.execute_external(&f.work,"work.update",br#"{"schema_version":1,"event_id":"change","expected_revision":2,"title":"Current changed title"}"#,&mut SystemEnvironment).unwrap();
    r.expected_revision = store.read_work(&f.work).unwrap().revision;
    let before = snapshot(&f.db());
    let result = store.prepare_agent_launch(&f.work, &r, false, &mut SystemEnvironment);
    assert_eq!(
        result.err().unwrap().code(),
        "agent-launch-context-hash-changed"
    );
    assert_eq!(snapshot(&f.db()), before);
    r.context_hash.clear();
    let fresh = store
        .prepare_agent_launch(&f.work, &r, true, &mut SystemEnvironment)
        .unwrap();
    r.context_hash = fresh.agent.context.unwrap().sha256;
    let committed = store
        .prepare_agent_launch(&f.work, &r, false, &mut SystemEnvironment)
        .unwrap();
    assert!(committed.created);
    assert!(committed.agent.prompt.contains("Current changed title"));
    assert!(committed.agent.prompt.contains("révision du travail 4"));
    store.close().unwrap();
}

/// Public saved-policy changes affect new admissions, never an already frozen
/// intention. Diagnostic replay checks a consistent row and preserves every SQL
/// table on success/refusal; consistency is not authentication or execution.
#[test]
fn manual_launch_guard_policy_is_frozen_and_replay_is_read_only() {
    use swarm_engine::{decode_agent_guard_trace, EngineSettingsChange, SettingCap, SettingCount};
    let f = Fixture::new();
    let prepared = f.run(false).unwrap();
    let frozen = prepared.agent.native_tool_guard_policy.as_ref().unwrap();
    assert_eq!(
        frozen.lookback_multiplier,
        SettingCap(Some(SettingCount(4)))
    );
    let frozen_revision = frozen.settings_revision.clone();
    let trace=decode_agent_guard_trace(br#"{"schema_version":1,"events":[{"call":{"at_ms":0,"id":"A","name":"Read","input":{}}},{"result":{"at_ms":1,"id":"A","failed":true,"message":"permission denied"}}],"final_at_ms":1}"#).unwrap();
    for factor in [Some(1), None] {
        let store = RootStore::open(&f.root, true).unwrap();
        let doc = store.settings().unwrap();
        let mut values = doc.values;
        values.agent_interleaved_failure_lookback_multiplier =
            Some(SettingCap(factor.map(SettingCount)));
        let mut change = EngineSettingsChange {
            schema_version: doc.schema_version,
            event_id: format!("window-{}", doc.revision.0),
            expected_revision: doc.revision,
            values,
            reason: "fixture next-attempt policy".into(),
            preview_token: String::new(),
        };
        change.preview_token = store.preview_settings(&change).unwrap().preview_token;
        store
            .apply_settings(&change, &mut SystemEnvironment)
            .unwrap();
        store.close().unwrap();
        let before = snapshot(&f.db());
        let mut store = RootStore::open(&f.root, false).unwrap();
        let report = store.replay_agent_guard("session", &trace).unwrap();
        assert_eq!(report.settings_revision, frozen_revision);
        assert_eq!(
            report.lookback_multiplier,
            SettingCap(Some(SettingCount(4)))
        );
        assert_eq!(report.observation.total_errors, 1);
        store.close().unwrap();
        assert_eq!(snapshot(&f.db()), before);
        let replay = f.run(false).unwrap();
        assert_eq!(replay.agent.native_tool_guard_policy.as_ref(), Some(frozen));
    }
    let original: Vec<u8> = f
        .db()
        .query_row("SELECT body FROM agents WHERE id='session'", [], |r| {
            r.get(0)
        })
        .unwrap();
    for fault in ["id", "work_id", "missing_snapshot", "unknown_constraint"] {
        // Valid control on the same fixture before each independent injection.
        let mut store = RootStore::open(&f.root, false).unwrap();
        assert!(store.replay_agent_guard("session", &trace).is_ok());
        store.close().unwrap();
        let mut body: serde_json::Value = serde_json::from_slice(&original).unwrap();
        match fault {
            "id" => body["id"] = "other".into(),
            "work_id" => body["work_id"] = "other".into(),
            "missing_snapshot" => {
                body.as_object_mut()
                    .unwrap()
                    .remove("native_tool_guard_policy");
            }
            _ => body["native_tool_guard_policy"]["future_constraint"] = true.into(),
        }
        let db = f.db();
        db.execute(
            "UPDATE agents SET body=? WHERE id='session'",
            [serde_json::to_vec(&body).unwrap()],
        )
        .unwrap();
        db.close().unwrap();
        let before = snapshot(&f.db());
        let mut store = RootStore::open(&f.root, false).unwrap();
        assert!(
            store.replay_agent_guard("session", &trace).is_err(),
            "{fault}"
        );
        store.close().unwrap();
        assert_eq!(snapshot(&f.db()), before);
        let db = f.db();
        db.execute("UPDATE agents SET body=? WHERE id='session'", [&original])
            .unwrap();
        db.close().unwrap();
        let mut store = RootStore::open(&f.root, false).unwrap();
        assert!(store.replay_agent_guard("session", &trace).is_ok());
        store.close().unwrap();
    }
}

/// A complete historical v16 document cannot silently acquire the new guard
/// default. Refused admission preserves all tables; explicit preview/apply v17
/// enables the same request with the selected factor and retained old history.
#[test]
fn manual_launch_guard_legacy_settings_require_explicit_adoption() {
    use swarm_engine::{
        EngineSettingsChange, EngineSettingsDocument, EngineSettingsValues, SettingCap,
        SettingCount,
    };
    let f = Fixture::new();
    assert!(f.run(true).is_ok());
    let mut raw =
        serde_json::to_value(EngineSettingsValues::installation_defaults().unwrap()).unwrap();
    for field in swarm_engine::EngineSettingsCatalog::installation()
        .unwrap()
        .fields
    {
        if field.introduced_schema_version > 16 {
            raw.as_object_mut().unwrap().remove(&field.name);
        }
    }
    let values = EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
    let old =
        EngineSettingsDocument::initial(values, "fixture".into(), "2026-10-19T00:00:00Z".into())
            .unwrap();
    assert_eq!(old.schema_version, 16);
    // Isolated historical-fixture construction only; no live root is patched.
    fs::write(
        f.root.join(".swarm/engine-settings.json"),
        serde_json::to_vec(&old).unwrap(),
    )
    .unwrap();
    let before = snapshot(&f.db());
    assert_eq!(
        f.run(false).err().unwrap().code(),
        "settings-tool-guard-policy-not-configured"
    );
    assert_eq!(snapshot(&f.db()), before);
    let store = RootStore::open(&f.root, true).unwrap();
    let doc = store.settings().unwrap();
    let mut values = doc.values.clone();
    values.agent_interleaved_failure_lookback_multiplier = Some(SettingCap(Some(SettingCount(1))));
    let mut change = EngineSettingsChange {
        schema_version: 17,
        event_id: "adopt-guard".into(),
        expected_revision: doc.revision,
        values,
        reason: "Explicit fixture guard adoption".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    assert_eq!(store.settings().unwrap(), doc);
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    store.close().unwrap();
    let store = RootStore::open(&f.root, false).unwrap();
    let adopted = store.settings().unwrap();
    store.close().unwrap();
    assert_eq!(adopted.schema_version, 17);
    assert_eq!(adopted.history[0], old.history[0]);
    let prepared = f.run(false).unwrap();
    let snapshot = prepared.agent.native_tool_guard_policy.unwrap();
    assert_eq!(snapshot.settings_revision, adopted.revision.0.to_string());
    assert_eq!(
        snapshot.lookback_multiplier,
        SettingCap(Some(SettingCount(1)))
    );
}

/// Extract only the underlying SQLite numeric family, never private SQL/path.
/// A typed cause must survive public Root wrapping; absence remains explicit.
fn sqlite_failure(error: &(dyn std::error::Error + 'static)) -> Option<i32> {
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(error) = error.downcast_ref::<rusqlite::Error>() {
            if let Some(sql) = error.sqlite_error() {
                return Some(sql.extended_code);
            }
        }
        current = error.source();
    }
    None
}
/// Held real writer plus saved zero SQLite wait/2 retries is deterministic,
/// without a temporal test deadline. Exhaustion must preserve every table and
/// original SQL cause; explicit release is the new precondition for recovery.
#[test]
fn manual_launch_saved_storage_retry_policy_and_explicit_release() {
    use swarm_engine::{EngineSettingsChange, SettingCount};
    let f = Fixture::new();
    assert!(f.run(true).is_ok());
    let store = RootStore::open(&f.root, true).unwrap();
    let doc = store.settings().unwrap();
    let mut values = doc.values;
    values.storage_busy_timeout_ms = SettingCount(0);
    values.storage_busy_retries = Some(SettingCount(2));
    values.storage_busy_retry_delay_ms = SettingCount(0);
    let mut change = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "busy-policy".into(),
        expected_revision: doc.revision,
        values,
        reason: "Deterministic held-writer fixture, no deadline".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    store.close().unwrap();
    assert!(f.run(true).is_ok());
    let before = snapshot(&f.db());
    let mut writer = f.db();
    let held = writer
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    let outcome = f.run(false);
    assert_eq!(snapshot(&f.db()), before);
    held.rollback().unwrap();
    writer.close().unwrap();
    let error = outcome.err().expect("held writer must refuse");
    assert_eq!(sqlite_failure(&error).map(|code| code & 255), Some(5));
    println!("NATIVE_HELD_WRITER_SQLITE_FAMILY 5");
    assert_eq!(error.code(), "agent-launch-storage-busy");
    match &error {
        swarm_engine::RootStoreError::Launch(error) => match error.as_ref() {
            swarm_engine::AgentLaunchError::Busy {
                attempts,
                settings_revision,
                ..
            } => {
                assert_eq!(*attempts, 3);
                assert_eq!(settings_revision.0, 2);
            }
            _ => panic!("missing structured busy metadata"),
        },
        _ => panic!("wrong public error family"),
    }
    assert!(f.run(false).unwrap().created);
    assert!(!f.run(false).unwrap().created);
}

/// Real held-writer fixture shared by stop/cancel/claim cases. Saved limits are
/// explicit, no elapsed-time deadline. Refusal preserves all tables; release is
/// the observable recovery precondition. No registered provider child executes.
fn lifecycle_storage_case(operation: &str) {
    use swarm_engine::{EngineSettingsChange, RootStoreError, SettingCount};
    let f = Fixture::new();
    assert!(f.run(false).unwrap().created);
    let store = RootStore::open(&f.root, true).unwrap();
    let doc = store.settings().unwrap();
    let mut values = doc.values;
    values.storage_busy_timeout_ms = SettingCount(0);
    values.storage_busy_retries = Some(SettingCount(1));
    values.storage_busy_retry_delay_ms = SettingCount(0);
    let mut change = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "lifecycle-storage-policy".into(),
        expected_revision: doc.revision,
        values,
        reason: "Owned held-writer lifecycle fixture".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    store.close().unwrap();
    let before = snapshot(&f.db());
    let mut writer = f.db();
    let held = writer
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    let invoke = |store: &mut RootStore| -> Result<(), RootStoreError> {
        match operation {
            "stop" => store
                .stop_agent("session", &mut SystemEnvironment)
                .map(|_| ()),
            "cancel" => store
                .cancel_queued_agent("session", &mut SystemEnvironment)
                .map(|_| ()),
            "claim" => store.claim_agent("session").map(|_| ()),
            _ => panic!("invalid fixture operation"),
        }
    };
    let mut store = RootStore::open(&f.root, true).unwrap();
    let outcome = invoke(&mut store);
    store.close().unwrap();
    assert_eq!(snapshot(&f.db()), before);
    held.rollback().unwrap();
    writer.close().unwrap();
    let error = outcome.expect_err("held lifecycle writer must refuse");
    assert_eq!(sqlite_failure(&error).map(|v| v & 255), Some(5));
    assert_eq!(error.code(), "agent-storage-busy", "operation {operation}");
    match error {
        RootStoreError::AgentStorageBusy {
            attempts,
            settings_revision,
            ..
        } => {
            assert_eq!(attempts, 2);
            assert_eq!(settings_revision.0, 2);
        }
        _ => panic!("missing typed lifecycle exhaustion"),
    }
    let mut store = RootStore::open(&f.root, true).unwrap();
    invoke(&mut store).unwrap();
    let agent = store.read_agent("session").unwrap();
    store.close().unwrap();
    assert_eq!(agent.child_pid, 0);
    match operation {
        "stop" => assert_eq!(agent.desired, "stop"),
        "cancel" => assert_eq!(agent.status, "interrupted"),
        "claim" => {
            assert_eq!(agent.status, "starting");
            assert_eq!(agent.supervisor_pid, i64::from(std::process::id()));
        }
        _ => unreachable!(),
    }
    let settled = snapshot(&f.db());
    let mut store = RootStore::open(&f.root, true).unwrap();
    let replay = invoke(&mut store);
    store.close().unwrap();
    if operation == "claim" {
        assert_eq!(replay.err().unwrap().code(), "agent-already-claimed");
    } else {
        replay.unwrap();
    }
    assert_eq!(snapshot(&f.db()), settled);
}
/// Stop preserves its typed BUSY cause and applies the saved retry policy.
#[test]
fn lifecycle_stop_saved_storage_policy_and_recovery() {
    lifecycle_storage_case("stop")
}
/// Cancellation keeps its atomic settlement and frozen metadata across retries.
#[test]
fn lifecycle_cancel_saved_storage_policy_and_recovery() {
    lifecycle_storage_case("cancel")
}
/// Registration uses actual kernel identity only after the writer is available.
#[test]
fn lifecycle_claim_saved_storage_policy_and_recovery() {
    lifecycle_storage_case("claim")
}

/// A previously opened Root is not permission to mutate a later unknown DB
/// epoch. Valid stop witness precedes private epoch fault; restore37 explicitly
/// before recovery. No real mission database or process is modified.
#[test]
fn lifecycle_stop_checks_epoch_inside_its_transaction() {
    let control = Fixture::new();
    assert!(control.run(false).unwrap().created);
    let mut healthy = RootStore::open(&control.root, true).unwrap();
    assert_eq!(
        healthy
            .stop_agent("session", &mut SystemEnvironment)
            .unwrap()
            .desired,
        "stop"
    );
    healthy.close().unwrap();
    let f = Fixture::new();
    assert!(f.run(false).unwrap().created);
    let mut store = RootStore::open(&f.root, true).unwrap();
    let writer = f.db();
    writer.pragma_update(None, "user_version", 38).unwrap();
    let before = snapshot(&writer);
    let outcome = store.stop_agent("session", &mut SystemEnvironment);
    store.close().unwrap();
    let after = snapshot(&writer);
    writer.pragma_update(None, "user_version", 37).unwrap();
    writer.close().unwrap();
    assert!(outcome.is_err(), "unknown epoch accepted a durable stop");
    assert_eq!(outcome.err().unwrap().code(), "storage_version_conflict");
    assert_eq!(before, after);
    let mut store = RootStore::open(&f.root, true).unwrap();
    assert_eq!(
        store
            .stop_agent("session", &mut SystemEnvironment)
            .unwrap()
            .desired,
        "stop"
    );
    store.close().unwrap();
}

/// A virtual observed clock, with optional expiry on the actual staged Work
/// event identity callback. No sleep, elapsed-time limit or timestamp-call count.
struct LeaseLaunchClock {
    now: String,
    expire_on_event: Option<String>,
    advanced: bool,
}
impl swarm_engine::WorkEnvironment for LeaseLaunchClock {
    fn identifier(&mut self, prefix: &str) -> Result<String, swarm_engine::RuntimeError> {
        if prefix == "launch-" {
            if let Some(next) = self.expire_on_event.take() {
                self.now = next;
                self.advanced = true;
            }
        }
        swarm_engine::WorkEnvironment::identifier(&mut SystemEnvironment, prefix)
    }
    fn timestamp(&mut self) -> Result<String, swarm_engine::RuntimeError> {
        Ok(self.now.clone())
    }
    fn operator(&mut self) -> Result<String, swarm_engine::RuntimeError> {
        swarm_engine::WorkEnvironment::operator(&mut SystemEnvironment)
    }
    fn git(
        &mut self,
        root: &std::path::Path,
    ) -> Result<swarm_engine::GitObservation, swarm_engine::RuntimeError> {
        swarm_engine::WorkEnvironment::git(&mut SystemEnvironment, root)
    }
}
/// Find the original typed lease refusal through the public Root/launch wrappers;
/// no message-string matching or successful empty result can satisfy a guard.
fn lease_launch_cause(error: &(dyn std::error::Error + 'static)) -> Option<&'static str> {
    let mut current = Some(error);
    while let Some(e) = current {
        if let Some(m) = e.downcast_ref::<swarm_engine::MissionSupervisionError>() {
            return Some(m.code());
        }
        current = e.source();
    }
    None
}
/// Preview rolls back every SQL effect; actual admission and replay retain one
/// original generation receipt. Another Root/stopped handle refuses, while a new
/// owner can read the existing replay without taking its historical attribution.
#[test]
fn leased_manual_launch_preview_receipt_replay_and_binding() {
    let f = Fixture::new();
    let mut root = RootStore::open(&f.root, true).unwrap();
    let start = root.read_work(&f.work).unwrap().created;
    let lease = root
        .claim_mission_supervision(&f.work, "declared conductor", &start)
        .unwrap();
    let mut clock = LeaseLaunchClock {
        now: start.clone(),
        expire_on_event: None,
        advanced: false,
    };
    let request = f.request();
    let before = snapshot(&f.db());
    let preview = root
        .prepare_agent_launch_under_lease(&lease, &request, true, &mut clock)
        .unwrap();
    assert!(preview.preview_only);
    assert!(!preview.created);
    assert_eq!(snapshot(&f.db()), before);
    let other = Fixture::new();
    let mut other_root = RootStore::open(&other.root, true).unwrap();
    let other_before = snapshot(&other.db());
    let error =
        match other_root.prepare_agent_launch_under_lease(&lease, &request, true, &mut clock) {
            Err(e) => e,
            Ok(_) => panic!("foreign Root accepted"),
        };
    assert_eq!(
        lease_launch_cause(&error),
        Some("mission-lease-root-mismatch")
    );
    assert_eq!(snapshot(&other.db()), other_before);
    other_root.close().unwrap();
    let outcome = root
        .prepare_agent_launch_under_lease(&lease, &request, false, &mut clock)
        .unwrap();
    assert!(outcome.created);
    assert_eq!(outcome.agent.child_pid, 0);
    assert_eq!(outcome.agent.origin, "operateur");
    let receipts = root.read_mission_work_receipts(&f.work).unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].kind, "agent.start");
    assert_eq!(receipts[0].generation, lease.generation());
    assert_eq!(receipts[0].work_revision, 3);
    assert_eq!(
        receipts[0].claim_settings_revision,
        lease.settings_revision()
    );
    fs::remove_file(f.root.join(".swarm/providers.json")).unwrap();
    let recorded = snapshot(&f.db());
    let replay = root
        .prepare_agent_launch_under_lease(&lease, &request, false, &mut clock)
        .unwrap();
    assert!(!replay.created);
    assert_eq!(snapshot(&f.db()), recorded);
    assert_eq!(root.read_mission_work_receipts(&f.work).unwrap(), receipts);
    root.update_mission_supervision(&lease, &start, true)
        .unwrap();
    let stopped = snapshot(&f.db());
    let error = match root.prepare_agent_launch_under_lease(&lease, &request, false, &mut clock) {
        Err(e) => e,
        Ok(_) => panic!("stopped lease replay accepted"),
    };
    assert_eq!(lease_launch_cause(&error), Some("mission-lease-lost"));
    assert_eq!(snapshot(&f.db()), stopped);
    let replacement = root
        .claim_mission_supervision(&f.work, "declared conductor", &start)
        .unwrap();
    assert_ne!(replacement.generation(), lease.generation());
    let prior = snapshot(&f.db());
    assert!(
        !root
            .prepare_agent_launch_under_lease(&replacement, &request, false, &mut clock)
            .unwrap()
            .created
    );
    assert_eq!(snapshot(&f.db()), prior);
    assert_eq!(root.read_mission_work_receipts(&f.work).unwrap(), receipts);
    replacement.close().unwrap();
    lease.close().unwrap();
    root.close().unwrap();
}
/// The valid preview controls a virtual expiry after actual event staging;
/// rollback must restore all SQL tables, including agent/budget/Work and receipts,
/// then the same request succeeds under an unchanged still-current stored lease.
#[test]
fn leased_manual_launch_expiry_after_staging_rolls_back_and_recovers() {
    let f = Fixture::new();
    let mut root = RootStore::open(&f.root, true).unwrap();
    let start = root.read_work(&f.work).unwrap().created;
    let lease = root
        .claim_mission_supervision(&f.work, "conductor", &start)
        .unwrap();
    let mut clock = LeaseLaunchClock {
        now: start.clone(),
        expire_on_event: None,
        advanced: false,
    };
    let request = f.request();
    assert!(
        root.prepare_agent_launch_under_lease(&lease, &request, true, &mut clock)
            .unwrap()
            .preview_only
    );
    let before = snapshot(&f.db());
    let t = swarm_wire_json::go_rfc3339::parse(&start)
        .unwrap()
        .checked_add(time::Duration::milliseconds(
            lease.policy().stale_after_ms.0,
        ))
        .unwrap()
        .checked_add(time::Duration::nanoseconds(1))
        .unwrap();
    clock.expire_on_event = Some(
        swarm_engine::format_utc_timestamp(t.unix_timestamp(), i64::from(t.nanosecond())).unwrap(),
    );
    let error = match root.prepare_agent_launch_under_lease(&lease, &request, false, &mut clock) {
        Err(e) => e,
        Ok(_) => panic!("expired staged admission committed"),
    };
    assert!(clock.advanced);
    assert_eq!(
        lease_launch_cause(&error),
        Some("mission-lease-not-present")
    );
    assert_eq!(snapshot(&f.db()), before);
    assert!(root.read_mission_work_receipts(&f.work).unwrap().is_empty());
    clock.now = start;
    assert!(
        root.prepare_agent_launch_under_lease(&lease, &request, false, &mut clock)
            .unwrap()
            .created
    );
    assert_eq!(root.read_mission_work_receipts(&f.work).unwrap().len(), 1);
    lease.close().unwrap();
    root.close().unwrap();
}
/// A valid preview precedes a reserved-name ledger collision. Receipt refusal
/// after agent staging must roll back all tables; removing only the fixture view
/// permits the same admission, with no provider child or duplicate reservation.
#[test]
fn leased_manual_launch_receipt_failure_rolls_back_and_recovers() {
    let f = Fixture::new();
    let mut root = RootStore::open(&f.root, true).unwrap();
    let start = root.read_work(&f.work).unwrap().created;
    let lease = root
        .claim_mission_supervision(&f.work, "conductor", &start)
        .unwrap();
    let mut clock = LeaseLaunchClock {
        now: start,
        expire_on_event: None,
        advanced: false,
    };
    let request = f.request();
    assert!(
        root.prepare_agent_launch_under_lease(&lease, &request, true, &mut clock)
            .unwrap()
            .preview_only
    );
    let db = f.db();
    db.execute_batch("CREATE VIEW native_mission_work_receipts AS SELECT 1 AS fixture_marker")
        .unwrap();
    let before = snapshot(&db);
    let error = match root.prepare_agent_launch_under_lease(&lease, &request, false, &mut clock) {
        Err(e) => e,
        Ok(_) => panic!("broken receipt ledger accepted"),
    };
    assert_eq!(
        lease_launch_cause(&error),
        Some("mission-command-schema-invalid")
    );
    assert_eq!(snapshot(&db), before);
    db.execute_batch("DROP VIEW native_mission_work_receipts")
        .unwrap();
    assert!(
        root.prepare_agent_launch_under_lease(&lease, &request, false, &mut clock)
            .unwrap()
            .created
    );
    assert_eq!(root.read_mission_work_receipts(&f.work).unwrap().len(), 1);
    db.close().unwrap();
    lease.close().unwrap();
    root.close().unwrap();
}

/// Locate the original typed organization cause through the public Root wrappers.
fn organization_launch_reason(error: &(dyn std::error::Error + 'static)) -> Option<&'static str> {
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(error) = error.downcast_ref::<swarm_engine::MissionOrganizationError>() {
            return Some(error.reason());
        }
        current = error.source();
    }
    None
}
/// A configured hierarchy still refuses the unported managed branch. Omitting
/// reviewer/capacity yields the precise prerequisite without SQL effects; restoring
/// the original manual fixture permits admission, never a provider process.
#[test]
fn planned_launch_preserves_organization_and_reviewer_refusals() {
    let f = Fixture::new();
    assert!(f.run(true).unwrap().preview_only);
    let root = RootStore::open(&f.root, true).unwrap();
    let original = root.read_work(&f.work).unwrap();
    root.close().unwrap();
    let template=swarm_engine::decode_work(br#"{"schema_version":1,"id":"work","revision":1,"criteria":["test"],"planning":{"version":1,"max_tasks":10,"max_decisions":10,"max_activations":10,"provider":"fixture","human_review_authorized":"declared","scopes":[{"id":"root","revision":1,"requirements":["req-1"]}],"reviewer":{"provider":"fixture","authorized":"declared","max_calls":10}}}"#).unwrap();
    let mut managed = original.clone();
    managed.planning = template.planning;
    for task in managed.tasks.iter_mut().flatten() {
        task.scope_id = "root".into();
        task.requirements = Some(vec!["req-1".into()]);
        task.validation_policy = swarm_engine::decode_task(
            br#"{"id":"task","validation_policy":{"mode":"human","authorized_at":"declared"}}"#,
        )
        .unwrap()
        .validation_policy;
    }
    swarm_engine::validate_mission_organization(&managed).unwrap();
    let db = f.db();
    let body = swarm_engine::encode_model(&managed).unwrap();
    db.execute(
        "UPDATE works SET body=? WHERE id=?",
        rusqlite::params![body, f.work],
    )
    .unwrap();
    let before = snapshot(&db);
    let error = match f.run(true) {
        Err(error) => error,
        Ok(_) => panic!("reviewer without provider digest accepted"),
    };
    assert!(
        matches!(&error,swarm_engine::RootStoreError::Launch(error) if error.code()=="reviewer_unavailable")
    );
    assert_eq!(snapshot(&db), before);
    let mut omitted = managed.clone();
    omitted.planning.as_mut().unwrap().reviewer = None;
    db.execute(
        "UPDATE works SET body=? WHERE id=?",
        rusqlite::params![swarm_engine::encode_model(&omitted).unwrap(), f.work],
    )
    .unwrap();
    let before = snapshot(&db);
    let error = match f.run(true) {
        Err(error) => error,
        Ok(_) => panic!("omitted reviewer accepted"),
    };
    assert_eq!(
        organization_launch_reason(&error),
        Some("organization-reviewer-missing")
    );
    assert_eq!(snapshot(&db), before);
    let mut exhausted = managed;
    exhausted
        .planning
        .as_mut()
        .unwrap()
        .reviewer
        .as_mut()
        .unwrap()
        .calls = 10;
    db.execute(
        "UPDATE works SET body=? WHERE id=?",
        rusqlite::params![swarm_engine::encode_model(&exhausted).unwrap(), f.work],
    )
    .unwrap();
    let before = snapshot(&db);
    let error = match f.run(true) {
        Err(error) => error,
        Ok(_) => panic!("exhausted reviewer accepted"),
    };
    assert_eq!(
        organization_launch_reason(&error),
        Some("reviewer-budget-exhausted")
    );
    assert_eq!(snapshot(&db), before);
    db.execute(
        "UPDATE works SET body=? WHERE id=?",
        rusqlite::params![swarm_engine::encode_model(&original).unwrap(), f.work],
    )
    .unwrap();
    db.close().unwrap();
    let recovered = f.run(false).unwrap();
    assert!(recovered.created);
    assert_eq!(recovered.agent.child_pid, 0);
}

/// Public admission observes a valid cooldown before malformed clear metadata.
/// No quota/account is cleared: this edits only the owned fixture, and every
/// refusal must preserve all SQL values and reservations. No child is started.
#[test]
fn malformed_provider_cooldown_clearance_refuses_public_admission_without_mutation() {
    let f = Fixture::new();
    assert!(f.run(true).unwrap().preview_only);
    println!("NATIVE_COOLDOWN_FIXTURE {}", f.root.display());
    let dir = f.root.join(".swarm/provider-cooldowns");
    fs::create_dir(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    let path = dir.join("fixture.json");
    let mut value = swarm_engine::ProviderCooldown {
        provider: "fixture".into(),
        observed_at: "2026-10-20T00:00:00Z".into(),
        source: "fixture".into(),
        signal: "backoff".into(),
        ..Default::default()
    };
    fs::write(&path, swarm_engine::encode_model(&value).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let db = f.db();
    let before = snapshot(&db);
    let error = match f.run(true) {
        Err(error) => error,
        Ok(_) => panic!("active cooldown accepted"),
    };
    assert_eq!(error.code(), "provider_cooldown");
    assert_eq!(snapshot(&db), before);
    value.cleared_at = "PRIVATE-MALFORMED-CLEARANCE".into();
    fs::write(&path, swarm_engine::encode_model(&value).unwrap()).unwrap();
    let error = match f.run(true) {
        Err(error) => error,
        Ok(_) => panic!("malformed clearance accepted by public preview"),
    };
    assert_eq!(error.code(), "agent-provider-cooldown-clear-time-invalid");
    assert!(!format!("{error:?} {error}").contains("PRIVATE-MALFORMED"));
    assert_eq!(snapshot(&db), before);
    let committed = match f.run(false) {
        Err(error) => error,
        Ok(_) => panic!("malformed clearance admitted"),
    };
    assert_eq!(
        committed.code(),
        "agent-provider-cooldown-clear-time-invalid"
    );
    assert_eq!(snapshot(&db), before);
    value.cleared_at = "2026-10-20T00:01:00Z".into();
    fs::write(&path, swarm_engine::encode_model(&value).unwrap()).unwrap();
    assert!(f.run(true).unwrap().preview_only);
    assert_eq!(snapshot(&db), before);
    value.observed_at = "PRIVATE-MALFORMED-OBSERVATION".into();
    fs::write(&path, swarm_engine::encode_model(&value).unwrap()).unwrap();
    let invalid = match f.run(true) {
        Err(error) => error,
        Ok(_) => panic!("malformed observation accepted"),
    };
    assert_eq!(
        invalid.code(),
        "agent-provider-cooldown-observed-time-invalid"
    );
    assert_eq!(snapshot(&db), before);
    value.observed_at = "2026-10-20T00:00:00Z".into();
    value.cleared_at.clear();
    fs::write(&path, swarm_engine::encode_model(&value).unwrap()).unwrap();
    assert_eq!(
        match f.run(true) {
            Err(error) => error,
            Ok(_) => panic!("uncleared cooldown accepted"),
        }
        .code(),
        "provider_cooldown"
    );
    fs::remove_file(&path).unwrap();
    assert!(f.run(true).unwrap().preview_only);
    assert_eq!(snapshot(&db), before);
}
