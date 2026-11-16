#!/usr/bin/env python3
"""Single canonical implementation of the repository fixture area.

Operator interdiction (2026-10-31): project data must never live in `/tmp`.
tmpfs dies on reboot and retained evidence would point at a vanished root.
Every fixture root, scratch area and evidence scratch in this repository is
allocated under the Admin setting `runtime_temporary_directory` (installation
default `<repo>/.swarm-fixtures/`, mode 0700, gitignored) instead.

`fixture_root(prefix)` mirrors `tempfile.mkdtemp(prefix=...)`: callers pass
the exact same prefix they used before, so root-name assertions keep holding,
and get a durable `Path` back. Prefer importing this module over re-creating
the area: `tests.rust_agent_model_route.fixture_root` delegates here too.
For `tempfile.TemporaryDirectory(dir=...)` and `mkstemp(dir=...)` callers,
`fixture_area()` returns the same root directory so one cleanup/retention
owner stays.

Note: `tempfile.mkstemp(dir=destination.parent)` is adjacent-to-destination
atomic-write scaffolding, not fixture allocation; it follows the destination
and needs no change here.
"""
from pathlib import Path
import errno
import io
from contextlib import contextmanager
import locale
import json
import os
import stat
import tempfile

_REPO = Path(__file__).resolve().parent.parent


@contextmanager
def _held_fd(path, flags, *, dir_fd=None):
    """Own one descriptor; preserve a primary failure if closing also fails.

    Open errors propagate unchanged. A secondary close error is retained on
    the primary exception as fixture_close_errors, never converted to success.
    No retry or externally visible pathname/identity guarantee is introduced.
    """
    fd = os.open(path, flags, dir_fd=dir_fd)
    primary = None
    try:
        yield fd
    except BaseException as error:
        primary = error
        raise
    finally:
        try:
            os.close(fd)
        except OSError as error:
            if primary is None:
                raise
            primary.fixture_close_errors = getattr(primary, 'fixture_close_errors', ()) + (error,)


def _read_saved(repo):
    """Read saved fixture settings relative to held repo/parent descriptors.

    Return (kind, text): absent only after a missing directory entry is
    observed relative to a held parent; directory/special have no content.
    Open may follow existing symlinks. If an open reports ENOENT but lstat
    still sees an entry (for example a dangling symlink), re-raise that open
    error rather than infer absence. Other open/stat/read/decode errors also
    propagate. Parent replacement after opening cannot redirect the leaf read.
    Leaf O_NONBLOCK plus fstat avoids blocking on an opened FIFO; it is an IO
    mechanism, not an execution timeout or a new path policy. One regular-file
    descriptor supplies the whole consumed buffer. Decoding follows Python
    text defaults, including UTF-8 mode, as Path.read_text(encoding=None) did.
    The 64 KiB read chunk is
    only a transport size, never a settings size limit.

    This is not an atomic filesystem snapshot: bytes can still change during
    a read, and new entries after observed absence belong to a later access.
    The explicit repository base is opened as supplied; no OS confinement or
    exclusion of symlinks above that base is claimed. Closing errors retain
    their primary cause via _held_fd. No directories or settings are written.
    """
    with _held_fd(repo, os.O_RDONLY | os.O_DIRECTORY) as root_fd:
        try:
            parent = os.open('.swarm', os.O_RDONLY | os.O_DIRECTORY, dir_fd=root_fd)
        except FileNotFoundError:
            try:
                os.stat('.swarm', dir_fd=root_fd, follow_symlinks=False)
            except FileNotFoundError:
                return 'absent', None
            raise
        # Transfer the opened parent into the same explicit close discipline.
        # _held_fd opens by itself, so this descriptor is owned here directly.
        primary = None
        try:
            try:
                leaf = os.open('engine-settings.json', os.O_RDONLY | os.O_NONBLOCK,
                               dir_fd=parent)
            except FileNotFoundError:
                try:
                    os.stat('engine-settings.json', dir_fd=parent, follow_symlinks=False)
                except FileNotFoundError:
                    return 'absent', None
                raise
            try:
                mode = os.fstat(leaf).st_mode
                if stat.S_ISDIR(mode):
                    return 'directory', None
                if not stat.S_ISREG(mode):
                    return 'special', None
                chunks = []
                while True:
                    chunk = os.read(leaf, 65536)
                    if not chunk:
                        break
                    chunks.append(chunk)
                # Preserve Path.read_text(encoding=None): UTF-8 mode first,
                # otherwise the effective locale; do not impose a JSON codec.
                encoding = io.text_encoding(None)
                if encoding == 'locale':
                    encoding = locale.getencoding()
                return 'file', b''.join(chunks).decode(encoding)
            except BaseException as error:
                primary = error
                raise
            finally:
                try:
                    os.close(leaf)
                except OSError as error:
                    if primary is None:
                        raise
                    primary.fixture_close_errors = getattr(primary, 'fixture_close_errors', ()) + (error,)
        except BaseException as error:
            primary = error
            raise
        finally:
            try:
                os.close(parent)
            except OSError as error:
                if primary is None:
                    raise
                primary.fixture_close_errors = getattr(primary, 'fixture_close_errors', ()) + (error,)


