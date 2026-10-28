#!/usr/bin/env python3
"""Observe an ordinary queued launch through the qualified public historical CLI.

Public prepare creates a real reserved Agent. After cancellation/reopening, one
owned local fake provider executes under its actual supervisor; no account is used.
Queued identity stays unconfirmed; early report names prove no completion. Public
reconcile cancels the same attempt, releasing its reservation. Typed SQLite and
physical bytes must stay unchanged around every status read. No paid provider,
SDK, SQL writer, retry, watchdog, restart or acceptance. Retain failure originals.
"""
import argparse
import copy
import json
import os
from pathlib import Path
import tempfile
import traceback
import signal
import subprocess
import time
import rust_mission_status_contract as shared
import rust_status_submitted_window as windows


def active_case(driver):
    """Prepare publicly, inspect unconfirmed execution/report, cancel and recover.

    Fixture budget10/reserve1 and the owned local program are explicit inputs. Only public
    operations mutate the datastore. The owned docs file exercises actual report
    discovery; its presence never validates the task. Failures keep original IO.
    """
    def cli(label, command, request=None):
        """Decode one successful public reply; failures preserve recorded channels."""
        code, raw, _ = driver.cli(label, command, request)
        shared.require(code == 0, 'active fixture public operation refused')
        return json.loads(raw)

    argv = [driver.binary, 'mission', 'status', driver.work, '--root', driver.root, '--json']
    result, _ = driver.read('active-control', argv)
    shared.require(result[0] == 0 and not result[2]
                   and json.loads(result[1])['tasks'][0]['state'] == 'configure',
                   'active valid unstarted control refused')
    work = cli('active-work', ['work', 'show', driver.work])['work']
    work = cli('active-budget', ['budget', 'apply', driver.work],
               dict(schema_version=1, event_id='budget', expected_revision=work['revision'],
                    budget=dict(limit_usd=10, reserve_per_launch_usd=1,
                                estimate_source='Owned inert status fixture', reference_date='2026-10-23')))
    program = """#!/usr/bin/env python3
import os,json,sys,time,tempfile
from pathlib import Path
base=Path(__file__).parent
raw=sys.stdin.buffer.read()
fd,name=tempfile.mkstemp(dir=base,prefix='.ready-')
with os.fdopen(fd,'w') as stream:
    json.dump(dict(pid=os.getpid(),ppid=os.getppid()),stream);stream.flush();os.fsync(stream.fileno())
os.replace(name,base/'active-ready.json')
while not (base/'active-release').exists(): time.sleep(0.01)
(base/'handoff.md').write_bytes(b'Owned local report; no acceptance.\\n')
print(json.dumps(dict(type='result',usage=dict(input_tokens=7,output_tokens=8),total_cost_usd=0)),flush=True)
"""
    executable = driver.root/'active-worker'; compile(program,str(executable),'exec')
    shared.publish(executable,program.encode()); executable.chmod(0o700)
    cli('active-provider', ['providers', 'config', 'init'],
        dict(schema_version=1, providers={'fixture': dict(command=str(executable), args=[])}))
    prepared = cli('active-prepare', ['agent', 'prepare', driver.work],
                   dict(schema_version=1, event_id='prepare', expected_revision=work['revision'],
                        task_id='task', provider='fixture', capture_output=True))['agent']
    shared.require(prepared['status'] == 'queued', 'active witness is not queued')
    result, _ = driver.read('active-queued', argv)
    shared.require(result[0] == 0 and not result[2], 'queued historical status refused')
    status = json.loads(result[1]); task = status['tasks'][0]
    shared.require(status['running'] == status['uncertain_agents'] == 1
                   and status['active_agents'] == status['validated'] == status['review'] == 0
                   and task['state'] == 'running' and task['action'] == 'inspect'
                   and task['result']['state'] == 'execution_unconfirmed'
                   and task['result']['process_state'] == 'queued/unconfirmed'
                   and task['result']['validation_state'] == 'pending_process'
                   and task['attempt']['agent_id'] == prepared['id'],
                   'queued observation invented live process, validation or identity')
    original = windows.shown(driver,'active-window-original')
    values = copy.deepcopy(original['values']); values['work_view_status_recent_agents'] = '0'
    errors = []
    try:
        windows.save(driver,'active-window-zero',original,values)
        hidden, _ = driver.read('active-window-hidden',argv)
        shared.require(hidden[0] != 0 and hidden[1] == b''
                       and b'mission-status-task-history-unported' in hidden[2],
                       'hidden active Agent invented readiness or successful empty status')
    except BaseException as error:
        errors.append(error)
    finally:
        try:
            current = windows.shown(driver,'active-window-before-restore')
            if current['values'] != original['values']:
                windows.save(driver,'active-window-restore',current,original['values'])
        except BaseException as error:
            errors.append(error)
    if errors: raise BaseExceptionGroup('Hidden active Agent refusal and saved setting restoration',errors)
    restored, _ = driver.read('active-window-recovered',argv)
    shared.require(restored[0] == 0 and json.loads(restored[1])['uncertain_agents'] == 1,
                   'active window restoration did not recover observation')
    reports = driver.root/'docs'; reports.mkdir(exist_ok=True)
    shared.publish(reports/'task.md', b'Owned early report; no completed process or validation.\n')
    result, _ = driver.read('active-early-report', argv)
    shared.require(result[0] == 0 and not result[2], 'queued early report refused')
    task = json.loads(result[1])['tasks'][0]
    shared.require(task['result']['report_state'] == 'early_unverified'
                   and task['result']['report_id'] == 'docs/task.md'
                   and task['result']['validation_state'] == 'pending_process',
                   'early report became completion or validation')
    cancelled = cli('active-cancel', ['agent', 'reconcile', prepared['id']])
    shared.require(cancelled['cancelled_before_start'] is True
                   and cancelled['agent']['status'] == 'interrupted'
                   and cancelled['agent']['id'] == prepared['id'],
                   'active cancellation did not preserve original attempt')
    result, _ = driver.read('active-cancelled', argv)
    shared.require(result[0] == 0 and not result[2], 'cancelled status recovery refused')
    status = json.loads(result[1])
    shared.require(status['running'] == status['active_agents'] == status['uncertain_agents'] == status['validated'] == 0
                   and status['tasks'][0]['result']['state'] == 'stopped_early',
                   'cancelled observation retained active process or validation')
    running = running_case(driver, cli)
    return dict(public_prepare=True, queued_unconfirmed=True, early_report_unverified=True,
                same_attempt_cancelled=True, paid_provider=False, reads_unchanged=True,
                running_process_public_witness=running, global_migration_acceptance=False)


