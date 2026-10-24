#!/usr/bin/env python3
"""Real FIFO Binding3 refusal/cancellation using qualified old SDK04, without downgrade.

Fresh currentv4 witness precedes oldnormal200 setup. Original v3 FIFO is released by
public cancellation while its owner lives. EOF is qualified structurally before
spawn; no old completed recovery, paid provider, user Root or global watchdog.
"""
import argparse
from functools import lru_cache
import hashlib
import importlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import traceback


@lru_cache(maxsize=1)
def shared():
    """Load frozen generic helpers in controller only; old setup child never calls this."""
    import rust_mission_legacy_intent as legacy
    return legacy


def require(condition, message):
    """Refuse invalid fixture evidence with a static diagnostic, without private values."""
    if not condition:
        raise RuntimeError(message)


def digest(raw):
    """Bind exact bytes without claiming identity or acceptance."""
    return hashlib.sha256(raw).hexdigest()


def publish(path, value):
    """Retain owned evidence through the established checked helper; IO errors propagate."""
    shared().publish(path, value)


def runtime_flags():
    """Require enabled historical assert before effects; retain safe flags only."""
    require(sys.flags.optimize == 0, 'historical proof requires Python assertions enabled')
    return dict(optimize=sys.flags.optimize, dont_write_bytecode=sys.dont_write_bytecode)


def bundle_inputs(path, root, old_binary, expected_hash=None):
    """Verify immutable public14 helper inventory and actual oldnormal linkage.

    Source validation is not a successful setup. Reject path escape, mismatched
    byte count/hash or entrypoint before importing archived Python.
    """
    raw = path.read_bytes()
    require(expected_hash is None or digest(raw) == expected_hash, 'v3 helper manifest changed')
    value = json.loads(raw)
    require(type(value['schema_version']) is int and value['schema_version'] == 1
            and type(value['binding_schema']) is int and value['binding_schema'] == 3
            and value['entrypoint'] == 'tests/rust_planning_worker_public_flow.py:flow'
            and value['source_recipe_result']['status'] == 'PASS', 'v3 helper provenance differs')
    base = path.parent.resolve()
    relative = Path(value['helpers_root'])
    require(not relative.is_absolute() and '..' not in relative.parts
            and (base / relative).resolve() == root.resolve() and root.resolve().is_relative_to(base), 'v3 helper root escapes bundle')
    require(digest(old_binary.read_bytes()) == value['old_binary']['sha256'], 'v3 old normal differs')
    files = {}
    for row in value['files']:
        relative = Path(row['path'])
        require(not relative.is_absolute() and '..' not in relative.parts, 'v3 helper path escapes bundle')
        member = (base / relative).resolve()
        require(member.is_relative_to(root.resolve()) and member not in files and member.is_file(), 'v3 helper inventory path differs')
        content = member.read_bytes()
        require(len(content) == row['bytes'] and digest(content) == row['sha256'], 'v3 helper source differs')
        files[member] = row['sha256']
    require(files and not value['missing_local_imports'], 'v3 import graph incomplete')
    return value, files, digest(raw)


def old_imports(files):
    """Verify complete loaded rust_* set against immutable old files; no current fallback."""
    result = {}
    for name, module in tuple(sys.modules.items()):
        if name.startswith('rust_'):
            path = Path(getattr(module, '__file__', '')).resolve()
            require(path in files and digest(path.read_bytes()) == files[path], 'v3 setup imported unlisted source')
            result[name] = dict(path=str(path), sha256=files[path])
    require(set(result) == {path.stem for path in files}, 'v3 import closure differs')
    return result


