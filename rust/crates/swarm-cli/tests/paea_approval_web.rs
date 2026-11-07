//! Positive two-phase CLI witness for `paea approval-web` (one ignored
//! positive witness plus small named selector-validation tests).
//! One real `swarm-rust paea approval-web` child prepares the exact transfer
//! call plus the frozen approve command on the retained native source project,
//! publishes BOTH exact permissions in one prepared event, binds the existing
//! native HTTP server after the pin-only private frame and the explicitly
//! signed two-permission declaration, and the fixture consumes the approval
//! exactly once through the real bound server with an EMPTY POST under the
//! labeled simulated enrollment authority. The positive witness is the typed
//! ack state AND the durable approved action in the PROJECT root, with the
//! SERVICE root Trust state and the managed Git refs byte-identical
//! throughout and a clean SIGINT drain whose observation records are asserted
//! against the real `NativeOutcomeRecord` producer contract (status, stable
//! error family, optional trust_publication) — never reinterpreted as the
//! business reply, and never satisfied by adding fields to the product.
//! Fictitious effects only: no live mission database, no /tmp usage, no Git
//! ref effect, no UID separation, no human presence, no accepted service and
//! no delivered-profile claim. The exercised child image is selected
//! EXPLICITLY before spawn: the Cargo source fixture binary via
//! `CARGO_BIN_EXE_swarm-rust` unless BOTH `SWARM_PAEA_APPROVAL_WEB_CLI` and
//! `SWARM_PAEA_APPROVAL_WEB_CLI_SHA256` designate an external binary whose
//! real, executable file matches the expected SHA-256; any partial,
//! relative, invalid or hash-mismatched selection refuses before spawn and
//! never falls back to the Cargo fixture binary. The selection hash pins
//! the file bytes at selection time only: it proves neither a delivered or
//! normal runtime profile; the spawned child's real `/proc/<pid>/exe`
//! identity is observed (with boot id and start ticks) while the child is
//! owned and recorded in the process receipt — an observation, not a
//! profile claim. Two sibling modules provide the raw-evidence foundations and
//! are WIRED to this witness: `paea_capture_stream` (batched raw-stream
//! capture primitive), `paea_capture_live` (live raw recorded-stream reader
//! readable before producer EOF) and `paea_capture_child` (owned child +
//! both concurrent raw captures, explicit stdin transfer via `take_stdin`
//! for the frame EOF, and the `signal_stop` pidfd decision wrapper). The
//! positive witness now captures the child's stdout/stderr into two
//! exclusive private evidence files retained even on PASS, reads prepared
//! and bound events through the live stdout reader, runs the whole HTTP
//! controller under `signal_stop(SIGINT)` and asserts the drained records
//! from the conserved raw stdout file. Refusal, replay and pin variants
//! and the delivered-profile adapter remain separate later tasks,
//! deliberately not started here.
#![cfg(target_os = "linux")]
#[path = "../../swarm-enrollment-authority/tests/common/authenticated_fixture.rs"]
mod fixture;
#[path = "common/paea_capture_stream.rs"]
mod paea_capture_stream;
#[path = "common/paea_capture_live.rs"]
mod paea_capture_live;
#[path = "common/paea_capture_child.rs"]
mod paea_capture_child;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};
use swarm_engine::{RootStore, SystemEnvironment};
use swarm_identity_contract::{Permission, Scope};
use swarm_protection_contract::{canonicalize, Action, AdmissionContext, Proposal, SourceRef};

use paea_capture_child::{OwnedChildCapture, SignalStopResult};
use paea_capture_stream::private_fixture_leaf;

const PIN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PASSWORD: &[u8] = b"STARTUP_FIXTURE_PASSWORD";

/// Business assertion failure of the witness's own HTTP controller: the
/// ORIGINAL panic payload is preserved verbatim alongside the process
/// receipt and both raw captures, then re-raised unwound (on the normal
/// original-only path `resume_unwind` keeps the real panic type; on the
/// COMPOUND finished-path receipt failure the typed `CompoundPanic`
/// carrier truthfully wraps it — the top-level unwound type is the
/// carrier, and the original payload stays downcastable inside) — no
/// secondary failure ever replaces the first business cause.
/// Witness-local to this test (bounded
/// helper, never a product or shared-module API).
struct WitnessPanic {
    message: String,
    payload: Option<Box<dyn std::any::Any + Send>>,
}

impl WitnessPanic {
    /// Panic with the conserved material: the receipt and raw evidence are
    /// already written by the caller, so this re-emits the ORIGINAL
    /// payload verbatim when one exists (`resume_unwind` keeps the real
    /// panic site/type for the harness), or fails with the message
    /// otherwise (refusal routes without a payload). `raise` makes NO
    /// compound promise: on `Some` the ORIGINAL payload alone is resumed
    /// verbatim (original top-level panic type preserved) and `message` is
    /// DROPPED by `resume_unwind` — a secondary that must survive the
    /// unwind is therefore never parked in `message`. The ONE compound
    /// finished-path failure (original payload AND a refused receipt write
    /// together) goes through the companion `WitnessPanic::raise_compound`
    /// and the typed `CompoundPanic` carrier, which retains BOTH through
    /// the unwind (top-level type truthfully the carrier; original payload
    /// downcastable inside); no other secondary preservation is claimed
    /// here.
    fn raise(self) -> ! {
        match self.payload {
            Some(payload) => std::panic::resume_unwind(payload),
            None => panic!("{}", self.message),
        }
    }
}

/// Typed COMPOUND-failure carrier, unwound ONLY when a refused finished-path
/// receipt write coincides with an original controller-panic payload: that
/// secondary receipt cause must survive the unwind, and `raise`'s `Some`
/// arm would drop it together with `message`. Truthfully a WRAPPER — the
/// top-level unwound type is this carrier, NOT the original panic type;
/// the original payload stays verbatim and downcastable in `original`.
/// Witness-local to this test (bounded helper, never a product or
/// shared-module API).
struct CompoundPanic {
    /// Secondary receipt cause EXACTLY as the builder produced it: a
    /// stage-tagged `String` (e.g. `receipt atomic rename refused: ...`),
    /// not a typed structured OS cause and not a traversable error chain.
    receipt_cause: String,
    /// The ORIGINAL `Box<dyn Any + Send>` payload, retained verbatim for
    /// the regression guard's downcast.
    original: Box<dyn std::any::Any + Send>,
}

impl WitnessPanic {
    /// Companion raise for the COMPOUND finished-path failure ONLY (an
    /// original payload AND a refused receipt write together). The plain
    /// `raise` above is unchanged: on the normal original-only path it
    /// keeps the original top-level panic type via `resume_unwind`.
    fn raise_compound(original: Box<dyn std::any::Any + Send>, receipt_cause: String) -> ! {
        std::panic::resume_unwind(Box::new(CompoundPanic {
            receipt_cause,
            original,
        }))
    }
}

/// Writes the witness's persistent process receipt ATOMICALLY (same-dir
/// uniquely named temp file, exclusive create with the 0600 mode applied AT
/// CREATION via `OpenOptionsExt::mode` — NO post-create protection window,
/// the old create-then-chmod gap is gone — then fsync file, rename, fsync
/// parent directory) and returns the receipt path. The `process_outcome`
/// object is built by the CALLER strictly from the REAL process facts of
/// that route: `finished_process_outcome` immediately after the real
/// `Finished` destructure (real `ExitStatus` code, Unix signal and success,
/// each raw stream's REAL terminal variant with its EXACT `Completed` byte
/// count, controller-panic PRESENCE only), or `not_finished_process_outcome`
/// on the typed refusal routes (explicit UNKNOWN / not-finished envelope —
/// no invented status, count or settled state). The private controller-panic
/// payload is NEVER serialized into the receipt. Content: receipt schema
/// version, witness label, relative paths inside the retained leaf + their
/// SHA-256 raw byte hashes + the exact raw bytes of the selection (argv,
/// mode, pid, boot/start ticks, `/proc/<pid>/exe` SHA-256).
///
/// Failure shape: this builder returns `Err(String)` — a STAGE-TAGGED error
/// TEXT (e.g. `receipt atomic rename refused: <io error display>`), NOT a
/// typed structured OS cause and NOT a traversable error chain; the stage
/// prefix is the only structure and no cause traversal is claimed or
/// provided. The CALL SITE decides: on the finished path a receipt failure
/// stays a FAILURE (never a fake PASS) — without a controller panic it is
/// a plain panic carrying that stage-tagged text; with one, the typed
/// `CompoundPanic` carrier unwinds BOTH the ORIGINAL payload (verbatim,
/// downcastable inside) and this text as the secondary receipt cause
/// (never swallowed, never disguised; the top-level unwound type is
/// truthfully the carrier, not the original panic type). Refusal routes
/// raise their real cause with the receipt text in their panic message. The path is printed ONCE per call as
/// a single manifest JSON line on STDERR (never a log tail, no session
/// secret) for Codex's manual identification — stderr, NOT stdout: under
/// `--nocapture` a `println!` interleaves with the harness's own stdout
/// lines (e.g. `test ... ok`), while the recipe's `required_stdout` pins
/// must match the harness output exactly. Witness-local to this test; the
/// shared capture types are untouched.
///
/// Returns `Ok((receipt_path, leaf))`; the leaf is never removed here — the
/// receipt and both raw streams stay on disk even on PASS until Codex
/// receives them.
fn write_process_receipt(
    leaf: &Path,
    witness: &str,
    manifest: &str,
    selection: &Value,
    process_outcome: &Value,
) -> Result<(PathBuf, PathBuf), String> {
    // Relative paths (never absolute paths that would leak the runner's
    // private area spelling) + real byte hashes of the conserved files.
    let stdout_name = "stdout.raw";
    let stderr_name = "stderr.raw";
    let stdout_bytes = fs::read(leaf.join(stdout_name))
        .map_err(|e| format!("retained stdout.raw unreadable at receipt time: {e}"))?;
    let stderr_bytes = fs::read(leaf.join(stderr_name))
        .map_err(|e| format!("retained stderr.raw unreadable at receipt time: {e}"))?;
    let entries = json!({
        "stdout": {
            "path": stdout_name,
            "sha256": format!("{:x}", Sha256::digest(&stdout_bytes)),
            "bytes": stdout_bytes.len(),
        },
        "stderr": {
            "path": stderr_name,
            "sha256": format!("{:x}", Sha256::digest(&stderr_bytes)),
            "bytes": stderr_bytes.len(),
        },
    });
    let receipt = json!({
        "schema_version": 1,
        "witness": witness,
        "selection": selection,
        "artifacts": entries,
        "process": process_outcome,
    });
    let body = serde_json::to_vec_pretty(&receipt)
        .map_err(|e| format!("receipt serialization refused: {e}"))?;
    let receipt_path = leaf.join("process-receipt.json");
    // Atomic replacement: uniquely named temp file in the SAME directory,
    // created EXCLUSIVELY with its 0600 mode AT CREATION via
    // `OpenOptionsExt::mode` — there is NO post-create protection window
    // (no create-then-chmod gap) — then fsync, rename over the target, and
    // fsync the directory so the rename is durable. A receipt left by an
    // earlier route in the same run is REPLACED whole — a newer receipt
    // never mixes with an older one, and the rename can never fail on a
    // pre-existing temp of its own.
    let temp_path = leaf.join(format!(
        "process-receipt.json.tmp-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("receipt wall clock refused: {e}"))?
            .as_nanos()
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp_path)
        .map_err(|e| format!("receipt temp creation refused: {e}"))?;
    file.write_all(&body)
        .map_err(|e| format!("receipt write refused: {e}"))?;
    file.sync_all()
        .map_err(|e| format!("receipt fsync refused: {e}"))?;
    drop(file);
    fs::rename(&temp_path, &receipt_path)
        .map_err(|e| format!("receipt atomic rename refused: {e}"))?;
    let dir = fs::File::open(leaf).map_err(|e| format!("leaf dir open refused: {e}"))?;
    dir.sync_all()
        .map_err(|e| format!("leaf dir fsync refused: {e}"))?;
    drop(dir);
    eprintln!(
        "{}",
        json!({
            "manifest": manifest,
            "receipt": receipt_path.display().to_string(),
        })
    );
    Ok((receipt_path, leaf.to_path_buf()))
}

/// Builds the witness-local `process` receipt object for a REAL Finished
/// collect, strictly from the actual finished facts: the real `ExitStatus`
/// (`code()` plus the Unix `signal()` — on Linux exactly one of the two is
/// `Some`, both recorded verbatim), `success()`, each raw stream's REAL
/// terminal variant with the EXACT `Completed` byte count (`Failed` and
/// `Panicked` are recorded as the explicit NON-completed variants with a
/// `null` byte count — never a partial count dressed as a total), and the
/// controller-panic PRESENCE only (the private payload is never serialized
/// into the receipt JSON). No wait count and no settled-state claim is
/// invented: the object records exactly what the one real finish returned,
/// nothing else. Witness-local to this test; the shared capture types are
/// untouched.
fn finished_process_outcome(
    exit: &std::process::ExitStatus,
    stdout_terminal: &paea_capture_live::LiveStreamState,
    stderr_terminal: &paea_capture_live::LiveStreamState,
    controller_panic_present: bool,
) -> Value {
    /// Contract: maps ONE conserved stream's REAL terminal state to its
    /// receipt pair — the exact variant name plus a byte count ONLY where a
    /// count truly exists. `Completed` carries its EXACT byte count;
    /// `Failed` and `Panicked` are the explicit NON-completed variants and
    /// record a `null` count (never a partial count dressed as a total).
    /// There is deliberately no catch-all arm: any future terminal variant
    /// must be mapped explicitly here, never silently defaulted.
    fn terminal_fields(state: &paea_capture_live::LiveStreamState) -> (Value, Value) {
        match state {
            paea_capture_live::LiveStreamState::Completed { bytes } => {
                (json!("Completed"), json!(bytes))
            }
            paea_capture_live::LiveStreamState::Failed { .. } => (json!("Failed"), Value::Null),
            paea_capture_live::LiveStreamState::Panicked { .. } => {
                (json!("Panicked"), Value::Null)
            }
        }
    }
    let (stdout_variant, stdout_completed_bytes) = terminal_fields(stdout_terminal);
    let (stderr_variant, stderr_completed_bytes) = terminal_fields(stderr_terminal);
    json!({
        "finished": true,
        "exit_code": exit.code(),
        "signal": exit.signal(),
        "success": exit.success(),
        "stdout_terminal": stdout_variant,
        "stdout_completed_bytes": stdout_completed_bytes,
        "stderr_terminal": stderr_variant,
        "stderr_completed_bytes": stderr_completed_bytes,
        "controller_panic": controller_panic_present,
    })
}

/// Builds the witness-local `process` receipt object for the typed refusal
/// routes (preparation refused, signal refused, wait refused): the process
/// outcome is EXPLICITLY UNKNOWN / NOT FINISHED — `finished` is `false` and
/// every fact the route does not actually carry (exit code, Unix signal,
/// success, terminal variants, `Completed` byte counts) stays `null` with
/// NO invented status, count or settled state. The controller-panic
/// presence is recorded ONLY where the route actually carries it (the
/// signal-refused refusal exposes the real flag; routes without that fact
/// record `null`): the typed ownership of these routes and their
/// original-panic preservation remain a separate, still-OPEN task and are
/// NOT claimed resolved or exercised here. Witness-local to this test.
fn not_finished_process_outcome(controller_panic_presence: Option<bool>) -> Value {
    json!({
        "finished": false,
        "exit_code": Value::Null,
        "signal": Value::Null,
        "success": Value::Null,
        "stdout_terminal": Value::Null,
        "stdout_completed_bytes": Value::Null,
        "stderr_terminal": Value::Null,
        "stderr_completed_bytes": Value::Null,
        "controller_panic": controller_panic_presence,
    })
}

/// Witness-local typed carrier of the ONE fixed parent WaitRefused route
/// (the SIGINT-stopped route whose finish came back as
/// `ChildCaptureOutcome::WaitRefused`). EVERY conserved fact is carried BY
/// VALUE: the original non-Clone wait-refusal cause (moved verbatim —
/// `io::Error` is not Clone, never copied or re-created), the SAME child
/// (stdin already closed; NEVER re-waited by the conservation — the exit
/// status stays UNKNOWN, zero automatic wait after a real refusal), BOTH
/// captures as their REAL joined terminal states (their `Failed` /
/// `Panicked` issues and causes are retained verbatim, never masked),
/// both evidence paths, the ORIGINAL controller panic payload when one
/// was retained (only its presence ever reaches the receipt; the payload
/// text never does), and the process-receipt outcome as DATA —
/// `Ok(receipt path)` on the receipt success path or `Err(stage-tagged
/// receipt text)` on the secondary receipt-write failure (never
/// swallowed, never disguised as a success). A `catch_unwind` receiver
/// can recover the carrier from the panic payload BY VALUE
/// (`downcast::<ParentWaitRefusedCarrier>()` on the boxed payload — a
/// bare `downcast_ref` borrow would NOT pass ownership) and then decide
/// the actual cleanup explicitly (including the FIRST real `Child::wait`
/// when no real wait has run). Witness-local to this test (bounded
/// helper, never a product or shared-module API).
struct ParentWaitRefusedCarrier {
    context: &'static str,
    error: std::io::Error,
    child: std::process::Child,
    stdout_state: paea_capture_live::LiveStreamState,
    stderr_state: paea_capture_live::LiveStreamState,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    controller_panic: Option<Box<dyn std::any::Any + Send>>,
    receipt: Result<PathBuf, String>,
}

/// Manual Debug: the child is identified by its pid (diagnostic only —
/// `Child` itself is not `Debug` and a pid may be recycled after exit),
/// the controller panic only by its presence.
impl std::fmt::Debug for ParentWaitRefusedCarrier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParentWaitRefusedCarrier")
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
            .field("receipt", &self.receipt)
            .finish()
    }
}

/// Conservation of a WaitRefused handback on the parent's SIGINT-stopped
/// route into the complete typed [`ParentWaitRefusedCarrier`] — the ONE
/// WaitRefused route fixed in this slice, shared VERBATIM by the
/// production witness arm and the dedicated regression guard below. EVERY
/// handback field is moved in BY VALUE (nothing dropped, no silent
/// abandonment); BOTH captures join to their REAL terminal states BEFORE
/// the receipt is written (on this route the SIGINT stop was accepted and
/// the stdin-EOF permission was already given, but an accepted stop
/// request is NOT termination proof — the joins keep the same documented
/// blocking limit as the finish contract and end only at the producer's
/// real causal EOF: no timer, no kill, no retry; `Failed`/`Panicked`
/// states and their causes are retained verbatim); the receipt is the
/// EXPLICIT
/// UNKNOWN / not-finished envelope — the exit status stays UNKNOWN — and
/// now records the controller-panic PRESENCE the route actually knows
/// (never the payload text). The child is NOT re-waited here and no
/// signal, kill, timer, retry or second conservation reader exists: the
/// receipt reuses the ONE existing `write_process_receipt` and the real
/// producer contracts. Resolving the handback (an explicit FIRST real
/// `Child::wait` when no real wait has run, or any other decision) is the
/// `catch_unwind` receiver's explicit job after recovering the carrier BY
/// VALUE; this helper itself resolves nothing.
fn conserve_parent_wait_refused_carrier(
    context: &'static str,
    witness: &str,
    manifest: &str,
    leaf: &Path,
    selection: &Value,
    handback: paea_capture_child::WaitRefusalHandback,
    controller_panic: Option<Box<dyn std::any::Any + Send>>,
) -> ParentWaitRefusedCarrier {
    let paea_capture_child::WaitRefusalHandback {
        error,
        child,
        stdout_stream,
        stderr_stream,
        stdout_path,
        stderr_path,
    } = handback;
    let stdout_state = stdout_stream.join();
    let stderr_state = stderr_stream.join();
    let receipt = write_process_receipt(
        leaf,
        witness,
        manifest,
        selection,
        &not_finished_process_outcome(Some(controller_panic.is_some())),
    )
    .map(|(receipt_path, _)| receipt_path);
    ParentWaitRefusedCarrier {
        context,
        error,
        child,
        stdout_state,
        stderr_state,
        stdout_path,
        stderr_path,
        controller_panic,
        receipt,
    }
}

