use rusqlite::{params, Connection};
use swarm_engine::{
    decode_work, list_recorded_events, read_recorded_work, stage_recorded_change, RecordedChange,
    RecordedChangeOutcome,
};
fn work(revision: i64) -> swarm_engine::Work {
    decode_work(format!(r#"{{"schema_version":1,"id":"work","revision":{revision},"title":"candidate","updated":"2026-10-18T00:00:00Z","tasks":[]}}"#).as_bytes()).unwrap()
}
fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.busy_timeout(std::time::Duration::ZERO).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON;PRAGMA user_version=37;CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,body BLOB NOT NULL);CREATE TABLE events(id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id),revision INTEGER NOT NULL,kind TEXT NOT NULL,at TEXT NOT NULL,payload BLOB NOT NULL,request BLOB NOT NULL,UNIQUE(work_id,revision));CREATE TABLE owner_notes(note TEXT);CREATE TABLE sidecar(value TEXT);").unwrap();
    db
}
fn change<'a>(
    w: &'a swarm_engine::Work,
    kind: &'a str,
    expected: i64,
    event: &'a str,
) -> RecordedChange<'a> {
    RecordedChange {
        work: w,
        kind,
        event,
        expected_revision: expected,
        request: b"{\"request\":true}",
    }
}
fn create(db: &mut Connection) {
    let mut tx = db.transaction().unwrap();
    assert!(matches!(
        stage_recorded_change(&mut tx, change(&work(1), "work.create", 0, "create")).unwrap(),
        RecordedChangeOutcome::Staged
    ));
    tx.commit().unwrap();
}
fn valid_update_control(db: &mut Connection) {
    let mut tx = db.transaction().unwrap();
    assert!(matches!(
        stage_recorded_change(&mut tx, change(&work(2), "work.update", 1, "update")).unwrap(),
        RecordedChangeOutcome::Staged
    ));
    tx.rollback().unwrap();
}
#[test]
fn work_and_event_are_staged_together_commit_is_owned_by_caller_and_replay_is_current() {
    let mut db = fixture();
    {
        let mut tx = db.transaction().unwrap();
        let w = work(1);
        assert!(matches!(
            stage_recorded_change(&mut tx, change(&w, "work.create", 0, "create")).unwrap(),
            RecordedChangeOutcome::Staged
        ));
        assert_eq!(read_recorded_work(&tx, "work").unwrap(), w);
        let ev = list_recorded_events(&tx, "work").unwrap();
        assert_eq!(ev.len(), 1);
        assert_eq!(
            ev[0].payload.as_deref(),
            Some(b"{\"request\":true}".as_slice())
        );
        tx.rollback().unwrap();
    }
    assert!(list_recorded_events(&db, "work").unwrap().is_empty());
    assert!(read_recorded_work(&db, "work").is_err());
    create(&mut db);
    let mut tx = db.transaction().unwrap();
    let w = work(2);
    assert!(matches!(
        stage_recorded_change(&mut tx, change(&w, "work.update", 1, "update")).unwrap(),
        RecordedChangeOutcome::Staged
    ));
    let mut invalid = work(-1);
    invalid.schema_version = 0;
    match stage_recorded_change(&mut tx, change(&invalid, "work.create", -1, "create")).unwrap() {
        RecordedChangeOutcome::Replayed(current) => assert_eq!(*current, w),
        _ => panic!("expected replay"),
    }
    assert_eq!(list_recorded_events(&tx, "work").unwrap().len(), 2);
    tx.commit().unwrap();
    assert_eq!(read_recorded_work(&db, "work").unwrap().revision, 2);
}
#[test]
fn event_failure_rolls_back_work_and_identity_stamp_without_losing_owner_changes_then_recovers() {
    let mut db = fixture();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    db.execute_batch(swarm_engine::IDENTITY_PLAN_HEADS_SQL)
        .unwrap();
    create(&mut db);
    valid_update_control(&mut db);
    let before = swarm_engine::read_identity_plan_stamp(
        &db,
        "work",
        swarm_engine::IdentityPlanProjection::V1,
    )
    .unwrap();
    db.execute_batch("CREATE TRIGGER fail_event BEFORE INSERT ON events WHEN NEW.kind='work.update' BEGIN INSERT INTO sidecar VALUES('during');SELECT RAISE(ABORT,'PRIVATE failure');END").unwrap();
    let mut tx = db.transaction().unwrap();
    tx.execute("INSERT INTO owner_notes VALUES('before')", [])
        .unwrap();
    let e =
        stage_recorded_change(&mut tx, change(&work(2), "work.update", 1, "update")).unwrap_err();
    assert!(!format!("{e:?}").contains("PRIVATE"));
    assert!(std::error::Error::source(&e).is_some());
    assert!(!e.outer_transaction_lost());
    assert_eq!(read_recorded_work(&tx, "work").unwrap().revision, 1);
    assert_eq!(list_recorded_events(&tx, "work").unwrap().len(), 1);
    assert_eq!(
        swarm_engine::read_identity_plan_stamp(
            &tx,
            "work",
            swarm_engine::IdentityPlanProjection::V1
        )
        .unwrap(),
        before
    );
    assert_eq!(
        tx.query_row("SELECT count(*) FROM owner_notes", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        tx.query_row("SELECT count(*) FROM sidecar", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    tx.execute_batch("DROP TRIGGER fail_event").unwrap();
    assert!(stage_recorded_change(&mut tx, change(&work(2), "work.update", 1, "update")).is_ok());
    tx.commit().unwrap();
    assert_eq!(read_recorded_work(&db, "work").unwrap().revision, 2);
}
#[test]
fn stale_revision_invalid_envelope_overflow_and_corruption_are_refused_without_partial_writes() {
    let mut db = fixture();
    create(&mut db);
    valid_update_control(&mut db);
    for (revision, expected) in [(2, 0), (3, 1), (-1, 1)] {
        let mut tx = db.transaction().unwrap();
        assert!(stage_recorded_change(
            &mut tx,
            change(&work(revision), "work.update", expected, "bad")
        )
        .is_err());
        assert_eq!(read_recorded_work(&tx, "work").unwrap().revision, 1);
        assert_eq!(list_recorded_events(&tx, "work").unwrap().len(), 1);
        tx.rollback().unwrap();
    }
    let mut tx = db.transaction().unwrap();
    let mut wrong = work(2);
    wrong.schema_version = 2;
    assert!(stage_recorded_change(&mut tx, change(&wrong, "work.update", 1, "bad")).is_err());
    tx.rollback().unwrap();
    let max = work(i64::MAX);
    db.execute(
        "UPDATE works SET revision=?,body=?",
        params![i64::MAX, swarm_engine::encode_model(&max).unwrap()],
    )
    .unwrap();
    let mut tx = db.transaction().unwrap();
    assert_eq!(
        stage_recorded_change(&mut tx, change(&max, "work.update", i64::MAX, "overflow"))
            .unwrap_err()
            .code(),
        "recorded-change-overflow"
    );
    assert_eq!(read_recorded_work(&tx, "work").unwrap().revision, i64::MAX);
    tx.rollback().unwrap();
    db.execute(
        "UPDATE works SET revision=1,body=?",
        [swarm_engine::encode_model(&work(1)).unwrap()],
    )
    .unwrap();
    valid_update_control(&mut db);
    db.execute("UPDATE works SET body=?", [b"PRIVATE invalid".as_slice()])
        .unwrap();
    let mut tx = db.transaction().unwrap();
    assert_eq!(
        stage_recorded_change(&mut tx, change(&work(2), "work.update", 1, "bad"))
            .unwrap_err()
            .code(),
        "format"
    );
    assert_eq!(list_recorded_events(&tx, "work").unwrap().len(), 1);
    tx.rollback().unwrap();
}
#[test]
fn ignored_insert_or_trigger_tampering_cannot_report_staged_success() {
    for sql in ["CREATE TRIGGER ignore_work BEFORE INSERT ON works BEGIN SELECT RAISE(IGNORE);END", "CREATE TRIGGER ignore_event BEFORE INSERT ON events BEGIN SELECT RAISE(IGNORE);END", "CREATE TRIGGER alter_event AFTER INSERT ON events BEGIN UPDATE events SET payload=x'7b7d' WHERE id=NEW.id;END", "CREATE TRIGGER alter_work AFTER INSERT ON events BEGIN UPDATE works SET body=x'7b7d' WHERE id=NEW.work_id;END"] {
  let mut db=fixture();create(&mut db);db.execute("DELETE FROM events",[]).unwrap();db.execute("DELETE FROM works",[]).unwrap();db.execute_batch(sql).unwrap();
  let mut tx=db.transaction().unwrap();assert!(stage_recorded_change(&mut tx,change(&work(1),"work.create",0,"create")).is_err());assert!(list_recorded_events(&tx,"work").unwrap().is_empty());assert!(read_recorded_work(&tx,"work").is_err());tx.rollback().unwrap();
 }
}
#[test]
fn rollback_trigger_reports_lost_outer_transaction_and_allows_a_fresh_recovery() {
    let mut db = fixture();
    create(&mut db);
    valid_update_control(&mut db);
    db.execute_batch("CREATE TRIGGER lose_transaction BEFORE INSERT ON events WHEN NEW.kind='work.update' BEGIN SELECT RAISE(ROLLBACK,'PRIVATE rollback');END").unwrap();
    let mut tx = db.transaction().unwrap();
    tx.execute("INSERT INTO owner_notes VALUES('before')", [])
        .unwrap();
    let e =
        stage_recorded_change(&mut tx, change(&work(2), "work.update", 1, "update")).unwrap_err();
    assert!(e.outer_transaction_lost());
    assert!(!format!("{e:?}").contains("PRIVATE"));
    assert!(tx.is_autocommit());
    drop(tx);
    assert_eq!(read_recorded_work(&db, "work").unwrap().revision, 1);
    assert_eq!(list_recorded_events(&db, "work").unwrap().len(), 1);
    assert_eq!(
        db.query_row("SELECT count(*) FROM owner_notes", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    db.execute_batch("DROP TRIGGER lose_transaction").unwrap();
    let mut tx = db.transaction().unwrap();
    assert!(stage_recorded_change(&mut tx, change(&work(2), "work.update", 1, "update")).is_ok());
    tx.commit().unwrap();
}
#[test]
fn main_tables_are_targeted_even_with_temporary_names() {
    let mut db = fixture();
    db.execute_batch("CREATE TEMP TABLE works(id);CREATE TEMP TABLE events(id)")
        .unwrap();
    create(&mut db);
    assert_eq!(read_recorded_work(&db, "work").unwrap().revision, 1);
    assert_eq!(list_recorded_events(&db, "work").unwrap().len(), 1);
}

#[test]
fn outer_commit_failure_is_not_a_staged_success_and_rolls_back_the_whole_unit() {
    let mut db = fixture();
    db.execute_batch("CREATE TABLE deferred_child(work_id TEXT REFERENCES works(id) DEFERRABLE INITIALLY DEFERRED)").unwrap();
    {
        let mut tx = db.transaction().unwrap();
        assert!(
            stage_recorded_change(&mut tx, change(&work(1), "work.create", 0, "create")).is_ok()
        );
        tx.rollback().unwrap();
    }
    let mut tx = db.transaction().unwrap();
    tx.execute("INSERT INTO deferred_child VALUES('missing')", [])
        .unwrap();
    assert!(matches!(
        stage_recorded_change(&mut tx, change(&work(1), "work.create", 0, "create")).unwrap(),
        RecordedChangeOutcome::Staged
    ));
    assert!(tx.commit().is_err());
    assert!(read_recorded_work(&db, "work").is_err());
    assert!(list_recorded_events(&db, "work").unwrap().is_empty());
    assert_eq!(
        db.query_row("SELECT count(*) FROM deferred_child", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    create(&mut db);
}

#[test]
fn v2_identity_head_and_projection_history_survive_a_rejected_change() {
    let mut db = fixture();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    db.execute_batch(swarm_engine::IDENTITY_PLAN_HEADS_SQL)
        .unwrap();
    create(&mut db);
    {
        let mut tx = db.transaction().unwrap();
        swarm_engine::upgrade_identity_plan_projection(&mut tx).unwrap();
        tx.commit().unwrap();
    }
    valid_update_control(&mut db);
    let stamp = swarm_engine::read_identity_plan_stamp(
        &db,
        "work",
        swarm_engine::IdentityPlanProjection::V2,
    )
    .unwrap();
    let history: i64 = db
        .query_row(
            "SELECT count(*) FROM agent_identity_plan_projection_history",
            [],
            |r| r.get(0),
        )
        .unwrap();
    db.execute_batch("CREATE TRIGGER fail_event BEFORE INSERT ON events WHEN NEW.kind='work.update' BEGIN SELECT RAISE(ABORT,'v2 fixture');END").unwrap();
    let mut tx = db.transaction().unwrap();
    assert!(stage_recorded_change(&mut tx, change(&work(2), "work.update", 1, "update")).is_err());
    assert_eq!(
        swarm_engine::read_identity_plan_stamp(
            &tx,
            "work",
            swarm_engine::IdentityPlanProjection::V2
        )
        .unwrap(),
        stamp
    );
    assert_eq!(
        tx.query_row(
            "SELECT count(*) FROM agent_identity_plan_projection_history",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        history
    );
    tx.rollback().unwrap();
}

#[test]
fn identity_bytes_are_revalidated_even_when_sql_collation_matches_changed_ids() {
    let mut wrongly_accepted = Vec::new();
    for (name,trigger) in [("work-id","CREATE TRIGGER change_id AFTER INSERT ON events BEGIN UPDATE works SET id=lower(id) WHERE id=NEW.work_id;END"), ("event-id","CREATE TRIGGER change_id AFTER INSERT ON events BEGIN UPDATE events SET id=lower(id) WHERE id=NEW.id;END")] {
        let mut db=fixture();db.execute_batch("DROP TABLE events;DROP TABLE works;CREATE TABLE works(id TEXT PRIMARY KEY COLLATE NOCASE,revision INTEGER NOT NULL,body BLOB NOT NULL);CREATE TABLE events(id TEXT PRIMARY KEY COLLATE NOCASE,work_id TEXT NOT NULL REFERENCES works(id),revision INTEGER NOT NULL,kind TEXT NOT NULL,at TEXT NOT NULL,payload BLOB NOT NULL,request BLOB NOT NULL,UNIQUE(work_id,revision))").unwrap();
        let mut candidate=work(1);candidate.id="WORK".into();
        {let mut tx=db.transaction().unwrap();assert!(stage_recorded_change(&mut tx,change(&candidate,"work.create",0,"CREATE")).is_ok());tx.rollback().unwrap();}
        db.execute_batch(trigger).unwrap();let mut tx=db.transaction().unwrap();
        match stage_recorded_change(&mut tx,change(&candidate,"work.create",0,"CREATE")) {
            Ok(_)=>wrongly_accepted.push(name),
            Err(e)=>{assert_eq!(e.code(),"recorded-change-postcondition");assert_eq!(tx.query_row("SELECT count(*) FROM works",[],|r|r.get::<_,i64>(0)).unwrap(),0);assert_eq!(tx.query_row("SELECT count(*) FROM events",[],|r|r.get::<_,i64>(0)).unwrap(),0);}
        }
        tx.rollback().unwrap();
    }
    assert!(
        wrongly_accepted.is_empty(),
        "changed identity bytes wrongly accepted: {wrongly_accepted:?}"
    );
}

#[test]
fn replay_requires_exact_event_identity_in_both_prefix_and_storage_api() {
    let mut db = fixture();
    db.execute_batch("DROP TABLE events;DROP TABLE works;CREATE TABLE works(id TEXT PRIMARY KEY COLLATE NOCASE,revision INTEGER NOT NULL,body BLOB NOT NULL);CREATE TABLE events(id TEXT PRIMARY KEY COLLATE NOCASE,work_id TEXT NOT NULL REFERENCES works(id),revision INTEGER NOT NULL,kind TEXT NOT NULL,at TEXT NOT NULL,payload BLOB NOT NULL,request BLOB NOT NULL,UNIQUE(work_id,revision))").unwrap();
    let candidate = work(1);
    {
        let mut tx = db.transaction().unwrap();
        assert!(
            stage_recorded_change(&mut tx, change(&candidate, "work.create", 0, "CREATE")).is_ok()
        );
        tx.commit().unwrap();
    }
    let mut tx = db.transaction().unwrap();
    assert!(swarm_engine::reserve_recorded_command(
        &tx,
        "",
        "work.create",
        "CREATE",
        b"{\"request\":true}"
    )
    .unwrap()
    .is_some());
    assert!(matches!(
        stage_recorded_change(&mut tx, change(&candidate, "work.create", 0, "CREATE")).unwrap(),
        RecordedChangeOutcome::Replayed(_)
    ));
    let mut wrongly_accepted = Vec::new();
    match swarm_engine::reserve_recorded_command(
        &tx,
        "",
        "work.create",
        "create",
        b"{\"request\":true}",
    ) {
        Ok(_) => wrongly_accepted.push("prefix"),
        Err(e) => assert_eq!(e.code(), "event_conflict"),
    }
    match stage_recorded_change(&mut tx, change(&candidate, "work.create", 0, "create")) {
        Ok(_) => wrongly_accepted.push("storage"),
        Err(e) => assert_eq!(e.code(), "event_conflict"),
    }
    assert_eq!(list_recorded_events(&tx, "work").unwrap().len(), 1);
    tx.rollback().unwrap();
    assert!(
        wrongly_accepted.is_empty(),
        "different key bytes replayed: {wrongly_accepted:?}"
    );
}

#[test]
fn stale_transaction_handle_cannot_start_a_new_implicitly_committed_unit() {
    let mut db = fixture();
    let candidate = work(1);
    {
        let mut tx = db.transaction().unwrap();
        assert!(
            stage_recorded_change(&mut tx, change(&candidate, "work.create", 0, "create")).is_ok()
        );
        tx.rollback().unwrap();
    }
    let mut tx = db.transaction().unwrap();
    tx.execute_batch("ROLLBACK").unwrap();
    assert!(tx.is_autocommit());
    let prefix = swarm_engine::reserve_recorded_command(
        &tx,
        "",
        "work.create",
        "create",
        b"{\"request\":true}",
    );
    let prefix_accepted = prefix.is_ok();
    if let Err(e) = prefix {
        assert_eq!(e.code(), "command-transaction-inactive");
    }
    let staged = stage_recorded_change(&mut tx, change(&candidate, "work.create", 0, "create"));
    let stage_accepted = staged.is_ok();
    if let Err(e) = staged {
        assert_eq!(e.code(), "command-transaction-inactive");
        assert!(e.outer_transaction_lost());
    }
    let rows: (i64, i64) = tx
        .query_row(
            "SELECT (SELECT count(*) FROM works),(SELECT count(*) FROM events)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    drop(tx);
    assert!(!prefix_accepted&&!stage_accepted&&rows==(0,0),"stale handle accepted: prefix={prefix_accepted},stage={stage_accepted},durable_rows={rows:?}");
    create(&mut db);
    assert_eq!(read_recorded_work(&db, "work").unwrap().revision, 1);
}
