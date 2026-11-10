#!/usr/bin/env python3
"""Public native watch/status journey with explicitly owned inert providers.

Uses real public planning/start/watch/status-native/agent/completion operations.
SIGINT/SIGTERM/pause act on the owned conductor only; actual independently granted
workers must survive and complete. No Go, paid provider, live DB mutation,
installation, arbitrary watchdog or automatic acceptance. The native status/JSONL
schemas are Rust extensions, not claims of legacy Go JSON parity.
"""
import argparse
import json
import os
from pathlib import Path
import select
import selectors
import signal
import socket
import struct
import subprocess
import sys
import traceback

import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as planning
import rust_mission_watch_worker as worker
import rust_workspace_fifo_completion as completion
import rust_native_parent_death as parent_death

check = worker.check


def mission_control(owned, operation, event):
    """Apply public pause/stop against the latest Work revision; preserve all history."""
    revision = owned.ok(['work', 'show', owned.work])['work']['revision']
    return owned.ok(['mission', operation, owned.work], dict(schema_version=1,
        event_id=event, expected_revision=revision, reason='Owned public watch fixture'))


def status_native(owned, agent_id=None):
    """Require the explicitly native status identity and any supplied observed Agent.

    This is observation only: no liveness or execution authority is inferred from
    JSON. Process identities are checked separately against actual kernel peers.
    """
    value = owned.ok(['mission', 'status-native', owned.work])
    check(type(value) is dict and set(value) == {'schema_version', 'work', 'revision', 'controls', 'supervision', 'agents', 'limitations'}
          and type(value.get('schema_version')) is int
          and value['schema_version'] == 1 and value.get('work') == owned.work,
          'native status identity/schema mismatch')
    check(type(value.get('revision')) is int and value['revision'] > 0,
          'native status revision missing')
    controls = value.get('controls')
    check(type(controls) is dict and set(controls) == {'revision', 'paused', 'autonomy', 'slots', 'consent_enabled', 'archived'},
          'native status controls shape invalid')
    check(type(controls['revision']) is int and controls['revision'] > 0
          and type(controls['archived']) is bool,
          'native status controls scalar invalid')
    check(all(controls[key] is None or type(controls[key]) is bool for key in ('paused', 'consent_enabled'))
          and (controls['slots'] is None or type(controls['slots']) is int)
          and (controls['autonomy'] is None or type(controls['autonomy']) is str),
          'native status optional controls type invalid')
    supervision = value.get('supervision')
    if supervision is not None:
        check(type(supervision) is dict and set(supervision) == {'state', 'generation', 'last_check_at', 'heartbeat_observed_at', 'last_check_failed'},
              'native status supervision shape invalid')
        check(supervision['state'] in {'legacy_unverified', 'stopped', 'present', 'cycle_error', 'stale', 'future'}
              and (supervision['generation'] is None or type(supervision['generation']) is int)
              and type(supervision['last_check_at']) is str and type(supervision['heartbeat_observed_at']) is str
              and type(supervision['last_check_failed']) is bool,
              'native status supervision scalar invalid')
    check(type(value.get('limitations')) is list and bool(value['limitations'])
          and all(type(item) is str and bool(item) for item in value['limitations']),
          'native status observation limits missing')
    check(type(value.get('agents')) is list, 'native status agents missing')
    for agent in value['agents']:
        check(type(agent) is dict and set(agent) == {'id', 'task', 'status', 'desired', 'supervisor_pid', 'child_pid'}
              and all(type(agent[key]) is str for key in ('id', 'task', 'status', 'desired'))
              and all(type(agent[key]) is int for key in ('supervisor_pid', 'child_pid')),
              'native status Agent record shape/types invalid')
    if agent_id is not None:
        check(sum(agent.get('id') == agent_id for agent in value['agents']) == 1,
              'native status target Agent missing or duplicated')
    return value


