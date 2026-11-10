//! LIVE raw recorded-stream reader for the PAEA approval-web child evidence
//! work: a producer worker drains a caller-owned `Read + Send` source into
//! an exclusively new evidence file, and a consumer reader receives the
//! SAME bytes already conserved BEFORE producer EOF — so the future parser
//! can observe a live child's output as it is written, not only after exit.
//! No business parser, no Child, no provider, no Git, no live DB and no
//! integration-approval test change here.
//!
//! Contract highlights (details on each item):
//! * conservation reuses the shared cores of `paea_capture_stream`
//!   (`open_new_capture_file`, `conserve_chunks`) — ONE conservation loop,
//!   no divergent copy; exact bytes, exclusive 0600 creation, prefix kept
//!   on failure, real causes preserved;
//! * a prepared-input entry lets the Child owner reserve evidence files
//!   exclusively (`LiveRecordedStream::reserve_evidence` →
//!   `ReservedEvidenceFile`) BEFORE any irreversible pipe transfer and
//!   start each stream on the real reserved descriptor
//!   (`ReservedEvidenceFile::start`) — same worker body, same shared
//!   conservation core; the direct `start(Read, path)` form keeps its
//!   signature and observable contract for its existing callers and
//!   guards and DELEGATES to `reserve_evidence` +
//!   `ReservedEvidenceFile::start`, so both start forms share ONE private
//!   reservation core (exclusive creation + consumer-descriptor
//!   preparation — no duplicated preparation);
//! * reader integrity: the consumer is bound to the REAL reserved
//!   descriptor, never to the pathname. Both start forms prepare a
//!   consumer read descriptor (`try_clone` of the exclusively opened
//!   descriptor) BEFORE any worker or irreversible transfer, and
//!   `LiveEvidenceReader` reads positionally (`read_at`, never a seek) so
//!   the shared file-description offset cannot corrupt worker writes; a
//!   rename of the evidence file or a forged same-length replacement at
//!   the original pathname cannot reach the consumer;
//! * truncation integrity: a physical EOF while the core still promises
//!   undrained conserved bytes (external truncation of the evidence file)
//!   surfaces as an explicit `UnexpectedEof` consumer error — never a
//!   premature clean EOF; the producer's historical terminal state stays
//!   verbatim in `join`;
//! * causal waiting only: the consumer blocks on a condvar signalled after
//!   each successful write and on every terminal producer state — never a
//!   timed poll, watchdog, retry or ceiling; cancelling a stuck arbitrary
//!   source belongs to the future Child owner, not here;
//! * clean consumer EOF is justified ONLY by a COMPLETED producer: once
//!   the terminal state is published and all conserved bytes are drained,
//!   a `Failed`/`Panicked` producer surfaces to the consumer as an
//!   observable read error (safe diagnostic: stage, verbatim cause,
//!   secondary flush error; a panic payload is never unpacked or moved
//!   into it) — the original outcome stays verbatim for `join`, an empty
//!   read buffer stays protocol `Ok(0)`, and an invariant without an
//!   outcome refuses loudly instead of a clean EOF;
//! * the consumer-error regression guards
//!   `live_stream_consumer_receives_source_fault_as_error_not_clean_eof`
//!   and
//!   `live_stream_consumer_receives_panicking_source_as_error_not_clean_eof`
//!   pin that consumer-error contract (witness first, then fault or panic,
//!   real outcome collected before the terminal assertions);
//! * the reader-integrity regression guards (delivered as RED
//!   counterexamples, then repaired by the reader-integrity lot — the
//!   assertions below are the repaired contract's pins):
//!   `live_stream_reserved_descriptor_survives_path_replacement` pins that
//!   the consumer follows the REAL reserved descriptor, not a reopen of
//!   the pathname after a reserved-file rename plus a forged same-length
//!   replacement, and
//!   `live_stream_truncated_evidence_refuses_premature_clean_eof` pins
//!   that a shortened evidence file against a `Completed` producer must
//!   surface as an `UnexpectedEof` error, never a premature clean EOF;
//! * the zero-prefix terminal-edge guards
//!   `live_stream_zero_prefix_failure_and_empty_buffer_preserve_error` and
//!   `live_stream_zero_prefix_panic_and_empty_buffer_preserve_payload`
//!   pin the consumer contract when the producer fails or panics BEFORE
//!   any byte: a truly empty healthy stream completes cleanly first, then
//!   the empty-buffer protocol `Ok(0)` must NOT consume the terminal
//!   error and the next nonempty read must surface it (safe diagnostic;
//!   the panic payload is never unpacked into it);
//! * a rendered byte is WRITTEN but not yet fsynced before producer EOF;
//!   the durability anchor is the producer's EOF-path `sync_all` — this
//!   stage is deliberately not the final durable proof;
//! * explicit caller closure: the caller closes its own source, then
//!   collects the producer result with `join` — an implicit drop proves
//!   nothing and no error is hidden in `let _=`.
//!
//! Guards run under the runner's explicit TMPDIR via the shared
//! `private_fixture_leaf` (never /tmp, never a system fallback), each
//! cleaning only its own leaf; fixture sources are local simulations only
//! (no Child, provider, Git, live DB, no environment mutation). Compiled
//! only inside the `paea_approval_web` integration test crate (Linux-only
//! through its crate-level `cfg`), no new dependency.

use std::collections::VecDeque;
use std::io::Read;
use std::os::unix::fs::FileExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use super::paea_capture_stream::{
    conserve_chunks, open_new_capture_file, private_fixture_leaf, StreamCaptureError,
    StreamCaptureStage,
};

/// Terminal state of the producer worker. On `Failed` the written prefix
/// is kept in the file; on `Completed` every conserved byte is flushed and
/// fsynced. `Panicked` preserves the worker's panic payload verbatim as
/// the observable error (never swallowed, never an empty success).
pub enum LiveStreamState {
    /// Producer finished successfully: exact-bytes EOF, flush and sync_all
    /// done; `bytes` is the conserved file length.
    Completed { bytes: u64 },
    /// Producer failed during conservation; `error` carries stage, verbatim
    /// cause, confirmed byte floor and best-effort flush secondary, and the
    /// file keeps the written prefix (see `StreamCaptureError`).
    Failed { error: StreamCaptureError },
    /// The worker thread panicked; its payload is preserved verbatim.
    Panicked {
        payload: Box<dyn std::any::Any + Send>,
    },
}

/// Manual `Debug` because `Box<dyn Any + Send>` has no `Debug`; the payload
/// is described by its type id, never unpacked (its content stays verbatim
/// for the caller in `join`).
impl std::fmt::Debug for LiveStreamState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiveStreamState::Completed { bytes } => {
                f.debug_struct("Completed").field("bytes", bytes).finish()
            }
            LiveStreamState::Failed { error } => {
                f.debug_struct("Failed").field("error", error).finish()
            }
            LiveStreamState::Panicked { .. } => f
                .debug_struct("Panicked")
                .field("payload", &"Box<dyn Any + Send>")
                .finish(),
        }
    }
}

/// Progress snapshot returned by [`LiveRecordedStream::wait_written`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveProgressSnapshot {
    /// Bytes confirmed written by successful write calls at observation.
    pub written: u64,
    /// Total write operations at observation.
    pub writes: u64,
    /// True once the producer reached a terminal state.
    pub terminal: bool,
}

/// Shared causal core between the producer worker and the consumer: write
/// progress under a Mutex, a Condvar for waiting (never a timed poll), the
/// terminal state published exactly once. Holds no bytes: the consumer
/// reads conserved bytes directly from the evidence file — one
/// conservation loop, no divergent buffer.
#[derive(Debug, Default)]
struct LiveCore {
    state: Mutex<LiveProgress>,
    signal: Condvar,
    terminal: Mutex<Option<LiveStreamState>>,
}

impl LiveCore {
    /// Records a successful write of `n` bytes and wakes every waiter.
    fn progressed(&self, n: usize) {
        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        state.written += n as u64;
        state.writes += 1;
        drop(state);
        self.signal.notify_all();
    }

    /// Publishes the terminal state exactly once (later calls are no-ops)
    /// and wakes every waiter. Tolerates poisoned locks.
    fn publish_terminal(&self, outcome: LiveStreamState) {
        let mut terminal = match self.terminal.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if terminal.is_none() {
            *terminal = Some(outcome);
        }
        drop(terminal);
        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        state.done = true;
        drop(state);
        self.signal.notify_all();
    }

