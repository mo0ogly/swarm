#!/usr/bin/env python3
"""Exercise the ignored engine ledger test on one owned public inert fixture.

The supplied normal CLI prepares a real ready Work; the supplied cfg(test) binary
admits/observes/cancels its own queued worker without executing that worker. No
paid provider, SQL fixture seeding, API addition or global watchdog is used.
Original outputs and the fixture remain available on success or failure. This
recipe proves no installed server, independent acceptance or universal purity.
"""
import argparse
import json
import math
import os
from pathlib import Path
import re
import stat
import struct
import subprocess
import sys
import time

import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as planning

TEST = ('root_store::mission_supervision_store::worker_continuation::ledger_sdk::'
        'continuation_keeps_real_reservation_rows')
KIND = 'owned-inert-continuation-ledger-fixture'


def require(condition, code):
    """Reject a missing precondition with safe static context, never raw data."""
    if not condition:
        raise RuntimeError(code)


def fresh(path):
    """Require an absent child of an existing owned real directory.

    Dangling symlinks also refuse. This is an isolated harness precondition, not
    OS confinement or a concurrent-writer guarantee; no file is deleted here.
    """
    path = Path(path)
    parent = path.parent
    info = parent.lstat()
    require(parent.is_absolute() and stat.S_ISDIR(info.st_mode)
            and info.st_uid == os.getuid() and parent.resolve() == parent,
            'fixture parent ownership or canonical identity invalid')
    require(not os.path.lexists(path), 'fixture output already exists')
    return path


def qualify(normal, test, binary, test_binary):
    """Match actual bytes to retained default normal and engine test profiles.

    Compiler graph collection remains the caller's qualification obligation.
    A test artifact never becomes a production binary through this check.
    """
    require(type(normal) is dict and normal.get('no_fixture_feature') is True
            and normal.get('sha256') == fixture.digest(binary.read_bytes()),
            'normal profile or binary hash mismatch')
    require(type(test) is dict and test.get('status') == 'COMPILED_DEFAULT_TEST_ARTIFACT',
            'engine test profile status invalid')
    artifact = test.get('artifact', {})
    require(type(artifact) is dict and artifact.get('features') == []
            and artifact.get('profile', {}).get('test') is True
            and artifact.get('target', {}).get('name') == 'swarm_engine'
            and artifact.get('target', {}).get('kind') == ['lib']
            and test.get('binary_sha256') == fixture.digest(test_binary.read_bytes()),
            'engine default test profile or binary hash mismatch')


def invoke(argv, environment, output, label):
    """Retain argv and original streams; return actual exit without a deadline.

    Environment values, including private fixture context, are passed unchanged
    but never dumped. Errors retain their Python cause; no retry is attempted.
    The child belongs to subprocess.run, which waits after its interruption
    cleanup; this harness never signals processes based on a numeric PID alone.
    """
    command = fresh(output / (label + '.command.json'))
    fixture.publish(command, dict(argv=list(map(str, argv)), global_watchdog=None,
                    environment_values_retained=False,
                    fixture_environment_forwarded=True,
                    started_unix=time.time()))
    stdout = fresh(output / (label + '.stdout'))
    stderr = fresh(output / (label + '.stderr'))
    with stdout.open('xb') as out, stderr.open('xb') as err:
        result = subprocess.run(list(map(str, argv)), env=environment,
                                stdin=subprocess.DEVNULL, stdout=out, stderr=err)
    fixture.publish(fresh(output / (label + '.exit.json')),
                    dict(exit=result.returncode, ended_unix=time.time()))
    return result.returncode, stdout.read_bytes(), stderr.read_bytes()


def listed(code, stdout):
    """Require the one ignored test by exact listed name, never a zero-test list."""
    require(code == 0, 'ignored test listing failed')
    lines = stdout.decode('utf-8').splitlines()
    require(lines.count(TEST + ': test') == 1, 'exact ignored ledger test absent')
    require(sum(line.endswith(': test') for line in lines) == 1,
            'ignored listing selected unexpected tests')


def ran(code, stdout):
    """Require the exact named test and one successful nonignored execution."""
    require(code == 0, 'ledger engine test failed')
    text = stdout.decode('utf-8')
    names = [line for line in text.splitlines()
             if line.startswith('test ') and not line.startswith('test result:')]
    require(names == ['test ' + TEST + ' ... ok'], 'ledger named test success absent')
    summaries = re.findall(r'^test result:.*$', text, re.M)
    require(len(summaries) == 1 and re.match(
            r'^test result: ok\. 1 passed; 0 failed; 0 ignored;', summaries[0])
            is not None, 'ledger execution count invalid')