def setup_child(request_path):
    """Execute archived flow against oldnormal in a separate interpreter/source search.

    Returns genuine Root/work/cache, not a manufactured Agent or Binding. Setup
    has its own inert planner controls; no queued-worker admission occurs here.
    """
    flags = runtime_flags()
    import sysconfig
    request = json.loads(request_path.read_bytes())
    _, files, sha = bundle_inputs(Path(request['manifest']), Path(request['helpers_root']), Path(request['old_binary']), request['manifest_sha256'])
    stdlib = Path(sysconfig.get_path('stdlib')).resolve()
    sys.path = [str(Path(request['helpers_root']) / 'tests'), str(stdlib), str(stdlib / 'lib-dynload')]
    sys.dont_write_bytecode = True
    require(not any(name.startswith('rust_') for name in sys.modules), 'v3 setup loaded current helpers')
    planning = importlib.import_module('rust_planning_worker_public_flow')
    old_imports(files)
    out = Path(request['output']).resolve()
    require(out.is_dir() and not any(out.iterdir()), 'v3 setup output not fresh owned directory')
    ready, owned = planning.flow(Path(request['old_binary']).resolve(), out, before_worker=True, return_fixture=True)
    require(ready['status'] == 'READY_FOR_NATIVE_WORKER' and ready['worker_started'] is False
            and ready['work_id'] == owned.work and ready['root'] == str(owned.root), 'v3 flow readiness differs')
    planning.fixture.publish(out / 'setup-result.json', dict(root=str(owned.root), work=owned.work, ready=ready,
        cache_root=str(owned.cache_root), helpers_manifest_sha256=sha, loaded_helpers=old_imports(files),
        python_flags=flags, settings_sha256=digest((owned.root / '.swarm/engine-settings.json').read_bytes())))


def prepare_old(args, out):
    """Prepare archived Root200, then require old/new public reads without settings adoption.

    Capture selected safe environment only. Child source imports and Root metadata
    come from actual old flow, not a user-supplied live Root argument.
    """
    out.mkdir(mode=0o700)
    manifest, _, sha = bundle_inputs(args.old_helpers_manifest, args.old_helpers_root, args.old_binary)
    require(digest(args.legacy_sdk.read_bytes()) == manifest['sdk']['sha256']
            and digest(args.legacy_profile.read_bytes()) == manifest['sdk']['profile_sha256'], 'v3 SDK differs from helper provenance')
    setup = out / 'archived-setup'
    setup.mkdir(mode=0o700)
    request = dict(manifest=str(args.old_helpers_manifest.resolve()), helpers_root=str(args.old_helpers_root.resolve()),
        manifest_sha256=sha, old_binary=str(args.old_binary.resolve()), output=str(setup))
    publish(out / 'setup-input.json', request)
    argv = [sys.executable, '-B', str(Path(__file__).resolve()), '--prepare-old-child', str(out / 'setup-input.json')]
    publish(out / 'setup-command.json', dict(argv=argv, cwd=str(args.old_helpers_root.resolve()), safe_environment=dict(PYTHONDONTWRITEBYTECODE='1'), global_watchdog=None))
    channels, errors, completed = [], [], None
    try:
        channels.append((out / 'setup.stdout').open('xb'))
        channels.append((out / 'setup.stderr').open('xb'))
        completed = subprocess.run(argv, cwd=args.old_helpers_root.resolve(), env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'),
            stdin=subprocess.DEVNULL, stdout=channels[0], stderr=channels[1])
    except BaseException as error:
        errors.append(error)
    finally:
        for stream in channels:
            try:
                stream.close()
            except BaseException as error:
                errors.append(error)
    if errors:
        raise BaseExceptionGroup('v3 archived setup and channel cleanup', errors)
    publish(out / 'setup-exit.json', dict(exit=completed.returncode, actual_popen_wait=True))
    require(completed.returncode == 0, 'v3 old setup prerequisite failed')
    actual = json.loads((setup / 'setup-result.json').read_bytes())
    require(actual['helpers_manifest_sha256'] == sha, 'v3 setup manifest differs')
    legacy = shared()
    owned = legacy.current().fixture.Fixture.__new__(legacy.current().fixture.Fixture)
    owned.root, owned.work, owned.output = Path(actual['root']).resolve(), actual['work'], out
    owned.label, owned.calls = 'v3-public', []
    owned.cache_root = Path(actual['cache_root'])
    owned.cache = owned.cache_root / 'models_cache.json'
    owned.environment = dict(os.environ, CODEX_HOME=str(owned.cache_root))
    before, _ = legacy.business_snapshot(owned)
    settings = owned.root / '.swarm/engine-settings.json'
    require(digest(settings.read_bytes()) == actual['settings_sha256'], 'v3 original settings changed')
    owned.binary = args.old_binary.resolve()
    old = owned.ok(['engine-settings', 'show'])
    old_work = owned.ok(['work', 'show', owned.work])['work']
    owned.binary = args.binary.resolve()
    new = owned.ok(['engine-settings', 'show'])
    new_work = owned.ok(['work', 'show', owned.work])['work']
    require(old['schema_version'] == new['schema_version'] and old['revision'] == new['revision']
            and all(key in new['values'] and legacy.json_equal(value, new['values'][key]) for key,value in old['values'].items())
            and old_work['id'] == new_work['id'] == owned.work and old_work['revision'] == new_work['revision']
            and legacy.json_equal(legacy.business_snapshot(owned)[0], before)
            and digest(settings.read_bytes()) == actual['settings_sha256'], 'v3 public observations mutated original state')
    publish(out / 'settings.original.json', settings.read_bytes())
    publish(out / 'old-new-prerequisite.json', dict(old=old, new=new, setup=actual, business_unchanged=True))
    return owned


