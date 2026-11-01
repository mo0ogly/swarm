#!/usr/bin/env python3
"""Verify graph business editing through CLI-created private Roots and Chromium.

Four independent Works isolate language/theme variants. No provider or live data.
Workspace overlay is an explicit iteration-only option; final native proof uses
embedded resources. Original commands, source bytes, diagnostics and persisted
state are retained. There is no automatic retry or global verification watchdog.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import rust_native_http_cli as native
import rust_agent_model_route as retained


def main():
    """Set up real tasks, run the public flow and check exact persisted outcomes."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--workspace-preview', action='store_true')
    parser.add_argument('--variant', choices=['fr-etat','fr-sombre','en-etat','en-sombre'], help='Target one language/theme during diagnosis; default covers all four')
    args = parser.parse_args()
    binary, out = args.binary.resolve(), args.output.resolve()
    out.mkdir(parents=True, mode=0o700, exist_ok=False)
    sources = ['tests/rust_graph_business.py', 'tests/rust_graph_business.cjs', 'web/cockpit-native-graph.js', 'web/graph.css', 'web/i18n-en.js']
    before = {name: hashlib.sha256(Path(name).read_bytes()).hexdigest() for name in sources}
    retained.publish(out / 'inputs-before.json', before)
    for i, name in enumerate(sources):
        retained.publish(out / f'source-{i}.bin', Path(name).read_bytes())
    retained.publish(out / 'binary.json', dict(path=str(binary), sha256=hashlib.sha256(binary.read_bytes()).hexdigest()))
    root = native.owned_root(binary)
    works = {}
    for label in ([args.variant] if args.variant else ['fr-etat', 'fr-sombre', 'en-etat', 'en-sombre']):
        file = out / (label + '-create.json')
        retained.publish(file, dict(schema_version=1, event_id='create-'+label, expected_revision=0, title='Business '+label, objective='Atomic analyst editing', scope='Isolated fixture', criteria=['Verified']))
        work = native.cli(binary, root, ['work', 'create', '--input', str(file)])['work']
        file = out / (label + '-task.json')
        retained.publish(file, dict(schema_version=1, event_id='task-'+label, expected_revision=1, id='a', title='PRIVATE analyst '+label, deliverable='docs/a.md', criteria=['Verified'], depends=[]))
        work = native.cli(binary, root, ['task', 'add', work['id'], '--input', str(file)])['work']
        works[label] = work['id']
    server = native.Server(binary, root, out, 'graph-business')
    primary = None
    try:
        server.capture_async(out / 'server.jsonl')
        cfg = dict(address=server.address, key=server.key, works=works, output=str(out / 'browser'), workspace_preview=args.workspace_preview, assets={('/'+Path(name).name): before[name] for name in ['web/cockpit-native-graph.js', 'web/graph.css', 'web/i18n-en.js']})
        with (out / 'browser.stdout').open('xb') as stdout, (out / 'browser.stderr').open('xb') as stderr:
            result = subprocess.run(['node', 'tests/rust_graph_business.cjs'], input=json.dumps(cfg).encode(), stdout=stdout, stderr=stderr)
        retained.publish(out / 'browser-result.json', dict(argv=['node','tests/rust_graph_business.cjs'],returncode=result.returncode, global_watchdog=None, variant=args.variant))
        after = {name: hashlib.sha256(Path(name).read_bytes()).hexdigest() for name in sources}
        retained.publish(out / 'inputs-after.json', after)
        assert before == after, 'business-verification-source-changed'
        assert result.returncode == 0, (out / 'browser.stderr').read_text()
        for label, work in works.items():
            actual = native.cli(binary, root, ['work', 'show', work])['work']
            assert actual['revision'] == 10, ('business-final-revision', label, actual['revision'])
            assert sorted(t['id'] for t in actual['tasks']) == ['a', 'clone-'+label], ('business-final-tasks', label)
            assert next(t for t in actual['tasks'] if t['id']=='a')['title'] == 'Concurrent '+label, ('compensation-original', label)
        retained.publish(out / 'result.json', dict(status='PASS', root=str(root), works=works, binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), workspace_preview=args.workspace_preview, embedded_assets_verified=not args.workspace_preview))
        print((out / 'browser.stdout').read_text(), end='')
        print('GRAPH_BUSINESS_PERSISTENCE_VERIFIED')
    except BaseException as cause:
        primary = cause
        raise
    finally:
        try:
            server.cleanup()
        except BaseException as cleanup:
            if primary is not None:
                raise BaseExceptionGroup('Graph business verification and cleanup', [primary, cleanup])
            raise


if __name__ == '__main__':
    main()
