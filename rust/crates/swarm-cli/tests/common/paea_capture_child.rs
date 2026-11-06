//! CHILD raw-capture owner for the PAEA approval-web child evidence work:
//! owns a caller-created `std::process::Child` (spawned by the caller with
//! stdout AND stderr piped), reserves BOTH evidence files exclusively
//! BEFORE any pipe transfer, then drains BOTH pipes concurrently through
//! the existing `LiveRecordedStream` — each stream started on its REAL
//! reserved descriptor — into two exclusively new 0600 evidence files:
//! ONE conservation contract (the shared `conserve_chunks` core), never a
//! duplicated loop. The child's stdin and the live stdout capture
//! stay exposed for the future stdin/prepared/bound protocol (incremental
//! reads before producer EOF), and `finish` closes stdin explicitly, waits
//! the child's REAL termination (exit code or signal — a natural nonzero
//! exit is a kept result, never success) and joins BOTH capture streams,
//! returning the exit status, both capture issues and both evidence paths;
//! a refused wait hands the child, BOTH still-running captures, both paths
//! and the original cause back UNRESOLVED (`WaitRefusalHandback`) — never
//! a second wait attempt; an unexpected refusal is conserved by the
//! test-side helpers into a COMPLETE typed carrier
//! (`WaitRefusalCarrier`: cause, same child, both capture terminal
//! states, both paths, any retained controller panic) that is really
//! transferred to the caller-level decision point and failed with as a
//! typed recoverable payload — the child is never re-waited by
//! conservation and its exit status stays UNKNOWN.
//!
//! Deliberately NOT in this lot (next sub-tasks, explicitly not started):
//! the setup-refusal campaign, any business wiring (approval-web witness,
//! Root PROJET / normal CLI profile) and any `/proc/<pid>/exe` identity
//! proof. This owner is qualified for NORMAL explicit usage plus TWO
//! wrappers. The COOPERATIVE controller wrapper below
//! (`run_controller_then_finish`): the wrapper keeps the capture owned
//! across the controller call and ALWAYS runs the explicit finish — on
//! the normal path after the controller returned, on the panic path after
//! `catch_unwind` — returning the complete real outcome AND the original
//! panic payload verbatim; it qualifies ONLY a child whose own protocol
//! establishes a cooperative stop at stdin EOF (the guards' scripts block
//! on a single `read(1)`) and is neither arbitrary-child cancellation nor
//! a general Drop mechanism. The EXPLICIT signal wrapper (`signal_stop`)
//! below covers a child that IGNORES stdin EOF: the caller provides the
//! signal decision itself (no implicit timeout, policy or retry), the
//! wrapper holds a pidfd created DIRECTLY from the owned child BEFORE the
//! controller call (never an arbitrary-PID API), requests the signal
//! through that descriptor on the normal AND the panic path, and only
//! after a successful request runs the existing explicit finish (stdin
//! EOF, real wait, both joins); a refused preparation or signal returns
//! the FULL ownership (child, still-running captures, paths, pidfd when
//! acquired, original cause and any controller panic) typed and
//! UNRESOLVED — no blind wait, no fake success, no silent drop. The
//! stdin/live-output causal handshake and
//! the controller-panic retention are exercised by the dedicated guards
//! below (test only; the pre-existing owner operations are unchanged).
//!
//! Contract highlights (details on each item):
//! * operations act ONLY on the owned `Child` handle (`wait` through it);
//!   no arbitrary PID is ever signalled — `id()` is diagnostic only and a
//!   pid may be recycled after exit, which is why an identity proof is a
//!   future task, never claimed here;
//! * no `Drop`: dropping without `finish` is a documented programming
//!   error — the child keeps running, the workers keep draining and
//!   nothing is silently cleaned or killed (no silent-error drop);
//! * causal waiting only: live progress via the shared condvar, the real
//!   `Child::wait`, then the joins — no timer, poll, retry, volume ceiling
//!   and no global environment mutation anywhere;
//! * wait-refusal handback: exactly ONE wait attempt per finish (the
//!   public path passes the real `Child::wait` through the private seam
//!   handler); a refusal returns the original cause, the child, BOTH
//!   still-running captures and both evidence paths, and the exit status
//!   stays UNKNOWN — no automatic re-wait exists anywhere in this
//!   module; an unexpected refusal is conserved into the COMPLETE typed
//!   `WaitRefusalCarrier` (nothing dropped, any retained controller
//!   panic travels with it) and transferred really to the caller-level
//!   decision point as a typed recoverable panic payload, while
//!   resolving a refused wait is the CALLER's explicit decision (the
//!   dedicated guard below pins this with a SYNTHETIC injection AND
//!   exercises the real typed boundary end-to-end: conservation,
//!   `fail_unexpected_wait_refusal`, `catch_unwind`, BY-VALUE recovery
//!   of the carrier, then the FIRST real wait);
//! * every start refusal keeps the real cause and returns the child with
//!   ALL pipes it initially owned intact — pipe preconditions are
//!   verified WITHOUT taking and evidence files are reserved exclusively
//!   before any irreversible pipe transfer, so no capture is ever started
//!   before a refusal; an evidence refusal is either stage `Open`
//!   (exclusive creation refused — no newly created target at that path)
//!   or stage `ReaderPrepare` (creation succeeded, consumer-descriptor
//!   preparation refused — the identified EMPTY 0600 target created at
//!   the caller-provided path is preserved on disk, never deleted); a
//!   stderr-evidence refusal additionally conserves the identified EMPTY
//!   stdout artifact of its dropped reservation (closed descriptor, no
//!   worker) and can therefore keep BOTH the stdout artifact AND the
//!   empty stderr target — a setup problem never becomes an empty success
//!   and never deletes the original evidence;
//! * guards use the shared `private_fixture_leaf` under the runner's
//!   explicit TMPDIR (never /tmp), keep the leaf on failure, and spawn a
//!   real local interpreter with a constant script only (no provider, no
//!   live DB, no UID/enrollment) — synthetic children prove no business
//!   acceptance.
//!
//! Repair history (separate lots; archived evidence never rewritten): the
//! tests-only pinning lot added the two refusal guards below and delivered
//! FINAL NON EXÉCUTÉ; the real Codex run of that lot confirmed the two
//! lost-ownership defects it pinned — `start` took the stdout pipe BEFORE
//! verifying the stderr pipe and dropped it at the `StderrNotPiped`
//! refusal, and the `StdoutEvidenceRefused` refusal dropped BOTH pipes
//! (1 PASS / 2 FAILED / 0 ignored / 12 filtered, exit 101, green
//! compilation, sources and FAIL originals archived). THIS lot repairs the
//! general contract those pins targeted: preconditions verified without
//! taking, both reservations before any pipe transfer, streams started on
//! the real reserved descriptors, and every refusal returning the child
//! with all initially-owned pipes intact plus the real cause. The two
//! guards below are now regression pins of that repaired contract.
//! One later addition (EOF transfer API, no guard change): `take_stdin`
//! moves the real owned stdin end to the caller so an explicit drop can
//! deliver the child's protocol EOF before `finish`.
//!
//! Compiled only inside the `paea_approval_web` integration test crate
//! (Linux-only through its crate-level `cfg`), with no new dependency.

use std::fs;
use std::io::Read;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::paea_capture_live::{LiveRecordedStream, LiveStreamState};
use super::paea_capture_stream::{private_fixture_leaf, StreamCaptureError};

/// Owner of one freshly spawned child and its TWO concurrent raw-stream
/// captures. Created only by [`OwnedChildCapture::start`] from a child the
/// caller just spawned with both output pipes; consumed exactly once by
/// [`OwnedChildCapture::finish`].
///
/// Usage class and limits (NORMAL EXPLICIT plus the COOPERATIVE wrapper
/// below): the caller spawns, reads or waits through the exposed handles,
/// then calls `finish` explicitly — or hands the capture to
/// `run_controller_then_finish`, which qualifies PANICKING controllers
/// while keeping the finish explicit (stdin EOF, real wait, both joins)
/// on both the normal and the panic path — or to `signal_stop`, which
/// qualifies a child that IGNORES stdin EOF through an EXPLICIT
/// caller-provided signal request bound to a pidfd held before the
/// controller call (see that method's own contract), and delivers stdin
/// to the caller by transfer (`take_stdin`) when the protocol requires an
/// explicit EOF drop before further output. This lot does NOT qualify the
/// owner for setup refusals beyond the ownership-preserving refusals
/// below, or business integration. There is NO `Drop`: dropping an unfinished owner leaks the
/// still-running child and its two worker threads by design (nothing is
/// silently killed or closed); a child that never exits keeps `finish`
/// waiting forever (no timer, no kill — explicit cancellation is the
/// `signal_stop` decision wrapper, never automatic). If the child hands its pipes to grandchildren, pipe EOF (and
/// therefore the joins) may outlast the direct child's exit — plain Unix
/// pipe semantics the future integration must account for.
pub struct OwnedChildCapture {
    child: std::process::Child,
    stdout_stream: LiveRecordedStream,
    stderr_stream: LiveRecordedStream,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

impl std::fmt::Debug for OwnedChildCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedChildCapture")
            .field("child_pid", &self.child.id())
            .field("stdout_path", &self.stdout_path)
            .field("stderr_path", &self.stderr_path)
            .finish()
    }
}

/// Start refusal that NEVER abandons what the child already owned: every
/// variant returns the child with ALL pipes it initially owned still in
/// place (nothing killed, nothing deleted, NO capture started —
/// reservations and their preparations happen before any pipe transfer),
/// and the stderr-evidence variant additionally names the conserved stdout
/// artifact of the reservation dropped at the refusal (an identified EMPTY
/// 0600 file, closed descriptor, no worker — conserved, never deleted).
/// An evidence refusal is stage `Open` (exclusive creation refused — no
/// newly created target at that path) or stage `ReaderPrepare` (creation
/// succeeded, consumer-descriptor preparation refused — the identified
/// EMPTY 0600 target created at the caller-provided path is preserved on
/// disk); see each variant's doc.
pub enum OwnedChildCaptureStartError {
    /// The caller's child has no piped stdout (precondition violation);
    /// the untouched child is returned, nothing was started or reserved.
    StdoutNotPiped { child: std::process::Child },
    /// The caller's child has no piped stderr (precondition violation);
    /// the child is returned with its stdout pipe still in place (the
    /// pipe was verified WITHOUT being taken), nothing was reserved.
    StderrNotPiped { child: std::process::Child },
    /// The stdout evidence reservation refused; the child is returned with
    /// both pipes still in place (verified without taking). Stage `Open`:
    /// the exclusive creation refused — no file was created at the
    /// caller-provided stdout path, nothing to conserve. Stage
    /// `ReaderPrepare`: the exclusive creation SUCCEEDED and the
    /// consumer-descriptor preparation refused afterwards — the identified
    /// EMPTY 0600 target created at the caller-provided stdout path is
    /// PRESERVED on disk (never deleted, never reported as "no effect";
    /// closed descriptor, no worker), and `error` carries the real
    /// preparation cause with zero confirmed bytes and no flush error.
    StdoutEvidenceRefused {
        child: std::process::Child,
        error: StreamCaptureError,
    },
    /// The stderr evidence reservation refused AFTER the stdout
    /// reservation succeeded: the child is returned with BOTH pipes still
    /// in place, and `stdout_artifact_path` names the stdout reservation
    /// dropped at the refusal — an identified EMPTY 0600 file with a
    /// closed descriptor and NO worker, deliberately conserved as partial
    /// evidence (never deleted, never reported as a started capture).
    /// Stage `Open`: the stderr exclusive creation refused — no file was
    /// created at `stderr_path`. Stage `ReaderPrepare`: the stderr
    /// exclusive creation SUCCEEDED and its consumer-descriptor
    /// preparation refused afterwards — the identified EMPTY 0600 stderr
    /// target created at `stderr_path` is ALSO preserved on disk (never
    /// deleted; closed descriptor, no worker), so a stderr failure can
    /// keep BOTH the reserved stdout artifact AND the empty stderr target.
    StderrEvidenceRefused {
        child: std::process::Child,
        stdout_artifact_path: PathBuf,
        stderr_path: PathBuf,
        error: StreamCaptureError,
    },
}

impl std::fmt::Debug for OwnedChildCaptureStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StdoutNotPiped { child } => f
                .debug_struct("StdoutNotPiped")
                .field("child_pid", &child.id())
                .finish(),
            Self::StderrNotPiped { child } => f
                .debug_struct("StderrNotPiped")
                .field("child_pid", &child.id())
                .finish(),
            Self::StdoutEvidenceRefused { child, error } => f
                .debug_struct("StdoutEvidenceRefused")
                .field("child_pid", &child.id())
                .field("error", error)
                .finish(),
            Self::StderrEvidenceRefused {
                child,
                stdout_artifact_path,
                stderr_path,
                error,
            } => f
                .debug_struct("StderrEvidenceRefused")
                .field("child_pid", &child.id())
                .field("stdout_artifact_path", stdout_artifact_path)
                .field("stderr_path", stderr_path)
                .field("error", error)
                .finish(),
        }
    }
}

/// UNRESOLVED handback after a refused wait: everything the caller needs
/// to decide explicitly what happens next — the ORIGINAL (non-Clone)
/// refusal cause moved verbatim, the child with its stdin ALREADY closed
/// (the EOF permission was given before the wait), BOTH still-running
/// captures (their pipes are held by their workers; no join ran on the
/// refusal path, so these are NOT terminal states) and both evidence
/// paths. The exit status is UNKNOWN and stays UNKNOWN: this struct never
/// invents a status, a signal or a termination. Resolving this handback
/// (re-waiting the child, joining the captures) is the CALLER's explicit
/// decision — nothing in this module does it automatically.
pub struct WaitRefusalHandback {
    /// The original wait refusal cause, moved verbatim (`io::Error` is
    /// not Clone — never copied or re-created).
    pub error: std::io::Error,
    /// The owned child, stdin already closed (EOF permission given).
    pub child: std::process::Child,
    /// The still-RUNNING stdout capture (NOT joined — no terminal state).
    pub stdout_stream: LiveRecordedStream,
    /// The still-RUNNING stderr capture (NOT joined — no terminal state).
    pub stderr_stream: LiveRecordedStream,
    /// Evidence path of the stdout capture.
    pub stdout_path: PathBuf,
    /// Evidence path of the stderr capture.
    pub stderr_path: PathBuf,
}

impl std::fmt::Debug for WaitRefusalHandback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaitRefusalHandback")
            .field("error", &self.error)
            .field("child_pid", &self.child.id())
            .field("stdout_path", &self.stdout_path)
            .field("stderr_path", &self.stderr_path)
            .field("streams", &"LiveRecordedStream(..) x2, still running")
            .finish()
    }
}

/// Outcome of an explicit [`OwnedChildCapture::finish`]: the REAL child
/// termination plus BOTH joined capture outcomes — never a masked error,
/// never a nonzero exit turned into success. Capture `Failed`/`Panicked`
/// states are delivered verbatim (see `LiveStreamState`).
pub enum ChildCaptureOutcome {
    /// `Child::wait` succeeded. `exit` is the real `ExitStatus` (code or
    /// signal; nonzero is a KEPT result for the caller to judge), and both
    /// captures are joined terminal states with their evidence paths.
    Finished {
        exit: std::process::ExitStatus,
        stdout_capture: LiveStreamState,
        stderr_capture: LiveStreamState,
        stdout_path: PathBuf,
        stderr_path: PathBuf,
    },
    /// The wait attempt failed: the exit status is UNKNOWN and nothing is
    /// faked. The complete [`WaitRefusalHandback`] — original cause, child,
    /// BOTH still-running captures, both evidence paths — is returned
    /// UNRESOLVED for the caller to decide explicitly (no automatic
    /// re-wait exists in this module; see the seam contract).
    WaitRefused(WaitRefusalHandback),
}

impl std::fmt::Debug for ChildCaptureOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Finished {
                exit,
                stdout_capture,
                stderr_capture,
                stdout_path,
                stderr_path,
            } => f
                .debug_struct("Finished")
                .field("exit", exit)
                .field("stdout_capture", stdout_capture)
                .field("stderr_capture", stderr_capture)
                .field("stdout_path", stdout_path)
                .field("stderr_path", stderr_path)
                .finish(),
            Self::WaitRefused(handback) => f
                .debug_struct("WaitRefused")
                .field("handback", handback)
                .finish(),
        }
    }
}

