use rusqlite::{params, types::Value, Connection};
use swarm_engine::{list_recorded_works, read_recorded_work};
fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.busy_timeout(std::time::Duration::ZERO).unwrap();
    db.execute_batch(
        "CREATE TABLE works(id TEXT PRIMARY KEY, revision INTEGER NOT NULL, body BLOB NOT NULL);",
    )
    .unwrap();
    db
}
fn body(id: &str, revision: i64) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"schema_version":1,"id":id,"revision":revision,"title":"SQL reader fixture","tasks":[]})).unwrap()
}
fn insert(db: &Connection, id: &str) {
    db.execute(
        "INSERT INTO main.works VALUES(?,1,?)",
        params![id, body(id, 1)],
    )
    .unwrap();
}
fn rows(db: &Connection) -> Vec<(String, String, String, String)> {
    db.prepare(
        "SELECT hex(id),typeof(revision),hex(revision),hex(body) FROM main.works ORDER BY rowid",
    )
    .unwrap()
    .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
    .unwrap()
    .collect::<rusqlite::Result<_>>()
    .unwrap()
}
#[test]
fn sql_readers_return_content_actual_order_and_meaningful_empty_or_missing_results() {
    let db = fixture();
    assert!(list_recorded_works(&db).unwrap().is_empty());
    assert_eq!(
        read_recorded_work(&db, "missing").unwrap_err().code(),
        "work-missing"
    );
    insert(&db, "work-a");
    insert(&db, "work-b");
    let before = rows(&db);
    let list = list_recorded_works(&db).unwrap();
    assert_eq!(
        list.iter().map(|w| w.id.as_str()).collect::<Vec<_>>(),
        ["work-b", "work-a"]
    );
    assert_eq!(list[1], read_recorded_work(&db, "work-a").unwrap());
    assert_eq!(list[0].title, "SQL reader fixture");
    assert_eq!(list[0].revision, 1);
    assert_eq!(rows(&db), before);
    assert_eq!(
        read_recorded_work(&db, "work-a' OR 1=1")
            .unwrap_err()
            .code(),
        "row-identity"
    );
    assert_eq!(rows(&db), before);
}
#[test]
fn a_bad_row_rejects_the_whole_list_without_mutation_and_recovers() {
    for invalid in [
        b"".as_slice(),
        b"null",
        b"[]",
        b"{",
        b"\xff",
        br#"{"schema_version":2,"id":"work-a","revision":1}"#,
        br#"{"schema_version":1,"id":"wrong","revision":1}"#,
        br#"{"schema_version":1,"id":"work-a","revision":2}"#,
    ] {
        let db = fixture();
        insert(&db, "work-a");
        insert(&db, "work-b");
        assert_eq!(list_recorded_works(&db).unwrap().len(), 2);
        db.execute("UPDATE main.works SET body=? WHERE id='work-a'", [invalid])
            .unwrap();
        let before = rows(&db);
        assert!(read_recorded_work(&db, "work-a").is_err());
        // Newest valid row is decoded first. No partial list escapes afterwards.
        assert!(list_recorded_works(&db).is_err());
        assert_eq!(rows(&db), before);
        db.execute(
            "UPDATE main.works SET body=? WHERE id='work-a'",
            [body("work-a", 1)],
        )
        .unwrap();
        assert_eq!(list_recorded_works(&db).unwrap().len(), 2);
    }
}
#[test]
fn row_revision_types_follow_go_scan_and_still_require_envelope_coherence() {
    let db = fixture();
    insert(&db, "work-a");
    for good in [
        Value::Integer(1),
        Value::Text("+01".into()),
        Value::Blob(b"+01".to_vec()),
        Value::Real(1.0),
    ] {
        db.execute("UPDATE main.works SET revision=?", [good])
            .unwrap();
        assert_eq!(read_recorded_work(&db, "work-a").unwrap().revision, 1);
    }
    for bad in [
        Value::Integer(0),
        Value::Integer(-1),
        Value::Integer(2),
        Value::Blob(b"1.0".to_vec()),
        Value::Blob(b"1e0".to_vec()),
        Value::Blob(b" 1".to_vec()),
        Value::Blob(b"9223372036854775808".to_vec()),
        Value::Real(1.5),
    ] {
        db.execute("UPDATE main.works SET revision=1", []).unwrap();
        assert!(read_recorded_work(&db, "work-a").is_ok());
        db.execute("UPDATE main.works SET revision=?", [bad])
            .unwrap();
        let before = rows(&db);
        assert!(list_recorded_works(&db).is_err());
        assert_eq!(rows(&db), before);
    }
    db.execute("UPDATE main.works SET revision=1", []).unwrap();
    assert!(read_recorded_work(&db, "work-a").is_ok());
}
#[test]
fn sql_readers_preserve_caller_transaction_snapshot_and_ignore_temp_shadows() {
    let mut db = fixture();
    insert(&db, "work-a");
    {
        let tx = db.transaction().unwrap();
        insert(&tx, "work-b");
        assert_eq!(list_recorded_works(&tx).unwrap().len(), 2);
        assert!(read_recorded_work(&tx, "work-b").is_ok());
        tx.rollback().unwrap();
    }
    assert_eq!(list_recorded_works(&db).unwrap().len(), 1);
    assert_eq!(
        read_recorded_work(&db, "work-b").unwrap_err().code(),
        "work-missing"
    );
    db.execute_batch("CREATE TEMP TABLE works(id TEXT,revision INTEGER,body BLOB);")
        .unwrap();
    db.execute(
        "INSERT INTO temp.works VALUES('work-a',9,?)",
        [body("work-a", 9)],
    )
    .unwrap();
    assert_eq!(read_recorded_work(&db, "work-a").unwrap().revision, 1);
    assert_eq!(list_recorded_works(&db).unwrap().len(), 1);
}
#[test]
fn database_and_decode_causes_remain_available_while_messages_are_safe() {
    let db = fixture();
    insert(&db, "work-a");
    db.execute(
        "UPDATE main.works SET body=?",
        [b"private-body-not-json".as_slice()],
    )
    .unwrap();
    let err = read_recorded_work(&db, "work-a").unwrap_err();
    assert!(std::error::Error::source(&err).is_some());
    assert!(!format!("{err:?}").contains("private-body"));
    db.execute_batch("DROP TABLE main.works;").unwrap();
    let err = list_recorded_works(&db).unwrap_err();
    assert_eq!(err.code(), "work-storage");
    assert!(std::error::Error::source(&err).is_some());
}
