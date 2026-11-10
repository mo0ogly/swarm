//! Owned W2 fixture: public mission start/claim/dispatch leaves one real queued
//! v4 worker, then stops/closes the conductor without spawn, cancellation or refund.
//! The fixed worker profile/workspace/slots are explicit inert fixture inputs,
//! persisted by mission.start; they are not product defaults or new policies.
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt, fs,
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::PathBuf,
};
use swarm_engine::{
    MissionLease, MissionWorkerDispatchOutcome as Dispatch, RootStore, RootStoreError as E,
    SystemEnvironment,
};

/// Static fixture phase with typed source; never render a private cause message.
struct Phase {
    code: &'static str,
    cause: Box<dyn Error + Send + Sync>,
}
impl fmt::Debug for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl Error for Phase {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}
/// Preserve a concrete failure through the public runtime constructor.
fn phase(code: &'static str, cause: impl Error + Send + Sync + 'static) -> E {
    E::Runtime(swarm_engine::RuntimeError::new(
        swarm_engine::RuntimeOperation::Platform,
        Phase {
            code,
            cause: Box::new(cause),
        },
    ))
}
/// Combine independent action/cleanup errors without claiming rollback.
fn join(primary: Option<E>, cleanup: E) -> E {
    match primary {
        None => cleanup,
        Some(primary) => E::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        },
    }
}
/// Fixture precondition failure, never an inferred permission or successful skip.
fn require(valid: bool, code: &'static str) -> Result<(), E> {
    if valid {
        Ok(())
    } else {
        Err(E::Refused(code))
    }
}
/// Safe native error tree for the external SDK consumer. Every known compound
/// branch and numeric IO/SQLite code is retained; private Display is never used.
struct Failure(E);
impl fmt::Debug for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pending: Vec<&(dyn Error + 'static)> = vec![&self.0];
        let mut seen: Vec<*const dyn Error> = Vec::new();
        let mut records = Vec::new();
        while let Some(error) = pending.pop() {
            let pointer = error as *const dyn Error;
            if seen.iter().any(|prior| std::ptr::eq(*prior, pointer)) {
                continue;
            }
            seen.push(pointer);
            let links = swarm_engine::native_error_links(error);
            let phase = error.downcast_ref::<Phase>().map(|phase| phase.code);
            records.push(json!({"phase":phase,"labels":links.labels,"io_errno":links.io_errno,"sqlite_code":links.sqlite_code}));
            pending.extend(links.children.into_iter().rev().map(|(_, child)| child));
        }
        f.debug_struct("RestartSeedFailure")
            .field("diagnostics", &records)
            .finish()
    }
}

/// Explicit marker-correlated fixture coordinates, never datastore authority.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: u8,
    kind: String,
    root: PathBuf,
    work: String,
    result: PathBuf,
}
impl Input {
    /// Require the approved env/JSON/marker, canonical private owned Root and
    /// fresh owned output parent. Missing or malformed input fails before opening
    /// Root; no default Root/work/output or implicit initialization is allowed.
    fn read() -> Result<Self, E> {
        let path = std::env::var_os("SWARM_MISSION_WATCH_RESTART_SEED")
            .ok_or(E::Refused("restart-seed-input-required"))?;
        let raw: Value = serde_json::from_slice(
            &fs::read(PathBuf::from(path)).map_err(|e| phase("restart-seed-input-read", e))?,
        )
        .map_err(|e| phase("restart-seed-input-json", e))?;
        let input: Self = serde_json::from_value(raw.clone())
            .map_err(|e| phase("restart-seed-input-fields", e))?;
        require(
            input.schema_version == 1 && input.kind == "owned-inert-mission-watch-restart-seed",
            "restart-seed-kind",
        )?;
        require(
            input.root.is_absolute()
                && input
                    .root
                    .canonicalize()
                    .map_err(|e| phase("restart-seed-root-canonical", e))?
                    == input.root,
            "restart-seed-root-absolute-canonical",
        )?;
        let meta = fs::symlink_metadata(&input.root)
            .map_err(|e| phase("restart-seed-root-metadata", e))?;
        require(
            meta.is_dir()
                && meta.uid() == rustix::process::geteuid().as_raw()
                && meta.mode() & 0o777 == 0o700,
            "restart-seed-private-owned-root",
        )?;
        let marker: Value = serde_json::from_slice(
            &fs::read(
                input
                    .root
                    .join(".swarm/verification-mission-watch-restart-seed.json"),
            )
            .map_err(|e| phase("restart-seed-marker-read", e))?,
        )
        .map_err(|e| phase("restart-seed-marker-json", e))?;
        require(
            marker == raw && !input.work.is_empty(),
            "restart-seed-marker-mismatch",
        )?;
        require(
            input.result.is_absolute()
                && input
                    .result
                    .symlink_metadata()
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
            "restart-seed-output-not-fresh",
        )?;
        let parent = input
            .result
            .parent()
            .ok_or(E::Refused("restart-seed-output-parent"))?;
        let meta =
            fs::symlink_metadata(parent).map_err(|e| phase("restart-seed-output-metadata", e))?;
        require(
            meta.is_dir()
                && meta.uid() == rustix::process::geteuid().as_raw()
                && parent
                    .canonicalize()
                    .map_err(|e| phase("restart-seed-output-canonical", e))?
                    == parent,
            "restart-seed-output-owned-parent",
        )?;
        Ok(input)
    }
}

