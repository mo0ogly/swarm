"""Controlled three-process fixture producer; no tracing or compiler admission.

Run with Python >=3.11 and -I -S -B. Caller leases the private fixture directory,
seed and this source. Python startup accesses are outside the explicit fixture
operations. This is not authority enforcement or OS confinement.
"""
import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys


def _raise_causes(body, cleanup, receipt=None):
    """Raise retained original causes, optionally attaching known child facts.

    No IO or inference. Empty causes return; a lone cause preserves its type.
    Receipt is caller-owned JSON-safe metadata, never captured child text.
    When cleanup wraps a body that already carries child facts, forward that
    carrier to the new top level; the original object remains in the group.
    """
    causes = ([body] if body is not None else []) + cleanup
    if causes:
        error = causes[0] if len(causes) == 1 else BaseExceptionGroup(
            'fixture body and cleanup failures', causes)
        if receipt is None and body is not None:
            receipt = getattr(body, 'fixture_child_receipt', None)
        if receipt is not None:
            error.fixture_child_receipt = receipt
        raise error


@contextmanager
def _owned_fd(fd):
    """Own an already opened fixture fd; attempt close and preserve both causes.

    The caller supplies the fd and lease. A refused close does not prove closure.
    """
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
    _raise_causes(body, cleanup)


def _snapshot(fd):
    """Return selected current fd facts; errors propagate, no lifetime guarantee."""
    st = os.fstat(fd)
    return {'dev': st.st_dev, 'ino': st.st_ino, 'size': st.st_size,
            'mtime_ns': st.st_mtime_ns, 'ctime_ns': st.st_ctime_ns}


def _read(fd):
    """Consume a nonempty regular fixture fd from its initial offset and hash it.

    Exact fstat size determines requests, not a quota. Refuse short EOF, a trailing
    byte or selected metadata drift. Returns bytes/facts/digest; does not close.
    Entire fixture is held in memory. Errors propagate to its fd owner.
    """
    if not stat.S_ISREG(os.fstat(fd).st_mode):
        raise ValueError('fixture is not regular')
    before = _snapshot(fd)
    if before['size'] <= 0:
        raise ValueError('fixture is empty')
    chunks, received = [], 0
    while received < before['size']:
        data = os.read(fd, before['size'] - received)
        if not data:
            raise ValueError('fixture premature EOF')
        chunks.append(data)
        received += len(data)
    if os.read(fd, 1):
        raise ValueError('fixture trailing bytes')
    after = _snapshot(fd)
    if before != after:
        raise ValueError('fixture metadata drift')
    data = b''.join(chunks)
    return data, {'before': before, 'after': after,
                  'sha256': hashlib.sha256(data).hexdigest()}


def _spawn(root, python, role, *, spoof=False):
    """Spawn one next role and retain raw output before parsing its JSON.

    Caller leases fresh role.stdout.raw/role.stderr.raw names under root.
    Explicit interpreter/source/root, LC_ALL=C only, closed extra fds, no shell,
    timeout or retry. Cache PID and numeric exit before fsync/close/hash/decode.
    On a wait failure, attempt kill/wait only for this owned child; retain all
    original causes. A failed kill/wait leaves outcome uncertain. Private raw
    files preserve child diagnostics without including their bytes in errors.
    Partial outputs remain. Healthy return includes receipt and nested report.
    """
    argv = [str(python) if not spoof else 'cd01-owned-child', '-I', '-S', '-B',
            str(Path(__file__).resolve()), '--root', str(root), '--python',
            str(python), '--role', role]
    env = {'LC_ALL': 'C'}
    paths = {name: root / (role + '.' + name + '.raw')
             for name in ('stdout', 'stderr')}
    receipt = {'argv': argv, 'environment': env, 'pid': None,
               'returncode': None, 'execution_attempted': False,
               'outputs': {name: str(path) for name, path in paths.items()}}
    child, body, streams = None, None, {}
    try:
        for name, path in paths.items():
            streams[name] = path.open('xb', buffering=0)
        receipt['execution_attempted'] = True
        child = subprocess.Popen(argv, executable=str(python), env=env,
                                 stdin=subprocess.DEVNULL,
                                 stdout=streams['stdout'], stderr=streams['stderr'],
                                 close_fds=True)
        receipt['pid'] = child.pid
        receipt['returncode'] = child.wait()
    except BaseException as error:
        body = error
    cleanup = []
    if child is not None and child.returncode is None:
        try:
            child.kill()
        except BaseException as error:
            cleanup.append(error)
        try:
            receipt['returncode'] = child.wait()
        except BaseException as error:
            cleanup.append(error)
    if child is not None:
        # wait() may raise after the Popen object already learned its exit.
        # Preserve that observed value (None stays unknown), even on this path.
        receipt['returncode'] = child.returncode
    for stream in streams.values():
        try:
            os.fsync(stream.fileno())
        except BaseException as error:
            cleanup.append(error)
        try:
            stream.close()
        except BaseException as error:
            cleanup.append(error)
    _raise_causes(body, cleanup, receipt)
    try:
        out, err = paths['stdout'].read_bytes(), paths['stderr'].read_bytes()
        receipt.update(stdout_bytes=len(out), stderr_bytes=len(err),
                       stdout_sha256=hashlib.sha256(out).hexdigest(),
                       stderr_sha256=hashlib.sha256(err).hexdigest())
        if receipt['returncode'] != 0:
            raise ValueError('owned fixture child nonzero; numeric receipt retained')
        if err:
            raise ValueError('owned fixture child emitted diagnostics')
        report = json.loads(out)
        if (not isinstance(report, dict) or report.get('role') != role or
                type(report.get('pid')) is not int or report['pid'] != receipt['pid']):
            raise ValueError('owned fixture child marker mismatch')
        return {'receipt': receipt, 'report': report}
    except BaseException as error:
        _raise_causes(error, [], receipt)


