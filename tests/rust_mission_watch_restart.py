#!/usr/bin/env python3
"""W2: continue a genuine queued worker under a new public watch generation.

A separately qualified SDK creates the old intention through real engine APIs,
then closes its lease/Root before any provider. A second qualified SDK exercises
real stale/pause/replay/conflict/stopped-lease calls, leaving one adoption receipt.
The public watch must preserve that prefix and reuse the original
Agent, attempt, FIFO turn and reservation. Only owned inert Roots/providers are
permitted; SQL below observes immutable ledgers and never seeds authority.
"""
import argparse
import base64
import json
import os
from pathlib import Path
import select
import selectors
import signal
import socket
import sqlite3
import subprocess
import sys
import traceback

import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as planning
import rust_mission_watch_public as public
import rust_mission_watch_worker as worker
import rust_workspace_fifo_completion as completion
import rust_native_parent_death as parent_death

check = worker.check


def typed_cell(value):
    """Preserve every observed SQLite scalar's type and exact BLOB/float bytes."""
    if value is None:
        return ['null', None]
    if type(value) is int:
        return ['integer', value]
    if type(value) is float:
        return ['real', value.hex()]
    if isinstance(value, bytes):
        return ['blob', base64.b64encode(value).decode('ascii')]
    check(type(value) is str, 'unexpected SQLite cell type')
    return ['text', value]


def admissions(owned):
    """Observe all Work admission requests and reservations in one readonly snapshot.

    Rows preserve ordering, duplicate multiplicity and cell types; no absent row
    becomes a successful empty admission. Separate costs/CLI reads are not a
    transaction with this snapshot, and no global mutation absence is claimed.
    """
    database = sqlite3.connect((owned.root / '.swarm/state.db').as_uri() + '?mode=ro', uri=True, timeout=0)
    try:
        database.execute('BEGIN')
        agents = database.execute('SELECT id,task_id,cwd,request FROM agents WHERE work_id=? ORDER BY id', (owned.work,)).fetchall()
        reservations = database.execute('SELECT agent_id,work_id,amount,state FROM reservations WHERE work_id=? ORDER BY agent_id', (owned.work,)).fetchall()
        value = dict(agents=[[typed_cell(item) for item in row] for row in agents],
                     reservations=[[typed_cell(item) for item in row] for row in reservations])
    except BaseException as primary:
        try:
            database.close()
        except BaseException as cleanup:
            raise BaseExceptionGroup('admission observation/close', [primary, cleanup])
        raise
    database.close()
    return value


def verify_same_admission(before, active, original, running, costs_before, costs_active, turn_before, turn_active):
    """Require byte/type-identical admission/reserve/cost history before provider output.

    Known Agent registration fields may change, but id/attempt/task/native mission
    provenance must remain the original admission. This does not authorize reuse;
    the separate new-generation engine adoption receipt must be verified too.
    """
    check(before == active and bool(before['agents']) and bool(before['reservations']),
          'restart duplicated or changed admission/reservation history')
    for key in ('id', 'attempt_id', 'task_id', 'native_mission'):
        check(key in original and key in running
              and json.dumps(original[key], sort_keys=True, allow_nan=False) == json.dumps(running[key], sort_keys=True, allow_nan=False),
              'restart changed original Agent attempt or mission provenance')
    check(original['status'] == 'queued' and running['status'] == 'running',
          'restart did not advance the same actual queued Agent')
    check(json.dumps(costs_before, sort_keys=True, allow_nan=False)
          == json.dumps(costs_active, sort_keys=True, allow_nan=False),
          'restart changed reported attempt or cost history before output')
    check(turn_before == turn_active and turn_before['state'] == 'acquired',
          'restart replaced or released the original FIFO turn')


def verify_terminal_history(before, final, agent_id):
    """Require no second admission and exactly one reserved→estimated transition.

    Other recorded reservations keep their exact typed values. Estimated amount
    is retained, never reset/refunded. This is fixture ledger observation only.
    """
    check(before['agents'] == final['agents'], 'completion created a second admission')
    expected = json.loads(json.dumps(before['reservations']))
    matching = [row for row in expected if row[0] == ['text', agent_id]]
    check(len(matching) == 1 and matching[0][3] == ['text', 'reserved'],
          'original target reservation missing or not reserved')
    matching[0][3] = ['text', 'estimated']
    check(expected == final['reservations'], 'completion reservation history differs from one settlement')


