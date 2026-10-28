#!/usr/bin/env python3
"""Preserve genuine v1/v2 intentions while refusing new grants under current v4.

A fresh current v4 worker witness precedes each old fixture. Historical helpers
and normal create their own settings, never downgraded metadata. The old SDK
owns its live heartbeat until finish; queued cancellation is separate from its
checked close/reap. No paid model, Go, user Root, SQL authority edits or watchdog.
"""
import argparse
from functools import lru_cache
from collections import Counter
import hashlib
import importlib
import io
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tarfile
import traceback


@lru_cache(maxsize=1)
def current():
    """Load current helpers lazily so the isolated old setup child imports none of them."""
    import rust_mission_root_instance as rootprobe
    return rootprobe


def require(condition, message):
    """Refuse fixture contract failure with a static message, never private row values."""
    if not condition:
        raise RuntimeError(message)


def python_runtime():
    """Require enabled historical asserts; return only safe interpreter verification flags.

    Optimized Python disables assertions inside immutable old helpers, so it is an
    unsupported verification precondition, not a product/administrator policy.
    This check has no Root/provider effects and never records the environment.
    """
    require(sys.flags.optimize == 0, 'historical verification requires Python assertions enabled')
    return dict(optimize=sys.flags.optimize, dont_write_bytecode=sys.dont_write_bytecode)


def digest(raw):
    """Hash exact source/output bytes without claiming actor authenticity."""
    return hashlib.sha256(raw).hexdigest()


def publish(path, value):
    """Retain owned evidence through the established atomic helper; IO errors propagate."""
    current().fixture.publish(path, value)


def json_equal(a, b):
    """Compare JSON types as well as values; numeric booleans cannot satisfy a binding."""
    return json.dumps(a, sort_keys=True, separators=(',', ':'), allow_nan=False) == json.dumps(b, sort_keys=True, separators=(',', ':'), allow_nan=False)


def assert_binding(state, schema):
    """Validate actual schema2 readiness and untouched v1/v2 binding before new grant.

    root_instance and FIFO must be absent, including null; v1 omits parent, v2
    requires canonical lower SHA256. This observation creates no identity proof.
    """
    require(type(state.get('fixture_schema_version')) is int and state['fixture_schema_version'] == 2 and state.get('ready') is True, 'legacy readiness shape differs')
    binding = state['binding']
    require(type(schema) is int and schema in (1, 2) and isinstance(binding, dict)
            and type(binding.get('schema_version')) is int and binding['schema_version'] == schema,
            'historical binding version differs')
    require('root_instance' not in binding and 'workspace_turn_seq' not in binding, 'historical binding acquired new identity or FIFO metadata')
    if schema == 1:
        require('parent_profile_sha256' not in binding, 'v1 binding unexpectedly has parent')
    else:
        parent = binding.get('parent_profile_sha256')
        require(type(parent) is str and len(parent) == 64 and all(c in '0123456789abcdef' for c in parent), 'v2 parent hash differs')


def sql_identifier(value):
    """Quote only a SQLite-owned schema identifier; never interpolate request values."""
    return '"' + value.replace('"', '""') + '"'


def sql_cell(value):
    """Preserve SQLite storage types in private snapshots, including BLOB/text distinction."""
    if isinstance(value, bytes):
        return dict(kind='blob', value=value.hex())
    return dict(kind=type(value).__name__, value=value)


def business_snapshot(owned):
    """Retain coherent complete schema/rows, masking only this live owner's two pulse fields.

    Never exclude a journal/table wholesale. stopped_at, events and all Agent bodies
    remain compared. Snapshot only while the original owner is live; finish may
    legitimately journal its cycle and is inspected separately. No SQL mutation.
    """
    db = sqlite3.connect((owned.root / '.swarm/state.db').as_uri() + '?mode=ro', uri=True, timeout=0)
    primary = None
    try:
        db.execute('BEGIN')
        schema = db.execute("SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY type,name").fetchall()
        tables = {}
        pulses = []
        for kind, name, _, _ in schema:
            if kind != 'table':
                continue
            rows = db.execute('SELECT * FROM ' + sql_identifier(name)).fetchall()
            columns = [row[1] for row in db.execute('PRAGMA table_info(' + sql_identifier(name) + ')')]
            encoded = []
            for row in rows:
                values = list(row)
                if name == 'mission_supervision' and 'work_id' in columns and row[columns.index('work_id')] == owned.work:
                    for field in ('heartbeat_at', 'next_check_at'):
                        if field in columns:
                            pulses.append(dict(field=field, value=values[columns.index(field)]))
                            values[columns.index(field)] = '<owned-live-pulse>'
                encoded.append([sql_cell(value) for value in values])
            tables[name] = dict(columns=columns, rows=sorted(encoded, key=lambda row: json.dumps(row, sort_keys=True)))
        return dict(schema=schema, tables=tables), pulses
    except BaseException as error:
        primary = error
        raise
    finally:
        try:
            db.close()
        except BaseException as error:
            if primary is not None:
                raise BaseExceptionGroup('legacy snapshot and close', [primary, error])
            raise


