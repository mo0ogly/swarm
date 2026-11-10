#!/usr/bin/env python3
"""Observe W4 retained APIs and ordinary status on an owned fixture Root.

The supplied normal CLI and SDK example must have prequalified default matching
engine profiles; this driver neither builds nor qualifies a compiler graph. CLI
init/create/add/gate prepare the initial inactive data. A later public budget,
provider configuration and prepare/reconcile sequence records a cancellation
before execution. A second Work runs one local fake provider process and submits
its actual report; no provider account or private SQL write is used.
A real method3 gate controls altered-artifact staleness, output ENOSPC cause
preservation and byte-restoration recovery. /dev/full is a Linux transport
failure fixture, never an execution policy or watchdog. SDK
retained inputs and freshness coverage are separate sequential observations,
not one atomic whole-status snapshot. Every read/refusal is bracketed by the
complete typed SQLite logical hash and actual DB file hash. Marker/operator
attribution is fixture data, never authentication. Zero recorded calls is no
invoice or model-activity measurement. Historical mission status JSON is also
exercised for ordinary unconfigured Work and a manually cancelled Agent with
absent/partial report discovery, contained symlink compatibility and outside-Root
symlink refusal, then a completed/submitted local worker pending review without
gate. Selected cooldown records cover active/expired/cleared/corrupt and recovery.
Active Agent/planning branches and installation remain
unqualified. No global
watchdog, sleeps, retries or live Root is used.

Review axes: method3/check literals are explicit gate fixture inputs; ENOSPC
and /dev/full describe the Linux transport. Terminal budget values are explicit
fixture inputs and public persisted choices, with no operator limit added.
Changed invocation/observation helpers and gate/terminal/completed cases document their IO and
limits. Original SDK channels/exit, typed-safe Gate/IO labels and restoration
errors are retained; neither nonzero exit nor empty redirected stdout alone
proves refusal, recovery, no mutation or complete status.
"""
import argparse
import copy
import errno
from datetime import datetime
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import stat
import struct
import subprocess
import tempfile
import time
import traceback

import rust_agent_model_route as fixture

KIND = 'owned-inactive-status-fixture'
MARKER = '.swarm/verification-status-contract.json'
SOURCE = 'agent.lifecycle + agent.progress + provider.result + task.review-history + planning_calls + events'
NOTE = 'Les appels d’outils, les appels IA et les contrôles du moteur sont des mesures différentes. Une mesure absente n’est pas un zéro ; les relances internes du fournisseur ne sont pas toutes observables. Une réservation d’appel ne prouve pas son envoi au fournisseur.'


def require(condition, code):
    """Refuse with a static safe diagnostic, never interpolate private observations."""
    if not condition:
        raise RuntimeError(code)


def digest(raw):
    """Hash complete actual bytes; hashes never replace a missing observation."""
    return hashlib.sha256(raw).hexdigest()


def publish(path, value):
    """Publish fresh evidence atomically; only marker replacement is separately authorized."""
    require(not os.path.lexists(path), 'status evidence output already exists')
    fixture.publish(path, value)


def qualify(normal, sdk, binary, sdk_binary):
    """Consume actual native/example profile contracts, not requalify the build graph.

    Default-core identity/features and real binary bytes must match the retained
    qualifiers. Missing flags, false/boolean identities or wrong SDK targets fail.
    This is not independent compiler attestation or a production SDK designation.
    """
    require(type(normal) is dict and normal.get('status') == 'VERIFIED_DEFAULT_NATIVE_PROFILE'
            and normal.get('no_fixture_feature') is True and normal.get('all_core_features_empty') is True
            and normal.get('all_core_test_false') is True and normal.get('sha256') == digest(binary.read_bytes()),
            'normal default profile or binary mismatch')
    require(type(sdk) is dict and sdk.get('status') == 'VERIFIED_SDK_EXAMPLE_DEFAULT_CORE'
            and sdk.get('example') == 'mission_status_observation' and sdk.get('engine_features_empty') is True
            and sdk.get('shared_workspace_features_match_normal') is True and sdk.get('test_profile') is False
            and sdk.get('binary_sha256') == digest(sdk_binary.read_bytes())
            and sdk.get('normal_profile_sha256') == normal['sha256'],
            'status SDK default profile binary or matching normal core mismatch')


def expect_refusal(operation):
    """Require a real guard refusal; an unrelated exception is not accepted as that proof."""
    try:
        operation()
    except RuntimeError:
        return
    raise RuntimeError('invalid status guard control was accepted')


def profile_guards(normal, sdk, binary, sdk_binary):
    """Valid prequalified inputs precede altered-profile refusals and recovery."""
    qualify(normal, sdk, binary, sdk_binary)
    for field, value in (('engine_features_empty', False), ('test_profile', True),
                         ('normal_profile_sha256', '0' * 64), ('binary_sha256', None),
                         ('example', 'different_example')):
        bad = copy.deepcopy(sdk); bad[field] = value
        expect_refusal(lambda bad=bad: qualify(normal, bad, binary, sdk_binary))
    bad = copy.deepcopy(normal); bad['all_core_features_empty'] = False
    expect_refusal(lambda: qualify(bad, sdk, binary, sdk_binary))
    qualify(normal, sdk, binary, sdk_binary)
    return 6


def identifier(value):
    """Quote only SQLite-owned schema names; request values use SQL parameters."""
    return '"' + value.replace('"', '""') + '"'


def typed_cell(kind, value):
    """Keep all SQLite storage classes, raw text/BLOB octets and exact real bits."""
    if kind == b'null':
        require(value is None, 'SQLite null storage class mismatch')
        return ['null']
    if kind == b'integer':
        require(type(value) is int, 'SQLite integer storage class mismatch')
        return ['integer', value]
    if kind == b'real':
        require(type(value) is float, 'SQLite real storage class mismatch')
        return ['real', struct.pack('>d', value).hex()]
    require(kind in (b'text', b'blob') and type(value) is bytes, 'SQLite octet storage class mismatch')
    return [kind.decode('ascii'), value.hex()]


def encode_rows(rows):
    """Hash a stable multiset of complete typed rows without exporting private values."""
    encoded = [json.dumps(row, ensure_ascii=True, separators=(',', ':')).encode() for row in rows]
    encoded.sort()
    return digest(b'\n'.join(encoded))


def physical(database):
    """Require the owned regular single-link DB; record actual inode and file bytes."""
    info = database.lstat()
    require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and info.st_nlink == 1
            and stat.S_IMODE(info.st_mode) == 0o600, 'status fixture DB ownership type or mode invalid')
    return dict(device=info.st_dev, inode=info.st_ino, size=info.st_size,
                sha256=digest(database.read_bytes()))