def assert_binding3(binding):
    """Validate recorded v3 shape/parent/seq; this metadata alone supplies no authority."""
    require(isinstance(binding, dict) and type(binding.get('schema_version')) is int
            and binding['schema_version'] == 3 and 'root_instance' not in binding, 'genuine Binding3 shape differs')
    parent, seq = binding.get('parent_profile_sha256'), binding.get('workspace_turn_seq')
    require(type(parent) is str and len(parent) == 64 and all(char in '0123456789abcdef' for char in parent), 'v3 parent hash differs')
    require(type(seq) is str and seq.isascii() and seq.isdecimal() and 0 < int(seq) <= 2**63 - 1
            and str(int(seq)) == seq, 'v3 FIFO sequence differs')


def fifo_resources(owned, agent):
    """Read actual public turn/reservation and readonly append-only private history."""
    completion = shared().current().watch.completion
    turn = completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, agent['id'])
    money = completion.reservation(owned, agent['id'])
    db = sqlite3.connect((owned.root / '.swarm/state.db').as_uri() + '?mode=ro', uri=True, timeout=0)
    db.row_factory = sqlite3.Row
    primary = None
    try:
        db.execute('BEGIN')
        rows = [dict(row) for row in db.execute('SELECT * FROM workspace_turn_events WHERE turn_seq=? ORDER BY event_seq',
            (int(agent['native_mission']['workspace_turn_seq']),))]
    except BaseException as error:
        primary = error
        raise
    finally:
        shared().current().close_database(db, primary)
    require(rows and turn['seq'] == agent['native_mission']['workspace_turn_seq'], 'v3 FIFO journal or actual sequence missing')
    return dict(turn=turn, reservation=money, journal=rows)


def acquired(value, agent, work):
    """Require real held FIFO and journal ownership before refusal, without authorizing execution."""
    turn, last = value['turn'], value['journal'][-1]
    require(turn['state'] == 'acquired' and turn['agent'] == agent['id'] and turn['work'] == work
            and turn['task'] == agent['task_id'] and value['reservation'][0] == 'reserved'
            and turn['seq'] == agent['native_mission']['workspace_turn_seq'], 'v3 resources not held by original Agent')
    require(type(last['turn_seq']) is int and str(last['turn_seq']) == turn['seq']
            and last['state'] == 'acquired' and last['transition'] == 'acquired'
            and last['agent_id'] == turn['agent'] and last['work_id'] == work and last['task_id'] == turn['task']
            and last['workspace'] == turn['workspace'] and last['requested_at'] == turn['requested_at']
            and last['acquired_at'] == turn['acquired_at'] and last['released_at'] == '', 'v3 acquired history differs')


