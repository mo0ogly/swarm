"""Inert regression tests for the fixture profile qualification adapter.

No compiler, network, storage mount, provider or business process is touched:
storage admission, cargo execution and space checks are injected as stubs at
the qualify() callable seams only. The CLI exposes no injection or execution
authority; a dedicated test asserts that. These tests validate the adapter
boundary (files, receipts, refusals), not a mirror of its internals.
"""
import hashlib
import json
import sys
import types
from pathlib import Path

import pytest
from fixture_area import fixture_root, fixture_area
import errno
import os
import stat

@pytest.fixture
def tmp_path():
    """Allocate retained synthetic files under the configured project area."""
    return fixture_root("fixture-profile-inert-")

TOOLS = Path(__file__).resolve().parents[1] / 'tools' / 'verification'
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

import native_build_outcome  # noqa: E402
import qualify_fixture_profile as adapter  # noqa: E402

PACKAGE = 'swarm-engine'
TEST_NAME = 'paea_git_promote_call'
FEATURES = ['test-fixtures']


@pytest.fixture(autouse=True)
def clean_build_environment(monkeypatch):
    """Remove refused environment keys so tests never depend on the shell."""
    for key in adapter.ENV_REFUSALS:
        monkeypatch.delenv(key, raising=False)


def sha_file(path):
    """Hash current file bytes."""
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def canon(path):
    """Canonical absolute path string, as the adapter config requires."""
    return str(Path(path).resolve())