def preserve_history(before, after):
    """Require old audit rows/claims/receipts to survive permitted cancellation or cycle append.

    These are source-defined append-only histories, not operator retention policy.
    Compare multisets of typed rows; no old duplicate or body may disappear.
    """
    for table in ('events', 'agent_logs', 'cockpit_events', 'mission_coordination_events',
                  'native_mission_claim_receipts', 'native_mission_work_receipts'):
        if table in before['tables']:
            require(table in after['tables'] and before['tables'][table]['columns'] == after['tables'][table]['columns'],
                    'historical audit schema changed')
            old = Counter(json.dumps(row, sort_keys=True, separators=(',', ':')) for row in before['tables'][table]['rows'])
            new = Counter(json.dumps(row, sort_keys=True, separators=(',', ':')) for row in after['tables'][table]['rows'])
            require(not old - new, 'historical audit row was removed or altered')


def workspace_files(owned):
    """Hash actual workspace/program/report files, excluding datastore technical locks.

    Directories and symlinks are not followed. Private DB bodies are covered by
    SQL snapshot; this is not a claim that no technical lock file was created.
    """
    result = {}
    for path in owned.root.rglob('*'):
        relative = path.relative_to(owned.root)
        if relative.parts[0] == '.swarm':
            continue
        require(not path.is_symlink(), 'legacy workspace symlink unqualified')
        if path.is_file():
            result[str(relative)] = digest(path.read_bytes())
    result['.swarm/engine-settings.json'] = digest((owned.root / '.swarm/engine-settings.json').read_bytes())
    return result


def live_owner(owned, peer, out, label):
    """Correlate actual live Popen peer with its recorded epoch and retain raw pulse row.

    A readiness message alone is not ownership; no claim of heartbeat advancement
    or stopped lease follows. Root/SDK/profile inputs separately bind executable.
    """
    require(peer.poll() is None, 'historical owner exited before controlled operation')
    observed = current().watch.observation(peer.pid)
    db = sqlite3.connect((owned.root / '.swarm/state.db').as_uri() + '?mode=ro', uri=True, timeout=0)
    primary = None
    try:
        row = db.execute('SELECT pid,process_stamp FROM native_mission_lease_epochs WHERE work_id=?', (owned.work,)).fetchone()
        require(row is not None and row[0] == peer.pid and row[1] == observed['stamp'], 'historical lease owner differs from owned peer')
    except BaseException as error:
        primary = error
        raise
    finally:
        current().close_database(db, primary)
    _, pulses = business_snapshot(owned)
    publish(out / (label + '-owner.json'), dict(process=observed, pulses=pulses, observed_live=True, lease_stopped_not_proven=True))


def replay_legacy_control(owned, out, original):
    """Replay the old writer's actual public start request with no new consent/event.

    The native stored canonical request is retained independently. Only the two
    owner heartbeat fields can advance during comparison, never Work or events.
    """
    before, _ = business_snapshot(owned)
    work = owned.ok(['work', 'show', owned.work])['work']
    db = sqlite3.connect((owned.root / '.swarm/state.db').as_uri() + '?mode=ro', uri=True, timeout=0)
    primary = None
    try:
        row = db.execute("SELECT request FROM events WHERE id=? AND work_id=? AND kind='mission.start'", ('legacy-profile-start', owned.work)).fetchone()
        require(row is not None, 'original mission start request absent')
        canonical = row[0]
    except BaseException as error:
        primary = error
        raise
    finally:
        current().close_database(db, primary)
    intent = json.loads(canonical)
    require(intent['event_id'] == original['event_id'] == 'legacy-profile-start', 'legacy control identity differs')
    owned.ok(['mission', 'start', owned.work], original)
    require(json_equal(owned.ok(['work', 'show', owned.work])['work'], work)
            and json_equal(business_snapshot(owned)[0], before), 'old control replay mutated business state')
    publish(out / 'old-control-canonical.original', canonical)
    publish(out / 'old-control-public-request.json', original)
    publish(out / 'old-control-replay.json', dict(status='PASS_CONTROL_REPLAY', work_and_events_unchanged=True))
    sys.stdout.write('PASS exact old control request replay preserves consent and events\n')
    sys.stdout.flush()