def released(before, after, agent):
    """Require one exact release of the old generation while preserving all acquired rows."""
    turn = after['turn']
    stable = ('seq', 'work', 'task', 'workspace', 'agent', 'requested_at', 'acquired_at')
    require(turn['state'] == 'released' and isinstance(turn['released_at'], str) and bool(turn['released_at'])
            and all(shared().json_equal(turn[key], before['turn'][key]) for key in stable)
            and shared().json_equal(after['reservation'], ['released', before['reservation'][1]])
            and len(after['journal']) == len(before['journal']) + 1
            and shared().json_equal(after['journal'][:-1], before['journal']), 'v3 release altered generation or old history')
    last = after['journal'][-1]
    require(type(last['turn_seq']) is int and last['state'] == 'released' and last['transition'] == 'released'
            and last['agent_id'] == agent['id'] and str(last['turn_seq']) == turn['seq'] and last['work_id'] == turn['work']
            and last['task_id'] == turn['task'] and last['workspace'] == turn['workspace']
            and last['requested_at'] == turn['requested_at'] and last['acquired_at'] == turn['acquired_at']
            and last['released_at'] == turn['released_at'] == agent['ended'], 'v3 exact released snapshot differs')


def eof_documents():
    """Build inspected SDK04 typed pre-spawn EOF shapes; no arbitrary message matching."""
    def node(labels):
        """Create one source-defined safe diagnostic node with no SQL/IO cause."""
        return dict(labels=labels, sqlite_code=None, io_errno=None)
    empty, platform, refusal = node([]), node(['runtime-platform']), node(['watch-fixture-input-protocol'])
    trace = dict(pid=None, phase='', supervisor=None, initial_supervisor=None, session_state=None, conductor=None,
        early_exit=None, spawned_publication_attempted=False, spawned_publication_confirmed=False)
    failure = dict(status='FAIL', causes=[[platform, platform, empty, dict(fixture_phase='watch-fixture-cycle'), empty, refusal]],
        child_ownership=trace, worker_not_stopped_by_fixture=True)
    cycle = dict(diagnostics=dict(action_started=True, action_succeeded=False,
        causes=[dict(refusal, phase='action', path=['root'], recorded_work=None)]), causes=[empty, refusal])
    return failure, cycle


def verify_eof(failure, cycle, code):
    """Accept exact typed EOF phase plus actual nonzero wait; every unknown cleanup branch fails.

    This is not old v1/v2's IO cause marker. No CLOSED or checked-close receipt is
    invented; persisted stop and actual process reap remain separate observations.
    """
    expected_failure, expected_cycle = eof_documents()
    require(type(code) is int and code != 0 and shared().json_equal(failure, expected_failure)
            and shared().json_equal(cycle, expected_cycle), 'v3 SDK EOF phase causes or ownership differ')


def canonical_start(owned, out):
    """Retain actual old canonical start/receipt provenance; never claim unavailable public replay."""
    db = sqlite3.connect((owned.root / '.swarm/state.db').as_uri() + '?mode=ro', uri=True, timeout=0)
    primary = None
    try:
        db.execute('BEGIN')
        row = db.execute("SELECT request FROM events WHERE work_id=? AND id=? AND kind='mission.start'",
            (owned.work, 'watch-worker-start')).fetchone()
        require(row is not None and isinstance(row[0], (str, bytes)), 'old v3 canonical start absent')
        raw = row[0].encode() if isinstance(row[0], str) else row[0]
    except BaseException as error:
        primary = error
        raise
    finally:
        shared().current().close_database(db, primary)
    publish(out / 'old-start-canonical.original', raw)
    publish(out / 'old-start-provenance.json', dict(sha256=digest(raw), public_start_request_published=False, public_start_replay_proven=False))