def idle_planning_record(event, work):
    """Validate the exact native idle record for an already-planned fixture.

    An idle pass has no selected scope/input/claim, no applied decision and no
    observation authority. SettingCount is canonical positive decimal text;
    Work revision is a positive integer. Unexpected keys, applied planning or
    malformed fields refuse safely without exposing raw event data or granting
    any operation. This permits observation only, not another model call.
    """
    fields = {'schema_version','event','work','status','selected_scope','input_event','claim_id',
              'settings_revision','decision_applied','work_revision','mission_complete','automatic_planner',
              'automatic_reviewer','automatic_validation','observation_is_permission'}
    check(type(event) is dict and set(event) == fields, 'public idle planning record shape invalid')
    setting = event['settings_revision']
    check(type(event['schema_version']) is int and event['schema_version'] == 1
          and event['event'] == 'mission-planning-step' and event['work'] == work and event['status'] == 'idle'
          and all(event[key] is None for key in ('selected_scope','input_event','claim_id'))
          and type(setting) is str and bool(setting) and setting[0] in '123456789'
          and all('0' <= char <= '9' for char in setting)
          and type(event['work_revision']) is int and event['work_revision'] > 0
          and event['automatic_planner'] is True
          and all(event[key] is False for key in ('decision_applied','mission_complete','automatic_reviewer',
                  'automatic_validation','observation_is_permission')), 'public idle planning record contract invalid')


def public_watch_events(raw, work, agent_id, reason):
    """Validate the native public JSONL departure contract against the actual worker.

    Exactly one matching worker was granted in this fixture, then remained live
    when watch closed; no exited event or completion claim is accepted. This
    validates public statements, while kernel/socket/receipt checks prove effects.
    Already-planned fixtures may emit exact idle planning observations. Applied
    planning remains unexpected here; W3 validates its real claims separately.
    """
    events = [json.loads(line) for line in raw.splitlines() if line.strip()]
    check(bool(events) and all(type(item) is dict for item in events),
          'public watch JSONL empty or malformed')
    known = {'mission-watch-started', 'mission-worker-started', 'mission-worker-exited',
             'mission-watch-blocked', 'mission-watch-stopped', 'mission-planning-step'}
    for event in events:
        check(type(event.get('schema_version')) is int and event['schema_version'] == 1,
              'public watch event schema mismatch')
        check(event.get('work') == work and event.get('event') in known,
              'public watch event Work/name mismatch')
        if event['event'] == 'mission-planning-step':
            idle_planning_record(event,work)
        if event['event'] != 'mission-worker-exited':
            check(event.get('mission_complete') is False, 'public watch overclaimed or omitted mission completion limit')
    check(events[0]['event'] == 'mission-watch-started' and
          sum(event['event'] == 'mission-watch-started' for event in events) == 1,
          'public watch start event missing or duplicated')
    check(events[0].get('observation_is_permission') is False,
          'public watch start overclaimed observation authority')
    started = [event for event in events if event['event'] == 'mission-worker-started']
    check(len(started) == 1 and started[0].get('agent_id') == agent_id
          and started[0].get('session_state') == 'pending' and started[0].get('provider_start_verified') is False,
          'public watch worker admission event mismatches actual target')
    check(not any(event['event'] == 'mission-worker-exited' for event in events),
          'public watch claimed exit before blocked provider release')
    stopped = events[-1]
    check(stopped['event'] == 'mission-watch-stopped' and stopped.get('reason') == reason
          and stopped.get('lease_released') is True and stopped.get('root_closed') is True,
          'public watch final cleanup event mismatch')
    check(type(stopped.get('children_still_running')) is int and stopped['children_still_running'] == 1,
          'public watch running child count mismatch')
    return events


def canonical_desired(agent):
    """Decode only Agent's documented omitted-empty-string representation.

    Agent.desired uses serde skip_serializing_if String::is_empty. Absence means
    the stored empty string; present strings are preserved exactly. Null, numbers
    and booleans are invalid, never policy defaults or successful empty values.
    """
    value = agent['desired'] if 'desired' in agent else ''
    check(type(value) is str, 'canonical Agent desired must be a string or omitted-empty')
    return value


