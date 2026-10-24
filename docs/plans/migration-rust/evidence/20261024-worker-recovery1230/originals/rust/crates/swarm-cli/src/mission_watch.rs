//! Conductor with one owned asynchronous role phase and independent workers.
//! Complete Go role concurrency, registered/managed recovery, web integration
//! and global mission acceptance remain unestablished. The native web command
//! reuses the owned session below; server-wide Go parity is not established.
use crate::{Arguments, CliError};
use serde_json::{json, Value};
use std::{
    error::Error,
    fmt,
    future::{poll_fn, Future},
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    task::Poll,
    time::Duration,
};
use swarm_engine::{
    MissionCycleFailure, MissionCycleMonitor, MissionLease,
    MissionWorkerContinuationState as State, MissionWorkerDispatchOutcome as Dispatch, RootStore,
    RootStoreError as RootError, SystemEnvironment,
};
use tokio::signal::unix::{signal, Signal, SignalKind};
mod role_job;
/// Retain an owned-thread panic without rendering its potentially private payload.
pub(super) fn owned_panic(payload: Box<dyn std::any::Any + Send>) -> RootError {
    role_job::panicked(payload)
}

/// Private safe phase context. Debug/Display emit only its static code;
/// source retains the original typed cause, never its private formatted message.
struct PhaseError {
    code: &'static str,
    cause: Box<dyn Error + Send + Sync>,
}
impl fmt::Debug for PhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl fmt::Display for PhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl Error for PhaseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}
/// Project this module's static phase codes and actual numeric child wait status.
/// No error message or private Agent content is rendered. The source remains
/// traversable through native error links; these observations authorize no retry.
pub(super) fn safe_labels(error: &(dyn Error + 'static)) -> Vec<String> {
    if let Some(error) = error.downcast_ref::<PhaseError>() {
        return vec![error.code.into()];
    }
    if let Some(error) = error.downcast_ref::<ChildExit>() {
        return error
            .code
            .map(|n| format!("mission-worker-exit-code:{n}"))
            .into_iter()
            .chain(
                error
                    .signal
                    .map(|n| format!("mission-worker-exit-signal:{n}")),
            )
            .collect();
    }
    Vec::new()
}
/// Wrap a typed cause through the public runtime constructor. This grants no
/// retry and makes no no-effect claim; engine-private launch constructors stay private.
fn phase(code: &'static str, cause: impl Error + Send + Sync + 'static) -> RootError {
    RootError::Runtime(swarm_engine::RuntimeError::new(
        swarm_engine::RuntimeOperation::Platform,
        PhaseError {
            code,
            cause: Box::new(cause),
        },
    ))
}
/// Join independent failures through the established native cleanup tree. The
/// secondary branch is not proof that the original action was rolled back.
fn join(primary: Option<RootError>, secondary: RootError) -> RootError {
    match primary {
        None => secondary,
        Some(primary) => RootError::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(secondary),
        },
    }
}
/// Retain a known recorded/replayed admission after any later failure, without
/// inventing a Work revision. This is neither a new commit nor proof of rollback,
/// resource release, provider start or acceptance; no refund/cancel is performed.
fn preserve_admission(admission_recorded: bool, error: RootError) -> RootError {
    if admission_recorded {
        RootError::LifecycleRecorded {
            primary: Box::new(error),
        }
    } else {
        error
    }
}
/// Emit and flush a versioned local observation. This stream is not a durable
/// journal, launch grant or proof of downstream receipt; blocked sinks remain sync.
fn emit(out: &mut dyn Write, record: &Value) -> Result<(), RootError> {
    let bytes = serde_json::to_vec(record).map_err(|e| phase("mission-watch-json", e))?;
    out.write_all(&bytes)
        .and_then(|()| out.write_all(b"\n"))
        .and_then(|()| out.flush())
        .map_err(|e| phase("mission-watch-output", e))
}

/// Foreground OS signals or owned server shutdown end a successful session. Neither
/// an idle dispatch nor a worker exit establishes mission acceptance/completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stop {
    Interrupt,
    Terminate,
    /// Owned server lifetime ended; never described as an OS signal.
    ControllerShutdown,
}
impl Stop {
    /// Closed protocol vocabulary, never a user-supplied phase string.
    fn name(self) -> &'static str {
        match self {
            Self::Interrupt => "sigint",
            Self::Terminate => "sigterm",
            Self::ControllerShutdown => "controller-shutdown",
        }
    }
}
/// Tokio registration lives on its own IO thread so synchronous inference never
/// prevents delivery to the local interruption source. No Root, grant or timer is
/// owned by this thread; saved cadence/grace remain authoritative elsewhere.
struct SignalStreams {
    interrupt: Signal,
    terminate: Signal,
}
impl SignalStreams {
    /// Register before startup acknowledgement and before any lease or process.
    fn new(runtime: &tokio::runtime::Runtime) -> Result<Self, RootError> {
        let _entered = runtime.enter();
        Ok(Self {
            interrupt: signal(SignalKind::interrupt())
                .map_err(|e| phase("mission-watch-sigint", e))?,
            terminate: signal(SignalKind::terminate())
                .map_err(|e| phase("mission-watch-sigterm", e))?,
        })
    }
    /// Observe only actual delivered OS signals; a closed stream is an error.
    fn poll(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<Stop, RootError>> {
        for (stream, reason) in [
            (&mut self.interrupt, Stop::Interrupt),
            (&mut self.terminate, Stop::Terminate),
        ] {
            match stream.poll_recv(cx) {
                Poll::Ready(Some(())) => return Poll::Ready(Ok(reason)),
                Poll::Ready(None) => {
                    return Poll::Ready(Err(RootError::Refused(
                        "mission-watch-signal-stream-closed",
                    )))
                }
                Poll::Pending => {}
            }
        }
        Poll::Pending
    }
}
/// Retain an opaque original panic without rendering private payload contents.
struct SignalThreadPanic {
    _payload: std::sync::Mutex<Box<dyn std::any::Any + Send>>,
}
impl fmt::Debug for SignalThreadPanic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("mission-watch-signal-thread-panicked")
    }
}
impl fmt::Display for SignalThreadPanic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("mission-watch-signal-thread-panicked")
    }
}
impl Error for SignalThreadPanic {}
/// Execute the actual listener body and wake a scoped role on error or unwind.
/// Retain opaque panic data; this does not replace the process panic hook or
/// claim rollback. No authority follows; callers still owe acknowledgement/join.
fn run_signal_listener(
    source: &swarm_engine::ExecutionInterrupt,
    operation: impl FnOnce() -> Result<(), RootError>,
) -> Result<(), RootError> {
    let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(payload) => Err(phase(
            "mission-watch-signal-thread",
            SignalThreadPanic {
                _payload: std::sync::Mutex::new(payload),
            },
        )),
    };
    match result {
        Ok(()) => Ok(()),
        Err(error) => match source.request(swarm_engine::ExecutionInterruptReason::ListenerFailure)
        {
            Ok(()) => Err(error),
            Err(wake) => Err(join(
                Some(error),
                phase("mission-watch-signal-failure-wake", wake),
            )),
        },
    }
}
/// Own the acknowledged signal listener, its explicit shutdown and local source.
/// Cached delivered stop is terminal for this watch; no mission consent is changed.
struct Signals {
    source: swarm_engine::ExecutionInterrupt,
    received: std::sync::mpsc::Receiver<Stop>,
    stopped: Option<Stop>,
    quit: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<Result<(), RootError>>>,
}
impl Signals {
    /// Join actual listener result, preserving IO failures and opaque panic data.
    fn joined(handle: std::thread::JoinHandle<Result<(), RootError>>) -> Result<(), RootError> {
        match handle.join() {
            Ok(result) => result,
            Err(payload) => Err(phase(
                "mission-watch-signal-thread",
                SignalThreadPanic {
                    _payload: std::sync::Mutex::new(payload),
                },
            )),
        }
    }
    /// Acknowledge real Tokio registration before returning. A failed startup
    /// joins and retains its original cause; no unbounded retry is performed.
    fn new() -> Result<Self, RootError> {
        let source = swarm_engine::ExecutionInterrupt::new()
            .map_err(|e| phase("mission-watch-interruption-source", e))?;
        let observer = source.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (tx, received) = std::sync::mpsc::channel();
        let (quit, mut quit_rx) = tokio::sync::oneshot::channel();
        let handle = std::thread::Builder::new()
            .name("swarm-watch-signals".into())
            .spawn(move || {
                run_signal_listener(&observer, || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| phase("mission-watch-runtime", e))?;
                    let mut streams = SignalStreams::new(&runtime)?;
                    ready_tx
                        .send(())
                        .map_err(|e| phase("mission-watch-signal-ready", e))?;
                    let stop = runtime.block_on(poll_fn(|cx| {
                        if std::pin::Pin::new(&mut quit_rx).poll(cx).is_ready() {
                            return Poll::Ready(Ok(None));
                        }
                        match streams.poll(cx) {
                            Poll::Ready(result) => Poll::Ready(result.map(Some)),
                            Poll::Pending => Poll::Pending,
                        }
                    }))?;
                    if let Some(stop) = stop {
                        tx.send(stop)
                            .map_err(|e| phase("mission-watch-signal-publish", e))?;
                        observer
                            .request(match stop {
                                Stop::Interrupt => {
                                    swarm_engine::ExecutionInterruptReason::Interrupt
                                }
                                Stop::Terminate => {
                                    swarm_engine::ExecutionInterruptReason::Terminate
                                }
                                Stop::ControllerShutdown => swarm_engine::ExecutionInterruptReason::ControllerShutdown,
                            })
                            .map_err(|e| phase("mission-watch-interruption-publish", e))?;
                    }
                    Ok(())
                })
            })
            .map_err(|e| phase("mission-watch-signal-thread-spawn", e))?;
        if ready_rx.recv().is_err() {
            Self::joined(handle)?;
            return Err(RootError::Refused("mission-watch-signal-startup-missing"));
        }
        Ok(Self {
            source,
            received,
            stopped: None,
            quit: Some(quit),
            thread: Some(handle),
        })
    }
    /// Construct a local server-owned session without an OS listener. The owner
    /// retains the sender and requests the matching ExecutionInterrupt before
    /// publishing stop; channel loss remains a failure. No process is signalled.
    fn local(source: swarm_engine::ExecutionInterrupt, received: std::sync::mpsc::Receiver<Stop>) -> Self {
        Self { source, received, stopped: None, quit: None, thread: None }
    }
    /// Observe a delivered stop without waiting; disconnected listener is a
    /// failure, never a fabricated normal shutdown. Its joined cause is retained.
    fn ready(&mut self) -> Result<Option<Stop>, RootError> {
        if self.stopped.is_some() {
            return Ok(self.stopped);
        }
        match self.received.try_recv() {
            Ok(stop) => {
                self.stopped = Some(stop);
                Ok(Some(stop))
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.close()?;
                Err(RootError::Refused("mission-watch-signal-stream-closed"))
            }
        }
    }
    /// Wait only the saved conductor cadence. Timeout is a scheduled cycle, not
    /// a provider duration cap or verification watchdog; no extra poll is added.
    fn wait(&mut self, cadence: Duration) -> Result<Option<Stop>, RootError> {
        if let Some(stop) = self.ready()? {
            return Ok(Some(stop));
        }
        match self.received.recv_timeout(cadence) {
            Ok(stop) => {
                self.stopped = Some(stop);
                Ok(Some(stop))
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                self.close()?;
                Err(RootError::Refused("mission-watch-signal-stream-closed"))
            }
        }
    }
    /// Withdraw listener wait and join it. A closed quit receiver only says the
    /// listener already ended; the join result still decides success/failure.
    /// No signal/Root/provider is invented from shutdown-channel closure.
    fn close(&mut self) -> Result<(), RootError> {
        if let Some(quit) = self.quit.take() {
            match quit.send(()) {
                Ok(()) | Err(()) => {}
            }
        }
        self.thread.take().map_or(Ok(()), Self::joined)
    }
}
/// Use only the claim's validated persisted policy, with no new retry/default
/// delay. Conversion errors retain their cause instead of clamping a policy.
pub(super) fn cadence(policy: &swarm_engine::MissionSupervisionPolicy) -> Result<Duration, RootError> {
    policy.validate().map_err(RootError::Settings)?;
    Ok(Duration::from_millis(
        u64::try_from(policy.poll_interval_ms.0)
            .map_err(|e| phase("mission-watch-cadence-invalid", e))?,
    ))
}