def resources(owned, agent):
    """Read reservation and any actual FIFO rows; never invent v1/v2 turn history."""
    db = sqlite3.connect((owned.root / '.swarm/state.db').as_uri() + '?mode=ro', uri=True, timeout=0)
    primary = None
    try:
        db.execute('BEGIN')
        row = db.execute('SELECT state,amount FROM reservations WHERE agent_id=?', (agent,)).fetchone()
        require(row is not None, 'legacy reservation absent')
        table = db.execute("SELECT name FROM sqlite_schema WHERE type='table' AND name='workspace_turns'").fetchone()
        turns = db.execute('SELECT * FROM workspace_turns WHERE agent_id=?', (agent,)).fetchall() if table else []
        require(not turns, 'historical v1/v2 unexpectedly has FIFO history')
        return dict(reservation=list(row), turns=[])
    except BaseException as error:
        primary = error
        raise
    finally:
        current().close_database(db, primary)


def refuse_and_cancel(owned, peer, state, out):
    """While old owner lives, refuse new v1/v2 grant then cancel/reconcile/replay publicly.

    Stop proves desired-only; cancellation is interrupted, not accepted/completed.
    Binding/body snapshots and providers are compared across refusal and replay.
    finish is deliberately outside this contract because it journals a real cycle.
    """
    agent = state['agent']
    queued = owned.ok(['agent', 'show', agent])['agent']
    require(queued['status'] == 'queued' and json_equal(queued['native_mission'], state['binding']), 'queued legacy Agent differs')
    require(not (owned.root / 'model-received.json').exists(), 'legacy provider already entered')
    live_owner(owned, peer, out, 'before-refusal')
    replay_legacy_control(owned, out, state['public_start_request'])
    before, _ = business_snapshot(owned)
    files = workspace_files(owned)
    work = owned.ok(['work', 'show', owned.work])['work']
    before_resources = resources(owned, agent)
    require(before_resources['reservation'][0] == 'reserved', 'legacy queued reservation was not held')
    publish(out / 'before-grant-business.json', before)
    publish(out / 'original-agent.json', queued)
    code, response = owned.call(['agent', 'run', agent])
    codes = current().refusal_codes(response)
    require(code == 2 and 'mission-agent-instance-required' in codes, 'legacy new grant did not retain exact instance-required refusal')
    require(json_equal(owned.ok(['agent', 'show', agent])['agent'], queued)
            and json_equal(business_snapshot(owned)[0], before) and workspace_files(owned) == files,
            'legacy grant refusal mutated business/provider state')
    publish(out / 'grant-refusal.json', dict(exit=code, codes=sorted(codes), business_unchanged=True,
            provider_started=False, new_grant=False))
    stop = owned.ok(['agent', 'stop', agent])
    stopped = owned.ok(['agent', 'show', agent])['agent']
    require(stop['desired'] == 'stop' and stop['termination_confirmed'] is False
            and stopped['status'] == 'queued' and stopped['desired'] == 'stop'
            and json_equal(stopped['native_mission'], state['binding']), 'legacy stop was not desired-only')
    require(json_equal(owned.ok(['work', 'show', owned.work])['work'], work)
            and json_equal(resources(owned, agent), before_resources), 'desired-only stop changed Work or resources')
    publish(out / 'desired-stop.json', dict(reply=stop, agent=stopped))
    reply = owned.ok(['agent', 'reconcile', agent])
    after = owned.ok(['agent', 'show', agent])['agent']
    require(reply['cancelled_before_start'] is True and reply['accepted_by_cancellation'] is False
            and after['status'] == 'interrupted' and after['desired'] == 'stop'
            and after['stop_kind'] == 'operateur' and isinstance(after['ended'], str) and bool(after['ended'])
            and json_equal(after['native_mission'], state['binding']), 'legacy reconciliation did not retain cancelled intention')
    after_work = owned.ok(['work', 'show', owned.work])['work']
    task = next(task for task in after_work['tasks'] if task['id'] == after['task_id'])
    before_task = next(task for task in work['tasks'] if task['id'] == after['task_id'])
    require(task['status'] == 'blocked' and task['attempts'][-1]['id'] == queued['attempt_id']
            and task['attempts'][-1]['status'] == 'interrupted'
            and json_equal(task['attempts'][:-1], before_task['attempts'][:-1]), 'legacy cancellation lost task attempt history')
    require(json_equal(resources(owned, agent), dict(reservation=['released', before_resources['reservation'][1]], turns=[])),
            'legacy cancellation did not preserve released reservation and absent FIFO')
    settled, _ = business_snapshot(owned)
    preserve_history(before, settled)
    publish(out / 'cancelled-business.json', settled)
    owned.ok(['agent', 'reconcile', agent])
    require(json_equal(business_snapshot(owned)[0], settled) and workspace_files(owned) == files,
            'legacy cancellation replay changed business or provider')
    live_owner(owned, peer, out, 'after-replay')
    publish(out / 'cancelled-agent.json', after)
    publish(out / 'cancelled-work.json', after_work)
    return after, after_work, files


