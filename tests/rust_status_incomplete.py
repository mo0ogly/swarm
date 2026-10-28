"""Ordinary incomplete-delivery and abandonment status through public paths.

Adapted from the GLM tests-agent proposal against actual Fixture/DTO contracts.
Only public task updates mutate the owned fixture; no SQL writer, real provider
or global watchdog. Missing proofs/errors are never silently ignored.
"""
import copy
import json
import subprocess
from datetime import datetime
from rust_agent_model_route import publish
from rust_status_completion import normalize_live_volumes


def verify(server, owned, check):
    """Observe a valid ordinary completion, mark it incomplete and restore it.

    Independent reads have independent observed_at timestamps; retain originals,
    validate these timestamps and live volume availability, comparing all stable content. Reads preserve
    Work/Agent/reservations. Browser only reads; public updates intentionally add
    history. Every failure propagates to the caller's retained server cleanup.
    """
    endpoint = '/api/v1/mission?work='+owned.work+'&action=status'
    baseline = owned.ok(['mission', 'status', owned.work])
    check('incomplete-baseline', baseline['validated'] == 0 and baseline['tasks'][0]['result']['state'] == 'completed_unproven')
    def update(event, blocker):
        """Use current revision and public task update; never rewrite stored rows."""
        revision = owned.ok(['work', 'show', owned.work])['work']['revision']
        return owned.ok(['task', 'update', owned.work], dict(schema_version=1,
            event_id=event, expected_revision=revision, id='task', status='blocked', blocker=blocker))
    update('incomplete-status', 'Livraison incomplète : explicit owned criterion')
    snapshot = owned.snapshot()
    for number in [1, 2]:
        cli = owned.ok(['mission', 'status', owned.work])
        http = server.request(endpoint, observe=False)
        publish(owned.output/('incomplete-'+str(number)+'-observations.json'), dict(cli=cli, http=http))
        compared = []
        for tag, doc in [('CLI', cli), ('HTTP', http)]:
            result = doc['tasks'][0]['result']
            check('incomplete-'+str(number)+'-'+tag+'-result', doc['validated'] == 0
                and result['state'] == 'delivery_incomplete' and result['process_state'] == 'completed'
                and result['validation_state'] == 'not_validated' and result['report_state'] == 'absent')
            value = copy.deepcopy(doc)
            stamps = [value[section]['observed_at'] for section in ['runtime', 'spending']]
            check('incomplete-'+str(number)+'-'+tag+'-clock', stamps[0] == stamps[1]
                and bool(datetime.fromisoformat(stamps[0].replace('Z', '+00:00')).tzinfo))
            for section in ['runtime', 'spending']: value[section]['observed_at'] = '<per-read clock>'
            normalize_live_volumes(value, 'incomplete'+'-'+str(number)+'-'+tag, check)
            compared.append(value)
        check('incomplete-'+str(number)+'-CLI-HTTP', compared[0] == compared[1])
        check('incomplete-'+str(number)+'-unchanged', owned.snapshot() == snapshot)
    config = dict(address=server.address, key=server.key, root=str(owned.root), work=owned.work, output=str(owned.output), resultState='delivery_incomplete', markers=dict(fr='explicit owned criterion',en='explicit owned criterion'))
    with (owned.output/'incomplete-browser.stdout').open('xb') as out, (owned.output/'incomplete-browser.stderr').open('xb') as err:
        result = subprocess.run(['node', 'tests/rust_status_incomplete_ui.cjs'], input=json.dumps(config).encode(), stdout=out, stderr=err)
    check('incomplete-browser', result.returncode == 0)
    check('incomplete-browser-unchanged', owned.snapshot() == snapshot)
    update('incomplete-status-restore', 'Processus terminé ; handoff, preuves et validation requis')
    restored = owned.ok(['mission', 'status', owned.work])
    check('incomplete-restored', restored['validated'] == 0 and restored['tasks'][0]['result']['state'] == 'completed_unproven')


def verify_abandoned(server, owned, check):
    """Observe an abandoned task status through public CLI and HTTP only.

    Public mutation: one task update marking task 'task' abandoned (current
    revision via 'work show'), witnessed by a completed_unproven baseline whose
    attempt identity (agent_id/attempt_id) must persist. Readonly checks: two
    independent CLI/HTTP status reads are saved as original observations, all
    stable fields are compared after timestamp and live-volume checks using the
    same normalization as verify(), and snapshots must
    be unchanged across the reads. The browser reads the same fixture in four language/theme cases; abandoned
    is never restored (abandoned->blocked is an invalid transition). Every
    failure, missing field or propagation error is raised to the caller; this
    function accepts nothing and validates only what is observed.
    """
    endpoint = '/api/v1/mission?work='+owned.work+'&action=status'
    baseline = owned.ok(['mission', 'status', owned.work])
    check('abandoned-baseline', baseline['validated'] == 0 and baseline['tasks'][0]['result']['state'] == 'completed_unproven')
    task = baseline['tasks'][0]
    check('abandoned-baseline-attempt', task['attempt']['agent_id'] == 'session')
    base_attempt_id = task['attempt']['attempt_id']
    revision = owned.ok(['work', 'show', owned.work])['work']['revision']
    owned.ok(['task', 'update', owned.work], dict(schema_version=1,
        event_id='abandoned-status', expected_revision=revision, id='task', status='abandoned'))
    snapshot = owned.snapshot()
    for number in [1, 2]:
        cli = owned.ok(['mission', 'status', owned.work])
        http = server.request(endpoint, observe=False)
        publish(owned.output/('abandoned-'+str(number)+'-observations.json'), dict(cli=cli, http=http))
        compared = []
        for tag, doc in [('CLI', cli), ('HTTP', http)]:
            task = doc['tasks'][0]
            result = task['result']
            check('abandoned-'+str(number)+'-'+tag+'-result', doc['validated'] == 0
                and task['state'] == 'abandoned' and result['state'] == 'abandoned'
                and result['process_state'] == 'completed' and result['validation_state'] == 'abandoned')
            check('abandoned-'+str(number)+'-'+tag+'-attempt', task['attempt']['agent_id'] == 'session'
                and task['attempt']['attempt_id'] == base_attempt_id)
            value = copy.deepcopy(doc)
            stamps = [value[section]['observed_at'] for section in ['runtime', 'spending']]
            check('abandoned-'+str(number)+'-'+tag+'-clock', stamps[0] == stamps[1]
                and bool(datetime.fromisoformat(stamps[0].replace('Z', '+00:00')).tzinfo))
            for section in ['runtime', 'spending']: value[section]['observed_at'] = '<per-read clock>'
            normalize_live_volumes(value, 'abandoned-'+str(number)+'-'+tag, check)
            compared.append(value)
        check('abandoned-'+str(number)+'-CLI-HTTP', compared[0] == compared[1])
        check('abandoned-'+str(number)+'-unchanged', owned.snapshot() == snapshot)
    config = dict(address=server.address, key=server.key, root=str(owned.root),
        work=owned.work, output=str(owned.output), resultState='abandoned',
        markers=dict(fr='Voir la décision', en='View decision'))
    with (owned.output/'abandoned-browser.stdout').open('xb') as out, (owned.output/'abandoned-browser.stderr').open('xb') as err:
        result = subprocess.run(['node', 'tests/rust_status_incomplete_ui.cjs'], input=json.dumps(config).encode(), stdout=out, stderr=err)
    check('abandoned-browser', result.returncode == 0)
    check('abandoned-browser-unchanged', owned.snapshot() == snapshot)
