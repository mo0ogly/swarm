#!/usr/bin/env python3
"""Verify closed-fixture Go→Rust initialization and pre-business-write rollback.

Explicit frozen binaries only. Go creates inert data; the supported Rust init
migrates it without agents/services. Public reads and all existing SQL columns
are compared. The maintained offline snapshot tool restores a fresh owned path.
This proves neither live backup, post-cutover-write downgrade, active processes,
identity adoption nor production rollback. No provider call/global watchdog.
"""
import argparse
from contextlib import closing
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import tempfile

from rust_agent_model_route import publish

ROOT = Path(__file__).resolve().parents[1]


def data(root, original=None):
    """Read only owned fixture rows, retaining original-column comparisons.

    SQL identifiers come exclusively from SQLite schema metadata and are quoted.
    Connections close on success/error. New native tables are not old-data loss.
    """
    quote = lambda s: '"' + s.replace('"', '""') + '"'
    with closing(sqlite3.connect((root / '.swarm/state.db').as_uri() + '?mode=ro', uri=True)) as connection:
        tables = original or {row[0]: [field[1] for field in connection.execute('PRAGMA table_info(' + quote(row[0]) + ')')] for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")}
        return {name: {'columns': value['columns'] if original else value,
                       'rows': sorted(list(connection.execute('SELECT ' + ','.join(quote(c) for c in (value['columns'] if original else value)) + ' FROM ' + quote(name))), key=repr)} for name, value in tables.items()}


def run(go, rust, output):
    """Own all fixture writes; retain command bytes and refusal/recovery evidence."""
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    root = Path(tempfile.mkdtemp(prefix='swarm-cutover-'))
    publish(output / 'birth.json', {'kind': 'owned_inert_fixture_birth', 'root': str(root), 'go_sha256': hashlib.sha256(go.read_bytes()).hexdigest(), 'rust_sha256': hashlib.sha256(rust.read_bytes()).hexdigest(), 'live_database': False})
    checks = []
    calls = []

    def check(name, condition):
        """Retain each asserted effect; exit zero alone is never the contract."""
        assert condition, name
        checks.append(name)
        publish(output / 'checks.json', checks)
        print('PASS ' + name, flush=True)

    def call(binary, args, value=None, selected=root):
        """Execute only explicit local fixture operations and retain both streams."""
        name = 'call-' + str(len(calls))
        argv = [str(binary), '--root', str(selected), '--json', *args]
        if value is not None:
            path = output / (name + '.input.json')
            publish(path, value)
            argv += ['--input', str(path)]
        result = subprocess.run(argv, stdin=subprocess.DEVNULL, capture_output=True)
        publish(output / (name + '.stdout'), result.stdout)
        publish(output / (name + '.stderr'), result.stderr)
        calls.append({'argv': argv, 'exit': result.returncode})
        publish(output / 'calls.json', calls)
        return result

    assert call(go, ['init']).returncode == 0
    created = call(go, ['work', 'create'], dict(schema_version=1, event_id='fixture-create', expected_revision=0, title='Bascule Ω <historique>', objective='Retenir les données', scope='Fixture inerte', criteria=['Conservation vérifiée']))
    assert created.returncode == 0, created.stderr
    work = json.loads(created.stdout)['work']['id']
    added = call(go, ['task', 'add', work], dict(schema_version=1, event_id='fixture-task', expected_revision=1, id='task', title='Tâche historique', deliverable='Données', criteria=['Lecture exacte']))
    assert added.returncode == 0, added.stderr
    budget = call(go, ['budget', 'apply', work], dict(schema_version=1, event_id='fixture-budget', expected_revision=2, budget=dict(limit_usd=17, reserve_per_launch_usd=3, estimate_source='Owned fixture values', reference_date='2026-10-28')))
    assert budget.returncode == 0, budget.stderr
    read = call(go, ['work', 'show', work])
    assert read.returncode == 0
    expected = json.loads(read.stdout)['work']
    observed_budget = call(go, ['budget', 'show', work])
    assert observed_budget.returncode == 0
    observed_budget = json.loads(observed_budget.stdout)
    before = data(root)
    check('go-valid-work-task-budget-control', expected['tasks'][0]['id'] == 'task' and observed_budget['budget']['budget']['limit_usd'] == 17)
    snapshot = output / 'snapshot'
    command = ['python3', str(ROOT / 'tools/rust-migration/cutover_snapshot.py'), 'capture', '--fixture-root', str(root), '--output', str(snapshot)]
    capture = subprocess.run(command, capture_output=True)
    publish(output / 'capture.stdout', capture.stdout); publish(output / 'capture.stderr', capture.stderr)
    assert capture.returncode == 0, capture.stderr
    check('snapshot-preserves-go-rows', data(root, before) == before)
    converted = call(rust, ['init'])
    check('supported-rust-init-succeeds', converted.returncode == 0)
    read = call(rust, ['work', 'show', work])
    check('rust-reads-identical-work-json', read.returncode == 0 and json.loads(read.stdout)['work'] == expected)
    check('conversion-preserves-all-existing-sql-columns', data(root, before) == before)
    check('explicit-rust-settings-created', (root / '.swarm/engine-settings.json').is_file())
    destination = output / 'restored-root'
    command = ['python3', str(ROOT / 'tools/rust-migration/cutover_snapshot.py'), 'restore', '--snapshot', str(snapshot), '--destination', str(destination)]
    restored = subprocess.run(command, capture_output=True)
    publish(output / 'restore.stdout', restored.stdout); publish(output / 'restore.stderr', restored.stderr)
    check('offline-restore-to-fresh-root-succeeds', restored.returncode == 0)
    check('rollback-preserves-all-original-sql-columns', data(destination, before) == before)
    check('rollback-keeps-original-settings-absence', not (destination / '.swarm/engine-settings.json').exists())
    read = call(go, ['work', 'show', work], selected=destination)
    check('go-reads-restored-identical-work-json', read.returncode == 0 and json.loads(read.stdout)['work'] == expected)
    publish(output / 'result.json', {'status': 'PASS', 'checks': checks, 'work': work, 'go_sha256': hashlib.sha256(go.read_bytes()).hexdigest(), 'rust_sha256': hashlib.sha256(rust.read_bytes()).hexdigest(), 'root': str(root), 'restored_root': str(destination), 'provider_started': False, 'business_write_after_conversion': False, 'production_rollback_qualified': False})
    print('GO_RUST_CLOSED_CUTOVER_VERIFIED ' + str(len(checks)), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--go-binary', type=Path, required=True)
    parser.add_argument('--rust-binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    run(args.go_binary.resolve(), args.rust_binary.resolve(), args.output.resolve())
