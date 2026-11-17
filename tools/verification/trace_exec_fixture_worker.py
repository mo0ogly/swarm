'''Controlled same-PID execve/execveat fixture producer; no tracing admission.

Run with Python >= 3.11 on Linux (execveat requires Linux); -I -S -B is
recommended. The caller leases the absolute executable, private protocol
document and new receipt leaf paths, and explicitly authorizes only the
public fictitious argv and environment bytes carried by the document; the
module never explicitly reads or inspects the parent environment or merges it
into the final exec mapping. Interpreter/dynamic-loader startup is outside
these explicit operations and is not covered by this claim. This module is a fixture-only exec data producer: it does not launch
strace, choose a string width, parse traces, collect Cargo output or
authorize any use, and it provides no authority enforcement, OS confinement,
path or inode authentication, or toolchain qualification. ELF magic and
buffer digests are format observations, not safety proofs; the execve path
relies on the caller stable-pathname precondition and the fd-derived digest
is not an atomic pathname binding claim. The pre-exec receipt records intent
only with outcome UNKNOWN, renders no environment values, and attempt or
receipt facts never infer a successful exec. There is no retry, no timeout,
no size cap and never an execve fallback for execveat.
'''

import argparse
from contextlib import contextmanager
import ctypes
import errno
import hashlib
import json
import os
import re
import secrets
import stat
import sys

SCHEMA = 'trace-exec-fixture/1'
# Linux ABI constant from <linux/fcntl.h>: an ABI fact, not operator policy.
AT_EMPTY_PATH = 4096
# Applied before fstat so a FIFO can never make the worker block on open.
_DATA_FLAGS = os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK
# Held until the exec attempt; O_CLOEXEC keeps it from leaking into a
# replaced image on the success path, where this context never unwinds.
_EXECUTABLE_FLAGS = os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK
_RECEIPT_TEMP_PREFIX = '.trace-exec-fixture-receipt-'
# One os.read request size; never a cap on the total bytes a fixture may have.
_READ_BLOCK = 1 << 20
_SHA256_PATTERN = re.compile(r'[0-9a-f]{64}')
_LOWER_HEX_DIGITS = frozenset('0123456789abcdef')


class FixtureExecReturnedError(RuntimeError):
    '''Raised when a mocked or otherwise unexpected exec call returns instead of replacing the process.'''


@contextmanager
def _owned_fd(fd):
    '''Own one already-open descriptor; close it exactly once and combine causes.

    The body original error and any close error are raised together as one
group preserving both original objects; a lone cause keeps its type. A
refused close never proves closure.
    '''
    body = None
    try:
        yield fd
    except BaseException as error:
        body = error
    cleanup = []
    try:
        os.close(fd)
    except BaseException as error:
        cleanup.append(error)
    causes = ([body] if body is not None else []) + cleanup
    if not causes:
        return
    if len(causes) == 1:
        raise causes[0]
    raise BaseExceptionGroup('fixture body and close cleanup', causes)


def _reject_duplicate_members(pairs):
    '''object_pairs_hook refusing any duplicate JSON member name at any depth.'''
    unique = {}
    for key, value in pairs:
        if key in unique:
            raise ValueError('duplicate JSON member refused')
        unique[key] = value
    return unique


def _decode_hex(value, field, *, allow_empty):
    '''Decode one strictly lowercase, even-length hex JSON field into raw bytes.

    Strict typing refuses non-strings, including JSON booleans and numbers.
The field label is a constant, so no untrusted byte is ever rendered.
    '''
    if type(value) is not str:
        raise ValueError(field + ' must be a JSON string of lowercase hex digits')
    if not value and not allow_empty:
        raise ValueError(field + ' must not be empty')
    if len(value) % 2:
        raise ValueError(field + ' must contain an even number of hex digits')
    if any(char not in _LOWER_HEX_DIGITS for char in value):
        raise ValueError(field + ' must use lowercase 0-9a-f hex digits only')
    return bytes.fromhex(value)


