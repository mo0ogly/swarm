#!/usr/bin/env python3
"""Verify historical accepted status from a real public completed worker and gate.

Only an explicit fixture operator accepts, through public commands. Reads must
classify current/stale/restored proofs, profiled manual/pause/dependencies and
taskless hierarchical configuration/start/pause, provider waits, operator claims,
lease renewal, empty decisions, saved activation cap and first planned todos/dependencies
including a publicly delegated child scope, without read mutation.
Initial root and delegated-child automatic readiness covers recorded budget and pause
recovery with no conductor; task ownership and planning consumption stay unchanged.
A real delegated local worker is observed running then awaiting review; public
queued cancellation and actual exit7 failure stay unvalidated. Includes public
output refusal/recovery. No paid provider, private SQL seeding, SDK, watchdog,
restart or global acceptance.
"""
import argparse
import copy
import json
from pathlib import Path
import traceback
from datetime import datetime, timezone
import time
import sys
import os
import signal
import subprocess
import rust_planning_worker_public_flow as planned_flow
import rust_manual_mission as manual_mission
import rust_agent_model_route as fixture
import rust_mission_status_contract as shared
import rust_status_submitted_window as windows


def accepted_case(driver):
    """Record an explicit fixture gate/acceptance and observe current/stale proofs.

    Reuses an actual completed/submitted local worker and its saved profile.
    A new unstarted dependent stays manual, pauses and resumes without a launch. The public gate is an
    operator assertion on this owned fixture, never independent review. Status
    reads may not change retained Work, Agent, reservations or settings. Restore
    the original report even after failure; no SQL mutation or counter reset.
    """
    def cli(label, command, request=None):
        """Decode one successful public result, retaining refusal channels on error."""
        code, raw, _ = driver.cli(label,command,request)
        shared.require(code == 0,'accepted fixture public operation refused')
        return json.loads(raw)

    work = cli('accepted-work',['work','show',driver.work])['work']; task = work['tasks'][0]
    report = driver.root/'handoff.md'; original = report.read_bytes()
    checks = [dict(id=key,gate=phase,domain='quality',mandatory=True,severity='minor',penalty=1,max_penalty=100)
              for key,phase in task['plan_checks'].items()]
    checks.append(dict(id='fixture-delivery',gate='delivery',domain='quality',mandatory=True,
                       severity='minor',penalty=1,max_penalty=100))
    document = dict(method_version='3',scope_id=task['id'],artifacts={'handoff.md':shared.digest(original)},
                    domains={'quality':1},checks=checks,
                    results=[dict(id=c['id'],status='PASS',count=0,evidence=['handoff.md']) for c in checks])
    work = cli('accepted-gate',['gate',driver.work],
               dict(schema_version=1,event_id='accepted-gate',expected_revision=work['revision'],
                    task_id=task['id'],phase='delivery',name='Explicit owned operator gate',document=document))['work']
    work = cli('accepted-decision',['task','update',driver.work],
               dict(schema_version=1,event_id='accepted-decision',expected_revision=work['revision'],
                    id=task['id'],status='accepted'))['work']
    shared.require(work['tasks'][0]['status'] == 'accepted','explicit fixture acceptance not recorded')
    work = cli('accepted-dependent',['task','add',driver.work],
               dict(schema_version=1,event_id='accepted-dependent',expected_revision=work['revision'],
                    id='dependent',title='Unstarted dependent',deliverable='Later report',
                    criteria=['Wait only when prerequisite is not current'],depends=[task['id']]))['work']
    # Keep the real inherited Work profile; this todo has no attempt or Agent.
    argv = [driver.binary,'mission','status',driver.work,'--root',driver.root,'--json']

    def observe(label, stale=False):
        """Assert real freshness drives validation and dependency explanations, without authority."""
        result, _ = driver.read(label,argv)
        shared.require(result[0] == 0 and not result[2],'accepted historical status refused')
        status = json.loads(result[1]); parent, dependent = status['tasks']
        waits = dependent['waiting_on']
        shared.require(status['validated'] == (0 if stale else 1)
                       and status['review'] == status['running'] == status['active_agents'] == 0
                       and parent['state'] == ('intervention' if stale else 'validated')
                       and parent['result']['state'] == ('validation_stale' if stale else 'validated')
                       and parent['result']['validation_state'] == ('stale' if stale else 'fresh')
                       and parent['action'] == ('reopen' if stale else 'report')
                       and dependent['state'] == ('waiting' if stale else 'manual')
                       and dependent['result']['state'] == 'not_started'
                       and dependent['action'] == 'inspect'
                       and type(waits) is list and bool(waits) is stale
                       and (not stale or (len(waits) == 1 and waits[0]['task'] == parent['id'] and waits[0]['state'] == 'stale')),
                       'accepted freshness or dependent waits became invented validation')
        shared.require(cli(label+'-work',['work','show',driver.work])['work'] == work,
                       'accepted observation rewrote recorded decision')
        return status

    current = observe('accepted-current')
    work = cli('profile-pause',['mission','pause',driver.work],
               dict(schema_version=1,event_id='profile-pause',expected_revision=work['revision'],reason='Owned profile pause observation'))
    result, _ = driver.read('profile-paused',argv)
    shared.require(result[0] == 0 and not result[2],'profile pause status refused')
    paused = json.loads(result[1]); dependent = paused['tasks'][1]
    shared.require(paused['paused'] is True and dependent['state'] == 'waiting'
                   and paused['active_agents'] == paused['running'] == 0,
                   'profile pause invented a departure or lost waiting')
    work = cli('profile-unpause',['mission','unpause',driver.work],
               dict(schema_version=1,event_id='profile-unpause',expected_revision=work['revision'],reason='Owned profile resume observation'))
    observe('profile-resumed')

    errors = []
    try:
        fixture.publish(report,original+b'Owned proof change after acceptance\n')
        observe('accepted-stale',True)
    except BaseException as error:
        errors.append(error)
    finally:
        try: fixture.publish(report,original)
        except BaseException as error: errors.append(error)
    try: observe('accepted-restored')
    except BaseException as error: errors.append(error)
    if errors: raise BaseExceptionGroup('Accepted stale proof and original restoration',errors)
    original_settings = windows.shown(driver,'accepted-window-original')
    values = copy.deepcopy(original_settings['values']); values['work_view_status_recent_agents'] = '0'
    errors = []
    try:
        windows.save(driver,'accepted-window-hide',original_settings,values)
        hidden = observe('accepted-outside-window')
        shared.require(not hidden['tasks'][0]['attempt'].get('agent_id')
                       and all(hidden['spending'][key] == current['spending'][key] for key in ['calls','attempts','rows','worker_retries','recorded_control_executions']),
                       'hidden accepted Agent invented identity or changed full accounting')
    except BaseException as error:
        errors.append(error)
    finally:
        try:
            saved = windows.shown(driver,'accepted-window-before-restore')
            if saved['values'] != original_settings['values']:
                windows.save(driver,'accepted-window-restore',saved,original_settings['values'])
        except BaseException as error: errors.append(error)
    try: observe('accepted-window-restored')
    except BaseException as error: errors.append(error)
    if errors: raise BaseExceptionGroup('Accepted display window and setting restoration',errors)
    return dict(operator_fixture_accepted=True,independent_review=False,current_then_stale_then_restored=True,
                dependencies_current_not_hardcoded=True,profiled_todo_manual=True,automatic_execution_not_qualified=True,no_status_mutation=True,hidden_agent_keeps_full_costs=True,
                global_migration_acceptance=False)



