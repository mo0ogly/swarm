#!/usr/bin/env python3
"""Observe a root-admitted Rust libtest pair, without building or retrying.

Inputs: an explicit build record and fresh evidence directory; optional global
mode requires the record's owned compiler repository/marker. One named Rust probe
is invoked per case/variant. Compilation, zero tests, failed controls, IO failures
and invalid reports refuse; they never count as behavioral RED. Declared source
hashes/binary are rechecked before/after, not inferred from a record's name.
This is self-consistency, not compiler-graph qualification or execution authority.
No provider/service, implicit shell, timeout, installation or cache reset.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import stat
import subprocess
import sys
import native_build_outcome as outcomes

CASES = ('presence', 'missing', 'json', 'permission', 'cache', 'concurrency')
PAIRED = 'fixture_area::paired_harness::paired_probe'
GLOBAL = 'fixture_area::paired_harness::global_wrapper_probe'


def sha(path):
    """Hash full retained bytes; read failures propagate instead of empty success."""
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_new(path, value):
    """Publish a fresh owned JSON artifact, flushing; never replace historical data.

    IO errors propagate and leave the run incomplete; no rollback/retry is implied.
    """
    with Path(path).open('x', encoding='utf-8') as stream:
        os.fchmod(stream.fileno(), 0o600)
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def require(condition, reason):
    """Refuse an unmet prerequisite/observable even under optimized Python."""
    if not condition:
        raise ValueError(reason)


def admit(record_path):
    """Read the explicit root-selected profile/source/binary record.

    Verify exact bytes and the libtest scope. Root must independently qualify the
    effective compiler graph and storage beforehand; this parser grants no rights.
    The record must include every effective input, plus all harness sources.
    """
    record = json.loads(record_path.read_bytes())
    require(isinstance(record, dict), 'pair-build-record-object')
    require(type(record.get('schema_version')) is int and record['schema_version'] == 1, 'pair-build-record-schema')
    require(record.get('scope') == 'cd01-paired-libtest', 'pair-build-record-scope')
    require(record.get('base') == '230d4b25441a4fa83dc2d993e20bcbc60f193ab8', 'pair-build-base')
    require(isinstance(record.get('profile'), dict) and type(record['profile'].get('all_features')) is bool
            and type(record['profile'].get('test_fixtures')) is bool, 'pair-profile-types')
    require(record.get('profile') == {'package': 'swarm-engine', 'kind': 'lib', 'cargo_profile': 'test',
                                    'all_features': False, 'features': [], 'test_fixtures': False, 'panic': 'unwind'}, 'pair-build-profile')
    require(isinstance(record.get('compiler_inputs'), dict) and record['compiler_inputs'], 'pair-compiler-inputs-missing')
    require(Path(record['binary']).is_absolute(), 'pair-binary-absolute-required')
    binary = Path(record['binary']).resolve(strict=True)
    require(binary.is_file() and os.access(binary, os.X_OK), 'pair-binary-unavailable')
    require(not binary.stat().st_mode & (stat.S_ISUID | stat.S_ISGID), 'pair-setid-binary-refused')
    require(sha(binary) == record['binary_sha256'], 'pair-binary-mismatch')
    root = Path(record['compiler_repository']).resolve(strict=True)
    required = {'rust/crates/swarm-engine/Cargo.toml', 'rust/crates/swarm-engine/src/lib.rs',
                'rust/crates/swarm-engine/src/fixture_area/resolution_tests.rs',
                'rust/crates/swarm-engine/src/fixture_area.rs',
                'rust/crates/swarm-engine/src/fixture_area/resolution.rs',
                'rust/crates/swarm-engine/src/fixture_area/paired_harness.rs',
                'rust/crates/swarm-engine/src/fixture_area/paired_legacy.rs',
                'rust/crates/swarm-engine/src/engine_settings.rs',
                'rust/crates/swarm-engine/src/engine_settings/runtime_temporary.rs',
                'rust/crates/swarm-engine/config/engine-settings.json', 'rust/Cargo.lock', 'rust/Cargo.toml'}
    require(required.issubset(record.get('harness_source_relpaths', [])), 'pair-required-harness-inputs')
    for rel in required:
        require(str(root / rel) in record['compiler_inputs'], 'pair-required-source-not-in-compiler-inputs')
    # Merely naming all these paths does not establish dependency closure.
    producer = Path(__file__).with_name('native_build_outcome.py').resolve(strict=True)
    require(Path(outcomes.__file__).resolve(strict=True) == producer, 'pair-outcome-module-location')
    require(sha(producer) == '27b2bf5ebc57647876c00a338685cde71eaacea89d4bf1b55894fa33da5722ba', 'pair-outcome-source-pin')
    record['_outcome_source'] = {'path': str(producer), 'sha256': sha(producer)}
    controller = Path(__file__).resolve(strict=True)
    require(record.get('controller_sha256') == sha(controller), 'pair-controller-source-pin')
    record['_controller_source'] = {'path': str(controller), 'sha256': sha(controller)}
    check_inputs(record)
    return record, binary, root


def joined_failure(label, errors, custody):
    """Keep original typed causes and custody on a single exception/group.

    Metadata describes observed completion/publication, not rollback or success.
    Do not format arbitrary child bytes or infer an exit from their presence.
    """
    error = errors[0] if len(errors) == 1 else BaseExceptionGroup(label, errors)
    previous = getattr(error, 'cd01_custody', None)
    if previous is not None and previous is not custody:
        custody.setdefault('earlier_custody', []).append(previous)
    error.cd01_custody = custody
    if isinstance(custody.get('native_outcome'), dict):
        error.native_outcome = custody['native_outcome']
    return error


def check_inputs(record):
    """Independently observe every admitted source AND image, even after errors.

    Failed reads are unknown hashes, observed mismatches remain known changes.
    All original errors and per-path attempted/known states reach the consumer.
    This is readback, not compiler-graph qualification or authentication.
    """
    facts = {'attempted': True, 'sources': {}, 'image': None, 'ok': None}
    errors = []
    items = [('sources', name, expected) for name, expected in record['compiler_inputs'].items()]
    items.append(('image', record['binary'], record['binary_sha256']))
    for key in ('_controller_record', '_outcome_source', '_controller_source'):
        if key in record:
            items.append(('sources', record[key]['path'], record[key]['sha256']))
    for kind, name, expected in items:
        observation = {'attempted': True, 'sha256': None, 'ok': None}
        try:
            path = Path(name)
            require(path.is_absolute() and path.is_file(), 'pair-input-path')
            observation['sha256'] = sha(path)
            observation['ok'] = observation['sha256'] == expected
            require(observation['ok'], 'pair-image-changed' if kind == 'image' else 'pair-compiler-input-changed')
        except BaseException as error:
            observation['ok'] = False
            observation['error'] = outcomes.describe(error)
            errors.append(error)
        if kind == 'image': facts['image'] = observation
        else: facts['sources'][name] = observation
    facts['ok'] = not errors
    if errors:
        error = joined_failure('CD01 input/image observations refused', errors, {'input_observation': facts})
        error.input_observation = facts
        raise error
    return facts


class InputAdmission:
    """Adapt CD01 exact-input checks to the unchanged common outcome producer.

    Every check records known/unknown observations even on refusal. This grants
    no storage/build authority and never reruns a process.
    """
    def __init__(self, record):
        """Own this invocation's record reference and observation sequence."""
        self.record, self.observations = record, []

    def check(self):
        """Observe inputs independently and retain either result or failed facts."""
        try:
            result = check_inputs(self.record)
        except BaseException as error:
            self.observations.append(getattr(error, 'input_observation', {'attempted': True, 'ok': None, 'error': outcomes.describe(error)}))
            raise
        self.observations.append(result)
        return result


