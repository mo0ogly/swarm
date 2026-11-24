#!/usr/bin/env python3
"""Verify recomputation, CSRF, replay, stale approval and browser on an owned Root."""
import argparse
import json
import subprocess
from pathlib import Path
from urllib.parse import urlencode
import rust_native_http_cli as native
import rust_agent_model_route as fixture


def verify(binary, out):
    """Change the plan publicly. Observe the same Work before and after approval."""
    out.mkdir(exist_ok=False)
    root = native.owned_root(binary)

    def mutate(args, **request):
        """Call the supported CLI and preserve exact synthetic inputs."""
        name = out / (request['event_id'] + '.json')
        fixture.publish(name, dict(schema_version=1, **request))
        return native.cli(binary, root, [*args, '--input', str(name)])['work']

    work = mutate(['work', 'create'], event_id='workflow-create', expected_revision=0,
                  title='Workflow architecture fixture', objective='Audit a plan',
                  scope='Owned fixture', criteria=['Evidence'])
    for name, dependencies in [('a', []), ('b', ['a'])]:
        work = mutate(['task', 'add', work['id']], event_id=f'add-{name}',
                      expected_revision=work['revision'], id=name, title=name,
                      deliverable=f'{name}.md', criteria=['Evidence'], depends=dependencies)

    server = native.Server(binary, root, out, 'architecture')
    failure = None
    try:
        server.capture_async(out / 'frames.jsonl')
        path = '/api/v1/planning/workflow-architecture?' + urlencode({'work': work['id']})
        first = server.request(path, observe=False)
        assert first['assessment']['recommendation'] == 'sequential'
        assert first['assessment']['dependencies'] == 1 and not first['approved']
        assert first['revision'] == str(work['revision'])
        server.request(path, authorized=False, status=403, observe=False)
        approved_req = {key: first[key] for key in ['revision', 'proposal']}
        server.request(path, approved_req, status=403, observe=False)
        assert server.request(path, observe=False) == first
        print('PASS readonly assessment and CSRF refusal', flush=True)

        cfg = dict(address=server.address, key=server.key, work=work['id'])
        subprocess.run(['node', 'tests/rust_workflow_architecture_ui.cjs'],
                       input=json.dumps(cfg), text=True, check=True)
        csrf = server.request('/api/v1/native-session', observe=False)['csrf_token']
        approved = server.request(path, observe=False)
        assert approved['approved'] and approved['approval']['revision'] == first['revision']
        assert server.request(path, approved_req, csrf=csrf, observe=False) == approved
        assert native.cli(binary, root, ['work', 'show', work['id']])['work'] == work
        print('PASS bound approval and exact replay leave Work unchanged', flush=True)

        changed = mutate(['task', 'add', work['id']], event_id='add-independent',
                         expected_revision=work['revision'], id='c', title='Independent',
                         deliverable='c.md', criteria=['Evidence'], depends=[])
        current = server.request(path, observe=False)
        assert current['revision'] == str(changed['revision'])
        assert current['assessment']['recommendation'] == 'insufficient'
        assert not current['approved'] and current['approval'] == approved['approval']
        server.request(path, approved_req, csrf=csrf, status=409, observe=False)
        request = {key: current[key] for key in approved_req}
        server.request(path, request, csrf=csrf, status=409, observe=False)
        assert server.request(path, observe=False) == current
        assert native.cli(binary, root, ['work', 'show', work['id']])['work'] == changed
        print('PASS changed graph recomputes and stale or indeterminate approval refuses', flush=True)
        fixture.publish(out / 'result.json', dict(status='PASS',
            binary_sha256=fixture.digest(binary.read_bytes()), first=first,
            approved=approved, current=current))
    except BaseException as error:
        failure = error
    finally:
        native.finish(server, failure)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    verify(args.binary.resolve(), args.output.resolve())
