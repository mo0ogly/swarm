"""Fixture file rendezvous protocol between a managed fixture provider child
and its parent, without any AF_UNIX socket.

This module is the file-channel replacement for the former socket control
channel of the managed-auto fixture providers. The Biba kernel guard
voluntarily denies ``socket``/``socketpair`` for ``AF_UNIX`` so that an
outside Unix service can never become a same-UID authority bypass; this
protocol therefore uses only regular files inside one workspace directory.

Protocol inventory (all inside the workspace, one provider per turn):

- ``ready-<turn>.json`` — published by the child after it subscribed its
  inotify watch; holds ``nonce``, ``pid``, ``ppid``, ``sid``, ``pgid``.
- ``probe-<turn>-i<instance>-<seq>.request.json`` — published by the parent
  for each identified probe; holds ``{'nonce': ..., 'probe': True}``.
- ``probe-...-.reply.json`` — published by the child for each request; holds
  the same identity fields as the ready file.
- ``release-<turn>.json`` — published by the parent; must hold exactly
  ``{'nonce': ..., 'release': True}`` or the child refuses and exits non
  zero.

Limits, stated plainly: this module is a fixture protocol only. It is
neither a confinement mechanism nor an acceptance result. It proves nothing
about the real Biba policy; the managed-auto integration must establish
that separately. ``FileRendezvous`` receives an identity that the caller has
ALREADY validated and a caller-owned pidfd; it never manufactures, validates
or grants any UID/root authority, never signals a process, never closes the
pidfd, and a report never authorizes a PID — declared fields must match the
provided identity. There is no delay, no temporal poll, no volume cap, no
retry loop and no arbitrary watchdog anywhere: every wait is causal on
inotify events or pidfd readability. Publication is atomic (exclusive 0600
temp, fsync, rename) so readers observe files only after rename; originals
are never deleted.

``FileRendezvous`` optionally accepts ``terminal_fds``: caller-owned
``(role, fd)`` pairs observed by the same causal selector so a wait can also
exit when an external terminal (for example the conductor server) fires.
The option never closes or duplicates a caller descriptor and is absent by
default, which preserves the original contract unchanged.
"""

import ctypes
import json
import os
import selectors
import stat
from itertools import count
from pathlib import Path

_INOTIFY_CLOSE_WRITE = 0x00000008
_INOTIFY_MOVED_TO = 0x00000080
_INOTIFY_CREATE = 0x00000100
# 65536 bytes is the technical batch size chosen for this module: one read of
# this size drains a full inotify event batch in practice and bounds the read
# size for regular-file publications. The kernel does not mandate this value
# (it only caps one inotify event); it is a chosen technical batch size, not a
# policy cap.
_READ_BATCH = 65536

_IDENTITY_FIELDS = ('pid', 'ppid', 'sid', 'pgid')

_LIBC = ctypes.CDLL(None, use_errno=True)
_LIBC.inotify_init1.argtypes = [ctypes.c_int]
_LIBC.inotify_init1.restype = ctypes.c_int
_LIBC.inotify_add_watch.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_uint32]
_LIBC.inotify_add_watch.restype = ctypes.c_int

_INSTANCE_SEQUENCE = count(1)


def watch_directory(path):
    """Subscribe an inotify watch on ``path`` and return the descriptor.

    The descriptor is ``O_CLOEXEC | O_NONBLOCK`` and observes CLOSE_WRITE,
    MOVED_TO and CREATE events, which is exactly what atomic rename
    publication produces. Events only wake observation; callers still
    inspect the actual fresh file bytes. On add-watch failure the init
    descriptor is closed and the primary plus secondary causes are conserved
    in a ``BaseExceptionGroup``. Raises ``OSError`` when initialization or
    subscription fails.
    """
    fd = _LIBC.inotify_init1(os.O_CLOEXEC | os.O_NONBLOCK)
    if fd < 0:
        raise OSError(ctypes.get_errno(), 'owned inotify initialization')
    mask = _INOTIFY_CLOSE_WRITE | _INOTIFY_MOVED_TO | _INOTIFY_CREATE
    if _LIBC.inotify_add_watch(fd, os.fsencode(path), mask) < 0:
        error = OSError(ctypes.get_errno(), 'owned inotify watch')
        try:
            os.close(fd)
        except BaseException as cleanup:
            raise BaseExceptionGroup('inotify setup/close', [error, cleanup]) from cleanup
        raise error
    return fd