/// Pure candidate classification only. A non-stopped queued intention from a
/// previous generation still needs authoritative engine adoption before spawn.
/// Existing current-generation queued or registered processes are never adopted.
fn adoption_candidate(known_child: bool, state: &State) -> bool {
    !known_child
        && matches!(
            state,
            State::QueuedUnregisteredObserved {
                current_lease_generation: false,
                ..
            }
        )
}
/// Retained queued entries outside the narrow adoption candidate still refuse.
/// Registered entries may be observed without an owned child handle: this never
/// adopts, relaunches, waits for or signals them, and still defers planning. The
/// engine retains canonical resource accounting. A candidate is not permission;
/// adoption and malformed/unproven observations can still refuse.
fn retained_allowed(known_child: bool, state: &State) -> Result<(), &'static str> {
    match state {
        State::QueuedUnregisteredObserved { .. }
        | State::StopRequestedUnregisteredObserved { .. }
            if !known_child && !adoption_candidate(known_child, state) =>
        {
            Err("mission-watch-retained-queued-requires-recovery")
        }
        State::Refused { .. } => Err("mission-watch-continuation-refused"),
        _ => Ok(()),
    }
}
/// One actual child handle, never a PID-only signalling capability. Dropping a
/// still-running handle on watch exit neither kills nor waits for that worker.
struct Worker {
    agent: String,
    process: Child,
}
/// Build the existing private bootstrap argv with three null streams, as the
/// Go launcher does. Ordinary inherited PGID is essential: child setsid owns its
/// independent session; process_group/pre_exec/shell are deliberately absent.
fn worker_command(executable: &Path, root: &Path, agent: &str) -> Command {
    let mut command = Command::new(executable);
    command
        .arg("_mission_worker_entry")
        .arg(agent)
        .arg("--root")
        .arg(root)
        .arg("--json")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}
/// Spawn one existing admitted/adopted Agent through the unchanged private
/// bootstrap and track the real Child before output can fail. The caller checks
/// signals/heartbeat; the independent child gate revalidates all execution rights.
fn start_worker(
    executable: &Path,
    root: &Path,
    agent: &str,
    work: &str,
    children: &mut Vec<Worker>,
    out: &mut dyn Write,
) -> Result<(), RootError> {
    let child = worker_command(executable, root, agent)
        .spawn()
        .map_err(|error| phase("mission-watch-worker-spawn", error))?;
    let pid = child.id();
    children.push(Worker {
        agent: agent.into(),
        process: child,
    });
    emit(
        out,
        &json!({"schema_version":1,"event":"mission-worker-started","work":work,"agent_id":agent,
        "supervisor_pid":pid,"session_state":"pending","provider_start_verified":false,"mission_complete":false,"observation_is_permission":false}),
    )
}
/// Retain real exit coordinates behind a static diagnostic. This is a child
/// wait status, not provider success, terminal Agent proof or mission acceptance.
#[derive(Debug)]
struct ChildExit {
    code: Option<i32>,
    signal: Option<i32>,
}
impl fmt::Display for ChildExit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("mission worker process exited unsuccessfully")
    }
}
impl Error for ChildExit {}
/// Try-reap every owned handle once without blocking, emit actual exit status and
/// retain all IO/output/failure causes. A failed child never becomes silent success.
/// Zero-exit active/queued Agents remain explicit incomplete-runtime failures.
fn reap(
    store: &RootStore,
    work: &str,
    children: &mut Vec<Worker>,
    out: &mut dyn Write,
) -> Result<(), RootError> {
    use std::os::unix::process::ExitStatusExt;
    let mut failure = None;
    let mut index = 0;
    while index < children.len() {
        let status = match children[index].process.try_wait() {
            Ok(None) => {
                index += 1;
                continue;
            }
            Ok(Some(status)) => status,
            Err(error) => {
                failure = Some(join(failure, phase("mission-watch-child-try-wait", error)));
                index += 1;
                continue;
            }
        };
        let child = children.remove(index);
        let record = json!({"schema_version":1,"event":"mission-worker-exited","work":work,"agent_id":child.agent,
            "supervisor_pid":child.process.id(),"exit_code":status.code(),"exit_signal":status.signal(),"mission_complete":false});
        if let Err(error) = emit(out, &record) {
            failure = Some(join(failure, error));
        }
        if !status.success() {
            failure = Some(join(
                failure,
                phase(
                    "mission-watch-worker-exited-unsuccessfully",
                    ChildExit {
                        code: status.code(),
                        signal: status.signal(),
                    },
                ),
            ));
        }
        match store.read_agent(&child.agent) {
            Ok(agent) if agent.is_active() => {
                failure = Some(join(
                    failure,
                    RootError::Refused("mission-watch-child-exited-with-active-intention"),
                ))
            }
            Ok(_) => {}
            Err(error) => failure = Some(join(failure, error)),
        }
    }
    failure.map_or(Ok(()), Err)
}
/// Observe before every dispatch/adoption without grant or cancellation. Return
/// old-generation queued candidate IDs and whether any automatic worker still
/// retains an active context; the actual adoption API validates
/// ownership, receipt and current controls. All semantic sibling refusals survive.
fn continuation(
    store: &RootStore,
    lease: &MissionLease,
    children: &[Worker],
) -> Result<(Vec<String>, bool), RootError> {
    let observed = store
        .observe_mission_worker_continuation(lease)
        .map_err(|error| phase("mission-watch-continuation", *error))?;
    let mut failure = None;
    let mut candidates = Vec::new();
    let mut active_context = false;
    for entry in observed.entries {
        active_context |= planning_context_pending(&entry.state);
        let known = children.iter().any(|child| child.agent == entry.agent_id);
        if adoption_candidate(known, &entry.state) {
            candidates.push(entry.agent_id.clone());
        }
        if let Err(code) = retained_allowed(known, &entry.state) {
            let error = match entry.state {
                State::Refused { cause, .. } => phase("mission-watch-continuation-refused", cause),
                _ => RootError::Refused(code),
            };
            failure = Some(join(failure, error));
        }
    }
    failure.map_or(Ok((candidates, active_context)), Err)
}

