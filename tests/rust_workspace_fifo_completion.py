#!/usr/bin/env python3
"""Verify public FIFO completion recovery after the real SDK owner has exited.

Public planning supplies one genuine scoped worker. A separately qualified SDK
runs an owned inert provider, injects a negative journal fault during settlement,
restores its cause and exits with the turn still held. This harness reads that
state, reconciles through the normal CLI and verifies durable release/replay.
No positive SQL seed, real model, acceptance, installation or global watchdog.
All channels, original receipts, fixture paths and failures remain retained.
"""
import argparse
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import time
import traceback

import rust_agent_model_route as fixture
import rust_mission_task_order as diagnostic
import rust_planning_worker_public_flow as planning


def reservation(owned, agent):
    """Read the exact owned Agent reservation without writing or resetting it.

    Missing rows refuse; statement/connection errors propagate with their cause.
    Amount/state are existing product observations, not fixture permission or a
    measured invoice. The immutable result can be compared across reconciliation.
    """
    db = sqlite3.connect('file:' + str(owned.root / '.swarm/state.db') + '?mode=ro', uri=True)
    try:
        row = db.execute('SELECT state,amount FROM reservations WHERE agent_id=?', (agent,)).fetchone()
        assert row is not None, 'real admission reservation missing'
        return list(row)
    finally:
        db.close()


def provider_files(root):
    """Observe all actual inert-provider markers and reports under this Root.

    Include planner markers, counts, exact bytes, inode/size/mtime; no invented
    process count is inferred. Unchanged files plus public state/replay assertions
    witness this fixture's no-second-provider result, not model authenticity or
    general OS confinement. Files remain owned originals; IO errors refuse.
    """
    paths = set(root.rglob('model-received*.json')) | set(root.rglob('handoff.md'))
    result = {}
    for path in sorted(paths):
        stat = path.stat()
        result[str(path.relative_to(root))] = dict(sha256=fixture.digest(path.read_bytes()),
                inode=stat.st_ino, device=stat.st_dev, size=stat.st_size, mtime_ns=stat.st_mtime_ns)
    return dict(count=len(result), files=result)


def one_turn(queue, work, agent):
    """Require exactly the actual owned task/Agent turn; duplicates never vanish.

    This reads the public strict queue DTO, not a forged claim or a launch grant.
    Other retained Works may exist in the fixture and are not silently rewritten.
    """
    turns = [turn for turn in queue['turns']
             if turn['work'] == work and turn['task'] == 'task' and turn['agent'] == agent]
    assert len(turns) == 1, 'expected exactly one actual worker turn'
    return turns[0]


def missing_terminal_release(owned, out, agent, seq, valid_replay, providers):
    """Remove only genuine release proof after healthy reconciliation/replay.

    This negative corruption never fabricates permission: exact original rows
    and complete SQL snapshots are retained before deletion. Restore those same
    rows in finally, compare the original database, then require public recovery
    success even when the negative unexpectedly succeeded. Independent refusal,
    restore and post-restore failures are all retained before raising. No original
    failed observation is overwritten by the restored successful witness.
    """
    before = diagnostic.snapshot(owned)
    rows = None
    failures = []
    try:
        db = sqlite3.connect(owned.root / '.swarm/state.db')
        try:
            rows = db.execute('SELECT * FROM workspace_turn_events WHERE turn_seq=? AND work_id=? AND agent_id=? AND state=? ORDER BY event_seq',
                              (int(seq), owned.work, agent, 'released')).fetchall()
            assert len(rows) == 1, 'expected genuine single terminal release proof'
            fixture.publish(out / 'terminal-release-original.json', dict(rows=rows, snapshot=before))
            deleted = db.execute('DELETE FROM workspace_turn_events WHERE turn_seq=? AND work_id=? AND agent_id=? AND state=?',
                                 (int(seq), owned.work, agent, 'released')).rowcount
            assert deleted == len(rows)
            db.commit()
        finally:
            db.close()
        fault = diagnostic.snapshot(owned)
        fixture.publish(out / 'terminal-release-fault.json', dict(snapshot=fault))
        code, response = owned.call(['agent', 'reconcile', agent])
        fixture.publish(out / 'terminal-release-refusal.json', dict(exit=code, response=response))
        assert diagnostic.snapshot(owned) == fault, 'terminal refusal mutated the damaged fixture'
        assert provider_files(owned.root) == providers
        assert code != 0, 'terminal reconciliation accepted missing FIFO release proof'
        assert response['failure']['code'], 'missing explicit typed refusal'
        assert 'mission-agent-turn-release-missing' in response['failure']['causes'], 'wrong terminal FIFO refusal boundary'
    except BaseException:
        failures.append(dict(phase='missing-terminal-proof', cause=traceback.format_exc()))
    finally:
        try:
            if rows:
                db = sqlite3.connect(owned.root / '.swarm/state.db')
                try:
                    for row in rows:
                        current = db.execute('SELECT * FROM workspace_turn_events WHERE event_seq=?', (row[0],)).fetchone()
                        if current is None:
                            placeholders = ','.join('?' for _ in row)
                            db.execute('INSERT INTO workspace_turn_events VALUES(' + placeholders + ')', row)
                        else:
                            assert current == row, 'original event identity was unexpectedly replaced'
                    db.commit()
                finally:
                    db.close()
            assert diagnostic.snapshot(owned) == before, 'exact terminal-proof restoration failed'
        except BaseException:
            failures.append(dict(phase='restore-terminal-proof', cause=traceback.format_exc()))
    try:
        restored = owned.ok(['agent', 'reconcile', agent])
        assert restored == valid_replay
        assert diagnostic.snapshot(owned) == before
        assert provider_files(owned.root) == providers
        fixture.publish(out / 'terminal-release-restored.json', dict(response=restored, original_sql_restored=True))
    except BaseException:
        failures.append(dict(phase='public-replay-after-restoration', cause=traceback.format_exc()))
    fixture.publish(out / 'terminal-release-proof-check.json', dict(status='FAIL' if failures else 'PASS', failures=failures))
    if failures:
        raise RuntimeError('terminal FIFO proof check failed; original phases and restoration retained')


