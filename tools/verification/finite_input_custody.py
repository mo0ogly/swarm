"""Finite-input custody: stage raw bytes from a caller-leased descriptor.

Standalone Python-stdlib helper, separate from the existing rustc capture
wrapper and from Cargo: nothing wires it into a build, no file outside the
caller-supplied output directory is touched, and no compiler, subprocess
or service is executed.  The helper reads no environment variables,
consults no argv, and never treats descriptor 0 or sys.stdin specially;
the input descriptor is always explicitly selected by the caller.

Preconditions declared by the caller (never established here)
--------------------------------------------------------------
The caller holds, externally, a finite cooperative single-writer /
single-reader fixture lease: one owned input descriptor whose producer
will close, an exclusive fresh private output directory (current-uid
owned, no group/other permission bits), and the mount/reserve descriptors
for the backing volume.  allocation_id is required provenance and is
never authentication.  The helper validates argument types, paths and
directory ownership before consuming any input byte, but a lease is a
caller assertion; a library call cannot prove it.

What capture does
-----------------
capture(input_fd, output_dir, allocation_id) reads the selected
descriptor to observed EOF (only a zero-length read counts as EOF),
writes the raw bytes without decoding, normalisation or size cap into
<output_dir>/input.partial (O_CREAT|O_EXCL), fsyncs and closes that
writer exactly once, hardlinks input.partial to input.bin (no-clobber),
reopens input.bin read-only and checks offset 0, writes manifest.partial,
fsyncs it, publishes manifest.json exclusively by no-clobber hardlink
from manifest.partial, fsyncs the output directory, closes the directory,
and only then returns (read_only_fd, manifest).  Raw bytes and JSON never
share a file; SHA-256 and the byte count describe only the bytes staged
in input.bin.

What validate_staged does
-------------------------
validate_staged(output_dir, allocation_id) is a strict read-only
consumer of an already published custody record: it writes, links,
renames, deletes and chmods nothing, executes nothing, reads no
environment variable, returns no descriptor, and never mutates a
record, not even on failure.  It re-checks the directory (real
directory via lstat, no symlink, current-uid owned, private mode),
requires exactly the four custody names input.partial, input.bin,
manifest.partial and manifest.json and nothing else in the directory,
verifies each is a regular file owned by the current uid with no
group/other permission bits, and checks current hardlink-pair
identity by device and inode, without proving creation history.  It then requires identical
manifest.partial and manifest.json bytes, parses the JSON refusing any
duplicate member at any depth, and validates the exact producer schema:
the complete member set with no missing and no foreign member, the
fixed staging and publication names and tokens, the requested
allocation_id (equality with the caller's expected provenance string,
never authentication or confinement), eof_observed and
staged_copy_complete_of_observed_stream exactly the boolean true,
byte_count an exact non-negative int (a bool is refused), sha256
exactly 64 lowercase hex digits, and the complete not_claimed block.
Finally it recomputes the byte count and SHA-256 of input.bin by
reading it exactly once, read-only, and fails unless both match the
manifest: the recomputation, never the names, labels or JSON alone, is
the hash evidence.  Only FileNotFoundError establishes a missing name;
any other error, including a permission error, keeps its original
cause and is never treated as absence.  Failures keep the static
message and typed state through FiniteInputCustodyError at stages
validate, read-manifest, parse-manifest, schema-manifest and
verify-input, with ready_fd_close_attempted always false for validator
failures.  There is no retry, fallback, timeout, size cap or cleanup,
and a successful validation is not evidence that any receiver runs,
consumes bytes or succeeds, nor of any compiler, lineage or full-graph
fact, nor of any historical input.

FIFO-to-regular-file differences (declared, not an equivalence)
---------------------------------------------------------------
When the leased descriptor is a FIFO or pipe, staging changes both timing
and seekability.  Timing: the producer must run to EOF before capture
returns (pre-read/spool); afterwards receiver reads no longer block on or
race with producer timing.  Seekability: the staged copy is a seekable,
re-readable regular file with a visible size, while a FIFO is
consume-once and non-seekable.  The receiver consumes the staged file,
never the original stream; no equivalence between the two is claimed.

Observed EOF is not upstream intent
-----------------------------------
Observed EOF proves only that a read returned zero bytes.  Whether the
upstream producer intended the stream to be complete is unknown and stays
unknown even when EOF is observed: an early producer close is
indistinguishable from a finished one, and the manifest records this as
explicitly not claimed.

Failure behaviour
-----------------
Any validation, read, write (including a zero-progress write), fsync,
close, link, reopen or manifest-publication failure aborts with
FiniteInputCustodyError: one static message plus typed stage / primary /
secondary state fields.  Every descriptor has at most one close attempt;
failures on error-path closes become secondary causes; a descriptor
already reopened on input.bin may be closed by the failure path
(ready_fd_close_attempted).  Partial files are retained, nothing is ever deleted,
rollback is never inferred, input is never re-read after a failure, and
no failure report file is guaranteed to be written.  There is no retry,
fallback, timeout, size cap or cleanup.  A successful return means only
that the staged data and the manifest were published and a read-only
descriptor at offset 0 exists; it is not evidence that any receiver runs,
consumes bytes, or succeeds.

No historical stdin reconstruction, build/profile qualification,
installation or acceptance is performed or claimed by this module; the
existing rustc capture wrapper and collector remain unchanged.
"""