impl OwnedChildCapture {
    /// Takes over `child` (spawned by the caller with stdout AND stderr
    /// piped — a precondition, not a guess) and starts BOTH captures
    /// immediately and concurrently through the existing
    /// `LiveRecordedStream` into two exclusively reserved files
    /// (`create_new`, 0600, via the shared cores — no second conservation
    /// loop), each stream started on its REAL reserved descriptor.
    ///
    /// Inputs: `child` freshly created and owned by the caller;
    /// `stdout_evidence`/`stderr_evidence` new explicit paths inside the
    /// runner's private Unix area (the guards use `private_fixture_leaf`).
    ///
    /// Order (the ownership repair): (1) both pipe preconditions are
    /// verified WITHOUT taking any pipe; (2) BOTH evidence files are then
    /// reserved exclusively BEFORE any irreversible pipe transfer — so no
    /// capture is ever started before a refusal and no pipe is ever lost;
    /// (3) only then are both pipes taken and each stream started on its
    /// real reserved descriptor.
    ///
    /// Effects on success: takes both pipes and spawns exactly two worker
    /// threads. The child itself is not started, signalled or
    /// reconfigured here.
    ///
    /// Errors: see `OwnedChildCaptureStartError` — every refusal returns
    /// the child with ALL pipes it initially owned still in place plus the
    /// real cause (both reservations and their preparations happen BEFORE
    /// the irreversible pipe transfer of step (3), so no pipe is ever
    /// taken or lost before a refusal). An evidence refusal is stage
    /// `Open` (exclusive creation refused — no newly created target at
    /// that path) or stage `ReaderPrepare` (creation succeeded,
    /// consumer-descriptor preparation refused — the identified EMPTY
    /// 0600 target created at the caller-provided path is PRESERVED on
    /// disk, never deleted, never reported as "no effect"). On a
    /// stderr-evidence refusal the dropped stdout reservation is conserved
    /// as an identified EMPTY 0600 artifact (closed descriptor, no worker)
    /// and named in the variant, and a stderr `ReaderPrepare` refusal
    /// additionally keeps the empty stderr target — a stderr failure can
    /// hold BOTH artifacts. Nothing is ever deleted or killed here.
    ///
    /// Limits: normal explicit usage only (see the module and struct
    /// docs); no Drop, no timer, no cancellation, no identity proof. A
    /// thread-spawn failure after the successful reservation/start
    /// sequence panics (documented `LiveRecordedStream` limit — the
    /// fixture-scope resource exhaustion is not masked as a refusal); it
    /// is the one remaining state where a started stream can be lost, and
    /// it stays explicit rather than hidden.
    pub fn start(
        mut child: std::process::Child,
        stdout_evidence: &Path,
        stderr_evidence: &Path,
    ) -> Result<Self, OwnedChildCaptureStartError> {
        // (1) Preconditions verified WITHOUT taking: a missing pipe leaves
        // the child untouched — every initially-owned pipe stays in place.
        let stdout_pipe_owned = child.stdout.is_some();
        let stderr_pipe_owned = child.stderr.is_some();
        if !stdout_pipe_owned {
            return Err(OwnedChildCaptureStartError::StdoutNotPiped { child });
        }
        if !stderr_pipe_owned {
            return Err(OwnedChildCaptureStartError::StderrNotPiped { child });
        }
        // (2) BOTH evidence reservations BEFORE any irreversible pipe
        // transfer: the stdout reservation is taken first, so a stderr
        // refusal can name the stdout artifact it drops (identified empty
        // 0600 file, closed descriptor, no worker — conserved).
        let stdout_reservation =
            match LiveRecordedStream::reserve_evidence(stdout_evidence) {
                Ok(reservation) => reservation,
                Err(error) => {
                    return Err(OwnedChildCaptureStartError::StdoutEvidenceRefused {
                        child,
                        error,
                    });
                }
            };
        let stdout_artifact_path = stdout_reservation.path().to_path_buf();
        let stderr_reservation =
            match LiveRecordedStream::reserve_evidence(stderr_evidence) {
                Ok(reservation) => reservation,
                Err(error) => {
                    return Err(OwnedChildCaptureStartError::StderrEvidenceRefused {
                        child,
                        stdout_artifact_path,
                        stderr_path: stderr_evidence.to_path_buf(),
                        error,
                    });
                }
            };
        // (3) Irreversible transfer: both pipes are taken only now, and
        // each stream starts on its REAL reserved descriptor (no
        // check-exists-then-reopen, no second conservation loop).
        let stdout_pipe = child
            .stdout
            .take()
            .expect("stdout pipe verified piped above");
        let stderr_pipe = child
            .stderr
            .take()
            .expect("stderr pipe verified piped above");
        let stdout_stream = stdout_reservation.start(stdout_pipe);
        let stderr_stream = stderr_reservation.start(stderr_pipe);
        Ok(Self {
            child,
            stdout_stream,
            stderr_stream,
            stdout_path: stdout_artifact_path,
            stderr_path: stderr_evidence.to_path_buf(),
        })
    }

    /// Diagnostic child identity (the owned child's pid at call time).
    /// NOT an identity proof: a pid is recycled after exit; the
    /// `/proc/<pid>/exe` claim is a future task and is never made here.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// Explicit stdin access for the future protocol: the caller spawned
    /// the child with stdin piped when it wants the handshake; closing is
    /// the EOF permission. `None` when the caller piped no stdin.
    pub fn stdin(&mut self) -> Option<&mut std::process::ChildStdin> {
        self.child.stdin.as_mut()
    }

    /// TRANSFER (not borrow) of the child's piped stdin end to the caller:
    /// the real owned `ChildStdin` is MOVED out (the capture's own field is
    /// left `None`), so the caller can write its protocol frame and DROP
    /// the end explicitly — the drop closes the descriptor and delivers the
    /// real EOF the child's guarded frame reader waits on (it reads to EOF,
    /// so a borrowed end closed by the capture could not establish it:
    /// `finish` closes stdin only at the final stop, far too late for a
    /// protocol that continues after the frame). `None` when no stdin was
    /// piped or the end was already transferred (a second call can never
    /// re-create it). Afterwards `stdin` also returns `None`; `finish`
    /// keeps its no-op EOF step and nothing else changes. Effects: no
    /// byte is written here, the descriptor is never closed by this
    /// method — closing is the caller's explicit drop (or an unwind of a
    /// caller frame holding the end, which delivers EOF the same way).
    /// Limits: one transfer only; after it the caller owns the end
    /// exclusively and the capture can no longer write the frame itself.
    pub fn take_stdin(&mut self) -> Option<std::process::ChildStdin> {
        self.child.stdin.take()
    }

    /// Live stdout capture: causal progress (`wait_written`) and the
    /// incremental `evidence_reader` (bytes readable BEFORE producer EOF)
    /// for the future stdin/prepared/bound protocol.
    pub fn stdout(&mut self) -> &mut LiveRecordedStream {
        &mut self.stdout_stream
    }

    /// Live stderr capture (same contract as `stdout`).
    pub fn stderr(&mut self) -> &mut LiveRecordedStream {
        &mut self.stderr_stream
    }

    /// Evidence path of the stdout capture (returned again by `finish`).
    pub fn stdout_path(&self) -> &Path {
        &self.stdout_path
    }

    /// Evidence path of the stderr capture (returned again by `finish`).
    pub fn stderr_path(&self) -> &Path {
        &self.stderr_path
    }

    /// Explicit one-shot closure (NORMAL PATH): (1) closes our stdin end —
    /// the child's EOF permission (no-op when no stdin was piped);
    /// (2) waits the child's REAL termination through the owned handle —
    /// never a kill, never a timer, no pid probing; (3) joins BOTH captures
    /// (after the exit their pipes reach EOF causally; a grandchild holding
    /// a pipe end can delay this — documented struct limit) and returns the
    /// real `ExitStatus` plus both verbatim capture outcomes and both
    /// evidence paths. A wait failure returns `WaitRefused` with the child
    /// and both running captures handed back — never a fake success, and
    /// NEVER a second wait attempt here: the original cause, the child,
    /// both captures and both evidence paths reach the caller unresolved
    /// ([`WaitRefusalHandback`]; resolving or re-waiting is the CALLER's
    /// explicit decision, see the seam contract below). The exit status is
    /// judged by the CALLER: nonzero is a kept result here, never
    /// normalized to success, and `Failed`/`Panicked` captures are never
    /// converted into completions.
    pub fn finish(self) -> ChildCaptureOutcome {
        self.finish_with_wait(std::process::Child::wait)
    }

    /// PRIVATE wait seam shared by the public `finish` (which passes the
    /// REAL `std::process::Child::wait`) and the test-crate refusal guard
    /// (which injects a SYNTHETIC distinctive wait error BEFORE any real
    /// wait). ONE wait attempt per finish: the `FnOnce` handler is called
    /// exactly once on the owned child, after the stdin-EOF permission;
    /// there is no retry loop and no automatic second wait anywhere. On
    /// refusal the original (non-Clone) cause, the child, BOTH
    /// still-running captures and both evidence paths are handed back
    /// UNRESOLVED in [`ChildCaptureOutcome::WaitRefused`] — the joins are
    /// NOT run on that path (the streams stay running and owned by the
    /// handback), the exit status stays UNKNOWN, and nothing is logged,
    /// dropped or faked. The handler shape is the uncurried real method
    /// shape (`&mut Child -> io::Result<ExitStatus>`), so the public path
    /// exercises the exact same code as production `Child::wait`.
    fn finish_with_wait(
        mut self,
        wait: impl FnOnce(&mut std::process::Child) -> std::io::Result<std::process::ExitStatus>,
    ) -> ChildCaptureOutcome {
        drop(self.child.stdin.take());
        let exit = match wait(&mut self.child) {
            Ok(exit) => exit,
            Err(error) => {
                return ChildCaptureOutcome::WaitRefused(WaitRefusalHandback {
                    error,
                    child: self.child,
                    stdout_stream: self.stdout_stream,
                    stderr_stream: self.stderr_stream,
                    stdout_path: self.stdout_path,
                    stderr_path: self.stderr_path,
                });
            }
        };
        let stdout_capture = self.stdout_stream.join();
        let stderr_capture = self.stderr_stream.join();
        ChildCaptureOutcome::Finished {
            exit,
            stdout_capture,
            stderr_capture,
            stdout_path: self.stdout_path,
            stderr_path: self.stderr_path,
        }
    }

    /// Runs the caller's controller closure while keeping THIS capture
    /// owned by the wrapper, then runs the explicit one-shot finish
    /// exactly once on BOTH exits (COOPERATIVE qualification): on the
    /// normal path after the closure returned, and on the panic path
    /// after `catch_unwind` — the finish is always the real stdin-EOF +
    /// `Child::wait` + BOTH joins, never a signal, timer or poll. The
    /// complete real outcome (Finished or WaitRefused) AND the original
    /// panic payload, if any, are returned verbatim in
    /// [`ControllerCaptureResult`] — nothing ignored, nothing resumed
    /// here, no retry; the caller judges both BEFORE any verdict or
    /// re-raise.
    ///
    /// Qualification limit: only a child whose own protocol establishes a
    /// cooperative stop at stdin EOF (the guards' scripts block on a
    /// single `read(1)`). This wrapper is NEITHER arbitrary-child
    /// cancellation NOR a general Drop/unwind mechanism: a child that
    /// ignores stdin EOF keeps `wait` blocked with no timer (documented
    /// `finish` limit), an internal panic inside `finish` itself or
    /// resource exhaustion is NOT covered, and a panicking controller
    /// holding a second reference to exposed handles cannot occur here
    /// because the wrapper holds the sole mutable borrow during the call
    /// (no second capture loop exists).
    pub fn run_controller_then_finish(
        self,
        controller: impl FnOnce(&mut OwnedChildCapture),
    ) -> ControllerCaptureResult {
        let slot: std::cell::RefCell<Option<OwnedChildCapture>> =
            std::cell::RefCell::new(Some(self));
        let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut guard = slot.borrow_mut();
            let capture = guard
                .as_mut()
                .expect("the wrapper owns the capture until its finish");
            controller(capture);
        }));
        let capture = slot
            .borrow_mut()
            .take()
            .expect("the capture stays in the slot: the controller borrow ends inside the closure");
        let outcome = capture.finish();
        ControllerCaptureResult {
            outcome,
            controller_panic: body.err(),
        }
    }

    /// EXPLICIT signal-stop decision wrapper, DISTINCT from the unchanged
    /// COOPERATIVE [`OwnedChildCapture::run_controller_then_finish`] above:
    /// for a child whose protocol IGNORES stdin EOF (e.g. a long-lived
    /// server), the CALLER provides the stop decision itself as the
    /// `signal` argument — this wrapper adds NO implicit timeout, policy,
    /// retry, poll, cap, bare-PID kill or fallback signal.
    ///
    /// Inputs: `self` — the freshly created, not-yet-reaped owned capture
    /// of which the wrapper is the SOLE waiter during the call (the caller
    /// must not wait or signal the child concurrently); `signal` — the
    /// exact caller-chosen signal (the decision is the caller's, the
    /// wrapper never chooses); `controller` — the same shape as the
    /// cooperative wrapper (records observations, returns `()`, may
    /// panic).
    ///
    /// Order and ownership: (1) a pidfd is opened DIRECTLY from THIS
    /// owned child — `pidfd_open` on the child's own pid — BEFORE the
    /// controller call, so the descriptor designates the process this
    /// capture owns even if its pid were recycled afterwards; no
    /// arbitrary-PID API is ever used. (2) The controller runs under
    /// `catch_unwind` with the capture owned by the wrapper's slot (same
    /// slot pattern as the cooperative wrapper). (3) On BOTH exits —
    /// normal return AND controller panic — the caller's signal is
    /// requested EXACTLY ONCE through that held descriptor
    /// (`pidfd_send_signal`). (4) ONLY after a SUCCESSFUL signal request
    /// does the wrapper run the existing explicit finish — stdin EOF, ONE
    /// real `Child::wait`, BOTH joins — via [`OwnedChildCapture::finish`],
    /// and returns the complete real outcome ([`ChildCaptureOutcome`])
    /// plus the ORIGINAL controller panic payload, if any.
    ///
    /// Returns [`SignalStopResult`]: `Stopped` (the exact signal result is
    /// encoded in the variant itself — `pidfd_send_signal` returned
    /// `Ok(())` — and the finish ran to its complete real result),
    /// `PreparationRefused` (the pidfd_open was refused BEFORE the
    /// controller ran) or `SignalRefused` (the signal request through the
    /// held pidfd was refused after the controller). EVERY refusal keeps
    /// the FULL ownership typed — the intact capture (unreaped child with
    /// its stdin NOT closed, BOTH still-RUNNING captures, both evidence
    /// paths), the pidfd when it was acquired, the original non-Clone
    /// cause and any retained controller panic — and NEVER waits blindly:
    /// the child's stop is NOT established on a refusal, so the wrapper
    /// does not close stdin, does not wait, does not join, and never fakes
    /// a success or silently drops the child; resolution stays the
    /// CALLER's explicit decision.
    ///
    /// Errors: the child PID representation is validated EXPLICITLY
    /// BEFORE the pidfd open (a `u32` that does not fit the `i32`
    /// `RawPid` range, or a ZERO pid refused by `Pid::from_raw`, returns
    /// `PreparationRefused` with an `InvalidInput` cause and the intact
    /// capture, controller NOT run); `pidfd_open`/`pidfd_send_signal`
    /// refusals return the real OS cause converted through the standard
    /// `From<rustix Errno> for std::io::Error` contract (raw OS code
    /// preserved) in the matching variant; a `WaitRefused` finish outcome
    /// keeps the existing [`WaitRefusalHandback`] contract unchanged.
    ///
    /// Limits: the wrapper proves the signal REQUEST succeeded, not the
    /// child's reaction — after a successful request the finish's real
    /// wait stays causal with NO timer, so a child that ignores or masks
    /// the requested signal keeps `wait` blocked (documented `finish`
    /// limit; the dedicated guard below uses a child whose SIGINT handler
    /// establishes the stop). The pidfd proves process identity for THIS
    /// descriptor only — it proves NEITHER UID independence, NOR OS-level
    /// confinement, NOR business acceptance; refusal of preparation or
    /// signal paths other than the exercised guard's are EXPLICIT limits,
    /// not covered behavior. Linux pidfd semantics apply (the test crate
    /// compiles Linux-only); an internal panic inside `finish` itself or
    /// resource exhaustion is not covered.
    pub fn signal_stop(
        self,
        signal: rustix::process::Signal,
        controller: impl FnOnce(&mut OwnedChildCapture),
    ) -> SignalStopResult {
        // (1) pidfd held BEFORE the controller call, DIRECTLY from this
        // owned child — never an arbitrary-PID API. The PID representation
        // is validated EXPLICITLY first: the `u32` child id is converted
        // with `try_from` (never a truncating cast, never `unsafe`) and
        // `Pid::from_raw` accepts only a NON-ZERO `RawPid`
        // (`Option<Pid>`); each refused representation returns
        // `PreparationRefused` with a useful technical cause and the
        // capture INTACT — the child is never lost to a conversion. A
        // refusal here means the controller has NOT run and the capture
        // is untouched.
        let child_pid = match i32::try_from(self.child.id()) {
            Ok(raw) => match rustix::process::Pid::from_raw(raw) {
                Some(pid) => pid,
                None => {
                    return SignalStopResult::PreparationRefused(SignalStopPreparationRefusal {
                        error: std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("child pid {raw} is zero and cannot be a non-zero rustix Pid"),
                        ),
                        capture: self,
                    });
                }
            },
            Err(_) => {
                return SignalStopResult::PreparationRefused(SignalStopPreparationRefusal {
                    error: std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!(
                            "child pid {} exceeds the i32 RawPid range and cannot be a rustix Pid",
                            self.child.id()
                        ),
                    ),
                    capture: self,
                });
            }
        };
        let pidfd = match rustix::process::pidfd_open(
            child_pid,
            rustix::process::PidfdFlags::empty(),
        ) {
            Ok(pidfd) => pidfd,
            Err(error) => {
                return SignalStopResult::PreparationRefused(SignalStopPreparationRefusal {
                    // rustix returns its `Errno`; the handback keeps the
                    // ORIGINAL OS cause through the standard
                    // `From<Errno> for std::io::Error` contract (raw OS
                    // code preserved — no invented message, no empty
                    // success).
                    error: error.into(),
                    capture: self,
                });
            }
        };
        // (2) Controller under catch_unwind, capture owned by the slot.
        let slot: std::cell::RefCell<Option<OwnedChildCapture>> =
            std::cell::RefCell::new(Some(self));
        let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut guard = slot.borrow_mut();
            let capture = guard
                .as_mut()
                .expect("the wrapper owns the capture until its finish");
            controller(capture);
        }));
        let capture = slot
            .borrow_mut()
            .take()
            .expect("the capture stays in the slot: the controller borrow ends inside the closure");
        // (3) The signal is requested EXACTLY ONCE through the held
        // descriptor on BOTH exits; the descriptor designates the owned
        // child even after an unwind. A refusal keeps the capture intact
        // (stdin NOT closed, both captures still running) plus the pidfd,
        // the real cause and any controller panic — no blind wait.
        if let Err(error) = rustix::process::pidfd_send_signal(&pidfd, signal) {
            return SignalStopResult::SignalRefused(SignalStopSignalRefusal {
                // rustix `Errno` → `std::io::Error` via the standard
                // `From` contract: the original OS cause and raw code
                // travel to the caller unchanged.
                error: error.into(),
                capture,
                pidfd,
                controller_panic: body.err(),
            });
        }
        // (4) ONLY after a SUCCESSFUL signal request: the existing
        // explicit finish (stdin EOF, ONE real wait, BOTH joins).
        SignalStopResult::Stopped(ControllerCaptureResult {
            outcome: capture.finish(),
            controller_panic: body.err(),
        })
    }
}