def reap_known_provider(identity, descriptor, out):
    """Clean up the already-authenticated peer using only its held pidfd/stamp.

    Call after the owned supervisor was actually reaped. WNOHANG first proves
    current child ownership. A still-live adopted peer must also have this parent
    and its original stamp before pidfd SIGTERM; unknown PIDs are never signalled.
    If its supervisor already reaped it, require the held pidfd to show exit.
    Preserve failures; this failure-only cleanup is not product recovery evidence.
    """
    try:
        waited, status = os.waitpid(identity['pid'], os.WNOHANG)
    except ChildProcessError:
        check(bool(select.select([descriptor], [], [], 0)[0]),
              'known provider still live without adopted child ownership')
        fixture.publish(out / 'cleanup-provider.json', dict(pid=identity['pid'],
            pidfd_exit_observed=True, reaped_by_this_harness=False, cleanup_only=True))
        return
    if waited == 0:
        if not select.select([descriptor], [], [], 0)[0]:
            current = worker.observation(identity['pid'], allow_terminal=True)
            check(current['stamp'] == identity['stamp'] and current['ppid'] == os.getpid(),
                  'known adopted provider identity or parent changed')
            if current['state'] not in ('Z', 'X'):
                signal.pidfd_send_signal(descriptor, signal.SIGTERM)
        waited, status = os.waitpid(identity['pid'], 0)
    check(waited == identity['pid'], 'known provider waitpid target mismatch')
    fixture.publish(out / 'cleanup-provider.json', dict(pid=waited, raw_wait_status=status,
        reaped_by_this_harness=True, cleanup_only=True))


