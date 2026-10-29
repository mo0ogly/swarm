//! Linux process primitive; callers still owe current authority, reservations
//! and identity checks. Normalized policy grants no execution authority.
use crate::{
    local_directory, normalize_validation_policy, system_timestamp, EngineSettingsDocument,
    SettingCount, ValidationControl, ValidationControlResult, ValidationOutput, ValidationPolicy,
};
use rustix::{
    event::{poll, PollFd, PollFlags, Timespec},
    process::{kill_process_group, pidfd_open, Pid, PidfdFlags, Signal},
};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt,
    io::{self, Read, Write},
    os::{
        fd::{BorrowedFd, OwnedFd},
        unix::{net::UnixStream, process::CommandExt},
    },
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub struct ValidationRun {
    pub result: ValidationControlResult,
    pub output: ValidationOutput,
    pub settings_revision: SettingCount,
    pub timed_out: bool,
    /// Signal delivery is distinct from observed termination of every descendant.
    pub group_signal_delivered: bool,
    pub output_drain_timed_out: bool,
}
impl fmt::Debug for ValidationRun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValidationRun")
            .field("executed", &self.result.executed)
            .field("passed", &self.result.passed)
            .field("exit_code", &self.result.exit_code)
            .field("output", &self.output)
            .field("timed_out", &self.timed_out)
            .finish()
    }
}
pub struct ValidationRunCleanup {
    pub operation: &'static str,
    pub cause: io::Error,
}
pub struct ValidationRunError {
    operation: &'static str,
    cause: Box<dyn Error + Send + Sync>,
    cleanup: Vec<ValidationRunCleanup>,
    captured: Option<Box<ValidationOutput>>,
    unreaped: Option<Box<Child>>,
    ownership_lost_id: Option<u32>,
    reaped_status: Option<std::process::ExitStatus>,
}
impl ValidationRunError {
    /// Retain the primary operation and typed cause before cleanup or wait.
    /// No output, wait receipt or ownership capability is fabricated; callers
    /// attach those only after observing their actual IO/kernel outcomes.
    fn new(operation: &'static str, cause: impl Error + Send + Sync + 'static) -> Self {
        Self {
            operation,
            cause: Box::new(cause),
            cleanup: Vec::new(),
            captured: None,
            unreaped: None,
            ownership_lost_id: None,
            reaped_status: None,
        }
    }
    pub fn operation(&self) -> &'static str {
        self.operation
    }
    pub fn code(&self) -> &'static str {
        "validation-run"
    }
    pub fn cleanup(&self) -> &[ValidationRunCleanup] {
        &self.cleanup
    }
    /// Borrow any retained bounded validation output, including failure output.
    /// Absence stays unknown; these bytes establish neither success nor complete
    /// draining and may contain private fixture/provider data. No IO occurs.
    pub fn captured(&self) -> Option<&ValidationOutput> {
        self.captured.as_deref()
    }
    /// Actual successful wait status retained even when stream/close/control
    /// later fails. None means no such receipt is available, never exit0. This
    /// proves neither complete output nor termination of escaped descendants.
    pub fn reaped_exit_status(&self) -> Option<std::process::ExitStatus> {
        self.reaped_status
    }
    /// Failed kill/wait never means cleanup. Transfer the owned child to the
    /// caller for explicit recovery; it must not mark the attempt finished.
    pub fn take_unreaped_process(&mut self) -> Option<Child> {
        self.unreaped.take().map(|c| *c)
    }
    pub fn unresolved_process_id(&self) -> Option<u32> {
        self.unreaped.as_ref().map(|c| c.id())
    }
    /// Diagnostic identity only: the OS no longer establishes child ownership.
    /// It must never be used as a signal/reap capability.
    pub fn ownership_lost_process_id(&self) -> Option<u32> {
        self.ownership_lost_id
    }
}
impl fmt::Display for ValidationRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "validation-run: {}", self.operation)
    }
}
impl fmt::Debug for ValidationRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for ValidationRunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}
fn record(error: &mut Option<ValidationRunError>, operation: &'static str, cause: io::Error) {
    if let Some(error) = error {
        error
            .cleanup
            .push(ValidationRunCleanup { operation, cause })
    } else {
        *error = Some(ValidationRunError::new(operation, cause))
    }
}
fn record_wait(error: &mut Option<ValidationRunError>, id: u32, cause: io::Error) {
    let lost = cause.raw_os_error() == Some(nix::errno::Errno::ECHILD as i32);
    record(error, "wait4", cause);
    if lost {
        if let Some(error) = error {
            error.ownership_lost_id = Some(id)
        }
    }
}
fn retain_unreaped(child: Child, error: &mut Option<ValidationRunError>) {
    if let Some(error) = error {
        if error.ownership_lost_id.is_none() {
            error.unreaped = Some(Box::new(child))
        }
    }
}
fn close(
    fd: impl std::os::fd::IntoRawFd,
    error: &mut Option<ValidationRunError>,
    operation: &'static str,
) {
    if let Err(cause) = nix::unistd::close(fd) {
        record(error, operation, cause.into())
    }
}
fn terminate(group: Pid) -> io::Result<()> {
    match kill_process_group(group, Signal::KILL) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
fn valid_text(mut bytes: &[u8]) -> String {
    let mut text = String::new();
    while !bytes.is_empty() {
        match std::str::from_utf8(bytes) {
            Ok(t) => {
                text.push_str(t);
                break;
            }
            Err(e) => {
                let (prefix, rest) = bytes.split_at(e.valid_up_to());
                if let Ok(prefix) = std::str::from_utf8(prefix) {
                    text.push_str(prefix)
                }
                match e.error_len() {
                    Some(n) => bytes = &rest[n..],
                    None => break,
                }
            }
        }
    }
    text
}
/// Crate-owned supervisor hooks; these callbacks grant no launch authority.
/// `started` runs after spawn and pidfd acquisition, before stream polling. A
/// caller requiring registration before provider execution must spawn a gated
/// helper: notification alone cannot undo code already executed by a command.
/// `next_wake` supplies its own monotonic policy deadline, or None for no timer;
/// after waking, the controller must advance it or intentionally request a signal.
/// `observe` may request TERM/KILL only. Typed IO causes trigger owned cleanup;
/// no hidden polling interval, timeout, retry, persistence or task acceptance.
pub(crate) trait OwnedProcessControl {
    /// Register the newly owned real PID, optionally releasing a caller-held
    /// input gate only after its durable checks. Failure triggers owned cleanup.
    fn started(&mut self, pid: u32) -> io::Result<()>;
    /// Observe actual leader readiness after successful group cleanup. This
    /// starts a caller-configured drain window, never a provider duration cap.
    /// Failure follows the same owned cleanup path; legacy callers do nothing.
    fn leader_exited(&mut self) -> io::Result<()> {
        Ok(())
    }
    /// Next caller-defined monotonic wake; None installs no periodic timer.
    /// This coordinate does not establish provider progress or a global deadline.
    fn next_wake(&self) -> Option<Instant>;
    /// Optional controller-owned readable wake source, borrowed for this poll
    /// only. None preserves the existing relative-wake behavior without adding
    /// a periodic timer. The owner consumes/disarms readiness in observe; ERR,
    /// HUP or invalid descriptors fail through owned cleanup. This grants no
    /// authority and cannot be a socket whose EOF is a business success.
    fn wake_descriptor(&self) -> Option<BorrowedFd<'_>> {
        None
    }
    /// Optional local stop wake, separate from the existing policy timer.
    /// Borrow only until handling stop; ERR/HUP/NVAL enters owned cleanup.
    /// No external signal permission or operator deadline follows.
    fn interruption_descriptor(&self)->Option<BorrowedFd<'_>> {None}
    /// Inspect caller state before polling; None continues, TERM/KILL request a
    /// real owned-group signal. Other signals and IO failures refuse with cleanup.
    fn observe(&mut self) -> io::Result<Option<Signal>>;
}
struct Observation {
    deadline: bool,
    leader_finished: bool,
    group_attempted: bool,
    group_signalled: bool,
}
/// One owned output descriptor and its caller-provided streaming sink.
struct Channel<'a> {
    reader: UnixStream,
    sink: &'a mut dyn Write,
    eof: bool,
}
/// Drain every stream while observing the owned leader without reaping it.
/// An absent deadline really means no policy timeout. Controller wakeups only
/// bound the next poll; they never mark a global deadline exceeded. Controller,
/// signal, sink/poll/read and optional controller-descriptor failures propagate
/// to the common cleanup path. The descriptor is borrowed only during poll; its
/// readiness prompts the next observe without adding a relative/global timeout. Transport
/// chunks are allocation details, not an output cap or operator polling policy.
fn observe(
    channels: &mut [Channel<'_>],
    pidfd: &OwnedFd,
    group: Pid,
    deadline: Option<Instant>,
    state: &mut Observation,
    mut control: Option<&mut dyn OwnedProcessControl>,
) -> Result<(), ValidationRunError> {
    while channels.iter().any(|c| !c.eof) || !state.leader_finished {
        if let Some(controller) = control.as_mut() {
            if let Some(signal) = controller
                .observe()
                .map_err(|e| ValidationRunError::new("process-control", e))?
            {
                if !matches!(signal, Signal::TERM | Signal::KILL) {
                    return Err(ValidationRunError::new(
                        "process-control-signal",
                        io::Error::other("unsupported owned process directive"),
                    ));
                }
                match kill_process_group(group, signal) {
                    Ok(()) | Err(rustix::io::Errno::SRCH) => {}
                    Err(e) => {
                        return Err(ValidationRunError::new(
                            "process-control-signal",
                            io::Error::from(e),
                        ))
                    }
                }
            }
        }
        let now = Instant::now();
        let remaining = deadline.map(|d| d.saturating_duration_since(now));
        if remaining.is_some_and(|d| d.is_zero()) {
            state.deadline = true;
            break;
        }
        let wake = control
            .as_ref()
            .and_then(|c| c.next_wake())
            .map(|d| d.saturating_duration_since(now));
        let wait = match (remaining, wake) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, None) | (None, a) => a,
        };
        let timeout = wait
            .map(Timespec::try_from)
            .transpose()
            .map_err(|e| ValidationRunError::new("deadline-representation", e))?;
        let (ready, leader_ready) = {
            let mut fds: Vec<_> = channels
                .iter()
                .filter(|c| !c.eof)
                .map(|c| PollFd::new(&c.reader, PollFlags::IN))
                .collect();
            let leader_index = fds.len();
            if !state.leader_finished {
                fds.push(PollFd::new(pidfd, PollFlags::IN));
            }
            let control_index = control
                .as_ref()
                .and_then(|c| c.wake_descriptor())
                .map(|fd| {
                    let index = fds.len();
                    fds.push(PollFd::from_borrowed_fd(fd, PollFlags::IN));
                    index
                });
            let interruption_index=control.as_ref().and_then(|c|c.interruption_descriptor()).map(|fd|{
                let index=fds.len();fds.push(PollFd::from_borrowed_fd(fd,PollFlags::IN));index
            });
            match poll(&mut fds, timeout.as_ref()) {
                Err(rustix::io::Errno::INTR) => continue,
                Err(e) => return Err(ValidationRunError::new("poll", io::Error::from(e))),
                Ok(_) => {}
            }
            if fds.iter().any(|f| f.revents().contains(PollFlags::NVAL)) {
                return Err(ValidationRunError::new(
                    "poll-descriptor",
                    io::Error::other("invalid process descriptor"),
                ));
            }
            if [control_index,interruption_index].into_iter().flatten().any(|i|fds[i].revents().intersects(PollFlags::ERR | PollFlags::HUP))
            {
                return Err(ValidationRunError::new(
                    "process-control-descriptor",
                    io::Error::other("closed or failed control descriptor"),
                ));
            }
            (
                fds[..leader_index]
                    .iter()
                    .map(|f| !f.revents().is_empty())
                    .collect::<Vec<_>>(),
                !state.leader_finished && !fds[leader_index].revents().is_empty(),
            )
        };
        if leader_ready {
            // The unreaped leader prevents reuse of its numeric process group.
            state.leader_finished = true;
            state.group_attempted = true;
            terminate(group).map_err(|e| ValidationRunError::new("kill-group", e))?;
            state.group_signalled = true;
            if let Some(controller) = control.as_mut() {
                controller
                    .leader_exited()
                    .map_err(|e| ValidationRunError::new("process-control-leader-exit", e))?;
            }
        }
        for (channel, ready) in channels.iter_mut().filter(|c| !c.eof).zip(ready) {
            if !ready {
                continue;
            }
            let mut bytes = [0u8; 8192];
            match channel.reader.read(&mut bytes) {
                Ok(0) => channel.eof = true,
                Ok(n) => channel
                    .sink
                    .write_all(&bytes[..n])
                    .map_err(|e| ValidationRunError::new("retain-output", e))?,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => return Err(ValidationRunError::new("read-output", e)),
            }
        }
    }
    Ok(())
}
/// Create a nonblocking reader and checked-close both endpoints on failure.
fn output_channel() -> Result<(UnixStream, UnixStream), ValidationRunError> {
    let (reader, writer) =
        UnixStream::pair().map_err(|e| ValidationRunError::new("output-channel", e))?;
    if let Err(cause) = reader.set_nonblocking(true) {
        let mut error = Some(ValidationRunError::new("output-nonblocking", cause));
        close(reader, &mut error, "close-reader");
        close(writer, &mut error, "close-writer");
        return Err(error.unwrap());
    }
    Ok((reader, writer))
}
/// Close every owned reader, preserving failures alongside the primary cause.
fn close_channels(channels: Vec<Channel<'_>>, error: &mut Option<ValidationRunError>) {
    for channel in channels {
        close(channel.reader, error, "close-reader");
    }
}
/// Kernel observations only; these fields confer no execution authority.
pub(crate) struct OwnedProcessRun {
    pub usage: wait4::ResUse,
    pub timed_out: bool,
    pub group_signal_delivered: bool,
    pub output_drain_timed_out: bool,
}
/// Execute a preconfigured command in a dedicated process group and drain sinks.
///
/// `stderr=None` merges both streams in kernel write order; `Some` keeps stderr
/// separate. The caller owns policy, directory/environment and authorization.
/// A deadline is monotonic and optional, never silently invented. The leader
/// stays unreaped until group cleanup; pidfd/wait4, read/write/close errors and
/// any unreaped child are retained for caller recovery. No shell is introduced.
pub(crate) fn run_owned_command(
    command: Command,
    deadline: Option<Instant>,
    stdout: &mut dyn Write,
    stderr: Option<&mut dyn Write>,
) -> Result<OwnedProcessRun, ValidationRunError> {
    run_controlled_command(command, Stdio::null(), deadline, stdout, stderr, None)
}
/// Run one owned process with explicit input and optional supervisor hooks.
/// Directory/environment/command, input gate and all policy remain caller-owned.
/// Notification occurs only after pidfd acquisition; failures kill/reap the owned
/// group and retain unresolved ownership/cleanup as in the legacy wrapper.
/// Streams always drain separately or merged as requested. No global deadline
/// is invented; controller wakeups do not become timeout/completion evidence.
/// This primitive performs no SQL registration, provider authorization or task
/// acceptance. A child may already have executed before notification unless the
/// caller deliberately starts a helper blocked behind an unreleased input gate.
pub(crate) fn run_controlled_command(
    mut command: Command,
    input: Stdio,
    deadline: Option<Instant>,
    stdout: &mut dyn Write,
    stderr: Option<&mut dyn Write>,
    mut control: Option<&mut dyn OwnedProcessControl>,
) -> Result<OwnedProcessRun, ValidationRunError> {
    let (reader, writer) = output_channel()?;
    let mut channels = vec![Channel {
        reader,
        sink: stdout,
        eof: false,
    }];
    let writer2 = if let Some(sink) = stderr {
        match output_channel() {
            Ok((reader, writer)) => {
                channels.push(Channel {
                    reader,
                    sink,
                    eof: false,
                });
                writer
            }
            Err(e) => {
                let mut error = Some(e);
                close(writer, &mut error, "close-writer");
                close_channels(channels, &mut error);
                return Err(error.unwrap());
            }
        }
    } else {
        match writer.try_clone() {
            Ok(fd) => fd,
            Err(e) => {
                let mut error = Some(ValidationRunError::new("output-clone", e));
                close(writer, &mut error, "close-writer");
                close_channels(channels, &mut error);
                return Err(error.unwrap());
            }
        }
    };
    command
        .stdin(input)
        .stdout(Stdio::from(OwnedFd::from(writer)))
        .stderr(Stdio::from(OwnedFd::from(writer2)))
        .process_group(0);
    let child = command.spawn();
    drop(command); // Parent writer copies must close before observing EOF.
    let child = match child {
        Ok(c) => c,
        Err(e) => {
            let mut error = Some(ValidationRunError::new("spawn", e));
            close_channels(channels, &mut error);
            return Err(error.unwrap());
        }
    };
    let group = match i32::try_from(child.id())
        .ok()
        .filter(|n| *n > 1)
        .and_then(Pid::from_raw)
    {
        Some(pid) => pid,
        None => {
            let mut child = child;
            let mut error = Some(ValidationRunError::new(
                "child-pid",
                io::Error::other("invalid child PID"),
            ));
            if let Err(e) = child.kill() {
                record(&mut error, "kill-leader", e);
                error.as_mut().unwrap().unreaped = Some(Box::new(child));
            } else if let Err(e) = child.wait() {
                record(&mut error, "wait", e);
                error.as_mut().unwrap().unreaped = Some(Box::new(child));
            }
            close_channels(channels, &mut error);
            return Err(error.unwrap());
        }
    };
    let mut state = Observation {
        deadline: false,
        leader_finished: false,
        group_attempted: false,
        group_signalled: false,
    };
    let pidfd = match pidfd_open(group, PidfdFlags::empty()) {
        Ok(fd) => fd,
        Err(e) => {
            let mut error = Some(ValidationRunError::new("pidfd-open", io::Error::from(e)));
            reap(child, group, &mut state, &mut error);
            close_channels(channels, &mut error);
            return Err(error.unwrap());
        }
    };
    let registered = match control.as_mut() {
        Some(controller) => controller
            .started(child.id())
            .map_err(|e| ValidationRunError::new("process-registration", e)),
        None => Ok(()),
    };
    let mut error = match registered {
        Ok(()) => observe(&mut channels, &pidfd, group, deadline, &mut state, control).err(),
        Err(error) => Some(error),
    };
    let usage = reap(child, group, &mut state, &mut error);
    close_channels(channels, &mut error);
    close(pidfd, &mut error, "close-pidfd");
    if let Some(mut error) = error {
        error.reaped_status = usage.as_ref().map(|u| u.status);
        return Err(error);
    }
    Ok(OwnedProcessRun {
        usage: usage.ok_or_else(|| {
            ValidationRunError::new("wait-status", io::Error::other("missing process status"))
        })?,
        timed_out: state.deadline,
        group_signal_delivered: state.group_signalled,
        output_drain_timed_out: state.deadline && state.leader_finished,
    })
}