/// Structured result of [`OwnedChildCapture::run_controller_then_finish`]:
/// BOTH real outcomes are kept verbatim and judged by the caller — never
/// a masked cleanup issue, never a swallowed panic. `outcome` is the
/// complete real finish result (Finished or WaitRefused, ownership
/// already returned by `finish` on its refusal path);
/// `controller_panic` is the ORIGINAL panic payload exactly when the
/// controller panicked, `None` on the normal path.
pub struct ControllerCaptureResult {
    pub outcome: ChildCaptureOutcome,
    pub controller_panic: Option<Box<dyn std::any::Any + Send>>,
}

impl std::fmt::Debug for ControllerCaptureResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControllerCaptureResult")
            .field("outcome", &self.outcome)
            .field(
                "controller_panic",
                &self.controller_panic.as_ref().map(|_| "<payload>"),
            )
            .finish()
    }
}

/// Structured result of [`OwnedChildCapture::signal_stop`]: the
/// caller-chosen signal was REQUESTED successfully through the pidfd held
/// before the controller call (`Stopped`) or the request failed
/// (`PreparationRefused` / `SignalRefused`). EVERY variant keeps the real
/// facts — no masked refusal, no swallowed panic, no fake success.
pub enum SignalStopResult {
    /// The signal request succeeded through the held descriptor on the
    /// normal OR the panic path; the finish ran to its COMPLETE real
    /// result and the ORIGINAL controller panic payload, if any, travels
    /// with it. The caller judges BOTH (including any `WaitRefused`
    /// handback inside `outcome`) before any verdict.
    Stopped(ControllerCaptureResult),
    /// `pidfd_open` was refused BEFORE the controller ran: the capture is
    /// returned INTACT (unreaped child, stdin NOT closed, BOTH still-
    /// running captures, both evidence paths) plus the real non-Clone
    /// cause — the wrapper waited NOTHING and ran NO controller, so the
    /// child's stop is NOT established and resolution stays the caller's
    /// explicit decision.
    PreparationRefused(SignalStopPreparationRefusal),
    /// The signal request through the ALREADY-HELD pidfd was refused
    /// AFTER the controller ran (on the normal or panic path): the
    /// capture is returned INTACT (child stdin NOT closed, BOTH
    /// still-RUNNING captures, both paths) together with the held pidfd,
    /// the real non-Clone cause and any retained controller panic — no
    /// blind wait (the child's stop is NOT established), no fake success,
    /// no silent drop; resolution stays the caller's explicit decision.
    SignalRefused(SignalStopSignalRefusal),
}

impl std::fmt::Debug for SignalStopResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped(result) => f
                .debug_tuple("Stopped")
                .field(result)
                .finish(),
            Self::PreparationRefused(refusal) => f
                .debug_tuple("PreparationRefused")
                .field(refusal)
                .finish(),
            Self::SignalRefused(refusal) => f
                .debug_tuple("SignalRefused")
                .field(refusal)
                .finish(),
        }
    }
}

/// FULL-ownership handback of a refused pidfd preparation in
/// [`OwnedChildCapture::signal_stop`]: the INTACT capture (the child is
/// unreaped, its stdin NOT closed, BOTH captures still RUNNING, both
/// evidence paths owned), the real non-Clone `io::Error` cause, and NO
/// controller observation (the controller never ran). Nothing is waited,
/// joined or closed by this refusal.
pub struct SignalStopPreparationRefusal {
    pub error: std::io::Error,
    pub capture: OwnedChildCapture,
}

impl std::fmt::Debug for SignalStopPreparationRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalStopPreparationRefusal")
            .field("error", &self.error)
            .field("capture", &self.capture)
            .finish()
    }
}

/// FULL-ownership handback of a refused signal request in
/// [`OwnedChildCapture::signal_stop`]: the INTACT capture (child stdin
/// NOT closed, BOTH captures still RUNNING, both paths), the ALREADY-HELD
/// pidfd (kept for the caller's explicit decision — no automatic retry
/// exists), the real non-Clone `io::Error` cause and the ORIGINAL
/// controller panic payload when the controller panicked. Nothing is
/// waited, joined or closed by this refusal.
pub struct SignalStopSignalRefusal {
    pub error: std::io::Error,
    pub capture: OwnedChildCapture,
    pub pidfd: rustix::fd::OwnedFd,
    pub controller_panic: Option<Box<dyn std::any::Any + Send>>,
}

impl std::fmt::Debug for SignalStopSignalRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalStopSignalRefusal")
            .field("error", &self.error)
            .field("capture", &self.capture)
            .field("pidfd", &self.pidfd)
            .field(
                "controller_panic",
                &self.controller_panic.as_ref().map(|_| "<payload>"),
            )
            .finish()
    }
}

// ---- Guard: one normal synthetic child witness ------------------------------

/// Raw stdout bytes of the guard's constant script: CRLF, NUL and a
/// non-UTF-8 byte, preserved verbatim by the capture (no decode/filter).
const GUARD_STDOUT: &[u8] = b"STDOUT-RAW\r\nwith\x00nul\xFFbyte\n";

/// Raw stderr bytes of the guard's constant script: NUL, an invalid-UTF-8
/// pair (`\xc3\x28`) and CRLF, preserved verbatim.
const GUARD_STDERR: &[u8] = b"STDERR-RAW\x00\xc3\x28\r\nsecond\n";

/// Constant local script for the guard's real child: writes the exact raw
/// bytes on both streams (flushed), then terminates NATURALLY with exit
/// code 7 — a nonzero termination is a kept result, never a success.
const GUARD_SCRIPT: &str = "import sys\n\
    sys.stdout.buffer.write(b'STDOUT-RAW\\r\\nwith\\x00nul\\xFFbyte\\n')\n\
    sys.stdout.buffer.flush()\n\
    sys.stderr.buffer.write(b'STDERR-RAW\\x00\\xc3\\x28\\r\\nsecond\\n')\n\
    sys.stderr.buffer.flush()\n\
    raise SystemExit(7)\n";

/// Starts the capture or refuses with the real cause while resolving the
/// returned ownership observably BEFORE refusing: the child (present in
/// every start-error variant) is waited explicitly through its own handle
/// (never killed), and only then does this panic with the refusal — a
/// setup problem is never masked into an empty success. Since the
/// ownership repair NO capture is ever started before a refusal (the
/// stderr-evidence refusal drops its stdout reservation as an identified
/// empty artifact inside `start`, see `StderrEvidenceRefused`), so no
/// stream join belongs here anymore. The guard's leaf stays in place on
/// that path (documented shared limit, no Drop guard).
fn start_or_refuse(
    child: std::process::Child,
    stdout_evidence: &Path,
    stderr_evidence: &Path,
) -> OwnedChildCapture {
    match OwnedChildCapture::start(child, stdout_evidence, stderr_evidence) {
        Ok(capture) => capture,
        Err(refusal) => {
            // The refusal diagnostic is rendered BEFORE the ownership is
            // destructured (the debug text is frozen, then the child is
            // resolved observably).
            let detail = format!("{refusal:?}");
            let mut child = match refusal {
                OwnedChildCaptureStartError::StdoutNotPiped { child }
                | OwnedChildCaptureStartError::StderrNotPiped { child }
                | OwnedChildCaptureStartError::StdoutEvidenceRefused { child, .. }
                | OwnedChildCaptureStartError::StderrEvidenceRefused { child, .. } => child,
            };
            let exit = child.wait();
            panic!(
                "child capture start refused; ownership resolved before refusing \
                 (child wait {exit:?}): {detail}"
            );
        }
    }
}

