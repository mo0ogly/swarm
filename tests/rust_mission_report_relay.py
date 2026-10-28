#!/usr/bin/env python3
"""Public resumed-watch handoff relay using a real completed native worker.

The existing maintained watch fixture plans/delegates/runs a real inert child.
Its child writes docs/task.md before exit. Subsequent watches use explicit public mission
start consent and no manual task.submit. Planning is explicitly paused to isolate relay
from another paid/native role; controls/reviewer/acceptance remain unclaimed.
Positive control precedes ambiguity and mtime faults on a second owned Root.
No SQL writer, live Root, global watchdog, retry loop or synthetic terminal row.
"""
import argparse
import json
import os
from pathlib import Path
import select
import signal
import sqlite3
import subprocess
import traceback

import rust_agent_model_route as fixture
import rust_mission_watch_planning as planning
import rust_mission_watch_worker as worker
import rust_native_parent_death as parent_death
from rust_planning_activation import apply_policy

check = worker.check


def rows(owned,sql,parameters):
    """Read isolated fixture metadata through a read-only, nonblocking probe.

    Zero SQLite wait requests immediate observation, not a global verification
    watchdog or product policy. SQL/read and independent close failures both
    survive; no live mission or SQL authority mutation is admitted here.
    """
    db=sqlite3.connect((owned.root/'.swarm/state.db').as_uri()+'?mode=ro',uri=True,timeout=0)
    primary=None;value=None
    try:value=db.execute(sql,parameters).fetchall()
    except BaseException as error:primary=error
    try:db.close()
    except BaseException as error:
        if primary is not None:raise BaseExceptionGroup('fixture metadata read/close',[primary,error])
        raise
    if primary is not None:raise primary
    return value


def work(owned):
    """Read the actual public Work, without inferring completion from a process."""
    return owned.ok(['work', 'show', owned.work])['work']


def cycle_complete(records, completed_event=None):
    """Require an actual dispatch synchronization and the requested role receipt.

    None observes dispatch only; it never establishes asynchronous role completion.
    A controls/review caller must name its durable public event before sending stop.
    Missing/early/wrong-role events remain incomplete; this grants no task acceptance.
    """
    synchronized = any(r.get('event') == 'mission-watch-blocked' and r.get('synchronization_committed') is True for r in records)
    return synchronized and (completed_event is None or any(r.get('event') == completed_event for r in records))


def cycle(owned, out, started=None, refusal=False, completed_event=None):
    """Wait for dispatch plus the requested role's actual receipt before stopping.

    Inotify and the held child pidfd observe progress without a wall timeout.
    Missing output is not success: child exit fails with retained channels. The
    dispatch marker is independent of asynchronous controls/review completion:
    completed_event is required by callers asserting those roles. It is None for
    dispatch-only observation. Optional started observes a real
    owned child before releasing it or applying a public revocation. refusal waits
    for actual nonzero watch exit; caller must assert its specific cause/effects.
    Finally always attempts graceful
    stop/reap and descriptor close; primary and cleanup causes remain grouped.
    """
    out.mkdir()
    notifications = worker.watch_directory(out, modifications=True)
    process = descriptor = None
    errors = []
    value = None
    streams = []
    try:
        streams = [worker.spool(out / name) for name in ['watch.stdout', 'watch.stderr']]
        argv = [str(owned.binary), 'mission', 'watch', owned.work, '--root', str(owned.root), '--json']
        fixture.publish(out / 'command.json', dict(argv=argv, global_watchdog=None))
        process = subprocess.Popen(argv, env=owned.environment, stdin=subprocess.DEVNULL, stdout=streams[0], stderr=streams[1])
        descriptor = os.pidfd_open(process.pid)
        if started is not None: started(process, descriptor)
        while value is None:
            raw = (out / 'watch.stdout').read_bytes()
            records = [json.loads(line) for line in raw.splitlines(keepends=True) if line.endswith(b'\n')]
            if not refusal and cycle_complete(records, completed_event):
                value = records
                break
            ready, _, _ = select.select([notifications, descriptor], [], [])
            if descriptor in ready:
                if refusal:
                    value=records
                    break
                raise RuntimeError('watch exited before its post-relay cycle marker')
            if notifications in ready:
                os.read(notifications, 65536)  # kernel event read batch, not policy
    except BaseException as error:
        errors.append(error)
    finally:
        if process is not None:
            try:
                if process.poll() is None:
                    check(descriptor is not None, 'watch cleanup lacks owned pidfd')
                    signal.pidfd_send_signal(descriptor, signal.SIGINT)
                code = process.wait()
                fixture.publish(out / 'exit.json', dict(exit=code))
                check((code != 0) if refusal else (code == 0), 'watch exit does not match expected completion/refusal')
            except BaseException as error:
                errors.append(error)
        for stream in streams:
            for operation in (stream.flush, lambda stream=stream: os.fsync(stream.fileno()), stream.close):
                try: operation()
                except BaseException as error: errors.append(error)
        for fd in [notifications, descriptor]:
            if fd is not None:
                try: os.close(fd)
                except BaseException as error: errors.append(error)
    if errors:
        raise BaseExceptionGroup('public report relay watch/cleanup', errors)
    return value