def snapshot(root, work, operator):
    """Read every actual schema/table/cell type in one readonly SQLite transaction.

    Include sqlite internal tables, complete schemas and schema rootpages; no
    table/pulse/journal is masked. Raw text is hashed as octets, REAL by IEEE bits.
    Queries, rollback and close failures remain grouped. DB bytes must also be
    unchanged while observing, so a concurrent writer cannot become a false PASS.
    """
    database_path = root / '.swarm/state.db'
    before_file = physical(database_path)
    database = sqlite3.connect(database_path.as_uri() + '?mode=ro', uri=True, timeout=0)
    database.text_factory = bytes
    errors = []; result = None; begun = False
    try:
        database.execute('BEGIN'); begun = True
        schema = database.execute('SELECT type,name,tbl_name,rootpage,sql FROM sqlite_schema ORDER BY type,name').fetchall()
        schema_typed = [[typed_cell(b'null' if value is None else b'integer' if type(value) is int else b'text', value)
                         for value in row] for row in schema]
        tables = {}
        for kind, raw_name, _, _, _ in schema:
            if kind != b'table':
                continue
            name = raw_name.decode('utf8')
            columns = database.execute('PRAGMA table_xinfo(' + identifier(name) + ')').fetchall()
            names = [column[1].decode('utf8') for column in columns]
            require(bool(names), 'status SQLite table columns missing')
            projection = ','.join('typeof(' + identifier(name) + '),' + identifier(name) for name in names)
            rows = database.execute('SELECT ' + projection + ' FROM ' + identifier(name)).fetchall()
            cells = [[typed_cell(row[index], row[index + 1]) for index in range(0, len(row), 2)] for row in rows]
            column_typed = [[typed_cell(b'null' if value is None else b'integer' if type(value) is int else b'text', value)
                             for value in column] for column in columns]
            tables[name] = dict(columns=names, column_types_sha256=encode_rows(column_typed),
                                rows=len(rows), typed_rows_sha256=encode_rows(cells))
        record = database.execute('SELECT revision,body FROM works WHERE id=?', [work]).fetchall()
        require(len(record) == 1 and type(record[0][0]) is int, 'status fixture actual Work missing')
        body = json.loads(record[0][1]); tasks = body['tasks']
        require(tasks is None or type(tasks) is list, 'status stored Work task representation invalid')
        facts = dict(revision=record[0][0], task_count=0 if tasks is None else len(tasks),
                     launch_profile_present=body.get('launch_profile') is not None,
                     task_ids=[] if tasks is None else [task['id'] for task in tasks], task_gate_ids={})
        # This owned fixture records an explicitly nonempty gate name. The pure
        # producer trims that name into gate_id; no scope_id fallback is needed
        # or guessed here. Absent gate keeps the ten-field presentation schema.
        for task in [] if tasks is None else tasks:
            gate = task.get('gate')
            if gate is not None:
                require(type(gate) is dict and type(gate.get('name')) is str
                        and bool(gate['name']) and gate['name'] == gate['name'].strip(),
                        'status fixture gate must have its actual explicit nonempty name')
                facts['task_gate_ids'][task['id']] = gate['name']
        for field, table, predicate, values in (
            ('agent_count', 'agents', 'work_id=?', [work]),
            ('event_count', 'events', 'work_id=?', [work]),
            ('planning_call_count', 'planning_calls', "work_id=? AND state!='released'", [work]),
            ('exchange_count', 'agent_exchanges', 'work_id=?', [work])):
            facts[field] = database.execute('SELECT count(*) FROM ' + identifier(table) + ' WHERE ' + predicate, values).fetchone()[0]
        facts['visit_present'] = bool(database.execute('SELECT 1 FROM session_visits WHERE work_id=? AND operator=?', [work, operator]).fetchall())
        logical = dict(schema_sha256=encode_rows(schema_typed), tables=tables)
        result = dict(logical_sha256=digest(json.dumps(logical, sort_keys=True, separators=(',', ':')).encode()),
                      logical=logical, facts=facts)
    except BaseException as error:
        errors.append(error)
    if begun:
        try: database.rollback()
        except BaseException as error: errors.append(error)
    try: database.close()
    except BaseException as error: errors.append(error)
    try:
        after_file = physical(database_path)
        require(after_file == before_file, 'status snapshot DB changed during readonly observation')
        if result is not None: result['database'] = after_file
    except BaseException as error:
        errors.append(error)
    if errors: raise BaseExceptionGroup('status readonly snapshot query rollback close or file observation', errors)
    return result


class Driver:
    """Own one fixture and exact invocation evidence; only the explicit local fake runs."""
    def __init__(self, binary, sdk, out, root, operator, environment):
        """Freeze absolute coordinates before changing the child's working directory.

        Relative evidence/input paths are anchored to the caller cwd, without
        opening files or resolving symlinks. This is no canonical Root/identity
        check or authority; the actual product path still owes those guards.
        Optional SDK stays absent for CLI-only tests. No process is started here.
        """
        self.binary = Path(binary).absolute()
        self.sdk = None if sdk is None else Path(sdk).absolute()
        self.out, self.root = Path(out).absolute(), Path(root).absolute()
        self.operator, self.environment = operator, environment
        self.work = None; self.number = 0; self.reads = []

    def invoke(self, label, argv, output_full=False):
        """Run/reap one supplied executable with retained channels and no timeout.

        Environment is forwarded without being dumped. Original operation and
        stream flush/fsync/close failures stay joined; no retry or empty success.
        output_full redirects the actual child stdout to Linux /dev/full. Its
        retained stdout file is empty by construction; command metadata records
        the transport, while stderr and exit retain the actual failure.
        """
        self.number += 1; prefix = self.out / (f'{self.number:03}-' + label)
        publish(Path(str(prefix) + '.command.json'), dict(argv=list(map(str, argv)), global_watchdog=None,
                environment_values_retained=False, stdout_transport='/dev/full' if output_full else 'retained-file',
                started_monotonic_ns=time.monotonic_ns()))
        streams = []; errors = []; result = None; sink = None
        try:
            for suffix in ('.stdout', '.stderr'):
                stream = Path(str(prefix) + suffix).open('xb', buffering=0); streams.append(stream)
            if output_full:
                require(os.uname().sysname == 'Linux' and stat.S_ISCHR(os.stat('/dev/full').st_mode),
                        'status ENOSPC fixture requires Linux full character device')
                sink = open('/dev/full', 'wb', buffering=0)
            result = subprocess.run(list(map(str, argv)), cwd=self.root, env=self.environment,
                                    stdin=subprocess.DEVNULL, stdout=sink if output_full else streams[0], stderr=streams[1])
        except BaseException as error:
            errors.append(error)
        if sink is not None:
            try: sink.close()
            except BaseException as error: errors.append(error)
        for stream in streams:
            for operation in (stream.flush, lambda stream=stream: os.fsync(stream.fileno()), stream.close):
                try: operation()
                except BaseException as error: errors.append(error)
        if result is not None:
            try: publish(Path(str(prefix) + '.exit.json'), dict(exit=result.returncode, ended_monotonic_ns=time.monotonic_ns()))
            except BaseException as error: errors.append(error)
        if errors: raise BaseExceptionGroup('status invocation and evidence cleanup', errors)
        return result.returncode, Path(str(prefix) + '.stdout').read_bytes(), Path(str(prefix) + '.stderr').read_bytes()

    def cli(self, label, command, value=None):
        """Invoke the actual public CLI, with a fresh retained JSON request if supplied."""
        argv = [self.binary, *command, '--root', self.root, '--json']
        if value is not None:
            path = self.out / (label + '.input.json'); publish(path, value); argv.extend(['--input', path])
        return self.invoke(label, argv)

    def read(self, label, argv, output_full=False):
        """Compare complete typed DB and file observations around one actual read/refusal."""
        before = snapshot(self.root, self.work, self.operator)
        publish(self.out / (label + '.before.json'), before)
        errors = []; result = None
        try: result = self.invoke(label, argv, output_full=output_full)
        except BaseException as error: errors.append(error)
        try:
            after = snapshot(self.root, self.work, self.operator)
            publish(self.out / (label + '.after.json'), after)
            require(after == before, 'status read or refusal mutated retained database')
        except BaseException as error:
            errors.append(error)
        if errors: raise BaseExceptionGroup('status read invocation and no-mutation evidence', errors)
        self.reads.append(dict(label=label, logical_sha256=before['logical_sha256'], database_sha256=before['database']['sha256']))
        return result, before['facts']

    def sdk_read(self, label, work=None, output_full=False):
        """Read the public engine API through the owned marker-gated SDK, not CLI status."""
        return self.read(label, [self.sdk, self.root, self.work if work is None else work, self.operator], output_full=output_full)