def verify_seed(value, work):
    """Validate the actual SDK producer's queued-v4/stopped-owner declaration.

    Public Agent/SQL/kernel observations remain separate requirements; the SDK
    explicitly does not claim per-reservation SQL verification or launch permission.
    """
    check(type(value) is dict and type(value.get('schema_version')) is int
          and value['schema_version'] == 1 and value.get('status') == 'QUEUED_RESTART_SEED'
          and value.get('work') == work, 'restart seed schema/Work mismatch')
    for key in ('agent_id', 'task_id', 'attempt_id'):
        check(type(value.get(key)) is str and bool(value[key]), 'restart seed coordinates missing')
    check(type(value.get('generation')) is int and value['generation'] > 0,
          'restart seed original generation invalid')
    binding = value.get('binding')
    check(type(binding) is dict and type(binding.get('schema_version')) is int
          and binding['schema_version'] == 4 and binding.get('generation') == value['generation'],
          'restart seed genuine Binding4 missing')
    for key in ('provider_spawn_called', 'agent_cancelled', 'consumption_reset',
                'observation_is_permission', 'reservation_rows_verified'):
        check(value.get(key) is False, 'restart seed overclaimed or changed worker state')
    for key in ('lease_released', 'lease_closed', 'root_closed', 'worker_projections_unchanged'):
        check(value.get(key) is True, 'restart seed cleanup/projection proof incomplete')
    check(value.get('before_conductor_stop') == value.get('after_conductor_stop')
          and type(value.get('before_conductor_stop')) is dict,
          'restart seed conductor stop changed worker projections')
    return value


def seed_queue(owned, sdk, out):
    """Run the qualified real SDK producer on an explicit marker-correlated fixture.

    Its process must exit0 after closing the old lease/Root, without starting a
    provider. Retain original stdout/stderr/exit before interpreting producer JSON.
    """
    request = dict(schema_version=1, kind='owned-inert-mission-watch-restart-seed',
                   root=str(owned.root), work=owned.work, result=str(out / 'seed-result.json'))
    path = out / 'seed-request.json'
    fixture.publish(path, request)
    fixture.publish(owned.root / '.swarm/verification-mission-watch-restart-seed.json', request)
    argv = [str(sdk)]
    fixture.publish(out / 'seed-command.json', dict(argv=argv, input_environment_key='SWARM_MISSION_WATCH_RESTART_SEED',
                   input_environment_path=str(path), global_watchdog=None))
    environment = dict(owned.environment, SWARM_MISSION_WATCH_RESTART_SEED=str(path))
    result = subprocess.run(argv, env=environment, stdin=subprocess.DEVNULL, capture_output=True)
    fixture.publish(out / 'seed.stdout', result.stdout)
    fixture.publish(out / 'seed.stderr', result.stderr)
    fixture.publish(out / 'seed-exit.json', dict(exit=result.returncode))
    check(result.returncode == 0, 'actual restart seed producer failed; original channels retained')
    return verify_seed(json.loads(Path(request['result']).read_bytes()), owned.work)



