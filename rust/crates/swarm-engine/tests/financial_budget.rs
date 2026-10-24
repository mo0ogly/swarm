use rusqlite::Connection;
use swarm_engine::{
    check_launch_budget, decode_budget_change, encode_model, reserve_planning_call,
    validate_budget, Budget, GoFloat,
};
/// Four actual ledger column shapes in an isolated SQL fixture; no live mission.
fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE budgets(work_id TEXT PRIMARY KEY,body BLOB NOT NULL);CREATE TABLE reservations(agent_id TEXT PRIMARY KEY,work_id TEXT,amount REAL,state TEXT);CREATE TABLE assist_reservations(turn_id TEXT PRIMARY KEY,work_id TEXT,amount REAL,state TEXT);CREATE TABLE preparation_reservations(turn_id TEXT PRIMARY KEY,work_id TEXT,work_amount REAL,state TEXT);CREATE TABLE planning_calls(id TEXT PRIMARY KEY,work_id TEXT,scope_id TEXT,estimate REAL,usage BLOB,state TEXT,created_at TEXT);").unwrap();
    db
}
/// Persist validated fixture policy with Go-compatible floats and date metadata.
fn policy(db: &Connection, limit: f64) {
    let budget = Budget {
        limit_usd: GoFloat(limit),
        reserve_per_launch_usd: GoFloat(1.0),
        estimate_source: "fixture estimate".into(),
        reference_date: "2026-10-19".into(),
        ..Default::default()
    };
    validate_budget(&budget).unwrap();
    db.execute(
        "INSERT OR REPLACE INTO budgets VALUES('work',?)",
        [encode_model(&budget).unwrap()],
    )
    .unwrap();
}
/// Equality fits the remaining estimate; both sums and failed reservation are observable.
#[test]
fn financial_budget_combines_four_ledgers_and_reserves_without_reset() {
    let mut db = fixture();
    policy(&db, 5.0);
    db.execute_batch("INSERT INTO reservations VALUES('agent','work',1,'reserved');INSERT INTO assist_reservations VALUES('turn','work',1,'estimated');INSERT INTO preparation_reservations VALUES('prep','work',1,'reserved');INSERT INTO planning_calls VALUES('prior','work','root',1,NULL,'finished','at');").unwrap();
    let tx = db.transaction().unwrap();
    check_launch_budget(&tx, "work").unwrap();
    reserve_planning_call(&tx, "work", "root", "call", "2026-10-19T00:00:00Z").unwrap();
    let count: i64 = tx
        .query_row("SELECT count(*) FROM planning_calls", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
    assert_eq!(
        check_launch_budget(&tx, "work").unwrap_err().code(),
        "budget_exhausted"
    );
    assert!(reserve_planning_call(&tx, "work", "root", "denied", "2026-10-19T00:00:00Z").is_err());
    assert_eq!(
        tx.query_row("SELECT count(*) FROM planning_calls", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    tx.rollback().unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM planning_calls", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
/// Absent/zero policy is the existing unlimited contract; it never invents actual cost.
#[test]
fn financial_budget_unlimited_has_zero_planning_estimate_and_checked_duplicates() {
    let mut db = fixture();
    let tx = db.transaction().unwrap();
    check_launch_budget(&tx, "work").unwrap();
    reserve_planning_call(&tx, "work", "root", "call", "2026-10-19T00:00:00Z").unwrap();
    assert_eq!(
        tx.query_row(
            "SELECT estimate FROM planning_calls WHERE id='call'",
            [],
            |r| r.get::<_, f64>(0)
        )
        .unwrap(),
        0.0
    );
    assert!(reserve_planning_call(&tx, "work", "root", "call", "2026-10-19T00:00:00Z").is_err());
    tx.rollback().unwrap();
    policy(&db, 0.0);
    let tx = db.transaction().unwrap();
    check_launch_budget(&tx, "work").unwrap();
}
/// Invalid persisted policy or active ledger values refuse after a valid witness,
/// preserve original data and recover after restoring the same private fixture.
#[test]
fn financial_budget_refuses_invalid_policy_and_ledger_then_recovers() {
    let mut db = fixture();
    policy(&db, 5.0);
    {
        let tx = db.transaction().unwrap();
        check_launch_budget(&tx, "work").unwrap();
        tx.rollback().unwrap();
    }
    db.execute(
        "INSERT INTO reservations VALUES('bad','work',-1,'reserved')",
        [],
    )
    .unwrap();
    {
        let tx = db.transaction().unwrap();
        assert!(check_launch_budget(&tx, "work").is_err());
        tx.rollback().unwrap();
    }
    db.execute("DELETE FROM reservations WHERE agent_id='bad'", [])
        .unwrap();
    {
        let tx = db.transaction().unwrap();
        check_launch_budget(&tx, "work").unwrap();
        tx.rollback().unwrap();
    }
    db.execute(
        "UPDATE budgets SET body=?",
        [br#"{"limit_usd":0,"reserve_per_launch_usd":-1}"#.as_slice()],
    )
    .unwrap();
    {
        let tx = db.transaction().unwrap();
        assert!(check_launch_budget(&tx, "work").is_err());
        tx.rollback().unwrap();
    }
    policy(&db, 5.0);
    let tx = db.transaction().unwrap();
    check_launch_budget(&tx, "work").unwrap();
}
/// Go-shaped strict DTO uses finite monetary values, real dates and whitespace semantics.
#[test]
fn financial_budget_codec_and_validation_preserve_canonical_contract() {
    let raw=br#"{"schema_version":1,"event_id":"budget","expected_revision":2,"budget":{"limit_usd":5,"reserve_per_launch_usd":1,"estimate_source":"Estimate <&>","reference_date":"2024-02-29"}}"#;
    let valid = decode_budget_change(raw).unwrap();
    validate_budget(&valid.budget).unwrap();
    let encoded = encode_model(&valid).unwrap();
    assert!(String::from_utf8_lossy(&encoded).contains("\"limit_usd\":5,"));
    for key in 0..5 {
        let mut b = valid.budget.clone();
        match key {
            0 => b.limit_usd = GoFloat(-1.0),
            1 => b.reserve_per_launch_usd = GoFloat(f64::NAN),
            2 => b.reference_date = "2023-02-29".into(),
            3 => b.estimate_source = "\u{00a0}".into(),
            _ => b.reserve_per_launch_usd = GoFloat(0.0),
        };
        assert!(validate_budget(&b).is_err());
    }
    assert!(decode_budget_change(br#"{"budget":{"unknown":1}}"#).is_err());
    assert!(decode_budget_change(br#"{"budget":{"limit_usd":1e999}}"#).is_err());
    use std::error::Error;
    let error = decode_budget_change(br#"{"budget":{"limit_usd":"PRIVATE"}}"#).unwrap_err();
    assert!(error.source().is_some());
    assert!(!format!("{error:?}").contains("PRIVATE"));
    validate_budget(&Budget::default()).unwrap();
}
/// Finite deterministic engine observations; they are fixture values, not identity proofs.
#[derive(Default)]
struct Environment {
    deny_operator: bool,
}
impl swarm_engine::WorkEnvironment for Environment {
    fn identifier(&mut self, prefix: &str) -> Result<String, swarm_engine::RuntimeError> {
        Ok(format!("{prefix}fixture"))
    }
    fn timestamp(&mut self) -> Result<String, swarm_engine::RuntimeError> {
        Ok("2026-10-19T00:00:00Z".into())
    }
    fn operator(&mut self) -> Result<String, swarm_engine::RuntimeError> {
        if self.deny_operator {
            return Err(swarm_engine::RuntimeError::new(
                swarm_engine::RuntimeOperation::Platform,
                std::io::Error::other("PRIVATE operator unavailable"),
            ));
        }
        Ok("operator/fixture".into())
    }
    fn git(
        &mut self,
        _: &std::path::Path,
    ) -> Result<swarm_engine::GitObservation, swarm_engine::RuntimeError> {
        Ok(swarm_engine::GitObservation {
            state: Default::default(),
            issues: vec![],
        })
    }
}
/// Own all minimal SQL consumers of the real Work transaction; no source copies,
/// provider or live root. Missing schema must fail rather than be bypassed.
fn engine_fixture() -> Connection {
    let mut db = fixture();
    db.execute_batch("PRAGMA user_version=37;CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,body BLOB NOT NULL);CREATE TABLE events(id TEXT PRIMARY KEY,work_id TEXT NOT NULL,revision INTEGER NOT NULL,kind TEXT NOT NULL,at TEXT NOT NULL,payload BLOB NOT NULL,request BLOB NOT NULL,UNIQUE(work_id,revision));CREATE TABLE agents(work_id TEXT,task_id TEXT,status TEXT);CREATE TABLE cockpit_controls(work_id TEXT,paused);CREATE TABLE preparation_launch_locks(work_id TEXT,task_id TEXT,released INTEGER);CREATE TABLE cockpit_events(work_id TEXT,at TEXT,kind TEXT,message TEXT);").unwrap();
    engine(&mut db,&mut Environment::default(),"work.create",serde_json::json!({"schema_version":1,"event_id":"create","expected_revision":0,"title":"Budget fixture","objective":"Proof","scope":"Fixture","criteria":["Proof"]})).unwrap();
    db
}
/// Submit external JSON through WorkEngine's actual codec/guard/stage/commit path.
fn engine(
    db: &mut Connection,
    e: &mut Environment,
    kind: &str,
    value: serde_json::Value,
) -> Result<swarm_engine::WorkMutationOutcome, swarm_engine::WorkEngineError> {
    let limits = swarm_engine::EngineMutationLimits {
        task: swarm_engine::TaskDefinitionLimits::legacy_go(),
        policy: swarm_engine::ValidationPolicyLimits::legacy_go(),
    };
    swarm_engine::WorkEngine::new(db, std::path::Path::new("."), e, &limits).execute_external(
        if kind == "work.create" {
            ""
        } else {
            "w-fixture"
        },
        kind,
        &serde_json::to_vec(&value).unwrap(),
    )
}
/// Build a valid external policy with deliberately untrusted actor/time metadata.
fn change(event: &str, revision: i64, limit: f64) -> serde_json::Value {
    serde_json::json!({"schema_version":1,"event_id":event,"expected_revision":revision,"budget":{"limit_usd":limit,"reserve_per_launch_usd":1,"estimate_source":"Fixture estimate","reference_date":"2026-10-19","actor":"FORGED","updated":"FORGED"}})
}
/// Policy and event commit with Work revision; replay clears declared actor/time,
/// never resets reservations or overwrites a later policy with an earlier event.
#[test]
fn financial_budget_configuration_is_atomic_engine_owned_and_replay_safe() {
    let mut db = engine_fixture();
    db.execute(
        "INSERT INTO reservations VALUES('agent','w-fixture',0.75,'reserved')",
        [],
    )
    .unwrap();
    let request = change("budget-a", 1, 5.0);
    let out = engine(
        &mut db,
        &mut Environment::default(),
        "budget.configure",
        request.clone(),
    )
    .unwrap();
    assert_eq!(out.work.revision, 2);
    let saved: Vec<u8> = db
        .query_row(
            "SELECT body FROM budgets WHERE work_id='w-fixture'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(value["actor"], "operator/fixture");
    assert_eq!(value["updated"], "2026-10-19T00:00:00Z");
    let mut replay = request.clone();
    replay["budget"]["actor"] = "OTHER".into();
    replay["budget"]["updated"] = "OTHER".into();
    assert!(
        engine(
            &mut db,
            &mut Environment {
                deny_operator: true
            },
            "budget.configure",
            replay
        )
        .unwrap()
        .replayed
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM cockpit_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    engine(
        &mut db,
        &mut Environment::default(),
        "budget.configure",
        change("budget-b", 2, 4.0),
    )
    .unwrap();
    assert!(
        engine(
            &mut db,
            &mut Environment {
                deny_operator: true
            },
            "budget.configure",
            request
        )
        .unwrap()
        .replayed
    );
    let saved: Vec<u8> = db
        .query_row(
            "SELECT body FROM budgets WHERE work_id='w-fixture'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&saved).unwrap()["limit_usd"],
        4
    );
    assert_eq!(
        db.query_row(
            "SELECT amount FROM reservations WHERE agent_id='agent'",
            [],
            |r| r.get::<_, f64>(0)
        )
        .unwrap(),
        0.75
    );
    assert_eq!(
        db.query_row("SELECT revision FROM works WHERE id='w-fixture'", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        3
    );
}
/// Valid witness precedes each silent SQL refusal. Work/policy/event stay intact,
/// then exactly the same intent succeeds after fixture trigger removal.
#[test]
fn financial_budget_ignored_sql_writes_refuse_rollback_and_recover() {
    let mut db = engine_fixture();
    engine(
        &mut db,
        &mut Environment::default(),
        "budget.configure",
        change("witness", 1, 5.0),
    )
    .unwrap();
    for (i, table) in ["budgets", "cockpit_events"].into_iter().enumerate() {
        let revision = 2 + i64::try_from(i).unwrap();
        let budget: Vec<u8> = db
            .query_row("SELECT body FROM budgets", [], |r| r.get(0))
            .unwrap();
        let events: i64 = db
            .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
            .unwrap();
        let cockpit: i64 = db
            .query_row("SELECT count(*) FROM cockpit_events", [], |r| r.get(0))
            .unwrap();
        db.execute_batch(&format!(
            "CREATE TRIGGER deny BEFORE INSERT ON {table} BEGIN SELECT RAISE(IGNORE); END;"
        ))
        .unwrap();
        let request = change(&format!("retry-{i}"), revision, 3.0);
        assert!(engine(
            &mut db,
            &mut Environment::default(),
            "budget.configure",
            request.clone()
        )
        .is_err());
        assert_eq!(
            db.query_row("SELECT revision FROM works", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            revision
        );
        assert_eq!(
            db.query_row("SELECT body FROM budgets", [], |r| r.get::<_, Vec<u8>>(0))
                .unwrap(),
            budget
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            events
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM cockpit_events", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            cockpit
        );
        db.execute_batch("DROP TRIGGER deny").unwrap();
        engine(
            &mut db,
            &mut Environment::default(),
            "budget.configure",
            request,
        )
        .unwrap();
    }
}
/// Two real connections serialize admission: writer contention refuses without a
/// second reservation, then fresh transaction observes exhaustion after commit.
/// SQLite busy0 is a fixture lock-observation mode, not a product timeout/watchdog.
#[test]
fn financial_budget_two_connections_cannot_reserve_the_same_remaining_estimate() {
    let dir = LedgerFixture::new();
    let path = dir.0.join("ledger.db");
    let source = fixture();
    let schema: String = source
        .query_row(
            "SELECT group_concat(sql,';') FROM sqlite_master WHERE type='table'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut first = Connection::open(&path).unwrap();
    first.execute_batch(&schema).unwrap();
    policy(&first, 1.0);
    let mut second = Connection::open(&path).unwrap();
    first.busy_timeout(std::time::Duration::ZERO).unwrap();
    second.busy_timeout(std::time::Duration::ZERO).unwrap();
    {
        let witness = second.transaction().unwrap();
        check_launch_budget(&witness, "work").unwrap();
        witness.rollback().unwrap();
    }
    let tx = first
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    {
        let refused = second.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate);
        assert!(
            matches!(refused,Err(rusqlite::Error::SqliteFailure(ref e,_))if e.code==rusqlite::ErrorCode::DatabaseBusy)
        );
    }
    reserve_planning_call(&tx, "work", "root", "first", "2026-10-19T00:00:00Z").unwrap();
    tx.commit().unwrap();
    let retry = second
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    assert_eq!(
        check_launch_budget(&retry, "work").unwrap_err().code(),
        "budget_exhausted"
    );
    retry.rollback().unwrap();
    assert_eq!(
        second
            .query_row("SELECT count(*) FROM planning_calls", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
/// Engine observation failure leaves policy/Work/events untouched; same intent
/// recovers after the actual prerequisite changes, without resetting history.
#[test]
fn financial_budget_operator_failure_preserves_cause_and_recovers_same_intent() {
    use std::error::Error;
    let mut db = engine_fixture();
    let request = change("operator", 1, 5.0);
    let error = engine(
        &mut db,
        &mut Environment {
            deny_operator: true,
        },
        "budget.configure",
        request.clone(),
    )
    .unwrap_err();
    assert!(error.source().is_some());
    assert!(!format!("{error:?}").contains("PRIVATE"));
    assert_eq!(
        db.query_row("SELECT revision FROM works", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM budgets", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM cockpit_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    engine(
        &mut db,
        &mut Environment::default(),
        "budget.configure",
        request,
    )
    .unwrap();
    assert_eq!(
        db.query_row("SELECT revision FROM works", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

/// Private disposable ledger directory; retain original failed fixtures on panic.
struct LedgerFixture(std::path::PathBuf);
impl LedgerFixture {
    /// Use the existing secure identifier helper, with no extra dev dependency.
    fn new() -> Self {
        let path = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("swarm-financial-").unwrap());
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path).unwrap();
        Self(path)
    }
}
impl Drop for LedgerFixture {
    /// Remove only this owned successful fixture; preserve data if a check panics.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
}

/// Starting an already reserved agent is not a second financial admission. Exact
/// committed capacity fits; the existing new-launch API still correctly refuses
/// a second allocation. Validation must not mutate the estimate or any ledger.
#[test]
fn reserved_agent_start_counts_its_existing_estimate_once_without_reset() {
    let mut db = fixture();
    policy(&db, 1.0);
    db.execute_batch("INSERT INTO reservations VALUES('agent','work',1,'reserved')")
        .unwrap();
    let tx = db.transaction().unwrap();
    swarm_engine::check_reserved_agent_budget(&tx, "work", "agent").unwrap();
    assert!(check_launch_budget(&tx, "work").is_err());
    assert_eq!(
        tx.query_row(
            "SELECT amount FROM reservations WHERE agent_id='agent'",
            [],
            |r| r.get::<_, f64>(0)
        )
        .unwrap(),
        1.0
    );
    assert_eq!(
        tx.query_row(
            "SELECT state FROM reservations WHERE agent_id='agent'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "reserved"
    );
    tx.rollback().unwrap();
}
/// Missing/foreign/released/changed estimates and newly excessive commitments
/// refuse before a provider can be released. A valid witness precedes each fault;
/// restoring only the private fixture precondition permits the same read check.
#[test]
fn reserved_agent_budget_refuses_inconsistent_or_excessive_ledgers_and_recovers() {
    for fault in [
        "foreign",
        "released",
        "amount",
        "missing",
        "other-ledger",
        "tightened",
    ] {
        let mut db = fixture();
        policy(&db, 1.0);
        db.execute_batch("INSERT INTO reservations VALUES('agent','work',1,'reserved')")
            .unwrap();
        let tx = db.transaction().unwrap();
        swarm_engine::check_reserved_agent_budget(&tx, "work", "agent").unwrap();
        tx.rollback().unwrap();
        match fault {
            "foreign" => {
                db.execute_batch("UPDATE reservations SET work_id='other'")
                    .unwrap();
            }
            "released" => {
                db.execute_batch("UPDATE reservations SET state='released'")
                    .unwrap();
            }
            "amount" => {
                db.execute_batch("UPDATE reservations SET amount=0.5")
                    .unwrap();
            }
            "missing" => {
                db.execute_batch("UPDATE reservations SET agent_id='other'")
                    .unwrap();
            }
            "other-ledger" => {
                db.execute_batch(
                    "INSERT INTO assist_reservations VALUES('turn','work',1,'estimated')",
                )
                .unwrap();
            }
            _ => policy(&db, 0.5),
        }
        let before: Vec<(String, String, f64, String)> = db
            .prepare("SELECT agent_id,work_id,amount,state FROM reservations")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let tx = db.transaction().unwrap();
        assert!(
            swarm_engine::check_reserved_agent_budget(&tx, "work", "agent").is_err(),
            "{fault}"
        );
        tx.rollback().unwrap();
        let after: Vec<(String, String, f64, String)> = db
            .prepare("SELECT agent_id,work_id,amount,state FROM reservations")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(before, after);
        db.execute_batch("UPDATE reservations SET agent_id='agent',work_id='work',amount=1,state='reserved'; UPDATE assist_reservations SET state='released'").unwrap();
        policy(&db, 1.0);
        let tx = db.transaction().unwrap();
        swarm_engine::check_reserved_agent_budget(&tx, "work", "agent").unwrap();
        tx.rollback().unwrap();
    }
}

/// Unlimited policy preserves absence of a reservation, but corrupt present
/// estimates still refuse. No missing policy becomes measured zero provider cost.
#[test]
fn reserved_agent_budget_unlimited_keeps_absence_distinct_from_corruption() {
    let mut db = fixture();
    let tx = db.transaction().unwrap();
    swarm_engine::check_reserved_agent_budget(&tx, "work", "agent").unwrap();
    tx.rollback().unwrap();
    db.execute_batch("INSERT INTO reservations VALUES('agent','work',-1,'reserved')")
        .unwrap();
    let tx = db.transaction().unwrap();
    assert!(swarm_engine::check_reserved_agent_budget(&tx, "work", "agent").is_err());
    tx.rollback().unwrap();
    db.execute_batch("UPDATE reservations SET amount=0")
        .unwrap();
    let tx = db.transaction().unwrap();
    swarm_engine::check_reserved_agent_budget(&tx, "work", "agent").unwrap();
    tx.rollback().unwrap();
    assert_eq!(
        db.query_row("SELECT amount FROM reservations", [], |r| r
            .get::<_, f64>(0))
            .unwrap(),
        0.0
    );
}
/// An actual SQLite callback fails during the owned reservation read. Safe
/// diagnostic and typed source survive; explicit statement cleanup cannot turn
/// this refusal into an absent row. Same view recovers after its fault clears.
#[test]
fn reserved_agent_budget_sql_failure_is_not_an_empty_success_and_recovers() {
    use rusqlite::functions::FunctionFlags;
    use std::error::Error;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    let mut db = fixture();
    policy(&db, 1.0);
    let fail = Arc::new(AtomicBool::new(false));
    let injected = fail.clone();
    db.create_scalar_function(
        "reserved_amount78",
        0,
        FunctionFlags::SQLITE_UTF8,
        move |_| {
            if injected.load(Ordering::SeqCst) {
                Err(rusqlite::Error::UserFunctionError(Box::new(
                    std::io::Error::other("PRIVATE_SQL_FAILURE"),
                )))
            } else {
                Ok(1.0)
            }
        },
    )
    .unwrap();
    db.execute_batch("DROP TABLE reservations; CREATE VIEW reservations AS SELECT 'agent' AS agent_id,'work' AS work_id,reserved_amount78() AS amount,'reserved' AS state").unwrap();
    let tx = db.transaction().unwrap();
    swarm_engine::check_reserved_agent_budget(&tx, "work", "agent").unwrap();
    tx.rollback().unwrap();
    fail.store(true, Ordering::SeqCst);
    let tx = db.transaction().unwrap();
    let error = swarm_engine::check_reserved_agent_budget(&tx, "work", "agent").unwrap_err();
    assert_eq!(error.code(), "budget-storage");
    assert!(!format!("{error} {error:?}").contains("PRIVATE_SQL_FAILURE"));
    assert!(error.source().is_some());
    tx.rollback().unwrap();
    fail.store(false, Ordering::SeqCst);
    let tx = db.transaction().unwrap();
    swarm_engine::check_reserved_agent_budget(&tx, "work", "agent").unwrap();
    tx.rollback().unwrap();
}

/// An admitted planning call counts its existing estimate once, starts no
/// process, and cannot turn the funding check into a second reservation.
#[test]
fn reserved_planning_start_counts_existing_estimate_once() {
    let mut db = fixture();
    policy(&db, 1.0);
    let tx = db.transaction().unwrap();
    reserve_planning_call(&tx, "work", "root", "claim", "2026-10-20T12:00:00Z").unwrap();
    tx.commit().unwrap();
    let tx = db.transaction().unwrap();
    swarm_engine::check_reserved_planning_start_budget(&tx, "work", "root", "claim").unwrap();
    assert!(check_launch_budget(&tx, "work").is_err());
    assert_eq!(
        tx.query_row(
            "SELECT estimate FROM planning_calls WHERE id='claim'",
            [],
            |r| r.get::<_, f64>(0)
        )
        .unwrap(),
        1.0
    );
    tx.rollback().unwrap();
}
/// Every fault follows a valid funding witness. This SQL fixture mutates no live
/// data; the guard itself must change neither rows nor policy and preserve cause.
#[test]
fn reserved_planning_start_refuses_missing_foreign_observed_or_corrupt_rows() {
    for sql in [
        "DELETE FROM planning_calls",
        "UPDATE planning_calls SET work_id='foreign'",
        "UPDATE planning_calls SET scope_id='foreign'",
        "UPDATE planning_calls SET estimate=-1",
        "UPDATE planning_calls SET state='released'",
        "UPDATE planning_calls SET state='finished'",
        "UPDATE planning_calls SET usage=x'6e756c6c'",
        "UPDATE planning_calls SET state='unknown'",
    ] {
        let mut db = fixture();
        policy(&db, 2.0);
        let tx = db.transaction().unwrap();
        reserve_planning_call(&tx, "work", "root", "claim", "2026-10-20T12:00:00Z").unwrap();
        tx.commit().unwrap();
        let tx = db.transaction().unwrap();
        swarm_engine::check_reserved_planning_start_budget(&tx, "work", "root", "claim").unwrap();
        tx.rollback().unwrap();
        db.execute_batch(sql).unwrap();
        let before = db.total_changes();
        let tx = db.transaction().unwrap();
        assert!(
            swarm_engine::check_reserved_planning_start_budget(&tx, "work", "root", "claim")
                .is_err(),
            "{sql}"
        );
        tx.rollback().unwrap();
        assert_eq!(db.total_changes(), before);
    }
}
/// Old unlimited estimate0 cannot become a finite-budget permit. A newly
/// reserved call may fit the explicit policy without rewriting the old row.
#[test]
fn reserved_planning_start_rechecks_policy_change_and_keeps_old_reservation() {
    let mut db = fixture();
    let tx = db.transaction().unwrap();
    reserve_planning_call(&tx, "work", "root", "old", "2026-10-20T12:00:00Z").unwrap();
    tx.commit().unwrap();
    let tx = db.transaction().unwrap();
    swarm_engine::check_reserved_planning_start_budget(&tx, "work", "root", "old").unwrap();
    tx.rollback().unwrap();
    policy(&db, 2.0);
    let tx = db.transaction().unwrap();
    assert!(
        swarm_engine::check_reserved_planning_start_budget(&tx, "work", "root", "old").is_err()
    );
    reserve_planning_call(&tx, "work", "root", "fresh", "2026-10-20T12:00:01Z").unwrap();
    swarm_engine::check_reserved_planning_start_budget(&tx, "work", "root", "fresh").unwrap();
    assert_eq!(
        tx.query_row(
            "SELECT estimate FROM planning_calls WHERE id='old'",
            [],
            |r| r.get::<_, f64>(0)
        )
        .unwrap(),
        0.0
    );
    tx.commit().unwrap();
}

/// Increasing the current per-call reserve never upgrades a previously admitted
/// amount. A separate fresh reservation counts both amounts, without reset.
#[test]
fn reserved_planning_start_refuses_increased_reserve_without_upgrading_old_row() {
    let mut db = fixture();
    policy(&db, 2.0);
    let tx = db.transaction().unwrap();
    reserve_planning_call(&tx, "work", "root", "old", "2026-10-20T12:00:00Z").unwrap();
    swarm_engine::check_reserved_planning_start_budget(&tx, "work", "root", "old").unwrap();
    tx.commit().unwrap();
    let next = Budget {
        limit_usd: GoFloat(4.0),
        reserve_per_launch_usd: GoFloat(2.0),
        estimate_source: "Explicit fixture reserve change".into(),
        reference_date: "2026-10-20".into(),
        ..Default::default()
    };
    validate_budget(&next).unwrap();
    db.execute(
        "UPDATE budgets SET body=? WHERE work_id='work'",
        [encode_model(&next).unwrap()],
    )
    .unwrap();
    let tx = db.transaction().unwrap();
    assert!(
        swarm_engine::check_reserved_planning_start_budget(&tx, "work", "root", "old").is_err()
    );
    reserve_planning_call(&tx, "work", "root", "fresh", "2026-10-20T12:00:01Z").unwrap();
    swarm_engine::check_reserved_planning_start_budget(&tx, "work", "root", "fresh").unwrap();
    assert_eq!(
        tx.query_row(
            "SELECT estimate FROM planning_calls WHERE id='old'",
            [],
            |r| r.get::<_, f64>(0)
        )
        .unwrap(),
        1.0
    );
    assert_eq!(
        tx.query_row("SELECT sum(estimate) FROM planning_calls", [], |r| r
            .get::<_, f64>(0))
            .unwrap(),
        3.0
    );
    tx.rollback().unwrap();
}

/// The transaction-type adapter cannot make a bare connection sufficient for
/// planning funding. A valid reservation first passes in TX; bare access refuses
/// without mutation, then the same reservation still passes in a fresh TX.
#[test]
fn reserved_planning_start_requires_active_transaction_without_mutation() {
    let mut db = fixture();
    policy(&db, 1.0);

    let tx = db.transaction().unwrap();
    reserve_planning_call(&tx, "work", "root", "claim", "2026-10-20T12:00:00Z").unwrap();
    swarm_engine::check_reserved_planning_start_budget(&tx, "work", "root", "claim").unwrap();
    tx.commit().unwrap();
    let before: (f64, String) = db
        .query_row(
            "SELECT estimate,state FROM planning_calls WHERE id='claim'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let error = swarm_engine::check_reserved_planning_start_budget(&db, "work", "root", "claim")
        .unwrap_err();
    assert_eq!(error.reason(), "budget-transaction-required");
    let after: (f64, String) = db
        .query_row(
            "SELECT estimate,state FROM planning_calls WHERE id='claim'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(before, after);
    let tx = db.transaction().unwrap();
    swarm_engine::check_reserved_planning_start_budget(&tx, "work", "root", "claim").unwrap();
    tx.rollback().unwrap();
}
