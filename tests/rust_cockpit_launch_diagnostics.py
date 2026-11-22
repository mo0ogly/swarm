#!/usr/bin/env python3
"""Verify Rust automatic launch diagnostics through authenticated HTTP and Chrome.

An isolated plan uses the local inert provider fixture. It starts with an
eligible task, a waiting dependency, and no attempted launch. Remove and
restore the provider executable bit, then publicly pause and resume.
The recipe checks unchanged Work, queue, Agents and budget for each read.
The protected HTTP fixture has no conductor. No provider process, live mission
or global watchdog is used.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
from urllib.parse import urlencode

import rust_agent_model_route as retained
import rust_mission_watch_public as mission
import rust_native_http_cli as native
import rust_planning_attach as attachment


def main():
    """Create the fixture, run named assertions, and close its owned process.

    Keep HTTP outcomes and browser stdout/stderr in a new evidence directory.
    Baseline mode records the missing endpoint and absent cockpit notice.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--http-fixture', type=Path)
    parser.add_argument('--baseline', action='store_true')
    args = parser.parse_args()
    if not args.baseline and args.http_fixture is None:
        parser.error('--http-fixture is required for positive verification')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve(strict=True)
    owned = attachment.run(binary, output / 'attachment', graph=True, return_fixture=True)
    work = owned.work
    program = owned.root / 'codex'
    before = owned.ok(['work', 'show', work])['work']
    owned.ok(['mission', 'start', work], dict(
        schema_version=1, event_id='diagnostic-start', expected_revision=before['revision'],
        reason='Isolated inert cockpit witness', slots=2,
        profile=dict(provider='fixture', role='worker', workspace='workspace-task',
                     level='standard', capture_output=True),
    ))

    web = binary if args.baseline else args.http_fixture.resolve(strict=True)
    server = native.Server(web, owned.root, output, 'launch', environment=owned.environment)
    retained.publish(output / 'binaries.json', {
        'cli_sha256': retained.digest(binary.read_bytes()), 'web_sha256': retained.digest(web.read_bytes()),
    })
    primary = None
    try:
        server.capture_async(output / 'outcomes.ndjson')
        base = '/api/v1/mission?' + urlencode(dict(work=work))
        url = base + '&action=launch-diagnostics'

        def get(path, **kwargs):
            """Read authenticated JSON through the owned server and retain the result."""
            return server.request(path, observe=False, **kwargs)

        def state():
            """Observe public outcomes the diagnostic may affect if it accidentally writes."""
            cockpit = get('/api/v1/cockpit-read?' + urlencode(dict(work=work)))
            workspaces = {directory.name: directory.stat().st_mtime_ns
                          for directory in owned.root.glob('workspace-*')}
            assert workspaces
            return dict(work=owned.ok(['work', 'show', work])['work'], queue=get(base + '&action=queue'),
                        agents=cockpit['agents'], budget=cockpit['budget'], workspaces=workspaces)

        def observe(name, expected):
            """Require reason codes and a selected Work revision, with no read side effects."""
            original = state()
            projected = get(url)
            assert projected['work'] == work and projected['revision'] == original['work']['revision'], projected
            actual = {item['task']: set(item['codes']) for item in projected['refusals']}
            assert len(actual) == len(projected['refusals']) and actual.keys() == expected.keys(), projected
            assert all(code in actual[task] for task, code in expected.items()), projected
            assert all(set(item) == {'task', 'codes'} for item in projected['refusals']), projected
            after = state()
            retained.publish(output / f'{name}.json', {'diagnostics': projected, 'before': original, 'after': after})
            assert str(owned.root) not in json.dumps(projected), 'private root in diagnostic'
            assert after == original, [key for key in original if original[key] != after[key]]
            print('PASS ' + name, flush=True)

        def browser(phase):
            """Exercise the actual served UI using an isolated authenticated browser."""
            config = dict(phase=phase, work=work, address=server.address, key=server.key,
                          output=str(output), root=str(owned.root))
            with (output / f'{phase}.stdout').open('xb') as stdout, (output / f'{phase}.stderr').open('xb') as stderr:
                result = subprocess.run(['node', 'tests/rust_cockpit_launch_diagnostics.cjs'],
                    input=json.dumps(config).encode(), stdout=stdout, stderr=stderr, check=False)
            print((output / f'{phase}.stdout').read_text(), end='', flush=True)
            assert result.returncode == 0, (phase, str(output / f'{phase}.stderr'))

        if args.baseline:
            refusal = get(url, status=501)
            assert refusal['failure']['code'] == 'web-operation-unported', refusal
            program.chmod(0o600)
            browser('baseline')
            print('PASS baseline-missing-engine-diagnostics', flush=True)
            return

        dependencies = {'grpc': 'current-independent-review-required'}
        observe('healthy-candidate-and-pending-dependency', dependencies)
        server.request(url, status=403, authorized=False, observe=False)
        assert get(base)['paused'] is False
        program.chmod(0o600)
        observe('nonexecutable-inert-provider', {**dependencies, 'task': 'agent-provider-not-executable',
                                                 'parallel': 'agent-provider-not-executable'})
        browser('blocked')
        program.chmod(0o700)
        observe('provider-restored-without-attempt-or-reservation', dependencies)
        saved_program = program.with_name('retained-inert-program')
        program.rename(saved_program)
        os.mkfifo(program, 0o700)
        try:
            observe('special-provider-file-refused-without-open', {
                **dependencies, 'task': 'agent-provider-not-executable',
                'parallel': 'agent-provider-not-executable'})
        finally:
            program.unlink()
            saved_program.rename(program)
        browser('restored')
        mission.mission_control(owned, 'pause', 'diagnostic-pause')
        observe('paused-mission-refusal', {None: 'planning-mission-paused'})
        browser('paused')
        mission.mission_control(owned, 'resume', 'diagnostic-resume')
        observe('resumed-mission-clears-pause', dependencies)
        mission.mission_control(owned, 'stop', 'diagnostic-stop')
        observe('stopped-mission-stays-paused', {None: 'planning-mission-paused'})
        mission.mission_control(owned, 'unpause', 'diagnostic-unpause-after-stop')
        observe('withdrawn-consent', {None: 'mission-consent-required'})
        final = state()
        assert final['agents'] == [] and final['queue']['turns'] == [], final
        assert final['budget']['committed_usd'] == 0, final['budget']
        assert not list(owned.root.glob('**/model-received.json'))
        print('PASS reads-created-no-agent-queue-claim-budget-charge-or-provider-invocation', flush=True)
    except BaseException as error:
        primary = error
    finally:
        native.finish(server, primary)
        assert server.process.returncode == 0 and server.records[-1]['event'] == 'web-stopped', server.records[-1]


if __name__ == '__main__':
    main()
