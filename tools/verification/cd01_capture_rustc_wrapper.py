#!/usr/bin/env python3
"""CD01 rustc wrapper: publish exact pre-exec invocation intent, then delegate.

Fixed protocol (this wrapper defines and accepts no optional flags):
    argv[1]              compiler path exactly as supplied by Cargo
    argv[2:]             compiler arguments, byte-for-byte as supplied
    CD01_CAPTURE_DIR     required: absolute, existing, caller-leased, private,
                         effective-uid-owned directory under which this
                         invocation creates one exclusive record directory
    CD01_SPACEGUARD_PATH required: absolute, existing, executable, root-selected
                         trusted admission guard

Missing or invalid configuration refuses (exit 78) before any side effect; no
defaults, fallbacks, retries, timeouts, caps, quotas or configuration mutation
exist here. No environment dump or mutation is performed; the explicitly
configured guard path is recorded as part of the intended delegation.

Per invocation (including repeated same-PID invocations, which always get
distinct directories and never share an append file) the wrapper creates one
exclusive 0700 directory ``cd01-intent-<pid>-<random hex>`` beneath the opened
capture-dir fd (final component opened O_NOFOLLOW|O_DIRECTORY and fstat-checked
as effective-uid-owned and private; fds are held through publication and carry
O_CLOEXEC). Inside it:

    intent.partial  exclusive 0600 file, full-write loop (refuses zero
                    progress), data fsync, close-once; retained forever
    intent.json     no-clobber hard link of intent.partial (never renamed or
                    overwritten), created only after the partial record is
                    complete; the invocation directory is fsynced before
                    delegation
    failure.json    on any failure after the record directory exists: an
                    exclusive 0600 record of the failure stage, publication
                    state and exec_attempted flag; its own publication failure
                    is reported as a cause and success is never faked

Captured values -- compiler path, every argument, cwd, guard path -- are
recorded reversibly as hex of os.fsencode(...)/os.getcwdb() bytes (surrogate-
escaped non-UTF-8 argv round-trips on POSIX); JSON is pure ASCII; nothing is
decoded, truncated or length-capped. Delegation is os.execv() in this same
PID with the current environment untouched:
os.execv(guard, [guard, compiler, *arguments]); the guard is expected to exec
the compiler with argv[0] set to the explicit compiler path. A successful exec
never returns; if exec ever returns (test injection only) it is an error, not
success.

Every record states that it is an INTENT receipt only and that the outcome is
UNKNOWN. It never proves that the compiler ran, nor its version, identity,
return code, or any build graph; a record written before exec is not an
observed compiled output. Provenance, volume reservation and interpreter/
guard/tool pinning belong to the outer collector; this file claims none of it.

Distinction from operator policy: this wrapper implements no operator policy.
The guard's persisted storage policy is neither read nor mutated; the guard is
only exec'd. Stages, the record schema, POSIX errnos preserved on refusals
(EEXIST collision, ELOOP no-follow, ENOTDIR, EACCES, EIO, ...) and exit codes
78 (refused configuration), 1 (failed) and 130 (interrupted) are
wrapper-technical facts, separate from any operator threshold or decision.

Trust and confinement limits (caller lease): CD01_CAPTURE_DIR and
CD01_SPACEGUARD_PATH are trusted caller decisions used as ordinary stable
paths. O_NOFOLLOW opens, exclusive creation and private-mode fstat checks are
hygiene, not adversarial path confinement; a same-UID attacker with write
access to the leased directory is out of scope. argv may contain secrets: no
generic secrecy guarantee is made, and the caller is responsible for
authorizing that these inputs may be persisted.

Library hook (packet20): main() accepts a keyword-only stdin_lease. None
keeps the exact v1 protocol and record bytes unless the packet23 operator
manifest environment pair below selects an entry; with neither an explicit
lease nor the pair configured, command-line/Cargo use activates no input
capture and never reads descriptor 0. An explicit lease is a trusted-caller
library decision: an
exact five-member mapping {allocation_id, input_fd, compiler_path_hex,
compiler_arguments_hex, cwd_hex} validated in full -- including equality
with the wrapper argv and the current working directory -- before any byte
is consumed or any directory or record is created. It is a declared finite
cooperative single-writer input/privacy lease, not authentication, producer
ancestry, future EOF proof or OS confinement; no argv-minus/FIFO/env-token
inference and no hardcoded producer or mission paths exist. A selected
invocation keeps the exclusive invocation directory and publishes a v2
pre-exec intent linking the allocation id and the relative custody
directory with transfer status PENDING (never rewritten retrospectively).
Only after that intent is durably published does it create the private
stdin-custody directory, lazily import the reviewed sibling
finite_input_custody module (a missing module refuses the selected
invocation; no fallback lookup or installation), capture the leased
descriptor, register the returned read-only descriptor in close-once
custody, dup2 it onto descriptor 0 inheritable, close the spare exactly
once, publish an exclusive stdin-transfer receipt whose exec outcome stays
UNKNOWN, and only then delegate. The receipt never proves compiler
consumption. Any capture, dup2, close or publication failure delegates
nothing and falls back to no consumed-input path; a successful dup2 may
already have replaced descriptor 0 and that state is retained as observed,
with rollback never inferred.

Operator manifest selection (packet23): the CLI entry point additionally
accepts the paired optional environment variables CD01_STDIN_LEASE_MANIFEST
(absolute byte path) and CD01_STDIN_LEASE_MANIFEST_SHA256 (exactly 64
lowercase ASCII hexadecimal digits). Both absent keeps the exact v1
protocol; a partial pair refuses (78); the pair combined with an explicit
library stdin_lease argument refuses before any capture, record creation or
descriptor-0 read. The manifest file is opened once O_NOFOLLOW|O_CLOEXEC
readonly, must be a private effective-uid-owned regular file (fstat), is
read fully with no size cap and no import fallback, and its descriptor is
closed exactly once through the existing held custody before any parsing,
so neither delegation nor a refusal leaks it and no double close occurs.
The SHA-256 of the exact read bytes must equal the configured digest before
any JSON parse or selection; the loader's read window is this one file
only, while the descriptor-0 read window remains the reviewed custody
helper's finite read-until-EOF behavior. The document must be the exact
protocol cd01.stdin_lease_manifest/1 with top members exactly {schema,
record_kind, leases}, record_kind operator_stdin_lease_manifest, and a
possibly empty leases list whose entries are exactly {allocation_id,
input_fd, selector, declarations, repeat_policy}: a nonempty unique
allocation_id, the exact integer 0 for input_fd (never bool), the literal
repeat_policy "repeatable" only, the three declarations
finite_cooperative_single_writer_input, producer_will_close_input_fd and
persistence_authorized each the boolean True (never the integer 1), and a
selector {compiler_path_hex, compiler_arguments_hex, cwd_hex} of lowercase
even-length NUL-free hex decoding to a nonempty path and cwd with a
possibly empty argument list that may contain empty arguments. No operator
name, mission or date policy exists in the protocol; duplicate JSON
members, duplicate allocation ids and duplicate exact selector tuples
(even with distinct ids) refuse, and every entry is validated before any
selection. An entry whose selector bytes equal this invocation's compiler
bytes, argument bytes and current working directory yields the existing
five-member lease and the reviewed v2 flow; no match keeps exact v1 and
never reads descriptor 0. Exact invocation byte equality is not source
identity, ancestry, an enforced future EOF, confidentiality against the
same UID or native proof, and eligibility is never inferred from '-'
argv, crate names, rustix or source text; the finite, private, cooperative
single-writer declarations remain caller preconditions. Selection is per
invocation with no shared counter, lock, global state or single-use
marker; repeated exact invocations select independently and receive their
own exclusive receipts. Malformed schema/hash/type refuses (78), open,
read and close errors keep their original type and errno as operational
failure (1), interrupts stay 130, unreadable or malformed manifests never
fall back implicitly, no raw manifest text, digest or diagnostic value is
echoed, and recovery is a corrected re-invocation with no cleanup or
rollback. The collector packet24 source optionally stages the helper and
manifest, supplies the validated environment pair and reconciles receipts;
its native Cargo wiring and actual compiler consumption remain unverified.
Without its explicit four-argument group it retains the default v1 path.

Source-only proposal: wiring into real Cargo builds requires separate review
and allocation. No cleanup is ever performed: partial and complete records are
always kept, and a reported publication error never implies a rollback.
"""