/// NORMAL witness of the child owner: a real local `/usr/bin/python3 -c`
/// child (constant script, no provider, no live DB, no UID/enrollment)
/// writes exact raw bytes — NUL, non-UTF-8 and CRLF — on BOTH streams and
/// terminates naturally with exit code 7. The guard collects the exposed
/// live progress snapshots on both captures WITHOUT asserting, then
/// collects the REAL finish outcome (true child wait + BOTH joined
/// captures) BEFORE ANY terminal assertion:
/// the nonzero exit stays a kept result (asserted as code 7, and asserted
/// NOT a success), both captures must be clean `Completed` with the exact
/// byte counts, both evidence files must hold the exact raw bytes (no
/// decode/filter/normalization) with private 0600 modes, and the exposed
/// stdin handle must stay available for the future protocol. NOT in this
/// guard: the stdin handshake before EOF and the cooperative
/// controller-panic retention have their own dedicated guards below (via
/// the wrapper `run_controller_then_finish`), while the setup-refusal
/// campaign beyond the refusal pins here remains a next sub-task. The leaf
/// is removed only
/// on success and kept on failure; synthetic children prove no business
/// acceptance.
#[test]
fn child_capture_owner_collects_exact_raw_streams_and_real_exit() {
    let leaf = private_fixture_leaf("child-normal");
    let stdout_evidence = leaf.join("child-stdout.bin");
    let stderr_evidence = leaf.join("child-stderr.bin");
    let child = std::process::Command::new("/usr/bin/python3")
        .arg("-c")
        .arg(GUARD_SCRIPT)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("synthetic local python3 child must spawn");
    let mut capture = start_or_refuse(child, &stdout_evidence, &stderr_evidence);
    // Real collection WITHOUT asserting yet: snapshots of the exposed stdin
    // and of the causal progress on BOTH captures (no poll, no timer), then
    // the REAL finish (stdin EOF, true wait, BOTH joins) — every assertion
    // runs only after this full real collection, so no failed assertion can
    // ever abort before the child's true issue is conserved.
    let stdin_exposed = capture.stdin().is_some();
    let stdout_progress = capture.stdout().wait_written(GUARD_STDOUT.len() as u64);
    let stderr_progress = capture.stderr().wait_written(GUARD_STDERR.len() as u64);
    let outcome = capture.finish();
    // Terminal assertions — none ran before the real collection above.
    assert!(
        stdin_exposed,
        "piped stdin must stay exposed for the future protocol"
    );
    assert!(
        stdout_progress.written >= GUARD_STDOUT.len() as u64,
        "stdout must be conserved before the join: {stdout_progress:?}"
    );
    assert!(
        stderr_progress.written >= GUARD_STDERR.len() as u64,
        "stderr must be conserved before the join: {stderr_progress:?}"
    );
    // The by-value match below consumes `outcome`; its Debug is captured
    // first so the capture-issue verdict below can still name the outcome.
    let outcome_debug = format!("{outcome:?}");
    let (exit, stdout_capture, stderr_capture) = match outcome {
        ChildCaptureOutcome::Finished {
            exit,
            stdout_capture,
            stderr_capture,
            ..
        } => (exit, stdout_capture, stderr_capture),
        ChildCaptureOutcome::WaitRefused(handback) => {
            // The refusal is conserved EXPLICITLY before the verdict —
            // never masked, never silently dropped: the COMPLETE typed
            // carrier (cause, SAME child, both joined capture states,
            // both paths) is transferred to the caller-level decision
            // point and failed with as a typed recoverable payload; the
            // child's exit status stays UNKNOWN (no automatic re-wait
            // exists in this module).
            fail_unexpected_wait_refusal(conserve_unexpected_wait_refusal_without_rewait(
                "child_capture_owner_collects_exact_raw_streams_and_real_exit",
                handback,
                None,
            ));
        }
    };
    // Natural nonzero termination is a KEPT result: asserted as the real
    // code 7 and explicitly NOT a success.
    assert_eq!(
        exit.code(),
        Some(7),
        "child must terminate naturally with its real code 7: {exit:?}"
    );
    assert!(
        !exit.success(),
        "a nonzero natural exit must never be reported as a success: {exit:?}"
    );
    // BOTH captures must complete cleanly with the exact byte counts.
    match (stdout_capture, stderr_capture) {
        (
            LiveStreamState::Completed { bytes: stdout_bytes },
            LiveStreamState::Completed { bytes: stderr_bytes },
        ) => {
            assert_eq!(
                stdout_bytes,
                GUARD_STDOUT.len() as u64,
                "stdout capture byte count"
            );
            assert_eq!(
                stderr_bytes,
                GUARD_STDERR.len() as u64,
                "stderr capture byte count"
            );
        }
        other => panic!(
            "both captures must complete cleanly, got {other:?} (outcome {outcome_debug})"
        ),
    }
    // Exact raw bytes on disk — NUL, non-UTF-8 and CRLF preserved verbatim.
    assert_eq!(
        fs::read(&stdout_evidence).expect("stdout evidence read"),
        GUARD_STDOUT,
        "raw stdout must be exact (no decode, filter or normalization)"
    );
    assert_eq!(
        fs::read(&stderr_evidence).expect("stderr evidence read"),
        GUARD_STDERR,
        "raw stderr must be exact (no decode, filter or normalization)"
    );
    // Private evidence modes: both files stay 0600.
    for path in [&stdout_evidence, &stderr_evidence] {
        let mode = fs::metadata(path)
            .expect("evidence metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "evidence {path:?} must stay private");
    }
    fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

/// Real collection of a refused start's returned child, shared by BOTH
/// refusal guards below: closes our stdin end first (the child's EOF
/// permission — stdin is never a reason to wait), drains every pipe the
/// child STILL owns concurrently to causal EOF through dedicated workers
/// (no timer, no poll, no retry, no volume ceiling), joins the workers
/// inside the scope, and only then waits the child's REAL termination
/// through its own handle — never a kill, never an invented status; a
/// nonzero or signalled exit is a kept result.
///
/// Returns `(exit, stdout_drained, stderr_drained)`: a drain is `None`
/// exactly when the corresponding pipe was not owned at refusal time.
/// Bytes on a pipe the defect already lost inside `start` are NOT awaited
/// — they can never arrive again.
///
/// Errors: a failed drain read or a refused wait panics with the real
/// cause (never masked, never turned into a synthetic status).
///
/// Limits: proves no business acceptance; the drain consumes the pipes,
/// so any ownership pin on the returned child must be snapshotted by the
/// caller BEFORE calling this.
fn collect_refused_child_observably(
    mut child: std::process::Child,
) -> (
    std::process::ExitStatus,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
) {
    drop(child.stdin.take());
    let (stdout_drained, stderr_drained) = std::thread::scope(|scope| {
        let stdout_worker = child.stdout.as_mut().map(|pipe| {
            scope.spawn(move || {
                let mut drained: Vec<u8> = Vec::new();
                pipe.read_to_end(&mut drained).expect(
                    "draining the still-owned stdout pipe must read to its causal EOF",
                );
                drained
            })
        });
        let stderr_worker = child.stderr.as_mut().map(|pipe| {
            scope.spawn(move || {
                let mut drained: Vec<u8> = Vec::new();
                pipe.read_to_end(&mut drained).expect(
                    "draining the still-owned stderr pipe must read to its causal EOF",
                );
                drained
            })
        });
        let stdout_drained = stdout_worker
            .map(|worker| worker.join().expect("stdout drain worker must not panic"));
        let stderr_drained = stderr_worker
            .map(|worker| worker.join().expect("stderr drain worker must not panic"));
        (stdout_drained, stderr_drained)
    });
    let exit = child
        .wait()
        .expect("real wait of the refused child through its own handle");
    (exit, stdout_drained, stderr_drained)
}

/// Healthy-witness closer shared by BOTH refusal guards below: finishes
/// the witness through the owner and requires the full sound baseline
/// BEFORE any fault injection — real natural exit code 7 (never reported
/// as a success) and BOTH captures clean `Completed` with the exact raw
/// byte counts of the constant script. A `WaitRefused` outcome is NEVER
/// resolved here: the COMPLETE typed carrier is conserved (nothing
/// dropped, the child never re-waited — its exit status stays UNKNOWN)
/// and failed with at the caller level as a typed recoverable payload; a
/// non-`Completed` capture panics with its real issue (never masked,
/// never normalized). Contract: `witness` is the not-yet-finished owner
/// of a fully piped synthetic child on the guard's shared leaf; the
/// witness's evidence files stay in that leaf (removed only on the
/// guard's final success). Proves no business acceptance.
fn finish_healthy_witness_or_panic(witness: OwnedChildCapture) {
    match witness.finish() {
        ChildCaptureOutcome::Finished {
            exit,
            stdout_capture,
            stderr_capture,
            ..
        } => {
            assert_eq!(
                exit.code(),
                Some(7),
                "healthy witness must really finish with its real code 7: {exit:?}"
            );
            assert!(
                !exit.success(),
                "a nonzero natural exit must never be reported as a success: {exit:?}"
            );
            match (stdout_capture, stderr_capture) {
                (
                    LiveStreamState::Completed { bytes: stdout_bytes },
                    LiveStreamState::Completed { bytes: stderr_bytes },
                ) => {
                    assert_eq!(
                        stdout_bytes,
                        GUARD_STDOUT.len() as u64,
                        "healthy witness stdout capture must complete with the exact byte count"
                    );
                    assert_eq!(
                        stderr_bytes,
                        GUARD_STDERR.len() as u64,
                        "healthy witness stderr capture must complete with the exact byte count"
                    );
                }
                other => panic!(
                    "healthy witness must complete BOTH captures cleanly before any \
                     fault injection, got {other:?}"
                ),
            }
        }
        ChildCaptureOutcome::WaitRefused(handback) => {
            fail_unexpected_wait_refusal(conserve_unexpected_wait_refusal_without_rewait(
                "healthy witness wait refused; no injection may run",
                handback,
                None,
            ));
        }
    }
}

/// Regression pin of the repaired `StderrNotPiped` refusal contract
/// (general ownership-repair lot). The archived tests-only-lot run proved
/// the defect this pin targets: `start` took the stdout pipe BEFORE
/// verifying stderr and dropped it at the refusal, so the returned child
/// had lost its stdout payload (`stdout_pipe_owned` observed None) —
/// sources and FAIL originals are archived, never rewritten. The repaired
/// contract must keep the stdout pipe owned and the payload retrievable,
/// create no evidence file at the refusal, and let the child reach its
/// natural code 7. Discipline unchanged: a full healthy witness (same
/// leaf, same payload, both pipes) really finishes through the owner
/// FIRST — BOTH captures confirmed clean `Completed` with the exact byte
/// counts — only then is the refused variant injected, and the child's
/// real issue (drain of still-owned pipes + real wait) is fully collected
/// BEFORE any assertion, with the real exit and pipe presence exposed in
/// a synthetic diagnostic ahead of the pins. The child writes its full
/// stdout payload and exits 7 naturally. The leaf is removed only on
/// success and kept on failure; synthetic children prove no business
/// acceptance.
#[test]
fn child_capture_start_refusal_stderr_not_piped_keeps_stdout_pipe_owned() {
    let leaf = private_fixture_leaf("child-refuse-stderr");
    let stdout_evidence = leaf.join("must-not-exist-stdout.bin");
    let stderr_evidence = leaf.join("must-not-exist-stderr.bin");
    // Healthy witness FIRST: same leaf, same payload, really finished via
    // the owner — the injection below never runs unless the normal path
    // already proved itself on these exact preconditions.
    let witness_child = std::process::Command::new("/usr/bin/python3")
        .arg("-c")
        .arg(GUARD_SCRIPT)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("witness python3 child must spawn");
    let witness = start_or_refuse(
        witness_child,
        &leaf.join("witness-stdout.bin"),
        &leaf.join("witness-stderr.bin"),
    );
    finish_healthy_witness_or_panic(witness);
    // Injection: stdout piped, stderr NOT piped — the exact StderrNotPiped
    // precondition.
    let refused_child = std::process::Command::new("/usr/bin/python3")
        .arg("-c")
        .arg(GUARD_SCRIPT)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("refused python3 child must spawn");
    let child = match OwnedChildCapture::start(refused_child, &stdout_evidence, &stderr_evidence) {
        Ok(_) => panic!("a child without piped stderr must be refused, never silently captured"),
        Err(OwnedChildCaptureStartError::StderrNotPiped { child }) => child,
        Err(other) => panic!("expected StderrNotPiped, got {other:?}"),
    };
    // Ownership snapshotted BEFORE the drain consumes the pipes; no
    // assertion yet.
    let stdout_pipe_owned = child.stdout.is_some();
    let (exit, stdout_drained, stderr_drained) = collect_refused_child_observably(child);
    // Stable contract pins of the repaired refusal: a refused start
    // creates no evidence file, and this injection never piped stderr.
    assert!(
        !stdout_evidence.exists() && !stderr_evidence.exists(),
        "a refused start must not create any evidence file"
    );
    assert!(
        stderr_drained.is_none(),
        "this injection never piped stderr; no drain may appear"
    );
    // Regression pin of the repaired contract (the archived defect lost
    // this pipe and returned None): the stdout pipe must stay owned by
    // the returned child — the real exit and the pipe presence were both
    // collected above, BEFORE this pin.
    assert!(
        stdout_pipe_owned,
        "StderrNotPiped must keep the stdout pipe in the returned child \
         (regression pin: archived defect took stdout before verifying \
         stderr and dropped it at the refusal; real exit {exit:?}, \
         stdout_pipe_owned {stdout_pipe_owned})"
    );
    // Ownership chain: the owned pipe must let the child's real stdout
    // bytes be retrieved, and the child must then reach its natural
    // code 7.
    let drained = stdout_drained.expect("an owned stdout pipe must drain the child's real bytes");
    assert_eq!(
        drained, GUARD_STDOUT,
        "the preserved stdout pipe must deliver the exact raw payload"
    );
    assert_eq!(
        exit.code(),
        Some(7),
        "with the stdout pipe owned and drained, the child must reach its \
         natural code 7: {exit:?}"
    );
    fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

/// Regression pin of the repaired `StdoutEvidenceRefused` refusal contract
/// (general ownership-repair lot). The archived tests-only-lot run proved
/// the defects this pin targets: `start` moved the stdout pipe into the
/// stream start (dropped when the exclusive creation refused) and dropped
/// the already-taken stderr pipe at return, so the returned child had
/// lost BOTH pipes — sources and FAIL originals are archived, never
/// rewritten. The repaired contract must keep BOTH pipes in the returned
/// child, carry the real cause (`AlreadyExists`), keep the pre-existing
/// evidence file byte-identical, create no second file, and let the child
/// reach its natural code 7. Discipline unchanged: a full healthy witness
/// really finishes through the owner FIRST on the same leaf — BOTH
/// captures confirmed clean `Completed` with the exact byte counts — its
/// stdout evidence file is the KNOWN EXISTING file the refused start then
/// collides with; the child's real issue (drain + real wait) is fully
/// collected BEFORE the pins, with the real exit and both pipe snapshots
/// exposed in a synthetic diagnostic ahead of them. The real cause is
/// pinned through the refusal's frozen Debug (`AlreadyExists`) plus the
/// untouched existing file. Leaf removed only on success; synthetic
/// children prove no business acceptance.
#[test]
fn child_capture_start_refusal_stdout_evidence_refused_keeps_both_pipes_owned() {
    let leaf = private_fixture_leaf("child-refuse-stdout-evidence");
    let existing_stdout = leaf.join("witness-stdout.bin");
    let stderr_evidence = leaf.join("must-not-exist-stderr.bin");
    // Healthy witness FIRST: same leaf, same payload, really finished via
    // the owner — its stdout evidence is the known existing file below.
    let witness_child = std::process::Command::new("/usr/bin/python3")
        .arg("-c")
        .arg(GUARD_SCRIPT)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("witness python3 child must spawn");
    let witness = start_or_refuse(
        witness_child,
        &existing_stdout,
        &leaf.join("witness-stderr.bin"),
    );
    finish_healthy_witness_or_panic(witness);
    let existing_bytes = fs::read(&existing_stdout).expect("witness stdout evidence read");
    assert_eq!(
        existing_bytes, GUARD_STDOUT,
        "the witness must leave the exact known payload in the existing file"
    );
    // Injection: BOTH pipes piped, stdout evidence path ALREADY EXISTS.
    let refused_child = std::process::Command::new("/usr/bin/python3")
        .arg("-c")
        .arg(GUARD_SCRIPT)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("refused python3 child must spawn");
    let (child, error) = match OwnedChildCapture::start(refused_child, &existing_stdout, &stderr_evidence)
    {
        Ok(_) => panic!("starting onto an existing evidence file must be refused"),
        Err(OwnedChildCaptureStartError::StdoutEvidenceRefused { child, error }) => (child, error),
        Err(other) => panic!("expected StdoutEvidenceRefused, got {other:?}"),
    };
    let error_detail = format!("{error:?}");
    // Ownership snapshotted BEFORE the drain consumes the pipes; no
    // assertion yet.
    let stdout_pipe_owned = child.stdout.is_some();
    let stderr_pipe_owned = child.stderr.is_some();
    let (exit, stdout_drained, stderr_drained) = collect_refused_child_observably(child);
    // Stable pins of the repaired refusal: the real exclusive-creation
    // cause, the untouched pre-existing evidence file and no second file
    // created by the refused start.
    assert!(
        error_detail.contains("AlreadyExists"),
        "the refusal must carry the real exclusive-creation cause: {error_detail}"
    );
    assert_eq!(
        fs::read(&existing_stdout).expect("existing stdout evidence read"),
        existing_bytes,
        "the pre-existing evidence file must stay byte-identical through the refusal"
    );
    assert!(
        !stderr_evidence.exists(),
        "a refused start must not create the second evidence file"
    );
    // Regression pins of the repaired contract (the archived defect lost
    // BOTH pipes and returned None for both): each pipe must stay owned
    // by the returned child — the real exit and both pipe snapshots were
    // collected above, BEFORE these pins.
    assert!(
        stdout_pipe_owned,
        "StdoutEvidenceRefused must keep the stdout pipe in the returned \
         child (regression pin: archived defect moved the pipe into the \
         stream start and dropped it with its refusal; real exit {exit:?}, \
         stdout_pipe_owned {stdout_pipe_owned})"
    );
    assert!(
        stderr_pipe_owned,
        "StdoutEvidenceRefused must keep the stderr pipe in the returned \
         child (regression pin: archived defect dropped the already-taken \
         stderr pipe at the refusal return; real exit {exit:?}, \
         stderr_pipe_owned {stderr_pipe_owned})"
    );
    // Ownership chain: both owned pipes must drain the exact raw
    // payloads, and the child must reach its natural code 7.
    let stdout_bytes = stdout_drained.expect("an owned stdout pipe must drain the child's real bytes");
    assert_eq!(
        stdout_bytes, GUARD_STDOUT,
        "the preserved stdout pipe must deliver the exact raw payload"
    );
    let stderr_bytes = stderr_drained.expect("an owned stderr pipe must drain the child's real bytes");
    assert_eq!(
        stderr_bytes, GUARD_STDERR,
        "the preserved stderr pipe must deliver the exact raw payload"
    );
    assert_eq!(
        exit.code(),
        Some(7),
        "with both pipes owned and drained, the child must reach its \
         natural code 7: {exit:?}"
    );
    fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

// ---- Guard: stdin/live-output causal handshake (before producer EOF) ------

/// Raw stdout prefix of the handshake guard's constant script: NUL, a lone
/// non-UTF-8 byte (`\xff`) and CRLF, written and flushed BEFORE the script
/// blocks on its single stdin byte.
const HANDSHAKE_STDOUT_PREFIX: &[u8] = b"HANDSHAKE-OUT-PRE\x00\xff\r\n";

/// Raw stderr prefix of the same script: NUL, an invalid-UTF-8 pair
/// (`\x80\x81`) and CRLF, written and flushed on the OTHER stream before
/// the same wait — the two prefixes are distinct byte sequences.
const HANDSHAKE_STDERR_PREFIX: &[u8] = b"HANDSHAKE-ERR-PRE\x00\x80\x81\r\n";

/// Known stdout tail, emitted only AFTER the script's stdin wait returns.
const HANDSHAKE_STDOUT_TAIL: &[u8] = b"HANDSHAKE-OUT-TAIL\r\nend\n";

/// Known stderr tail, emitted only AFTER the same release.
const HANDSHAKE_STDERR_TAIL: &[u8] = b"HANDSHAKE-ERR-TAIL\r\nend\n";

/// The exactly-ONE release byte the controller writes to the exposed stdin
/// to let the script pass its `read(1)` barrier.
const HANDSHAKE_RELEASE_BYTE: u8 = b'R';

/// Constant local script for the handshake guard's real child: writes and
/// flushes BOTH distinct raw prefixes (NUL, non-UTF-8, CRLF), then blocks
/// on exactly ONE stdin byte, then writes and flushes BOTH known tails and
/// terminates NATURALLY with exit code 9 — a nonzero termination is a kept
/// result, never a success.
const HANDSHAKE_SCRIPT: &str = "import sys\n\
    sys.stdout.buffer.write(b'HANDSHAKE-OUT-PRE\\x00\\xff\\r\\n')\n\
    sys.stdout.buffer.flush()\n\
    sys.stderr.buffer.write(b'HANDSHAKE-ERR-PRE\\x00\\x80\\x81\\r\\n')\n\
    sys.stderr.buffer.flush()\n\
    sys.stdin.buffer.read(1)\n\
    sys.stdout.buffer.write(b'HANDSHAKE-OUT-TAIL\\r\\nend\\n')\n\
    sys.stdout.buffer.flush()\n\
    sys.stderr.buffer.write(b'HANDSHAKE-ERR-TAIL\\r\\nend\\n')\n\
    sys.stderr.buffer.flush()\n\
    raise SystemExit(9)\n";

/// Reads EXACTLY `len` bytes through the causal evidence reader (its `Read`
/// impl waits for the promised bytes — no poll, no timer) and returns them
/// or the real cause. The handshake guard RECORDS this result and asserts
/// it only after the real finish, so a read failure can never abort before
/// the child's true issue is conserved.
fn handshake_read_exactly(
    reader: &mut impl std::io::Read,
    len: usize,
) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

/// Complete typed carrier of an UNEXPECTED wait refusal, conserved at a
/// caller level before any verdict. EVERY field of the original
/// [`WaitRefusalHandback`] is moved in — nothing is dropped, no
/// `DropChild` silent abandonment: the original non-Clone cause (moved
/// verbatim, `io::Error` is not Clone), the SAME child (stdin already
/// closed, NEVER re-waited by the conservation — the exit status stays
/// UNKNOWN, zero automatic wait after a real refusal), BOTH capture
/// terminal states (real joins — their issues stay conserved, never
/// masked), both evidence paths, the ORIGINAL controller panic payload
/// when one was retained, and the failing site's `context` label. A
/// `catch_unwind` site can recover the carrier from the panic payload
/// BY VALUE (`downcast::<WaitRefusalCarrier>()` on the boxed payload — a
/// bare `downcast_ref` borrow would NOT pass ownership) and then decide
/// explicitly (an explicit re-wait remains the CALLER's own decision —
/// the FIRST real wait when no real wait has run yet, as the dedicated
/// guard below pins with its injection seam — never an automatic one).
/// Built ONLY by
/// [`conserve_unexpected_wait_refusal_without_rewait`]; failed with ONLY
/// by [`fail_unexpected_wait_refusal`].
struct WaitRefusalCarrier {
    context: &'static str,
    error: std::io::Error,
    child: std::process::Child,
    stdout_state: LiveStreamState,
    stderr_state: LiveStreamState,
    stdout_path: std::path::PathBuf,
    stderr_path: std::path::PathBuf,
    controller_panic: Option<Box<dyn std::any::Any + Send>>,
}

/// Manual Debug: the child is identified by its pid (diagnostic only —
/// `Child` itself is not `Debug` and a pid may be recycled after exit),
/// the controller panic only by its presence.
impl std::fmt::Debug for WaitRefusalCarrier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaitRefusalCarrier")
            .field("context", &self.context)
            .field("error", &self.error)
            .field("child_pid", &self.child.id())
            .field("stdout_state", &self.stdout_state)
            .field("stderr_state", &self.stderr_state)
            .field("stdout_path", &self.stdout_path)
            .field("stderr_path", &self.stderr_path)
            .field(
                "controller_panic_present",
                &self.controller_panic.is_some(),
            )
            .finish()
    }
}

/// Conservation of a refusal the test did NOT expect, into the COMPLETE
/// typed [`WaitRefusalCarrier`]: EVERY field of the handback is moved in
/// (nothing dropped, no silent `DropChild`), BOTH captures join to their
/// real terminal states (their issues stay conserved in the carrier,
/// never masked), the child is NOT re-waited — the contract forbids an
/// automatic second wait, so the exit status stays UNKNOWN — and the
/// caller's retained controller panic payload (if any) travels with the
/// carrier instead of being lost. The `context` label names the failing
/// site for the verdict diagnostics; it is a label only and never
/// replaces the conserved ownership.
fn conserve_unexpected_wait_refusal_without_rewait(
    context: &'static str,
    handback: WaitRefusalHandback,
    controller_panic: Option<Box<dyn std::any::Any + Send>>,
) -> WaitRefusalCarrier {
    let WaitRefusalHandback {
        error,
        child,
        stdout_stream,
        stderr_stream,
        stdout_path,
        stderr_path,
    } = handback;
    let stdout_state = stdout_stream.join();
    let stderr_state = stderr_stream.join();
    WaitRefusalCarrier {
        context,
        error,
        child,
        stdout_state,
        stderr_state,
        stdout_path,
        stderr_path,
        controller_panic,
    }
}

/// Explicit failure for a refusal the test did NOT expect: the COMPLETE
/// typed carrier is transferred REALLY to this caller-level decision
/// point (nothing dropped on the way) and is panicked WITH the test as a
/// TYPED RECOVERABLE payload — a `catch_unwind` site can recover the
/// payload BY VALUE (`downcast::<WaitRefusalCarrier>()` on the boxed
/// payload — a bare `downcast_ref` borrow would not pass ownership) and
/// thereby recover the cause, the SAME child, both joined capture states,
/// both paths and any original controller panic; the panic is a typed
/// boundary, never an abandonment. The conserved facts are printed first
/// (the payload itself is not a message string). Limits: the child is
/// NEVER re-waited here — the exit status stays UNKNOWN (zero automatic
/// wait after a real refusal); an explicit re-wait remains the caller's
/// own decision — the FIRST real wait when no real wait has run yet (the
/// injection-seam case the dedicated guard pins) — never an automatic
/// one.
fn fail_unexpected_wait_refusal(carrier: WaitRefusalCarrier) -> ! {
    eprintln!(
        "child test unexpected wait refusal — {context}: cause {error}; \
         child pid {pid} NEVER re-waited (exit status UNKNOWN); stdout \
         {stdout:?}, stderr {stderr:?}; stdout path {stdout_path:?}, \
         stderr path {stderr_path:?}; original controller panic retained: \
         {panic_present}",
        context = carrier.context,
        error = carrier.error,
        pid = carrier.child.id(),
        stdout = carrier.stdout_state,
        stderr = carrier.stderr_state,
        stdout_path = carrier.stdout_path,
        stderr_path = carrier.stderr_path,
        panic_present = carrier.controller_panic.is_some(),
    );
    std::panic::panic_any(carrier);
}

/// Spawns the handshake guard's real local `/usr/bin/python3 -c` child
/// (constant script, stdin/stdout/stderr all piped) and starts its capture
/// or panics with the refusal after resolving the returned ownership (see
/// `start_or_refuse`). Shared by BOTH handshake guards below.
fn spawn_and_start_handshake_child(
    stdout_evidence: &Path,
    stderr_evidence: &Path,
) -> OwnedChildCapture {
    let child = std::process::Command::new("/usr/bin/python3")
        .arg("-c")
        .arg(HANDSHAKE_SCRIPT)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("synthetic local python3 handshake child must spawn");
    start_or_refuse(child, stdout_evidence, stderr_evidence)
}

/// The HEALTHY controller both guards use as their causal handshake core:
/// it consumes BOTH exact raw prefixes through the live evidence readers
/// while the script is still blocked on its single stdin byte, then writes
/// the ONE release byte — the causal ordering prefixes → release proves
/// readability before producer EOF. Every observation is RECORDED into the
/// caller's slot (the wrapper contract keeps the controller returning
/// `()`); NOTHING is asserted inside, so the only panic this controller
/// can raise is the wrapper contract's own. The sequence prefixes →
/// release → finish is completed by the wrapper's explicit finish on BOTH
/// exits.
fn run_handshake_prefix_reads_controller(
    capture: &mut OwnedChildCapture,
    recorded_slot: &std::cell::RefCell<Option<HandshakeRecorded>>,
) {
    let stdout_prefix_read = handshake_read_exactly(
        capture.stdout().evidence_reader(),
        HANDSHAKE_STDOUT_PREFIX.len(),
    );
    let stderr_prefix_read = handshake_read_exactly(
        capture.stderr().evidence_reader(),
        HANDSHAKE_STDERR_PREFIX.len(),
    );
    let stdin_exposed = capture.stdin().is_some();
    let release_write = match capture.stdin() {
        Some(stdin) => Some(stdin.write_all(&[HANDSHAKE_RELEASE_BYTE])),
        None => None,
    };
    recorded_slot.replace(Some((
        stdout_prefix_read,
        stderr_prefix_read,
        stdin_exposed,
        release_write,
    )));
}

/// The recorded controller observations shared by BOTH guards: the two
/// prefix reads, whether stdin stayed exposed, and the release write.
type HandshakeRecorded = (
    std::io::Result<Vec<u8>>,
    std::io::Result<Vec<u8>>,
    bool,
    Option<std::io::Result<()>>,
);

/// Terminal verification shared by the handshake guards, run ONLY after
/// the wrapper returned the real collection: exact prefix observations,
/// exact total raw bytes on BOTH evidence files (no decode/filter/
/// normalization), real exit code 9 never reported as a success, and BOTH
/// captures clean `Completed` with the exact byte counts and private 0600
/// modes. Each run states its own expectation: `expected_panic` (the
/// healthy runs expect NO retained controller panic, the panic run
/// expects its receipt — borrowed here so the caller can re-raise it
/// afterwards) and `expect_release` (the released runs expect the single
/// release byte written `Ok`, the panic run expects it NEVER written).
/// A `WaitRefused` outcome is a KEPT, UNRESOLVED verdict here: the
/// handback (original cause, child, both still-running captures, both
/// paths) is moved back OUT to the caller level unchanged — no re-wait,
/// no join, no invented status runs inside this checker. Errors: returns
/// `Err(WaitRefusalHandback)` exactly for that transfer — every other
/// failed assertion panics with its real cause.
fn assert_handshake_collection(
    recorded: HandshakeRecorded,
    expected_panic: bool,
    expect_release: bool,
    controller_panic: &Option<Box<dyn std::any::Any + Send>>,
    outcome: ChildCaptureOutcome,
    stdout_evidence: &Path,
    stderr_evidence: &Path,
) -> Result<(), WaitRefusalHandback> {
    let (stdout_prefix_read, stderr_prefix_read, stdin_exposed, release_write) = recorded;
    assert_eq!(
        controller_panic.is_some(),
        expected_panic,
        "the checker itself must first see the expected panic receipt state \
         (expected_panic {expected_panic})"
    );
    // Terminal assertions — none ran before the real collection above.
    assert!(
        stdin_exposed,
        "piped stdin must stay exposed for the handshake release"
    );
    match (&release_write, expect_release) {
        (Some(Ok(())), true) => {}
        (None, false) => {}
        other => panic!(
            "the release write must match this run's expectation \
             (expect_release {expect_release}): {other:?}"
        ),
    }
    assert_eq!(
        stdout_prefix_read.expect("causal stdout prefix read"),
        HANDSHAKE_STDOUT_PREFIX,
        "the exact raw stdout prefix must be readable through the \
         evidence reader BEFORE the release byte"
    );
    assert_eq!(
        stderr_prefix_read.expect("causal stderr prefix read"),
        HANDSHAKE_STDERR_PREFIX,
        "the exact raw stderr prefix must be readable through the \
         evidence reader BEFORE the release byte"
    );
    let (exit, stdout_capture, stderr_capture) = match outcome {
        ChildCaptureOutcome::Finished {
            exit,
            stdout_capture,
            stderr_capture,
            ..
        } => (exit, stdout_capture, stderr_capture),
        // The refusal is NEVER resolved here: the complete unresolved
        // handback is transferred clearly to the caller level (the
        // guards decide explicitly; the contract forbids an automatic
        // re-wait inside the checker).
        ChildCaptureOutcome::WaitRefused(handback) => return Err(handback),
    };
    assert_eq!(
        exit.code(),
        Some(9),
        "released child must terminate naturally with its real code 9: {exit:?}"
    );
    assert!(
        !exit.success(),
        "a nonzero natural exit must never be reported as a success: {exit:?}"
    );
    match (stdout_capture, stderr_capture) {
        (
            LiveStreamState::Completed { bytes: stdout_bytes },
            LiveStreamState::Completed { bytes: stderr_bytes },
        ) => {
            assert_eq!(
                stdout_bytes,
                (HANDSHAKE_STDOUT_PREFIX.len() + HANDSHAKE_STDOUT_TAIL.len()) as u64,
                "stdout capture byte count (prefix + tail)"
            );
            assert_eq!(
                stderr_bytes,
                (HANDSHAKE_STDERR_PREFIX.len() + HANDSHAKE_STDERR_TAIL.len()) as u64,
                "stderr capture byte count (prefix + tail)"
            );
        }
        other => panic!(
            "both captures must complete cleanly, got {other:?} — a capture \
             issue is never masked into a completion"
        ),
    }
    // Exact total raw bytes on disk — prefixes then tails, nothing
    // added, removed or normalized.
    let mut expected_stdout = Vec::new();
    expected_stdout.extend_from_slice(HANDSHAKE_STDOUT_PREFIX);
    expected_stdout.extend_from_slice(HANDSHAKE_STDOUT_TAIL);
    assert_eq!(
        fs::read(stdout_evidence).expect("stdout evidence read"),
        expected_stdout,
        "raw stdout evidence must be exactly prefix then tail"
    );
    let mut expected_stderr = Vec::new();
    expected_stderr.extend_from_slice(HANDSHAKE_STDERR_PREFIX);
    expected_stderr.extend_from_slice(HANDSHAKE_STDERR_TAIL);
    assert_eq!(
        fs::read(stderr_evidence).expect("stderr evidence read"),
        expected_stderr,
        "raw stderr evidence must be exactly prefix then tail"
    );
    // Private evidence modes: both files stay 0600.
    for path in [stdout_evidence, stderr_evidence] {
        let mode = fs::metadata(path)
            .expect("evidence metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "evidence {path:?} must stay private");
    }
    Ok(())
}

/// HANDSHAKE guard: a real local `/usr/bin/python3 -c` child (constant
/// script, no provider, no live DB, no UID/enrollment) writes and flushes
/// BOTH distinct raw prefixes (NUL, non-UTF-8, CRLF), then blocks on
/// exactly ONE stdin byte, and only after that release emits the known
/// tails and exits with its real code 9. The healthy controller consumes
/// BOTH exact prefixes through the live evidence readers BEFORE writing
/// the single release byte to the exposed stdin — this causal ordering
/// proves that captured bytes are readable BEFORE producer EOF; the
/// cooperative wrapper then runs the explicit finish (stdin EOF, real
/// wait, BOTH joins) after the controller returned normally. Inputs:
/// explicit `private_fixture_leaf` under the runner's configured TMPDIR
/// (never /tmp). Side effects: two 0600 evidence files in the leaf, one
/// synthetic child, exactly one stdin byte — no arbitrary PID signal, no
/// timer, no poll, no retry, no volume ceiling, no global environment
/// mutation. Errors: the prefix reads and the release write are RECORDED
/// as Results by the controller and asserted only AFTER the wrapper
/// returned the real collection, so no failed observation can abort
/// before the child's true issue is conserved; the wrapper guarantees the
/// finish on BOTH exits (see `run_controller_then_finish`). Asserts, after
/// real collection only: no controller panic, exact prefix observations,
/// exact total raw bytes on BOTH evidence files, real exit code 9 never
/// reported as a success, and BOTH captures clean `Completed` with the
/// exact byte counts. The leaf is removed only on success and kept on
/// failure; synthetic children prove no business acceptance.
#[test]
fn child_capture_handshake_reads_both_prefixes_before_stdin_release() {
    let leaf = private_fixture_leaf("child-handshake");
    let stdout_evidence = leaf.join("handshake-stdout.bin");
    let stderr_evidence = leaf.join("handshake-stderr.bin");
    let capture = spawn_and_start_handshake_child(&stdout_evidence, &stderr_evidence);
    // Recorded observations live in this slot: the wrapper contract keeps
    // the controller returning `()`, so the healthy controller writes its
    // Results here instead of returning them.
    let recorded_slot: std::cell::RefCell<Option<HandshakeRecorded>> =
        std::cell::RefCell::new(None);
    let recorded_slot_ref = &recorded_slot;
    let result = capture.run_controller_then_finish(move |capture| {
        run_handshake_prefix_reads_controller(capture, recorded_slot_ref);
    });
    let ControllerCaptureResult {
        outcome,
        controller_panic,
    } = result;
    let recorded = recorded_slot
        .borrow_mut()
        .take()
        .expect("the healthy controller must record its observations");
    // Terminal verification — none ran before the real collection above.
    // A refusal would come back UNRESOLVED and is conserved into the
    // COMPLETE typed carrier (with the retained controller panic payload
    // moved in) and failed with explicitly at this caller level (the
    // contract forbids an automatic re-wait).
    let refusal_result = assert_handshake_collection(
        recorded,
        false,
        true,
        &controller_panic,
        outcome,
        &stdout_evidence,
        &stderr_evidence,
    );
    if let Err(handback) = refusal_result {
        fail_unexpected_wait_refusal(conserve_unexpected_wait_refusal_without_rewait(
            "child_capture_handshake_reads_both_prefixes_before_stdin_release",
            handback,
            controller_panic,
        ));
    }
    fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

/// Distinctive original marker carried by the panic controller's payload:
/// asserted VERBATIM in the re-raised payload after real collection.
const PANIC_CONTROLLER_MARKER: &str = "CONTROLLER-PANIC-BEFORE-RELEASE-7f3a";

/// CONTROLLER-PANIC guard: proves that a panicking controller cannot
/// bypass the real finish and that the ORIGINAL payload reaches the
/// caller verbatim. Healthy witness FIRST in the same leaf with the SAME
/// payload bytes and protocol (its own real `/usr/bin/python3 -c` child,
/// both prefixes, release, finish, full terminal verification via
/// `assert_handshake_collection`), THEN the real panic phase: another
/// finite synthetic child with the SAME constant script has its controller
/// read both raw prefixes through the evidence readers and panic with a
/// DISTINCTIVE original message BEFORE the release byte — the wrapper
/// catches that panic, still runs the real finish via stdin EOF (the
/// script's single `read(1)` returns), and returns the complete real
/// outcome AND the original payload. Verified AFTER real collection, in
/// this order: the retained receipt IS the original (non-None), the finish
/// is real (exit code 9, NOT a success, both captures `Completed` with
/// exact counts, exact prefix+tail bytes on disk, 0600 modes), the release
/// byte was NEVER written (the child released on EOF alone, recorded
/// `release_write` None), and finally the ORIGINAL payload re-raised
/// inside a test `catch_unwind` carries the same distinctive message —
/// proving the caller can judge the receipt and re-raise afterwards.
/// Inputs: one explicit `private_fixture_leaf` under the runner's
/// configured TMPDIR (never /tmp), FOUR 0600 evidence files, two synthetic
/// children. Limits: NO arbitrary PID signal, no timer, no poll, no retry,
/// no volume ceiling, no global environment mutation; the panic controller
/// panics DELIBERATELY (no manufactured child issue); a secondary cleanup
/// failure would keep its real cause printed and the test failing (never
/// masked). The leaf is removed only on success and kept on failure;
/// synthetic children prove no business acceptance.
#[test]
fn child_capture_controller_panic_keeps_real_finish_and_original_payload() {
    let leaf = private_fixture_leaf("child-controller-panic");
    // HEALTHY WITNESS FIRST — same leaf, same payload bytes, same
    // protocol; a witness failure fails the test before the panic phase.
    let witness_stdout = leaf.join("witness-stdout.bin");
    let witness_stderr = leaf.join("witness-stderr.bin");
    let witness = spawn_and_start_handshake_child(&witness_stdout, &witness_stderr);
    let witness_slot: std::cell::RefCell<Option<HandshakeRecorded>> =
        std::cell::RefCell::new(None);
    let witness_slot_ref = &witness_slot;
    let witness_result = witness.run_controller_then_finish(move |capture| {
        run_handshake_prefix_reads_controller(capture, witness_slot_ref);
    });
    let ControllerCaptureResult {
        outcome: witness_outcome,
        controller_panic: witness_panic,
    } = witness_result;
    let witness_recorded = witness_slot
        .borrow_mut()
        .take()
        .expect("the healthy witness controller must record its observations");
    let witness_refusal_result = assert_handshake_collection(
        witness_recorded,
        false,
        true,
        &witness_panic,
        witness_outcome,
        &witness_stdout,
        &witness_stderr,
    );
    if let Err(witness_handback) = witness_refusal_result {
        fail_unexpected_wait_refusal(conserve_unexpected_wait_refusal_without_rewait(
            "healthy controller-panic witness wait refused; no panic phase may run",
            witness_handback,
            witness_panic,
        ));
    }
    // PANIC PHASE — a second real child with the SAME constant script; its
    // controller records its causal reads BEFORE the distinctive original
    // panic (BEFORE the release byte — the child stays blocked on stdin).
    let panic_stdout = leaf.join("panic-stdout.bin");
    let panic_stderr = leaf.join("panic-stderr.bin");
    let capture = spawn_and_start_handshake_child(&panic_stdout, &panic_stderr);
    let panic_slot: std::cell::RefCell<Option<HandshakeRecorded>> =
        std::cell::RefCell::new(None);
    let panic_marker = PANIC_CONTROLLER_MARKER;
    let panic_slot_ref = &panic_slot;
    let panic_result = capture.run_controller_then_finish(move |capture| {
        let stdout_prefix_read = handshake_read_exactly(
            capture.stdout().evidence_reader(),
            HANDSHAKE_STDOUT_PREFIX.len(),
        );
        let stderr_prefix_read = handshake_read_exactly(
            capture.stderr().evidence_reader(),
            HANDSHAKE_STDERR_PREFIX.len(),
        );
        let stdin_was_exposed = capture.stdin().is_some();
        panic_slot_ref.replace(Some((
            stdout_prefix_read,
            stderr_prefix_read,
            stdin_was_exposed,
            // The release byte is NEVER written in this run: the child
            // releases on EOF alone.
            None,
        )));
        panic!(
            "{panic_marker} stdout prefix read complete before the release"
        );
    });
    let ControllerCaptureResult {
        outcome: panic_outcome,
        controller_panic: mut panic_receipt,
    } = panic_result;
    // The ORIGINAL payload is retained verbatim: the receipt is present.
    assert!(
        panic_receipt.is_some(),
        "the wrapper must retain the controller's original panic payload"
    );
    let panic_recorded = panic_slot
        .borrow_mut()
        .take()
        .expect("the panic controller must record its reads before panicking");
    // Real-finish verification via the SHARED terminal check — the panic
    // run must satisfy the SAME collection contract as the healthy runs
    // (real exit 9, both Completed exact counts, exact prefix+tail files,
    // 0600 modes) with the two panic-run expectations: the receipt is
    // retained (borrowed so the re-raise below stays possible) and the
    // release byte was NEVER written. The recorded stdout prefix read is
    // first checked on a BORROWED view (the recording itself is consumed
    // by the shared check below).
    assert_eq!(
        panic_recorded
            .0
            .as_ref()
            .expect("causal stdout prefix read before the panic"),
        &HANDSHAKE_STDOUT_PREFIX.to_vec(),
        "the exact raw stdout prefix must be readable through the evidence \
         reader BEFORE the controller panic"
    );
    let panic_refusal_result = assert_handshake_collection(
        panic_recorded,
        true,
        false,
        &panic_receipt,
        panic_outcome,
        &panic_stdout,
        &panic_stderr,
    );
    if let Err(panic_handback) = panic_refusal_result {
        fail_unexpected_wait_refusal(conserve_unexpected_wait_refusal_without_rewait(
            "panic-phase wait refused",
            panic_handback,
            panic_receipt,
        ));
    }
    // ORIGINAL payload re-emission verified via a test catch_unwind: the
    // receipt was conserved above, so the caller can judge first and
    // re-raise afterwards.
    let re_raise = panic_receipt
        .take()
        .expect("the panic receipt must still be present for the re-raise");
    let recaught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        std::panic::resume_unwind(re_raise);
    }));
    let payload = recaught.expect_err("resuming the receipt must panic again");
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "<non-string payload>".to_string());
    assert!(
        message.contains(PANIC_CONTROLLER_MARKER),
        "the re-raised payload must be the ORIGINAL distinctive message: \
         {message:?}"
    );
    fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

// ---- Guard: wait refusal keeps cause and returned ownership -----------------

/// Distinctive SYNTHETIC refusal cause injected by the refusal guard's
/// seam handler BEFORE any real wait. LABEL: this refusal is SYNTHETIC —
/// it proves the handback contract, NOT a real OS `Child::wait` error.
const SYNTHETIC_REFUSAL_MARKER: &str = "SYNTHETIC-WAIT-REFUSAL-4b91";

/// Distinctive ORIGINAL controller payload HELD by the typed-carrier
/// round-trip (an owned `String`, boxed as `Any + Send` BEFORE the panic
/// door): its transmission through the typed boundary is verified by
/// EXACT value equality after the by-value recovery — never displayed as
/// the proof and never invented after the door.
const CARRIER_ROUNDTRIP_PANIC_MARKER: &str = "CARRIER-ROUNDTRIP-ORIGINAL-PANIC-9d47";

/// Raw stdout bytes of the refusal guard's constant script: identical raw
/// shape to the handshake prefixes (NUL, non-UTF-8, CRLF), flushed before
/// the stdin block.
const REFUSAL_STDOUT_PREFIX: &[u8] = b"REFUSAL-OUT\x00\xff\r\n";

/// Raw stderr bytes of the refusal guard's constant script (distinct from
/// stdout), flushed before the stdin block.
const REFUSAL_STDERR_PREFIX: &[u8] = b"REFUSAL-ERR\x00\x80\x81\r\n";

/// Raw stdout tail emitted by the refusal guard's script AFTER the stdin
/// EOF permission (its single `read(1)` returns), then real exit 5.
const REFUSAL_STDOUT_TAIL: &[u8] = b"REFUSAL-OUT-TAIL-EOF\r\n";

/// Raw stderr tail emitted by the refusal guard's script AFTER the stdin
/// EOF permission, before the real exit 5.
const REFUSAL_STDERR_TAIL: &[u8] = b"REFUSAL-ERR-TAIL-EOF\r\n";

/// Constant local FINITE cooperative script for the refusal guard's real
/// child: writes and flushes BOTH distinct raw prefixes, blocks on
/// exactly ONE stdin byte (a single `read(1)` — stdin EOF releases it),
/// emits the known tails and terminates NATURALLY with exit code 5.
/// Plaintext source, no provider, no live DB, no UID/enrollment.
const REFUSAL_SCRIPT: &str = "import sys\n\
    sys.stdout.buffer.write(b'REFUSAL-OUT\\x00\\xff\\r\\n')\n\
    sys.stdout.buffer.flush()\n\
    sys.stderr.buffer.write(b'REFUSAL-ERR\\x00\\x80\\x81\\r\\n')\n\
    sys.stderr.buffer.flush()\n\
    got = sys.stdin.buffer.read(1)\n\
    assert got == b'', f'unexpected stdin bytes: {got!r}'\n\
    sys.stdout.buffer.write(b'REFUSAL-OUT-TAIL-EOF\\r\\n')\n\
    sys.stdout.buffer.flush()\n\
    sys.stderr.buffer.write(b'REFUSAL-ERR-TAIL-EOF\\r\\n')\n\
    sys.stderr.buffer.flush()\n\
    raise SystemExit(5)\n";

/// Terminal verification shared by the refusal guard's witness and final
/// phases: exact prefix observations, real exit code 5 never reported as
/// a success, BOTH captures clean `Completed` with the exact byte counts,
/// exact total raw bytes on BOTH evidence files (no decode/filter/
/// normalization) and private 0600 modes. Inputs are ALREADY-collected
/// results — nothing here runs before the caller collected the real
/// issue, and no input is modified. Proves no business acceptance.
fn assert_refusal_script_collection(
    stdout_prefix_read: std::io::Result<Vec<u8>>,
    stderr_prefix_read: std::io::Result<Vec<u8>>,
    waited: std::io::Result<std::process::ExitStatus>,
    stdout_state: LiveStreamState,
    stderr_state: LiveStreamState,
    stdout_evidence: &Path,
    stderr_evidence: &Path,
) {
    assert_eq!(
        stdout_prefix_read.expect("causal stdout prefix read"),
        REFUSAL_STDOUT_PREFIX,
        "the exact raw stdout prefix must be readable through the \
         evidence reader BEFORE the finish"
    );
    assert_eq!(
        stderr_prefix_read.expect("causal stderr prefix read"),
        REFUSAL_STDERR_PREFIX,
        "the exact raw stderr prefix must be readable through the \
         evidence reader BEFORE the finish"
    );
    let exit = waited.expect(
        "the real wait must succeed: wait refusals are handed back \
         unresolved, never retried inside the module",
    );
    assert_eq!(
        exit.code(),
        Some(5),
        "the released child must terminate naturally with its real code \
         5: {exit:?}"
    );
    assert!(
        !exit.success(),
        "a nonzero natural exit must never be reported as a success: {exit:?}"
    );
    match (stdout_state, stderr_state) {
        (
            LiveStreamState::Completed { bytes: stdout_bytes },
            LiveStreamState::Completed { bytes: stderr_bytes },
        ) => {
            assert_eq!(
                stdout_bytes,
                (REFUSAL_STDOUT_PREFIX.len() + REFUSAL_STDOUT_TAIL.len()) as u64,
                "stdout capture byte count (prefix + tail)"
            );
            assert_eq!(
                stderr_bytes,
                (REFUSAL_STDERR_PREFIX.len() + REFUSAL_STDERR_TAIL.len()) as u64,
                "stderr capture byte count (prefix + tail)"
            );
        }
        other => panic!(
            "both captures must complete cleanly, got {other:?} — a \
             capture issue is never masked into a completion"
        ),
    }
    let mut expected_stdout = Vec::new();
    expected_stdout.extend_from_slice(REFUSAL_STDOUT_PREFIX);
    expected_stdout.extend_from_slice(REFUSAL_STDOUT_TAIL);
    assert_eq!(
        fs::read(stdout_evidence).expect("stdout evidence read"),
        expected_stdout,
        "raw stdout evidence must be exactly prefix then tail"
    );
    let mut expected_stderr = Vec::new();
    expected_stderr.extend_from_slice(REFUSAL_STDERR_PREFIX);
    expected_stderr.extend_from_slice(REFUSAL_STDERR_TAIL);
    assert_eq!(
        fs::read(stderr_evidence).expect("stderr evidence read"),
        expected_stderr,
        "raw stderr evidence must be exactly prefix then tail"
    );
    for path in [stdout_evidence, stderr_evidence] {
        let mode = fs::metadata(path)
            .expect("evidence metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "evidence {path:?} must stay private");
    }
}

/// WAIT-REFUSAL OWNERSHIP guard: pins the repaired refusal contract — ONE
/// wait attempt per finish, the handback UNRESOLVED at the caller level —
/// AND exercises the REAL typed recovery boundary end-to-end. Healthy
/// witness FIRST: a real `/usr/bin/python3 -c` child with the SAME
/// constant script, same leaf, same capture protocol — both causal prefix
/// reads through the live evidence readers, the PUBLIC finish (real
/// `Child::wait` through the private seam), then the full shared terminal
/// verification. THEN the injection phase: another real child with the
/// SAME script is captured, and `finish` is called through the PRIVATE
/// seam with a SYNTHETIC `FnOnce` handler that records its own invocation
/// count and the child pid it saw, NEVER touches the child and returns a
/// DISTINCTIVE `io::Error` BEFORE any real wait (the public path passes
/// the real `Child::wait`; no global mock, no RLIMIT, no OS-resource
/// exhaustion). On the returned handback, the test first SNAPSHOTS the
/// facts as data WITHOUT asserting (no verdict): the cause text via
/// `to_string()` (`io::Error` is never Clone — no copy, no re-creation),
/// the single handler invocation count, the child pid the handler saw,
/// both evidence paths and the stdin exposure. It THEN conserves the
/// handback with the REAL
/// `conserve_unexpected_wait_refusal_without_rewait` (EVERY field moved
/// in, NOTHING dropped; BOTH captures REALLY join to their terminal
/// states — structurally proving the module did NOT join on the refusal
/// path, since a double join would panic; NO re-wait — the exit status
/// stays UNKNOWN) carrying a DISTINCTIVE ORIGINAL controller payload
/// boxed BEFORE the panic door. The conserved carrier is then
/// transferred REALLY through the TYPED boundary: the REAL
/// `fail_unexpected_wait_refusal` (the same function every
/// unexpected-refusal site uses) inside a test `catch_unwind`, and the
/// boxed payload recovered BY VALUE into the complete
/// `WaitRefusalCarrier` (a bare `downcast_ref` borrow would NOT pass
/// ownership). The recovered carrier's facts are SNAPSHOTTED as data
/// (context label, recovered cause text, recovered child pid, both
/// paths, both capture states, payload presence) — still NO verdict.
/// The child is resolved EXPLICITLY (the caller's own decision, no
/// automatic retry): the precondition is genuinely changed because the
/// injected handler never reached the real `Child::wait`, so this direct
/// re-wait is the FIRST real wait — and BOTH captures are NEVER joined a
/// second time (they are already terminal states inside the carrier;
/// a second join is structurally impossible). ONLY THEN, after the
/// typed round-trip, the FIRST real wait and the real joins, does the
/// test run ALL terminal assertions, none weakened: the conserved
/// context label comes back verbatim, the snapshot cause IS the original
/// injected message (equality through the typed transfer), the cause
/// comes back UNCHANGED from the recovered carrier (same text across
/// conservation, `panic_any`, `catch_unwind` and the by-value downcast —
/// moved, never copied or re-created), the seam handler ran EXACTLY ONCE
/// (no automatic additional wait attempt exists anywhere in the module —
/// the contract has no retry), the SAME child came back at every stage
/// (pid equality: handback child == the single handler invocation's
/// child; recovered carrier child == handback child; the pid stays
/// reserved until reaped, so the comparison is deterministic), both
/// evidence paths are conserved exactly through the typed round-trip,
/// the ORIGINAL controller payload is present inside the recovered
/// carrier, downcasts BY VALUE to its original `String` type and equals
/// the held original EXACTLY (never displayed, never invented after the
/// door), both prefix reads succeeded while the child was still blocked,
/// and stdin was exposed with no release byte ever written (the script
/// itself asserts its single stdin read returned EOF, so any release
/// byte would surface as a real exit-1 failure in the terminal
/// verification). The shared terminal verification then proves the real
/// collection: exit code 5, BOTH `Completed` captures with the exact
/// 37/38 byte counts, exact prefix+tail raw evidence files and 0600
/// modes. Inputs: one explicit `private_fixture_leaf` under the runner's
/// configured TMPDIR (never /tmp), FOUR 0600 evidence files, two
/// synthetic children. Limits: the injected refusal is SYNTHETIC — it
/// proves the handback AND typed-recovery contract, NOT a real OS wait
/// error; the default panic hook may print during the `catch_unwind` (no
/// global hook mutation is allowed); no arbitrary PID signal, no timer,
/// no poll, no retry cap, no volume ceiling, no global environment
/// mutation; causes and original artifacts are kept (leaf retained on
/// failure). Synthetic children prove no business acceptance.
#[test]
fn child_capture_wait_refusal_preserves_cause_and_returned_ownership() {
    let leaf = private_fixture_leaf("child-wait-refusal");
    // HEALTHY WITNESS FIRST — same leaf, same constant script, same
    // capture protocol; a witness failure fails the test before the
    // injection phase.
    let witness_stdout = leaf.join("witness-stdout.bin");
    let witness_stderr = leaf.join("witness-stderr.bin");
    let witness_child = std::process::Command::new("/usr/bin/python3")
        .arg("-c")
        .arg(REFUSAL_SCRIPT)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("synthetic local python3 witness child must spawn");
    let mut witness = start_or_refuse(witness_child, &witness_stdout, &witness_stderr);
    // Causal prefix reads while the child is still blocked on its stdin
    // byte; the Results are asserted only after the real collection below.
    let witness_stdout_prefix = handshake_read_exactly(
        witness.stdout().evidence_reader(),
        REFUSAL_STDOUT_PREFIX.len(),
    );
    let witness_stderr_prefix = handshake_read_exactly(
        witness.stderr().evidence_reader(),
        REFUSAL_STDERR_PREFIX.len(),
    );
    let witness_outcome = witness.finish();
    let (witness_waited, witness_stdout_state, witness_stderr_state) =
        match witness_outcome {
            ChildCaptureOutcome::Finished {
                exit,
                stdout_capture,
                stderr_capture,
                ..
            } => (Ok(exit), stdout_capture, stderr_capture),
            ChildCaptureOutcome::WaitRefused(handback) => {
                fail_unexpected_wait_refusal(
                    conserve_unexpected_wait_refusal_without_rewait(
                        "refusal-guard healthy witness wait refused",
                        handback,
                        None,
                    ),
                )
            }
        };
    assert_refusal_script_collection(
        witness_stdout_prefix,
        witness_stderr_prefix,
        witness_waited,
        witness_stdout_state,
        witness_stderr_state,
        &witness_stdout,
        &witness_stderr,
    );
    // INJECTION PHASE — a second real child with the SAME constant script.
    let injected_stdout = leaf.join("injected-stdout.bin");
    let injected_stderr = leaf.join("injected-stderr.bin");
    let refusal_child = std::process::Command::new("/usr/bin/python3")
        .arg("-c")
        .arg(REFUSAL_SCRIPT)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("synthetic local python3 refusal child must spawn");
    let mut capture = start_or_refuse(refusal_child, &injected_stdout, &injected_stderr);
    // Causal prefix reads BEFORE the finish (the child stays blocked on
    // its stdin byte until the finish's stdin-EOF permission releases
    // it); Results asserted after the real collection below.
    let injected_stdout_prefix = handshake_read_exactly(
        capture.stdout().evidence_reader(),
        REFUSAL_STDOUT_PREFIX.len(),
    );
    let injected_stderr_prefix = handshake_read_exactly(
        capture.stderr().evidence_reader(),
        REFUSAL_STDERR_PREFIX.len(),
    );
    let injected_stdin_exposed = capture.stdin().is_some();
    // THE SEAM CALL: finish through the private handler with a SYNTHETIC
    // refusal injected BEFORE any real wait. The handler is FnOnce,
    // records its OWN invocation count and the child pid it saw, NEVER
    // touches the child and is called EXACTLY ONCE (the contract's one
    // wait attempt per finish).
    let seam_invocations = std::cell::Cell::new(0u32);
    let seam_child_pid = std::cell::Cell::new(0u32);
    let seam_invocations_ref = &seam_invocations;
    let seam_child_pid_ref = &seam_child_pid;
    let injected_cause =
        format!("{SYNTHETIC_REFUSAL_MARKER} injected before any real wait");
    let outcome = capture.finish_with_wait(move |handler_child| {
        seam_child_pid_ref.set(handler_child.id());
        seam_invocations_ref.set(seam_invocations_ref.get() + 1);
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            injected_cause,
        ))
    });
    // The caller receives the UNRESOLVED handback. Its facts are first
    // SNAPSHOTED as data (no verdict): the cause text via `to_string()`
    // (`io::Error` is never Clone — no copy, no re-creation), the single
    // handler invocation count, the child pid the handler saw, both
    // evidence paths and the stdin exposure. NOTHING is asserted yet.
    let handback = match outcome {
        ChildCaptureOutcome::WaitRefused(handback) => handback,
        ChildCaptureOutcome::Finished { exit, .. } => panic!(
            "the SYNTHETIC seam handler must refuse the wait; the finish \
             must NOT report a Finished outcome: {exit:?}"
        ),
    };
    let WaitRefusalHandback {
        error: refused,
        child,
        stdout_stream,
        stderr_stream,
        stdout_path: refused_stdout_path,
        stderr_path: refused_stderr_path,
    } = handback;
    let refused_cause_text = refused.to_string();
    let seam_invocation_count = seam_invocations.get();
    let handback_child_pid = child.id();
    // TYPED CONSERVATION at the caller level — the REAL production-shape
    // helper moves EVERY handback field into the complete carrier and
    // joins BOTH captures to their real terminal states (the joins also
    // structurally prove the module did NOT join on the refusal path: a
    // double join would panic). NO re-wait here — the exit status stays
    // UNKNOWN through the conservation. A DISTINCTIVE ORIGINAL controller
    // payload travels INSIDE the carrier: it is boxed BEFORE the panic
    // door and verified by EXACT value equality after the recovery
    // (never displayed, never invented).
    const ROUNDTRIP_CONTEXT: &str =
        "typed-carrier round-trip refusal conserved before the typed failure";
    let original_controller_payload: Box<dyn std::any::Any + Send> =
        Box::new(CARRIER_ROUNDTRIP_PANIC_MARKER.to_string());
    let conserved = conserve_unexpected_wait_refusal_without_rewait(
        ROUNDTRIP_CONTEXT,
        WaitRefusalHandback {
            error: refused,
            child,
            stdout_stream,
            stderr_stream,
            stdout_path: refused_stdout_path,
            stderr_path: refused_stderr_path,
        },
        Some(original_controller_payload),
    );
    // THE REAL TYPED BOUNDARY — the conserved carrier is failed with the
    // REAL `fail_unexpected_wait_refusal` (the same function every
    // unexpected-refusal site uses) inside a test `catch_unwind`, and the
    // boxed payload is recovered BY VALUE into the complete carrier (a
    // bare `downcast_ref` borrow would not pass ownership). The default
    // panic hook may print during the catch (no global hook mutation is
    // allowed); the hook never displays the payload itself.
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        fail_unexpected_wait_refusal(conserved);
    }));
    let boxed_payload = caught
        .expect_err("the typed refusal failure must really panic with the \
                     conserved carrier as its payload");
    let carrier = boxed_payload
        .downcast::<WaitRefusalCarrier>()
        .expect("the typed panic payload must downcast BY VALUE into the \
                 complete WaitRefusalCarrier");
    // FACT SNAPSHOTS from the recovered carrier — data only, NO verdict:
    // the owned `Box<WaitRefusalCarrier>` is UNBOXED BY VALUE with the
    // deref-move `*carrier` (Box is the only smart pointer allowing a
    // move out through its Deref), so EVERY field is destructured by
    // move — nothing cloned, no cause re-creation, no borrow replacing
    // the ownership transfer. The failing-site context label, the
    // recovered cause text via `to_string()` (the original non-Clone
    // cause moved through conservation AND the panic boundary), the
    // recovered child pid, both evidence paths, both capture states and
    // the original payload presence. The payload box itself stays
    // UNOPENED until the verdict phase.
    let WaitRefusalCarrier {
        context: carrier_context,
        error: recovered_error,
        mut child,
        stdout_state,
        stderr_state,
        stdout_path: recovered_stdout_path,
        stderr_path: recovered_stderr_path,
        controller_panic: recovered_panic,
    } = *carrier;
    let recovered_cause_text = recovered_error.to_string();
    let recovered_child_pid = child.id();
    let recovered_panic_present = recovered_panic.is_some();
    // EXPLICIT CALLER RESOLUTION through the recovered carrier — the
    // precondition IS changed: the injected seam handler never reached
    // the real `Child::wait`, so this direct re-wait is the FIRST real
    // wait of this child (a deliberate caller decision, not an automatic
    // handler retry; no signal, no timer, no poll). BOTH captures are
    // ALREADY joined terminal states inside the carrier — they are NEVER
    // joined a second time (structurally impossible: the carrier holds
    // states, not streams).
    let waited = child.wait();
    // ALL terminal verdicts — only now, AFTER the typed round-trip, the
    // FIRST real wait and the real joins, none weakened.
    assert_eq!(
        carrier_context, ROUNDTRIP_CONTEXT,
        "the recovered carrier must carry the conserved failing-site \
         context label verbatim"
    );
    assert_eq!(
        refused_cause_text,
        format!("{SYNTHETIC_REFUSAL_MARKER} injected before any real wait"),
        "the handback must carry the ORIGINAL injected cause verbatim \
         (snapshot equality — io::Error moved, never copied or re-created)"
    );
    assert_eq!(
        recovered_cause_text, refused_cause_text,
        "the original injected cause must come back UNCHANGED through the \
         REAL typed boundary (conservation, panic_any, catch_unwind, \
         by-value downcast): same text, moved, never copied or re-created"
    );
    assert_eq!(
        seam_invocation_count, 1,
        "exactly ONE wait attempt per finish: no automatic additional \
         wait attempt may exist (the seam handler must not be re-called)"
    );
    assert_eq!(
        handback_child_pid, seam_child_pid.get(),
        "the handback must return the SAME child the single wait attempt \
         saw (ownership conserved into the handback)"
    );
    assert_eq!(
        recovered_child_pid, handback_child_pid,
        "the recovered carrier must hold the SAME child through the whole \
         typed round-trip (conservation, panic and by-value recovery)"
    );
    assert_eq!(
        recovered_stdout_path, injected_stdout,
        "the stdout evidence path must be conserved through the typed \
         round-trip"
    );
    assert_eq!(
        recovered_stderr_path, injected_stderr,
        "the stderr evidence path must be conserved through the typed \
         round-trip"
    );
    assert!(
        recovered_panic_present,
        "the ORIGINAL controller payload must travel INSIDE the recovered \
         carrier (present after the typed round-trip)"
    );
    let recovered_payload_text = *recovered_panic
        .expect("the original payload must still be present for its \
                 by-value verification")
        .downcast::<String>()
        .expect("the original payload must downcast BY VALUE to its \
                 original String type");
    assert_eq!(
        recovered_payload_text, CARRIER_ROUNDTRIP_PANIC_MARKER,
        "the ORIGINAL controller payload must be transmitted EXACTLY \
         through the typed boundary (by-value equality with the held \
         original — never displayed, never invented)"
    );
    assert!(
        injected_stdin_exposed,
        "piped stdin must stay exposed for the finish's EOF permission"
    );
    assert_refusal_script_collection(
        injected_stdout_prefix,
        injected_stderr_prefix,
        waited,
        stdout_state,
        stderr_state,
        &recovered_stdout_path,
        &recovered_stderr_path,
    );
    fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