def typed_row(row):
    """Validate all four SQLite cells without coercing types or discarding rows.

    Text contains nonempty UTF8 bytes; amount preserves INTEGER or exact REAL
    bits. NULL/blob/unknown cells refuse this normal reservation schema. Limits
    here are SQLite/IEEE/UTF8 representation, not operator execution policies.
    """
    require(type(row) is list and len(row) == 4, 'ledger row shape invalid')
    for index, cell in enumerate(row):
        require(type(cell) is dict and len(cell) == 1, 'ledger cell shape invalid')
        kind, raw = next(iter(cell.items()))
        if index in (0, 1, 3):
            require(kind == 'Text' and type(raw) is list and len(raw) > 0
                    and all(type(n) is int and 0 <= n <= 255 for n in raw),
                    'ledger text cell invalid')
            bytes(raw).decode('utf-8')
        elif kind == 'Integer':
            require(type(raw) is int and -(1 << 63) <= raw < (1 << 63),
                    'ledger integer cell invalid')
        else:
            require(kind == 'Real' and type(raw) is int and 0 <= raw < (1 << 64),
                    'ledger real cell invalid')
            require(math.isfinite(struct.unpack('>d', raw.to_bytes(8, 'big'))[0]),
                    'ledger amount nonfinite')
    return row


def validate_result(value, root, work):
    """Require two exact positive same-Root ledger proofs and completed cleanup.

    root is the owned fixture correlation from the matching input markers; the
    engine output correlates both observations by work/target. It contains no
    OS identity or execution permission. All raw rows/types are compared, not
    only counters or booleans; missing/empty results never qualify as success.
    """
    require(type(value) is dict and value.get('status') == 'PASS',
            'ledger result status invalid')
    require(root.is_absolute(), 'ledger fixture root must be absolute')
    require(value.get('scope') == 'owned-queued-agent-reservations-same-Root-connection',
            'ledger result scope invalid')
    for key in ('reservation_rows_verified', 'lease_released', 'lease_closed', 'root_closed'):
        require(value.get(key) is True, 'ledger proof or cleanup missing')
    for key in ('observations_are_permission', 'provider_spawn_called'):
        require(value.get(key) is False, 'ledger proof claims unsupported effects')
    observations = value.get('observations')
    require(type(observations) is list and len(observations) == 2,
            'ledger needs exactly two observations')
    target = None
    previous = None
    for ordinal, observation in enumerate(observations, 1):
        require(type(observation) is dict and type(observation.get('ordinal')) is int
                and observation['ordinal'] == ordinal, 'ledger ordinal invalid')
        for key in ('all_rows_equal', 'agent_work_receipts_fifo_equal', 'target_unique',
                    'current_generation', 'total_changes_unchanged', 'positive_target_reservation'):
            require(observation.get(key) is True, 'ledger observation assertion missing')
        agent, bits = observation.get('target_agent_id'), observation.get('target_amount_ieee_bits')
        require(type(agent) is str and bool(agent) and observation.get('target_work_id') == work,
                'ledger target correlation invalid')
        require(type(bits) is int and 0 <= bits < (1 << 64), 'ledger target bits invalid')
        amount = struct.unpack('>d', bits.to_bytes(8, 'big'))[0]
        require(math.isfinite(amount) and amount > 0, 'ledger target amount not positive finite')
        current = (agent, bits)
        require(target is None or target == current, 'ledger target changed between observations')
        target = current
        before, after = observation.get('ledger_before'), observation.get('ledger_after')
        require(type(before) is list and len(before) > 0 and type(after) is list,
                'ledger rows absent or empty')
        for row in before + after:
            typed_row(row)
        require(before == after, 'ledger rows differ despite assertion')
        require(previous is None or previous == before, 'ledger changed between observations')
        previous = after
        expected = [{'Text': list(agent.encode())}, {'Text': list(work.encode())},
                    {'Real': bits}, {'Text': list(b'reserved')}]
        require(sum(row == expected for row in before) == 1
                and sum(row[0] == expected[0] for row in before) == 1,
                'ledger exact positive reserved target absent or duplicated')
    return value