import hashlib
import json
import os
import stat

__all__ = ['capture', 'validate_staged', 'FiniteInputCustodyError']

STAGING_NAME = 'input.partial'
FINAL_NAME = 'input.bin'
MANIFEST_STAGING_NAME = 'manifest.partial'
MANIFEST_NAME = 'manifest.json'
MANIFEST_SCHEMA = 'finite-input-custody/1'
_READ_CHUNK_SIZE = 65536  # Technical streaming buffer, never an input-size limit.

# Indirect syscall targets.  They exist so the unit regressions can inject
# read/write/fsync/close/link/open faults without mutating the shared os
# module; the helper itself performs each operation exactly once and never
# retries or varies it.
_os_open = os.open
_os_read = os.read
_os_write = os.write
_os_fsync = os.fsync
_os_close = os.close
_os_link = os.link
_os_lseek = os.lseek
_os_lstat = os.lstat
_os_fstat = os.fstat
_os_getuid = os.getuid


class FiniteInputCustodyError(Exception):
    """Static-message custody failure with typed, bounded state fields.

    str() of this exception is always the same fixed sentence; every
    structured fact lives in the attributes so callers never parse text.

    Attributes:
        original_exception: original unrendered cause object, or None.
        secondary_exceptions: original unrendered cleanup causes.
        stage: fixed token naming the failed step: one of validate,
            open-dir, open-staging, read-input, write-staging,
            fsync-staging, close-staging, link-input, reopen-input,
            open-manifest, write-manifest, fsync-manifest,
            close-manifest, link-manifest, fsync-dir, close-dir, unexpected
            for capture, and validate, read-manifest, parse-manifest,
            schema-manifest, verify-input, unexpected for the read-only
            validate_staged consumer.
        primary: dict for the first failure with bounded keys op (fixed
            operation token: a syscall name or one of the fixed check
            tokens validate, list, parse, schema, verify), name (fixed
            file, argument or schema-member label), errno
            (int or None), detail (None) and, for
            non-errno conditions, reason (a fixed token such as
            zero-progress, target-exists or not-private-mode).  It never
            contains input bytes, environment values or caller free text.
        secondary: list of same-shaped dicts for failures that happened
            while handling the primary failure (error-path closes).
        output_dir: the normalised output directory path when known,
            else None.
        ready_fd_close_attempted: True when a descriptor already reopened
            read-only on input.bin existed and had a close attempt on the failure
            path; a failed close does not prove closure.  It is
            capture-flow state: validate_staged failures always carry
            false. Each validator-held descriptor has one close attempt;
            a first failure is primary and cleanup failures are secondary.
    """

    def __init__(self, stage, primary, secondary=(), output_dir=None,
                 ready_fd_close_attempted=False):
        """Retain bounded metadata and original cause objects without rendering private text."""
        super().__init__('finite input custody failed')
        self.stage = stage
        self.original_exception = getattr(primary, 'exception', None)
        self.secondary_exceptions = tuple(getattr(x, 'exception', None) for x in secondary)
        self.primary = dict(primary)
        self.secondary = [dict(item) for item in secondary]
        self.output_dir = output_dir
        self.ready_fd_close_attempted = bool(ready_fd_close_attempted)


class _Cause(dict):
    """Bounded diagnostic metadata plus an unrendered original cause object."""

    def __init__(self, fields, exception):
        """Retain bounded metadata and original cause objects without rendering private text."""
        super().__init__(fields)
        self.exception = exception


class _Abort(Exception):
    """Internal primary-failure carrier.

    Always converted into FiniteInputCustodyError at the capture boundary;
    it never escapes this module.
    """

    def __init__(self, stage, primary):
        """Retain bounded metadata and original cause objects without rendering private text."""
        Exception.__init__(self, stage)
        self.stage = stage
        self.primary = primary


def _cause(op, name, exc=None, reason=None):
    """Build one bounded cause dict for exception state fields.

    op and name are fixed tokens, errno comes from the OSError (or is
    None), detail is None; original exception objects are retained separately and reason is a fixed
    token for non-errno conditions.  No input bytes, environment values
    or caller-controlled free text are included.
    """
    errno_value = getattr(exc, 'errno', None)
    detail = None  # Foreign exception text is never rendered.
    cause = _Cause({'op': op, 'name': name, 'errno': errno_value, 'detail': detail}, exc)
    if reason is not None:
        cause['reason'] = reason
    return cause


def _write_all(fd, data, label, stage):
    """Write every byte of data to fd, looping over short writes.

    Short writes are normal and simply continue.  A write returning zero
    or a negative count makes no progress and aborts with a
    zero-progress cause; an OSError aborts with the original errno.
    Neither is ever retried.  Raises _Abort(stage, ...) on failure;
    on success, the handed-over bytes have been written in order; failure may leave a prefix.
    """
    view = memoryview(data)
    while view:
        try:
            written = _os_write(fd, view)
        except OSError as exc:
            raise _Abort(stage, _cause('write', label, exc)) from None
        if written <= 0:
            raise _Abort(stage, _cause('write', label, reason='zero-progress'))
        view = view[written:]


def _fsync_checked(fd, label, stage):
    """fsync one helper-held descriptor once; abort custody on failure."""
    try:
        _os_fsync(fd)
    except OSError as exc:
        raise _Abort(stage, _cause('fsync', label, exc)) from None