// ---- Guard: EXPLICIT owned-signal stop of an EOF-ignoring child -----------

/// Raw stdout prefix of the signal guard's constant script: NUL, a lone
/// non-UTF-8 byte (`\xfe`) and CRLF, written and flushed BEFORE the script
/// blocks waiting for its signal.
const SIGNAL_STDOUT_PREFIX: &[u8] = b"SIGNAL-OUT\x00\xfe\r\n";

/// Raw stderr prefix of the same script: NUL, invalid-UTF-8 continuation
/// bytes (`\x82\x83`) and CRLF, written and flushed on the OTHER stream
/// before the same wait — the two prefixes are distinct byte sequences.
const SIGNAL_STDERR_PREFIX: &[u8] = b"SIGNAL-ERR\x00\x82\x83\r\n";

/// Known stdout tail, emitted ONLY by the script's SIGINT handler — its
/// presence on the evidence file proves the signal was really DELIVERED
/// and HANDLED (a default SIGINT death would leave no tail and no exit
/// code 0).
const SIGNAL_STDOUT_TAIL: &[u8] = b"SIGNAL-OUT-TAIL-SIGINT\r\n";

/// Known stderr tail, emitted only by the same handler.
const SIGNAL_STDERR_TAIL: &[u8] = b"SIGNAL-ERR-TAIL-SIGINT\r\n";

