#!/usr/bin/env python3
"""Verify submitted status outside the recent-Agent view on an owned Root.

Require a qualified normal binary. Reuse the permanent completed-worker fixture,
then save/reopen a zero display window through public settings operations. Every
status read is bracketed by full typed/database hashes. Restore the original
window even on failure. No SDK, live Root, paid model, acceptance, SQL writes,
retry, global watchdog or installed-server restart. A local fixture process is
not an authenticated provider and its declared cost is not an invoice.
"""
import argparse
import copy
import json
from pathlib import Path
import traceback

import rust_agent_model_route as fixture
import rust_mission_status_contract as shared


def shown(driver, label):
    """Read actual settings JSON; require success and retain original channels."""
    code, raw, _ = driver.cli(label, ['engine-settings', 'show'])
    shared.require(code == 0, 'window settings show refused')
    return json.loads(raw)


def save(driver, label, document, values):
    """Public preview/apply/reopen; assert pure preview and persisted exact values.

    Errors propagate without claiming no publication. Caller owns restoration
    against freshly read state, including an uncertain apply/ack outcome.
    """
    change = dict(schema_version=document['schema_version'], event_id=label,
                  expected_revision=document['revision'], values=values,
                  reason='Owned status display-window verification')
    code, raw, _ = driver.cli(label+'-preview', ['engine-settings', 'preview'], change)
    shared.require(code == 0, 'window settings preview refused')
    change['preview_token'] = json.loads(raw)['preview_token']
    shared.require(shown(driver, label+'-preview-reopen') == document,
                   'window preview changed saved settings')
    code, _, _ = driver.cli(label+'-apply', ['engine-settings', 'apply'], change)
    shared.require(code == 0, 'window settings apply refused')
    actual = shown(driver, label+'-saved')
    shared.require(actual['values'] == values and actual['history'][:-1] == document['history'],
                   'window saved values or retained history differ')
    return actual


def window_case(driver):
    """Assert hidden Agent identity, retained task attempt and full accounting.

    A valid completed/submitted control precedes the zero-view case. Original
    settings are restored via fresh public preview/apply, even after an uncertain
    failed operation. Primary, restoration and recovery errors remain distinct.
    """
    argv = [driver.binary, 'mission', 'status', driver.work, '--root', driver.root, '--json']
    result, _ = driver.read('window-control', argv)
    shared.require(result[0] == 0 and not result[2], 'window valid control refused')
    before = json.loads(result[1])
    shared.require(before['tasks'][0]['result']['process_state'] == 'completed',
                   'window control lacks actual completed Agent')
    original = shown(driver, 'window-original')
    values = copy.deepcopy(original['values']); values['work_view_status_recent_agents'] = '0'
    errors = []
    try:
        saved = save(driver, 'window-hide-agents', original, values)
        result, _ = driver.read('window-agent-hidden', argv)
        shared.require(result[0] == 0 and not result[2], 'submitted outside window refused')
        shared.require(shown(driver, 'window-hidden-settings-stable') == saved, 'status read changed saved window settings')
        hidden = json.loads(result[1]); task = hidden['tasks'][0]; presentation = task['result']
        shared.require(task['state'] == 'review' and task['action'] == 'report'
                       and presentation['state'] == 'result_to_review'
                       and presentation['process_state'] == 'unknown'
                       and presentation['validation_state'] == 'missing_gate'
                       and hidden['validated'] == 0,
                       'hidden submitted result invented execution or acceptance')
        shared.require(not task['attempt'].get('agent_id')
                       and task['attempt'].get('attempt_id') == before['tasks'][0]['attempt']['attempt_id']
                       and not presentation.get('attempt_id')
                       and presentation['report_id'] == before['tasks'][0]['result']['report_id'],
                       'hidden result lost retained task attempt/report or invented Agent identity')
        for key in ['calls', 'attempts', 'rows', 'worker_retries', 'recorded_control_executions']:
            shared.require(hidden['spending'][key] == before['spending'][key],
                           'display window changed complete accounting')
    except BaseException as error:
        errors.append(error)
    finally:
        try:
            current = shown(driver, 'window-before-restore')
            if current['values'] != original['values']:
                save(driver, 'window-restore', current, original['values'])
            recovered, _ = driver.read('window-recovered', argv)
            shared.require(recovered[0] == 0 and not recovered[2], 'window recovery refused')
            value = json.loads(recovered[1])
            shared.require(value['tasks'] == before['tasks'], 'window recovery lost original task projection')
            shared.require(value['spending']['calls'] == before['spending']['calls'],
                           'window recovery changed retained costs')
        except BaseException as error:
            errors.append(error)
    if errors:
        raise BaseExceptionGroup('Submitted window verification and restoration', errors)
    return dict(zero_view_submitted_review=True, process_unknown=True,
                no_projected_agent_identity=True, retained_task_attempt=True, full_accounting_preserved=True,
                settings_restored=True, whole_migration_parity=False)