def _close_checked(held, label, stage):
    """Close one held descriptor exactly once; abort custody on failure.

    The label is removed from held before the close attempt so a failed
    close is never attempted a second time.
    """
    fd = held.pop(label)
    try:
        _os_close(fd)
    except OSError as exc:
        raise _Abort(stage, _cause('close', label, exc)) from None


def _release_all(held):
    """Best-effort single close of every descriptor still held.

    Returns (ready_fd_close_attempted, secondary_causes): ready_fd_close_attempted is True
    when the reopened input.bin descriptor was still held (it existed and
    has now had one close attempt; its final state is unknown on close failure).  Close failures are appended as
    bounded cause dicts; nothing is retried and nothing is deleted.
    """
    closed_ready = False
    causes = []
    for label in list(held):
        fd = held.pop(label)
        if label == FINAL_NAME:
            closed_ready = True
        try:
            _os_close(fd)
        except BaseException as exc:
            causes.append(_cause('close', label, exc))
    return closed_ready, causes


def _validate(input_fd, output_dir, allocation_id):
    """Validate argument types and directory preconditions before any read.

    Checks, in order: input_fd is a real non-negative int (bool rejected),
    allocation_id a nonempty str, output_dir a nonempty str path that
    lstat reports as a real directory (no symlink) owned by the current
    uid with no group/other permission bits, none of the four custody
    names already present, and the input descriptor statable.  Everything
    happens before a single input byte is consumed; the output parent is
    never created.

    Raises FiniteInputCustodyError with stage validate and a bounded
    primary cause on the first violated check.  Returns the normalised
    output directory path string.
    """
    if isinstance(input_fd, bool) or not isinstance(input_fd, int):
        raise FiniteInputCustodyError(
            'validate', _cause('validate', 'input-fd', reason='not-an-int'))
    if input_fd < 0:
        raise FiniteInputCustodyError(
            'validate', _cause('validate', 'input-fd', reason='negative-fd'))
    if not isinstance(allocation_id, str) or not allocation_id:
        raise FiniteInputCustodyError(
            'validate',
            _cause('validate', 'allocation-id', reason='not-nonempty-string'))
    try:
        output_path = os.fspath(output_dir)
    except TypeError:
        raise FiniteInputCustodyError(
            'validate',
            _cause('validate', 'output-dir', reason='not-a-path')) from None
    if not isinstance(output_path, str) or not output_path:
        raise FiniteInputCustodyError(
            'validate',
            _cause('validate', 'output-dir', reason='not-nonempty-str-path'))
    try:
        dir_stat = _os_lstat(output_path)
    except OSError as exc:
        raise FiniteInputCustodyError(
            'validate', _cause('lstat', 'output-dir', exc),
            output_dir=output_path) from None
    if not stat.S_ISDIR(dir_stat.st_mode):
        raise FiniteInputCustodyError(
            'validate', _cause('lstat', 'output-dir', reason='not-a-directory'),
            output_dir=output_path)
    if dir_stat.st_uid != _os_getuid():
        raise FiniteInputCustodyError(
            'validate',
            _cause('lstat', 'output-dir', reason='not-current-uid-owned'),
            output_dir=output_path)
    if stat.S_IMODE(dir_stat.st_mode) & 0o077:
        raise FiniteInputCustodyError(
            'validate', _cause('lstat', 'output-dir', reason='not-private-mode'),
            output_dir=output_path)
    for name in (STAGING_NAME, FINAL_NAME, MANIFEST_STAGING_NAME, MANIFEST_NAME):
        try:
            _os_lstat(os.path.join(output_path, name))
        except FileNotFoundError:
            continue
        except OSError as exc:
            raise FiniteInputCustodyError(
                'validate', _cause('lstat', name, exc),
                output_dir=output_path) from None
        raise FiniteInputCustodyError(
            'validate', _cause('lstat', name, reason='target-exists'),
            output_dir=output_path)
    try:
        _os_fstat(input_fd)
    except OSError as exc:
        raise FiniteInputCustodyError(
            'validate', _cause('fstat', 'input-fd', exc),
            output_dir=output_path) from None
    return output_path