def _read_all(fd):
    '''Read one open descriptor to EOF in fixed-size requests with no total cap.'''
    blocks = []
    while True:
        block = os.read(fd, _READ_BLOCK)
        if not block:
            return b''.join(blocks)
        blocks.append(block)


def _require_regular(status, what):
    '''Refuse a descriptor whose observed mode is not a regular file.

    The observation constrains the caller-leased input; it is not an
authorization decision and grants no confinement.
    '''
    if not stat.S_ISREG(status.st_mode):
        raise ValueError(what + ' is not a regular file')


def _require_private_regular(status, what):
    '''Refuse anything that is not a private regular file owned by the current euid.

    Ownership and mode checks are caller-lease observations used to reject
unsuitable inputs, not authorization or confinement claims.
    '''
    _require_regular(status, what)
    if status.st_uid != os.geteuid():
        raise ValueError(what + ' is not owned by the current euid')
    if status.st_mode & 0o077:
        raise ValueError(what + ' is not private: group or other permission bits are set')


def _read_private_document(path, expected_sha256):
    '''Open the leased data path non-following and non-blocking, then verify it.

    O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK are applied before fstat so a FIFO can
never make the worker wait; the descriptor must be a private regular file
owned by the current euid. The buffer is read to EOF with no size cap and
its actual digest must equal the caller-declared value before any parsing.
    '''
    fd = os.open(path, _DATA_FLAGS)
    with _owned_fd(fd):
        _require_private_regular(os.fstat(fd), 'fixture data')
        raw = _read_all(fd)
    if hashlib.sha256(raw).hexdigest() != expected_sha256:
        raise ValueError('fixture data buffer sha256 mismatch')
    return raw


def _parse_document(raw):
    '''Strictly parse one trace-exec-fixture/1 buffer into argv, environment and hex names.

    Exactly the three protocol members are accepted and duplicate members are
refused at any depth. The argv vector must be nonempty, a fixture-protocol
domain rule and not a claim about Linux in general. Decoded arguments may
be empty or non-UTF-8 but never contain NUL; environment names are nonempty
without NUL or '='; values may be empty or non-UTF-8 without NUL.
    '''
    document = json.loads(raw, object_pairs_hook=_reject_duplicate_members)
    if type(document) is not dict:
        raise ValueError('fixture document must be one JSON object')
    if set(document) != {'schema', 'argv_hex', 'environment_hex'}:
        raise ValueError('fixture members must be exactly schema, argv_hex, environment_hex')
    if document['schema'] != SCHEMA:
        raise ValueError('unsupported fixture schema')
    raw_argv = document['argv_hex']
    if type(raw_argv) is not list or not raw_argv:
        raise ValueError('fixture argv_hex must be a nonempty JSON array')
    argv = [_decode_hex(entry, 'fixture argv entry', allow_empty=True)
            for entry in raw_argv]
    for argument in argv:
        if b'\x00' in argument:
            raise ValueError('fixture argv entry contains a NUL byte')
    raw_environment = document['environment_hex']
    if type(raw_environment) is not dict:
        raise ValueError('fixture environment_hex must be one JSON object')
    environment = {}
    names = []
    for member_name, member_value in raw_environment.items():
        name = _decode_hex(member_name, 'fixture environment name', allow_empty=False)
        if b'\x00' in name or b'=' in name:
            raise ValueError('fixture environment name contains NUL or =')
        value = _decode_hex(member_value, 'fixture environment value', allow_empty=True)
        if b'\x00' in value:
            raise ValueError('fixture environment value contains a NUL byte')
        environment[name] = value
        names.append(member_name)
    return argv, environment, names


def _cwd_hex():
    '''Return the lowercase hex of the raw current-directory bytes; errors propagate.'''
    return os.fsencode(os.getcwd()).hex()