def running_case(driver, cli):
    """Observe a real owned local worker, freeze its supervisor, then finish/submit.

    SIGSTOP/SIGCONT target only this Popen child after public Agent identity and
    PID equality checks; kernel stop acknowledgement freezes its DB heartbeat
    writer for before/after purity checks. A stopped supervisor still exists,
    and confirmation proves neither provider progress nor successful completion.
    Finally always resumes it and releases the owned provider's file handshake;
    original and cleanup failures remain grouped. No global watchdog or retry.
    """
    work = cli('running-work', ['work', 'show', driver.work])['work']
    work = cli('running-reopen', ['task', 'update', driver.work],
               dict(schema_version=1,event_id='running-reopen',expected_revision=work['revision'],id='task',status='todo'))['work']
    prepared = cli('running-prepare', ['agent','prepare',driver.work],
                   dict(schema_version=1,event_id='running-prepare',expected_revision=work['revision'],
                        task_id='task',provider='fixture',capture_output=True))['agent']
    argv = [str(driver.binary),'agent','run',prepared['id'],'--root',str(driver.root),'--json']
    shared.publish(driver.out/'running-command.json',dict(argv=argv,global_watchdog=None,
                   scope='owned local process only; no provider account'))
    errors = []; stopped = False; observed = None
    stdout = driver.out/'running.stdout'; stderr = driver.out/'running.stderr'
    with stdout.open('xb') as out, stderr.open('xb') as err:
        child = subprocess.Popen(argv,stdout=out,stderr=err,env=driver.environment)
        try:
            ready = driver.root/'active-ready.json'
            while not ready.exists():
                shared.require(child.poll() is None,'owned worker exited before readiness; see running IO')
                time.sleep(0.01)  # fixture handshake polling; no product deadline or retry.
            received = json.loads(ready.read_bytes())
            agent = cli('running-agent-show',['agent','show',prepared['id']])['agent']
            while agent['status'] == 'starting':
                shared.require(child.poll() is None,'owned supervisor exited while awaiting first heartbeat')
                time.sleep(0.01)  # wait for the real persisted pulse, not a status rewrite.
                agent = cli('running-await-pulse',['agent','show',prepared['id']])['agent']
            shared.require(agent['supervisor_pid'] == child.pid == received['ppid']
                           and agent['child_pid'] == received['pid'] and agent['status'] == 'running',
                           'owned running supervisor/provider PID linkage differs')
            os.kill(child.pid,signal.SIGSTOP)
            stopped = True
            pid, state = os.waitpid(child.pid,os.WUNTRACED)
            shared.require(pid == child.pid and os.WIFSTOPPED(state),'owned supervisor stop not acknowledged')
            result, _ = driver.read('active-running',[driver.binary,'mission','status',driver.work,'--root',driver.root,'--json'])
            shared.require(result[0] == 0 and not result[2],'running historical status refused')
            observed = json.loads(result[1]); task = observed['tasks'][0]
            shared.require(observed['active_agents'] == observed['running'] == 1
                           and observed['uncertain_agents'] == observed['validated'] == 0
                           and task['result']['state'] == 'in_progress'
                           and task['result']['process_state'] == 'running'
                           and task['result']['validation_state'] == 'pending_process',
                           'running status lost live identity or invented validation')
        except BaseException as error:
            errors.append(error)
        finally:
            try:
                shared.publish(driver.root/'active-release',b'finish owned local provider\n')
            except BaseException as error:
                errors.append(error)
                try: cli('running-cleanup-stop',['agent','stop',prepared['id']])
                except BaseException as stop_error: errors.append(stop_error)
            try:
                if stopped: os.kill(child.pid,signal.SIGCONT)
            except BaseException as error:
                errors.append(error)
            try:
                code = child.wait()
                shared.require(code == 0,'owned worker did not settle successfully; see running IO')
            except BaseException as error:
                errors.append(error)
    if errors: raise BaseExceptionGroup('Running status and owned process recovery',errors)
    settled = json.loads(stdout.read_bytes())['agent']
    shared.require(settled['status'] == 'completed' and settled['id'] == prepared['id'],
                   'real worker completion lost original Agent')
    work = cli('running-finished-work',['work','show',driver.work])['work']
    report = driver.root/'handoff.md'
    cli('running-submit',['task','submit',driver.work],
        dict(schema_version=1,event_id='running-submit',expected_revision=work['revision'],id='task',
             agent_id=settled['id'],report_path='handoff.md',report_sha256=shared.digest(report.read_bytes())))
    result, _ = driver.read('active-running-submitted',[driver.binary,'mission','status',driver.work,'--root',driver.root,'--json'])
    shared.require(result[0] == 0 and not result[2],'running result submission status refused')
    status = json.loads(result[1])
    shared.require(status['active_agents'] == status['uncertain_agents'] == status['validated'] == 0
                   and status['review'] == 1 and status['tasks'][0]['result']['state'] == 'result_to_review',
                   'completed submitted result lost pending review or became accepted')
    return dict(real_supervisor_observed=True,owned_supervisor_frozen_for_purity=True,
                provider_progress_proven_by_status=False,completed_then_submitted=True,
                accepted=False)


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
    paths = [Path(__file__), Path(shared.__file__), Path(shared.fixture.__file__), Path(windows.__file__), args.profile, binary]
    inputs = {str(path.resolve()): shared.digest(path.read_bytes()) for path in paths}
    shared.publish(output/'inputs.json', inputs)
    for label, path in [('method.py', Path(__file__)), ('shared.py', Path(shared.__file__)),
                        ('fixture.py', Path(shared.fixture.__file__)), ('profile.json', args.profile)]:
        shared.publish(output/label, path.read_bytes())
    root = Path(tempfile.mkdtemp(prefix='swarm-active-'))
    shared.publish(output/'owned-root.json', dict(root=str(root), scope='owned isolated fixture',
                   binary_sha256=profile['sha256'], actor_authenticated=False))
    driver = shared.Driver(binary, None, output, root, 'owned-active-operator', dict(os.environ))
    try:
        code, _, _ = driver.cli('init', ['init'])
        shared.require(code == 0, 'active owned Root initialization refused')
        request = dict(schema_version=1, event_id='create', expected_revision=0, title='Active observation fixture',
                       objective='Observe queued and real local running agents', scope='Owned local Root', criteria=['No inferred acceptance'])
        code, raw, _ = driver.cli('create', ['work', 'create'], request)
        shared.require(code == 0, 'active Work creation refused')
        work = json.loads(raw)['work']; driver.work = work['id']
        request = dict(schema_version=1, event_id='task', expected_revision=work['revision'],
                       id='task', title='Explicit active', deliverable='Report', criteria=['No mutation'])
        code, _, _ = driver.cli('task', ['task', 'add', driver.work], request)
        shared.require(code == 0, 'active task creation refused')
        checks = active_case(driver)
        shared.require(all(shared.digest(Path(path).read_bytes()) == sha for path, sha in inputs.items()),
                       'active declared inputs changed during verification')
        shared.publish(output/'result.json', dict(status='PASS', checks=checks, reads=driver.reads,
                       root=str(root), paid_provider=False, installed=False, global_watchdog=None))
        print('PASS public queued and running observation, early report, cancellation and submission recovery', flush=True)
    except BaseException as primary:
        try:
            shared.publish(output/'failure.json', dict(status='FAIL', root=str(root), traceback=traceback.format_exc()))
        except BaseException as cleanup:
            raise BaseExceptionGroup('Active verification and failure publication', [primary, cleanup]) from None
        raise


if __name__ == '__main__':
    main()