def verify_adoption_api_result(value, work, agent_id):
    """Require all real replay/refusal checks in the qualified SDK's fixed corpus.

    Hash projections are sequential public observations, not per-row ledger or
    transferable permission. The caller separately verifies every actual admission
    and reservation row. Missing/empty diagnostics or replay recovery never pass.
    """
    check(type(value) is dict and type(value.get('schema_version')) is int
          and value['schema_version'] == 1 and value.get('status') == 'ADOPTION_REFUSALS_VERIFIED'
          and value.get('work') == work and value.get('agent_id') == agent_id,
          'adoption API checker identity/schema mismatch')
    check(type(value.get('generation')) is int and value['generation'] > 0
          and type(value.get('adoption_event_id')) is str and bool(value['adoption_event_id'])
          and value.get('created') is True and value.get('exact_replay_created') is False
          and value.get('recovery_replay_created') is False,
          'adoption API checker positive/replay facts invalid')
    for key in ('reservation_rows_verified', 'provider_spawn_called', 'agent_cancelled',
                'consumption_reset', 'observation_is_permission'):
        check(value.get(key) is False, 'adoption API checker overclaimed or changed runtime')
    for key in ('lease_released', 'lease_closed', 'root_closed'):
        check(value.get(key) is True, 'adoption API checker cleanup incomplete')
    corpus = [('stale-revision', 'revision_conflict'), ('paused-permission', 'planning-mission-paused'),
              ('positive-adoption', None), ('exact-replay', None),
              ('replay-conflict', 'mission-adoption-replay-conflict'), ('recovery-replay', None),
              ('same-epoch-new-event', 'mission-adoption-new-lease-required'), ('stopped-lease', 'mission-lease-lost')]
    checks = value.get('checks')
    check(type(checks) is list and len(checks) == len(corpus), 'adoption API checker corpus incomplete')
    for item, (name, expected_code) in zip(checks, corpus):
        check(type(item) is dict and item.get('name') == name
              and item.get('public_projections_unchanged') is True
              and item.get('reservation_rows_verified') is False,
              'adoption API checker check identity/projection scope invalid')
        before, after = item.get('before'), item.get('after')
        check(type(before) is dict and json.dumps(before, sort_keys=True, allow_nan=False)
              == json.dumps(after, sort_keys=True, allow_nan=False),
              'adoption API checker operation changed original projections')
        for key in ('agent_sha256', 'work_sha256', 'controls_sha256'):
            digest = before.get(key)
            check(type(digest) is str and len(digest) == 64 and all(byte in '0123456789abcdef' for byte in digest),
                  'adoption API checker projection digest invalid')
        for key in ('queue_turn_count', 'receipt_count'):
            check(type(before.get(key)) is int and before[key] >= 0,
                  'adoption API checker projection count invalid')
        check(before.get('reservation_rows_verified') is False, 'adoption API projection overclaimed ledger')
        if expected_code is not None:
            causes = item.get('cause_codes')
            check(item.get('verdict') == 'REFUSED' and item.get('expected_code') == expected_code
                  and type(causes) is list and bool(causes)
                  and any(type(node) is dict and type(node.get('labels')) is list and expected_code in node['labels'] for node in causes),
                  'adoption API checker concrete refusal cause missing')
        else:
            check(item.get('verdict') == 'ADOPTED' and item.get('event_id') == value['adoption_event_id']
                  and type(item.get('generation')) is int and item['generation'] == value['generation']
                  and item.get('created') is (name == 'positive-adoption'),
                  'adoption API checker created/replay/recovery outcome invalid')
    return value


def check_adoption_api(owned, sdk, agent_id, out):
    """Exercise the qualified replay/refusal SDK on the original owned intention.

    This fixture may append one adoption receipt and explicitly mutate public
    pause controls. It cannot admit another Agent, reset consumption or spawn.
    The real output contract is checked before any subsequent watch execution.
    """
    request = dict(schema_version=1, kind='owned-inert-mission-adoption-refusals', root=str(owned.root),
                   work=owned.work, agent_id=agent_id, result=str(out / 'adoption-api-result.json'))
    path = out / 'adoption-api-request.json'
    fixture.publish(path, request)
    fixture.publish(owned.root / '.swarm/verification-mission-adoption-refusals.json', request)
    fixture.publish(out / 'adoption-api-command.json', dict(argv=[str(sdk)],
                    input_environment_key='SWARM_MISSION_ADOPTION_REFUSALS', input_environment_path=str(path)))
    result = subprocess.run([str(sdk)], env=dict(owned.environment, SWARM_MISSION_ADOPTION_REFUSALS=str(path)),
                            stdin=subprocess.DEVNULL, capture_output=True)
    fixture.publish(out / 'adoption-api.stdout', result.stdout)
    fixture.publish(out / 'adoption-api.stderr', result.stderr)
    fixture.publish(out / 'adoption-api-exit.json', dict(exit=result.returncode))
    check(result.returncode == 0, 'actual adoption API checker failed; original channels retained')
    return verify_adoption_api_result(json.loads(Path(request['result']).read_bytes()), owned.work, agent_id)


