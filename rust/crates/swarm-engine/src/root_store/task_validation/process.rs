//! Protected native task-control transport. Commands come only from the
//! current stored policy. Filesystem/UID trust is not OS confinement or a model
//! identity proof; inherited PATH retains the existing named-program semantics.
use super::{
    store::{self, Job, JobPhase, Step, StepPhase},
    Prepared, Request,
};
use crate::{
    root_store::{finished, RootStore, RootStoreError as E},
    validation_run::{run_controlled_command, run_validation_control_using, OwnedProcessControl},
    ValidationRun, ValidationRunError,
};
use rustix::process::Signal;
use std::{
    convert::Infallible,
    io::{self, Read, Write},
    os::unix::{net::UnixStream, process::CommandExt},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Preserve original typed causes under a safe static operation family. No raw
/// paths, command text or private context is promoted to public diagnostic text.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(crate::AgentLaunchError::caused(code, error)))
}
/// Checked saved milliseconds, never an invented verification watchdog.
fn after(now: Instant, ms: i64) -> Result<Instant, E> {
    let ms = u64::try_from(ms).map_err(|e| caused("task-validation-duration", e))?;
    now.checked_add(Duration::from_millis(ms))
        .ok_or(E::Refused("task-validation-duration-unrepresentable"))
}
/// Compare actual current kernel owner, not a request's actor or PID assertion.
fn owner(job: &Job) -> Result<(), E> {
    let actual = crate::SupervisorIdentity::current()
        .map_err(|e| caused("task-validation-owner-observation", e))?;
    if actual.pid() != job.owner_pid
        || actual.process_stamp() != job.owner_stamp
        || actual.host() != job.owner_host
    {
        return Err(E::Refused("task-validation-owner-changed"));
    }
    Ok(())
}
/// Reconstruct current source under saved-settings reservation and one read SQL
/// snapshot. The original source hash and root identity must still match; the
/// relevant execution-policy generation and planning-control epoch bind authority.
/// This starts no command and grants no permission; sequential file reads are
/// not a filesystem snapshot or protection from a malicious same-UID writer.
pub(super) fn source(store: &RootStore, job: &Job, managed: Option<&super::managed_execution::Session>) -> Result<Prepared, E> {
    if let Some(session) = managed { session.observe(&store.root)?; }
    let launch = |e| E::Launch(Box::new(e));
    let lock = store.directory.reserve(false, false).map_err(E::Storage)?;
    let result = (|| {
        let settings = store.read_settings()?;
        let tx = store.db.unchecked_transaction().map_err(E::Sql)?;
        let result = (|| {
            let actual = store::job(&tx, &job.id)
                .map_err(launch)?
                .ok_or(E::Refused("task-validation-job-missing"))?;
            if actual != *job || actual.phase != JobPhase::Active {
                return Err(E::Refused("task-validation-owner-source-changed"));
            }
            let w = crate::read_recorded_work(&tx, &job.work_id).map_err(E::Work)?;
            let r = Request {
                schema_version: 1,
                event_id: job.event_id.clone(),
                expected_revision: w.revision,
                task_id: job.task_id.clone(),
            };
            let p = match managed {
                Some(session) => super::managed_execution::prepare(&tx, &store.directory, &store.root, &settings, &job.work_id, &r, &session.data),
                None => super::prepare(&tx, &store.root, &settings, &job.work_id, &r),
            }.map_err(launch)?;
            let identity = store.directory.identity().map_err(E::Storage)?;
            super::mission_authority::observe(&tx,&store.directory,job,&w,&p.preview.producer_agent_id,&settings).map_err(launch)?;
            if super::managed_execution::bound_source(&tx,job,&p.preview.source_sha256,p.managed.as_ref().map(|m| &m.candidate)).map_err(launch)? != job.source_sha256
                || identity.0.to_string() != job.directory_device
                || identity.1.to_string() != job.directory_inode
            {
                return Err(E::Refused("task-validation-preparation-changed"));
            }
            Ok(p)
        })();
        finished(result, tx.rollback().map_err(E::Sql))
    })();
    finished(result, lock.close().map_err(E::Storage))
}
/// Prepare exact policy argv for this actual registered helper and committed
/// slot. Peer/parent PID, starttime, process group, host and source are checked
/// before returning the command; the caller must close Root before exec.
fn gate_command(store: &RootStore, id: &str, peer: i64, managed: &mut Option<super::managed_execution::Session>) -> Result<Command, E> {
    let (job, step) = store::step(&store.db, id).map_err(|e| E::Launch(Box::new(e)))?;
    let current = crate::SupervisorIdentity::current()
        .map_err(|e| caused("task-validation-child-observation", e))?;
    if step.phase != StepPhase::Permitted
        || step.permission != 1
        || step.child_pid != Some(current.pid())
        || step.child_stamp.as_deref() != Some(current.process_stamp())
        || job.owner_host != current.host()
        || peer != job.owner_pid
    {
        return Err(E::Refused("task-validation-gate-identity-or-permission"));
    }
    let child = crate::observe_linux_process(std::process::id())
        .map_err(|e| caused("task-validation-child-observation", e))?;
    let parent = crate::observe_linux_process(
        u32::try_from(job.owner_pid).map_err(|e| caused("task-validation-parent-pid", e))?,
    )
    .map_err(|e| caused("task-validation-parent-observation", e))?;
    if i64::from(child.parent_pid()) != job.owner_pid
        || Some(i64::from(child.process_group_id())) != step.group_id
        || parent.process_stamp() != job.owner_stamp
    {
        return Err(E::Refused("task-validation-gate-parent-or-group"));
    }
    *managed = super::managed_execution::for_helper(store, &job)?;
    let prepared = source(store, &job, managed.as_ref())?;
    let control = prepared.control(step.ordinal, &step.control_id).map_err(|e| E::Launch(Box::new(e)))?;
    let args = control
        .command
        .as_deref()
        .filter(|v| !v.is_empty())
        .ok_or(E::Refused("task-validation-command-required"))?;
    let mut command = Command::new(&args[0]);
    command.args(&args[1..]);
    command.current_dir(match managed.as_ref() {
        Some(session) => {
            // Verify physical containment first, then keep cwd holder-anchored
            // through exec. Same-UID path mutation is not OS confinement.
            crate::local_directory(&session.workspace(), &control.dir)
                .map_err(|e| caused("task-validation-gate-directory", e))?;
            session.workspace().join(&control.dir)
        }
        None => crate::local_directory(&store.root, &control.dir)
            .map_err(|e| caused("task-validation-gate-directory", e))?,
    });
    command.stdin(Stdio::null());
    Ok(command)
}
/// Private CLI helper only. Actual Unix peer credentials and parent-death signal
/// precede the one-byte grant; committed permission and fresh source precede
/// exact command exec. The diagnostic socket is CLOEXEC and stdin becomes null.
/// EOF alone proves no exec; caller must also observe the owned wait result.
/// Failure returns typed causes and a private safe frame, never a zero success.
/// Same-UID mutation, executable/PATH attestation and escaped descendants remain
/// outside this trust boundary; no task or independent review is accepted here.
/// Shared raw credential validation rejects PID0/invalid UID-GID before grant or Root use.
pub(super) fn execute_gate(root: &Path, id: &str) -> Result<Infallible, E> {
    if !swarm_identity_contract::name(id.as_bytes()) {
        return Err(E::Refused("task-validation-step-id-invalid"));
    }
    let fd = rustix::io::fcntl_dupfd_cloexec(io::stdin(), 3)
        .map_err(|e| caused("task-validation-channel-duplicate", e))?;
    let mut channel = UnixStream::from(fd);
    let mut managed = None;
    let operation: Result<Infallible, E> = (|| {
        channel
            .peer_addr()
            .map_err(|e| caused("task-validation-channel-kind", e))?;
        crate::supervisor_identity::arm_parent_death_signal()
            .map_err(|e| caused("task-validation-parent-death", e))?;
        let peer = crate::linux_peer_credentials::read(&channel)
            .map_err(|e| caused("task-validation-peer-credentials", e))?;
        if peer.uid() != rustix::process::geteuid().as_raw() {
            return Err(E::Refused("task-validation-peer-user-changed"));
        }
        let mut grant = [0u8; 1];
        io::stdin()
            .read_exact(&mut grant)
            .map_err(|e| caused("task-validation-gate-input", e))?;
        if grant != [1] {
            return Err(E::Refused("task-validation-gate-grant-invalid"));
        }
        let store = RootStore::open(root, false)?;
        let mut command = finished(
            gate_command(&store, id, i64::from(peer.pid()), &mut managed),
            store.close(),
        )?;
        Err(caused("task-validation-command-exec", command.exec()))
    })();
    // The outer owner survives Root close and helper exec. On an exec/Root
    // failure, preserve every candidate release cause before diagnostic output.
    let operation = finished(operation, managed.map_or(Ok(()), |session| session.close()));
    let error = match operation {
        Ok(never) => match never {},
        Err(e) => e,
    };
    let report = serde_json::to_vec(&crate::agent_execution_gate::diagnostic(&error))
        .map_err(|e| caused("task-validation-diagnostic-encode", e))
        .and_then(|v| {
            channel
                .write_all(&v)
                .map_err(|e| caused("task-validation-diagnostic-write", e))
        });
    let cleanup =
        nix::unistd::close(channel).map_err(|e| caused("task-validation-channel-close", e));
    finished(Err(error), finished(report, cleanup))
}
/// Actual foreground observations are returned even after an owned-process
/// error. An unreaped child remains inside ValidationRunError; no tuple absence
/// or spawn error is converted into no fork, a reap or a successful empty result.
pub(super) struct Observation {
    pub step: Step,
    pub result: Result<ValidationRun, ValidationRunError>,
    pub grant_attempted: bool,
    pub gate: Option<crate::agent_execution_gate::GateFailure>,
    pub interruption: Option<E>,
    pub channel_closed: bool,
    /// Independent IO/codec/close errors coexist with the original owned result.
    /// The publisher must refuse success whenever this list is nonempty.
    pub transport_errors: Vec<E>,
    /// Actual owned wait status, never reconstructed from a sentinel exit code.
    pub reaped_status: Option<std::process::ExitStatus>,
}
/// Root-owned live process guard. Poll/grace/drain use the captured persisted
/// policy; the control's original contractual deadline belongs to the runner.
struct Control<'a> {
    store: &'a mut RootStore,
    job: &'a Job,
    lease: Option<&'a crate::MissionLease>,
    managed: Option<&'a super::managed_execution::Session>,
    step: Step,
    channel: Option<UnixStream>,
    bytes: Vec<u8>,
    eof: bool,
    grant_attempted: bool,
    policy: crate::AgentExecutionPolicy,
    next: Instant,
    drain: Option<Instant>,
    kill: Option<Instant>,
    leader_done: bool,
    killed: bool,
    interruption: Option<E>,
}
impl Control<'_> {
    /// Consume only the owned private helper endpoint. Allocation/IO failures
    /// refuse; transport chunks are not a policy cap and EOF is not exec proof.
    fn read_gate(&mut self) -> Result<(), E> {
        if self.eof {
            return Ok(());
        }
        let channel = self
            .channel
            .as_mut()
            .ok_or(E::Refused("task-validation-channel-missing"))?;
        loop {
            let mut bytes = [0u8; 4096];
            match channel.read(&mut bytes) {
                Ok(0) => {
                    self.eof = true;
                    return Ok(());
                }
                Ok(n) => {
                    self.bytes
                        .try_reserve(n)
                        .map_err(|e| caused("task-validation-gate-allocation", e))?;
                    self.bytes.extend_from_slice(&bytes[..n])
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(e) => return Err(caused("task-validation-gate-read", e)),
            }
        }
    }
    /// Commit one checked registration/permission edge and update the in-memory
    /// observation only after known commit. Unknown SQL outcome leaves the old
    /// observation for explicit recovery and never triggers a gate write.
    fn transition(&mut self, new: Step) -> Result<(), E> {
        let old = self.step.clone();
        let observed = &mut self.step;
        crate::agent_storage_retry::execute(
            &mut self.store.db,
            &self.store.directory,
            false,
            |db, settings| {
                let committed = crate::agent_execution_store::transaction(db, |tx| {
                    super::mission_authority::permission(tx,&self.store.directory,self.lease,self.job,&self.store.root,settings,self.managed.map(|s| &s.data))?;
                    store::change_step(tx, &old, &new)
                });
                if committed.is_ok() {
                    *observed = new.clone();
                }
                committed.map_err(|e| E::Launch(Box::new(e)))
            },
        )?;
        Ok(())
    }
}
impl OwnedProcessControl for Control<'_> {
    /// Observe the actual gated child, register its immutable tuple, recheck
    /// source and commit permission. Record grant_attempted before the first
    /// possible write, including a partial/failed write. No callback DTO grants it.
    fn started(&mut self, pid: u32) -> io::Result<()> {
        (|| {
            owner(self.job)?;
            source(self.store, self.job, self.managed)?;
            let child = crate::observe_linux_process(pid)
                .map_err(|e| caused("task-validation-child-observation", e))?;
            if i64::from(child.parent_pid()) != self.job.owner_pid
                || child.process_group_id() != pid
            {
                return Err(E::Refused("task-validation-child-parent-or-group"));
            }
            let mut registered = self.step.clone();
            registered.phase = StepPhase::Registered;
            registered.child_pid = Some(i64::from(pid));
            registered.parent_pid = Some(i64::from(child.parent_pid()));
            registered.group_id = Some(i64::from(pid));
            registered.child_stamp = Some(child.process_stamp().into());
            self.transition(registered)?;
            source(self.store, self.job, self.managed)?;
            let mut permitted = self.step.clone();
            permitted.phase = StepPhase::Permitted;
            permitted.permission = 1;
            self.transition(permitted)?;
            self.grant_attempted = true;
            self.channel
                .as_mut()
                .ok_or(E::Refused("task-validation-channel-missing"))?
                .write_all(&[1])
                .map_err(|e| caused("task-validation-grant-write", e))?;
            self.next = after(Instant::now(), self.policy.poll_interval_ms.0)?;
            Ok(())
        })()
        .map_err(io::Error::other)
    }
    /// Begin only the saved post-leader drain window. This is not an execution
    /// timeout; the owned runner has already signalled the observed group.
    fn leader_exited(&mut self) -> io::Result<()> {
        self.leader_done = true;
        self.drain = self
            .policy
            .output_drain_ms
            .0
            .map(|v| after(Instant::now(), v.0))
            .transpose()
            .map_err(io::Error::other)?;
        Ok(())
    }
    /// Return persisted observation/grace/drain coordinates, no global watchdog.
    fn next_wake(&self) -> Option<Instant> {
        [
            Some(self.next),
            self.drain,
            if self.killed { None } else { self.kill },
        ]
        .into_iter()
        .flatten()
        .min()
    }
    /// Borrow local stop wake separately from persisted cadence/grace/drain.
    /// Once interrupted or leader-exited, normal stop timers alone remain active.
    fn interruption_descriptor(&self)->Option<std::os::fd::BorrowedFd<'_>>{
        if self.leader_done||self.interruption.is_some(){None}else{self.store.interruption.as_ref().map(crate::ExecutionInterrupt::descriptor)}
    }
    /// Re-read actual authority at the saved interval. Pause/source failure
    /// preserves its typed cause and requests TERM then saved-grace KILL only
    /// for this owned group. IO/clock/drain errors use the common cleanup path.
    fn observe(&mut self) -> io::Result<Option<Signal>> {
        (|| {
            self.read_gate()?;
            let now = Instant::now();
            if !self.leader_done && self.interruption.is_none() && self.store.interruption.as_ref().map(crate::ExecutionInterrupt::reason).transpose().map_err(|e|caused("task-validation-interrupt-observation",e))?.flatten().is_some(){
                self.interruption=Some(E::Refused("task-validation-execution-interrupted"));
                self.kill=Some(after(now,self.policy.term_grace_ms.0)?);return Ok(Some(Signal::TERM));
            }
            if self.drain.is_some_and(|d| now >= d) {
                return Err(E::Refused("task-validation-output-drain-expired"));
            }
            if !self.killed && self.kill.is_some_and(|d| now >= d) {
                self.killed = true;
                return Ok(Some(Signal::KILL));
            }
            if now >= self.next {
                self.next = after(now, self.policy.poll_interval_ms.0)?;
                if !self.leader_done && self.interruption.is_none() {
                    if let Err(e) =
                        owner(self.job).and_then(|_| source(self.store, self.job, self.managed).map(|_| ()))
                    {
                        self.interruption = Some(e);
                        self.kill = Some(after(now, self.policy.term_grace_ms.0)?);
                        return Ok(Some(Signal::TERM));
                    }
                }
            }
            Ok(None)
        })()
        .map_err(io::Error::other)
    }
}
/// Execute one exact preauthorized slot via the same binary's private helper.
/// Actual start/wait/output are handled by the existing owned-process primitive;
/// normalized target preparation/capture/deadline/results are shared. A safe
/// private gate frame, interruption or descriptor failure cannot become PASS.
/// Return original cleanup/ownership errors and observed slot/grant metadata;
/// outer Err is reserved for setup before the owned-process primitive. Once
/// attempted, independent errors stay in Observation and cannot erase wait facts.
/// caller still owes durable receipts, atomic publication and reconciliation.
pub(super) fn run(
    store: &mut RootStore,
    job: &Job,
    step: &Step,
    control: &crate::ValidationControl,
    lease: Option<&crate::MissionLease>,
    managed: Option<&super::managed_execution::Session>,
) -> Result<Observation, E> {
    owner(job)?;
    if let Some(session) = managed { session.refresh(store, job)?; }
    let prepared = source(store, job, managed)?;
    let actual = prepared.control(step.ordinal, &step.control_id).map_err(|e| E::Launch(Box::new(e)))?;
    let (stored_job, stored_step) =
        store::step(&store.db, &step.id).map_err(|e| E::Launch(Box::new(e)))?;
    if actual != control
        || stored_job != *job
        || stored_step != *step
        || step.phase != StepPhase::Prepared
    {
        return Err(E::Refused("task-validation-control-arguments-changed"));
    }
    let settings = prepared.settings;
    let policy = settings
        .values
        .agent_execution_policy()
        .map_err(E::Settings)?;
    let (left, right) =
        UnixStream::pair().map_err(|e| caused("task-validation-channel-create", e))?;
    if let Err(e) = left.set_nonblocking(true) {
        return finished(
            Err(caused("task-validation-channel-nonblocking", e)),
            finished(
                nix::unistd::close(left).map_err(|e| caused("task-validation-channel-close", e)),
                nix::unistd::close(right).map_err(|e| caused("task-validation-channel-close", e)),
            ),
        );
    }
    let root = store.root.clone();
    // Linux self-image binding is an identity invariant, not an operator path.
    // Reopening current_exe() by pathname could select a replacement build.
    let mut command = Command::new("/proc/self/exe");
    command
        .args(["_validation_gate", &step.id, "--root"])
        .arg(&root);
    let mut supervisor = Control {
        store,
        job,
        lease,
        managed,
        step: step.clone(),
        channel: Some(left),
        bytes: Vec::new(),
        eof: false,
        grant_attempted: false,
        policy,
        next: Instant::now(),
        drain: None,
        kill: None,
        leader_done: false,
        killed: false,
        interruption: None,
    };
    let mut reaped_status = None;
    let control_root = managed.map(|s| s.workspace()).unwrap_or_else(|| root.clone());
    let mut result =
        run_validation_control_using(&control_root, control, &settings, |_, deadline, output| {
            let observed = run_controlled_command(
                command,
                Stdio::from(std::os::fd::OwnedFd::from(right)),
                Some(deadline),
                output,
                None,
                Some(&mut supervisor),
            );
            if let Ok(actual) = &observed {
                reaped_status = Some(actual.usage.status);
            }
            observed
        });
    if reaped_status.is_none() {
        reaped_status = result
            .as_ref()
            .err()
            .and_then(ValidationRunError::reaped_exit_status);
    }
    let mut transport_errors = Vec::new();
    let decoded = supervisor
        .read_gate()
        .and_then(|_| crate::agent_supervision::decode_gate(&supervisor.bytes, supervisor.eof));
    let gate = match decoded {
        Ok(frame) => frame,
        Err(error) => {
            transport_errors.push(error);
            None
        }
    };
    let close = match supervisor.channel.take() {
        Some(channel) => {
            nix::unistd::close(channel).map_err(|e| caused("task-validation-channel-close", e))
        }
        None => Err(E::Refused("task-validation-channel-missing")),
    };
    let channel_closed = close.is_ok();
    if let Err(error) = close {
        transport_errors.push(error);
    }
    // A complete owned result and independent transport failures both survive.
    // Never erase a prior process error merely because closing also failed.
    if let Ok(run) = &mut result {
        if gate.is_some() || supervisor.interruption.is_some() || !transport_errors.is_empty() {
            run.result.passed = false;
        }
    }
    Ok(Observation {
        step: supervisor.step,
        result,
        grant_attempted: supervisor.grant_attempted,
        gate,
        interruption: supervisor.interruption,
        channel_closed,
        transport_errors,
        reaped_status,
    })
}