def checked_phase(record, work, custody, phase):
    """Always attempt post-validation input/image observation before propagation.

    A primary child/parser/publication error does not skip this check; postcheck
    failure is secondary and cannot replace it. Unknown states remain explicit.
    """
    errors, value = [], None
    try:
        value = work()
    except BaseException as error:
        errors.append(error)
    post = {'attempted': True, 'ok': None, 'phase': phase}
    try:
        post['facts'] = check_inputs(record)
        post['ok'] = True
    except BaseException as error:
        post['ok'] = False
        post['facts'] = getattr(error, 'input_observation', None)
        post['error'] = outcomes.describe(error)
        errors.append(error)
    custody['post_validation_inputs'] = post
    if errors:
        raise joined_failure('CD01 phase and independent postcheck failed', errors, custody)
    return value


def retained_frames(error, seen=None):
    """Traverse real groups/cause/context to retain custody on nested failures.

    The common failure_document keeps primary/secondary exceptions; these frames
    additionally carry input/child observations without relying on error text.
    """
    seen = set() if seen is None else seen
    if id(error) in seen: return []
    seen.add(id(error))
    frames = []
    for key in ('cd01_custody', 'native_outcome', 'input_observation'):
        if hasattr(error, key): frames.append({key: getattr(error, key)})
    for child in getattr(error, 'exceptions', ()):
        frames.extend(retained_frames(child, seen))
    if error.__cause__ is not None: frames.extend(retained_frames(error.__cause__, seen))
    elif error.__context__ is not None and not error.__suppress_context__:
        frames.extend(retained_frames(error.__context__, seen))
    return frames


def failure_document(error):
    """Carry the common typed error tree plus all actual custody frames to final
    failure.json/fallback; known numeric exits never become a generic message."""
    value = outcomes.failure_document(error)
    value['cd01_custody_frames'] = retained_frames(error)
    value['status'] = 'INCOMPLETE_OR_REFUSED'
    value['behavioral_red_established_by_failure'] = False
    return value