/// Classify retained automatic context conservatively, without inferring exec
/// from PID/registration/permission. Manual and proved terminal states do not
/// defer planning; semantic refusals remain errors in the continuation consumer.
fn planning_context_pending(state: &State) -> bool {
    matches!(
        state,
        State::QueuedUnregisteredObserved { .. }
            | State::StopRequestedUnregisteredObserved { .. }
            | State::RegisteredObserved
    )
}

/// Observe the Go automatic-entry controls without granting planner permission.
/// Missing controls never imply consent. Native scope/profile/budget/private-gate
/// checks remain authoritative; there is no conductor-lease identity at that gate.
fn planning_entry(
    control: &swarm_engine::MissionControlView,
    pending: bool,
) -> Option<&'static str> {
    if pending {
        Some("deferred-active-worker-context")
    } else if control.archived {
        Some("mission-archived")
    } else if control.paused != Some(false) {
        Some("mission-paused-or-unspecified")
    } else if control.consent_enabled != Some(true) {
        Some("mission-consent-not-enabled")
    } else if control.autonomy != Some("autonome") {
        Some("mission-not-automatic")
    } else {
        None
    }
}

/// Project a coherent step without serializing its private Work/prompt. A genuine
/// idle has no claim/run; refusal remains an error. This observation grants no
/// execution, acceptance or completion and contains no invented run defaults.
fn planning_record(
    step: &swarm_engine::PlanningStepOutcome,
    work: &str,
    revision: i64,
) -> Result<Value, RootError> {
    let applied = match (step.status, step.run.as_ref()) {
        ("idle", None)
            if step.selected_scope.is_none()
                && step.input_event.is_none()
                && step.claim_id.is_none() =>
        {
            false
        }
        ("applied", Some(run))
            if run.work.id == work
                && step.claim_id.as_ref() == Some(&run.claim_id)
                && step.selected_scope.is_some()
                && step.input_event.is_some() =>
        {
            run.decision_applied
        }
        _ => {
            return Err(RootError::Refused(
                "mission-watch-planning-result-incoherent",
            ))
        }
    };
    Ok(
        json!({"schema_version":1,"event":"mission-planning-step","work":work,"status":step.status,
        "selected_scope":step.selected_scope,"input_event":step.input_event,"claim_id":step.claim_id,
        "settings_revision":step.settings_revision,"decision_applied":applied,"work_revision":revision,
        "mission_complete":false,"automatic_planner":true,"automatic_reviewer":false,"automatic_validation":false,"observation_is_permission":false}),
    )
}

/// Execute at most one configured foreground activation in this heartbeat pass.
/// Any queued/registered automatic worker may retain its immutable revision
/// window (registration is not exec proof), so planning is
/// visibly deferred until no active worker context remains. No cancellation/refund/retry is
/// introduced. W1/W2 without configured planning remain unchanged. WorkRecorded
/// from the engine and known returned Work survive later read/render failures.
/// The selected planner job calls this synchronously in its owned peer, while
/// main only uses pending=true for non-executing deferral observations. Existing
/// cleanup handles scoped stops; complete Go simultaneous roles remain unported.
fn planning_once(
    store: &mut RootStore,
    work: &str,
    pending: bool,
    out: &mut dyn Write,
    recorded: &mut Option<(String, i64)>,
) -> Result<(), RootError> {
    let observed = store.read_work(work)?;
    if observed
        .planning
        .as_ref()
        .is_none_or(|p| p.provider.is_empty())
    {
        return Ok(());
    }
    let control = store
        .mission_control_view(work)
        .map_err(|e| phase("mission-watch-planning-controls", e))?;
    let reason = if observed.planning.as_ref().is_some_and(|p| p.paused) {
        Some("planning-paused")
    } else {
        planning_entry(&control, pending)
    };
    if let Some(reason) = reason {
        return emit(
            out,
            &json!({"schema_version":1,"event":"mission-watch-blocked","work":work,"reason":reason,
            "automatic_planner":true,"automatic_reviewer":false,"automatic_validation":false,"mission_complete":false,"observation_is_permission":false}),
        );
    }
    let step = store.step_planning(work)?;
    if let Some(run) = &step.run {
        *recorded = Some((run.work.id.clone(), run.work.revision));
    }
    let current = store.read_work(work)?;
    let record = planning_record(&step, work, current.revision)?;
    emit(out, &record)
}

/// Preserve the latest returned planner or report-relay Work through every later failure,
/// including dispatch, supervision, cleanup and final publication. This records
/// known state, not a new commit/rollback or a claim that the revision is current.
fn preserve_planning(recorded: Option<&(String, i64)>, result: RootError) -> RootError {
    match recorded {
        Some((work, revision)) => RootError::WorkRecorded {
            primary: Box::new(result),
            work: work.clone(),
            revision: *revision,
        },
        None => result,
    }
}