def write(path, data):
    """Create parents and write bytes or text."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data if isinstance(data, bytes) else data.encode())
    return path


class StubAdmission:
    """Inert build_storage.admitted stand-in with optional check failure."""

    def __init__(self, fail_at=None):
        self.fail_at = fail_at
        self.checks = 0

    def check(self):
        self.checks += 1
        if self.fail_at is not None and self.checks >= self.fail_at:
            raise ValueError('injected-storage-drift')
        return {'stub': True, 'checks': self.checks, 'mode': 'registered-external',
                'mount': {'target': str(fixture_area())},
                'anchor_device': fixture_area().stat().st_dev}

    def __enter__(self):
        return self

    def __exit__(self, *args):
        return False


def stub_space(paths, minimum):
    """Inert space observation exactly at the persisted minimum."""
    return [{'path': str(path), 'available_bytes': minimum} for path in paths]


def build_world(tmp_path):
    """Materialize a minimal inert world: frozen tree, cache, maps, config."""
    root = tmp_path / 'root'
    frozen = tmp_path / 'frozen'
    target = tmp_path / 'target'
    output = tmp_path / 'output'
    target.mkdir(parents=True)
    registry_file = write(
        tmp_path / 'registry/src/index.crates.io-x/log-0.4.20/src/lib.rs',
        'REGISTRY\n')
    demo = write(frozen / 'rust/crates/demo/src/lib.rs', 'pub fn demo() {}\n')
    toml = write(frozen / 'rust/Cargo.toml', '[workspace]\n')
    lock = write(frozen / 'rust/Cargo.lock', 'lock\n')
    external_policy = write(root / '.cargo/external-targets.json',
                            '{"schema_version":1,"targets":[]}\n')
    space_policy = write(root / '.cargo/storage-policy.json',
                         '{"schema_version":1,"minimum_free_bytes":1}\n')
    source_map = {'rust/Cargo.toml': sha_file(toml),
                  'rust/Cargo.lock': sha_file(lock),
                  'rust/crates/demo/src/lib.rs': sha_file(demo)}
    source_map_path = write(tmp_path / 'evidence/source-map.json',
                            json.dumps(source_map, indent=2, sort_keys=True) + '\n')
    registry_map = {canon(registry_file): sha_file(registry_file)}
    registry_map_path = write(tmp_path / 'evidence/registry-map.json',
                              json.dumps(registry_map, indent=2,
                                         sort_keys=True) + '\n')
    config = {'schema_version': 1, 'root': canon(root), 'frozen': canon(frozen),
              'workspace_root': canon(frozen / 'rust'), 'target': canon(target),
              'output': canon(output), 'manifest_path': 'rust/Cargo.toml',
              'source_map': {'path': canon(source_map_path),
                             'sha256': sha_file(source_map_path)},
              'registry_map': {'path': canon(registry_map_path),
                               'sha256': sha_file(registry_map_path)},
              'cargo_lock': {'path': canon(lock), 'sha256': sha_file(lock)},
              'external_targets_policy': {'path': canon(external_policy),
                                          'sha256': sha_file(external_policy)},
              'space_policy': {'path': canon(space_policy),
                               'sha256': sha_file(space_policy)},
              'package': PACKAGE, 'test': TEST_NAME, 'features': list(FEATURES)}
    config_path = write(tmp_path / 'evidence/config.json',
                        json.dumps(config, indent=2) + '\n')
    return types.SimpleNamespace(root=root, frozen=frozen, target=target,
                                 output=output, registry_file=registry_file,
                                 demo=demo, config=config, config_path=config_path,
                                 config_sha256=sha_file(config_path))


def fake_execute(lines, *, returncode=0, before=None, fail=None):
    """Build an inert replacement mirroring native_build_outcome.execute's
    observable contract: admission check, channel files, both receipts, then a
    completed result or a failure carrying native_outcome."""
    def _execute(argv, *, out, name, admission, publish, digest, failure_code,
                 **kwargs):
        if before is not None:
            before()
        admission.check()
        (out / (name + '.stdout')).write_bytes(
            ''.join(json.dumps(line) + '\n' for line in lines).encode())
        (out / (name + '.stderr')).write_bytes(b'')
        outcome = {'returncode': returncode, 'execution_attempted': True,
                   'issues': [], 'stdout_sha256': digest(out / (name + '.stdout')),
                   'stderr_sha256': digest(out / (name + '.stderr'))}
        publish(out / (name + '-result.json'), json.dumps(outcome) + '\n')
        publish(out / (name + '-admission-after.json'), json.dumps({
            'subprocess': outcome, 'admission_after': {'checked': True, 'ok': True},
            'receipt_publication': {'ok': True}}) + '\n')
        if fail is not None:
            raise fail
        return types.SimpleNamespace(returncode=returncode)
    return _execute


def artifact_record(elf_path, *, features=None, name=TEST_NAME, profile=None):
    """One inert cargo compiler-artifact record for the selected test."""
    return {'reason': 'compiler-artifact', 'package_id': PACKAGE + ' 0.1.0',
            'target': {'name': name, 'kind': ['test']},
            'profile': {'test': True} if profile is None else profile,
            'features': list(FEATURES) if features is None else features,
            'executable': str(elf_path), 'fresh': False}


def make_cache(world, dep_entries):
    """Create the fake fixture ELF, one generated file and one depfile whose
    relative/absolute entries follow the adapter's classification contract."""
    elf = write(world.target / ('debug/deps/' + TEST_NAME + '-abc123'),
                'FAKE ELF\n')
    elf.chmod(0o700)
    generated = write(world.target / 'debug/build/demo-xyz/output', 'generated\n')
    entries = [entry.format(generated=canon(generated),
                            registry=canon(world.registry_file))
               for entry in dep_entries]
    line = 'debug/deps/' + TEST_NAME + '-abc123.o: ' + ' '.join(entries)
    write(world.target / ('debug/deps/' + TEST_NAME + '-abc123.d'), line + '\n')
    return elf, generated


def run(world, lines, *, admission=None, execute=None):
    """Qualify with inert stubs and default handling."""
    return adapter.qualify(
        world.config_path, world.config_sha256,
        admission=admission or (lambda root, target: StubAdmission()),
        execute=execute or fake_execute(lines), space=stub_space)


def control_records(elf):
    """Default success records: one artifact plus one build-finished."""
    return [artifact_record(elf), {'reason': 'build-finished', 'success': True}]