def _publish_receipt(receipt_parent, receipt_leaf, payload):
    '''Publish the intent receipt once through the full private protocol.

    The parent must be a private directory owned by the current euid. A fresh
single-attempt 0600 temp leaf is created, fully written through a partial
write loop, fsynced, hard-linked onto the absent receipt leaf with
no-clobber semantics, and the directory is fsynced. Every outcome preserves
the raw temp or final file as evidence: nothing is ever deleted, retried or
overwritten, and no step infers exec success.
    '''
    parent_fd = os.open(receipt_parent,
                        os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW)
    with _owned_fd(parent_fd):
        status = os.fstat(parent_fd)
        if not stat.S_ISDIR(status.st_mode):
            raise ValueError('receipt parent is not a directory')
        if status.st_uid != os.geteuid():
            raise ValueError('receipt parent is not owned by the current euid')
        if status.st_mode & 0o077:
            raise ValueError('receipt parent is not private: group or other bits are set')
        temp_name = (_RECEIPT_TEMP_PREFIX + str(os.getpid()) + '-'
                     + secrets.token_hex(8) + '.tmp')
        temp_fd = os.open(temp_name,
                          os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC,
                          0o600, dir_fd=parent_fd)
        with _owned_fd(temp_fd):
            os.fchmod(temp_fd, 0o600)
            view = memoryview(payload)
            while view:
                written = os.write(temp_fd, view)
                if written <= 0:
                    raise OSError(errno.EIO, 'fixture receipt write made no progress')
                view = view[written:]
            os.fsync(temp_fd)
        os.link(temp_name, receipt_leaf, src_dir_fd=parent_fd,
                dst_dir_fd=parent_fd, follow_symlinks=False)
        os.fsync(parent_fd)


def _execveat(fd, argv, environment_pairs):
    '''Execute the held descriptor via libc execveat with AT_EMPTY_PATH.

    ctypes restype and argtypes are declared explicitly, and every raw bytes
object plus both NULL-terminated arrays stay referenced for the whole native
call. AT_EMPTY_PATH (4096) is the Linux ABI constant, not operator policy.
An unavailable symbol or a refused call raises the original safe errno;
there is never an execve fallback. Any other return raises instead of
reporting success.
    '''
    libc = ctypes.CDLL(None, use_errno=True)
    execute = getattr(libc, 'execveat')  # A missing symbol raises AttributeError unchanged.
    execute.restype = ctypes.c_int
    execute.argtypes = [ctypes.c_int, ctypes.c_char_p,
                        ctypes.POINTER(ctypes.c_char_p),
                        ctypes.POINTER(ctypes.c_char_p), ctypes.c_int]
    retained = [list(argv), list(environment_pairs)]
    argv_array = (ctypes.c_char_p * (len(argv) + 1))(*argv, None)
    environment_array = (ctypes.c_char_p * (len(environment_pairs) + 1))(
        *environment_pairs, None)
    retained.append(argv_array)
    retained.append(environment_array)
    result = execute(fd, b'', argv_array, environment_array, AT_EMPTY_PATH)
    if result == -1:
        code = ctypes.get_errno()
        raise OSError(code, os.strerror(code))
    raise FixtureExecReturnedError(
        'execveat returned unexpectedly; success cannot be inferred')


def _attempt_exec(mode, executable, fd, argv, environment):
    '''Delegate exactly one same-PID exec attempt for the selected mode.

    Both success paths replace the process at the same PID and never return.
The execve path passes the caller pathname under the caller stable-pathname
precondition. A mocked or unexpected return raises.
    '''
    if mode == 'execve':
        os.execve(os.fsencode(executable), list(argv), dict(environment))
        raise FixtureExecReturnedError(
            'os.execve returned; a successful exec never returns')
    if mode == 'execveat':
        pairs = [name + b'=' + value for name, value in environment.items()]
        _execveat(fd, list(argv), pairs)
        raise FixtureExecReturnedError(
            '_execveat returned; a successful exec never returns')
    raise ValueError('unsupported fixture mode')


