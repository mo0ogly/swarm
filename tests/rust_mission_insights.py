#!/usr/bin/env python3
"""Public changes/spending reads on a fresh inactive Root, with retained evidence.

No providers, live missions, retry or global watchdog. The
same historical producer must supply status and its two subdocuments; incomplete
contracts must refuse rather than publish an empty success. Timestamps from
separate spending reads are validated before comparison, never silently dropped.
"""
import argparse
import copy
import datetime
import hashlib
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
from rust_agent_model_route import publish


def verify(binary, output):
    """Retain every argv/channel and assert DTO content, refusal and recovery.

    Public init/work/task commands write the isolated Root; a named SQL fault
    inserts and removes only a malformed local visit after valid witnesses. Snapshots use
    a readonly SQL dump, not file mtimes/WAL layout. Process/JSON/IO failures
    propagate with originals retained; a failure never produces PASS evidence.
    """
    output.mkdir(parents=True, exist_ok=False)
    root = output / 'root'
    root.mkdir()
    checks, calls = [], []

    def check(name, condition):
        """Require a unique meaningful assertion; expose progress without waiting."""
        assert condition, name
        assert name not in checks, name
        checks.append(name)
        print('PASS insights-' + name, flush=True)

    def call(args, value=None, language='fr', machine=True):
        """Run one public CLI, preserving both channels before any assertion."""
        argv = [str(binary), *args, '--root', str(root), '--lang', language]
        if machine:
            argv.append('--json')
        name = 'call-' + str(len(calls) + 1)
        if value is not None:
            path = output / (name + '.input.json')
            publish(path, value)
            argv += ['--input', str(path)]
        done = subprocess.run(argv, stdin=subprocess.DEVNULL, capture_output=True)
        calls.append(dict(argv=argv, exit=done.returncode))
        publish(output / (name + '.stdout'), done.stdout)
        publish(output / (name + '.stderr'), done.stderr)
        publish(output / 'commands.json', calls)
        return done

    def ok(args, value=None, language='fr'):
        """Require successful singular JSON and an empty diagnostic channel."""
        done = call(args, value, language)
        assert done.returncode == 0, (args, done.returncode, done.stderr.decode())
        assert not done.stderr, done.stderr.decode()
        return json.loads(done.stdout)

    def snapshot():
        """Hash all persisted SQL state using readonly access to this fixture."""
        with sqlite3.connect('file:' + str(root / '.swarm/state.db') + '?mode=ro', uri=True) as db:
            return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()

    def spending(value):
        """Validate the per-read UTC clock then normalize that single field."""
        value = copy.deepcopy(value)
        stamp = datetime.datetime.fromisoformat(value['observed_at'].replace('Z', '+00:00'))
        assert stamp.tzinfo is not None and stamp.utcoffset() == datetime.timedelta(0)
        value['observed_at'] = '<per-read clock>'
        return value

    ok(['init'])
    work = ok(['work', 'create'], dict(schema_version=1, event_id='insights-create',
        expected_revision=0, title='Insights fixture', objective='Read changes and spending',
        scope='Inactive isolated fixture', criteria=['No mutation during reads']))['work']['id']
    for stage in ['empty', 'task']:
        if stage == 'task':
            revision = ok(['work', 'show', work])['work']['revision']
            ok(['task', 'add', work], dict(schema_version=1, event_id='insights-task',
                expected_revision=revision, id='owned-task', title='Recorded task',
                deliverable='A report', criteria=['Observable result']))
        before = snapshot()
        for language in ['fr', 'en']:
            baseline = ok(['mission', 'status', work], language=language)
            changes = ok(['mission', 'changes', work], language=language)
            costs = ok(['mission', 'spending', work], language=language)
            tag = stage + '-' + language
            check(tag + '-changes-exact', changes == baseline['changes_since_visit'])
            check(tag + '-spending-exact', spending(costs) == spending(baseline['spending']))
            check(tag + '-first-visit', changes['first_visit'] is True and changes['to_revision'] >= 1)
            check(tag + '-spending-unknown-not-invented', costs['worker_retries'] == 0
                  and costs['recorded_control_executions'] == 0 and bool(costs['measurement_source']))
            for operation in ['changes', 'spending']:
                text = call(['mission', operation, work], language=language, machine=False)
                check(tag + '-' + operation + '-human', text.returncode == 0 and not text.stderr
                      and json.loads(text.stdout.decode().split('\n', 1)[1]) is not None)
        check(stage + '-reads-preserve-all-SQL', snapshot() == before)
    for operation in ['changes', 'spending']:
        before = snapshot()
        missing = call(['mission', operation, 'missing-owned-work'])
        check(operation + '-missing-refuses', missing.returncode != 0 and not missing.stdout
              and bool(json.loads(missing.stderr)['failure']['code']))
        invalid = call(['mission', operation, work], value={})
        check(operation + '-input-refuses', invalid.returncode != 0 and not invalid.stdout)
        check(operation + '-refusals-preserve-SQL', snapshot() == before)
        check(operation + '-recovery', isinstance(ok(['mission', operation, work]), dict))
    before = snapshot()
    database = root / '.swarm/state.db'
    # Same local-OS visit selector as operator_identity(), not authentication.
    operator = socket.gethostname() + '/uid:' + str(os.getuid())
    with sqlite3.connect(database) as db:
        db.execute('PRAGMA foreign_keys=ON')
        assert db.execute('SELECT count(*) FROM session_visits WHERE work_id=? AND operator=?', (work, operator)).fetchone()[0] == 0
        db.execute('INSERT INTO session_visits(work_id,operator,revision,at) VALUES(?,?,?,?)',
                   (work, operator, b'invalid-integer-fixture', '2026-10-24T00:00:00Z'))
    publish(output / 'fault.json', dict(scope='Malformed visit in owned inactive Root only', work=work,
        selector='local host/uid, not authenticated actor', restoration='Delete this inserted visit only'))
    try:
        broken = snapshot()
        for operation in ['status', 'changes', 'spending']:
            failed = call(['mission', operation, work])
            check(operation + '-malformed-refuses', failed.returncode != 0 and not failed.stdout
                  and bool(json.loads(failed.stderr)['failure']['code']))
            check(operation + '-fault-not-mutated', snapshot() == broken)
    finally:
        with sqlite3.connect(database) as db:
            db.execute('DELETE FROM session_visits WHERE work_id=? AND operator=?', (work, operator))
    check('fault-restored-exact-SQL', snapshot() == before)
    for operation in ['changes', 'spending']:
        check(operation + '-fault-recovery', isinstance(ok(['mission', operation, work]), dict))
    publish(output / 'result.json', dict(status='PASS', assertions=checks, binary=str(binary),
        binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
        limits=['No Go executable comparison', 'No historical configured/batch mission acceptance',
                'No provider execution', 'No browser or HTTP contract change']))


def main():
    """Require an explicit binary and fresh retained output; propagate all failures."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    verify(args.binary.resolve(), args.output.resolve())


if __name__ == '__main__':
    main()