/// Explicit typed failure for the parent witness's WaitRefused route: the
/// COMPLETE carrier is transferred REALLY to this typed boundary and
/// panicked WITH the test as a TYPED RECOVERABLE payload — a
/// `catch_unwind` receiver can recover it BY VALUE
/// (`downcast::<ParentWaitRefusedCarrier>()` on the boxed payload) and
/// decide the actual cleanup explicitly (including the FIRST real
/// `Child::wait` when no real wait has run). The conserved facts are
/// printed first (the payload itself is not a message string); the child
/// is NEVER re-waited here and the exit status stays UNKNOWN through this
/// boundary. Limits: this typed carrier only transfers ownership to a
/// recovering `catch_unwind` receiver — with no higher consumer, the
/// libtest top-level drop does NOT establish any external cleanup, the
/// unexpected handback stays VISIBLE as this witness's failure, and NO
/// claim is made that all refusal routes are resolved (PreparationRefused
/// and SignalRefused remain explicitly OPEN and untouched).
fn fail_parent_wait_refused(carrier: ParentWaitRefusedCarrier) -> ! {
    eprintln!(
        "parent witness unexpected wait refusal — {context}: cause {error}; \
         child pid {pid} NEVER re-waited here (exit status UNKNOWN); \
         stdout {stdout:?}, stderr {stderr:?}; stdout path {stdout_path:?}, \
         stderr path {stderr_path:?}; original controller panic retained: \
         {panic_present}; receipt outcome: {receipt:?}",
        context = carrier.context,
        error = carrier.error,
        pid = carrier.child.id(),
        stdout = carrier.stdout_state,
        stderr = carrier.stderr_state,
        stdout_path = carrier.stdout_path,
        stderr_path = carrier.stderr_path,
        panic_present = carrier.controller_panic.is_some(),
        receipt = carrier.receipt,
    );
    std::panic::panic_any(carrier);
}

/// Constant cooperative script for the parent wait-refusal guard's real
/// local children (`/usr/bin/python3 -c`, stdin + both raw streams
/// piped — the SAME preconditions as the parent's stopped route): fixed
/// prefix bytes on BOTH streams, then a block on the single stdin byte
/// that only the finish's stdin-EOF permission releases, then fixed final
/// bytes and the constant exit code. Fully deterministic: no clock, no
/// network, no UID, no live authority.
const PARENT_WAIT_REFUSAL_SCRIPT: &str = "\
import sys
sys.stdout.buffer.write(b'parent-refusal stdout prefix\\n')
sys.stdout.buffer.flush()
sys.stderr.buffer.write(b'parent-refusal stderr prefix\\n')
sys.stderr.buffer.flush()
sys.stdin.buffer.read(1)
sys.stdout.buffer.write(b'parent-refusal stdout final\\n')
sys.stdout.buffer.flush()
sys.stderr.buffer.write(b'parent-refusal stderr final\\n')
sys.stderr.buffer.flush()
sys.exit(7)
";
const PARENT_WAIT_REFUSAL_STDOUT_PREFIX: &[u8] = b"parent-refusal stdout prefix\n";
const PARENT_WAIT_REFUSAL_STDERR_PREFIX: &[u8] = b"parent-refusal stderr prefix\n";
const PARENT_WAIT_REFUSAL_STDOUT_FINAL: &[u8] = b"parent-refusal stdout final\n";
const PARENT_WAIT_REFUSAL_STDERR_FINAL: &[u8] = b"parent-refusal stderr final\n";
/// The constant real exit code of the guard's script child.
const PARENT_WAIT_REFUSAL_EXIT_CODE: i32 = 7;
/// Distinctive marker inside the SYNTHETIC seam refusal cause (the
/// injected error is a test seam, never a real OS wait failure).
const PARENT_WAIT_REFUSAL_SEAM_MARKER: &str = "synthetic parent-guard wait refusal";
/// Distinctive marker carried INSIDE the typed boundary as the original
/// controller panic payload; verified by EXACT value equality after the
/// BY-VALUE recovery (never displayed, never serialized into a receipt).
const PARENT_WAIT_REFUSAL_PANIC_MARKER: &str =
    "parent wait-refusal original controller panic payload marker";
const PARENT_GUARD_WITNESS: &str = "paea_capture_parent_wait_refusal_retains_owned_handoff";
const PARENT_GUARD_MANIFEST: &str = "paea-capture-parent-wait-refusal-guard-receipt";
const PARENT_GUARD_HEALTHY_CONTEXT: &str = "parent guard healthy witness wait refused";
const PARENT_GUARD_INJECTED_CONTEXT: &str =
    "parent guard injected wait refusal conserved before the typed boundary";
const PARENT_GUARD_COLLISION_CONTEXT: &str =
    "parent guard injected wait refusal with the receipt rename collision";

/// Spawns ONE real local script child with the guard's fixed
/// preconditions (stdin, stdout and stderr ALL piped).
fn spawn_parent_wait_refusal_child() -> std::process::Child {
    std::process::Command::new("/usr/bin/python3")
        .arg("-c")
        .arg(PARENT_WAIT_REFUSAL_SCRIPT)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("synthetic local python3 parent wait-refusal child must spawn")
}

/// Starts the capture for a guard child whose start is NOT guaranteed
/// to succeed: fresh private paths and all-three-piped stdio make a
/// refusal unlikely but never impossible, so the owned typed start
/// refusal is NEVER dropped. The refusal is the actual four-variant
/// `paea_capture_child::OwnedChildCaptureStartError` ENUM (no
/// `.error`/`.child` fields exist across its variants), so the SAME
/// child is extracted per exact variant — `StdoutNotPiped`,
/// `StderrNotPiped`, `StdoutEvidenceRefused`, `StderrEvidenceRefused` —
/// while a witness-local `ParentGuardStartCause` mirrors every
/// non-child field as owned data: both preserved refusal paths and the
/// non-Clone `StreamCaptureError` moved verbatim (never cloned, never
/// stringified), so the original cause stays Debug-reportable AND keeps
/// its real typed error fields inside the typed
/// `ParentGuardStartFailure` carrier. The original cause is snapshotted
/// FIRST, the pid pin is snapshotted before the pipes are consumed, and
/// the returned child ownership is resolved through the shared
/// observable collector — called by full path
/// `paea_capture_child::collect_refused_child_observably` (this parent
/// has no glob import): it closes our stdin end, drains every
/// still-owned pipe concurrently to causal EOF, then ONE real wait — a
/// plain `child.wait()` would hang forever on this stdin-blocking
/// script. The collector's real collected outcome and — if the
/// collector itself panics — its opaque secondary cleanup payload are
/// both retained BY VALUE in the carrier's `collected_or_cleanup_panic`
/// field: the secondary is never downcast-guessed to `String`, never
/// stringified, never silently dropped, and never allowed to replace
/// the reported primary cause, and no panic message claims to preserve
/// the original payload text. Only then does the panic report the
/// preserved primary refusal together with the REAL collected facts.
/// Scope: `pub(crate)`, test-only witness scaffolding for this guard —
/// not a production API. This helper claims NO coverage of an
/// unexpected setup failure: no guard phase exercises this route.
pub(crate) fn start_parent_guard_capture(
    child: std::process::Child,
    stdout_path: &std::path::Path,
    stderr_path: &std::path::Path,
) -> OwnedChildCapture {
    // Witness-local cause mirror of the actual four-variant start
    // refusal: every non-child field kept as owned data. The generic
    // parameter is deliberately only NAMED after the producer's
    // non-Clone stream-capture error type — inference binds it to the
    // concrete error carried by the refused variant, so no producer
    // type path is assumed or needed here.
    #[derive(Debug)]
    enum ParentGuardStartCause<StreamCaptureError> {
        StdoutNotPiped,
        StderrNotPiped,
        StdoutEvidenceRefused { error: StreamCaptureError },
        StderrEvidenceRefused {
            stdout_artifact_path: std::path::PathBuf,
            stderr_path: std::path::PathBuf,
            error: StreamCaptureError,
        },
    }

    // The shared collector's outcome: either its real collected facts
    // (typed by inference — never assumed) or the collector's own
    // opaque secondary panic payload, retained BY VALUE.
    enum ParentGuardCollectedOrCleanupPanic<Collected> {
        Collected(Collected),
        CleanupPanic(Box<dyn std::any::Any + Send>),
    }

    // Typed failure carrier: the preserved cause plus whichever
    // outcome the shared collector produced.
    struct ParentGuardStartFailure<Collected, StreamCaptureError> {
        cause: ParentGuardStartCause<StreamCaptureError>,
        collected_or_cleanup_panic: ParentGuardCollectedOrCleanupPanic<Collected>,
    }

    match OwnedChildCapture::start(child, stdout_path, stderr_path) {
        Ok(capture) => capture,
        Err(refusal) => {
            // Split the actual enum refusal into the SAME child (handed
            // to the shared collector below) and a typed witness of
            // every non-child field, preserving the exact variant and
            // its owned data (the non-Clone error is moved, not cloned;
            // nothing is dropped, and nothing about the collector's
            // return type is assumed beyond what it hands back).
            let (cause, refused_child) = match refusal {
                paea_capture_child::OwnedChildCaptureStartError::StdoutNotPiped { child } => {
                    (ParentGuardStartCause::StdoutNotPiped, child)
                }
                paea_capture_child::OwnedChildCaptureStartError::StderrNotPiped { child } => {
                    (ParentGuardStartCause::StderrNotPiped, child)
                }
                paea_capture_child::OwnedChildCaptureStartError::StdoutEvidenceRefused {
                    child,
                    error,
                } => (ParentGuardStartCause::StdoutEvidenceRefused { error }, child),
                paea_capture_child::OwnedChildCaptureStartError::StderrEvidenceRefused {
                    child,
                    stdout_artifact_path,
                    stderr_path,
                    error,
                } => (
                    ParentGuardStartCause::StderrEvidenceRefused {
                        stdout_artifact_path,
                        stderr_path,
                        error,
                    },
                    child,
                ),
            };
            let refused_pid = refused_child.id();
            let cleanup = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                paea_capture_child::collect_refused_child_observably(refused_child)
            }));
            let failure = ParentGuardStartFailure {
                cause,
                collected_or_cleanup_panic: match cleanup {
                    Ok(collected) => ParentGuardCollectedOrCleanupPanic::Collected(collected),
                    Err(secondary) => ParentGuardCollectedOrCleanupPanic::CleanupPanic(secondary),
                },
            };
            match &failure.collected_or_cleanup_panic {
                ParentGuardCollectedOrCleanupPanic::Collected(collected) => eprintln!(
                    "parent guard capture start refused — primary cause preserved (typed \
                     witness, Debug): {cause_debug:?}; refused child pid {refused_pid} \
                     resolved observably before this panic — shared collector outcome: \
                     {collected:?}; setup-refusal route not exercised by this guard; \
                     no coverage claim",
                    cause_debug = failure.cause
                ),
                // The `_` here ignores only the variant binding: the
                // opaque payload itself stays BY VALUE inside `failure`'s
                // `collected_or_cleanup_panic` field — never dropped
                // early, never stringified, never downcast-guessed.
                ParentGuardCollectedOrCleanupPanic::CleanupPanic(_) => eprintln!(
                    "parent guard capture start refused — primary cause preserved (typed \
                     witness, Debug): {cause_debug:?}; refused child pid {refused_pid}; \
                     secondary cleanup panic preserved alongside the primary cause — the \
                     collector panicked and its opaque `Box<dyn Any + Send>` payload is \
                     retained BY VALUE in the typed `ParentGuardStartFailure` carrier's \
                     `collected_or_cleanup_panic` field; this report deliberately does \
                     not stringify, downcast-guess, or claim to preserve the original \
                     payload, and the secondary never replaces the primary cause",
                    cause_debug = failure.cause
                ),
            }
            std::panic::resume_unwind(Box::new(failure));
        }
    }
}

/// Asserts that one joined capture state is the REAL `Completed` terminal
/// variant with the EXACT expected byte count; `Failed`/`Panicked` are
/// named explicitly (their causes stay conserved in the state) and are
/// never converted into completions.
fn assert_parent_guard_completed(
    state: &paea_capture_live::LiveStreamState,
    expected_bytes: usize,
    label: &str,
) {
    match state {
        paea_capture_live::LiveStreamState::Completed { bytes } => {
            assert_eq!(
                *bytes,
                u64::try_from(expected_bytes).expect("expected byte count must fit in u64"),
                "{label} completed byte count"
            );
        }
        paea_capture_live::LiveStreamState::Failed { .. } => {
            panic!("{label} must be Completed, got Failed (cause conserved in the state)");
        }
        paea_capture_live::LiveStreamState::Panicked { .. } => {
            panic!("{label} must be Completed, got Panicked (cause conserved in the state)");
        }
    }
}