/// Project every queue/resource refusal separately, never private Display text.
/// These effects are observations, not authorization or an assertion of rollback.
fn dispatch_record(outcome: &Dispatch, work: &str) -> Value {
    let (queue, resources) = match outcome {
        Dispatch::Prepared {
            queue, resources, ..
        }
        | Dispatch::NoAdmission { queue, resources } => (Some(queue), resources),
        Dispatch::Replayed {
            queue, resources, ..
        } => (queue.as_ref(), resources),
    };
    let permission = queue
        .and_then(|queue| queue.permission_refusal.as_ref())
        .map(|error| crate::safe_causes(error));
    let refused: Vec<_> = queue
        .into_iter()
        .flat_map(|queue| queue.refused.iter())
        .map(|entry| json!({"task_id":entry.task,"causes":crate::safe_causes(&entry.error)}))
        .collect();
    let resources: Vec<_> = resources
        .iter()
        .map(|resource| match resource {
            swarm_engine::MissionDispatchResourceRefusal::Fifo { task, blocker } => {
                json!({"task_id":task,"kind":"fifo","blocker":blocker})
            }
            swarm_engine::MissionDispatchResourceRefusal::Occupied { task, cause } => {
                json!({"task_id":task,"kind":"occupied","causes":crate::safe_causes(cause)})
            }
            swarm_engine::MissionDispatchResourceRefusal::Capacity { task, cause } => {
                json!({"task_id":task,"kind":"capacity","causes":crate::safe_causes(cause)})
            }
        })
        .collect();
    let has_refusal = permission.is_some() || !refused.is_empty() || !resources.is_empty();
    json!({"schema_version":1,"event":"mission-watch-blocked","work":work,
        "reason":if has_refusal {"dispatch-refused-or-blocked"} else {"no-ready-worker"},
        "synchronization_committed":queue.is_some(),"work_revision":queue.map(|q|q.revision),
        "ready_count":queue.map(|q|q.ready.len()),"permission_refusal":permission,"queue_refusals":refused,"resource_refusals":resources,
        "automatic_planner":true,"automatic_reviewer":false,"automatic_validation":false,"mission_complete":false,"observation_is_permission":false})
}
/// Preserve refusal causes if publication fails after committed synchronization
/// or admission. Other outcomes remain typed until the operation is consumed.
fn dispatch_causes(outcome: Dispatch, publication: RootError) -> RootError {
    let (queue, resources) = match outcome {
        Dispatch::Prepared {
            queue, resources, ..
        }
        | Dispatch::NoAdmission { queue, resources } => (Some(queue), resources),
        Dispatch::Replayed {
            queue, resources, ..
        } => (queue, resources),
    };
    let mut result = publication;
    if let Some(queue) = queue {
        if let Some(error) = queue.permission_refusal {
            result = join(Some(result), phase("mission-watch-permission", error));
        }
        for entry in queue.refused {
            result = join(Some(result), phase("mission-watch-candidate", entry.error));
        }
    }
    for resource in resources {
        match resource {
            swarm_engine::MissionDispatchResourceRefusal::Occupied { cause, .. }
            | swarm_engine::MissionDispatchResourceRefusal::Capacity { cause, .. } => {
                result = join(Some(result), cause)
            }
            swarm_engine::MissionDispatchResourceRefusal::Fifo { .. } => {}
        }
    }
    result
}
/// Consume only a live publisher's exact clean interruption plus our actual
/// delivered signal. Preserve known Work before output. Cleanup/other errors
/// remain failures; an output failure retains the original interruption and known
/// Work in its error chain. Stopping accepts no task and changes no consent.
fn interrupted_phase(
    error: RootError,
    work: &str,
    signals: &mut Signals,
    recorded: &mut Option<(String, i64)>,
    out: &mut dyn Write,
) -> Result<Stop, RootError> {
    if let Some((role, known, revision)) = error
        .recorded_execution_interruption()
        .filter(|(_, known, _)| *known == work)
    {
        let event = match role {
            "planner" => "mission-planning-interrupted",
            "reviewer" => "mission-review-interrupted",
            "controls" => "mission-controls-interrupted",
            _ => return Err(error),
        };
        if recorded
            .as_ref()
            .is_none_or(|(work, prior)| work != known || revision > *prior)
        {
            *recorded = Some((known.to_owned(), revision));
        }
        match signals.ready() {
            Ok(Some(stop)) => {
                let record = json!({"schema_version":1,"event":event,"phase":role,"work":known,"revision":revision,"reason":stop.name(),"failed_call_recorded":true,"decision_applied":false,"task_accepted":false,"mission_complete":false});
                if let Err(output_error) = emit(out, &record) {
                    return Err(join(Some(error), output_error));
                }
                return Ok(stop);
            }
            Ok(None) => {}
            Err(signal_error) => return Err(join(Some(error), signal_error)),
        }
    }
    Err(error)
}
/// Borrowed resources of one caller-owned foreground session. Grouping them
/// adds no defaults or policy and transfers no lease, process or signal authority.
struct SessionIo<'a> {
    root: &'a Path,
    executable: &'a Path,
    signals: &'a mut Signals,
    children: &'a mut Vec<Worker>,
    admission_recorded: &'a mut bool,
    planning_recorded: &'a mut Option<(String, i64)>,
    out: &'a mut dyn Write,
}

/// Relay real completed worker handoffs, including previous watch generations.
/// The engine owns selection and the transaction fence. Waiting is not permission
/// or completion; its changed reason is printed once per session/Agent. The
/// transient map is an output deduplicator, not retention policy or authority.
/// Errors retain original causes.
/// The shared session also retains known relay state through later phase errors.
/// A failed stdout write after submission preserves its known Work revision.
fn relay_reports(
    store: &mut RootStore,
    lease: &MissionLease,
    work: &str,
    out: &mut dyn Write,
    waiting: &mut std::collections::BTreeMap<String, &'static str>,
    recorded: &mut Option<(String, i64)>,
) -> Result<(), RootError> {
    for agent in store.list_agents(work)? {
        if agent.status != "completed" || agent.role != "worker" {
            continue;
        }
        match store.relay_mission_report(lease, &agent.id)? {
            swarm_engine::MissionReportRelayOutcome::Submitted {
                work,
                revision,
                agent_id,
                report,
            } => {
                *recorded = Some((work.clone(), revision));
                waiting.remove(&agent.id);
                let record = json!({"schema_version":1,"event":"mission-report-submitted","work":work,"revision":revision,"agent_id":agent_id,"report":report,"task_accepted":false,"automatic_validation":false,"automatic_reviewer":false,"mission_complete":false});
                if let Err(primary) = emit(out, &record) {
                    return Err(RootError::WorkRecorded {
                        primary: Box::new(primary),
                        work,
                        revision,
                    });
                }
            }
            swarm_engine::MissionReportRelayOutcome::Waiting { reason } => {
                if waiting.get(&agent.id).copied() != Some(reason) {
                    emit(
                        out,
                        &json!({"schema_version":1,"event":"mission-report-waiting","work":work,"agent_id":agent.id,"reason":reason,"mission_complete":false}),
                    )?;
                    waiting.insert(agent.id, reason);
                }
            }
            swarm_engine::MissionReportRelayOutcome::NotApplicable => {
                waiting.remove(&agent.id);
            }
        }
    }
    Ok(())
}

/// Only a pre-synchronization revision observation with no attempted preparation,
/// resources or compound cause permits a fresh scheduled observation. A committed,
/// uncertain, replay/preview/admission or cleanup failure remains fatal. Classification
/// uses typed protocol variants, never formatted error text or a generic source walk.
fn stale_dispatch_observation(error: &swarm_engine::MissionWorkerDispatchFailure) -> bool {
    error.phase == "revision"
        && matches!(error.queue, swarm_engine::MissionDispatchQueueEffect::NotAttempted)
        && matches!(error.preparation, swarm_engine::MissionDispatchPreparationEffect::NotAttempted)
        && error.resources.is_empty()
        && matches!(&error.cause, swarm_engine::MissionDispatchCause::Mission(cause)
            if matches!(cause.as_ref(), swarm_engine::MissionSupervisionError::Refused("revision_conflict")))
}

