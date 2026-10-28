//! Explicit queued-intention dispatch fixture; no provider runs in this SDK.
use rusqlite::{types::Value, Connection};
use serde_json::{json, Value as Json};
use std::{error::Error, fmt, fs, io::Write, path::Path};
use swarm_engine::{
    AgentLaunchError, MissionDispatchResourceRefusal as Resource, MissionLease,
    MissionWorkerDispatchOutcome as Outcome, MissionWorkerDispatchRequest, RootStore,
    RootStoreError, SystemEnvironment,
};
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

/// Return a safe static fixture failure through normal Result cleanup. Never
/// panic or format actual Agent/SQL/prompt values when an assertion fails.
fn require(condition: bool, message: &'static str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

/// Write and flush one static evidence marker with checked IO. Broken stdout or
/// flush failure returns the original IO error through exercise's Result, so
/// main still closes all leases/Root/SQL handles. A partial published marker is
/// not proof of overall success; no EPIPE runtime injection is claimed here.
fn write_marker(message: &'static str) -> Result<()> {
    let mut output = std::io::stdout().lock();
    writeln!(output, "{message}")?;
    output.flush()?;
    Ok(())
}

/// Read one real stored manual launch through its authoritative wire decoder.
/// No request/profile is manufactured by SQL. Read/decoder/finalization errors
/// remain errors; no DB mutation or authority follows from this observation.
fn recorded_launch(db: &Connection, agent: &str) -> Result<swarm_engine::Launch> {
    let mut statement = db.prepare("SELECT CAST(request AS BLOB) FROM main.agents WHERE id=?")?;
    let raw = statement
        .query_row([agent], |row| row.get::<_, Vec<u8>>(0))
        .map_err(Into::into);
    let raw = finish(raw, statement.finalize().map_err(Into::into))?;
    Ok(swarm_engine::decode_launch(&raw)?)
}

/// Project all known error branches into safe static labels and numeric codes,
/// including secondary cleanup/custom IO inners. Pointer deduplication terminates
/// cycles without a hidden depth cutoff. No private Display, SQL/Agent/prompt
/// data or inferred effect/permission is emitted; the original typed error stays
/// owned by the caller throughout this borrowed diagnostic inspection.
fn safe_causes(error: &(dyn Error + 'static)) -> Vec<Json> {
    let mut pending = vec![error];
    let mut seen: Vec<*const dyn Error> = Vec::new();
    let mut result = Vec::new();
    while let Some(error) = pending.pop() {
        let pointer = error as *const dyn Error;
        if seen.iter().any(|prior| std::ptr::eq(*prior, pointer)) {
            continue;
        }
        seen.push(pointer);
        let links = swarm_engine::native_error_links(error);
        result.push(json!({"labels": links.labels, "sqlite_code": links.sqlite_code, "io_errno": links.io_errno}));
        pending.extend(links.children.into_iter().map(|(_, child)| child));
    }
    result
}

/// Exercise a clean namespace/revision replay refusal under a real owning lease.
/// Require its exact direct typed conflict, before any queue sync/preparation,
/// and unchanged ALL-table state. Composite/cleanup/unknown failures retain their
/// complete typed report as Err instead of passing due to a nested conflict.
/// Safe diagnostics are metadata only; no retry, provider, mutation or new
/// identity follows from this expected denial. Caller still closes every handle.
fn replay_refusal(
    store: &mut RootStore,
    lease: &MissionLease,
    request: &MissionWorkerDispatchRequest,
    db: &Connection,
    case: &'static str,
) -> Result<Json> {
    let before = snapshot(db)?;
    let error = match store.dispatch_mission_worker(lease, request, &mut SystemEnvironment) {
        Err(error) => error,
        Ok(_) => return Err("conflicting original unexpectedly dispatched".into()),
    };
    let clean = matches!(&error.cause,
        swarm_engine::MissionDispatchCause::Root(root) if matches!(root.as_ref(), RootStoreError::Launch(launch)
            if matches!(launch.as_ref(), AgentLaunchError::Refused("agent-launch-event-conflict"))));
    let before_sync = error.phase == "replay"
        && matches!(
            &error.queue,
            swarm_engine::MissionDispatchQueueEffect::NotAttempted
        )
        && matches!(&error.preparation, swarm_engine::MissionDispatchPreparationEffect::ReplayAttempted { event_id } if event_id == &request.event_id)
        && error.resources.is_empty();
    if !clean || !before_sync {
        return Err(error);
    }
    let observed = safe_causes(error.as_ref());
    if snapshot(db)? != before {
        return Err(Box::new(Failures(vec![
            error,
            "expected replay refusal changed SQL".into(),
        ])));
    }
    Ok(
        json!({"case":case,"phase":"replay","queue":"not_attempted","all_sql_unchanged":true,"causes":observed}),
    )
}

/// Explicit public fixture consent/profile, with one persisted mission slot.
/// Values describe this finite witness, not a dispatcher concurrency policy.
fn start(store: &mut RootStore, work: &str, event: &str, workspace: &str) -> Result<()> {
    let value = json!({"schema_version":1,"event_id":event,"expected_revision":store.read_work(work)?.revision,
        "reason":"Owned intention selection verification","profile":{"provider":"fixture","role":"worker","workspace":workspace,"level":"standard","capture_output":true},"slots":1});
    store.execute_external(
        work,
        "mission.start",
        &serde_json::to_vec(&value)?,
        &mut SystemEnvironment,
    )?;
    Ok(())
}

/// Use actual saved task overrides and engine-created leases. The first target
/// contender is older-FIFO-blocked; select the next disjoint task, replay the SAME
/// identity without another row; reject real manual/foreign-Work/changed-revision
/// replay namespaces before any synchronization. Pause/clean the real older blocker so the shared
/// contender becomes FIFO-free, observe its capacity refusal as a global pass
/// stop while preserving the disjoint acquired owner, then
/// expose pause cleanup/refusal while preserving acquired ownership. A real
/// heartbeat wraps initial selection only; no provider runs. No SQL seeds claims,
/// profiles, permissions, queue or attempts. Snapshot comparisons exclude no SQL
/// table. Evidence marker writes/flush return IO errors instead of panicking;
/// main retains handle cleanup even after partial output. All SDK errors/cleanup
/// remain explicit, not converted to an empty result.
fn exercise(
    store: &mut RootStore,
    leases: &mut Vec<MissionLease>,
    db: &Connection,
    root: &Path,
    work: &str,
    blocker: &str,
) -> Result<Json> {
    fs::create_dir(root.join("dispatch-disjoint"))?;
    start(store, blocker, "dispatch-blocker-start", "dispatch-shared")?;
    start(store, work, "dispatch-target-start", "dispatch-disjoint")?;
    leases.push(store.claim_mission_supervision(
        blocker,
        "dispatch-blocker-owner",
        &swarm_engine::system_timestamp()?,
    )?);
    leases.push(store.claim_mission_supervision(
        work,
        "dispatch-target-owner",
        &swarm_engine::system_timestamp()?,
    )?);
    let older = store.sync_mission_workspace_queue(&leases[0])?;
    if let Some(error) = older.permission_refusal {
        return Err(Box::new(error));
    }
    require(
        older.ready.iter().any(|id| id == "task"),
        "actual blocker must be ready",
    )?;
    let configured = store.read_work(work)?;
    let task = configured
        .tasks
        .as_ref()
        .and_then(|tasks| tasks.iter().find(|task| task.id == "task"))
        .ok_or("retry task absent")?;
    require(
        task.planning_retry
            && task.launch_profile.as_ref().is_some_and(|profile| {
                Path::new(&profile.workspace) == root.join("dispatch-shared")
            }),
        "first contender requires a genuine saved shared profile and public retry",
    )?;
    let request = MissionWorkerDispatchRequest {
        schema_version: 1,
        event_id: "dispatch-selected-agent".into(),
        expected_revision: configured.revision,
    };
    let selected = store.run_mission_cycle(&leases[1], |store, lease, monitor| {
        if !monitor.is_running() {
            return Err(RootStoreError::Refused("fixture-heartbeat-not-running"));
        }
        store
            .dispatch_mission_worker(lease, &request, &mut SystemEnvironment)
            .map_err(|cause| {
                RootStoreError::Launch(Box::new(AgentLaunchError::Cause {
                    code: "fixture-dispatch-operation",
                    cause,
                }))
            })
    })?;
    let agent = match selected {
        Outcome::Prepared {
            queue,
            intention,
            resources,
        } => {
            require(
                queue.permission_refusal.is_none(),
                "unexpected dispatch permission refusal",
            )?;
            require(
                matches!(resources.first(),Some(Resource::Fifo{task,blocker:older}) if task=="task" && older.older_work==blocker),
                "first resource refusal is not the original older FIFO blocker",
            )?;
            require(
                (intention.agent.task_id) == ("task-2"),
                "selected task differs from the disjoint witness",
            )?;
            require(
                (intention.agent.id) == (request.event_id),
                "selected intention has the wrong operation identity",
            )?;
            require(
                (intention.agent.status) == ("queued"),
                "selected intention is not queued",
            )?;
            require(
                intention.created && !intention.preview_only,
                "expected a new actual intention rather than preview or replay",
            )?;
            intention.agent
        }
        _ => return Err("expected one genuinely prepared disjoint task".into()),
    };
    write_marker("PASS first FIFO-blocked contender yields to actual disjoint queued intention")?;
    let replay_before = snapshot(db)?;
    let replay = store.dispatch_mission_worker(&leases[1], &request, &mut SystemEnvironment)?;
    match replay {
        Outcome::Replayed {
            queue,
            intention,
            resources,
        } => {
            require(
                queue.is_none() && resources.is_empty(),
                "replay unexpectedly synchronized or retained new resources",
            )?;
            require(
                !intention.created && intention.agent == agent,
                "same event must reuse original exact Agent",
            )?;
        }
        _ => return Err("same event did not replay its original intention".into()),
    }
    require(
        (snapshot(db)?) == (replay_before),
        "replay must not synchronize or record another intention",
    )?;
    write_marker("PASS same dispatch identity replays without new intention or queue mutation")?;
    let manual = recorded_launch(db, "dispatch-prime-worker")?;
    require(
        manual.event_id == "dispatch-prime-worker" && manual.origin != "conducteur",
        "manual namespace witness is not the original public manual launch",
    )?;
    let manual_request = MissionWorkerDispatchRequest {
        schema_version: 1,
        event_id: manual.event_id,
        expected_revision: manual.expected_revision,
    };
    let revision_request = MissionWorkerDispatchRequest {
        expected_revision: request
            .expected_revision
            .checked_add(1)
            .ok_or("fixture revision cannot be incremented")?,
        ..request.clone()
    };
    let replay_refusals = vec![
        replay_refusal(store, &leases[1], &manual_request, db, "manual_namespace")?,
        replay_refusal(store, &leases[0], &request, db, "other_work")?,
        replay_refusal(
            store,
            &leases[1],
            &revision_request,
            db,
            "original_revision",
        )?,
    ];
    write_marker(
        "PASS manual foreign Work and changed original revision refuse before synchronization",
    )?;
    let target_held_before = store
        .mission_workspace_queue(work)?
        .turns
        .into_iter()
        .find(|turn| turn.agent == agent.id)
        .ok_or("target acquired turn absent before blocker cleanup")?;
    let blocker_pause = json!({"schema_version":1,"event_id":"dispatch-blocker-pause",
        "expected_revision":store.read_work(blocker)?.revision,"reason":"Owned blocker waiting cleanup before capacity witness"});
    store.execute_external(
        blocker,
        "mission.pause",
        &serde_json::to_vec(&blocker_pause)?,
        &mut SystemEnvironment,
    )?;
    let blocker_cleanup = store.sync_mission_workspace_queue(&leases[0])?;
    require(
        matches!(
            blocker_cleanup.permission_refusal,
            Some(swarm_engine::MissionSupervisionError::Refused(
                "planning-mission-paused"
            ))
        ),
        "blocker cleanup lacks the exact paused permission refusal",
    )?;
    require(
        blocker_cleanup.ready.is_empty(),
        "paused blocker retained ready candidates",
    )?;
    let capacity_view = store.mission_workspace_queue(work)?;
    require(
        !capacity_view
            .blockers
            .iter()
            .any(|value| value.task == "task"),
        "shared task remains FIFO blocked after its real older owner cleanup",
    )?;
    require(
        capacity_view
            .turns
            .iter()
            .find(|turn| turn.agent == agent.id)
            .ok_or("target acquired turn missing after blocker cleanup")?
            == &target_held_before,
        "blocker cleanup changed target acquired ownership",
    )?;
    let capacity_agents = store.list_agents(work)?;
    let capacity = MissionWorkerDispatchRequest {
        schema_version: 1,
        event_id: "dispatch-capacity-no-agent".into(),
        expected_revision: store.read_work(work)?.revision,
    };
    let denied = store.dispatch_mission_worker(&leases[1], &capacity, &mut SystemEnvironment)?;
    match denied {
        Outcome::NoAdmission { queue, resources } => {
            require(
                queue.permission_refusal.is_none(),
                "unexpected dispatch permission refusal",
            )?;
            require(
                queue.ready.iter().any(|id| id == "task"),
                "capacity witness has no expected ready task",
            )?;
            require(
                matches!(resources.last(),Some(Resource::Capacity{task,..}) if task=="task"),
                "capacity did not stop on the expected task",
            )?;
            require(
                (resources
                    .iter()
                    .filter(|value| matches!(value, Resource::Capacity { .. }))
                    .count())
                    == (1),
                "expected exactly one global capacity refusal",
            )?;
        }
        _ => return Err("occupied mission capacity admitted a second intention".into()),
    }
    require(
        store.list_agents(work)? == capacity_agents,
        "capacity stop must create no Agent",
    )?;
    write_marker("PASS occupied saved mission capacity stops the pass with an explicit refusal")?;
    let held_before = store
        .mission_workspace_queue(work)?
        .turns
        .into_iter()
        .find(|turn| turn.agent == agent.id)
        .ok_or("acquired intention turn absent")?;
    let pause = json!({"schema_version":1,"event_id":"dispatch-pause","expected_revision":store.read_work(work)?.revision,"reason":"Owned permission withdrawal"});
    store.execute_external(
        work,
        "mission.pause",
        &serde_json::to_vec(&pause)?,
        &mut SystemEnvironment,
    )?;
    let paused = MissionWorkerDispatchRequest {
        schema_version: 1,
        event_id: "dispatch-paused-no-agent".into(),
        expected_revision: store.read_work(work)?.revision,
    };
    match store.dispatch_mission_worker(&leases[1], &paused, &mut SystemEnvironment)? {
        Outcome::NoAdmission { queue, resources } => {
            require(
                queue.ready.is_empty() && resources.is_empty(),
                "paused dispatch retained unexpected ready tasks or resources",
            )?;
            require(
                matches!(
                    queue.permission_refusal,
                    Some(swarm_engine::MissionSupervisionError::Refused(
                        "planning-mission-paused"
                    ))
                ),
                "paused dispatch lacks its exact typed permission refusal",
            )?;
        }
        _ => return Err("paused dispatch did not retain explicit permission refusal".into()),
    }
    require(
        store.list_agents(work)? == capacity_agents,
        "pause cleanup must not create or change an Agent",
    )?;
    let after = store.mission_workspace_queue(work)?;
    require(
        (after
            .turns
            .iter()
            .find(|turn| turn.agent == agent.id)
            .ok_or("held turn missing after pause")?)
            == (&held_before),
        "permission cleanup changed acquired ownership",
    )?;
    require(
        after
            .turns
            .iter()
            .filter(|turn| turn.work == work && turn.agent.is_empty())
            .all(|turn| turn.state == "released"),
        "permission cleanup retained an unowned waiting turn",
    )?;
    require(
        !root.join("dispatch-shared/model-received.json").exists(),
        "dispatch unexpectedly ran a shared provider",
    )?;
    require(
        !root.join("dispatch-disjoint/model-received.json").exists(),
        "dispatch unexpectedly ran a disjoint provider",
    )?;
    write_marker("PASS permission withdrawal remains visible and keeps acquired ownership")?;
    Ok(
        json!({"status":"PASS","work":work,"blocker":blocker,"agent_id":agent.id,"task_id":agent.task_id,"turn":held_before,
        "disjoint_after_fifo":true,"same_identity_replay":true,"capacity_global_pass_stop":true,"permission_refusal_visible":true,
        "replay_refusals":replay_refusals,"clean_replay_refusal_count":3,
        "provider_started_by_dispatch":false,"paid_provider":false,"task_accepted":false}),
    )
}

/// Route the real private gate if requested, otherwise require exact owned input
/// marker. Close every opened lease, Root and SQL connection on success/failure;
/// publish a fresh fsynced result only after cleanup. No installed service changes.
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
        return Err("owned dispatch input required".into());
    }
    let input: Json = serde_json::from_slice(&fs::read(&args[1])?)?;
    let root = Path::new(input["root"].as_str().ok_or("root absent")?).canonicalize()?;
    let marker: Json =
        serde_json::from_slice(&fs::read(root.join(".swarm/verification-dispatch.json"))?)?;
    if marker != input || input["kind"] != "owned-inert-dispatch-fixture" {
        return Err("owned marker mismatch".into());
    }
    let work = input["work"].as_str().ok_or("work absent")?;
    let blocker = input["blocker"].as_str().ok_or("blocker absent")?;
    let mut store = RootStore::open(&root, true)?;
    let db = match Connection::open(root.join(".swarm/state.db")) {
        Ok(db) => db,
        Err(error) => return finish(Err(Box::new(error)), store.close().map_err(Into::into)),
    };
    let mut leases = Vec::new();
    let result = exercise(&mut store, &mut leases, &db, &root, work, blocker);
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