def close_descriptor(fd):
    """Close one descriptor and return ``None``.

    This is the module-level seam used by every close in this module so a
    test can inject a close refusal; the real behaviour is exactly
    ``os.close(fd)`` and failures propagate unchanged so callers conserve
    their original cause.
    """
    os.close(fd)


class _FileDescriptor:
    """Own one descriptor for the duration of a ``with`` block.

    ``__enter__`` returns the raw descriptor; ``__exit__`` closes it through
    the module ``close_descriptor`` seam. When the body succeeded, a close
    refusal propagates alone. When the body already raised, the close refusal
    is grouped with that primary cause in a ``BaseExceptionGroup`` so both
    the primary and the secondary refusal are conserved, and the ``with``
    only completes after the close outcome is known. Never suppresses the
    body exception.
    """

    def __init__(self, fd):
        self._fd = fd

    def __enter__(self):
        return self._fd

    def __exit__(self, kind, value, traceback):
        try:
            close_descriptor(self._fd)
        except BaseException as cleanup:
            if kind is None:
                raise
            raise BaseExceptionGroup('file rendezvous close', [value, cleanup]) from cleanup
        return False


def _check_component(name):
    """Refuse any rendezvous name that is not one plain workspace component.

    Raises ``ValueError`` for empty names, path separators, ``.``, ``..`` or
    NUL bytes; this is the traversal and outside-workspace refusal for every
    name this module builds or accepts.
    """
    if (
        not name
        or name in ('.', '..')
        or '/' in name
        or '\x00' in name
        or os.sep in name
        or (os.altsep and os.altsep in name)
    ):
        raise ValueError('rendezvous name must be one plain workspace component: %r' % (name,))


def _write_all(fd, data):
    """Write ``data`` fully to ``fd``, looping over short writes.

    Regular-file ``os.write`` may accept fewer bytes than requested; this
    helper loops until the memoryview is exhausted. Errors propagate with
    the bytes already written left on the caller to handle.
    """
    view = memoryview(data)
    while view:
        view = view[os.write(fd, view):]


def _check_turn_name(turn_name):
    """Validate one explicit turn name used in rendezvous file names.

    Raises ``ValueError`` unless ``turn_name`` is a non-empty single path
    component; a traversal turn name is refused here at construction time,
    which is the outside-workspace witness for the parent side.
    """
    if not isinstance(turn_name, str):
        raise ValueError('turn name must be a string')
    try:
        _check_component(turn_name)
    except ValueError as error:
        raise ValueError('invalid turn name: %s' % (error,)) from error


def program(nonce, turn_name):
    """Return the source of an inert file-protocol provider for one turn.

    The returned program mirrors the observable contract of the former
    socket provider, with every socket replaced by workspace files: it reads
    ``--model`` from argv and the raw stdin to EOF, writes
    ``model-received.json`` (fields ``pid``, ``ppid``, ``stdin_sha256``,
    ``stdin_bytes``, ``argv``), subscribes its inotify watch on the workspace
    BEFORE publishing ``ready-<turn>.json`` (exclusive 0600 temp, fsync,
    rename; fields ``nonce``, ``pid``, ``ppid``, ``sid``, ``pgid``), answers
    each identified probe request with an atomically published reply carrying
    the same identity, validates the probe and release nonces, then on a
    valid release writes ``handoff.md`` and prints the three fixture JSONL
    events (usage 7/8, ``total_cost_usd`` 1) and exits 0. No model is paid
    and no socket of any family is created.

    Defects are preserved: every refusal raises with its cause, the traceback
    reaches stderr and the real exit status is non zero. Raises ``ValueError``
    for an empty non-string nonce or an invalid turn name.
    """
    if not isinstance(nonce, str) or not nonce:
        raise ValueError('provider nonce must be a non-empty string')
    _check_turn_name(turn_name)
    source = _PROGRAM_TEMPLATE.replace('@NONCE@', repr(nonce)).replace('@TURN@', repr(turn_name))
    return source


