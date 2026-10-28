#!/usr/bin/env python3
"""Verify native queue observation/refusal/recovery on an owned inert Root.

Public operations create Work/task records. Queue rows are explicit observation
fixtures inserted only into that owned database: no real enqueue/acquire/release,
lease, authority, provider or automatic FIFO execution is claimed. Requires a
qualified normal binary/profile and fresh output; all channels/failures survive.
"""
import argparse
import json
import sqlite3
import traceback
from pathlib import Path
import rust_agent_model_route as fixture
from rust_mission_task_order import snapshot


def main():
    """Assert coherent exact ordinals/blockers, malformed-row refusal and restore.

    Only fixture SQL data is mutated; public observation must preserve every SQL
    table. Source/hash/method/channel errors propagate with retained diagnostics.
    DB close is mandatory; Python chaining retains a primary failure on cleanup.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['binary', 'profile', 'output']:
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(exist_ok=False)
    fixture.publish(out / 'method.py', Path(__file__).read_bytes())
    try:
        profile = json.loads(args.profile.read_bytes())
        assert profile['no_fixture_feature'] and fixture.digest(args.binary.read_bytes()) == profile['sha256']
        f = fixture.Fixture(args.binary.resolve(), out, 'workspace-queue')
        before = snapshot(f)
        empty = f.ok(['mission', 'queue', f.work])
        assert empty['work'] == f.work and empty['turns'] == [] and empty['blockers'] == []
        assert 'no-acquisition-or-launch' in empty['scope'] and snapshot(f) == before
        other = f.ok(['work', 'create'], dict(schema_version=1, event_id='queue-other-create', expected_revision=0,
                    title='Other queue fixture', objective='Observe only', scope='No admission', criteria=['No launch']))['work']['id']
        for work, tasks in [(other, ['older', 'released']), (f.work, ['sibling', 'late'])]:
            for task in tasks:
                revision = f.ok(['work', 'show', work])['work']['revision']
                f.ok(['task', 'add', work], dict(schema_version=1, event_id='queue-add-' + task,
                     expected_revision=revision, id=task, title=task, deliverable='Observation', criteria=['No launch']))
        db = sqlite3.connect(f.root / '.swarm/state.db')
        try:
            now = '2026-10-22T10:00:00Z'
            maximum = 9223372036854775807  # SQLite i64 fixture data, not policy.
            shared, disjoint = str(f.root / 'shared'), str(f.root / 'disjoint')
            rows = [(maximum - 4, other, 'older', shared, 'waiting', '', now, '', ''),
                    (maximum - 3, f.work, 'task', shared + '/child', 'waiting', '', now, '', ''),
                    (maximum - 2, f.work, 'sibling', disjoint, 'waiting', '', now, '', ''),
                    (maximum - 1, other, 'released', disjoint, 'released', '', now, '', now),
                    (maximum, f.work, 'late', shared, 'waiting', '', now, '', '')]
            db.executemany('INSERT INTO workspace_turns VALUES(?,?,?,?,?,?,?,?,?)', rows)
            db.commit()
            valid = snapshot(f)
            view = f.ok(['mission', 'queue', f.work])
            assert [r['seq'] for r in view['turns']] == [str(r[0]) for r in rows]
            expected = [dict(task=task, candidate_seq=str(seq), older_seq=str(maximum - 4), older_work=other, older_task='older')
                        for task, seq in [('task', maximum - 3), ('late', maximum)]]
            assert view['blockers'] == expected and snapshot(f) == valid
            assert f.ok(['mission', 'queue', f.work]) == view and snapshot(f) == valid
            # These updates are observation DATA, not authorized turn transitions.
            db.execute('UPDATE workspace_turns SET state=?,released_at=? WHERE seq=?', ('released', now, maximum - 4))
            db.commit()
            released = snapshot(f)
            value = f.ok(['mission', 'queue', f.work])
            assert value['blockers'] == [dict(task='late', candidate_seq=str(maximum), older_seq=str(maximum - 3), older_work=f.work, older_task='task')]
            assert snapshot(f) == released
            db.execute('UPDATE workspace_turns SET state=?,released_at=?,agent_id=?,acquired_at=? WHERE seq=?',
                       ('acquired', '', 'observed-agent', now, maximum - 4))
            db.commit()
            acquired = snapshot(f)
            assert f.ok(['mission', 'queue', f.work])['blockers'] == expected and snapshot(f) == acquired
            db.execute('UPDATE workspace_turns SET state=?,agent_id=?,acquired_at=? WHERE seq=?', ('waiting', '', '', maximum - 4))
            db.commit()
            assert snapshot(f) == valid
            for field, bad, original, code in [
                ('workspace', 'relative', shared, 'workspace-turn-identity-or-path-invalid'),
                ('requested_at', 'invalid-time', now, 'workspace-turn-timestamp-invalid'),
                ('agent_id', 'unexpected-owner', '', 'workspace-turn-owner-state-invalid'),
            ]:
                # Column identifiers are this closed fixture list, never user input.
                db.execute('UPDATE workspace_turns SET ' + field + '=? WHERE seq=?', (bad, maximum - 4))
                db.commit()
                malformed = snapshot(f)
                status, wire = f.call(['mission', 'queue', f.work])
                assert status != 0 and code in json.dumps(wire), 'specific queue refusal required; original channels retained'
                assert snapshot(f) == malformed
                db.execute('UPDATE workspace_turns SET ' + field + '=? WHERE seq=?', (original, maximum - 4))
                db.commit()
                assert snapshot(f) == valid
                assert f.ok(['mission', 'queue', f.work]) == view and snapshot(f) == valid
        finally:
            db.close()
        for argv, value in [(['mission', 'queue', 'missing-work'], None),
                            (['mission', 'queue', f.work], dict(schema_version=1))]:
            before = snapshot(f)
            status, _ = f.call(argv, value)
            assert status != 0 and snapshot(f) == before
        assert not (f.root / 'model-received.json').exists()
        fixture.publish(out / 'result.json', dict(status='PASS', observation_rows=5, malformed_refusals=3,
                        exact_i64=True, queue_writes_are_fixture_only=True, acquisition_verified=False, provider_called=False))
        print('PASS native workspace queue observation, exact ordinals, refusals and recovery', flush=True)
    except BaseException as error:
        fixture.publish(out / 'failure.json', dict(status='FAIL', message=str(error), traceback=traceback.format_exc()))
        raise


if __name__ == '__main__':
    main()