def automatic_case(driver, delegated=False):
    """Observe prepared initial planned todos automatically through public operations.

    A valid configure witness precedes start. No conductor/provider runs: an
    otherwise-ready task must show supervise/waiting, not execution. Saved budget
    insufficiency, dependency waits and pause/recovery are read without mutation;
    Driver.read compares complete typed/physical snapshots on every observation.
    Only owned fixture Work/budget/controls are changed, consumption is preserved.
    Delegated mode requires real child-owned tasks; labels, events and workspace
    are distinct from the root witness. Every observation also reopens public Work
    to assert unchanged task ownership, attempts, reviews and planning counters.
    Public refusal channels remain retained; no retry or private SQL seeding.
    """
    prefix = 'delegated-automatic' if delegated else 'automatic'
    def label_for(label):
        """Namespace retained evidence and event IDs without rewriting request bodies."""
        return prefix + label.removeprefix('automatic')

    def cli(label, command, request):
        """Retain one public operation and return its actual current Work."""
        label = label_for(label)
        request = dict(request, event_id=label_for(request['event_id']))
        code, raw, _ = driver.cli(label, command, request)
        shared.require(code == 0, 'automatic public operation refused: '+label)
        value = json.loads(raw)
        return value if command[0] in ('budget','mission') else value['work']

    code, raw, _ = driver.cli(label_for('automatic-work'),['work','show',driver.work])
    shared.require(code == 0, 'prepared automatic Work unavailable')
    work = json.loads(raw)['work']
    shared.require(work['id'] == driver.work and len(work['tasks']) == 2
        and all(t['scope_id'] == ('child' if delegated else 'root') for t in work['tasks']),
        'automatic fixture has wrong task owner')
    ownership = [(scope['id'],scope.get('parent',''),scope.get('requirements',[]))
                 for scope in work['planning']['scopes']]
    counters = (work['planning']['activations'],work['planning']['decisions'])
    if delegated:
        shared.require(ownership == [('root','',[]),('child','root',['req-1'])],
                       'delegated automatic fixture lacks public requirement transfer')
    argv = [driver.binary,'mission','status',driver.work,'--root',driver.root,'--json']

    def observe(label, state, action, authorized, paused=False):
        """Assert both task meanings and zero execution with pure reads."""
        result, _ = driver.read(label_for(label), argv)
        shared.require(result[0] == 0 and not result[2], 'automatic status refused: '+label)
        status = json.loads(result[1]); first, dependent = status['tasks']
        shared.require(status['authorized'] is authorized and status['enabled'] is False
            and status['paused'] is paused and status['validated'] == status['review'] == status['running'] == 0
            and status['active_agents'] == status['uncertain_agents'] == 0
            and first['state'] == state and first['action'] == action
            and first['result']['state'] == 'not_started'
            and dependent['state'] == 'waiting' and dependent['target'] == 'first',
            'automatic status invented launch/validation or lost dependency: '+label)
        code, raw, _ = driver.cli(label_for(label)+'-work',['work','show',driver.work])
        shared.require(code == 0, 'automatic public Work reread refused: '+label)
        current = json.loads(raw)['work']
        shared.require(current == work and
            [(scope['id'],scope.get('parent',''),scope.get('requirements',[]))
             for scope in current['planning']['scopes']] == ownership
            and (current['planning']['activations'],current['planning']['decisions']) == counters
            and all(t['scope_id'] == ('child' if delegated else 'root') and t['status'] == 'todo'
                    and not t.get('attempts') and not t.get('independent_review') for t in current['tasks']),
            'automatic observation changed ownership/consumption or invented execution/review')
        return status

    observe('automatic-configure-control','configure','configure',False)
    work = cli('automatic-budget',['budget','apply',driver.work],dict(schema_version=1,
        event_id='automatic-budget',expected_revision=work['revision'],budget=dict(limit_usd=10,
        reserve_per_launch_usd=1,estimate_source='Owned fixture, not pricing',reference_date='2026-10-23')))
    (driver.root/(prefix+'-worker')).mkdir(mode=0o700)
    work = cli('automatic-start',['mission','start',driver.work],dict(schema_version=1,
        event_id='automatic-start',expected_revision=work['revision'],reason='Owned automatic read fixture',slots=1,
        profile=dict(provider='fixture',role='worker',workspace=prefix+'-worker',level='standard',capture_output=True)))
    observe('automatic-authorized','waiting','supervise',True)
    work = cli('automatic-pause',['mission','pause',driver.work],dict(schema_version=1,
        event_id='automatic-pause',expected_revision=work['revision'],reason='Owned pause'))
    observe('automatic-paused','waiting','inspect',True,True)
    work = cli('automatic-unpause',['mission','unpause',driver.work],dict(schema_version=1,
        event_id='automatic-unpause',expected_revision=work['revision'],reason='Owned resume'))
    observe('automatic-resumed','waiting','supervise',True)
    work = cli('automatic-budget-held',['budget','apply',driver.work],dict(schema_version=1,
        event_id='automatic-budget-held',expected_revision=work['revision'],budget=dict(limit_usd=1,
        reserve_per_launch_usd=2,estimate_source='Owned insufficient reserve',reference_date='2026-10-23')))
    held = observe('automatic-budget-held','intervention','inspect',True)
    shared.require(held['tasks'][0]['reason'] == 'Budget estimatif insuffisant pour un nouveau départ.',
                   'automatic budget reason changed or refusal became readiness')
    work = cli('automatic-budget-restored',['budget','apply',driver.work],dict(schema_version=1,
        event_id='automatic-budget-restored',expected_revision=work['revision'],budget=dict(limit_usd=10,
        reserve_per_launch_usd=1,estimate_source='Owned restored reserve',reference_date='2026-10-23')))
    observe('automatic-budget-restored','waiting','supervise',True)
    checks = dict(initial_planned_automatic_readiness=True,automatic_budget_refusal_recovered=True,
                  automatic_pause_recovered=True,automatic_read_never_launches=True,
                  automatic_ownership_and_consumption_preserved=True)
    return {('delegated_'+key if delegated else key):value for key,value in checks.items()}