_PROGRAM_TEMPLATE = r'''#!/usr/bin/python3
import ctypes, hashlib, json, os, selectors, sys
from pathlib import Path
nonce = @NONCE@
turn = @TURN@
def publish(name, payload):
    temp = '.' + name + '.tmp'
    descriptor = os.open(temp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        stream = os.fdopen(descriptor, 'wb')
        try:
            stream.write((json.dumps(payload) + '\n').encode())
            stream.flush()
            os.fsync(stream.fileno())
        finally:
            stream.close()
        if os.path.exists(name):
            raise RuntimeError('provider publication ' + name + ' already exists')
        os.replace(temp, name)
    except BaseException:
        try:
            os.unlink(temp)
        except OSError:
            pass
        raise
args = sys.argv[1:]
model = args[args.index('--model') + 1]
raw = sys.stdin.buffer.read()
observed = dict(pid=os.getpid(), ppid=os.getppid(), stdin_sha256=hashlib.sha256(raw).hexdigest(), stdin_bytes=len(raw), argv=args)
descriptor = os.open('model-received.json', os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
stream = os.fdopen(descriptor, 'w', encoding='utf-8')
try:
    json.dump(observed, stream)
    stream.flush()
    os.fsync(stream.fileno())
finally:
    stream.close()
libc = ctypes.CDLL(None, use_errno=True)
libc.inotify_init1.argtypes = [ctypes.c_int]
libc.inotify_init1.restype = ctypes.c_int
libc.inotify_add_watch.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_uint32]
libc.inotify_add_watch.restype = ctypes.c_int
descriptor = libc.inotify_init1(os.O_CLOEXEC | os.O_NONBLOCK)
if descriptor < 0:
    raise OSError(ctypes.get_errno(), 'provider inotify')
try:
    if libc.inotify_add_watch(descriptor, b'.', 8 | 128 | 256) < 0:
        raise OSError(ctypes.get_errno(), 'provider workspace watch')
    identity = dict(pid=os.getpid(), ppid=os.getppid(), sid=os.getsid(0), pgid=os.getpgrp(), nonce=nonce)
    publish('ready-' + turn + '.json', identity)
    answered = set()
    parent_pidfd = os.pidfd_open(os.getppid())
    try:
        with selectors.DefaultSelector() as selector:
            selector.register(descriptor, selectors.EVENT_READ, 'wake')
            selector.register(parent_pidfd, selectors.EVENT_READ, 'parent')
            while not Path('release-' + turn + '.json').exists():
                for entry in sorted(os.listdir('.')):
                    if not (entry.startswith('probe-' + turn + '-') and entry.endswith('.request.json')):
                        continue
                    if entry in answered:
                        continue
                    request = json.loads(Path(entry).read_bytes())
                    if request != {'nonce': nonce, 'probe': True}:
                        raise RuntimeError('provider probe request nonce mismatch')
                    publish(entry[:-len('.request.json')] + '.reply.json', identity)
                    answered.add(entry)
                for key, _ in selector.select():
                    if key.data == 'parent':
                        raise RuntimeError('provider parent exited before release')
                    try:
                        os.read(descriptor, 65536)
                    except BlockingIOError:
                        pass
    finally:
        os.close(parent_pidfd)
    if json.loads(Path('release-' + turn + '.json').read_bytes()) != {'nonce': nonce, 'release': True}:
        raise RuntimeError('provider release mismatch')
finally:
    os.close(descriptor)
descriptor = os.open('handoff.md', os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
report = os.fdopen(descriptor, 'w', encoding='utf-8')
try:
    report.write('Owned inert worker\nInput SHA256: ' + observed['stdin_sha256'] + '\n')
    report.flush()
    os.fsync(report.fileno())
finally:
    report.close()
print(json.dumps({'type': 'system', 'subtype': 'init', 'model': model}), flush=True)
print(json.dumps({'type': 'assistant', 'message': {'model': model, 'content': [{'type': 'text', 'text': 'owned independent worker'}]}}), flush=True)
print(json.dumps({'type': 'result', 'usage': {'input_tokens': 7, 'output_tokens': 8}, 'total_cost_usd': 1}), flush=True)
'''