def test_valid_control_qualifies_with_distinct_linkage(tmp_path):
    world = build_world(tmp_path)
    elf, generated = make_cache(world, ['crates/demo/src/lib.rs',
                                        '{registry}', '{generated}'])
    stub = StubAdmission()
    summary = run(world, control_records(elf),
                  admission=lambda root, target: stub)
    out = world.output
    assert summary['status'] == 'FIXTURE_PROFILE_QUALIFIED'
    assert stub.checks == 3
    retained = json.loads((out / 'retained-elf.json').read_text())
    copied = Path(retained['path'])
    assert copied.is_file() and copied != elf
    assert copied.is_relative_to(canon(out))
    assert retained['sha256'] == sha_file(copied) == sha_file(elf)
    assert retained['features'] == FEATURES
    assert retained['profile']['test'] is True
    assert retained['body_executed'] is False
    assert retained['target']['name'] == TEST_NAME
    assert retained['source_inputs_sha256'] == world.config['source_map']['sha256']
    assert retained['source_manifest']['sha256'] == \
        world.config['source_map']['sha256']
    assert retained['registry_manifest']['sha256'] == \
        world.config['registry_map']['sha256']
    assert json.loads((out / 'depfile-missing.json').read_text()) == []
    assert json.loads((out / 'depfile-unmatched.json').read_text()) == {}
    proof = json.loads((out / 'depfile-proof.json').read_text())
    assert proof['workspace_all_declared'] is True
    result = json.loads((out / 'result.json').read_text())
    assert result['exit'] == 0 and result['source_stable']
    assert result['registry_stable']
    assert result['build_finished'] == [{'reason': 'build-finished',
                                         'success': True}]
    actual = json.loads((out / 'actual-source-dependencies.json').read_text())
    assert actual[canon(world.demo)] == sha_file(world.demo)
    assert actual[canon(world.registry_file)] == sha_file(world.registry_file)
    classes = json.loads((out / 'dependency-classes.json').read_text())
    assert canon(generated) in classes['generated']
    raw = out / ('raw-depfiles/' + TEST_NAME + '-abc123.d')
    assert raw.read_bytes() == (world.target / (
        'debug/deps/' + TEST_NAME + '-abc123.d')).read_bytes()
    assert (out / 'raw-generated/debug/build/demo-xyz/output').read_bytes() \
        == b'generated\n'
    assert (out / 'source-inputs.json').read_bytes() == \
        Path(world.config['source_map']['path']).read_bytes()
    freeze = json.loads((out / 'freeze.json').read_text())
    assert freeze['frozen'] == canon(world.frozen)
    assert freeze['fixture_target'] == canon(world.target)
    assert freeze['source_inputs_sha256'] == world.config['source_map']['sha256']
    invocation = json.loads((out / 'invocation.json').read_text())
    for flag in ('--offline', '--locked', '--no-run', '--message-format=json'):
        assert flag in invocation['argv']
    assert invocation['admission_tools']['build_storage']['sha256'] == \
        sha_file(TOOLS / 'build_storage.py')


def test_wrong_relative_base_refuses_without_fallback(tmp_path):
    world = build_world(tmp_path)
    write(world.frozen / 'crates/misplaced/src/lib.rs', 'misplaced\n')
    elf, _ = make_cache(world, ['crates/misplaced/src/lib.rs'])
    with pytest.raises(ValueError, match='depfile-audit'):
        run(world, control_records(elf))
    missing = json.loads((world.output / 'depfile-missing.json').read_text())
    assert missing and 'rust/crates/misplaced' in missing[0]['resolved']
    assert (world.frozen / 'crates/misplaced/src/lib.rs').is_file()
    assert not (world.output / 'retained-elf.json').exists()


def test_unbound_workspace_dependency_refuses(tmp_path):
    world = build_world(tmp_path)
    other = write(world.frozen / 'rust/crates/other/src/lib.rs', 'other\n')
    elf, _ = make_cache(world, ['crates/other/src/lib.rs'])
    with pytest.raises(ValueError, match='depfile-audit'):
        run(world, control_records(elf))
    unmatched = json.loads((world.output / 'depfile-unmatched.json').read_text())
    assert unmatched[canon(other)] == 'workspace-source-unbound'
    assert not (world.output / 'retained-elf.json').exists()


def test_unbound_external_dependency_refuses(tmp_path):
    world = build_world(tmp_path)
    stranger = write(tmp_path / 'unbound/other.rs', 'stranger\n')
    elf, _ = make_cache(world, [str(canon(stranger))])
    with pytest.raises(ValueError, match='depfile-audit'):
        run(world, control_records(elf))
    unmatched = json.loads((world.output / 'depfile-unmatched.json').read_text())
    assert unmatched[canon(stranger)] == 'unclassified-external'


def test_source_drift_refuses_before_child(tmp_path):
    world = build_world(tmp_path)
    world.demo.write_bytes(b'drifted\n')
    started = []

    def must_not_run(argv, **kwargs):
        started.append(argv)
        raise AssertionError('child must not start')

    with pytest.raises(ValueError, match='source-input-hash'):
        adapter.qualify(world.config_path, world.config_sha256,
                        admission=lambda root, target: StubAdmission(),
                        execute=must_not_run, space=stub_space)
    assert not started
    assert not world.output.exists()