def cancel_v3(owned, peer, queued, out):
    """Refuse current grant, then stop/cancel/replay exact original FIFO3 while owner lives."""
    legacy = shared()
    agent = owned.ok(['agent', 'show', queued['agent_id']])['agent']
    assert_binding3(agent['native_mission'])
    require(agent['status'] == 'queued' and legacy.json_equal(agent['native_mission'], queued['native_mission']), 'v3 queued Agent differs')
    legacy.live_owner(owned, peer, out, 'before-refusal')
    canonical_start(owned, out)
    prior = fifo_resources(owned, agent)
    acquired(prior, agent, owned.work)
    before, _ = legacy.business_snapshot(owned)
    files = legacy.workspace_files(owned)
    work = owned.ok(['work', 'show', owned.work])['work']
    publish(out / 'original-agent.json', agent)
    publish(out / 'acquired-resources.json', prior)
    publish(out / 'before-grant-business.json', before)
    code, response = owned.call(['agent', 'run', agent['id']])
    require(code == 2 and 'mission-agent-instance-required' in legacy.current().refusal_codes(response), 'v3 grant lacks exact required-instance refusal')
    require(legacy.json_equal(owned.ok(['agent', 'show', agent['id']])['agent'], agent)
            and legacy.json_equal(legacy.business_snapshot(owned)[0], before)
            and legacy.json_equal(fifo_resources(owned, agent), prior) and legacy.workspace_files(owned) == files,
            'v3 refused grant mutated original state or provider')
    publish(out / 'grant-refusal.json', dict(exit=code, causes=sorted(legacy.current().refusal_codes(response)), business_unchanged=True, legacy_provider_started=False))
    stop = owned.ok(['agent', 'stop', agent['id']])
    stopped = owned.ok(['agent', 'show', agent['id']])['agent']
    require(stop['desired'] == 'stop' and stop['termination_confirmed'] is False and stopped['status'] == 'queued'
            and stopped['desired'] == 'stop' and legacy.json_equal(stopped['native_mission'], agent['native_mission'])
            and legacy.json_equal(fifo_resources(owned, agent), prior)
            and legacy.json_equal(owned.ok(['work', 'show', owned.work])['work'], work), 'v3 desired-only stop settled resources')
    publish(out / 'desired-stop.json', dict(reply=stop, agent=stopped))
    reply = owned.ok(['agent', 'reconcile', agent['id']])
    after = owned.ok(['agent', 'show', agent['id']])['agent']
    require(reply['cancelled_before_start'] is True and reply['accepted_by_cancellation'] is False
            and after['status'] == 'interrupted' and after['desired'] == 'stop' and after['stop_kind'] == 'operateur'
            and isinstance(after['ended'], str) and bool(after['ended'])
            and legacy.json_equal(after['native_mission'], agent['native_mission']), 'v3 cancellation lost original binding')
    released_resources = fifo_resources(owned, after)
    released(prior, released_resources, after)
    after_work = owned.ok(['work', 'show', owned.work])['work']
    task = next(task for task in after_work['tasks'] if task['id'] == after['task_id'])
    old_task = next(task for task in work['tasks'] if task['id'] == after['task_id'])
    require(task['status'] == 'blocked' and task['attempts'][-1]['id'] == agent['attempt_id']
            and task['attempts'][-1]['status'] == 'interrupted'
            and legacy.json_equal(task['attempts'][:-1], old_task['attempts'][:-1]), 'v3 cancellation altered old attempts')
    settled, _ = legacy.business_snapshot(owned)
    legacy.preserve_history(before, settled)
    owned.ok(['agent', 'reconcile', agent['id']])
    require(legacy.json_equal(legacy.business_snapshot(owned)[0], settled)
            and legacy.json_equal(fifo_resources(owned, after), released_resources)
            and legacy.workspace_files(owned) == files, 'v3 cancellation replay changed state')
    legacy.live_owner(owned, peer, out, 'after-replay')
    publish(out / 'released-resources.json', released_resources)
    publish(out / 'cancelled-business.json', settled)
    publish(out / 'cancelled-agent.json', after)
    publish(out / 'cancelled-work.json', after_work)
    return after, after_work, released_resources, settled, files