def sync_digest(path, observations):
    """After common producer captures returncode/signal, independently attempt
    channel fsync, close and hash, retaining every original failure. Unbuffered
    producer channels need no Python flush; a failed sync never skips stderr's
    independent producer iteration. No retry of an uncertain close.
    """
    state = {'attempted': True, 'sync': None, 'close': None, 'sha256': None}
    errors, stream = [], None
    try:
        stream = path.open('rb', buffering=0)
        try:
            os.fsync(stream.fileno()); state['sync'] = {'ok': True}
        except BaseException as error:
            state['sync'] = {'ok': False, 'error': outcomes.describe(error)}; errors.append(error)
    except BaseException as error:
        state['sync'] = {'ok': None, 'error': outcomes.describe(error)}; errors.append(error)
    finally:
        if stream is not None:
            try: stream.close(); state['close'] = {'ok': True}
            except BaseException as error:
                state['close'] = {'ok': False, 'ownership_uncertain': True, 'error': outcomes.describe(error)}; errors.append(error)
    try: state['sha256'] = sha(path)
    except BaseException as error: state['hash_error'] = outcomes.describe(error); errors.append(error)
    observations[path.name] = state
    if errors: raise joined_failure('CD01 channel custody refused', errors, {'channel': state})
    return state['sha256']


def invoke(record, binary, name, root, token, output, variant, case):
    """Use common process custody, then independently validate retained reports
    and post-observe inputs even on every failure. The exact manual probe is
    explicitly selected with --ignored; ordinary libtest runs must skip it.
    Never infer completion from
    channel existence. Known hash-bound stdout permits diagnostic validation after
    collector errors; this never permits PASS. One child, EOF, no watchdog/retry.
    """
    env = dict(os.environ)
    overrides = {'SWARM_CD01_PAIR_ROOT': str(root), 'SWARM_CD01_PAIR_TOKEN': token,
                 'SWARM_CD01_PAIR_VARIANT': variant, 'SWARM_CD01_PAIR_CASE': case}
    env.update(overrides)
    argv = [str(binary), name, '--exact', '--ignored', '--nocapture', '--test-threads=1']
    custody = {'argv': argv, 'environment_overrides': overrides, 'completion': 'unknown',
               'native_outcome': None, 'channel_observations': {}, 'validation': {'attempted': False}}
    record.setdefault('_invocations', []).append(custody)
    admission = InputAdmission(record)

    def publish_native(path, text):
        """Capture producer's already typed completion before fallible receipt
        publication. A refused write may be visible/uncertain, never rollback.
        """
        value = json.loads(text)
        if path.name == 'probe-result.json':
            custody['native_outcome'] = {'subprocess': value}
        elif path.name == 'probe-admission-after.json':
            custody['native_outcome'] = value
        write_new(path, value)

    def work():
        """Keep every collector/protocol cause, then expose one coherent result
        only if neither custody nor validation has failed.
        """
        errors, result, data = [], None, None
        write_new(output / 'command-intent.json', {'argv': argv, 'environment_overrides': overrides, 'global_watchdog': None})
        try:
            result = outcomes.execute(argv, out=output, name='probe', admission=admission,
                publish=publish_native, digest=lambda path: sync_digest(path, custody['channel_observations']),
                failure_code='pair-child-nonzero', env=env)
        except BaseException as error:
            errors.append(error)
            if isinstance(getattr(error, 'native_outcome', None), dict): custody['native_outcome'] = error.native_outcome
        custody['admission_observations'] = admission.observations
        native = custody.get('native_outcome')
        process = native.get('subprocess') if isinstance(native, dict) else None
        if result is not None:
            custody['observed_returncode'] = result.returncode
        elif isinstance(process, dict):
            custody['observed_returncode'] = process.get('returncode')
        known = isinstance(process, dict) and process.get('execution_attempted') is True and type(process.get('returncode')) is int
        custody['completion'] = 'known' if known else 'unknown'
        bound = process.get('stdout_sha256') if known else None
        if known and isinstance(bound, str) and len(bound) == 64 and all(c in '0123456789abcdef' for c in bound):
            custody['validation']['attempted'] = True
            try:
                stdout = (output / 'probe.stdout').read_bytes()
                require(hashlib.sha256(stdout).hexdigest() == bound, 'pair-retained-stdout-changed')
                raw = stdout.decode('utf-8')
                require('running 1 test' in raw and 'test result: ok. 1 passed; 0 failed;' in raw, 'pair-one-named-test-not-established')
                require(name in raw, 'pair-named-test-missing')
                marker = ('CD01_PAIR_PROBE_COMPLETE ' + variant + ' ' + case) if name == PAIRED else ('CD01_GLOBAL_WRAPPER_COMPLETE ' + case)
                require(marker in raw, 'pair-completion-marker-missing')
                raw_probe = (root / 'probe.json').read_bytes()
                with (output / 'probe.original.json').open('xb') as retained:
                    os.fchmod(retained.fileno(), 0o600)
                    retained.write(raw_probe); retained.flush(); os.fsync(retained.fileno())
                for artifact in ('fault-input.bin', 'after-fault-input.bin', 'effect-count.bin'):
                    if (root / artifact).is_file():
                        with (output / artifact).open('xb') as retained:
                            os.fchmod(retained.fileno(), 0o600)
                            retained.write((root / artifact).read_bytes()); retained.flush(); os.fsync(retained.fileno())
                data = json.loads(raw_probe)
                require(isinstance(data, dict), 'pair-report-object')
                require(type(data.get('schema_version')) is int and data['schema_version'] == 1 and data.get('case') == case, 'pair-report-identity')
                write_new(output / 'probe-copy.json', data)
                custody['validation']['ok'] = True
            except BaseException as error:
                custody['validation']['ok'] = False
                custody['validation']['error'] = outcomes.describe(error)
                errors.append(error)
        else:
            custody['validation']['reason'] = 'completion-or-stdout-binding-unknown'
            if not errors: errors.append(ValueError('pair-known-completion-and-channel-required'))
        if errors: raise joined_failure('CD01 collector and report validation failed', errors, custody)
        return data

    return checked_phase(record, work, custody, 'after-child-and-report-validation')