import errno
import hashlib
import json
import os
import stat
import sys

PROGRAM = "cd01_capture_rustc_wrapper"

CAPTURE_DIR_ENV = b"CD01_CAPTURE_DIR"
SPACEGUARD_ENV = b"CD01_SPACEGUARD_PATH"

INTENT_PARTIAL_NAME = b"intent.partial"
INTENT_JSON_NAME = b"intent.json"
FAILURE_JSON_NAME = b"failure.json"

INTENT_SCHEMA = "cd01.rustc_intent.v1"
INTENT_SCHEMA_V2 = "cd01.rustc_intent.v2"
FAILURE_SCHEMA = "cd01.rustc_wrapper_failure.v1"
STDIN_TRANSFER_SCHEMA = "cd01.stdin_transfer.v1"
STDIN_CUSTODY_NAME = b"stdin-custody"
STDIN_TRANSFER_NAME = b"stdin-transfer.json"
STDIN_LEASE_MEMBERS = frozenset((
    "allocation_id",
    "input_fd",
    "compiler_path_hex",
    "compiler_arguments_hex",
    "cwd_hex",
))

# Operator manifest selection (packet23): paired optional environment
# variables. Both absent keeps the exact v1 protocol; a partial pair or a
# combination with an explicit library stdin_lease argument refuses before
# any capture, record creation or descriptor-0 read.
STDIN_LEASE_MANIFEST_ENV = b"CD01_STDIN_LEASE_MANIFEST"
STDIN_LEASE_MANIFEST_SHA256_ENV = b"CD01_STDIN_LEASE_MANIFEST_SHA256"
STDIN_LEASE_MANIFEST_SCHEMA = "cd01.stdin_lease_manifest/1"
STDIN_LEASE_MANIFEST_KIND = "operator_stdin_lease_manifest"
STDIN_LEASE_MANIFEST_SHA256_LENGTH = 64

EXIT_REFUSED = 78       # missing/invalid configuration (EX_CONFIG value; wrapper-technical)
EXIT_FAILED = 1         # operational failure: capture, publication or delegation
EXIT_INTERRUPTED = 130  # KeyboardInterrupt convention (128 + SIGINT)

_PARTIAL_FILE_FLAGS = os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC
_DIR_OPEN_FLAGS = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC


class Refusal(Exception):
    """Missing/invalid configuration or input: refuse before any side effect.

    Instances must be constructed only with literal text written in this
    module. The note is echoed to stderr and embedded in reports verbatim, so
    it must never contain argv, environment values, paths or foreign exception
    text. One late refusal exists in the selected stdin-lease flow only: a
    missing sibling custody helper is refused after the intent record, still
    before any input byte is read and with no fallback lookup or installation.
    """


class WrapperReportingError(Exception):
    """Retain causes when the final safe stderr diagnostic cannot be published.

    Attributes retain the original primary exception, ordered secondary causes,
    static stage and interrupt fact in memory for a parent consumer. The rendered
    exception is deliberately static: raw exception messages, argv and environment
    values must not appear in a fallback traceback. Raising this does not prove
    durable failure.json publication or diagnostic delivery. No retry is made.
    """

    def __init__(self, primary, stage, causes, interrupted):
        """Keep original cause objects without interpolating their text."""
        super().__init__("wrapper diagnostic publication failed; outcome UNKNOWN")
        self.primary = primary
        self.stage = stage
        self.causes = tuple(causes)
        self.interrupted = interrupted


class ExecReturned(Exception):
    """Raised when os.execv returns; a real successful exec never returns.

    Reachable only when exec is injected or fails to replace the process; the
    wrapper treats it as an error, never as success.
    """


def _validate_path_config(raw, name):
    """Validate one required byte-string path configuration value; return it.

    The value must be non-empty, NUL-free and absolute (leading b"/" on
    POSIX). This validator never interpolates the value into refusal diagnostics.
    The trusted guard path is separately recorded by the intent renderer.
    """
    if not raw:
        raise Refusal(name + " is required and must not be empty")
    if b"\x00" in raw:
        raise Refusal(name + " must not contain NUL bytes")
    if not raw.startswith(b"/"):
        raise Refusal(name + " must be an absolute path")
    return raw


def _require_private_dir(status):
    """Refuse unless a fstat describes an effective-uid-owned private directory.

    Hygiene check under the caller lease, not adversarial confinement: a
    same-UID attacker with access to the leased directory is out of scope.
    """
    if not stat.S_ISDIR(status.st_mode):
        raise Refusal("capture parent must be a directory")
    if status.st_uid != os.geteuid():
        raise Refusal("capture directory must be owned by the effective uid")
    if stat.S_IMODE(status.st_mode) & 0o077:
        raise Refusal("capture directory must be private (no group/other permission bits)")


def _write_all(fd, payload):
    """Write the whole payload, looping over short writes; refuse zero progress.

    os.write() may write fewer bytes than requested; the loop continues from
    the unwritten remainder. A zero-progress write raises OSError(EIO) rather
    than looping forever; EINTR is already retried inside os.write itself.
    """
    view = memoryview(payload)
    while view:
        written = os.write(fd, view)
        if written <= 0:
            raise OSError(errno.EIO, "os.write reported zero progress")
        view = view[written:]


def _hold(fds, fd, label):
    """Register fd for exactly-once closing under a static diagnostic label."""
    entry = {"fd": fd, "label": label, "closed": False}
    fds.append(entry)
    return entry


def _close_once(entry):
    """Close a held descriptor exactly once; never retry; return any error.

    Returns None on success, otherwise the original close exception for the
    caller to preserve as a cleanup cause. An entry is never closed twice,
    even after a failed close.
    """
    if entry["closed"]:
        return None
    entry["closed"] = True
    try:
        os.close(entry["fd"])
    except BaseException as error:
        return error
    return None


def _errno_of(error):
    """Return the numeric POSIX errno carried by error, or None when absent."""
    code = getattr(error, "errno", None)
    return code if isinstance(code, int) else None