def planning_case(driver):
    """Observe public initial planning, start and pause without executing a model.

    A valid ordinary empty control precedes planning configuration. Actual public
    controls establish authorization; status must keep it distinct from a live
    conductor. Each read compares full typed and physical DB snapshots. This
    covers operator planning activations and empty decisions, never model execution,
    task dispatch, independent review or closure. Saved activation limits remain real.
    All writes target the owned fixture; original evidence survives any failure.
    """
    def cli(label, command, request=None):
        """Retain channels and require a successful public operation, no retry."""
        code, raw, _ = driver.cli(label, command, request)
        shared.require(code == 0, 'initial planning public operation refused: '+label)
        return json.loads(raw)

    work = cli('planning-status-create', ['work','create'], dict(schema_version=1,
        event_id='planning-status-create', expected_revision=0, title='Initial planning status',
        objective='Observe configured organization', scope='Owned status fixture',
        criteria=['No execution or closure inferred']))['work']
    driver.work = work['id']
    argv = [driver.binary,'mission','status',driver.work,'--root',driver.root,'--json']

    def observe(label, ready, authorized, paused=False, planning_paused=False):
        """Assert exact configuration/authorization distinctions and no read mutation."""
        result, _ = driver.read(label, argv)
        shared.require(result[0] == 0 and not result[2], 'initial planning status refused: '+label)
        value = json.loads(result[1])
        shared.require(value['organization']['ready'] is ready
                       and value['authorized'] is authorized and value['paused'] is paused
                       and value['enabled'] is False and value['supervision']['state'] == 'absent'
                       and value['total'] == 0 and value['validated'] == 0 and value['tasks'] == []
                       and value['active_agents'] == 0 and value['uncertain_agents'] == 0,
                       'initial planning invented execution, validation or permission')
        if ready:
            shared.require('0/2 appels' in value['organization']['verification']
                           and value['understanding']['actor_kind'] == ('user' if planning_paused or work['planning'].get('activations',0) >= work['planning']['max_activations'] else 'supervisor')
                           and value['understanding']['situation'] != 'termine',
                           'configured planning claims a review or closure')
        return value

    observe('planning-status-ordinary-control', False, False)
    checks = {'req-1':[dict(id='owned-proof', command=['python3','-c',"from pathlib import Path; assert Path('handoff.md').is_file()"],
                          criteria=[1], inputs=['handoff.md'], review_output=False,
                          justification='Explicit owned fixture proof') ]}
    work = cli('planning-status-enable',['planning','enable',driver.work],dict(schema_version=1,
        event_id='planning-status-enable',expected_revision=work['revision'], provider='fixture',
        level='standard',max_activations=3,max_tasks=10,max_decisions=10,max_review_calls=2,checks=checks))
    observe('planning-status-configured', True, False)
    work = cli('planning-status-budget',['budget','apply',driver.work],dict(schema_version=1,
        event_id='planning-status-budget',expected_revision=work['revision'],budget=dict(limit_usd=10,
        reserve_per_launch_usd=1,estimate_source='Owned inert fixture, not pricing',reference_date='2026-10-23')))
    (driver.root/'planning-status-worker').mkdir(mode=0o700)
    work = cli('planning-status-start',['mission','start',driver.work],dict(schema_version=1,
        event_id='planning-status-start',expected_revision=work['revision'],reason='Owned status fixture',slots=1,
        profile=dict(provider='fixture',role='worker',workspace='planning-status-worker',level='standard',capture_output=True)))
    observe('planning-status-authorized', True, True)
    work = cli('planning-status-pause',['mission','pause',driver.work],dict(schema_version=1,
        event_id='planning-status-pause',expected_revision=work['revision'],reason='Owned pause fixture'))
    observe('planning-status-paused', True, True, paused=True)
    work = cli('planning-status-unpause',['mission','unpause',driver.work],dict(schema_version=1,
        event_id='planning-status-unpause',expected_revision=work['revision'],reason='Owned pause recovery'))
    observe('planning-status-resumed', True, True)
    for action in ['pause','resume']:
        work = cli('planning-status-plan-'+action,['planning',action,driver.work],dict(schema_version=1,
            event_id='planning-status-plan-'+action,expected_revision=work['revision']))
        observe('planning-status-plan-'+action, True, True, planning_paused=action == 'pause')
    # Inject only an owned cooldown file, never SQL authority; restore all bytes.
    path = driver.root/'.swarm/provider-cooldowns/fixture.json'
    path.parent.mkdir(mode=0o700, exist_ok=True)
    original = path.read_bytes() if path.exists() else None
    errors = []
    try:
        base = dict(provider='fixture',observed_at='2026-10-23T00:00:00Z',
                    source='owned-planning-status',signal='backoff')
        future = int(time.time()) + 86400  # fixture expiry, not a test watchdog/policy
        expected_date = datetime.fromtimestamp(future, timezone.utc).strftime('%Y-%m-%d %H:%M:%S UTC')
        for suffix, reset, kind in [('unknown',0,'user'), ('known',future,'none')]:
            fixture.publish(path, dict(base,resets_at=reset))
            saved = path.read_bytes()
            result, _ = driver.read('planning-status-cooldown-'+suffix, argv)
            shared.require(result[0] == 0 and not result[2] and path.read_bytes() == saved,
                           'initial planning provider wait refused or changed')
            value = json.loads(result[1])
            shared.require(value['understanding']['actor_kind'] == kind and not value['enabled']
                           and value['authorized'] and value['organization']['ready']
                           and value['next'] == value['understanding']['next_step']
                           and len(value['provider_cooldowns']) == 1,
                           'provider wait invented authority or omitted observation')
            text = value['understanding']['what']
            shared.require(('Aucune heure de reprise fournie.' in text) if reset == 0
                           else expected_date in text, 'wrong provider expiry explanation')
        work = cli('planning-cooldown-pause',['mission','pause',driver.work],dict(schema_version=1,
            event_id='planning-cooldown-pause',expected_revision=work['revision'],reason='Owned cooldown pause'))
        result, _ = driver.read('planning-status-cooldown-paused', argv)
        shared.require(result[0] == 0, 'paused provider wait refused')
        value = json.loads(result[1])
        shared.require(value['paused'] and not value['enabled'] and
                       'La mission reste en pause' in value['next'], 'cooldown hides pause')
        work = cli('planning-cooldown-unpause',['mission','unpause',driver.work],dict(schema_version=1,
            event_id='planning-cooldown-unpause',expected_revision=work['revision'],reason='Owned cooldown recovery'))
        for suffix, data in [('expired',dict(base,resets_at=1)),
                             ('cleared',dict(base,resets_at=future,cleared_at='2026-10-23T01:00:00Z'))]:
            fixture.publish(path,data)
            value = observe('planning-status-cooldown-'+suffix, True, True)
            shared.require(not value.get('provider_cooldowns'), 'inactive cooldown presented as active')
        for suffix, data, code in [
            ('negative',dict(base,resets_at=-1),b'agent-provider-cooldown-invalid'),
            ('range',dict(base,resets_at=9223372036854775807),b'mission-status-cooldown-date-invalid'),
            ('malformed',dict(base,observed_at='invalid'),b'agent-provider-cooldown-observed-time-invalid')]:
            fixture.publish(path,data)
            saved = path.read_bytes()
            result, _ = driver.read('planning-status-cooldown-'+suffix, argv)
            shared.require(result[0] != 0 and code in result[2] and not result[1]
                           and path.read_bytes() == saved, 'invalid provider wait became successful or changed')
    except BaseException as error:
        errors.append(error)
    finally:
        try:
            if original is None: path.unlink(missing_ok=True)
            else: fixture.publish(path, original)
            observe('planning-status-cooldown-recovered', True, True)
        except BaseException as error:
            errors.append(error)
    if errors: raise BaseExceptionGroup('Initial planning cooldown observation and restoration', errors)
    shared.require(not work.get('tasks') and work['planning'].get('activations',0) == 0
                   and work['planning'].get('decisions',0) == 0,
                   'status journey executed a planner or created tasks')
    # Explicit fixture lease, within the saved operator policy; not a test watchdog.
    for index in range(3):
        if index:
            work = cli('planning-activation-resume-'+str(index), ['planning','resume',driver.work],
                dict(schema_version=1,event_id='planning-activation-resume-'+str(index),
                     expected_revision=work['revision'],reason='Owned next operator observation'))
            observe('planning-activation-ready-'+str(index), True, True)
        scope = work['planning']['scopes'][0]
        work = cli('planning-activation-claim-'+str(index), ['planning','claim',driver.work],
            dict(schema_version=1,event_id='planning-activation-claim-'+str(index),expected_revision=work['revision'],
                 scope='root',scope_revision=scope['revision'],holder='owned-status-planner',lease_seconds=60))
        shared.require(work['planning']['activations'] == index+1, 'claim counter not retained')
        observe('planning-activation-claimed-'+str(index), True, True)
        if index == 0:
            scope = work['planning']['scopes'][0]
            work = cli('planning-activation-renew', ['planning','lease-renew',driver.work],
                dict(schema_version=1,event_id='planning-activation-renew',expected_revision=work['revision'],
                     scope='root',scope_revision=scope['revision'],holder=scope['holder'],
                     generation=scope['generation'],lease_seconds=60))
            shared.require(work['planning']['activations'] == 1, 'renewal consumed another activation')
            observe('planning-activation-renewed', True, True)
        scope = work['planning']['scopes'][0]
        work = cli('planning-activation-decide-'+str(index), ['planning','decide',driver.work],
            dict(schema_version=1,event_id='planning-activation-decide-'+str(index),expected_revision=work['revision'],
                 scope='root',scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'],
                 input_events=scope['delivery']['events'],operations=[],reason='Explicit owned empty decision'))
        scope = work['planning']['scopes'][0]
        shared.require(work['planning']['decisions'] == index+1 and scope['state'] == 'waiting'
                       and not scope.get('holder') and not scope.get('lease_until'), 'decision fences not released')
        observe('planning-activation-decided-'+str(index), True, True)
        if index == 0:
            for action in ['pause','resume']:
                work = cli('planning-activation-'+action, ['planning',action,driver.work],
                    dict(schema_version=1,event_id='planning-activation-'+action,expected_revision=work['revision']))
                observe('planning-activation-'+action, True, True, planning_paused=action == 'pause')
    # A valid claimed/decided witness precedes the exhausted saved-policy refusal.
    shared.require(work['planning']['activations'] == work['planning']['max_activations'] == 3,
                   'fixture did not reach its explicit saved activation cap')
    before = cli('planning-activation-before-cap', ['work','show',driver.work])['work']
    scope = work['planning']['scopes'][0]
    cap_snapshot = shared.snapshot(driver.root,driver.work,driver.operator)
    code, raw, stderr = driver.cli('planning-activation-cap-refusal', ['planning','claim',driver.work],
        dict(schema_version=1,event_id='planning-activation-cap-refusal',expected_revision=work['revision'],
             scope='root',scope_revision=scope['revision'],holder='owned-status-planner',lease_seconds=60))
    shared.require(code != 0 and not raw and json.loads(stderr)['failure']['code'] == 'budget_exhausted',
                   'exhausted activation unexpectedly granted')
    shared.require(cli('planning-activation-after-cap', ['work','show',driver.work])['work'] == before,
                   'refused activation mutated Work')
    shared.require(shared.snapshot(driver.root,driver.work,driver.operator) == cap_snapshot,
                   'refused activation mutated retained state')
    limited = observe('planning-activation-exhausted', True, True)
    shared.require(limited['understanding']['what'] == 'Le plafond de planification est atteint.'
                   and not work.get('tasks'), 'saved cap or taskless history hidden')
    return dict(initial_planning_read_only=True,configuration_not_review=True,
                authorization_not_conductor=True,mission_and_planning_pause_recovered=True,
                cooldown_wait_expiry_clearance_invalid_and_recovery=True,
                operator_claim_renew_empty_decisions_and_saved_cap=True,
                model_execution_not_qualified=True)