/// PARENT REGRESSION (the ONE named guard of this slice):
/// `paea_capture_parent_wait_refusal_retains_owned_handoff` — the typed
/// receipt / ownership boundary of the parent witness's WaitRefused
/// route.
///
/// Phase 1 — HEALTHY CONTROL: an actual valid child under the SAME
/// preconditions as the parent's stopped route (stdin + both raw streams
/// piped, cooperative stdin-EOF stop) with a REAL finish: the real
/// stdin-EOF permission, the real one-attempt `Child::wait` and BOTH real
/// joins; the constant exit code, BOTH `Completed` terminal byte counts
/// and the EXACT raw stream bytes are asserted.
///
/// Phase 2 — INJECTED REFUSAL, receipt success path: a second real child;
/// the finish runs through the ACTUAL `finish_with_wait` FnOnce seam with
/// a SYNTHETIC refusal injected BEFORE any real wait (a test seam ONLY —
/// this is NOT a real OS wait failure). The handback is conserved with
/// the SAME helper the production WaitRefused arm uses
/// (`conserve_parent_wait_refused_carrier`): BOTH captures join to their
/// real terminal states BEFORE the receipt, the UNKNOWN / not-finished
/// receipt envelope records the controller-panic PRESENCE (never the
/// payload text), the receipt path travels as data, and a DISTINCTIVE
/// original controller-panic payload marker travels INSIDE the carrier.
/// The typed boundary is failed with the SAME `fail_parent_wait_refused`
/// the production arm uses, inside a test `catch_unwind`; the boxed
/// payload is recovered BY VALUE (a borrow downcast would not pass
/// ownership), every field is destructured by move, and the facts are
/// asserted as data: single seam invocation, same child pid, verbatim
/// non-Clone cause, completed terminal states, exact raw bytes, UNKNOWN
/// receipt envelope with `controller_panic: true` and no raw payload
/// text, and the exact payload marker. The RECEIVER then decides the
/// actual cleanup explicitly: the injected handler never reached the real
/// `Child::wait`, so the direct `child.wait()` is the FIRST real wait of
/// this child — the real exit code is retained, no orphan.
///
/// Phase 3 — SECONDARY RECEIPT FAILURE as data: a third real child on a
/// fresh leaf whose receipt target path is pre-created as a real
/// DIRECTORY, so the receipt's atomic rename REALLY refuses (actual
/// directory rename collision, feasible inside the same bounded guard).
/// The same helper conserved the handback and the SAME typed boundary
/// fails; the recovered carrier carries
/// `Err("receipt atomic rename refused: …")` as data (never swallowed,
/// never a fake success), the joined terminal states stay conserved, and
/// the receiver again runs the explicit FIRST real wait — the real exit
/// code is retained, no orphan. The refused temp receipt file and the
/// collision directory stay in the private failure leaf as retained
/// evidence.
///
/// Limits (visible, never hidden): the injected refusal is a TEST SEAM,
/// not a real OS wait failure; the typed carrier transfers ownership ONLY
/// to the `catch_unwind` receiver that recovers it — the libtest
/// top-level drop establishes no external cleanup, and if no higher
/// consumer recovers the production witness's unexpected handback, that
/// handback stays VISIBLE as the witness failure; NO claim is made that
/// all refusal routes are resolved (PreparationRefused and SignalRefused
/// remain explicitly OPEN and untouched); there is no automatic re-wait,
/// no signal, no kill, no timeout and no retry anywhere in this guard.
#[test]
fn paea_capture_parent_wait_refusal_retains_owned_handoff() {
    let healthy_leaf = private_fixture_leaf("parent-wait-refusal-handoff-healthy");
    let injected_leaf = private_fixture_leaf("parent-wait-refusal-handoff-injected");
    let selection = json!({
        "guard": "paea_capture_parent_wait_refusal_retains_owned_handoff",
        "child": "constant local /usr/bin/python3 -c script; stdin and both raw streams piped; cooperative stdin-EOF stop",
    });
    let mut expected_stdout = PARENT_WAIT_REFUSAL_STDOUT_PREFIX.to_vec();
    expected_stdout.extend_from_slice(PARENT_WAIT_REFUSAL_STDOUT_FINAL);
    let mut expected_stderr = PARENT_WAIT_REFUSAL_STDERR_PREFIX.to_vec();
    expected_stderr.extend_from_slice(PARENT_WAIT_REFUSAL_STDERR_FINAL);

    // PHASE 1 — HEALTHY CONTROL under the same preconditions, REAL finish.
    let healthy_stdout_path = healthy_leaf.join("stdout.raw");
    let healthy_stderr_path = healthy_leaf.join("stderr.raw");
    let healthy = start_parent_guard_capture(
        spawn_parent_wait_refusal_child(),
        &healthy_stdout_path,
        &healthy_stderr_path,
    );
    let (healthy_exit, healthy_stdout_state, healthy_stderr_state) = match healthy.finish() {
        paea_capture_child::ChildCaptureOutcome::Finished {
            exit,
            stdout_capture,
            stderr_capture,
            ..
        } => (exit, stdout_capture, stderr_capture),
        paea_capture_child::ChildCaptureOutcome::WaitRefused(handback) => {
            fail_parent_wait_refused(conserve_parent_wait_refused_carrier(
                PARENT_GUARD_HEALTHY_CONTEXT,
                PARENT_GUARD_WITNESS,
                PARENT_GUARD_MANIFEST,
                &healthy_leaf,
                &selection,
                handback,
                None,
            ));
        }
    };
    assert_eq!(
        healthy_exit.code(),
        Some(PARENT_WAIT_REFUSAL_EXIT_CODE),
        "the healthy control child must terminate with its constant real exit code"
    );
    assert_parent_guard_completed(
        &healthy_stdout_state,
        expected_stdout.len(),
        "healthy stdout capture",
    );
    assert_parent_guard_completed(
        &healthy_stderr_state,
        expected_stderr.len(),
        "healthy stderr capture",
    );
    assert_eq!(
        fs::read(&healthy_stdout_path).expect("healthy stdout evidence must be readable"),
        expected_stdout,
        "the healthy control must retain the EXACT raw stdout bytes"
    );
    assert_eq!(
        fs::read(&healthy_stderr_path).expect("healthy stderr evidence must be readable"),
        expected_stderr,
        "the healthy control must retain the EXACT raw stderr bytes"
    );
    // The SAME receipt builder the WaitRefused route reuses must also
    // succeed on the healthy control's REAL Finished facts BEFORE any
    // fault is injected (not just the raw files): the builder reads the
    // exact `stdout.raw` / `stderr.raw` names this fresh private leaf
    // retains, so this success proves the per-phase leaf naming
    // precondition that the injected and collision phases rely on. The
    // child exited by its constant code (asserted above), so the signal
    // field is genuinely Null and success is genuinely false for exit
    // code 7 (never reported as a success); the terminal labels mirror
    // the real `Completed` variants asserted above.
    let healthy_receipt_outcome = json!({
        "finished": true,
        "exit_code": healthy_exit.code(),
        "signal": Value::Null,
        "success": healthy_exit.success(),
        "stdout_terminal": "Completed",
        "stdout_completed_bytes": expected_stdout.len(),
        "stderr_terminal": "Completed",
        "stderr_completed_bytes": expected_stderr.len(),
        "controller_panic": false,
    });
    let (healthy_receipt_path, _) = write_process_receipt(
        &healthy_leaf,
        PARENT_GUARD_WITNESS,
        PARENT_GUARD_MANIFEST,
        &selection,
        &healthy_receipt_outcome,
    )
    .expect(
        "the healthy control's receipt write must succeed on its REAL \
         Finished facts before any fault is injected",
    );
    let healthy_receipt: Value = serde_json::from_slice(
        &fs::read(&healthy_receipt_path)
            .expect("the healthy control's receipt must be readable"),
    )
    .expect("the healthy control's receipt must be valid JSON");
    assert_eq!(
        healthy_receipt["process"]["finished"], json!(true),
        "the healthy control's receipt must record its real Finished facts"
    );
    assert_eq!(
        healthy_receipt["process"]["exit_code"], json!(PARENT_WAIT_REFUSAL_EXIT_CODE),
        "the healthy control's receipt must record the real constant exit code"
    );
    assert_eq!(
        healthy_receipt["process"]["success"], json!(false),
        "the healthy control's receipt must record the real success flag \
         (exit code 7 is NOT a success)"
    );
    assert_eq!(
        healthy_receipt["artifacts"]["stdout"]["bytes"], json!(expected_stdout.len()),
        "the healthy control's receipt must account for the real retained stdout bytes"
    );
    assert_eq!(
        healthy_receipt["artifacts"]["stderr"]["bytes"], json!(expected_stderr.len()),
        "the healthy control's receipt must account for the real retained stderr bytes"
    );

    // PHASE 2 — ONE explicit injected wait refusal BEFORE any real wait
    // (actual FnOnce seam; test seam only, NOT a real OS wait failure).
    let injected_stdout_path = injected_leaf.join("stdout.raw");
    let injected_stderr_path = injected_leaf.join("stderr.raw");
    let injected = start_parent_guard_capture(
        spawn_parent_wait_refusal_child(),
        &injected_stdout_path,
        &injected_stderr_path,
    );
    let seam_invocations = std::cell::Cell::new(0u32);
    let seam_child_pid = std::cell::Cell::new(0u32);
    let seam_invocations_ref = &seam_invocations;
    let seam_child_pid_ref = &seam_child_pid;
    let injected_cause =
        format!("{PARENT_WAIT_REFUSAL_SEAM_MARKER} injected before any real wait");
    let injected_cause_text = injected_cause.clone();
    let injected_outcome = injected.finish_with_wait(move |handler_child| {
        seam_child_pid_ref.set(handler_child.id());
        seam_invocations_ref.set(seam_invocations_ref.get() + 1);
        Err(std::io::Error::new(std::io::ErrorKind::Other, injected_cause))
    });
    let handback = match injected_outcome {
        paea_capture_child::ChildCaptureOutcome::WaitRefused(handback) => handback,
        paea_capture_child::ChildCaptureOutcome::Finished { exit, .. } => panic!(
            "the SYNTHETIC seam handler must refuse the wait; the finish \
             must NOT report a Finished outcome: {exit:?}"
        ),
    };
    // Conservation with the SAME helper the production WaitRefused arm
    // uses — joins BEFORE the receipt, UNKNOWN envelope, presence-only
    // panic record, receipt success path as data, and the DISTINCTIVE
    // original controller-panic payload marker INSIDE the carrier.
    let conserved = conserve_parent_wait_refused_carrier(
        PARENT_GUARD_INJECTED_CONTEXT,
        PARENT_GUARD_WITNESS,
        PARENT_GUARD_MANIFEST,
        &injected_leaf,
        &selection,
        handback,
        Some(Box::new(PARENT_WAIT_REFUSAL_PANIC_MARKER.to_string())
            as Box<dyn std::any::Any + Send>),
    );
    // THE REAL TYPED BOUNDARY — failed with the SAME function the
    // production WaitRefused arm uses, inside a test catch_unwind; the
    // boxed payload is recovered BY VALUE (the default panic hook may
    // print during the catch — no global hook mutation is allowed).
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        fail_parent_wait_refused(conserved);
    }));
    let boxed_payload = caught.expect_err(
        "the typed refusal failure must really panic with the conserved \
         carrier as its payload",
    );
    let carrier = boxed_payload
        .downcast::<ParentWaitRefusedCarrier>()
        .expect(
            "the typed panic payload must downcast BY VALUE into the \
             complete ParentWaitRefusedCarrier",
        );
    // Every field destructured BY MOVE out of the recovered box — nothing
    // cloned, no cause re-creation, no borrow replacing the ownership
    // transfer. Facts are snapshotted as data BEFORE any verdict.
    let ParentWaitRefusedCarrier {
        context: carrier_context,
        error: recovered_error,
        mut child,
        stdout_state,
        stderr_state,
        stdout_path: recovered_stdout_path,
        stderr_path: recovered_stderr_path,
        controller_panic: recovered_panic,
        receipt: recovered_receipt,
    } = *carrier;
    let recovered_child_pid = child.id();
    // EXPLICIT RECEIVER RESOLUTION FIRST — immediately after the BY-VALUE
    // destructure and BEFORE every verdict and evidence read: the injected
    // seam handler never reached the real `Child::wait`, so this is the
    // FIRST real wait of this child (a deliberate receiver decision for
    // the synthetic refusal-before-actual-wait case — no automatic wait,
    // retry, signal or kill exists anywhere in this guard). The REAL exit
    // code is snapshotted now, so even a failing later verdict cannot
    // drop the unreaped child and its real runtime issue stays retained.
    let recovered_exit_code = child
        .wait()
        .expect("the FIRST real wait of the injected child must succeed")
        .code();
    let recovered_cause_text = recovered_error.to_string();
    let recovered_cause_kind = recovered_error.kind();
    let seam_invocation_count = seam_invocations.get();
    let seam_seen_pid = seam_child_pid.get();
    let recovered_panic_present = recovered_panic.is_some();
    let receipt_path = match &recovered_receipt {
        Ok(path) => path.clone(),
        Err(text) => panic!(
            "the injected guard's receipt write must succeed on the clean \
             leaf; unexpected secondary receipt failure conserved as data: {text}"
        ),
    };
    let marker_text = match recovered_panic {
        Some(payload) => match payload.downcast::<String>() {
            Ok(text) => *text,
            Err(_) => panic!(
                "the original controller panic payload must keep its exact \
                 type through the typed boundary"
            ),
        },
        None => panic!(
            "the original controller panic payload must travel INSIDE the \
             carrier, not be dropped"
        ),
    };
    // FACT VERDICTS — all data-level, nothing invented:
    assert_eq!(
        carrier_context, PARENT_GUARD_INJECTED_CONTEXT,
        "the failing-site context label must travel with the carrier"
    );
    assert_eq!(
        seam_invocation_count, 1,
        "the FnOnce seam handler is the ONE wait attempt of this finish \
         (no retry, no automatic reattempt)"
    );
    assert_eq!(
        seam_seen_pid, recovered_child_pid,
        "the SAME original child (same pid) must travel through the \
         handback and the typed carrier"
    );
    assert_eq!(
        recovered_cause_text, injected_cause_text,
        "the original non-Clone refusal cause must reach the receiver \
         verbatim (moved, never copied or re-created)"
    );
    assert_eq!(
        recovered_cause_kind,
        std::io::ErrorKind::Other,
        "the recovered cause must keep its real error kind"
    );
    assert!(
        recovered_panic_present,
        "the original controller panic payload must be present"
    );
    assert_eq!(
        marker_text, PARENT_WAIT_REFUSAL_PANIC_MARKER,
        "the original controller panic payload must survive BY VALUE, \
         verbatim and downcastable"
    );
    assert!(
        receipt_path.starts_with(&injected_leaf),
        "the receipt must be written inside the guard's private leaf"
    );
    assert_parent_guard_completed(
        &stdout_state,
        expected_stdout.len(),
        "injected stdout capture",
    );
    assert_parent_guard_completed(
        &stderr_state,
        expected_stderr.len(),
        "injected stderr capture",
    );
    assert_eq!(
        fs::read(&recovered_stdout_path).expect("injected stdout evidence must be readable"),
        expected_stdout,
        "the REAL raw stdout bytes must be retained through the typed boundary"
    );
    assert_eq!(
        fs::read(&recovered_stderr_path).expect("injected stderr evidence must be readable"),
        expected_stderr,
        "the REAL raw stderr bytes must be retained through the typed boundary"
    );
    // UNKNOWN exit receipt: the envelope is explicitly not-finished with
    // no invented code/signal/success/terminal state, records the
    // controller-panic PRESENCE the route actually knows, and NEVER
    // carries the raw payload text.
    let receipt_raw =
        fs::read(&receipt_path).expect("the retained process receipt must be readable");
    let receipt_json: Value = serde_json::from_slice(&receipt_raw)
        .expect("the retained process receipt must be valid JSON");
    assert_eq!(
        receipt_json["process"]["finished"], json!(false),
        "the receipt must record the UNKNOWN / not-finished envelope"
    );
    assert_eq!(
        receipt_json["process"]["exit_code"], Value::Null,
        "the exit status stays UNKNOWN: no invented code"
    );
    assert_eq!(
        receipt_json["process"]["signal"], Value::Null,
        "the exit status stays UNKNOWN: no invented signal"
    );
    assert_eq!(
        receipt_json["process"]["success"], Value::Null,
        "the exit status stays UNKNOWN: no invented success flag"
    );
    assert_eq!(
        receipt_json["process"]["stdout_terminal"], Value::Null,
        "the UNKNOWN envelope invents no terminal state"
    );
    assert_eq!(
        receipt_json["process"]["stderr_terminal"], Value::Null,
        "the UNKNOWN envelope invents no terminal state"
    );
    assert_eq!(
        receipt_json["process"]["controller_panic"], json!(true),
        "the UNKNOWN envelope must record the controller-panic PRESENCE \
         the route actually knows"
    );
    assert_eq!(
        receipt_json["artifacts"]["stdout"]["bytes"], json!(expected_stdout.len()),
        "the receipt must account for the REAL retained stdout bytes"
    );
    assert_eq!(
        receipt_json["artifacts"]["stderr"]["bytes"], json!(expected_stderr.len()),
        "the receipt must account for the REAL retained stderr bytes"
    );
    let receipt_text = String::from_utf8(receipt_raw).expect("the receipt must be UTF-8");
    assert!(
        !receipt_text.contains(PARENT_WAIT_REFUSAL_PANIC_MARKER),
        "the receipt must NEVER contain the raw controller-panic payload text"
    );
    // RECEIVER VERDICT on the snapshotted REAL exit — the child was
    // already reaped immediately after the BY-VALUE destructure, so no
    // failing assertion above can orphan it.
    assert_eq!(
        recovered_exit_code,
        Some(PARENT_WAIT_REFUSAL_EXIT_CODE),
        "the real exit code must be retained after the explicit first wait — no orphan"
    );

    // PHASE 3 — SECONDARY RECEIPT FAILURE AS DATA: a fresh leaf where the
    // receipt target path is pre-created as a real DIRECTORY, so the
    // existing write_process_receipt REALLY refuses at its atomic rename
    // (actual directory rename collision, feasible inside the same
    // bounded guard). The refused temp receipt file and the collision
    // directory stay in the private failure leaf as retained evidence.
    let collision_leaf = private_fixture_leaf("parent-wait-refusal-receipt-collision");
    fs::create_dir(collision_leaf.join("process-receipt.json"))
        .expect("the collision leaf must accept the receipt-path directory");
    let collision_stdout_path = collision_leaf.join("stdout.raw");
    let collision_stderr_path = collision_leaf.join("stderr.raw");
    let collision = start_parent_guard_capture(
        spawn_parent_wait_refusal_child(),
        &collision_stdout_path,
        &collision_stderr_path,
    );
    let collision_seam_invocations = std::cell::Cell::new(0u32);
    let collision_seam_ref = &collision_seam_invocations;
    let collision_cause = format!(
        "{PARENT_WAIT_REFUSAL_SEAM_MARKER} collision branch injected before any real wait"
    );
    let collision_cause_text = collision_cause.clone();
    let collision_outcome = collision.finish_with_wait(move |handler_child| {
        let _ = handler_child.id();
        collision_seam_ref.set(collision_seam_ref.get() + 1);
        Err(std::io::Error::new(std::io::ErrorKind::Other, collision_cause))
    });
    let collision_handback = match collision_outcome {
        paea_capture_child::ChildCaptureOutcome::WaitRefused(handback) => handback,
        paea_capture_child::ChildCaptureOutcome::Finished { exit, .. } => panic!(
            "the SYNTHETIC collision seam handler must refuse the wait: {exit:?}"
        ),
    };
    let collision_conserved = conserve_parent_wait_refused_carrier(
        PARENT_GUARD_COLLISION_CONTEXT,
        PARENT_GUARD_WITNESS,
        PARENT_GUARD_MANIFEST,
        &collision_leaf,
        &selection,
        collision_handback,
        None,
    );
    let collision_caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        fail_parent_wait_refused(collision_conserved);
    }));
    let collision_carrier = collision_caught
        .expect_err(
            "the collision typed refusal failure must really panic with the \
             conserved carrier as its payload",
        )
        .downcast::<ParentWaitRefusedCarrier>()
        .expect(
            "the collision typed panic payload must downcast BY VALUE into \
             the complete ParentWaitRefusedCarrier",
        );
    let ParentWaitRefusedCarrier {
        error: collision_error,
        child: mut collision_child,
        stdout_state: collision_stdout_state,
        stderr_state: collision_stderr_state,
        receipt: collision_receipt,
        ..
    } = *collision_carrier;
    // EXPLICIT RECEIVER RESOLUTION FIRST — immediately after the BY-VALUE
    // destructure and BEFORE every verdict and evidence read: the FIRST
    // real wait of this child (a deliberate receiver decision, never an
    // automatic reattempt — no automatic wait, retry, signal or kill
    // exists anywhere in this guard). The REAL exit code is snapshotted
    // now, so even a failing later verdict cannot drop the unreaped child
    // and its real runtime issue stays retained.
    let collision_exit_code = collision_child
        .wait()
        .expect("the FIRST real wait of the collision child must succeed")
        .code();
    let collision_receipt_text = match &collision_receipt {
        Ok(path) => panic!(
            "the receipt rename collision must FAIL the receipt write; \
             unexpected success: {}",
            path.display()
        ),
        Err(text) => text.clone(),
    };
    assert!(
        collision_receipt_text.starts_with("receipt atomic rename refused:"),
        "the secondary receipt failure must be the REAL stage-tagged rename \
         refusal conserved as data: {collision_receipt_text}"
    );
    assert_eq!(
        collision_error.to_string(),
        collision_cause_text,
        "the non-Clone refusal cause must also survive the collision branch verbatim"
    );
    assert_parent_guard_completed(
        &collision_stdout_state,
        expected_stdout.len(),
        "collision stdout capture",
    );
    assert_parent_guard_completed(
        &collision_stderr_state,
        expected_stderr.len(),
        "collision stderr capture",
    );
    assert_eq!(
        collision_seam_invocations.get(), 1,
        "ONE wait attempt per finish on the collision branch too"
    );
    // RECEIVER VERDICT on the snapshotted REAL exit — the child was
    // already reaped immediately after the BY-VALUE destructure, so no
    // failing assertion above can orphan it.
    assert_eq!(
        collision_exit_code,
        Some(PARENT_WAIT_REFUSAL_EXIT_CODE),
        "the collision child's real exit code must be retained — no orphan"
    );
}

/// Archive one byte-exact proof copy inside an existing private proof leaf.
///
/// Purpose: retain exact receipt bytes OUTSIDE the tested leaf so the
/// healthy copy survives the later writer overwrite of the tested-leaf
/// target and the recovered copy is preserved alongside it for the next
/// dependent task to re-read.
///
/// Input: `proof_leaf` is an existing 0700 private leaf created by
/// `private_fixture_leaf`; `file_name` is a single path component used
/// verbatim; `bytes` are the exact bytes to preserve.
///
/// Output: the `PathBuf` of the newly created copy inside `proof_leaf`.
///
/// Errors: any creation, write, flush, or fsync failure panics with the
/// underlying message; nothing is retried or swallowed, so a fresh
/// partial proof copy fails the guard honestly.
///
/// Side effects: creates exactly one new file, mode 0600 at creation via
/// `create_new` (duplicate creation is refused), then write_all, flush, and
/// sync_all; the tested leaf and its final EXACT 3 files are untouched.
///
/// Limits: single-purpose for the healthy and recovery receipt copies;
/// not a general copy framework; does not read contents back (the
/// existing readback tests stay as is); the caller must keep both
/// copies inside the SAME proof leaf.
fn archive_proof_copy(proof_leaf: &Path, file_name: &str, bytes: &[u8]) -> PathBuf {
    let target = proof_leaf.join(file_name);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&target)
        .unwrap_or_else(|e| panic!("proof copy create refused for {file_name}: {e}"));
    file.write_all(bytes)
        .unwrap_or_else(|e| panic!("proof copy write failed for {file_name}: {e}"));
    file.flush()
        .unwrap_or_else(|e| panic!("proof copy flush failed for {file_name}: {e}"));
    file.sync_all()
        .unwrap_or_else(|e| panic!("proof copy fsync failed for {file_name}: {e}"));
    target
}

/// Digest a fault-inventory snapshot into a stable lowercase-hex SHA-256.
///
/// Purpose: let the fault-observation manifest record an actual before/after
/// raw inventory hash so a reviewer can verify the tested leaf stayed
/// byte-identical across the fault window instead of trusting a bare equality
/// flag.
///
/// Inputs: `inventory` - a snapshot of `(file name, file bytes)` pairs ordered
/// by file name (the guard passes a name-sorted canonical copy of its
/// `before_fault` capture, or a post-restore capture from
/// `paea_receipt_create_fault_inventory_checked`).
///
/// Results: a 64-character lowercase hex digest computed over, for each pair
/// in order: the name bytes, one NUL separator byte, the byte length as
/// little-endian u64, then the file bytes.
///
/// Errors: none; pure computation over in-memory data.
///
/// Side effects: none.
///
/// Limits: a test-observation aid, not a security boundary; strength is
/// bounded by SHA-256 and by snapshot ordering, so callers must feed snapshots
/// built with the same name-sorted ordering or identical directories can
/// yield different digests.
fn paea_receipt_create_fault_inventory_digest(inventory: &[(String, Vec<u8>)]) -> String {
    let mut hasher = Sha256::new();
    for (name, bytes) in inventory {
        hasher.update(name.as_bytes());
        hasher.update([0u8]);
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Fallibly capture the tested leaf's flat file inventory for fault records.
///
/// Purpose: mirror the panicking private inventory helper for the post-restore
/// observation, but report read failures as `Err` data so an unexpected
/// inventory failure cannot erase the already-captured primary and restore
/// causes with a panic.
///
/// Inputs: `leaf` - the tested fixture directory; only its direct entries are
/// read, as raw bytes.
///
/// Results: `Ok` with the inventory sorted by file name as
/// `Vec<(String, Vec<u8>)>`, the same shape the private helper returns on the
/// happy path.
///
/// Errors: `Err(String)` naming the exact failing stage (`read_dir`, directory
/// entry read, non-UTF8 file name, or per-file `read`) with the underlying
/// `std::io::Error` rendered into the string; nothing panics.
///
/// Side effects: none; strictly read-only, never writes to the tested leaf.
///
/// Limits: one directory level, no recursion; any non-UTF8 name or read error
/// fails the whole capture rather than being skipped, matching the panicking
/// helper's strictness; entries that are not readable regular files surface as
/// per-file read errors.
fn paea_receipt_create_fault_inventory_checked(
    leaf: &Path,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut names: Vec<String> = Vec::new();
    let entries =
        fs::read_dir(leaf).map_err(|e| format!("read_dir {}: {e}", leaf.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| format!("read_dir entry in {}: {e}", leaf.display()))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|raw| format!("non-UTF8 file name {raw:?}"))?;
        names.push(name);
    }
    names.sort();
    let mut inventory: Vec<(String, Vec<u8>)> = Vec::new();
    for name in names {
        let bytes = fs::read(leaf.join(&name)).map_err(|e| format!("read {name}: {e}"))?;
        inventory.push((name, bytes));
    }
    Ok(inventory)
}

/// Persist the fault-observation manifest as JSON mode 0600 and echo it.
///
/// Purpose: retain the actual fault facts (primary stage/cause, observed probe
/// errno, euid/owner/mode facts, actual restore result, before/after raw
/// inventory digests and equality) on disk in the external proof leaf BEFORE
/// any verdict assertion, and print the manifest and its path to stderr so
/// Codex can re-read them after stability.
///
/// Inputs: `proof_leaf` - the existing exclusive 0700 proof directory external
/// to the tested leaf; `observations` - the fully assembled JSON value to
/// persist verbatim.
///
/// Results: `Ok(PathBuf)` with the manifest path after the file has been
/// created, written, flushed and fsynced.
///
/// Errors: `Err(String)` naming the exact failing stage (JSON serialization,
/// create, write, flush, or fsync) with the underlying error; deliberately
/// returns instead of panicking so the caller can report the primary and
/// restore causes together with this secondary proof-write failure.
///
/// Side effects: creates exactly one new file
/// `paea-receipt-create-fault-observations.json` (mode 0600 at creation via
/// `create_new`) inside `proof_leaf`, and writes two stderr lines (the path,
/// then the JSON manifest). Never touches the tested leaf.
///
/// Limits: no retry, no permission fallback, no overwrite - a pre-existing
/// manifest makes `create_new` fail and the caller must fail honestly with all
/// causes; the 0600 mode is applied at creation only and is not re-chmodded;
/// the stored file is not read back for validation.
fn paea_receipt_create_fault_observations_write(
    proof_leaf: &Path,
    observations: &Value,
) -> Result<PathBuf, String> {
    let manifest_path = proof_leaf.join("paea-receipt-create-fault-observations.json");
    let bytes = serde_json::to_vec_pretty(observations)
        .map_err(|e| format!("serialize fault observations JSON: {e}"))?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&manifest_path)
        .map_err(|e| format!("create fault observations {}: {e}", manifest_path.display()))?;
    file.write_all(&bytes)
        .map_err(|e| format!("write fault observations {}: {e}", manifest_path.display()))?;
    file.flush()
        .map_err(|e| format!("flush fault observations {}: {e}", manifest_path.display()))?;
    file.sync_all()
        .map_err(|e| format!("fsync fault observations {}: {e}", manifest_path.display()))?;
    eprintln!(
        "paea-receipt-create fault observations manifest path: {}",
        manifest_path.display()
    );
    eprintln!("paea-receipt-create fault observations manifest: {observations}");
    Ok(manifest_path)
}

