#!/usr/bin/env python3
"""Check analyst disclosures at all design widths on an isolated native Root.

Uses the supported CLI for setup and HTTP/browser for observations. No provider,
live datastore, mutation after setup, retries or global watchdog. Retains binary
identity, original browser diagnostics and exact Work readback; cleanup failures
propagate with the original failure rather than hiding it.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import rust_native_http_cli as native
import rust_agent_model_route as retained


def main():
    """Create one real task, verify its UI without writes, and close owned IO."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    binary, out = args.binary.resolve(), args.output.resolve()
    out.mkdir(parents=True, mode=0o700, exist_ok=False)
    retained.publish(out / 'binary.json', dict(path=str(binary), sha256=hashlib.sha256(binary.read_bytes()).hexdigest()))
    root = native.owned_root(binary)
    request = dict(schema_version=1, event_id='create', expected_revision=0, title='Analyst widths', objective='Readable controls', scope='Isolated fixture', criteria=['No page overflow'])
    retained.publish(out / 'create.json', request)
    work = native.cli(binary, root, ['work', 'create', '--input', str(out / 'create.json')])['work']
    retained.publish(out / 'task.json', dict(schema_version=1, event_id='task', expected_revision=work['revision'], id='a', title='Analyst task', deliverable='docs/a.md', criteria=['Visible'], depends=[]))
    work = native.cli(binary, root, ['task', 'add', work['id'], '--input', str(out / 'task.json')])['work']
    server = native.Server(binary, root, out, 'responsive')
    primary = None
    try:
        server.capture_async(out / 'server.jsonl')
        cfg = dict(address=server.address, key=server.key, work=work['id'], output=str(out / 'browser'))
        with (out / 'browser.stdout').open('xb') as stdout, (out / 'browser.stderr').open('xb') as stderr:
            result = subprocess.run(['node', 'tests/rust_graph_analyst_responsive.cjs'], input=json.dumps(cfg).encode(), stdout=stdout, stderr=stderr)
        assert result.returncode == 0, (out / 'browser.stderr').read_text()
        assert native.cli(binary, root, ['work', 'show', work['id']])['work'] == work, 'responsive-mutated-work'
        retained.publish(out / 'result.json', dict(status='PASS', unchanged_work=True, binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), global_watchdog=None))
        print((out / 'browser.stdout').read_text(), end='')
        print('ANALYST_RESPONSIVE_UNCHANGED_WORK_PASS')
    except BaseException as cause:
        primary = cause
        raise
    finally:
        try:
            server.cleanup()
        except BaseException as cleanup:
            if primary is not None:
                raise BaseExceptionGroup('Responsive verification and cleanup', [primary, cleanup])
            raise


if __name__ == '__main__':
    main()