def _capture_staged(input_fd, output_path, allocation_id, held):
    """Run the linear staging flow; raise _Abort on the first failure.

    held maps fixed labels to open descriptors; each entry is removed
    exactly once by its checked close, and the caller's error path closes
    any remaining entry at most once.  Effects: opens and closes the
    output directory once for its fsync, creates input.partial and
    manifest.partial exclusively, hardlinks input.bin and manifest.json
    no-clobber, fsyncs both files and the directory.  The ready input.bin
    descriptor is registered in held until the final hand-off so a later
    failure still closes it exactly once.  Returns (ready_fd, manifest)
    only after both publications, the directory fsync and the directory
    close succeed; limits: no retry, no deletion, no re-read of consumed
    input.
    """
    digest = hashlib.sha256()
    byte_count = 0

    dir_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    try:
        dir_fd = _os_open(output_path, dir_flags)
    except OSError as exc:
        raise _Abort('open-dir', _cause('open', 'output-dir', exc)) from None
    held['output-dir'] = dir_fd
    try:
        opened_dir_stat = _os_fstat(dir_fd)
    except OSError as exc:
        raise _Abort('open-dir', _cause('fstat', 'output-dir', exc)) from None
    if not stat.S_ISDIR(opened_dir_stat.st_mode):
        raise _Abort(
            'open-dir', _cause('fstat', 'output-dir', reason='not-a-directory'))

    staging_path = os.path.join(output_path, STAGING_NAME)
    try:
        staging_fd = _os_open(
            staging_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except OSError as exc:
        raise _Abort('open-staging', _cause('open', STAGING_NAME, exc)) from None
    held[STAGING_NAME] = staging_fd

    while True:
        try:
            chunk = _os_read(input_fd, _READ_CHUNK_SIZE)
        except OSError as exc:
            raise _Abort('read-input', _cause('read', 'input-fd', exc)) from None
        if chunk == b'':
            break  # observed EOF: the only EOF evidence this helper accepts
        _write_all(staging_fd, chunk, STAGING_NAME, 'write-staging')
        digest.update(chunk)
        byte_count += len(chunk)

    _fsync_checked(staging_fd, STAGING_NAME, 'fsync-staging')
    _close_checked(held, STAGING_NAME, 'close-staging')

    final_path = os.path.join(output_path, FINAL_NAME)
    try:
        _os_link(staging_path, final_path)
    except OSError as exc:
        raise _Abort('link-input', _cause('link', FINAL_NAME, exc)) from None

    try:
        ready_fd = _os_open(final_path, os.O_RDONLY)
    except OSError as exc:
        raise _Abort('reopen-input', _cause('open', FINAL_NAME, exc)) from None
    held[FINAL_NAME] = ready_fd
    try:
        ready_offset = _os_lseek(ready_fd, 0, os.SEEK_CUR)
    except OSError as exc:
        raise _Abort('reopen-input', _cause('lseek', FINAL_NAME, exc)) from None
    if ready_offset != 0:
        raise _Abort(
            'reopen-input', _cause('lseek', FINAL_NAME, reason='offset-not-zero'))

    manifest = {
        'schema': MANIFEST_SCHEMA,
        'allocation_id': allocation_id,
        'eof_observed': True,
        'byte_count': byte_count,
        'sha256': digest.hexdigest(),
        'raw_staging_file': STAGING_NAME,
        'raw_published_file': FINAL_NAME,
        'raw_publication': 'no-clobber-hardlink',
        'manifest_staging_file': MANIFEST_STAGING_NAME,
        'manifest_published_file': MANIFEST_NAME,
        'manifest_publication': 'no-clobber-hardlink',
        'staged_copy_complete_of_observed_stream': True,
        'not_claimed': {
            'upstream_intent_complete': 'unknown-even-if-eof-observed',
            'fd_readiness': 'fact-of-the-call-not-this-file',
            'api_return': 'fact-of-the-call-not-this-file',
            'receiver_consumption': 'unknown',
            'delegation_intent': 'unknown',
            'consumer_success': 'unknown',
        },
    }
    manifest_bytes = (json.dumps(manifest, sort_keys=True, indent=2) + '\n').encode('utf-8')

    manifest_partial_path = os.path.join(output_path, MANIFEST_STAGING_NAME)
    try:
        manifest_fd = _os_open(
            manifest_partial_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except OSError as exc:
        raise _Abort(
            'open-manifest', _cause('open', MANIFEST_STAGING_NAME, exc)) from None
    held[MANIFEST_STAGING_NAME] = manifest_fd
    _write_all(manifest_fd, manifest_bytes, MANIFEST_STAGING_NAME, 'write-manifest')
    _fsync_checked(manifest_fd, MANIFEST_STAGING_NAME, 'fsync-manifest')
    _close_checked(held, MANIFEST_STAGING_NAME, 'close-manifest')

    manifest_path = os.path.join(output_path, MANIFEST_NAME)
    try:
        _os_link(manifest_partial_path, manifest_path)
    except OSError as exc:
        raise _Abort('link-manifest', _cause('link', MANIFEST_NAME, exc)) from None

    _fsync_checked(dir_fd, 'output-dir', 'fsync-dir')
    _close_checked(held, 'output-dir', 'close-dir')

    # Success hand-off: the ready descriptor leaves helper custody and
    # becomes caller-owned; nothing else is left open by this helper.
    del held[FINAL_NAME]
    return ready_fd, manifest


def capture(input_fd, output_dir, allocation_id):
    """Stage raw bytes from a caller-leased finite input descriptor.

    Reads input_fd to observed EOF and stages the bytes, unmodified,
    inside the exclusive private directory output_dir; on success returns
    (read_only_fd, manifest) where read_only_fd is a caller-owned
    O_RDONLY descriptor on input.bin positioned at offset 0 and manifest
    is the same mapping published as manifest.json.

    Preconditions:
        The caller, not this helper, holds a finite cooperative
        single-writer/single-reader fixture lease on input_fd (its
        producer will close), owns the exclusive fresh private output
        directory (current-uid owned, no group/other permission bits) and
        retains the mount/reserve descriptors externally.  allocation_id
        is a required nonempty provenance string and is never
        authentication.  The input descriptor is borrowed: it is read,
        never closed, replaced or duplicated for return.

    Validation (before any input byte is consumed):
        argument types; nonempty allocation_id; output_dir a real
        directory (lstat, no symlink), owned by the current uid, with no
        group/other permission bits; the four custody names absent; the
        input descriptor statable.  The output parent is never created.

    Effects:
        Creates input.partial and manifest.partial (O_CREAT|O_EXCL, mode
        0600); hardlinks input.bin and manifest.json from them no-clobber
        (pre-existing names abort untouched); fsyncs both files and the
        output directory; opens and closes the directory once for that
        fsync; closes the staging writer, the manifest writer and the
        directory exactly once each before returning, so only the
        returned descriptor remains open from this helper.

    Errors:
        Raises FiniteInputCustodyError (static message; typed
        stage/primary/secondary state) when validation, reading, writing
        (including zero-progress), fsync, close, link, reopen or manifest
        publication fails.  Every descriptor has at most one close attempt;
        error-path close failures are appended as secondary causes; a
        ready input.bin descriptor may be closed by the failure path
        (ready_fd_close_attempted).  No rollback is inferred, no file is ever
        deleted, partial state is retained, input is never re-read after
        a failure, and no failure report file is guaranteed.

    Limits:
        EOF is observed only as a zero-length read; the helper cannot
        prove a producer will ever close, and upstream intent stays
        unknown even at observed EOF.  No timeout, size cap, retry,
        fallback or environment read; no compiler, subprocess or service
        execution; no claim that any receiver consumed or will consume
        the staged bytes; the manifest describes staged data only, not
        API-return, receiver or delegation success.
    """
    held = {}
    output_path = None
    try:
        output_path = _validate(input_fd, output_dir, allocation_id)
        return _capture_staged(input_fd, output_path, allocation_id, held)
    except BaseException as error:
        attempted_ready, secondary = _release_all(held)
        if isinstance(error, FiniteInputCustodyError):
            # Validation has no held descriptors; retain its exact cause object.
            raise
        if isinstance(error, _Abort):
            stage, primary = error.stage, error.primary
        else:
            stage = 'unexpected'
            primary = _cause('operation', 'capture', error, 'unexpected')
        raise FiniteInputCustodyError(
            stage, primary, secondary, output_dir=output_path,
            ready_fd_close_attempted=attempted_ready,
        ) from None


# ----------------------------------------------------------------------
# Read-only validation of an already staged record (validate_staged).
# Additive only: nothing above this banner, capture in particular,
# changes behaviour in any way.
# ----------------------------------------------------------------------

_CUSTODY_NAMES = (STAGING_NAME, FINAL_NAME, MANIFEST_STAGING_NAME,
                  MANIFEST_NAME)
_LOWERCASE_HEX = '0123456789abcdef'

# The exact manifest schema _capture_staged publishes above; the
# read-only validator mirrors it member for member and never accepts a
# partial or guessed schema.
_EXPECTED_MANIFEST_MEMBERS = frozenset((
    'schema', 'allocation_id', 'eof_observed', 'byte_count', 'sha256',
    'raw_staging_file', 'raw_published_file', 'raw_publication',
    'manifest_staging_file', 'manifest_published_file',
    'manifest_publication', 'staged_copy_complete_of_observed_stream',
    'not_claimed'))
_EXPECTED_NOT_CLAIMED = {
    'upstream_intent_complete': 'unknown-even-if-eof-observed',
    'fd_readiness': 'fact-of-the-call-not-this-file',
    'api_return': 'fact-of-the-call-not-this-file',
    'receiver_consumption': 'unknown',
    'delegation_intent': 'unknown',
    'consumer_success': 'unknown',
}


def _reject_duplicate_members(pairs):
    """object_pairs_hook that refuses any duplicate JSON member.

    Raises ValueError on the first repeated member at any nesting
    depth; the exception object is retained unrendered by the parse
    cause and its text never enters the bounded cause fields.
    """
    mapping = {}
    for key, value in pairs:
        if key in mapping:
            raise ValueError('duplicate JSON member')
        mapping[key] = value
    return mapping


def _validate_staged_directory(output_dir, allocation_id):
    """Validate validate_staged's arguments and record directory.

    allocation_id must be a nonempty provenance string (never
    authentication or confinement); output_dir must resolve to a
    nonempty str path whose lstat reports a real directory (no symlink)
    owned by the current uid with no group/other permission bits.  The
    directory must already exist; it is never created.  Raises
    FiniteInputCustodyError with stage validate and a bounded primary
    cause on the first violated check; nothing is opened and no record
    byte is read.  Returns the normalised output directory path.
    """
    if not isinstance(allocation_id, str) or not allocation_id:
        raise FiniteInputCustodyError(
            'validate',
            _cause('validate', 'allocation-id',
                   reason='not-nonempty-string'))
    try:
        output_path = os.fspath(output_dir)
    except TypeError:
        raise FiniteInputCustodyError(
            'validate',
            _cause('validate', 'output-dir', reason='not-a-path')) from None
    if not isinstance(output_path, str) or not output_path:
        raise FiniteInputCustodyError(
            'validate',
            _cause('validate', 'output-dir',
                   reason='not-nonempty-str-path'))
    try:
        dir_stat = _os_lstat(output_path)
    except OSError as exc:
        raise FiniteInputCustodyError(
            'validate', _cause('lstat', 'output-dir', exc),
            output_dir=output_path) from None
    if not stat.S_ISDIR(dir_stat.st_mode):
        raise FiniteInputCustodyError(
            'validate',
            _cause('lstat', 'output-dir', reason='not-a-directory'),
            output_dir=output_path)
    if dir_stat.st_uid != _os_getuid():
        raise FiniteInputCustodyError(
            'validate',
            _cause('lstat', 'output-dir', reason='not-current-uid-owned'),
            output_dir=output_path)
    if stat.S_IMODE(dir_stat.st_mode) & 0o077:
        raise FiniteInputCustodyError(
            'validate',
            _cause('lstat', 'output-dir', reason='not-private-mode'),
            output_dir=output_path)
    return output_path


def _check_staged_names(output_path):
    """Require exactly the four custody names as a private hardlinked set.

    For each of the four names in fixed order: lstat must succeed, and
    only FileNotFoundError establishes absence (a typed target-missing
    refusal); any other OSError, including a permission error, keeps
    its original cause and is never treated as absence.  Each entry
    must be a regular file (lstat, so symlinks are refused) owned by
    the current uid with no group/other permission bits.  Both
    current pairs must be real hardlinks (not historical creation proof): input.partial/input.bin
    and manifest.partial/manifest.json share device and inode.  Finally
    the directory listing must be exactly the four names; any extra
    entry is refused.  Raises FiniteInputCustodyError with stage
    validate on the first violation; nothing is opened and no record
    byte is read.
    """
    entries = {}
    for name in _CUSTODY_NAMES:
        try:
            entry = _os_lstat(os.path.join(output_path, name))
        except FileNotFoundError as exc:
            raise FiniteInputCustodyError(
                'validate',
                _cause('lstat', name, exc, reason='target-missing'),
                output_dir=output_path) from None
        except OSError as exc:
            raise FiniteInputCustodyError(
                'validate', _cause('lstat', name, exc),
                output_dir=output_path) from None
        if not stat.S_ISREG(entry.st_mode):
            raise FiniteInputCustodyError(
                'validate',
                _cause('lstat', name, reason='not-a-regular-file'),
                output_dir=output_path)
        if entry.st_uid != _os_getuid():
            raise FiniteInputCustodyError(
                'validate',
                _cause('lstat', name, reason='not-current-uid-owned'),
                output_dir=output_path)
        if stat.S_IMODE(entry.st_mode) & 0o077:
            raise FiniteInputCustodyError(
                'validate',
                _cause('lstat', name, reason='not-private-mode'),
                output_dir=output_path)
        entries[name] = entry
    raw_pair = (entries[STAGING_NAME].st_dev, entries[STAGING_NAME].st_ino)
    raw_final = (entries[FINAL_NAME].st_dev, entries[FINAL_NAME].st_ino)
    if raw_pair != raw_final:
        raise FiniteInputCustodyError(
            'validate',
            _cause('lstat', FINAL_NAME, reason='hardlink-pair-mismatch'),
            output_dir=output_path)
    manifest_pair = (entries[MANIFEST_STAGING_NAME].st_dev,
                     entries[MANIFEST_STAGING_NAME].st_ino)
    manifest_final = (entries[MANIFEST_NAME].st_dev,
                      entries[MANIFEST_NAME].st_ino)
    if manifest_pair != manifest_final:
        raise FiniteInputCustodyError(
            'validate',
            _cause('lstat', MANIFEST_NAME, reason='hardlink-pair-mismatch'),
            output_dir=output_path)
    try:
        listing = sorted(os.listdir(output_path))
    except OSError as exc:
        raise FiniteInputCustodyError(
            'validate', _cause('list', 'output-dir', exc),
            output_dir=output_path) from None
    if listing != sorted(_CUSTODY_NAMES):
        raise FiniteInputCustodyError(
            'validate',
            _cause('list', 'output-dir', reason='unexpected-entry'),
            output_dir=output_path)


def _read_staged_file(output_path, name, stage, held):
    """Read one staged file to observed EOF, read-only, closing it once.

    Opens <output_path>/<name> O_RDONLY|O_NOFOLLOW, fstat-verifies a
    regular file, reads to the zero-length-read EOF in transport chunks
    and closes the descriptor exactly once through held custody.  The
    open descriptor is registered under the file-name label so the
    validate_staged failure path closes it at most once and records a
    failed close as a secondary cause.  Raises _Abort(stage, ...) with
    the original cause on the first failure; nothing is written,
    retried or seeked.
    """
    try:
        fd = _os_open(os.path.join(output_path, name),
                      os.O_RDONLY | os.O_NOFOLLOW)
    except OSError as exc:
        raise _Abort(stage, _cause('open', name, exc)) from None
    held[name] = fd
    try:
        opened = _os_fstat(fd)
    except OSError as exc:
        raise _Abort(stage, _cause('fstat', name, exc)) from None
    if not stat.S_ISREG(opened.st_mode):
        raise _Abort(
            stage, _cause('fstat', name, reason='not-a-regular-file'))
    chunks = []
    while True:
        try:
            chunk = _os_read(fd, _READ_CHUNK_SIZE)
        except OSError as exc:
            raise _Abort(stage, _cause('read', name, exc)) from None
        if chunk == b'':
            break
        chunks.append(chunk)
    _close_checked(held, name, stage)
    return b''.join(chunks)


def _parse_staged_manifest(manifest_bytes):
    """Decode and parse the published manifest, refusing duplicate members.

    Requires valid UTF-8 and valid JSON; the object_pairs_hook refuses
    any duplicate member at any nesting depth.  Raises
    _Abort('parse-manifest', ...) with the original decode or parse
    exception retained unrendered; no file text is copied into the
    bounded cause fields.
    """
    try:
        text = manifest_bytes.decode('utf-8')
    except UnicodeDecodeError as exc:
        raise _Abort('parse-manifest',
                     _cause('parse', MANIFEST_NAME, exc)) from None
    try:
        return json.loads(text, object_pairs_hook=_reject_duplicate_members)
    except ValueError as exc:
        raise _Abort('parse-manifest',
                     _cause('parse', MANIFEST_NAME, exc)) from None


def _schema_abort(name, reason):
    """Raise one bounded schema failure: fixed member label, fixed token.

    The label is a fixed schema member name (or the manifest file name
    for structural failures) and the reason is one of the fixed tokens
    not-object, missing, unexpected-member, wrong-type, wrong-value; no
    file text is ever copied into the cause.
    """
    raise _Abort('schema-manifest', _cause('schema', name, reason=reason))


def _require_exact_string(member, value, expected):
    """Require one schema member to be exactly the expected string."""
    if not isinstance(value, str):
        _schema_abort(member, 'wrong-type')
    if value != expected:
        _schema_abort(member, 'wrong-value')


def _require_exact_true(member, value):
    """Require one schema member to be exactly the boolean true."""
    if not isinstance(value, bool):
        _schema_abort(member, 'wrong-type')
    if value is not True:
        _schema_abort(member, 'wrong-value')


def _validate_manifest_schema(parsed, allocation_id):
    """Validate the parsed manifest against the exact producer schema.

    Mirrors, member for member, the manifest _capture_staged publishes:
    the complete member set with no member missing and no foreign
    member accepted, the fixed staging/publication names and
    publication tokens, the requested allocation_id, eof_observed and
    staged_copy_complete_of_observed_stream exactly the boolean true,
    byte_count an exact non-negative int (a bool is refused), sha256
    exactly 64 lowercase hex digits, and the complete not_claimed
    block.  Raises _Abort('schema-manifest', ...) on the first
    deviation; causes carry only fixed member labels and fixed reason
    tokens, never file text.
    """
    if not isinstance(parsed, dict):
        _schema_abort(MANIFEST_NAME, 'not-object')
    present = set(parsed)
    for member in sorted(_EXPECTED_MANIFEST_MEMBERS - present):
        _schema_abort(member, 'missing')
    if present - _EXPECTED_MANIFEST_MEMBERS:
        _schema_abort(MANIFEST_NAME, 'unexpected-member')
    _require_exact_string('schema', parsed['schema'], MANIFEST_SCHEMA)
    _require_exact_string('allocation_id', parsed['allocation_id'],
                          allocation_id)
    _require_exact_true('eof_observed', parsed['eof_observed'])
    _require_exact_true('staged_copy_complete_of_observed_stream',
                        parsed['staged_copy_complete_of_observed_stream'])
    byte_count = parsed['byte_count']
    if isinstance(byte_count, bool) or not isinstance(byte_count, int):
        _schema_abort('byte_count', 'wrong-type')
    if byte_count < 0:
        _schema_abort('byte_count', 'wrong-value')
    checksum = parsed['sha256']
    if not isinstance(checksum, str):
        _schema_abort('sha256', 'wrong-type')
    if len(checksum) != 64 or any(c not in _LOWERCASE_HEX for c in checksum):
        _schema_abort('sha256', 'wrong-value')
    for member, expected in (
            ('raw_staging_file', STAGING_NAME),
            ('raw_published_file', FINAL_NAME),
            ('raw_publication', 'no-clobber-hardlink'),
            ('manifest_staging_file', MANIFEST_STAGING_NAME),
            ('manifest_published_file', MANIFEST_NAME),
            ('manifest_publication', 'no-clobber-hardlink')):
        _require_exact_string(member, parsed[member], expected)
    not_claimed = parsed['not_claimed']
    if not isinstance(not_claimed, dict):
        _schema_abort('not_claimed', 'wrong-type')
    if not_claimed != _EXPECTED_NOT_CLAIMED:
        _schema_abort('not_claimed', 'wrong-value')


def _verify_input_digest(output_path, manifest, held):
    """Recompute input.bin's byte count and SHA-256; compare the manifest.

    Opens input.bin O_RDONLY|O_NOFOLLOW, fstat-verifies a regular file,
    streams it exactly once to observed EOF updating SHA-256 and the
    byte count, and closes the descriptor exactly once.  Compares both
    recomputed values against the already schema-validated manifest
    members and raises _Abort('verify-input', ...) on any mismatch or
    read failure.  This recomputation, not the record's names, labels
    or JSON alone, is the hash evidence; nothing is written, the file
    is never read twice and no size cap applies.
    """
    try:
        fd = _os_open(os.path.join(output_path, FINAL_NAME),
                      os.O_RDONLY | os.O_NOFOLLOW)
    except OSError as exc:
        raise _Abort('verify-input',
                     _cause('open', FINAL_NAME, exc)) from None
    held[FINAL_NAME] = fd
    try:
        opened = _os_fstat(fd)
    except OSError as exc:
        raise _Abort('verify-input',
                     _cause('fstat', FINAL_NAME, exc)) from None
    if not stat.S_ISREG(opened.st_mode):
        raise _Abort('verify-input',
                     _cause('fstat', FINAL_NAME,
                            reason='not-a-regular-file'))
    digest = hashlib.sha256()
    actual_count = 0
    while True:
        try:
            chunk = _os_read(fd, _READ_CHUNK_SIZE)
        except OSError as exc:
            raise _Abort('verify-input',
                         _cause('read', FINAL_NAME, exc)) from None
        if chunk == b'':
            break
        digest.update(chunk)
        actual_count += len(chunk)
    _close_checked(held, FINAL_NAME, 'verify-input')
    if actual_count != manifest['byte_count']:
        raise _Abort('verify-input',
                     _cause('verify', FINAL_NAME,
                            reason='byte-count-mismatch'))
    if digest.hexdigest() != manifest['sha256']:
        raise _Abort('verify-input',
                     _cause('verify', FINAL_NAME,
                            reason='sha256-mismatch'))


def validate_staged(output_dir, allocation_id):
    """Validate an already staged custody record strictly, read-only.

    Re-checks the published record in output_dir against the exact
    manifest schema capture publishes and against the staged bytes
    themselves, and returns the validated manifest mapping.  It
    writes, links, renames, deletes and chmods nothing, executes
    nothing, reads no environment variable, returns no descriptor, and
    never mutates a record, not even on failure.

    Preconditions:
        The caller owns the existing private directory (current-uid
        owned, no group/other permission bits) holding a record a
        successful capture published, and holds externally whatever
        fixture lease and volume state back it; this call can never
        prove that lease.  allocation_id is the expected nonempty
        provenance string, compared for equality with the manifest
        member; it is never authentication or confinement.

    Checks, in order:
        argument types; the directory (real directory via lstat, no
        symlink, current-uid owned, private mode); exactly the four
        custody names and nothing else in the directory; each of them
        a regular file (lstat, no symlink), current-uid owned, private
        mode; both current hardlink pairs by device/inode identity
        (input.partial/input.bin and manifest.partial/manifest.json);
        identical manifest.partial and manifest.json bytes; JSON
        parsed with duplicate-member refusal; the exact producer
        schema (complete member set, fixed names and tokens, the
        expected allocation_id, eof_observed and
        staged_copy_complete_of_observed_stream exactly boolean true,
        byte_count an exact non-negative int never a bool, sha256
        exactly 64 lowercase hex digits, the complete not_claimed
        block); and the recomputed byte count and SHA-256 of input.bin
        read in one read-only pass.  The recomputation, not the names,
        labels or JSON alone, is the hash evidence.

    Returns:
        A fresh mapping parsed from the published manifest bytes; it
        equals the manifest capture returned for this record.
        Mutating the returned mapping never touches the record.  No
        descriptor is returned; every descriptor opened is closed
        exactly once.

    Errors:
        Raises FiniteInputCustodyError (static message, typed
        stage/primary/secondary state) at stages validate,
        read-manifest, parse-manifest, schema-manifest or
        verify-input, with the original exception retained unrendered.
        Only FileNotFoundError establishes a missing name; any other
        error, including a permission error, keeps its original cause
        and is never treated as absence.  A first close failure is primary;
        failures during cleanup become secondary causes;
        ready_fd_close_attempted stays false for validator failures.
        A KeyboardInterrupt or other unexpected failure becomes the
        unexpected stage with its original cause retained.

    Limits:
        No retry, fallback, timeout, size cap, environment read or
        cleanup; input.bin is read exactly once per call and nothing
        is cached between calls.  Success proves only that the
        published record is internally consistent and matches the
        staged bytes; it is not evidence that any receiver runs,
        consumes bytes or succeeds, nor of any compiler, lineage,
        full-graph or historical-input fact, and upstream intent stays
        unknown even at observed EOF.
    """
    held = {}
    output_path = None
    try:
        output_path = _validate_staged_directory(output_dir, allocation_id)
        _check_staged_names(output_path)
        staged_bytes = _read_staged_file(
            output_path, MANIFEST_STAGING_NAME, 'read-manifest', held)
        published_bytes = _read_staged_file(
            output_path, MANIFEST_NAME, 'read-manifest', held)
        if staged_bytes != published_bytes:
            raise _Abort('read-manifest',
                         _cause('verify', MANIFEST_NAME,
                                reason='staged-published-mismatch'))
        manifest = _parse_staged_manifest(published_bytes)
        _validate_manifest_schema(manifest, allocation_id)
        _verify_input_digest(output_path, manifest, held)
        return manifest
    except BaseException as error:
        # _release_all is reused for its single-attempt close semantics;
        # its capture-flow ready flag is deliberately discarded here, so
        # validator failures always carry ready_fd_close_attempted=False.
        _, secondary = _release_all(held)
        if isinstance(error, FiniteInputCustodyError):
            # Raised before any descriptor was held; keep it exactly.
            raise
        if isinstance(error, _Abort):
            stage, primary = error.stage, error.primary
        else:
            stage = 'unexpected'
            primary = _cause('operation', 'validate-staged', error,
                             'unexpected')
        raise FiniteInputCustodyError(
            stage, primary, secondary, output_dir=output_path,
        ) from None