/// The heartbeat stays active for the entire gate/loop, not just the admission.
/// Worker work begins only through dispatch or queued-v4 adoption plus the private CLI
/// bootstrap. Each iteration re-observes retained state without changing Binding4. Missing reviewer/validation roles produce a visible blocked
/// state; no idle/terminal worker state is promoted to complete mission success.
/// One owned controls/reviewer/planner peer may block while main observes/dispatches.
/// Completion is polled at saved cadence, joined and published by this thread.
/// Completed control/review outcomes drain in order before a planner activation.
/// At most one role phase is owned, preserving the prior role-writer exclusion;
/// complete Go simultaneous-role scheduling remains unported.
fn drive(
    store: &mut RootStore,
    lease: &MissionLease,
    monitor: &MissionCycleMonitor,
    work: &str,
    session: SessionIo<'_>,
) -> Result<Stop, RootError> {
    let SessionIo {
        root,
        executable,
        signals,
        children,
        admission_recorded,
        planning_recorded,
        out,
    } = session;
    let cadence = cadence(lease.policy())?;
    let mut last_blocked = None;
    let mut pending_dispatch: Option<swarm_engine::MissionWorkerDispatchRequest> = None;
    let mut waiting_reports = std::collections::BTreeMap::new();
    std::thread::scope(|scope| {
        let mut job = None;
        let mut role = role_job::Role::Controls;
        let main = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    loop {
        if !monitor.is_running() {
            return Err(RootError::Refused("mission-watch-heartbeat-not-running"));
        }
        if let Some(stop) = signals.ready()? {
            return Ok(stop);
        }
        if job.as_ref().is_some_and(std::thread::ScopedJoinHandle::is_finished) {
            let report = role_job::joined(job.take().expect("checked owned role"));
            let produced = report.recorded.is_some();
            if let Err(error) = role_job::publish(report,work,planning_recorded,out) {
                return interrupted_phase(error,work,signals,planning_recorded,out);
            }
            role = role.next(produced);
        }
        reap(store, work, children, out)?;
        relay_reports(
            store,
            lease,
            work,
            out,
            &mut waiting_reports,
            planning_recorded,
        )?;
        let (candidates, active_context) = continuation(store, lease, children)?;
        // Deferral is an observation of retained worker state, not a role job.
        // Keep it visible before dispatch regardless of the current async phase.
        if active_context {
            planning_once(store,work,true,out,planning_recorded)?;
        }
        if job.is_none() {
            // Existing roles remain ordered; worker observation/dispatch continues
            // during each owned phase. No simultaneous role writer is introduced.
            if role == role_job::Role::Planner && active_context {
                role = role_job::Role::Controls;
            } else {
                job=Some(role_job::start(scope,role,root.to_owned(),work.to_owned(),lease,signals.source.clone())?);
            }
        }
        if !monitor.is_running() {
            return Err(RootError::Refused("mission-watch-heartbeat-not-running"));
        }
        if let Some(stop) = signals.ready()? {
            return Ok(stop);
        }
        for agent_id in candidates {
            if !monitor.is_running() {
                return Err(RootError::Refused("mission-watch-heartbeat-not-running"));
            }
            if let Some(stop) = signals.ready()? {
                return Ok(stop);
            }
            let request = swarm_engine::MissionWorkerAdoptionRequest {
                schema_version: 1,
                event_id: swarm_engine::generate_identifier("watch-adopt-")
                    .map_err(|error| phase("mission-watch-adoption-event-id", error))?,
                expected_revision: store.read_work(work)?.revision,
                agent_id: agent_id.clone(),
            };
            let adopted = store
                .adopt_mission_worker(lease, &request)
                .map_err(|error| phase("mission-watch-adoption", error))?;
            // This records continued ownership of an existing admission, not a
            // new attempt, reservation, FIFO acquisition or reset of consumption.
            *admission_recorded = true;
            if adopted.agent.id != agent_id
                || adopted.agent.status != "queued"
                || adopted.agent.supervisor_pid != 0
                || adopted.agent.child_pid != 0
                || adopted.generation != lease.generation()
            {
                return Err(RootError::Refused(
                    "mission-watch-adoption-result-incoherent",
                ));
            }
            emit(
                out,
                &json!({"schema_version":1,"event":"mission-worker-adopted","work":work,
                "agent_id":adopted.agent.id,"generation":adopted.generation,"event_id":adopted.event_id,
                "created":adopted.created,"provider_start_verified":false,"mission_complete":false,
                "observation_is_permission":false}),
            )?;
            if !monitor.is_running() {
                return Err(RootError::Refused("mission-watch-heartbeat-not-running"));
            }
            if let Some(stop) = signals.ready()? {
                return Ok(stop);
            }
            start_worker(executable, root, &agent_id, work, children, out)?;
            last_blocked = None;
        }
        let revision = store.read_work(work)?.revision;
        let request = match pending_dispatch.take() {
            Some(mut request) => { request.expected_revision = revision; request }
            None => swarm_engine::MissionWorkerDispatchRequest {
                schema_version: 1,
                event_id: swarm_engine::generate_identifier("watch-")
                    .map_err(|e| phase("mission-watch-event-id", e))?,
                expected_revision: revision,
            },
        };
        let outcome = match store.dispatch_mission_worker(lease, &request, &mut SystemEnvironment) {
            Ok(outcome) => outcome,
            Err(error) if stale_dispatch_observation(&error) => {
                let error = phase("mission-watch-dispatch", *error);
                let observation=json!({"schema_version":1,"event":"mission-watch-blocked","work":work,
                    "reason":"work-revision-changed","event_id":request.event_id,"expected_revision":request.expected_revision,
                    "synchronization_committed":false,"preparation_attempted":false,"causes":crate::safe_causes(&error),
                    "mission_complete":false,"observation_is_permission":false});
                if let Err(publication)=emit(out,&observation) { return Err(join(Some(error),publication)); }
                // Keep operation identity. Only a later scheduled cycle rereads the
                // actual Work; this never retries a recorded/uncertain preparation.
                pending_dispatch=Some(request);
                if let Some(stop)=signals.wait(cadence)? { return Ok(stop); }
                continue;
            }
            Err(error) => return Err(phase("mission-watch-dispatch", *error)),
        };
        // The actual outcome establishes admission/replay before output, spawn
        // or any other fallible consumer; a later error cannot erase this fact.
        if matches!(
            &outcome,
            Dispatch::Prepared { .. } | Dispatch::Replayed { .. }
        ) {
            *admission_recorded = true;
        }
        let diagnostic = dispatch_record(&outcome, work);
        let no_admission = matches!(&outcome, Dispatch::NoAdmission { .. });
        let has_refusals = diagnostic["reason"] == "dispatch-refused-or-blocked";
        if (no_admission || has_refusals) && last_blocked.as_ref() != Some(&diagnostic) {
            if let Err(error) = emit(out, &diagnostic) {
                return Err(dispatch_causes(outcome, error));
            }
            last_blocked = Some(diagnostic);
        }
        match outcome {
            Dispatch::Prepared { intention, .. } => {
                if !intention.created
                    || intention.preview_only
                    || intention.agent.status != "queued"
                {
                    return Err(RootError::Refused("mission-watch-dispatch-not-new-queued"));
                }
                if !monitor.is_running() {
                    return Err(RootError::Refused("mission-watch-heartbeat-not-running"));
                }
                if let Some(stop) = signals.ready()? {
                    return Ok(stop);
                }
                start_worker(executable, root, &intention.agent.id, work, children, out)?;
                last_blocked = None;
            }
            Dispatch::Replayed { .. } => {
                return Err(RootError::Refused(
                    "mission-watch-unexpected-dispatch-replay",
                ))
            }
            Dispatch::NoAdmission { .. } => {}
        }
        if let Some(stop) = signals.wait(cadence)? {
            return Ok(stop);
        }
    }
    })).unwrap_or_else(|payload|Err(role_job::panicked(payload)));
        match job {
            Some(handle) => {
                role_job::finish(handle, main, work, signals, planning_recorded, out)
            }
            None => main,
        }
    })
}
/// Decompose the engine cycle's known action/supervision/journal branches into
/// native compound errors; source() alone would discard independent failures.
/// A successful callback followed by failure remains an unusable cycle, never a
/// successful watch. This starts no retry and preserves original typed causes.
fn cycle_failure(error: MissionCycleFailure<Stop>) -> RootError {
    let succeeded = error.action.as_ref().is_some_and(Result::is_ok);
    let mut result = error.action.and_then(Result::err);
    if let Some(error) = error.supervision {
        result = Some(join(result, phase("mission-watch-supervision", error)));
    }
    if let Some(error) = error.journal {
        result = Some(join(result, phase("mission-watch-cycle-journal", error)));
    }
    let result = result.unwrap_or(RootError::Refused("mission-watch-cycle-result-missing"));
    if succeeded {
        phase("mission-watch-cycle-after-action", result)
    } else {
        result
    }
}

/// Run an explicit foreground partial watch with no input document. Install
/// signal handling, open a real Root, claim a real lease and dispatch through the
/// engine. Previous-generation queued v4 intentions use the engine's append-only
/// adoption API, then the same helper; no registered process is adopted, and no
/// attempt, consumption, reservation or FIFO acquisition is manufactured here.
/// Null child streams follow the Go launch contract; independent worker
/// stderr is not a diagnostic pipe. Real effects remain in engine history.
/// A Prepared/Replayed admission is retained through LifecycleRecorded on any
/// later failure, including final output, without inventing a Work revision,
/// new commit, acceptance or rollback proof. It neither cancels nor refunds.
/// SIGINT/SIGTERM stop an owned foreground planner, control or reviewer via TERM/grace/
/// wait/failed-receipt paths before releasing the conductor; only clean durably
/// recorded interruption plus actual delivered stop returns0. Granted independent
/// workers are not killed/waited. Other cleanup failures remain errors. Full
/// Go asynchronous scheduling remains open; cancellation is not its substitute.
/// JSON mode is a versioned JSONL extension, not parity with Go's text watch.
/// Known returned planner Work retains the highest same-Work revision on subsequent
/// failure, independently of worker admission.
/// Plain mode currently uses the same explicit observation lines. No claim of
/// complete mission or public acceptance. Controls, review and planner phases
/// run in one owned scoped peer at a time under the actual lease/consent gates;
/// deferral for active workers is observed independently before each dispatch;
/// JSON phase observations grant no permission; native scope/gate guards govern
/// actual execution. Legacy automatic_* flags describe the emitting phase, not
/// all capabilities of this conductor; phase-specific recorded events prove runs.
/// Planning is deferred for every queued/registered active automatic worker;
/// registration/permission are not proof of provider exec or context release.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.pos[0] != "mission"
        || a.pos[1] != "watch"
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let root = PathBuf::from(&a.root)
        .canonicalize()
        .map_err(|e| CliError::Root(Box::new(phase("mission-watch-root-path", e))))?;
    let executable = std::env::current_exe()
        .map_err(|e| CliError::Root(Box::new(phase("mission-watch-current-exe", e))))?;
    let mut signals = Signals::new().map_err(|e| CliError::Root(Box::new(e)))?;
    let store = match RootStore::open(&root, true) {
        Ok(store) => store,
        Err(primary) => {
            return Err(CliError::Root(Box::new(match signals.close() {
                Ok(()) => primary,
                Err(secondary) => join(Some(primary), secondary),
            })))
        }
    };
    session(&root, &executable, &a.pos[2], signals, store, None, out)
}

