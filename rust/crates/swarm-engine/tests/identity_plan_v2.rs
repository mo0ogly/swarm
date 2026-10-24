use rusqlite::{params, Connection};
use swarm_engine::{
    identity_plan_digest, identity_plan_digest_for, read_identity_plan_stamp,
    register_identity_plan_projection, upgrade_identity_plan_projection,
    IdentityPlanProjection::{V1, V2},
    IDENTITY_PLAN_HEADS_SQL,
};
use swarm_identity_contract::Scope;
const BASE: &[u8] = br#"{"tasks":[{"id":"task-a","owner":"agent-a"}]}"#;
const RUNTIME: &[u8] = br#"{"tasks":[{"id":"task-a","owner":"agent-a","automatic_validation":{"state":"passed"},"manual_override":{"actor":"human","reason":"fixture"}}]}"#;
const CHANGED: &[u8] = br#"{"tasks":[{"id":"task-a","owner":"agent-b"}]}"#;
fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.busy_timeout(std::time::Duration::ZERO).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE works(id TEXT PRIMARY KEY,body BLOB NOT NULL); CREATE TABLE caller_notes(note TEXT); PRAGMA user_version=36;").unwrap();
    register_identity_plan_projection(&db).unwrap();
    db.execute("INSERT INTO works VALUES('work-a',?)", [BASE])
        .unwrap();
    db.execute_batch(IDENTITY_PLAN_HEADS_SQL).unwrap();
    db
}
fn schema(db: &Connection) -> Vec<(String, String, Option<String>)> {
    db.prepare("SELECT type,name,sql FROM sqlite_schema ORDER BY name")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}