def read_adoptions(owned, agent_id):
    """Read every actual append-only adoption row for this owned Agent.

    Missing table is observed legacy absence before first adoption, never an
    authority shortcut. SQL/JSON errors refuse and original bytes are retained in
    private evidence. This observer does not validate engine rights independently.
    """
    database = sqlite3.connect((owned.root / '.swarm/state.db').as_uri() + '?mode=ro', uri=True, timeout=0)
    try:
        database.execute('BEGIN')
        exists = database.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='native_mission_worker_adoptions'").fetchall()
        if not exists:
            rows = []
        else:
            rows = database.execute('SELECT work_id,agent_id,event_id,generation,previous_generation,request,receipt FROM native_mission_worker_adoptions WHERE work_id=? AND agent_id=? ORDER BY generation,event_id', (owned.work, agent_id)).fetchall()
        value = [dict(work=row[0], agent=row[1], event_id=row[2], generation=row[3], previous_generation=row[4],
                      request=json.loads(row[5]), receipt=json.loads(row[6]),
                      request_raw_base64=base64.b64encode(row[5]).decode('ascii'),
                      receipt_raw_base64=base64.b64encode(row[6]).decode('ascii')) for row in rows]
    except BaseException as primary:
        try:
            database.close()
        except BaseException as cleanup:
            raise BaseExceptionGroup('adoption journal observation/close', [primary, cleanup])
        raise
    database.close()
    return value


def verify_restart_events(raw, work, agent_id, original_generation, generation, rows, prefix=()):
    """Check one real adoption event/row, followed by exactly one normal helper.

    Exact original Binding4/attempt/ledger preservation is verified separately.
    Event and receipt are observations, not substitute authority for the native
    gate. No provider-start verification is inferred from either publication.
    """
    events = [json.loads(line) for line in raw.splitlines() if line.strip()]
    check(bool(events) and all(type(event) is dict and type(event.get('schema_version')) is int
          and event['schema_version'] == 1 and event.get('work') == work for event in events),
          'restart watch JSONL empty or schema/Work mismatch')
    adopted = [event for event in events if event.get('event') == 'mission-worker-adopted']
    check(len(adopted) == 1 and adopted[0].get('agent_id') == agent_id
          and type(adopted[0].get('generation')) is int and adopted[0]['generation'] == generation
          and adopted[0].get('created') is True
          and adopted[0].get('provider_start_verified') is False and adopted[0].get('mission_complete') is False
          and adopted[0].get('observation_is_permission') is False,
          'restart event did not prove one observed new-generation adoption')
    check(len(rows) == len(prefix) + 1 and rows[:len(prefix)] == list(prefix),
          'restart adoption journal missing duplicated or rewrote prior receipts')
    previous = prefix[-1]['generation'] if prefix else original_generation
    row = rows[-1]
    check(row['work'] == work and row['agent'] == agent_id and row['event_id'] == adopted[0].get('event_id')
          and row['generation'] == generation and row['previous_generation'] == previous,
          'restart journal header does not match event/original generation')
    request, receipt = row['request'], row['receipt']
    check(type(request.get('schema_version')) is int and request['schema_version'] == 1 and request.get('agent_id') == agent_id
          and request.get('event_id') == row['event_id'], 'restart journal original request mismatch')
    check(type(receipt.get('schema_version')) is int and receipt['schema_version'] == 1 and receipt.get('work') == work and receipt.get('agent') == agent_id
          and receipt.get('event_id') == row['event_id'] and receipt.get('original_generation') == original_generation
          and receipt.get('previous_generation') == previous and receipt.get('generation') == generation,
          'restart journal receipt chain mismatch')
    without_adoption = b'\n'.join(json.dumps(event).encode() for event in events if event.get('event') != 'mission-worker-adopted')
    public.public_watch_events(without_adoption, work, agent_id, 'sigint')
    adopted_position = next(index for index, event in enumerate(events) if event.get('event') == 'mission-worker-adopted')
    started_position = next(index for index, event in enumerate(events) if event.get('event') == 'mission-worker-started')
    check(adopted_position < started_position, 'restart helper preceded adoption publication')
    return events