/// Run the existing conductor with caller-owned Root, executable and local stop.
/// The server must bind this Root to its own datastore before calling and keep
/// the sender alive until stop/join. This claims the ordinary native lease;
/// all dispatch, role, identity and budget guards remain authoritative. No OS
/// listener, new policy or output sink is installed. Errors retain known effects
/// and cleanup branches; independent granted workers are neither killed nor waited.
/// Ok(false) means an active competing lease was observed and no session ran;
/// Ok(true) means the owned session stopped or a pre-start local stop was observed.
pub(super) fn owned_session(
    root: &Path, executable: &Path, work: &str, store: RootStore,
    source: swarm_engine::ExecutionInterrupt,
    received: std::sync::mpsc::Receiver<Stop>, out: &mut dyn Write,
) -> Result<bool, CliError> {
    match source.reason() {
        Ok(None) => {},
        Ok(Some(_)) => return store.close().map(|_| true).map_err(|e|CliError::Root(Box::new(e))),
        Err(error) => {
            let error=phase("web-conductor-stop-observation",error);
            return Err(CliError::Root(Box::new(match store.close(){Ok(())=>error,Err(close)=>join(Some(error),close)})));
        }
    }
    let claim = swarm_engine::system_timestamp()
        .map_err(|e| phase("web-conductor-clock", e))
        .and_then(|now| store.claim_mission_supervision(work, "swarm-rust web", &now)
            .map_err(|e| RootError::Mutation(swarm_engine::WorkEngineError::Mission(Box::new(e)))));
    match claim {
        Ok(lease) => session(root, executable, work, Signals::local(source, received), store, Some(lease), out).map(|_| true),
        Err(error) => {
            let busy = matches!(error, RootError::Mutation(swarm_engine::WorkEngineError::Mission(ref cause))
                if matches!(cause.as_ref(), swarm_engine::MissionSupervisionError::Refused("mission-lease-already-held")));
            // A busy lease is an observation only; any close failure is still fatal.
            let failure = match store.close() {
                Ok(()) if busy => return Ok(false),
                Ok(()) => error,
                Err(close) => join(Some(error), close),
            };
            Err(CliError::Root(Box::new(failure)))
        }
    }
}

/// Shared CLI/server session lifecycle. Signals and Root are already owned;
/// claim, heartbeat, role drain, lease release and explicit close preserve their
/// existing order and original causes. A pre-start stop acquires no lease.
fn session(root: &Path, executable: &Path, work: &str, mut signals: Signals,
    mut store: RootStore, mut lease: Option<MissionLease>, out: &mut dyn Write) -> Result<u8, CliError> {
    let mut children = Vec::new();
    let mut admission_recorded = false;
    let mut planning_recorded = None;
    let result = (|| {
        if let Some(stop) = signals.ready()? { return Ok(stop); }
        if lease.is_none() {
        let owned = store
            .claim_mission_supervision(
                work,
                "swarm-rust mission watch",
                &swarm_engine::system_timestamp().map_err(|e| phase("mission-watch-clock", e))?,
            )
            .map_err(|e| phase("mission-watch-claim", e))?;
        lease = Some(owned);
        }
        let owned = lease
            .as_ref()
            .ok_or(RootError::Refused("mission-watch-lease-missing"))?;
        // Validate the retained candidate list before announcing watch readiness;
        // actual adoption/spawn occurs under run_mission_cycle's live heartbeat.
        continuation(&store, owned, &children)?;
        emit(
            out,
            &json!({"schema_version":1,"event":"mission-watch-started","work":work,"generation":owned.generation(),
            "poll_interval_ms":owned.policy().poll_interval_ms,"settings_revision":owned.settings_revision(),
            "mission_complete":false,"observation_is_permission":false}),
        )?;
        let interruption = signals.source.clone();
        store
            .with_execution_interrupt(&interruption, |store| {
                store.run_mission_cycle(owned, |store, owned, monitor| {
                    drive(
                        store,
                        owned,
                        monitor,
                        work,
                        SessionIo {
                            root,
                            executable,
                            signals: &mut signals,
                            children: &mut children,
                            admission_recorded: &mut admission_recorded,
                            planning_recorded: &mut planning_recorded,
                            out,
                        },
                    )
                })
            })
            .map_err(|error| cycle_failure(*error))
    })();
    let (stop, mut failure) = match result {
        Ok(stop) => (Some(stop), None),
        Err(error) => (None, Some(error)),
    };
    if let Err(error) = signals.close() {
        failure = Some(join(failure, error));
    }
    if let Err(error) = reap(&store, work, &mut children, out) {
        failure = Some(join(failure, error));
    }
    let lease_acquired = lease.is_some();
    if let Some(owned) = lease {
        match swarm_engine::system_timestamp() {
            Ok(now) => {
                if let Err(error) = store.update_mission_supervision(&owned, &now, true) {
                    failure = Some(join(failure, phase("mission-watch-release-lease", error)));
                }
            }
            Err(error) => {
                failure = Some(join(failure, phase("mission-watch-cleanup-clock", error)))
            }
        }
        if let Err(error) = owned.close() {
            failure = Some(join(failure, phase("mission-watch-close-lease", error)));
        }
    }
    if let Err(error) = store.close() {
        failure = Some(join(failure, error));
    }
    if let Some(error) = failure {
        return Err(CliError::Root(Box::new(preserve_admission(
            admission_recorded,
            preserve_planning(planning_recorded.as_ref(), error),
        ))));
    }
    let stop = stop.ok_or_else(|| {
        CliError::Root(Box::new(preserve_admission(
            admission_recorded,
            preserve_planning(
                planning_recorded.as_ref(),
                RootError::Refused("mission-watch-stop-result-missing"),
            ),
        )))
    })?;
    emit(out, &json!({"schema_version":1,"event":"mission-watch-stopped","work":work,"reason":stop.name(),
        "children_still_running":children.len(),"lease_released":lease_acquired,"root_closed":true,"mission_complete":false}))
        .map_err(|error| CliError::Root(Box::new(preserve_admission(admission_recorded, preserve_planning(planning_recorded.as_ref(), error)))))?;
    // Child Drop neither kills nor waits. Workers already through their private
    // gate keep their own lifecycle/consent protections and completion duties.
    drop(children);
    Ok(0)
}