def _render_intent_record(compiler, arguments, guard, stdin_plan=None):
    """Render the pre-exec intent record as ASCII JSON bytes.

    Every captured value is reversible hex of exact bytes: compiler path and
    arguments via os.fsencode() of the operating-system argv (surrogateescape
    round-trips non-UTF-8 names on POSIX), cwd via os.getcwdb(), guard path
    likewise. Nothing is truncated or capped. The record states intent only
    and marks the outcome UNKNOWN; it never claims a compiler version,
    identity, return code, or any observed build output. No environment dump
    is made; the configured guard path is recorded explicitly. Input argv and
    intended delegation are recorded as
    separate fields.

    stdin_plan=None renders the unchanged v1 record. A plan mapping with
    allocation_id and custody_relative_directory renders the v2 record: the
    same members plus the stdin_transfer block that links the lease
    allocation id and the custody directory that will be created only after
    this record is durable. The transfer status is PENDING at intent time and
    this record is never rewritten afterwards; only the separate
    stdin-transfer receipt reports what later happened, and an old collector
    that expects the v1 schema rejects the v2 record by design.
    """
    record = {
        "schema": INTENT_SCHEMA,
        "record_kind": "pre_exec_intent",
        "outcome": "UNKNOWN",
        "outcome_note": (
            "intent receipt recorded before delegation; the compiler has not "
            "been observed to run; a successful exec never returns, so no "
            "outcome, version, identity, return code or build graph is or can "
            "be recorded by this process"
        ),
        "pid": os.getpid(),
        "cwd_hex": os.getcwdb().hex(),
        "compiler_path_hex": compiler.hex(),
        "compiler_arguments_hex": [argument.hex() for argument in arguments],
        "spaceguard_path_hex": guard.hex(),
        "delegation": {
            "mode": "os.execv in the same process (same PID)",
            "executable_hex": guard.hex(),
            "argv_hex": [guard.hex(), compiler.hex()]
                        + [argument.hex() for argument in arguments],
            "argv0_note": (
                "the exec's argv[0] is the spaceguard path; the guard is "
                "expected to exec the compiler with argv[0] set to the "
                "explicit compiler path"
            ),
            "environment_note": (
                "current process environment is delegated untouched; its "
                "values are neither recorded nor modified"
            ),
        },
        "record_location": {
            "naming": (
                "exclusive mkdir 0700 'cd01-intent-<pid>-<random hex>' under "
                "CD01_CAPTURE_DIR"
            ),
            "layout": (
                "intent.partial and intent.json (failure.json only after "
                "failures); contents mean intent only, outcome UNKNOWN"
            ),
        },
        "capture": {
            "encoding_note": (
                "each *_hex field reverses via bytes.fromhex(); text inputs "
                "were os.fsencode()d from the operating-system argv"
            ),
            "limits": "none: no truncation and no length caps",
            "append_mode": (
                "none: every invocation, including repeated same-PID "
                "invocations, writes its own exclusively created directory; "
                "records never share an append file"
            ),
        },
        "privacy": {
            "argv_secrecy": (
                "argv may contain secrets; no generic secrecy guarantee is "
                "made; the caller authorizes that these inputs may persist"
            ),
            "environment_values": "not recorded",
        },
    }
    if stdin_plan is not None:
        record["schema"] = INTENT_SCHEMA_V2
        record["stdin_transfer"] = {
            "status": "PENDING",
            "allocation_id": stdin_plan["allocation_id"],
            "custody_relative_directory": stdin_plan["custody_relative_directory"],
            "note": (
                "at intent time no input byte has been read and no custody "
                "directory exists; the transfer status stays PENDING and this "
                "record is never rewritten; the separate stdin-transfer "
                "receipt is the only completion record and it never proves "
                "compiler consumption"
            ),
        }
    return (json.dumps(record, ensure_ascii=True, sort_keys=True, indent=2)
            + "\n").encode("ascii")


def _render_failure_record(primary, stage, state, interrupted):
    """Render the wrapper failure record as ASCII JSON bytes.

    Reports only data-free facts: static stage labels, exception type names,
    numeric errnos, boolean publication-state flags and exec_attempted.
    Missing or incomplete publication flags mean the on-disk state is unknown;
    they never prove a rollback or that nothing was written. argv contents,
    environment values and paths are never included. For a selected
    stdin-lease invocation the record additionally carries the bounded
    attempt/completion booleans and, when the sibling custody helper failed,
    its bounded metadata capsule (fixed stage/op/name/reason tokens and
    numeric errnos and interruption category only); the unselected v1 record is unchanged.
    """
    record = {
        "schema": FAILURE_SCHEMA,
        "record_kind": "wrapper_failure",
        "outcome": "UNKNOWN",
        "outcome_note": (
            "a wrapper failure does not observe the compiler; the compiler "
            "outcome remains unknown"
        ),
        "interrupted": interrupted,
        "exec_attempted": state["exec_attempted"],
        "publication": {
            "invocation_dir_created": state["invocation_dir_name"] is not None,
            "capture_dir_fsynced_after_create": state["capture_dir_fsynced"],
            "intent_partial_created": state["intent_partial_created"],
            "intent_data_fsynced": state["intent_partial_fsynced"],
            "intent_json_published": state["intent_linked"],
            "invocation_dir_fsynced_after_link": state["intent_dir_fsynced"],
            "durability_note": (
                "a file may be visible even when its publication reported an "
                "error; absence of intent.json never proves intent.partial is "
                "absent or that state rolled back"
            ),
        },
        "primary_stage": stage,
        "primary_error_type": type(primary).__name__,
        "primary_errno": _errno_of(primary),
    }
    if state["stdin_selected"]:
        # Selected stdin-lease flow only; the unselected v1 failure record is
        # byte-identical to before. Every boolean below reports an observed
        # attempt or completion and never infers that a step had no effect.
        record["stdin_transfer"] = {
            "selected": True,
            "capture_attempted": state["stdin_capture_attempted"],
            "capture_completed": state["stdin_capture_completed"],
            "fd_dup2_attempted": state["stdin_dup2_attempted"],
            "fd_dup2_completed": state["stdin_dup2_completed"],
            "spare_close_attempted": state["stdin_spare_close_attempted"],
            "spare_close_completed": state["stdin_spare_close_completed"],
            "receipt_file_created": state["stdin_transfer_created"],
            "receipt_data_fsynced": state["stdin_transfer_fsynced"],
            "receipt_published": state["stdin_receipt_published"],
            "receipt_dir_fsynced": state["stdin_receipt_dir_fsynced"],
            "state_note": (
                "flags report observed attempts/completions only; a completed "
                "dup2 may already have replaced descriptor 0 even when a later "
                "step failed, and no rollback is ever inferred"
            ),
        }
        custody_error = state.get("stdin_custody_error")
        if custody_error is not None:
            record["stdin_transfer"]["custody_error"] = custody_error
    return (json.dumps(record, ensure_ascii=True, sort_keys=True, indent=2)
            + "\n").encode("ascii")


def _exclusively_write_file(dir_fd, name, payload, fds, causes, label, state=None):
    """Exclusive-create, fully write, fsync and close exactly once a new file.

    Never overwrites: O_CREAT|O_EXCL refuses an existing name (EEXIST). Raises
    the original open/write/fsync/close error of the first failing operation,
    preserving its type and errno under the caller's stage label. A close
    error never masks an earlier primary error: it is appended to causes as
    "<label>_close" instead. When state is given, sets "<label>_created" after
    the exclusive open and "<label>_fsynced" after a successful data fsync.
    """
    fd = os.open(name, _PARTIAL_FILE_FLAGS, 0o600, dir_fd=dir_fd)
    entry = _hold(fds, fd, label + "_fd")
    if state is not None:
        state[label + "_created"] = True
    primary = None
    try:
        _write_all(fd, payload)
        os.fsync(fd)
        if state is not None:
            state[label + "_fsynced"] = True
    except BaseException as error:
        primary = error
    close_error = _close_once(entry)
    if primary is not None:
        if close_error is not None:
            causes.append((label + "_close", close_error))
        raise primary
    if close_error is not None:
        raise close_error


