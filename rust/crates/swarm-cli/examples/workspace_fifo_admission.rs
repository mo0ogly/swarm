//! Owned verification fixture only: real leases/FIFO/agent lifecycle, inert worker.
use rusqlite::{types::Value, Connection, ErrorCode};
use serde_json::{json, Value as Json};
use std::{
    error::Error,
    fmt, fs,
    io::Write,
    path::Path,
    sync::{Arc, Barrier},
};
use swarm_engine::{MissionLease, MissionQueueRegistration, RootStore, SystemEnvironment};
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
/// Inspect typed SQLite causes, never match diagnostic wording or hide other errors.
fn sql_code(mut error: &(dyn Error + 'static), expected: ErrorCode) -> bool {
    loop {
        if let Some(rusqlite::Error::SqliteFailure(code, _)) =
            error.downcast_ref::<rusqlite::Error>()
        {
            if code.code == expected {
                return true;
            }
        }
        match error.source() {
            Some(cause) => error = cause,
            None => return false,
        }
    }
}
/// Configure explicit operator fixture consent/profile via the public Work writer.
fn start(store: &mut RootStore, work: &str, event: &str, workspace: &str) -> Result<()> {
    let raw = json!({"schema_version":1,"event_id":event,"expected_revision":store.read_work(work)?.revision,
        "reason":"Owned FIFO verification", "profile":{"provider":"fixture","role":"worker","workspace":workspace,"level":"standard","capture_output":true},"slots":1});
    store.execute_external(
        work,
        "mission.start",
        &serde_json::to_vec(&raw)?,
        &mut SystemEnvironment,
    )?;
    Ok(())
}
/// Derive only identity/revision of a requested intention, never a lease or profile.
fn request(
    store: &RootStore,
    work: &str,
    event: &str,
) -> Result<swarm_engine::MissionWorkerRequest> {
    Ok(swarm_engine::MissionWorkerRequest {
        schema_version: 1,
        event_id: event.into(),
        expected_revision: store.read_work(work)?.revision,
        task_id: "task".into(),
    })
}
/// Require the actual selected task and retain its typed refusal if not ready.
fn ready(value: MissionQueueRegistration) -> Result<()> {
    if let Some(error) = value.permission_refusal {
        return Err(Box::new(error));
    }
    if value.ready.iter().any(|task| task == "task") {
        return Ok(());
    }
    if let Some(refusal) = value.refused.into_iter().find(|value| value.task == "task") {
        return Err(Box::new(refusal.error));
    }
    Err("missing candidate and refusal".into())
}
/// Exercise public FIFO under two true leases; only sqlite_sequence is corrupted
/// as a negative late-journal fixture after Root open. Acquisition, paused waiting
/// cleanup and queued cancellation each require typed FULL plus complete rollback,
/// followed by exact counter restoration and real recovery. Preserve exact held
/// turn/reservation values over pause cleanup, and preview an overlapping younger
/// task after cancellation before requeueing the older Work. No claim, permission,
/// budget or Agent state is seeded through SQL. No provider runs on the shared
/// path; the disjoint worker is an owned inert executable. No global watchdog,
/// timeout or product retry policy is introduced; failures preserve fixture state.
fn exercise(
    store: &mut RootStore,
    leases: &mut Vec<MissionLease>,
    db: &Connection,
    root: &Path,
    first: &str,
    second: &str,
) -> Result<Json> {
    start(store, first, "fifo-start-first", "fifo-shared")?;
    start(store, second, "fifo-start-second", "fifo-shared")?;
    leases.push(store.claim_mission_supervision(
        first,
        "fifo-owner-first",
        &swarm_engine::system_timestamp()?,
    )?);
    leases.push(store.claim_mission_supervision(
        second,
        "fifo-owner-second",
        &swarm_engine::system_timestamp()?,
    )?);
    ready(store.sync_mission_workspace_queue(&leases[0])?)?;
    let initial = store.mission_workspace_queue(first)?;
    assert_eq!(initial.turns.len(), 2); // Two actual first-Work tasks.
    let a = RootStore::open(root, true)?;
    let b = RootStore::open(root, true)?;
    let gate = Arc::new(Barrier::new(2));
    let results = std::thread::scope(|scope| {
        let one = &leases[0];
        let two = &leases[1];
        let ga = gate.clone();
        let gb = gate.clone();
        let ta = scope.spawn(move || {
            let mut a = a;
            ga.wait();
            let value = a.sync_mission_workspace_queue(one);
            (value, a.close())
        });
        let tb = scope.spawn(move || {
            let mut b = b;
            gb.wait();
            let value = b.sync_mission_workspace_queue(two);
            (value, b.close())
        });
        Ok::<_, Box<dyn Error>>((
            ta.join().map_err(|_| "first sync thread panicked")?,
            tb.join().map_err(|_| "second sync thread panicked")?,
        ))
    })?;
    for (index, (value, close)) in [results.0, results.1].into_iter().enumerate() {
        close?;
        match value {
            Ok(value) => ready(value)?,
            Err(error)
                if sql_code(&error, ErrorCode::DatabaseBusy)
                    || sql_code(&error, ErrorCode::DatabaseLocked) =>
            {
                // New precondition: both competing transactions have ended.
                ready(store.sync_mission_workspace_queue(&leases[index])?)?;
            }
            Err(error) => return Err(Box::new(error)),
        }
    }
    let view = store.mission_workspace_queue(second)?;
    assert_eq!(view.blockers.len(), 1);
    assert_eq!(view.blockers[0].older_work, first);
    let original_second = view
        .turns
        .iter()
        .find(|turn| turn.work == second && turn.task == "task")
        .ok_or("second turn absent")?
        .seq
        .clone();
    let denied = request(store, second, "fifo-denied-second")?;
    let before = snapshot(db)?;
    let error = store
        .prepare_mission_worker(&leases[1], &denied, false, &mut SystemEnvironment)
        .err()
        .ok_or("younger turn admitted")?;
    let mut cause: &(dyn Error + 'static) = &error;
    let found = loop {
        if matches!(
            cause.downcast_ref::<swarm_engine::MissionSupervisionError>(),
            Some(swarm_engine::MissionSupervisionError::Refused(
                "workspace-turn-fifo-blocked"
            ))
        ) {
            break true;
        }
        match cause.source() {
            Some(next) => cause = next,
            None => break false,
        }
    };
    assert!(found, "expected exact typed FIFO refusal");
    assert_eq!(snapshot(db)?, before);
    println!("PASS actual concurrent Root sync and FIFO denial before any Agent exists");
    let launch = request(store, first, "fifo-first-agent")?;
    assert!(
        store
            .prepare_mission_worker(&leases[0], &launch, true, &mut SystemEnvironment)?
            .preview_only
    );
    let original_counter: i64 = db.query_row(
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
    let fault = snapshot(db)?;
    let error = store
        .prepare_mission_worker(&leases[0], &launch, false, &mut SystemEnvironment)
        .err()
        .ok_or("journal overflow admitted")?;
    assert!(
        sql_code(&error, ErrorCode::DiskFull),
        "expected typed journal ordinal exhaustion"
    );
    assert_eq!(snapshot(db)?, fault);
    assert_eq!(
        db.execute(
            "UPDATE sqlite_sequence SET seq=? WHERE name='workspace_turn_events'",
            [original_counter]
        )?,
        1
    );
    let first_agent =
        store.prepare_mission_worker(&leases[0], &launch, false, &mut SystemEnvironment)?;
    let binding: Json = serde_json::from_str(
        first_agent
            .agent
            .native_mission
            .as_ref()
            .ok_or("binding absent")?
            .0
            .get(),
    )?;
    assert_eq!(binding["schema_version"], 3);
    assert!(binding["workspace_turn_seq"].is_string());
    println!("PASS late journal write failure rollback and same-request recovery with Binding v3");
    let unchanged = snapshot(db)?;
    assert!(
        !store
            .prepare_mission_worker(&leases[0], &launch, false, &mut SystemEnvironment)?
            .created
    );
    assert_eq!(snapshot(db)?, unchanged);
    let acquired_before_pause = store
        .mission_workspace_queue(first)?
        .turns
        .into_iter()
        .find(|turn| turn.work == first && turn.task == "task")
        .ok_or("held turn absent before pause")?;
    let reservations_before_pause = snapshot(db)?
        .into_iter()
        .find(|(name, _)| name == "reservations")
        .ok_or("reservation table absent before pause")?;
    let pause = json!({"schema_version":1,"event_id":"fifo-pause-first","expected_revision":store.read_work(first)?.revision,"reason":"Owned waiting cleanup and acquired retention witness"});
    store.execute_external(
        first,
        "mission.pause",
        &serde_json::to_vec(&pause)?,
        &mut SystemEnvironment,
    )?;
    let before_cleanup_fault = snapshot(db)?;
    let cleanup_counter: i64 = db.query_row(
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
    let cleanup_fault = snapshot(db)?;
    let cleanup_error = store
        .sync_mission_workspace_queue(&leases[0])
        .err()
        .ok_or("journal overflow cleanup unexpectedly succeeded")?;
    assert!(
        sql_code(&cleanup_error, ErrorCode::DiskFull),
        "expected typed journal exhaustion during waiting cleanup"
    );
    assert_eq!(
        snapshot(db)?,
        cleanup_fault,
        "waiting cleanup error must roll back all SQL effects"
    );
    assert_eq!(
        db.execute(
            "UPDATE sqlite_sequence SET seq=? WHERE name='workspace_turn_events'",
            [cleanup_counter]
        )?,
        1
    );
    assert_eq!(
        snapshot(db)?,
        before_cleanup_fault,
        "exact original journal counter must restore the valid precondition"
    );
    let cleanup = store.sync_mission_workspace_queue(&leases[0])?;
    assert!(cleanup.ready.is_empty());
    assert!(matches!(
        cleanup.permission_refusal,
        Some(swarm_engine::MissionSupervisionError::Refused(
            "planning-mission-paused"
        ))
    ));
    let paused = store.mission_workspace_queue(first)?;
    assert_eq!(
        paused
            .turns
            .iter()
            .find(|turn| turn.work == first && turn.task == "task")
            .ok_or("held turn absent")?,
        &acquired_before_pause,
        "pause cleanup must preserve every acquired turn field including seq, owner and timestamps"
    );
    assert_eq!(
        snapshot(db)?
            .into_iter()
            .find(|(name, _)| name == "reservations")
            .ok_or("reservation table absent after pause")?,
        reservations_before_pause,
        "pause cleanup must preserve the exact reservation rows"
    );
    assert_eq!(
        paused
            .turns
            .iter()
            .find(|turn| turn.work == first && turn.task == "task-2")
            .ok_or("waiting cleanup absent")?
            .state,
        "released"
    );
    println!("PASS paused mission releases waiting turns and preserves acquired ownership");
    store.stop_agent(&first_agent.agent.id, &mut SystemEnvironment)?;
    let before_cancel_fault = snapshot(db)?;
    let cancel_counter: i64 = db.query_row(
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
    let cancel_fault = snapshot(db)?;
    let cancel_error = store
        .cancel_queued_agent(&first_agent.agent.id, &mut SystemEnvironment)
        .err()
        .ok_or("journal overflow cancellation unexpectedly succeeded")?;
    assert!(
        sql_code(&cancel_error, ErrorCode::DiskFull),
        "expected typed journal exhaustion during acquired release"
    );
    assert_eq!(
        snapshot(db)?,
        cancel_fault,
        "release error must roll back Agent, task, budget and journal effects"
    );
    assert_eq!(
        db.execute(
            "UPDATE sqlite_sequence SET seq=? WHERE name='workspace_turn_events'",
            [cancel_counter]
        )?,
        1
    );
    assert_eq!(
        snapshot(db)?,
        before_cancel_fault,
        "exact counter restoration must recover the prior cancellation precondition"
    );
    let cancelled = store.cancel_queued_agent(&first_agent.agent.id, &mut SystemEnvironment)?;
    assert_eq!(cancelled.status, "interrupted");
    let released = snapshot(db)?;
    store.cancel_queued_agent(&first_agent.agent.id, &mut SystemEnvironment)?;
    assert_eq!(snapshot(db)?, released);
    let overlapping_before = snapshot(db)?;
    let overlapping_request = request(store, second, "fifo-overlapping-preview-after-release")?;
    let overlapping_preview = store.prepare_mission_worker(
        &leases[1],
        &overlapping_request,
        true,
        &mut SystemEnvironment,
    )?;
    assert!(
        overlapping_preview.preview_only,
        "younger same-workspace candidate must advance after safe release"
    );
    assert_eq!(
        snapshot(db)?,
        overlapping_before,
        "overlapping progress preview must retain no mutation"
    );
    assert!(!root.join("fifo-shared/model-received.json").exists());
    println!("PASS late cleanup/release faults and overlapping progress");
    let unpause = json!({"schema_version":1,"event_id":"fifo-unpause-first","expected_revision":store.read_work(first)?.revision,"reason":"Owned cleanup recovery"});
    store.execute_external(
        first,
        "mission.unpause",
        &serde_json::to_vec(&unpause)?,
        &mut SystemEnvironment,
    )?;
    let restored = store.sync_mission_workspace_queue(&leases[0])?;
    assert!(restored.ready.iter().any(|task| task == "task-2"));
    start(store, second, "fifo-second-disjoint", "fifo-disjoint")?;
    ready(store.sync_mission_workspace_queue(&leases[1])?)?;
    let after = store.mission_workspace_queue(second)?;
    assert!(after.blockers.is_empty());
    let next = after
        .turns
        .iter()
        .find(|turn| turn.work == second && turn.task == "task")
        .ok_or("new second turn absent")?;
    assert_ne!(next.seq, original_second);
    let old_count: i64 = db.query_row(
        "SELECT count(*) FROM workspace_turn_events WHERE turn_seq=? AND state='released'",
        [original_second.parse::<i64>()?],
        |row| row.get(0),
    )?;
    assert_eq!(old_count, 1);
    assert!(after
        .turns
        .iter()
        .any(|turn| turn.work == first && turn.task == "task-2" && turn.state == "waiting"));
    let second_agent = store.prepare_mission_worker(
        &leases[1],
        &request(store, second, "fifo-second-agent")?,
        false,
        &mut SystemEnvironment,
    )?;
    let actual = store.run_mission_cycle(&leases[1], |store, _, monitor| {
        assert!(monitor.is_running());
        store.run_agent(&second_agent.agent.id)
    })?;
    assert_eq!(actual.agent.status, "completed");
    assert!(!actual.task_accepted);
    let final_view = store.mission_workspace_queue(second)?;
    assert_eq!(
        final_view
            .turns
            .iter()
            .find(|turn| turn.work == second && turn.task == "task")
            .ok_or("terminal turn absent")?
            .state,
        "released"
    );
    assert!(root.join("fifo-disjoint/model-received.json").is_file());
    assert!(!root.join("fifo-shared/model-received.json").exists());
    println!("PASS public cancellation/replay, requeue history, disjoint worker completion and atomic release");
    Ok(
        json!({"status":"PASS","two_real_leases":true,"two_connection_sync":true,"binding_schema":3,"safe_cancel_and_completion_release":true,"journal_fault_after_root_open":true,"paused_cleanup_preserves_acquired":true,"late_cleanup_release_faults_and_overlapping_progress":true,"provider":"owned inert executable","task_accepted":false}),
    )
}
/// Route the real private gate; otherwise require an owned fixture marker and
/// explicitly close all leases/Root after either operation success or failure.
/// Publish fresh result bytes atomically after flush/fsync; an existing result
/// refuses overwrite and original failures remain available in the caller channels.
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
        return Err("owned FIFO fixture input required".into());
    }
    let input: Json = serde_json::from_slice(&fs::read(&args[1])?)?;
    let root = Path::new(input["root"].as_str().ok_or("root absent")?).canonicalize()?;
    let marker: Json =
        serde_json::from_slice(&fs::read(root.join(".swarm/verification-fifo.json"))?)?;
    if marker != input || marker["kind"] != "owned-inert-fifo-fixture" {
        return Err("fixture marker mismatch".into());
    }
    let first = input["first"].as_str().ok_or("first Work absent")?;
    let second = input["second"].as_str().ok_or("second Work absent")?;
    let db = Connection::open(root.join(".swarm/state.db"))?;
    let mut store = RootStore::open(&root, true)?;
    let mut leases = Vec::new();
    let result = exercise(&mut store, &mut leases, &db, &root, first, second);
    let mut failures = Vec::new();
    for lease in leases {
        if let Err(error) = lease.close() {
            failures.push(Box::new(error) as Box<dyn Error>);
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
    let destination = Path::new(input["result"].as_str().ok_or("result path absent")?);
    let mut output =
        tempfile::NamedTempFile::new_in(destination.parent().ok_or("result parent absent")?)?;
    output.write_all(&serde_json::to_vec_pretty(&result)?)?;
    output.flush()?;
    output.as_file().sync_all()?;
    output.persist_noclobber(destination)?;
    fs::File::open(destination.parent().ok_or("result parent absent")?)?.sync_all()?;
    println!("PASS actual lease-owned workspace FIFO lifecycle");
    Ok(())
}
