//! Owned fixture: real dispatch/heartbeat, safe private CLI session bootstrap,
//! then verified independent provider entry before conductor exit/signal.
//! A true queued continuation is observed without changing public Agent/Work,
//! receipt or FIFO projections before any probe/spawn. Reservation rows are not
//! exposed by the SDK API and are not claimed verified.
//! This is not the product `mission watch` command and invokes no paid provider.
#![forbid(unsafe_code)]
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, BufRead, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};
use swarm_engine::{
    MissionLease, MissionWorkerContinuationObservation,
    MissionWorkerContinuationState as ContinuationState, MissionWorkerDispatchOutcome as Dispatch,
    MissionWorkerDispatchRequest, RootBinding, RootStore, RootStoreError as E, RuntimeError,
    RuntimeOperation, SystemEnvironment,
};
type Result<T> = std::result::Result<T, E>;

/// Preserve independent operation, stop, close and publication failures. This
/// aggregate makes no assertion that any failed action had no durable effects.
struct Failures(Vec<Box<dyn Error>>);
impl fmt::Debug for Failures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Failures")
            .field(
                "safe_causes",
                &self
                    .0
                    .iter()
                    .map(|error| causes(error.as_ref()))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}
impl fmt::Display for Failures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "watch fixture failed: {} causes retained", self.0.len())
    }
}
impl Error for Failures {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.first().map(|e| e.as_ref())
    }
}
/// Keep typed IO/JSON/kernel/dispatch causes behind a static safe phase label.
fn caused(code: &'static str, error: impl Error + Send + Sync + 'static) -> E {
    E::Runtime(RuntimeError::new(
        RuntimeOperation::Platform,
        Phase {
            code,
            cause: Box::new(error),
        },
    ))
}
/// Fixture-only phase context; source preserves the original concrete error and
/// display intentionally omits provider output, paths and private runtime data.
#[derive(Debug)]
struct Phase {
    code: &'static str,
    cause: Box<dyn Error + Send + Sync>,
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
/// Observations retained from the first successful spawn even if its first
/// publication fails. PID alone grants no signal authority; the harness must
/// establish real owned-child identity through its kernel subreaper relationship.
/// Initial identity is retained separately from the latest observation; pending
/// session state makes no claim that setsid has or has not already run.
#[derive(Clone, Debug, Default)]
struct SpawnTrace {
    pid: Option<u32>,
    phase: &'static str,
    supervisor: Option<Value>,
    initial_supervisor: Option<Value>,
    session_state: Option<&'static str>,
    conductor: Option<Value>,
    early_exit: Option<Value>,
    publication_attempted: bool,
    publication_confirmed: bool,
    continuation: Option<Value>,
}
impl SpawnTrace {
    /// Project only known kernel observations and phase, never infer liveness,
    /// permission or absence of a file after a failed directory synchronization.
    fn document(&self) -> Value {
        json!({"pid":self.pid,"phase":self.phase,"supervisor":self.supervisor,
            "initial_supervisor":self.initial_supervisor,"session_state":self.session_state,
            "conductor":self.conductor,"early_exit":self.early_exit,
            "spawned_publication_attempted":self.publication_attempted,
            "spawned_publication_confirmed":self.publication_confirmed,
            "continuation_observation":self.continuation})
    }
}
/// Preserve the original operation failure together with the child observation.
/// Neither this error nor its safe projection signals, kills or abandons a child.
#[derive(Debug)]
struct ChildFailure {
    cause: E,
    trace: SpawnTrace,
}
impl fmt::Display for ChildFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("watch-fixture-child-retained")
    }
}
impl Error for ChildFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}
/// Preserve an original failure even when publishing its evidence also fails.
fn failed_publication(primary: E, publication: Result<()>) -> E {
    match publication {
        Ok(()) => primary,
        Err(cleanup) => E::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        },
    }
}
/// Refuse a failed fixture condition through normal cleanup, never panic/accept.
fn require(condition: bool, code: &'static str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(E::Refused(code))
    }
}
/// Atomically publish a fresh observation and synchronize bytes plus directory.
/// A post-rename sync failure is visible publication, not proof of rollback.
fn publish(path: &Path, value: &Value) -> Result<()> {
    let parent = path
        .parent()
        .ok_or(E::Refused("watch-fixture-output-parent"))?;
    let mut file = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| caused("watch-fixture-output-create", e))?;
    file.write_all(
        &serde_json::to_vec_pretty(value).map_err(|e| caused("watch-fixture-output-json", e))?,
    )
    .map_err(|e| caused("watch-fixture-output-write", e))?;
    file.flush()
        .map_err(|e| caused("watch-fixture-output-flush", e))?;
    file.as_file()
        .sync_all()
        .map_err(|e| caused("watch-fixture-output-sync", e))?;
    file.persist_noclobber(path)
        .map_err(|e| caused("watch-fixture-output-publish", e))?;
    File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(|e| caused("watch-fixture-output-directory-sync", e))
}
/// Retain safe labels/numeric causes from every established wrapper branch.
/// Pointer deduplication terminates cycles, without a hidden depth/time cutoff.
fn causes(error: &(dyn Error + 'static)) -> Vec<Value> {
    let mut pending = vec![error];
    let mut seen: Vec<*const dyn Error> = Vec::new();
    let mut out = Vec::new();
    while let Some(error) = pending.pop() {
        let pointer = error as *const dyn Error;
        if seen.iter().any(|previous| std::ptr::eq(*previous, pointer)) {
            continue;
        }
        seen.push(pointer);
        let links = swarm_engine::native_error_links(error);
        out.push(json!({"labels":links.labels,"io_errno":links.io_errno,"sqlite_code":links.sqlite_code}));
        if let Some(phase) = error.downcast_ref::<Phase>() {
            out.push(json!({"fixture_phase":phase.code}));
        }
        if let Some(unexpected) = error.downcast_ref::<UnexpectedDispatch>() {
            out.push(unexpected.observations());
            pending.extend(unexpected.errors());
        }
        if let Some(unexpected) = error.downcast_ref::<UnexpectedContinuation>() {
            out.push(json!({"continuation_observation":unexpected.document}));
            pending.extend(unexpected.errors());
        }
        if let Some(child) = error.downcast_ref::<ChildFailure>() {
            out.push(json!({"child_ownership":child.trace.document()}));
        }
        pending.extend(links.children.into_iter().map(|(_, child)| child));
    }
    out
}
/// Explicit fixture paths only; exact marker equality proves fixture correlation,
/// never Root permission, actor authenticity or qualification of the binary.
struct RootInstanceProbe {
    queued: PathBuf,
    observation: PathBuf,
    spawn_ready: Option<PathBuf>,
}
/// Strictly decode the optional probe protocol without filesystem/Root effects.
/// Absent means the historical graceful/signal route; null/partial/unknown
/// configuration and an unpaired mode refuse instead of falling back to B1.
/// Schema1 retains the original resume barrier; schema2 requires a distinct
/// spawn_ready output and adds the explicit post-check spawn command barrier.
fn probe_configuration(raw: &Value) -> Result<Option<RootInstanceProbe>> {
    let Some(value) = raw.get("root_instance_probe") else {
        require(
            matches!(raw["mode"].as_str(), Some("graceful" | "signal")),
            "watch-fixture-mode",
        )?;
        return Ok(None);
    };
    require(raw["mode"] == "probe", "watch-fixture-probe-mode")?;
    let object = value
        .as_object()
        .ok_or(E::Refused("watch-fixture-probe-shape"))?;
    let schema = value["schema_version"].as_i64();
    require(matches!(schema, Some(1 | 2)), "watch-fixture-probe-schema")?;
    require(
        object.len() == (if schema == Some(2) { 4 } else { 3 })
            && (schema != Some(2) || object.contains_key("spawn_ready"))
            && object.contains_key("schema_version")
            && object.contains_key("queued")
            && object.contains_key("observation"),
        "watch-fixture-probe-shape",
    )?;
    let path = |name| -> Result<PathBuf> {
        let path = value[name]
            .as_str()
            .map(PathBuf::from)
            .ok_or(E::Refused("watch-fixture-probe-path"))?;
        require(path.is_absolute(), "watch-fixture-probe-path")?;
        Ok(path)
    };
    let queued = path("queued")?;
    let observation = path("observation")?;
    let spawn_ready = if schema == Some(2) {
        Some(path("spawn_ready")?)
    } else {
        None
    };
    if let Some(spawn_ready) = &spawn_ready {
        require(
            spawn_ready != &queued && spawn_ready != &observation && queued != observation,
            "watch-fixture-output-collision",
        )?;
    }
    Ok(Some(RootInstanceProbe {
        queued,
        observation,
        spawn_ready,
    }))
}
/// Refuse output aliasing by exact decoded paths without filesystem effects.
/// Callers separately require absolute fresh sibling paths before any publication.
fn distinct_output_paths(paths: &[&PathBuf]) -> Result<()> {
    for (index, path) in paths.iter().enumerate() {
        require(
            !paths[..index].contains(path),
            "watch-fixture-output-collision",
        )?;
    }
    Ok(())
}
/// Explicit paths whose current marker still must match exactly before effects.
struct Input {
    root: PathBuf,
    work: String,
    binary: PathBuf,
    binary_sha256: String,
    result: PathBuf,
    spawned: PathBuf,
    ready: PathBuf,
    stdout: PathBuf,
    stderr: PathBuf,
    mode: String,
    probe: Option<RootInstanceProbe>,
}
impl Input {
    /// Resolve an existing owned Root/binary and require fresh sibling outputs.
    /// No directory, provider, consent or budget is implicitly initialized here.
    fn read(raw: &Value) -> Result<Self> {
        let probe = probe_configuration(raw)?;
        let text = |key| {
            raw[key]
                .as_str()
                .ok_or(E::Refused("watch-fixture-input-field"))
        };
        require(
            raw["kind"] == "owned-inert-watch-worker-fixture",
            "watch-fixture-kind",
        )?;
        let root = Path::new(text("root")?)
            .canonicalize()
            .map_err(|e| caused("watch-fixture-root", e))?;
        let marker: Value = serde_json::from_slice(
            &fs::read(root.join(".swarm/verification-watch-worker.json"))
                .map_err(|e| caused("watch-fixture-marker-read", e))?,
        )
        .map_err(|e| caused("watch-fixture-marker-json", e))?;
        require(&marker == raw, "watch-fixture-marker-mismatch")?;
        let binary = Path::new(text("binary")?)
            .canonicalize()
            .map_err(|e| caused("watch-fixture-binary", e))?;
        let binary_sha256 = format!(
            "{:x}",
            Sha256::digest(fs::read(&binary).map_err(|e| caused("watch-fixture-binary-read", e))?)
        );
        require(
            binary_sha256 == text("binary_sha256")?,
            "watch-fixture-binary-hash",
        )?;
        let result = PathBuf::from(text("result")?);
        let parent = result
            .parent()
            .ok_or(E::Refused("watch-fixture-output-parent"))?;
        require(
            result.is_absolute() && parent.is_dir(),
            "watch-fixture-output-directory",
        )?;
        let output = |key| -> Result<PathBuf> {
            let path = PathBuf::from(text(key)?);
            require(
                path.is_absolute() && path.parent() == Some(parent) && !path.exists(),
                "watch-fixture-output-not-fresh",
            )?;
            Ok(path)
        };
        let value = Self {
            root,
            work: text("work")?.into(),
            binary,
            binary_sha256,
            result: output("result")?,
            spawned: output("spawned")?,
            ready: output("ready")?,
            stdout: output("supervisor_stdout")?,
            stderr: output("supervisor_stderr")?,
            mode: text("mode")?.into(),
            probe,
        };
        let mut paths = vec![
            &value.result,
            &value.spawned,
            &value.ready,
            &value.stdout,
            &value.stderr,
        ];
        if let Some(probe) = &value.probe {
            for path in [&probe.queued, &probe.observation]
                .into_iter()
                .chain(probe.spawn_ready.iter())
            {
                require(
                    path.is_absolute() && path.parent() == Some(parent) && !path.exists(),
                    "watch-fixture-output-not-fresh",
                )?;
                paths.push(path);
            }
        }
        distinct_output_paths(&paths)?;
        Ok(value)
    }
}
/// Create a private durable channel BEFORE spawn, never an observer pipe whose
/// closure could terminate the independent supervisor or lose its final error.
fn channel(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| caused("watch-fixture-channel-create", e))?;
    file.sync_all()
        .map_err(|e| caused("watch-fixture-channel-sync", e))?;
    File::open(
        path.parent()
            .ok_or(E::Refused("watch-fixture-output-parent"))?,
    )
    .and_then(|dir| dir.sync_all())
    .map_err(|e| caused("watch-fixture-channel-directory-sync", e))?;
    Ok(file)
}
/// Observe actual kernel PID/stamp/PPID/PGID/SID. Integers and labels are not
/// signal capabilities; this fixture never signals a process based on them.
fn process(pid: u32) -> Result<Value> {
    let actual =
        swarm_engine::observe_linux_process(pid).map_err(|e| caused("watch-fixture-process", e))?;
    require(
        !matches!(actual.state(), 'Z' | 'X'),
        "watch-fixture-process-departed",
    )?;
    let raw = i32::try_from(pid).map_err(|e| caused("watch-fixture-process-range", e))?;
    let id = rustix::process::Pid::from_raw(raw).ok_or(E::Refused("watch-fixture-process-id"))?;
    let sid = rustix::process::getsid(Some(id))
        .map_err(|e| caused("watch-fixture-process-session", e))?;
    Ok(
        json!({"pid":actual.pid(),"stamp":actual.process_stamp(),"ppid":actual.parent_pid(),
        "pgid":actual.process_group_id(),"sid":sid.as_raw_nonzero().get(),"state":actual.state().to_string()}),
    )
}
/// Read exactly one fixture control line. EOF/other text is a real failure, not
/// a stop grant, provider-release instruction, retry or verification deadline.
fn command(expected: &'static str) -> Result<()> {
    let mut line = String::new();
    let count = io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| caused("watch-fixture-input-read", e))?;
    require(
        count != 0 && line == expected,
        "watch-fixture-input-protocol",
    )
}
/// Write an explicit named observation and flush; failures remain normal errors.
fn announcement(message: &'static str) -> Result<()> {
    let mut output = io::stdout().lock();
    writeln!(output, "{message}")
        .and_then(|()| output.flush())
        .map_err(|e| caused("watch-fixture-announcement", e))
}