/// Constant local FINITE signal-stop script for the explicit-signal
/// guard's real child: installs its SIGINT handler FIRST (before ANY
/// output), writes and flushes BOTH distinct raw prefixes (NUL, non-UTF-8,
/// CRLF), then blocks causally in `signal.pause()` — it NEVER reads
/// stdin, so stdin EOF does NOT stop it; ONLY the requested signal
/// releases it. The handler writes BOTH known tails (flushed) and exits 0
/// — the controlled termination this guard pins. Plaintext source, no
/// provider, no live DB, no UID/enrollment.
const SIGNAL_SCRIPT: &str = "import signal, sys\n\
    def handler(signum, frame): sys.stdout.buffer.write(b'SIGNAL-OUT-TAIL-SIGINT\\r\\n'); \
    sys.stdout.buffer.flush(); \
    sys.stderr.buffer.write(b'SIGNAL-ERR-TAIL-SIGINT\\r\\n'); \
    sys.stderr.buffer.flush(); \
    raise SystemExit(0)\n\
    signal.signal(signal.SIGINT, handler)\n\
    sys.stdout.buffer.write(b'SIGNAL-OUT\\x00\\xfe\\r\\n')\n\
    sys.stdout.buffer.flush()\n\
    sys.stderr.buffer.write(b'SIGNAL-ERR\\x00\\x82\\x83\\r\\n')\n\
    sys.stderr.buffer.flush()\n\
    signal.pause()\n";