def attach_provider(connection, conductor, binary, root):
    """Bind kernel peer to this owned conductor's actual normal helper child.

    Exact argv/executable, UID, start stamps, parentage and public Agent identity
    are all checked; peer-supplied JSON cannot authorize an arbitrary PID signal.
    Return supervisor/provider identities, pidfds and real helper Agent id.
    """
    peer, uid, _ = struct.unpack('3i', connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
    check(uid == os.getuid(), 'owned public provider UID mismatch')
    provider = worker.observation(peer)
    supervisor = worker.observation(provider['ppid'])
    check(supervisor['ppid'] == conductor['pid'], 'public supervisor not owned conductor child')
    args = Path('/proc', str(supervisor['pid']), 'cmdline').read_bytes().split(b'\0')[:-1]
    check(len(args) == 6 and args[:2] == [os.fsencode(binary), b'_mission_worker_entry']
          and args[3:] == [b'--root', os.fsencode(root), b'--json'],
          'public supervisor argv differs from helper/Root')
    agent_id = os.fsdecode(args[2])
    check(fixture.digest(Path('/proc', str(supervisor['pid']), 'exe').read_bytes())
          == fixture.digest(binary.read_bytes()), 'public supervisor binary mismatch')
    supervisor_fd = worker.attach(supervisor)
    try:
        actual_provider, provider_fd = worker.owned_provider_peer(connection, supervisor)
    except BaseException as primary:
        worker.close_descriptor(supervisor_fd, primary)
        raise
    return supervisor, supervisor_fd, actual_provider, provider_fd, agent_id


def reap_cleanup_helper(identity):
    """Reap a verified adopted helper, tolerating its live→zombie exit race.

    Caller must first verify actual parentage, exact CLI/Root argv and executable.
    A pidfd plus repeated stamp/parent checks binds a possible signal; terminal
    state is reaped without signalling. Only ESRCH with readable held pidfd may
    become an observed exit. Other errors and descriptor-close causes propagate.
    No retry, timeout, unknown-PID signalling or provider permission follows.
    """
    pid = identity['pid']
    waited, status = os.waitpid(pid, os.WNOHANG)
    if waited == pid:
        return dict(pid=waited,status=status,role='owned-helper-already-terminal')
    check(waited == 0, 'unexpected cleanup helper wait identity')
    before = worker.observation(pid, allow_terminal=True)
    check(before['stamp'] == identity['stamp'] and before['ppid'] == os.getpid(),
          'cleanup helper ownership changed before pidfd')
    descriptor = os.pidfd_open(pid)
    try:
        after = worker.observation(pid, allow_terminal=True)
        check(after['stamp'] == identity['stamp'] and after['ppid'] == os.getpid(),
              'cleanup helper ownership changed after pidfd')
        sent = False
        if after['state'] not in ('Z','X') and not select.select([descriptor],[],[],0)[0]:
            try:
                signal.pidfd_send_signal(descriptor, signal.SIGTERM)
                sent = True
            except ProcessLookupError:
                check(bool(select.select([descriptor],[],[],0)[0]), 'cleanup helper ESRCH without observed exit')
        waited, status = os.waitpid(pid, 0)
        check(waited == pid, 'cleanup helper reap identity mismatch')
        return dict(pid=waited,status=status,role='owned-helper-cleanup-only',signal_sent=sent)
    finally:
        worker.close_descriptor(descriptor, sys.exception())


def cleanup_owned_children(binary, root, out):
    """Reap only adopted exact helper children from this owned Root on failure.

    A child must be an actual kernel child, match exact binary and Root argv and
    retain its stamp around pidfd attachment. Unknown children are never signalled.
    This is failure cleanup, not evidence of successful product recovery.
    """
    children = Path('/proc', str(os.getpid()), 'task', str(os.getpid()), 'children').read_text().split()
    records = []
    for raw_pid in children:
        pid = int(raw_pid)
        current = worker.observation(pid, allow_terminal=True)
        if current['state'] == 'Z':
            waited, status = os.waitpid(pid, 0)
            records.append(dict(pid=waited, status=status, role='unknown-owned-zombie'))
            continue
        argv = Path('/proc', str(pid), 'cmdline').read_bytes().split(b'\0')[:-1]
        expected = (len(argv) == 6 and argv[:2] == [os.fsencode(binary), b'_mission_worker_entry']
                    and argv[3:] == [b'--root', os.fsencode(root), b'--json'])
        if not expected:
            records.append(dict(pid=pid, role='unknown-live-not-signalled'))
            continue
        check(current['ppid'] == os.getpid() and fixture.digest(Path('/proc', str(pid), 'exe').read_bytes())
              == fixture.digest(binary.read_bytes()), 'adopted helper ownership mismatch')
        records.append(reap_cleanup_helper(current))
    fixture.publish(out / 'cleanup-children.json', records)
    check(not any(item['role'] == 'unknown-live-not-signalled' for item in records),
          'unknown adopted live child remains; not signalled')


def run_case(binary, out, mode, while_running=None):
    """Run one genuine public watch departure; require actual independent completion.

    Each case receives a fresh public planned Root and explicit fixture-only
    profile/budget. Failure retains originals and attempts owned cleanup without
    converting a refusal, publication failure or uncertain effect into PASS.
    An optional owned-fixture callback runs after watch exit and verified worker
    survival, before release. It may exercise a resumed public watch; exceptions
    flow through this fixture's existing worker cleanup. It grants no process
    authority and must close its own handles before returning or raising.
    Real provider entry precedes parent publication: the maintained running-state
    barrier observes actual DB progress before requiring canonical running.
    """
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    ready, owned = planning.flow(binary, out / 'planning', before_worker=True, return_fixture=True)
    root = owned.root
    nonce = fixture.digest((mode + str(root)).encode())
    fixture.publish(root / 'codex', worker.provider_program(nonce).encode())
    (root / 'codex').chmod(0o700)
    (root / 'watch-worker').mkdir(mode=0o700)
    owned.ok(['mission', 'start', owned.work], dict(schema_version=1, event_id='public-watch-start',
        expected_revision=ready['revision'], reason='Owned public conductor fixture', slots=1,
        profile=dict(provider='fixture', role='worker', workspace='watch-worker', level='standard', capture_output=True)))
    initial = status_native(owned)
    fixture.publish(out / 'native-status-before.json', initial)
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    process = connection = None
    watch_fd = supervisor_fd = provider_fd = None
    supervisor = provider = None
    primary = None
    errors = []
    channels = []
    supervisor_reaped = False
    result = None
    try:
        listener.bind(str(root / 'watch-provider.sock'))
        os.chmod(root / 'watch-provider.sock', 0o600)
        listener.listen(1)
        stdout, stderr = worker.spool(out / 'watch.stdout'), worker.spool(out / 'watch.stderr')
        channels.extend((stdout, stderr))
        argv = [str(binary), 'mission', 'watch', owned.work, '--root', str(root), '--json']
        fixture.publish(out / 'watch-command.json', dict(argv=argv, mode=mode, global_watchdog=None))
        process = subprocess.Popen(argv, env=owned.environment, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
        conductor = worker.observation(process.pid)
        watch_fd = worker.attach(conductor)
        with selectors.DefaultSelector() as selector:
            selector.register(listener, selectors.EVENT_READ, 'provider')
            selector.register(watch_fd, selectors.EVENT_READ, 'conductor')
            while connection is None:
                for key, _ in selector.select():
                    check(key.data == 'provider', 'public watch exited before provider entry')
                    connection, _ = listener.accept()
        supervisor, supervisor_fd, provider, provider_fd, agent_id = attach_provider(connection, conductor, binary, root)
        entry = worker.frame(connection)
        check(entry['nonce'] == nonce and entry['pid'] == provider['pid']
              and entry['ppid'] == supervisor['pid'], 'public provider nonce/kernel identity mismatch')
        supervisor = worker.supervisor_session(supervisor, conductor, True)
        agent = worker.wait_running(owned, agent_id, watch_fd, supervisor_fd)
        check(agent['status'] == 'running' and agent['supervisor_pid'] == supervisor['pid']
              and agent['child_pid'] == provider['pid'] and agent['native_execution']['phase'] == 'permission_recorded',
              'public watch worker lacks actual registered grant')
        worker.verify_current_root_binding(agent['native_mission'], root)
        active_status = status_native(owned, agent_id)
        observed_agent = next(value for value in active_status['agents'] if value['id'] == agent_id)
        check(observed_agent == dict(id=agent_id, task=agent['task_id'], status=agent['status'], desired=canonical_desired(agent),
              supervisor_pid=supervisor['pid'], child_pid=provider['pid']),
              'native status Agent differs from actual canonical Agent/kernel registration')
        fixture.publish(out / 'native-status-active.json', active_status)
        turn = completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, agent_id)
        reservation = completion.reservation(owned, agent_id)
        check(turn['state'] == 'acquired' and reservation == ['reserved', 1.0],
              'public watch worker resources not held')
        lease = worker.supervision(owned)
        if mode == 'pause':
            mission_control(owned, 'pause', 'public-watch-pause')
            check(owned.ok(['mission', 'show', owned.work])['paused'] is True,
                  'public pause not persisted')
        check(mode in ('pause', 'sigint', 'sigterm'), 'unknown fixture stop case')
        stop_reason = 'sigterm' if mode == 'sigterm' else 'sigint'
        signal.pidfd_send_signal(watch_fd, signal.SIGTERM if stop_reason == 'sigterm' else signal.SIGINT)
        code = process.wait()
        check(code == 0, 'public watch did not close successfully')
        fixture.publish(out / 'watch-exit.json', dict(exit=code, mode=mode, kernel_wait=True))
        current = worker.observation(supervisor['pid'])
        check(current['stamp'] == supervisor['stamp'] and current['ppid'] == os.getpid(),
              'public granted worker not independently adopted after watch exit')
        check(worker.observation(provider['pid'])['stamp'] == provider['stamp'],
              'public provider departed with conductor')
        connection.sendall(b'Q')
        survived = worker.frame(connection)
        check(survived['nonce'] == nonce and survived['pid'] == provider['pid'],
              'provider did not reply after public watch reaped')
        check(completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, agent_id) == turn
              and completion.reservation(owned, agent_id) == reservation,
              'public watch exit released active worker resources')
        fixture.publish(out / 'survival.json', dict(provider=survived, supervisor=current, turn=turn, reservation=reservation))
        resumed = while_running(binary, owned, out / 'resumed-watch', agent_id) if while_running is not None else None
        fixture.publish(root / 'watch-worker/release-worker.json', dict(nonce=nonce, release=True))
        select.select([supervisor_fd], [], [])
        waited, status = os.waitpid(supervisor['pid'], 0)
        supervisor_reaped = True
        check(waited == supervisor['pid'] and os.WIFEXITED(status) and os.WEXITSTATUS(status) == 0,
              'actual public worker helper did not reap with exit0')
        final = owned.ok(['agent', 'show', agent_id])
        check(final['agent']['status'] == 'completed', 'public worker completion not recorded')
        observed_work = owned.ok(['work', 'show', owned.work])['work']
        observed_task = next(task for task in observed_work['tasks'] if task['id'] == 'task')
        check(observed_task['status'] == 'blocked', 'public completion unexpectedly accepted task')
        # This local adapter derives nonacceptance from the actual public Work;
        # agent.show does not claim to supply the agent.run completion DTO.
        receipt = worker.retain_completion(owned, out,
            dict(agent=final['agent'], task_accepted=observed_task['status'] != 'blocked'),
            agent, supervisor, provider)
        after_turn = completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, agent_id)
        check(after_turn['seq'] == turn['seq'] and after_turn['state'] == 'released'
              and completion.reservation(owned, agent_id) == ['estimated', 1.0],
              'public completion did not release exact worker resources')
        final_status = status_native(owned, agent_id)
        fixture.publish(out / 'native-status-after.json', final_status)
        mission_control(owned, 'stop', 'public-watch-final-stop')
        control = owned.ok(['mission', 'show', owned.work])
        check(control['paused'] is True and control['consent_enabled'] is False,
              'public mission stop did not pause and revoke consent')
        check(next(task for task in owned.ok(['work', 'show', owned.work])['work']['tasks'] if task['id'] == 'task')['status'] == 'blocked',
              'provider completion accepted task')
        events = public_watch_events((out / 'watch.stdout').read_bytes(), owned.work, agent_id, stop_reason)
        result = dict(status='PASS', mode=mode, root=str(root), work=owned.work, agent=agent_id,
            public_cli=True, independent_worker_survived=True, actual_provider_entry=True,
            supervisor_waitpid_exit=0, completion_receipt=receipt, watch_events=events,
            task_accepted=False, paid_provider=False, installed=False, global_watchdog=None,
            queued_preexisting_public_restart='NOT TESTED; prior engine continuation evidence remains separate',
            status_legacy_json_parity=False, native_status_extension=True,
            original_lease=lease)
        if resumed is not None:
            result['resumed_registered_observation'] = resumed
    except BaseException as error:
        primary = error
        worker.retain_failure(errors, lambda: fixture.publish(out / 'failure.json',
            dict(status='FAIL', cause=''.join(traceback.format_exception(error)), mode=mode, root=str(root))))
    finally:
        if primary is not None:
            if process is not None and process.poll() is None:
                try:
                    check(watch_fd is not None and worker.observation(process.pid)['stamp'] == conductor['stamp'],
                          'cleanup watch ownership unknown')
                    signal.pidfd_send_signal(watch_fd, signal.SIGTERM)
                    process.wait()
                except BaseException as error:
                    errors.append(error)
            try:
                release = root / 'watch-worker/release-worker.json'
                if not release.exists():
                    fixture.publish(release, dict(nonce=nonce, release=True))
                if supervisor is not None and not supervisor_reaped:
                    if not select.select([supervisor_fd], [], [], 0)[0]:
                        check(worker.observation(supervisor['pid'])['stamp'] == supervisor['stamp'],
                              'cleanup supervisor identity changed')
                        signal.pidfd_send_signal(supervisor_fd, signal.SIGTERM)
                    os.waitpid(supervisor['pid'], 0)
                    supervisor_reaped = True
            except BaseException as error:
                errors.append(error)
            if provider is not None and provider_fd is not None and supervisor_reaped:
                try:
                    reap_known_provider(provider, provider_fd, out)
                except BaseException as error:
                    errors.append(error)
            try:
                cleanup_owned_children(binary, root, out)
            except BaseException as error:
                errors.append(error)
        for item in (connection, listener):
            if item is not None:
                try:
                    item.close()
                except BaseException as error:
                    errors.append(error)
        for stream in channels:
            for operation in (stream.flush, lambda stream=stream: os.fsync(stream.fileno()), stream.close):
                try:
                    operation()
                except BaseException as error:
                    errors.append(error)
        for descriptor in (watch_fd, supervisor_fd, provider_fd):
            if descriptor is not None:
                try:
                    os.close(descriptor)
                except BaseException as error:
                    errors.append(error)
    if primary is not None or errors:
        raise BaseExceptionGroup('public watch fixture observation/cleanup', ([primary] if primary else []) + errors)
    fixture.publish(out / 'result.json', result)
    return result