/// Byte-exact inventory of every entry directly inside `leaf`, sorted by
/// file name: two equal inventories mean the same names with the same
/// contents, so a missing or leftover receipt temp file - or any mutated
/// byte - shows up as a mismatch instead of a guessed temp filename. Only
/// call this while `leaf` is at 0700, so a panic here can never leave the
/// owned fixture unrestored.
fn paea_receipt_create_inventory(leaf: &Path) -> Vec<(String, Vec<u8>)> {
    let mut names: Vec<String> = fs::read_dir(leaf)
        .unwrap_or_else(|e| panic!("inventory read_dir refused for {leaf:?}: {e}"))
        .map(|entry| {
            entry.unwrap_or_else(|e| {
                panic!("inventory read_dir entry refused for {leaf:?}: {e}")
            })
        })
        .map(|entry| {
            entry
                .file_name()
                .into_string()
                .unwrap_or_else(|raw| panic!("inventory filename not UTF-8 for {leaf:?}: {raw:?}"))
        })
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let bytes = fs::read(leaf.join(&name))
                .unwrap_or_else(|e| panic!("inventory read refused for {leaf:?}/{name}: {e}"));
            (name, bytes)
        })
        .collect()
}

/// Explicit refusal - never a PASS - when the same-uid/0700 OS precondition
/// of the 0500 temp-create refusal does not apply. The already-attempted
/// restore outcome is embedded as data so the owned fixture's state is
/// never silent.
fn paea_receipt_create_precondition_refusal(leaf: &Path, why: &str, restore: std::io::Result<()>) -> ! {
    panic!(
        "receipt temp-create precondition not applicable on {leaf:?}: {why}; restore-to-0700 outcome {restore:?}; refusing instead of passing"
    );
}

/// Receipt temp-CREATE refusal on a fresh private leaf, TEST-ONLY bounded
/// slice: after a real healthy receipt write/readback with controlled
/// `stdout.raw`/`stderr.raw`, the same-uid owner drops the owned 0700
/// fixture directory to 0500 so the existing raw evidence stays readable
/// while the unchanged receipt writer's exclusive `create_new` temp open is
/// refused by the OS (EACCES, os error 13) at the known stage `receipt temp
/// creation refused` - a fresh-creation refusal, never a rename collision,/// and the nanosecond temp filename is never guessed. The owned fixture is
/// restored to 0700 BEFORE any assertion that could panic, any secondary
/// restore error is preserved as data, and an explicit fresh recovery write
/// with a distinct payload is then read back. Limitations surfaced, not
/// hidden: the writer reports stage and cause as a single `Err(String)`,
/// not a typed OS error chain, so the errno is validated through std's
/// locale-independent `os error 13` display; the UID precondition is
/// explicit and refuses safely instead of passing when not applicable;
/// child-wait, live HTTP, installation, and Authority acceptance coverage
/// is out of scope. No synthetic OS error, no new uid, no live root, no
/// hardcoded /tmp, no inferred timestamp, no retry of unchanged input, no
/// automatic kill, no timeout; raw evidence and all receipts stay on disk
/// on failure and on PASS.
#[test]
fn paea_capture_receipt_temp_create_refusal_keeps_stage_inventory_and_recovers_fresh() {
    use std::os::unix::fs::MetadataExt;

    // Fresh exclusive-0700 private service-storage leaf under the explicit
    // TMPDIR, preserved on disk: never a user mission, never TempDir, never
    // a hardcoded /tmp.
    let leaf: PathBuf = private_fixture_leaf("receipt-temp-create-guard");

    // Controlled evidence the receipt will conserve - real files, real
    // bytes, written before anything is faulted.
    let stdout_bytes = b"controlled stdout prologue for the receipt temp-create refusal guard\n";
    let stderr_bytes = b"controlled stderr prologue for the receipt temp-create refusal guard\n";
    fs::write(leaf.join("stdout.raw"), stdout_bytes)
        .unwrap_or_else(|e| panic!("controlled stdout.raw write refused on fresh fixture {leaf:?}: {e}"));
    fs::write(leaf.join("stderr.raw"), stderr_bytes)
        .unwrap_or_else(|e| panic!("controlled stderr.raw write refused on fresh fixture {leaf:?}: {e}"));

    // Same-uid ownership precondition, with the process's actual effective
    // uid read directly via rustix::process::geteuid() rather than inferred
    // from a probe file. The fixture directory must be 0700 and owned by
    // that same uid; otherwise the 0500 refusal below would not be the
    // observed OS precondition and this guard refuses instead of passing.
    let euid = rustix::process::geteuid().as_raw();
    let leaf_meta = fs::metadata(&leaf).unwrap_or_else(|e| panic!("fixture metadata refused on {leaf:?}: {e}"));
    let dir_uid = leaf_meta.uid();
    let dir_mode_bits = leaf_meta.permissions().mode() & 0o777;
    if dir_mode_bits != 0o700 || dir_uid != euid {
        panic!(
            "receipt temp-create precondition not applicable on {leaf:?}: directory mode {dir_mode_bits:o} uid {dir_uid} versus effective uid {euid}, expected 0700 owned by this process; refusing instead of passing"
        );
    }

    // Healthy actual receipt write/readback BEFORE any fault: the unchanged
    // writer runs for real on the owned 0700 fixture, and its conserved
    // hashes are validated against the controlled bytes, not just an exit
    // code.
    let witness = "paea-capture-receipt-temp-create-refusal";
    let selection = json!({
        "guard": "paea_capture_receipt_temp_create_refusal_keeps_stage_inventory_and_recovers_fresh",
        "stage": "healthy-prologue",
    });
    let process_outcome = json!({"fixture_only": true, "phase": "healthy-prologue"});
    let (healthy_receipt_path, healthy_receipt_leaf) = write_process_receipt(
        &leaf,
        witness,
        "paea-capture-receipt-temp-create-refusal/healthy-prologue",
        &selection,
        &process_outcome,
    )
    .unwrap_or_else(|e| panic!("healthy receipt write refused before any fault: {e}"));
    assert_eq!(
        healthy_receipt_path,
        leaf.join("process-receipt.json"),
        "the healthy receipt must land at the writer's real target path"
    );
    assert_eq!(healthy_receipt_leaf, leaf, "the healthy receipt leaf must be the private fixture");
    let expected_stdout_sha = format!("{:x}", Sha256::digest(stdout_bytes));
    let expected_stderr_sha = format!("{:x}", Sha256::digest(stderr_bytes));
    let healthy_receipt_bytes = fs::read(&healthy_receipt_path)
        .unwrap_or_else(|e| panic!("healthy receipt readback refused: {e}"));
    // Retain an exact copy of the healthy receipt outside the tested leaf
    // before the writer later overwrites the tested-leaf target.
    let proof_leaf = private_fixture_leaf("receipt-proof-copies");
    let healthy_proof_copy = archive_proof_copy(
        &proof_leaf,
        "healthy-receipt.bin",
        &healthy_receipt_bytes,
    );
    eprintln!(
        "{}",
        json!({
            "locator": "healthy_receipt_copy",
            "proof_leaf": proof_leaf.display().to_string(),
            "file": "healthy-receipt.bin",
            "path": healthy_proof_copy.display().to_string(),
            "len": healthy_receipt_bytes.len()
        })
    );
    let healthy_receipt: Value = serde_json::from_slice(&healthy_receipt_bytes)
        .unwrap_or_else(|e| panic!("healthy receipt readback is not JSON: {e}"));
    assert_eq!(healthy_receipt["schema_version"], json!(1), "healthy receipt schema_version");
    assert_eq!(healthy_receipt["witness"], json!(witness), "healthy receipt witness");
    assert_eq!(healthy_receipt["selection"], selection, "healthy receipt selection");
    assert_eq!(healthy_receipt["process"], process_outcome, "healthy receipt process outcome");
    assert_eq!(healthy_receipt["artifacts"]["stdout"]["path"], json!("stdout.raw"), "healthy receipt stdout path");
    assert_eq!(
        healthy_receipt["artifacts"]["stdout"]["sha256"],
        json!(expected_stdout_sha),
        "healthy receipt stdout sha256 must be the real hash of the controlled bytes"
    );
    assert_eq!(healthy_receipt["artifacts"]["stdout"]["bytes"], json!(stdout_bytes.len()), "healthy receipt stdout byte count");
    assert_eq!(healthy_receipt["artifacts"]["stderr"]["path"], json!("stderr.raw"), "healthy receipt stderr path");
    assert_eq!(
        healthy_receipt["artifacts"]["stderr"]["sha256"],
        json!(expected_stderr_sha),
        "healthy receipt stderr sha256 must be the real hash of the controlled bytes"
    );
    assert_eq!(healthy_receipt["artifacts"]["stderr"]["bytes"], json!(stderr_bytes.len()), "healthy receipt stderr byte count");

    // Byte-exact snapshot while everything is healthy and the leaf is still
    // 0700, taken BEFORE the fault, so the post-refusal comparison proves
    // the refusal created no temp file and mutated no byte.
    let before_fault = paea_receipt_create_inventory(&leaf);

    // The fault: the same-uid owner drops the directory to 0500 - existing
    // raw files stay readable, creation does not.
    fs::set_permissions(&leaf, fs::Permissions::from_mode(0o500))
        .unwrap_or_else(|e| panic!("chmod 0500 refused on owned fixture {leaf:?}: {e}"));

    // OS precondition gate under 0500, BEFORE the writer runs: the existing
    // raw stays readable, and an exclusive create of our OWN probe name
    // (never a guessed temp name) is refused with EACCES 13. If either half
    // does not hold - for example under a supervisor that bypasses
    // directory permissions - restore 0700 and refuse explicitly instead
    // of passing.
    if let Err(read_err) = fs::read(leaf.join("stdout.raw")) {
        let restore = fs::set_permissions(&leaf, fs::Permissions::from_mode(0o700));
        paea_receipt_create_precondition_refusal(
            &leaf,
            &format!("existing stdout.raw not readable under 0500: {read_err}"),
            restore,
        );
    }
    let probe_path = leaf.join(".paea-receipt-create-probe");
    let probe_open = fs::OpenOptions::new().write(true).create_new(true).open(&probe_path);
    // The probe's actual error is data: capture the raw errno before matching
    // so the expected-EACCES arm cannot silently drop the local Err, and so
    // the fault-observation manifest below records what really happened.
    let observed_probe_errno: Option<i32> = match &probe_open {
        Ok(_) => None,
        Err(probe_err) => probe_err.raw_os_error(),
    };
    match probe_open {
        Ok(_) => {
            let remove = fs::remove_file(&probe_path);
            let restore = fs::set_permissions(&leaf, fs::Permissions::from_mode(0o700));
            paea_receipt_create_precondition_refusal(
                &leaf,
                &format!("exclusive create under 0500 unexpectedly succeeded (probe removal outcome {remove:?}); a supervisor or uid bypasses directory permissions"),
                restore,
            );
        }
        Err(probe_err) => {
            if probe_err.raw_os_error() != Some(13) {
                let restore = fs::set_permissions(&leaf, fs::Permissions::from_mode(0o700));
                paea_receipt_create_precondition_refusal(
                    &leaf,
                    &format!("exclusive create under 0500 failed with {probe_err} (raw_os_error {:?}), expected EACCES 13", probe_err.raw_os_error()),
                    restore,
                );
            }
        }
    }

    // The actual refusal under test: the unchanged writer must fail at its
    // temp-CREATE stage on this fresh leaf. No temp name is guessed and no
    // temp pre-exists, so this cannot be a rename collision; the primary
    // stage/cause is captured as data first, before any assertion.
    let primary: String = match write_process_receipt(
        &leaf,
        witness,
        "paea-capture-receipt-temp-create-refusal/fault",
        &selection,
        &process_outcome,
    ) {
        Ok(_) => {
            let restore = fs::set_permissions(&leaf, fs::Permissions::from_mode(0o700));
            panic!(
                "receipt write unexpectedly succeeded under 0500 after the refused EACCES probe; restore-to-0700 outcome {restore:?}; refusing instead of passing"
            );
        }
        Err(cause) => cause,
    };

    // Restore 0700 FIRST - before any assertion that could panic - so no
    // assertion failure can ever leave the owned fixture unrestored. A
    // secondary restore error, if any, is preserved as data.
    let restore_error: Option<String> =
        match fs::set_permissions(&leaf, fs::Permissions::from_mode(0o700)) {
            Ok(()) => None,
            Err(e) => Some(format!("chmod 0700 restore refused on {leaf:?}: {e}")),
        };
    // Fault observations are captured and persisted BEFORE any verdict
    // assertion and BEFORE the restore-refusal panic below, so neither a
    // failed restore nor a later assertion failure can erase the observed
    // facts. The restore attempt above still precedes every post-fault read.
    // The primary is the writer's Err(String) cause, captured verbatim: no
    // structured stage exists, and that String-stage limitation is recorded
    // as data in the manifest. Post-restore reads are fallible on purpose so
    // an unexpected read failure cannot replace the primary/restore causes.
    let after_restore_inventory: Result<Vec<(String, Vec<u8>)>, String> =
        paea_receipt_create_fault_inventory_checked(&leaf);
    // Digests and equality use a name-sorted canonical copy of the before
    // snapshot so the comparison cannot depend on the private helper's
    // internal ordering.
    let mut before_fault_canonical = before_fault.clone();
    before_fault_canonical.sort_by(|a, b| a.0.cmp(&b.0));
    let before_fault_sha256 = paea_receipt_create_fault_inventory_digest(&before_fault_canonical);
    let (after_fault_sha256, after_fault_error, inventories_equal, after_fault_file_count): (
        Option<String>,
        Option<String>,
        Option<bool>,
        Option<usize>,
    ) = match &after_restore_inventory {
        Ok(after) => (
            Some(paea_receipt_create_fault_inventory_digest(after)),
            None,
            Some(before_fault_canonical == *after),
            Some(after.len()),
        ),
        Err(cause) => (None, Some(cause.clone()), None, None),
    };
    let (actual_mode, meta_error): (Option<u32>, Option<String>) = match fs::metadata(&leaf) {
        Ok(m) => (Some(m.permissions().mode() & 0o7777), None),
        Err(meta_err) => (None, Some(meta_err.to_string())),
    };
    let fault_observations = json!({
        "guard": "paea-receipt-create-temp-create-refusal/fault-observations",
        "phase": "fixture_only",
        "primary_stage_limitation":
            "primary is the writer's Err(String) cause, captured verbatim; no structured stage field exists, so the stage is whatever that string records",
        "primary_cause": &primary,
        "observed_probe_errno": observed_probe_errno,
        "observed_probe_errno_is_expected_eacces": observed_probe_errno == Some(13),
        "euid": euid,
        "leaf_owner_uid": dir_uid,
        "leaf_mode_bits_guard_captured": dir_mode_bits,
        "leaf_mode_bits_after_restore": {
            "actual_mode": actual_mode,
            "meta_error": meta_error
        },
        "restore_result": &restore_error,
        "restore_succeeded": restore_error.is_none(),
        "before_fault_inventory_sha256": &before_fault_sha256,
        "after_restore_inventory_sha256": &after_fault_sha256,
        "after_restore_inventory_error": &after_fault_error,
        "before_after_inventory_equal": inventories_equal,
        "inventory_comparison_note":
            "digests and equality are over name-sorted canonical order of (name, bytes) pairs",
        "before_fault_file_count": before_fault.len(),
        "after_restore_file_count": after_fault_file_count,
        "process_outcome_note":
            "fixture-only outcome data passed to the writer; this guard spawns no real child process and no real process outcome is claimed",
    });
    if let Err(proof_cause) =
        paea_receipt_create_fault_observations_write(&proof_leaf, &fault_observations)
    {
        panic!(
            "secondary fault-observation proof write refused; failing honestly with all causes as data; primary stage/cause {primary:?}; secondary restore error {restore_error:?}; proof write error {proof_cause}"
        );
    }
    if let Some(restore_cause) = &restore_error {
        panic!(
            "owned fixture could not be restored to 0700 before assertions; refusing with both causes as data; primary stage/cause {primary:?}; secondary restore error {restore_cause}; fault observations were persisted to the external proof leaf before this panic"
        );
    }

    // Known stage, preserved verbatim by the Err(String) writer contract.
    assert_eq!(
        actual_mode,
        Some(0o700),
        "known-stage post-restore mode mismatch: actual_mode={:?} meta_error={:?} primary={:?} restore_error={:?}",
        actual_mode,
        meta_error,
        primary,
        restore_error
    );
    assert!(
        meta_error.is_none(),
        "known-stage metadata observation failed: meta_error={:?} primary={:?} restore_error={:?}",
        meta_error,
        primary,
        restore_error
    );
    assert!(
        primary.starts_with("receipt temp creation refused: "),
        "expected the known receipt temp-creation stage in the primary cause, got {primary:?}"
    );
    // The writer returns Err(String), not a typed OS error chain, so the
    // errno is validated through the locale-independent `os error 13`
    // suffix that std's io::Error Display always appends for EACCES - the
    // limitation is surfaced, not hidden.
    assert!(
        primary.contains("os error 13"),
        "expected EACCES (os error 13) inside the temp-creation cause, got {primary:?}"
    );

    // No guessed temp name, no rename collision: the refusal must have left
    // the leaf byte-for-byte identical - same entries, same contents, no
    // temp remnant, raw evidence and healthy receipt conserved whole.
    let after_refusal = paea_receipt_create_inventory(&leaf);
    assert_eq!(
        after_refusal, before_fault,
        "leaf inventory must be byte-identical after the temp-create refusal (no temp file, no mutated byte); primary cause {primary}"
    );
    let stdout_after = fs::read(leaf.join("stdout.raw"))
        .unwrap_or_else(|e| panic!("stdout.raw readback after restore refused: {e}"));
    let stderr_after = fs::read(leaf.join("stderr.raw"))
        .unwrap_or_else(|e| panic!("stderr.raw readback after restore refused: {e}"));
    assert_eq!(stdout_after, &stdout_bytes[..], "controlled stdout.raw bytes unchanged by the refusal");
    assert_eq!(stderr_after, &stderr_bytes[..], "controlled stderr.raw bytes unchanged by the refusal");
    assert_eq!(
        format!("{:x}", Sha256::digest(&stdout_after)),
        expected_stdout_sha,
        "stdout.raw sha256 unchanged by the refusal"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&stderr_after)),
        expected_stderr_sha,
        "stderr.raw sha256 unchanged by the refusal"
    );

    // Explicit fresh recovery on the SAME restored leaf: a distinct payload
    // proves the recovered receipt was really written after the restore and
    // is not the stale healthy receipt; this is a different input state,
    // never a retry of the unchanged faulted input.
    let recovery_selection = json!({
        "guard": "paea_capture_receipt_temp_create_refusal_keeps_stage_inventory_and_recovers_fresh",
        "stage": "recovery-after-temp-create-refusal",
    });
    let recovery_outcome = json!({"fixture_only": true, "phase": "recovered-after-temp-create-refusal"});
    let (recovered_path, recovered_leaf) = write_process_receipt(
        &leaf,
        witness,
        "paea-capture-receipt-temp-create-refusal/recovery",
        &recovery_selection,
        &recovery_outcome,
    )
    .unwrap_or_else(|e| panic!("fresh recovery receipt write refused after restore to 0700: {e}"));
    assert_eq!(recovered_path, leaf.join("process-receipt.json"), "recovery receipt target path");
    assert_eq!(recovered_leaf, leaf, "recovery receipt leaf");
    let recovered_bytes = fs::read(&recovered_path)
        .unwrap_or_else(|e| panic!("recovery receipt readback refused: {e}"));
    // Archive the exact recovered bytes into the SAME private proof leaf
    // under a different exclusive name.
    let recovered_proof_copy = archive_proof_copy(
        &proof_leaf,
        "recovered-receipt.bin",
        &recovered_bytes,
    );
    eprintln!(
        "{}",
        json!({
            "locator": "recovered_receipt_copy",
            "proof_leaf": proof_leaf.display().to_string(),
            "file": "recovered-receipt.bin",
            "path": recovered_proof_copy.display().to_string(),
            "len": recovered_bytes.len()
        })
    );
    let recovered: Value = serde_json::from_slice(&recovered_bytes)
        .unwrap_or_else(|e| panic!("recovery receipt readback is not JSON: {e}"));
    assert_eq!(recovered["schema_version"], json!(1), "recovery receipt schema_version");
    assert_eq!(recovered["witness"], json!(witness), "recovery receipt witness");
    assert_eq!(
        recovered["selection"], recovery_selection,
        "recovery receipt must carry the distinct recovery payload"
    );
    assert_eq!(
        recovered["process"], recovery_outcome,
        "recovery receipt must carry the distinct recovery payload, proving the write happened after the restore"
    );
    assert_eq!(
        recovered["artifacts"]["stdout"]["sha256"],
        json!(expected_stdout_sha),
        "recovery receipt conserves the same controlled stdout bytes"
    );
    assert_eq!(
        recovered["artifacts"]["stderr"]["sha256"],
        json!(expected_stderr_sha),
        "recovery receipt conserves the same controlled stderr bytes"
    );
    // Final real-file inventory: exactly the two controlled raw streams and
    // the single replaced receipt - no temp remnant from any route.
    let final_names: Vec<String> = paea_receipt_create_inventory(&leaf)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        final_names,
        vec!["process-receipt.json".to_string(), "stderr.raw".to_string(), "stdout.raw".to_string()],
        "recovered leaf must contain exactly the two raw streams and the single receipt, with no temp remnant"
    );
}