/// The SIGINT signal as the guard's EXPLICIT caller-provided stop
/// decision, passed verbatim to `signal_stop` — the wrapper never chooses
/// a signal itself and the script's handler is installed for exactly this
/// signal. (The rustix variant name follows the POSIX naming; an exact
/// compile-time mismatch would be caught by the recipe's build.)
const SIGNAL_GUARD_SIGINT: rustix::process::Signal = rustix::process::Signal::INT;

/// Distinctive ORIGINAL message carried by the panic controller's payload
/// in the explicit-signal guard: asserted VERBATIM in the re-raised
/// payload after real collection (never displayed as the proof and never
/// invented after the door).
const SIGNAL_CONTROLLER_PANIC_MARKER: &str = "CONTROLLER-PANIC-BEFORE-SIGNAL-3c81";

/// The recorded controller observations shared by BOTH explicit-signal
/// phases: the two causal prefix reads and whether stdin stayed exposed.
/// NO release byte concept exists here — the child ignores stdin EOF.
type SignalRecorded = (
    std::io::Result<Vec<u8>>,
    std::io::Result<Vec<u8>>,
    bool,
);

/// The causal controller core both explicit-signal phases use: it consumes
/// BOTH exact raw prefixes through the live evidence readers while the
/// script is still blocked waiting for its signal, and records the
/// observations into the caller's slot (the wrapper contract keeps the
/// controller returning `()`). NOTHING is asserted inside, NO stdin byte
/// is written and NO signal is sent by the controller itself — the stop
/// decision belongs to the wrapper's caller-provided signal alone.
fn run_signal_prefix_reads_controller(
    capture: &mut OwnedChildCapture,
    recorded_slot: &std::cell::RefCell<Option<SignalRecorded>>,
) {
    let stdout_prefix_read = handshake_read_exactly(
        capture.stdout().evidence_reader(),
        SIGNAL_STDOUT_PREFIX.len(),
    );
    let stderr_prefix_read = handshake_read_exactly(
        capture.stderr().evidence_reader(),
        SIGNAL_STDERR_PREFIX.len(),
    );
    let stdin_exposed = capture.stdin().is_some();
    recorded_slot.replace(Some((stdout_prefix_read, stderr_prefix_read, stdin_exposed)));
}