def run_api_case(binary, sdk, refusal_sdk, out):
    """Test real refusals/replay separately from the fresh launch witness.

    Pause/unpause intentionally advances Work revision; it never refreshes the
    retained execution context. This Root is not reused as a positive provider
    witness. The eight authority API checks do not establish a launch permission.
    SQL observes every reservation/admission; no private mutation or provider.
    """
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    _, owned = planning.flow(binary, out / 'planning', before_worker=True, return_fixture=True)
    root = owned.root
    nonce = fixture.digest(('api-refusals' + str(root)).encode())
    fixture.publish(root / 'codex', worker.provider_program(nonce).encode())
    (root / 'codex').chmod(0o700)
    markers_before = completion.provider_files(root)
    seed = seed_queue(owned, sdk, out)
    original = owned.ok(['agent', 'show', seed['agent_id']])['agent']
    before = admissions(owned)
    costs_before = owned.ok(['costs', 'show', owned.work])
    turn_before = completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, original['id'])
    checker = check_adoption_api(owned, refusal_sdk, original['id'], out)
    prefix = read_adoptions(owned, original['id'])
    check(len(prefix) == 1 and prefix[0]['generation'] == checker['generation']
          and prefix[0]['event_id'] == checker['adoption_event_id']
          and prefix[0]['receipt']['original_generation'] == seed['generation'],
          'adoption checker did not create exactly one real original-chain receipt')
    after_checker = admissions(owned)
    after_agent = owned.ok(['agent', 'show', original['id']])['agent']
    after_costs = owned.ok(['costs', 'show', owned.work])
    after_turn = completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, original['id'])
    check(before == after_checker
          and json.dumps(original, sort_keys=True, allow_nan=False) == json.dumps(after_agent, sort_keys=True, allow_nan=False)
          and turn_before == after_turn
          and json.dumps(costs_before, sort_keys=True, allow_nan=False) == json.dumps(after_costs, sort_keys=True, allow_nan=False),
          'adoption API checker changed actual Agent admission FIFO reservation or consumption')
    check(completion.provider_files(root) == markers_before, 'adoption API checker unexpectedly started provider')
    fixture.publish(out / 'adoption-api-observed.json', dict(admissions=after_checker,
                    prefix=prefix, costs=after_costs, turn=after_turn,
                    original_agent_unchanged=True, no_second_admission=True, reservation_rows_verified=True))
    result = dict(status='PASS', root=str(root), work=owned.work, checker=checker,
                  original_generation=seed['generation'], original_agent_unchanged=True,
                  reservation_rows_verified=True, admission_rows_unchanged=True,
                  provider_started=False, execution_context_refreshed=False,
                  used_as_positive_launch_witness=False)
    fixture.publish(out / 'result.json', result)
    return result


def verify_fresh_execution_revision(admission_rows, agent_id, work_revision):
    """Refuse a destroyed positive witness before launching any helper.

    The real retained Launch protocol advances Work by one on admission. This
    checks only that existing execution precondition, not rights or permission;
    pause/unpause cannot refresh the original context by restoring control values.
    """
    rows = [row for row in admission_rows['agents'] if row[0] == ['text', agent_id]]
    check(len(rows) == 1 and rows[0][-1][0] == 'blob', 'fresh revision witness requires exact original request')
    request = json.loads(base64.b64decode(rows[0][-1][1], validate=True))
    revision = request.get('expected_revision')
    check(type(revision) is int and type(work_revision) is int and work_revision == revision + 1,
          'positive launch witness context changed since original admission')