def _failure_record_dir_fd(state, fds, causes):
    """Return a usable invocation-directory fd for failure.json, or None.

    Prefers the held invocation-directory fd; otherwise makes a single
    no-follow reopen attempt through the held parent fd (no retry loop).
    Reopen errors are preserved as causes; returning None simply skips the
    failure record when the invocation directory is unreachable.
    """
    entry = state["invocation_dir_entry"]
    if entry is not None and not entry["closed"]:
        return entry["fd"]
    parent = state["parent_dir_entry"]
    name = state["invocation_dir_name"]
    if parent is None or parent["closed"] or name is None:
        return None
    try:
        fd = os.open(name, _DIR_OPEN_FLAGS, dir_fd=parent["fd"])
    except BaseException as error:
        causes.append(("failure_record_dir_reopen", error))
        return None
    reopened = _hold(fds, fd, "invocation_dir_fd_reopened")
    state["invocation_dir_entry"] = reopened
    return fd


def _safe_fields(error):
    """Render an exception data-free: type name, numeric errno, static note.

    Never str(error): foreign exception text can embed argv, env values and
    paths. Refusal notes are module-local literals by construction.
    """
    fields = ["error=" + type(error).__name__]
    code = _errno_of(error)
    if code is not None:
        fields.append("errno=%d" % code)
    if isinstance(error, Refusal) and error.args:
        fields.append("note=" + str(error.args[0]))
    return " ".join(fields)


def _emit_stderr(primary, stage, causes, interrupted):
    """Print a data-free failure summary; single attempt, no second channel.

    Lines contain the wrapper name, a kind word, static stage labels,
    exception type names, numeric errnos and static Refusal notes only. Raw
    argv, environment values, exception text and paths are never printed.
    If write/flush fails, raise WrapperReportingError from no visible context;
    it retains primary, cleanup and reporting causes rather than swallowing them.
    It supplies no second output channel or claim of durable receipt.
    """
    if interrupted:
        kind = "interrupted"
    elif isinstance(primary, Refusal):
        kind = "refused"
    else:
        kind = "failure"
    lines = ["%s: %s stage=%s %s" % (PROGRAM, kind, stage, _safe_fields(primary))]
    for cause_stage, error in causes:
        lines.append("%s: cause stage=%s %s"
                     % (PROGRAM, cause_stage, _safe_fields(error)))
    try:
        sys.stderr.write("\n".join(lines) + "\n")
        sys.stderr.flush()
    except BaseException as error:
        raise WrapperReportingError(
            primary, stage, [*causes, ("stderr_publication", error)], interrupted
        ) from None


def _handle_failure(primary, stage, state, fds, causes):
    """Record and report a failure; always return a nonzero exit code.

    The primary cause is reported first and never replaced by a cleanup or
    failure-record outcome. When the invocation directory is reachable, an
    exclusive failure.json is attempted carrying the publication state and
    exec_attempted flag; if its own publication fails, that cause is reported
    too and success is never faked. Every held descriptor is then closed
    exactly once, with no retries. KeyboardInterrupt and other non-Exception
    BaseExceptions, including cleanup causes learned after record publication,
    are reported as interrupted in the final terminal result. The immutable
    failure record retains only facts known at its earlier publication point.
    """
    interrupted = isinstance(primary, KeyboardInterrupt) or not isinstance(primary, Exception) or state.get("stdin_custody_interrupted", False)
    record_dir_fd = _failure_record_dir_fd(state, fds, causes)
    if record_dir_fd is not None:
        try:
            payload = _render_failure_record(primary, stage, state, interrupted)
            _exclusively_write_file(record_dir_fd, FAILURE_JSON_NAME, payload,
                                    fds, causes, "failure_record")
            os.fsync(record_dir_fd)
        except BaseException as error:
            causes.append(("failure_record_publication", error))
    for entry in fds:
        close_error = _close_once(entry)
        if close_error is not None:
            causes.append((entry["label"] + "_close", close_error))
    # Cleanup happens after the immutable failure record's publication point.
    # Retain its causes and classify the final terminal result from all known
    # primary/cleanup interruptions; do not rewrite the earlier record.
    interrupted = interrupted or any(not isinstance(error, Exception)
                                     for _, error in causes)
    _emit_stderr(primary, stage, causes, interrupted)
    if interrupted:
        return EXIT_INTERRUPTED
    if isinstance(primary, Refusal):
        return EXIT_REFUSED
    return EXIT_FAILED


def _delegate(guard, compiler, arguments, state):
    """Replace this process with the guard via os.execv (same PID).

    Delegates as [guard, compiler, *arguments] with the current environment
    untouched; the guard is expected to exec the compiler with argv[0] set to
    the explicit compiler path. Marks exec_attempted before the call so a
    failed exec is still reported as attempted. Returns only if execv returns
    without replacing the process (possible solely under test injection):
    that is raised as ExecReturned, never treated as success.
    """
    delegation_argv = [guard, compiler]
    delegation_argv.extend(arguments)
    state["exec_attempted"] = True
    os.execv(guard, delegation_argv)
    raise ExecReturned("os.execv returned without replacing the process")


def _validate_stdin_lease(stdin_lease, compiler, arguments):
    """Validate an explicit trusted-caller stdin lease in full; return (id, fd).

    The lease must be a dict with exactly the five lease members:
    allocation_id (nonempty str), input_fd (an exact nonnegative int, never
    bool), compiler_path_hex and cwd_hex (hex strings that reverse to bytes)
    and compiler_arguments_hex (a list of hex strings). The reversed compiler
    bytes, argument bytes and current-working-directory bytes must equal the
    wrapper's own os.fsencode(argv) values and os.getcwdb(); any other shape
    or any mismatch is refused before any descriptor is read and before any
    directory or record is created. Hex is only a transport form: equality is
    decided on the decoded bytes. The lease is a caller declaration of a
    finite cooperative single-writer input/privacy lease; it is never
    authentication, producer ancestry, future EOF proof or OS confinement,
    and no argv-minus/FIFO/env-token inference exists here. The validated
    input descriptor is only borrowed later by the custody capture; without a
    lease this wrapper never reads descriptor 0.
    """
    if not isinstance(stdin_lease, dict):
        raise Refusal("stdin lease must be a dict with exactly the five lease members")
    if set(stdin_lease) != STDIN_LEASE_MEMBERS:
        raise Refusal("stdin lease must contain exactly the five lease members")
    allocation_id = stdin_lease["allocation_id"]
    if not isinstance(allocation_id, str) or not allocation_id:
        raise Refusal("stdin lease allocation_id must be a nonempty string")
    input_fd = stdin_lease["input_fd"]
    if type(input_fd) is not int:
        raise Refusal("stdin lease input_fd must be an exact integer file descriptor")
    if input_fd < 0:
        raise Refusal("stdin lease input_fd must be nonnegative")
    compiler_path_hex = stdin_lease["compiler_path_hex"]
    if not isinstance(compiler_path_hex, str):
        raise Refusal("stdin lease compiler_path_hex must be a hex string")
    try:
        leased_compiler = bytes.fromhex(compiler_path_hex)
    except ValueError:
        raise Refusal("stdin lease compiler_path_hex must be valid hexadecimal") from None
    if leased_compiler != compiler:
        raise Refusal("stdin lease compiler_path_hex does not match argv[1]")
    arguments_hex = stdin_lease["compiler_arguments_hex"]
    if not isinstance(arguments_hex, list):
        raise Refusal("stdin lease compiler_arguments_hex must be a list of hex strings")
    leased_arguments = []
    for entry in arguments_hex:
        if not isinstance(entry, str):
            raise Refusal("stdin lease compiler_arguments_hex must contain only hex strings")
        try:
            leased_arguments.append(bytes.fromhex(entry))
        except ValueError:
            raise Refusal("stdin lease compiler_arguments_hex must be valid hexadecimal") from None
    if leased_arguments != arguments:
        raise Refusal("stdin lease compiler_arguments_hex does not match argv[2:]")
    cwd_hex = stdin_lease["cwd_hex"]
    if not isinstance(cwd_hex, str):
        raise Refusal("stdin lease cwd_hex must be a hex string")
    try:
        leased_cwd = bytes.fromhex(cwd_hex)
    except ValueError:
        raise Refusal("stdin lease cwd_hex must be valid hexadecimal") from None
    if leased_cwd != os.getcwdb():
        raise Refusal("stdin lease cwd_hex does not match the current working directory")
    return allocation_id, input_fd