def planning_todo_case(driver, delegated=False):
    """Observe initial todos on root or a publicly delegated child, with dependencies.

    Create only owned public Work/planning data; no profile, Agent, attempt or
    review is fabricated. Public abandonment/reopening must keep dependencies
    non-current and preserve counters/ownership. Delegated mode first transfers req-1 publicly, then
    claims the child and creates its tasks. Missing review stays non-current. Reads compare full
    typed/physical snapshots; pauses preserve decisions and cannot grant a start.
    Refusal channels are retained and stop the journey without automatic retry.
    """
    prefix = 'delegated-todo' if delegated else 'planned-todo'
    def cli(label, command, request):
        """Namespace each owned case’s event/label and retain its public JSON result."""
        label = label.replace('planned-todo',prefix)
        request = dict(request, event_id=request['event_id'].replace('planned-todo',prefix))
        code, raw, _ = driver.cli(label, command, request)
        shared.require(code == 0, 'planned todo public operation refused: '+label)
        return json.loads(raw)
    work = cli('planned-todo-create',['work','create'],dict(schema_version=1,event_id='planned-todo-create',
        expected_revision=0,title='First planned tasks',objective='Observe before dispatch',scope='Owned fixture',
        criteria=['First task must be configured before a launch']))['work']
    driver.work = work['id']
    checks = {'req-1':[dict(id='planned-proof',command=['python3','-c',"from pathlib import Path; assert Path('handoff.md').is_file()"],
        criteria=[1],inputs=['handoff.md'],review_output=False,justification='Explicit owned fixture check')]}
    work = cli('planned-todo-enable',['planning','enable',driver.work],dict(schema_version=1,
        event_id='planned-todo-enable',expected_revision=work['revision'],provider='fixture',level='standard',
        max_activations=3,max_tasks=10,max_decisions=10,max_review_calls=2,checks=checks))
    argv = [driver.binary,'mission','status',driver.work,'--root',driver.root,'--json']
    for label in ['planned-todo-control','planned-todo-claimed']:
        if label.endswith('claimed'):
            scope = work['planning']['scopes'][0]
            work = cli('planned-todo-claim',['planning','claim',driver.work],dict(schema_version=1,
                event_id='planned-todo-claim',expected_revision=work['revision'],scope='root',
                scope_revision=scope['revision'],holder='owned-todo-planner',lease_seconds=60))
        result, _ = driver.read(label.replace('planned-todo',prefix),argv)
        shared.require(result[0] == 0 and not result[2] and json.loads(result[1])['total'] == 0,
                       'planned todo valid control refused')
    if delegated:
        scope = work['planning']['scopes'][0]
        work = cli('planned-todo-delegate',['planning','decide',driver.work],dict(schema_version=1,
            event_id='planned-todo-delegate',expected_revision=work['revision'],scope='root',
            scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'],
            input_events=scope['delivery']['events'],operations=[dict(kind='delegate',id='child',
                title='Owned delegated scope',requirements=['req-1'],next='Prepare before dispatch',
                max_tasks=2,max_activations=2)],reason='Explicit owned delegation'))
        root_scope, child_scope = work['planning']['scopes']
        shared.require(root_scope['id'] == 'root' and not root_scope.get('requirements')
            and child_scope['id'] == 'child' and child_scope['parent'] == 'root'
            and child_scope['requirements'] == ['req-1'], 'public delegation lost requirement ownership')
        result, _ = driver.read(prefix+'-tree',argv)
        shared.require(result[0] == 0 and not result[2] and json.loads(result[1])['total'] == 0,
                       'publicly delegated tree status refused')
        work = cli('planned-todo-child-claim',['planning','claim',driver.work],dict(schema_version=1,
            event_id='planned-todo-child-claim',expected_revision=work['revision'],scope='child',
            scope_revision=child_scope['revision'],holder='owned-child-planner',lease_seconds=60))
        result, _ = driver.read(prefix+'-child-claimed',argv)
        shared.require(result[0] == 0 and not result[2] and json.loads(result[1])['total'] == 0,
                       'claimed delegated scope status refused')
    scope = next(scope for scope in work['planning']['scopes'] if scope['id'] == ('child' if delegated else 'root'))
    operations = [dict(kind='task',id=key,title=title,requirements=['req-1'],deliverable='Owned future report',
        criteria=['Explicit requirement check'],depends=depends,next='Configure before dispatch')
        for key,title,depends in [('first','First task',[]),('dependent','Dependent task',['first'])]]
    work = cli('planned-todo-decide',['planning','decide',driver.work],dict(schema_version=1,
        event_id='planned-todo-decide',expected_revision=work['revision'],scope=scope['id'],scope_revision=scope['revision'],
        holder=scope['holder'],generation=scope['generation'],input_events=scope['delivery']['events'],
        operations=operations,reason='Explicit owned first task decision'))
    for action in [None,'pause','resume']:
        if action:
            work = cli('planned-todo-'+action,['planning',action,driver.work],dict(schema_version=1,
                event_id='planned-todo-'+action,expected_revision=work['revision']))
        result, _ = driver.read(prefix+'-status-'+str(action),argv)
        shared.require(result[0] == 0 and not result[2], 'planned todo status refused')
        value = json.loads(result[1]); first, child = value['tasks']
        shared.require(value['total'] == 2 and value['validated'] == value['review'] == value['running'] == 0
            and value['active_agents'] == value['uncertain_agents'] == 0
            and value['authorized'] is False and value['enabled'] is False and value['organization']['ready'] is True,
            'planned todo configuration invented execution or authority')
        shared.require(first['state'] == first['action'] == 'configure'
            and child['state'] == 'waiting' and child['target'] == 'first'
            and child['action'] == 'inspect' and child['label'] == 'Examiner le prérequis'
            and all(t['result']['state'] == t['result']['process_state'] == t['result']['validation_state'] == 'not_started'
                    for t in [first,child]), 'planned todo dependency or result invented current evidence')
        shared.require(work['planning']['activations'] == work['planning']['decisions'] == (2 if delegated else 1)
            and all(t['status'] == 'todo' and not t.get('attempts') and not t.get('independent_review') for t in work['tasks']),
            'planned task observation changed retained history')
    counters = (work['planning']['activations'],work['planning']['decisions'])
    ownership = [(scope['id'],scope.get('parent',''),scope.get('requirements',[]))
                 for scope in work['planning']['scopes']]
    for decision in ['abandoned','todo']:
        label = 'planned-todo-'+('abandon' if decision == 'abandoned' else 'reopen')
        work = cli(label,['task','update',driver.work],dict(schema_version=1,event_id=label,
            expected_revision=work['revision'],id='first',status=decision,
            next='Explicit owned decision; retain its history'))['work']
        result, _ = driver.read(prefix+'-decision-'+decision,argv)
        shared.require(result[0] == 0 and not result[2], 'planned abandonment/reopening status refused')
        value = json.loads(result[1]); first, child = value['tasks']
        shared.require(value['validated'] == value['review'] == value['running'] == value['active_agents'] == 0
            and value['authorized'] is False and value['enabled'] is False
            and value['organization']['ready'] is True and value['understanding']['situation'] != 'termine',
            'planned abandonment invented validation or completion')
        shared.require(first['state'] == ('abandoned' if decision == 'abandoned' else 'configure')
            and first['action'] == ('inspect' if decision == 'abandoned' else 'configure')
            and first['result']['state'] == first['result']['validation_state'] == ('abandoned' if decision == 'abandoned' else 'not_started')
            and first['result']['process_state'] == 'not_started'
            and child['state'] == 'waiting' and child['target'] == 'first'
            and child['result']['validation_state'] == 'not_started',
            'planned decision lost nonvalidation or current dependency wait')
        shared.require((work['planning']['activations'],work['planning']['decisions']) == counters
            and [(scope['id'],scope.get('parent',''),scope.get('requirements',[])) for scope in work['planning']['scopes']] == ownership
            and work['tasks'][0]['status'] == decision
            and not work['tasks'][0].get('attempts') and not work['tasks'][0].get('independent_review'),
            'task decision changed planning ownership/consumption or invented attempt')
        if decision == 'abandoned':
            # Successful abandonment is the same-precondition witness before faults.
            for suffix, patch, expected in [
                ('definition',dict(status='todo',title='Unpermitted contract edit'),'format'),
                ('stale',dict(status='todo',expected_revision=work['revision']-1),'revision_conflict'),
                ('start',dict(status='running'),'command_not_ported')]:
                label = prefix+'-refuse-'+suffix
                before = shared.snapshot(driver.root,driver.work,driver.operator)
                proposal = dict(schema_version=1,event_id=label,expected_revision=work['revision'],id='first')
                proposal.update(patch)
                code, raw, stderr = driver.cli(label,['task','update',driver.work],proposal)
                shared.require(code != 0 and not raw and json.loads(stderr)['failure']['code'] == expected,
                               'scoped initial decision widened fields/revision/start: '+suffix)
                shared.require(shared.snapshot(driver.root,driver.work,driver.operator) == before,
                               'refused scoped decision mutated retained state: '+suffix)
    # Replaying an existing event returns current Work; it never reapplies old state.
    before = shared.snapshot(driver.root,driver.work,driver.operator)
    shared.publish(driver.out/(prefix+'-replay.before.json'),before)
    original = json.loads((driver.out/(prefix+'-abandon.input.json')).read_bytes())
    code, raw, _ = driver.cli(prefix+'-abandon-replay',['task','update',driver.work],original)
    after = shared.snapshot(driver.root,driver.work,driver.operator)
    shared.publish(driver.out/(prefix+'-replay.after.json'),after)
    shared.require(code == 0 and json.loads(raw)['work'] == work and after == before,
        'replayed abandonment reapplied old state or lost current Work')
    if delegated:
        shared.require(all(t['scope_id'] == 'child' for t in work['tasks']), 'tasks escaped delegated owner')
        return dict(delegated_tree_and_child_claim_visible=True,delegated_todo_dependencies_and_pause_pure=True,
                    delegated_missing_review_not_accepted=True,delegated_abandonment_reopened_without_validation=True)
    return dict(planned_initial_tasks_configure=True,planned_dependencies_wait_for_current_acceptance=True,
                planning_todo_pause_resume_pure=True,planned_missing_review_not_accepted=True,
                planned_abandonment_reopened_without_validation=True)