def meets_target(data, root):
    """Judge the same named behavioral contract for either implementation.

    No missing API/build fact can reach this function. Only actual observed
    values, causes, retained decisions and inert effects determine RED/green.
    Fixed-only access/Arc metadata is judged separately, not used to invent RED.
    """
    require(data.get('scope') == 'owned-adapted-context-pair', 'pair-report-scope')
    require(data['control']['kind'] == 'path' and data['control']['path'] == str(root / 'saved-A'), 'pair-control-invalid')
    require(data['recovery']['kind'] == 'path' and data['recovery']['path'] == str(root / 'new-B'), 'pair-recovery-invalid')
    before = data['effects']['before_fault']
    require(before == 1, 'pair-control-effect-invalid')
    calls, retained, effects = data['observed'], data['retained'], data['effects']
    require(len(calls) == (4 if data['case'] == 'concurrency' else 1), 'pair-call-count')
    if data['case'] == 'presence':
        return (all(c.get('kind') == 'path' and c.get('path') == str(root / 'saved-A') for c in calls)
                and retained.get('path') == str(root / 'saved-A')
                and effects == {'before_fault': 1, 'after_fault': 2, 'after_retained': 3, 'after_recovery': 4})
    # Cache/concurrency RED must measure retained behavior after repair, not
    # merely the newer typed error channel. JSON is the separate cause contract.
    retained_refusal = retained.get('kind') == 'refusal'
    if data['case'] in ('cache', 'concurrency'):
        refused = all(c.get('kind') == 'refusal' for c in calls)
    else:
        refused = all(c.get('kind') == 'refusal' and c.get('typed') is True for c in calls)
        retained_refusal = retained_refusal and retained.get('typed') is True
    if data['case'] == 'missing':
        refused = refused and all(c.get('stage') == 'settings-runtime-temporary-not-configured' for c in calls)
    elif data['case'] == 'permission':
        refused = refused and all(c.get('errno') == 13 for c in calls)
    elif data['case'] == 'json':
        refused = refused and all(c.get('json_cause') is True for c in calls)
    return (refused and retained_refusal
            and effects == {'before_fault': 1, 'after_fault': 1, 'after_retained': 1, 'after_recovery': 2})


def pairs(record, binary, output):
    """Run six shared scenarios for each variant, requiring valid controls and
    explicit behavioral legacy RED on the five refusal/cause/cache cases.
    Captured legacy opacity is evidence, not a guessed modern error or skip.
    """
    outcomes = {}
    identities = {}
    for case in CASES:
        for variant in ('legacy', 'fixed'):
            run = output / (case + '-' + variant)
            run.mkdir(mode=0o700)
            root = run / 'repo'; root.mkdir(mode=0o700)
            token = secrets.token_hex(16)
            (root / '.cd01-pair-owner').write_text(token, encoding='utf-8')
            data = invoke(record, binary, PAIRED, root, token, run, variant, case)
            require(data.get('variant') == variant, 'pair-variant-mismatch')
            require(data['identity']['euid'] == os.geteuid() and data['identity']['egid'] == os.getegid(), 'pair-child-identity-mismatch')
            require(data['identity']['root_mode'] == 0o700, 'pair-root-mode')
            if case in identities:
                require(data['identity'] == identities[case], 'pair-preconditions-identity-mode-differ')
            else:
                identities[case] = data['identity']
            require(data.get('settings_unchanged') is True and data['fault_bytes_sha256'] == data['after_fault_bytes_sha256'], 'pair-settings-mutated')
            target = meets_target(data, root)
            if variant == 'fixed':
                require(target, 'pair-fixed-target-refused-' + case)
                require(sum(c.get('performed_access') is True for c in data['observed']) == 1, 'pair-fixed-access-count')
                require(data['retained'].get('performed_access') is False, 'pair-fixed-retained-access')
                if case != 'presence':
                    require(data.get('same_retained_refusal') is True, 'pair-fixed-generation-not-retained')
                if case == 'concurrency':
                    require(data.get('same_refusal_generation') is True, 'pair-fixed-concurrent-generation')
            else:
                require(target == (case == 'presence'), 'pair-legacy-counterexample-not-established-' + case)
            outcomes[case + ':' + variant] = 'TARGET_MET' if target else 'BEHAVIORAL_RED'
    return outcomes