def observation(code, stdout, stderr, work, facts, stale=False):
    """Require the SDK's exact partial observations and real zero-recorded-call spending.

    Result keys include gate_id exactly when the real task has a named gate; its
    value must match the persisted observation, never a synthetic presentation ID.
    Empty retained call/row/attempt arrays establish no recorded spending calls in
    this fixture, not a paid invoice, freshness, model activity or complete status.
    """
    require(code == 0 and stderr == b'', 'status SDK observation failed')
    value = json.loads(stdout)
    fields = {'schema_version','work','revision','task_count','agent_count','event_count','planning_call_count',
              'exchange_count','visit_present','freshness_task_count','spending','task_results','task_result_causes','complete_mission_status','observation_is_permission'}
    require(type(value) is dict and set(value) == fields and type(value['schema_version']) is int
            and value['schema_version'] == 1 and value['work'] == work
            and value['complete_mission_status'] is False and value['observation_is_permission'] is False,
            'status SDK partial identity or schema differs')
    for field in ('revision','task_count','agent_count','event_count','planning_call_count','exchange_count'):
        require(type(value[field]) is int and value[field] == facts[field], 'status SDK count differs from actual retained data')
    require(type(value['freshness_task_count']) is int and value['freshness_task_count'] == facts['task_count'],
            'status freshness task coverage differs from actual Work tasks')
    require(facts['launch_profile_present'] is False, 'status fixture unexpectedly acquired a launch profile')
    require(type(value['visit_present']) is bool and value['visit_present'] == facts['visit_present'], 'status SDK visit differs')
    require(all(value[field] == 0 for field in ('agent_count','planning_call_count','exchange_count'))
            and value['visit_present'] is False, 'status inactive fixture contains execution history')
    results = value['task_results']
    require(type(results) is list and len(results) == len(facts['task_ids'])
            and type(value['task_result_causes']) is list, 'status actual task results or semantic causes differ')
    if stale:
        causes = value['task_result_causes']
        require(bool(causes) and all(type(branch) is list and bool(branch) for branch in causes),
                'status stale observation lost original typed Gate branches')
        labels = []
        for branch in causes:
            for node in branch:
                require(type(node) is dict and set(node) == {'labels','io_errno','sqlite_code'}
                        and type(node['labels']) is list and all(type(label) is str for label in node['labels'])
                        and all(node[field] is None or type(node[field]) is int for field in ('io_errno','sqlite_code')),
                        'status semantic cause safe projection schema differs')
                labels.extend(node['labels'])
        require('gate-evidence-invalid' in labels, 'status stale observation lost original safe Gate label')
    else:
        require(value['task_result_causes'] == [], 'status valid observation invented semantic causes')
    result_fields = {'process_label','report_label','validation_label','state','label','process_state',
                     'report_state','validation_state','reason','next_step'}
    for result, task in zip(results, facts['task_ids']):
        gate_id = facts['task_gate_ids'].get(task)
        actual_fields = result_fields | ({'gate_id'} if gate_id is not None else set())
        require(type(result) is list and len(result) == 2 and result[0] == task
                and type(result[1]) is dict and set(result[1]) == actual_fields,
                'status task result identity or exact schema differs')
        presentation = result[1]
        if gate_id is not None:
            require(presentation['gate_id'] == gate_id, 'status task result gate identity differs from actual saved gate')
        require(all(type(item) is str and item for item in presentation.values())
                and presentation['state'] == presentation['process_state'] == 'not_started'
                and presentation['validation_state'] == ('failed_or_stale' if stale else 'not_started')
                and presentation['report_state'] == 'absent'
                and presentation['process_label'] == 'Pas encore commencé'
                and presentation['validation_label'] == ('Contrôles en échec ou preuves périmées' if stale else 'Pas encore commencé')
                and presentation['report_label'] == 'Aucun rapport soumis',
                'status task result invented execution, report or validation')
    spending = value['spending']
    require(type(spending) is dict and set(spending) == {'calls','observed_at','measurement_source','rows',
                'recorded_control_executions','worker_retries','note','attempts'}, 'status spending exact schema differs')
    require(all(type(spending[field]) is list and spending[field] == [] for field in ('calls','rows','attempts'))
            and all(type(spending[field]) is int and spending[field] == 0 for field in ('recorded_control_executions','worker_retries'))
            and spending['measurement_source'] == SOURCE and spending['note'] == NOTE,
            'status empty retained spending was replaced or overclaimed')
    require(type(spending['observed_at']) is str and bool(spending['observed_at']), 'status observation timestamp absent')
    require(datetime.fromisoformat(spending['observed_at'].replace('Z','+00:00')).tzinfo is not None,
            'status observation timestamp lacks timezone')
    return value


def observation_guards(code, stdout, stderr, work, facts):
    """Use the real valid observation before forged counters/arrays/authority and recovery."""
    value = observation(code, stdout, stderr, work, facts)
    for field, bad in (('task_count', True), ('event_count', facts['event_count'] + 1), ('complete_mission_status', True), ('freshness_task_count', True)):
        changed = copy.deepcopy(value); changed[field] = bad
        expect_refusal(lambda changed=changed: observation(0, json.dumps(changed).encode(), b'', work, facts))
    changed = copy.deepcopy(value); changed['spending']['calls'] = None
    expect_refusal(lambda: observation(0, json.dumps(changed).encode(), b'', work, facts))
    for field in ('task_results','task_result_causes'):
        changed = copy.deepcopy(value); changed[field] = None
        expect_refusal(lambda changed=changed: observation(0, json.dumps(changed).encode(), b'', work, facts))
    observation(code, stdout, stderr, work, facts)
    return 7


def refusal(result, expected, private):
    """Require nonempty safe typed refusal diagnostics and absolutely empty SDK output."""
    code, stdout, stderr = result
    require(code != 0 and stdout == b'' and stderr.startswith(b'Error: StatusObservationFailure')
            and ('"' + expected + '"').encode() in stderr,
            'status SDK refusal missing expected typed cause or leaked successful output')
    require(all(item not in stdout and item not in stderr for item in private), 'status refusal exposed private fixture input')