    /// Causal wait (no timed poll): blocks while fewer than `target` bytes
    /// are confirmed written and no terminal state is published, waking on
    /// every progress or terminal signal. Returns the observed snapshot.
    /// Tolerates (recovers from) a poisoned lock via `into_inner()` — a
    /// panicked lock holder does not surface as an error here and cannot
    /// wedge the wait; the poisoned-mutex recovery is the documented
    /// behavior, not an error path.
    fn wait_for(&self, target: u64) -> LiveProgressSnapshot {
        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        while state.written < target && !state.done {
            state = match self.signal.wait(state) {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
        }
        LiveProgressSnapshot {
            written: state.written,
            writes: state.writes,
            terminal: state.done,
        }
    }

    /// Blocks until the terminal state is published (no timeout). Tolerates
    /// a poisoned lock.
    fn wait_terminal(&self) {
        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        while !state.done {
            state = match self.signal.wait(state) {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
        }
    }

    /// Takes the published terminal state, if any.
    fn take_terminal(&self) -> Option<LiveStreamState> {
        let mut terminal = match self.terminal.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        terminal.take()
    }

    /// Peeks at the published terminal state for the consumer reader
    /// WITHOUT consuming it: the original outcome keeps its ownership in
    /// the core for `join` (`take_terminal`). Only safe diagnostic copies
    /// are built: `Failed` contributes its stage, cause kind, verbatim
    /// cause and any secondary flush error; `Panicked` is reported as a
    /// panic without ever unpacking, copying or moving its payload.
    /// Returns `None` only when no terminal state is published at all.
    fn peek_consumer_outcome(&self) -> Option<ConsumerTerminalOutcome> {
        let terminal = match self.terminal.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        match terminal.as_ref() {
            Some(LiveStreamState::Completed { .. }) => {
                Some(ConsumerTerminalOutcome::CleanEof)
            }
            Some(LiveStreamState::Failed { error }) => {
                let mut message = format!(
                    "live evidence stream failed before clean EOF (stage {:?}, \
                     {} bytes conserved): {}",
                    error.stage, error.bytes_written, error.cause
                );
                if let Some(flush) = error.flush_error.as_ref() {
                    message.push_str(&format!(
                        "; secondary flush error {:?}: {flush}",
                        flush.kind()
                    ));
                }
                Some(ConsumerTerminalOutcome::Error(std::io::Error::new(
                    error.cause.kind(),
                    message,
                )))
            }
            Some(LiveStreamState::Panicked { .. }) => {
                Some(ConsumerTerminalOutcome::Error(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "live evidence producer panicked before clean EOF; its panic \
                     payload is preserved verbatim for `join` and is never unpacked \
                     into a consumer error",
                )))
            }
            None => None,
        }
    }
}

/// Ready-made, non-consuming consumer view of a published terminal state
/// (see `LiveCore::peek_consumer_outcome`): clean EOF is justified ONLY by
/// a completed producer, while any other terminal state becomes an
/// observable consumer error after the conserved prefix. The original
/// `LiveStreamState` stays owned by the core for `join`.
enum ConsumerTerminalOutcome {
    /// Clean `Ok(0)` EOF is allowed: the producer completed exactly.
    CleanEof,
    /// Observable consumer read error with a safe diagnostic (kind and
    /// message built from copies of the failure's fields; a panic payload
    /// is never unpacked or moved here).
    Error(std::io::Error),
}

#[derive(Debug, Default, Clone, Copy)]
struct LiveProgress {
    written: u64,
    writes: u64,
    done: bool,
}

/// A live raw recorded stream: the producer worker conserves the source's
/// exact bytes into the exclusive evidence file while the caller reads the
/// already-written prefix directly from that file through
/// [`LiveRecordedStream::evidence_reader`] — the file is the single source
/// of truth, so the consumer can never diverge from the conserved bytes.
///
/// Caller contract and limits: start it, read incrementally through
/// [`Self::evidence_reader`] (each read waits causally for newly written
/// bytes), then CLOSE THE CALLER'S OWN SOURCE and finish explicitly with
/// [`Self::join`] — an implicit drop proves nothing and is a documented
/// programming error, not a completion. A source that stalls forever keeps
/// the consumer waiting forever: cancelling a stuck source belongs to the
/// future Child owner, not to any timeout added here. Rendered bytes are
/// WRITTEN but not fsynced before producer EOF; durability anchors at the
/// producer's EOF-path `sync_all` (the reader never depends on that sync).
/// A consumer-side read error is a real error, never masked; the
/// conservation outcome stays in `join`'s terminal state.
pub struct LiveRecordedStream {
    core: Arc<LiveCore>,
    /// Consumer read side over the conserved file (see `evidence_reader`).
    reader: LiveEvidenceReader,
    worker: Option<JoinHandle<()>>,
}

impl LiveRecordedStream {
    /// Starts the producer worker: creates the evidence file EXCLUSIVELY
    /// (`create_new`, 0600 — a pre-existing file, including a dangling
    /// symlink, refuses BEFORE any worker or reader exists, leaving the
    /// entry byte-identical and no ambiguous start), then spawns the
    /// worker that conserves `source`'s exact bytes via the shared cores
    /// with per-write progress notification, publishing exactly one
    /// terminal state (`Completed`/`Failed`/`Panicked`).
    ///
    /// Inputs and preconditions: `source` is owned and `Read + Send`;
    /// `evidence` is a new explicit path inside the runner's private Unix
    /// area (never /tmp by policy — the caller chooses the location; the
    /// guards use the shared `private_fixture_leaf`).
    ///
    /// Delegation (ONE reservation core for BOTH start forms): this direct
    /// form is exactly `reserve_evidence(evidence)?` followed by
    /// `ReservedEvidenceFile::start(source)` — exclusive creation and
    /// consumer-descriptor preparation happen ONCE in the shared private
    /// reservation core (`reserve_with_preparation`, called with a real
    /// `File::try_clone`), then the reservation starts the worker on the
    /// real reserved descriptors. No duplicated preparation exists.
    ///
    /// Effects: creates the file (0600) and one worker thread. No socket
    /// is created. The consumer read descriptor is prepared (a `try_clone`
    /// of the created descriptor) BEFORE the worker spawns.
    ///
    /// Errors: exclusive-creation refusal returns `StreamCaptureError`
    /// (stage `Open`) with the real cause and no artifact; reader-descriptor
    /// preparation failure returns stage `ReaderPrepare` with the real
    /// cause and the created EMPTY 0600 artifact PRESERVED (never deleted,
    /// never reported as "no effect"); no worker is created in either case.
    /// Thread-spawn failure panics: fixture-scope resource exhaustion is
    /// not masked as a capture state.
    ///
    /// Limits: no timeout/retry/ceiling anywhere; the worker owns no Child
    /// and cancels nothing — a stuck source blocks until the future Child
    /// owner cancels it. To reserve the evidence file BEFORE an
    /// irreversible transfer instead, use `reserve_evidence` plus
    /// `ReservedEvidenceFile::start` (same worker body, same cores).
    pub fn start(
        source: impl Read + Send + 'static,
        evidence: &Path,
    ) -> Result<Self, StreamCaptureError> {
        let reservation = Self::reserve_evidence(evidence)?;
        Ok(reservation.start(source))
    }

    /// Exclusively reserves the evidence file (`create_new`, 0600, shared
    /// `open_new_capture_file` core) WITHOUT starting any worker: the Child
    /// owner uses this to secure BOTH evidence files BEFORE any
    /// irreversible pipe transfer, then starts each stream on the real
    /// reserved descriptor with [`ReservedEvidenceFile::start`].
    ///
    /// Inputs and preconditions: `evidence` is a new explicit path inside
    /// the runner's private Unix area (same rule as `start`).
    ///
    /// Descriptor preparation (BEFORE any worker): the reservation holds
    /// the REAL exclusively-opened descriptor and prepares its CONSUMER
    /// read side here, by `try_clone`-ing that same descriptor into a
    /// second descriptor over the SAME file description. This is the ONE
    /// shared private reservation core (`reserve_with_preparation`, called
    /// here with a real `File::try_clone`): the direct `start(Read, path)`
    /// form delegates to this same reservation, so exclusive creation and
    /// consumer-descriptor preparation exist exactly once for both start
    /// forms (no duplicated preparation). The two descriptors share the
    /// file offset (they are one open file description), which is exactly
    /// why the reader NEVER seeks: it reads positionally via `read_at`, so
    /// its reads cannot move the worker's write offset (see
    /// `LiveEvidenceReader`). A preparation failure returns
    /// `StreamCaptureError` with stage `ReaderPrepare` and the real cause:
    /// the created EMPTY 0600 artifact is PRESERVED on disk (never
    /// deleted, never reported as "no effect" — this is explicitly not
    /// stage `Open`), the reservation is dropped (descriptor closed) and no
    /// worker exists.
    ///
    /// Errors: exclusive-creation refusal returns stage `Open` with the
    /// real cause and no artifact; reader-descriptor preparation failure
    /// returns stage `ReaderPrepare` with the real cause and the preserved
    /// EMPTY artifact. In both cases no worker, no started capture and no
    /// other side effect exists. No check-exists-then-reopen: the returned
    /// reservation holds both real descriptors.
    ///
    /// Limits: the caller converts the reservation with `start` or drops
    /// it — dropping WITHOUT starting closes both descriptors and KEEPS the
    /// identified EMPTY 0600 artifact on disk (documented partial-evidence
    /// state, never deleted here); a later thread-spawn failure panics as
    /// in `start`.
    pub fn reserve_evidence(
        evidence: &Path,
    ) -> Result<ReservedEvidenceFile, StreamCaptureError> {
        Self::reserve_with_preparation(evidence, std::fs::File::try_clone)
    }

    /// Private reservation core shared by BOTH public start forms (the
    /// direct `start(Read, path)` delegates to `reserve_evidence`, which
    /// calls this with a real `File::try_clone`): exclusively creates the
    /// evidence file via the shared `open_new_capture_file` core, then
    /// prepares the consumer read side with `prepare_reader` applied to
    /// the REAL reserved write descriptor, and returns the not-yet-started
    /// reservation holding BOTH descriptors over the SAME file
    /// description. ONE exclusive-creation and ONE preparation step for
    /// both start forms — no duplicated preparation.
    ///
    /// Preconditions: `evidence` is a new explicit path (same rule as
    /// `reserve_evidence`); `prepare_reader` is `FnOnce(&File) ->
    /// io::Result<File>` receiving the REAL exclusively-opened write
    /// descriptor and returning the prepared consumer descriptor over the
    /// SAME file description. The only intended non-`File::try_clone`
    /// preparation is a deterministic failure injection used by the
    /// test-crate guards of this module (guard
    /// `live_stream_reader_prepare_refusal_keeps_empty_artifact_then_recovers_fresh`);
    /// there is no second conservation loop and no public API expansion
    /// beyond the existing two entry points.
    ///
    /// Effects: creates the 0600 file (unless the creation refuses) and,
    /// on success, holds both descriptors in the returned reservation; no
    /// worker, no reader and no other side effect exists here.
    ///
    /// Errors: exclusive-creation refusal returns stage `Open` with the
    /// real cause, zero confirmed bytes and no flush error — no new file
    /// is created and no reservation is held; existing entries are unchanged; a `prepare_reader` failure
    /// returns stage `ReaderPrepare` with the REAL cause preserved
    /// verbatim (kind and message), zero confirmed bytes and no flush
    /// error — the created EMPTY 0600 artifact is PRESERVED on disk at the
    /// caller-provided path (never deleted, never reported as "no
    /// effect") and the write descriptor is dropped (closed); no
    /// reservation escapes in either case.
    fn reserve_with_preparation(
        evidence: &Path,
        prepare_reader: impl FnOnce(&std::fs::File) -> std::io::Result<std::fs::File>,
    ) -> Result<ReservedEvidenceFile, StreamCaptureError> {
        let file = open_new_capture_file(evidence)?;
        // Consumer-side preparation on the REAL reserved descriptor, still
        // BEFORE any worker: the prepared descriptor is stored and the
        // original stays the worker's write side.
        let reader_file = prepare_reader(&file).map_err(|cause| StreamCaptureError {
            stage: StreamCaptureStage::ReaderPrepare,
            cause,
            bytes_written: 0,
            flush_error: None,
        })?;
        Ok(ReservedEvidenceFile {
            file: Some(file),
            reader_file: Some(reader_file),
            path: evidence.to_path_buf(),
        })
    }