def main():
    """Verify the normal binary profile, run fresh cases, restore subreaper state.

    No SDK binary is used: the tested public CLI owns conductor admission/spawn.
    Default nonfixture binary/source hashes are checked before and after all cases.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    binary, out = args.binary.resolve(), args.output.resolve()
    out.mkdir(parents=True, mode=0o700, exist_ok=False)
    profile = json.loads(args.profile.read_bytes())
    check(profile['no_fixture_feature'] is True and profile['sha256'] == fixture.digest(binary.read_bytes()),
          'public watch normal profile/hash mismatch')
    fixture.publish(out / 'method.py', Path(__file__).read_bytes())
    fixture.publish(out / 'profile.json', profile)
    sources = {str(Path(module.__file__).resolve()): fixture.digest(Path(module.__file__).read_bytes())
               for name, module in tuple(sys.modules.items()) if name.startswith('rust_') and getattr(module, '__file__', None)}
    sources[str(Path(__file__).resolve())] = fixture.digest(Path(__file__).read_bytes())
    fixture.publish(out / 'inputs.json', dict(sources=sources, binary_sha256=profile['sha256']))
    def cases():
        """Run valid paused witness before signal variants on equivalent fresh Roots."""
        values = []
        for mode in ('pause', 'sigint', 'sigterm'):
            values.append(run_case(binary, out / mode, mode))
            print('PASS public mission watch ' + mode + ' preserves independently granted worker completion', flush=True)
        check(all(fixture.digest(Path(path).read_bytes()) == digest for path, digest in sources.items()),
              'public watch harness sources changed')
        check(profile['sha256'] == fixture.digest(binary.read_bytes()), 'public watch binary changed')
        return dict(status='PASS', cases=values, actual_public_watch=True, native_status_extension=True,
                    legacy_status_json_parity=False, source_stable=True, paid_provider=False,
                    installed=False, task_accepted=False, global_watchdog=None)
    result = worker.subreaper_scope(cases,
        lambda error: fixture.publish(out / 'failure.json', dict(status='FAIL', cause=''.join(traceback.format_exception(error)))),
        parent_death.subreaper)
    fixture.publish(out / 'result.json', result)
    print('REPLAY_EVIDENCE ' + str(out), flush=True)


if __name__ == '__main__':
    main()