def exercise(binary, test_binary, output):
    """Prepare one fresh Work and preserve its actual Fixture environment.

    The engine test receives matching external/Root markers and a fresh output.
    No cleanup removes evidence or data; engine stop/reconcile/lease cleanup is
    asserted through its returned proof. Primary/publication failures are joined.
    """
    setup = output / 'setup'
    setup.mkdir(mode=0o700)
    ready, owned = planning.flow(binary, setup, before_worker=True, return_fixture=True)
    root = owned.root
    require(root.is_absolute() and root.resolve() == root and root.lstat().st_uid == os.getuid(),
            'public fixture Root identity invalid')
    require(ready.get('status') == 'READY_FOR_NATIVE_WORKER'
            and ready.get('root') == str(root) and ready.get('work_id') == owned.work
            and ready.get('worker_started') is False and ready.get('accepted') is False,
            'public ready fixture contract invalid')
    require(not (root / 'model-received.json').exists(), 'worker fixture already entered')
    result = fresh(output / 'engine-result.json')
    value = dict(schema_version=1, kind=KIND, root=str(root), work=owned.work,
                 result=str(result))
    marker = fresh(root / '.swarm' / 'verification-continuation-ledger.json')
    descriptor = fresh(output / 'fixture.json')
    fixture.publish(marker, value)
    fixture.publish(descriptor, value)
    require(json.loads(marker.read_bytes()) == json.loads(descriptor.read_bytes()),
            'fixture marker publication mismatch')
    environment = dict(owned.environment)
    environment['SWARM_CONTINUATION_LEDGER_FIXTURE'] = str(descriptor)
    print('PHASE ledger fixture ready; invoking one ignored engine test', flush=True)
    code, stdout, _ = invoke([test_binary, TEST, '--ignored', '--exact', '--nocapture'],
                             environment, output, 'ledger')
    ran(code, stdout)
    require(not (root / 'model-received.json').exists(), 'worker provider unexpectedly entered')
    require(result.is_file() and not result.is_symlink(), 'engine ledger result absent or substituted')
    proof = json.loads(result.read_bytes())
    validate_result(proof, root, owned.work)
    return proof


def publish_failure(output, error):
    """Retain safe failure metadata without replacing its primary exception.

    The caller rethrows the original after successful publication. A publication
    failure joins both original exception objects; no partial evidence is erased.
    """
    try:
        fixture.publish(fresh(output / 'failure.json'),
                        dict(status='FAIL', error_type=type(error).__name__,
                             evidence_preserved=True, global_watchdog=None))
    except BaseException as publication:
        raise BaseExceptionGroup('ledger verification/publication', [error, publication])


def main():
    """Qualify two supplied artifacts, list one ignored test and run one fixture.

    Fresh evidence and original failures survive; no global watchdog is added.
    Fixture CODEX_HOME is preserved without exposing its environment contents.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'test-binary', 'test-profile', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    binary, test_binary, output = args.binary.resolve(), args.test_binary.resolve(), args.output.resolve()
    output.mkdir(parents=True, mode=0o700, exist_ok=False)
    try:
        fixture.publish(output / 'method.py', Path(__file__).read_bytes())
        normal, test = json.loads(args.profile.read_bytes()), json.loads(args.test_profile.read_bytes())
        qualify(normal, test, binary, test_binary)
        fixture.publish(output / 'profiles.json', dict(normal=normal, engine_test=test))
        sources = {str(Path(module.__file__).resolve()): fixture.digest(Path(module.__file__).read_bytes())
                   for name, module in tuple(sys.modules.items())
                   if name.startswith('rust_') and getattr(module, '__file__', None)}
        sources[str(Path(__file__).resolve())] = fixture.digest(Path(__file__).read_bytes())
        fixture.publish(output / 'inputs.json', dict(sources=sources,
                        binary_sha256=normal['sha256'], test_sha256=test['binary_sha256']))
        code, stdout, _ = invoke([test_binary, TEST, '--list', '--ignored', '--exact'],
                                 dict(os.environ), output, 'list')
        listed(code, stdout)
        proof = exercise(binary, test_binary, output)
        qualify(normal, test, binary, test_binary)
        require(all(fixture.digest(Path(path).read_bytes()) == sha for path, sha in sources.items()),
                'harness sources changed during execution')
        fixture.publish(fresh(output / 'result.json'), dict(status='PASS', engine=proof,
                        source_stable=True, global_watchdog=None, paid_provider=False,
                        worker_provider_started=False, fixture_planning_uses_inert_processes=True,
                        accepted=False, installed=False))
        print('PASS real nonempty reservation ledger preserved across two continuation observations', flush=True)
        print('REPLAY_EVIDENCE=' + str(output), flush=True)
    except BaseException as error:
        publish_failure(output, error)
        raise


if __name__ == '__main__':
    main()