def historical(result, total, terminal=None, report=None, completed=None):
    """Assert the historical envelope on ordinary Work, optionally after cancellation.

    No native/schema wrapper is accepted. Storage and version observations must
    be present; unknown binary identity must remain unknown. These checks do not
    A supplied actual cancelled Agent must retain its original identity and
    unknown usage/cost independently of its released reservation. An optional
    report is discovered fixture text, never submitted or accepted evidence.
    A completed fixture instead retains its actual submitted report and declared
    local-program usage/cost, requiring review with missing gate, never acceptance.
    These checks do not qualify active/planning paths or a Go reference binary.
    """
    code, stdout, stderr = result
    require(code == 0 and stderr == b'', 'historical public status failed')
    value = json.loads(stdout)
    fields = {'changes_since_visit','spending','guidance','runtime','evidence_stage','organization',
              'authorized','enabled','paused','summary','next','validated','review','running',
              'active_agents','uncertain_agents','total','supervision','understanding','coordination','tasks'}
    require(type(value) is dict and set(value) == fields, 'historical status keys or native wrapper mismatch')
    require(value['total'] == total and len(value['tasks']) == total
            and all(value[key] == 0 for key in ('validated','running','active_agents','uncertain_agents'))
            and value['review'] == (0 if completed is None else 1),
            'historical status actual task counts mismatch')
    require(value['authorized'] is False and value['enabled'] is False
            and value['organization']['ready'] is False, 'ordinary status invented launch authority')
    if terminal is None and completed is None:
        require(value['spending']['calls'] in (None, []) and value['spending']['attempts'] in (None, []),
                'ordinary status invented recorded calls')
    elif terminal is not None:
        calls = value['spending']['calls']; attempts = value['spending']['attempts']
        require(type(calls) is list and len(calls) == 1 and type(attempts) is list and len(attempts) == 1,
                'terminal status lost actual historical attempt')
        call, attempt = calls[0], attempts[0]
        require(call['id'] == 'agent/' + terminal['id'] and call['attempt'] == terminal['attempt_id']
                and call['state'] == 'interrupted' and call['usage'] is None
                and attempt['agent'] == terminal['id'] and attempt['attempt'] == terminal['attempt_id']
                and attempt['process_state'] == 'interrupted' and attempt['usage_missing'] is True
                and attempt['measurement'] == 'unknown' and attempt['measured'] is False
                and attempt['recorded_calls'] == 1 and attempt['task_acceptance_recorded'] is False
                and attempt['cost']['attempts_without_cost'] == 1 and attempt['cost']['attempts_with_cost'] == 0,
                'terminal status invented measured usage cost acceptance or producer identity')
        task = value['tasks'][0]; presentation = task['result']
        require(task['id'] == terminal['task_id'] and task['state'] == 'intervention'
                and task['action'] == 'inspect' and task['label'] == 'Examiner avant reprise'
                and task['attempt']['agent_id'] == terminal['id']
                and task['attempt']['attempt_id'] == terminal['attempt_id']
                and presentation['state'] == 'stopped_early' and presentation['process_state'] == 'interrupted'
                and presentation['attempt_id'] == terminal['attempt_id']
                and presentation['validation_state'] == 'not_validated'
                and presentation['report_state'] == ('absent' if report is None else 'partial_possible')
                and presentation.get('report_id') == report,
                'terminal status lost stopped process original attempt or actual report')
    else:
        calls = value['spending']['calls']; attempts = value['spending']['attempts']
        require(type(calls) is list and len(calls) == 1 and type(attempts) is list and len(attempts) == 1,
                'completed status lost actual producer history')
        call, attempt = calls[0], attempts[0]; task = value['tasks'][0]; presentation = task['result']
        require(call['id'] == 'agent/' + completed['id'] and call['attempt'] == completed['attempt_id']
                and call['state'] == 'completed' and call['usage'] == completed['usage']
                and attempt['agent'] == completed['id'] and attempt['attempt'] == completed['attempt_id']
                and attempt['process_state'] == 'completed' and attempt['usage_missing'] is False
                and attempt['recorded_calls'] == 1 and attempt['task_acceptance_recorded'] is False
                and attempt['cost']['reported_usd'] == 25 and attempt['cost']['attempts_with_cost'] == 1
                and attempt['cost']['attempts_without_cost'] == 0,
                'completed status lost actual local-program declaration or invented acceptance')
        require(task['id'] == completed['task_id'] and task['state'] == 'review' and task['action'] == 'report'
                and task['label'] == 'Examiner le résultat' and task['attempt']['agent_id'] == completed['id']
                and task['attempt']['attempt_id'] == completed['attempt_id']
                and presentation['state'] == 'result_to_review' and presentation['process_state'] == 'completed'
                and presentation['attempt_id'] == completed['attempt_id'] and presentation['report_state'] == 'submitted'
                and presentation['report_id'] == 'handoff.md' and presentation['validation_state'] == 'missing_gate',
                'submitted completed report did not remain pending review without gate')
    runtime = value['runtime']
    require(type(runtime) is dict and len(runtime['volumes']) == 2
            and all(type(volume['state']) is str for volume in runtime['volumes'])
            and type(runtime['version']) is dict and type(runtime['version_diagnostic']) is dict,
            'historical status runtime or version producers missing')
    require(value['changes_since_visit']['first_visit'] is True,
            'public read unexpectedly marked an operator visit')
    return value


def public_storage_policy_case(driver, argv):
    """Save/reopen real operator storage thresholds through CLI and observe use.

    The deliberately large fixture threshold forces a blocked runtime without
    filling a disk or starting an Agent. Preview must preserve saved settings;
    restored original values must recover. This is not browser/Admin acceptance.
    """
    code, raw, _ = driver.cli('storage-policy-show',['engine-settings','show'])
    require(code == 0, 'storage policy public show failed'); original = json.loads(raw)
    change = dict(schema_version=original['schema_version'],event_id='public-storage-block',expected_revision=original['revision'],
                  reason='Owned storage threshold consumer fixture',values=copy.deepcopy(original['values']))
    change['values'].update(runtime_storage_stop_bytes='9223372036854775807',
                            runtime_storage_warning_bytes='9223372036854775807',runtime_storage_stop_inodes='0')
    code, raw, _ = driver.cli('storage-policy-preview',['engine-settings','preview'],change)
    require(code == 0, 'storage policy public preview failed'); change['preview_token'] = json.loads(raw)['preview_token']
    code, raw, _ = driver.cli('storage-policy-preview-reopen',['engine-settings','show'])
    require(code == 0 and json.loads(raw) == original, 'storage policy preview mutated persisted state')
    code, raw, _ = driver.cli('storage-policy-apply',['engine-settings','apply'],change)
    require(code == 0, 'storage policy public apply failed')
    code, raw, _ = driver.cli('storage-policy-reopen',['engine-settings','show'])
    require(code == 0, 'storage policy reopen failed'); saved = json.loads(raw)
    require(saved['values'] == change['values'] and saved['revision'] > original['revision'],
            'storage policy persisted values differ')
    result, _ = driver.read('public-storage-blocked',argv); blocked = historical(result,1)
    require(blocked['runtime']['state'] == 'blocked' and blocked['runtime']['launch_allowed'] is False,
            'actual status consumer ignored saved storage thresholds')
    restore = dict(schema_version=saved['schema_version'],event_id='public-storage-restore',expected_revision=saved['revision'],
                   reason='Restore original owned storage fixture values',values=original['values'])
    code, raw, _ = driver.cli('storage-policy-restore-preview',['engine-settings','preview'],restore)
    require(code == 0, 'storage policy restore preview failed'); restore['preview_token']=json.loads(raw)['preview_token']
    code, _, _ = driver.cli('storage-policy-restore',['engine-settings','apply'],restore)
    require(code == 0, 'storage policy restore apply failed')
    result, _ = driver.read('public-storage-recovered',argv); recovered=historical(result,1)
    require(recovered['runtime']['state'] in ('ready','warning') and recovered['runtime']['launch_allowed'] is True,
            'actual storage threshold consumer did not recover')
    print('PASS persisted public storage thresholds preview reopen block and recovery',flush=True)


def marker_case(driver, marker, patch, label, work, expected, private):
    """Apply only an owned marker fault; always restore it before the next API witness."""
    changed = dict(marker, **patch); path = driver.root / MARKER
    require(json.loads(path.read_bytes()) == marker, 'status marker fault lacks valid original')
    errors = []
    try:
        fixture.publish(path, changed)
        result, _ = driver.sdk_read(label, work)
        refusal(result, expected, private)
    except BaseException as error:
        errors.append(error)
    finally:
        try:
            fixture.publish(path, marker)
            require(json.loads(path.read_bytes()) == marker, 'status marker original not restored')
        except BaseException as error:
            errors.append(error)
    if errors: raise BaseExceptionGroup('status marker refusal and restoration', errors)
    result, facts = driver.sdk_read(label + '-recovery')
    observation(*result, driver.work, facts)


