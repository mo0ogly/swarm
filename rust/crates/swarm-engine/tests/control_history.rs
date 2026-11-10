use rusqlite::Connection;
use std::error::Error;
use swarm_engine::{read_validation_control_history, Attempt, AutomaticValidation, Task};

/// Owned SQL fixture with the actual event column types; no live mission is used.
fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE events(work_id TEXT,kind TEXT,revision INTEGER,payload BLOB)")
        .unwrap();
    db
}
/// Keep actual receipt JSON bytes and event revision separate from its tested revision.
fn add(db: &Connection, revision: i64, raw: &[u8]) {
    db.execute(
        "INSERT INTO events VALUES('work','task.auto-validation',?1,?2)",
        rusqlite::params![revision, raw],
    )
    .unwrap();
}
/// Declare ownership through task attempts, not through the event's claimed task name.
fn task() -> Task {
    Task {
        attempts: Some(vec![Attempt {
            id: "attempt".into(),
            ..Default::default()
        }]),
        ..Default::default()
    }
}
/// Exact receipt data used as a valid control before a SQL or malformed-history fault.
fn receipt() -> &'static [u8] {
    br#"{"attempt_id":"attempt","revision":7,"receipt":"old","controls":[{"id":"control","executed":true,"passed":true,"exit_code":0,"command":[],"wall_duration_ms":12}]}"#
}

/// Newest-first ordering, ownership, current-receipt exclusion and receipt deduplication match Go.
#[test]
fn history_preserves_order_owned_attempts_current_exclusion_and_go_unknown_fields() {
    let db = fixture();
    add(&db, 1, receipt());
    add(
        &db,
        2,
        br#"{"attempt_id":"other","receipt":"foreign","controls":[{"id":"foreign"}]}"#,
    );
    add(&db, 3, receipt());
    add(&db,4,br#"{"attempt_id":"attempt","revision":9,"receipt":"new","controls":[{"id":"latest","started_at":"start","exit_code":3,"command":["cargo","test"]}]}"#);
    let history = read_validation_control_history(&db, "work", &task(), None).unwrap();
    assert_eq!(
        history.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(),
        ["latest", "control"]
    );
    assert_eq!(history[0].result, "failed");
    assert_eq!(history[0].execution, "not_executed");
    assert_eq!(history[0].revision, "9");
    assert_eq!(history[0].exit_code, Some(3));
    assert_eq!(history[1].command, None);
    assert_eq!(history[1].wall_duration_ms, Some(12));
    assert!(history.iter().all(|x| x.freshness == "historical"));
    let current = AutomaticValidation {
        receipt: "new".into(),
        ..Default::default()
    };
    assert_eq!(
        read_validation_control_history(&db, "work", &task(), Some(&current))
            .unwrap()
            .len(),
        1
    );
    assert!(
        read_validation_control_history(&db, "missing", &task(), None)
            .unwrap()
            .is_empty()
    );
}

/// A malformed receipt cannot become a successful empty history; original bytes remain intact.
#[test]
fn malformed_history_refuses_without_mutation_and_recovers() {
    let db = fixture();
    add(&db, 1, receipt());
    for raw in [
        b"{private-invalid".as_slice(),
        br#"{"attempt_id":42}"#,
        b"[]",
        b"{}{}",
    ] {
        assert_eq!(
            read_validation_control_history(&db, "work", &task(), None)
                .unwrap()
                .len(),
            1
        );
        add(&db, 2, raw);
        let error = read_validation_control_history(&db, "work", &task(), None).unwrap_err();
        assert_eq!(error.code(), "control-history-format");
        assert!(error.source().is_some());
        assert!(!format!("{error:?}").contains("private-invalid"));
        let stored: Vec<u8> = db
            .query_row("SELECT payload FROM events WHERE revision=2", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(stored, raw);
        db.execute("DELETE FROM events WHERE revision=2", [])
            .unwrap();
        assert_eq!(
            read_validation_control_history(&db, "work", &task(), None)
                .unwrap()
                .len(),
            1
        );
    }
}

/// A missing table preserves its SQL cause, with a valid witness before fault and after repair.
#[test]
fn storage_failure_is_not_empty_history_and_no_attempt_needs_no_query() {
    let db = fixture();
    add(&db, 1, receipt());
    assert_eq!(
        read_validation_control_history(&db, "work", &task(), None)
            .unwrap()
            .len(),
        1
    );
    db.execute("ALTER TABLE events RENAME TO retained_events", [])
        .unwrap();
    let error = read_validation_control_history(&db, "work", &task(), None).unwrap_err();
    assert_eq!(error.code(), "control-history-storage");
    assert!(error.source().unwrap().is::<rusqlite::Error>());
    assert!(
        read_validation_control_history(&db, "work", &Task::default(), None)
            .unwrap()
            .is_empty()
    );
    db.execute("ALTER TABLE retained_events RENAME TO events", [])
        .unwrap();
    assert_eq!(
        read_validation_control_history(&db, "work", &task(), None)
            .unwrap()
            .len(),
        1
    );
}

/// A real SQLite scalar failure while reading rows yields no partial success and recovers.
#[test]
fn row_iteration_failure_preserves_sql_cause_and_recovers_without_mutation() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    let db = fixture();
    add(&db, 1, receipt());
    db.execute_batch("ALTER TABLE events RENAME TO retained_events")
        .unwrap();
    let fault = Arc::new(AtomicBool::new(false));
    let configured = fault.clone();
    db.create_scalar_function(
        "history_fixture_payload",
        1,
        rusqlite::functions::FunctionFlags::SQLITE_UTF8,
        move |context| {
            if configured.load(Ordering::SeqCst) {
                return Err(rusqlite::Error::UserFunctionError(Box::new(
                    std::io::Error::other("private-history-read-failure"),
                )));
            }
            context.get::<Vec<u8>>(0)
        },
    )
    .unwrap();
    db.execute_batch("CREATE VIEW events AS SELECT work_id,kind,revision,history_fixture_payload(payload) AS payload FROM retained_events").unwrap();
    assert_eq!(
        read_validation_control_history(&db, "work", &task(), None)
            .unwrap()
            .len(),
        1
    );
    fault.store(true, Ordering::SeqCst);
    let error = read_validation_control_history(&db, "work", &task(), None).unwrap_err();
    assert!(error.source().is_some());
    assert!(!format!("{error:?}").contains("private-history-read-failure"));
    match &error {
        swarm_engine::ControlHistoryError::Sql(_) => {}
        swarm_engine::ControlHistoryError::Finalize { primary, .. } => {
            assert_eq!(primary.code(), "control-history-storage")
        }
        _ => panic!("wrong error family: {error:?}"),
    }
    fault.store(false, Ordering::SeqCst);
    assert_eq!(
        read_validation_control_history(&db, "work", &task(), None)
            .unwrap()
            .len(),
        1
    );
    let stored: Vec<u8> = db
        .query_row("SELECT payload FROM retained_events", [], |row| row.get(0))
        .unwrap();
    assert_eq!(stored, receipt());
}
