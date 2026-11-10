#[cfg(unix)]
use rusqlite::params;
use rusqlite::{Connection, Transaction};
use swarm_engine::{
    reserve_recorded_command, RecordedCommandError, RECORDED_COMMAND_SCHEMA_VERSION,
};
fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.busy_timeout(std::time::Duration::ZERO).unwrap();
    db.execute_batch("CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER,body BLOB); CREATE TABLE events(id,work_id,kind,request); PRAGMA user_version=37;").unwrap();
    db.execute("INSERT INTO works VALUES('work',2,?)",[b"{\"schema_version\":1,\"id\":\"work\",\"revision\":2,\"title\":\"current\",\"tasks\":[]}".as_slice()]).unwrap();
    db
}
fn replay(
    tx: &Transaction<'_>,
    work: &str,
    kind: &str,
    request: &[u8],
) -> Result<Option<swarm_engine::Work>, RecordedCommandError> {
    reserve_recorded_command(tx, work, kind, "event-1", request)
}
fn event(db: &Connection) {
    db.execute(
        "INSERT INTO events VALUES('event-1','work','work.update',?)",
        [b"{\"x\":1}".as_slice()],
    )
    .unwrap();
}
#[test]
fn replay_reads_current_work_and_preserves_outer_transaction_and_main_target() {
    let mut db = fixture();
    event(&db);
    db.execute_batch("CREATE TEMP TABLE works(id);CREATE TEMP TABLE events(id);CREATE TABLE owned(value);INSERT INTO owned VALUES(1)").unwrap();
    let tx = db.transaction().unwrap();
    tx.execute("INSERT INTO owned VALUES(2)", []).unwrap();
    let w = replay(&tx, "work", "work.update", b"{\"x\":1}")
        .unwrap()
        .unwrap();
    assert_eq!(w.title, "current");
    assert_eq!(w.revision, 2);
    assert!(replay(&tx, "", "work.update", b"{\"x\":1}")
        .unwrap()
        .is_some());
    assert!(
        reserve_recorded_command(&tx, "missing", "work.create", "new-event", b"{}")
            .unwrap()
            .is_none()
    );
    tx.rollback().unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM owned", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT revision FROM main.works", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
}
#[test]
fn conflicts_preserve_bytes_and_recover_without_claiming_new_command_is_valid() {
    let mut db = fixture();
    event(&db);
    let tx = db.transaction().unwrap();
    assert!(replay(&tx, "work", "work.update", b"{\"x\":1}")
        .unwrap()
        .is_some());
    for (work, kind, request) in [
        ("other", "work.update", b"{\"x\":1}".as_slice()),
        ("work", "task.update", b"{\"x\":1}"),
        ("work", "work.update", b"{ \"x\":1 }"),
    ] {
        assert_eq!(
            replay(&tx, work, kind, request).unwrap_err().code(),
            "event_conflict"
        );
        assert!(replay(&tx, "work", "work.update", b"{\"x\":1}")
            .unwrap()
            .is_some());
    }
    // No business validation: absence is not permission or a successful mutation.
    assert!(
        reserve_recorded_command(&tx, "bad work", "unknown", "new-event", b"invalid")
            .unwrap()
            .is_none()
    );
    tx.rollback().unwrap();
}
#[test]
fn version_is_checked_before_replay_and_bad_work_never_becomes_empty_success() {
    let mut db = fixture();
    event(&db);
    {
        let tx = db.transaction().unwrap();
        assert!(replay(&tx, "work", "work.update", b"{\"x\":1}")
            .unwrap()
            .is_some());
        tx.rollback().unwrap();
    }
    db.pragma_update(None, "user_version", RECORDED_COMMAND_SCHEMA_VERSION + 1)
        .unwrap();
    {
        let tx = db.transaction().unwrap();
        assert_eq!(
            replay(&tx, "work", "different", b"bad").unwrap_err().code(),
            "storage_version_conflict"
        );
        tx.rollback().unwrap();
    }
    db.pragma_update(None, "user_version", RECORDED_COMMAND_SCHEMA_VERSION)
        .unwrap();
    db.execute("UPDATE works SET body=?", [b"PRIVATE invalid".as_slice()])
        .unwrap();
    {
        let tx = db.transaction().unwrap();
        let e = replay(&tx, "work", "work.update", b"{\"x\":1}").unwrap_err();
        assert!(!format!("{e:?}").contains("PRIVATE"));
        assert!(std::error::Error::source(&e).is_some());
        tx.rollback().unwrap();
    }
    db.execute(
        "UPDATE works SET body=?",
        [b"{\"schema_version\":1,\"id\":\"work\",\"revision\":2}".as_slice()],
    )
    .unwrap();
    let tx = db.transaction().unwrap();
    assert!(replay(&tx, "work", "work.update", b"{\"x\":1}")
        .unwrap()
        .is_some());
    tx.rollback().unwrap();
}
#[test]
fn invalid_event_fails_before_sql_and_request_null_compares_as_empty_bytes() {
    let mut db = fixture();
    event(&db);
    {
        let tx = db.transaction().unwrap();
        assert!(replay(&tx, "work", "work.update", b"{\"x\":1}")
            .unwrap()
            .is_some());
        tx.rollback().unwrap();
    }
    db.execute("UPDATE events SET request=NULL", []).unwrap();
    {
        let tx = db.transaction().unwrap();
        assert!(replay(&tx, "work", "work.update", b"").unwrap().is_some());
        tx.rollback().unwrap();
    }
    db.execute_batch("DROP TABLE works").unwrap();
    let tx = db.transaction().unwrap();
    assert_eq!(
        reserve_recorded_command(&tx, "work", "kind", "bad event", b"")
            .unwrap_err()
            .code(),
        "event-identity"
    );
    assert!(reserve_recorded_command(&tx, "work", "kind", "event", b"").is_err());
    tx.rollback().unwrap();
}
#[cfg(unix)]
fn disk_fixture() -> (std::path::PathBuf, Connection, Connection) {
    use std::os::unix::fs::DirBuilderExt;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .join(".cache");
    std::fs::create_dir_all(&root).unwrap();
    let folder = root.join(format!(
        "command-fixture-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&folder)
        .unwrap();
    let path = folder.join("state.db");
    let a = Connection::open(&path).unwrap();
    a.busy_timeout(std::time::Duration::ZERO).unwrap();
    a.execute_batch("PRAGMA journal_mode=WAL;PRAGMA user_version=37;CREATE TABLE works(id,revision,body);CREATE TABLE events(id,work_id,kind,request)").unwrap();
    let b = Connection::open(&path).unwrap();
    b.busy_timeout(std::time::Duration::ZERO).unwrap();
    (path, a, b)
}
#[cfg(unix)]
#[test]
fn missing_work_still_reserves_writer_busy_is_explicit_and_recovers() {
    let (path, mut a, mut b) = disk_fixture();
    {
        let tx = a.transaction().unwrap();
        assert!(
            reserve_recorded_command(&tx, "missing", "work.create", "event", b"{}")
                .unwrap()
                .is_none()
        );
        tx.rollback().unwrap();
    }
    let tx = a.transaction().unwrap();
    assert!(
        reserve_recorded_command(&tx, "missing", "work.create", "event", b"{}")
            .unwrap()
            .is_none()
    );
    assert!(b
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .is_err());
    tx.rollback().unwrap();
    let blocking = b
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    let tx = a.transaction().unwrap();
    let e = reserve_recorded_command(&tx, "missing", "work.create", "event", b"{}").unwrap_err();
    assert!(e.is_busy());
    tx.rollback().unwrap();
    blocking.rollback().unwrap();
    let tx = a.transaction().unwrap();
    assert!(
        reserve_recorded_command(&tx, "missing", "work.create", "event", b"{}")
            .unwrap()
            .is_none()
    );
    tx.rollback().unwrap();
    a.close().unwrap();
    b.close().unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(path.parent().unwrap()).unwrap();
}
#[cfg(unix)]
#[test]
fn stale_wal_snapshot_is_refused_and_fresh_transaction_recovers() {
    let (path, mut a, b) = disk_fixture();
    {
        let tx = a.transaction().unwrap();
        assert!(
            reserve_recorded_command(&tx, "missing", "kind", "event", b"")
                .unwrap()
                .is_none()
        );
        tx.rollback().unwrap();
    }
    let tx = a.transaction().unwrap();
    assert_eq!(
        tx.query_row("SELECT count(*) FROM works", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    b.execute(
        "INSERT INTO works VALUES('w',1,?)",
        params![b"{\"schema_version\":1,\"id\":\"w\",\"revision\":1}".as_slice()],
    )
    .unwrap();
    let e = reserve_recorded_command(&tx, "w", "kind", "event", b"").unwrap_err();
    assert!(e.is_busy());
    assert_eq!(e.sqlite_extended_code(), Some(517));
    tx.rollback().unwrap();
    let tx = a.transaction().unwrap();
    assert!(reserve_recorded_command(&tx, "w", "kind", "event", b"")
        .unwrap()
        .is_none());
    tx.rollback().unwrap();
    a.close().unwrap();
    b.close().unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(path.parent().unwrap()).unwrap();
}

#[test]
fn trigger_failure_is_not_retryable_busy_and_retains_original_cause_without_leak() {
    let mut db = fixture();
    event(&db);
    {
        let tx = db.transaction().unwrap();
        assert!(replay(&tx, "work", "work.update", b"{\"x\":1}")
            .unwrap()
            .is_some());
        tx.rollback().unwrap();
    }
    db.execute_batch("CREATE TRIGGER fail_reserved_update BEFORE UPDATE ON works BEGIN SELECT RAISE(ABORT,'PRIVATE storage failure'); END").unwrap();
    {
        let tx = db.transaction().unwrap();
        let e = replay(&tx, "work", "work.update", b"{\"x\":1}").unwrap_err();
        assert!(!e.is_busy());
        assert_eq!(e.sqlite_extended_code(), Some(1811));
        assert!(!format!("{e:?}").contains("PRIVATE"));
        assert!(std::error::Error::source(&e).is_some());
        tx.rollback().unwrap();
    }
    db.execute_batch("DROP TRIGGER fail_reserved_update")
        .unwrap();
    let tx = db.transaction().unwrap();
    assert!(replay(&tx, "work", "work.update", b"{\"x\":1}")
        .unwrap()
        .is_some());
    tx.rollback().unwrap();
}