def stale_gate_output_case(driver, task_work, private):
    """Record a real public gate, retain staleness through success/output failure.

    Required check IDs/phases come from the actual public task.add response;
    method3 fixture values match root_store/material_path_tests, not a new policy.
    Every SDK call is DB/file-hash bracketed. Artifact restoration runs even when
    output loses a cause; recovery is exercised before grouped errors are raised.
    Only this owned inactive fixture artifact is changed, never SQLite state.
    """
    tasks = task_work['tasks']
    require(type(tasks) is list and len(tasks) == 1 and tasks[0]['id'] == 'owned-task',
            'status gate fixture actual task absent')
    task = tasks[0]; plan_checks = task['plan_checks']
    require(type(plan_checks) is dict and bool(plan_checks)
            and all(type(key) is str and key and type(phase) is str and phase for key, phase in plan_checks.items()),
            'status gate fixture lacks actual required check IDs and phases')
    proof = b'PRIVATE-OWNED-STATUS-GATE-PROOF'
    altered = b'PRIVATE-OWNED-STATUS-ALTERED-PROOF'
    path = driver.root/'proof.txt'
    require(not os.path.lexists(path), 'status gate artifact must be fresh')
    publish(path, proof)
    checks = [dict(id=key, gate=phase, domain='quality', mandatory=True, severity='minor',
                   penalty=1, max_penalty=100) for key, phase in plan_checks.items()]
    document = dict(method_version='3', scope_id=task['id'], artifacts={'proof.txt': digest(proof)},
                    domains={'quality': 1}, checks=checks,
                    results=[dict(id=check['id'], status='PASS', count=0, evidence=['proof.txt']) for check in checks])
    code, raw, _ = driver.cli('gate-record',['gate',driver.work],dict(schema_version=1,event_id='status-gate',
        expected_revision=task_work['revision'],task_id=task['id'],phase='delivery',name='Owned result proof',document=document))
    require(code == 0, 'status public method3 gate control failed')
    saved = json.loads(raw)['work']
    require(saved['revision'] == task_work['revision'] + 1 and saved['tasks'][0]['gate'] is not None
            and saved['tasks'][0]['gate']['name'] == 'Owned result proof',
            'status real gate or revision not recorded')
    result, facts = driver.sdk_read('gate-current-control')
    control = observation(*result, driver.work, facts)
    publish(driver.out/'gate-current-observation.json', control)
    errors = []; stale_value = None; output_refusal = False
    try:
        fixture.publish(path, altered)
        result, facts = driver.sdk_read('gate-stale-observation')
        stale_value = observation(*result, driver.work, facts, stale=True)
        require(all(item not in result[1] and item not in result[2] for item in [*private, proof, altered]),
                'status stale observation exposed private fixture content')
        publish(driver.out/'gate-stale-observation.json', stale_value)
        print('PASS status actual stale gate observation retains failed_or_stale and safe Gate cause', flush=True)
        result, _ = driver.sdk_read('gate-stale-output-full', output_full=True)
        refusal(result, 'gate-evidence-invalid', [*private, proof, altered, str(driver.root).encode(),
                                                driver.work.encode(), driver.operator.encode(), task['id'].encode()])
        code, stdout, stderr = result
        require(code != 0 and stdout == b'' and b'"runtime-output"' in stderr
                and ('"os-errno:' + str(errno.ENOSPC) + '"').encode() in stderr
                and ('"io_errno": Number(' + str(errno.ENOSPC) + ')').encode() in stderr,
                'status output failure lost actual typed IO ENOSPC cause')
        output_refusal = True
        print('PASS status stale gate output ENOSPC retains original Gate and IO causes without private data', flush=True)
    except BaseException as error:
        errors.append(error)
    finally:
        try:
            fixture.publish(path, proof)
            require(path.read_bytes() == proof, 'status gate original artifact not restored')
        except BaseException as error:
            errors.append(error)
    try:
        result, facts = driver.sdk_read('gate-restored-recovery')
        recovered = observation(*result, driver.work, facts)
        # Spending time is an actual sequential observation; only that timestamp
        # is excluded from exact recovery comparison, with no broader masking.
        expected = copy.deepcopy(control); actual = copy.deepcopy(recovered)
        del expected['spending']['observed_at']; del actual['spending']['observed_at']
        require(actual == expected, 'status gate byte restoration did not recover original observation')
        publish(driver.out/'gate-recovered-observation.json', recovered)
        print('PASS status original gate bytes restore positive observation without DB mutation', flush=True)
    except BaseException as error:
        errors.append(error)
    if errors: raise BaseExceptionGroup('status stale gate output cause preservation and restoration recovery', errors)
    return dict(stale_gate_observed=stale_value is not None, original_gate_cause_preserved=True,
                output_enospc_refused=output_refusal, output_gate_and_io_causes_preserved=True,
                gate_artifact_restored=True, gate_observation_recovered=True)