def _require_private_regular_file(status):
    """Refuse unless a fstat describes an effective-uid-owned private regular file.

    Applied to the operator manifest before any byte is read or parsed. As
    with the directory checks this is hygiene under the caller lease, not
    adversarial confinement: a same-UID attacker able to replace the
    manifest file is out of scope, and the paired SHA-256 pins the exact
    bytes that are parsed.
    """
    if not stat.S_ISREG(status.st_mode):
        raise Refusal("stdin lease manifest must be a regular file")
    if status.st_uid != os.geteuid():
        raise Refusal("stdin lease manifest must be owned by the effective uid")
    if stat.S_IMODE(status.st_mode) & 0o077:
        raise Refusal("stdin lease manifest must be private (no group/other permission bits)")


def _decode_lowercase_hex(value, name):
    """Decode one lowercase even-length hex string into exact NUL-free bytes.

    Accepts only str values of even length whose characters are ASCII
    lowercase hexadecimal digits; uppercase letters, whitespace, signs and
    odd lengths refuse. Decoded bytes must not contain NUL. The value
    itself never appears in refusal diagnostics.
    """
    if not isinstance(value, str):
        raise Refusal(name + " must be a lowercase hexadecimal string")
    if len(value) % 2 != 0 or any(character not in "0123456789abcdef" for character in value):
        raise Refusal(name + " must be lowercase even-length hexadecimal")
    decoded = bytes.fromhex(value)
    if b"\x00" in decoded:
        raise Refusal(name + " must not decode to NUL bytes")
    return decoded


def _validate_manifest_sha256_env(raw):
    """Validate the manifest SHA-256 environment value; return it as text.

    The value must be exactly 64 lowercase ASCII hexadecimal digits; any
    other type, length, casing or byte refuses before the manifest file is
    opened. The value is never echoed into diagnostics.
    """
    if (not isinstance(raw, bytes)
            or len(raw) != STDIN_LEASE_MANIFEST_SHA256_LENGTH
            or any(byte not in b"0123456789abcdef" for byte in raw)):
        raise Refusal(
            "CD01_STDIN_LEASE_MANIFEST_SHA256 must be exactly 64 lowercase "
            "hexadecimal digits")
    return raw.decode("ascii")


def _reject_duplicate_manifest_members(pairs):
    """Object-pairs hook that refuses duplicate JSON member names anywhere.

    Applied at every object depth of the manifest document; a repeated
    member name refuses the whole manifest with a static message before
    any entry is interpreted.
    """
    members = {}
    for key, value in pairs:
        if key in members:
            raise Refusal("stdin lease manifest contains duplicate JSON member names")
        members[key] = value
    return members


def _parse_manifest_document(data):
    """Parse the exact verified manifest bytes into JSON; refuse statically.

    Parsing happens only after the SHA-256 of the same bytes has matched.
    Any UTF-8 decode or JSON syntax error, and any duplicate member at any
    object depth, refuses with a static module-local message; parsed text,
    byte offsets and snippets never reach diagnostics.
    """
    try:
        return json.loads(data.decode("utf-8"), object_pairs_hook=_reject_duplicate_manifest_members)
    except Refusal:
        raise
    except ValueError:
        raise Refusal("stdin lease manifest is not valid JSON") from None


def _validate_manifest_entry(entry):
    """Validate one manifest lease entry in full; return its normalized tuple.

    The entry must be a dict with exactly the members allocation_id,
    input_fd, selector, declarations and repeat_policy: a nonempty string
    allocation_id, the exact integer 0 for input_fd (never bool), the
    literal "repeatable" repeat_policy only, declarations with exactly the
    three members finite_cooperative_single_writer_input,
    producer_will_close_input_fd and persistence_authorized, each the
    boolean True (never the integer 1), and a selector with exactly
    compiler_path_hex, compiler_arguments_hex and cwd_hex decoding from
    lowercase even-length hex to NUL-free bytes with a nonempty compiler
    path and cwd; the argument list may be empty and may contain empty
    arguments. No operator name, mission or date member exists anywhere.
    """
    if not isinstance(entry, dict):
        raise Refusal("each manifest lease entry must be a JSON object")
    if set(entry) != {"allocation_id", "input_fd", "selector", "declarations", "repeat_policy"}:
        raise Refusal("each manifest lease entry must contain exactly the five lease-entry members")
    allocation_id = entry["allocation_id"]
    if not isinstance(allocation_id, str) or not allocation_id:
        raise Refusal("manifest lease allocation_id must be a nonempty string")
    input_fd = entry["input_fd"]
    if type(input_fd) is not int or input_fd != 0:
        raise Refusal("manifest lease input_fd must be the exact integer 0")
    if not isinstance(entry["repeat_policy"], str) or entry["repeat_policy"] != "repeatable":
        raise Refusal("manifest lease repeat_policy must be the literal repeatable")
    declarations = entry["declarations"]
    if not isinstance(declarations, dict) or set(declarations) != {
            "finite_cooperative_single_writer_input",
            "producer_will_close_input_fd",
            "persistence_authorized",
    }:
        raise Refusal("manifest lease declarations must contain exactly the three declaration members")
    for name in ("finite_cooperative_single_writer_input",
                 "producer_will_close_input_fd",
                 "persistence_authorized"):
        if declarations[name] is not True:
            raise Refusal("manifest lease declarations must each be the boolean true")
    selector = entry["selector"]
    if not isinstance(selector, dict) or set(selector) != {"compiler_path_hex", "compiler_arguments_hex", "cwd_hex"}:
        raise Refusal("manifest lease selector must contain exactly the three selector members")
    selector_compiler = _decode_lowercase_hex(selector["compiler_path_hex"],
                                              "manifest lease compiler_path_hex")
    if not selector_compiler:
        raise Refusal("manifest lease compiler_path_hex must decode to a nonempty path")
    arguments_hex = selector["compiler_arguments_hex"]
    if not isinstance(arguments_hex, list):
        raise Refusal("manifest lease compiler_arguments_hex must be a list of hex strings")
    selector_arguments = []
    for argument in arguments_hex:
        selector_arguments.append(
            _decode_lowercase_hex(argument, "manifest lease compiler_arguments_hex"))
    selector_cwd = _decode_lowercase_hex(selector["cwd_hex"], "manifest lease cwd_hex")
    if not selector_cwd:
        raise Refusal("manifest lease cwd_hex must decode to a nonempty path")
    return allocation_id, input_fd, (selector_compiler, tuple(selector_arguments), selector_cwd)