def run_planned_observed(driver, owned, prepared):
    """Observe own native supervisor/process before allowing its real completion.

    Requires the explicit inert release-handshake program installed before
    prepare. Actual Popen/recorded PID/PPID linkage precedes SIGSTOP and kernel
    stop acknowledgement, freezing only this owned heartbeat writer for snapshots.
    Finally release/resume/wait always run; primary and cleanup causes are grouped.
    No global watchdog, SQL seed, acceptance, live-user process or provider account.
    """
    argv=[str(driver.binary),'agent','run',prepared['agent']['id'],'--root',str(driver.root),'--json']
    shared.publish(driver.out/'planned-live.command.json',dict(argv=argv,global_watchdog=None))
    errors=[];stopped=False;code=None
    with (driver.out/'planned-live.stdout').open('xb') as out,(driver.out/'planned-live.stderr').open('xb') as err:
        child=subprocess.Popen(argv,stdout=out,stderr=err,env=driver.environment)
        try:
            while not (owned.root/'planned-worker-ready').exists():
                shared.require(child.poll() is None,'planned worker exited before handshake')
                time.sleep(0.01)  # fixture handshake polling, no product deadline.
            received=json.loads((owned.root/'model-received.json').read_bytes())
            agent=owned.ok(['agent','show',prepared['agent']['id']])['agent']
            while agent['status']=='starting':
                shared.require(child.poll() is None,'planned supervisor exited before heartbeat')
                time.sleep(0.01)
                agent=owned.ok(['agent','show',prepared['agent']['id']])['agent']
            shared.require(agent['supervisor_pid']==child.pid==received['ppid']
                and agent['child_pid']==received['pid'] and agent['status']=='running',
                'planned process/supervisor identity linkage differs')
            os.kill(child.pid,signal.SIGSTOP);stopped=True
            pid,state=os.waitpid(child.pid,os.WUNTRACED)
            shared.require(pid==child.pid and os.WIFSTOPPED(state),'planned supervisor stop not acknowledged')
            result,_=driver.read('planned-worker-real-running',
                [driver.binary,'mission','status',driver.work,'--root',driver.root,'--json'])
            shared.require(result[0]==0 and not result[2],'planned running historical status refused')
            status=json.loads(result[1]);task=status['tasks'][0]
            shared.require(status['running']==status['active_agents']==1
                and status['uncertain_agents']==status['validated']==status['review']==0
                and task['result']['state']=='in_progress'
                and task['result']['process_state']=='running'
                and task['result']['validation_state']=='pending_process'
                and task['attempt']['agent_id']==prepared['agent']['id'],
                'planned running observation invented identity or validation')
        except BaseException as error:errors.append(error)
        finally:
            try:shared.publish(owned.root/'planned-worker-release',b'finish owned fixture\n')
            except BaseException as error:
                errors.append(error)
                try:owned.ok(['agent','stop',prepared['agent']['id']])
                except BaseException as stop_error:errors.append(stop_error)
            try:
                if stopped:os.kill(child.pid,signal.SIGCONT)
            except BaseException as error:errors.append(error)
            try:
                code=child.wait()
                shared.require(code==0,'planned worker did not settle successfully')
            except BaseException as error:errors.append(error)
    if errors:raise BaseExceptionGroup('Planned live observation and owned process recovery',errors)
    return json.loads((driver.out/'planned-live.stdout').read_bytes())