def normalize_provider_fixture(driver):
    """Adopt canonical provider representation through public preview/apply.

    The shared model fixture initially omits optional env/limit fields. Their
    existing writer normalizes them; the completed-worker preservation test
    requires that canonical baseline. Preserve original field values, verify a
    pure preview and actual publication without launching a provider.
    """
    path = driver.root/'.swarm/providers.json'; original = path.read_bytes()
    configuration = json.loads(original)
    code, raw, _ = driver.cli('window-provider-inspect', ['providers','config','inspect'])
    shared.require(code == 0, 'window provider baseline inspection refused')
    request = dict(schema_version=1, expected_digest=json.loads(raw)['digest'], configuration=configuration)
    code, raw, _ = driver.cli('window-provider-preview', ['providers','config','preview'], request)
    shared.require(code == 0 and path.read_bytes() == original, 'window provider preview mutated baseline')
    preview = json.loads(raw)
    shared.require(preview['preview'] and not preview['applied'] and not preview['provider_execution_started'],
                   'window provider preview executed a process')
    request['preview_token'] = preview['preview_token']
    code, raw, _ = driver.cli('window-provider-adopt', ['providers','config','apply'], request)
    shared.require(code == 0, 'window canonical provider adoption refused')
    applied = json.loads(raw); saved = json.loads(path.read_bytes())
    shared.require(applied['applied'] and not applied['provider_execution_started']
                   and saved == applied['configuration'], 'window canonical provider not saved')
    for name, value in configuration['providers'].items():
        shared.require(all(saved['providers'][name][key] == item for key,item in value.items()),
                       'window canonical provider changed original supplied fields')


def main():
    """Bind supplied profile/binary/sources; retain fixture and original failures."""
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['binary', 'profile', 'output']:
        parser.add_argument('--'+key, type=Path, required=True)
    args = parser.parse_args(); output = args.output.resolve(); output.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve(); profile = json.loads(args.profile.read_bytes())
    shared.require(profile['status'] == 'VERIFIED_DEFAULT_NATIVE_PROFILE'
                   and profile['no_fixture_feature'] and profile['all_core_features_empty']
                   and profile['sha256'] == shared.digest(binary.read_bytes()),
                   'normal qualified profile or binary mismatch')
    paths = [Path(__file__), Path(shared.__file__), Path(fixture.__file__), args.profile, binary]
    hashes = {str(path.resolve()): shared.digest(path.read_bytes()) for path in paths}
    shared.publish(output/'inputs.json', hashes)
    for label, path in [('method.py', Path(__file__)), ('shared.py', Path(shared.__file__)),
                        ('fixture.py', Path(fixture.__file__)), ('profile.json', args.profile)]:
        shared.publish(output/label, path.read_bytes())
    owned = None
    try:
        owned = fixture.Fixture(binary, output, 'submitted-window')
        driver = shared.Driver(binary, None, output, owned.root, 'owned-window-operator', owned.environment)
        driver.work = owned.work
        code, _, _ = driver.invoke('window-git-init', ['git', 'init', '-q', owned.root])
        shared.require(code == 0, 'window owned Git initialization failed')
        normalize_provider_fixture(driver)
        completed = shared.completed_case(driver)
        checked = window_case(driver)
        shared.require(all(shared.digest(Path(path).read_bytes()) == sha for path, sha in hashes.items()),
                       'window declared source/profile/binary changed')
        shared.publish(output/'result.json', dict(status='PASS', checks=checked, completed=completed,
                        reads=driver.reads, root=str(owned.root), installed=False, paid_provider=False,
                        global_watchdog=None))
        print('PASS actual submitted result outside Agent window, full costs and restoration', flush=True)
    except BaseException as primary:
        try:
            shared.publish(output/'failure.json', dict(status='FAIL', traceback=traceback.format_exc(),
                           root=None if owned is None else str(owned.root)))
        except BaseException as cleanup:
            raise BaseExceptionGroup('Submitted window verification and failure publication', [primary, cleanup]) from None
        raise


if __name__ == '__main__':
    main()