def run_case(binary, sdk, out):
    """Seed one genuine old queued intention, then continue it through public watch.

    Old/new owner processes are separate. The running provider is authenticated
    through its actual kernel peer/normal helper, not seed JSON or PID labels.
    Original failures remain and only owned actors may be cleaned up.
    """
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    _, owned = planning.flow(binary, out / 'planning', before_worker=True, return_fixture=True)
    root = owned.root
    nonce = fixture.digest(('restart' + str(root)).encode())
    fixture.publish(root / 'codex', worker.provider_program(nonce).encode())
    (root / 'codex').chmod(0o700)
    markers_before = completion.provider_files(root)
    seed = seed_queue(owned, sdk, out)
    check(completion.provider_files(root) == markers_before, 'seed unexpectedly invoked worker provider')
    original = owned.ok(['agent', 'show', seed['agent_id']])['agent']
    check(original['id'] == seed['agent_id'] and original['attempt_id'] == seed['attempt_id']
          and original['native_mission'] == seed['binding'] and original['status'] == 'queued',
          'seed output differs from actual queued Agent')
    worker.verify_current_root_binding(original['native_mission'], root)
    seed_status = public.status_native(owned, original['id'])
    check(seed_status['supervision'] is not None and seed_status['supervision']['state'] == 'stopped'
          and seed_status['supervision']['generation'] == seed['generation'],
          'old conductor lease is not observably stopped')
    fixture.publish(out / 'seed-native-status.json', seed_status)
    before = admissions(owned)
    check(read_adoptions(owned, original['id']) == [], 'restart seed already contains an adoption')
    costs_before = owned.ok(['costs', 'show', owned.work])
    turn_before = completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, original['id'])
    verify_fresh_execution_revision(before, original['id'], owned.ok(['work', 'show', owned.work])['work']['revision'])
    prefix = []
    fixture.publish(out / 'before.json', dict(admissions=before, costs=costs_before, turn=turn_before,
                    original_agent_id=original['id'], original_attempt_id=original['attempt_id'],
                    original_generation=seed['generation']))
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    process = connection = None
    watch_fd = supervisor_fd = provider_fd = None
    supervisor = provider = None
    channels = []
    primary = None
    errors = []
    supervisor_reaped = False
    result = None
    try:
        listener.bind(str(root / 'watch-provider.sock'))
        os.chmod(root / 'watch-provider.sock', 0o600)
        listener.listen(1)
        stdout, stderr = worker.spool(out / 'watch.stdout'), worker.spool(out / 'watch.stderr')
        channels.extend((stdout, stderr))
        argv = [str(binary), 'mission', 'watch', owned.work, '--root', str(root), '--json']
        fixture.publish(out / 'watch-command.json', dict(argv=argv, global_watchdog=None))
        process = subprocess.Popen(argv, env=owned.environment, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
        conductor = worker.observation(process.pid)
        watch_fd = worker.attach(conductor)
        with selectors.DefaultSelector() as selector:
            selector.register(listener, selectors.EVENT_READ, 'provider')
            selector.register(watch_fd, selectors.EVENT_READ, 'conductor')
            while connection is None:
                for key, _ in selector.select():
                    check(key.data == 'provider', 'restarted public watch exited before provider entry')
                    connection, _ = listener.accept()
        supervisor, supervisor_fd, provider, provider_fd, agent_id = public.attach_provider(connection, conductor, binary, root)
        check(agent_id == original['id'], 'watch created a second Agent instead of continuing queued intention')
        entry = worker.frame(connection)
        check(entry['nonce'] == nonce and entry['pid'] == provider['pid']
              and entry['ppid'] == supervisor['pid'], 'continued provider nonce/kernel peer mismatch')
        supervisor = worker.supervisor_session(supervisor, conductor, True)
        running = owned.ok(['agent', 'show', agent_id])['agent']
        check(running['supervisor_pid'] == supervisor['pid'] and running['child_pid'] == provider['pid']
              and running['native_execution']['phase'] == 'permission_recorded',
              'continued worker does not hold a real native grant')
        active = admissions(owned)
        costs_active = owned.ok(['costs', 'show', owned.work])
        turn_active = completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, agent_id)
        verify_same_admission(before, active, original, running, costs_before, costs_active, turn_before, turn_active)
        status = public.status_native(owned, agent_id)
        check(type(status['supervision']['generation']) is int
              and status['supervision']['generation'] > seed['generation'],
              'continued worker still belongs to old conductor generation')
        fixture.publish(out / 'active.json', dict(admissions=active, costs=costs_active, turn=turn_active,
                        native_status=status, same_agent_attempt_verified=True, no_second_admission_verified=True,
                        reservation_rows_verified=True, consumption_reset=False))
        signal.pidfd_send_signal(watch_fd, signal.SIGINT)
        check(process.wait() == 0, 'continued public watch failed graceful SIGINT shutdown')
        current = worker.observation(supervisor['pid'])
        check(current['stamp'] == supervisor['stamp'] and current['ppid'] == os.getpid(),
              'continued granted worker not independently adopted after watch exit')
        connection.sendall(b'Q')
        survived = worker.frame(connection)
        check(survived['nonce'] == nonce and survived['pid'] == provider['pid'],
              'continued provider did not execute after conductor reaped')
        check(admissions(owned) == before, 'watch shutdown changed original admission/reservation history')
        fixture.publish(root / 'watch-worker/release-worker.json', dict(nonce=nonce, release=True))
        select.select([supervisor_fd], [], [])
        waited, status_code = os.waitpid(supervisor['pid'], 0)
        supervisor_reaped = True
        check(waited == supervisor['pid'] and os.WIFEXITED(status_code) and os.WEXITSTATUS(status_code) == 0,
              'continued real supervisor did not finish/reap with exit0')
        final_agent = owned.ok(['agent', 'show', agent_id])['agent']
        check(final_agent['status'] == 'completed' and final_agent['attempt_id'] == original['attempt_id'],
              'continued worker did not complete same original attempt')
        task = next(task for task in owned.ok(['work', 'show', owned.work])['work']['tasks'] if task['id'] == 'task')
        check(task['status'] == 'blocked', 'continued provider completion accepted task')
        receipt = worker.retain_completion(owned, out,
            dict(agent=final_agent, task_accepted=task['status'] != 'blocked'), running, supervisor, provider)
        terminal = admissions(owned)
        verify_terminal_history(before, terminal, agent_id)
        final_costs = owned.ok(['costs', 'show', owned.work])
        check(final_costs['attempts_with_cost'] + final_costs['attempts_without_cost']
              == costs_before['attempts_with_cost'] + costs_before['attempts_without_cost'],
              'completion introduced or removed a reported worker attempt')
        check(final_costs['attempts_with_cost'] == costs_before['attempts_with_cost'] + 1
              and final_costs['attempts_without_cost'] == costs_before['attempts_without_cost'] - 1
              and final_costs['reported_usd'] == costs_before['reported_usd'] + 1,
              'completion did not record exactly one explicit inert provider cost')
        final_turn = completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, agent_id)
        check(final_turn['seq'] == turn_before['seq'] and final_turn['state'] == 'released',
              'continued completion replaced or did not release original FIFO turn')
        adoption_rows = read_adoptions(owned, agent_id)
        fixture.publish(out / 'adoption-rows.json', adoption_rows)
        events = verify_restart_events((out / 'watch.stdout').read_bytes(), owned.work, agent_id,
                                       seed['generation'], status['supervision']['generation'], adoption_rows, prefix)
        result = dict(status='PASS', root=str(root), work=owned.work, agent_id=agent_id,
            attempt_id=original['attempt_id'], original_generation=seed['generation'],
            new_generation=status['supervision']['generation'], same_agent_attempt=True,
            no_second_admission=True, reservation_rows_verified=True, consumption_reset=False,
            preserved_adoption_prefix=prefix, context_controls_not_mutated_before_grant=True,
            provider_entry_verified=True, independent_worker_completion=True, completion_receipt=receipt,
            terminal_admissions=terminal, final_costs=final_costs, final_turn=final_turn, events=events,
            paid_provider=False, installed=False, task_accepted=False, global_watchdog=None)
    except BaseException as error:
        primary = error
        worker.retain_failure(errors, lambda: fixture.publish(out / 'failure.json',
            dict(status='FAIL', cause=''.join(traceback.format_exception(error)), root=str(root))))
    finally:
        if primary is not None:
            if process is not None and process.poll() is None:
                try:
                    check(watch_fd is not None and worker.observation(process.pid)['stamp'] == conductor['stamp'],
                          'continued cleanup watch ownership unknown')
                    signal.pidfd_send_signal(watch_fd, signal.SIGTERM)
                    process.wait()
                except BaseException as error:
                    errors.append(error)
            try:
                release = root / 'watch-worker/release-worker.json'
                if not release.exists():
                    fixture.publish(release, dict(nonce=nonce, release=True))
                if supervisor is not None and not supervisor_reaped:
                    if not select.select([supervisor_fd], [], [], 0)[0]:
                        check(worker.observation(supervisor['pid'])['stamp'] == supervisor['stamp'],
                              'continued cleanup supervisor identity changed')
                        signal.pidfd_send_signal(supervisor_fd, signal.SIGTERM)
                    os.waitpid(supervisor['pid'], 0)
                    supervisor_reaped = True
            except BaseException as error:
                errors.append(error)
            if provider is not None and provider_fd is not None and supervisor_reaped:
                try:
                    public.reap_known_provider(provider, provider_fd, out)
                except BaseException as error:
                    errors.append(error)
            try:
                public.cleanup_owned_children(binary, root, out)
            except BaseException as error:
                errors.append(error)
        for resource in (connection, listener):
            if resource is not None:
                try:
                    resource.close()
                except BaseException as error:
                    errors.append(error)
        for stream in channels:
            for operation in (stream.flush, lambda stream=stream: os.fsync(stream.fileno()), stream.close):
                try:
                    operation()
                except BaseException as error:
                    errors.append(error)
        for descriptor in (watch_fd, supervisor_fd, provider_fd):
            if descriptor is not None:
                try:
                    os.close(descriptor)
                except BaseException as error:
                    errors.append(error)
    if primary is not None or errors:
        raise BaseExceptionGroup('continued public watch observation/cleanup', ([primary] if primary else []) + errors)
    fixture.publish(out / 'result.json', result)
    return result