class FileRendezvous:
    """Parent side of the file rendezvous for one turn of one provider child.

    The constructor receives the workspace directory, the expected nonce, the
    explicit turn name, an identity dict that the caller has ALREADY
    validated, and a caller-owned pidfd. It copies the four ``pid``/``ppid``/
    ``sid``/``pgid`` fields for comparison, never manufactures or validates
    any UID/root authority, never signals a process and never closes the
    pidfd; the pidfd is only observed for readability, which the kernel
    provides exactly when the child has terminated. Declared nonce and PID
    fields in published files must match the provided identity — a report
    never authorizes a PID.

    The API is explicit: ``ready()``, ``probe()`` and ``release()``. The
    inotify subscription happens before the first read; each wait is a causal
    ``select`` on the watch descriptor and the pidfd, so a child that dies
    before ready or before a probe reply terminates the wait with a refusal
    instead of looping. Publications are read completely only after their
    atomic rename, are validated by ``lstat`` (regular file, own UID, single
    link) and are opened ``O_NOFOLLOW``; symlinks, hard links, traversal
    names, outside-workspace paths, wrong nonces and mismatched declared PIDs
    are refused with the original files preserved. Parent publications are
    exclusive 0600 temp plus fsync plus rename; existing files are never
    replaced or deleted. Probe names embed a process-wide instance number and
    a per-instance sequence, so probes are identified and never reused, with
    no cap on their count.

    Limits: fixture protocol only — neither confinement nor acceptance, and
    no proof of the real Biba policy. ``close()`` closes the watch descriptor
    through the module seam; the pidfd remains caller-owned.
    """

    def __init__(self, workspace, nonce, turn_name, identity, pidfd, terminal_fds=()):
        """Store the rendezvous parameters and validate their shapes only.

        ``workspace`` is the provider child's working directory; ``nonce``
        must be a non-empty string; ``turn_name`` must be one plain path
        component (traversal refused here); ``identity`` must carry the four
        already-validated integer fields; ``pidfd`` must be a non-negative
        caller-owned descriptor. ``terminal_fds`` is an optional sequence of
        caller-owned ``(role, fd)`` pairs for external terminals (for example
        the conductor server descriptor); every role must be a non-empty
        string and every fd a non-negative int. The descriptors are observed
        by the same causal selector so a wait can exit when an external
        terminal fires, with a precise role-named error; the caller keeps
        full ownership and this class never closes or dups them. The default
        empty sequence preserves the original contract. Raises ``ValueError``
        on any shape violation. No filesystem access happens in the
        constructor.
        """
        terminal_pairs = []
        for pair in terminal_fds:
            role, fd = pair
            if not isinstance(role, str) or not role:
                raise ValueError('rendezvous terminal role must be a non-empty string')
            if not isinstance(fd, int) or fd < 0:
                raise ValueError('rendezvous terminal fd must be a non-negative descriptor')
            terminal_pairs.append((role, fd))
        self._terminal_fds = tuple(terminal_pairs)
        if not isinstance(nonce, str) or not nonce:
            raise ValueError('rendezvous nonce must be a non-empty string')
        _check_turn_name(turn_name)
        if not isinstance(pidfd, int) or pidfd < 0:
            raise ValueError('rendezvous pidfd must be a non-negative descriptor')
        missing = [field for field in _IDENTITY_FIELDS if field not in identity]
        if missing:
            raise ValueError('rendezvous identity misses already-validated fields: %s' % (missing,))
        for field in _IDENTITY_FIELDS:
            if not isinstance(identity[field], int):
                raise ValueError('rendezvous identity field %s must be an integer' % (field,))
        self._workspace = Path(workspace)
        self._nonce = nonce
        self._turn_name = turn_name
        self._identity = {field: identity[field] for field in _IDENTITY_FIELDS}
        self._pidfd = pidfd
        self._watch_fd = None
        self._sequence = 0
        self._instance = next(_INSTANCE_SEQUENCE)
        self._ready_seen = False
        self._released = False

    def ready(self):
        """Wait for the child's ready publication and return its payload.

        Subscribes the workspace watch before the first read, then causally
        waits for ``ready-<turn>.json``: a valid publication returns its
        parsed dict after the declared nonce and the four PID fields are
        checked against the provided identity; a child death observed on the
        pidfd raises ``RuntimeError``; a fired external terminal raises a
        role-named ``RuntimeError``; any validation refusal raises
        ``RuntimeError`` or ``OSError`` with the original file preserved.
        Marks the protocol as ready, allowing probes and one release.
        """
        payload = self._await_publication('ready-' + self._turn_name + '.json')
        self._require_identity(payload, 'ready')
        self._ready_seen = True
        return payload

    def probe(self):
        """Publish one uniquely named probe request and return its reply.

        Refuses before ``ready()``. The request name embeds the turn, the
        process-wide instance number and a per-instance sequence, so probes
        are never reused and their count is uncapped. The request is
        published atomically (exclusive 0600 temp, fsync, rename, refusing
        to replace any existing file), the reply is awaited causally on the
        watch, pidfd and any declared external terminal, and the reply's
        declared identity is checked like the ready payload before it is
        returned. Raises ``RuntimeError`` on protocol violations, child
        death or a fired external terminal; original files are preserved.
        """
        if not self._ready_seen:
            raise RuntimeError('rendezvous probe before ready')
        self._ensure_watch()
        self._sequence += 1
        stem = 'probe-%s-i%d-%06d' % (self._turn_name, self._instance, self._sequence)
        self._publish_request(stem + '.request.json', {'nonce': self._nonce, 'probe': True})
        payload = self._await_publication(stem + '.reply.json')
        self._require_identity(payload, 'probe reply')
        return payload

    def release(self):
        """Publish the release file for this turn and return its payload.

        Refuses before ``ready()`` and refuses a second release. The file
        ``release-<turn>.json`` holds exactly ``{'nonce': ..., 'release':
        True}`` and is published atomically without replacing any existing
        file. The child validates the nonce, then writes its handoff report
        and its events and exits 0 on its own; this method does not wait and
        never signals the child. Raises ``RuntimeError`` on protocol
        violations.
        """
        if not self._ready_seen:
            raise RuntimeError('rendezvous release before ready')
        if self._released:
            raise RuntimeError('rendezvous release already published')
        payload = {'nonce': self._nonce, 'release': True}
        self._publish_request('release-' + self._turn_name + '.json', payload)
        self._released = True
        return payload

    def close(self):
        """Close the watch descriptor through the module seam, idempotently.

        The pidfd is caller-owned and is never closed here. A close refusal
        propagates to the caller with its cause conserved; after ``close()``
        the rendezvous holds no descriptor of its own.
        """
        fd, self._watch_fd = self._watch_fd, None
        if fd is not None:
            close_descriptor(fd)

    def _ensure_watch(self):
        """Subscribe the workspace inotify watch once, before any read.

        Raises ``OSError`` when initialization or subscription fails; on
        combined failure the primary and secondary causes are conserved in a
        ``BaseExceptionGroup`` by :func:`watch_directory`.
        """
        if self._watch_fd is None:
            self._watch_fd = watch_directory(self._workspace)

    def _require_identity(self, payload, context):
        """Check declared nonce and PID fields against the provided identity.

        Raises ``RuntimeError`` on any mismatch; a publication never
        authorizes a PID, it can only repeat the identity the caller already
        established.
        """
        if payload.get('nonce') != self._nonce:
            raise RuntimeError('rendezvous %s declared nonce mismatch' % (context,))
        for field in _IDENTITY_FIELDS:
            if payload.get(field) != self._identity[field]:
                raise RuntimeError('rendezvous %s declared %s mismatch' % (context, field))

    def _await_publication(self, name):
        """Causally wait for one publication and return its parsed payload.

        The watch is subscribed before the first read. Each iteration first
        inspects the actual file (a complete publication found before the
        subscription still counts), then blocks in ``select`` on the watch
        descriptor, the pidfd and any declared external terminal. A readable
        pidfd means the child has terminated and raises ``RuntimeError``
        naming the missing file; a fired external terminal raises a
        ``RuntimeError`` naming its role and never closes the caller's
        descriptor; a readable watch is drained with one ABI-sized batch
        read and the loop re-inspects the file. There is no timeout, poll,
        cap or retry: the only exits are the payload, a refusal, the child's
        real death or a declared external terminal. File presence takes
        precedence over death because the file is inspected first in each
        iteration.
        """
        self._ensure_watch()
        with selectors.DefaultSelector() as selector:
            selector.register(self._watch_fd, selectors.EVENT_READ)
            selector.register(self._pidfd, selectors.EVENT_READ)
            for role, fd in self._terminal_fds:
                selector.register(fd, selectors.EVENT_READ, role)
            while True:
                payload = self._read_publication(name)
                if payload is not None:
                    return payload
                for key, _ in selector.select():
                    if key.fd == self._pidfd:
                        raise RuntimeError('rendezvous provider died before publishing ' + name)
                    if key.data is not None:
                        raise RuntimeError('rendezvous external terminal fired: ' + key.data)
                    try:
                        os.read(self._watch_fd, _READ_BATCH)
                    except BlockingIOError:
                        pass

    def _read_publication(self, name):
        """Read and parse one publication, or return ``None`` while absent.

        The name is checked as a single workspace component (traversal and
        outside-workspace refusal). ``lstat`` must show a regular file owned
        by this UID with exactly one link — symlinks and hard links are
        refused with the original preserved. The file is opened
        ``O_NOFOLLOW``, its inode identity is re-checked against the lstat
        result, and the bytes are read fully inside the descriptor's guarded
        lifetime; JSON and dict-shape refusals carry their cause. Returns
        ``None`` only when the name is absent.
        """
        _check_component(name)
        path = self._workspace / name
        try:
            info = os.lstat(path)
        except FileNotFoundError:
            return None
        if not stat.S_ISREG(info.st_mode):
            raise RuntimeError('rendezvous publication ' + name + ' is not a regular file')
        if info.st_uid != os.getuid():
            raise RuntimeError('rendezvous publication ' + name + ' ownership mismatch')
        if info.st_nlink != 1:
            raise RuntimeError('rendezvous publication ' + name + ' has been linked')
        try:
            fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
        except FileNotFoundError:
            return None
        with _FileDescriptor(fd) as owned:
            opened = os.fstat(owned)
            if (opened.st_dev, opened.st_ino) != (info.st_dev, info.st_ino):
                raise RuntimeError('rendezvous publication ' + name + ' changed while opening')
            if opened.st_uid != os.getuid() or opened.st_nlink != 1:
                raise RuntimeError('rendezvous publication ' + name + ' changed identity while opening')
            chunks = []
            while True:
                chunk = os.read(owned, _READ_BATCH)
                if not chunk:
                    break
                chunks.append(chunk)
            data = b''.join(chunks)
            try:
                payload = json.loads(data)
            except ValueError as error:
                raise RuntimeError('rendezvous publication ' + name + ' is not valid JSON') from error
        if not isinstance(payload, dict):
            raise RuntimeError('rendezvous publication ' + name + ' is not a JSON object')
        return payload

    def _publish_request(self, name, payload):
        """Publish a parent request atomically, exclusively, without deletion.

        Refuses to replace any existing file of the same name (originals are
        never deleted or overwritten). The payload is written to an exclusive
        0600 temp file, fsynced, then renamed onto the final name so readers
        observe it only complete. On failure the temp file this call created
        is removed and the cause propagates; pre-existing files are never
        touched.
        """
        _check_component(name)
        final = self._workspace / name
        temp = self._workspace / ('.' + name + '.tmp')
        if os.path.lexists(final):
            raise RuntimeError('rendezvous refusing to replace existing publication ' + name)
        fd = os.open(temp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        try:
            with _FileDescriptor(fd) as owned:
                _write_all(owned, (json.dumps(payload) + '\n').encode())
                os.fsync(owned)
            os.replace(temp, final)
        except BaseException:
            try:
                os.unlink(temp)
            except OSError:
                pass
            raise