/// Public projections only, compared exactly in memory across conductor stop.
/// Budget is aggregate observation, not proof of the individual reservation rows.
#[derive(PartialEq)]
struct Snapshot {
    agent: Vec<u8>,
    work: Vec<u8>,
    queue: swarm_engine::MissionWorkspaceQueue,
    receipts: Vec<swarm_engine::MissionWorkReceipt>,
    budget: Option<swarm_engine::WorkBudgetState>,
}
impl Snapshot {
    /// Read independent public snapshots; no external transaction/private SQL is
    /// opened, no projection is substituted for a capability or reservation ledger.
    fn read(store: &RootStore, work: &str, agent: &str) -> Result<Self, E> {
        Ok(Self {
            agent: serde_json::to_vec(&store.read_agent(agent)?)
                .map_err(|e| phase("restart-seed-agent-encode", e))?,
            work: serde_json::to_vec(&store.read_work(work)?)
                .map_err(|e| phase("restart-seed-work-encode", e))?,
            queue: store
                .mission_workspace_queue(work)
                .map_err(|e| phase("restart-seed-queue", e))?,
            receipts: store
                .read_mission_work_receipts(work)
                .map_err(|e| phase("restart-seed-receipts", e))?,
            budget: store.work_budget_state(work)?,
        })
    }
    /// Safe fixture coordinates/hashes/queue and aggregate budget; no prompts,
    /// private Work/Agent bodies, tokens or process identity stamps are published.
    fn document(&self) -> Value {
        json!({"agent_sha256":format!("{:x}",Sha256::digest(&self.agent)),
            "work_sha256":format!("{:x}",Sha256::digest(&self.work)),"queue":self.queue,
            "receipt_count":self.receipts.len(),"budget_aggregate":self.budget,"reservation_rows_verified":false})
    }
}
/// Reject unexpected dispatch while retaining every typed queue/resource cause.
/// A successful admission/replay remains known even when this narrow seed refuses.
fn unexpected_dispatch(outcome: Dispatch) -> E {
    let (queue, resources) = match outcome {
        Dispatch::Prepared {
            queue, resources, ..
        }
        | Dispatch::NoAdmission { queue, resources } => (Some(queue), resources),
        Dispatch::Replayed {
            queue, resources, ..
        } => (queue, resources),
    };
    let mut error = E::Refused("restart-seed-dispatch-not-prepared");
    if let Some(queue) = queue {
        if let Some(cause) = queue.permission_refusal {
            error = join(Some(error), phase("restart-seed-permission", cause));
        }
        for refusal in queue.refused {
            error = join(Some(error), phase("restart-seed-candidate", refusal.error));
        }
    }
    for resource in resources {
        match resource {
            swarm_engine::MissionDispatchResourceRefusal::Capacity { cause, .. }
            | swarm_engine::MissionDispatchResourceRefusal::Occupied { cause, .. } => {
                error = join(Some(error), cause)
            }
            swarm_engine::MissionDispatchResourceRefusal::Fifo { .. } => {}
        }
    }
    error
}
/// Start the explicit fixture profile, acquire and admit one real queued worker.
/// No worker process, role orchestration, cancellation or reservation edit occurs.
fn prepare(
    store: &mut RootStore,
    input: &Input,
    lease: &mut Option<MissionLease>,
    admission: &mut bool,
    retained: &mut Option<(String, Snapshot)>,
) -> Result<Value, E> {
    require(
        store.list_agents(&input.work)?.is_empty(),
        "restart-seed-work-not-fresh",
    )?;
    let budget = store
        .work_budget(&input.work)?
        .ok_or(E::Refused("restart-seed-positive-budget-required"))?;
    require(
        budget.reserve_per_launch_usd.0.is_finite() && budget.reserve_per_launch_usd.0 > 0.0,
        "restart-seed-positive-budget-required",
    )?;
    fs::create_dir(input.root.join("watch-worker"))
        .map_err(|e| phase("restart-seed-workspace-create", e))?;
    let profile = json!({"provider":"fixture","role":"worker","workspace":"watch-worker","level":"standard","capture_output":true});
    let start = json!({"schema_version":1,"event_id":"restart-seed-start","expected_revision":store.read_work(&input.work)?.revision,
        "reason":"Owned restart fixture; explicit inert worker profile and one slot","slots":1,"profile":profile});
    store.execute_external(
        &input.work,
        "mission.start",
        &serde_json::to_vec(&start).map_err(|e| phase("restart-seed-start-json", e))?,
        &mut SystemEnvironment,
    )?;
    *lease = Some(
        store
            .claim_mission_supervision(
                &input.work,
                "Owned queued restart seed",
                &swarm_engine::system_timestamp().map_err(|e| phase("restart-seed-clock", e))?,
            )
            .map_err(|e| phase("restart-seed-claim", e))?,
    );
    let owned = lease
        .as_ref()
        .ok_or(E::Refused("restart-seed-lease-missing"))?;
    let request = swarm_engine::MissionWorkerDispatchRequest {
        schema_version: 1,
        event_id: "restart-seed-agent".into(),
        expected_revision: store.read_work(&input.work)?.revision,
    };
    let outcome = store
        .dispatch_mission_worker(owned, &request, &mut SystemEnvironment)
        .map_err(|e| phase("restart-seed-dispatch", *e))?;
    *admission = matches!(
        &outcome,
        Dispatch::Prepared { .. } | Dispatch::Replayed { .. }
    );
    let intention = match outcome {
        Dispatch::Prepared {
            intention,
            queue,
            resources,
        } if queue.permission_refusal.is_none()
            && queue.refused.is_empty()
            && resources.is_empty() =>
        {
            intention
        }
        other => return Err(unexpected_dispatch(other)),
    };
    let agent = &intention.agent;
    require(
        intention.created
            && !intention.preview_only
            && agent.status == "queued"
            && agent.desired.is_empty()
            && agent.supervisor_pid == 0
            && agent.child_pid == 0
            && agent.native_execution.is_none()
            && agent.native_execution_result.is_none(),
        "restart-seed-not-unregistered-queued",
    )?;
    let binding: Value = serde_json::from_str(
        agent
            .native_mission
            .as_ref()
            .ok_or(E::Refused("restart-seed-binding-missing"))?
            .0
            .get(),
    )
    .map_err(|e| phase("restart-seed-binding-json", e))?;
    require(
        binding["schema_version"] == 4 && binding["generation"] == owned.generation(),
        "restart-seed-binding4-required",
    )?;
    let observed = store
        .observe_mission_worker_continuation(owned)
        .map_err(|e| phase("restart-seed-observe", *e))?;
    require(
        observed.entries.len() == 1
            && observed.entries[0].agent_id == agent.id
            && matches!(&observed.entries[0].state,
        swarm_engine::MissionWorkerContinuationState::QueuedUnregisteredObserved { generation,current_lease_generation:true } if *generation==owned.generation()),
        "restart-seed-original-queued-not-proved",
    )?;
    let snapshot = Snapshot::read(store, &input.work, &agent.id)?;
    require(
        snapshot
            .queue
            .turns
            .iter()
            .filter(|turn| {
                turn.agent == agent.id
                    && turn.work == input.work
                    && turn.state == "acquired"
                    && binding["workspace_turn_seq"] == turn.seq
            })
            .count()
            == 1,
        "restart-seed-acquired-turn-required",
    )?;
    let document = json!({"schema_version":1,"status":"QUEUED_RESTART_SEED","work":input.work,"agent_id":agent.id,"task_id":agent.task_id,"attempt_id":agent.attempt_id,
        "binding":binding,"generation":owned.generation(),"before_conductor_stop":snapshot.document(),"fixture_profile":profile,"fixture_slots":1,
        "provider_spawn_called":false,"agent_cancelled":false,"consumption_reset":false,"observation_is_permission":false,"reservation_rows_verified":false});
    *retained = Some((agent.id.clone(), snapshot));
    Ok(document)
}
/// Create-only durable fixture output. Failure never overwrites an original;
/// partial publication is uncertain and makes the SDK fail, not an empty success.
fn publish(path: &std::path::Path, document: &Value) -> Result<(), E> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| phase("restart-seed-output-create", e))?;
    let bytes =
        serde_json::to_vec_pretty(document).map_err(|e| phase("restart-seed-output-json", e))?;
    file.write_all(&bytes)
        .and_then(|()| file.write_all(b"\n"))
        .and_then(|()| file.flush())
        .and_then(|()| file.sync_all())
        .map_err(|e| phase("restart-seed-output-write-sync", e))?;
    fs::File::open(
        path.parent()
            .ok_or(E::Refused("restart-seed-output-parent"))?,
    )
    .and_then(|dir| dir.sync_all())
    .map_err(|e| phase("restart-seed-output-parent-sync", e))
}
/// Retain action plus every lease stop/close/Root close cause. No Agent is
/// reconciled or signalled: the real queued intention is the next watch's input.
fn run(input: &Input) -> Result<(), E> {
    let mut store = RootStore::open(&input.root, true)?;
    let mut lease = None;
    let mut admission = false;
    let mut retained = None;
    let action = prepare(&mut store, input, &mut lease, &mut admission, &mut retained);
    let (mut document, mut failure) = match action {
        Ok(value) => (Some(value), None),
        Err(error) => (None, Some(error)),
    };
    if let Some(owned) = lease {
        match swarm_engine::system_timestamp() {
            Ok(now) => {
                if let Err(error) = store.update_mission_supervision(&owned, &now, true) {
                    failure = Some(join(failure, phase("restart-seed-stop-lease", error)));
                }
            }
            Err(error) => failure = Some(join(failure, phase("restart-seed-cleanup-clock", error))),
        }
        if let Some((agent, before)) = &retained {
            match Snapshot::read(&store, &input.work, agent) {
                Ok(after) => {
                    if let Err(error) = require(
                        before == &after,
                        "restart-seed-stop-mutated-worker-projections",
                    ) {
                        failure = Some(join(failure, error));
                    }
                    if let Some(document) = &mut document {
                        document["after_conductor_stop"] = after.document();
                        document["worker_projections_unchanged"] = json!(before == &after);
                    }
                }
                Err(error) => failure = Some(join(failure, error)),
            }
        }
        if let Err(error) = owned.close() {
            failure = Some(join(failure, phase("restart-seed-close-lease", error)));
        }
    }
    if let Err(error) = store.close() {
        failure = Some(join(failure, error));
    }
    let result = match failure {
        Some(error) => Err(error),
        None => {
            let mut document = document.ok_or(E::Refused("restart-seed-result-missing"))?;
            document["lease_released"] = json!(true);
            document["lease_closed"] = json!(true);
            document["root_closed"] = json!(true);
            publish(&input.result, &document)
        }
    };
    result.map_err(|error| {
        if admission {
            E::LifecycleRecorded {
                primary: Box::new(error),
            }
        } else {
            error
        }
    })
}
/// Missing/malformed input is a real error; no positive skip or default fixture.
fn main() -> Result<(), Failure> {
    let input = Input::read().map_err(Failure)?;
    run(&input).map_err(Failure)
}
