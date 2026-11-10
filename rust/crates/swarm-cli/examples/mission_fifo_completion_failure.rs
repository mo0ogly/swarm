//! Owned negative fixture: inert provider reaped, FIFO settlement fails, then
//! actual live-owner recovery refuses. Parent must reconcile after this SDK exits.
use rusqlite::{types::Value, Connection, ErrorCode};
use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt, fs, io::Write, path::Path};
use swarm_engine::{MissionLease, RootStore, SystemEnvironment};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Snapshot = Vec<(String, Vec<Vec<Value>>)>;
/// Preserve independent operation/close causes; this fixture aggregate is not a
/// product error classification or an assertion that a failed action had no effect.
#[derive(Debug)]
struct Failures(Vec<Box<dyn Error>>);
impl fmt::Display for Failures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "verification/cleanup failed: {} causes retained",
            self.0.len()
        )
    }
}
impl Error for Failures {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.first().map(|cause| cause.as_ref())
    }
}
/// Join two checked outcomes without dropping a secondary cleanup cause.
fn finish<T>(result: Result<T>, cleanup: Result<()>) -> Result<T> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(first), Err(second)) => Err(Box::new(Failures(vec![first, second]))),
    }
}
/// Read all actual SQL tables, quoting identifiers from SQLite metadata; finalize
/// every statement and preserve independent errors. Includes sqlite_sequence.
fn snapshot(db: &Connection) -> Result<Snapshot> {
    let mut statement =
        db.prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")?;
    let names = (|| -> Result<Vec<String>> {
        let mut rows = statement.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row.get(0)?);
        }
        Ok(out)
    })();
    let names = finish(names, statement.finalize().map_err(Into::into))?;
    let mut out = Vec::new();
    for name in names {
        let mut statement = db.prepare(&format!(
            "SELECT * FROM \"{}\" ORDER BY rowid",
            name.replace('"', "\"\"")
        ))?;
        let count = statement.column_count();
        let data = (|| -> Result<Vec<Vec<Value>>> {
            let mut rows = statement.query([])?;
            let mut data = Vec::new();
            while let Some(row) = rows.next()? {
                data.push(
                    (0..count)
                        .map(|column| row.get(column))
                        .collect::<std::result::Result<_, _>>()?,
                );
            }
            Ok(data)
        })();
        out.push((
            name,
            finish(data, statement.finalize().map_err(Into::into))?,
        ));
    }
    Ok(out)
}

