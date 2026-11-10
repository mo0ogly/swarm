use rusqlite::{params, Connection};
use swarm_engine::{
    identity_plan_digest, register_identity_plan_projection, IDENTITY_PLAN_HEADS_SQL,
};
const BASE:&[u8]=br#"{"id":"work-a","revision":1,"title":"plan","tasks":[{"id":"task-a","owner":"agent-a","status":"todo"}]}"#;
fn fixture(raw: &[u8]) -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.busy_timeout(std::time::Duration::ZERO).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,body BLOB NOT NULL);PRAGMA user_version=36;").unwrap();
    db.execute("INSERT INTO works VALUES('work-a',1,?)", [raw])
        .unwrap();
    register_identity_plan_projection(&db).unwrap();
    db
}
fn migrate(db: &mut Connection) -> rusqlite::Result<()> {
    let tx = db.transaction()?;
    tx.execute_batch(IDENTITY_PLAN_HEADS_SQL)?;
    tx.commit()
}
fn stamp(db: &Connection) -> (i64, String) {
    db.query_row(
        "SELECT generation,digest FROM agent_identity_plan_heads WHERE work_id='work-a'",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .unwrap()
}
fn schema(db: &Connection) -> Vec<(String, String, Option<String>)> {
    db.prepare("SELECT type,name,sql FROM sqlite_master ORDER BY name")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

#[test]
fn plan_projection_keeps_unknown_fields_exact_numbers_and_excludes_only_runtime_data() {
    let digest = identity_plan_digest(BASE).unwrap();
    assert_eq!(digest.len(), 64);
    assert_eq!(digest,identity_plan_digest(br#"{"tasks":[{"status":"done","owner":"agent-a","id":"task-a","attempts":[{"anything":"runtime"}]}],"title":"plan","id":"work-a","revision":7,"summary":"runtime","git":{},"project_memory":[]}"#).unwrap());
    assert_ne!(digest,identity_plan_digest(br#"{"id":"work-a","revision":1,"title":"plan","tasks":[{"id":"task-a","owner":"agent-b","status":"todo"}]}"#).unwrap());
    for (left, right) in [
        (
            br#"{"unknown":9007199254740992}"#.as_slice(),
            br#"{"unknown":9007199254740993}"#.as_slice(),
        ),
        (br#"{"unknown":1}"#, br#"{"unknown":1.0}"#),
        (br#"{"unknown":0}"#, br#"{"unknown":-0}"#),
    ] {
        assert_ne!(
            identity_plan_digest(left).unwrap(),
            identity_plan_digest(right).unwrap()
        );
    }
    assert_eq!(
        identity_plan_digest(br#"{"custom":{"x":1,"x":2}}"#).unwrap(),
        identity_plan_digest(br#"{"custom":{"x":2}}"#).unwrap()
    );
    assert_eq!(
        identity_plan_digest(br#"{"custom":"\ud800"}"#).unwrap(),
        identity_plan_digest(br#"{"custom":"\ufffd"}"#).unwrap()
    );
}

#[test]
fn identity_sql_function_and_triggers_bind_generations_and_refuse_tampering_atomically() {
    let mut db = fixture(BASE);
    migrate(&mut db).unwrap();
    let first = stamp(&db);
    assert_eq!(first, (1, identity_plan_digest(BASE).unwrap()));
    let metadata=br#"{"id":"work-a","revision":2,"title":"plan","summary":"new result","tasks":[{"id":"task-a","owner":"agent-a","status":"done"}]}"#;
    db.execute(
        "UPDATE works SET revision=2,body=? WHERE id='work-a'",
        [metadata],
    )
    .unwrap();
    assert_eq!(stamp(&db), first);
    let changed=br#"{"id":"work-a","revision":3,"title":"new definition","tasks":[{"id":"task-a","owner":"agent-a","status":"done"}]}"#;
    db.execute(
        "UPDATE works SET revision=3,body=? WHERE id='work-a'",
        [changed],
    )
    .unwrap();
    let second = stamp(&db);
    assert_eq!(second, (2, identity_plan_digest(changed).unwrap()));
    let transaction = db.transaction().unwrap();
    transaction
        .execute("UPDATE works SET body=? WHERE id='work-a'", [BASE])
        .unwrap();
    assert_eq!(stamp(&transaction).0, 3);
    transaction.rollback().unwrap();
    assert_eq!(stamp(&db), second);
    // Valid same-ID control before corrupting the durable head.
    db.execute("UPDATE works SET id=id WHERE id='work-a'", [])
        .unwrap();
    assert_eq!(stamp(&db), second);
    db.execute("UPDATE agent_identity_plan_heads SET digest='tampered'", [])
        .unwrap();
    assert!(db
        .execute("UPDATE works SET body=? WHERE id='work-a'", [BASE])
        .is_err());
    let retained: Vec<u8> = db
        .query_row("SELECT body FROM works WHERE id='work-a'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(retained, changed);
    db.execute("UPDATE agent_identity_plan_heads SET digest=?", [&second.1])
        .unwrap();
    db.execute("UPDATE works SET body=? WHERE id='work-a'", [changed])
        .unwrap();
    assert_eq!(stamp(&db), second);
    assert!(db
        .execute("UPDATE works SET id='other' WHERE id='work-a'", [])
        .is_err());
    assert_eq!(stamp(&db), second);
    // Function failure in the AFTER trigger rolls back the whole statement.
    db.execute("UPDATE works SET body=? WHERE id='work-a'", [changed])
        .unwrap();
    assert!(db
        .execute(
            "UPDATE works SET body=? WHERE id='work-a'",
            [b"null".as_slice()]
        )
        .is_err());
    assert_eq!(stamp(&db), second);
    let retained: Vec<u8> = db
        .query_row("SELECT body FROM works WHERE id='work-a'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(retained, changed);
    db.execute("UPDATE works SET body=? WHERE id='work-a'", [changed])
        .unwrap();
    db.execute(
        "UPDATE agent_identity_plan_heads SET generation=9223372036854775807",
        [],
    )
    .unwrap();
    assert!(db
        .execute("UPDATE works SET body=? WHERE id='work-a'", [BASE])
        .is_err());
    assert_eq!(stamp(&db).0, i64::MAX);
    let retained: Vec<u8> = db
        .query_row("SELECT body FROM works WHERE id='work-a'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(retained, changed);
}

#[test]
fn identity_schema_fragment_failure_rolls_back_ddl_and_version_without_partial_migration() {
    let mut control = fixture(BASE);
    migrate(&mut control).unwrap();
    assert_eq!(
        control
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        37
    );
    for raw in [b"null".as_slice(), b"[]", b"{bad", b""] {
        let mut control = fixture(BASE);
        migrate(&mut control).unwrap();
        assert_eq!(stamp(&control).0, 1);
        let mut db = fixture(raw);
        let before = schema(&db);
        assert!(migrate(&mut db).is_err());
        let after = schema(&db);
        assert_eq!(before, after);
        assert_eq!(
            db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            36
        );
        let retained: Vec<u8> = db
            .query_row("SELECT body FROM works", [], |r| r.get(0))
            .unwrap();
        assert_eq!(retained, raw);
        db.execute("UPDATE works SET body=?", [BASE]).unwrap();
        migrate(&mut db).unwrap();
        assert_eq!(stamp(&db).0, 1);
    }
}

#[test]
fn plan_projection_checks_go_depth_after_adding_the_versioned_wrapper() {
    let valid = format!("{{\"unknown\":{}0{}}}", "[".repeat(9998), "]".repeat(9998));
    assert!(identity_plan_digest(valid.as_bytes()).is_ok());
    let too_deep = format!("{{\"unknown\":{}0{}}}", "[".repeat(9999), "]".repeat(9999));
    assert!(identity_plan_digest(too_deep.as_bytes()).is_err());
    let omitted = too_deep.replace("unknown", "git");
    assert!(identity_plan_digest(omitted.as_bytes()).is_ok());
}

#[test]
fn invalid_plans_and_sql_parameter_types_return_no_digest_or_private_diagnostic() {
    let db = fixture(BASE);
    let control: String = db
        .query_row("SELECT swarm_identity_plan_digest_v1(?)", [BASE], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(control, identity_plan_digest(BASE).unwrap());
    for raw in [
        b"null".as_slice(),
        b"[]",
        b"1",
        b"",
        br#"{"tasks":[1]}"#,
        br#"{"launch_profile":1}"#,
        br#"{"planning":{}}"#,
        br#"{"planning":{"scopes":[1]}}"#,
        br#""private-plan""#,
    ] {
        let error = identity_plan_digest(raw).unwrap_err();
        assert!(!format!("{error} {error:?}").contains("private-plan"));
        assert!(db
            .query_row("SELECT swarm_identity_plan_digest_v1(?)", [raw], |r| r
                .get::<_, String>(
                0
            ))
            .is_err());
    }
    assert!(db
        .query_row(
            "SELECT swarm_identity_plan_digest_v1(?)",
            params![rusqlite::types::Null],
            |r| r.get::<_, String>(0)
        )
        .is_err());
    assert!(db
        .query_row("SELECT swarm_identity_plan_digest_v1(?)", [1_i64], |r| {
            r.get::<_, String>(0)
        })
        .is_err());
}