def prepare(binary, out, controls=False, control_gate=False, review_fault=False, review_gate=False):
    """Produce the report through a real inert worker before starting relay tests.

    Source program/report variant is fixed before provider preflight. All setup
    and pause/start operations use public CLI. Retained original worker receipts
    and stopped watch generation are consumed rather than fabricated.
    controls=True keeps planning active for the explicit next control journey;
    default planning pause isolates report selection without control execution.
    control_gate selects the owned socket-controlled saved check before preflight.
    review_fault/review_gate fix malformed/held reviewer variants before preflight.
    """
    result, owned = planning.run_case(binary, out / 'producer', relay_report=True, return_fixture=True, control_gate=control_gate, review_fault=review_fault, review_gate=review_gate)
    check(result['task_accepted'] is False and result['actual_provider_entry'], 'producer did not actually execute')
    current = work(owned)
    if not controls:
        owned.ok(['planning', 'pause', owned.work], dict(schema_version=1, event_id='relay-pause-planning', expected_revision=current['revision'], reason='Isolate report relay from subsequent planning'))
    current = work(owned)
    saved = current['launch_profile']
    profile = {k: saved[k] for k in ['provider','role','level','capture_output']}
    profile['workspace'] = str(Path(saved['workspace']).relative_to(owned.root))
    owned.ok(['mission', 'start', owned.work], dict(schema_version=1, event_id='relay-new-consent', expected_revision=current['revision'], reason='Explicit new consent after the producer fixture stopped mission', slots=1, profile=profile))
    report = owned.root / 'watch-worker/docs/task.md'
    check(report.is_file() and b'Input SHA256:' in report.read_bytes(), 'native child did not write actual report')
    return owned, result, report


def assert_submitted(owned, before_agent, costs, events):
    """Require canonical submitted state and unchanged original Agent/spending."""
    current = work(owned)
    task = next(t for t in current['tasks'] if t['id'] == 'task')
    check(task['status'] == 'submitted', 'watch did not submit the actual handoff')
    check(not task.get('automatic_validation') and not task.get('independent_review'), 'relay fabricated quality evidence')
    check(owned.ok(['agent', 'show', before_agent['id']])['agent'] == before_agent, 'relay changed original producer')
    check(owned.ok(['costs', 'show', owned.work]) == costs, 'relay changed spending/reservation')
    relays = [r for r in events if r.get('event') == 'mission-report-submitted']
    check(len(relays) == 1 and relays[0]['agent_id'] == before_agent['id'] and relays[0]['report'] == 'watch-worker/docs/task.md'
          and relays[0]['task_accepted'] is False, 'missing exact automatic report event')
    check(not any(c['argv'][1:3] == ['task', 'submit'] for c in owned.calls), 'test manually submitted report')
    return current


def run(binary, out):
    """Verify valid control, ambiguity refusal, stale refusal, settings use and recovery."""
    positive = out / 'positive';positive.mkdir()
    owned, proof, report = prepare(binary, positive)
    positive_root = str(owned.root)
    agent = owned.ok(['agent', 'show', proof['agent']])['agent']
    costs = owned.ok(['costs', 'show', owned.work])
    events = cycle(owned, positive / 'watch')
    assert_submitted(owned, agent, costs, events)
    # A second watch observes submitted state; no duplicate report event or call.
    again = cycle(owned, positive / 'replay')
    check(not any(r.get('event') == 'mission-report-submitted' for r in again), 'already submitted worker relayed twice')
    negative = out / 'negative';negative.mkdir()
    owned, proof, report = prepare(binary, negative)
    agent = owned.ok(['agent', 'show', proof['agent']])['agent']
    costs = owned.ok(['costs', 'show', owned.work])
    duplicate = report.with_name('task-handoff-extra.md')
    fixture.publish(duplicate, report.read_bytes())
    before = work(owned)
    refused = cycle(owned, negative / 'ambiguous')
    check(work(owned) == before and not any(r.get('event') == 'mission-report-submitted' for r in refused), 'ambiguous report mutated Work')
    duplicate.unlink()  # only owned injected fault, original report remains
    original = report.stat()
    from datetime import datetime
    started = datetime.fromisoformat(agent['started'].replace('Z', '+00:00'))
    old_ns = int(started.timestamp()) * 1_000_000_000 - 1_000_000_000
    os.utime(report, ns=(original.st_atime_ns, old_ns))
    apply_policy(owned, 'relay-strict-zero', dict(agent_report_clock_skew_seconds='0'))
    before = work(owned)
    refused = cycle(owned, negative / 'stale-zero')
    check(work(owned) == before and not any(r.get('event') == 'mission-report-submitted' for r in refused), 'zero setting was not consumed')
    apply_policy(owned, 'relay-margin-two', dict(agent_report_clock_skew_seconds='2'))
    recovered = cycle(owned, negative / 'margin-recovery')
    assert_submitted(owned, agent, costs, recovered)
    os.utime(report, ns=(original.st_atime_ns, original.st_mtime_ns))
    return dict(status='PASS', actual_native_worker=True, previous_watch_generation=True, automatic_submission=True,
                no_manual_submit=True, ambiguous_refusal=True, zero_setting_consumed=True, two_second_recovery=True,
                original_producer_and_spending_preserved=True, duplicate_submission_refused=True,
                roots=[positive_root, str(owned.root)], task_accepted=False, automatic_validation=False, automatic_reviewer=False, installed=False)


def main():
    """Bind tests to the qualified default-profile bytes and retain fresh evidence."""
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ['binary', 'profile', 'output']: parser.add_argument('--' + flag, required=True, type=Path)
    args = parser.parse_args();binary = args.binary.resolve();out = args.output.resolve();out.mkdir(parents=True)
    profile = json.loads(args.profile.read_bytes())
    check(profile['no_fixture_feature'] and profile['sha256'] == fixture.digest(binary.read_bytes()), 'relay binary/profile mismatch')
    result = worker.subreaper_scope(lambda: run(binary, out), lambda e: fixture.publish(out / 'failure.json', dict(status='FAIL',cause=''.join(traceback.format_exception(e)))), parent_death.subreaper)
    fixture.publish(out / 'result.json', result)
    print('PASS public watch relays actual report, refuses ambiguity and consumes saved clock policy', flush=True)


if __name__ == '__main__': main()
