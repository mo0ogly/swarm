use rusqlite::{params, Connection};
use swarm_engine::{encode_recorded_events, list_recorded_events};
fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE events(id,work_id,revision,kind,at,payload);")
        .unwrap();
    db
}
fn insert(db: &Connection, revision: i64, payload: &[u8]) {
    db.execute(
        "INSERT INTO main.events VALUES('event','work',?,'change','now',?)",
        params![revision, payload],
    )
    .unwrap();
}
#[test]
fn reads_bytes_orders_filters_and_keeps_callers_transaction() {
    let mut db = fixture();
    insert(&db, 2, b"{ \"n\":2 }");
    insert(&db, -1, b"null");
    db.execute_batch("CREATE TEMP TABLE events(id); INSERT INTO temp.events VALUES('shadow');")
        .unwrap();
    let tx = db.transaction().unwrap();
    insert(&tx, 3, b"[]");
    let events = list_recorded_events(&tx, "work").unwrap();
    assert_eq!(
        events.iter().map(|v| v.revision).collect::<Vec<_>>(),
        [-1, 2, 3]
    );
    assert_eq!(
        events[1].payload.as_deref(),
        Some(b"{ \"n\":2 }".as_slice())
    );
    assert!(list_recorded_events(&tx, "work' OR 1=1")
        .unwrap()
        .is_empty());
    tx.rollback().unwrap();
    assert_eq!(list_recorded_events(&db, "work").unwrap().len(), 2);
}
#[test]
fn scan_rejects_text_null_and_wrong_revision_without_partial_result() {
    let db = fixture();
    insert(&db, 1, b"{}");
    for sql in [
        "INSERT INTO events VALUES('bad','work',2,'k','t','{}')",
        "INSERT INTO events VALUES('bad','work',2,'k','t',NULL)",
        "INSERT INTO events VALUES('bad','work','2.0','k','t',x'7b7d')",
        "INSERT INTO events VALUES(NULL,'work',2,'k','t',x'7b7d')",
    ] {
        db.execute_batch(sql).unwrap();
        let e = list_recorded_events(&db, "work").unwrap_err();
        assert_eq!(e.code(), "event-storage");
        assert!(std::error::Error::source(&e).is_some());
        db.execute(
            "DELETE FROM events WHERE rowid=(SELECT max(rowid) FROM events)",
            [],
        )
        .unwrap();
        assert_eq!(list_recorded_events(&db, "work").unwrap().len(), 1);
    }
}
#[test]
fn serialization_keeps_raw_lexemes_and_refuses_invalid_payload() {
    let db = fixture();
    insert(&db, 1, b"{ \"x\":1e999, \"x\":\"\\ud800\" }");
    let events = list_recorded_events(&db, "work").unwrap();
    let encoded = encode_recorded_events(&events, false).unwrap();
    assert!(String::from_utf8(encoded)
        .unwrap()
        .contains(r#"{"x":1e999,"x":"\ud800"}"#));
    insert(&db, 2, b"PRIVATE invalid");
    let bad = list_recorded_events(&db, "work").unwrap();
    let e = encode_recorded_events(&bad, false).unwrap_err();
    assert_eq!(e.code(), "event-payload");
    assert!(!format!("{e:?}").contains("PRIVATE"));
    assert!(!format!("{:?}", bad[1]).contains("PRIVATE"));
    db.execute("DELETE FROM events WHERE revision=2", [])
        .unwrap();
    assert!(encode_recorded_events(&list_recorded_events(&db, "work").unwrap(), false).is_ok());
}
#[test]
fn blob_empty_is_nil_and_invalid_utf8_is_preserved_only_in_raw_payload() {
    let db = fixture();
    insert(&db, 1, b"");
    let ev = list_recorded_events(&db, "work").unwrap();
    assert_eq!(ev[0].payload, None);
    assert!(encode_recorded_events(&ev, false)
        .unwrap()
        .ends_with(b"\"payload\":null}]"));
    db.execute(
        "UPDATE events SET id=?,payload=?",
        params![vec![0xe2_u8, 0x82], b"\"<>&\xe2\x80\xa8\xff\"".as_slice()],
    )
    .unwrap();
    let ev = list_recorded_events(&db, "work").unwrap();
    let plain = encode_recorded_events(&ev, false).unwrap();
    assert!(plain.windows(12).any(|v| v == b"\\ufffd\\ufffd"));
    assert!(plain.windows(5).any(|v| v == b"\xe2\x80\xa8\xff\""));
    let escaped = encode_recorded_events(&ev, true).unwrap();
    assert!(escaped.windows(6).any(|v| v == b"\\u2028"));
    assert!(escaped.contains(&0xff));
}
#[test]
fn deep_payload_go_limit_and_missing_table_recovery() {
    let db = fixture();
    for depth in [10_000, 10_001] {
        let mut raw = vec![b'['; depth];
        raw.push(b'0');
        raw.extend(vec![b']'; depth]);
        insert(&db, 1, &raw);
        let ev = list_recorded_events(&db, "work").unwrap();
        assert_eq!(encode_recorded_events(&ev, false).is_ok(), depth == 10_000);
        db.execute("DELETE FROM events", []).unwrap();
    }
    db.execute_batch("DROP TABLE events").unwrap();
    assert!(list_recorded_events(&db, "work").is_err());
    db.execute_batch("CREATE TABLE events(id,work_id,revision,kind,at,payload)")
        .unwrap();
    assert_eq!(
        encode_recorded_events(&list_recorded_events(&db, "missing").unwrap(), false).unwrap(),
        b"[]"
    );
}