def main():
    """Qualify matched normal/SDK profiles, execute one vertical W2 fixture and retain.

    Only root's final frozen recipe may exercise real product binaries. Source and
    all three binary hashes must remain unchanged; no install or provider account is used.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'sdk-binary', 'sdk-profile', 'refusal-sdk-binary', 'refusal-sdk-profile', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    binary, sdk, refusal_sdk, out = args.binary.resolve(), args.sdk_binary.resolve(), args.refusal_sdk_binary.resolve(), args.output.resolve()
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    normal, profile = json.loads(args.profile.read_bytes()), json.loads(args.sdk_profile.read_bytes())
    refusal_profile = json.loads(args.refusal_sdk_profile.read_bytes())
    check(normal['no_fixture_feature'] is True and normal['sha256'] == fixture.digest(binary.read_bytes()),
          'restart normal profile/hash mismatch')
    check(profile['example'] == 'mission_watch_restart_seed' and profile['engine_features_empty'] is True
          and profile['shared_workspace_features_match_normal'] is True and profile['test_profile'] is False
          and profile['normal_profile_sha256'] == normal['sha256']
          and profile['binary_sha256'] == fixture.digest(sdk.read_bytes()), 'restart SDK effective profile/hash mismatch')
    check(refusal_profile['example'] == 'mission_worker_adoption_refusals'
          and refusal_profile['engine_features_empty'] is True
          and refusal_profile['shared_workspace_features_match_normal'] is True and refusal_profile['test_profile'] is False
          and refusal_profile['normal_profile_sha256'] == normal['sha256']
          and refusal_profile['binary_sha256'] == fixture.digest(refusal_sdk.read_bytes()),
          'adoption refusal SDK effective profile/hash mismatch')
    fixture.publish(out / 'profiles.json', dict(normal=normal, sdk=profile, refusal_sdk=refusal_profile))
    fixture.publish(out / 'method.py', Path(__file__).read_bytes())
    sources = {str(Path(module.__file__).resolve()): fixture.digest(Path(module.__file__).read_bytes())
               for name, module in tuple(sys.modules.items()) if name.startswith('rust_') and getattr(module, '__file__', None)}
    sources[str(Path(__file__).resolve())] = fixture.digest(Path(__file__).read_bytes())
    fixture.publish(out / 'inputs.json', dict(sources=sources, binary_sha256=normal['sha256'], sdk_sha256=profile['binary_sha256'],
                    refusal_sdk_sha256=refusal_profile['binary_sha256']))
    api_result = run_api_case(binary, sdk, refusal_sdk, out / 'api-refusals')
    result = worker.subreaper_scope(lambda: run_case(binary, sdk, out / 'continued'),
        lambda error: fixture.publish(out / 'failure.json', dict(status='FAIL', cause=''.join(traceback.format_exception(error)))),
        parent_death.subreaper)
    check(all(fixture.digest(Path(path).read_bytes()) == sha for path, sha in sources.items()), 'restart harness sources changed')
    check(normal['sha256'] == fixture.digest(binary.read_bytes()) and profile['binary_sha256'] == fixture.digest(sdk.read_bytes())
          and refusal_profile['binary_sha256'] == fixture.digest(refusal_sdk.read_bytes()),
          'restart product or SDK binary changed')
    fixture.publish(out / 'result.json', dict(status='PASS', api_refusals=api_result, case=result, sources_stable=True,
                    paid_provider=False, installed=False, task_accepted=False, global_watchdog=None))
    print('PASS public mission watch continues same queued Agent attempt FIFO reservation without second admission', flush=True)
    print('REPLAY_EVIDENCE ' + str(out), flush=True)


if __name__ == '__main__':
    main()