fn heads(db: &Connection) -> Vec<(String, i64, String, i64)> {
    db.prepare("SELECT work_id,generation,digest,projection_version FROM agent_identity_plan_heads ORDER BY work_id").unwrap().query_map([], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap().collect::<rusqlite::Result<_>>().unwrap()
}
fn upgrade(db: &mut Connection) {
    let mut tx = db.transaction().unwrap();
    upgrade_identity_plan_projection(&mut tx).unwrap();
    tx.commit().unwrap();
}

#[test]
fn reproduced_runtime_results_should_not_change_plan_definition() {
    let valid = identity_plan_digest_for(BASE, V2).unwrap();
    assert_eq!(valid.len(), 64);
    assert_eq!(valid, identity_plan_digest_for(RUNTIME, V2).unwrap());
    // The old algorithm remains readable byte-for-byte, including its defect.
    assert_ne!(
        identity_plan_digest(BASE).unwrap(),
        identity_plan_digest(RUNTIME).unwrap()
    );
    assert_eq!(
        identity_plan_digest(BASE).unwrap(),
        identity_plan_digest_for(BASE, V1).unwrap()
    );
    assert_ne!(identity_plan_digest(BASE).unwrap(), valid);
    assert_ne!(valid, identity_plan_digest_for(CHANGED, V2).unwrap());
    for key in [
        "auto_validation",
        "override",
        "automatic_validation",
        "manual_override",
    ] {
        let raw = format!(
            "{{\"tasks\":[{{\"id\":\"task-a\",\"owner\":\"agent-a\",\"{key}\":{{\"value\":1}}}}]}}"
        );
        assert_eq!(valid, identity_plan_digest_for(raw.as_bytes(), V2).unwrap());
    }
    // Only task-level runtime fields are newly excluded; unknown definitions bind.
    for raw in [
        br#"{"tasks":[{"id":"task-a","owner":"agent-a","custom":1}]}"#.as_slice(),
        br#"{"tasks":[{"id":"task-a","owner":"agent-a"}],"manual_override":true}"#,
    ] {
        assert_ne!(valid, identity_plan_digest_for(raw, V2).unwrap());
    }
    for invalid in [b"".as_slice(), b"null", b"[]", b"{", br#"{"planning":{}}"#] {
        assert_eq!(valid, identity_plan_digest_for(BASE, V2).unwrap());
        assert!(identity_plan_digest_for(invalid, V2).is_err());
        assert_eq!(valid, identity_plan_digest_for(BASE, V2).unwrap());
    }
}

#[test]
fn conversion_advances_every_generation_preserves_history_and_invalidates_old_scope() {
    let mut db = fixture();
    db.execute("INSERT INTO works VALUES('work-b',?)", [RUNTIME])
        .unwrap();
    db.execute(
        "UPDATE agent_identity_plan_heads SET generation=8 WHERE work_id='work-b'",
        [],
    )
    .unwrap();
    let before = heads(&db);
    let old = read_identity_plan_stamp(&db, "work-a", V1).unwrap();
    let scope = Scope {
        work_id: old.work_id.clone(),
        plan_version: old.generation,
        plan_digest: old.digest.clone(),
    };
    assert!(old.matches_scope(&scope));
    assert!(read_identity_plan_stamp(&db, "work-a", V2).is_err());
    let mut tx = db.transaction().unwrap();
    tx.execute("INSERT INTO caller_notes VALUES('kept')", [])
        .unwrap();
    let converted = upgrade_identity_plan_projection(&mut tx).unwrap();
    assert_eq!(converted.len(), 2);
    assert_eq!((converted[0].generation, converted[1].generation), (2, 9));
    let history: Vec<(String, i64, String, i64, i64, String, i64)> = tx
        .prepare("SELECT * FROM agent_identity_plan_projection_history ORDER BY work_id")
        .unwrap()
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    for (i, h) in history.iter().enumerate() {
        assert_eq!(
            (&h.0, h.1, &h.2, h.3),
            (&before[i].0, before[i].1, &before[i].2, 1)
        );
        assert_eq!(
            (h.4, &h.5, h.6),
            (converted[i].generation, &converted[i].digest, 2)
        );
    }
    assert!(!converted[0].matches_scope(&scope));
    assert!(read_identity_plan_stamp(&tx, "work-a", V1).is_err());
    assert!(tx
        .execute(
            "UPDATE agent_identity_plan_projection_history SET from_digest='tamper'",
            []
        )
        .is_err());
    assert!(tx
        .execute("DELETE FROM agent_identity_plan_projection_history", [])
        .is_err());
    tx.commit().unwrap();
    assert_eq!(
        read_identity_plan_stamp(&db, "work-a", V2).unwrap(),
        converted[0]
    );
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        37
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM caller_notes", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn v2_sql_updates_keep_runtime_outcomes_and_bind_definition_and_atomic_failures() {
    let mut db = fixture();
    upgrade(&mut db);
    let first = read_identity_plan_stamp(&db, "work-a", V2).unwrap();
    for raw in [RUNTIME, BASE] {
        db.execute("UPDATE works SET body=? WHERE id='work-a'", [raw])
            .unwrap();
        assert_eq!(first, read_identity_plan_stamp(&db, "work-a", V2).unwrap());
    }
    db.execute("UPDATE works SET body=? WHERE id='work-a'", [CHANGED])
        .unwrap();
    let second = read_identity_plan_stamp(&db, "work-a", V2).unwrap();
    assert_eq!(second.generation, first.generation + 1);
    assert_ne!(second.digest, first.digest);
    for raw in [b"".as_slice(), b"null", b"[]", b"{", br#"{"planning":{}}"#] {
        assert_eq!(second, read_identity_plan_stamp(&db, "work-a", V2).unwrap());
        assert!(db
            .execute("UPDATE works SET body=? WHERE id='work-a'", [raw])
            .is_err());
        assert_eq!(second, read_identity_plan_stamp(&db, "work-a", V2).unwrap());
        assert_eq!(
            db.query_row("SELECT body FROM works", [], |r| r.get::<_, Vec<u8>>(0))
                .unwrap(),
            CHANGED
        );
    }
    assert!(db.execute("UPDATE works SET id='renamed'", []).is_err());
    db.execute("INSERT INTO works VALUES('work-new',?)", [BASE])
        .unwrap();
    assert_eq!(
        read_identity_plan_stamp(&db, "work-new", V2)
            .unwrap()
            .generation,
        1
    );
    db.execute(
        "UPDATE agent_identity_plan_heads SET digest=? WHERE work_id='work-a'",
        ["0".repeat(64)],
    )
    .unwrap();
    assert!(read_identity_plan_stamp(&db, "work-a", V2).is_err());
    assert!(db
        .execute("UPDATE works SET body=? WHERE id='work-a'", [BASE])
        .is_err());
}

#[test]
fn conversion_and_outer_transaction_roll_back_and_can_be_retried() {
    let mut db = fixture();
    let before = (schema(&db), heads(&db));
    {
        let mut tx = db.transaction().unwrap();
        upgrade_identity_plan_projection(&mut tx).unwrap();
        assert!(read_identity_plan_stamp(&tx, "work-a", V2).is_ok());
        tx.rollback().unwrap();
    }
    assert_eq!((schema(&db), heads(&db)), before);
    assert!(read_identity_plan_stamp(&db, "work-a", V1).is_ok());
    upgrade(&mut db);
    let after = (schema(&db), heads(&db));
    let mut tx = db.transaction().unwrap();
    assert_eq!(
        upgrade_identity_plan_projection(&mut tx)
            .unwrap_err()
            .code(),
        "identity_plan_projection_schema"
    );
    tx.commit().unwrap();
    assert_eq!((schema(&db), heads(&db)), after);
}

#[test]
fn partial_ddl_failure_rolls_back_savepoint_preserving_caller_work_then_recovers() {
    let mut db = fixture();
    // Successful control under the same schema and data, then roll back.
    let mut tx = db.transaction().unwrap();
    upgrade_identity_plan_projection(&mut tx).unwrap();
    tx.rollback().unwrap();
    db.execute_batch("CREATE TABLE agent_identity_plan_heads_v2(collision TEXT);")
        .unwrap();
    let before = (schema(&db), heads(&db));
    let mut tx = db.transaction().unwrap();
    tx.execute("INSERT INTO caller_notes VALUES('preserved')", [])
        .unwrap();
    let error = upgrade_identity_plan_projection(&mut tx).unwrap_err();
    assert_eq!(error.code(), "identity_plan_storage");
    assert!(std::error::Error::source(&error).is_some());
    assert!(!format!("{error:?}").contains("collision"));
    assert_eq!((schema(&tx), heads(&tx)), before);
    assert_eq!(
        tx.query_row("SELECT count(*) FROM caller_notes", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    tx.commit().unwrap();
    db.execute_batch("DROP TABLE agent_identity_plan_heads_v2;")
        .unwrap();
    upgrade(&mut db);
    assert!(read_identity_plan_stamp(&db, "work-a", V2).is_ok());
}

#[test]
fn missing_or_tampered_heads_and_generation_overflow_are_refused_before_writes() {
    for fault in ["digest", "missing", "overflow"] {
        let mut db = fixture();
        let mut control = db.transaction().unwrap();
        upgrade_identity_plan_projection(&mut control).unwrap();
        control.rollback().unwrap();
        let original = heads(&db)[0].clone();
        match fault {
            "digest" => {
                db.execute(
                    "UPDATE agent_identity_plan_heads SET digest=?",
                    ["f".repeat(64)],
                )
                .unwrap();
            }
            "missing" => {
                db.execute("DELETE FROM agent_identity_plan_heads", [])
                    .unwrap();
            }
            _ => {
                db.execute(
                    "UPDATE agent_identity_plan_heads SET generation=?",
                    [i64::MAX],
                )
                .unwrap();
            }
        }
        let before = (schema(&db), heads(&db));
        let mut tx = db.transaction().unwrap();
        let error = upgrade_identity_plan_projection(&mut tx).unwrap_err();
        if fault == "overflow" {
            assert_eq!(error.code(), "identity_plan_generation_overflow");
        }
        assert_eq!((schema(&tx), heads(&tx)), before);
        tx.commit().unwrap();
        db.execute("DELETE FROM agent_identity_plan_heads", [])
            .unwrap();
        db.execute(
            "INSERT INTO agent_identity_plan_heads VALUES(?,?,?,?)",
            params![original.0, original.1, original.2, original.3],
        )
        .unwrap();
        upgrade(&mut db);
        assert!(read_identity_plan_stamp(&db, "work-a", V2).is_ok());
    }
}

#[test]
fn unsupported_schema_dependencies_and_disabled_foreign_keys_are_refused() {
    for fault in ["version", "foreign_keys", "trigger", "dependency"] {
        let mut db = fixture();
        let mut control = db.transaction().unwrap();
        upgrade_identity_plan_projection(&mut control).unwrap();
        control.rollback().unwrap();
        match fault {
            "version" => db.execute_batch("PRAGMA user_version=38;").unwrap(),
            "foreign_keys" => db.execute_batch("PRAGMA foreign_keys=OFF;").unwrap(),
            "trigger" => db.execute_batch("CREATE TRIGGER other_head_contract AFTER DELETE ON agent_identity_plan_heads BEGIN SELECT 1; END;").unwrap(),
            _ => db.execute_batch("CREATE TABLE other_contract(head TEXT REFERENCES agent_identity_plan_heads(work_id) ON DELETE CASCADE);").unwrap(),
        }
        let before = (schema(&db), heads(&db));
        let mut tx = db.transaction().unwrap();
        assert_eq!(
            upgrade_identity_plan_projection(&mut tx)
                .unwrap_err()
                .code(),
            "identity_plan_projection_schema"
        );
        assert_eq!((schema(&tx), heads(&tx)), before);
        tx.commit().unwrap();
    }
}

#[test]
fn v2_preserves_number_lexemes_depth_and_refuses_overflow_without_mutation() {
    for (a, b) in [
        ("1", "1.0"),
        ("0", "-0"),
        ("9007199254740992", "9007199254740993"),
    ] {
        assert_ne!(
            identity_plan_digest_for(format!("{{\"custom\":{a}}}").as_bytes(), V2).unwrap(),
            identity_plan_digest_for(format!("{{\"custom\":{b}}}").as_bytes(), V2).unwrap()
        );
    }
    let deep = format!("{{\"custom\":{}0{}}}", "[".repeat(9998), "]".repeat(9998));
    assert!(identity_plan_digest_for(deep.as_bytes(), V2).is_ok());
    let too_deep = deep.replacen("0", "[0]", 1);
    assert!(identity_plan_digest_for(too_deep.as_bytes(), V2).is_err());
    let omitted = too_deep.replacen("custom", "git", 1);
    assert!(identity_plan_digest_for(omitted.as_bytes(), V2).is_ok());
    let mut db = fixture();
    upgrade(&mut db);
    db.execute(
        "UPDATE agent_identity_plan_heads SET generation=?",
        [i64::MAX],
    )
    .unwrap();
    let before = read_identity_plan_stamp(&db, "work-a", V2).unwrap();
    db.execute("UPDATE works SET body=?", [RUNTIME]).unwrap();
    assert_eq!(before, read_identity_plan_stamp(&db, "work-a", V2).unwrap());
    assert!(db.execute("UPDATE works SET body=?", [CHANGED]).is_err());
    assert_eq!(before, read_identity_plan_stamp(&db, "work-a", V2).unwrap());
    assert_eq!(
        db.query_row("SELECT body FROM works", [], |r| r.get::<_, Vec<u8>>(0))
            .unwrap(),
        RUNTIME
    );
}

#[test]
fn temporary_tables_must_not_supply_a_persisted_identity_stamp() {
    let mut db = fixture();
    upgrade(&mut db);
    let actual = read_identity_plan_stamp(&db, "work-a", V2).unwrap();
    db.execute_batch("CREATE TEMP TABLE works(id TEXT,body BLOB);CREATE TEMP TABLE agent_identity_plan_heads(work_id TEXT,generation INTEGER,digest TEXT,projection_version INTEGER);").unwrap();
    db.execute("INSERT INTO temp.works VALUES('work-a',?)", [CHANGED])
        .unwrap();
    db.execute(
        "INSERT INTO temp.agent_identity_plan_heads VALUES('work-a',99,?,2)",
        [identity_plan_digest_for(CHANGED, V2).unwrap()],
    )
    .unwrap();
    assert_eq!(read_identity_plan_stamp(&db, "work-a", V2).unwrap(), actual);
}

#[test]
fn component_conversion_refuses_temporary_shadows_then_recovers() {
    let mut db = fixture();
    let mut control = db.transaction().unwrap();
    upgrade_identity_plan_projection(&mut control).unwrap();
    control.rollback().unwrap();
    // Matching values must not hide the unsupported temporary-table contract.
    db.execute_batch("CREATE TEMP TABLE works AS SELECT * FROM main.works;")
        .unwrap();
    let before = (schema(&db), heads(&db));
    let mut tx = db.transaction().unwrap();
    assert_eq!(
        upgrade_identity_plan_projection(&mut tx)
            .unwrap_err()
            .code(),
        "identity_plan_projection_schema"
    );
    assert_eq!((schema(&tx), heads(&tx)), before);
    tx.commit().unwrap();
    db.execute_batch("DROP TABLE temp.works;").unwrap();
    upgrade(&mut db);
}

#[test]
fn stale_transaction_handle_cannot_implicitly_commit_projection_conversion() {
    let mut db = fixture();
    {
        let mut tx = db.transaction().unwrap();
        upgrade_identity_plan_projection(&mut tx).unwrap();
        tx.rollback().unwrap();
    }
    let mut tx = db.transaction().unwrap();
    tx.execute_batch("ROLLBACK").unwrap();
    assert!(tx.is_autocommit());
    let result = upgrade_identity_plan_projection(&mut tx);
    let accepted = result.is_ok();
    if let Err(e) = result {
        assert_eq!(e.code(), "identity_plan_transaction_inactive");
    }
    let version: i64 = tx
        .query_row(
            "SELECT projection_version FROM agent_identity_plan_heads WHERE work_id='work-a'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    drop(tx);
    assert!(
        !accepted && version == 1,
        "inactive conversion accepted={accepted},durable_projection={version}"
    );
    let mut tx = db.transaction().unwrap();
    upgrade_identity_plan_projection(&mut tx).unwrap();
    tx.commit().unwrap();
    assert!(read_identity_plan_stamp(&db, "work-a", V2).is_ok());
}