def test_registry_drift_refuses_before_child(tmp_path):
    world = build_world(tmp_path)
    world.registry_file.write_bytes(b'changed\n')
    started = []

    def must_not_run(argv, **kwargs):
        started.append(argv)
        raise AssertionError('child must not start')

    with pytest.raises(ValueError, match='registry-input-hash'):
        adapter.qualify(world.config_path, world.config_sha256,
                        admission=lambda root, target: StubAdmission(),
                        execute=must_not_run, space=stub_space)
    assert not started


def test_compile_failure_with_postcheck_and_storage_drift_retains_all_causes(
        tmp_path):
    world = build_world(tmp_path)

    def corrupt():
        world.demo.write_bytes(b'drifted-during-run\n')

    failure = ValueError('injected-compile-failure')
    failure.native_outcome = {'subprocess': {'returncode': 1}}
    stub = StubAdmission(fail_at=3)
    with pytest.raises(BaseExceptionGroup) as caught:
        adapter.qualify(world.config_path, world.config_sha256,
                        admission=lambda root, target: stub,
                        execute=fake_execute([], before=corrupt, fail=failure,
                                             returncode=1),
                        space=stub_space)
    causes = caught.value.exceptions
    assert len(causes) == 3
    assert any('injected-compile-failure' in str(item) for item in causes)
    assert any('source-drift' in str(item) for item in causes)
    assert any('injected-storage-drift' in str(item) for item in causes)
    document = native_build_outcome.failure_document(caught.value)
    assert document['native_outcome'] == {'subprocess': {'returncode': 1}}
    result = json.loads((world.output / 'result.json').read_text())
    assert result['exit'] == 1 and result['source_stable'] is False
    assert stub.checks == 3


def test_wrong_features_refuse_without_retained_elf(tmp_path):
    world = build_world(tmp_path)
    elf, _ = make_cache(world, ['crates/demo/src/lib.rs', '{registry}',
                                '{generated}'])
    records = [artifact_record(elf, features=[]),
               {'reason': 'build-finished', 'success': True}]
    with pytest.raises(ValueError, match='features-mismatch'):
        run(world, records)
    assert not (world.output / 'retained-elf.json').exists()
    assert json.loads((world.output / 'artifacts.json').read_text())


def test_non_test_profile_refuses(tmp_path):
    world = build_world(tmp_path)
    elf, _ = make_cache(world, ['crates/demo/src/lib.rs', '{registry}',
                                '{generated}'])
    records = [artifact_record(elf, profile={'test': False}),
               {'reason': 'build-finished', 'success': True}]
    with pytest.raises(ValueError, match='profile-not-test'):
        run(world, records)
    assert not (world.output / 'retained-elf.json').exists()


def test_no_matching_executable_refuses(tmp_path):
    world = build_world(tmp_path)
    elf, _ = make_cache(world, ['crates/demo/src/lib.rs', '{registry}',
                                '{generated}'])
    records = [artifact_record(elf, name='some_other_test'),
               {'reason': 'build-finished', 'success': True}]
    with pytest.raises(ValueError, match='artifact-not-unique'):
        run(world, records)
    assert not (world.output / 'retained-elf.json').exists()
    assert json.loads((world.output / 'artifacts.json').read_text())


def test_config_hash_mismatch_refuses_before_any_output(tmp_path):
    world = build_world(tmp_path)
    with pytest.raises(ValueError, match='config-hash-mismatch'):
        adapter.qualify(world.config_path, '0' * 64)
    assert not world.output.exists()


def test_duplicate_config_key_refuses(tmp_path):
    world = build_world(tmp_path)
    raw = b'{"schema_version": 1, "schema_version": 1}'
    path = write(tmp_path / 'evidence/duplicate.json', raw)
    with pytest.raises(ValueError, match='duplicate'):
        adapter.qualify(path, sha_file(path))
    assert not world.output.exists()


def test_cli_refuses_config_hash_mismatch_without_execution(tmp_path, monkeypatch,
                                                            capsys):
    world = build_world(tmp_path)
    monkeypatch.setattr(sys, 'argv',
                        ['qualify_fixture_profile.py', '--config',
                         str(world.config_path), '--config-sha256', '0' * 64])
    assert adapter.main() == 1
    assert 'fixture-config-hash-mismatch' in capsys.readouterr().err
    assert not world.output.exists()