def terminal_case(driver, argv):
    """Prepare/cancel publicly, then read original terminal identity and report.

    Budget inputs are explicit owned fixture choices, not product defaults.
    providers config init writes an inert /bin/cat configuration; no agent run
    or provider subprocess is invoked. Budget apply returns Work directly.
    Report bytes are owned fixture material. A selected symlink to a regular
    contained target must be accepted, matching Go safeReport/artifact access.
    Only an outside-Root target must refuse; symlinks themselves are not unsafe.
    Original bytes are restored even on failure before a recovery observation.
    All status observations include complete DB logical/file hash brackets.
    """
    code, raw, _ = driver.cli('terminal-work-show', ['work', 'show', driver.work])
    require(code == 0, 'terminal public Work read failed'); work = json.loads(raw)['work']
    code, raw, _ = driver.cli('terminal-budget', ['budget', 'apply', driver.work],
        dict(schema_version=1, event_id='status-terminal-budget', expected_revision=work['revision'],
             budget=dict(limit_usd=10, reserve_per_launch_usd=1,
                         estimate_source='Owned inert cancellation fixture', reference_date='2026-10-23')))
    require(code == 0, 'terminal public budget failed'); work = json.loads(raw)
    require(work['id'] == driver.work and type(work['revision']) is int and type(work['tasks']) is list,
            'terminal budget response is not actual direct Work')
    code, raw, _ = driver.cli('terminal-provider', ['providers', 'config', 'init'],
        dict(schema_version=1, providers={'fixture': dict(command='/bin/cat', args=[])}))
    require(code == 0 and json.loads(raw)['provider_execution_started'] is False,
            'terminal provider configuration did not remain inert')
    code, raw, _ = driver.cli('terminal-prepare', ['agent', 'prepare', driver.work],
        dict(schema_version=1, event_id='status-terminal-session', expected_revision=work['revision'],
             task_id='owned-task', provider='fixture', capture_output=True))
    require(code == 0, 'terminal public prepare failed'); prepared = json.loads(raw)['agent']
    require(prepared['status'] == 'queued', 'terminal control was not queued')
    code, raw, _ = driver.cli('terminal-reconcile', ['agent', 'reconcile', prepared['id']])
    require(code == 0, 'terminal public cancellation reconciliation failed'); reconciled = json.loads(raw)
    agent = reconciled['agent']
    require(reconciled['cancelled_before_start'] is True and reconciled['accepted_by_cancellation'] is False
            and agent['status'] == 'interrupted' and agent['id'] == prepared['id']
            and agent['attempt_id'] == prepared['attempt_id'], 'terminal original cancellation receipt differs')
    code, raw, _ = driver.cli('terminal-work-reopen', ['work', 'show', driver.work])
    require(code == 0, 'terminal public Work reopen failed'); work = json.loads(raw)['work']
    require(work['tasks'][0]['status'] == 'blocked' and work['tasks'][0]['attempts'][-1]['status'] == 'interrupted',
            'terminal cancellation did not persist blocked original attempt')
    database = sqlite3.connect((driver.root/'.swarm/state.db').as_uri() + '?mode=ro', uri=True, timeout=0)
    errors = []; reservation = None
    try:
        reservation = database.execute('SELECT state,amount FROM reservations WHERE agent_id=?', [agent['id']]).fetchall()
    except BaseException as error:
        errors.append(error)
    try: database.close()
    except BaseException as error: errors.append(error)
    if errors: raise BaseExceptionGroup('terminal readonly reservation query and close', errors)
    require(reservation == [('released', 1.0)], 'terminal reservation release or original amount differs')
    publish(driver.out/'terminal-reservation.json', dict(agent=agent['id'], state=reservation[0][0],
                                                       original_reserved_usd=reservation[0][1]))
    result, _ = driver.read('public-terminal-absent-report', argv); historical(result, 1, agent)
    reports = driver.root/'docs'; reports.mkdir(mode=0o700)
    report = reports/'owned-task.md'; original = b'Owned cancelled attempt report; incomplete and unsubmitted.\n'
    publish(report, original)
    result, _ = driver.read('public-terminal-partial-report', argv); historical(result, 1, agent, 'docs/owned-task.md')
    contained = driver.root/'contained-report.txt'
    publish(contained, b'Owned contained regular report target.\n')
    outside = Path(tempfile.mkdtemp(prefix='rust-status-report-outside-', dir=driver.root.parent))
    publish(driver.out/'terminal-outside-root.json', dict(root=str(driver.root), outside_fixture=str(outside),
                                                        reason='Owned outside-Root report containment fault'))
    outside_report = outside/'report.txt'; publish(outside_report, b'Owned outside-Root regular report target.\n')
    errors = []
    try:
        report.unlink(); report.symlink_to(contained)
        result, _ = driver.read('public-terminal-report-contained-symlink', argv)
        historical(result, 1, agent, 'docs/owned-task.md')
        report.unlink(); report.symlink_to(outside_report)
        result, _ = driver.read('public-terminal-report-outside-symlink', argv)
        require(result[0] != 0 and result[1] == b'', 'terminal outside-Root report became successful output')
        failure = json.loads(result[2])['failure']
        require('gate-evidence-invalid' in failure['causes'],
                'terminal outside-Root report lost public Gate contract refusal')
    except BaseException as error:
        errors.append(error)
    finally:
        try:
            if report.is_symlink(): report.unlink()
            publish(report, original)
            require(report.read_bytes() == original, 'terminal report original bytes not restored')
        except BaseException as error:
            errors.append(error)
    try:
        result, _ = driver.read('public-terminal-report-restored', argv); historical(result, 1, agent, 'docs/owned-task.md')
    except BaseException as error:
        errors.append(error)
    if errors: raise BaseExceptionGroup('terminal report refusal restoration and recovery', errors)
    print('PASS public terminal identity unknown usage report contained-link compatibility outside-link refusal and byte recovery', flush=True)
    return dict(work=driver.work, agent=agent['id'], attempt=agent['attempt_id'], task_status='blocked', process_state='interrupted',
                cancelled_before_start=True, provider_execution_started=False, reservation_state='released',
                original_reserved_usd=1, historical_recorded_calls=1, usage_missing=True,
                report_state='partial_possible', contained_report_symlink_accepted=True,
                outside_report_symlink_refused=True, report_byte_recovery=True, accepted=False)


def completed_case(driver):
    """Run one local fake provider and submit its real report on a second Work.

    Keep the first Work/Agent history intact. Public provider preview/apply adds
    a provider while preserving existing configuration and verifying preview
    nonmutation. The existing fixture program records argv/stdin and emits known
    usage/cost; the appended atomic writer creates its actual handoff. No account,
    model, SQL seed, acceptance, gate, retry or global watchdog is involved.
    Budget100/reserve1 are explicit fixture choices, not product defaults.
    Public status/output refusal/recovery reads retain complete DB hashes and
    original channels. A successful child remains distinct from task acceptance.
    """
    previous_work = driver.work
    code, raw, _ = driver.cli('completed-previous-work', ['work', 'show', previous_work])
    require(code == 0, 'completed previous Work read failed'); previous = json.loads(raw)['work']
    code, raw, _ = driver.cli('completed-work-create', ['work', 'create'],
        dict(schema_version=1, event_id='status-completed-create', expected_revision=0,
             title='Completed worker status fixture', objective='Observe submitted local worker result',
             scope='Owned local program only', criteria=['Observe pending review, never acceptance']))
    require(code == 0, 'completed public Work create failed'); work = json.loads(raw)['work']; driver.work = work['id']
    code, raw, _ = driver.cli('completed-task-add', ['task', 'add', driver.work],
        dict(schema_version=1, event_id='status-completed-add', expected_revision=work['revision'],
             id='completed-task', title='Observe real local report', deliverable='Report', criteria=['Actual local process and report']))
    require(code == 0, 'completed public task add failed'); work = json.loads(raw)['work']
    code, raw, _ = driver.cli('completed-budget', ['budget', 'apply', driver.work],
        dict(schema_version=1, event_id='status-completed-budget', expected_revision=work['revision'],
             budget=dict(limit_usd=100, reserve_per_launch_usd=1,
                         estimate_source='Owned local fake result fixture', reference_date='2026-10-23')))
    require(code == 0, 'completed public budget failed'); work = json.loads(raw)
    require(work['id'] == driver.work and type(work['revision']) is int,
            'completed budget did not return actual direct Work')
    program = fixture.PROGRAM + "\nimport tempfile\nfd,name=tempfile.mkstemp(dir='.',prefix='.handoff-')\nwith os.fdopen(fd,'wb') as stream:\n    stream.write(b'Owned local completed worker report; observed fixture only.\\n');stream.flush();os.fsync(stream.fileno())\nos.replace(name,'handoff.md')\n"
    executable = driver.root/'status-worker'; compile(program, str(executable), 'exec')
    publish(executable, program.encode()); os.chmod(executable, 0o700)
    providers = driver.root/'.swarm/providers.json'; original = providers.read_bytes(); configuration = json.loads(original)
    require('completed-fixture' not in configuration['providers'], 'completed provider fixture must be fresh')
    configuration['providers']['completed-fixture'] = dict(command=str(executable), args=['--model', 'fixture-standard'])
    code, raw, _ = driver.cli('completed-provider-inspect', ['providers', 'config', 'inspect'])
    require(code == 0, 'completed provider inspection failed'); current = json.loads(raw)
    request = dict(schema_version=1, expected_digest=current['digest'], configuration=configuration)
    code, raw, _ = driver.cli('completed-provider-preview', ['providers', 'config', 'preview'], request)
    require(code == 0, 'completed public provider preview failed'); preview = json.loads(raw)
    require(preview['preview'] is True and preview['applied'] is False and preview['provider_execution_started'] is False
            and providers.read_bytes() == original, 'completed provider preview mutated configuration or executed provider')
    request['preview_token'] = preview['preview_token']
    code, raw, _ = driver.cli('completed-provider-apply', ['providers', 'config', 'apply'], request)
    require(code == 0, 'completed public provider apply failed'); applied = json.loads(raw)
    saved = json.loads(providers.read_bytes()); old = json.loads(original)
    require(applied['applied'] is True and applied['changed'] is True and applied['provider_execution_started'] is False
            and all(saved['providers'][key] == value for key, value in old['providers'].items())
            and saved['providers']['completed-fixture']['command'] == str(executable)
            and saved['providers']['completed-fixture']['args'] == ['--model', 'fixture-standard'],
            'completed provider configuration did not persist or altered existing provider')
    code, raw, _ = driver.cli('completed-prepare', ['agent', 'prepare', driver.work],
        dict(schema_version=1, event_id='status-completed-session', expected_revision=work['revision'],
             task_id='completed-task', provider='completed-fixture', capture_output=True))
    require(code == 0, 'completed public prepare failed'); prepared = json.loads(raw)['agent']
    code, raw, _ = driver.cli('completed-run', ['agent', 'run', prepared['id']])
    require(code == 0, 'completed local fake provider run failed'); result = json.loads(raw); agent = result['agent']
    require(agent['status'] == 'completed' and result['task_accepted'] is False
            and agent['id'] == prepared['id'] and agent['attempt_id'] == prepared['attempt_id']
            and agent['usage']['input_tokens'] == 7 and agent['usage']['output_tokens'] == 8
            and agent['usage']['provider_reported_cost_usd'] == 25, 'completed fake process declaration or identity differs')
    actual = json.loads((driver.root/'model-received.json').read_bytes())
    require(actual['stdin_sha256'] == digest(prepared['prompt'].encode()) and actual['pid'] > 0
            and actual['pid'] == agent['child_pid'] and actual['ppid'] == agent['supervisor_pid'],
            'completed fake provider did not observe actual prepared stdin')
    report = driver.root/'handoff.md'; report_bytes = report.read_bytes()
    require(report_bytes == b'Owned local completed worker report; observed fixture only.\n',
            'completed fake process report bytes differ')
    code, raw, _ = driver.cli('completed-work-reopen', ['work', 'show', driver.work])
    require(code == 0, 'completed public Work reopen failed'); work = json.loads(raw)['work']
    attempts = copy.deepcopy(work['tasks'][0]['attempts'])
    code, raw, _ = driver.cli('completed-submit', ['task', 'submit', driver.work],
        dict(schema_version=1, event_id='status-completed-submit', expected_revision=work['revision'],
             id='completed-task', agent_id=agent['id'], report_path='handoff.md', report_sha256=digest(report_bytes)))
    require(code == 0, 'completed actual report submission failed'); submitted = json.loads(raw)['work']
    require(submitted['tasks'][0]['status'] == 'submitted' and submitted['tasks'][0]['attempts'] == attempts,
            'completed public submission lost original attempts or invented acceptance')
    argv = [driver.binary, 'mission', 'status', driver.work, '--root', driver.root, '--json']
    result, _ = driver.read('public-completed-submitted', argv); historical(result, 1, completed=agent)
    result, _ = driver.read('public-completed-output-full', argv, output_full=True)
    require(result[0] != 0 and result[1] == b'' and result[2] != b'', 'completed status output failure became success')
    result, _ = driver.read('public-completed-output-recovered', argv); historical(result, 1, completed=agent)
    require(report.read_bytes() == report_bytes, 'completed status changed actual submitted report')
    code, raw, _ = driver.cli('completed-previous-work-preserved', ['work', 'show', previous_work])
    require(code == 0 and json.loads(raw)['work'] == previous,
            'completed new Work changed original cancelled Work history')
    print('PASS public completed local worker submitted report pending review missing gate and output recovery', flush=True)
    return dict(work=driver.work, agent=agent['id'], attempt=agent['attempt_id'], process_state='completed',
                task_status='submitted', result_state='result_to_review', validation_state='missing_gate',
                report_sha256=digest(report_bytes), local_process=True, paid_provider=False, accepted=False,
                declared_fake_cost_usd=25, invoice_measurement=False)