    /// Shared worker-spawning core of BOTH start forms (direct
    /// `start(Read, path)` and [`ReservedEvidenceFile::start`]): consumes
    /// the REAL reserved write descriptor — no reopen, no second
    /// conservation loop — and the pre-prepared consumer read descriptor
    /// (the `try_clone` made before any worker, see `reserve_evidence`),
    /// then spawns the worker that conserves `source`'s exact bytes via
    /// the shared cores with per-write progress notification, publishing
    /// exactly one terminal state (`Completed`/`Failed`/`Panicked`). No
    /// fallible step remains here: descriptor preparation already
    /// succeeded in the callers.
    ///
    /// Effects: one worker thread. Errors: thread-spawn failure panics
    /// (fixture-scope resource exhaustion is not masked as a capture
    /// state) — documented limit also carried by the callers.
    fn start_on_reserved(
        file: std::fs::File,
        reader_file: std::fs::File,
        source: impl Read + Send + 'static,
    ) -> Self {
        let core = Arc::new(LiveCore::default());
        let worker_core = Arc::clone(&core);
        let worker = std::thread::spawn(move || {
            let mut file = file;
            let mut source = source;
            // The whole worker body is wrapped so that ANY panic still
            // publishes `Panicked` and wakes every waiter (never an
            // eternal wait on a finished producer).
            let outcome = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // The conservation contract is `FnMut` (notification after
                // each successful write), so the binding is used mutably
                // and must be declared as such.
                let mut notify = |chunk: &[u8]| worker_core.progressed(chunk.len());
                match conserve_chunks(&mut source, &mut file, &mut notify) {
                    Ok(bytes) => LiveStreamState::Completed { bytes },
                    Err(error) => LiveStreamState::Failed { error },
                }
            })) {
                Ok(outcome) => outcome,
                Err(payload) => LiveStreamState::Panicked { payload },
            };
            worker_core.publish_terminal(outcome);
        });
        Self {
            reader: LiveEvidenceReader::from_held_descriptor(Arc::clone(&core), reader_file),
            core,
            worker: Some(worker),
        }
    }

    /// Consumer read side over the conserved file: receives the SAME bytes
    /// already conserved (exact, no normalization) — bytes become readable
    /// as soon as the producer's write succeeded, BEFORE producer EOF.
    /// Each read waits causally for availability (condvar), never by timed
    /// polling.
    ///
    /// Limit: this reader's clean EOF alone proves nothing by itself —
    /// admission of EOF is `join`'s terminal state, and a clean EOF is
    /// justified ONLY by a `Completed` producer (`Failed`/`Panicked`
    /// surface as read errors instead, see `LiveEvidenceReader::read`).
    /// In practice the terminal state is published BEFORE the consumer can
    /// observe a bare physical EOF (the producer's own EOF-then-flush path
    /// publishes it first), so the reader's API EOF is already guarded by
    /// the producer terminal; a PHYSICAL EOF against confirmed-but-undrained
    /// bytes — only reachable through external same-inode truncation of the
    /// evidence file — is a distinct error (`UnexpectedEof`), not a clean
    /// EOF. Same-size in-place content tampering through another descriptor
    /// is NOT authenticated (see `LiveEvidenceReader` limits).
    pub fn evidence_reader(&mut self) -> &mut LiveEvidenceReader {
        &mut self.reader
    }

    /// Causally waits (condvar, no timed poll) until at least `target`
    /// bytes are confirmed written or the producer reaches a terminal
    /// state, then returns the observed snapshot. The terminal state's
    /// kind is delivered by `join`.
    pub fn wait_written(&self, target: u64) -> LiveProgressSnapshot {
        self.core.wait_for(target)
    }

    /// Explicit closure: the caller must have closed its own source first,
    /// then call this ONCE to collect the producer's real outcome. Waits
    /// causally for the terminal publication (no timeout), joins the
    /// worker without swallowing a late panic (mapped to
    /// `LiveStreamState::Panicked`), and returns the terminal state.
    ///
    /// Errors: panics if called twice (documented misuse, not a hidden
    /// no-op), or if the worker exited cleanly without publishing a
    /// terminal state (impossible by construction; explicit invariant
    /// refusal instead of an empty success).
    pub fn join(mut self) -> LiveStreamState {
        let worker = self
            .worker
            .take()
            .expect("join called twice on LiveRecordedStream");
        self.core.wait_terminal();
        let published = self.core.take_terminal();
        match worker.join() {
            Ok(()) => match published {
                Some(state) => state,
                None => panic!(
                    "worker joined cleanly but published no terminal state (invariant refusal)"
                ),
            },
            Err(payload) => LiveStreamState::Panicked { payload },
        }
    }
}

/// An evidence file exclusively reserved (`create_new`, 0600) but NOT yet
/// started: the Child owner's intermediate state between reservation and
/// stream start. It holds BOTH real descriptors over the SAME file
/// description — the write side (consumed by `start` as the worker's
/// conservation target) and the pre-prepared consumer read side (the
/// `try_clone` made at reservation time, BEFORE any worker or irreversible
/// pipe transfer). No check-exists-then-reopen exists anywhere: `start`
/// consumes the real descriptors directly, no second conservation loop.
///
/// Limits: dropping a reservation WITHOUT starting it closes BOTH
/// descriptors and deliberately KEEPS the identified EMPTY 0600 artifact on
/// disk (the documented partial-evidence state after a later-step refusal;
/// this type never deletes it and never reports a fake success or a
/// started capture).
pub struct ReservedEvidenceFile {
    file: Option<std::fs::File>,
    reader_file: Option<std::fs::File>,
    path: PathBuf,
}

impl ReservedEvidenceFile {
    /// Evidence path the reservation was created for (the identified
    /// artifact a later refusal must conserve, never delete).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Starts the producer worker on the REAL reserved descriptor — the
    /// exclusively opened file is consumed directly as the write side, the
    /// reservation's pre-prepared consumer descriptor becomes the reader
    /// (both bound to the SAME file description), with the same worker
    /// body, shared conservation cores and terminal-state contract as
    /// [`LiveRecordedStream::start`]. No descriptor preparation happens
    /// here: it already succeeded at reservation time, BEFORE any
    /// irreversible pipe transfer.
    ///
    /// Effects: consumes the reservation and spawns one worker thread.
    ///
    /// Errors: thread-spawn failure panics (documented fixture-scope
    /// limit, same as `start`); the exclusive-creation and
    /// reader-preparation refusals happened at reservation time and cannot
    /// occur here.
    ///
    /// Misuse: calling twice panics — the descriptors are consumed by the
    /// first call (same documented-misuse class as
    /// `LiveRecordedStream::join`).
    pub fn start(mut self, source: impl Read + Send + 'static) -> LiveRecordedStream {
        let file = self
            .file
            .take()
            .expect("reserved evidence file already started (documented misuse)");
        let reader_file = self
            .reader_file
            .take()
            .expect("reserved consumer descriptor already started (documented misuse)");
        LiveRecordedStream::start_on_reserved(file, reader_file, source)
    }
}

impl Drop for ReservedEvidenceFile {
    /// Closes BOTH held descriptors (write and prepared consumer side)
    /// when the reservation ends without a start; the identified EMPTY
    /// 0600 artifact stays on disk (never deleted here — conserving or
    /// removing it is the caller's explicit evidence decision).
    fn drop(&mut self) {
        self.file.take();
        self.reader_file.take();
    }
}

/// Consumer-side reader over the conserved evidence file: yields the exact
/// bytes already written by the producer (no normalization), waiting
/// causally on the shared core for bytes beyond the last observed length —
/// never a timed poll. Clean EOF is justified ONLY by a COMPLETED
/// producer: after all conserved bytes are drained, `Failed`/`Panicked`
/// surface as observable read errors (safe diagnostic; the original
/// outcome stays owned by the core for `join`).
///
/// Descriptor binding (reader integrity): the reader NEVER opens the
/// evidence pathname. It is constructed from a descriptor `try_clone`d
/// from the REAL reserved descriptor BEFORE any worker or irreversible
/// transfer, so it is bound to the SAME actual file (inode) the worker
/// conserves into — a rename of the evidence file or a same-size forged
/// replacement planted at the original pathname is invisible to it. The
/// clone shares the file description's offset with the worker's write
/// descriptor, which is why every consumer read is POSITIONAL
/// (`std::os::unix::fs::FileExt::read_at` at this reader's own `offset`):
/// the reader never seeks, so its reads cannot move the worker's write
/// offset or observe it.
///
/// Limits: same-size in-place external tampering with the file CONTENT
/// (overwriting bytes through another descriptor at the same inode) is NOT
/// authenticated or detected — the reader conserves exact bytes, it is not
/// a hash chain; any stronger tamper evidence belongs to a later owner.
/// Thread-spawn failure at stream start panics (fixture-scope resource
/// exhaustion, never masked as a capture state); a caller that drops the
/// stream without `join` strands the worker by documented contract.
pub struct LiveEvidenceReader {
    core: Arc<LiveCore>,
    file: std::fs::File,
    offset: u64,
}

impl LiveEvidenceReader {
    /// Builds the consumer from the HELD descriptor prepared before the
    /// worker (no pathname open, no check-exists-then-reopen). Panics
    /// never: the descriptor arrived already prepared — the ONE shared
    /// private reservation core (`reserve_with_preparation`, used by both
    /// public constructors `start` and `reserve_evidence` + `start`)
    /// refuses with stage `ReaderPrepare` BEFORE any worker when the
    /// consumer-descriptor preparation fails (a real `File::try_clone`
    /// for both public constructors), so this call is unfallible by
    /// construction.
    fn from_held_descriptor(core: Arc<LiveCore>, file: std::fs::File) -> Self {
        Self { core, file, offset: 0 }
    }
}