def configured_area(repo=_REPO, *, observer=None):
    """Resolve saved/default fixture settings under adopted D1–D5.

    Read saved bytes via _read_saved once. Only observed absent entries may
    select the shipped default; dangling links and IO/parse/shape errors
    propagate. Missing/null saved field is not-configured. Validate the Rust
    reference path grammar without normalization; relative values use repo.
    No directory/settings write occurs. Observations derive from the consumed
    buffer and safe classifications, not a reconstruction read. Metadata is
    introspected, not validated as a complete product settings document.
    _AREA remains import-cached and unobserved; explicit calls resolve afresh.
    Descriptor-relative reads do not promise an atomic file-content snapshot
    or OS confinement. Observer failures are secondary to resolution errors
    (fixture_observation_errors); close failures use fixture_close_errors.
    Errors propagate to callers, which own stopping dependent work. A failed
    observer after successful resolution raises rather than omit evidence.
    """
    observation_errors = []
    def emit(kind, **details):
        """Deliver one same-access event; retain callback errors without rereading."""
        if observer is not None:
            try:
                observer(dict(schema_version=1, kind=kind, **details))
            except Exception as error:
                observation_errors.append(error)
    saved = repo / '.swarm' / 'engine-settings.json'
    source = 'saved'
    try:
        value = None
        try:
            kind, consumed_text = _read_saved(repo)
        except OSError as error:
            # Uncertain access (dangling link, denied traversal or read):
            # absence is not established; fail closed
            # with the original cause and never consult the default.
            emit('candidate', source='saved', is_file=False,
                 absence_established=False, entry_kind='uncertain',
                 error_type=type(error).__name__)
            raise
        emit('candidate', source='saved', is_file=kind == 'file',
             absence_established=kind == 'absent', entry_kind=kind)
        if kind == 'file':
            document = json.loads(consumed_text)
            # Introspection is observation only; the original .get path below
            # still raises on invalid shapes instead of changing decisions.
            metadata = {}
            if isinstance(document, dict):
                metadata = {key: document[key] for key in ('schema_version', 'revision')
                            if type(document.get(key)) is int}
            emit('read', source='saved', document_type=type(document).__name__,
                 values_type=type(document.get('values', {})).__name__ if isinstance(document, dict) else 'not_observed',
                 metadata=metadata, metadata_validation='not_performed')
            value = document.get('values', {}).get('runtime_temporary_directory')
            emit('field', source='saved', value_type=type(value).__name__,
                 present='runtime_temporary_directory' in document.get('values', {}))
        elif kind == 'directory':
            # C-D02: present but not a readable file; refuse with its own
            # cause and origin 'saved'; the shipped default is never consulted.
            raise IsADirectoryError(errno.EISDIR, 'saved engine settings is a directory')
        elif kind == 'special':
            raise OSError(errno.EINVAL, 'saved engine settings is not a regular file')
        if kind == 'absent':
            source = 'default'
            defaults = repo / 'rust/crates/swarm-engine/config/engine-settings.json'
            document = json.loads(defaults.read_text())
            emit('read', source='default', document_type=type(document).__name__)
            value = document['runtime_temporary_directory']
            emit('field', source='default', value_type=type(value).__name__, present=True)
        elif value is None:
            # C-D03: a present saved document whose field is missing or null
            # is not-configured; never a fallback, adoption, or write.
            raise ValueError('runtime_temporary_directory not configured in saved engine settings; explicit adoption required')
        if not isinstance(value, str) \
                or not value or '\0' in value or value == '/' or value.endswith('/') \
                or value == '/tmp' or value.startswith('/tmp/') \
                or any(c in ('', '.', '..') for c in (value[1:] if value.startswith('/') else value).split('/')):
            raise ValueError('runtime_temporary_directory invalid for fixtures (Rust reference grammar; raw value withheld)')
        path = Path(value)
        result = path if path.is_absolute() else repo / path
        emit('resolved', path=str(result), source=source, validation='rust-reference-grammar-python')
    except BaseException as error:
        emit('resolution_error', source=source, error_type=type(error).__name__,
             errno=getattr(error, 'errno', None))
        if observation_errors:
            error.fixture_observation_errors = tuple(observation_errors)
        raise
    if observation_errors:
        raise observation_errors[0]
    return result


_AREA = configured_area()


def fixture_area():
    """Return the repository fixture area directory, creating it 0700 if absent.

    The area is created on first use, never by import (no artifact allocation
    on import). Errors propagate: a read-only checkout cannot host fixtures.
    """
    _AREA.mkdir(mode=0o700, parents=True, exist_ok=True)
    return _AREA


def fixture_root(prefix):
    """Create a private durable fixture root under the repository fixture area.

    Purpose: replace `tempfile.mkdtemp(prefix)` calls that would place project
    data in `/tmp`. Inputs: `prefix` (same convention as mkdtemp, usually
    `swarm-...-`). Output: a `Path` to a new 0700-by-inheritance directory,
    named `<prefix><random>` (8 random chars, like mkdtemp). Errors: `OSError`
    propagates if the area cannot be created. Side effects: creates the area
    (0700) and the root. Limits: the caller owns retention and cleanup — this
    area holds fixture evidence, not a disposable cache; regeneration follows
    the documented recipe path (`tools/verification/README.md`). Path length
    is short enough for AF_UNIX sockets inside the root (kernel limit 108).
    """
    return Path(tempfile.mkdtemp(prefix=prefix, dir=fixture_area()))