def test_cli_exposes_no_execution_or_injection_flags(monkeypatch):
    monkeypatch.setattr(sys, 'argv',
                        ['qualify_fixture_profile.py', '--config',
                         '/nonexistent.json', '--config-sha256', '0' * 64,
                         '--execute'])
    with pytest.raises(SystemExit) as caught:
        adapter.main()
    assert caught.value.code == 2


def healthy_before_fault(tmp_path):
    """Establish the same successful inert prerequisites before fault injection."""
    w = build_world(tmp_path / 'healthy')
    elf, _ = make_cache(w, ['crates/demo/src/lib.rs', '{registry}', '{generated}'])
    assert run(w, control_records(elf))['status'] == 'FIXTURE_PROFILE_QUALIFIED'


def test_retained_elf_keeps_owner_execute_permission(tmp_path):
    healthy_before_fault(tmp_path)
    w = build_world(tmp_path / 'fault')
    elf, _ = make_cache(w, ['crates/demo/src/lib.rs'])
    summary = run(w, control_records(elf))
    retained = Path(summary['retained_elf']['path'])
    assert retained.stat().st_mode & stat.S_IXUSR


def test_invocation_publication_failure_prevents_child(tmp_path, monkeypatch):
    healthy_before_fault(tmp_path)
    w = build_world(tmp_path / 'fault')
    elf, _ = make_cache(w, ['crates/demo/src/lib.rs'])
    original = adapter.publish
    started = []
    fake = fake_execute(control_records(elf))
    def refuse_invocation(path, text):
        if Path(path).name == 'invocation.json':
            raise OSError(errno.ENOSPC, 'injected-invocation-publication')
        return original(path, text)
    def execute(*args, **kwargs):
        started.append(True)
        return fake(*args, **kwargs)
    monkeypatch.setattr(adapter, 'publish', refuse_invocation)
    with pytest.raises(BaseException) as caught:
        run(w, control_records(elf), execute=execute)
    assert 'injected-invocation-publication' in json.dumps(native_build_outcome.failure_document(caught.value))
    assert not started


def test_shared_generated_input_is_retained_once(tmp_path):
    healthy_before_fault(tmp_path)
    w = build_world(tmp_path / 'fault')
    elf, _ = make_cache(w, ['crates/demo/src/lib.rs', '{generated}'])
    first = w.target / ('debug/deps/' + TEST_NAME + '-abc123.d')
    write(w.target / 'debug/deps/second.d', first.read_bytes())
    summary = run(w, control_records(elf))
    assert summary['depfile_proof']['counts']['generated'] == 1


def test_unavailable_admission_creates_no_output(tmp_path):
    healthy_before_fault(tmp_path)
    w = build_world(tmp_path / 'fault')
    elf, _ = make_cache(w, ['crates/demo/src/lib.rs'])
    class Refused(StubAdmission):
        def __enter__(self):
            raise OSError(errno.ENODEV, 'injected-volume-unavailable')
    with pytest.raises(OSError, match='volume-unavailable'):
        run(w, control_records(elf), admission=lambda root, target: Refused())
    assert not w.output.exists()


def test_failed_file_sync_leaves_no_partial_published_receipt(tmp_path, monkeypatch):
    control = tmp_path / 'control.json'
    adapter.publish(control, 'complete')
    assert control.read_text() == 'complete'
    fault = tmp_path / 'fault.json'
    def refuse_sync(_):
        raise OSError(errno.ENOSPC, 'injected-file-sync')
    monkeypatch.setattr(adapter.os, 'fsync', refuse_sync)
    with pytest.raises(OSError, match='file-sync'):
        adapter.publish(fault, 'must-not-be-visible')
    assert not fault.exists()


def test_directory_sync_failure_preserves_applied_publication(tmp_path, monkeypatch):
    control = tmp_path / 'control.json'
    adapter.publish(control, 'complete')
    assert control.read_text() == 'complete'
    fault = tmp_path / 'fault.json'
    real = adapter.os.fsync
    def refuse_directory(fd):
        if stat.S_ISDIR(os.fstat(fd).st_mode):
            raise OSError(errno.EIO, 'injected-directory-sync')
        return real(fd)
    monkeypatch.setattr(adapter.os, 'fsync', refuse_directory)
    with pytest.raises(OSError, match='directory-sync'):
        adapter.publish(fault, 'published-but-sync-uncertain')
    assert fault.read_text() == 'published-but-sync-uncertain'