def finish_peer(peer, stderr, ready, out):
    """Finish the owned old callback, retain original channels and real wait independently.

    Old SDK closes bindings but does not stop its persisted lease; cycle journal
    mutations after finish are legitimate observations, not refusal snapshot noise.
    Every independent transport/publication/close failure is retained and raised.
    """
    errors = []
    if peer is not None:
        if peer.stdin is not None:
            try:
                if peer.poll() is None:
                    peer.stdin.write(b'finish\n')
                    peer.stdin.flush()
            except BaseException as error:
                errors.append(error)
            try:
                peer.stdin.close()
            except BaseException as error:
                errors.append(error)
            peer.stdin = None
        try:
            tail, _ = peer.communicate()
            publish(out / 'legacy.stdout', ready + tail)
            publish(out / 'legacy-exit.json', dict(returncode=peer.returncode, actual_popen_wait=True,
                    persisted_lease_stop_not_proven=True))
            require(peer.returncode == 0, 'legacy owner action cycle or checked close failed')
        except BaseException as error:
            errors.append(error)
    if stderr is not None:
        try:
            stderr.close()
        except BaseException as error:
            errors.append(error)
    if errors:
        raise BaseExceptionGroup('legacy finish and independent cleanup', errors)


def validate_profiles(args):
    """Bind all actual normal/SDK origins before side effects, including current v4 witness.

    Old binaries are explicit and hashed, never deduced from cache filenames. Profiles
    are local qualification inputs, not proof that a specific operation has run.
    """
    normal = json.loads(args.profile.read_bytes())
    old = json.loads(args.old_profile.read_bytes())
    legacy = json.loads(args.legacy_profile.read_bytes())
    witness = json.loads(args.current_sdk_profile.read_bytes())
    require(normal['no_fixture_feature'] is True and digest(args.binary.read_bytes()) == normal['sha256'], 'current normal profile differs')
    require(old['no_fixture_feature'] is True and digest(args.old_binary.read_bytes()) == old['sha256'], 'old normal profile differs')
    require(legacy['engine_features_empty'] is True and digest(args.legacy_sdk.read_bytes()) == legacy['binary_sha256']
            and legacy['normal_profile_sha256'] == old['sha256'] and normal['sha256'] != old['sha256'], 'old SDK/normal linkage differs')
    require(witness['example'] == 'mission_watch_worker' and witness['engine_features_empty'] is True
            and witness['shared_workspace_features_match_normal'] is True and witness['test_profile'] is False
            and witness['normal_profile_sha256'] == normal['sha256']
            and digest(args.current_sdk.read_bytes()) == witness['binary_sha256'], 'current witness SDK/normal linkage differs')
    return normal, old, legacy, witness


