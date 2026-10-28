#!/usr/bin/env python3
"""Retain current ignored Go application sources without executing Go.

This captures all root/cmd/internal Go files and go.mod/go.sum, not an effective
compiler graph, embedded assets, final approved reference or a parity result.
Reads Git tree metadata for comparison, preserves current bytes in one archive,
and refuses source drift. A fresh output retains partial failure evidence.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile
import build_behavior_matrix as publication
from build_behavior_matrix import publish


def source_names():
    """Inventory current root/cmd/internal Go names, never compiler eligibility.

    Missing source roots refuse; no cached names or empty-success fallback. This
    is an observation, not a filesystem lock or a guarantee against later edits.
    """
    for name in ['cmd', 'internal']:
        if not Path(name).is_dir():
            raise ValueError('source directory missing: ' + name)
    return sorted({str(p) for d in ['cmd', 'internal'] for p in Path(d).rglob('*.go')}
                  | {str(p) for p in Path('.').glob('*.go')})


def collect(reference, output):
    """Archive exact current source bytes and compare with a historical Git tree.

    Requires repository cwd and existing canonical source directories. Reads only
    source/Git, no compiler/provider/runtime. Errors or drift propagate without a
    successful result; atomic files may already exist after a later failure.
    No deletion, stage, commit, source rewrite or reference acceptance occurs.
    """
    output.mkdir(parents=True, exist_ok=False)
    publish(output / 'method.py', Path(__file__).read_bytes())
    publisher_path = Path(publication.__file__)
    publisher = publisher_path.read_bytes()
    publish(output / 'publisher.py', publisher)
    commit = subprocess.check_output(['git', 'rev-parse', reference + '^{commit}'], text=True).strip()
    tree = subprocess.check_output(['git', 'ls-tree', '-r', '-z', commit])
    old = {}
    for entry in tree.split(b'\0'):
        if not entry:
            continue
        metadata, name = entry.split(b'\t', 1)
        name = name.decode()
        if name.endswith('.go') and (name.startswith(('cmd/', 'internal/')) or '/' not in name):
            old[name] = metadata.decode().split()[2]
    names = source_names()
    if not old or not names:
        raise ValueError('empty source reference')
    data = {}
    for name in names + ['go.mod', 'go.sum']:
        p = Path(name)
        if p.is_symlink() or not p.is_file():
            raise ValueError('source is not a regular non-symlink path: ' + name)
        data[name] = p.read_bytes()
    rows = {}
    for name in names:
        raw = data[name]
        git_hash = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
        rows[name] = dict(sha256=hashlib.sha256(raw).hexdigest(), bytes=len(raw),
                          historical_git_blob=old.get(name), current_git_blob=git_hash,
                          state='ADDED' if name not in old else 'UNCHANGED' if old[name] == git_hash else 'MODIFIED',
                          test=name.endswith('_test.go'))
    stream = io.BytesIO()
    with tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, raw in sorted(data.items()):
            info = tarfile.TarInfo(name)
            info.size = len(raw)
            info.mode = 0o600
            archive.addfile(info, io.BytesIO(raw))
    archived = stream.getvalue()
    with tarfile.open(fileobj=io.BytesIO(archived), mode='r:gz') as archive:
        for name, raw in data.items():
            if archive.extractfile(name).read() != raw:
                raise ValueError('archive mismatch: ' + name)
    for name, raw in data.items():
        if Path(name).read_bytes() != raw:
            raise ValueError('source drift: ' + name)
    if source_names() != names:
        raise ValueError('source inventory changed during capture')
    if publisher_path.read_bytes() != publisher:
        raise ValueError('publisher changed during capture')
    publish(output / 'application-sources.tar.gz', archived)
    counts = dict(historical_production=sum(not n.endswith('_test.go') for n in old),
                  historical_tests=sum(n.endswith('_test.go') for n in old),
                  current_production=sum(not r['test'] for r in rows.values()),
                  current_tests=sum(r['test'] for r in rows.values()),
                  modified_existing=sum(r['state'] == 'MODIFIED' for r in rows.values()),
                  added=sum(r['state'] == 'ADDED' for r in rows.values()),
                  missing=sum(n not in rows for n in old))
    result = dict(status='SOURCE_BYTES_RETAINED_NOT_APPROVED_BASELINE', reference=commit,
                  scope=__doc__, counts=counts, files=rows,
                  additional_inputs={n: hashlib.sha256(data[n]).hexdigest() for n in ['go.mod', 'go.sum']},
                  archive_sha256=hashlib.sha256(archived).hexdigest(), archive_members=len(data),
                  archive_bytes=len(archived), compiler_graph=False, embedded_assets=False,
                  publisher_sha256=hashlib.sha256(publisher).hexdigest(),
                  source_names_rechecked=True,
                  reference_approved=False, go_executed=False, installed=False)
    publish(output / 'result.json', json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(counts))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', required=True)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    collect(args.reference, args.output)