def stop_probe(peer, sdk_fd, identity, out):
    """Close the owned SDK stdin, reap it, retain every close/wait/publication failure.

    Normal EOF never sends resume or spawns a worker. If EOF pipe close fails,
    known pidfd/stamp may interrupt ONLY this SDK to avoid a stranded owner; that
    path stays FAIL, never an expected EOF or graceful close. Unknown identity
    cannot authorize a signal. No group signal or timeout is introduced.
    """
    import signal
    errors = []
    failed_pipe = False
    if peer.stdin is not None:
        try:
            peer.stdin.close()
        except BaseException as error:
            errors.append(error)
            failed_pipe = True
        peer.stdin = None
    if failed_pipe and peer.poll() is None:
        try:
            require(sdk_fd is not None and identity is not None, 'v3 cleanup lacks held SDK identity')
            observed = shared().current().watch.observation(peer.pid)
            require(observed['pid'] == identity['pid'] == peer.pid and observed['stamp'] == identity['stamp']
                    and observed['ppid'] == os.getpid(), 'v3 cleanup SDK identity changed')
            signal.pidfd_send_signal(sdk_fd, signal.SIGTERM)
        except BaseException as error:
            errors.append(error)
            # A raced exit may already be observable; still perform the actual wait.
            if peer.poll() is None:
                try:
                    publish(out / 'sdk-cleanup-unresolved.json', dict(status='FAIL_KNOWN_SDK_NOT_REAPED',
                        pid=peer.pid, stamp=identity['stamp'] if identity is not None else None,
                        pipe_close_failed=True, signal_not_confirmed=True, lease_stop_not_proven=True))
                except BaseException as publication:
                    errors.append(publication)
                raise BaseExceptionGroup('v3 EOF close and unresolved SDK cleanup', errors)
    try:
        peer.poll()
        peer.wait()
        publish(out / 'sdk-exit.json', dict(returncode=peer.returncode, actual_popen_wait=True, interrupted_due_pipe_failure=failed_pipe,
            checked_close_not_inferred=True))
    except BaseException as error:
        errors.append(error)
    if errors:
        raise BaseExceptionGroup('v3 EOF and independent cleanup', errors)
    return peer.returncode


def stopped_lease(owned):
    """Observe actual stopped_at after old SDK's stop attempt, never infer it from OS exit."""
    value = shared().current().supervision_snapshot(owned.root / '.swarm', owned.work)
    require(isinstance(value['stopped_at'], str) and bool(value['stopped_at']), 'v3 SDK did not record lease stop')
    return value