@pytest.mark.parametrize('category', ['source', 'registry', 'lock'])
def test_input_io_cause_survives_before_child(tmp_path, monkeypatch, category):
    healthy_before_fault(tmp_path)
    w = build_world(tmp_path / 'fault')
    denied = {'source': w.demo, 'registry': w.registry_file,
              'lock': Path(w.config['cargo_lock']['path'])}[category]
    real = adapter.digest
    def refuse_one(path):
        if Path(path) == denied:
            raise PermissionError(errno.EACCES, 'injected-input-permission', str(path))
        return real(path)
    monkeypatch.setattr(adapter, 'digest', refuse_one)
    with pytest.raises(PermissionError, match='input-permission'):
        run(w, [])
    assert not w.output.exists()


def test_lock_binding_is_the_declared_workspace_lock(tmp_path):
    healthy_before_fault(tmp_path)
    w = build_world(tmp_path / 'fault')
    elf, _ = make_cache(w, ['crates/demo/src/lib.rs'])
    other = write(w.frozen / 'rust/unrelated-lock', 'other-lock')
    w.config['cargo_lock'] = {'path': canon(other), 'sha256': sha_file(other)}
    w.config_path.write_text(json.dumps(w.config))
    w.config_sha256 = sha_file(w.config_path)
    with pytest.raises(ValueError, match='lock-placement'):
        run(w, control_records(elf))
    assert not w.output.exists()


def test_failed_admission_receipt_cannot_qualify(tmp_path):
    healthy_before_fault(tmp_path)
    w = build_world(tmp_path / 'fault')
    elf, _ = make_cache(w, ['crates/demo/src/lib.rs'])
    base = fake_execute(control_records(elf))
    def failed_receipt(*args, **kwargs):
        result = base(*args, **kwargs)
        path = kwargs['out'] / (kwargs['name'] + '-admission-after.json')
        data = json.loads(path.read_text())
        data['admission_after']['ok'] = False
        path.write_text(json.dumps(data))
        return result
    with pytest.raises(ValueError, match='native-receipt'):
        run(w, control_records(elf), execute=failed_receipt)
    assert not (w.output / 'retained-elf.json').exists()


def test_wrong_target_kind_cannot_qualify(tmp_path):
    healthy_before_fault(tmp_path)
    w = build_world(tmp_path / 'fault')
    elf, _ = make_cache(w, ['crates/demo/src/lib.rs'])
    artifact = artifact_record(elf)
    artifact['target']['kind'] = ['bin']
    with pytest.raises(ValueError, match='artifact-kind'):
        run(w, [artifact, {'reason': 'build-finished', 'success': True}])
    assert not (w.output / 'retained-elf.json').exists()


@pytest.mark.parametrize('raw', [
    b'{"reason":"other","reason":"compiler-artifact"}\n',
    b'{"reason":"other","invalid":NaN}\n'])
def test_cargo_messages_use_strict_json(raw):
    assert adapter.parse_messages(b'{"reason":"build-finished","success":true}\n')[1]
    with pytest.raises(ValueError):
        adapter.parse_messages(raw)


def test_bound_input_maps_preserve_exact_crlf_bytes(tmp_path):
    healthy_before_fault(tmp_path)
    w = build_world(tmp_path / 'fault')
    elf, _ = make_cache(w, ['crates/demo/src/lib.rs', '{registry}'])
    for field in ['source_map', 'registry_map']:
        path = Path(w.config[field]['path'])
        path.write_bytes(path.read_bytes().replace(b'\n', b'\r\n'))
        w.config[field]['sha256'] = sha_file(path)
    w.config_path.write_text(json.dumps(w.config))
    w.config_sha256 = sha_file(w.config_path)
    summary = run(w, control_records(elf))
    for field, output in [('source_map', 'source-inputs.json'),
                          ('registry_map', 'registry-inputs.json')]:
        assert (w.output / output).read_bytes() == Path(w.config[field]['path']).read_bytes()
        assert sha_file(w.output / output) == w.config[field]['sha256']
    assert summary['source_inputs_sha256'] == sha_file(w.output / 'source-inputs.json')


