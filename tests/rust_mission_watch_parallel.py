#!/usr/bin/env python3
"""Exercise worker-first parallel admission through the public Rust conductor.

An attached plan has two independent ready workers in separate workspaces,
another worker with an unfinished dependency, and a pending root decision.
The two inert programs wait for local release files until both have real native
execution permission. No model, live mission or global watchdog is used.
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


def run(binary, output):
    """Require two concurrent process grants and preserve the pending root event."""
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
        expected_revision=before['revision'], reason='Owned two-slot inert witness', slots=2,
        profile=dict(provider='fixture', role='worker', workspace='workspace-task',
                     level='standard', capture_output=True)))
    expected = {'task': root / 'workspace-task', 'parallel': root / 'workspace-parallel'}
    granted = {}
    try:
        with ExitStack() as stack:
            selector = stack.enter_context(selectors.DefaultSelector())
            for task, directory in expected.items():
                fd = worker.watch_directory(directory, modifications=True)
                stack.callback(os.close, fd)
                selector.register(fd, selectors.EVENT_READ, task)
            stdout = stack.enter_context(worker.spool(output / 'watch.stdout'))
            stderr = stack.enter_context(worker.spool(output / 'watch.stderr'))
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
                current = owned.ok(['work', 'show', work_id])['work']
                assert planning.pending(current, 'root') == pending
                assert current['planning'].get('activations', 0) == before['planning'].get('activations', 0)
                active = public.status_native(owned)
                running = {item['task']: item['id'] for item in active['agents']
                           if item['status'] == 'running'}
                assert running == {task: observed[2] for task, observed in granted.items()}, active
                fixture.publish(output / 'simultaneous-status.json', active)
                public.mission_control(owned, 'pause', 'parallel-watch-pause')
            finally:
                if process.poll() is None:
                    signal.pidfd_send_signal(watch_fd, signal.SIGINT)
                code = process.wait()
                fixture.publish(output / 'exit.json', dict(exit=code))
                for directory in expected.values():
                    fixture.publish(directory / 'release-worker.json', dict(nonce=nonce, release=True))
                for supervisor, supervisor_fd, _ in granted.values():
                    select.select([supervisor_fd], [], [])
                    waited, status = os.waitpid(supervisor['pid'], 0)
                    assert waited == supervisor['pid'] and os.waitstatus_to_exitcode(status) == 0
            assert code == 0, 'conductor failed during two-worker observation'
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
    return dict(status='PASS', simultaneous_workers=sorted(granted), dependency_waited=True,
                root_activation_unchanged=True, paused=True, consent_revoked=True,
                paid_provider=False)


def main():
    """Bind the real CLI to its qualified profile and retain failures and outcomes."""
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    binary, output = args.binary.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    profile = json.loads(args.profile.read_bytes())
    assert profile['no_fixture_feature'] and profile['sha256'] == fixture.digest(binary.read_bytes())
    fixture.publish(output / 'profile.json', profile)
    result = worker.subreaper_scope(
        lambda: run(binary, output),
        lambda error: fixture.publish(output / 'failure.json', dict(
            status='FAIL', cause=''.join(traceback.format_exception(error)))),
        parent_death.subreaper)
    fixture.publish(output / 'result.json', result)
    print('PASS two isolated inert workers run before competing root planning', flush=True)
    print('PASS public mission preserves pause, resume and consent revocation', flush=True)


if __name__ == '__main__':
    main()
