#![cfg(target_os = "linux")]
#[path = "support/task_report.rs"]
mod support;
use swarm_engine::{RootStore, SystemEnvironment, WorkEngine};
/// Valid witness precedes an event INSERT fault after staged Work mutation.
/// Complete rollback preserves Work/events/agent/receipt, then identical intent
/// recovers after only the owned TEMP trigger is removed; replay adds nothing.
#[test]
fn task_report_event_failure_rolls_back_and_identical_intent_recovers() {
    let witness = support::Fixture::new();
    let mut s = RootStore::open(&witness.root, true).unwrap();
    s.execute_external(
        &witness.work,
        "task.submit",
        &witness.request("witness"),
        &mut SystemEnvironment,
    )
    .unwrap();
    s.close().unwrap();
    let f = support::Fixture::new();
    let s = RootStore::open(&f.root, false).unwrap();
    let settings = s.settings().unwrap();
    s.close().unwrap();
    let mut db = rusqlite::Connection::open(f.root.join(".swarm/state.db")).unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    let snapshot = |db: &rusqlite::Connection| {
        let w = swarm_engine::read_recorded_work(db, &f.work).unwrap();
        let events = db
            .query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap();
        let agent = swarm_engine::read_agent(db, "session").unwrap();
        let receipt = db
            .query_row(
                "SELECT state,amount FROM reservations WHERE agent_id='session'",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)),
            )
            .unwrap();
        (w, events, agent, receipt)
    };
    let before = snapshot(&db);
    let limits = settings.values.mutation_limits().unwrap();
    let request = f.request("submit");
    db.execute_batch("CREATE TEMP TRIGGER report_fault BEFORE INSERT ON main.events WHEN NEW.kind='task.submit' BEGIN SELECT RAISE(ABORT,'PRIVATE event fault'); END;").unwrap();
    let error = WorkEngine::new(&mut db, &f.root, &mut SystemEnvironment, &limits)
        .with_planning_settings(&settings)
        .execute_external(&f.work, "task.submit", &request)
        .unwrap_err();
    assert!(!error.to_string().contains("PRIVATE"));
    assert!(db.is_autocommit());
    assert_eq!(snapshot(&db), before);
    db.execute_batch("DROP TRIGGER report_fault").unwrap();
    let result = WorkEngine::new(&mut db, &f.root, &mut SystemEnvironment, &limits)
        .with_planning_settings(&settings)
        .execute_external(&f.work, "task.submit", &request)
        .unwrap();
    assert!(!result.replayed);
    assert_eq!(result.work.tasks.as_ref().unwrap()[0].status, "submitted");
    assert_eq!(
        result.work.tasks.as_ref().unwrap()[0].attempts,
        before.0.tasks.as_ref().unwrap()[0].attempts
    );
    let after = snapshot(&db);
    assert_eq!(after.2, before.2);
    assert_eq!(after.3, before.3);
    assert!(
        WorkEngine::new(&mut db, &f.root, &mut SystemEnvironment, &limits)
            .with_planning_settings(&settings)
            .execute_external(&f.work, "task.submit", &request)
            .unwrap()
            .replayed
    );
    assert_eq!(snapshot(&db), after);
    db.close().unwrap();
}