def run_old_probe(args, normal_old, out):
    """Own old SDK queued barrier through refusal/cancel, typed EOF/reap and public reopen.

    Current CLI65 is external to the SDK's honest oldnormal input. No resume,
    finish-probe or verify-provider is sent. Unknown cleanup keeps FAIL and Root.
    """
    legacy, watch = shared(), shared().current().watch
    owned = prepare_old(args, out)
    request = dict(kind='owned-inert-watch-worker-fixture', root=str(owned.root), work=owned.work,
        binary=str(args.old_binary.resolve()), binary_sha256=normal_old['sha256'], mode='probe',
        result=str(out / 'sdk-result.json'), spawned=str(out / 'spawned.json'), ready=str(out / 'unused-ready.json'),
        supervisor_stdout=str(out / 'supervisor.stdout'), supervisor_stderr=str(out / 'supervisor.stderr'),
        root_instance_probe=dict(schema_version=1, queued=str(out / 'queued.json'), observation=str(out / 'unused-observation.json')))
    publish(out / 'request.json', request)
    publish(owned.root / '.swarm/verification-watch-worker.json', request)
    peer = identity = None
    sdk_fd = notify_fd = None
    channels, errors = [], []
    cancelled = None
    try:
        channels.append(watch.spool(out / 'sdk.stdout'))
        channels.append(watch.spool(out / 'sdk.stderr'))
        notify_fd = watch.watch_directory(out)
        argv = [str(args.legacy_sdk.resolve()), str(out / 'request.json')]
        publish(out / 'sdk-command.json', dict(argv=argv, global_watchdog=None))
        peer = subprocess.Popen(argv, env=owned.environment, stdin=subprocess.PIPE, stdout=channels[0], stderr=channels[1])
        identity = watch.observation(peer.pid)
        sdk_fd = watch.attach(identity)
        queued = watch.wait_json(out / 'queued.json', peer, sdk_fd, notify_fd)
        require(queued['status'] == 'ROOT_INSTANCE_PROBE_QUEUED' and queued['agent_status'] == 'queued'
                and queued['work'] == owned.work and queued['agent_id'] == 'watch-worker-agent'
                and queued['original_binding_matches_open_store'] is True and queued['original_binding_retained'] is True
                and queued['supervisor_spawned'] is False and queued['directory_observation_is_authority'] is False
                and queued['conductor']['pid'] == identity['pid'] and queued['conductor']['stamp'] == identity['stamp'], 'v3 actual queued barrier differs')
        assert_binding3(queued['native_mission'])
        cancelled = cancel_v3(owned, peer, queued, out)
    except BaseException as error:
        errors.append(error)
    if peer is not None:
        try:
            stop_probe(peer, sdk_fd, identity, out)
        except BaseException as error:
            errors.append(error)
    for stream in channels:
        try:
            stream.flush()
            os.fsync(stream.fileno())
        except BaseException as error:
            errors.append(error)
        try:
            stream.close()
        except BaseException as error:
            errors.append(error)
    for descriptor in (sdk_fd, notify_fd):
        if descriptor is not None:
            try:
                os.close(descriptor)
            except BaseException as error:
                errors.append(error)
    if errors:
        raise BaseExceptionGroup('v3 queued operation and owner cleanup', errors)
    failure = json.loads((out / 'sdk-result.failure.json').read_bytes())
    cycle = json.loads((out / 'sdk-result.cycle-failure.json').read_bytes())
    verify_eof(failure, cycle, peer.returncode)
    require(not any((out / name).exists() for name in ('spawned.json', 'supervisor.stdout', 'supervisor.stderr', 'sdk-result.json')),
            'v3 SDK unexpectedly spawned or claimed successful probe')
    after, work, released_resources, settled, files = cancelled
    stop = stopped_lease(owned)
    post, pulses = legacy.business_snapshot(owned)
    legacy.preserve_history(settled, post)
    require(legacy.json_equal(owned.ok(['agent', 'show', after['id']])['agent'], after)
            and legacy.json_equal(owned.ok(['work', 'show', owned.work])['work'], work)
            and legacy.json_equal(fifo_resources(owned, after), released_resources)
            and legacy.workspace_files(owned) == files, 'v3 owner finish changed cancelled intention/history/resources')
    owned.ok(['agent', 'reconcile', after['id']])
    require(legacy.json_equal(legacy.business_snapshot(owned)[0], post) and legacy.workspace_files(owned) == files,
            'v3 reopened cancellation replay mutated state')
    publish(out / 'post-eof-business.json', dict(snapshot=post, pulses=pulses, stopped_lease=stop))
    publish(out / 'result.json', dict(status='PASS_GENUINE_V3_NO_NEW_AUTHORITY', agent=after, root=str(owned.root),
        released_resources=released_resources, owner_reaped=True, expected_eof_typed=True, old_worker_spawned=False,
        sdk_graceful_closed=False, persisted_lease_stopped=True, public_start_replay_proven=False,
        old_completed_recovery_proven=False, new_generation_replay_proven=False))
    sys.stdout.write('PASS genuine historical v3 grant refused; exact FIFO cancellation and replay preserved\n')
    sys.stdout.write('PASS old v3 SDK EOF typed action refusal and actual reap preserve cancelled provenance\n')
    sys.stdout.flush()