/// Fresh native service storage; never open a user mission as a fixture.
fn service() -> (tempfile::TempDir, RootStore) {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = RootStore::initialize(dir.path(), &mut SystemEnvironment).unwrap();
    (dir, root)
}

/// Explicit actual first-publication project, writable: the same retained
/// SWARM_PAEA_SOURCE_FIXTURE the engine and web harnesses use, never a copy.
fn project() -> (PathBuf, RootStore, String, String) {
    let proof = PathBuf::from(
        std::env::var_os("SWARM_PAEA_SOURCE_FIXTURE").expect("explicit new source fixture"),
    );
    let value: Value =
        serde_json::from_slice(&fs::read(proof.join("result.json")).unwrap()).unwrap();
    assert_eq!(value["status"], "PASS");
    assert_eq!(value["task_accepted"], true);
    assert_eq!(value["paid_provider"], false);
    let path = PathBuf::from(value["root"].as_str().unwrap());
    let root = RootStore::open(&path, true).unwrap();
    let work = value["work"].as_str().unwrap().to_owned();
    let current = root.current_managed_git_publication(&work).unwrap();
    (path, root, work, current.candidate_commit)
}

/// Read only a fixture plan, finalizing the independent connection before use.
fn scope(dir: &Path, work: &str) -> Scope {
    let db = rusqlite::Connection::open_with_flags(
        dir.join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let version: i64 = db
        .query_row(
            "SELECT projection_version FROM agent_identity_plan_heads WHERE work_id=?",
            [work],
            |r| r.get(0),
        )
        .unwrap();
    let projection = match version {
        1 => swarm_engine::IdentityPlanProjection::V1,
        2 => swarm_engine::IdentityPlanProjection::V2,
        _ => panic!("unknown fixture projection"),
    };
    let stamp = swarm_engine::read_identity_plan_stamp(&db, work, projection).unwrap();
    db.close().unwrap();
    Scope {
        work_id: stamp.work_id,
        plan_version: stamp.generation,
        plan_digest: stamp.digest,
    }
}

/// Read the current Trust revision from the fixture's own bounded state file.
fn current_trust_revision(root: &Path) -> i64 {
    let state: Value =
        serde_json::from_slice(&fs::read(root.join(".swarm/trust-actions.json")).unwrap())
            .unwrap();
    state["revision"].as_i64().expect("fixture Trust revision")
}

/// Actual project Trust configuration for this witness: the enrolled principal
/// maps to the single identity holding admin+effect+approver (one
/// enrollment-principal mapping per root), with separate bearer identities for
/// the mandate/request producers. Idempotent on both regimes: a fresh root
/// applies at expected revision 0 with a revision-stable event id, a retained
/// root keeps the revision-bound event id at its actual current revision.
fn effect_config(root: &Path) -> Vec<u8> {
    let fresh = !root.join(".swarm/trust-actions.json").exists();
    let revision = if fresh { 0 } else { current_trust_revision(root) };
    serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("approval-web-cli-effect-config-{revision}"),
        "expected_revision":revision,
        "reason":"Explicit fixture effect and approver roles for the approval web CLI witness",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":[
            {"subject":"enrolled-effect","integrity":3,"roles":["admin","effect","approver"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}},
            {"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},
            {"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},
            {"subject":"agreement-admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]}
        ],
    }))
    .unwrap()
}

/// Valid Git proposal for the service-assigned repository and actual candidate.
fn proposal(id: &str, candidate: &str) -> Proposal {
    Proposal {
        schema_version: 1,
        request_id: "approval-web-cli-fixture".into(),
        action: Action::Git {
            repository_id: id.into(),
            target_ref: "refs/heads/release".into(),
            expected_old_oid: "1".repeat(40),
            candidate_oid: candidate.into(),
        },
    }
}

/// Declared context rechecked under held authority; one declared source.
fn context(trust_revision: i64) -> AdmissionContext {
    AdmissionContext {
        project_id: "fixture-project".into(),
        principal_id: "operator".into(),
        instance_id: "fixture-instance".into(),
        mandate_id: "fixture-mandate".into(),
        mandate_revision: 1,
        policy_revision: trust_revision,
        policy_digest: "aa".repeat(32),
        tool_manifest_digest: "bb".repeat(32),
        sources: vec![SourceRef {
            source_id: "fixture-source".into(),
            version: "1".into(),
            digest: "cc".repeat(32),
            integrity: 2,
            confidentiality_labels: vec![],
        }],
    }
}

/// Resolve/pin actual installed Git; shared fixture pattern from the engine
/// and web tests, so the ref observation cannot drift to another binary.
fn git_executable() -> &'static std::path::Path {
    static TOOL: std::sync::OnceLock<(std::path::PathBuf, String)> = std::sync::OnceLock::new();
    let (path, hash) = TOOL.get_or_init(|| {
        let path = std::env::split_paths(&std::env::var_os("PATH").expect("fixture PATH required"))
            .map(|p| p.join("git"))
            .find(|p| p.is_file() && fs::metadata(p).unwrap().permissions().mode() & 0o111 != 0)
            .expect("real Git required")
            .canonicalize()
            .unwrap();
        let hash = format!("{:x}", Sha256::digest(fs::read(&path).unwrap()));
        let version = Command::new(&path).arg("--version").output().unwrap();
        assert!(version.status.success());
        (path, hash)
    });
    assert_eq!(
        &format!("{:x}", Sha256::digest(fs::read(path).unwrap())),
        hash,
        "fixture Git changed"
    );
    path
}

/// The approval-web CLI image this witness exercises, selected explicitly
/// before any spawn: the Cargo source fixture binary by default, or an
/// external binary whose two selection entries both conform.
#[derive(Debug)]
struct SelectedCli {
    /// Absolute, canonical path of the image actually exercised.
    path: PathBuf,
    /// Selection mode label, asserted by the selector tests: either
    /// `cargo-source-fixture` (no external selection requested) or
    /// `external-explicit-selection` (both entries validated). Neither mode
    /// claims a delivered or normal runtime profile.
    mode: &'static str,
}

/// Assembles the selector's filesystem-error refusal diagnostic from the
/// concrete fact (always including the requested path, e.g. "path … does
/// not resolve to a real file") and the real I/O error. Output contract:
/// `{fact}: {os cause} (os error {code}); no fallback to the Cargo fixture
/// binary`, with the numeric code rendered only when the platform reports
/// one (`raw_os_error`), never invented; non-OS errors keep their Display
/// cause alone. Shared by every filesystem error branch of
/// `select_approval_web_cli_from` (resolve, stat, read) so no filesystem
/// refusal can omit the no-fallback marker again. Pure formatting:
/// infallible, no I/O, no environment access.
fn filesystem_refusal(detail: String, error: &std::io::Error) -> String {
    match error.raw_os_error() {
        Some(code) => format!(
            "{detail}: {error} (os error {code}); no fallback to the Cargo fixture binary"
        ),
        None => format!("{detail}: {error}; no fallback to the Cargo fixture binary"),
    }
}

/// Pure selector for the witness CLI image, from the two raw selection
/// entries as read from the environment (`None` when a variable is absent).
/// Fixture input of this test file, not a product policy.
///
/// Inputs and formats:
/// * path entry — filesystem path; must already be absolute, must resolve
///   (canonicalized here) to a real, executable regular file;
/// * hash entry — expected SHA-256 of the file bytes, exactly 64 lowercase
///   hexadecimal characters.
///
/// Decision and refusal order (each refusal names its concrete cause and
/// carries the no-fallback marker): exactly one entry present → refuse (a
/// requested selection with a missing companion entry is ambiguous); path
/// not absolute → refuse; unresolvable, non-file or non-executable path →
/// refuse (the filesystem failures — resolve, stat, read — are rendered by
/// `filesystem_refusal`, which appends the real OS cause with its numeric
/// code when the platform reports one and always carries the no-fallback
/// marker); malformed hash entry → refuse; digest mismatch → refuse. Only
/// the both-absent case returns the Cargo source fixture binary
/// (`CARGO_BIN_EXE_swarm-rust`); no other branch ever returns it.
///
/// Limits: the matching hash pins the file bytes at selection time only —
/// it proves neither a delivered or normal runtime profile. The spawned
/// child's real `/proc/<pid>/exe` identity is captured by the witness
/// itself (with boot id and start ticks) while the child is owned, and is
/// asserted for presence — never claimed as a delivered profile.
fn select_approval_web_cli_from(
    cli: Option<&std::ffi::OsStr>,
    sha256: Option<&std::ffi::OsStr>,
) -> Result<SelectedCli, String> {
    match (cli, sha256) {
        (None, None) => Ok(SelectedCli {
            path: PathBuf::from(env!("CARGO_BIN_EXE_swarm-rust")),
            mode: "cargo-source-fixture",
        }),
        (Some(_), None) => Err("external CLI selection requested but \
            SWARM_PAEA_APPROVAL_WEB_CLI_SHA256 is absent; no fallback to \
            the Cargo fixture binary"
            .to_owned()),
        (None, Some(_)) => Err("external CLI hash requested but \
            SWARM_PAEA_APPROVAL_WEB_CLI is absent; no fallback to the \
            Cargo fixture binary"
            .to_owned()),
        (Some(raw_path), Some(raw_hash)) => {
            let requested = PathBuf::from(raw_path);
            if !requested.is_absolute() {
                return Err(format!(
                    "external CLI path must be absolute, got {requested:?}; \
                     no fallback to the Cargo fixture binary"
                ));
            }
            let path = requested.canonicalize().map_err(|error| {
                filesystem_refusal(
                    format!("external CLI path {requested:?} does not resolve to a real file"),
                    &error,
                )
            })?;
            if !path.is_file() {
                return Err(format!(
                    "external CLI path {path:?} is not a regular file; no fallback to the Cargo fixture binary"
                ));
            }
            let executable = fs::metadata(&path)
                .map_err(|error| {
                    filesystem_refusal(format!("external CLI file {path:?} unreadable"), &error)
                })?
                .permissions()
                .mode()
                & 0o111
                != 0;
            if !executable {
                return Err(format!(
                    "external CLI file {path:?} is not executable; no fallback to the Cargo fixture binary"
                ));
            }
            let hash_text = raw_hash.to_str().ok_or_else(|| {
                "external CLI SHA256 entry must be UTF-8 text; no fallback to the Cargo fixture binary"
                    .to_owned()
            })?;
            let well_formed = hash_text.len() == 64
                && hash_text
                    .bytes()
                    .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
            if !well_formed {
                return Err(format!(
                    "external CLI SHA256 entry must be exactly 64 lowercase hex characters, got {hash_text:?}; no fallback to the Cargo fixture binary"
                ));
            }
            let bytes = fs::read(&path).map_err(|error| {
                filesystem_refusal(format!("external CLI file {path:?} unreadable"), &error)
            })?;
            let digest = format!("{:x}", Sha256::digest(&bytes));
            if digest != hash_text {
                return Err(format!(
                    "external CLI file {path:?} SHA256 {digest} does not match the expected {hash_text}; no fallback to the Cargo fixture binary"
                ));
            }
            Ok(SelectedCli {
                path,
                mode: "external-explicit-selection",
            })
        }
    }
}

/// The witness's own CLI image: read the two selection environment
/// variables — `SWARM_PAEA_APPROVAL_WEB_CLI` (absolute path of the external
/// binary) and `SWARM_PAEA_APPROVAL_WEB_CLI_SHA256` (its expected SHA-256,
/// 64 lowercase hex characters) — apply the pure selector, and panic on any
/// refusal so an invalid selection stops the witness BEFORE the child
/// spawn, never defaulting to the Cargo fixture binary. Both variables
/// absent keeps the Cargo source fixture binary. Fixture-input category:
/// these variables select the image exercised by THIS test only; they are
/// no product setting and prove no delivered profile.
fn approval_web_cli() -> SelectedCli {
    select_approval_web_cli_from(
        std::env::var_os("SWARM_PAEA_APPROVAL_WEB_CLI").as_deref(),
        std::env::var_os("SWARM_PAEA_APPROVAL_WEB_CLI_SHA256").as_deref(),
    )
    .unwrap_or_else(|refusal| {
        panic!("approval-web CLI selection refused before spawn: {refusal}")
    })
}

/// Actual Git ref lines of the work's managed bare repository, through the
/// real Git binary with the hardened environment; hashed comparison basis for
/// the no-Git-effect byte witness.
fn managed_git_refs(root: &Path, work: &str) -> String {
    let git_dir = root
        .join(".swarm/managed")
        .join(work)
        .join("repository.git");
    let mut git = Command::new(git_executable());
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"GIT_") {
            git.env_remove(key);
        }
    }
    let out = git
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args([
            "--git-dir",
            git_dir.to_str().expect("managed git dir encoding"),
            "for-each-ref",
            "--format=%(refname) %(objectname)",
        ])
        .output()
        .expect("git for-each-ref must run");
    assert!(
        out.status.success(),
        "git for-each-ref failed on {}: {}",
        git_dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("git ref listing utf-8")
}

/// SERVICE root Trust state bytes; a genuinely absent state file (fresh
/// service root) reads as empty, any other read error panics — an actual read
/// failure is never disguised as absence.
fn service_trust_state(root: &Path) -> Vec<u8> {
    match fs::read(root.join(".swarm/trust-actions.json")) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => panic!(
            "trust state read failed on {}: {e} (an actual read error is not an absence)",
            root.display()
        ),
    }
}

/// Actual HTTP/1.1 request with connection-close and the original UTF-8 body.
/// No timer, provider or retry; the owned server must finish the complete
/// response. GET carries the session cookie only; POST additionally carries
/// Origin, CSRF and the once-used Enrollment credential (labeled simulated
/// fixture password, never logged).
fn http(address: &str, cookie: &str, path: &str, body: Option<&str>, csrf: &str) -> Value {
    let mut socket = TcpStream::connect(address).unwrap();
    let request = match body {
        None => format!(
            "GET {path} HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n"
        ),
        Some(raw) => format!(
            "POST {path} HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nOrigin: http://{address}\r\nX-Swarm-CSRF: {csrf}\r\nAuthorization: Enrollment {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{raw}",
            URL_SAFE_NO_PAD.encode(PASSWORD),
            raw.len()
        ),
    };
    socket.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).unwrap();
    let (headers, body) = response.split_once("\r\n\r\n").unwrap();
    assert!(headers.starts_with("HTTP/1.1 200"), "HTTP refused: {headers}");
    serde_json::from_str(body).unwrap()
}

/// Reads ONE complete newline-terminated startup line from the live stdout
/// evidence reader of an owned capture, byte by byte, causally before any
/// producer EOF. Fixture-local helper of THIS test file, extracted from the
/// witness's two live startup loops and shared by the REAL positive
/// prepared and bound reads and by the startup-EOF guard — never a product
/// or shared-module API and no new authority: it only reads an
/// already-owned raw capture stream and decides nothing beyond that line.
///
/// IO: each step performs one blocking `read` into a one-byte stack
/// buffer; a read error is a refusal (panic) carrying the phase and the OS
/// cause — no retry, no timer, no extra buffering.
///
/// EOF: a stream end BEFORE the terminating newline is a FAILURE — the
/// helper panics naming the phase and the raw capture path (never the
/// captured bytes themselves, so no credential or business content can
/// leak through the diagnostic) and never returns a partial line as a
/// success or a JSON value. The terminating newline is consumed and is
/// NOT part of the returned line. The EOF diagnostic text is retained
/// verbatim from the witness's loops (including its `approval-web`
/// prefix) so the positive witness's failure output is unchanged.
///
/// UTF-8: the accumulated raw bytes are decoded EXACTLY once at the
/// newline with `String::from_utf8`, so multibyte characters survive
/// intact (replacing the former byte-as-char accumulation); invalid
/// UTF-8 is a refusal naming the phase and the raw capture path.
fn read_complete_startup_line<R: std::io::Read>(
    mut reader: R,
    phase: &str,
    raw_capture_display: &str,
) -> String {
    let mut bytes = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = reader
            .read(&mut byte)
            .unwrap_or_else(|e| panic!("live stdout read refused {phase}: {e}"));
        if n == 0 {
            panic!(
                "approval-web stdout closed {phase}: the raw stdout \
                 capture at {raw_capture_display} keeps the real bytes for diagnosis"
            );
        }
        if byte[0] == b'\n' {
            break;
        }
        bytes.push(byte[0]);
    }
    String::from_utf8(bytes).unwrap_or_else(|error| {
        panic!(
            "startup stdout line at phase {phase} is not valid UTF-8; the raw stdout \
             capture at {raw_capture_display} keeps the real bytes for diagnosis: {error}"
        )
    })
}