def planned_terminal_case(parent, outcome):
    """Observe real public delegated cancellation/failure, without SQL manufacture.

    Each outcome owns a fresh publicly delegated child/task. Cancellation never
    starts a provider; failed executes one inert program with actual exit7.
    True queued status is the valid control before the terminal transition.
    Handoff/ownership/attempts and reservation semantics are checked, then read
    purity and no-validation remain explicit. These cases do not request a retry.
    """
    shared.require(outcome in ('cancelled','failed'),'unknown planned terminal fixture')
    prefix='planned-terminal-'+outcome
    ready,owned=planned_flow.flow(parent.binary,parent.out/prefix,before_worker=True,return_fixture=True)
    driver=shared.Driver(parent.binary,None,parent.out,owned.root,parent.operator,owned.environment)
    driver.work=ready['work_id'];argv=[driver.binary,'mission','status',driver.work,'--root',driver.root,'--json']
    if outcome=='failed':
        fixture.publish(owned.root/'codex',(manual_mission.PROGRAM+'\nraise SystemExit(7)\n').encode())
        (owned.root/'codex').chmod(0o700)
    preview=owned.prepare(preview=True);owned.request['model_policy_hash']=preview['agent']['model_route']['policy_hash']
    prepared=owned.prepare()['agent']
    result,_=driver.read(prefix+'-queued-control',argv)
    shared.require(result[0]==0 and not result[2] and json.loads(result[1])['uncertain_agents']==1,
                   'planned terminal queued witness refused')
    if outcome=='cancelled':
        settled=owned.ok(['agent','reconcile',prepared['id']])
        shared.require(settled['cancelled_before_start'] is True
            and settled['agent']['status']=='interrupted'
            and not (owned.root/'model-received.json').exists()
            and owned.reservation()==('released',1.0),'planned queued cancellation lost release or started provider')
    else:
        code,settled=owned.call(['agent','run',prepared['id']])
        shared.require(code==0 and settled['agent']['status']=='failed' and settled['agent']['exit_code']==7
            and settled['task_accepted'] is False and owned.reservation()==('estimated',1.0),
            'planned real failure lost exit/reservation or became acceptance')
    work=owned.ok(['work','show',owned.work])['work'];task=work['tasks'][0]
    shared.require(task['scope_id']=='child' and task['status']=='blocked' and len(task['attempts'])==1
        and work['planning']['reviewer'].get('calls',0)==0,'planned terminal lost ownership or invented review/attempt')
    result,_=driver.read(prefix+'-terminal',argv)
    shared.require(result[0]==0 and not result[2],'planned terminal historical status refused: '+outcome)
    status=json.loads(result[1]);task=status['tasks'][0]
    shared.require(status['running']==status['active_agents']==status['uncertain_agents']==status['validated']==status['review']==0
        and task['state']=='intervention' and task['result']['state']=='stopped_early'
        and task['result']['process_state']==('interrupted' if outcome=='cancelled' else 'failed')
        and task['result']['validation_state']=='not_validated'
        and task['attempt']['agent_id']==prepared['id'],
        'planned terminal observation retained liveness or invented validation')
    shared.require(owned.ok(['work','show',owned.work])['work']==work,'terminal status mutated original Work')
    parent.reads.extend(driver.reads)
    return {prefix.replace('-','_')+'_observed':True,prefix.replace('-','_')+'_root':str(owned.root)}