def validate_bundle(manifest_path, helpers_root, schema, old_binary, expected_manifest_sha=None):
    """Verify the selected immutable historical helper inventory and old normal linkage.

    Every listed relative file/hash is checked before imports; selected root must
    be exactly manifest-relative. Inventory is provenance, not execution success.
    """
    raw = manifest_path.read_bytes()
    if expected_manifest_sha is not None:
        require(digest(raw) == expected_manifest_sha, 'helper manifest changed')
    manifest = json.loads(raw)
    require(type(manifest.get('schema_version')) is int and manifest['schema_version'] == 1, 'helper manifest schema differs')
    variant = manifest['variants']['v' + str(schema)]
    require(type(variant['binding_schema']) is int and variant['binding_schema'] == schema, 'helper variant version differs')
    base = manifest_path.parent.resolve()
    relative = Path(variant['helpers_root'])
    require(not relative.is_absolute() and '..' not in relative.parts, 'helper root escapes bundle')
    root = (base / relative).resolve()
    require(root.is_relative_to(base) and helpers_root.resolve() == root
            and variant['entrypoint'] == 'tests/rust_mission_task_profile.py:prepare', 'historical entrypoint differs')
    require(digest(old_binary.read_bytes()) == variant['old_binary']['sha256'], 'bundle old normal differs')
    files = {}
    for item in variant['files']:
        relative = Path(item['path'])
        require(not relative.is_absolute() and '..' not in relative.parts, 'helper input escapes bundle')
        path = (base / relative).resolve()
        require(path.is_relative_to(root) and path.is_file() and path not in files, 'helper input path differs')
        data = path.read_bytes()
        require(len(data) == item['bytes'] and digest(data) == item['sha256'], 'historical helper bytes differ')
        files[path] = item['sha256']
    require(files and not variant['missing_local_imports'], 'historical import graph is incomplete')
    return variant, files, digest(raw)


def loaded_helpers(files):
    """Require every actual rust_* import from the immutable selected inventory.

    Called before and after prepare; no current checkout fallback is permitted.
    This verifies imported source bytes, not a compiler graph or execution grant.
    """
    loaded = {}
    for name, module in tuple(sys.modules.items()):
        if name.startswith('rust_'):
            path = Path(getattr(module, '__file__', '')).resolve()
            require(path in files and digest(path.read_bytes()) == files[path], 'historical import used unlisted source')
            loaded[name] = dict(path=str(path), sha256=files[path])
    require('rust_mission_task_profile' in loaded and set(loaded) == {path.stem for path in files},
            'historical helper import closure differs from complete inventory')
    return loaded


def prepare_old_child(request_file):
    """Run only the archived setup in a separate interpreter with isolated module search.

    This branch precedes loading any current rust_* helper. It executes the real
    archived prepare using the supplied old normal, returns real Root/work/request,
    and publishes source hashes. No settings/private DB edits or provider account.
    """
    flags = python_runtime()
    import sysconfig
    request = json.loads(request_file.read_bytes())
    root = Path(request['helpers_root']).resolve()
    _, files, manifest_sha = validate_bundle(Path(request['manifest']), root, request['schema'],
        Path(request['old_binary']), request['manifest_sha256'])
    stdlib = Path(sysconfig.get_path('stdlib')).resolve()
    sys.path = [str(root / 'tests'), str(stdlib), str(stdlib / 'lib-dynload')]
    sys.dont_write_bytecode = True
    require(not any(name.startswith('rust_') for name in sys.modules), 'setup child loaded current helpers')
    configured = importlib.import_module('rust_mission_task_profile')
    loaded_helpers(files)
    out = Path(request['output']).resolve()
    require(out.is_dir() and not any(out.iterdir()), 'historical setup output is not fresh empty owned directory')
    owned = configured.prepare(Path(request['old_binary']).resolve(), out)
    loaded = loaded_helpers(files)
    actual_request = out / 'request.json'
    require(json.loads(actual_request.read_bytes()) == dict(root=str(owned.root), work=owned.work), 'historical setup request differs')
    configured.fixture.publish(out / 'setup-result.json', dict(root=str(owned.root), work=owned.work,
        request_file=str(actual_request), cache_root=str(owned.cache_root), helpers_manifest_sha256=manifest_sha,
        loaded_helpers=loaded, python_flags=flags, settings_sha256=digest((owned.root / '.swarm/engine-settings.json').read_bytes())))


