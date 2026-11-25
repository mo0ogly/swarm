#!/usr/bin/env python3
"""Exercise worker-first parallel admission through the public Rust conductor.

An attached plan has two independent ready workers in separate workspaces,
another worker with an unfinished dependency, and a pending root decision.
At two slots, both programs hold native permission concurrently. At one slot,
the first is held until the second is refused for capacity. No model, live mission or global watchdog is used.
"""
import argparse
from contextlib import ExitStack
import json
import os
from pathlib import Path
import selectors
import select
import signal
import subprocess
import traceback

import rust_agent_model_route as fixture
import rust_mission_watch_public as public
import rust_mission_watch_worker as worker
import rust_native_parent_death as parent_death
import rust_planning_attach as attachment
import rust_planning_worker_public_flow as planning


def watch_records(output):
    """Read complete conductor records from the owned stream; ignore a partial last line."""
    return [json.loads(line) for line in (output / 'watch.stdout').read_bytes().split(b'\n')[:-1]]


def capacity_refusal(output, conductor_fd, notifications, first_agent, waiting):
    """Read the typed refusal for the second worker while the first is held.

    The caller subscribes before spawning the watch. A second helper start
    or conductor exit before release fails the owned fixture.
    """
    while True:
        records = watch_records(output)
        starts = [record['agent_id'] for record in records
                  if record['event'] == 'mission-worker-started']
        assert starts in ([], [first_agent]), starts
        for record in records:
            for refusal in record.get('resource_refusals', []):
                if refusal['task_id'] == waiting and refusal['kind'] == 'capacity':
                    assert starts == [first_agent], starts
                    assert 'mission-agent-slots-occupied' in refusal['causes'], refusal
                    return record
        ready, _, _ = select.select([conductor_fd, notifications], [], [])
        assert conductor_fd not in ready, 'conductor exited before capacity refusal'
        os.read(notifications, 65536)