/// The full positive path on the retained native source fixture: real CLI
/// child, both exact permissions signed at bootstrap, pin-only frame, real
/// bound server, one empty authorized POST, durable approved action in the
/// PROJECT root, byte-identical SERVICE root and Git refs, clean SIGINT drain.
/// The child is owned by `OwnedChildCapture`: both raw streams are conserved
/// into exclusive private evidence files retained even on PASS, prepared and
/// bound are read through the LIVE stdout reader (the stdin frame end is
/// transferred and dropped for the real EOF the guarded reader waits on),
/// the ENTIRE post-START bootstrap — the /proc identity reads (pid, start
/// ticks, boot id, and the SHA-256 of the live image read through
/// `/proc/<pid>/exe` itself, never a reopened resolved symlink target), the
/// permission declaration, the pin frame with its EOF, both LIVE stdout
/// reads of prepared and bound, and the whole HTTP controller — runs under
/// the single `signal_stop(SIGINT)` wrapper (a bootstrap or business panic
/// keeps the signal owned and the captures truly collected before the
/// ORIGINAL payload is re-emitted). The receipt selection starts as the
/// minimal observed pre-spawn facts (mode, path, argv) plus an explicit
/// phase and is enriched under the guard as each fact is observed, so a
/// refused stop or a panicking bootstrap leaves the receipt carrying only
/// what was truly observed; the trust fixture is retained through an outer
/// Option, assigned once the pin frame is delivered, and closed only on the
/// stopped, no-panic final path. The drained observation records are parsed
/// from the conserved RAW stdout file and asserted against the real
/// `NativeOutcomeRecord` producer contract (exactly one
/// `trust_publication=recorded`, no refusal family), never read as the
/// business ack reply. The process receipt (selection, pid, boot/start
/// ticks, real `/proc/<pid>/exe` hash, artifact hashes) is written atomically
/// and fsynced before the final verdict; the leaf is never removed here.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn approval_web_cli_binds_and_consumes_exact_preparation_once() {
    let (project_path, project, work, candidate) = project();
    project
        .trust("config-apply", &effect_config(&project_path), "")
        .unwrap_or_else(|e| panic!("explicit fixture config refused: {}", e.code()));
    let trust_revision = current_trust_revision(&project_path);
    let fixture_scope = scope(&project_path, &work);
    let (service_dir, service) = service();
    service
        .paea_initialize_git_intents("fixture-project", PIN)
        .unwrap();
    // Owned private bare child, hardened like the engine's own job checks.
    let bare = service_dir.path().join(".swarm/bare");
    fs::create_dir(&bare).unwrap();
    fs::set_permissions(&bare, fs::Permissions::from_mode(0o700)).unwrap();
    let mut git_init = Command::new(git_executable());
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"GIT_") {
            git_init.env_remove(key);
        }
    }
    let init = git_init
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args(["-c", "init.templateDir=", "init", "--bare"])
        .arg(&bare)
        .output()
        .unwrap();
    assert!(init.status.success());
    let binding = service
        .paea_provision_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    let repository_id = binding.repository_id().to_owned();
    drop(binding);
    // The exact canonical action bytes the preparation digests and the
    // engine-derived agreement terms for the frozen approve command.
    let prop = proposal(&repository_id, &candidate);
    let ctx = context(trust_revision);
    let exact = canonicalize(&prop, &ctx).unwrap().bytes().to_vec();
    let terms = project
        .paea_git_agreement_terms(&exact)
        .unwrap()
        .expect("recipe terms derivable for this canonical Git action");
    assert_eq!(terms.binding, "recipe");
    assert_eq!(terms.revision, trust_revision);
    let resource = terms.resource.clone();
    let amount = terms.amount_minor;
    let currency = terms.currency.clone();
    let beneficiary = terms.beneficiary.clone();
    // Real mandate and pending request on the project root (bearer tokens).
    let mandate_id = format!("approval-web-cli-{}", std::process::id());
    let trust = |op: &str, value: Value, token: &str| -> Value {
        serde_json::from_slice(
            &project
                .trust(op, &serde_json::to_vec(&value).unwrap(), token)
                .unwrap_or_else(|e| panic!("{op} refused: {}", e.code()))
                .bytes,
        )
        .unwrap()
    };
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("approval-mandate-{mandate_id}"),
            "mandate_id":mandate_id,
            "subject":"requester",
            "actions":["settle"],
            "resource":resource.clone(),
            "max_amount_minor":amount,
            "currency":currency,
            "beneficiary":beneficiary.clone(),
        }),
        "fixture-admin-token",
    );
    let action: Value = trust(
        "request",
        json!({
            "schema_version":1,
            "event_id":format!("approval-request-{mandate_id}"),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":amount,
            "currency":currency,
            "beneficiary":beneficiary,
            "idempotency_key":format!("approval-key-{mandate_id}"),
        }),
        "fixture-requester-token",
    );
    assert_eq!(action["state"], "pending_approval");
    // The frozen approve command: exact terms of the pending action, written
    // as the explicit command file the CLI freezes at startup.
    let command_path = service_dir.path().join("approve-command.json");
    fs::write(
        &command_path,
        serde_json::to_vec(&json!({
            "schema_version":1,
            "event_id":format!("approval-approve-{mandate_id}"),
            "action_id":action["id"],
            "mandate_id":mandate_id,
            "content_digest":action["content_digest"],
            "resource":terms.resource,
            "amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
        }))
        .unwrap(),
    )
    .unwrap();
    // Byte witnesses BEFORE the child runs: the SERVICE root Trust state and
    // the managed Git refs must stay identical through the whole flow.
    let service_before = service_trust_state(service_dir.path());
    let refs_before = managed_git_refs(&project_path, &work);
    // Request document file (transfer variant: proposal + context).
    let input_path = service_dir.path().join("approval-request.json");
    fs::write(
        &input_path,
        serde_json::to_vec(&json!({
            "proposal":{"schema_version":1,"request_id":"approval-web-cli-fixture","action":{"kind":"repository.promote","repository_id":repository_id,"target_ref":"refs/heads/release","expected_old_oid":"1".repeat(40),"candidate_oid":candidate}},
            "context":{"project_id":"fixture-project","principal_id":"operator","instance_id":"fixture-instance","mandate_id":"fixture-mandate","mandate_revision":1,"policy_revision":trust_revision,"policy_digest":"aa".repeat(32),"tool_manifest_digest":"bb".repeat(32),"sources":[{"source_id":"fixture-source","version":"1","digest":"cc".repeat(32),"integrity":2,"confidentiality_labels":[]}]}
        }))
        .unwrap(),
    )
    .unwrap();
    let declaration_dir = service_dir.path().join("operator-declaration");
    fs::create_dir_all(&declaration_dir).unwrap();
    fs::set_permissions(&declaration_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let declaration = declaration_dir.join("declaration.json");
    // Real child through the actual CLI contract: the explicitly selected
    // image (Cargo source fixture binary unless both selection entries
    // designate an exact-hash external binary — never an implicit
    // fallback), preparing the transfer of the exact approve command,
    // bound to the clean PROJECT root. The selection is resolved here and
    // any refusal fires BEFORE this spawn.
    let cli = approval_web_cli();
    // Exact spawn arguments BEFORE the spawn: the runtime receipt pins the
    // selection mode, the argv and the child identity observed while the
    // child is genuinely owned (never invented after the fact).
    let argv: Vec<String> = [
        "--root",
        service_dir.path().to_str().unwrap(),
        "--json",
        "paea",
        "approval-web",
        "transfer",
        "approve",
        service_dir.path().to_str().unwrap(),
        project_path.to_str().unwrap(),
        &work,
        "bare",
        PIN,
        "transfer-pack",
        command_path.to_str().unwrap(),
        declaration.to_str().unwrap(),
        "--input",
        input_path.to_str().unwrap(),
    ]
    .iter()
    .map(|a| a.to_string())
    .collect();
    // Raw capture evidence: an exclusive private 0700 leaf under the
    // runner's EXPLICIT TMPDIR (never /tmp, no fallback), retained even on
    // PASS until Codex receives it. Both evidence files are created
    // exclusively inside it, 0600, by the capture owner itself.
    let leaf = private_fixture_leaf("approval-web-positive");
    let stdout_evidence = leaf.join("stdout.raw");
    let stderr_evidence = leaf.join("stderr.raw");
    let child = Command::new(&cli.path)
        .args(&argv)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("approval-web child spawn refused: {e}"));
    let mut owned = OwnedChildCapture::start(child, &stdout_evidence, &stderr_evidence)
        .unwrap_or_else(|refusal| {
            panic!(
                "approval-web raw capture start refused before any stream ran: {refusal:?}"
            )
        });
    // Minimal observed pre-spawn selection facts plus an explicit phase:
    // if the post-START bootstrap under the guard below panics, or the stop
    // is refused, the receipt carries only what was truly observed here —
    // never invented pid/exe metadata. The guard enriches this selection in
    // place as each fact is observed, and `prepared_line`, `bound_line` and
    // the fixture Option stay accessible on the outer scope after the route.
    let mut selection = json!({
        "mode": cli.mode,
        "path": cli.path.display().to_string(),
        "argv": argv,
        "phase": "capture-started-bootstrap-pending",
    });
    let mut prepared_line = String::new();
    let mut bound_line = String::new();
    let mut f_opt = None;
    let route = owned.signal_stop(rustix::process::Signal::INT, |owned| {
    // Runtime identity evidence, captured while the child IS owned and
    // running, BEFORE any finish: pid, the process start ticks and boot id
    // from /proc (recycling-proof pairing), and the REAL `/proc/<pid>/exe`
    // SHA-256 read from the live child. Nothing is invented after exit.
    let child_pid = owned.id();
    let proc_dir = PathBuf::from(format!("/proc/{child_pid}"));
    let exe_image = proc_dir.join("exe");
    // The hash is taken by reading `/proc/<pid>/exe` itself — the kernel
    // resolves the live image at read time; no reopened resolved symlink
    // target is involved.
    let exe_bytes = fs::read(&exe_image)
        .unwrap_or_else(|e| panic!("real /proc/{child_pid}/exe unreadable while child owned: {e}"));
    let exe_sha256 = format!("{:x}", Sha256::digest(&exe_bytes));
    let start_ticks = fs::read_to_string(proc_dir.join("stat"))
        .unwrap_or_else(|e| panic!("real /proc/{child_pid}/stat unreadable while child owned: {e}"))
        .rsplit(')')
        .next()
        .unwrap()
        .split_whitespace()
        .nth(19)
        .map(|f| f.to_string())
        .unwrap_or_else(|| panic!("starttime field missing in /proc/{child_pid}/stat"));
    let boot_id = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .unwrap_or_else(|e| panic!("boot_id unreadable: {e}"))
        .trim()
        .to_string();
    selection["pid"] = json!(child_pid);
    selection["proc_exe"] = json!(exe_image.display().to_string());
    selection["proc_exe_sha256"] = json!(exe_sha256);
    selection["proc_start_ticks"] = json!(start_ticks);
    selection["proc_boot_id"] = json!(boot_id);
    selection["phase"] = json!("proc-identity-observed");
    // LIVE stdout reader for the child protocol: the prepared event, then
    // — only after the pin frame was written AND the stdin end dropped —
    // the bound event. Each event is read as an exact newline-terminated
    // record from the conserved raw stream, causally, before producer EOF.
    let stdout_capture_display = owned.stdout_path().display().to_string();
    {
        let stdout_live = owned.stdout();
        let reader = stdout_live.evidence_reader();
        prepared_line = read_complete_startup_line(
            reader,
            "before the prepared event",
            &stdout_capture_display,
        );
    }
    assert!(
        !prepared_line.contains("STARTUP_FIXTURE_PASSWORD"),
        "simulated credential leaked on stdout: {prepared_line}"
    );
    let prepared: Value = serde_json::from_str(&prepared_line).unwrap();
    assert_eq!(prepared["event"], "paea-approval-web-prepared");
    assert_eq!(prepared["authority"], false);
    assert_eq!(prepared["restart_reusable"], false);
    assert_eq!(prepared["approval_operation"], "approve");
    assert!(
        prepared["call"].as_str().map_or(false, |s| !s.is_empty()),
        "prepared call kind must be a non-empty string: {prepared}"
    );
    let call_permission: Permission =
        serde_json::from_value(prepared["call_permission"].clone()).unwrap();
    let approval_permission: Permission =
        serde_json::from_value(prepared["approval_permission"].clone()).unwrap();
    // Out-of-band provisioning: ONE declaration signed with BOTH exact
    // permissions (the journal accepts one enroll per enrollment id).
    let f = fixture::Fixture::build_for_trust_actions(
        vec![call_permission, approval_permission],
        fixture_scope.clone(),
        trust_revision,
    );
    fs::copy(&f.path, &declaration).unwrap();
    fs::set_permissions(&declaration, fs::Permissions::from_mode(0o600)).unwrap();
    // Pin-only serve frame: header + 64-hex pin + newline, exactly 87 bytes,
    // no password suffix (the approval consumes over HTTP with the operator's
    // own Enrollment header). The frame is written through the TRANSFERRED
    // stdin end and the end is then DROPPED explicitly: the child's guarded
    // frame reader consumes stdin to EOF, so the EOF permission must arrive
    // here — the capture's own `finish` would close stdin far too late.
    let packet = format!("swarm-trust-source-v1\n{}\n", f.pin);
    assert_eq!(
        packet.len(),
        87,
        "pin-only serve frame must be exactly header+pin64+newline"
    );
    {
        let mut stdin = owned
            .take_stdin()
            .expect("the child was spawned with a piped stdin for the frame");
        stdin
            .write_all(packet.as_bytes())
            .unwrap_or_else(|e| panic!("pin-only frame write refused: {e}"));
        stdin
            .flush()
            .unwrap_or_else(|e| panic!("pin-only frame flush refused: {e}"));
        // Explicit drop = the real EOF permission; nothing else writes it.
    }
    // The frame is delivered and the pin consumed: retain the fixture for
    // explicit closure on the outer stopped, no-panic final path only.
    f_opt = Some(f);
    // Second live record: the bound event, causal, after the frame EOF.
    {
        let stdout_live = owned.stdout();
        let reader = stdout_live.evidence_reader();
        bound_line = read_complete_startup_line(
            reader,
            "before the bound event",
            &stdout_capture_display,
        );
    }
    let bound: Value = serde_json::from_str(&bound_line).unwrap();
    assert_eq!(bound["event"], "paea-approval-web-bound");
    assert_eq!(bound["runtime_ready"], false);
    assert_eq!(bound["mission_conductor"], false);
    let address = bound["address"].as_str().expect("bound address").to_string();
    selection["phase"] = json!("bootstrap-complete");
    // ENTIRE HTTP controller + business assertions INSIDE the owned
    // signal-stop wrapper: even if a business assertion panics, the pidfd
    // held since before the controller delivers the REAL SIGINT and the
    // captures are truly collected before the ORIGINAL payload is
    // re-emitted (no silent kill, no blind wait, no masked business
    // failure).
        // Real session of the really bound server: key file under the
        // service root, then the native session with its 64-hex CSRF token.
        let digest = format!("{:x}", Sha256::digest(address.as_bytes()));
        let key = fs::read_to_string(
            service_dir
                .path()
                .join(".swarm")
                .join(format!("web-session-{}", &digest[..16])),
        )
        .unwrap();
        let cookie = format!("swarm_session_{}={key}", &digest[..12]);
        let session = http(&address, &cookie, "/api/v1/native-session", None, "");
        let csrf = session["csrf_token"].as_str().unwrap();
        assert_eq!(csrf.len(), 64);
        // Startup observation: the approval IS bound and serves a NULL
        // capability only (the classified projection stays behind the
        // guarded presentation).
        let status = http(&address, &cookie, "/api/v1/paea/git-call/approval", None, "");
        assert_eq!(status["event"], "paea-git-call-approval-status");
        assert_eq!(status["bound"], true);
        assert_eq!(
            status["capability"],
            Value::Null,
            "approval status must serve no request-derived capability: {status}"
        );
        // The single consumption: EMPTY body, real session, CSRF and the
        // labeled simulated enrollment credential; exactly once, no retry.
        let reply = http(&address, &cookie, "/api/v1/paea/git-call/approval", Some(""), csrf);
        // Exact typed ack of this one approve — shapes verified by the real
        // run: `approved` (never `admitted` as a stand-in), the engine's
        // typed trust-status projection, redacted, acknowledging OUR frozen
        // action.
        assert_eq!(
            reply["state"], "approved",
            "typed approval ack must be exactly `approved`: {reply}"
        );
        assert_eq!(
            reply["kind"], "trust-status-ack-v1",
            "the ack must be the engine's typed trust status: {reply}"
        );
        assert_eq!(
            reply["object_id"], action["id"],
            "the ack must acknowledge our exact frozen action: {reply}"
        );
        assert_eq!(
            reply["content_redacted"], true,
            "the ack must serve no unredacted business content: {reply}"
        );
        // Durable positive witness in the RIGHT root: the action is approved
        // in the PROJECT root's own Trust state, attributed to the enrolled
        // operator.
        let stored: Value = serde_json::from_slice(
            &fs::read(project_path.join(".swarm/trust-actions.json")).unwrap(),
        )
        .unwrap();
        let entry = &stored["actions"][action["id"].as_str().unwrap()];
        assert_eq!(
            entry["state"],
            "approved",
            "durable project action must be approved: {entry}"
        );
        assert_eq!(
            stored["evidence"].as_array().unwrap().last().unwrap()
                ["identity_context"]["principal_id"],
            "operator"
        );
        // Separation and no-effect witnesses: SERVICE root Trust state and
        // the managed Git refs are byte-identical to before the child ran.
        assert_eq!(
            service_trust_state(service_dir.path()),
            service_before,
            "SERVICE root trust state must stay byte-identical"
        );
        assert_eq!(
            managed_git_refs(&project_path, &work),
            refs_before,
            "no Git ref effect is allowed on this path"
        );
    });
    // Explicit typed routes of the owned signal stop. Only `Stopped` is
    // exercised by this positive witness; the refusal routes are handled
    // WITHOUT claiming they were exercised: their full ownership is
    // conserved (intact capture, pidfd when held, real cause, any controller
    // panic), the process receipt is written, and the failure is raised with
    // the real cause — never a log-and-abandon, never a blind wait, never a
    // second signal attempt.
    let stopped = match route {
        SignalStopResult::Stopped(result) => result,
        SignalStopResult::PreparationRefused(refusal) => {
            // Typed refusal route, NOT claimed exercised: the receipt
            // carries the explicit UNKNOWN / not-finished outcome envelope
            // (no invented status, count or settled state); its typed
            // ownership and original-panic preservation stay OPEN. A
            // receipt failure stays a failure and conserves the real
            // refusal cause as the primary information.
            let (receipt, _) = write_process_receipt(
                &leaf,
                "approval_web_cli_binds_and_consumes_exact_preparation_once",
                "paea-approval-web-positive-process-receipt",
                &selection,
                &not_finished_process_outcome(None),
            )
            .unwrap_or_else(|e| {
                panic!(
                    "SIGINT preparation refused before the controller ran (pidfd_open; \
                     cause: {}) AND its process receipt write was also refused: {e}",
                    refusal.error
                )
            });
            panic!(
                "SIGINT preparation refused before the controller ran (pidfd_open) — \
                 receipt: {}; intact capture conserved: {:?}; cause: {}",
                receipt.display(),
                refusal.capture,
                refusal.error
            );
        }
        SignalStopResult::SignalRefused(refusal) => {
            // Typed refusal route, NOT claimed exercised: the receipt
            // carries the explicit UNKNOWN / not-finished outcome envelope
            // plus the ONE fact this route really carries (the real
            // controller-panic PRESENCE flag); its typed ownership and
            // original-panic preservation stay OPEN. A receipt failure
            // stays a failure and conserves the real refusal cause as the
            // primary information.
            let (receipt, _) = write_process_receipt(
                &leaf,
                "approval_web_cli_binds_and_consumes_exact_preparation_once",
                "paea-approval-web-positive-process-receipt",
                &selection,
                &not_finished_process_outcome(Some(refusal.controller_panic.is_some())),
            )
            .unwrap_or_else(|e| {
                panic!(
                    "SIGINT request refused through the held pidfd (cause: {}; controller \
                     panic retained: {}) AND its process receipt write was also refused: {e}",
                    refusal.error,
                    refusal.controller_panic.is_some()
                )
            });
            panic!(
                "SIGINT request refused through the held pidfd — receipt: {}; intact \
                 capture conserved: {:?}; cause: {}; controller panic retained: {}",
                receipt.display(),
                refusal.capture,
                refusal.error,
                refusal.controller_panic.is_some()
            );
        }
    };
    // The controller's outcome under `Stopped`: the real finish (stdin EOF,
    // ONE wait, BOTH joins) ran to its complete result; a WaitRefused
    // handback is conserved whole (child + BOTH still-running captures +
    // original cause) and never blindly re-waited; a controller panic is
    // re-emitted AFTER the receipt and both raw streams are secured.
    let (exit, stdout_terminal, stderr_terminal, stdout_path, stderr_path) = match stopped.outcome
    {
        paea_capture_child::ChildCaptureOutcome::Finished {
            exit,
            stdout_capture,
            stderr_capture,
            stdout_path,
            stderr_path,
        } => (exit, stdout_capture, stderr_capture, stdout_path, stderr_path),
        paea_capture_child::ChildCaptureOutcome::WaitRefused(handback) => {
            // THE ONE WaitRefused route fixed in this slice: the COMPLETE
            // handback is conserved into the witness-local typed
            // [`ParentWaitRefusedCarrier`] — BOTH captures join to their
            // real terminal states BEFORE the receipt (the SIGINT stop was
            // accepted on this route and the stdin-EOF permission was
            // already given, but an accepted stop request is NOT
            // termination proof — the joins keep the same documented
            // blocking limit as the finish contract and end only at the
            // producer's real causal EOF; `Failed`/`Panicked` states and
            // their causes are retained verbatim, never masked), the
            // receipt is the
            // EXPLICIT UNKNOWN / not-finished envelope that now records
            // the controller-panic PRESENCE this route actually knows
            // (never the payload text), and the receipt success path OR
            // the secondary receipt-write error travels as DATA inside the
            // carrier. The original non-Clone cause, the SAME child (never
            // re-waited here — the exit status stays UNKNOWN; no signal,
            // kill, timer or retry exists) and the original controller
            // panic payload move BY VALUE through the typed recoverable
            // boundary — no formatted panic pretends to conserve the
            // ownership. A higher `catch_unwind` receiver can recover the
            // carrier BY VALUE and decide the actual cleanup explicitly
            // (including the FIRST real `Child::wait` when no real wait
            // has run); with no higher consumer this unexpected handback
            // stays VISIBLE as the witness failure (the libtest top-level
            // drop establishes no external cleanup) — no claim is made
            // that all refusal routes are resolved: PreparationRefused
            // and SignalRefused remain explicitly OPEN and untouched.
            fail_parent_wait_refused(conserve_parent_wait_refused_carrier(
                "approval-web positive witness wait refused after the accepted SIGINT",
                "approval_web_cli_binds_and_consumes_exact_preparation_once",
                "paea-approval-web-positive-process-receipt",
                &leaf,
                &selection,
                handback,
                stopped.controller_panic,
            ));
        }
    };
    // Finished-path evidence increment: the process receipt is written
    // ATOMICALLY (0600 AT CREATION — no post-create protection window) and
    // fsynced IMMEDIATELY after the real Finished destructure, BEFORE any
    // exit, terminal or publication verdict and BEFORE any controller-panic
    // re-emission, strictly from the REAL finished facts (real
    // `ExitStatus` code, Unix signal and success, both REAL terminal
    // variants with exact `Completed` byte counts, controller-panic
    // PRESENCE — never the private payload; no invented wait count, no
    // invented settled state). A receipt failure stays a FAILURE: with an
    // original controller panic the typed `CompoundPanic` carrier unwinds
    // BOTH the original payload (verbatim, downcastable inside the
    // carrier; top-level type truthfully the carrier) and the stage-tagged
    // secondary receipt cause — the plain `raise` is deliberately NOT used
    // here, its `Some` arm would drop that cause together with `message`;
    // without a payload it is a plain panic carrying the same stage-tagged
    // text. Logging alone is never the resolution.
    let finished_outcome = finished_process_outcome(
        &exit,
        &stdout_terminal,
        &stderr_terminal,
        stopped.controller_panic.is_some(),
    );
    let (receipt, retained_leaf) = match write_process_receipt(
        &leaf,
        "approval_web_cli_binds_and_consumes_exact_preparation_once",
        "paea-approval-web-positive-process-receipt",
        &selection,
        &finished_outcome,
    ) {
        Ok(paths) => paths,
        Err(receipt_error) => match stopped.controller_panic {
            // Compound failure: the typed `CompoundPanic` carrier unwinds
            // BOTH the original payload (verbatim, downcastable inside the
            // carrier) and the stage-tagged secondary receipt cause. The
            // plain `raise` is NOT used — its `Some` arm resumes the
            // payload alone and would drop the receipt cause with
            // `message`.
            Some(original) => WitnessPanic::raise_compound(original, receipt_error),
            None => panic!(
                "finished-path process receipt refused after the real Finished collect; \
                 no controller panic payload exists to preserve: {receipt_error}"
            ),
        },
    };
    // Business panic handling: the ORIGINAL payload is re-emitted only
    // after the process receipt (paths + hashes + REAL finished outcome) is
    // durably on disk — written immediately after the Finished destructure
    // above, BEFORE this re-emission, and REUSED here (no duplicate final
    // receipt, no second manifest line). The captured raw streams stay in
    // the leaf regardless.
    if let Some(original) = stopped.controller_panic {
        WitnessPanic {
            message: format!(
                "business controller panicked; receipt and raw captures conserved at {} \
                 and {} (receipt {})",
                stdout_path.display(),
                stderr_path.display(),
                receipt.display()
            ),
            payload: Some(original),
        }
        .raise();
    }
    assert!(
        exit.success(),
        "approval-web must exit cleanly after the real SIGINT: {exit:?}"
    );
    // The stderr capture is asserted ONLY through its own terminal state
    // (drained bytes available in the conserved file) — never re-read
    // through a separate conservation loop.
    let stderr_bytes = match stderr_terminal {
        paea_capture_live::LiveStreamState::Completed { .. } => fs::read(&stderr_path)
            .unwrap_or_else(|e| panic!("conserved stderr.raw unreadable: {e}")),
        other => panic!("stderr capture did not complete cleanly: {other:?}"),
    };
    assert!(
        !String::from_utf8_lossy(&stderr_bytes).contains("STARTUP_FIXTURE_PASSWORD"),
        "simulated credential leaked on stderr"
    );
    // NativeOutcomeRecord v1 oracle on the REAL record contract, parsed
    // from the CONSERVED RAW stdout file after the stop (no second
    // conservation loop, no drain thread): prepared and bound are distinct
    // witness events, then every remaining line is a typed projection —
    // `schema_version` 1, HTTP `status`, `error`, `recorded_work`,
    // `configuration_may_be_recorded`, and an optional `trust_publication`
    // — carrying NO business `state`. The observed real flow of this run:
    // records without publication, then exactly ONE
    // `trust_publication` = "recorded"; no refusal, no invented recorded
    // work and no recordable configuration may appear.
    let stdout_bytes = match stdout_terminal {
        paea_capture_live::LiveStreamState::Completed { .. } => fs::read(&stdout_path)
            .unwrap_or_else(|e| panic!("conserved stdout.raw unreadable: {e}")),
        other => panic!("stdout capture did not complete cleanly: {other:?}"),
    };
    let stdout_text = String::from_utf8(stdout_bytes)
        .unwrap_or_else(|e| panic!("conserved stdout must be UTF-8 event lines: {e}"));
    let lines: Vec<&str> = stdout_text
        .lines()
        .filter(|l| !l.is_empty())
        .collect();
    assert!(
        lines.len() >= 2,
        "prepared and bound events must both be present in the conserved stdout: {stdout_text:?}"
    );
    assert_eq!(
        lines[0], prepared_line,
        "first conserved stdout line must be the live-read prepared event"
    );
    assert_eq!(
        lines[1], bound_line,
        "second conserved stdout line must be the live-read bound event"
    );
    assert!(
        !stdout_text.contains("STARTUP_FIXTURE_PASSWORD"),
        "simulated credential leaked on stdout"
    );
    let observations: Vec<Value> = lines[2..]
        .iter()
        .map(|raw| {
            serde_json::from_str(raw)
                .unwrap_or_else(|error| panic!("public outcome line must be JSON: {raw}: {error}"))
        })
        .collect();
    assert!(
        !observations.is_empty(),
        "no public outcome line retained in the conserved stdout"
    );
    let mut recorded_publications = 0usize;
    for record in &observations {
        assert_eq!(
            record["schema_version"], 1,
            "outcome record must be schema v1: {record}"
        );
        assert_eq!(
            record["status"], 200,
            "outcome record must carry the real HTTP status: {record}"
        );
        assert_eq!(
            record["error"], Value::Null,
            "outcome record must carry no error on this approved path: {record}"
        );
        assert_eq!(
            record["recorded_work"], Value::Null,
            "outcome record must invent no recorded work: {record}"
        );
        assert_eq!(
            record["configuration_may_be_recorded"], false,
            "outcome record must invent no recordable configuration: {record}"
        );
        match record.get("trust_publication") {
            None | Some(Value::Null) => {}
            Some(publication) if publication.as_str() == Some("recorded") => {
                recorded_publications += 1;
            }
            Some(other) => panic!(
                "outcome record must not invent a trust publication state: {other} in {record}"
            ),
        }
    }
    assert_eq!(
        recorded_publications, 1,
        "exactly one outcome record must carry the recorded trust publication: {observations:?}"
    );
    // Persistent verdict evidence: the process receipt (relative paths +
    // SHA-256 raw hashes + exact selection bytes + the REAL finished
    // process outcome) was ALREADY written ATOMICALLY and fsynced
    // immediately after the real Finished destructure — BEFORE every exit,
    // terminal and publication verdict and before any controller-panic
    // re-emission — and is REUSED here: NO duplicate final receipt is
    // written and the single manifest JSON line already located it. The
    // leaf (stdout.raw, stderr.raw, receipt) is NEVER removed — not even
    // on PASS — until Codex receives it.
    assert!(
        receipt.is_file() && retained_leaf == leaf,
        "receipt must exist inside the retained leaf: {} in {}",
        receipt.display(),
        retained_leaf.display()
    );
    // Explicit fixture closure after the tested owners exited: the retained
    // Option is consumed only here, on the stopped, no-panic final path.
    f_opt
        .take()
        .expect("trust fixture must be retained on the stopped no-panic final path")
        .close();
    service.close().unwrap();
    project.close().unwrap();
}