def _safe_report(error):
    """Project nested failures to JSON without rendering exception messages.

    Preserve type and numeric errno for groups, explicit causes and implicit
    contexts, including suppressed contexts. Repeated references terminate the
    projection without losing the first occurrence. Only worker-created safe
    metadata is attached by the caller; this is not arbitrary-object sanitizing.
    """
    seen = set()

    def typed(member):
        """Project one error graph node; mark aliases/cycles without recursion."""
        record = {'type': type(member).__name__}
        if id(member) in seen:
            record['repeated_reference'] = True
            return record
        seen.add(id(member))
        code = getattr(member, 'errno', None)
        if type(code) is int:
            record['errno'] = code
            record['errno_name'] = errno.errorcode.get(code)
        if isinstance(member, BaseExceptionGroup):
            record['children'] = [typed(child) for child in member.exceptions]
        if member.__cause__ is not None:
            record['cause'] = typed(member.__cause__)
        if member.__context__ is not None:
            record['context'] = typed(member.__context__)
        return record

    primary = typed(error)
    report = {'schema': 'trace-exec-fixture-error/1', 'primary': primary,
              'secondary': primary.get('children', [])}
    facts = getattr(error, 'fixture_exec_receipt', None)
    if isinstance(facts, dict):
        report['fixture_exec_receipt'] = facts
    return json.dumps(report, sort_keys=True)


def _is_interruption(error):
    """Find non-Exception failures anywhere in the cause/group/context graph.

    Runtime SystemExit, GeneratorExit and KeyboardInterrupt all prevent normal
    completion, including cleanup occurrences. Traverse aliases/cycles once;
    ordinary Exception-only groups remain ordinary failures.
    """
    pending, seen = [error], set()
    while pending:
        member = pending.pop()
        if id(member) in seen:
            continue
        seen.add(id(member))
        if isinstance(member, BaseExceptionGroup):
            pending.extend(member.exceptions)
        elif not isinstance(member, Exception):
            return True
        pending.extend(link for link in (member.__cause__, member.__context__)
                       if link is not None)
    return False


def _parse_args(argv):
    '''Parse and shape-check the six required CLI fields before any input output.

    Paths must be absolute caller-leased paths, digests exactly 64 lowercase
hex digits, and --receipt a new leaf name inside its parent directory.
    '''
    parser = argparse.ArgumentParser(
        description='Controlled same-PID exec fixture producer; see the module docstring.')
    parser.add_argument('--mode', choices=('execve', 'execveat'), required=True)
    parser.add_argument('--executable', required=True)
    parser.add_argument('--executable-sha256', required=True)
    parser.add_argument('--data', required=True)
    parser.add_argument('--data-sha256', required=True)
    parser.add_argument('--receipt', required=True)
    args = parser.parse_args(argv)
    for field in ('executable', 'data', 'receipt'):
        if not os.path.isabs(getattr(args, field)):
            parser.error('--' + field.replace('_', '-') +
                         ' must be an absolute caller-leased path')
    for field, flag in (('executable_sha256', '--executable-sha256'),
                        ('data_sha256', '--data-sha256')):
        if not _SHA256_PATTERN.fullmatch(getattr(args, field)):
            parser.error(flag + ' must be exactly 64 lowercase hex digits')
    if os.path.basename(args.receipt) in ('', '.', '..'):
        parser.error('--receipt must name a new leaf inside its private parent directory')
    return args


