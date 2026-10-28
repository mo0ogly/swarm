//! W2 owned fixture: real adoption refusals, one positive adoption and exact
//! replay/recovery through public APIs only. No SQL, process spawn or refund.
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
    MissionLease, MissionWorkerAdoptionRequest as Request, RootStore, RootStoreError as E,
    SystemEnvironment,
};

/// Safe phase context retains the concrete source without private formatting.
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
/// Wrap typed errors via the existing public runtime constructor.
fn phase(code: &'static str, cause: impl Error + Send + Sync + 'static) -> E {
    E::Runtime(swarm_engine::RuntimeError::new(
        swarm_engine::RuntimeOperation::Platform,
        Phase {
            code,
            cause: Box::new(cause),
        },
    ))
}
/// Preserve independent operation/cleanup failures without asserting rollback.
fn join(primary: Option<E>, cleanup: E) -> E {
    match primary {
        None => cleanup,
        Some(primary) => E::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        },
    }
}
/// Fixture assertions refuse explicitly; no missing input becomes a skipped PASS.
fn require(valid: bool, code: &'static str) -> Result<(), E> {
    if valid {
        Ok(())
    } else {
        Err(E::Refused(code))
    }
}
/// Project all known native branches and safe local phases, never private Display.
/// Pointer identity terminates cycles without an arbitrary depth/count policy.
fn diagnostics(error: &(dyn Error + 'static)) -> Vec<Value> {
    let mut pending = vec![error];
    let mut seen: Vec<*const dyn Error> = Vec::new();
    let mut result = Vec::new();
    while let Some(error) = pending.pop() {
        let pointer = error as *const dyn Error;
        if seen.iter().any(|p| std::ptr::eq(*p, pointer)) {
            continue;
        }
        seen.push(pointer);
        let links = swarm_engine::native_error_links(error);
        result.push(json!({"phase":error.downcast_ref::<Phase>().map(|p|p.code),"labels":links.labels,"io_errno":links.io_errno,"sqlite_code":links.sqlite_code}));
        pending.extend(links.children.into_iter().rev().map(|(_, child)| child));
    }
    result
}
/// External SDK failure keeps all safe causes, including secondary cleanup.
struct Failure(E);
impl fmt::Debug for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdoptionChecksFailed")
            .field("diagnostics", &diagnostics(&self.0))
            .finish()
    }
}

