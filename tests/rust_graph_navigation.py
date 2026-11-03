#!/usr/bin/env python3
"""Run native graph browser checks on one private CLI-created Root.

Requires a native executable and fresh output directory. Creates three tasks via
the supported CLI, starts only its owned HTTP process, passes the cookie on stdin
and retains original browser output. No provider, live Root, retry or watchdog.
Failures and cleanup errors propagate; roots and original evidence are preserved.
"""
import argparse
import hashlib
import json
import subprocess
import time
from pathlib import Path
import rust_native_http_cli as native
import rust_agent_model_route as retained


def main():
    """Create the valid control, execute the actual browser, then close owned IO."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--workspace-preview', action='store_true', help='Explicit workspace JS/CSS overlay for iteration; never embedded-build evidence')
    parser.add_argument('--analyst-only', action='store_true', help='Targeted analyst controls, plus empty graph and browser diagnostics; no editing proof')
    parser.add_argument('--editing', action='store_true', help='Additionally exercise task creation and dependency save/refusal in the actual engine')
    parser.add_argument('--skip-search-check', action='store_true', help='Diagnostic before-image only: exercise busy feedback before search exists')
    args = parser.parse_args()
    if args.analyst_only and args.editing:
        parser.error('--analyst-only does not exercise business editing')
    binary, out = args.binary.resolve(), args.output.resolve()
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    retained.publish(out / 'binary.json',dict(path=str(binary),sha256=hashlib.sha256(binary.read_bytes()).hexdigest()))
    source_names=['tests/rust_graph_navigation.py','tests/rust_graph_navigation_ui.cjs','tests/rust_graph_analyst_ui.cjs','tests/rust_graph_direct_ui.cjs','web/cockpit-native-graph.js','web/cockpit-native.js','web/graph.css','web/i18n-en.js']
    source_hashes={name:hashlib.sha256(Path(name).read_bytes()).hexdigest() for name in source_names}
    retained.publish(out / 'inputs-before.json',source_hashes)
    for index,name in enumerate(source_names):
        retained.publish(out / ('source-'+str(index)+'.bin'),Path(name).read_bytes())
    root = native.owned_root(binary)
    def mutate(command, request):
        """Use fresh JSON inputs and the public CLI; never edit datastore files."""
        file = out / (request['event_id'] + '.json')
        retained.publish(file, request)
        return native.cli(binary, root, [*command, '--input', str(file)])['work']
    work = mutate(['work', 'create'], dict(schema_version=1, event_id='create', expected_revision=0,
        title='Graph navigation', objective='Restore the graph workbench', scope='Private fixture', criteria=['Verified interactions']))
    for index, task in enumerate(['a', 'b', 'c']):
        work = mutate(['task', 'add', work['id']], dict(schema_version=1, event_id='add-'+task,
            expected_revision=work['revision'], id=task, title={'a':'Task a · Alpha','b':'Review','c':'Tâche Café · Alpha'}[task],
            deliverable='docs/'+task+'.md', criteria=['Verified'], depends=['a'] if index else []))
    empty = mutate(['work', 'create'], dict(schema_version=1, event_id='empty-create', expected_revision=0,
        title='Empty graph', objective='Create the first task', scope='Private fixture', criteria=['First task is real']))
    server = native.Server(binary, root, out, 'graph')
    primary = None
    try:
        server.capture_async(out / 'server.jsonl')
        assets = {route: hashlib.sha256(Path(source).read_bytes()).hexdigest() for route, source in {
            '/cockpit-native-graph.js': 'web/cockpit-native-graph.js', '/cockpit-native.js': 'web/cockpit-native.js', '/i18n-en.js':'web/i18n-en.js', '/graph.css': 'web/graph.css'}.items()}
        cfg = dict(address=server.address, key=server.key, output=str(out / 'browser'), work=work['id'], empty_work=empty['id'], assets=assets, workspace_preview=args.workspace_preview, editing=args.editing, skip_search_check=args.skip_search_check, analyst_only=args.analyst_only)
        # Stream exact originals to fresh files so progress is inspectable during
        # an unbounded browser action; EOF/exit and cleanup remain authoritative.
        started=time.time()
        with (out / 'browser.stdout').open('xb') as stdout, (out / 'browser.stderr').open('xb') as stderr:
            result = subprocess.run(['node', 'tests/rust_graph_navigation_ui.cjs'], input=json.dumps(cfg).encode(), stdout=stdout, stderr=stderr)
        retained.publish(out / 'browser-result.json',dict(argv=['node','tests/rust_graph_navigation_ui.cjs'],returncode=result.returncode,started=started,ended=time.time(),global_watchdog=None))
        final_hashes={name:hashlib.sha256(Path(name).read_bytes()).hexdigest() for name in source_names}
        retained.publish(out / 'inputs-after.json',final_hashes)
        assert final_hashes == source_hashes, 'verification-inputs-changed-during-browser'
        assert result.returncode == 0, 'browser-exit-'+str(result.returncode)+': '+(out / 'browser.stderr').read_text()
        final = native.cli(binary, root, ['work', 'show', work['id']])['work']
        if args.editing:
            assert final['revision'] == work['revision'] + 13 and len(final['tasks']) == 8, 'editing-persistence-revisions'
            assert final['tasks'][:3] == work['tasks'], 'editing-changed-original-tasks'
            assert all(not task.get('depends') for task in final['tasks'][3:]), 'dependency-clear-not-persisted'
        else:
            assert final == work, 'presentation-mutated-work'
        final_empty = native.cli(binary, root, ['work', 'show', empty['id']])['work']
        if args.editing:
            assert final_empty['revision'] == 2 and len(final_empty['tasks']) == 1 and final_empty['tasks'][0]['id'] == 'first', 'first-task-creation-not-persisted'
        else:
            assert final_empty == empty, 'empty-presentation-mutated-work'
        retained.publish(out / 'result.json', dict(status='PASS', binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), root=str(root), unchanged_work=not args.editing, editing=args.editing, assets=assets, workspace_preview=args.workspace_preview, embedded_assets_verified=not args.workspace_preview))
        print((out / 'browser.stdout').read_text(), end='')
        print('GRAPH_NAVIGATION_VERIFIED')
    except BaseException as cause:
        primary = cause
        raise
    finally:
        try:
            server.cleanup()
        except BaseException as cleanup:
            if primary is not None:
                raise BaseExceptionGroup('Graph verification and cleanup', [primary, cleanup])
            raise


if __name__ == '__main__':
    main()
