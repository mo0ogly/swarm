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
import json
import tempfile

_REPO = Path(__file__).resolve().parent.parent


def configured_area(repo=_REPO, *, observer=None):
    """Resolve the existing saved/default fixture choice, optionally observing it.

    Return a repo-relative or absolute Path using the historical Python rules.
    Missing/null fields still select the shipped default; this does NOT implement
    adopted C-D01. No directory is created. Optional observer receives safe events
    from this same access/buffer, never a reconstruction read. It sees field types
    and validated paths, not full settings or invalid raw values. is_file=False
    is not certified absence. Import-time _AREA remains unobserved and cached.
    JSON/IO/value failures propagate with original types; observer failures are
    deferred so they cannot replace a primary resolution failure (retained in
    fixture_observation_errors). A failed observer on a successful resolution
    raises its original error, rather than returning silently incomplete proof.
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
        present = saved.is_file()
        emit('candidate', source='saved', is_file=present, absence_established=False)
        if present:
            document = json.loads(saved.read_text())
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
        if value is None:
            source = 'default'
            defaults = repo / 'rust/crates/swarm-engine/config/engine-settings.json'
            document = json.loads(defaults.read_text())
            emit('read', source='default', document_type=type(document).__name__)
            value = document['runtime_temporary_directory']
            emit('field', source='default', value_type=type(value).__name__, present=True)
        if not isinstance(value, str) or not value or '\0' in value or value.rstrip('/') in ('', '/tmp') \
                or value.startswith('/tmp/') or any(c in ('', '.', '..') for c in value.strip('/').split('/')):
            raise ValueError('runtime_temporary_directory invalid for fixtures: ' + repr(value))
        path = Path(value)
        result = path if path.is_absolute() else repo / path
        emit('resolved', path=str(result), source=source, validation='historical-python')
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