def global_processes(record, binary, compiler_root, output):
    """Separately exercise the actual AFTER wrapper in three fresh processes.

    Require an explicit owned build-root marker and no prior .swarm/report. Only
    own generated config/report is renamed between children, preserving originals.
    Compiler config/sources are never replaced. This is not paired legacy proof.
    """
    token = record['owned_repository_token']
    require((compiler_root / '.cd01-build-owner').read_text() == token, 'pair-global-build-owner')
    shared = Path(__file__).resolve().parents[2]
    require(compiler_root != shared and compiler_root != Path('/home/fpizzi/Documents/ChatGPT/swarm'), 'pair-global-shared-root-refused')
    require(not (compiler_root / '.git').exists() and not (compiler_root / '.git').is_symlink(), 'pair-global-checkout-refused')
    for name in ('.swarm', 'probe.json', '.cd01-pair-owner', '.cd01-control-settings', 'probe-control.json', 'probe-refusal.json'):
        require(not (compiler_root / name).exists() and not (compiler_root / name).is_symlink(), 'pair-global-root-not-fresh')
    (compiler_root / '.cd01-pair-owner').write_text(token, encoding='utf-8')
    results = {}
    for case in ('control', 'refusal', 'adoption'):
        run = output / case; run.mkdir(mode=0o700)
        data = invoke(record, binary, GLOBAL, compiler_root, token, run, 'fixed', case)
        require(data.get('scope') == 'fixed-global-wrapper-process', 'pair-global-report-scope')
        if case == 'control':
            require(data.get('path') == str(compiler_root / 'saved-A'), 'pair-global-control')
            (compiler_root / '.swarm').rename(compiler_root / '.cd01-control-settings')
            (compiler_root / 'probe.json').rename(compiler_root / 'probe-control.json')
        elif case == 'refusal':
            require(data.get('same_refusal_generation') is True and data.get('no_cache_reset') is True, 'pair-global-retained-refusal')
            require(data['first'].get('typed') is True and data['first'].get('json_cause') is True, 'pair-global-original-cause')
            (compiler_root / 'probe.json').rename(compiler_root / 'probe-refusal.json')
        else:
            require(data.get('path') == str(compiler_root / 'new-B') and data.get('fresh_process') is True, 'pair-global-fresh-adoption')
        results[case] = data
    return results




def verify_retained(folder, *, final_controller=True):
    """Verify a retained inner manifest without launching a probe/build.

    Hash self-consistency is not authentication or independent acceptance. Refuse
    missing/escaping files and incomplete terminal metadata; never repair them.
    """
    folder = Path(folder).resolve(strict=True)
    require(not (folder / 'failure.json').exists() and not (folder / 'secondary-volume-close.json').exists(), 'pair-retained-run-refused')
    seal = json.loads((folder / 'seal.json').read_bytes())
    require(seal.get('status') == 'PASS' and seal.get('artifacts_sha256') == sha(folder / 'artifacts.json'), 'pair-seal-incomplete')
    manifest = json.loads((folder / 'artifacts.json').read_bytes())
    require(isinstance(manifest, dict) and {'result.json', 'admitted-record-copy.json', 'storage-readback.json'}.issubset(manifest), 'pair-inner-manifest-incomplete')
    for name, expected in manifest.items():
        rel = Path(name)
        require(not rel.is_absolute() and '..' not in rel.parts, 'pair-inner-artifact-name')
        path = folder / rel
        require(path.resolve(strict=True).is_relative_to(folder) and path.is_file(), 'pair-inner-artifact-escape')
        require(sha(path) == expected, 'pair-inner-artifact-changed')
    result = json.loads((folder / 'result.json').read_bytes())
    require(result.get('status') == 'RECORDED_TARGETS_MET' and result.get('scope') in ('paired', 'global'), 'pair-inner-status')
    if final_controller:
        verify_terminal_receipt(folder, result)


