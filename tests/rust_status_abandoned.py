#!/usr/bin/env python3
"""Verify historical status for a public abandon/reopen decision on an owned Root.

Require a qualified default CLI, create inert Work/task through public commands,
then compare complete typed SQLite/file observations around each status read.
No Agent, provider, SQL write, SDK, retry, watchdog, installation or acceptance.
Saved settings and the recorded decision must survive reads unchanged. Evidence
and the owned temporary Root remain available after success or failure.
"""
import argparse
import copy
import json
import os
from pathlib import Path
import tempfile
import traceback

import rust_mission_status_contract as shared


def decision_case(driver):
    """Observe valid configure -> abandoned/inspect -> configure recovery.

    Only public task.update changes state; status reads must not change the
    recorded Work, settings or any typed database row. Abandonment is never a
    validation or process claim. Original failures propagate with retained IO.
    """
    def cli(label, command, request=None):
        """Decode one successful public response; retain refusal channels otherwise."""
        code, raw, _ = driver.cli(label, command, request)
        shared.require(code == 0, 'abandoned fixture public operation refused')
        return json.loads(raw)

    argv = [driver.binary, 'mission', 'status', driver.work, '--root', driver.root, '--json']
    result, _ = driver.read('decision-control', argv)
    shared.require(result[0] == 0 and not result[2], 'decision valid control refused')
    control = json.loads(result[1])
    shared.require(control['tasks'][0]['state'] == 'configure'
                   and control['tasks'][0]['result']['state'] == 'not_started',
                   'decision control is not an unstarted task')
    settings = cli('decision-settings-before', ['engine-settings', 'show'])
    work = cli('decision-work-before', ['work', 'show', driver.work])['work']
    request = dict(schema_version=1, event_id='abandon', expected_revision=work['revision'],
                   id='task', status='abandoned', next='Explicitly retained decision')
    abandoned = cli('decision-abandon', ['task', 'update', driver.work], request)['work']
    result, _ = driver.read('decision-abandoned', argv)
    shared.require(result[0] == 0 and not result[2], 'abandoned historical status refused')
    status = json.loads(result[1]); task = status['tasks'][0]; presentation = task['result']
    shared.require(task['state'] == 'abandoned' and task['action'] == 'inspect'
                   and presentation['state'] == 'abandoned'
                   and presentation['validation_state'] == 'abandoned'
                   and presentation['process_state'] == 'not_started'
                   and not task['attempt'].get('agent_id')
                   and not presentation.get('attempt_id')
                   and status['validated'] == status['review'] == status['running']
                   == status['active_agents'] == 0,
                   'abandonment invented execution, Agent or validation')
    shared.require(cli('decision-work-retained', ['work', 'show', driver.work])['work'] == abandoned,
                   'abandoned read changed recorded decision')
    request = dict(schema_version=1, event_id='reopen', expected_revision=abandoned['revision'],
                   id='task', status='todo')
    reopened = cli('decision-reopen', ['task', 'update', driver.work], request)['work']
    result, _ = driver.read('decision-recovered', argv)
    shared.require(result[0] == 0 and not result[2], 'decision reopening status refused')
    recovered = json.loads(result[1])
    expected = copy.deepcopy(control['tasks'])
    expected[0]['recovery_preview']['revision'] = reopened['revision']
    expected[0]['recovery_preview']['correction'] = reopened['tasks'][0]['next']
    shared.require(recovered['tasks'] == expected and recovered['validated'] == 0,
                   'reopening lost unstarted projection or retained decision/revision')
    shared.require(cli('decision-work-recovered', ['work', 'show', driver.work])['work'] == reopened
                   and cli('decision-settings-after', ['engine-settings', 'show']) == settings,
                   'decision observation changed saved Work/settings')
    return dict(public_abandonment=True, nonvalidated=True, no_agent=True,
                reads_unchanged=True, reopened=True, global_migration_acceptance=False)


def main():
    """Bind profile/binary/source bytes and retain isolated public-CLI evidence.

    The supplied qualifier establishes the default build graph, not independent
    acceptance. Failure reporting preserves its primary exception if writing
    the report also fails; no live Root or private runtime state is used.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'output'):
        parser.add_argument('--'+name, required=True, type=Path)
    args = parser.parse_args(); output = args.output.resolve(); output.mkdir(mode=0o700)
    binary = args.binary.resolve(strict=True); profile = json.loads(args.profile.read_bytes())
    shared.require(profile.get('status') == 'VERIFIED_DEFAULT_NATIVE_PROFILE'
                   and profile.get('no_fixture_feature') is True
                   and profile.get('all_core_features_empty') is True
                   and profile.get('all_core_test_false') is True
                   and profile.get('sha256') == shared.digest(binary.read_bytes()),
                   'normal qualified profile or binary mismatch')
    paths = [Path(__file__), Path(shared.__file__), Path(shared.fixture.__file__), args.profile, binary]
    inputs = {str(path.resolve()): shared.digest(path.read_bytes()) for path in paths}
    shared.publish(output/'inputs.json', inputs)
    for label, path in [('method.py', Path(__file__)), ('shared.py', Path(shared.__file__)),
                        ('fixture.py', Path(shared.fixture.__file__)), ('profile.json', args.profile)]:
        shared.publish(output/label, path.read_bytes())
    root = Path(tempfile.mkdtemp(prefix='swarm-abandoned-'))
    shared.publish(output/'owned-root.json', dict(root=str(root), scope='owned inactive fixture',
                   binary_sha256=profile['sha256'], actor_authenticated=False))
    driver = shared.Driver(binary, None, output, root, 'owned-decision-operator', dict(os.environ))
    try:
        code, _, _ = driver.cli('init', ['init'])
        shared.require(code == 0, 'decision owned Root initialization refused')
        request = dict(schema_version=1, event_id='create', expected_revision=0, title='Decision fixture',
                       objective='Observe decisions', scope='Owned local Root', criteria=['No inferred acceptance'])
        code, raw, _ = driver.cli('create', ['work', 'create'], request)
        shared.require(code == 0, 'decision Work creation refused')
        work = json.loads(raw)['work']; driver.work = work['id']
        request = dict(schema_version=1, event_id='task', expected_revision=work['revision'],
                       id='task', title='Explicit decision', deliverable='Report', criteria=['No mutation'])
        code, _, _ = driver.cli('task', ['task', 'add', driver.work], request)
        shared.require(code == 0, 'decision task creation refused')
        checks = decision_case(driver)
        shared.require(all(shared.digest(Path(path).read_bytes()) == sha for path, sha in inputs.items()),
                       'decision declared inputs changed during verification')
        shared.publish(output/'result.json', dict(status='PASS', checks=checks, reads=driver.reads,
                       root=str(root), paid_provider=False, installed=False, global_watchdog=None))
        print('PASS public abandonment nonvalidated, unchanged reads and reopen recovery', flush=True)
    except BaseException as primary:
        try:
            shared.publish(output/'failure.json', dict(status='FAIL', root=str(root), traceback=traceback.format_exc()))
        except BaseException as cleanup:
            raise BaseExceptionGroup('Abandoned verification and failure publication', [primary, cleanup]) from None
        raise


if __name__ == '__main__':
    main()