/// Traverse the real public native wrapper matrix, including secondary cleanup
/// and IO custom-inner branches. Pointer deduplication prevents error cycles;
/// no diagnostic string, arbitrary depth bound or success inference is used.
fn contains(
    error: &(dyn Error + 'static),
    predicate: impl Fn(&(dyn Error + 'static)) -> bool,
) -> bool {
    let mut pending = vec![error];
    let mut seen: Vec<*const dyn Error> = Vec::new();
    while let Some(error) = pending.pop() {
        let pointer = error as *const dyn Error;
        if seen.iter().any(|prior| std::ptr::eq(*prior, pointer)) {
            continue;
        }
        seen.push(pointer);
        if predicate(error) {
            return true;
        }
        pending.extend(
            swarm_engine::native_error_links(error)
                .children
                .into_iter()
                .map(|(_, child)| child),
        );
    }
    false
}

/// Read the actual single Agent reservation using exact producer fields; absence
/// stays explicit. This read invents no budget and explicitly finalizes its cursor.
fn reservation(db: &Connection, agent: &str) -> Result<Option<(String, f64, String)>> {
    let mut statement =
        db.prepare("SELECT work_id,amount,state FROM main.reservations WHERE agent_id=?")?;
    let result = (|| -> Result<_> {
        let mut rows = statement.query([agent])?;
        let value = rows
            .next()?
            .map(|row| {
                Ok::<_, rusqlite::Error>((
                    row.get::<_, String>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .transpose()?;
        if rows.next()?.is_some() {
            return Err("duplicate Agent reservation".into());
        }
        Ok(value)
    })();
    finish(result, statement.finalize().map_err(Into::into))
}

/// Exercise only an owned Root already planned through public operations. Start
/// consent/profile and actual lease, register/safely prepare Binding3, then exhaust
/// ONLY the journal's SQLite ordinal as a negative fixture after Root open. The
/// inert provider really runs/reaps; typed FULL must preserve active Agent/turn/
/// reservation and a durable reaped receipt. Restore the exact counter and prove
/// actual reconcile refuses this still-live original supervisor. No positive
/// authority SQL, provider replay, timeout, acceptance or consumption reset occurs.
fn exercise(
    store: &mut RootStore,
    leases: &mut Vec<MissionLease>,
    db: &Connection,
    root: &Path,
    work: &str,
) -> Result<Json> {
    fs::create_dir(root.join("fifo-completion"))?;
    let start = json!({"schema_version":1,"event_id":"fifo-completion-start","expected_revision":store.read_work(work)?.revision,
        "reason":"Owned inert completion failure verification","profile":{"provider":"fixture","role":"worker","workspace":"fifo-completion","level":"standard","capture_output":true},"slots":1});
    store.execute_external(
        work,
        "mission.start",
        &serde_json::to_vec(&start)?,
        &mut SystemEnvironment,
    )?;
    leases.push(store.claim_mission_supervision(
        work,
        "fifo-completion-owner",
        &swarm_engine::system_timestamp()?,
    )?);
    let ready = store.sync_mission_workspace_queue(&leases[0])?;
    if let Some(error) = ready.permission_refusal {
        return Err(Box::new(error));
    }
    if !ready.ready.iter().any(|id| id == "task") {
        if let Some(value) = ready.refused.into_iter().find(|value| value.task == "task") {
            return Err(Box::new(value.error));
        }
        return Err("expected actual public ready task".into());
    }
    let request = swarm_engine::MissionWorkerRequest {
        schema_version: 1,
        event_id: "fifo-completion-agent".into(),
        expected_revision: store.read_work(work)?.revision,
        task_id: "task".into(),
    };
    let prepared =
        store.prepare_mission_worker(&leases[0], &request, false, &mut SystemEnvironment)?;
    let agent = &prepared.agent;
    let binding: Json = serde_json::from_str(
        agent
            .native_mission
            .as_ref()
            .ok_or("Binding absent")?
            .0
            .get(),
    )?;
    assert_eq!(binding["schema_version"], 3);
    let turn_seq = binding["workspace_turn_seq"]
        .as_str()
        .ok_or("turn seq absent")?
        .to_owned();
    let held_before = store
        .mission_workspace_queue(work)?
        .turns
        .into_iter()
        .find(|turn| turn.seq == turn_seq)
        .ok_or("prepared turn absent")?;
    assert_eq!(held_before.state, "acquired");
    assert_eq!(held_before.agent, agent.id);
    let reserved_before = reservation(db, &agent.id)?;
    assert!(
        reserved_before
            .as_ref()
            .is_some_and(|(scope, amount, state)| scope == work
                && *amount > 0.0
                && state == "reserved"),
        "public fixture must have an actual positive reserved envelope"
    );
    let marker = root.join("fifo-completion/model-received.json");
    assert!(!marker.exists(), "provider witness must be fresh");
    let counter: i64 = db.query_row(
        "SELECT seq FROM sqlite_sequence WHERE name='workspace_turn_events'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(
        db.execute(
            "UPDATE sqlite_sequence SET seq=? WHERE name='workspace_turn_events'",
            [i64::MAX]
        )?,
        1
    );
    let failure = store
        .run_mission_cycle(&leases[0], |store, _, monitor| {
            assert!(monitor.is_running());
            store.run_agent(&agent.id)
        })
        .err()
        .ok_or("completion unexpectedly settled with exhausted FIFO journal")?;
    let action = failure
        .action
        .as_ref()
        .and_then(|value| value.as_ref().err())
        .ok_or("cycle did not retain action failure")?;
    if !contains(
        action,
        |error| matches!(error.downcast_ref::<rusqlite::Error>(),Some(rusqlite::Error::SqliteFailure(code,_)) if code.code == ErrorCode::DiskFull),
    ) {
        return Err(failure);
    }
    if failure.supervision.is_some() || failure.journal.is_some() {
        return Err(failure);
    }
    let diagnostics = failure.diagnostics();
    assert!(
        marker.is_file(),
        "inert provider must really have written its marker"
    );
    let retained = store.read_agent(&agent.id)?;
    assert!(matches!(retained.status.as_str(), "starting" | "running"));
    assert_eq!(retained.supervisor_pid, i64::from(std::process::id()));
    assert!(
        retained.native_execution_result.is_none() && retained.ended.is_empty(),
        "failed SQL settlement must not appear recorded as terminal"
    );
    let held_after = store
        .mission_workspace_queue(work)?
        .turns
        .into_iter()
        .find(|turn| turn.seq == turn_seq)
        .ok_or("turn missing after failed settlement")?;
    assert_eq!(held_after, held_before);
    let reserved_after = reservation(db, &agent.id)?;
    assert_eq!(reserved_after, reserved_before);
    let receipt_file = format!(
        "native-completion-{:x}.json",
        Sha256::digest(agent.id.as_bytes())
    );
    let receipt_bytes = fs::read(root.join(".swarm").join(&receipt_file))?;
    let receipt: Json = serde_json::from_slice(&receipt_bytes)?;
    assert_eq!(receipt["schema_version"], 1);
    assert_eq!(receipt["expected"]["id"], agent.id);
    assert_eq!(receipt["incomplete_output"], false);
    assert_eq!(receipt["exit_code"], 0);
    assert_eq!(receipt["failed"], false);
    assert_eq!(receipt["interrupted"], false);
    assert_eq!(receipt["details"]["output_complete"], true);
    assert_eq!(receipt["may_have_started"], true);
    assert_eq!(receipt["details"]["kernel_reaped"], true);
    assert_eq!(receipt["details"]["workspace_release_confirmed"], true);
    assert_eq!(receipt["details"]["recovery_required"], false);
    println!("PASS provider completed but FIFO settlement journal failure retains ownership");
    assert_eq!(
        db.execute(
            "UPDATE sqlite_sequence SET seq=? WHERE name='workspace_turn_events'",
            [counter]
        )?,
        1
    );
    let before_reconcile = snapshot(db)?;
    let error = store
        .reconcile_agent(&agent.id, &mut SystemEnvironment)
        .err()
        .ok_or("live original supervisor unexpectedly reconciled")?;
    if !contains(&error, |cause| {
        matches!(
            cause.downcast_ref::<swarm_engine::AgentLaunchError>(),
            Some(swarm_engine::AgentLaunchError::Refused(
                "agent-completion-supervisor-still-live"
            ))
        )
    }) {
        return Err(Box::new(error));
    }
    assert_eq!(snapshot(db)?, before_reconcile);
    assert_eq!(
        store
            .mission_workspace_queue(work)?
            .turns
            .into_iter()
            .find(|turn| turn.seq == turn_seq)
            .ok_or("held turn disappeared")?,
        held_after
    );
    println!("PASS live owner reconcile refuses and preserves held FIFO state");
    Ok(
        json!({"status":"READY_FOR_POST_EXIT_RECONCILE","work":work,"agent_id":agent.id,"turn_seq":turn_seq,"pre_reconcile_status":retained.status,
        "held_turn":held_after,"reservation_before":reserved_before,"reservation_after":reserved_after,"receipt_sha256":format!("{:x}",Sha256::digest(&receipt_bytes)),"receipt_file":receipt_file,
        "provider_marker":"fifo-completion/model-received.json","completion_sqlite_full":true,"held_on_failure":true,"live_owner_reconcile_refused":true,"known_receipt":true,
        "diagnostics":diagnostics,"paid_provider":false,"task_accepted":false}),
    )
}

/// Dispatch the actual private gate, otherwise require the exact owned marker.
/// All opened leases/Root/SQL connection close after either outcome, preserving
/// independent failures. Publish a fresh atomically synced result only after
/// cleanup; parent must wait for THIS process exit before public CLI recovery.
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|value| value == "_agent_gate") {
        std::process::exit(i32::from(swarm_cli::run(
            &args[1..],
            "en",
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        )));
    }
    if args.len() != 2 {
        return Err("owned completion fixture input required".into());
    }
    let input: Json = serde_json::from_slice(&fs::read(&args[1])?)?;
    let root = Path::new(input["root"].as_str().ok_or("root absent")?).canonicalize()?;
    let marker: Json = serde_json::from_slice(&fs::read(
        root.join(".swarm/verification-fifo-completion.json"),
    )?)?;
    if marker != input || marker["kind"] != "owned-inert-fifo-completion-fixture" {
        return Err("fixture marker mismatch".into());
    }
    let work = input["work"].as_str().ok_or("work absent")?;
    let mut store = RootStore::open(&root, true)?;
    let db = match Connection::open(root.join(".swarm/state.db")) {
        Ok(db) => db,
        Err(error) => return finish(Err(Box::new(error)), store.close().map_err(Into::into)),
    };
    let mut leases = Vec::new();
    let result = exercise(&mut store, &mut leases, &db, &root, work);
    let mut failures: Vec<Box<dyn Error>> = Vec::new();
    for lease in leases {
        if let Err(error) = lease.close() {
            failures.push(Box::new(error));
        }
    }
    if let Err(error) = store.close() {
        failures.push(Box::new(error));
    }
    if let Err((_, error)) = db.close() {
        failures.push(Box::new(error));
    }
    let result = finish(
        result,
        if failures.is_empty() {
            Ok(())
        } else {
            Err(Box::new(Failures(failures)))
        },
    )?;
    let destination = Path::new(input["result"].as_str().ok_or("result absent")?);
    let parent = destination.parent().ok_or("result parent absent")?;
    let mut output = tempfile::NamedTempFile::new_in(parent)?;
    output.write_all(&serde_json::to_vec_pretty(&result)?)?;
    output.flush()?;
    output.as_file().sync_all()?;
    output.persist_noclobber(destination)?;
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}