TERMINAL_BINDINGS = ('controller-postcheck.json', 'seal.json', 'artifacts.json',
                     'admitted-record-copy.json', 'result.json')


def validate_post_observation(post, record, phase):
    """Validate retained successful source/image facts against the admitted copy.

    This checks historical observation shape/values, not current source bytes or
    compiler dependency closure. Unknown/failed/missing facts refuse; read/JSON
    failures propagate with their original causes. No child or repair is invoked.
    """
    require(isinstance(post, dict) and post.get('phase') == phase
            and post.get('attempted') is True and post.get('ok') is True
            and 'error' not in post, 'pair-terminal-postcheck-refused')
    require(isinstance(record, dict) and isinstance(record.get('compiler_inputs'), dict)
            and record['compiler_inputs'], 'pair-terminal-record-inputs')
    expected = dict(record['compiler_inputs'])
    for key in ('_controller_record', '_controller_source', '_outcome_source'):
        item = record.get(key)
        require(isinstance(item, dict) and isinstance(item.get('path'), str)
                and Path(item['path']).is_absolute(), 'pair-terminal-record-pin')
        if item['path'] in expected:
            require(expected[item['path']] == item.get('sha256'), 'pair-terminal-conflicting-pin')
        expected[item['path']] = item.get('sha256')
    def successful(observation, pin):
        """Require a typed complete hash observation; bool/unknown never pass."""
        require(isinstance(pin, str) and len(pin) == 64
                and all(char in '0123456789abcdef' for char in pin), 'pair-terminal-pin-shape')
        require(isinstance(observation, dict) and observation.get('attempted') is True
                and observation.get('ok') is True and observation.get('sha256') == pin
                and 'error' not in observation, 'pair-terminal-observation-refused')
    facts = post.get('facts')
    require(isinstance(facts, dict) and facts.get('attempted') is True
            and facts.get('ok') is True and 'error' not in facts
            and isinstance(facts.get('sources'), dict)
            and set(facts['sources']) == set(expected), 'pair-terminal-source-set')
    for name, pin in expected.items():
        require(isinstance(name, str) and Path(name).is_absolute(), 'pair-terminal-source-path')
        successful(facts['sources'][name], pin)
    require(isinstance(record.get('binary'), str) and Path(record['binary']).is_absolute(), 'pair-terminal-image-path')
    successful(facts.get('image'), record.get('binary_sha256'))


def verify_terminal_receipt(folder, result):
    """Consume the separately sealed late receipt and its two source snapshots.

    Read each binding from the exact bytes consumed; validate JSON schema/phase,
    successful facts and admitted record/image relations. Binding is local hash
    self-consistency, not authenticity, current inputs, controller exit0 or
    independent acceptance. Any IO/parse/hash/semantic failure propagates. Never
    repair, replay a probe or infer rollback from a visible incomplete write.
    """
    envelope_path = folder / 'terminal-integrity.json'
    require(envelope_path.resolve(strict=True).is_relative_to(folder) and envelope_path.is_file(), 'pair-terminal-envelope-path')
    envelope = json.loads(envelope_path.read_bytes())
    require(isinstance(envelope, dict) and set(envelope) == {
            'schema_version', 'scope', 'bindings', 'publication_postcheck'}, 'pair-terminal-envelope-shape')
    require(type(envelope['schema_version']) is int and envelope['schema_version'] == 1
            and envelope['scope'] == 'cd01-retained-terminal-integrity', 'pair-terminal-envelope-schema')
    require(isinstance(envelope['bindings'], dict)
            and set(envelope['bindings']) == set(TERMINAL_BINDINGS), 'pair-terminal-bindings')
    retained = {}
    for name in TERMINAL_BINDINGS:
        path = folder / name
        require(path.resolve(strict=True).is_relative_to(folder) and path.is_file(), 'pair-terminal-path')
        raw = path.read_bytes()
        require(hashlib.sha256(raw).hexdigest() == envelope['bindings'][name], 'pair-terminal-bytes-changed')
        retained[name] = json.loads(raw)
    require(isinstance(retained['seal.json'], dict) and retained['seal.json'].get('status') == 'PASS'
            and retained['seal.json'].get('artifacts_sha256') == envelope['bindings']['artifacts.json'], 'pair-terminal-inner-seal')
    require(isinstance(retained['artifacts.json'], dict), 'pair-terminal-inner-manifest')
    for name in ('admitted-record-copy.json', 'result.json'):
        require(retained['artifacts.json'].get(name) == envelope['bindings'][name], 'pair-terminal-inner-binding')
    record, receipt = retained['admitted-record-copy.json'], retained['controller-postcheck.json']
    require(retained['result.json'] == result, 'pair-terminal-result-readback-changed')
    require(isinstance(receipt, dict) and receipt.get('phase') == 'controller-work-and-publication', 'pair-terminal-receipt-phase')
    validate_post_observation(receipt.get('post_validation_inputs'), record, 'after-controller-and-evidence-publication')
    validate_post_observation(envelope['publication_postcheck'], record, 'after-controller-postcheck-publication')
    require(result.get('record_sha256') == record['_controller_record']['sha256']
            and result.get('binary_sha256') == record['binary_sha256'], 'pair-terminal-record-image-relation')
    # Inner bindings are also checked by verify_retained; no external signature
    # is claimed for either independently retained integrity document.