def _absent(dirfd, name):
    """Refuse an existing fixture name; observation is not race-free no-clobber."""
    try:
        os.stat(name, dir_fd=dirfd, follow_symlinks=False)
    except FileNotFoundError:
        return
    raise ValueError('fixture destination already exists')


def _child(root, python, state):
    """Exercise owned cwd/dirfd/dup/write/rename/child/unlink operations.

    Caller leases root/seed and fresh derived/renamed/output names. State is
    caller-owned role metadata, updated with the returned child receipt before
    fallible validation/unlink. Preserve partial
    output on failure; never remove seed/root. Returned fstat facts are worker
    observations, not automatically identities for every tracer event.
    """
    with _owned_fd(os.open(root, os.O_RDONLY | os.O_DIRECTORY)) as directory:
        os.chdir(root)
        with _owned_fd(os.open('seed', os.O_RDONLY, dir_fd=directory)) as source:
            with _owned_fd(os.dup(source)) as duplicate:
                os.dup2(source, duplicate)
                data, seed = _read(duplicate)
                descriptors = {'directory': directory, 'source': source,
                               'owned_duplicate': duplicate}
        _absent(directory, 'renamed')
        with _owned_fd(os.open('derived', os.O_WRONLY | os.O_CREAT | os.O_EXCL,
                               0o400, dir_fd=directory)) as output:
            written = 0
            while written < len(data):
                count = os.write(output, data[written:])
                if count <= 0:
                    raise ValueError('fixture write made no progress')
                written += count
            produced = _snapshot(output)
        os.rename('derived', 'renamed', src_dir_fd=directory, dst_dir_fd=directory)
        descendant = _spawn(root, python, 'grandchild')
        state['known_child'] = descendant['receipt']
        if descendant['report']['file']['sha256'] != seed['sha256']:
            raise ValueError('grandchild content hash mismatch')
        os.unlink('renamed', dir_fd=directory)
        _absent(directory, 'renamed')
        return {'role': 'child', 'pid': os.getpid(), 'seed': seed,
                'descriptors': descriptors, 'produced': produced,
                'written_bytes': written, 'rename': ['derived', 'renamed'],
                'unlinked': 'renamed', 'observed_absent_after_unlink': True,
                'child': descendant}


def main(argv=None):
    """Produce one declared role's JSON; propagate errors with retained causes.

    Required absolute --root is an existing caller-owned leased fixture directory
    containing a nonempty read-only seed and absent derived/renamed names.
    Required absolute --python is the caller-pinned interpreter for every role;
    carry it explicitly because spoofed argv[0] must not select another runtime.
    Required --role is parent/child/grandchild. Parent creates child, child creates
    grandchild; each spawn uses -I -S -B, explicit LC_ALL=C and closed extra fds.
    Healthy stdout is one JSON object, explicitly flushed inside the receipt
    boundary; failures remain nonzero with tracebacks
    and safe role/numeric receipt JSON on stderr, never raw child diagnostics.
    Role stdout/stderr raw files are retained privately in root; known child
    facts survive later output/unlink failures.
    Child alone changes its cwd, creates/renames/unlinks derived; every spawner
    creates private role output files. Caller sets restrictive umask. Partial files
    remain on error. No network/config inspection, compiler, tracing, authority
    enforcement, complete syscall/input graph or acceptance is provided.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--python', type=Path, required=True)
    parser.add_argument('--role', choices=('parent', 'child', 'grandchild'), required=True)
    args = parser.parse_args(argv)
    if not args.root.is_absolute() or not args.python.is_absolute():
        parser.error('root and python must be absolute caller-pinned paths')
    state = {'role': args.role, 'pid': os.getpid(), 'known_child': None}
    try:
        if args.role == 'parent':
            child = _spawn(args.root, args.python, 'child', spoof=True)
            state['known_child'] = child['receipt']
            result = {'role': 'parent', 'pid': os.getpid(), 'child': child}
        elif args.role == 'child':
            result = _child(args.root, args.python, state)
        else:
            with _owned_fd(os.open(args.root, os.O_RDONLY | os.O_DIRECTORY)) as directory:
                with _owned_fd(os.open('renamed', os.O_RDONLY, dir_fd=directory)) as source:
                    _, facts = _read(source)
            result = {'role': 'grandchild', 'pid': os.getpid(), 'file': facts}
        print(json.dumps(result), flush=True)
    except BaseException as error:
        error.fixture_role_receipt = state
        native = getattr(error, 'fixture_child_receipt', None)
        try:
            sys.stderr.write(json.dumps({'fixture_role_receipt': state,
                                         'spawn_receipt': native}) + '\n')
            sys.stderr.flush()
        except BaseException as publication:
            combined = BaseExceptionGroup('fixture failure and diagnostic publication',
                                          [error, publication])
            combined.fixture_role_receipt = state
            raise combined
        raise



if __name__ == '__main__':
    main()