/// Strict fixture coordinates, correlated by a marker, never product authority.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: u8,
    kind: String,
    root: PathBuf,
    work: String,
    agent_id: String,
    result: PathBuf,
}
impl Input {
    /// Validate env/JSON/marker and canonical owned private Root/fresh output
    /// before public Root open. No fallback fixture or implicit initialization.
    fn read() -> Result<Self, E> {
        let path = std::env::var_os("SWARM_MISSION_ADOPTION_REFUSALS")
            .ok_or(E::Refused("adoption-check-input-required"))?;
        let raw: Value = serde_json::from_slice(
            &fs::read(PathBuf::from(path)).map_err(|e| phase("adoption-check-input-read", e))?,
        )
        .map_err(|e| phase("adoption-check-input-json", e))?;
        let input: Self = serde_json::from_value(raw.clone())
            .map_err(|e| phase("adoption-check-input-fields", e))?;
        require(
            input.schema_version == 1 && input.kind == "owned-inert-mission-adoption-refusals",
            "adoption-check-kind",
        )?;
        require(
            input.root.is_absolute()
                && input
                    .root
                    .canonicalize()
                    .map_err(|e| phase("adoption-check-root-canonical", e))?
                    == input.root,
            "adoption-check-root-canonical-required",
        )?;
        let meta = fs::symlink_metadata(&input.root)
            .map_err(|e| phase("adoption-check-root-metadata", e))?;
        require(
            meta.is_dir()
                && meta.uid() == rustix::process::geteuid().as_raw()
                && meta.mode() & 0o777 == 0o700,
            "adoption-check-root-private-owned",
        )?;
        let marker: Value = serde_json::from_slice(
            &fs::read(
                input
                    .root
                    .join(".swarm/verification-mission-adoption-refusals.json"),
            )
            .map_err(|e| phase("adoption-check-marker-read", e))?,
        )
        .map_err(|e| phase("adoption-check-marker-json", e))?;
        require(
            marker == raw && !input.work.is_empty() && !input.agent_id.is_empty(),
            "adoption-check-marker-mismatch",
        )?;
        require(
            input.result.is_absolute()
                && input
                    .result
                    .symlink_metadata()
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
            "adoption-check-output-not-fresh",
        )?;
        let parent = input
            .result
            .parent()
            .ok_or(E::Refused("adoption-check-output-parent"))?;
        let meta =
            fs::symlink_metadata(parent).map_err(|e| phase("adoption-check-parent-metadata", e))?;
        require(
            meta.is_dir()
                && meta.uid() == rustix::process::geteuid().as_raw()
                && parent
                    .canonicalize()
                    .map_err(|e| phase("adoption-check-parent-canonical", e))?
                    == parent,
            "adoption-check-parent-owned-canonical",
        )?;
        Ok(input)
    }
}
/// Exact public projections around EACH API operation. Aggregate budget is not
/// an individual reservation ledger; the external readonly harness owns that proof.
#[derive(PartialEq)]
struct Snapshot {
    agent: Vec<u8>,
    work: Vec<u8>,
    controls: Vec<u8>,
    queue: swarm_engine::MissionWorkspaceQueue,
    receipts: Vec<swarm_engine::MissionWorkReceipt>,
    budget: Option<swarm_engine::WorkBudgetState>,
}
impl Snapshot {
    /// Read sequential public views without an external transaction or SQL access.
    fn read(store: &RootStore, input: &Input) -> Result<Self, E> {
        Ok(Self {
            agent: serde_json::to_vec(&store.read_agent(&input.agent_id)?)
                .map_err(|e| phase("adoption-check-agent-json", e))?,
            work: serde_json::to_vec(&store.read_work(&input.work)?)
                .map_err(|e| phase("adoption-check-work-json", e))?,
            controls: serde_json::to_vec(
                &store
                    .mission_control_view(&input.work)
                    .map_err(|e| phase("adoption-check-controls", e))?,
            )
            .map_err(|e| phase("adoption-check-controls-json", e))?,
            queue: store
                .mission_workspace_queue(&input.work)
                .map_err(|e| phase("adoption-check-queue", e))?,
            receipts: store
                .read_mission_work_receipts(&input.work)
                .map_err(|e| phase("adoption-check-receipts", e))?,
            budget: store.work_budget_state(&input.work)?,
        })
    }
    /// Emit hashes and public counts only; raw Agent/Work/prompt remains private.
    fn document(&self) -> Value {
        json!({"agent_sha256":format!("{:x}",Sha256::digest(&self.agent)),"work_sha256":format!("{:x}",Sha256::digest(&self.work)),
        "controls_sha256":format!("{:x}",Sha256::digest(&self.controls)),"queue_turn_count":self.queue.turns.len(),"receipt_count":self.receipts.len(),"budget_aggregate":self.budget,"reservation_rows_verified":false})
    }
}
/// Witness actual queued v4 under a present new lease before fault variants.
/// This calls the real observer and validates shape only, never fabricates adoption.
fn witness(store: &RootStore, input: &Input, lease: &MissionLease) -> Result<(), E> {
    store
        .update_mission_supervision(
            lease,
            &swarm_engine::system_timestamp().map_err(|e| phase("adoption-check-clock", e))?,
            false,
        )
        .map_err(|e| phase("adoption-check-heartbeat", e))?;
    let agent = store.read_agent(&input.agent_id)?;
    require(
        agent.status == "queued"
            && agent.supervisor_pid == 0
            && agent.child_pid == 0
            && agent.desired.is_empty(),
        "adoption-check-real-queued-required",
    )?;
    let binding: Value = serde_json::from_str(
        agent
            .native_mission
            .as_ref()
            .ok_or(E::Refused("adoption-check-binding-required"))?
            .0
            .get(),
    )
    .map_err(|e| phase("adoption-check-binding-json", e))?;
    require(
        binding["schema_version"] == 4,
        "adoption-check-binding4-required",
    )?;
    let observation = store
        .observe_mission_worker_continuation(lease)
        .map_err(|e| phase("adoption-check-observe", *e))?;
    require(
        observation.entries.len() == 1
            && observation.entries[0].agent_id == input.agent_id
            && matches!(
                &observation.entries[0].state,
                swarm_engine::MissionWorkerContinuationState::QueuedUnregisteredObserved { .. }
            ),
        "adoption-check-queued-observation-required",
    )?;
    Ok(())
}
/// A canonical current-revision request is the valid transport witness. Its
/// expected revision may be deliberately changed only by a named variant below.
fn request(store: &RootStore, input: &Input, event: &str) -> Result<Request, E> {
    let request = Request {
        schema_version: 1,
        event_id: event.into(),
        expected_revision: store.read_work(&input.work)?.revision,
        agent_id: input.agent_id.clone(),
    };
    request
        .validate()
        .map_err(|e| phase("adoption-check-request", e))?;
    Ok(request)
}
/// Expect a real typed refusal and unchanged public projections, reading after
/// even if the API unexpectedly succeeds. Original API and read errors survive.
fn refused(
    store: &RootStore,
    input: &Input,
    lease: &MissionLease,
    request: &Request,
    expected: &'static str,
    name: &'static str,
    adopted: &mut bool,
) -> Result<Value, E> {
    request
        .validate()
        .map_err(|e| phase("adoption-check-variant-shape", e))?;
    let before = Snapshot::read(store, input)?;
    let outcome = store.adopt_mission_worker(lease, request);
    let after = Snapshot::read(store, input);
    let cause = match (outcome, after) {
        (Err(cause), Ok(after)) => {
            if before != after {
                return Err(join(
                    Some(phase("adoption-check-original-refusal", cause)),
                    E::Refused("adoption-check-refusal-mutated-projections"),
                ));
            }
            cause
        }
        (Err(cause), Err(read)) => {
            return Err(join(
                Some(phase("adoption-check-original-refusal", cause)),
                read,
            ))
        }
        (Ok(_), after) => {
            *adopted = true;
            return Err(join(
                after.err(),
                E::Refused("adoption-check-expected-refusal-got-adoption"),
            ));
        }
    };
    let codes = diagnostics(&cause);
    if !codes.iter().any(|node| {
        node["labels"]
            .as_array()
            .is_some_and(|labels| labels.iter().any(|label| label == expected))
    }) {
        return Err(join(
            Some(phase("adoption-check-original-refusal", cause)),
            E::Refused("adoption-check-wrong-refusal"),
        ));
    }
    Ok(
        json!({"name":name,"verdict":"REFUSED","expected_code":expected,"cause_codes":codes,"public_projections_unchanged":true,"before":before.document(),"after":before.document(),"reservation_rows_verified":false}),
    )
}
/// Expect positive or exact replay on the real API without changing original
/// Agent/Work/queue/budget/receipts. Only the append-only adoption journal may grow.
fn accepted(
    store: &RootStore,
    input: &Input,
    lease: &MissionLease,
    request: &Request,
    created: bool,
    name: &'static str,
    adopted: &mut bool,
) -> Result<Value, E> {
    let before = Snapshot::read(store, input)?;
    let outcome = store
        .adopt_mission_worker(lease, request)
        .map_err(|e| phase("adoption-check-positive", e))?;
    *adopted = true;
    let after = Snapshot::read(store, input)?;
    require(before == after, "adoption-check-positive-mutated-originals")?;
    require(
        outcome.agent.id == input.agent_id
            && outcome.created == created
            && outcome.event_id == request.event_id
            && outcome.generation == lease.generation(),
        "adoption-check-positive-result",
    )?;
    Ok(
        json!({"name":name,"verdict":"ADOPTED","created":outcome.created,"event_id":outcome.event_id,"generation":outcome.generation,"public_projections_unchanged":true,"before":before.document(),"after":after.document(),"reservation_rows_verified":false}),
    )
}
/// Apply a real audited pause/unpause using its actual current Work revision.
/// These intentional mutations are setup, never attributed to the refused API.
fn control(store: &mut RootStore, input: &Input, operation: &str, event: &str) -> Result<(), E> {
    let request = json!({"schema_version":1,"event_id":event,"expected_revision":store.read_work(&input.work)?.revision,"reason":"Owned inert real adoption refusal fixture"});
    store.execute_external(
        &input.work,
        operation,
        &serde_json::to_vec(&request).map_err(|e| phase("adoption-check-control-json", e))?,
        &mut SystemEnvironment,
    )?;
    Ok(())
}
/// Execute concrete refusal/replay operations. One adoption receipt is left for
/// the next watch's new epoch; no child, cancellation, reset or refund occurs.
fn exercise(
    store: &mut RootStore,
    input: &Input,
    lease: &MissionLease,
    stop_attempted: &mut bool,
    adopted: &mut bool,
) -> Result<Value, E> {
    let mut checks = Vec::new();
    witness(store, input, lease)?;
    let mut stale = request(store, input, "adoption-check-stale")?;
    require(
        stale.expected_revision > 1,
        "adoption-check-stale-positive-revision-required",
    )?;
    stale.expected_revision -= 1;
    checks.push(refused(
        store,
        input,
        lease,
        &stale,
        "revision_conflict",
        "stale-revision",
        adopted,
    )?);
    witness(store, input, lease)?;
    control(store, input, "mission.pause", "adoption-check-pause")?;
    let paused = request(store, input, "adoption-check-paused")?;
    checks.push(refused(
        store,
        input,
        lease,
        &paused,
        "planning-mission-paused",
        "paused-permission",
        adopted,
    )?);
    control(store, input, "mission.unpause", "adoption-check-unpause")?;
    witness(store, input, lease)?;
    let exact = request(store, input, "adoption-check-positive")?;
    checks.push(accepted(
        store,
        input,
        lease,
        &exact,
        true,
        "positive-adoption",
        adopted,
    )?);
    witness(store, input, lease)?;
    checks.push(accepted(
        store,
        input,
        lease,
        &exact,
        false,
        "exact-replay",
        adopted,
    )?);
    let mut conflict = exact.clone();
    require(
        conflict.expected_revision > 1,
        "adoption-check-conflict-positive-revision-required",
    )?;
    conflict.expected_revision -= 1;
    checks.push(refused(
        store,
        input,
        lease,
        &conflict,
        "mission-adoption-replay-conflict",
        "replay-conflict",
        adopted,
    )?);
    witness(store, input, lease)?;
    checks.push(accepted(
        store,
        input,
        lease,
        &exact,
        false,
        "recovery-replay",
        adopted,
    )?);
    let new_event = request(store, input, "adoption-check-same-epoch")?;
    checks.push(refused(
        store,
        input,
        lease,
        &new_event,
        "mission-adoption-new-lease-required",
        "same-epoch-new-event",
        adopted,
    )?);
    witness(store, input, lease)?;
    // Retain a failed/uncertain stop without retrying it unchanged in cleanup.
    *stop_attempted = true;
    store
        .update_mission_supervision(
            lease,
            &swarm_engine::system_timestamp().map_err(|e| phase("adoption-check-stop-clock", e))?,
            true,
        )
        .map_err(|e| phase("adoption-check-stop-lease", e))?;
    checks.push(refused(
        store,
        input,
        lease,
        &exact,
        "mission-lease-lost",
        "stopped-lease",
        adopted,
    )?);
    Ok(
        json!({"schema_version":1,"status":"ADOPTION_REFUSALS_VERIFIED","work":input.work,"agent_id":input.agent_id,
        "adoption_event_id":exact.event_id,"generation":lease.generation(),"created":true,"exact_replay_created":false,"recovery_replay_created":false,
        "checks":checks,"reservation_rows_verified":false,"provider_spawn_called":false,"agent_cancelled":false,"consumption_reset":false,"observation_is_permission":false}),
    )
}
/// Create-only private output, flush/sync file AND parent; uncertainty remains an
/// error and never overwrites an existing original or becomes a positive result.
fn publish(input: &Input, value: &Value) -> Result<(), E> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&input.result)
        .map_err(|e| phase("adoption-check-output-create", e))?;
    file.write_all(
        &serde_json::to_vec_pretty(value).map_err(|e| phase("adoption-check-output-json", e))?,
    )
    .and_then(|()| file.write_all(b"\n"))
    .and_then(|()| file.flush())
    .and_then(|()| file.sync_all())
    .map_err(|e| phase("adoption-check-output-write-sync", e))?;
    fs::File::open(
        input
            .result
            .parent()
            .ok_or(E::Refused("adoption-check-output-parent"))?,
    )
    .and_then(|dir| dir.sync_all())
    .map_err(|e| phase("adoption-check-output-parent-sync", e))
}
/// Always close the actual lease/Root, retain independent cleanup causes, leave
/// the genuine queued Agent intact. An attempted stop is never repeated on an
/// unchanged failed/uncertain precondition; close errors remain independent.
fn run(input: &Input) -> Result<(), E> {
    let mut store = RootStore::open(&input.root, true)?;
    let mut lease = None;
    let mut stop_attempted = false;
    let mut adopted = false;
    let action = (|| {
        lease = Some(
            store
                .claim_mission_supervision(
                    &input.work,
                    "Owned adoption checks",
                    &swarm_engine::system_timestamp()
                        .map_err(|e| phase("adoption-check-claim-clock", e))?,
                )
                .map_err(|e| phase("adoption-check-claim", e))?,
        );
        exercise(
            &mut store,
            input,
            lease
                .as_ref()
                .ok_or(E::Refused("adoption-check-lease-missing"))?,
            &mut stop_attempted,
            &mut adopted,
        )
    })();
    let (mut value, mut failure) = match action {
        Ok(value) => (Some(value), None),
        Err(error) => (None, Some(error)),
    };
    if let Some(owned) = lease {
        if !stop_attempted {
            match swarm_engine::system_timestamp() {
                Ok(now) => {
                    if let Err(error) = store.update_mission_supervision(&owned, &now, true) {
                        failure = Some(join(failure, phase("adoption-check-cleanup-stop", error)));
                    }
                }
                Err(error) => {
                    failure = Some(join(failure, phase("adoption-check-cleanup-clock", error)))
                }
            }
        }
        if let Err(error) = owned.close() {
            failure = Some(join(failure, phase("adoption-check-close-lease", error)));
        }
    }
    if let Err(error) = store.close() {
        failure = Some(join(failure, error));
    }
    let result = match failure {
        Some(error) => Err(error),
        None => match value.take() {
            Some(mut value) => {
                value["lease_released"] = json!(true);
                value["lease_closed"] = json!(true);
                value["root_closed"] = json!(true);
                publish(input, &value)
            }
            None => Err(E::Refused("adoption-check-result-missing")),
        },
    };
    result.map_err(|error| {
        if adopted {
            E::LifecycleRecorded {
                primary: Box::new(error),
            }
        } else {
            error
        }
    })
}
/// Explicit fixture input and real failures; no provider or default fixture path.
fn main() -> Result<(), Failure> {
    let input = Input::read().map_err(Failure)?;
    run(&input).map_err(Failure)
}
