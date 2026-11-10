use rusqlite::Connection;
use std::{error::Error, io};
use swarm_engine::{
    decode_work, read_recorded_work, stage_recorded_change, stage_recorded_change_with_hook,
    RecordedChange, RecordedChangeOutcome, Work,
};
fn db() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("PRAGMA user_version=37;PRAGMA foreign_keys=ON;CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,body BLOB NOT NULL);CREATE TABLE events(id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id),revision INTEGER NOT NULL,kind TEXT NOT NULL,at TEXT NOT NULL,payload BLOB NOT NULL,request BLOB NOT NULL,UNIQUE(work_id,revision));CREATE TABLE notes(value TEXT);").unwrap();
    c
}
fn work(rev: i64) -> Work {
    decode_work(format!(r#"{{"schema_version":1,"id":"work","revision":{rev},"title":"candidate","updated":"2026-10-18T00:00:00Z","tasks":[]}}"#).as_bytes()).unwrap()
}
fn change(w: &Work) -> RecordedChange<'_> {
    RecordedChange {
        work: w,
        expected_revision: w.revision - 1,
        event: if w.revision == 1 { "create" } else { "update" },
        kind: if w.revision == 1 {
            "work.create"
        } else {
            "work.update"
        },
        request: b"{}",
    }
}
fn create(c: &mut Connection) {
    let mut t = c.transaction().unwrap();
    stage_recorded_change(&mut t, change(&work(1))).unwrap();
    t.commit().unwrap();
}
type HookCause = Box<dyn Error + Send + Sync>;
fn control(c: &mut Connection) {
    let mut tx = c.transaction().unwrap();
    stage_recorded_change_with_hook(&mut tx, change(&work(2)), |db, w| {
        assert_eq!(read_recorded_work(db, "work").unwrap(), *w);
        assert_eq!(
            db.query_row("SELECT count(*) FROM events WHERE id='update'", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
        db.execute("INSERT INTO notes VALUES('control')", [])?;
        Ok(())
    })
    .unwrap();
    tx.rollback().unwrap();
    assert_eq!(read_recorded_work(c, "work").unwrap().revision, 1);
}
#[test]
fn hook_observes_persisted_work_before_event_and_commits_with_both() {
    let mut c = db();
    create(&mut c);
    control(&mut c);
    let mut tx = c.transaction().unwrap();
    stage_recorded_change_with_hook(&mut tx, change(&work(2)), |db, w| {
        assert_eq!(read_recorded_work(db, "work").unwrap(), *w);
        assert_eq!(
            db.query_row("SELECT count(*) FROM events WHERE id='update'", [], |r| r
                .get::<_, i64>(
                0
            ))?,
            0
        );
        db.execute("INSERT INTO notes VALUES('hook')", [])?;
        Ok(())
    })
    .unwrap();
    tx.commit().unwrap();
    assert_eq!(read_recorded_work(&c, "work").unwrap().revision, 2);
    assert_eq!(
        c.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        c.query_row("SELECT value FROM notes", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "hook"
    );
}
#[test]
fn failed_hook_preserves_cause_rolls_back_its_effects_and_reuses_same_event_after_repair() {
    let mut c = db();
    create(&mut c);
    control(&mut c);
    let mut tx = c.transaction().unwrap();
    tx.execute("INSERT INTO notes VALUES('owner')", []).unwrap();
    let e = stage_recorded_change_with_hook(&mut tx, change(&work(2)), |db, _| {
        db.execute("INSERT INTO notes VALUES('private hook')", [])?;
        Err(Box::new(io::Error::other("PRIVATE hook cause")) as HookCause)
    })
    .unwrap_err();
    assert_eq!(e.code(), "recorded-change-hook");
    assert!(!format!("{e:?}").contains("PRIVATE"));
    assert!(e.source().unwrap().to_string().contains("PRIVATE"));
    assert!(!e.outer_transaction_lost());
    assert_eq!(read_recorded_work(&tx, "work").unwrap().revision, 1);
    assert_eq!(
        tx.query_row("SELECT count(*) FROM notes", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    stage_recorded_change_with_hook(&mut tx, change(&work(2)), |db, _| {
        db.execute("INSERT INTO notes VALUES('recovered')", [])?;
        Ok(())
    })
    .unwrap();
    tx.commit().unwrap();
    assert_eq!(
        c.query_row("SELECT group_concat(value,',') FROM notes", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "owner,recovered"
    );
}
#[test]
fn replay_precedes_hook_and_never_reapplies_hook_side_effects() {
    let mut c = db();
    let mut tx = c.transaction().unwrap();
    stage_recorded_change_with_hook(&mut tx, change(&work(1)), |db, _| {
        db.execute("INSERT INTO notes VALUES('once')", [])?;
        Ok(())
    })
    .unwrap();
    tx.commit().unwrap();
    let mut tx = c.transaction().unwrap();
    let mut invalid = work(1);
    invalid.schema_version = 0;
    let outcome = stage_recorded_change_with_hook(&mut tx, change(&invalid), |_, _| {
        panic!("hook must not run on replay")
    })
    .unwrap();
    assert!(matches!(outcome,RecordedChangeOutcome::Replayed(w)if w.revision==1));
    tx.rollback().unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM notes", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn hook_and_trigger_tampering_is_rejected_before_event_and_rolls_back() {
    let mut c = db();
    create(&mut c);
    for sql in [
        "UPDATE works SET body=x'7b7d' WHERE id='work'",
        "UPDATE works SET revision=8 WHERE id='work'",
        "PRAGMA user_version=99",
    ] {
        control(&mut c);
        let mut tx = c.transaction().unwrap();
        let e = stage_recorded_change_with_hook(&mut tx, change(&work(2)), |db, _| {
            db.execute_batch(sql)?;
            Ok(())
        })
        .unwrap_err();
        assert!(!e.outer_transaction_lost());
        assert_eq!(read_recorded_work(&tx, "work").unwrap().revision, 1);
        tx.commit().unwrap();
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            37
        );
    }
    c.execute_batch("CREATE TRIGGER tamper AFTER UPDATE ON works WHEN NEW.revision=2 BEGIN UPDATE works SET body=x'7b7d' WHERE id=NEW.id;END").unwrap();
    let mut tx = c.transaction().unwrap();
    assert!(
        stage_recorded_change_with_hook(&mut tx, change(&work(2)), |_, _| panic!(
            "invalid stored candidate must not reach hook"
        ))
        .is_err()
    );
    tx.rollback().unwrap();
    c.execute_batch("DROP TRIGGER tamper").unwrap();
    control(&mut c);
}
#[test]
fn hook_transaction_loss_is_reported_without_repeating_or_claiming_rollback() {
    for sql in ["ROLLBACK", "COMMIT"] {
        let mut c = db();
        create(&mut c);
        control(&mut c);
        let mut tx = c.transaction().unwrap();
        let e = stage_recorded_change_with_hook(&mut tx, change(&work(2)), |db, _| {
            db.execute_batch(sql)?;
            Ok(())
        })
        .unwrap_err();
        assert!(e.outer_transaction_lost());
        drop(tx);
        assert_eq!(
            read_recorded_work(&c, "work").unwrap().revision,
            if sql == "COMMIT" { 2 } else { 1 }
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM events WHERE id='update'", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
    }
}