def main():
    """Run fresh current v4 witness then genuine old v3 refusal/cancel/EOF, all inputs explicit.

    No imported PASS can replace a witness, no old SDK is recompiled, and no
    current metadata is downgraded. Every failure retains original evidence.
    """
    flags = runtime_flags()
    if len(sys.argv) == 3 and sys.argv[1] == '--prepare-old-child':
        setup_child(Path(sys.argv[2]).resolve())
        return
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'old-binary', 'old-profile', 'legacy-sdk', 'legacy-profile',
                 'current-sdk', 'current-sdk-profile', 'old-helpers-root', 'old-helpers-manifest', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(mode=0o700, exist_ok=False)
    try:
        legacy = shared()
        normal, old, sdk, current_sdk = legacy.validate_profiles(args)
        _, files, manifest_sha = bundle_inputs(args.old_helpers_manifest, args.old_helpers_root, args.old_binary)
        sources = {str(Path(__file__).resolve()): digest(Path(__file__).read_bytes()),
                   str(args.old_helpers_manifest.resolve()): manifest_sha}
        sources.update({str(path): sha for path,sha in files.items()})
        sources.update({str(Path(module.__file__).resolve()): digest(Path(module.__file__).read_bytes())
            for name,module in tuple(sys.modules.items()) if name.startswith('rust_') and getattr(module, '__file__', None)})
        publish(out / 'method.py', Path(__file__).read_bytes())
        publish(out / 'inputs.json', dict(profiles=dict(current=normal, old=old, sdk=sdk, current_sdk=current_sdk),
            safe_inputs={key:str(value.resolve()) for key,value in vars(args).items()}, python_flags=flags,
            source_hashes=sources, global_watchdog=None))
        witness = legacy.fresh_v4_witness(args, out, normal)
        require(witness['status'] == 'PASS_SAME_HOOK_WITNESS', 'v3 control current witness failed')
        sys.stdout.write('PASS current v4 worker witness before historical v3 intention\n')
        sys.stdout.flush()
        run_old_probe(args, old, out / 'legacy-v3')
        require(all(digest(Path(path).read_bytes()) == sha for path,sha in sources.items()), 'v3 verification source changed')
        require(digest(args.binary.read_bytes()) == normal['sha256'] and digest(args.old_binary.read_bytes()) == old['sha256']
                and digest(args.legacy_sdk.read_bytes()) == sdk['binary_sha256']
                and digest(args.current_sdk.read_bytes()) == current_sdk['binary_sha256'], 'v3 verification binary changed')
        publish(out / 'result.json', dict(status='PASS_GENUINE_V3_COMPATIBILITY_NO_NEW_AUTHORITY', current_v4_witness=witness,
            binding_schema=3, current_v4_execution=True, old_worker_spawned=False, current_grant_accepted=False,
            cancellation_replay=True, public_start_replay_proven=False, old_completed_recovery_proven=False,
            new_generation_replay_proven=False, installed=False, paid_model=False))
    except BaseException as primary:
        try:
            publish(out / 'failure.json', dict(status='FAIL', traceback=''.join(traceback.format_exception(primary))))
        except BaseException as publication:
            raise BaseExceptionGroup('v3 proof and failure publication', [primary, publication])
        raise


if __name__ == '__main__':
    main()