/// Keep an unexpected dispatch document owned behind a safe diagnostic. It is
/// neither an empty success nor a new grant; original typed per-task/resource
/// causes remain in the outcome and all known branches are published separately.
struct UnexpectedDispatch(Dispatch);
impl UnexpectedDispatch {
    /// Inspect every outcome's queue/resource branches without Agent/prompt data.
    /// Some(queue) means synchronization was committed, not fresh permission.
    fn parts(
        &self,
    ) -> (
        Option<&swarm_engine::MissionQueueRegistration>,
        &[swarm_engine::MissionDispatchResourceRefusal],
    ) {
        match &self.0 {
            Dispatch::Prepared {
                queue, resources, ..
            }
            | Dispatch::NoAdmission { queue, resources } => (Some(queue), resources),
            Dispatch::Replayed {
                queue, resources, ..
            } => (queue.as_ref(), resources),
        }
    }
    /// Borrow all original typed causes, including secondary resource refusals.
    /// Error::source alone cannot enumerate this multi-branch result.
    fn errors(&self) -> Vec<&(dyn Error + 'static)> {
        let (queue, resources) = self.parts();
        let mut errors: Vec<&dyn Error> = Vec::new();
        if let Some(queue) = queue {
            if let Some(error) = &queue.permission_refusal {
                errors.push(error);
            }
            errors.extend(queue.refused.iter().map(|r| &r.error as &dyn Error));
        }
        for resource in resources {
            use swarm_engine::MissionDispatchResourceRefusal as R;
            if let R::Occupied { cause, .. } | R::Capacity { cause, .. } = resource {
                errors.push(cause);
            }
        }
        errors
    }
    /// Project known effect kind and every refusal without serializing the Agent,
    /// prompt or runtime output. The original outcome remains owned in this error.
    fn observations(&self) -> Value {
        let (queue, resources) = self.parts();
        let kind = match &self.0 {
            Dispatch::Prepared { .. } => "Prepared",
            Dispatch::Replayed { .. } => "Replayed",
            Dispatch::NoAdmission { .. } => "NoAdmission",
        };
        let queue = queue.map(|queue| json!({"work":queue.work,"revision":queue.revision,
            "synchronization_committed":true,"ready":queue.ready,
            "permission_refusal":queue.permission_refusal.as_ref().map(|error| causes(error)),
            "refused":queue.refused.iter().map(|r| json!({"task":r.task,"causes":causes(&r.error)})).collect::<Vec<_>>()}));
        let resources: Vec<_> = resources
            .iter()
            .map(|resource| {
                use swarm_engine::MissionDispatchResourceRefusal as R;
                match resource {
                    R::Fifo { task, blocker } => {
                        json!({"task":task,"kind":"fifo","blocker":blocker})
                    }
                    R::Occupied { task, cause } => {
                        json!({"task":task,"kind":"occupied","causes":causes(cause)})
                    }
                    R::Capacity { task, cause } => {
                        json!({"task":task,"kind":"capacity","causes":causes(cause)})
                    }
                }
            })
            .collect();
        json!({"outcome":kind,"queue":queue,"resources":resources,"intention_recorded_or_replayed":!matches!(&self.0,Dispatch::NoAdmission{..})})
    }
}
impl fmt::Debug for UnexpectedDispatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unexpected fixture dispatch outcome (retained)")
    }
}
impl fmt::Display for UnexpectedDispatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
impl Error for UnexpectedDispatch {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.errors().into_iter().next()
    }
}
/// A fixture comparison of real public projections, excluding the independently
/// maintained mission heartbeat. No SQL/private datastore access or fabricated
/// row is used. Receipt/FIFO equality cannot attest unexposed reservation rows.
#[derive(PartialEq, Eq)]
struct ContinuationSnapshot {
    agent: Vec<u8>,
    work: Vec<u8>,
    receipts: Vec<swarm_engine::MissionWorkReceipt>,
    queue: swarm_engine::MissionWorkspaceQueue,
}
impl ContinuationSnapshot {
    /// Snapshot existing readers in sequence while no worker has been spawned.
    /// These are independent public read transactions, not a claimed distributed
    /// atomic snapshot. Any read/encoding error retains its typed cause.
    fn read(store: &RootStore, work: &str, agent: &str) -> Result<Self> {
        Ok(Self {
            agent: serde_json::to_vec(&store.read_agent(agent)?)
                .map_err(|e| caused("watch-fixture-continuation-agent-json", e))?,
            work: serde_json::to_vec(&store.read_work(work)?)
                .map_err(|e| caused("watch-fixture-continuation-work-json", e))?,
            receipts: store
                .read_mission_work_receipts(work)
                .map_err(|e| caused("watch-fixture-continuation-receipts", e))?,
            queue: store
                .mission_workspace_queue(work)
                .map_err(|e| caused("watch-fixture-continuation-queue", e))?,
        })
    }
    /// Publish digest/count observations, never raw prompt or full Agent bytes.
    /// The exact FIFO object is compared internally; counts do not stand in for it.
    fn document(&self) -> Value {
        json!({"agent_sha256":format!("{:x}",Sha256::digest(&self.agent)),
            "work_sha256":format!("{:x}",Sha256::digest(&self.work)),
            "receipt_count":self.receipts.len(),"fifo_turn_count":self.queue.turns.len(),
            "fifo_blocker_count":self.queue.blockers.len()})
    }
}
/// Retain a complete unexpected classification and every typed per-entry refusal.
/// Its safe document contains fixture coordinates/closed state names only. No
/// report or JSON grants permission to start another attempt.
struct UnexpectedContinuation {
    observation: MissionWorkerContinuationObservation,
    document: Value,
    read_error: Option<E>,
}
impl UnexpectedContinuation {
    /// Borrow all refusal causes; source alone cannot retain multiple entries.
    fn errors(&self) -> Vec<&(dyn Error + 'static)> {
        self.observation
            .entries
            .iter()
            .filter_map(|entry| match &entry.state {
                ContinuationState::Refused { cause, .. } => Some(cause as &(dyn Error + 'static)),
                _ => None,
            })
            .chain(
                self.read_error
                    .iter()
                    .map(|cause| cause as &(dyn Error + 'static)),
            )
            .collect()
    }
}
impl fmt::Debug for UnexpectedContinuation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UnexpectedContinuation(<retained>)")
    }
}
impl fmt::Display for UnexpectedContinuation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
impl Error for UnexpectedContinuation {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.errors().into_iter().next()
    }
}
/// Exercise the real continuation component on a committed queued intention,
/// before any fixture barrier or child process. Exactly one target must retain
/// its current lease generation. All other states/refusals are retained and make
/// this narrow fixture fail, never an empty success or a new retry/launch.
/// This verifies only the public Agent/Work/receipt/FIFO projections; it does not
/// claim reservation-row equality, liveness, admission or global watch delivery.
fn observe_queued_continuation(
    store: &RootStore,
    lease: &MissionLease,
    work: &str,
    agent: &str,
) -> Result<Value> {
    let before = ContinuationSnapshot::read(store, work, agent)?;
    let observed = store
        .observe_mission_worker_continuation(lease)
        .map_err(|e| caused("watch-fixture-continuation-observe", *e))?;
    let entries: Vec<Value> = observed.entries.iter().map(|entry| {
        let state = match &entry.state {
            ContinuationState::QueuedUnregisteredObserved { generation, current_lease_generation } =>
                json!({"state":"queued-unregistered-observed","generation":generation,"current_lease_generation":current_lease_generation}),
            ContinuationState::StopRequestedUnregisteredObserved { generation, current_lease_generation } =>
                json!({"state":"stop-requested-unregistered-observed","generation":generation,"current_lease_generation":current_lease_generation}),
            ContinuationState::RegisteredObserved => json!({"state":"registered-observed"}),
            ContinuationState::TerminalVerified { receipt_sha256, output_complete, provider_may_have_started } =>
                json!({"state":"terminal-verified","receipt_sha256":receipt_sha256,"output_complete":output_complete,"provider_may_have_started":provider_may_have_started}),
            ContinuationState::OutsideAutomaticWorkerScope => json!({"state":"outside-automatic-worker-scope"}),
            ContinuationState::Refused { cause, .. } => json!({"state":"refused","causes":causes(cause)}),
        };
        json!({"agent_id":entry.agent_id,"task_id":entry.task_id,"observation":state})
    }).collect();
    let after = match ContinuationSnapshot::read(store, work, agent) {
        Ok(after) => after,
        Err(cause) => {
            let document = json!({"status":"CONTINUATION_PUBLIC_AFTER_READ_FAILED",
                "target_agent_id":agent,"work":observed.work,"entries":entries,
                "before":before.document(),"after":null,"public_projections_unchanged":null,
                "agent_work_receipts_fifo_verified":false,"reservation_rows_verified":false,
                "scope":"public-projections-before-spawn","observation_is_permission":false});
            return Err(caused(
                "watch-fixture-continuation-after-read",
                UnexpectedContinuation {
                    observation: observed,
                    document,
                    read_error: Some(cause),
                },
            ));
        }
    };
    let target_count = observed
        .entries
        .iter()
        .filter(|entry| entry.agent_id == agent)
        .count();
    let target_queued = observed.entries.iter().any(|entry| entry.agent_id == agent &&
        matches!(&entry.state, ContinuationState::QueuedUnregisteredObserved { generation, current_lease_generation: true } if *generation == lease.generation()));
    let valid = before == after
        && observed.work == work
        && observed.generation == lease.generation()
        && observed.entries.len() == 1
        && target_count == 1
        && target_queued;
    let document = json!({"status":if valid {"QUEUED_CONTINUATION_VERIFIED"} else {"UNEXPECTED_CONTINUATION"},
        "target_agent_id":agent,"work":observed.work,"work_revision":observed.revision,
        "settings_revision":observed.settings_revision,"generation":observed.generation,
        "target_count":target_count,"entries":entries,"before":before.document(),"after":after.document(),
        "public_projections_unchanged":before==after,"agent_work_receipts_fifo_verified":valid,
        "reservation_rows_verified":false,"scope":"public-projections-before-spawn",
        "process_started_by_observation":false,"observation_is_permission":false});
    if !valid {
        return Err(caused(
            "watch-fixture-continuation-unexpected",
            UnexpectedContinuation {
                observation: observed,
                document,
                read_error: None,
            },
        ));
    }
    Ok(document)
}

/// Pause only an explicitly marked probe AFTER a real queued admission and
/// BEFORE any supervisor spawn. The retained RootBinding is checked against the
/// opened store; dev/ino are separately labelled pathname observations, not a
/// transferred Root capability or proof of the child datastore. Caller retains
/// and explicitly closes this binding after cycle/lease/Root cleanup. The existing
/// cycle heartbeat remains untouched. Schema2 additionally publishes a fresh
/// spawn_ready observation AFTER both original guards succeed, then waits for
/// `spawn\n`. This fixture-only late injection point does not reclassify the
/// observations as authority; the real child bootstrap independently checks Root.
fn probe_barrier(
    store: &RootStore,
    input: &Input,
    intention: &swarm_engine::AgentLaunchOutcome,
    probe: &RootInstanceProbe,
    binding: &RootBinding,
    monitor: &swarm_engine::MissionCycleMonitor,
    continuation: &Value,
) -> Result<()> {
    (|| {
        require(
            binding.matches(store)?,
            "watch-fixture-probe-original-binding",
        )?;
        let directory = fs::symlink_metadata(input.root.join(".swarm"))
            .map_err(|e| caused("watch-fixture-probe-directory-observe", e))?;
        require(directory.is_dir(), "watch-fixture-probe-directory-kind")?;
        let observed = json!({"status":"ROOT_INSTANCE_PROBE_QUEUED","work":input.work,
            "agent_id":intention.agent.id,"post_dispatch_work_revision":store.read_work(&input.work)?.revision,
            "native_mission":intention.agent.native_mission,"agent_status":intention.agent.status,
            "prompt_sha256":format!("{:x}",Sha256::digest(intention.agent.prompt.as_bytes())),
            "conductor":process(std::process::id())?,"root":input.root,
            "original_binding_matches_open_store":true,"original_binding_retained":true,
            "root_path_directory_observation":{"dev":directory.dev(),"ino":directory.ino()},
            "directory_observation_is_authority":false,"supervisor_spawned":false,
            "continuation_observation":continuation});
        publish(&probe.queued, &observed)?;
        announcement(
            "OBSERVED root-instance probe queued before supervisor spawn; awaiting resume",
        )?;
        command("resume\n")?;
        let binding_matches = binding.matches(store)?;
        require(binding_matches, "watch-fixture-probe-original-binding")?;
        let monitor_running = monitor.is_running();
        require(monitor_running, "watch-fixture-heartbeat-ended")?;
        if let Some(path) = &probe.spawn_ready {
            publish(
                path,
                &json!({"status":"ROOT_INSTANCE_PROBE_SPAWN_READY",
                "schema_version":2,"work":input.work,"agent_id":intention.agent.id,
                "original_binding_matches_open_store":binding_matches,
                "mission_cycle_monitor_running":monitor_running,
                "original_binding_retained":true,"supervisor_spawned":false,
                "observation_is_authority":false,"conductor":process(std::process::id())?}),
            )?;
            announcement(
                "OBSERVED original binding and cycle checks passed; awaiting fixture spawn command",
            )?;
            command("spawn\n")?;
        }
        Ok(())
    })()
}

/// Record exactly one child try_wait observation after the harness finishes its
/// independent clone/original inspection. Do not read the original store's Agent
/// or infer provider entry, completion, refusal, root binding or acceptance.
/// A running child yields an explicit absent exit observation, not a fake reap.
fn probe_observation(
    input: &Input,
    probe: &RootInstanceProbe,
    agent_id: &str,
    active: &mut Child,
    trace: &mut SpawnTrace,
    continuation: &Value,
) -> Result<Value> {
    trace.phase = "await-finish-probe";
    command("finish-probe\n")?;
    trace.phase = "probe-try-wait";
    let status = active
        .try_wait()
        .map_err(|e| caused("watch-fixture-probe-try-wait", e))?;
    let exit = status.map(|status| {
        use std::os::unix::process::ExitStatusExt;
        json!({"code":status.code(),"signal":status.signal(),"scope":"probe-final-try-wait"})
    });
    // Preserve a real collected status even if publishing the observation fails;
    // the scope explicitly does not classify this as a premature provider exit.
    trace.early_exit = exit.clone();
    let observed = json!({"status":"ROOT_INSTANCE_PROBE_OBSERVED","work":input.work,
        "agent_id":agent_id,"supervisor_pid":active.id(),
        "child_ownership":trace.document(),"worker_exit_observed":exit.is_some(),
        "child_reaped_by_sdk":exit.is_some(),
        "exit_observation":exit,"worker_completion_observed":false,
        "provider_entry_observed_by_sdk":false,"child_datastore_binding_observed":false,
        "observation_scope":"single-owned-child-try-wait","lease_released":false,
        "continuation_observation":continuation});
    trace.phase = "publish-probe-observation";
    publish(&probe.observation, &observed)?;
    announcement("OBSERVED root-instance probe child exit status or explicit absence retained")?;
    Ok(observed)
}
/// Dispatch one true queued intention and launch the EXISTING normal CLI runtime
/// through its safe private bootstrap. Spawn returns before session readiness;
/// only real provider verification below checks independent SID/PGID and identity.
/// No provider argv is built by this conductor fixture.
/// Child handles live outside the heartbeat callback and are never killed here.
/// try_wait can reap an already exited CLI before provider verification; normal
/// released-worker completion is not awaited. Harness owns post-exit waiting.
fn cycle(
    store: &mut RootStore,
    lease: &MissionLease,
    input: &Input,
    child: &mut Option<Child>,
    trace: &mut SpawnTrace,
    monitor: &swarm_engine::MissionCycleMonitor,
    probe_binding: Option<&RootBinding>,
) -> Result<Value> {
    require(monitor.is_running(), "watch-fixture-heartbeat-not-running")?;
    let request = MissionWorkerDispatchRequest {
        schema_version: 1,
        event_id: "watch-worker-agent".into(),
        expected_revision: store.read_work(&input.work)?.revision,
    };
    let outcome = store
        .dispatch_mission_worker(lease, &request, &mut SystemEnvironment)
        .map_err(|e| caused("watch-fixture-dispatch", *e))?;
    let intention = match outcome {
        Dispatch::Prepared { intention, .. } => intention,
        other => {
            let unexpected = UnexpectedDispatch(other);
            let observations = unexpected.observations();
            let primary = caused("watch-fixture-dispatch-not-prepared", unexpected);
            return Err(failed_publication(
                primary,
                publish(
                    &input.result.with_extension("dispatch-refusal.json"),
                    &json!({"status":"UNEXPECTED_DISPATCH","observations":observations}),
                ),
            ));
        }
    };
    require(
        intention.created && !intention.preview_only && intention.agent.status == "queued",
        "watch-fixture-intention-not-new-queued",
    )?;
    let continuation = observe_queued_continuation(store, lease, &input.work, &intention.agent.id)?;
    trace.continuation = Some(continuation.clone());
    if let Some(probe) = &input.probe {
        let binding = probe_binding.ok_or(E::Refused("watch-fixture-probe-binding-missing"))?;
        probe_barrier(
            store,
            input,
            &intention,
            probe,
            binding,
            monitor,
            &continuation,
        )?;
    }
    let stdout = channel(&input.stdout)?;
    let stderr = channel(&input.stderr)?;
    let argv = vec![
        input.binary.to_string_lossy().into_owned(),
        "_mission_worker_entry".into(),
        intention.agent.id.clone(),
        "--root".into(),
        input.root.to_string_lossy().into_owned(),
        "--json".into(),
    ];
    let mut spawn = Command::new(&input.binary);
    spawn
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    // Deliberately inherit PGID: making this child a group leader would cause
    // the product bootstrap's setsid syscall to refuse with EPERM after exec.
    *child = Some(
        spawn
            .spawn()
            .map_err(|e| caused("watch-fixture-supervisor-spawn", e))?,
    );
    trace.pid = child.as_ref().map(Child::id);
    trace.phase = "spawn-returned";
    trace.session_state = Some("pending");
    let active = child
        .as_mut()
        .ok_or(E::Refused("watch-fixture-supervisor-handle"))?;
    trace.phase = "observe-supervisor";
    let supervisor = process(active.id())?;
    trace.supervisor = Some(supervisor.clone());
    trace.initial_supervisor = Some(supervisor.clone());
    trace.phase = "observe-conductor";
    let conductor = process(std::process::id())?;
    trace.conductor = Some(conductor.clone());
    trace.phase = "verify-initial-parent";
    require(
        supervisor["ppid"] == std::process::id(),
        "watch-fixture-spawn-parent-mismatch",
    )?;
    trace.phase = "publish-spawned";
    trace.publication_attempted = true;
    publish(
        &input.spawned,
        &json!({"status":if input.probe.is_some() {"ROOT_INSTANCE_PROBE_SPAWNED"} else {"SUPERVISOR_SPAWNED"},"agent_id":intention.agent.id,"work":input.work,
        "conductor":conductor,"supervisor":supervisor,"argv":argv,"binary_sha256":input.binary_sha256,
        "supervisor_stdout":input.stdout,"supervisor_stderr":input.stderr,"mode":input.mode,
        "session_state":"pending","grant_observed":false,"continuation_observation":continuation}),
    )?;
    trace.publication_confirmed = true;
    trace.phase = "announce-spawned";
    if let Some(probe) = &input.probe {
        announcement("OBSERVED root-instance probe bootstrap spawned; child datastore and provider entry not inferred")?;
        return probe_observation(
            input,
            probe,
            &intention.agent.id,
            active,
            trace,
            &continuation,
        );
    }
    announcement(
        "OBSERVED existing CLI bootstrap spawned; session and provider entry not yet verified",
    )?;
    trace.phase = "await-provider-verification";
    command("verify-provider\n")?;
    trace.phase = "observe-early-exit";
    if let Some(status) = active
        .try_wait()
        .map_err(|e| caused("watch-fixture-supervisor-observe", e))?
    {
        use std::os::unix::process::ExitStatusExt;
        trace.early_exit = Some(json!({"code":status.code(),"signal":status.signal()}));
        return Err(failed_publication(
            E::Refused("watch-fixture-supervisor-exited-before-provider"),
            publish(
                &input.result.with_extension("early-exit.json"),
                &json!({"code":status.code(),"signal":status.signal()}),
            ),
        ));
    }
    trace.phase = "verify-provider";
    let agent = store.read_agent(&intention.agent.id)?;
    require(
        agent.status == "running"
            && agent.supervisor_pid == i64::from(active.id())
            && agent.child_pid > 0
            && agent.native_execution.is_some()
            && agent.native_execution_result.is_none(),
        "watch-fixture-real-provider-not-running",
    )?;
    let actual_supervisor = process(active.id())?;
    trace.supervisor = Some(actual_supervisor.clone());
    let provider_pid =
        u32::try_from(agent.child_pid).map_err(|e| caused("watch-fixture-provider-pid", e))?;
    let provider = process(provider_pid)?;
    require(
        actual_supervisor["pid"] == supervisor["pid"]
            && actual_supervisor["ppid"] == conductor["pid"]
            && actual_supervisor["stamp"] == agent.supervisor_identity
            && actual_supervisor["stamp"] == supervisor["stamp"]
            && provider["stamp"] == agent.child_identity
            && provider["ppid"] == active.id(),
        "watch-fixture-kernel-agent-mismatch",
    )?;
    trace.phase = "verify-provider-marker";
    let marker: Value = serde_json::from_slice(
        &fs::read(input.root.join("watch-worker/model-received.json"))
            .map_err(|e| caused("watch-fixture-provider-marker-read", e))?,
    )
    .map_err(|e| caused("watch-fixture-provider-marker-json", e))?;
    let prompt_sha256 = format!("{:x}", Sha256::digest(agent.prompt.as_bytes()));
    require(
        marker["pid"] == provider_pid
            && marker["ppid"] == active.id()
            && marker["stdin_sha256"] == prompt_sha256,
        "watch-fixture-provider-witness-mismatch",
    )?;
    trace.phase = "verify-independent-session-after-provider-entry";
    require(
        actual_supervisor["sid"] == active.id()
            && actual_supervisor["pgid"] == active.id()
            && actual_supervisor["sid"] != conductor["sid"],
        "watch-fixture-session-not-independent",
    )?;
    require(monitor.is_running(), "watch-fixture-heartbeat-ended")?;
    trace.session_state = Some("verified-independent");
    let observed = json!({"status":"PROVIDER_VERIFIED_UNDER_LIVE_CONDUCTOR","work":input.work,"agent_id":agent.id,
        "conductor":conductor,"supervisor":actual_supervisor,"provider":provider,"argv":argv,
        "initial_supervisor":supervisor,"session_state":"verified-independent",
        "binary_sha256":input.binary_sha256,"mode":input.mode,"provider_marker":"watch-worker/model-received.json",
        "prompt_sha256":prompt_sha256,"binding":agent.native_mission,"task_accepted":false,
        "supervisor_stdout":input.stdout,"supervisor_stderr":input.stderr,"lease_released":false,
        "continuation_observation":continuation});
    trace.phase = "publish-ready";
    publish(&input.ready, &observed)?;
    announcement("PASS actual dispatched provider entered under independent CLI supervisor and live conductor")?;
    trace.phase = "await-conductor-stop";
    command("stop-conductor\n")?;
    require(
        input.mode == "graceful",
        "watch-fixture-signal-mode-exited-without-signal",
    )?;
    Ok(observed)
}

/// Require an owned marker, explicitly configure the mission profile, then keep
/// the lease alive while a true independent CLI/provider enters. Graceful stdin
/// stop releases/closes and publishes before SDK exit. Signal mode intentionally
/// has NO successful completion receipt: harness must retain real wait status and
/// historical unreleased lease after actual SIGTERM/SIGINT, not invent cleanup.
/// Explicit probe mode instead pauses before spawn and returns only an owned
/// child wait observation, then closes the conductor without a B1 readiness claim.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("owned watch fixture request required".into());
    }
    let raw: Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let input = Input::read(&raw)?;
    let mut store = RootStore::open(&input.root, true)?;
    let mut lease = None;
    let mut child = None;
    let mut trace = SpawnTrace::default();
    let mut probe_binding = None;
    let action = (|| -> Result<Value> {
        if input.probe.is_some() {
            probe_binding = Some(store.bind_root()?);
        }
        fs::create_dir(input.root.join("watch-worker"))
            .map_err(|e| caused("watch-fixture-workspace-create", e))?;
        let start = json!({"schema_version":1,"event_id":"watch-worker-start","expected_revision":store.read_work(&input.work)?.revision,
            "reason":"Owned B1 independent runtime fixture","slots":1,"profile":{"provider":"fixture","role":"worker",
            "workspace":"watch-worker","level":"standard","capture_output":true}});
        store.execute_external(
            &input.work,
            "mission.start",
            &serde_json::to_vec(&start).map_err(|e| caused("watch-fixture-start-json", e))?,
            &mut SystemEnvironment,
        )?;
        lease = Some(
            store
                .claim_mission_supervision(
                    &input.work,
                    "Owned watch fixture",
                    &swarm_engine::system_timestamp().map_err(E::Runtime)?,
                )
                .map_err(|e| caused("watch-fixture-claim", e))?,
        );
        let owned = lease
            .as_ref()
            .ok_or(E::Refused("watch-fixture-lease-missing"))?;
        match store.run_mission_cycle(owned, |store, lease, monitor| {
            cycle(
                store,
                lease,
                &input,
                &mut child,
                &mut trace,
                monitor,
                probe_binding.as_ref(),
            )
        }) {
            Ok(value) => Ok(value),
            Err(error) => {
                let observation =
                    json!({"diagnostics":error.diagnostics(),"causes":causes(error.as_ref())});
                let primary = caused("watch-fixture-cycle", *error);
                Err(failed_publication(
                    primary,
                    publish(
                        &input.result.with_extension("cycle-failure.json"),
                        &observation,
                    ),
                ))
            }
        }
    })();
    let mut failures: Vec<Box<dyn Error>> = Vec::new();
    let observed = match action {
        Ok(value) => Some(value),
        Err(error) => {
            if trace.pid.is_some() {
                failures.push(Box::new(ChildFailure {
                    cause: error,
                    trace: trace.clone(),
                }) as Box<dyn Error>);
            } else {
                failures.push(Box::new(error));
            }
            None
        }
    };
    if let Some(owned) = lease {
        match swarm_engine::system_timestamp() {
            Ok(now) => {
                if let Err(error) = store.update_mission_supervision(&owned, &now, true) {
                    failures.push(Box::new(error));
                }
            }
            Err(error) => failures.push(Box::new(error)),
        }
        if let Err(error) = owned.close() {
            failures.push(Box::new(error));
        }
    }
    if let Err(error) = store.close() {
        failures.push(Box::new(error));
    }
    if let Some(binding) = probe_binding {
        if let Err(error) = binding.close() {
            failures.push(Box::new(error));
        }
    }
    // Child::drop does not kill or wait. try_wait above may already have reaped
    // an early exit; otherwise post-exit liveness/reap/settlement belong to the
    // harness. trace retains the actual spawn and any already observed status.
    drop(child);
    if failures.is_empty() {
        let mut result = observed.ok_or("watch fixture result missing")?;
        result["status"] = json!(if input.probe.is_some() {
            "ROOT_INSTANCE_PROBE_CLOSED"
        } else {
            "GRACEFUL_CONDUCTOR_STOPPED"
        });
        result["lease_released"] = json!(true);
        result["lease_closed"] = json!(true);
        result["root_closed"] = json!(true);
        if input.probe.is_some() {
            result["original_root_binding_closed"] = json!(true);
        }
        if input.probe.is_none() {
            result["worker_exit_observed"] = json!(false);
        }
        result["worker_completion_observed"] = json!(false);
        trace.phase = "publish-graceful-result";
        let publication = publish(&input.result, &result).and_then(|()| {
            trace.phase = "announce-graceful-result";
            announcement(if input.probe.is_some() {"OBSERVED root-instance probe conductor stopped and closed; worker effects require independent inspection"}
                else {"PASS graceful conductor stopped and closed without stopping independent worker"})
        });
        match publication {
            Ok(()) => Ok(()),
            Err(cause) => Err(Box::new(Failures(vec![Box::new(ChildFailure {
                cause,
                trace,
            })]))),
        }
    } else {
        let diagnostics: Vec<_> = failures
            .iter()
            .map(|error| causes(error.as_ref()))
            .collect();
        if let Err(error) = publish(
            &input.result.with_extension("failure.json"),
            &json!({"status":"FAIL","causes":diagnostics,"child_ownership":trace.document(),"worker_not_stopped_by_fixture":true}),
        ) {
            failures.push(Box::new(error));
        }
        Err(Box::new(Failures(failures)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swarm_engine::{
        Agent, AgentLaunchOutcome, MissionCandidateRefusal, MissionDispatchResourceRefusal as R,
        MissionQueueRegistration, MissionSupervisionError as M,
    };

    /// Replayed is an observed intention plus an optional committed queue. All
    /// refusal branches must remain visible without leaking Agent/prompt data.
    #[test]
    fn replayed_queue_and_secondary_resource_causes_are_visible(
    ) -> std::result::Result<(), Box<dyn Error>> {
        let mut agent = Agent::default();
        agent.prompt = "private-test-prompt".into();
        let error = UnexpectedDispatch(Dispatch::Replayed {
            queue: Some(MissionQueueRegistration {
                work: "work".into(),
                revision: 7,
                ready: vec![],
                permission_refusal: Some(M::Refused("fixture-permission")),
                refused: vec![MissionCandidateRefusal {
                    task: "task".into(),
                    error: M::Refused("fixture-readiness"),
                }],
            }),
            intention: Box::new(AgentLaunchOutcome {
                agent,
                created: false,
                preview_only: false,
            }),
            resources: vec![R::Occupied {
                task: "task".into(),
                cause: E::Cleanup {
                    primary: Some(Box::new(E::Refused("fixture-resource-primary"))),
                    cleanup: Box::new(E::Refused("fixture-resource-secondary")),
                },
            }],
        });
        let observations = error.observations();
        assert_eq!(observations["outcome"], "Replayed");
        assert_eq!(observations["queue"]["revision"], 7);
        assert_eq!(observations["queue"]["synchronization_committed"], true);
        assert_eq!(observations["intention_recorded_or_replayed"], true);
        let projection = serde_json::to_string(&causes(&error))?;
        for label in [
            "fixture-permission",
            "fixture-readiness",
            "fixture-resource-primary",
            "fixture-resource-secondary",
        ] {
            assert!(projection.contains(label), "missing typed cause: {label}");
        }
        assert!(!projection.contains("private-test-prompt"));
        assert!(error.source().is_some());
        Ok(())
    }

    /// Canonical replay without queue sync is distinct from a committed empty
    /// queue; its resource errors must still be projected and sourced.
    #[test]
    fn replay_without_queue_retains_resource_and_absence() -> std::result::Result<(), Box<dyn Error>>
    {
        let error = UnexpectedDispatch(Dispatch::Replayed {
            queue: None,
            intention: Box::new(AgentLaunchOutcome {
                agent: Agent::default(),
                created: false,
                preview_only: false,
            }),
            resources: vec![R::Capacity {
                task: "task".into(),
                cause: E::Refused("fixture-capacity"),
            }],
        });
        assert!(error.observations()["queue"].is_null());
        assert!(serde_json::to_string(&causes(&error))?.contains("fixture-capacity"));
        assert!(error.source().is_some());
        Ok(())
    }

    /// A failed first publication keeps an actually observed child identity in
    /// safe stderr diagnostics even when no JSON evidence can be published.
    #[test]
    fn post_spawn_failure_retains_phase_identity_and_original_cause() {
        let trace = SpawnTrace {
            pid: Some(4321),
            phase: "publish-spawned",
            supervisor: Some(json!({"pid":4321,"stamp":"kernel-stamp"})),
            publication_attempted: true,
            ..SpawnTrace::default()
        };
        let failure = Failures(vec![Box::new(ChildFailure {
            cause: E::Refused("fixture-publication"),
            trace,
        })]);
        let diagnostic = format!("{failure:?}");
        for value in [
            "4321",
            "kernel-stamp",
            "publish-spawned",
            "fixture-publication",
            "spawned_publication_confirmed",
        ] {
            assert!(
                diagnostic.contains(value),
                "missing safe ownership/cause: {value}"
            );
        }
    }

    /// Only an explicit, complete fixture mode enables the barrier. Unknown,
    /// null, partial and incorrectly paired options never fall back to B1.
    /// This pure parser test opens no Root/files and invokes no process/syscall.
    #[test]
    fn root_instance_probe_configuration_is_explicit_and_strict() -> Result<()> {
        for mode in ["graceful", "signal"] {
            assert!(probe_configuration(&json!({"mode":mode}))?.is_none());
        }
        let valid = json!({"mode":"probe","root_instance_probe":{
            "schema_version":1,"queued":"/fixture/queued.json","observation":"/fixture/observation.json"}});
        let configured =
            probe_configuration(&valid)?.ok_or(E::Refused("fixture-probe-required"))?;
        assert_eq!(configured.queued, PathBuf::from("/fixture/queued.json"));
        assert_eq!(
            configured.observation,
            PathBuf::from("/fixture/observation.json")
        );
        let mut invalid = vec![
            json!({"mode":"probe"}),
            json!({"mode":"probe","root_instance_probe":null}),
        ];
        for patch in [
            json!({"schema_version":2}),
            json!({"schema_version":1.0}),
            json!({"extra":true}),
            json!({"queued":null}),
            json!({"queued":"relative.json"}),
            json!({"observation":false}),
        ] {
            let mut value = valid.clone();
            for (key, entry) in patch
                .as_object()
                .ok_or(E::Refused("fixture-patch-required"))?
            {
                value["root_instance_probe"][key] = entry.clone();
            }
            invalid.push(value);
        }
        let mut missing = valid.clone();
        missing["root_instance_probe"]
            .as_object_mut()
            .ok_or(E::Refused("fixture-probe-required"))?
            .remove("queued");
        invalid.push(missing);
        for mode in ["graceful", "signal", "other"] {
            let mut value = valid.clone();
            value["mode"] = json!(mode);
            invalid.push(value);
        }
        for value in invalid {
            assert!(probe_configuration(&value).is_err());
        }
        Ok(())
    }

    /// Schema2 adds only a strict late fixture barrier. These pure guards check
    /// types, exact keys and both probe-internal and shared output collisions;
    /// they open no datastore and do not establish that a spawn/check occurred.
    #[test]
    fn root_instance_late_probe_schema_and_output_collisions_are_strict() -> Result<()> {
        let valid = json!({"mode":"probe","root_instance_probe":{
            "schema_version":2,"queued":"/fixture/queued.json",
            "observation":"/fixture/observation.json","spawn_ready":"/fixture/spawn-ready.json"}});
        let configured =
            probe_configuration(&valid)?.ok_or(E::Refused("fixture-probe-required"))?;
        let spawn_ready = configured
            .spawn_ready
            .ok_or(E::Refused("fixture-spawn-ready-required"))?;
        assert_eq!(spawn_ready, PathBuf::from("/fixture/spawn-ready.json"));
        for patch in [
            json!({"schema_version":1}),
            json!({"schema_version":2.0}),
            json!({"schema_version":3}),
            json!({"spawn_ready":null}),
            json!({"spawn_ready":true}),
            json!({"spawn_ready":"relative.json"}),
            json!({"spawn_ready":"/fixture/queued.json"}),
            json!({"spawn_ready":"/fixture/observation.json"}),
            json!({"observation":"/fixture/queued.json"}),
            json!({"extra":false}),
        ] {
            let mut invalid = valid.clone();
            for (key, value) in patch
                .as_object()
                .ok_or(E::Refused("fixture-patch-required"))?
            {
                invalid["root_instance_probe"][key] = value.clone();
            }
            assert!(probe_configuration(&invalid).is_err());
        }
        let mut missing = valid.clone();
        missing["root_instance_probe"]
            .as_object_mut()
            .ok_or(E::Refused("fixture-probe-required"))?
            .remove("spawn_ready");
        assert!(probe_configuration(&missing).is_err());
        let result = PathBuf::from("/fixture/result.json");
        distinct_output_paths(&[
            &configured.queued,
            &configured.observation,
            &spawn_ready,
            &result,
        ])?;
        // Input::read uses this exact guard on ALL common and probe outputs.
        for collision in [&configured.queued, &configured.observation, &spawn_ready] {
            assert!(distinct_output_paths(&[
                &configured.queued,
                &configured.observation,
                &spawn_ready,
                collision
            ])
            .is_err());
        }
        Ok(())
    }
    /// A late public after-read failure must not discard earlier per-intention
    /// refusals; safe projection walks all branches and omits free phase text.
    #[test]
    fn continuation_retains_entry_causes_with_late_public_read_failure(
    ) -> std::result::Result<(), Box<dyn Error>> {
        let entry = |label| swarm_engine::MissionWorkerContinuationEntry {
            agent_id: "owned-fixture-agent".into(),
            task_id: "owned-fixture-task".into(),
            state: ContinuationState::Refused {
                phase: "PRIVATE_PHASE",
                cause: E::Refused(label),
            },
        };
        let error = UnexpectedContinuation {
            observation: MissionWorkerContinuationObservation {
                work: "owned-fixture-work".into(),
                revision: 1,
                settings_revision: swarm_engine::SettingCount(1),
                generation: 1,
                observed_at: "2026-10-22T00:00:00Z".into(),
                entries: vec![entry("fixture-first-proof"), entry("fixture-second-proof")],
            },
            document: json!({"status":"CONTINUATION_PUBLIC_AFTER_READ_FAILED"}),
            read_error: Some(E::Cleanup {
                primary: Some(Box::new(E::Refused("fixture-after-query"))),
                cleanup: Box::new(E::Refused("fixture-after-close")),
            }),
        };
        assert_eq!(error.errors().len(), 3);
        assert!(error.source().is_some());
        let projected = serde_json::to_string(&causes(&error))?;
        for label in [
            "fixture-first-proof",
            "fixture-second-proof",
            "fixture-after-query",
            "fixture-after-close",
        ] {
            assert!(
                projected.contains(label),
                "missing retained typed cause: {label}"
            );
        }
        assert!(!projected.contains("PRIVATE_PHASE"));
        assert!(!format!("{error:?}").contains("PRIVATE_PHASE"));
        Ok(())
    }
}