impl Read for LiveEvidenceReader {
    /// Reads newly available conserved bytes, waiting causally when the
    /// producer has written nothing beyond `offset` and has not yet
    /// reached a terminal state; an empty read buffer stays protocol
    /// `Ok(0)`. Once the producer is terminal and every conserved byte
    /// has been delivered: `Completed` yields the clean `Ok(0)` EOF,
    /// while `Failed`/`Panicked` yield a real `Err` (never an empty
    /// success) whose diagnostic carries stage, verbatim cause and any
    /// secondary flush error — a panic payload is never unpacked here and
    /// stays verbatim for `join`. A terminal publication without an
    /// outcome refuses loudly instead of a clean EOF. Real underlying
    /// file read errors are returned verbatim.
    ///
    /// Physical EOF is distinct from the terminal-guarded API EOF: if the
    /// core confirms more written bytes than this reader's offset but the
    /// positional read returns 0 (the evidence file was externally
    /// truncated after the producer confirmed its writes), the read
    /// returns an explicit `UnexpectedEof` error — never a premature
    /// clean EOF, never a retry and never fabricated bytes. The
    /// producer's own terminal state is historical: it stays verbatim in
    /// `join` and is never rewritten to match the shortened file.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        loop {
            let snapshot = self.core.wait_for(self.offset + 1);
            if snapshot.written > self.offset {
                // Positional read at the reader's own offset: never a
                // seek (the descriptor shares the worker's file offset —
                // a seek here would corrupt or race worker writes).
                let max = (snapshot.written - self.offset).min(buf.len() as u64) as usize;
                let n = self.file.read_at(&mut buf[..max], self.offset)?;
                if n == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        format!(
                            "live evidence reader reached a physical EOF at offset {} \
                             while the core still promises {} written byte(s): the \
                             evidence file was externally truncated after the \
                             producer confirmed its writes; no clean EOF is \
                             fabricated and the producer's historical outcome is \
                             preserved for `join`",
                            self.offset, snapshot.written
                        ),
                    ));
                }
                self.offset += n as u64;
                return Ok(n);
            }
            if snapshot.terminal {
                return match self.core.peek_consumer_outcome() {
                    Some(ConsumerTerminalOutcome::CleanEof) => Ok(0),
                    Some(ConsumerTerminalOutcome::Error(cause)) => Err(cause),
                    None => panic!(
                        "terminal progress published without a terminal outcome; \
                         refusing a clean EOF (read after join is documented misuse)"
                    ),
                };
            }
        }
    }
}

// ---- Guards: local simulated sources only -------------------------------