def planned_submitted_case(parent):
    """Observe a real child-owned submitted report without accepting its result.

    Public planner proposals create the child/task, an inert native program runs
    once and the real report is submitted. No private SQL writes or paid model.
    Valid unstarted status precedes queued/unconfirmed, hidden-window refusal,
    recovery and early unverified report; no status read starts the provider.
    The actual completed/unsubmitted result still refuses;
    Submitted status retains identity/cost and blocks until required controls
    actually execute; it then awaits review without accepting the task.
    Driver compares full typed/physical DB snapshots, including output refusal
    and recovery. Owned fixture births and original CLI errors are retained.
    This proves neither independent review nor autonomous dispatch or identity.
    """
    ready, owned = planned_flow.flow(parent.binary,parent.out/'planned-worker',
                                    before_worker=True,return_fixture=True)
    driver = shared.Driver(parent.binary,None,parent.out,owned.root,parent.operator,owned.environment)
    driver.work = ready['work_id']
    # Atomic readiness marker follows the original program's fsynced observed input.
    marker="print(json.dumps({'type':'system'"
    barrier="import tempfile,time\nfrom pathlib import Path\nfd,name=tempfile.mkstemp(dir='.',prefix='.planned-ready-')\nwith os.fdopen(fd,'wb') as f: f.write(b'ready');f.flush();os.fsync(f.fileno())\nos.replace(name,'planned-worker-ready')\nwhile not Path('planned-worker-release').exists(): time.sleep(0.01)\n"
    shared.require(manual_mission.PROGRAM.count(marker)==1,'owned worker program anchor changed')
    program=manual_mission.PROGRAM.replace(marker,barrier+marker,1)
    compile(program,str(owned.root/'codex'),'exec');fixture.publish(owned.root/'codex',program.encode());(owned.root/'codex').chmod(0o700)
    argv = [driver.binary,'mission','status',driver.work,'--root',driver.root,'--json']
    result, _ = driver.read('planned-worker-configure-control',argv)
    shared.require(result[0] == 0 and not result[2] and json.loads(result[1])['tasks'][0]['state'] == 'configure',
                   'planned worker initial status witness refused')
    preview = owned.prepare(preview=True)
    owned.request['model_policy_hash'] = preview['agent']['model_route']['policy_hash']
    prepared = owned.prepare()
    def queued(label):
        """Read real queued child history without inferring a live process or grant."""
        result, _ = driver.read(label,argv)
        shared.require(result[0] == 0 and not result[2], 'planned queued status refused: '+label)
        status = json.loads(result[1]); task = status['tasks'][0]
        shared.require(status['running'] == status['uncertain_agents'] == 1
            and status['active_agents'] == status['validated'] == status['review'] == 0
            and task['state'] == 'running' and task['action'] == 'inspect'
            and task['result']['state'] == 'execution_unconfirmed'
            and task['result']['process_state'] == 'queued/unconfirmed'
            and task['result']['validation_state'] == 'pending_process'
            and task['attempt']['agent_id'] == prepared['agent']['id']
            and not (owned.root/'model-received.json').exists()
            and owned.reservation() == ('reserved',1.0),
            'planned queued observation invented launch, live identity, acceptance or changed reservation')
        return status
    queued('planned-worker-queued')
    original = windows.shown(driver,'planned-queued-window-original')
    values = copy.deepcopy(original['values']); values['work_view_status_recent_agents'] = '0'
    errors = []
    try:
        windows.save(driver,'planned-queued-window-hide',original,values)
        result, _ = driver.read('planned-worker-queued-hidden-refused',argv)
        shared.require(result[0] != 0 and not result[1]
            and b'mission-status-task-history-unported' in result[2],
            'hidden planned queued Agent became invented readiness or successful empty status')
    except BaseException as error: errors.append(error)
    finally:
        try:
            current = windows.shown(driver,'planned-queued-window-before-restore')
            if current['values'] != original['values']:
                windows.save(driver,'planned-queued-window-restore',current,original['values'])
        except BaseException as error: errors.append(error)
    try: queued('planned-worker-queued-restored')
    except BaseException as error: errors.append(error)
    if errors: raise BaseExceptionGroup('Planned queued display window and restoration',errors)
    docs = owned.root/'docs'; docs.mkdir(exist_ok=True)
    shared.publish(docs/'task.md',b'Owned early fixture report, never validated.\n')
    early = queued('planned-worker-queued-early-report')['tasks'][0]['result']
    shared.require(early['report_state'] == 'early_unverified' and early['report_id'] == 'docs/task.md',
                   'planned early report became submitted or validated')
    actual = owned.run_and_assert(prepared,run_observed=lambda:run_planned_observed(driver,owned,prepared))
    result, _ = driver.read('planned-worker-unsubmitted-refused',argv)
    shared.require(result[0] != 0 and not result[1] and result[2],
                   'planned completed unsubmitted result became successful status')
    manual_mission.submit(owned)
    work = owned.ok(['work','show',owned.work])['work']
    agent = owned.ok(['agent','show','session'])['agent']
    shared.require(work['tasks'][0]['status'] == 'submitted' and work['tasks'][0]['scope_id'] == 'child'
        and len(work['tasks'][0]['attempts']) == 1 and work['planning']['reviewer'].get('calls',0) == 0,
        'planned public handoff invented attempt/review or changed owner')
    def observe(label, current_controls=False):
        """Observe real missing/current controls; neither result accepts the task.

        current_controls is a fixture assertion, not a product execution policy.
        Missing required receipt blocks review; actual controls permit inspection.
        """
        result, _ = driver.read(label,argv)
        shared.require(result[0] == 0 and not result[2], 'planned submitted public status refused: '+label)
        status = json.loads(result[1]); task = status['tasks'][0]
        shared.require(status['total'] == 1 and status['review'] == (1 if current_controls else 0)
            and status['validated'] == status['running'] == status['active_agents'] == status['uncertain_agents'] == 0
            and status['organization']['ready'] is True and status['enabled'] is False
            and task['state'] == ('review' if current_controls else 'intervention')
            and task['result']['state'] == ('result_to_review' if current_controls else 'review_blocked')
            and task['result']['process_state'] == 'completed'
            and task['result']['validation_state'] == ('ready_for_decision' if current_controls else 'failed_or_stale')
            and task['attempt']['agent_id'] == agent['id']
            and task['attempt']['attempt_id'] == prepared['agent']['attempt_id'],
            'planned submitted status lost actual attempt or invented acceptance')
        shared.require(owned.ok(['work','show',owned.work])['work'] == work,
                       'planned status changed original Work')
        if not current_controls:
            shared.require(work['planning']['reviewer_required'] is True
                and not work['tasks'][0].get('automatic_validation')
                and task['result']['reason']=='contrôles moteur courants requis avant la revue',
                'missing required controller receipt became a reviewable result')
        return status
    visible = observe('planned-worker-submitted')
    result, _ = driver.read('planned-worker-output-refused',argv,output_full=True)
    shared.require(result[0] != 0 and not result[1] and result[2],
                   'planned submitted output failure became success')
    observe('planned-worker-output-recovered')
    before_budget=owned.reservation()
    controlled=owned.ok(['task','validate',owned.work],dict(schema_version=1,event_id='planned-status-controls',expected_revision=work['revision'],task_id=work['tasks'][0]['id']))
    shared.require(controlled['controls_passed'] and controlled['task_accepted'] is False,
                   'actual planned controls failed or accepted task')
    work=owned.ok(['work','show',owned.work])['work'];receipt=work['tasks'][0]['automatic_validation']
    shared.require(work['tasks'][0]['status']=='submitted' and work['planning']['reviewer'].get('calls',0)==0
        and owned.reservation()==before_budget and receipt['controls']
        and all(row['executed'] and row['passed'] and row['started_at'] and row['finished_at'] for row in receipt['controls'])
        and receipt['artifacts'][receipt['receipt']]==shared.digest((owned.root/receipt['receipt']).read_bytes()),
        'controls lost original execution/receipt/reservation or invoked reviewer')
    visible=observe('planned-controls-current',current_controls=True)
    original = windows.shown(driver,'planned-worker-window-original')
    values = copy.deepcopy(original['values']); values['work_view_status_recent_agents'] = '0'
    errors = []
    try:
        windows.save(driver,'planned-worker-window-hide',original,values)
        result, _ = driver.read('planned-worker-window-hidden',argv)
        shared.require(result[0] == 0 and not result[2], 'hidden planned submitted status refused')
        hidden = json.loads(result[1]); task = hidden['tasks'][0]
        shared.require(hidden['review'] == 1 and hidden['validated'] == 0
            and task['result']['state'] == 'result_to_review'
            and not task['attempt'].get('agent_id')
            and all(hidden['spending'][key] == visible['spending'][key]
                    for key in ['calls','attempts','rows','worker_retries','recorded_control_executions']),
            'hidden planned Agent changed full accounting or invented process identity')
    except BaseException as error: errors.append(error)
    finally:
        try:
            saved = windows.shown(driver,'planned-worker-window-before-restore')
            if saved['values'] != original['values']:
                windows.save(driver,'planned-worker-window-restore',saved,original['values'])
        except BaseException as error: errors.append(error)
    try: observe('planned-worker-window-restored',current_controls=True)
    except BaseException as error: errors.append(error)
    if errors: raise BaseExceptionGroup('Planned submitted display window and restoration',errors)
    shared.require(actual['stdin_sha256'] == shared.digest(prepared['agent']['prompt'].encode()),
                   'planned worker input changed')
    parent.reads.extend(driver.reads)
    return dict(planned_submitted_real_worker=True,planned_actual_running_observed=True,planned_submitted_not_accepted=True,
                planned_submitted_controls_required=True,planned_controls_really_executed=True,
                planned_submitted_output_recovered=True,planned_submitted_hidden_agent_keeps_costs=True,
                planned_queued_unconfirmed=True,planned_queued_window_refused_recovered=True,
                planned_queued_early_report_unverified=True,
                planned_submitted_fixture_root=str(owned.root))