/// Selector default: with both selection entries absent the witness keeps
/// the explicitly named Cargo source fixture binary — no external guess.
/// Pure function under test: the process environment is never mutated.
#[test]
fn approval_web_cli_selector_defaults_to_cargo_fixture() {
    let selected = select_approval_web_cli_from(None, None).unwrap();
    assert_eq!(
        selected.path,
        PathBuf::from(env!("CARGO_BIN_EXE_swarm-rust")),
        "default selection must be the named Cargo source fixture binary"
    );
    assert_eq!(selected.mode, "cargo-source-fixture");
}

/// Selector refusals needing no filesystem: a one-entry selection (path
/// without hash, hash without path), a relative path, an unresolvable
/// absolute path and malformed SHA-256 entries (63 chars, uppercase hex,
/// non-hex bytes) each refuse with their concrete cause, and every refusal
/// carries the no-fallback marker instead of the Cargo default.
#[test]
fn approval_web_cli_selector_refuses_incomplete_or_invalid_entries() {
    let valid_hash = "a".repeat(64);
    let no_fallback = "no fallback to the Cargo fixture binary";
    let error = select_approval_web_cli_from(Some(std::ffi::OsStr::new("/bin/true")), None)
        .unwrap_err();
    assert!(error.contains("SHA256") && error.contains(no_fallback), "{error}");
    let error =
        select_approval_web_cli_from(None, Some(std::ffi::OsStr::new(&valid_hash))).unwrap_err();
    assert!(error.contains("hash requested") && error.contains(no_fallback), "{error}");
    let error = select_approval_web_cli_from(
        Some(std::ffi::OsStr::new("relative/approval-web-cli")),
        Some(std::ffi::OsStr::new(&valid_hash)),
    )
    .unwrap_err();
    assert!(error.contains("absolute") && error.contains(no_fallback), "{error}");
    let error = select_approval_web_cli_from(
        Some(std::ffi::OsStr::new("/nonexistent/approval-web-cli-under-test")),
        Some(std::ffi::OsStr::new(&valid_hash)),
    )
    .unwrap_err();
    assert!(error.contains("does not resolve") && error.contains(no_fallback), "{error}");
    for malformed in ["a".repeat(63), "A".repeat(64), "zz".repeat(32)] {
        let error = select_approval_web_cli_from(
            Some(std::ffi::OsStr::new("/bin/true")),
            Some(std::ffi::OsStr::new(&malformed)),
        )
        .unwrap_err();
        assert!(
            error.contains("64 lowercase hex") && error.contains(no_fallback),
            "{error}"
        );
    }
}

/// Selector file-evidence refusals, then exact acceptance (recovery): a
/// real, non-executable file — this test's own source, kept non-executable
/// by the fixture environment, precondition asserted explicitly — with its
/// correct digest refuses; the pinned real Git binary with a foreign
/// digest refuses; and the same Git binary with its own SHA-256 is
/// accepted as the canonicalized external selection. These byte fixtures
/// validate the selector only: neither image is ever spawned as
/// approval-web, bound, or presented as an accepted service. No new
/// fixture directory is created.
#[test]
fn approval_web_cli_selector_refuses_file_evidence_then_accepts_exact() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/paea_approval_web.rs");
    assert!(
        fs::metadata(&source).unwrap().permissions().mode() & 0o111 == 0,
        "test source fixture must stay non-executable for this refusal case"
    );
    let source_digest = format!("{:x}", Sha256::digest(fs::read(&source).unwrap()));
    let error = select_approval_web_cli_from(
        Some(source.as_os_str()),
        Some(std::ffi::OsStr::new(&source_digest)),
    )
    .unwrap_err();
    assert!(
        error.contains("not executable") && error.contains("no fallback"),
        "{error}"
    );
    let git = git_executable();
    let foreign = format!("{:x}", Sha256::digest(b"selector foreign digest input"));
    let error = select_approval_web_cli_from(
        Some(git.as_os_str()),
        Some(std::ffi::OsStr::new(&foreign)),
    )
    .unwrap_err();
    assert!(
        error.contains("does not match") && error.contains("no fallback"),
        "{error}"
    );
    let own = format!("{:x}", Sha256::digest(fs::read(&git).unwrap()));
    let selected =
        select_approval_web_cli_from(Some(git.as_os_str()), Some(std::ffi::OsStr::new(&own)))
            .unwrap();
    assert_eq!(selected.path, git, "accepted selection must be canonical");
    assert_eq!(selected.mode, "external-explicit-selection");
}