def cooldown_case(driver):
    """Observe cooldown fixtures through public status after a real local worker.

    The prior completed-case is the valid no-cooldown control and supplies an
    actual selected profile. Only owned filesystem cooldown fixtures are replaced;
    no SQL seed, live Root, quota clearing or provider call occurs here. Every
    read/refusal retains DB hashes. Expiry=1 is a past epoch fixture, not a delay.
    Active/expired/cleared/corrupt/restored/absent observations are all required.
    Cleanup and recovery errors remain joined; missing cases cannot produce PASS.
    Admission refusal is separately exercised by the retained public engine test.
    """
    argv = [driver.binary, 'mission', 'status', driver.work, '--root', driver.root, '--json']
    directory = driver.root/'.swarm/provider-cooldowns'
    directory.mkdir(mode=0o700)
    path = directory/'completed-fixture.json'
    active = dict(provider='completed-fixture', observed_at='2026-10-23T00:00:00Z',
                  source='owned-status-fixture', signal='backoff')
    active_bytes = (json.dumps(active, sort_keys=True) + '\n').encode()
    errors = []
    try:
        fixture.publish(path, active_bytes)
        result, _ = driver.read('public-cooldown-active', argv)
        require(result[0] == 0 and result[2] == b'', 'active cooldown status failed')
        require(json.loads(result[1]).get('provider_cooldowns') == [active],
                'active cooldown record was omitted or altered')
        require(path.read_bytes() == active_bytes, 'status changed active cooldown')
        expired = dict(active, resets_at=1)
        fixture.publish(path, expired)
        result, _ = driver.read('public-cooldown-expired', argv)
        require(result[0] == 0 and result[2] == b''
                and 'provider_cooldowns' not in json.loads(result[1]), 'expired cooldown was still active')
        cleared = dict(active, cleared_at='2026-10-23T00:01:00Z')
        fixture.publish(path, cleared)
        result, _ = driver.read('public-cooldown-cleared', argv)
        require(result[0] == 0 and result[2] == b''
                and 'provider_cooldowns' not in json.loads(result[1]), 'cleared cooldown was still active')
        fixture.publish(path, dict(active, cleared_at='PRIVATE-MALFORMED-COOLDOWN'))
        result, _ = driver.read('public-cooldown-corrupt', argv)
        require(result[0] != 0 and result[1] == b''
                and b'agent-provider-cooldown-clear-time-invalid' in result[2]
                and b'PRIVATE-MALFORMED-COOLDOWN' not in result[2],
                'corrupt cooldown lost safe original cause or became success')
        fixture.publish(path, active_bytes)
        result, _ = driver.read('public-cooldown-record-restored', argv)
        require(result[0] == 0 and result[2] == b''
                and json.loads(result[1]).get('provider_cooldowns') == [active]
                and path.read_bytes() == active_bytes, 'cooldown original bytes did not recover')
    except BaseException as error:
        errors.append(error)
    finally:
        try:
            if path.exists(): path.unlink()
            directory.rmdir()
        except BaseException as error:
            errors.append(error)
    try:
        result, _ = driver.read('public-cooldown-absence-restored', argv)
        require(result[0] == 0 and result[2] == b''
                and 'provider_cooldowns' not in json.loads(result[1]), 'cooldown absence did not recover')
    except BaseException as error:
        errors.append(error)
    if errors: raise BaseExceptionGroup('cooldown observations cleanup and recovery', errors)
    print('PASS public selected cooldown active expired cleared corruption and byte recovery', flush=True)
    return dict(active_presented=True, expired_omitted=True, cleared_omitted=True,
                corruption_refused=True, original_bytes_restored=True, absent_recovered=True,
                admission_permission=False, fixture_files_only=True)