def publish_terminal_receipt(record, output, terminal_custody):
    """Publish the late receipt once, observe inputs independently, then bind it.

    Fresh exclusive writes only. A failed postcheck/publication leaves incomplete
    evidence with the original custody/secondary cause; no retry or false exit0.
    The envelope binds the last observed snapshots; it is not an unbounded claim
    about inputs after publication, authentication or the controller's future exit.
    """
    receipt_custody = {'phase': 'controller-postcheck-publication', 'earlier_custody': [terminal_custody]}
    checked_phase(record, lambda: write_new(output / 'controller-postcheck.json', terminal_custody),
                  receipt_custody, 'after-controller-postcheck-publication')
    integrity_custody = {'phase': 'terminal-integrity-publication-and-readback',
                         'earlier_custody': [receipt_custody]}
    def integrity_work():
        """Bind/read back once; retain uncertain write and known custody on error."""
        envelope = {'schema_version': 1, 'scope': 'cd01-retained-terminal-integrity',
                    'bindings': {name: sha(output / name) for name in TERMINAL_BINDINGS},
                    'publication_postcheck': receipt_custody['post_validation_inputs']}
        publication = {'attempted': True, 'ok': None, 'visibility_on_failure': 'unknown'}
        integrity_custody['publication'] = publication
        try:
            write_new(output / 'terminal-integrity.json', envelope)
        except BaseException:
            publication['ok'] = False
            raise
        publication['ok'] = True
        verify_retained(output)
    # The final envelope cannot recursively seal its own post-publication check.
    # Observe this phase independently anyway; on error keep that check and all
    # earlier custody in failure.json/fallback, without another write attempt.
    checked_phase(record, integrity_work, integrity_custody,
                  'after-terminal-integrity-publication-and-readback')


def seal_evidence(output):
    """Hash all newly retained evidence, read it back, then publish terminal seal.

    No historical file is replaced. Any IO/hash failure leaves incomplete
    evidence, not a PASS marker; no rerun or authenticity is conferred.
    """
    files = {str(path.relative_to(output)): sha(path) for path in output.rglob('*') if path.is_file()}
    write_new(output / 'artifacts.json', files)
    for name, expected in files.items():
        require(sha(output / name) == expected, 'pair-inner-readback-changed')
    write_new(output / 'seal.json', {'status': 'PASS', 'artifacts_sha256': sha(output / 'artifacts.json'),
              'limits': 'Retained hashes only; no authentication or global acceptance.'})
    verify_retained(output, final_controller=False)


def release_volumes(held):
    """Consume held descriptor ownership once, retaining uncertain close states.

    Never retry a close that could refer to a recycled fd. Normal-path failures
    refuse before PASS publication; failure-path diagnostics remain secondary.
    """
    errors = []
    while held:
        descriptor = held.pop()
        try:
            os.close(descriptor)
        except OSError as error:
            error.former_fd = descriptor
            error.ownership_uncertain = True
            errors.append(error)
    return errors


def cleanup_details(errors):
    """Render typed cleanup causes plus former-fd/uncertainty without retrying
    an ambiguous close or losing its identity in a plain error string.
    """
    return [{'cause': outcomes.describe(error), 'errno': getattr(error, 'errno', None),
             'former_fd': getattr(error, 'former_fd', None),
             'ownership_uncertain': getattr(error, 'ownership_uncertain', None)} for error in errors]