fn reap(
    child: Child,
    group: Pid,
    state: &mut Observation,
    error: &mut Option<ValidationRunError>,
) -> Option<wait4::ResUse> {
    reap_with(child, group, state, error, terminate)
}
/// Resolve only this owned Child/group after stream collection or a refusal.
/// Checked group cleanup precedes wait so a failed signal cannot release the
/// numeric process-group identity. A live-leader kill is only a fallback; missing
/// group confirmation retains Child for recovery. Successful wait stores its
/// actual status even in an existing error; failed wait retains ownership or its
/// explicit loss, never a fake exit0. Original and cleanup causes survive.
fn reap_with(
    mut child: Child,
    group: Pid,
    state: &mut Observation,
    error: &mut Option<ValidationRunError>,
    terminate_group: impl FnOnce(Pid) -> io::Result<()>,
) -> Option<wait4::ResUse> {
    if !state.group_attempted {
        state.group_attempted = true;
        match terminate_group(group) {
            Ok(()) => state.group_signalled = true,
            Err(e) => record(error, "kill-group", e),
        }
    }
    if !state.group_signalled {
        if !state.leader_finished {
            if let Err(e) = child.kill() {
                record(error, "kill-leader", e)
            }
        }
        // Even a dead leader must remain unreaped until group cleanup is
        // resolved, otherwise its numeric group ID may be reused.
        if let Some(e) = error {
            e.unreaped = Some(Box::new(child))
        }
        return None;
    }
    let collected = wait_owned_child(&child, group, error);
    if let (Some(usage), Some(failure)) = (&collected, error.as_mut()) {
        failure.reaped_status = Some(usage.status);
    }
    if collected.is_none() {
        retain_unreaped(child, error)
    }
    // After successful wait4, dropping Child must never wait/signal that PID.
    collected
}
fn wait_owned_child(
    child: &Child,
    group: Pid,
    error: &mut Option<ValidationRunError>,
) -> Option<wait4::ResUse> {
    match wait4::try_wait4_pid(child.id()) {
        Ok(Some(usage)) => return Some(usage),
        Ok(None) => {}
        Err(e) => {
            record_wait(error, child.id(), e);
            return None;
        }
    }
    let fd = match pidfd_open(group, PidfdFlags::empty()) {
        Ok(fd) => fd,
        Err(e) => {
            record(error, "cleanup-pidfd", e.into());
            return None;
        }
    };
    let ready = loop {
        let mut fds = [PollFd::new(&fd, PollFlags::IN)];
        match poll(&mut fds, None) {
            Err(rustix::io::Errno::INTR) => continue,
            Err(e) => {
                record(error, "cleanup-poll", e.into());
                break false;
            }
            Ok(_) if fds[0].revents().contains(PollFlags::IN) => break true,
            Ok(_) => {
                record(
                    error,
                    "cleanup-poll",
                    io::Error::other("unexpected process descriptor event"),
                );
                break false;
            }
        }
    };
    close(fd, error, "close-cleanup-pidfd");
    if !ready {
        return None;
    }
    // New precondition: kernel has now reported this owned child's exit.
    match wait4::try_wait4_pid(child.id()) {
        Ok(Some(usage)) => Some(usage),
        Ok(None) => {
            record(
                error,
                "wait4-status",
                io::Error::other("exit notification without waitable child"),
            );
            if let Some(error) = error {
                error.ownership_lost_id = Some(child.id())
            }
            None
        }
        Err(e) => {
            record_wait(error, child.id(), e);
            None
        }
    }
}
/// Execute one normalized validation control under a validated persisted snapshot.
///
/// `root` contains the configured relative directory; `control` must satisfy the
/// authoritative program, argument and deadline policy in `settings`. Captures
/// the configured output prefix while draining all bytes and returns execution,
/// per-child CPU/wall time, deadline and cleanup observations. No shell is added.
/// Errors preserve their cause, captured bytes, cleanup failures and an owned
/// unreaped child when recovery is required. This starts a local process but
/// creates no agent, reservation, identity permission or public endpoint; the
/// caller must establish current execution authority before invoking it.
pub fn run_validation_control(
    root: &Path,
    control: &ValidationControl,
    settings: &EngineSettingsDocument,
) -> Result<ValidationRun, ValidationRunError> {
    run_validation_control_using(root, control, settings, |command, deadline, output| {
        run_owned_command(command, Some(deadline), output, None)
    })
}
/// Share normalization, saved capture/deadline and observed result construction
/// with an internal protected transport. The adapter receives the normalized
/// command, contractual deadline and actual draining collector; it must return
/// an observed owned-process result or retain its real cleanup/ownership error.
/// It creates no authorization by accepting these data. Root callers still owe
/// current source/actor checks and private admission before any command exec.
/// Policy/path/representation/IO errors preserve original causes and captured
/// output; no retry, fallback command or new deadline/cap is introduced.
pub(crate) fn run_validation_control_using(
    root: &Path,
    control: &ValidationControl,
    settings: &EngineSettingsDocument,
    execute: impl FnOnce(
        Command,
        Instant,
        &mut crate::ValidationOutput,
    ) -> Result<OwnedProcessRun, ValidationRunError>,
) -> Result<ValidationRun, ValidationRunError> {
    settings
        .validate()
        .map_err(|e| ValidationRunError::new("settings", e))?;
    let mut output = ValidationOutput::from_settings(&settings.values)
        .map_err(|e| ValidationRunError::new("output-policy", e))?;
    let limits = settings
        .values
        .mutation_limits()
        .map_err(|e| ValidationRunError::new("settings-limits", e))?;
    let policy = normalize_validation_policy(
        ValidationPolicy {
            mode: "automatic".into(),
            controls: Some(vec![control.clone()]),
            ..Default::default()
        },
        &limits.policy,
    )
    .map_err(|e| ValidationRunError::new("policy", e))?;
    let c = policy
        .controls
        .and_then(|mut v| v.pop())
        .ok_or_else(|| ValidationRunError::new("policy", io::Error::other("missing control")))?;
    let dir = local_directory(root, &c.dir).map_err(|e| ValidationRunError::new("directory", e))?;
    let started = Instant::now();
    let seconds = u64::try_from(c.timeout_seconds)
        .map_err(|e| ValidationRunError::new("deadline-representation", e))?;
    let deadline = started
        .checked_add(Duration::from_secs(seconds))
        .ok_or_else(|| {
            ValidationRunError::new(
                "deadline-representation",
                io::Error::other("unrepresentable monotonic deadline"),
            )
        })?;
    let timestamp = system_timestamp().map_err(|e| ValidationRunError::new("clock-start", e))?;
    let args = c
        .command
        .as_ref()
        .ok_or_else(|| ValidationRunError::new("command", io::Error::other("missing command")))?;
    let mut command = Command::new(&args[0]);
    command.args(&args[1..]).current_dir(dir);
    let process = match execute(command, deadline, &mut output) {
        Ok(process) => process,
        Err(mut error) => {
            error.captured = Some(Box::new(output));
            return Err(error);
        }
    };
    let status = process.usage;
    let result = (|| {
        let cpu = status
            .rusage
            .utime
            .checked_add(status.rusage.stime)
            .ok_or_else(|| {
                ValidationRunError::new(
                    "cpu-representation",
                    io::Error::other("CPU duration overflow"),
                )
            })?;
        let cpu_ms = i64::try_from(cpu.as_millis())
            .map_err(|e| ValidationRunError::new("cpu-representation", e))?;
        let status = status.status;
        let finished =
            system_timestamp().map_err(|e| ValidationRunError::new("clock-finish", e))?;
        let wall = i64::try_from(started.elapsed().as_millis())
            .map_err(|e| ValidationRunError::new("duration-representation", e))?;
        let exit = if process.timed_out {
            -1
        } else {
            i64::from(status.code().unwrap_or(-1))
        };
        let mut result = ValidationControlResult {
            id: c.id,
            command: c.command,
            executed: true,
            passed: status.success() && !process.timed_out,
            exit_code: exit,
            output_sha256: format!("{:x}", Sha256::digest(output.retained())),
            started_at: timestamp,
            finished_at: finished,
            wall_duration_ms: Some(Box::new(wall)),
            cpu_duration_ms: Some(Box::new(cpu_ms)),
            ..Default::default()
        };
        result.summary = if process.timed_out {
            format!("délai de {}s dépassé", c.timeout_seconds)
        } else if result.passed {
            "contrôle réussi".into()
        } else {
            format!("contrôle en échec (code {exit})")
        };
        if !process.timed_out
            && !result.passed
            && c.environment_exit_codes
                .as_ref()
                .is_some_and(|v| v.contains(&exit))
        {
            result.environment_failure = true;
            result.summary=format!("environnement du contrôle indisponible (code {exit}) ; revalidation après correction des préconditions")
        }
        if output.truncated() {
            result.summary.push_str(&format!(
                " ; sortie tronquée à {} octets",
                output.retained().len()
            ))
        }
        if c.review_output {
            result.output_bytes = i64::try_from(output.total())
                .map_err(|e| ValidationRunError::new("count-representation", e))?;
            result.review_output = valid_text(output.retained());
            result.review_output_truncated = output.total() > result.review_output.len() as u64;
        }
        Ok::<_, ValidationRunError>(result)
    })();
    let result = match result {
        Ok(result) => result,
        Err(mut error) => {
            error.captured = Some(Box::new(output));
            return Err(error);
        }
    };
    Ok(ValidationRun {
        result,
        output,
        settings_revision: settings.revision,
        timed_out: process.timed_out,
        group_signal_delivered: process.group_signal_delivered,
        output_drain_timed_out: process.output_drain_timed_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn child(finished: bool) -> (Child, Pid, OwnedFd) {
        let c = Command::new("python3")
            .args([
                "-c",
                if finished {
                    "pass"
                } else {
                    "import signal;signal.pause()"
                },
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .unwrap();
        let group = Pid::from_raw(i32::try_from(c.id()).unwrap()).unwrap();
        let fd = pidfd_open(group, PidfdFlags::empty()).unwrap();
        if finished {
            let mut fds = [PollFd::new(&fd, PollFlags::IN)];
            poll(&mut fds, None).unwrap();
            assert!(fds[0].revents().contains(PollFlags::IN));
        }
        (c, group, fd)
    }
    fn state(finished: bool) -> Observation {
        Observation {
            deadline: false,
            leader_finished: finished,
            group_attempted: false,
            group_signalled: false,
        }
    }
    #[test]
    fn group_signal_refusal_keeps_unreaped_leader_and_primary_cause_then_recovers() {
        for finished in [false, true] {
            // Valid same-precondition control before the injected system refusal.
            let (c, group, fd) = child(finished);
            let mut error = None;
            assert!(reap(c, group, &mut state(finished), &mut error).is_some());
            assert!(error.is_none());
            nix::unistd::close(fd).unwrap();
            let (c, group, fd) = child(finished);
            let expected_id = c.id();
            let mut error = Some(ValidationRunError::new(
                "read-output",
                io::Error::other("PRIVATE_CAUSE"),
            ));
            let status = reap_with(c, group, &mut state(finished), &mut error, |_| {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            });
            let mut error = error.unwrap();
            let preserved = error.unresolved_process_id() == Some(expected_id);
            if let Some(mut c) = error.take_unreaped_process() {
                terminate(group).unwrap();
                c.wait().unwrap();
            }
            nix::unistd::close(fd).unwrap();
            assert!(preserved, "group signal refusal lost the unreaped leader");
            assert!(status.is_none());
            assert_eq!(error.operation(), "read-output");
            assert_eq!(error.cleanup.len(), 1);
            assert_eq!(error.cleanup[0].operation, "kill-group");
            assert_eq!(
                error.cleanup[0].cause.kind(),
                io::ErrorKind::PermissionDenied
            );
            assert!(!format!("{error:?}").contains("PRIVATE_CAUSE"));
            assert_eq!(error.source().unwrap().to_string(), "PRIVATE_CAUSE");
            let (c, group, fd) = child(finished);
            let mut recovered = None;
            assert!(reap(c, group, &mut state(finished), &mut recovered).is_some());
            assert!(recovered.is_none());
            nix::unistd::close(fd).unwrap();
        }
    }
    #[test]
    fn kernel_not_child_refusal_is_diagnostic_without_a_stale_signal_capability() {
        let (c, group, fd) = child(true);
        let mut valid = None;
        assert!(reap(c, group, &mut state(true), &mut valid).is_some());
        assert!(valid.is_none());
        nix::unistd::close(fd).unwrap();
        let (mut old, _, fd) = child(true);
        old.wait().unwrap();
        let id = old.id();
        // Waiting for self is rejected by the real kernel and cannot reap an
        // unrelated process. The old fixture child is already safely reaped.
        let cause = wait4::try_wait4_pid(std::process::id()).unwrap_err();
        assert_eq!(cause.raw_os_error(), Some(nix::errno::Errno::ECHILD as i32));
        let mut error = None;
        record_wait(&mut error, id, cause);
        retain_unreaped(old, &mut error);
        let mut error = error.unwrap();
        assert_eq!(error.operation(), "wait4");
        assert_eq!(error.ownership_lost_process_id(), Some(id));
        assert!(error.unresolved_process_id().is_none());
        assert!(error.take_unreaped_process().is_none());
        assert!(error.source().is_some());
        nix::unistd::close(fd).unwrap();
        let (c, group, fd) = child(true);
        let mut recovered = None;
        assert!(reap(c, group, &mut state(true), &mut recovered).is_some());
        assert!(recovered.is_none());
        nix::unistd::close(fd).unwrap();
    }
    /// Separately drain stdout and stderr beyond pipe capacity without a hidden deadline.
    #[test]
    fn shared_runner_drains_both_streams_and_preserves_sink_failure() {
        let mut command = Command::new("python3");
        command.args(["-c","import sys;sys.stderr.buffer.write(b'e'*262144);sys.stderr.flush();sys.stdout.buffer.write(b'o'*262144);sys.stdout.flush()"]);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let process = run_owned_command(command, None, &mut stdout, Some(&mut stderr)).unwrap();
        assert!(process.usage.status.success());
        assert!(!process.timed_out);
        assert_eq!(stdout, vec![b'o'; 262144]);
        assert_eq!(stderr, vec![b'e'; 262144]);
        struct Refuse;
        impl Write for Refuse {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("fixture sink refusal"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut command = Command::new("python3");
        command.args([
            "-c",
            "import sys;sys.stdout.buffer.write(b'x'*262144);sys.stdout.flush()",
        ]);
        let error = run_owned_command(command, None, &mut Refuse, None)
            .err()
            .unwrap();
        assert_eq!(error.operation(), "retain-output");
        assert!(error
            .source()
            .unwrap()
            .to_string()
            .contains("fixture sink refusal"));
        assert!(error.cleanup().is_empty());
        assert!(error.unresolved_process_id().is_none());
        let mut command = Command::new("python3");
        command.args(["-c", "print('recovery')"]);
        let mut output = Vec::new();
        assert!(run_owned_command(command, None, &mut output, None)
            .unwrap()
            .usage
            .status
            .success());
        assert_eq!(output, b"recovery\n");
    }
}

#[cfg(test)]
mod controlled_process_tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    /// Prepare executable fixture bytes in a separate, single-threaded writer.
    /// No writable executable FD enters this concurrent test process, where a
    /// different test's fork could inherit it until exec and cause ETXTBSY.
    /// Returns a private root only after the writer exits successfully; setup
    /// errors fail with their original cause. No retry or product policy changes.
    fn executable_fixture() -> std::path::PathBuf {
        use std::{fs, os::unix::fs::DirBuilderExt};
        let root = std::env::temp_dir().join(crate::generate_identifier("exec78-").unwrap());
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let script = "import pathlib,sys; root=pathlib.Path(sys.argv[1]); a=root/'valid'; b=root/'invalid'; a.write_bytes(b\"#!/bin/sh\\nprintf 'CONTROL\\\\n'\\n\"); b.write_bytes(b\"printf 'UNEXPECTED_SHELL\\\\n'\\n\"); a.chmod(0o700); b.chmod(0o700)";
        let output = Command::new("/usr/bin/python3")
            .args(["-c", script])
            .arg(&root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "fixture writer failed: {:?}",
            output.status
        );
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
        root
    }
    /// Exercise the exact shared runner, including group and custom stream FDs:
    /// explicit shebang control succeeds, plain text refuses ENOEXEC without an
    /// implicit shell, then the same valid executable recovers. No provider,
    /// global timeout or operator policy is introduced by this fixture.
    #[test]
    fn shared_runner_refuses_plain_text_exec_without_implicit_shell_and_recovers() {
        use std::fs;
        let root = executable_fixture();
        let valid = root.join("valid");
        let invalid = root.join("invalid");
        for (stage, file) in [&valid, &invalid, &valid].into_iter().enumerate() {
            let mut output = Vec::new();
            let result = run_owned_command(Command::new(file), None, &mut output, None);
            if file == &invalid {
                let error = result.err().expect("plain text must refuse");
                assert_eq!(error.operation(), "spawn");
                assert_eq!(
                    error
                        .source()
                        .unwrap()
                        .downcast_ref::<io::Error>()
                        .unwrap()
                        .raw_os_error(),
                    Some(8)
                );
                assert!(output.is_empty());
                assert!(error.unresolved_process_id().is_none());
            } else {
                let run = result.unwrap_or_else(|e| {
                    let cause = e.source().and_then(|c| c.downcast_ref::<io::Error>());
                    panic!("valid fixture stage={stage}, operation={}, io_kind={:?}, errno={:?}, root={root:?}", e.operation(), cause.map(io::Error::kind), cause.and_then(io::Error::raw_os_error));
                });
                assert!(run.usage.status.success());
                assert_eq!(output, b"CONTROL\n");
            }
        }
        fs::remove_dir_all(root).unwrap();
    }
    /// Real open-writer control: valid executable succeeds, a separate owned
    /// writer holds its inode open and shared spawn refuses ETXTBSY, then checked
    /// writer close/wait permits recovery. No automatic retry masks the refusal.
    /// This establishes the OS hazard, not the unknown errno of wide01's failure.
    #[test]
    fn shared_runner_retains_text_busy_refusal_and_recovers_after_writer_closes() {
        let root = executable_fixture();
        let valid = root.join("valid");
        let mut before = Vec::new();
        assert!(
            run_owned_command(Command::new(&valid), None, &mut before, None)
                .unwrap()
                .usage
                .status
                .success()
        );
        assert_eq!(before, b"CONTROL\n");
        let script = "import sys; f=open(sys.argv[1],'r+b'); sys.stdout.buffer.write(b'R'); sys.stdout.buffer.flush(); sys.stdin.buffer.read(1); f.close()";
        let mut writer = Command::new("/usr/bin/python3")
            .args(["-c", script])
            .arg(&valid)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut ready = [0u8; 1];
        let observed = writer.stdout.as_mut().unwrap().read_exact(&mut ready);
        let mut output = Vec::new();
        let denied = if observed.is_ok() && ready == *b"R" {
            Some(run_owned_command(
                Command::new(&valid),
                None,
                &mut output,
                None,
            ))
        } else {
            None
        };
        let mut input = writer.stdin.take().unwrap();
        let released = input.write_all(b"X");
        let input_closed = nix::unistd::close(std::os::fd::OwnedFd::from(input));
        let output_closed =
            nix::unistd::close(std::os::fd::OwnedFd::from(writer.stdout.take().unwrap()));
        let waited = writer.wait();
        observed.unwrap();
        assert_eq!(ready, *b"R");
        released.unwrap();
        input_closed.unwrap();
        output_closed.unwrap();
        assert!(waited.unwrap().success());
        let error = denied
            .unwrap()
            .err()
            .expect("open writer must refuse, no implicit retry");
        assert_eq!(error.operation(), "spawn");
        assert_eq!(
            error
                .source()
                .unwrap()
                .downcast_ref::<io::Error>()
                .unwrap()
                .raw_os_error(),
            Some(rustix::io::Errno::TXTBSY.raw_os_error())
        );
        assert!(output.is_empty() && error.unresolved_process_id().is_none());
        let mut after = Vec::new();
        assert!(
            run_owned_command(Command::new(&valid), None, &mut after, None)
                .unwrap()
                .usage
                .status
                .success()
        );
        assert_eq!(after, b"CONTROL\n");
        std::fs::remove_dir_all(root).unwrap();
    }
    /// Fixture-owned input gate; callback refusal must leave provider work blocked.
    struct Gate {
        input: UnixStream,
        refuse: bool,
        started: bool,
    }
    impl OwnedProcessControl for Gate {
        fn started(&mut self, pid: u32) -> io::Result<()> {
            assert!(std::path::Path::new(&format!("/proc/{pid}/stat")).exists());
            self.started = true;
            if self.refuse {
                return Err(io::Error::other("fixture registration refused"));
            }
            self.input.write_all(b"1")
        }
        fn next_wake(&self) -> Option<Instant> {
            None
        }
        fn observe(&mut self) -> io::Result<Option<Signal>> {
            Ok(None)
        }
    }
    /// Actual gated child receives input only after the registration callback;
    /// refusal kills/reaps the still-blocked child, without post-gate side effects.
    /// This fixture does not establish durable SQL ownership or provider authority.
    #[test]
    fn controlled_input_registration_refusal_keeps_gate_closed_and_recovers() {
        for refuse in [false, true, false] {
            let (input, child_input) = UnixStream::pair().unwrap();
            let mut gate = Gate {
                input,
                refuse,
                started: false,
            };
            let mut command = Command::new("python3");
            command.args(["-c", "import sys; permit=sys.stdin.buffer.read(1); assert permit==b'1'; print('GATED_WORK',flush=True)"]);
            let mut output = Vec::new();
            let result = run_controlled_command(
                command,
                Stdio::from(OwnedFd::from(child_input)),
                None,
                &mut output,
                None,
                Some(&mut gate),
            );
            assert!(gate.started);
            if refuse {
                let error = result.err().unwrap();
                assert_eq!(error.operation(), "process-registration");
                assert_eq!(error.unresolved_process_id(), None);
                assert!(!output
                    .windows(b"GATED_WORK".len())
                    .any(|w| w == b"GATED_WORK"));
            } else {
                assert!(result.unwrap().usage.status.success());
                assert_eq!(output, b"GATED_WORK\n");
            }
        }
    }
    /// READY is a fixture synchronization event, not a timeout. The controller
    /// requests TERM only after the owned child blocks it for a synchronous wait.
    struct Stop {
        ready: Arc<AtomicBool>,
        sent: bool,
        leader_exited: bool,
    }
    impl OwnedProcessControl for Stop {
        fn leader_exited(&mut self) -> io::Result<()> {
            self.leader_exited = true;
            Ok(())
        }
        fn started(&mut self, _pid: u32) -> io::Result<()> {
            Ok(())
        }
        fn next_wake(&self) -> Option<Instant> {
            None
        }
        fn observe(&mut self) -> io::Result<Option<Signal>> {
            if self.ready.load(Ordering::SeqCst) && !self.sent {
                self.sent = true;
                Ok(Some(Signal::TERM))
            } else {
                Ok(None)
            }
        }
    }
    /// Retain all bytes and publish only the complete fixture-ready marker.
    struct ReadyOutput {
        ready: Arc<AtomicBool>,
        bytes: Vec<u8>,
    }
    impl Write for ReadyOutput {
        fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(raw);
            if self.bytes.windows(6).any(|w| w == b"READY\n") {
                self.ready.store(true, Ordering::SeqCst);
            }
            Ok(raw.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    /// Real owned-group TERM after READY is consumed synchronously while stdout's
    /// buffer is held. Blocking before readiness removes asynchronous pause's
    /// lost-wakeup window. The second case requeues an already-received signal
    /// while blocked, proving pending-before-wait handling without a timed guard.
    /// Direct FD output/_exit avoid buffer reentry; neither case grants provider
    /// execution, duration policy or task acceptance.
    #[test]
    fn controlled_stop_is_applied_after_real_output_and_drains_final_bytes() {
        for queued_before_wait in [false, true] {
            let ready = Arc::new(AtomicBool::new(false));
            let mut control = Stop {
                ready: ready.clone(),
                sent: false,
                leader_exited: false,
            };
            let mut output = ReadyOutput {
                ready,
                bytes: vec![],
            };
            let mut command = Command::new("python3");
            command.args([
                "-c",
                r#"import io,os,signal,sys
signals={signal.SIGTERM}
signal.pthread_sigmask(signal.SIG_BLOCK,signals)
class Held(io.RawIOBase):
 def writable(self): return True
 def write(self,b):
  if os.write(1,b)!=len(b): os._exit(3)
  received=signal.sigwait(signals)
  if received!=signal.SIGTERM: os._exit(4)
  if sys.argv[1]=='queued':
   signal.raise_signal(received)
   if received not in signal.sigpending(): os._exit(5)
   if signal.sigwait(signals)!=received: os._exit(6)
  os.write(1,b'TERM\n')
  os._exit(0)
sys.stdout=io.TextIOWrapper(io.BufferedWriter(Held()))
print('READY',flush=True)
"#,
                if queued_before_wait {
                    "queued"
                } else {
                    "either-order"
                },
            ]);
            let result = run_controlled_command(
                command,
                Stdio::null(),
                None,
                &mut output,
                None,
                Some(&mut control),
            )
            .unwrap();
            assert!(control.sent);
            assert!(control.leader_exited);
            assert!(
                result.usage.status.success(),
                "actual exit {:?}; retained {:?}; group signal {}; timeout {}",
                result.usage.status,
                String::from_utf8_lossy(&output.bytes),
                result.group_signal_delivered,
                result.timed_out
            );
            assert_eq!(output.bytes, b"READY\nTERM\n");
            assert!(!result.timed_out);
        }
    }
    /// Fixture controls keep input blocked and exercise only kernel ownership.
    /// The immediate wake is an explicit already-due coordinate, not a product
    /// timeout or empirical performance limit. Errors must retain their cause.
    struct RefusingControl {
        fault: u8,
        calls: usize,
        wake: Option<Instant>,
    }
    impl OwnedProcessControl for RefusingControl {
        fn started(&mut self, _pid: u32) -> io::Result<()> {
            Ok(())
        }
        fn next_wake(&self) -> Option<Instant> {
            self.wake
        }
        fn observe(&mut self) -> io::Result<Option<Signal>> {
            self.calls += 1;
            match self.fault {
                0 => Ok(Some(Signal::TERM)),
                1 => Ok(Some(Signal::STOP)),
                2 => Err(io::Error::other("fixture control storage failure")),
                _ if self.calls == 1 => {
                    self.wake = Some(Instant::now());
                    Ok(None)
                }
                _ => {
                    self.wake = None;
                    Ok(Some(Signal::TERM))
                }
            }
        }
    }
    /// Valid termination precedes each injected control failure with identical
    /// blocked-input preconditions. Unsupported signals cannot suspend the child;
    /// IO failure retains its cause and cleanup reaps without releasing the gate.
    /// A due control wakeup is distinct from a global process deadline.
    #[test]
    fn controlled_refusals_preserve_causes_and_due_wake_is_not_a_timeout() {
        for fault in [0, 1, 0, 2, 0, 3] {
            let (_held, child_input) = UnixStream::pair().unwrap();
            let mut command = Command::new("python3");
            command.args([
                "-c",
                "import sys; sys.stdin.buffer.read(1); print('MUST_NOT_RUN')",
            ]);
            let mut control = RefusingControl {
                fault,
                calls: 0,
                wake: None,
            };
            let mut output = Vec::new();
            let result = run_controlled_command(
                command,
                Stdio::from(OwnedFd::from(child_input)),
                None,
                &mut output,
                None,
                Some(&mut control),
            );
            assert!(output.is_empty());
            if fault == 1 || fault == 2 {
                let error = result.err().unwrap();
                assert_eq!(
                    error.operation(),
                    if fault == 1 {
                        "process-control-signal"
                    } else {
                        "process-control"
                    }
                );
                assert!(error.unresolved_process_id().is_none());
                assert!(error.cleanup().is_empty());
                if fault == 2 {
                    assert!(error
                        .source()
                        .unwrap()
                        .to_string()
                        .contains("fixture control storage failure"));
                }
            } else {
                let result = result.unwrap();
                assert!(!result.timed_out);
                assert!(!result.usage.status.success());
                if fault == 3 {
                    assert!(control.calls >= 2);
                }
            }
        }
    }
    /// Fixture-only controller: an actual absolute kernel timer wakes either
    /// common owned-child poll hook; a closed socket exercises rejection for both.
    /// It grants no provider permission, persistence, acceptance or deadline policy.
    struct KernelWakeControl {
        timer: crate::PlanningBootTimer,
        broken: Option<UnixStream>,
        interruption_hook:bool,
        primed: bool,
        wakes: u64,
    }
    impl OwnedProcessControl for KernelWakeControl {
        /// The fixture checks a real owned PID; it never releases the input gate.
        fn started(&mut self, pid: u32) -> io::Result<()> {
            assert!(std::path::Path::new(&format!("/proc/{pid}/stat")).exists());
            Ok(())
        }
        /// This fixture has no relative timer, retry or process-wide watchdog.
        fn next_wake(&self) -> Option<Instant> {
            None
        }
        /// Borrow the actual kernel timer, or the controlled HUP fault source.
        fn wake_descriptor(&self) -> Option<std::os::fd::BorrowedFd<'_>> {
            use std::os::fd::AsFd;
            if self.interruption_hook{return None;}
            Some(match &self.broken {Some(fd)=>fd.as_fd(),None=>self.timer.as_fd()})
        }
        /// Exercise the second optional hook with the same real timer/HUP inputs;
        /// this is fixture placement, not an execution policy or extra deadline.
        fn interruption_descriptor(&self)->Option<std::os::fd::BorrowedFd<'_>>{
            use std::os::fd::AsFd;
            if !self.interruption_hook{return None;}
            Some(match &self.broken {Some(fd)=>fd.as_fd(),None=>self.timer.as_fd()})
        }
        /// Arm an already due timer on the first call, requiring a poll return
        /// before consuming it on the second. A real expiration asks KILL for
        /// only this owned blocked child; no artificial time limit is introduced.
        fn observe(&mut self) -> io::Result<Option<Signal>> {
            if !self.primed {
                self.primed = true;
                self.timer.arm(Some(crate::PlanningBootInstant::now()?))?;
                return Ok(None);
            }
            if let Some(count) = self.timer.take_expirations()? {
                self.wakes += count;
                self.timer.arm(None)?;
                return Ok(Some(Signal::KILL));
            }
            Ok(None)
        }
    }

    /// Valid silent-child timer controls bracket an injected HUP under the same
    /// blocked-input preconditions. Real signals/reaping and preserved failure
    /// cause, no output/gate release and successful recovery are asserted without
    /// a watchdog. This exercises the shared runner, not an actual model/planner.
    #[test]
    fn controlled_kernel_wake_and_bad_descriptor_cleanup_recover_without_timeout() {
        use std::os::unix::process::ExitStatusExt;
        for (interruption_hook,fault) in [false,true].into_iter().flat_map(|hook|[false,true,false].into_iter().map(move|fault|(hook,fault))){
            let (_held, child_input) = UnixStream::pair().unwrap();
            let broken = if fault {
                let (reader, peer) = UnixStream::pair().unwrap();
                nix::unistd::close(peer).unwrap();
                Some(reader)
            } else {
                None
            };
            let mut control = KernelWakeControl {
                timer: crate::PlanningBootTimer::new().unwrap(),
                broken,
                interruption_hook,
                primed: false,
                wakes: 0,
            };
            let mut command = Command::new("python3");
            command.args([
                "-c",
                "import sys; sys.stdin.buffer.read(1); print('MUST_NOT_RUN')",
            ]);
            let mut output = Vec::new();
            let result = run_controlled_command(
                command,
                Stdio::from(OwnedFd::from(child_input)),
                None,
                &mut output,
                None,
                Some(&mut control),
            );
            assert!(output.is_empty());
            assert!(control.primed);
            if fault {
                let error = result.err().unwrap();
                assert_eq!(error.operation(), "process-control-descriptor");
                assert!(error
                    .source()
                    .unwrap()
                    .to_string()
                    .contains("closed or failed control descriptor"));
                assert_eq!(error.unresolved_process_id(), None);
                assert!(error.cleanup().is_empty());
                assert_eq!(error.reaped_exit_status().unwrap().signal(), Some(9));
                assert_eq!(control.wakes, 0);
            } else {
                let run = result.unwrap();
                assert!(!run.timed_out);
                assert_eq!(run.usage.status.signal(), Some(9));
                assert_eq!(control.wakes, 1);
            }
            control.timer.close().unwrap();
            if let Some(fd) = control.broken {
                nix::unistd::close(fd).unwrap();
            }
        }
    }
}

#[cfg(target_os="linux")]
#[path="validation_run_recovery.rs"]
mod recovery;
#[cfg(target_os="linux")]
pub(crate) use recovery::{poll_retained_processes,has_unresolved_process};

#[cfg(all(test,target_os="linux"))]
pub(crate) use recovery::retained_child_for_owner_test;
