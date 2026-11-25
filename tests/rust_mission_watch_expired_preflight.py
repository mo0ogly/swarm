"""Exercise queued-adoption preflight renewal alongside an independent Work.

A and C run in the owned Web Root. Release A, stop Web cooperatively and keep C.
Admit B using the real SDK, then reopen Web and compare identities and ledgers.
The SDK leaves B's claim stopped after a committed admission. This witness
waits for the persisted original preflight to expire and verifies a scoped new
receipt before the same Agent finishes. C must stay alive through adoption.
"""
from contextlib import ExitStack
import argparse
import copy
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import select
import selectors
import signal
import sqlite3
import sys
import time
import traceback
from urllib.parse import urlencode

import fixture_area
import rust_agent_model_route as fixture
import rust_mission_watch_public as public
import rust_mission_watch_worker as worker
import rust_native_http_cli as plain_web
from rust_native_http_mission import request
import rust_native_parent_death as parent_death
import rust_planning_worker_public_flow as planning
import rust_planning_rejection_recovery as feedback
from rust_mission_report_relay import rows
from rust_workspace_fifo import ReusedFixture
import rust_mission_watch_restart as restart



def phase(out, label, **data):
    """Write an immutable observation and a concise live stage marker."""
    fixture.publish(out / (label + '.json'), data)
    print(label, flush=True)

def prepare_work(owned):
    """Create an explicit one-worker plan via real public transitions.

    The inert executable carries one explicit proposal per invocation and writes
    its observations only in the actual confined working directory. It never
    obtains root-file access to stage its response.
    """
    initial = owned.ok(['work','create'], dict(schema_version=1, event_id='fifo-create',
        expected_revision=0, title='FIFO witness', objective='Observe workspace admission',
        scope='Owned test only', criteria=['Retain one work result']))['work']
    owned.work = initial['id']
    checks = {'req-1':[dict(id='local-report', command=['python3','-c','print(1)'], criteria=[1],
                           inputs=['handoff.md'], justification='Owned fixture requirement')]}
    work = owned.ok(['planning','enable',owned.work], planning.request(event='fifo-enable',
        max_activations=6,checks=checks))
    for scope, operation in (
        ('root',dict(kind='delegate',id='child',title='Own scoped work',next='Create one task',
                     requirements=['req-1'],max_tasks=1,max_activations=3)),
        ('child',dict(kind='task',id='task',title='Local inert worker',next='Produce handoff',
                      requirements=['req-1'],deliverable='handoff.md',criteria=['Owned result'])),
    ):
        reply = dict(input_events=planning.pending(work,scope), reason='Explicit FIFO fixture', operations=[operation])
        assert reply['input_events']
        fixture.publish(owned.output/(scope+'-proposal.json'),reply)
        needle = "proposal=json.load(open('planning-proposal.json'))"
        assert feedback.PROGRAM.count(needle) == 1
        program = feedback.PROGRAM.replace(needle, 'proposal=json.loads('+repr(json.dumps(reply))+')')
        fixture.publish(owned.root/'codex',program.encode())
        (owned.root/'codex').chmod(0o700)
        result = owned.ok(['planning','step',owned.work])['run']
        assert result['decision_applied']
        work = result['work']
    assert len(work['tasks']) == 1 and work['tasks'][0]['scope_id'] == 'child'
    owned.ok(['budget','apply',owned.work],dict(schema_version=1,event_id='fifo-budget',
        expected_revision=work['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=1,
            estimate_source='Inert fixture reserve only',reference_date='2026-11-22')))

