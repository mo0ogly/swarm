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

Source-only proposal: wiring into real Cargo builds requires separate review
and allocation. No cleanup is ever performed: partial and complete records are
always kept, and a reported publication error never implies a rollback.
"""

import errno
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
FAILURE_SCHEMA = "cd01.rustc_wrapper_failure.v1"

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
    text.
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


def _render_intent_record(compiler, arguments, guard):
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
    return (json.dumps(record, ensure_ascii=True, sort_keys=True, indent=2)
            + "\n").encode("ascii")


def _render_failure_record(primary, stage, state, interrupted):
    """Render the wrapper failure record as ASCII JSON bytes.

    Reports only data-free facts: static stage labels, exception type names,
    numeric errnos, boolean publication-state flags and exec_attempted.
    Missing or incomplete publication flags mean the on-disk state is unknown;
    they never prove a rollback or that nothing was written. argv contents,
    environment values and paths are never included.
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
    BaseExceptions are reported as interrupted, not success.
    """
    interrupted = isinstance(primary, KeyboardInterrupt) or not isinstance(primary, Exception)
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


def main(argv=None):
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
        payload = _render_intent_record(compiler, arguments, guard)
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
        stage = "exec_delegation"
        _delegate(guard, compiler, arguments, state)
    except BaseException as primary:
        return _handle_failure(primary, stage, state, fds, causes)
    # Unreachable: _delegate raises ExecReturned whenever execv returns.
    return EXIT_FAILED


if __name__ == "__main__":
    sys.exit(main())
