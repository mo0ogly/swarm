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


def configured_area(repo=_REPO):
    """Resolve the fixture area from the Admin setting `runtime_temporary_directory`.

    Source order (no hard-coded path): the value saved in `repo/.swarm/
    engine-settings.json` (edited through the Admin modal), else the shipped
    installation default in `rust/crates/swarm-engine/config/engine-settings.json`
    (the same default the Admin proposes). A relative value is relative to
    `repo`. Refuses `/tmp` and anything below it (wiped at reboot) and empty or
    traversal values with ValueError; unreadable files propagate OSError.
    Read-only: no Root, database or setting is modified.
    """
    saved = repo / '.swarm' / 'engine-settings.json'
    value = None
    if saved.is_file():
        value = json.loads(saved.read_text()).get('values', {}).get('runtime_temporary_directory')
    if value is None:
        defaults = repo / 'rust/crates/swarm-engine/config/engine-settings.json'
        value = json.loads(defaults.read_text())['runtime_temporary_directory']
    if not isinstance(value, str) or not value or '\0' in value or value.rstrip('/') in ('', '/tmp') \
            or value.startswith('/tmp/') or any(c in ('', '.', '..') for c in value.strip('/').split('/')):
        raise ValueError('runtime_temporary_directory invalid for fixtures: ' + repr(value))
    path = Path(value)
    return path if path.is_absolute() else repo / path


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