/// Terminal verification shared by BOTH explicit-signal phases, run ONLY
/// after the wrapper returned the real collection: stdin stayed exposed
/// (and NO release byte ever written — the child stops on the signal
/// alone), exact prefix observations, real exit code 0 through the
/// handler (the controlled termination, asserted AS a success in contrast
/// to the nonzero-exit guards), BOTH captures clean `Completed` with the
/// exact prefix+tail byte counts, exact total raw bytes on BOTH evidence
/// files (no decode/filter/normalization) and private 0600 modes. Inputs
/// are ALREADY-collected results — nothing here runs before the caller
/// collected the real outcome, and no input is modified. Proves no
/// business acceptance.
fn assert_signal_script_collection(
    recorded: SignalRecorded,
    exit: std::process::ExitStatus,
    stdout_state: LiveStreamState,
    stderr_state: LiveStreamState,
    stdout_evidence: &Path,
    stderr_evidence: &Path,
) {
    let (stdout_prefix_read, stderr_prefix_read, stdin_exposed) = recorded;
    assert!(
        stdin_exposed,
        "piped stdin must stay exposed; the EOF-ignoring child is stopped \
         by the explicit signal alone (no release byte exists in this \
         guard)"
    );
    assert_eq!(
        stdout_prefix_read.expect("causal stdout prefix read"),
        SIGNAL_STDOUT_PREFIX,
        "the exact raw stdout prefix must be readable through the \
         evidence reader BEFORE the explicit signal"
    );
    assert_eq!(
        stderr_prefix_read.expect("causal stderr prefix read"),
        SIGNAL_STDERR_PREFIX,
        "the exact raw stderr prefix must be readable through the \
         evidence reader BEFORE the explicit signal"
    );
    assert_eq!(
        exit.code(),
        Some(0),
        "the EOF-ignoring child must terminate through its SIGINT handler \
         with its real code 0 (a default SIGINT death would be a signal \
         status with no tails): {exit:?}"
    );
    assert!(
        exit.success(),
        "code 0 through the handler is the CONTROLLED termination this \
         guard pins (distinct from every nonzero-exit guard): {exit:?}"
    );
    match (stdout_state, stderr_state) {
        (
            LiveStreamState::Completed { bytes: stdout_bytes },
            LiveStreamState::Completed { bytes: stderr_bytes },
        ) => {
            assert_eq!(
                stdout_bytes,
                (SIGNAL_STDOUT_PREFIX.len() + SIGNAL_STDOUT_TAIL.len()) as u64,
                "stdout capture byte count (prefix + handler tail)"
            );
            assert_eq!(
                stderr_bytes,
                (SIGNAL_STDERR_PREFIX.len() + SIGNAL_STDERR_TAIL.len()) as u64,
                "stderr capture byte count (prefix + handler tail)"
            );
        }
        other => panic!(
            "both captures must complete cleanly, got {other:?} — a \
             capture issue is never masked into a completion"
        ),
    }
    let mut expected_stdout = Vec::new();
    expected_stdout.extend_from_slice(SIGNAL_STDOUT_PREFIX);
    expected_stdout.extend_from_slice(SIGNAL_STDOUT_TAIL);
    assert_eq!(
        fs::read(stdout_evidence).expect("stdout evidence read"),
        expected_stdout,
        "raw stdout evidence must be exactly prefix then handler tail"
    );
    let mut expected_stderr = Vec::new();
    expected_stderr.extend_from_slice(SIGNAL_STDERR_PREFIX);
    expected_stderr.extend_from_slice(SIGNAL_STDERR_TAIL);
    assert_eq!(
        fs::read(stderr_evidence).expect("stderr evidence read"),
        expected_stderr,
        "raw stderr evidence must be exactly prefix then handler tail"
    );
    for path in [stdout_evidence, stderr_evidence] {
        let mode = fs::metadata(path)
            .expect("evidence metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "evidence {path:?} must stay private");
    }
}

/// EXPLICIT-SIGNAL guard: proves that a child which IGNORES stdin EOF is
/// stopped by the caller-provided signal through the pidfd held BEFORE
/// the controller call, on the normal AND the panic path, with the
/// COMPLETE real finish and the ORIGINAL panic payload retained. Healthy
/// witness FIRST in the same leaf with the SAME constant script and
/// protocol (its own real `/usr/bin/python3 -c` child installs its SIGINT
/// handler BEFORE any output, writes both raw prefixes, then blocks in
/// `signal.pause()` — it NEVER reads stdin, so stdin EOF cannot stop it):
/// the controller consumes both prefixes causally, then the wrapper
/// requests the caller's SIGINT through the held descriptor, and ONLY
/// after the successful request runs the existing explicit finish (stdin
/// EOF — no effect on this child — ONE real wait, BOTH joins). Verified
/// AFTER real collection: the result is `Stopped` with NO retained panic,
/// the tails exist ONLY after the signal (handler delivery proof), the
/// exit is the real code 0 through the handler, both captures are clean
/// `Completed` with exact counts, both evidence files hold exactly
/// prefix+tail with 0600 modes. THEN the panic phase: another real child
/// with the SAME script has its controller read both prefixes and panic
/// with a DISTINCTIVE original message BEFORE the wrapper's signal — the
/// wrapper still requests the signal through the SAME held descriptor on
/// the panic path, still runs the real finish, and returns the complete
/// real outcome AND the original payload. Verified AFTER real collection,
/// in this order: the result is `Stopped` again (the signal works on the
/// panic path), the retained receipt IS the original (non-None), the
/// finish is real (same full terminal verification via the shared
/// checker), and finally the ORIGINAL payload re-emitted inside a test
/// `catch_unwind` carries the same distinctive message — proving the
/// caller can judge the receipt and re-raise afterwards. NO release byte,
/// NO arbitrary-PID signal, NO timer/poll/retry/cap, NO fallback signal,
/// NO SIGKILL; the pidfd-open and signal-refused paths are NOT exercised
/// here — they stay EXPLICIT limits of the wrapper contract (typed
/// full-ownership refusals), never invented coverage. Inputs: one
/// explicit `private_fixture_leaf` under the runner's configured TMPDIR
/// (never /tmp), FOUR 0600 evidence files, two synthetic children. The
/// default panic hook may print during the catch (no global hook mutation
/// is allowed). The leaf is removed only on success and kept on failure;
/// synthetic children prove no business acceptance (the wrapper proves
/// neither UID independence, nor OS confinement, nor business
/// acceptance).
#[test]
fn child_capture_owned_signal_stops_eof_ignoring_child_and_retains_panic() {
    let leaf = private_fixture_leaf("child-signal-stop");
    // HEALTHY WITNESS FIRST — same leaf, same constant script, same
    // capture protocol; a witness failure fails the test before the panic
    // phase.
    let witness_stdout = leaf.join("witness-stdout.bin");
    let witness_stderr = leaf.join("witness-stderr.bin");
    let witness_child = std::process::Command::new("/usr/bin/python3")
        .arg("-c")
        .arg(SIGNAL_SCRIPT)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("synthetic local python3 witness child must spawn");
    let witness = start_or_refuse(witness_child, &witness_stdout, &witness_stderr);
    let witness_slot: std::cell::RefCell<Option<SignalRecorded>> =
        std::cell::RefCell::new(None);
    let witness_slot_ref = &witness_slot;
    let witness_result = witness.signal_stop(SIGNAL_GUARD_SIGINT, move |capture| {
        run_signal_prefix_reads_controller(capture, witness_slot_ref);
    });
    let (witness_outcome, witness_panic) = match witness_result {
        SignalStopResult::Stopped(result) => {
            let ControllerCaptureResult {
                outcome,
                controller_panic,
            } = result;
            (outcome, controller_panic)
        }
        other => panic!(
            "the healthy witness must reach a Stopped result (the pidfd \
             preparation and the signal request are NOT exercised-refusal \
             cases here): {other:?}"
        ),
    };
    assert!(
        witness_panic.is_none(),
        "the healthy witness controller must NOT panic"
    );
    let witness_recorded = witness_slot
        .borrow_mut()
        .take()
        .expect("the healthy witness controller must record its observations");
    let (witness_exit, witness_stdout_state, witness_stderr_state) =
        match witness_outcome {
            ChildCaptureOutcome::Finished {
                exit,
                stdout_capture,
                stderr_capture,
                ..
            } => (exit, stdout_capture, stderr_capture),
            ChildCaptureOutcome::WaitRefused(handback) => {
                fail_unexpected_wait_refusal(
                    conserve_unexpected_wait_refusal_without_rewait(
                        "signal-guard healthy witness wait refused",
                        handback,
                        witness_panic,
                    ),
                );
            }
        };
    assert_signal_script_collection(
        witness_recorded,
        witness_exit,
        witness_stdout_state,
        witness_stderr_state,
        &witness_stdout,
        &witness_stderr,
    );
    // PANIC PHASE — a second real child with the SAME constant script; its
    // controller records its causal reads BEFORE the distinctive original
    // panic, and NO signal is sent by the controller itself.
    let panic_stdout = leaf.join("panic-stdout.bin");
    let panic_stderr = leaf.join("panic-stderr.bin");
    let panic_child = std::process::Command::new("/usr/bin/python3")
        .arg("-c")
        .arg(SIGNAL_SCRIPT)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("synthetic local python3 panic-phase child must spawn");
    let capture = start_or_refuse(panic_child, &panic_stdout, &panic_stderr);
    let panic_slot: std::cell::RefCell<Option<SignalRecorded>> =
        std::cell::RefCell::new(None);
    let panic_marker = SIGNAL_CONTROLLER_PANIC_MARKER;
    let panic_slot_ref = &panic_slot;
    let panic_result = capture.signal_stop(SIGNAL_GUARD_SIGINT, move |capture| {
        let stdout_prefix_read = handshake_read_exactly(
            capture.stdout().evidence_reader(),
            SIGNAL_STDOUT_PREFIX.len(),
        );
        let stderr_prefix_read = handshake_read_exactly(
            capture.stderr().evidence_reader(),
            SIGNAL_STDERR_PREFIX.len(),
        );
        let stdin_was_exposed = capture.stdin().is_some();
        panic_slot_ref.replace(Some((stdout_prefix_read, stderr_prefix_read, stdin_was_exposed)));
        panic!(
            "{panic_marker} both prefixes read before the explicit signal"
        );
    });
    // The wrapper must STILL stop the child on the panic path: the result
    // is `Stopped` again, carrying the complete real finish AND the
    // ORIGINAL payload. NOTHING is asserted before this real collection.
    let (panic_outcome, mut panic_receipt) = match panic_result {
        SignalStopResult::Stopped(result) => {
            let ControllerCaptureResult {
                outcome,
                controller_panic,
            } = result;
            (outcome, controller_panic)
        }
        other => panic!(
            "the panic phase must ALSO reach a Stopped result: the wrapper \
             must request the signal through the held pidfd on the panic \
             path too: {other:?}"
        ),
    };
    // The ORIGINAL payload is retained verbatim: the receipt is present.
    assert!(
        panic_receipt.is_some(),
        "the wrapper must retain the controller's original panic payload \
         across the signal request and the real finish"
    );
    let panic_recorded = panic_slot
        .borrow_mut()
        .take()
        .expect("the panic controller must record its reads before panicking");
    // The recorded stdout prefix read is first checked on a BORROWED view
    // (the recording itself is consumed by the shared check below).
    assert_eq!(
        panic_recorded
            .0
            .as_ref()
            .expect("causal stdout prefix read before the panic"),
        &SIGNAL_STDOUT_PREFIX.to_vec(),
        "the exact raw stdout prefix must be readable through the evidence \
         reader BEFORE the controller panic"
    );
    let (panic_exit, panic_stdout_state, panic_stderr_state) = match panic_outcome {
        ChildCaptureOutcome::Finished {
            exit,
            stdout_capture,
            stderr_capture,
            ..
        } => (exit, stdout_capture, stderr_capture),
        ChildCaptureOutcome::WaitRefused(handback) => {
            fail_unexpected_wait_refusal(
                conserve_unexpected_wait_refusal_without_rewait(
                    "signal-guard panic phase wait refused",
                    handback,
                    panic_receipt,
                ),
            );
        }
    };
    assert_signal_script_collection(
        panic_recorded,
        panic_exit,
        panic_stdout_state,
        panic_stderr_state,
        &panic_stdout,
        &panic_stderr,
    );
    // ORIGINAL payload re-emission verified via a test catch_unwind: the
    // receipt was conserved above, so the caller can judge first and
    // re-raise afterwards.
    let re_raise = panic_receipt
        .take()
        .expect("the panic receipt must still be present for the re-raise");
    let recaught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        std::panic::resume_unwind(re_raise);
    }));
    let payload = recaught.expect_err("resuming the receipt must panic again");
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "<non-string payload>".to_string());
    assert!(
        message.contains(SIGNAL_CONTROLLER_PANIC_MARKER),
        "the re-raised payload must be the ORIGINAL distinctive message: \
         {message:?}"
    );
    fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}
