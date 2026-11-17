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

__all__ = ['capture', 'FiniteInputCustodyError']

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
            close-manifest, link-manifest, fsync-dir, close-dir, unexpected.
        primary: dict for the first failure with bounded keys op (fixed
            syscall token), name (fixed file or argument label), errno
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
            path; a failed close does not prove closure.
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
