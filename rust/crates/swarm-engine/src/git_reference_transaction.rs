//! Owned interactive Git protocol, not a PAEA admission or effect service.
use crate::validation_run::{run_controlled_command, OwnedProcessControl};
use crate::{GitCommandOutput, ValidationRunError};
use rustix::{
    event::{poll, PollFd, PollFlags, Timespec},
    process::Signal,
};
use std::{
    cell::RefCell,
    error::Error,
    fmt,
    io::{self, Write},
    os::{
        fd::{BorrowedFd, OwnedFd},
        unix::net::UnixStream,
    },
    process::{Command, ExitStatus, Stdio},
    rc::Rc,
    time::Instant,
};
use swarm_protection_contract::{Action, Proposal};

/// Trusted caller's decision after Git has prepared and retained its ref lock.
/// Commit requires current admission and a direct-ref check; this enum proves none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitReferenceDecision {
    Commit,
    Abort,
}
/// Observed protocol phase only, never an independently observed business effect.
/// NotRequested applies to this call; it says nothing about historical effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitReferenceObservation {
    NotRequested,
    CommitRequested,
    CommitAcknowledged,
    AbortAcknowledged,
}
impl GitReferenceObservation {
    /// Safe fixed diagnostic label. A requested commit requires reconciliation
    /// after failure; an acknowledgment alone does not authenticate the executable.
    pub fn code(self) -> &'static str {
        match self {
            Self::NotRequested => "git-reference-commit-not-requested",
            Self::CommitRequested => "git-reference-commit-requested",
            Self::CommitAcknowledged => "git-reference-commit-acknowledged",
            Self::AbortAcknowledged => "git-reference-abort-acknowledged",
        }
    }
}
/// Terminal protocol with successful owned-process exit. Caller still observes
/// the real ref and owes durable intent, protected custody and independent proof.
pub struct GitReferenceRun {
    pub observation: GitReferenceObservation,
    pub exit_status: ExitStatus,
    output: GitCommandOutput,
}
impl GitReferenceRun {
    /// Exact private streams; never render blindly. ACK bytes confer no authority.
    pub fn output(&self) -> &GitCommandOutput {
        &self.output
    }
}
impl fmt::Debug for GitReferenceRun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GitReferenceRun")
            .field("observation", &self.observation)
            .finish()
    }
}
/// Safe error retaining protocol observation, primary/process ownership and all
/// independent input-close failures. No variant establishes rollback or no effect.
pub struct GitReferenceError {
    code: &'static str,
    pub observation: GitReferenceObservation,
    cause: Box<dyn Error + Send + Sync>,
    cleanup: Vec<Box<dyn Error + Send + Sync>>,
    pub exit_status: Option<ExitStatus>,
    pub timed_out: Option<bool>,
    output: GitCommandOutput,
}
impl GitReferenceError {
    /// Construct a pre-process failure; private output stays empty, not invented.
    fn new(code: &'static str, cause: impl Error + Send + Sync + 'static) -> Self {
        Self {
            code,
            observation: GitReferenceObservation::NotRequested,
            cause: Box::new(cause),
            cleanup: vec![],
            exit_status: None,
            timed_out: None,
            output: GitCommandOutput::from_parts(vec![], vec![]),
        }
    }
    /// Fixed diagnostic only; private paths, command text and output are excluded.
    pub fn code(&self) -> &'static str {
        self.code
    }
    /// Exact observed private output, possibly partial. This accessor is not redacted.
    pub fn output(&self) -> &GitCommandOutput {
        &self.output
    }
    /// Primary and independent cleanup branches; source() supplies the first only.
    pub fn branches(&self) -> Vec<&(dyn Error + 'static)> {
        let mut out = vec![self.cause.as_ref() as &(dyn Error + 'static)];
        out.extend(
            self.cleanup
                .iter()
                .map(|e| e.as_ref() as &(dyn Error + 'static)),
        );
        out
    }
    /// Preserve owned-process recovery, including any unreaped Child, after an
    /// actual runner failure. No child is fabricated for preflight/exit refusals.
    pub fn process_error_mut(&mut self) -> Option<&mut ValidationRunError> {
        if self.cause.is::<ValidationRunError>() {
            return self.cause.downcast_mut();
        }
        self.cleanup.iter_mut().find_map(|e| e.downcast_mut())
    }
    /// Find this envelope through the existing concrete Box/IO/Root graph, with
    /// cycle detection and no message parsing. First call wins; never merge calls.
    pub fn find<'a>(error: &'a (dyn Error + 'static)) -> Option<&'a Self> {
        let mut pending = vec![error];
        let mut seen: Vec<*const dyn Error> = vec![];
        while let Some(e) = pending.pop() {
            let p = e as *const dyn Error;
            if seen.iter().any(|q| std::ptr::eq(*q, p)) {
                continue;
            }
            seen.push(p);
            if let Some(found) = e.downcast_ref::<Self>() {
                return Some(found);
            }
            pending.extend(
                crate::native_error_links(e)
                    .children
                    .into_iter()
                    .rev()
                    .map(|(_, e)| e),
            );
        }
        None
    }
}
impl fmt::Display for GitReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl fmt::Debug for GitReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for GitReferenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Start,
    Prepare,
    Commit,
    Abort,
    Complete,
}
struct Protocol {
    writer: Option<UnixStream>,
    phase: Phase,
    offset: usize,
    observation: GitReferenceObservation,
    refusal: Option<Box<dyn Error + Send + Sync>>,
}
/// Checked close of our input endpoint, never an implicit protocol commit.
fn close(writer: UnixStream) -> io::Result<()> {
    nix::unistd::close(writer).map_err(io::Error::from)
}
/// Nonblocking cancellation/descriptor observation and caller-supplied monotonic
/// deadline. No timer, retry, read/acknowledgment or new execution policy invented.
fn check(deadline: Option<Instant>, cancel: Option<BorrowedFd<'_>>) -> io::Result<()> {
    if deadline.is_some_and(|d| Instant::now() >= d) {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "git-reference-deadline",
        ));
    }
    if let Some(fd) = cancel {
        let mut fds = [PollFd::from_borrowed_fd(fd, PollFlags::IN)];
        poll(
            &mut fds,
            Some(&Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            }),
        )
        .map_err(io::Error::from)?;
        let ready = fds[0].revents();
        if ready.intersects(PollFlags::ERR | PollFlags::HUP | PollFlags::NVAL) {
            return Err(io::Error::other("git-reference-cancellation-descriptor"));
        }
        if ready.contains(PollFlags::IN) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "git-reference-cancelled",
            ));
        }
    }
    Ok(())
}
struct Control<'a> {
    state: Rc<RefCell<Protocol>>,
    preamble: Vec<u8>,
    deadline: Option<Instant>,
    cancel: Option<BorrowedFd<'a>>,
}
impl OwnedProcessControl for Control<'_> {
    /// Send no input until the existing runner has acquired the real pidfd.
    fn started(&mut self, _pid: u32) -> io::Result<()> {
        check(self.deadline, self.cancel)?;
        self.state
            .borrow_mut()
            .writer
            .as_mut()
            .ok_or_else(|| io::Error::other("git-reference-input-missing"))?
            .write_all(&self.preamble)
    }
    fn next_wake(&self) -> Option<Instant> {
        None
    }
    fn interruption_descriptor(&self) -> Option<BorrowedFd<'_>> {
        self.cancel
    }
    /// Readability is refusal, never permission. Parent owns/drains its cancel FD.
    fn observe(&mut self) -> io::Result<Option<Signal>> {
        check(self.deadline, self.cancel)?;
        Ok(None)
    }
}
struct Sink<'a, F> {
    state: Rc<RefCell<Protocol>>,
    decide: Option<F>,
    deadline: Option<Instant>,
    cancel: Option<BorrowedFd<'a>>,
    raw: Vec<u8>,
}
impl<F: FnOnce() -> Result<GitReferenceDecision, Box<dyn Error + Send + Sync>>> Write
    for Sink<'_, F>
{
    /// Parse exact finite ACKs incrementally. Decide only once after prepare;
    /// check interruption/deadline again before sending any COMMIT byte. A write
    /// failure after that decision conservatively retains CommitRequested.
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let outcome: io::Result<usize> = (|| {
            self.raw.extend_from_slice(bytes);
            for &byte in bytes {
                let complete = {
                    let mut s = self.state.borrow_mut();
                    let expected: &[u8] = match s.phase {
                        Phase::Start => b"start: ok\n",
                        Phase::Prepare => b"prepare: ok\n",
                        Phase::Commit => b"commit: ok\n",
                        Phase::Abort => b"abort: ok\n",
                        Phase::Complete => {
                            return Err(io::Error::other("git-reference-extra-output"))
                        }
                    };
                    if expected.get(s.offset) != Some(&byte) {
                        return Err(io::Error::other("git-reference-invalid-ack"));
                    }
                    s.offset += 1;
                    if s.offset == expected.len() {
                        s.offset = 0;
                        Some(s.phase)
                    } else {
                        None
                    }
                };
                match complete {
                    Some(Phase::Start) => self.state.borrow_mut().phase = Phase::Prepare,
                    Some(Phase::Prepare) => {
                        check(self.deadline, self.cancel)?;
                        let decision = match self
                            .decide
                            .take()
                            .ok_or_else(|| io::Error::other("git-reference-decision-missing"))?(
                        ) {
                            Ok(decision) => decision,
                            Err(cause) => {
                                self.state.borrow_mut().refusal = Some(cause);
                                GitReferenceDecision::Abort
                            }
                        };
                        check(self.deadline, self.cancel)?;
                        let mut s = self.state.borrow_mut();
                        let input: &[u8] = match decision {
                            GitReferenceDecision::Commit => {
                                s.observation = GitReferenceObservation::CommitRequested;
                                s.phase = Phase::Commit;
                                b"commit\n"
                            }
                            GitReferenceDecision::Abort => {
                                s.phase = Phase::Abort;
                                b"abort\n"
                            }
                        };
                        s.writer
                            .as_mut()
                            .ok_or_else(|| io::Error::other("git-reference-input-missing"))?
                            .write_all(input)?;
                    }
                    Some(Phase::Commit) | Some(Phase::Abort) => {
                        let mut s = self.state.borrow_mut();
                        s.observation = if s.phase == Phase::Commit {
                            GitReferenceObservation::CommitAcknowledged
                        } else {
                            GitReferenceObservation::AbortAcknowledged
                        };
                        s.phase = Phase::Complete;
                        if let Some(writer) = s.writer.take() {
                            close(writer)?
                        }
                    }
                    Some(Phase::Complete) | None => (),
                }
            }
            Ok(bytes.len())
        })();
        // State may already have consumed this ACK. Persistent cancellation or
        // an interrupted check is not a retryable, untouched OS write. Preserve
        // the original IO cause behind Other so Write::write_all cannot replay
        // the same bytes forever and bypass the owned process observer.
        outcome.map_err(|cause| {
            if cause.kind() == io::ErrorKind::Interrupted {
                io::Error::other(cause)
            } else {
                cause
            }
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Run a trusted, preconfigured Git command's interactive reference transaction.
/// Append only update-ref --no-deref --stdin after the caller's executable/options.
/// Validate the actual v1 Git proposal representation before spawning. Caller
/// supplies cwd/environment/tool identity, retained repository descriptors, saved
/// monotonic deadline (None means none), and an optional owned readable cancel FD.
/// The FD is borrowed and not drained/closed. Never pass an untrusted executable.
///
/// The synchronous decision runs once under Git's prepared lock; it MUST verify
/// direct reference type, exact OLD, current authority and durable intent, and must
/// not block outside the caller's control/deadline. Abort commits no new update.
/// Acknowledgments are protocol observations, not an independently read ref or a
/// permission. After any requested COMMIT, reconcile before retrying. This function
/// provisions no repository, PAEA grant, mandate, daemon, human or OS isolation.
///
/// A decision error attempts an explicit abort and retains the decision cause.
/// Forced process termination can leave Git lockfiles; this primitive never
/// removes them. Caller-owned durable recovery must establish their custody and
/// actual ref state before another publication. No filesystem cleanup is inferred.
/// Errors retain original causes, private partial streams, independent close
/// errors and actual runner-owned process recovery. Nonblocking input may refuse
/// partial/oversized transport instead of blocking outside the process observer.
/// Cancellation or expiry prevents sending COMMIT when observed before that send;
/// no real-time atomicity between a clock check and the kernel write is claimed.
pub fn run_git_reference_transaction<F>(
    mut git: Command,
    proposal: &Proposal,
    deadline: Option<Instant>,
    cancel: Option<BorrowedFd<'_>>,
    decide: F,
) -> Result<GitReferenceRun, GitReferenceError>
where
    F: FnOnce() -> Result<GitReferenceDecision, Box<dyn Error + Send + Sync>>,
{
    proposal
        .validate()
        .map_err(|e| GitReferenceError::new("git-reference-proposal", e))?;
    let Action::Git {
        target_ref,
        expected_old_oid,
        candidate_oid,
        ..
    } = &proposal.action
    else {
        return Err(GitReferenceError::new(
            "git-reference-domain",
            io::Error::other("Git proposal required"),
        ));
    };
    check(deadline, cancel).map_err(|e| GitReferenceError::new("git-reference-preflight", e))?;
    let (reader, writer) =
        UnixStream::pair().map_err(|e| GitReferenceError::new("git-reference-input", e))?;
    if let Err(cause) = writer.set_nonblocking(true) {
        let mut e = GitReferenceError::new("git-reference-nonblocking-input", cause);
        for s in [reader, writer] {
            if let Err(c) = close(s) {
                e.cleanup.push(Box::new(c))
            }
        }
        return Err(e);
    }
    let state = Rc::new(RefCell::new(Protocol {
        writer: Some(writer),
        phase: Phase::Start,
        offset: 0,
        observation: GitReferenceObservation::NotRequested,
        refusal: None,
    }));
    let mut control = Control {
        state: state.clone(),
        preamble: format!(
            "start\nupdate {target_ref} {candidate_oid} {expected_old_oid}\nprepare\n"
        )
        .into_bytes(),
        deadline,
        cancel,
    };
    let mut sink = Sink {
        state: state.clone(),
        decide: Some(decide),
        deadline,
        cancel,
        raw: vec![],
    };
    let mut stderr = vec![];
    git.args(["update-ref", "--no-deref", "--stdin"]);
    let run = run_controlled_command(
        git,
        Stdio::from(OwnedFd::from(reader)),
        deadline,
        &mut sink,
        Some(&mut stderr),
        Some(&mut control),
    );
    let mut s = state.borrow_mut();
    let mut cleanup = vec![];
    if let Some(writer) = s.writer.take() {
        if let Err(e) = close(writer) {
            cleanup.push(Box::new(e) as Box<dyn Error + Send + Sync>)
        }
    }
    let output = GitCommandOutput::from_parts(sink.raw, stderr);
    let observation = s.observation;
    let (exit_status, timed_out) = match &run {
        Ok(r) => (Some(r.usage.status), Some(r.timed_out)),
        Err(e) => (e.reaped_exit_status(), None),
    };
    let process_failure: Option<(&'static str, Box<dyn Error + Send + Sync>)> = match run {
        Err(e) => Some(("git-reference-process", Box::new(e))),
        Ok(run) if run.timed_out || !run.usage.status.success() => Some((
            "git-reference-exit",
            Box::new(io::Error::other("owned Git exit refused")),
        )),
        Ok(_) if s.phase != Phase::Complete => Some((
            "git-reference-incomplete",
            Box::new(io::Error::other("missing terminal acknowledgment")),
        )),
        Ok(_) => None,
    };
    if let Some(cause) = s.refusal.take() {
        if let Some((_, e)) = process_failure {
            cleanup.push(e)
        }
        return Err(GitReferenceError {
            code: "git-reference-decision",
            observation,
            cause,
            cleanup,
            exit_status,
            timed_out,
            output,
        });
    }
    if let Some((code, cause)) = process_failure {
        return Err(GitReferenceError {
            code,
            observation,
            cause,
            cleanup,
            exit_status,
            timed_out,
            output,
        });
    }
    if !cleanup.is_empty() {
        let cause = cleanup.remove(0);
        return Err(GitReferenceError {
            code: "git-reference-input-close",
            observation,
            cause,
            cleanup,
            exit_status,
            timed_out,
            output,
        });
    }
    Ok(GitReferenceRun {
        observation,
        exit_status: exit_status.expect("successful owned run has status"),
        output,
    })
}