def run(binary, sdk, out, fixture_base):
    """Start and clean up only the attributed test server and workers."""
    first = fixture.Fixture(binary, out / 'setup' / 'first', 'first')
    prepare_work(first)
    original = fixture.Fixture

    class AdditionalWork(ReusedFixture):
        """Reuse one Root and namespace public event IDs by evidence directory."""

        def call(self, argv, value=None):
            """Forward real CLI operations with distinct fixture event identifiers."""
            if value is not None and 'event_id' in value:
                value = copy.deepcopy(value)
                value['event_id'] = self.output.name + '-' + value['event_id']
            return original.call(self, argv, value)

    AdditionalWork.original = first
    fixture.Fixture = AdditionalWork
    try:
        second = AdditionalWork(binary, out / 'setup' / 'second', 'second')
        prepare_work(second)
        third = AdditionalWork(binary, out / 'setup' / 'third', 'third')
        prepare_work(third)
    finally:
        fixture.Fixture = original
        AdditionalWork.original = None
    works = dict(A=first, B=second, C=third)
    root = first.root
    # C keeps its normal writer heartbeat. Give read-only proof snapshots
    # the same SQLite contention allowance as this fixture's saved Root.
    saved = json.loads((root / '.swarm/engine-settings.json').read_text())
    timeout = int(saved['values']['storage_busy_timeout_ms']) / 1000

    ids = {name: context.work for name, context in works.items()}
    assert len(set(ids.values())) == 3 and all(item.root == root for item in works.values())
    assert root.is_relative_to(fixture_base)
    paths = dict(A=root / 'shared', B=root / 'watch-worker', C=root / 'separate')
    for name in ('A', 'C'):
        paths[name].mkdir(mode=0o700)
    nonce = fixture.digest(str(root).encode())
    source = worker.provider_program(nonce)
    gate_start = source.index('    with socket.socket(')
    gate_end = source.index('        if json.loads(', gate_start)
    source = source[:gate_start] + """    while not Path('release-worker.json').exists():
        try:
            os.read(fd,65536)
        except BlockingIOError:
            import select
            select.select([fd],[],[])
""" + source[gate_end:].replace('        if json.loads(', '    if json.loads(', 1)
    # Workers can only enter the three designated directories. The local program
    # has no remote-provider client and the original prompt is never logged.
    needle = 'args=sys.argv[1:]'
    assert source.count(needle) == 1
    source = source.replace(needle, 'assert str(Path.cwd()) in '+repr(list(map(str,paths.values())))+'\n'+needle)
    fixture.publish(out / 'inert-provider.py', source.encode())
    fixture.publish(root / 'codex', source.encode())
    (root / 'codex').chmod(0o700)
    phase(out, '01-fixture', root=str(root), works=ids, spaces={k:str(v) for k,v in paths.items()},
          binary_sha256=fixture.digest(binary.read_bytes()), watchdog=None, paid_provider=False)
    active = {}
    lease_before = {}
    server = None
    errors = []
    failure = None

    with ExitStack() as cleanup:
        markers = {}
        for name in ('A', 'C'):
            directory = paths[name]
            descriptor = worker.watch_directory(directory, modifications=True)
            cleanup.callback(os.close, descriptor)
            markers[name] = descriptor

        def start_server(label):
            """Start an attributed foreground server with the fixture catalogue."""
            instance = plain_web.Server(binary, root, out, label, environment=first.environment)
            identity = worker.observation(instance.process.pid)
            descriptor = worker.attach(identity)
            cleanup.callback(os.close, descriptor)
            assert fixture.digest(Path(f'/proc/{identity["pid"]}/exe').read_bytes()) == fixture.digest(binary.read_bytes())
            return instance, identity, descriptor

        def path(name, action):
            return '/api/v1/mission?' + urlencode(dict(work=ids[name], action=action))

        def control(name, action, event, **body):
            current = works[name].ok(['work','show',ids[name]])['work']
            return request(server, 'POST', path(name, action),
                dict(schema_version=1, event_id=event, expected_revision=current['revision'],
                     reason='Owned inert FIFO witness', **body), csrf)

        def consent(name):
            control(name, 'start', 'fixture-start-'+name, slots=1,
                    profile=dict(provider='fixture', role='worker', workspace=str(paths[name].relative_to(root)),
                                 level='standard', capture_output=True))

        def journal(name):
            """Read unchanged public Work and its recorded Agent view."""
            value = works[name].ok(['mission','status-native',ids[name]])
            assert isinstance(value['agents'], list)
            return value['agents']

        def state(name, expected):
            """Read the one actual standing or released FIFO row, when present."""
            queue = request(server, 'GET', path(name,'queue'))
            turns = [row for row in queue['turns'] if row['work'] == ids[name]]
            if not turns:
                return None, queue
            assert len(turns) == 1, turns
            row = turns[0]
            assert Path(row['workspace']) == paths[name], row
            return (row if row['state'] == expected else None), queue

        def wait_turn(name, expected, *, deny_entry=False, previous_lease=None):
            """Wait on actual datastore changes and detect early provider/server activity."""
            with selectors.DefaultSelector() as selector:
                descriptor = worker.watch_directory(root/'.swarm', modifications=True)
                selector.register(descriptor, selectors.EVENT_READ, 'root')
                selector.register(server_fd, selectors.EVENT_READ, 'server')
                if deny_entry:
                    selector.register(markers[name], selectors.EVENT_READ, 'provider')
                try:
                    while True:
                        row, view = state(name, expected)
                        if deny_entry:
                            assert not (paths[name]/'model-received.json').exists(), (name, 'worker entered prematurely', view)
                        if row is not None:
                            if previous_lease is None:
                                return row, view
                            current_lease = worker.supervision(works[name])
                            if not current_lease['stopped_at'] and current_lease['conductor'] != previous_lease['conductor']:
                                return row, view
                        for key, _ in selector.select():
                            assert key.data != 'server', (name, expected, 'server exited', view)
                            os.read(key.fd, 65536)
                            if key.data == 'provider':
                                assert not (paths[name]/'model-received.json').exists(), (name, 'worker entered prematurely', view)
                finally:
                    os.close(descriptor)

        def accept(name):
            """Bind the observed provider and helper to the correct public Agent."""
            marker = paths[name]/'model-received.json'
            while not marker.exists() or not marker.stat().st_size:
                ready,_,_ = select.select([markers[name],server_fd],[],[])
                assert server_fd not in ready, (name, 'server exited before worker marker')
                os.read(markers[name],65536)
            entry = json.loads(marker.read_bytes())
            provider = worker.observation(entry['pid'])
            sup = worker.observation(provider['ppid'])
            assert provider['ppid'] == entry['ppid'] and sup['ppid'] == server_identity['pid']
            assert fixture.digest(Path(f'/proc/{sup["pid"]}/exe').read_bytes()) == fixture.digest(binary.read_bytes())
            argv = Path(f'/proc/{sup["pid"]}/cmdline').read_bytes().split(b'\0')
            assert len(argv) == 7 and argv[:2] == [os.fsencode(binary),b'_mission_worker_entry']
            assert argv[3:] == [b'--root',os.fsencode(root),b'--json',b'']
            agent_id = os.fsdecode(argv[2])
            sup_fd = worker.attach(sup)
            cleanup.callback(os.close, sup_fd)
            provider_fd = worker.attach(provider)
            cleanup.callback(os.close, provider_fd)
            agent = worker.wait_running(works[name], agent_id, server_fd, sup_fd)
            assert agent['work_id'] == ids[name] and agent['child_pid'] == provider['pid']
            assert agent['native_execution']['phase'] == 'permission_recorded'
            assert Path(os.readlink(f'/proc/{provider["pid"]}/cwd')) == paths[name]
            worker.verify_current_root_binding(agent['native_mission'], root)
            active[name] = dict(supervisor=sup, supervisor_fd=sup_fd, provider=provider,
                                provider_fd=provider_fd, agent_id=agent_id)
            return agent_id

        def alive(name):
            entry = active[name]
            assert not select.select([entry['provider_fd'],entry['supervisor_fd']],[],[],0)[0]
            current = worker.observation(entry['provider']['pid'])
            assert current['stamp'] == entry['provider']['stamp']
            assert current['ppid'] == entry['supervisor']['pid']

        def release(name):
            fixture.publish(paths[name]/'release-worker.json', dict(nonce=nonce,release=True))

        def settlement(name):
            agent_id = active[name]['agent_id']
            records = rows(first,'SELECT work_id, state, amount FROM reservations WHERE agent_id=?',(agent_id,))
            assert len(records) == 1 and records[0][0] == ids[name], records
            return records[0]

        def epoch(name):
            """Read the actual fence of one isolated Work, with no SQL mutation."""
            uri = (root/'.swarm/state.db').as_uri()+'?mode=ro'
            connection = sqlite3.connect(uri, uri=True)
            try:
                connection.row_factory = sqlite3.Row
                records = connection.execute(
                    'SELECT generation,pid,process_stamp,heartbeat_at,stopped_at,'
                    'e.conductor_id AS conductor,stale_ms,poll_ms '
                    'FROM native_mission_lease_epochs e JOIN mission_supervision s '
                    'ON e.work_id=s.work_id AND e.conductor_id=s.conductor_id WHERE e.work_id=?',
                    (ids[name],)).fetchall()
                assert len(records) == 1, records
                return dict(records[0])
            finally:
                connection.close()

        def stop_server():
            nonlocal server
            record = server.stop()
            plain_web.finish(server, None)
            server = None
            return record

        try:
            server, server_identity, server_fd = start_server('before-seed')
            csrf = server.request('/api/v1/native-session')['csrf_token']
            consent('A')
            a_id = accept('A')
            a, _ = wait_turn('A', 'acquired')
            assert a['agent'] == a_id
            consent('C')
            c_id = accept('C')
            c, _ = wait_turn('C', 'acquired')
            assert c['agent'] == c_id
            alive('A')
            alive('C')
            phase(out, '02-a-c-active', a=a, c=c, c_pid=active['C']['provider']['pid'])
            control('A', 'stop', 'stop-a-before-seed')
            release('A')
            wait_turn('A', 'released')
            select.select([active['A']['supervisor_fd']], [], [])
            stop_server()
            alive('C')
            seed_dir = out / 'seed'
            seed_dir.mkdir(mode=0o700)
            seed = restart.seed_queue(second, sdk, seed_dir)
            queued_agent = second.ok(['agent','show',seed['agent_id']])['agent']
            assert queued_agent['status'] == 'queued' and seed['attempt_id'] == queued_agent['attempt_id']
            before = restart.admissions(second, busy_timeout=timeout)
            costs = second.ok(['costs', 'show', ids['B']])
            before_queue = second.ok(['mission','queue',ids['B']])
            b = restart.completion.one_turn(before_queue, ids['B'], queued_agent['id'])
            previous = epoch('B')
            assert previous['generation'] == seed['generation'] == 1 and previous['stopped_at']
            assert b['state'] == 'acquired' and len(before['agents']) == len(before['reservations']) == 1
            assert not (paths['B']/'model-received.json').exists()
            alive('C')
            phase(out, '03-committed-unstarted', agent=queued_agent['id'], attempt=queued_agent['attempt_id'], queue=b,
                  admissions=before,c_pid=active['C']['provider']['pid'],epoch=previous)
            original_preflight = queued_agent['preflight']
            expired_at = datetime.fromisoformat(original_preflight['valid_until'])
            while (remaining := (expired_at - datetime.now(timezone.utc)).total_seconds()) > 0:
                time.sleep(remaining)
            alive('C')
            phase(out, '03b-expired', original=original_preflight, observed_at=datetime.now(timezone.utc).isoformat())
            markers['B'] = worker.watch_directory(paths['B'], modifications=True)
            cleanup.callback(os.close, markers['B'])
            server, server_identity, server_fd = start_server('after-seed')
            csrf = server.request('/api/v1/native-session')['csrf_token']
            try:
                b_id = accept('B')
            except BaseException:
                alive('C')
                assert restart.admissions(second, busy_timeout=timeout)==before
                assert not (paths['B']/'model-received.json').exists()
                assert [len(journal(name)) for name in works]==[1,1,1]
                observed_c=third.ok(['agent','show',c_id])['agent']
                assert observed_c['status']=='running' and observed_c['child_pid']==active['C']['provider']['pid']
                adopted=restart.read_adoptions(second, queued_agent['id'], busy_timeout=timeout)
                assert len(adopted)==1
                phase(out, '04-refused-same-admission', agent=queued_agent['id'], attempt=queued_agent['attempt_id'],
                      c_agent=c_id,c_pid=observed_c['child_pid'],adoption=adopted)
                raise
            running = second.ok(['agent', 'show', b_id])['agent']
            acquired, queue = wait_turn('B','acquired')
            active_costs = second.ok(['costs','show',ids['B']])
            restart.verify_same_admission(before, restart.admissions(second, busy_timeout=timeout), queued_agent, running,
                                          costs, active_costs, b, acquired)
            records = restart.read_adoptions(second, b_id, busy_timeout=timeout)
            assert len(records) == 1
            adopted = records[0]['receipt']
            renewed = adopted['execution_preflight']
            assert adopted['schema_version'] == 2
            assert running['preflight'] == original_preflight
            assert renewed['scope'] == original_preflight['scope']
            assert expired_at <= datetime.fromisoformat(renewed['checked_at']) <= datetime.fromisoformat(adopted['recorded_at'])
            assert datetime.fromisoformat(adopted['recorded_at']) < datetime.fromisoformat(renewed['valid_until'])
            phase(out, '04a-renewed-receipt', original=original_preflight, adoption=records[0])
            current = epoch('B')
            assert current['generation'] == 2 and current['pid'] == server_identity['pid']
            assert not current['stopped_at'] and not queue['blockers']
            alive('C')
            c_agent = third.ok(['agent','show',c_id])['agent']
            assert c_agent['status']=='running' and c_agent['child_pid']==active['C']['provider']['pid']
            assert [len(journal(name)) for name in works] == [1, 1, 1]
            phase(out, '04-adopted-same-admission', agent=b_id, attempt=running['attempt_id'], turn=acquired,
                  c_pid=c_agent['child_pid'], c_agent=c_id, epoch=current)
            for name in ('B','C'):
                control(name,'stop','fixture-stop-'+name)
                release(name)
            for name in ('A','B','C'):
                select.select([active[name]['supervisor_fd']],[],[])
                if name == 'C':
                    pid, status = os.waitpid(active[name]['supervisor']['pid'],0)
                    assert pid == active[name]['supervisor']['pid'] and os.waitstatus_to_exitcode(status) == 0
                assert state(name,'released')[0] is not None
                observed = works[name].ok(['agent','show',active[name]['agent_id']])['agent']
                assert observed['status'] == 'completed'
            final = stop_server()
            queue = second.ok(['mission','queue',ids['B']])
            assert len(queue['turns']) == 3 and all(row['state']=='released' for row in queue['turns'])
            assert not queue['blockers']
            money = {name:settlement(name) for name in works}
            assert all(row[1:] == ('estimated',1.0) for row in money.values())
            events = rows(first,'SELECT turn_seq,transition,agent_id FROM workspace_turn_events WHERE work_id=? ORDER BY event_seq',(ids['B'],))
            assert events==[(int(b['seq']),'enqueued',''),(int(b['seq']),'acquired',b_id),(int(b['seq']),'released',b_id)],events
            adoption_table=rows(first,"SELECT name FROM sqlite_master WHERE type='table' AND name='native_mission_worker_adoptions'",())
            adoptions=rows(first,'SELECT agent_id,previous_generation,generation FROM native_mission_worker_adoptions WHERE work_id=?',(ids['B'],)) if adoption_table else []
            assert adoptions==[(b_id,1,2)],adoptions
            restart.verify_terminal_history(before, restart.admissions(second, busy_timeout=timeout), b_id)
            final_works = {name:journal(name) for name in works}
            assert [len(entries) for entries in final_works.values()]==[1,1,1]
            phase(out, '05-persisted-settlement',status='PASS',mode='sdk-stopped-owner',queue=queue,shutdown=final,
                  reservations=money,b_events=events,b_adoptions=adoptions,paid_provider=False)
        except BaseException as error:
            failure = error
            phase(out, 'failure', status='FAIL', cause=''.join(traceback.format_exception(error)))
        finally:
            if server is not None:
                worker.retain_failure(errors,lambda:plain_web.finish(server,failure))
            if failure is not None:
                for name in active:
                    if not (paths[name]/'release-worker.json').exists():
                        worker.retain_failure(errors,lambda name=name:release(name))
                worker.retain_failure(errors,lambda:public.cleanup_owned_children(binary,root,out))
        if failure is not None or errors:
            raise BaseExceptionGroup('Same Root FIFO verification',([failure] if failure else []) + errors)


def main():
    """Run against named binaries and a fresh evidence directory on an owned Root."""
    parser = argparse.ArgumentParser(description=__doc__)
    for argument in ('binary', 'sdk', 'output'):
        parser.add_argument('--' + argument, required=True, type=Path)
    args = parser.parse_args()
    binary, sdk, out = (getattr(args, name).resolve() for name in ('binary', 'sdk', 'output'))
    fixture_base = fixture_area.fixture_area().resolve()
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    inputs = {str(path): fixture.digest(path.read_bytes()) for path in (binary, sdk, Path(__file__).resolve())}
    fixture.publish(out / 'inputs.json', inputs)
    worker.subreaper_scope(lambda: run(binary, sdk, out, fixture_base),
                           lambda error: fixture.publish(out / 'harness-failure.json',
                               dict(status='FAIL', cause=''.join(traceback.format_exception(error)))),
                           parent_death.subreaper)
    assert all(fixture.digest(Path(path).read_bytes()) == digest for path, digest in inputs.items())
    print('PASS expired preflight renewed for the original Agent while C stayed active', flush=True)


if __name__ == '__main__':
    main()
