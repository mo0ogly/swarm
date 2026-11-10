//! Verification-only API caller over a marked owned fixture, never a product command.
use rusqlite::{types::Value, Connection};
use std::{error::Error, fs, path::Path};
use swarm_engine::{RootStore, SystemEnvironment, Work};

/// Exact fixture observation; values preserve SQL types and row ordering.
type Snapshot = (Work, Vec<Vec<Value>>);

/// Observe actual Work and mission/event rows, not a reconstructed positive seed.
/// Reads use the already opened Root and explicit SQLite statement finalization.
fn snapshot(
    store: &RootStore,
    db: &Connection,
    work: &str,
) -> Result<Snapshot, Box<dyn Error>> {
    let w = store.read_work(work)?;
    let mut values = Vec::new();
    for sql in [
        "SELECT * FROM cockpit_controls ORDER BY work_id",
        "SELECT * FROM cockpit_events ORDER BY seq",
        "SELECT * FROM events ORDER BY id",
        "SELECT * FROM agents ORDER BY id",
    ] {
        let mut statement = db.prepare(sql)?;
        let count = statement.column_count();
        {
            let mut rows = statement.query([])?;
            while let Some(row) = rows.next()? {
                values.push(
                    (0..count)
                        .map(|i| row.get(i))
                        .collect::<Result<Vec<Value>, _>>()?,
                );
            }
        }
        statement.finalize()?;
    }
    Ok((w, values))
}

/// Apply healthy start, then a negative-only SQL trigger AFTER Root.open.
/// Require a real SQLite trigger-constraint cause and rollback of profile,
/// controls, consent and canonical event together. Drop only the owned trigger,
/// retry the exact refused request and explicitly close both resources. No
/// provider process, fixture feature, positive SQL seed or model call occurs.
/// Input marker must match the canonical owned Root and Work. Errors/panics
/// leave fixture evidence for diagnosis; this tool grants no actor authority.
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 2);
    let input: serde_json::Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let root = Path::new(input["root"].as_str().ok_or("fixture root missing")?).canonicalize()?;
    let work = input["work"].as_str().ok_or("fixture work missing")?;
    let marker: serde_json::Value = serde_json::from_slice(&fs::read(
        root.join(".swarm/verification-mission-start.json"),
    )?)?;
    assert_eq!(marker["root"].as_str(), root.to_str());
    assert_eq!(marker["work"].as_str(), Some(work));
    let mut store = RootStore::open(&root, true)?;
    let mut request = input["request"].clone();
    request["expected_revision"] = serde_json::json!(store.read_work(work)?.revision);
    request["event_id"] = serde_json::json!("sdk-start-healthy");
    let healthy = store
        .execute_external(
            work,
            "mission.start",
            &serde_json::to_vec(&request)?,
            &mut SystemEnvironment,
        )?
        .result
        .work;
    let db = Connection::open(root.join(".swarm/state.db"))?;
    let before = snapshot(&store, &db, work)?;
    request["expected_revision"] = serde_json::json!(healthy.revision);
    request["event_id"] = serde_json::json!("sdk-start-refused");
    request["profile"]["instruction"] =
        serde_json::json!("Uncommitted instruction must not survive");
    db.execute_batch("CREATE TRIGGER fixture_atomic_start BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT,'Owned negative canonical-event fixture'); END")?;
    let error = store
        .execute_external(
            work,
            "mission.start",
            &serde_json::to_vec(&request)?,
            &mut SystemEnvironment,
        )
        .err()
        .ok_or("expected actual SQL refusal")?;
    let mut current: Option<&(dyn Error + 'static)> = Some(&error);
    let mut trigger = false;
    while let Some(cause) = current {
        if let Some(rusqlite::Error::SqliteFailure(code, _)) =
            cause.downcast_ref::<rusqlite::Error>()
        {
            trigger |= code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_TRIGGER;
        }
        current = cause.source();
    }
    assert!(
        trigger,
        "must reach real event-write failure, not an earlier guard"
    );
    assert_eq!(snapshot(&store, &db, work)?, before);
    db.execute_batch("DROP TRIGGER fixture_atomic_start")?;
    let recovered = store
        .execute_external(
            work,
            "mission.start",
            &serde_json::to_vec(&request)?,
            &mut SystemEnvironment,
        )?
        .result
        .work;
    assert_eq!(recovered.revision, healthy.revision + 1);
    assert_eq!(
        recovered.launch_profile.as_ref().unwrap().instruction,
        "Uncommitted instruction must not survive"
    );
    db.close().map_err(|(_, error)| error)?;
    store.close()?;
    println!(
        "{}",
        serde_json::json!({"status":"PASS","actual_sql_trigger_constraint":true,"profile_controls_consent_and_events_rolled_back":true,"same_request_recovered":true,"revision":recovered.revision,"work":work,"provider_called":false})
    );
    Ok(())
}
