#!/usr/bin/env python3
"""Bind reviewed CLI inventory candidates to exact sources, without acceptance.

Capture always creates a fresh manifest; check never refreshes hashes. Missing or
stale bindings refuse. The ledger is an explicitly limited static dispatch survey,
not a complete migration denominator or executable parity proof. No subprocess,
provider, network, live Root or PAEA effect. Source aliases outside the selected
checkout refuse; path checks are observations, not OS confinement.
"""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import sys
import tempfile

ANCHORS = ('rust/crates/swarm-cli/src/lib.rs', 'rust/crates/swarm-cli/src/agent.rs', 'rust/crates/swarm-cli/src/mission.rs')
CLASSES = {'wired', 'partial', 'explicit_refusal', 'unknown'}
FIELDS = ('surface', 'actual_source', 'classification', 'missing_contract', 'proof_needed', 'next_action')


def digest(raw):
    """Hash observed bytes only; a digest is not authority or acceptance."""
    return hashlib.sha256(raw).hexdigest()


def source(root, name):
    """Read one declared CLI source contained by the selected checkout."""
    if not isinstance(name, str) or not name.startswith('rust/crates/swarm-cli/src/'):
        raise ValueError('inventory-source-not-cli')
    relative = PurePosixPath(name)
    if relative.is_absolute() or '..' in relative.parts or relative.as_posix() != name:
        raise ValueError('inventory-source-path-invalid')
    path = (root / name).resolve()
    if not path.is_relative_to(root) or not path.is_file():
        raise ValueError('inventory-source-missing-or-external')
    return path.read_bytes()


def observe(rows, root):
    """Validate rows and bind all declared sources before/after observation.

    Classifications remain reviewed candidates. A wired branch proves no behavior;
    a refusal row may describe an invalid command, not a required missing feature.
    No row or finding is deleted/reclassified by binding. Observed equality is not
    an atomic filesystem snapshot or ABA guarantee.
    """
    if not isinstance(rows, list) or not rows:
        raise ValueError('inventory-rows-missing')
    names = set(ANCHORS)
    surfaces = set()
    for row in rows:
        if not isinstance(row, dict) or any(not isinstance(row.get(k), str) or not row[k] for k in FIELDS):
            raise ValueError('inventory-row-invalid')
        if row['surface'] in surfaces or row['classification'] not in CLASSES:
            raise ValueError('inventory-duplicate-or-classification-invalid')
        surfaces.add(row['surface'])
        files = row.get('source_files')
        if not isinstance(files, list) or not all(isinstance(item, str) for item in files) or len(files) != len(set(files)):
            raise ValueError('inventory-source-list-invalid')
        if row['classification'] != 'unknown' and not files:
            raise ValueError('inventory-visible-claim-unbound')
        names.update(files)
    before = {name: digest(source(root, name)) for name in sorted(names)}
    after = {name: digest(source(root, name)) for name in sorted(names)}
    if before != after:
        raise ValueError('inventory-source-changed-during-read')
    return before


def capture(rows, root):
    """Create a static candidate manifest with explicit source and scope limits."""
    return {'schema_version': 1, 'scope': 'static-cli-candidates-excluding-PAEA',
            'notice': 'Not parity, acceptance, required-feature completeness or a migration percentage.',
            'sources': observe(rows, root), 'rows': rows,
            'counts': dict(Counter(row['classification'] for row in rows))}


def check(document, root):
    """Refuse incomplete/stale bindings without changing the original manifest."""
    if not isinstance(document, dict) or document.get('schema_version') != 1 or document.get('scope') != 'static-cli-candidates-excluding-PAEA':
        raise ValueError('inventory-envelope-invalid')
    current = observe(document.get('rows'), root)
    if document.get('sources') != current:
        raise ValueError('inventory-bindings-missing-or-stale')
    if document.get('counts') != dict(Counter(row['classification'] for row in document['rows'])):
        raise ValueError('inventory-counts-invalid')
    return len(document['rows'])


def publish_new(path, raw):
    """Publish a complete fresh report without replacing another report.

    Adjacent temporary bytes are fsynced/read back before a no-replace link. A
    directory-sync error may follow visible publication; failure never proves
    absence. Primary and cleanup causes survive together. Originals stay intact.
    """
    fd, name = tempfile.mkstemp(prefix='.' + path.name + '.', dir=path.parent)
    primary = None
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw); stream.flush(); os.fsync(stream.fileno())
        if Path(name).read_bytes() != raw:
            raise ValueError('inventory-publish-readback-failed')
        os.link(name, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        sync_error = None
        try:
            os.fsync(directory)
        except BaseException as error:
            sync_error = error
        try:
            os.close(directory)
        except BaseException as cleanup:
            if sync_error is not None:
                raise BaseExceptionGroup('inventory directory sync and close failed', [sync_error, cleanup])
            raise
        if sync_error is not None:
            raise sync_error
    except BaseException as error:
        primary = error
    try:
        Path(name).unlink()
    except BaseException as cleanup:
        if primary is not None:
            raise BaseExceptionGroup('inventory publication and cleanup failed', [primary, cleanup])
        raise
    if primary is not None:
        raise primary


def main():
    """Capture/check explicit local files; input/refusal causes remain on stderr."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, default=Path(__file__).resolve().parents[2])
    sub = parser.add_subparsers(dest='command', required=True)
    bind = sub.add_parser('capture'); bind.add_argument('--rows', type=Path, required=True); bind.add_argument('--out', type=Path, required=True)
    verify = sub.add_parser('check'); verify.add_argument('--manifest', type=Path, required=True)
    args = parser.parse_args(); root = args.source_root.resolve()
    if args.command == 'capture':
        value = capture(json.loads(args.rows.read_bytes()), root)
        publish_new(args.out, (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode())
        print('CLI_INVENTORY_CAPTURED ' + str(len(value['rows'])))
    else:
        print('CLI_INVENTORY_BOUND ' + str(check(json.loads(args.manifest.read_bytes()), root)))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError) as error:
        print(type(error).__name__ + ': ' + str(error), file=sys.stderr)
        raise SystemExit(2)