def main():
    """Expose paired/global runs or retained-only verification. Failure records
    an incomplete/refused outcome; no relaunch, compilation or authority granted.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('paired', 'global', 'verify'))
    parser.add_argument('--evidence', type=Path)
    parser.add_argument('--record', type=Path, default=os.environ.get('SWARM_CD01_PAIR_BUILD_RECORD'))
    parser.add_argument('--output-parent', type=Path, default=os.environ.get('SWARM_CD01_PAIR_EVIDENCE_PARENT'))
    args = parser.parse_args()
    output = None
    held = []
    try:
        if args.mode == 'verify':
            require(args.evidence is not None, 'pair-retained-evidence-required')
            verify_retained(args.evidence)
            print('CD01_PAIR_RETAINED_HASHES_VERIFIED no-probe-replay')
            return 0
        require(args.record is not None and args.output_parent is not None, 'pair-explicit-record-parent-required')
        parent = args.output_parent.resolve(strict=True)
        require(parent.is_dir() and str(parent) != '/tmp' and not parent.is_relative_to('/tmp'), 'pair-evidence-parent')
        args.record = args.record.resolve(strict=True)
        record_before = sha(args.record)
        record, binary, compiler_root = admit(args.record)
        record['_controller_record'] = {'path': str(args.record), 'sha256': record_before}
        record['_invocations'] = []
        storage = record['evidence_storage']
        mounted = subprocess.run(['findmnt', '-J', '-T', str(parent), '-o', 'TARGET,UUID,FSTYPE,OPTIONS'],
                                 check=True, capture_output=True, text=True)
        mounts = json.loads(mounted.stdout)['filesystems']
        require(isinstance(mounts, list) and len(mounts) == 1 and isinstance(mounts[0], dict), 'pair-volume-readback-shape')
        volume = mounts[0]
        require(volume['target'] == storage['mountpoint'] and volume['uuid'] == storage['uuid']
                and volume['fstype'] == storage['fstype'] and 'rw' in volume['options'].split(','), 'pair-volume-identity')
        require(parent.is_relative_to(Path(storage['mountpoint'])), 'pair-parent-outside-admitted-volume')
        # Hold the actual mounted volume descriptor through all children. This
        # does not promise availability after physical removal/unmount by others.
        volume_fd = os.open(storage['mountpoint'], os.O_RDONLY | os.O_DIRECTORY)
        held.append(volume_fd)
        require(os.fstat(volume_fd).st_dev == parent.stat().st_dev, 'pair-volume-device-changed')
        output = parent / ('cd01-' + args.mode + '-' + secrets.token_hex(8)); output.mkdir(mode=0o700)
        write_new(output / 'storage-readback.json', {'observed': volume, 'device': os.fstat(volume_fd).st_dev,
                  'available_bytes': os.fstatvfs(volume_fd).f_bavail * os.fstatvfs(volume_fd).f_frsize,
                  'limit': 'Root storage/reserve allocation required separately; no quota or physical-removal guarantee.'})
        terminal_custody = {'phase': 'controller-work-and-publication', 'invocations': record['_invocations']}

        def work_and_publish():
            """Keep mode execution and all primary result/seal writes inside the
            independently post-observed phase; failures cannot bypass observation.
            """
            write_new(output / 'admitted-record-copy.json', record)
            result = pairs(record, binary, output) if args.mode == 'paired' else global_processes(record, binary, compiler_root, output)
            require(sha(args.record) == record_before, 'pair-build-record-changed')
            close_errors = release_volumes(held)
            if close_errors:
                raise joined_failure('CD01 volume close refused', close_errors, {'volume_close': cleanup_details(close_errors)})
            write_new(output / 'result.json', {'status': 'RECORDED_TARGETS_MET', 'scope': args.mode, 'observed': result,
                      'global_watchdog': None, 'record_sha256': sha(args.record), 'binary_sha256': sha(binary),
                      'limits': 'Root-admitted libtest only; adapted legacy body is not an old released public binary; no global engine acceptance.'})
            seal_evidence(output)
            return result

        checked_phase(record, work_and_publish, terminal_custody, 'after-controller-and-evidence-publication')
        # Receipt publication remains a fallible operation with a second explicit
        # observation; it is not a retry of the first receipt or of any child.
        publish_terminal_receipt(record, output, terminal_custody)
        print('CD01_PAIRED_BEHAVIOR_PASS six-pairs five-legacy-red twelve-named-probes' if args.mode == 'paired'
              else 'CD01_GLOBAL_WRAPPER_PASS control refusal fresh-process-adoption three-named-probes')
        print(json.dumps({'evidence': str(output), 'status': 'PASS'}))
        return 0
    except BaseException as error:
        cleanup = release_volumes(held)
        if cleanup:
            error = joined_failure('CD01 failure and volume cleanup refused', [error, *cleanup],
                                   {'phase': 'failure-cleanup', 'volume_close': cleanup_details(cleanup)})
        original = failure_document(error)
        if output is not None:
            try:
                write_new(output / 'failure.json', original)
            except BaseException as publication:
                combined = joined_failure('CD01 final failure publication refused', [error, publication], {'phase': 'failure-publication'})
                print(json.dumps({'original_and_secondary_failure': failure_document(combined)}), file=sys.stderr)
        else:
            print(json.dumps({'original_failure': original}), file=sys.stderr)
        print(json.dumps({'status': 'INCOMPLETE_OR_REFUSED', 'error_type': type(error).__name__, 'evidence': str(output)}))
        return 2
    finally:
        # Normally consumed in work/failure cleanup. No close retry if uncertain.
        secondary = release_volumes(held)
        if secondary:
            print(json.dumps({'secondary_volume_close': cleanup_details(secondary)}), file=sys.stderr)


if __name__ == '__main__':
    raise SystemExit(main())