def main():
    """Bind supplied profile/binary/sources; retain fixture and original failures."""
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['binary', 'profile', 'output']:
        parser.add_argument('--'+key, type=Path, required=True)
    args = parser.parse_args(); output = args.output.resolve(); output.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve(); profile = json.loads(args.profile.read_bytes())
    shared.require(profile['status'] == 'VERIFIED_DEFAULT_NATIVE_PROFILE'
                   and profile['no_fixture_feature'] and profile['all_core_features_empty'] and profile['all_core_test_false']
                   and profile['sha256'] == shared.digest(binary.read_bytes()),
                   'normal qualified profile or binary mismatch')
    paths = [Path(__file__), Path(shared.__file__), Path(fixture.__file__), Path(windows.__file__), args.profile, binary]
    paths.extend(Path(module.__file__) for name,module in list(sys.modules.items())
                 if name.startswith('rust_') and getattr(module,'__file__',None))
    hashes = {str(path.resolve()): shared.digest(path.read_bytes()) for path in paths}
    shared.publish(output/'inputs.json', hashes)
    for label, path in [('method.py', Path(__file__)), ('shared.py', Path(shared.__file__)),
                        ('fixture.py', Path(fixture.__file__)), ('profile.json', args.profile)]:
        shared.publish(output/label, path.read_bytes())
    owned = None
    try:
        owned = fixture.Fixture(binary, output, 'accepted-status')
        driver = shared.Driver(binary, None, output, owned.root, 'owned-window-operator', owned.environment)
        driver.work = owned.work
        code, _, _ = driver.invoke('window-git-init', ['git', 'init', '-q', owned.root])
        shared.require(code == 0, 'window owned Git initialization failed')
        windows.normalize_provider_fixture(driver)
        completed = shared.completed_case(driver)
        checked = accepted_case(driver)
        checked.update(planning_case(driver))
        checked.update(planning_todo_case(driver))
        checked.update(automatic_case(driver))
        checked.update(planning_todo_case(driver, delegated=True))
        checked.update(automatic_case(driver, delegated=True))
        checked.update(planned_submitted_case(driver))
        checked.update(planned_terminal_case(driver,'cancelled'))
        checked.update(planned_terminal_case(driver,'failed'))
        shared.require(all(shared.digest(Path(path).read_bytes()) == sha for path, sha in hashes.items()),
                       'window declared source/profile/binary changed')
        shared.publish(output/'result.json', dict(status='PASS', checks=checked, completed=completed,
                        reads=driver.reads, root=str(owned.root), installed=False, paid_provider=False,
                        global_watchdog=None))
        print('PASS accepted freshness, saved profile manual/pause, dependencies and initial planning', flush=True)
    except BaseException as primary:
        try:
            shared.publish(output/'failure.json', dict(status='FAIL', traceback=traceback.format_exc(),
                           root=None if owned is None else str(owned.root)))
        except BaseException as cleanup:
            raise BaseExceptionGroup('Submitted window verification and failure publication', [primary, cleanup]) from None
        raise


if __name__ == '__main__':
    main()