def main():
    """Bind real build profiles and exercise post-exit reconcile plus strict replay.

    Fresh output is mandatory. Actual SDK process exit precedes any normal CLI
    recovery; the live-owner refusal belongs to the SDK's named assertion. SDK
    receipt/hash and held turn/reservation must agree with actual retained files,
    normal public Agent/queue and read-only reservation SQL. Reconciliation must
    recover the completion, release the same turn, keep task unaccepted and retain
    the original estimate. Replay compares every SQL table and provider files.
    No missing/unknown outcome is converted to PASS or to absence of side effects.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('binary', 'profile', 'sdk-binary', 'sdk-profile', 'output'):
        parser.add_argument('--' + key, required=True, type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    root = None
    fixture.publish(out / 'method.py', Path(__file__).read_bytes())
    try:
        binary, sdk = args.binary.resolve(), args.sdk_binary.resolve()
        normal = json.loads(args.profile.read_bytes())
        profile = json.loads(args.sdk_profile.read_bytes())
        assert normal['no_fixture_feature'] is True
        assert normal['sha256'] == fixture.digest(binary.read_bytes())
        assert profile['engine_features_empty'] is True
        assert profile['shared_workspace_features_match_normal'] is True
        assert profile['test_profile'] is False
        assert profile['binary_sha256'] == fixture.digest(sdk.read_bytes())
        assert profile['normal_profile_sha256'] == normal['sha256']
        fixture.publish(out / 'profiles.json', dict(normal=normal, sdk=profile))
        sources = {str(Path(module.__file__).resolve()): fixture.digest(Path(module.__file__).read_bytes())
                   for name, module in tuple(sys.modules.items())
                   if name.startswith('rust_') and getattr(module, '__file__', None)}
        sources[str(Path(__file__).resolve())] = fixture.digest(Path(__file__).read_bytes())
        fixture.publish(out / 'inputs.json', dict(sources=sources,
                        normal_sha256=normal['sha256'], sdk_sha256=profile['binary_sha256']))
        ready, owned = planning.flow(binary, out / 'planning', before_worker=True, return_fixture=True)
        assert ready['status'] == 'READY_FOR_NATIVE_WORKER' and not ready['worker_started']
        root = Path(ready['root']).resolve()
        fixture.publish(out / 'ready.json', ready)
        request = dict(kind='owned-inert-fifo-completion-fixture', root=str(root),
                       work=ready['work_id'], result=str(out / 'sdk-result.json'))
        fixture.publish(root / '.swarm/verification-fifo-completion.json', request)
        fixture.publish(out / 'request.json', request)
        command = [str(sdk), str(out / 'request.json')]
        fixture.publish(out / 'command.json', dict(argv=command, global_watchdog=None,
                        fixture_cache=str(owned.cache_root)))
        started = time.time()
        run = subprocess.run(command, env=owned.environment, stdin=subprocess.DEVNULL, capture_output=True)
        fixture.publish(out / 'stdout', run.stdout)
        fixture.publish(out / 'stderr', run.stderr)
        fixture.publish(out / 'exit.json', dict(returncode=run.returncode,
                        started=started, finished=time.time()))
        assert run.returncode == 0, 'completion SDK failed; original channels retained'
        lines = run.stdout.decode('utf-8').splitlines()
        for name in (
            'PASS provider completed but FIFO settlement journal failure retains ownership',
            'PASS live owner reconcile refuses and preserves held FIFO state',
        ):
            assert lines.count(name) == 1, 'missing/duplicate named SDK assertion'
        result = json.loads((out / 'sdk-result.json').read_bytes())
        assert result['status'] == 'READY_FOR_POST_EXIT_RECONCILE'
        assert result['work'] == owned.work and result['agent_id'] == 'fifo-completion-agent'
        for name in ('completion_sqlite_full', 'held_on_failure', 'live_owner_reconcile_refused', 'known_receipt'):
            assert result[name] is True, name
        assert result['paid_provider'] is False and result['task_accepted'] is False
        agent_id = result['agent_id']
        marker = (root / result['provider_marker']).resolve()
        assert marker.is_relative_to(root) and marker == root / 'fifo-completion/model-received.json'
        assert marker.is_file()
        receipt_name = result['receipt_file']
        assert isinstance(receipt_name, str) and Path(receipt_name).name == receipt_name
        receipt = (root / '.swarm' / receipt_name).resolve()
        assert receipt.parent == root / '.swarm' and receipt.is_file()
        receipt_bytes = receipt.read_bytes()
        assert fixture.digest(receipt_bytes) == result['receipt_sha256']
        fixture.publish(out / 'original-native-completion.json', receipt_bytes)
        agent = owned.ok(['agent', 'show', agent_id])['agent']
        assert agent['status'] == result['pre_reconcile_status']
        assert agent['status'] in ('starting', 'running')
        assert agent['native_mission']['schema_version'] == 3
        assert agent['native_mission']['workspace_turn_seq'] == result['turn_seq']
        queue = owned.ok(['mission', 'queue', owned.work])
        held = one_turn(queue, owned.work, agent_id)
        assert held == result['held_turn'] and held['state'] == 'acquired'
        assert held['seq'] == result['turn_seq']
        before_reservation = reservation(owned, agent_id)
        # SDK uses the real producer tuple (work, amount, state); this read-only
        # observation requests (state, amount). Compare explicit fields instead
        # of inheriting an imagined reservation serialization.
        assert result['reservation_before'] == result['reservation_after']
        assert result['reservation_after'] == [owned.work, before_reservation[1], before_reservation[0]]
        assert before_reservation == ['reserved', 1.0]
        before_provider = provider_files(root)
        fixture.publish(out / 'post-owner-exit-held.json', dict(agent=agent, queue=queue,
                        reservation=before_reservation, provider_files=before_provider))
        recovered = owned.ok(['agent', 'reconcile', agent_id])
        assert recovered['completion_recovered'] is True
        assert recovered['cancelled_before_start'] is False
        assert recovered['accepted_by_cancellation'] is False
        assert recovered['agent']['status'] == 'completed'
        assert recovered['agent']['native_mission'] == agent['native_mission']
        after_queue = owned.ok(['mission', 'queue', owned.work])
        released = one_turn(after_queue, owned.work, agent_id)
        assert released['state'] == 'released' and released['seq'] == held['seq']
        assert released['released_at'] and released['acquired_at'] == held['acquired_at']
        after_reservation = reservation(owned, agent_id)
        assert after_reservation == ['estimated', before_reservation[1]]
        work = owned.ok(['work', 'show', owned.work])['work']
        task = next(task for task in work['tasks'] if task['id'] == 'task')
        assert task['status'] == 'blocked'
        assert provider_files(root) == before_provider
        snapshot = diagnostic.snapshot(owned)
        replay = owned.ok(['agent', 'reconcile', agent_id])
        assert replay == recovered and diagnostic.snapshot(owned) == snapshot
        assert owned.ok(['mission', 'queue', owned.work]) == after_queue
        assert reservation(owned, agent_id) == after_reservation
        assert provider_files(root) == before_provider
        missing_terminal_release(owned, out, agent_id, held['seq'], replay, before_provider)
        assert fixture.digest(binary.read_bytes()) == normal['sha256']
        assert fixture.digest(sdk.read_bytes()) == profile['binary_sha256']
        assert all(fixture.digest(Path(path).read_bytes()) == sha for path, sha in sources.items())
        fixture.publish(out / 'result.json', dict(status='PASS', root=str(root), agent_id=agent_id,
                        turn_seq=held['seq'], original_owner_exited=True, held_after_failure=True,
                        public_completion_recovered=True, released=released, reservation=after_reservation,
                        replay_all_sql_unchanged=True, provider_files_unchanged=True,
                        terminal_release_proof_required=True,
                        actual_provider_files=before_provider, task_accepted=False, installed=False,
                        paid_provider=False, global_watchdog=None, source_stable=True))
        print('PASS FIFO completion refusal retains acquired turn and reservation after owner exit', flush=True)
        print('PASS public reconciliation releases exact FIFO turn without accepting task', flush=True)
        print('PASS completion replay preserves all SQL and actual provider files', flush=True)
        print('PASS terminal replay refuses missing FIFO release proof and recovers', flush=True)
        print('REPLAY_EVIDENCE=' + str(out), flush=True)
    except BaseException:
        fixture.publish(out / 'failure.json', dict(status='FAIL', root=str(root) if root else None,
                        cause=traceback.format_exc()))
        raise


if __name__ == '__main__':
    main()