#[cfg(test)]
mod tests {
    /// Valid no-effect revision refusal precedes unknown effects, attempted replay,
    /// secondary/root causes and unrelated phases; none may be silently reobserved.
    #[test]
    fn stale_dispatch_reobservation_requires_exact_no_effect_proof() {
        use swarm_engine::{MissionWorkerDispatchFailure as F, MissionDispatchQueueEffect as Q,
            MissionDispatchPreparationEffect as P, MissionDispatchCause as C, MissionSupervisionError as M};
        let clean=||F { phase:"revision",queue:Q::NotAttempted,preparation:P::NotAttempted,
            resources:Vec::new(),cause:C::Mission(Box::new(M::Refused("revision_conflict"))) };
        assert!(super::stale_dispatch_observation(&clean()));
        let mut error=clean();error.phase="queue-observation-revision";assert!(!super::stale_dispatch_observation(&error));
        let mut error=clean();error.queue=Q::OutcomeUnknown;assert!(!super::stale_dispatch_observation(&error));
        let mut error=clean();error.preparation=P::ReplayAttempted{event_id:"same-id".into()};assert!(!super::stale_dispatch_observation(&error));
        let mut error=clean();error.cause=C::Root(Box::new(swarm_engine::RootStoreError::Refused("revision_conflict")));assert!(!super::stale_dispatch_observation(&error));
        let mut error=clean();error.cause=C::Mission(Box::new(M::Refused("mission-lease-lost")));assert!(!super::stale_dispatch_observation(&error));
        let mut error=clean();error.resources.push(swarm_engine::MissionDispatchResourceRefusal::Occupied{task:"task".into(),cause:swarm_engine::RootStoreError::Refused("fixture-resource")});assert!(!super::stale_dispatch_observation(&error));
        assert!(super::stale_dispatch_observation(&clean()));
    }
    /// A real local channel carries controller shutdown without inventing an OS
    /// signal. Valid idle/stop precedes disconnected-owner refusal and recovery.
    #[test]
    fn local_session_stop_and_owner_loss_are_distinct() {
        let source=swarm_engine::ExecutionInterrupt::new().unwrap();
        let (tx,rx)=std::sync::mpsc::channel();
        let mut signals=super::Signals::local(source.clone(),rx);
        assert_eq!(signals.ready().unwrap(),None);
        source.request(swarm_engine::ExecutionInterruptReason::ControllerShutdown).unwrap();
        tx.send(super::Stop::ControllerShutdown).unwrap();
        assert_eq!(signals.ready().unwrap(),Some(super::Stop::ControllerShutdown));
        assert_eq!(signals.wait(std::time::Duration::ZERO).unwrap(),Some(super::Stop::ControllerShutdown));
        signals.close().unwrap();
        let (tx,rx)=std::sync::mpsc::channel();
        let source=swarm_engine::ExecutionInterrupt::new().unwrap();
        let mut lost=super::Signals::local(source.clone(),rx);
        assert_eq!(lost.ready().unwrap(),None);drop(tx);
        assert!(matches!(lost.ready(),Err(swarm_engine::RootStoreError::Refused("mission-watch-signal-stream-closed"))));
        assert_eq!(source.reason().unwrap(),None);lost.close().unwrap();
        let (tx,rx)=std::sync::mpsc::channel();
        let mut recovered=super::Signals::local(source,rx);
        tx.send(super::Stop::ControllerShutdown).unwrap();
        assert_eq!(recovered.ready().unwrap(),Some(super::Stop::ControllerShutdown));
    }
    /// Synthetic stop metadata only, not publication/signal proof. The actual
    /// consumer must retain its original known Work and cancellation cause when
    /// a later write or flush fails; neither failure permits a clean-stop result.
    #[test]
    fn watch_interrupted_output_keeps_original_and_known_work() {
        struct Sink {
            flush: bool,
        }
        impl std::io::Write for Sink {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if self.flush {
                    Ok(bytes.len())
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "fixture-output",
                    ))
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "fixture-flush",
                ))
            }
        }
        let original = || swarm_engine::RootStoreError::WorkRecorded {
            primary: Box::new(swarm_engine::RootStoreError::Refused(
                "planning-execution-interrupted",
            )),
            work: "fixture-work".into(),
            revision: 3,
        };
        for flush in [false, true] {
            let mut listener = super::Signals::new().unwrap();
            listener.stopped = Some(super::Stop::Interrupt);
            let mut known = None;
            let mut valid = Vec::new();
            assert_eq!(
                super::interrupted_phase(
                    original(),
                    "fixture-work",
                    &mut listener,
                    &mut known,
                    &mut valid
                )
                .unwrap(),
                super::Stop::Interrupt
            );
            let error = super::interrupted_phase(
                original(),
                "fixture-work",
                &mut listener,
                &mut known,
                &mut Sink { flush },
            )
            .unwrap_err();
            listener.close().unwrap();
            assert_eq!(error.recorded_work(), Some(("fixture-work", 3)));
            let labels = crate::safe_causes(&crate::CliError::Root(Box::new(error)));
            assert!(labels
                .iter()
                .any(|label| label.contains("planning-execution-interrupted")));
            assert!(labels.iter().any(|label| label.contains("BrokenPipe")));
        }
    }
    /// Pure metadata witness before lower-revision input and recovery. A clean
    /// interrupted event must not replace a newer same-Work session observation;
    /// otherwise a later stop-output failure would report an older revision.
    #[test]
    fn watch_interruption_keeps_newer_session_work() {
        let mut listener = super::Signals::new().unwrap();
        listener.stopped = Some(super::Stop::Interrupt);
        let mut seen = Vec::new();
        for previous in [1, 5, 1] {
            let mut known = Some(("fixture-work".to_owned(), previous));
            let error = swarm_engine::RootStoreError::WorkRecorded {
                primary: Box::new(swarm_engine::RootStoreError::Refused(
                    "planning-execution-interrupted",
                )),
                work: "fixture-work".into(),
                revision: 3,
            };
            assert_eq!(
                super::interrupted_phase(
                    error,
                    "fixture-work",
                    &mut listener,
                    &mut known,
                    &mut Vec::new()
                )
                .unwrap(),
                super::Stop::Interrupt
            );
            seen.push(known.unwrap().1);
        }
        listener.close().unwrap();
        assert_eq!(seen, vec![3, 5, 3]);
    }
    /// Use the actual shared listener producer: a healthy body fabricates no
    /// stop, then a controlled panic must become a retained failure and wake.
    #[test]
    fn watch_listener_panic_must_wake_owned_role() {
        let source = swarm_engine::ExecutionInterrupt::new().unwrap();
        super::run_signal_listener(&source, || Ok(())).unwrap();
        assert_eq!(source.reason().unwrap(), None);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            super::run_signal_listener(&source, || std::panic::panic_any(123_u32))
        }));
        assert!(
            result.is_ok(),
            "listener panic must become a typed failure and wake source"
        );
        assert!(result.unwrap().is_err());
        assert_eq!(
            source.reason().unwrap(),
            Some(swarm_engine::ExecutionInterruptReason::ListenerFailure)
        );
    }
    /// Actual listener startup is acknowledged, an empty saved-cadence fixture
    /// returns no fabricated stop, and explicit shutdown joins idempotently.
    #[test]
    fn watch_signal_listener_startup_wait_and_checked_shutdown() {
        let mut listener = super::Signals::new().unwrap();
        assert_eq!(listener.ready().unwrap(), None);
        assert_eq!(listener.wait(std::time::Duration::ZERO).unwrap(), None);
        listener.close().unwrap();
        listener.close().unwrap();
        assert_eq!(listener.source.reason().unwrap(), None);
    }
    use super::*;
    /// The same valid entry control precedes each rejected variant. This pure
    /// guard grants nothing and consults no provider, clock, lease or Root.
    #[test]
    fn watch_planning_entry_requires_explicit_controls_and_defers_active_context() {
        let control = || swarm_engine::MissionControlView {
            revision: 1,
            paused: Some(false),
            autonomy: Some("autonome"),
            slots: None,
            consent_enabled: Some(true),
            archived: false,
        };
        assert_eq!(planning_entry(&control(), false), None);
        for mutate in [
            |v: &mut swarm_engine::MissionControlView| v.paused = Some(true),
            |v: &mut swarm_engine::MissionControlView| v.paused = None,
            |v: &mut swarm_engine::MissionControlView| v.consent_enabled = Some(false),
            |v: &mut swarm_engine::MissionControlView| v.consent_enabled = None,
            |v: &mut swarm_engine::MissionControlView| v.autonomy = Some("assiste"),
            |v: &mut swarm_engine::MissionControlView| v.autonomy = None,
            |v: &mut swarm_engine::MissionControlView| v.archived = true,
        ] {
            assert_eq!(planning_entry(&control(), false), None);
            let mut changed = control();
            mutate(&mut changed);
            assert!(planning_entry(&changed, false).is_some());
        }
        for state in [
            State::QueuedUnregisteredObserved {
                generation: 1,
                current_lease_generation: false,
            },
            State::StopRequestedUnregisteredObserved {
                generation: 1,
                current_lease_generation: true,
            },
            State::RegisteredObserved,
        ] {
            assert!(!planning_context_pending(
                &State::OutsideAutomaticWorkerScope
            ));
            assert!(planning_context_pending(&state));
            assert_eq!(
                planning_entry(&control(), planning_context_pending(&state)),
                Some("deferred-active-worker-context")
            );
        }
        assert!(!planning_context_pending(&State::TerminalVerified {
            receipt_sha256: "fixture".into(),
            output_complete: true,
            provider_may_have_started: true
        }));
    }

    /// Idle has no fabricated claim/run; applied reports only public causal
    /// coordinates. Invalid shapes refuse instead of becoming successful idle.
    #[test]
    fn watch_planning_projection_keeps_idle_distinct_and_excludes_private_work() {
        let mut step = swarm_engine::PlanningStepOutcome {
            schema_version: 1,
            status: "idle",
            accepted: false,
            selected_scope: None,
            input_event: None,
            settings_revision: swarm_engine::SettingCount(7),
            lease_seconds: None,
            claim_id: None,
            run: None,
        };
        let idle = planning_record(&step, "fixture-work", 5).unwrap();
        assert_eq!(idle["status"], "idle");
        assert_eq!(idle["decision_applied"], false);
        assert!(idle["claim_id"].is_null());
        step.status = "applied";
        assert!(planning_record(&step, "fixture-work", 5).is_err());
        step.run = Some(swarm_engine::PlanningRunOutcome {
            schema_version: 1,
            claim_id: "fixture-claim".into(),
            decision_event: "fixture-decision".into(),
            decision_applied: true,
            accepted: false,
            replayed: false,
            work: swarm_engine::Work {
                id: "fixture-work".into(),
                revision: 5,
                ..Default::default()
            },
        });
        step.claim_id = Some("fixture-claim".into());
        step.selected_scope = Some("fixture-scope".into());
        step.input_event = Some("fixture-input".into());
        let applied = planning_record(&step, "fixture-work", 5).unwrap();
        assert_eq!(applied["decision_applied"], true);
        assert_eq!(applied["settings_revision"], "7");
        assert!(!applied.to_string().contains("decision_event"));
        assert_eq!(applied["work"], "fixture-work");
        step.claim_id = Some("different-claim".into());
        assert!(planning_record(&step, "fixture-work", 5).is_err());
        let known = ("fixture-work".to_owned(), 5);
        assert!(!matches!(
            preserve_planning(None, RootError::Refused("fixture")),
            RootError::WorkRecorded { .. }
        ));
        let error = preserve_planning(
            Some(&known),
            RootError::Cleanup {
                primary: Some(Box::new(phase(
                    "fixture-output",
                    std::io::Error::from_raw_os_error(13),
                ))),
                cleanup: Box::new(phase(
                    "fixture-cleanup",
                    std::io::Error::from_raw_os_error(28),
                )),
            },
        );
        assert!(
            matches!(&error, RootError::WorkRecorded { work, revision: 5, .. } if work == "fixture-work")
        );
        let labels = crate::safe_causes(&error);
        for code in [
            "os-errno:13",
            "os-errno:28",
            "fixture-output",
            "fixture-cleanup",
        ] {
            assert!(labels.iter().any(|v| v == code));
        }
    }

    /// Unknown queued state still requires recovery. A registered observation
    /// needs no owned child handle, never becomes adoption permission and must
    /// still defer planning; this synthetic state is not actual liveness proof.
    #[test]
    fn watch_retained_intentions_distinguish_owned_pending_children() {
        let queued = State::QueuedUnregisteredObserved {
            generation: 1,
            current_lease_generation: true,
        };
        assert!(retained_allowed(true, &queued).is_ok());
        assert_eq!(
            retained_allowed(false, &queued),
            Err("mission-watch-retained-queued-requires-recovery")
        );
        assert!(retained_allowed(true, &State::RegisteredObserved).is_ok());
        assert!(retained_allowed(false, &State::RegisteredObserved).is_ok());
        assert!(!adoption_candidate(false, &State::RegisteredObserved));
        assert!(planning_context_pending(&State::RegisteredObserved));
        assert!(retained_allowed(false, &State::OutsideAutomaticWorkerScope).is_ok());
    }
    /// Persisted cadence has no local default/clamp. Invalid policy refuses;
    /// this pure guard neither sleeps nor observes a lease or OS process.
    #[test]
    fn watch_cadence_uses_validated_saved_milliseconds() {
        let mut policy = swarm_engine::MissionSupervisionPolicy {
            poll_interval_ms: swarm_engine::SettingCount(37),
            stale_after_ms: swarm_engine::SettingCount(99),
            future_tolerance_ms: swarm_engine::SettingCount(0),
        };
        assert_eq!(cadence(&policy).unwrap(), Duration::from_millis(37));
        policy.poll_interval_ms = swarm_engine::SettingCount(0);
        assert!(cadence(&policy).is_err());
    }
    /// Build only the real bootstrap argv, with literal IDs and absolute Root;
    /// no shell/session syscall/spawn occurs in this command-construction guard.
    #[test]
    fn watch_worker_argv_uses_existing_private_bootstrap() {
        let command = worker_command(
            Path::new("/owned/swarm-rust"),
            Path::new("/owned/root"),
            "agent-original",
        );
        assert_eq!(command.get_program(), "/owned/swarm-rust");
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [
                "_mission_worker_entry",
                "agent-original",
                "--root",
                "/owned/root",
                "--json"
            ]
        );
    }
    /// Idle and permission refusal are distinct explicit observations. Neither
    /// claims automatic roles or mission completion; private Display stays absent.
    #[test]
    fn watch_blocked_dispatch_preserves_permission_cause_without_fake_completion() {
        let queue = || swarm_engine::MissionQueueRegistration {
            work: "work-original".into(),
            revision: 1,
            ready: vec![],
            refused: vec![],
            permission_refusal: None,
        };
        let control = dispatch_record(
            &Dispatch::NoAdmission {
                queue: queue(),
                resources: vec![],
            },
            "work-original",
        );
        assert_eq!(control["reason"], "no-ready-worker");
        let mut denied = queue();
        denied.permission_refusal = Some(swarm_engine::MissionSupervisionError::Refused(
            "fixture-permission-denied",
        ));
        let record = dispatch_record(
            &Dispatch::NoAdmission {
                queue: denied,
                resources: vec![],
            },
            "work-original",
        );
        assert_eq!(record["reason"], "dispatch-refused-or-blocked");
        assert!(record["permission_refusal"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "fixture-permission-denied"));
        assert_eq!(record["automatic_planner"], true);
        assert_eq!(record["mission_complete"], false);
    }
    /// A successful callback cannot hide failed supervision or its cleanup.
    /// Known native wrappers preserve all branches without private error text.
    #[test]
    fn watch_cycle_after_action_keeps_supervision_and_journal_errors() {
        let error = cycle_failure(MissionCycleFailure {
            action: Some(Ok(Stop::Interrupt)),
            supervision: Some(swarm_engine::MissionSupervisionError::Refused(
                "fixture-supervision",
            )),
            journal: Some(swarm_engine::MissionSupervisionError::Refused(
                "fixture-journal",
            )),
        });
        let labels = crate::safe_causes(&error);
        for code in [
            "mission-watch-cycle-after-action",
            "fixture-supervision",
            "fixture-journal",
        ] {
            assert!(labels.iter().any(|label| label == code));
        }
    }
    /// Valid unrecorded failure precedes the recorded variant. Known admission
    /// adds only lifecycle preservation; primary IO and independent cleanup
    /// codes remain visible without manufacturing commit/revision/acceptance.
    #[test]
    fn watch_admission_preservation_keeps_primary_and_cleanup_causes() {
        let exit = phase(
            "mission-watch-worker-exit",
            ChildExit {
                code: Some(23),
                signal: None,
            },
        );
        assert!(crate::safe_causes(&exit).contains(&"mission-worker-exit-code:23".into()));
        let fixture = || RootError::Cleanup {
            primary: Some(Box::new(phase(
                "watch-fixture-primary",
                std::io::Error::from_raw_os_error(13),
            ))),
            cleanup: Box::new(phase(
                "watch-fixture-cleanup",
                std::io::Error::from_raw_os_error(28),
            )),
        };
        let control = preserve_admission(false, fixture());
        assert!(!matches!(&control, RootError::LifecycleRecorded { .. }));
        let control_codes = crate::safe_causes(&control);
        for code in [
            "os-errno:13",
            "os-errno:28",
            "watch-fixture-primary",
            "watch-fixture-cleanup",
        ] {
            assert!(control_codes.iter().any(|label| label == code));
        }
        assert!(!control_codes
            .iter()
            .any(|label| label == "agent-state-preserved"));
        let retained = preserve_admission(true, fixture());
        assert!(matches!(&retained, RootError::LifecycleRecorded { .. }));
        let labels = crate::safe_causes(&retained);
        for code in [
            "agent-state-preserved",
            "os-errno:13",
            "os-errno:28",
            "watch-fixture-primary",
            "watch-fixture-cleanup",
        ] {
            assert!(labels.iter().any(|label| label == code));
        }
    }
    /// A valid old queued candidate precedes current/stop/registered negatives.
    /// This classifies only candidates; it never simulates an adoption receipt.
    #[test]
    fn watch_adoption_candidates_are_only_unknown_old_nonstopped_queued() {
        let old = State::QueuedUnregisteredObserved {
            generation: 1,
            current_lease_generation: false,
        };
        assert!(adoption_candidate(false, &old));
        assert!(retained_allowed(false, &old).is_ok());
        assert!(!adoption_candidate(true, &old));
        let current = State::QueuedUnregisteredObserved {
            generation: 2,
            current_lease_generation: true,
        };
        assert!(adoption_candidate(false, &old));
        assert!(!adoption_candidate(false, &current));
        assert!(retained_allowed(false, &current).is_err());
        let stopping = State::StopRequestedUnregisteredObserved {
            generation: 1,
            current_lease_generation: false,
        };
        assert!(adoption_candidate(false, &old));
        assert!(!adoption_candidate(false, &stopping));
        assert!(retained_allowed(false, &stopping).is_err());
        assert!(adoption_candidate(false, &old));
        assert!(!adoption_candidate(false, &State::RegisteredObserved));
        assert!(retained_allowed(false, &State::RegisteredObserved).is_ok());
    }
}