def main():
    """Consume profiles, create one owned inactive Root, observe and retain every effect.

    No SDK build/qualification, paid provider, SQL seeding, telemetry, visit mark,
    production restart or global mission-status completion is performed here.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary','profile','sdk-binary','sdk-profile','output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    out = Path(os.path.abspath(args.output)); require(out.parent.resolve() == out.parent, 'status evidence parent must be canonical')
    require(out.parent.stat().st_uid == os.getuid() and not os.path.lexists(out), 'status evidence ownership or freshness differs')
    out.mkdir(mode=0o700)
    root = None
    try:
        binary, sdk = args.binary.resolve(strict=True), args.sdk_binary.resolve(strict=True)
        normal_raw, sdk_raw = args.profile.read_bytes(), args.sdk_profile.read_bytes()
        normal, sdk_profile = json.loads(normal_raw), json.loads(sdk_raw)
        qualify(normal, sdk_profile, binary, sdk)
        publish(out/'method.py', Path(__file__).read_bytes()); publish(out/'profiles.json', dict(normal=normal,sdk=sdk_profile))
        publish(out/'normal-profile.json', normal_raw); publish(out/'sdk-profile.json', sdk_raw)
        profile_hashes = {str(args.profile.resolve()): digest(normal_raw), str(args.sdk_profile.resolve()): digest(sdk_raw)}
        sources = {str(Path(__file__).resolve()): digest(Path(__file__).read_bytes()),
                   str(Path(fixture.__file__).resolve()): digest(Path(fixture.__file__).read_bytes())}
        publish(out/'inputs.json', dict(sources=sources,profile_files=profile_hashes,normal_sha256=normal['sha256'],sdk_sha256=sdk_profile['binary_sha256']))
        guards = profile_guards(normal, sdk_profile, binary, sdk)
        print('PASS status default-core profile control refusal recovery', flush=True)
        repository = Path(__file__).resolve().parents[1]; cache = repository/'.cache'; cache.mkdir(mode=0o700,exist_ok=True)
        require(cache.resolve() == cache and cache.stat().st_uid == os.getuid(), 'status Root cache must be canonical and owned')
        root = Path(tempfile.mkdtemp(prefix='rust-status-contract-',dir=cache)).resolve()
        environment = dict(os.environ)
        require(not any(environment.get(key) for key in ('GIT_DIR','GIT_WORK_TREE','GIT_COMMON_DIR','GIT_INDEX_FILE')), 'status owned Git fixture environment overrides present')
        (root/'codex-home').mkdir(mode=0o700); environment['CODEX_HOME'] = str(root/'codex-home')
        operator = 'owned-status-contract-operator'; driver = Driver(binary,sdk,out,root,operator,environment)
        publish(out/'owned-root.json', dict(schema_version=1,kind=KIND,root=str(root),operator=operator,
                normal_sha256=normal['sha256'],actor_authenticated=False,global_watchdog=None))
        code, _, _ = driver.invoke('git-init',['git','init','-q',root]); require(code == 0,'status owned Git initialization failed')
        code, raw, _ = driver.cli('init',['init']); require(code == 0,'status public initialization failed'); json.loads(raw)
        secret = b'PRIVATE-OWNED-STATUS-FIXTURE'
        code, raw, _ = driver.cli('work-create',['work','create'],dict(schema_version=1,event_id='status-create',expected_revision=0,
            title=secret.decode(),objective=secret.decode(),scope=secret.decode(),criteria=['Observe only; no task acceptance']))
        require(code == 0,'status public Work creation failed'); work = json.loads(raw)['work']; driver.work = work['id']
        marker = dict(schema_version=1,kind=KIND,root=str(root),work=driver.work,operator=operator)
        fixture.publish(root/MARKER,marker)
        result, facts = driver.sdk_read('empty-work'); value = observation(*result,driver.work,facts)
        require(value['task_count'] == 0 and value['revision'] > 0 and value['event_count'] > 0,
                'status initial Work or actual create event missing'); empty_events = value['event_count']; guards += observation_guards(*result,driver.work,facts)
        require(secret not in result[1] and secret not in result[2],'status SDK exposed private empty Work')
        print('PASS status partial API reads empty Work and zero recorded calls without DB mutation',flush=True)
        public, _ = driver.read('public-empty-work',[binary,'mission','status',driver.work,'--root',root,'--json'])
        historical(public,0)
        print('PASS historical public JSON reads actual empty Work without mutation',flush=True)
        code, raw, _ = driver.cli('task-add',['task','add',driver.work],dict(schema_version=1,event_id='status-add',
            expected_revision=value['revision'],id='owned-task',title=secret.decode(),deliverable='owned report',criteria=['Observe only']))
        require(code == 0,'status public task creation failed'); task_work = json.loads(raw)['work']
        result, facts = driver.sdk_read('task-without-profile'); value = observation(*result,driver.work,facts)
        require(value['task_count'] == 1 and value['revision'] > work['revision'] and value['event_count'] > empty_events,
                'status actual task revision or event change absent')
        require(secret not in result[1] and secret not in result[2],'status SDK exposed private task')
        publish(out/'partial-observation.json',value)
        print('PASS status partial API reads real task without profile and preserves absent spending',flush=True)
        marker_case(driver,marker,dict(work='missing-owned-status-work'),'unknown-work','missing-owned-status-work','work-missing',[secret])
        marker_case(driver,marker,dict(operator='PRIVATE-OWNED-OPERATOR-MISMATCH'),'wrong-marker-operator',None,
                    'status-observation-fixture-identity',[secret,b'PRIVATE-OWNED-OPERATOR-MISMATCH'])
        print('PASS status unknown Work and marker operator refusals preserve DB and recover',flush=True)
        argv = [binary,'mission','status',driver.work,'--root',root,'--json']
        result, _ = driver.read('public-todo',argv); historical(result,1)
        result, _ = driver.read('public-unknown-work',[binary,'mission','status','missing-owned-status-work','--root',root,'--json'])
        require(result[0] != 0 and result[1] == b'' and b'work-missing' in result[2], 'public unknown Work refusal missing')
        result, _ = driver.read('public-output-full',argv,output_full=True)
        require(result[0] != 0 and result[1] == b'' and result[2] != b'', 'public status output failure became success')
        result, _ = driver.read('public-output-recovery',argv); historical(result,1)
        print('PASS historical public JSON task unknown-work output refusal and recovery preserve DB',flush=True)
        public_storage_policy_case(driver,argv)
        gate_checks = stale_gate_output_case(driver, task_work, [secret])
        terminal_checks = terminal_case(driver, argv)
        completed_checks = completed_case(driver)
        cooldown_checks = cooldown_case(driver)
        qualify(normal,sdk_profile,binary,sdk)
        require(all(digest(Path(path).read_bytes()) == sha for path,sha in {**sources, **profile_hashes}.items()),'status harness or supplied profile inputs changed')
        require(guards == 13 and len(driver.reads) == 31, 'status declared guard or readonly observation coverage differs')
        publish(out/'result.json',dict(status='PASS',root=str(root),work=driver.work,pure_guard_refusals=guards,
            readonly_observations=driver.reads,readonly_observation_count=len(driver.reads),gate_cause_output_transport=gate_checks,
            terminal_agent=terminal_checks,completed_agent=completed_checks,selected_provider_cooldowns=cooldown_checks,partial_retained_inputs_and_spending_api=True,complete_mission_status=True,
            ordinary_unconfigured_cancelled_and_completed_submitted_scope_only=True,
            observation_is_permission=False,initial_recorded_calls=0,terminal_historical_recorded_calls=1,invoice_measurement=False,usage_zero_imputed=False,
            sdk_persistent_db_mutation=False,freshness_task_coverage_only=True,whole_status_atomic=False,live_root=False,paid_provider=False,installed=False,global_watchdog=None))
        print('REPLAY_EVIDENCE '+str(out),flush=True)
    except BaseException as primary:
        try:
            publish(out/'failure.json',dict(status='FAIL',root=None if root is None else str(root),cause=traceback.format_exc()))
        except BaseException as cleanup:
            raise BaseExceptionGroup('status driver and failure evidence publication',[primary,cleanup])
        raise


if __name__ == '__main__':
    main()