def _validate_manifest_document(document):
    """Validate the whole manifest document; return every normalized entry.

    The document must be a JSON object with exactly the members schema,
    record_kind and leases; schema must equal the exact protocol constant
    cd01.stdin_lease_manifest/1 and record_kind the exact
    operator_stdin_lease_manifest kind. leases must be a list and may be
    empty. Every entry is validated before any selection is attempted;
    duplicate allocation_id values refuse even between otherwise valid
    entries, and duplicate exact selector tuples refuse even with distinct
    allocation ids.
    """
    if not isinstance(document, dict):
        raise Refusal("stdin lease manifest must be a JSON object")
    if set(document) != {"schema", "record_kind", "leases"}:
        raise Refusal("stdin lease manifest must contain exactly schema, record_kind and leases")
    if document["schema"] != STDIN_LEASE_MANIFEST_SCHEMA:
        raise Refusal("stdin lease manifest schema must be the exact protocol constant")
    if document["record_kind"] != STDIN_LEASE_MANIFEST_KIND:
        raise Refusal("stdin lease manifest record_kind must be the exact operator manifest kind")
    if not isinstance(document["leases"], list):
        raise Refusal("stdin lease manifest leases must be a list")
    entries = []
    allocation_ids = set()
    selectors = set()
    for entry in document["leases"]:
        normalized = _validate_manifest_entry(entry)
        if normalized[0] in allocation_ids:
            raise Refusal("manifest lease allocation_id values must be unique")
        allocation_ids.add(normalized[0])
        if normalized[2] in selectors:
            raise Refusal("manifest lease selector tuples must be unique")
        selectors.add(normalized[2])
        entries.append(normalized)
    return entries