@pytest.mark.parametrize('linked', [False, True])
def test_publication_observation_survives_error_renderer(tmp_path, monkeypatch, linked):
    control = tmp_path / 'healthy'
    adapter.publish(control, 'healthy')
    assert control.read_text() == 'healthy'
    destination = tmp_path / 'fault'
    real = adapter.os.fsync
    def refuse(fd):
        if not linked or stat.S_ISDIR(os.fstat(fd).st_mode):
            raise OSError(errno.EIO, 'injected-publication-sync')
        return real(fd)
    monkeypatch.setattr(adapter.os, 'fsync', refuse)
    with pytest.raises(OSError) as caught:
        adapter.publish(destination, 'payload')
    renderer = getattr(adapter, 'failure_document', native_build_outcome.failure_document)
    document = renderer(caught.value)
    observations = document.get('publication_observations', [])
    assert len(observations) == 1
    assert observations[0]['link_succeeded'] is linked
    assert observations[0]['sha256'] == hashlib.sha256(b'payload').hexdigest()
    assert destination.exists() is linked


def test_recorded_child_exit_survives_helper_exception(tmp_path):
    healthy_before_fault(tmp_path)
    w = build_world(tmp_path / 'fault')
    failure = ValueError('injected-known-child-failure')
    failure.native_outcome = {'subprocess': {'returncode': 1}}
    with pytest.raises(ValueError, match='known-child-failure'):
        adapter.qualify(w.config_path, w.config_sha256,
                        admission=lambda root, target: StubAdmission(),
                        execute=fake_execute([], returncode=1, fail=failure),
                        space=stub_space)
    result = json.loads((w.output / 'result.json').read_text())
    assert result['exit'] == 1


@pytest.mark.parametrize('kind', ['depfile', 'generated'])
def test_raw_archive_uses_observed_buffer_when_source_changes(tmp_path, monkeypatch, kind):
    """Archive exactly the bytes hashed/analyzed, despite later source changes."""
    healthy_before_fault(tmp_path)
    world = build_world(tmp_path / 'fault')
    elf, generated = make_cache(world, ['crates/demo/src/lib.rs', '{registry}', '{generated}'])
    depfile = world.target / ('debug/deps/' + TEST_NAME + '-abc123.d')
    selected = depfile if kind == 'depfile' else generated
    original = selected.read_bytes()
    actual_read = Path.read_bytes
    reads = []

    def mutate_after_observation(path):
        data = actual_read(path)
        if path == selected:
            reads.append(data)
            if len(reads) == 1:
                path.write_bytes(b'changed-after-observation\n')
        return data

    monkeypatch.setattr(Path, 'read_bytes', mutate_after_observation)
    run(world, control_records(elf))
    archived = (world.output / 'raw-depfiles' / depfile.name if kind == 'depfile'
                else world.output / 'raw-generated' / generated.relative_to(world.target))
    assert archived.read_bytes() == original
    proof = json.loads((world.output / ('depfile-proof.json' if kind == 'depfile'
                                        else 'actual-source-dependencies.json')).read_text())
    hashes = proof['depfiles'] if kind == 'depfile' else proof
    assert hashes[str(selected)] == hashlib.sha256(original).hexdigest()


def test_bound_parent_component_resolves_from_cargo_workspace(tmp_path):
    """A parent component inside a hash-bound source is a valid Cargo depfile."""
    healthy_before_fault(tmp_path)
    world = build_world(tmp_path / 'case')
    elf, _ = make_cache(world, ['crates/demo/src/../src/lib.rs'])
    result = run(world, control_records(elf))
    assert result['status'] == 'FIXTURE_PROFILE_QUALIFIED'
    proof = json.loads((world.output / 'actual-source-dependencies.json').read_text())
    assert proof[str(world.demo)] == sha_file(world.demo)


@pytest.mark.parametrize('symlink', [False, True])
def test_unbound_parent_component_or_symlink_escape_refuses(tmp_path, symlink):
    """Canonical resolution still refuses unbound external paths and links."""
    healthy_before_fault(tmp_path)
    world = build_world(tmp_path / 'case')
    outside = write(tmp_path / 'case/outside.rs', 'OUTSIDE\n')
    if symlink:
        link = world.frozen / 'rust/crates/linked.rs'
        link.symlink_to(outside)
        entry = 'crates/linked.rs'
    else:
        entry = '../../outside.rs'
    elf, _ = make_cache(world, [entry])
    with pytest.raises(ValueError, match='depfile-audit'):
        run(world, control_records(elf))
    assert not (world.output / 'retained-elf.json').exists()