/// Startup EOF-before-prepared regression through the SHARED complete-line
/// startup reader extracted from the positive witness
/// (`read_complete_startup_line`), on REAL owned `python3` children
/// captured by `OwnedChildCapture` — one healthy witness first, then one
/// fault witness. Scope and limits: this guard qualifies ONLY the
/// witness-local startup reader and the existing capture, finish and
/// `signal_stop` ownership around it — it proves neither OS confinement,
/// nor human approval, nor global acceptance; it never touches product
/// code, the typed refusal ownership of the receipt (its ONLY receipt
/// surface is the witness-local FINISHED-path builder applied below to the
/// two REAL finished outcomes and, in the deterministic compound-refusal
/// regression, to a PRIVATE probe leaf whose receipt write is REALLY
/// refused by a real directory collision and therefore never lands), the
/// positive HTTP path or the retained
/// source fixture, and it invents no timer, retry, mock pid/proc/wait or
/// global environment mutation. Finished-path receipt increment: the SAME
/// witness-local receipt builder runs on the healthy REAL exit-0 finished
/// outcome FIRST and on the fault REAL finished outcome, both BEFORE any
/// verdict, and each persisted receipt is read back and personally
/// inspected (receipt schema version and real witness label, real exit
/// code, no Unix signal, real success flag, controller-panic PRESENCE only
/// and never the private payload, both `Completed` terminals with their
/// exact byte counts, real SHA-256 raw hashes and relative raw paths, and
/// the receipt file mode 0600 applied AT CREATION).
///
/// Healthy witness: a real child emits ONE complete prepared JSON line
/// (flushed) and then awaits stdin EOF; the shared reader returns the
/// exact line causally — while the child still awaits stdin, before any
/// release — and the finish's stdin EOF releases the child for the real
/// collect: clean exit, both `Completed` terminals and both raw streams
/// byte-exact (which pins both Completed byte counts); no EOF failure may
/// fire on the healthy path.
///
/// Fault witness: a real child IGNORES SIGINT, writes a partial stdout
/// fragment WITHOUT any newline plus one known stderr diagnostic, and
/// exits 7 NATURALLY without a prepared line. The SAME shared reader runs
/// under the existing `signal_stop(SIGINT)` wrapper: the EOF before the
/// newline is a FAILURE — the reader panics with the safe diagnostic
/// (phase and raw capture path, never the captured bytes), the wrapper's
/// real pidfd SIGINT request succeeds (`Stopped`), the finish's ONE real
/// wait reaps the child (its REAL exit 7 — never a fake cleanup exit),
/// both captures complete, the conserved raw ownership is read BEFORE any
/// verdict, and the ORIGINAL startup panic payload is observable only
/// after that collect and is asserted safe (phase and raw path present,
/// raw bytes absent). The finished-path receipt is written on this REAL
/// finished outcome BEFORE any verdict and its JSON is read back and
/// inspected: real exit 7, NOT a signal, no success, controller-panic
/// PRESENCE true (payload text and raw captured bytes absent from the
/// JSON), both `Completed` terminals with their exact byte counts, real
/// SHA-256 raw hashes and relative raw paths, 0600 mode applied AT
/// CREATION. The guard also adds ONE deterministic compound-failure
/// regression on a PRIVATE owned leaf: real controlled raw bytes first,
/// then an actual empty directory named `process-receipt.json` forcing the
/// builder's REAL rename refusal (no guessed timestamp, no faked OS error,
/// no live data); the unwound payload is caught and downcast to the typed
/// `CompoundPanic` carrier, asserting BOTH the secondary receipt
/// stage/cause and the downcast ORIGINAL probe marker, and that the probe
/// leaf is RETAINED on every outcome (no TempDir, no deletion call) as
/// durable failure evidence. On every real guard route the private leaf,
/// both raw files and the receipt stay on disk.
#[test]
fn paea_capture_startup_eof_retains_actual_exit_and_raw() {
    // ---- Healthy witness: complete prepared line, causal read, then the
    // ---- finish's stdin EOF release and the real collect.
    const HEALTHY_PREPARED_LINE: &str =
        r#"{"event":"paea-approval-web-prepared-shared-reader"}"#;
    let healthy_script = format!(
        "import sys\n\
         sys.stdout.write('{HEALTHY_PREPARED_LINE}\\n')\n\
         sys.stdout.flush()\n\
         sys.stdin.read()\n"
    );
    let healthy_leaf = private_fixture_leaf("approval-web-startup-reader-healthy");
    let healthy_child = Command::new("python3")
        .arg("-c")
        .arg(&healthy_script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("healthy startup-reader child spawn refused: {e}"));
    let mut healthy = OwnedChildCapture::start(
        healthy_child,
        &healthy_leaf.join("stdout.raw"),
        &healthy_leaf.join("stderr.raw"),
    )
    .unwrap_or_else(|refusal| {
        panic!("healthy startup-reader capture start refused: {refusal:?}")
    });
    let healthy_result = healthy.run_controller_then_finish(|owned| {
        let stdout_capture_display = owned.stdout_path().display().to_string();
        let line = {
            let stdout_live = owned.stdout();
            let reader = stdout_live.evidence_reader();
            read_complete_startup_line(
                reader,
                "before the prepared event",
                &stdout_capture_display,
            )
        };
        assert_eq!(
            line, HEALTHY_PREPARED_LINE,
            "the shared startup reader must return the exact causal prepared line"
        );
        // Causality: the child still awaits stdin EOF here — the release
        // happens only in the finish after this controller returns.
    });
    assert!(
        healthy_result.controller_panic.is_none(),
        "the healthy causal prepared read must not panic under the shared startup reader"
    );
    let (healthy_exit, healthy_stdout_terminal, healthy_stderr_terminal, healthy_stdout_path, healthy_stderr_path) =
        match healthy_result.outcome {
            paea_capture_child::ChildCaptureOutcome::Finished {
                exit,
                stdout_capture,
                stderr_capture,
                stdout_path,
                stderr_path,
            } => (exit, stdout_capture, stderr_capture, stdout_path, stderr_path),
            paea_capture_child::ChildCaptureOutcome::WaitRefused(handback) => panic!(
                "healthy startup-reader finish refused the real wait (child pid {}): {}",
                handback.child.id(),
                handback.error
            ),
        };
    // Finished-path receipt witness FIRST (healthy): the SAME receipt
    // builder runs on the actual finished outcome — REAL exit 0 — before
    // any exit/terminal verdict, and the file mode 0600 must be observed
    // (applied AT CREATION, no post-create protection window).
    let healthy_receipt_selection = json!({
        "guard": "paea_capture_startup_eof_retains_actual_exit_and_raw",
        "witness": "healthy_startup_reader",
        "argv": ["python3", "-c", healthy_script],
    });
    let (healthy_receipt_path, _) = write_process_receipt(
        &healthy_leaf,
        "paea_capture_startup_eof_retains_actual_exit_and_raw",
        "paea-capture-startup-eof-process-receipt",
        &healthy_receipt_selection,
        &finished_process_outcome(
            &healthy_exit,
            &healthy_stdout_terminal,
            &healthy_stderr_terminal,
            healthy_result.controller_panic.is_some(),
        ),
    )
    .unwrap_or_else(|e| panic!("healthy finished-path process receipt refused: {e}"));
    let healthy_receipt_mode = fs::metadata(&healthy_receipt_path)
        .unwrap_or_else(|e| panic!("healthy receipt metadata unreadable: {e}"))
        .permissions()
        .mode();
    assert_eq!(
        healthy_receipt_mode & 0o777,
        0o600,
        "healthy receipt file must carry the creation-time 0600 mode: {healthy_receipt_mode:o}"
    );
    assert!(
        healthy_exit.success(),
        "healthy child must exit cleanly after the causal stdin EOF release: {healthy_exit:?}"
    );
    match &healthy_stdout_terminal {
        paea_capture_live::LiveStreamState::Completed { bytes } => assert_eq!(
            *bytes,
            (HEALTHY_PREPARED_LINE.len() + 1) as u64,
            "healthy stdout Completed must pin the exact byte count (prepared line + newline): {healthy_stdout_terminal:?}"
        ),
        _ => panic!(
            "healthy stdout capture must complete: {healthy_stdout_terminal:?}"
        ),
    }
    match &healthy_stderr_terminal {
        paea_capture_live::LiveStreamState::Completed { bytes } => assert_eq!(
            *bytes, 0u64,
            "healthy stderr Completed must pin the exact byte count 0 (empty stream): {healthy_stderr_terminal:?}"
        ),
        _ => panic!(
            "healthy stderr capture must complete: {healthy_stderr_terminal:?}"
        ),
    }
    let mut expected_healthy_stdout = HEALTHY_PREPARED_LINE.as_bytes().to_vec();
    expected_healthy_stdout.push(b'\n');
    let healthy_stdout_bytes = fs::read(&healthy_stdout_path)
        .unwrap_or_else(|e| panic!("conserved healthy stdout.raw unreadable: {e}"));
    let healthy_stderr_bytes = fs::read(&healthy_stderr_path)
        .unwrap_or_else(|e| panic!("conserved healthy stderr.raw unreadable: {e}"));
    assert_eq!(
        healthy_stdout_bytes, expected_healthy_stdout,
        "conserved healthy stdout must be exactly the prepared line + newline; this byte \
         equality also pins the Completed stdout byte count"
    );
    assert!(
        healthy_stderr_bytes.is_empty(),
        "conserved healthy stderr must be empty (Completed count 0): {healthy_stderr_bytes:?}"
    );

    // Personal inspection of the persisted healthy receipt JSON: the REAL
    // finished facts — actual exit 0, NOT a signal, real success, no
    // controller panic, both Completed terminals with their exact byte
    // counts, real raw hash and relative raw path — plus the receipt
    // schema version and real witness label.
    let healthy_receipt: Value = serde_json::from_slice(
        &fs::read(&healthy_receipt_path)
            .unwrap_or_else(|e| panic!("written healthy receipt unreadable: {e}")),
    )
    .unwrap_or_else(|e| panic!("written healthy receipt must be valid JSON: {e}"));
    assert_eq!(
        healthy_receipt["witness"],
        "paea_capture_startup_eof_retains_actual_exit_and_raw",
        "healthy receipt must carry the guard's real witness label: {healthy_receipt}"
    );
    assert_eq!(
        healthy_receipt["schema_version"], 1,
        "healthy receipt must carry the receipt schema version: {healthy_receipt}"
    );
    assert_eq!(
        healthy_receipt["process"]["finished"], true,
        "healthy receipt must record the actual Finished outcome: {healthy_receipt}"
    );
    assert_eq!(
        healthy_receipt["process"]["exit_code"], 0,
        "healthy receipt must record the REAL exit 0: {healthy_receipt}"
    );
    assert_eq!(
        healthy_receipt["process"]["signal"], Value::Null,
        "healthy receipt must record that NO Unix signal killed the child: {healthy_receipt}"
    );
    assert_eq!(
        healthy_receipt["process"]["success"], true,
        "healthy receipt must record the real success: {healthy_receipt}"
    );
    assert_eq!(
        healthy_receipt["process"]["stdout_terminal"], "Completed",
        "healthy receipt must record the Completed stdout terminal: {healthy_receipt}"
    );
    assert_eq!(
        healthy_receipt["process"]["stdout_completed_bytes"],
        (HEALTHY_PREPARED_LINE.len() + 1) as u64,
        "healthy receipt must pin the exact Completed stdout byte count: {healthy_receipt}"
    );
    assert_eq!(
        healthy_receipt["process"]["stderr_terminal"], "Completed",
        "healthy receipt must record the Completed stderr terminal: {healthy_receipt}"
    );
    assert_eq!(
        healthy_receipt["process"]["stderr_completed_bytes"], 0u64,
        "healthy receipt must pin the exact Completed stderr byte count 0: {healthy_receipt}"
    );
    assert_eq!(
        healthy_receipt["process"]["controller_panic"], false,
        "healthy receipt must record the real controller-panic absence: {healthy_receipt}"
    );
    assert_eq!(
        healthy_receipt["artifacts"]["stdout"]["path"], "stdout.raw",
        "healthy receipt must carry the relative raw stdout path: {healthy_receipt}"
    );
    assert_eq!(
        healthy_receipt["artifacts"]["stdout"]["sha256"],
        format!("{:x}", Sha256::digest(&healthy_stdout_bytes)),
        "healthy receipt must carry the REAL raw stdout hash: {healthy_receipt}"
    );

    // Deterministic compound-failure regression on a PRIVATE owned leaf
    // (no live data, no shared guard leaf): real controlled raw bytes
    // first, then an ACTUAL empty directory named `process-receipt.json`
    // so the builder's REAL rename is refused by the filesystem itself
    // (no guessed timestamp, no faked OS error). The valid healthy receipt
    // witness above already ran FIRST and succeeded. The unwound payload is
    // caught and downcast to the typed `CompoundPanic` carrier: BOTH the
    // secondary receipt stage/cause and the downcast ORIGINAL probe marker
    // must survive the unwind.
    struct CompoundProbeOriginal;
    let probe_leaf = private_fixture_leaf("receipt-compound-refusal-probe");
    let probe_raw_stdout: &[u8] = b"compound-probe controlled stdout\n";
    let probe_raw_stderr: &[u8] = b"compound-probe controlled stderr\n";
    fs::write(probe_leaf.join("stdout.raw"), probe_raw_stdout)
        .unwrap_or_else(|e| panic!("compound probe stdout.raw write refused: {e}"));
    fs::write(probe_leaf.join("stderr.raw"), probe_raw_stderr)
        .unwrap_or_else(|e| panic!("compound probe stderr.raw write refused: {e}"));
    fs::create_dir(probe_leaf.join("process-receipt.json"))
        .unwrap_or_else(|e| panic!("compound probe collision directory refused: {e}"));
    let probe_selection = json!({
        "guard": "paea_capture_startup_eof_retains_actual_exit_and_raw",
        "witness": "compound_receipt_refusal_probe",
        "note": "real directory collision on process-receipt.json forces the REAL rename refusal",
    });
    // The probe carries no real process fact; the explicit not-finished
    // envelope reuses the existing helper and the receipt NEVER lands (the
    // rename is refused before any manifest line).
    let probe_outcome = not_finished_process_outcome(None);
    let probe_unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        match write_process_receipt(
            &probe_leaf,
            "paea_capture_startup_eof_retains_actual_exit_and_raw",
            "paea-capture-startup-eof-process-receipt",
            &probe_selection,
            &probe_outcome,
        ) {
            Ok(paths) => panic!(
                "compound probe receipt must NOT succeed against the real directory \
                 collision: {paths:?}"
            ),
            Err(receipt_error) => WitnessPanic::raise_compound(
                Box::new(CompoundProbeOriginal) as Box<dyn std::any::Any + Send>,
                receipt_error,
            ),
        }
    }));
    let probe_payload = probe_unwind
        .err()
        .unwrap_or_else(|| panic!("compound probe must unwind through the typed carrier"));
    let probe_carrier = probe_payload
        .downcast_ref::<CompoundPanic>()
        .unwrap_or_else(|| {
            panic!("compound probe payload must downcast to the typed CompoundPanic carrier")
        });
    assert!(
        probe_carrier
            .receipt_cause
            .contains("receipt atomic rename refused"),
        "compound probe carrier must carry the REAL rename-refusal stage: {}",
        probe_carrier.receipt_cause
    );
    assert!(
        probe_carrier.receipt_cause.contains("os error"),
        "compound probe carrier must carry the REAL underlying OS cause text: {}",
        probe_carrier.receipt_cause
    );
    assert!(
        !probe_carrier
            .receipt_cause
            .contains("receipt temp creation refused"),
        "the REAL refusal must be the rename stage, not temp creation: {}",
        probe_carrier.receipt_cause
    );
    assert!(
        !probe_carrier.receipt_cause.contains("compound-probe controlled"),
        "the secondary receipt cause must never carry raw stream content: {}",
        probe_carrier.receipt_cause
    );
    assert!(
        probe_carrier
            .original
            .downcast_ref::<CompoundProbeOriginal>()
            .is_some(),
        "compound probe carrier must retain the ORIGINAL probe payload downcastable"
    );
    // Prior evidence preserved by the REAL refusal: the collision directory
    // is untouched, the staged receipt temp survives the refused rename,
    // and both controlled raw files keep their exact bytes. The private
    // leaf is RETAINED on every outcome — success, REAL refusal, or probe
    // assertion panic — as durable failure proof; nothing live is touched,
    // and no deletion or cleanup call exists anywhere on this route.
    assert!(
        probe_leaf.join("process-receipt.json").is_dir(),
        "the REAL collision directory must survive the refused rename"
    );
    // Enumeration IO errors are REAL failures: collect every entry with
    // its actual cause BEFORE any filtering or counting.
    let probe_entries = fs::read_dir(&probe_leaf)
        .and_then(|read| read.collect::<Result<Vec<_>, std::io::Error>>())
        .unwrap_or_else(|e| panic!("compound probe leaf unreadable after refusal: {e}"));
    let probe_staged_temps = probe_entries
        .iter()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with("process-receipt.json.tmp-"))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(
        probe_staged_temps, 1,
        "exactly ONE staged receipt temp must survive the refused rename"
    );
    let probe_stdout_after = fs::read(probe_leaf.join("stdout.raw"))
        .unwrap_or_else(|e| panic!("compound probe stdout.raw unreadable after refusal: {e}"));
    assert_eq!(
        &probe_stdout_after[..],
        probe_raw_stdout,
        "compound probe conserved stdout.raw must keep its exact controlled bytes"
    );
    let probe_stderr_after = fs::read(probe_leaf.join("stderr.raw"))
        .unwrap_or_else(|e| panic!("compound probe stderr.raw unreadable after refusal: {e}"));
    assert_eq!(
        &probe_stderr_after[..],
        probe_raw_stderr,
        "compound probe conserved stderr.raw must keep its exact controlled bytes"
    );

    // ---- Fault witness: EOF before any newline under the REAL signal_stop
    // ---- wrapper; REAL natural exit 7; raw bytes conserved before any
    // ---- verdict; ORIGINAL startup panic observed after the collect.
    const FAULT_STDOUT_FRAGMENT: &str = "STARTUP-EOF-PARTIAL-NO-NEWLINE";
    const FAULT_STDERR_DIAGNOSTIC: &str = "STARTUP-EOF-FAULT-DIAGNOSTIC";
    let fault_script = format!(
        "import sys, signal\n\
         signal.signal(signal.SIGINT, signal.SIG_IGN)\n\
         sys.stdout.buffer.write(b'{FAULT_STDOUT_FRAGMENT}')\n\
         sys.stdout.buffer.flush()\n\
         sys.stderr.buffer.write(b'{FAULT_STDERR_DIAGNOSTIC}\\n')\n\
         sys.stderr.buffer.flush()\n\
         raise SystemExit(7)\n"
    );
    let fault_leaf = private_fixture_leaf("approval-web-startup-reader-fault");
    let fault_child = Command::new("python3")
        .arg("-c")
        .arg(&fault_script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("fault startup-reader child spawn refused: {e}"));
    let mut fault = OwnedChildCapture::start(
        fault_child,
        &fault_leaf.join("stdout.raw"),
        &fault_leaf.join("stderr.raw"),
    )
    .unwrap_or_else(|refusal| {
        panic!("fault startup-reader capture start refused: {refusal:?}")
    });
    let route = fault.signal_stop(rustix::process::Signal::INT, |owned| {
        let stdout_capture_display = owned.stdout_path().display().to_string();
        let stdout_live = owned.stdout();
        let reader = stdout_live.evidence_reader();
        let line = read_complete_startup_line(
            reader,
            "before the prepared event",
            &stdout_capture_display,
        );
        panic!(
            "the shared startup reader unexpectedly returned a complete line \
             despite the EOF-before-prepared fault: {line}"
        );
    });
    let stopped = match route {
        SignalStopResult::Stopped(result) => result,
        SignalStopResult::PreparationRefused(refusal) => panic!(
            "fault witness SIGINT preparation refused before the controller ran \
             (pidfd_open): {:?}; cause: {}",
            refusal.capture,
            refusal.error
        ),
        SignalStopResult::SignalRefused(refusal) => panic!(
            "fault witness SIGINT request refused through the held pidfd: {:?}; \
             cause: {}; retained controller panic: {}",
            refusal.capture,
            refusal.error,
            refusal.controller_panic.is_some()
        ),
    };
    let (exit, stdout_terminal, stderr_terminal, stdout_path, stderr_path) =
        match stopped.outcome {
            paea_capture_child::ChildCaptureOutcome::Finished {
                exit,
                stdout_capture,
                stderr_capture,
                stdout_path,
                stderr_path,
            } => (exit, stdout_capture, stderr_capture, stdout_path, stderr_path),
            paea_capture_child::ChildCaptureOutcome::WaitRefused(handback) => panic!(
                "fault witness child wait refused after the successful SIGINT request \
                 (child pid {}): {:?}; cause: {}",
                handback.child.id(),
                handback,
                handback.error
            ),
        };
    // Finished-path receipt on the ACTUAL fault outcome, BEFORE any
    // verdict: the SAME receipt builder runs on the real Finished facts
    // already in hand (the real reaped exit inside `exit`, both real
    // terminal states, the controller-panic PRESENCE observed on the
    // stopped result), and the 0600 mode must be observed as applied AT
    // CREATION.
    // Guard-only selection from TRUE safe actual facts ONLY: the real
    // executable name and the SHA-256 of the ACTUAL fault script bytes.
    // The raw fault script text is deliberately OMITTED from argv — the
    // script embeds the fault fragments that must never reach the receipt
    // — and NO fabricated, truncated or redacted pretend-argv field is
    // invented in its place. (The positive selection above keeps its exact
    // real argv.)
    let fault_receipt_selection = json!({
        "guard": "paea_capture_startup_eof_retains_actual_exit_and_raw",
        "witness": "fault_startup_eof_before_prepared",
        "executable": "python3",
        "script_sha256": format!("{:x}", Sha256::digest(fault_script.as_bytes())),
    });
    let (fault_receipt_path, _) = write_process_receipt(
        &fault_leaf,
        "paea_capture_startup_eof_retains_actual_exit_and_raw",
        "paea-capture-startup-eof-process-receipt",
        &fault_receipt_selection,
        &finished_process_outcome(
            &exit,
            &stdout_terminal,
            &stderr_terminal,
            stopped.controller_panic.is_some(),
        ),
    )
    .unwrap_or_else(|e| panic!("fault finished-path process receipt refused: {e}"));
    let fault_receipt_mode = fs::metadata(&fault_receipt_path)
        .unwrap_or_else(|e| panic!("fault receipt metadata unreadable: {e}"))
        .permissions()
        .mode();
    assert_eq!(
        fault_receipt_mode & 0o777,
        0o600,
        "fault receipt file must carry the creation-time 0600 mode: {fault_receipt_mode:o}"
    );
    // Raw ownership BEFORE any verdict: both conserved streams are read
    // into memory first, so every later judgment works on captured facts.
    let fault_stdout_bytes = fs::read(&stdout_path)
        .unwrap_or_else(|e| panic!("conserved fault stdout.raw unreadable: {e}"));
    let fault_stderr_bytes = fs::read(&stderr_path)
        .unwrap_or_else(|e| panic!("conserved fault stderr.raw unreadable: {e}"));
    // The ORIGINAL startup panic is observable only after the real
    // collect: the reader's EOF failure travels verbatim in `Stopped`.
    let payload = stopped
        .controller_panic
        .expect("the EOF-before-prepared fault must retain the ORIGINAL controller panic");
    let startup_panic_message: String = match payload.downcast::<String>() {
        Ok(text) => *text,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(text) => (*text).to_owned(),
            Err(_) => panic!(
                "startup controller panic payload is neither String nor &str; the \
                 original cause cannot be observed"
            ),
        },
    };
    match &stdout_terminal {
        paea_capture_live::LiveStreamState::Completed { bytes } => assert_eq!(
            *bytes,
            FAULT_STDOUT_FRAGMENT.len() as u64,
            "fault stdout Completed must pin the exact byte count (fragment without newline): {stdout_terminal:?}"
        ),
        _ => panic!("fault stdout capture must complete: {stdout_terminal:?}"),
    }
    match &stderr_terminal {
        paea_capture_live::LiveStreamState::Completed { bytes } => assert_eq!(
            *bytes,
            (FAULT_STDERR_DIAGNOSTIC.len() + 1) as u64,
            "fault stderr Completed must pin the exact byte count (diagnostic + newline): {stderr_terminal:?}"
        ),
        _ => panic!("fault stderr capture must complete: {stderr_terminal:?}"),
    }
    assert_eq!(
        exit.code(),
        Some(7),
        "the ONE real wait inside the finish must report the child's REAL natural exit 7 \
         (child actually reaped, never a fake cleanup exit): {exit:?}"
    );
    assert_eq!(
        fault_stdout_bytes.len(),
        FAULT_STDOUT_FRAGMENT.len(),
        "the Completed fault stdout terminal must conserve exactly the fragment bytes"
    );
    assert_eq!(
        fault_stderr_bytes.len(),
        FAULT_STDERR_DIAGNOSTIC.len() + 1,
        "the Completed fault stderr terminal must conserve exactly the diagnostic line bytes"
    );
    assert_eq!(
        fault_stdout_bytes,
        FAULT_STDOUT_FRAGMENT.as_bytes(),
        "conserved fault stdout must be exactly the partial fragment WITHOUT any newline"
    );
    let mut expected_fault_stderr = FAULT_STDERR_DIAGNOSTIC.as_bytes().to_vec();
    expected_fault_stderr.push(b'\n');
    assert_eq!(
        fault_stderr_bytes, expected_fault_stderr,
        "conserved fault stderr must be exactly the known diagnostic line"
    );
    let fault_stdout_path_display = stdout_path.display().to_string();
    assert!(
        startup_panic_message.contains("before the prepared event"),
        "the startup EOF diagnostic must retain the phase: {startup_panic_message}"
    );
    assert!(
        startup_panic_message.contains(fault_stdout_path_display.as_str()),
        "the startup EOF diagnostic must retain the raw capture path: {startup_panic_message}"
    );
    assert!(
        !startup_panic_message.contains(FAULT_STDOUT_FRAGMENT),
        "the startup EOF diagnostic must stay safe and never carry the raw captured bytes: \
         {startup_panic_message}"
    );
    assert!(
        !startup_panic_message.contains(FAULT_STDERR_DIAGNOSTIC),
        "the startup EOF diagnostic must never carry raw stream content: {startup_panic_message}"
    );
    // Personal inspection of the persisted fault receipt JSON: the REAL
    // finished facts — actual exit 7, NOT a signal, no success, controller
    // panic PRESENT (presence only; the payload text and the raw captured
    // bytes must be absent from the JSON), both Completed terminals with
    // their exact byte counts, and the real raw hashes/paths — all read
    // back from the file on disk.
    let fault_receipt: Value = serde_json::from_slice(
        &fs::read(&fault_receipt_path)
            .unwrap_or_else(|e| panic!("written fault receipt unreadable: {e}")),
    )
    .unwrap_or_else(|e| panic!("written fault receipt must be valid JSON: {e}"));
    assert_eq!(
        fault_receipt["witness"],
        "paea_capture_startup_eof_retains_actual_exit_and_raw",
        "fault receipt must carry the guard's real witness label: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["schema_version"], 1,
        "fault receipt must carry the receipt schema version: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["process"]["finished"], true,
        "fault receipt must record the actual Finished outcome: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["process"]["exit_code"], 7,
        "fault receipt must record the REAL exit 7: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["process"]["signal"], Value::Null,
        "fault receipt must record that the child was NOT killed by a Unix signal: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["process"]["success"], false,
        "fault receipt must record the real unsuccessful exit: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["process"]["controller_panic"], true,
        "fault receipt must record the retained controller-panic PRESENCE: {fault_receipt}"
    );
    let fault_receipt_serialized = serde_json::to_string(&fault_receipt)
        .unwrap_or_else(|e| panic!("fault receipt re-serialization refused: {e}"));
    assert!(
        !fault_receipt_serialized.contains("before the prepared event")
            && !fault_receipt_serialized.contains(FAULT_STDOUT_FRAGMENT)
            && !fault_receipt_serialized.contains(FAULT_STDERR_DIAGNOSTIC),
        "the receipt JSON must never carry the private panic payload text or the raw \
         captured bytes: {fault_receipt_serialized}"
    );
    assert_eq!(
        fault_receipt["process"]["stdout_terminal"], "Completed",
        "fault receipt must record the Completed stdout terminal: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["process"]["stdout_completed_bytes"],
        FAULT_STDOUT_FRAGMENT.len() as u64,
        "fault receipt must pin the exact Completed stdout byte count: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["process"]["stderr_terminal"], "Completed",
        "fault receipt must record the Completed stderr terminal: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["process"]["stderr_completed_bytes"],
        (FAULT_STDERR_DIAGNOSTIC.len() + 1) as u64,
        "fault receipt must pin the exact Completed stderr byte count: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["artifacts"]["stdout"]["path"], "stdout.raw",
        "fault receipt must carry the relative raw stdout path: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["artifacts"]["stderr"]["path"], "stderr.raw",
        "fault receipt must carry the relative raw stderr path: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["artifacts"]["stdout"]["sha256"],
        format!("{:x}", Sha256::digest(&fault_stdout_bytes)),
        "fault receipt must carry the REAL raw stdout hash of the conserved bytes: {fault_receipt}"
    );
    assert_eq!(
        fault_receipt["artifacts"]["stderr"]["sha256"],
        format!("{:x}", Sha256::digest(&fault_stderr_bytes)),
        "fault receipt must carry the REAL raw stderr hash of the conserved bytes: {fault_receipt}"
    );
}