def run(binary, output, slots=2):
    """Observe real grants at the selected capacity and retain their evidence.

    At one slot, wait for the typed refusal before releasing the first worker.
    At two slots, hold both concurrently. Stop and reap owned processes on exit.
    """
    owned = attachment.run(binary, output / 'attachment', graph=True, return_fixture=True)
    work_id, root = owned.work, owned.root
    before = owned.ok(['work', 'show', work_id])['work']
    pending = planning.pending(before, 'root')
    assert pending, 'attach produced no competing root decision'
    nonce = fixture.digest(str(root).encode())
    program = worker.provider_program(nonce)
    gate_start = program.index('    with socket.socket(')
    gate_end = program.index('        if json.loads(', gate_start)
    program = program[:gate_start] + """    while not Path('release-worker.json').exists():
        try:
            os.read(fd,65536)
        except BlockingIOError:
            import select
            select.select([fd],[],[])
""" + program[gate_end:].replace('        if json.loads(', '    if json.loads(', 1)
    fixture.publish(root / 'codex', program.encode())
    (root / 'codex').chmod(0o700)
    owned.ok(['mission', 'start', work_id], dict(
        schema_version=1, event_id='parallel-watch-start',
        expected_revision=before['revision'], reason='Owned inert capacity witness', slots=slots,
        profile=dict(provider='fixture', role='worker', workspace='workspace-task',
                     level='standard', capture_output=True)))
    expected = {'task': root / 'workspace-task', 'parallel': root / 'workspace-parallel'}
    granted = {}
    released_early = None
    try:
        with ExitStack() as stack:
            selector = stack.enter_context(selectors.DefaultSelector())
            for task, directory in expected.items():
                fd = worker.watch_directory(directory, modifications=True)
                stack.callback(os.close, fd)
                selector.register(fd, selectors.EVENT_READ, task)
            stdout = stack.enter_context(worker.spool(output / 'watch.stdout'))
            stderr = stack.enter_context(worker.spool(output / 'watch.stderr'))
            if slots == 1:
                notifications = worker.watch_directory(output, modifications=True)
                stack.callback(os.close, notifications)
            argv = [str(binary), 'mission', 'watch', work_id, '--root', str(root), '--json']
            fixture.publish(output / 'command.json', dict(argv=argv, global_watchdog=None))
            process = subprocess.Popen(argv, env=owned.environment, stdin=subprocess.DEVNULL,
                                       stdout=stdout, stderr=stderr)
            conductor = worker.observation(process.pid)
            watch_fd = worker.attach(conductor)
            stack.callback(os.close, watch_fd)
            try:
                selector.register(watch_fd, selectors.EVENT_READ, 'conductor')
                while granted.keys() != expected.keys():
                    for key, _ in selector.select():
                        task = key.data
                        assert task != 'conductor', 'conductor exited before both native grants'
                        os.read(key.fd, 65536)
                        marker = expected[task] / 'model-received.json'
                        if not marker.is_file() or not marker.stat().st_size:
                            continue
                        entry = json.loads(marker.read_bytes())
                        selector.unregister(key.fd)
                        provider = worker.observation(entry['pid'])
                        supervisor = worker.observation(provider['ppid'])
                        assert provider['ppid'] == entry['ppid'] and supervisor['ppid'] == conductor['pid']
                        assert fixture.digest(Path(f'/proc/{supervisor["pid"]}/exe').read_bytes()) == fixture.digest(binary.read_bytes())
                        command = Path(f'/proc/{supervisor["pid"]}/cmdline').read_bytes().split(b'\0')
                        assert len(command) == 7 and command[1] == b'_mission_worker_entry'
                        assert command[3:] == [b'--root', os.fsencode(root), b'--json', b'']
                        agent_id = os.fsdecode(command[2])
                        supervisor_fd = worker.attach(supervisor)
                        stack.callback(os.close, supervisor_fd)
                        agent = worker.wait_running(owned, agent_id, watch_fd, supervisor_fd)
                        assert agent['task_id'] == task and task not in granted
                        assert agent['native_execution']['phase'] == 'permission_recorded'
                        assert agent['child_pid'] == provider['pid']
                        assert Path(os.readlink(f'/proc/{provider["pid"]}/cwd')) == expected[task]
                        granted[task] = (supervisor, supervisor_fd, agent_id)
                        if slots == 1 and len(granted) == 1:
                            waiting = next(name for name in expected if name != task)
                            refusal = capacity_refusal(output, watch_fd, notifications,
                                                       agent_id, waiting)
                            status = public.status_native(owned)
                            assert {item['id'] for item in status['agents']} == {agent_id}
                            assert not (expected[waiting] / 'model-received.json').exists()
                            fixture.publish(output / 'capacity-refusal.json', dict(
                                refusal=refusal, native_status=status, running_task=task))
                            fixture.publish(expected[task] / 'release-worker.json', dict(
                                nonce=nonce, release=True))
                            select.select([supervisor_fd], [], [])
                            released_early = task
                            assert owned.ok(['agent', 'show', agent_id])['agent']['status'] == 'completed'
                current = owned.ok(['work', 'show', work_id])['work']
                assert planning.pending(current, 'root') == pending
                assert current['planning'].get('activations', 0) == before['planning'].get('activations', 0)
                active = public.status_native(owned)
                running = {item['task']: item['id'] for item in active['agents']
                           if item['status'] == 'running'}
                expected_running = {name: observed[2] for name, observed in granted.items()
                                    if name != released_early}
                assert running == expected_running, active
                assert {item['task'] for item in active['agents']} <= expected.keys()
                fixture.publish(output / ('active-status.json' if slots == 1 else
                                          'simultaneous-status.json'), active)
                public.mission_control(owned, 'pause', 'parallel-watch-pause')
            finally:
                if process.poll() is None:
                    signal.pidfd_send_signal(watch_fd, signal.SIGINT)
                code = process.wait()
                fixture.publish(output / 'exit.json', dict(exit=code))
                for directory in expected.values():
                    fixture.publish(directory / 'release-worker.json', dict(nonce=nonce, release=True))
                for task, (supervisor, supervisor_fd, _) in granted.items():
                    if task == released_early:
                        continue
                    select.select([supervisor_fd], [], [])
                    waited, status = os.waitpid(supervisor['pid'], 0)
                    assert waited == supervisor['pid'] and os.waitstatus_to_exitcode(status) == 0
            assert code == 0, 'conductor failed during worker observation'
        if slots == 1:
            first, second = (value[2] for value in granted.values())
            timeline = [(record['event'], record['agent_id'], record.get('exit_code'))
                        for record in watch_records(output)
                        if record['event'] in ('mission-worker-started', 'mission-worker-exited')]
            assert timeline == [('mission-worker-started', first, None),
                                ('mission-worker-exited', first, 0),
                                ('mission-worker-started', second, None)], timeline
            fixture.publish(output / 'single-slot-timeline.json', timeline)
        assert owned.ok(['mission', 'show', work_id])['paused'] is True
        for _, _, agent_id in granted.values():
            assert owned.ok(['agent', 'show', agent_id])['agent']['status'] == 'completed'
        public.mission_control(owned, 'resume', 'parallel-watch-resume')
        assert owned.ok(['mission', 'show', work_id])['paused'] is False
        public.mission_control(owned, 'stop', 'parallel-watch-revoke')
        revoked = owned.ok(['mission', 'show', work_id])
        assert revoked['paused'] is True and revoked['consent_enabled'] is False
    finally:
        public.cleanup_owned_children(binary, root, output)
    return dict(status='PASS', slots=slots, grant_order=list(granted),
                simultaneous_workers=sorted(granted) if slots == 2 else [],
                capacity_refusal_verified=slots == 1, dependent_worker_not_admitted=True,
                root_activation_unchanged=True, paused=True, consent_revoked=True,
                paid_provider=False)


def main():
    """Bind the real CLI to its qualified profile and retain failures and outcomes."""
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--slots', type=int, choices=(1, 2), default=2)
    args = parser.parse_args()
    binary, output = args.binary.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    profile = json.loads(args.profile.read_bytes())
    assert profile['no_fixture_feature'] and profile['sha256'] == fixture.digest(binary.read_bytes())
    fixture.publish(output / 'profile.json', profile)
    result = worker.subreaper_scope(
        lambda: run(binary, output, args.slots),
        lambda error: fixture.publish(output / 'failure.json', dict(
            status='FAIL', cause=''.join(traceback.format_exception(error)))),
        parent_death.subreaper)
    fixture.publish(output / 'result.json', result)
    if args.slots == 1:
        print('PASS one slot defers the independent worker until the first completes', flush=True)
    else:
        print('PASS two isolated inert workers run before competing root planning', flush=True)
    print('PASS public mission preserves pause, resume and consent revocation', flush=True)


if __name__ == '__main__':
    main()