def _select_manifest_lease(manifest_path, manifest_sha256, compiler, arguments, fds):
    """Load, hash-verify, parse and select from the operator manifest; or None.

    The paired environment values are validated first (absolute byte path
    through the shared path validator; exactly 64 lowercase hexadecimal
    digest digits), then the manifest is opened once O_NOFOLLOW|O_CLOEXEC
    readonly and O_NONBLOCK (avoiding FIFO-open waits before type validation),
    required to be a private effective-uid-owned regular file via
    fstat, and read fully with no size cap. The held descriptor is closed
    exactly once through the caller's close-once custody before any
    parsing, so neither delegation nor any later refusal leaks it and no
    double close can occur; open, read and close errors keep their original
    type and errno, and interrupts stay interrupts, flowing through the
    existing _handle_failure custody loop that closes every held descriptor
    exactly once. The SHA-256 of the exact read bytes must equal the
    configured digest before any JSON parse or selection; a mismatch, a
    malformed document, entry, selector, declaration or policy type
    refuses. No raw manifest text, digest or diagnostic value is ever
    echoed. When one validated entry's selector bytes equal this
    invocation's compiler bytes, argument bytes and current working
    directory, the equivalent five-member lease mapping is revalidated by
    the existing library validator and returned as (allocation_id,
    input_fd); with no match None is returned and the caller keeps the
    exact v1 protocol without ever reading descriptor 0. Selection is per
    invocation with no shared counter, lock, global state or single-use
    marker: repeated exact invocations select independently and receive
    their own exclusive receipts. The manifest declares a finite, private,
    cooperative single-writer input under caller authority; it is not
    source identity, ancestry, an enforced future EOF, confidentiality
    against the same UID or native proof, and eligibility is never inferred
    from '-' argv, crate names, rustix or source text. The manifest loader
    window is one full buffered read of this file only; the descriptor-0
    read window remains the reviewed custody helper's finite
    read-until-EOF behavior.
    """
    manifest_path = _validate_path_config(manifest_path, "CD01_STDIN_LEASE_MANIFEST")
    expected = _validate_manifest_sha256_env(manifest_sha256)
    manifest_fd = os.open(manifest_path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    entry = _hold(fds, manifest_fd, "stdin_manifest_fd")
    _require_private_regular_file(os.fstat(manifest_fd))
    chunks = []
    while True:
        chunk = os.read(manifest_fd, 65536)  # Transport chunk, never a manifest-size cap.
        if not chunk:
            break
        chunks.append(chunk)
    close_error = _close_once(entry)
    if close_error is not None:
        raise close_error
    data = b"".join(chunks)
    if hashlib.sha256(data).hexdigest() != expected:
        raise Refusal("stdin lease manifest bytes do not match the configured SHA-256")
    current_cwd = os.getcwdb()
    for allocation_id, input_fd, (selector_compiler, selector_arguments, selector_cwd) in \
            _validate_manifest_document(_parse_manifest_document(data)):
        if (selector_compiler == compiler and selector_arguments == tuple(arguments)
                and selector_cwd == current_cwd):
            lease = {
                "allocation_id": allocation_id,
                "input_fd": input_fd,
                "compiler_path_hex": selector_compiler.hex(),
                "compiler_arguments_hex": [argument.hex() for argument in selector_arguments],
                "cwd_hex": selector_cwd.hex(),
            }
            return _validate_stdin_lease(lease, compiler, arguments)
    return None


def _load_custody_module(fds):
    """Load only the explicit sibling source, without import-path/cache fallback.

    Selected fixture calls only. Open the regular source beside this wrapper,
    read its bytes once under the caller lease, close once via held custody,
    then compile those bytes in a private module namespace. No bytecode cache
    is consulted or written and no sys.modules entry is replaced. Standard
    library dependencies remain ordinary imports and outer qualification.
    Read/close/compile errors escape with their original causes; the caller's
    existing failure path closes held descriptors and prevents delegation.
    This is a trusted caller source lease, not adversarial confinement.
    """
    import types
    sibling = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                           'finite_input_custody.py')
    source_fd = os.open(sibling, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    entry = _hold(fds, source_fd, 'stdin_helper_source_fd')
    if not stat.S_ISREG(os.fstat(source_fd).st_mode):
        raise Refusal('stdin custody sibling source must be a regular file')
    chunks = []
    while True:
        chunk = os.read(source_fd, 65536)  # Transport chunk, never a source-size cap.
        if not chunk:
            break
        chunks.append(chunk)
    error = _close_once(entry)
    if error is not None:
        raise error
    module = types.ModuleType('cd01_selected_finite_input_custody')
    module.__file__ = sibling
    exec(compile(b''.join(chunks), sibling, 'exec'), module.__dict__)
    return module


def _bounded_custody_capsule(error):
    """Bound a sibling custody helper failure into JSON-safe metadata.

    Keeps only the helper's fixed stage token, its bounded primary/secondary
    cause dicts (fixed op/name tokens, numeric errnos, an always-None detail
    and optional fixed reason tokens) and the ready-descriptor close flag;
    Primary or cleanup non-Exception BaseExceptions preserve the existing
    technical interrupted category and status 130. The helper's original exception objects stay in memory through the
    caller's cause list and are never rendered. Paths, input bytes,
    environment values and free text are excluded.
    """
    originals = (error.original_exception, *error.secondary_exceptions)
    interrupted = any(isinstance(cause, BaseException) and not isinstance(cause, Exception)
                      for cause in originals)
    return {
        "interrupted": interrupted,
        "stage": error.stage,
        "primary": dict(error.primary),
        "secondary": [dict(item) for item in error.secondary],
        "ready_fd_close_attempted": bool(error.ready_fd_close_attempted),
    }


def _render_stdin_transfer_record(allocation_id, manifest):
    """Render the post-transfer, pre-exec stdin receipt as ASCII JSON bytes.

    Records only observed facts: the input digest reported by the manifest, byte count and
    staging/publication linkage produced by the sibling helper, the
    descriptor-0 replacement and spare-close attempt/completion flags, and
    an UNKNOWN delegation outcome. It never proves that the compiler
    consumed the replaced descriptor nor that delegation ran or succeeded,
    and no input bytes, environment values or paths are included.
    """
    record = {
        "schema": STDIN_TRANSFER_SCHEMA,
        "record_kind": "stdin_transfer_receipt",
        "outcome": "UNKNOWN",
        "outcome_note": (
            "receipt of staging and descriptor replacement only; it never "
            "proves that the compiler consumed descriptor 0 or that "
            "delegation ran or succeeded"
        ),
        "pid": os.getpid(),
        "allocation_id": allocation_id,
        "custody_relative_directory": STDIN_CUSTODY_NAME.decode("ascii"),
        "observed_custody_manifest": {
            "schema": manifest["schema"],
            "allocation_id": manifest["allocation_id"],
            "eof_observed": manifest["eof_observed"],
            "byte_count": manifest["byte_count"],
            "sha256": manifest["sha256"],
            "raw_staging_file": manifest["raw_staging_file"],
            "raw_published_file": manifest["raw_published_file"],
            "raw_publication": manifest["raw_publication"],
        },
        "descriptor_replacement": {
            "target_fd": 0,
            "inheritable": True,
            "dup2_attempted": True,
            "dup2_completed": True,
            "spare_close_attempted": True,
            "spare_close_completed": True,
        },
        "delegation": {
            "mode": "os.execv in the same process (same PID)",
            "outcome": "UNKNOWN",
        },
    }
    return (json.dumps(record, ensure_ascii=True, sort_keys=True, indent=2)
            + "\n").encode("ascii")


def main(argv=None, *, stdin_lease=None):
    """Run the wrapper protocol; return a nonzero code or replace the process.

    If the final diagnostic channel fails, WrapperReportingError escapes with
    original cause objects and a static safe message; CLI termination is nonzero.
    This exception does not claim on-disk rollback or diagnostic delivery.

    argv defaults to sys.argv: argv[1] is the compiler path, argv[2:] the
    exact compiler arguments; no optional wrapper flags exist. On success
    this function does not return: os.execv replaces the process with the
    guard. Every failure path -- refused configuration (78), operational
    failure (1), interrupt (130) -- returns nonzero, attempts an exclusive
    failure.json when the invocation directory exists, closes every held
    descriptor exactly once, and prints a data-free summary to stderr with
    the primary cause first. No downstream exec ever happens after a
    publication failure.

    stdin_lease=None with both operator manifest environment variables
    absent keeps the exact v1 protocol and record bytes. An explicit lease
    is validated in full against argv and the current working directory
    before anything is consumed or created, and selects the v2 flow: the
    same exclusive invocation directory
    and a v2 intent record with transfer status PENDING; after the intent is
    durably published, the private custody directory is created, the reviewed
    sibling finite_input_custody source is lazily loaded (a missing source
    refuses; no fallback) and used to stage the leased descriptor, the
    returned read-only descriptor is registered in close-once custody, dup2d
    onto descriptor 0 inheritable, the spare is closed exactly once, the
    exclusive stdin-transfer receipt is published, and only then does
    delegation run. Any capture, dup2, close or publication failure means no
    exec and no consumed-input fallback; a completed dup2 may already have
    replaced descriptor 0 and that state is retained, never rolled back.

    The CLI can equally select through the paired operator environment
    variables CD01_STDIN_LEASE_MANIFEST (absolute byte path) and
    CD01_STDIN_LEASE_MANIFEST_SHA256 (exactly 64 lowercase hexadecimal
    digits), decided in the same pre-effect window before any capture,
    record creation or descriptor-0 read: both absent keeps v1, a partial
    pair refuses, and either member combined with an explicit stdin_lease
    argument refuses. The manifest is opened once O_NOFOLLOW|O_CLOEXEC
    readonly, must be a private effective-uid-owned regular file via fstat,
    is read fully with no size cap, and its descriptor is closed exactly
    once through the shared close-once custody before any parsing. The
    SHA-256 of the exact read bytes must match the configured digest before
    the exact cd01.stdin_lease_manifest/1 protocol is parsed and every
    entry is validated; a selector entry matching this invocation's
    compiler, argument and working-directory bytes selects the same v2
    flow, while no match keeps v1 and never reads descriptor 0. Malformed
    schema/hash/type refuses (78), IO failures keep their original errno
    (1), interrupts stay 130, and no unreadable or malformed manifest ever
    falls back implicitly; no raw manifest text, digest or diagnostic value
    is echoed. Selection is per invocation with no shared counter, lock,
    global state or single-use marker.
    """
    if argv is None:
        argv = sys.argv
    fds = []     # held descriptors; each is closed exactly once
    causes = []  # secondary causes retained next to the primary (ExceptionGroup not needed)
    state = {
        "exec_attempted": False,
        "capture_dir_fsynced": False,
        "intent_partial_created": False,
        "intent_partial_fsynced": False,
        "intent_linked": False,
        "intent_dir_fsynced": False,
        "parent_dir_entry": None,
        "invocation_dir_entry": None,
        "invocation_dir_name": None,
        # Selected stdin-lease flow only; every flag reports an observed
        # attempt or completion and never infers a step had no effect. Set
        # to True only after a full library or operator-manifest lease is
        # selected; it never reflects the mere presence of an argument.
        "stdin_selected": False,
        "stdin_capture_attempted": False,
        "stdin_capture_completed": False,
        "stdin_dup2_attempted": False,
        "stdin_dup2_completed": False,
        "stdin_spare_close_attempted": False,
        "stdin_spare_close_completed": False,
        "stdin_transfer_created": False,
        "stdin_transfer_fsynced": False,
        "stdin_receipt_published": False,
        "stdin_receipt_dir_fsynced": False,
    }
    stage = "startup"
    try:
        stage = "argv_protocol"
        if len(argv) < 2:
            raise Refusal(
                "expected the compiler path as argv[1] and compiler arguments "
                "as argv[2:]; no optional wrapper flags exist")
        stage = "argv_bytes"
        compiler = os.fsencode(argv[1])
        arguments = [os.fsencode(argument) for argument in argv[2:]]
        if not compiler:
            raise Refusal("the compiler path argv[1] must not be empty")
        for raw_value in [compiler, *arguments]:
            if b"\x00" in raw_value:
                raise Refusal("compiler path and arguments must not contain NUL bytes")
        stage = "stdin_lease_validate"
        # Full lease validation (shape and equality with argv/cwd) happens
        # here, before any descriptor is read and before any directory or
        # record is created; mismatches refuse with exit 78. The paired
        # operator manifest environment is decided in the same pre-effect
        # window: a partial pair refuses, the pair combined with an explicit
        # library stdin_lease argument refuses, and a valid manifest selects
        # at most one lease. No selection at all keeps the exact v1
        # protocol and never reads descriptor 0.
        allocation_id = None
        input_fd = None
        if (stdin_lease is not None
                or STDIN_LEASE_MANIFEST_ENV in os.environb
                or STDIN_LEASE_MANIFEST_SHA256_ENV in os.environb):
            manifest_path = os.environb.get(STDIN_LEASE_MANIFEST_ENV)
            manifest_sha256 = os.environb.get(STDIN_LEASE_MANIFEST_SHA256_ENV)
            if (manifest_path is None) != (manifest_sha256 is None):
                raise Refusal(
                    "CD01_STDIN_LEASE_MANIFEST and CD01_STDIN_LEASE_MANIFEST_SHA256 "
                    "must be configured together or both absent")
            if manifest_path is not None and stdin_lease is not None:
                raise Refusal(
                    "an explicit stdin_lease argument cannot be combined with the "
                    "operator manifest environment pair")
            if stdin_lease is not None:
                allocation_id, input_fd = _validate_stdin_lease(stdin_lease, compiler, arguments)
            else:
                selected = _select_manifest_lease(
                    manifest_path, manifest_sha256, compiler, arguments, fds)
                if selected is not None:
                    allocation_id, input_fd = selected
        state["stdin_selected"] = allocation_id is not None
        stage = "env_capture_dir"
        capture_dir = os.environb.get(CAPTURE_DIR_ENV)
        if capture_dir is None:
            raise Refusal(
                "CD01_CAPTURE_DIR is required: the caller-leased private "
                "absolute output directory; there is no default or fallback")
        capture_dir = _validate_path_config(capture_dir, "CD01_CAPTURE_DIR")
        stage = "env_spaceguard"
        guard = os.environb.get(SPACEGUARD_ENV)
        if guard is None:
            raise Refusal(
                "CD01_SPACEGUARD_PATH is required: the absolute trusted guard "
                "executable chosen by the root selector; there is no default "
                "or fallback")
        guard = _validate_path_config(guard, "CD01_SPACEGUARD_PATH")
        stage = "spaceguard_verify"
        guard_status = os.stat(guard)
        if not stat.S_ISREG(guard_status.st_mode):
            raise Refusal("CD01_SPACEGUARD_PATH must name an existing regular file")
        if not os.access(guard, os.X_OK):
            raise Refusal("CD01_SPACEGUARD_PATH must be executable")
        stage = "capture_dir_open"
        parent_fd = os.open(capture_dir, _DIR_OPEN_FLAGS)
        state["parent_dir_entry"] = _hold(fds, parent_fd, "capture_dir_fd")
        stage = "capture_dir_verify"
        _require_private_dir(os.fstat(parent_fd))
        stage = "invocation_dir_create"
        invocation_name = (b"cd01-intent-" + (b"%d" % os.getpid()) + b"-"
                           + os.urandom(16).hex().encode("ascii"))
        # Exclusive creation; a collision (EEXIST) refuses, with no retry.
        os.mkdir(invocation_name, 0o700, dir_fd=parent_fd)
        state["invocation_dir_name"] = invocation_name
        stage = "capture_dir_fsync"
        os.fsync(parent_fd)
        state["capture_dir_fsynced"] = True
        stage = "invocation_dir_open"
        invocation_fd = os.open(invocation_name, _DIR_OPEN_FLAGS, dir_fd=parent_fd)
        state["invocation_dir_entry"] = _hold(fds, invocation_fd, "invocation_dir_fd")
        stage = "invocation_dir_verify"
        _require_private_dir(os.fstat(invocation_fd))
        stage = "intent_render"
        stdin_plan = None
        if state["stdin_selected"]:
            stdin_plan = {"allocation_id": allocation_id, "custody_relative_directory": STDIN_CUSTODY_NAME.decode("ascii")}
        payload = _render_intent_record(compiler, arguments, guard, stdin_plan)
        # Covers exclusive create, full write, data fsync and once-close;
        # the original errno/type identifies the failing sub-operation.
        stage = "intent_partial_publication"
        _exclusively_write_file(invocation_fd, INTENT_PARTIAL_NAME, payload,
                                fds, causes, "intent_partial", state)
        stage = "intent_link"
        # No-clobber hard link of the retained partial; os.link never
        # overwrites and no rename/replacement is used anywhere.
        os.link(INTENT_PARTIAL_NAME, INTENT_JSON_NAME,
                src_dir_fd=invocation_fd, dst_dir_fd=invocation_fd)
        state["intent_linked"] = True
        stage = "invocation_dir_fsync"
        os.fsync(invocation_fd)
        state["intent_dir_fsynced"] = True
        if state["stdin_selected"]:
            # Selected stdin-lease flow. The intent above is durable and its
            # transfer status stays PENDING; nothing below rewrites it. Any
            # failure here means no exec and no consumed-input fallback; a
            # completed dup2 may already have replaced descriptor 0 and is
            # never rolled back. The leased descriptor is borrowed by the
            # helper, the only reader; this wrapper never auto-reads fd 0.
            stage = "stdin_custody_dir_create"
            os.mkdir(STDIN_CUSTODY_NAME, 0o700, dir_fd=invocation_fd)
            stage = "stdin_custody_import"
            finite_input_custody = _load_custody_module(fds)
            stage = "stdin_custody_capture"
            state["stdin_capture_attempted"] = True
            custody_path = os.fsdecode(os.path.join(capture_dir, invocation_name, STDIN_CUSTODY_NAME))
            try:
                ready_fd, manifest = finite_input_custody.capture(input_fd, custody_path, allocation_id)
            except finite_input_custody.FiniteInputCustodyError as helper_error:
                state["stdin_custody_error"] = _bounded_custody_capsule(helper_error)
                state["stdin_custody_interrupted"] = state["stdin_custody_error"]["interrupted"]
                causes.append(("stdin_custody_capture", helper_error))
                raise
            state["stdin_capture_completed"] = True
            stage = "stdin_fd_register"
            spare_entry = _hold(fds, ready_fd, "stdin_ready_fd")
            stage = "stdin_fd_dup2"
            state["stdin_dup2_attempted"] = True
            os.dup2(ready_fd, 0, inheritable=True)
            state["stdin_dup2_completed"] = True
            stage = "stdin_spare_close"
            state["stdin_spare_close_attempted"] = True
            spare_close_error = _close_once(spare_entry)
            if spare_close_error is not None:
                raise spare_close_error
            state["stdin_spare_close_completed"] = True
            stage = "stdin_receipt_render"
            receipt = _render_stdin_transfer_record(allocation_id, manifest)
            stage = "stdin_receipt_publication"
            _exclusively_write_file(invocation_fd, STDIN_TRANSFER_NAME, receipt, fds, causes, "stdin_transfer", state)
            state["stdin_receipt_published"] = True
            stage = "stdin_receipt_dir_fsync"
            os.fsync(invocation_fd)
            state["stdin_receipt_dir_fsynced"] = True
        stage = "exec_delegation"
        _delegate(guard, compiler, arguments, state)
    except BaseException as primary:
        return _handle_failure(primary, stage, state, fds, causes)
    # Unreachable: _delegate raises ExecReturned whenever execv returns.
    return EXIT_FAILED


if __name__ == "__main__":
    sys.exit(main())