def prepare_historical(args, out):
    """Invoke the archived setup child once, then observe the same Root through both CLIs.

    Source/binary/profile provenance is explicit. Settings bytes stay unchanged;
    public DTO extra fields are not mistaken for mutation. No live Root argument.
    """
    out.mkdir(mode=0o700)
    variant, files, manifest_sha = validate_bundle(args.old_helpers_manifest, args.old_helpers_root,
        args.binding_schema, args.old_binary)
    require(digest(args.legacy_sdk.read_bytes()) == variant['sdk']['sha256']
            and digest(args.legacy_profile.read_bytes()) == variant['sdk']['profile_sha256'], 'historical SDK differs from helper provenance')
    setup = out / 'archived-setup'
    setup.mkdir(mode=0o700)
    request = dict(manifest=str(args.old_helpers_manifest.resolve()), helpers_root=str(args.old_helpers_root.resolve()),
        manifest_sha256=manifest_sha, schema=args.binding_schema, old_binary=str(args.old_binary.resolve()), output=str(setup))
    publish(out / 'setup-input.json', request)
    argv = [sys.executable, '-B', str(Path(__file__).resolve()), '--prepare-old-child', str(out / 'setup-input.json')]
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')
    publish(out / 'setup-command.json', dict(argv=argv, cwd=str(args.old_helpers_root.resolve()),
        safe_environment=dict(PYTHONDONTWRITEBYTECODE='1'), global_watchdog=None))
    with (out / 'setup.stdout').open('xb') as stdout, (out / 'setup.stderr').open('xb') as stderr:
        completed = subprocess.run(argv, cwd=args.old_helpers_root.resolve(), env=env,
            stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
    publish(out / 'setup-exit.json', dict(exit=completed.returncode, actual_popen_wait=True))
    require(completed.returncode == 0, 'historical setup prerequisite failed')
    actual = json.loads((setup / 'setup-result.json').read_bytes())
    require(actual['helpers_manifest_sha256'] == manifest_sha, 'historical setup manifest correlation differs')
    # Import the current transport only AFTER the old interpreter has exited.
    owned = current().fixture.Fixture.__new__(current().fixture.Fixture)
    owned.root = Path(actual['root']).resolve()
    owned.work, owned.output, owned.label, owned.calls = actual['work'], out, 'historical-public', []
    owned.environment = dict(os.environ, CODEX_HOME=actual['cache_root'])
    owned.cache_root = Path(actual['cache_root'])
    owned.cache = owned.cache_root / 'models_cache.json'
    request_path = Path(actual['request_file']).resolve()
    require(request_path == (setup / 'request.json').resolve()
            and json.loads(request_path.read_bytes()) == dict(root=str(owned.root), work=owned.work), 'setup Root request attribution differs')
    require(digest((owned.root / '.swarm/engine-settings.json').read_bytes()) == actual['settings_sha256'], 'historical settings changed before observations')
    before, _ = business_snapshot(owned)
    owned.binary = args.old_binary.resolve()
    old_settings = owned.ok(['engine-settings', 'show'])
    old_work = owned.ok(['work', 'show', owned.work])['work']
    owned.binary = args.binary.resolve()
    new_settings = owned.ok(['engine-settings', 'show'])
    new_work = owned.ok(['work', 'show', owned.work])['work']
    require(new_settings['schema_version'] == old_settings['schema_version']
            and new_settings['revision'] == old_settings['revision']
            and all(key in new_settings['values'] and json_equal(value, new_settings['values'][key]) for key, value in old_settings['values'].items()),
            'new public settings read lost historical configuration')
    require(old_work['id'] == new_work['id'] == owned.work and old_work['revision'] == new_work['revision']
            and json_equal(business_snapshot(owned)[0], before)
            and digest((owned.root / '.swarm/engine-settings.json').read_bytes()) == actual['settings_sha256'],
            'old or new public observations mutated historical state')
    publish(out / 'settings.original.json', (owned.root / '.swarm/engine-settings.json').read_bytes())
    publish(out / 'old-new-prerequisite.json', dict(status='PASS_PUBLIC_OLD_NEW_PREREQUISITE',
        old_settings=old_settings, new_settings=new_settings, original_settings_sha256=actual['settings_sha256'],
        root=str(owned.root), work=owned.work, helpers_manifest_sha256=manifest_sha, loaded_helpers=actual['loaded_helpers']))
    # The marker request is the genuine archived producer output, copied as evidence only.
    publish(out / 'request.json', request_path.read_bytes())
    return owned


def run_legacy(args, out):
    """Prepare archived Root, operate while owner lives, finish/reap, then reopen cancellation.

    Checked old SDK cleanup failures stay independent. A full SQL freeze is only
    required before finish/replay, never across its legitimate cycle journal write.
    """
    owned = prepare_historical(args, out)
    peer = stderr = None
    ready = b''
    failures = []
    cancelled = work = files = closed_resources = settled = None
    try:
        stderr = (out / 'legacy.stderr').open('xb')
        argv = [str(args.legacy_sdk.resolve()), str(out / 'request.json')]
        publish(out / 'legacy-command.json', dict(argv=argv, global_watchdog=None, binding_schema=args.binding_schema))
        peer = subprocess.Popen(argv, env=owned.environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr)
        ready = peer.stdout.readline()
        publish(out / 'legacy-ready.original', ready)
        state = json.loads(ready)
        assert_binding(state, args.binding_schema)
        cancelled, work, files = refuse_and_cancel(owned, peer, state, out)
        closed_resources = resources(owned, cancelled['id'])
        settled, _ = business_snapshot(owned)
    except BaseException as error:
        failures.append(error)
    try:
        finish_peer(peer, stderr, ready, out)
    except BaseException as error:
        failures.append(error)
    if failures:
        raise BaseExceptionGroup('legacy action and original owner finish', failures)
    # Finishing may update cycle journal/lease rows; preserve, do not require equality.
    after_finish, pulses = business_snapshot(owned)
    preserve_history(settled, after_finish)
    require(json_equal(resources(owned, cancelled['id']), closed_resources), 'owner finish changed cancelled resources')
    publish(out / 'after-owner-finish-business.json', dict(snapshot=after_finish, pulses=pulses))
    reread = owned.ok(['agent', 'show', cancelled['id']])['agent']
    require(json_equal(reread, cancelled) and json_equal(owned.ok(['work', 'show', owned.work])['work'], work)
            and workspace_files(owned) == files, 'owner finish lost cancelled Agent Work or provider history')
    owned.ok(['agent', 'reconcile', cancelled['id']])
    require(json_equal(business_snapshot(owned)[0], after_finish) and workspace_files(owned) == files,
            'post-reopen cancellation replay changed state')
    publish(out / 'observed.json', dict(root=str(owned.root), work=owned.work, agent=cancelled,
        status='PASS_LEGACY_REFUSAL_AND_CANCELLATION', binding_schema=args.binding_schema,
        new_grant=False, legacy_executed_by_new_binary=False, legacy_provider_started=False,
        cancelled_terminal=True, old_completed_recovery_proven=False, lease_stopped_proven=False))
    sys.stdout.write(f'PASS historical v{args.binding_schema} intention refuses new grant; cancellation and replay preserve provenance\n')
    sys.stdout.flush()


def eof_control(args, out):
    """Fresh old queued intention retains historical EOF cause and actual owner exit.

    This tests protocol cleanup, not v4 grant. Original stdout/stderr and queued
    binding survive; a nonzero code alone is insufficient. No legacy queued provider
    is run; the archived setup has its own real inert prerequisite controls.
    """
    owned = prepare_historical(args, out)
    peer = stderr = None
    ready = b''
    errors = []
    try:
        stderr = (out / 'legacy.stderr').open('xb')
        argv = [str(args.legacy_sdk.resolve()), str(out / 'request.json')]
        publish(out / 'command.json', dict(argv=argv, global_watchdog=None, binding_schema=args.binding_schema))
        peer = subprocess.Popen(argv, env=owned.environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr)
        ready = peer.stdout.readline()
        publish(out / 'legacy-ready.original', ready)
        state = json.loads(ready)
        assert_binding(state, args.binding_schema)
    except BaseException as error:
        errors.append(error)
    if peer is not None:
        try:
            if peer.stdin is not None:
                peer.stdin.close()
                peer.stdin = None
        except BaseException as error:
            peer.stdin = None
            errors.append(error)
        try:
            tail, _ = peer.communicate()
            publish(out / 'legacy.stdout', ready + tail)
            publish(out / 'legacy-exit.json', dict(returncode=peer.returncode, actual_popen_wait=True, expected_failure=True))
        except BaseException as error:
            errors.append(error)
    if stderr is not None:
        try:
            stderr.close()
        except BaseException as error:
            errors.append(error)
    if errors:
        raise BaseExceptionGroup('EOF readiness and independent cleanup', errors)
    require(peer.returncode != 0 and b'legacy EOF cause retained' in (out / 'legacy.stderr').read_bytes(), 'legacy EOF original cause or owner exit missing')
    require(not (owned.root / 'model-received.json').exists(), 'legacy EOF started a provider')
    queued = owned.ok(['agent', 'show', state['agent']])['agent']
    require(queued['status'] == 'queued' and json_equal(queued['native_mission'], state['binding']), 'EOF did not preserve genuine queued binding')
    publish(out / 'result.json', dict(status='PASS', binding_schema=args.binding_schema,
        expected_failure=True, original_cause=True, owner_exited=True, legacy_provider_started=False,
        queued_intention_preserved=True, persisted_lease_stop_not_proven=True))
    label = 'legacy' if args.binding_schema == 1 else f'legacy v{args.binding_schema}'
    sys.stdout.write(f'PASS {label} EOF refuses with original cause and owner exit\n')
    sys.stdout.flush()


def fresh_v4_witness(args, out, normal):
    """Run the real maintained schema2/v4 probe before legacy setup with checked subreaper scope.

    Root/provider configuration is the same maintained inert fixture contract, but
    its Root is distinct from legacy's old-schema fixture. Only the real successful
    receipt is accepted; a manifest/PASS boolean cannot substitute for execution.
    """
    probe = current()
    return probe.watch.subreaper_scope(
        lambda: probe.probe_case(args.binary.resolve(), args.current_sdk.resolve(), normal,
            out / 'current-v4-witness', False, '', expected_binding_version=4, probe_schema_version=2),
        lambda error: publish(out / 'current-v4-witness-failure.json', dict(status='FAIL', traceback=''.join(traceback.format_exception(error)))),
        probe.watch.parent_death.subreaper)


def main():
    """Require real current v4 control first, then old grant refusal/cancel and EOF.

    Every normal/SDK/helper source is explicit; no old terminal completed/FIFOv3
    is invented. New run cannot consume a PASS boolean as its current witness.
    """
    flags = python_runtime()
    if len(sys.argv) == 3 and sys.argv[1] == '--prepare-old-child':
        prepare_old_child(Path(sys.argv[2]).resolve())
        return
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'old-binary', 'old-profile', 'legacy-sdk', 'legacy-profile',
                 'current-sdk', 'current-sdk-profile', 'old-helpers-root', 'old-helpers-manifest', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--binding-schema', type=int, choices=(1, 2), default=1)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(mode=0o700, exist_ok=False)
    try:
        publish(out / 'method.py', Path(__file__).read_bytes())
        normal, old, legacy, witness_profile = validate_profiles(args)
        _, helper_files, manifest_sha = validate_bundle(args.old_helpers_manifest, args.old_helpers_root, args.binding_schema, args.old_binary)
        source_paths = {Path(__file__).resolve(), args.old_helpers_manifest.resolve(), *helper_files}
        source_paths.update(Path(module.__file__).resolve() for name,module in tuple(sys.modules.items())
            if name.startswith('rust_') and getattr(module, '__file__', None))
        source_hashes = {str(path):digest(path.read_bytes()) for path in source_paths}
        publish(out / 'sources.json', source_hashes)
        publish(out / 'profiles.json', dict(current=normal, old=old, legacy=legacy, witness=witness_profile,
            safe_inputs={key:str(value.resolve()) if isinstance(value, Path) else value for key,value in vars(args).items()}, python_flags=flags, global_watchdog=None))
        witness = fresh_v4_witness(args, out, normal)
        require(witness['status'] == 'PASS_SAME_HOOK_WITNESS', 'fresh current v4 witness failed')
        sys.stdout.write('PASS current v4 worker witness before historical intention\n')
        sys.stdout.flush()
        run_legacy(args, out / 'legacy')
        eof_control(args, out / 'eof-control')
        require(digest(args.binary.read_bytes()) == normal['sha256']
                and digest(args.old_binary.read_bytes()) == old['sha256']
                and digest(args.legacy_sdk.read_bytes()) == legacy['binary_sha256']
                and digest(args.current_sdk.read_bytes()) == witness_profile['binary_sha256'], 'binary changed during legacy verification')
        require(all(digest(Path(path).read_bytes()) == sha for path,sha in source_hashes.items()), 'legacy helper or adapter source changed during verification')
        publish(out / 'result.json', dict(status='PASS_LEGACY_COMPATIBILITY_NO_NEW_AUTHORITY', binding_schema=args.binding_schema,
            current_v4_witness=witness, current_v4_execution=True, legacy_executed_by_new_binary=False, new_legacy_grant=False,
            cancelled_terminal_preserved=True, old_completed_recovery_proven=False, binding3_proven=False,
            paid_model=False, installed=False))
    except BaseException as primary:
        try:
            publish(out / 'failure.json', dict(status='FAIL', traceback=''.join(traceback.format_exception(primary))))
        except BaseException as publication:
            raise BaseExceptionGroup('legacy verification and failure publication', [primary, publication])
        raise


if __name__ == '__main__':
    main()
