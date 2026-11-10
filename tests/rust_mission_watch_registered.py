#!/usr/bin/env python3
"""Resume an actual public watch while its previously granted worker stays live.

The maintained fixture owns actual provider/kernel handles, exits the first watch,
checks worker survival, calls this observer, then releases and reaps that worker.
Only the new observer's own Popen/pidfd may be stopped here. No SQL writes, live
provider, implicit worker adoption, new consent, retry or global timeout.
"""
import argparse
import json
import os
import select
import signal
import subprocess
import traceback
from pathlib import Path

import rust_agent_model_route as fixture
import rust_mission_report_relay as relay
import rust_mission_watch_public as public
import rust_mission_watch_worker as worker
import rust_native_parent_death as parent_death
import rust_workspace_fifo_completion as completion
check = worker.check


def records(path):
    """Read complete flushed JSONL records; leave an incomplete suffix for later."""
    return [json.loads(line) for line in path.read_bytes().splitlines(keepends=True) if line.endswith(b'\n')]


def observe(binary, owned, out, agent):
    """Exercise a resumed observer while the authenticated fixture keeps its worker held.

    Stable canonical resources are compared before/after; advancing heartbeats are
    allowed. A refusal is not success. Both primary and own-watch cleanup failures
    survive, including partially opened spools or pidfd acquisition failure. Only
    this test's own unreaped Popen child is then signalled and waited; original
    workers remain owned by the surrounding fixture, never by stored PID labels.
    """
    out.mkdir(mode=0o700)
    before = owned.ok(['agent', 'show', agent])['agent']
    turn = completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, agent)
    money = completion.reservation(owned, agent)
    finance = relay.rows(owned, 'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id', (owned.work,))
    notify = process = descriptor = None
    stdout = stderr = None
    errors = []
    events = None
    try:
        notify = worker.watch_directory(out, modifications=True)
        stdout = worker.spool(out / 'watch.stdout')
        stderr = worker.spool(out / 'watch.stderr')
        argv = [str(binary), 'mission', 'watch', owned.work, '--root', str(owned.root), '--json']
        fixture.publish(out / 'argv.json', dict(argv=argv, global_watchdog=None))
        process = subprocess.Popen(argv, env=owned.environment, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
        descriptor = os.pidfd_open(process.pid)
        while True:
            events = records(out / 'watch.stdout')
            if any(e.get('event') == 'mission-watch-blocked' and e.get('synchronization_committed') is True for e in events):
                break
            ready, _, _ = select.select([notify, descriptor], [], [])
            check(descriptor not in ready, 'resumed watch refused actual registered worker before observation cycle')
            if notify in ready:
                os.read(notify, 65536)
        check(any(e.get('event') == 'mission-watch-blocked' and e.get('reason') == 'deferred-active-worker-context' for e in events), 'resumed watch did not defer planning for registered context')
        check(not any(e.get('event') in ['mission-worker-started', 'mission-worker-adopted'] for e in events), 'resumed observer granted or adopted existing worker')
        signal.pidfd_send_signal(descriptor, signal.SIGINT)
        check(process.wait() == 0, 'resumed watch stop failed')
        after = owned.ok(['agent', 'show', agent])['agent']
        keys = ['id', 'status', 'supervisor_pid', 'child_pid', 'native_execution']
        check({k: before[k] for k in keys} == {k: after[k] for k in keys}, 'resumed observer changed registered worker lifecycle')
        check(public.canonical_desired(before) == public.canonical_desired(after), 'resumed observer changed desired state')
        check(completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, agent) == turn, 'resumed observer changed active workspace turn')
        check(completion.reservation(owned, agent) == money, 'resumed observer changed active reservation')
        check(relay.rows(owned, 'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id', (owned.work,)) == finance, 'resumed observer created or reset model funding')
        status = public.status_native(owned, agent)
        check(len(status['agents']) == 1 and status['controls']['consent_enabled'] is True and status['controls']['paused'] is False, 'resumed observer created agent or changed consent')
        events = records(out / 'watch.stdout')
        check(events[-1]['event'] == 'mission-watch-stopped' and events[-1]['children_still_running'] == 0, 'resumed observer claimed ownership of foreign worker')
        fixture.publish(out / 'observed.json', dict(status='PASS', agent=agent, turn=turn, reservation=money, model_funding=finance, events=events, observation_is_permission=False))
    except BaseException as error:
        errors.append(error)
    finally:
        if process is not None:
            try:
                if process.poll() is None:
                    if descriptor is not None:
                        signal.pidfd_send_signal(descriptor, signal.SIGINT)
                    else:
                        # pidfd acquisition failed after our own Popen returned.
                        # Only that unreaped child handle is used, never a stored PID.
                        process.send_signal(signal.SIGINT)
                process.wait()
            except BaseException as error:
                errors.append(error)
        for channel in (stdout, stderr):
            if channel is not None:
                try:
                    channel.close()
                except BaseException as error:
                    errors.append(error)
        for fd in (notify, descriptor):
            if fd is not None:
                try:
                    os.close(fd)
                except BaseException as error:
                    errors.append(error)
    if errors:
        raise BaseExceptionGroup('registered observer and owned watch cleanup', errors)
    return dict(status='PASS', worker_observed=True, worker_adopted=False, new_worker=False, funding_preserved=True)


def main():
    """Bind exact delivered profile and run complete owned fixture under a subreaper."""
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ('binary', 'profile', 'output'):
        parser.add_argument('--' + flag, type=Path, required=True)
    args = parser.parse_args(); out = args.output.resolve(); out.mkdir(mode=0o700, exist_ok=False)
    binary = args.binary.resolve(strict=True); profile = json.loads(args.profile.read_bytes())
    fixture.publish(out / 'probe-source.py', Path(__file__).read_bytes())
    fixture.publish(out / 'public-fixture-source.py', Path(public.__file__).read_bytes())
    fixture.publish(out / 'inputs.json', dict(binary_sha256=profile['sha256'], profile_sha256=fixture.digest(args.profile.read_bytes()), global_watchdog=None))
    check(profile['no_fixture_feature'] and profile['sha256'] == fixture.digest(binary.read_bytes()), 'registered observer binary/profile mismatch')
    def run():
        """Require a healthy full journey before resuming a held registered worker."""
        positive = public.run_case(binary, out / 'positive', 'sigint')
        resumed = public.run_case(binary, out / 'resume', 'sigint', while_running=observe)
        return dict(status='PASS', positive=positive, resumed=resumed, global_watchdog=None, installed=False)
    result = worker.subreaper_scope(run, lambda e: fixture.publish(out / 'failure.json', dict(status='FAIL', cause=''.join(traceback.format_exception(e)))), parent_death.subreaper)
    fixture.publish(out / 'result.json', result)
    print('REGISTERED_OBSERVER_PASS same worker survives both watches and completes', flush=True)


if __name__ == '__main__':
    main()