/// First guard: deterministic handshake demonstrating that the FIRST exact
/// fragment is readable and conserved while the producer has NOT yet
/// received the EOF permission, then exact continuation, EOF and close.
/// The fixture source waits causally on a condvar script (no timed poll)
/// and always exits when the guard releases it.
#[test]
fn live_stream_renders_first_fragment_before_producer_eof() {
    let leaf = private_fixture_leaf("live-first-fragment");
    let evidence = leaf.join("live-capture.bin");
    let script = Arc::new(LiveScript::default());
    let first: Vec<u8> = b"FIRST-EXACT-FRAGMENT\r\nwith\0nul\xFFbytes".to_vec();
    let second: Vec<u8> = b"SECOND\r\nTAIL".to_vec();
    let script_for_source = Arc::clone(&script);
    // The body runs under catch_unwind so that a panicking assertion can
    // never leave the fixture worker blocked forever on the script
    // condvar: the release below runs in ALL cases and the original panic
    // is resumed afterwards, never masked.
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let source = LiveScriptSource {
            script: script_for_source,
        };
        let mut live = LiveRecordedStream::start(source, &evidence).expect("live stream start");
        script.push(first.clone());
        // The producer must render (write + notify) the first fragment
        // while the EOF permission is still held by the guard: the core
        // "before EOF" witness. Causal wait, no sleeping.
        let snapshot = live.wait_written(first.len() as u64);
        assert!(
            snapshot.written >= first.len() as u64,
            "first fragment must be rendered before producer EOF: {snapshot:?}"
        );
        assert!(
            snapshot.writes >= 1,
            "at least one write notification must have fired: {snapshot:?}"
        );
        // The consumer reads the same exact bytes from the conserved file
        // BEFORE producer EOF (no normalization, no wait for completion).
        let reader = live.evidence_reader();
        let mut seen = vec![0u8; first.len()];
        reader
            .read_exact(&mut seen)
            .expect("consumer read before producer EOF");
        assert_eq!(seen, first, "consumer must receive the exact first fragment");
        // Release the producer: second fragment, then the EOF permission.
        script.push(second.clone());
        script.permit_eof();
        let mut rest = Vec::new();
        reader
            .read_to_end(&mut rest)
            .expect("consumer read until producer EOF");
        assert_eq!(rest, second, "continuation must be exact, then EOF");
        // Explicit closure: join and verify completion.
        match live.join() {
            LiveStreamState::Completed { bytes } => {
                assert_eq!(
                    bytes,
                    (first.len() + second.len()) as u64,
                    "completed byte count"
                );
            }
            other => panic!("producer must complete cleanly, got {other:?}"),
        }
        // The evidence file holds the exact full stream (now synced at EOF).
        let mut expected = first.clone();
        expected.extend_from_slice(&second);
        assert_eq!(
            std::fs::read(&evidence).expect("evidence read"),
            expected,
            "conserved bytes must be exact"
        );
    }));
    // Release the fixture source unconditionally: a worker still waiting
    // for fragments or the EOF permission always reaches EOF and exits,
    // even when an assertion panicked above. On the panic path the leaf is
    // deliberately left in place for diagnosis before resuming.
    script.permit_eof();
    match body {
        Ok(()) => {
            std::fs::remove_dir_all(&leaf).expect("own leaf cleanup");
        }
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Second guard: a source failing after a real prefix. The terminal state
/// is `Failed` with the real verbatim cause, the waiter is released (no
/// eternal wait on a finished producer), the file keeps the exact written
/// prefix, and recovery on a NEW file with a healthy source succeeds
/// exactly. Precondition corrected: the healthy witness (same leaf, same
/// payload, full consumer contract via the shared helper) now runs BEFORE
/// the fault; the fresh recovery AFTER the fault is kept unchanged as a
/// second control — neither the separate healthy test nor that recovery
/// substitutes for the before-injection witness.
#[test]
fn live_stream_reports_source_fault_and_keeps_prefix_then_recovers_fresh() {
    let leaf = private_fixture_leaf("live-faulty-reader");
    let payload: Vec<u8> = (0..1000u32).map(|i| (i % 253) as u8).collect();
    // VALID WITNESS BEFORE THE FAULT: a healthy source over the SAME leaf
    // and payload must first complete the full consumer contract, so the
    // later failure can never be blamed on the fixture, the leaf or the
    // payload.
    let witness = leaf.join("fault-witness.bin");
    assert_healthy_consumer_witness(&witness, &payload);
    let evidence = leaf.join("failed-live.bin");
    let live = LiveRecordedStream::start(
        FailingLiveSource {
            data: payload.clone(),
            served: 0,
            fail_after: 700,
        },
        &evidence,
    )
    .expect("live stream start");
    // Causal wait for the fault to surface as a terminal state.
    let snapshot = live.wait_written(u64::MAX);
    assert!(snapshot.terminal, "fault must surface as a terminal state");
    match live.join() {
        LiveStreamState::Failed { error } => {
            assert_eq!(error.stage, StreamCaptureStage::Read, "{error:?}");
            assert_eq!(
                error.cause.kind(),
                std::io::ErrorKind::BrokenPipe,
                "{error:?}"
            );
            assert!(
                error.cause.to_string().contains("verifiable live prefix"),
                "real cause must be preserved verbatim: {error:?}"
            );
            assert_eq!(error.bytes_written, 700, "confirmed prefix: {error:?}");
            assert!(error.flush_error.is_none(), "{error:?}");
        }
        other => panic!("source fault must surface as Failed, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(&evidence).expect("evidence read"),
        &payload[..700],
        "written prefix must be kept verbatim"
    );
    // Recovery on a NEW file with a healthy source over the same data.
    let recovered = leaf.join("recovered-live.bin");
    let healthy =
        LiveRecordedStream::start(std::io::Cursor::new(payload.clone()), &recovered)
            .expect("recovery start");
    // No before-EOF semantics here: wait deterministically for the
    // producer's terminal state, then verify it through join.
    let snapshot = healthy.wait_written(u64::MAX);
    assert!(
        snapshot.terminal && snapshot.written == payload.len() as u64,
        "{snapshot:?}"
    );
    match healthy.join() {
        LiveStreamState::Completed { bytes } => assert_eq!(bytes, payload.len() as u64),
        other => panic!("recovery must complete, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(&recovered).expect("read back"),
        payload,
        "fresh recovery conserves exact bytes"
    );
    std::fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

/// Shared healthy consumer contract over one evidence path (used as the
/// before-fault witness by the consumer-error guards and as the corrected
/// precondition of the existing faulty-reader guard): a full healthy
/// stream must give the consumer the EXACT payload then a clean EOF, and
/// `join` must report `Completed` with the exact byte count. The real
/// `join` outcome is collected BEFORE any assertion so a failing
/// assertion never loses the worker. Panicking on any deviation, this
/// asserts the healthy precondition — it is NOT the fault path.
fn assert_healthy_consumer_witness(evidence: &Path, payload: &[u8]) {
    let mut stream =
        LiveRecordedStream::start(std::io::Cursor::new(payload.to_vec()), evidence)
            .expect("healthy witness start");
    let snapshot = stream.wait_written(u64::MAX);
    let mut reader = stream.evidence_reader();
    let mut seen = Vec::new();
    let read_result = reader.read_to_end(&mut seen);
    // The real outcome is collected BEFORE any assertion: a failing
    // assertion must never lose the worker (no unjoined producer left).
    let outcome = stream.join();
    assert!(
        read_result.is_ok(),
        "healthy witness consumer read must succeed to clean EOF: {read_result:?}"
    );
    assert!(
        snapshot.terminal && snapshot.written == payload.len() as u64,
        "healthy witness must conserve the whole payload first: {snapshot:?}"
    );
    assert_eq!(
        seen, payload,
        "healthy witness consumer must receive the exact payload"
    );
    match outcome {
        LiveStreamState::Completed { bytes } => {
            assert_eq!(bytes, payload.len() as u64, "{bytes}");
        }
        other => panic!("healthy witness must complete, got {other:?}"),
    }
}

/// Consumer-error guard: a source failing after a real conserved prefix
/// must surface the failure to the CONSUMER as an observable `Err` after
/// the exact prefix, never a clean `Ok(0)` EOF, while the producer's real
/// outcome stays verbatim for `join`.
///
/// Order: HEALTHY WITNESS FIRST on the same leaf/payload, then the local
/// fault; the real producer issue is collected (`join`) BEFORE the final
/// terminal assertions, and the fixture worker is never stranded if an
/// assertion panics (source closes on drop; the writer's file is owned by
/// the worker, no script condvar to release).
#[test]
fn live_stream_consumer_receives_source_fault_as_error_not_clean_eof() {
    let leaf = private_fixture_leaf("live-fault-consumer-eof");
    let payload: Vec<u8> = (0..1000u32).map(|i| (i % 253) as u8).collect();
    let prefix = 700;
    let witness = leaf.join("fault-witness-consumer.bin");
    // VALID WITNESS BEFORE THE FAULT: same leaf, same payload, healthy
    // source — full consumer contract (exact bytes + clean EOF +
    // Completed), so the later fault is attributable to the failing
    // source, not the fixture.
    assert_healthy_consumer_witness(&witness, &payload);
    let evidence = leaf.join("failed-consumer-eof.bin");
    // Local fault after a known conserved prefix, same fixture class as
    // the existing faulty-reader guard.
    let mut stream = LiveRecordedStream::start(
        FailingLiveSource {
            data: payload.clone(),
            served: 0,
            fail_after: prefix,
        },
        &evidence,
    )
    .expect("faulty stream start");
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Drain the real conserved prefix through the consumer.
        let reader = stream.evidence_reader();
        let mut seen = Vec::new();
        let read_result = reader.read_to_end(&mut seen);
        // COLLECT the real producer issue BEFORE any terminal assertion.
        let outcome = stream.join();
        match &outcome {
            LiveStreamState::Failed { error } => {
                assert_eq!(error.stage, StreamCaptureStage::Read, "{error:?}");
                assert_eq!(
                    error.cause.kind(),
                    std::io::ErrorKind::BrokenPipe,
                    "{error:?}"
                );
                assert!(
                    error
                        .cause
                        .to_string()
                        .contains("verifiable live prefix"),
                    "real cause must be preserved verbatim: {error:?}"
                );
                assert_eq!(
                    error.bytes_written, prefix as u64,
                    "confirmed prefix: {error:?}"
                );
                assert!(error.flush_error.is_none(), "{error:?}");
            }
            other => panic!("source fault must surface as Failed, got {other:?}"),
        }
        // Prefix conserved exactly.
        assert_eq!(
            seen, &payload[..prefix],
            "consumer must have received the exact conserved prefix"
        );
        assert_eq!(
            std::fs::read(&evidence).expect("evidence read"),
            &payload[..prefix],
            "written prefix must be kept verbatim in the file"
        );
        // CONTRACT PIN: the consumer read over the faulty source must have
        // returned Err after the exact prefix, never a clean success, and
        // that observable error must be a REAL diagnostic of the producer
        // failure — the cause's kind, the failure stage, the conserved byte
        // count and the verbatim cause, with no invented secondary flush
        // error (none occurred). The join outcome checked above is the
        // original producer cause these diagnostic copies come from.
        let read_error = match &read_result {
            Err(cause) => cause,
            Ok(seen) => panic!(
                "consumer got a clean read success {seen:?} over a FAILED \
                 source: the reader must surface the producer failure as an \
                 observable error after the conserved prefix, never a clean EOF"
            ),
        };
        assert_eq!(
            read_error.kind(),
            std::io::ErrorKind::BrokenPipe,
            "consumer diagnostic must carry the producer cause's kind: {read_error}"
        );
        let diagnostic = read_error.to_string();
        assert!(
            diagnostic.contains("stage Read"),
            "consumer diagnostic must name the producer failure stage Read: \
             {diagnostic}"
        );
        assert!(
            diagnostic.contains("700 bytes conserved"),
            "consumer diagnostic must carry the confirmed conserved byte \
             count: {diagnostic}"
        );
        assert!(
            diagnostic.contains("verifiable live prefix"),
            "consumer diagnostic must preserve the producer cause verbatim: \
             {diagnostic}"
        );
        assert!(
            !diagnostic.contains("secondary flush error"),
            "no secondary flush error occurred, so the diagnostic must not \
             claim one: {diagnostic}"
        );
    }));
    match body {
        Ok(()) => std::fs::remove_dir_all(&leaf).expect("own leaf cleanup"),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Consumer-error guard: a locally panicking source (after a known
/// conserved prefix) must wake the consumer with an observable error,
/// never a clean EOF; the real panic payload stays verbatim for `join`
/// and is checked for presence, never invented, never reduced to success.
///
/// Order: HEALTHY WITNESS BEFORE INJECTION, then the panic injection; the
/// real panic payload is collected via `join` and checked for presence
/// (never invented, never reduced to success) BEFORE the terminal
/// assertions; the fixture source closes on drop, so an assertion panic
/// never leaves an open fixture source or a stuck worker (no new
/// timeout/polling/retry/ceiling).
#[test]
fn live_stream_consumer_receives_panicking_source_as_error_not_clean_eof() {
    let leaf = private_fixture_leaf("live-panic-consumer-eof");
    let payload: Vec<u8> = (0..1000u32).map(|i| (i % 253) as u8).collect();
    let prefix = 700;
    let witness = leaf.join("panic-witness-consumer.bin");
    // VALID WITNESS BEFORE THE INJECTION: same leaf, same payload, healthy
    // source — full consumer contract first.
    assert_healthy_consumer_witness(&witness, &payload);
    // Locally panicking fixture source: delivers a real known prefix, then
    // panics with a distinctive payload (never invented downstream).
    struct PanickingLiveSource {
        data: Vec<u8>,
        served: usize,
        panic_after: usize,
    }
    impl Read for PanickingLiveSource {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.served >= self.panic_after {
                panic!("fixture source panicked after its verifiable live prefix");
            }
            let n = buf.len().min(self.panic_after - self.served);
            buf[..n].copy_from_slice(&self.data[self.served..self.served + n]);
            self.served += n;
            Ok(n)
        }
    }
    let evidence = leaf.join("panicked-consumer-eof.bin");
    let mut stream = LiveRecordedStream::start(
        PanickingLiveSource {
            data: payload.clone(),
            served: 0,
            panic_after: prefix,
        },
        &evidence,
    )
    .expect("panicking stream start");
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Drain the real conserved prefix through the consumer.
        let reader = stream.evidence_reader();
        let mut seen = Vec::new();
        let read_result = reader.read_to_end(&mut seen);
        // COLLECT the real producer outcome BEFORE any terminal assertion.
        let outcome = stream.join();
        // Real panic payload: presence checked, never invented and never
        // reduced to a success.
        let payload_message = match &outcome {
            LiveStreamState::Panicked { payload } => {
                let message = payload
                    .downcast_ref::<&'static str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
                assert!(
                    message
                        .is_some_and(|message| message
                            .contains("verifiable live prefix")),
                    "real panic payload must be preserved and identifiable: {outcome:?}"
                );
                message.unwrap()
            }
            other => panic!(
                "source panic must surface as Panicked, got {other:?}"
            ),
        };
        assert!(
            payload_message.contains("verifiable live prefix"),
            "panic payload must be the fixture's real message: {payload_message}"
        );
        // Prefix conserved exactly (consumer view and file view).
        assert_eq!(
            seen, &payload[..prefix],
            "consumer must have received the exact conserved prefix before the panic"
        );
        assert_eq!(
            std::fs::read(&evidence).expect("evidence read"),
            &payload[..prefix],
            "written prefix must be kept verbatim before the panic"
        );
        // CONTRACT PIN: the consumer read over the panicking source must
        // have returned Err after the exact prefix, never a clean success,
        // and that error must be the SAFE panic diagnostic: kind Other
        // (nothing else is knowable about a panic), the explicit
        // preserved-for-join wording, and NEVER any fragment of the real
        // payload text (the payload is never unpacked into a consumer
        // error; the join outcome above retains it verbatim).
        let read_error = match &read_result {
            Err(cause) => cause,
            Ok(seen) => panic!(
                "consumer got a clean read success {seen:?} over a PANICKED \
                 source: the reader must surface the producer panic as an \
                 observable error after the conserved prefix, never a clean EOF"
            ),
        };
        assert_eq!(
            read_error.kind(),
            std::io::ErrorKind::Other,
            "panic diagnostic must surface as Other (payload never unpacked): \
             {read_error}"
        );
        let diagnostic = read_error.to_string();
        assert!(
            diagnostic.contains("producer panicked before clean EOF"),
            "consumer diagnostic must name the producer panic explicitly: \
             {diagnostic}"
        );
        assert!(
            diagnostic.contains("preserved verbatim for `join`"),
            "consumer diagnostic must point at the preserved payload in \
             `join`: {diagnostic}"
        );
        assert!(
            !diagnostic.contains("verifiable live prefix"),
            "the real panic payload text must NEVER be unpacked into the \
             consumer diagnostic (it belongs to `join` alone): {diagnostic}"
        );
    }));
    match body {
        Ok(()) => std::fs::remove_dir_all(&leaf).expect("own leaf cleanup"),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Reader-integrity regression pin: a consumer over a RESERVED evidence
/// descriptor must read the conserved INODE, never reopen the pathname.
/// The guard reserves the evidence file, MOVES the reserved (still empty)
/// inode to a retained name BEFORE `reservation.start`, plants a forged
/// SAME-LENGTH file at the original pathname, then starts a finite
/// `Cursor` source carrying the expected original bytes (NUL, non-UTF-8,
/// CRLF). The producer conserves into the REAL reserved descriptor — the
/// moved inode — while the pathname holds only the forged decoy, so the
/// reader (built from the descriptor prepared before the worker) must
/// deliver the original bytes to the consumer. This was RED against the
/// former path-reopening reader; it now pins the repaired contract, and
/// no assertion may be weakened. The rename MUST happen before the start:
/// once `start` ran, the reader already holds its own descriptor and a
/// later rename would not exercise the defect.
/// Discipline: healthy witness FIRST on the same leaf/payload (full
/// consumer contract), then the injection; the real outcomes (consumer
/// drain, then the ACTUAL producer join) are collected BEFORE any verdict
/// assertion; the retained inode and the forged replacement are both
/// inspected; the leaf is removed only on success and kept on failure
/// (original failure artifacts preserved). Synthetic `Cursor` source only
/// — no Child, no provider, no live data.
#[test]
fn live_stream_reserved_descriptor_survives_path_replacement() {
    let leaf = private_fixture_leaf("live-reserved-descriptor");
    let original: Vec<u8> = b"RESERVED-ORIGINAL\r\npayload\0\xFF".to_vec();
    // HEALTHY WITNESS BEFORE THE INJECTION: same leaf, same payload, plain
    // start — the full consumer contract must hold first, so the later
    // failure is attributable to the path replacement, not the fixture.
    let witness = leaf.join("reserved-witness.bin");
    assert_healthy_consumer_witness(&witness, &original);
    // Injection: reserve, move the reserved inode away, forge a same-length
    // replacement at the original pathname, THEN start on the reservation.
    let reserved_path = leaf.join("reserved-evidence.bin");
    let retained = leaf.join("retained-original.raw");
    let reservation = LiveRecordedStream::reserve_evidence(&reserved_path)
        .expect("evidence reservation");
    std::fs::rename(&reserved_path, &retained)
        .expect("reserved inode must move to its retained name");
    assert!(
        !reserved_path.exists(),
        "fixture: the original pathname must be free after the rename"
    );
    let mut forged: Vec<u8> = Vec::with_capacity(original.len());
    while forged.len() < original.len() {
        forged.extend_from_slice(b"FORGED-PATH-SWAP");
    }
    forged.truncate(original.len());
    assert_ne!(forged, original, "fixture: replacement must differ");
    assert_eq!(
        forged.len(),
        original.len(),
        "fixture: replacement must have the same length"
    );
    std::fs::write(&reserved_path, &forged).expect("forged replacement write");
    let mut stream = reservation.start(std::io::Cursor::new(original.clone()));
    // REAL collection BEFORE any verdict assertion: the consumer drain,
    // then the actual producer join.
    let mut reader = stream.evidence_reader();
    let mut seen = Vec::new();
    let read_result = reader.read_to_end(&mut seen);
    let outcome = stream.join();
    // Verdict pins.
    assert!(
        read_result.is_ok(),
        "consumer read must itself succeed; integrity is judged on content: \
         {read_result:?}"
    );
    // THE COUNTEREXAMPLE PIN (RED with the current path-reopening reader):
    // the consumer must receive the conserved inode's bytes — exactly the
    // source payload — whatever happened to the pathname.
    assert_eq!(
        seen, original,
        "the consumer must read the conserved inode (the real reserved \
         descriptor), not the bytes now sitting at the original pathname; \
         a reader bound to the path instead of the reserved descriptor \
         delivers the forged replacement here (seen {seen:?})"
    );
    // The producer conserved into the MOVED inode through the real
    // reserved descriptor.
    assert_eq!(
        std::fs::read(&retained).expect("retained inode read"),
        original,
        "the conserved inode (retained name) must hold the exact source bytes"
    );
    // The forged replacement was never conserved over.
    assert_eq!(
        std::fs::read(&reserved_path).expect("forged replacement read"),
        forged,
        "the forged replacement at the original pathname must keep its bytes"
    );
    // The producer's own outcome: clean completion with the exact count.
    match outcome {
        LiveStreamState::Completed { bytes } => {
            assert_eq!(bytes, original.len() as u64, "completed byte count");
        }
        other => panic!("producer must complete cleanly, got {other:?}"),
    }
    std::fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

/// Reader-integrity regression pin: after the producer has published its
/// terminal `Completed` state, a clean consumer EOF is justified ONLY when
/// every promised conserved byte has actually been delivered from the
/// evidence file — a physical EOF on a shortened file must surface as a
/// real error, never as a premature `Ok(0)` success. The guard starts a
/// finite `Cursor` source, waits the terminal publication
/// (`LiveCore::wait_terminal`, private to this module) WITHOUT consuming,
/// backs its own captured file up inside the leaf (preserving the original
/// synthetic bytes), truncates ONLY its own synthetic evidence file to 0,
/// then collects the consumer result and the ACTUAL producer join BEFORE
/// any verdict assertion. The consumer read must return `Err`
/// (`UnexpectedEof`) with NO fabricated bytes, while the producer's
/// HISTORICAL outcome stays verbatim (`Completed` with the real conserved
/// count — never rewritten to match the shortened file). This was RED
/// against the former premature-EOF reader; it now pins the repaired
/// contract, and no assertion may be weakened. Discipline: healthy
/// witness FIRST on the same leaf/payload; backup and truncated artifact
/// stay inside the leaf (kept on a failed assertion for diagnosis; the
/// leaf is removed only on success). This reproduces missing promised
/// bytes against a physical EOF with a synthetic source only — no Child,
/// no provider, no live data.
#[test]
fn live_stream_truncated_evidence_refuses_premature_clean_eof() {
    let leaf = private_fixture_leaf("live-truncated-eof");
    let payload: Vec<u8> = b"TRUNCATION-TARGET\r\npayload\0with\xFFraw".to_vec();
    // HEALTHY WITNESS BEFORE THE INJECTION: same leaf, same payload — the
    // full consumer contract must hold first.
    let witness = leaf.join("truncation-witness.bin");
    assert_healthy_consumer_witness(&witness, &payload);
    let evidence = leaf.join("truncated-evidence.bin");
    let mut stream =
        LiveRecordedStream::start(std::io::Cursor::new(payload.clone()), &evidence)
            .expect("finite stream start");
    // Terminal publication first, WITHOUT consuming: the producer has
    // really finished; the promised count lives in the core.
    stream.core.wait_terminal();
    // Preserve the original synthetic bytes in a backup INSIDE the leaf,
    // then truncate ONLY the own synthetic evidence file to 0.
    let backup = leaf.join("truncated-backup.raw");
    std::fs::copy(&evidence, &backup).expect("evidence backup copy");
    assert_eq!(
        std::fs::read(&backup).expect("backup read"),
        payload,
        "fixture: the backup must preserve the original synthetic bytes"
    );
    std::fs::OpenOptions::new()
        .write(true)
        .open(&evidence)
        .and_then(|file| file.set_len(0))
        .expect("own synthetic evidence truncation to 0");
    assert_eq!(
        std::fs::metadata(&evidence).expect("evidence metadata").len(),
        0,
        "fixture: the truncation must be effective"
    );
    // REAL collection BEFORE any verdict assertion: consumer result, then
    // the actual producer join.
    let mut reader = stream.evidence_reader();
    let mut seen = Vec::new();
    let read_result = reader.read_to_end(&mut seen);
    let outcome = stream.join();
    // Verdict pins.
    // THE COUNTEREXAMPLE PIN (RED with the current premature-EOF reader):
    // missing promised bytes against a physical EOF must surface as a real
    // error, never a clean success.
    match &read_result {
        Err(cause) => assert_eq!(
            cause.kind(),
            std::io::ErrorKind::UnexpectedEof,
            "the reader must refuse a premature clean EOF with UnexpectedEof \
             when promised conserved bytes are missing from the evidence \
             file: {cause}"
        ),
        Ok(_) => panic!(
            "consumer got a clean read success {read_result:?} over a \
             truncated evidence file: the reader must refuse a premature \
             clean EOF (missing promised bytes), never fabricate one"
        ),
    }
    // No fabricated bytes: nothing may appear from the emptied file.
    assert!(
        seen.is_empty(),
        "no bytes may be fabricated after the truncation: {seen:?}"
    );
    // The producer's HISTORICAL outcome stays verbatim: the conservation
    // really completed with the exact count before the external truncation.
    match outcome {
        LiveStreamState::Completed { bytes } => {
            assert_eq!(bytes, payload.len() as u64, "historical byte count");
        }
        other => panic!("producer must stay Completed, got {other:?}"),
    }
    std::fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

/// Zero-prefix terminal-edge guard (failure variant): a producer failing
/// BEFORE its first byte must still publish its real `Failed` terminal
/// state, and the consumer must keep that error observable WITHOUT the
/// empty-buffer protocol consuming it. Order: HEALTHY WITNESSES FIRST on
/// the same leaf — a normal full stream, then a TRULY EMPTY healthy stream
/// (both full consumer contracts, so the later failure is attributable to
/// the injected fault and not to the fixture, the leaf or the empty-file
/// path); then the zero-prefix fault. The terminal publication is awaited
/// CAUSALLY (`LiveCore::wait_terminal`, condvar — no timeout, no poll); an
/// empty-buffer read then returns protocol `Ok(0)` WITHOUT consuming the
/// terminal error, and the NEXT NONEMPTY read returns the real `Err`:
/// the cause's kind (`BrokenPipe`), the producer's stage `Read`, the 0
/// confirmed conserved bytes and the verbatim cause. The evidence file
/// stays exactly empty, and the producer's ORIGINAL `Failed` outcome (real
/// cause, 0-byte floor, no flush error) is collected via `join` BEFORE any
/// verdict assertion. Own private leaf only, removed on success and kept
/// on failure; local synthetic source only — no Child, no provider, no
/// live data, no timing or environment mutation.
#[test]
fn live_stream_zero_prefix_failure_and_empty_buffer_preserve_error() {
    let leaf = private_fixture_leaf("live-zero-prefix-failure");
    // HEALTHY WITNESS 1: a normal full stream on the same leaf.
    let payload: Vec<u8> = b"zero-prefix-witness\r\n\0\xFF".to_vec();
    let witness = leaf.join("zero-prefix-failure-witness.bin");
    assert_healthy_consumer_witness(&witness, &payload);
    // HEALTHY WITNESS 2: a TRULY EMPTY healthy stream — a producer serving
    // no bytes at all must complete cleanly and give the consumer a
    // justified clean EOF over an exactly-empty file.
    let empty_witness = leaf.join("zero-prefix-empty-witness.bin");
    assert_healthy_consumer_witness(&empty_witness, &[]);
    // Zero-prefix failing source: fails on its FIRST read, before any
    // byte, with a distinctive BrokenPipe cause (local fixture only).
    struct ZeroPrefixFailingSource;
    impl Read for ZeroPrefixFailingSource {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "fixture zero-prefix source failed before its first byte",
            ))
        }
    }
    let evidence = leaf.join("zero-prefix-failure.bin");
    let mut stream = LiveRecordedStream::start(ZeroPrefixFailingSource, &evidence)
        .expect("zero-prefix failure stream start");
    // CAUSAL wait for the terminal publication (never a timed wait).
    stream.core.wait_terminal();
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let reader = stream.evidence_reader();
        // Empty-buffer protocol read FIRST: its real Result is COLLECTED,
        // never pre-asserted (no `.expect()` before the join).
        let empty_result = reader.read(&mut []);
        // NEXT NONEMPTY read: its real Result is collected too — the
        // empty-buffer read above must have consumed nothing.
        let mut buffer = [0u8; 64];
        let error_read = reader.read(&mut buffer);
        // The ACTUAL producer join BEFORE any verdict assertion: all three
        // real outcomes (empty Result, nonempty Result, join) are
        // collected FIRST; only then are the verdicts asserted.
        let outcome = stream.join();
        // Verdicts.
        assert!(
            matches!(empty_result, Ok(0)),
            "empty-buffer protocol read must return Ok(0) without consuming \
             the terminal error: {empty_result:?}"
        );
        let read_error = match &error_read {
            Err(cause) => cause,
            Ok(n) => panic!(
                "consumer must receive the producer failure as an observable \
                 error, got Ok({n}): {error_read:?}"
            ),
        };
        assert_eq!(
            read_error.kind(),
            std::io::ErrorKind::BrokenPipe,
            "consumer error must carry the producer cause's kind: {read_error}"
        );
        let diagnostic = read_error.to_string();
        assert!(
            diagnostic.contains("stage Read"),
            "consumer diagnostic must name the producer failure stage Read: \
             {diagnostic}"
        );
        assert!(
            diagnostic.contains("0 bytes conserved"),
            "consumer diagnostic must carry the 0 confirmed conserved bytes: \
             {diagnostic}"
        );
        assert!(
            diagnostic.contains("failed before its first byte"),
            "consumer diagnostic must preserve the producer cause verbatim: \
             {diagnostic}"
        );
        match &outcome {
            LiveStreamState::Failed { error } => {
                assert_eq!(error.stage, StreamCaptureStage::Read, "{error:?}");
                assert_eq!(
                    error.cause.kind(),
                    std::io::ErrorKind::BrokenPipe,
                    "{error:?}"
                );
                assert_eq!(
                    error.cause.to_string(),
                    "fixture zero-prefix source failed before its first byte",
                    "original cause must be preserved verbatim (full-string \
                     equality with the known fixture text): {error:?}"
                );
                assert_eq!(error.bytes_written, 0, "{error:?}");
                assert!(error.flush_error.is_none(), "{error:?}");
            }
            other => panic!(
                "zero-prefix failure must surface as Failed, got {other:?}"
            ),
        }
        // The evidence file stays exactly empty.
        assert_eq!(
            std::fs::metadata(&evidence).expect("evidence metadata").len(),
            0,
            "a zero-prefix failure must leave an exactly empty evidence file"
        );
        assert!(
            std::fs::read(&evidence).expect("evidence read").is_empty(),
            "no byte may appear in the evidence file of a zero-prefix failure"
        );
    }));
    match body {
        Ok(()) => std::fs::remove_dir_all(&leaf).expect("own leaf cleanup"),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Zero-prefix terminal-edge guard (panic variant): a producer panicking
/// BEFORE its first byte must still publish its real `Panicked` terminal
/// state, and the consumer must keep an observable error WITHOUT the
/// empty-buffer protocol consuming it — while the ORIGINAL panic payload
/// stays verbatim in `join`, never unpacked into a consumer error. Order:
/// HEALTHY WITNESSES FIRST on the same leaf (full stream, then a TRULY
/// EMPTY healthy stream, both full consumer contracts); then the
/// zero-prefix panic. The terminal publication is awaited CAUSALLY
/// (`LiveCore::wait_terminal`); an empty-buffer read returns protocol
/// `Ok(0)` WITHOUT consuming the terminal error; the NEXT NONEMPTY read
/// returns `Err` with kind Other and the SAFE panic diagnostic, which
/// names the producer panic and points at `join` but contains NO fragment
/// of the real payload text. The evidence file stays exactly empty, and
/// the producer's `Panicked` outcome — with the payload's original exact
/// text, checked for presence, never invented — is collected via `join`
/// BEFORE any verdict assertion. Own private leaf only, removed on success
/// and kept on failure; local synthetic source only — no Child, no
/// provider, no live data, no timing or environment mutation.
#[test]
fn live_stream_zero_prefix_panic_and_empty_buffer_preserve_payload() {
    let leaf = private_fixture_leaf("live-zero-prefix-panic");
    // HEALTHY WITNESS 1: a normal full stream on the same leaf.
    let payload: Vec<u8> = b"zero-prefix-panic-witness\r\n\0\xFF".to_vec();
    let witness = leaf.join("zero-prefix-panic-witness.bin");
    assert_healthy_consumer_witness(&witness, &payload);
    // HEALTHY WITNESS 2: a TRULY EMPTY healthy stream.
    let empty_witness = leaf.join("zero-prefix-panic-empty-witness.bin");
    assert_healthy_consumer_witness(&empty_witness, &[]);
    // Zero-prefix panicking source: panics on its FIRST read, before any
    // byte, with a distinctive payload (never invented downstream).
    struct ZeroPrefixPanickingSource;
    impl Read for ZeroPrefixPanickingSource {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            panic!("fixture zero-prefix source panicked before its first byte");
        }
    }
    let evidence = leaf.join("zero-prefix-panic.bin");
    let mut stream = LiveRecordedStream::start(ZeroPrefixPanickingSource, &evidence)
        .expect("zero-prefix panic stream start");
    // CAUSAL wait for the terminal publication (never a timed wait).
    stream.core.wait_terminal();
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let reader = stream.evidence_reader();
        // Empty-buffer protocol read FIRST: its real Result is COLLECTED,
        // never pre-asserted (no `.expect()` before the join).
        let empty_result = reader.read(&mut []);
        // NEXT NONEMPTY read: its real Result (the safe panic diagnostic)
        // is collected too — the empty-buffer read above must have
        // consumed nothing.
        let mut buffer = [0u8; 64];
        let error_read = reader.read(&mut buffer);
        // The ACTUAL producer join BEFORE any verdict assertion: all three
        // real outcomes (empty Result, nonempty Result, join) are
        // collected FIRST; only then are the verdicts asserted.
        let outcome = stream.join();
        // Verdicts.
        assert!(
            matches!(empty_result, Ok(0)),
            "empty-buffer protocol read must return Ok(0) without consuming \
             the terminal error: {empty_result:?}"
        );
        let read_error = match &error_read {
            Err(cause) => cause,
            Ok(n) => panic!(
                "consumer must receive the producer panic as an observable \
                 error, got Ok({n}): {error_read:?}"
            ),
        };
        assert_eq!(
            read_error.kind(),
            std::io::ErrorKind::Other,
            "panic diagnostic must surface as Other (payload never unpacked): \
             {read_error}"
        );
        let diagnostic = read_error.to_string();
        assert!(
            diagnostic.contains("producer panicked before clean EOF"),
            "consumer diagnostic must name the producer panic explicitly: \
             {diagnostic}"
        );
        assert!(
            !diagnostic.contains("panicked before its first byte"),
            "the real panic payload text must NEVER be unpacked into the \
             consumer diagnostic (it belongs to `join` alone): {diagnostic}"
        );
        // The ORIGINAL payload stays verbatim for `join`: presence checked,
        // never invented, never reduced to a success.
        match &outcome {
            LiveStreamState::Panicked { payload } => {
                let message = payload
                    .downcast_ref::<&'static str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
                assert!(
                    message.is_some_and(|message| message
                        == "fixture zero-prefix source panicked before its first byte"),
                    "original panic payload must be preserved verbatim \
                     (full-string equality with the known fixture text) and \
                     identifiable: {outcome:?}"
                );
            }
            other => panic!(
                "zero-prefix panic must surface as Panicked, got {other:?}"
            ),
        }
        // The evidence file stays exactly empty.
        assert_eq!(
            std::fs::metadata(&evidence).expect("evidence metadata").len(),
            0,
            "a zero-prefix panic must leave an exactly empty evidence file"
        );
        assert!(
            std::fs::read(&evidence).expect("evidence read").is_empty(),
            "no byte may appear in the evidence file of a zero-prefix panic"
        );
    }));
    match body {
        Ok(()) => std::fs::remove_dir_all(&leaf).expect("own leaf cleanup"),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Third guard: exclusive-creation refusal — an existing evidence file is
/// refused WITHOUT mutation and WITHOUT an ambiguous start (no worker, no
/// pipe, no state), the existing entry stays byte-identical, and a fresh
/// path still starts. VALID WITNESS FIRST: the existing file is itself
/// produced by a healthy completed live stream on this leaf, so the refusal
/// is exercised under the same preconditions as the successes.
#[test]
fn live_stream_refuses_existing_evidence_without_mutation_or_ambiguous_start() {
    let leaf = private_fixture_leaf("live-refuse-existing");
    let witness = leaf.join("witness.bin");
    let payload: Vec<u8> = b"witness-payload\r\n\xFF\0".to_vec();
    let witness_stream =
        LiveRecordedStream::start(std::io::Cursor::new(payload.clone()), &witness)
            .expect("witness start");
    // No before-EOF semantics here either: wait deterministically for the
    // producer's terminal state, then verify it through join.
    let snapshot = witness_stream.wait_written(u64::MAX);
    assert!(
        snapshot.terminal && snapshot.written == payload.len() as u64,
        "{snapshot:?}"
    );
    match witness_stream.join() {
        LiveStreamState::Completed { bytes } => assert_eq!(bytes, payload.len() as u64),
        other => panic!("witness must complete, got {other:?}"),
    }
    assert_eq!(std::fs::read(&witness).expect("read back"), payload);
    // Refusal: starting on the existing witness path must refuse BEFORE any
    // worker or reader exists, leaving the file byte-identical.
    let error = match LiveRecordedStream::start(
        std::io::Cursor::new(b"never written".to_vec()),
        &witness,
    ) {
        Err(error) => error,
        Ok(_) => panic!("existing evidence must refuse, not start"),
    };
    assert_eq!(error.stage, StreamCaptureStage::Open, "{error:?}");
    assert_eq!(
        error.cause.kind(),
        std::io::ErrorKind::AlreadyExists,
        "{error:?}"
    );
    assert_eq!(error.bytes_written, 0, "{error:?}");
    assert!(error.flush_error.is_none(), "{error:?}");
    assert_eq!(
        std::fs::read(&witness).expect("read back"),
        payload,
        "existing evidence must stay byte-identical"
    );
    std::fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

/// Reader-preparation refusal guard: a failed consumer-descriptor
/// preparation (stage `ReaderPrepare`) must keep the created EMPTY 0600
/// artifact on disk (never deleted, never reported as "no effect"), a
/// retry on the SAME path must then refuse at stage `Open` with
/// `AlreadyExists` while the empty artifact stays byte-identical, and a
/// PUBLIC start on a fresh path must still recover with the exact healthy
/// consumer contract.
///
/// Order: HEALTHY REAL-READER WITNESS FIRST on the same leaf (full
/// consumer contract via `assert_healthy_consumer_witness`), then the
/// injection; every real outcome (the refusal error, the second refusal,
/// the recovery's consumer drain and its ACTUAL producer join) is
/// collected BEFORE the assertions that judge it. The preparation failure
/// is a SYNTHETIC INJECTION through the private reservation helper's
/// preparation function (`io::ErrorKind::Other` with a distinctive
/// message) — it is NOT an actual OS resource exhaustion and proves
/// nothing about fd limits; it deterministically exercises the
/// ReaderPrepare mapping and the artifact contract. The injected cause is
/// asserted by FULL-STRING equality (real cause preserved verbatim).
/// Own private leaf only, removed on success and kept on failure; local
/// synthetic source only — no Child, no provider, no live data, no timing
/// or environment mutation.
#[test]
fn live_stream_reader_prepare_refusal_keeps_empty_artifact_then_recovers_fresh() {
    let leaf = private_fixture_leaf("live-reader-prepare-refusal");
    let payload: Vec<u8> = b"reader-prepare-witness\r\npayload\0\xFF".to_vec();
    // HEALTHY REAL-READER WITNESS FIRST: same leaf, same payload — the
    // full consumer contract must hold before any injection.
    let witness = leaf.join("reader-prepare-witness.bin");
    assert_healthy_consumer_witness(&witness, &payload);
    // SYNTHETIC INJECTION (not an actual OS resource exhaustion): the
    // private reservation helper is called with a preparation function
    // that deterministically fails with a distinctive Other error.
    let injected_message = "SYNTHETIC injection of a failed consumer-descriptor \
                            preparation (not an actual OS resource exhaustion)";
    let refused_path = leaf.join("reader-prepare-refused.bin");
    let refusal_error = match LiveRecordedStream::reserve_with_preparation(
        &refused_path,
        |_file| Err(std::io::Error::new(std::io::ErrorKind::Other, injected_message)),
    ) {
        Err(error) => error,
        Ok(_) => panic!(
            "the injected preparation failure must refuse, never reserve or \
             start a capture"
        ),
    };
    assert_eq!(
        refusal_error.stage,
        StreamCaptureStage::ReaderPrepare,
        "{refusal_error:?}"
    );
    assert_eq!(
        refusal_error.cause.kind(),
        std::io::ErrorKind::Other,
        "the real injected kind must surface as-is: {refusal_error:?}"
    );
    assert_eq!(
        refusal_error.cause.to_string(),
        injected_message,
        "the real injected cause must be preserved verbatim (full-string \
         equality): {refusal_error:?}"
    );
    assert_eq!(refusal_error.bytes_written, 0, "{refusal_error:?}");
    assert!(refusal_error.flush_error.is_none(), "{refusal_error:?}");
    // The refusal's own created artifact stays exactly EMPTY, private
    // 0600, never deleted (an Err here is explicitly NOT "no effect").
    let refused_metadata = std::fs::metadata(&refused_path)
        .expect("the refused start's own artifact must exist on disk");
    assert_eq!(
        refused_metadata.len(),
        0,
        "the ReaderPrepare artifact must be exactly empty: {refused_metadata:?}"
    );
    assert_eq!(
        refused_metadata.permissions().mode() & 0o777,
        0o600,
        "the ReaderPrepare artifact must keep its private 0600 mode"
    );
    assert!(
        std::fs::read(&refused_path)
            .expect("refused artifact read")
            .is_empty(),
        "no byte may appear in the ReaderPrepare artifact"
    );
    // Retry on the SAME now-existing path: exclusive creation refuses at
    // stage `Open` with `AlreadyExists`; the empty artifact stays
    // byte-identical (still exactly empty, still 0600).
    let retry_error = match LiveRecordedStream::reserve_evidence(&refused_path) {
        Err(error) => error,
        Ok(_) => panic!("an existing evidence path must refuse, never re-reserve"),
    };
    assert_eq!(retry_error.stage, StreamCaptureStage::Open, "{retry_error:?}");
    assert_eq!(
        retry_error.cause.kind(),
        std::io::ErrorKind::AlreadyExists,
        "{retry_error:?}"
    );
    assert_eq!(retry_error.bytes_written, 0, "{retry_error:?}");
    assert!(retry_error.flush_error.is_none(), "{retry_error:?}");
    let retry_metadata = std::fs::metadata(&refused_path)
        .expect("the empty artifact must survive the second refusal");
    assert_eq!(
        retry_metadata.len(),
        0,
        "the empty artifact must stay exactly empty through the Open refusal: \
         {retry_metadata:?}"
    );
    assert_eq!(
        retry_metadata.permissions().mode() & 0o777,
        0o600,
        "the empty artifact must keep its private 0600 mode through the Open \
         refusal"
    );
    // Recovery via PUBLIC start on a fresh path: the exact healthy
    // consumer contract (drain and ACTUAL join collected inside the
    // helper BEFORE its assertions).
    let recovered = leaf.join("reader-prepare-recovered.bin");
    assert_healthy_consumer_witness(&recovered, &payload);
    std::fs::remove_dir_all(&leaf).expect("own leaf cleanup");
}

// ---- Fixture sources -----------------------------------------------------

/// Deterministic fixture script shared between a guard and its simulated
/// source: queued fragments plus an explicit EOF permission, all under one
/// Mutex/Condvar pair — causal waiting only, no timed poll.
#[derive(Default)]
struct LiveScript {
    inner: Mutex<LiveScriptInner>,
    signal: Condvar,
}

#[derive(Default)]
struct LiveScriptInner {
    fragments: VecDeque<Vec<u8>>,
    /// Bytes already delivered from the front fragment.
    delivered: usize,
    eof_permitted: bool,
}

impl LiveScript {
    /// Queues one fragment and wakes the source.
    fn push(&self, fragment: Vec<u8>) {
        let mut inner = self.inner.lock().expect("script lock");
        inner.fragments.push_back(fragment);
        drop(inner);
        self.signal.notify_all();
    }

    /// Grants the EOF permission, waking the source.
    fn permit_eof(&self) {
        let mut inner = self.inner.lock().expect("script lock");
        inner.eof_permitted = true;
        drop(inner);
        self.signal.notify_all();
    }
}

/// Simulated live source over a [`LiveScript`]: delivers queued fragments
/// verbatim (chunked by the caller's buffer) and returns EOF only after
/// the explicit permission. It blocks on the script condvar while no
/// fragment is queued — which is why the guards always release the script
/// (see the first guard's release-all-path) so a panicking assertion never
/// leaves this worker stuck.
struct LiveScriptSource {
    script: Arc<LiveScript>,
}
impl Read for LiveScriptSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let mut inner = self.script.inner.lock().expect("script lock");
        loop {
            // The whole delivery decision is computed while the immutable
            // borrow of `inner.fragments` is alive, so the counter update
            // below never overlaps it: a fragment is exhausted exactly
            // when the copy consumed every remaining byte, which needs no
            // second look at the front fragment afterwards.
            let step = inner.fragments.front().map(|front| {
                let remaining = &front[inner.delivered..];
                let n = remaining.len().min(buf.len());
                buf[..n].copy_from_slice(&remaining[..n]);
                (n, remaining.len() == n)
            });
            if let Some((n, exhausted)) = step {
                inner.delivered += n;
                if exhausted {
                    inner.fragments.pop_front();
                    inner.delivered = 0;
                }
                return Ok(n);
            }
            if inner.eof_permitted {
                return Ok(0);
            }
            inner = self.script.signal.wait(inner).expect("script lock");
        }
    }
}

/// Simulated source failing after a verifiable prefix (live variant).
struct FailingLiveSource {
    data: Vec<u8>,
    served: usize,
    fail_after: usize,
}
impl Read for FailingLiveSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.served >= self.fail_after {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "fixture reader failed after its verifiable live prefix",
            ));
        }
        let n = buf.len().min(self.fail_after - self.served);
        buf[..n].copy_from_slice(&self.data[self.served..self.served + n]);
        self.served += n;
        Ok(n)
    }
}