def _run(args):
    '''Validate leased inputs, publish the intent-only receipt, then attempt exec.

    Every data, euid, mode, digest and protocol validation precedes
publication and exec. A fixed ELF image is required, which avoids shebang
interpreter special cases; the magic check is a format observation, not a
safety claim. No euid or privacy requirement applies to the executable, so a
root-owned file such as /usr/bin/true is acceptable, and executable
permission errors propagate from the exec itself. On success the process is
replaced at the same PID; every failure raises with the safe typed metadata
attached to the original error object, whatever cleanup happened.
    '''
    facts = {'schema': SCHEMA, 'mode': args.mode, 'pid': os.getpid(),
             'outcome': 'UNKNOWN', 'stage': 'read-data'}
    try:
        raw = _read_private_document(args.data, args.data_sha256)
        facts['stage'] = 'parse'
        argv, environment, names = _parse_document(raw)
        facts['argv_count'] = len(argv)
        facts['environment_entry_count'] = len(environment)
        facts['stage'] = 'verify-executable'
        exec_fd = os.open(args.executable, _EXECUTABLE_FLAGS)
        with _owned_fd(exec_fd):
            status = os.fstat(exec_fd)
            _require_regular(status, 'executable')
            image = _read_all(exec_fd)
            digest = hashlib.sha256(image).hexdigest()
            if digest != args.executable_sha256:
                raise ValueError('executable buffer sha256 mismatch')
            if not image.startswith(b'\x7fELF'):
                raise ValueError('executable lacks ELF magic; a format observation is not a safety claim')
            facts['stage'] = 'publish'
            receipt = {
                'schema': SCHEMA,
                'mode': args.mode,
                'pid': os.getpid(),
                'cwd_hex': _cwd_hex(),
                'executable_sha256': digest,
                'argv_hex': [argument.hex() for argument in argv],
                'environment_names_hex': list(names),
                'environment_entry_count': len(environment),
                'outcome': 'UNKNOWN',
                'note': 'intent only',
            }
            payload = (json.dumps(receipt) + '\n').encode('utf-8')
            _publish_receipt(os.path.dirname(args.receipt),
                             os.path.basename(args.receipt), payload)
            facts['stage'] = 'exec-attempt'
            _attempt_exec(args.mode, args.executable, exec_fd, argv, environment)
    except BaseException as error:
        error.fixture_exec_receipt = dict(facts)
        raise


def main(argv=None):
    '''Run one controlled fixture exec attempt from the six required CLI fields.

    Required fields: --mode execve|execveat; absolute --executable plus
--executable-sha256; absolute private --data plus --data-sha256; and an
absolute new --receipt leaf. The worker validates shapes, digests, protocol
bytes, regularity, euid ownership and modes before publishing the
intent-only receipt, then delegates exactly one same-PID exec attempt for
the selected mode. Real success replaces the process and never returns;
every failure raises with original causes preserved and no environment
value rendered anywhere.
    '''
    _run(_parse_args(argv))


def cli_main(argv=None):
    '''Run one CLI attempt, emit a safe stderr diagnostic and map the exit code.

    Parser help/errors exit before runtime handling (0/2). A runtime SystemExit
    cannot escape as success. A known primary or cleanup interruption returns
    130 and any other handled
failure returns 1; a real success replaces the process and never reaches a
return. If writing the diagnostic itself fails, the original and publication
causes are raised together as one group carrying the safe metadata.
    '''
    args = _parse_args(argv)  # Parser help/errors keep their own exit status.
    try:
        _run(args)
    except BaseException as error:
        try:
            sys.stderr.write(_safe_report(error) + '\n')
            sys.stderr.flush()
        except BaseException as publication:
            grouped = BaseExceptionGroup(
                'fixture failure and diagnostic publication', [error, publication])
            grouped.fixture_exec_receipt = getattr(error, 'fixture_exec_receipt', None)
            raise grouped
        return 130 if _is_interruption(error) else 1
    return 1  # Unreachable: every exec attempt replaces the process or raises.


def entry_main(argv=None):
    """Map the final CLI boundary without rendering an original traceback.

    Library cli_main preserves original body/publication error objects in a
    group. The executable boundary consumes that failure and returns 130/1
    without another stderr attempt, so a recovering stream cannot expose their
    messages through Python's uncaught traceback formatter. Parser help/errors
    keep their explicit SystemExit statuses; successful exec never returns.
    Call cli_main under a protected library boundary to inspect original causes.
    """
    try:
        return cli_main(argv)
    except SystemExit:
        raise
    except BaseException as error:
        return 130 if _is_interruption(error) else 1


if __name__ == '__main__':
    raise SystemExit(entry_main())
