#!/usr/bin/env python3
"""Qualify one fixture integration test ELF by compile-only Cargo execution.

Permanent, reusable adapter replacing unsafe copies of historical one-off
producers. One explicit, hash-bound JSON recipe carries every input; there are
no hidden operator defaults. The adapter re-verifies each prebound input before
spawn and again after the child (even on failure), sequentially reuses the
already registered external Cargo target through build_storage.admitted, runs
cargo test --offline --locked --no-run through native_build_outcome.execute,
and writes fresh evidence feeding the supplied PAEA preflight outputs contract
(freeze/result/retained-elf/depfile documents plus raw depfile and generated
copies).

It never registers or mutates storage policy, never deletes, cleans or unlocks
cache content, never executes a test body, does not schedule a managed default CLI-image build,
and adds no margin, cap, timeout or retry; Cargo offline mode is not a network sandbox. Qualification scope
is the observed Rust source/feature/test profile only; it is not a C/header/
build-script/proc-macro/system trust-graph, runtime, installation or business
acceptance proof.
"""
import argparse
import hashlib
import json
import os
import re
import shlex
import sys
import tempfile
from pathlib import Path, PurePosixPath

if sys.version_info < (3, 11):
    raise RuntimeError('Fixture profile qualification requires Python 3.11 or later')

import build_storage
import cargo_space_guard
import native_build_outcome

EXECUTION_NAME = 'fixture-compile'
ENV_REFUSALS = ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER',
                'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_RUSTFLAGS',
                'CARGO_BUILD_TARGET', 'RUSTUP_TOOLCHAIN', 'CARGO_HOME',
                'CARGO_TARGET_DIR')
CONFIG_KEYS = frozenset({
    'schema_version', 'root', 'frozen', 'workspace_root', 'target', 'output',
    'manifest_path', 'source_map', 'registry_map', 'cargo_lock',
    'external_targets_policy', 'space_policy', 'package', 'test', 'features'})
BINDING_KEYS = ('source_map', 'registry_map', 'cargo_lock',
                'external_targets_policy', 'space_policy')
SCOPE = ('Fixture integration ELF compile only; no test body execution, '
         'managed default CLI-image build, installation or runtime acceptance')
SHA256 = re.compile(r'[0-9a-f]{64}\Z')
NAME_CHARS = re.compile(r'[A-Za-z0-9][A-Za-z0-9_-]*\Z')
FEATURE_CHARS = re.compile(r'[A-Za-z0-9][A-Za-z0-9_./+-]*\Z')


def require(value, reason):
    """Refuse explicitly, including under Python -O; no successful fallback."""
    if not value:
        raise ValueError(reason)


def digest(path):
    """Hash full current file bytes; missing or unreadable inputs propagate."""
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def _loads(raw):
    """Parse strict JSON bytes: duplicate keys and nonfinite constants refuse."""
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'fixture-json-duplicate-key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'fixture-json-nonfinite'))


def read_json(path):
    """Read one strict JSON file; duplicate keys and nonfinite constants refuse."""
    return _loads(Path(path).read_bytes())


def publish(path, text):
    """Publish complete bytes atomically without replacing earlier evidence.

    A private adjacent temporary is fsynced/read back, then hard-linked to the
    absent destination. Pre-link failure leaves no published file. A failure
    after linking is applied/uncertain, never rollback. Own temporary cleanup
    and directory-sync/close errors are retained alongside the original cause.
    """
    path = Path(path)
    data = text.encode() if isinstance(text, str) else text
    require(isinstance(data, bytes), 'fixture-publication-bytes-required')
    descriptor, name = tempfile.mkstemp(dir=path.parent, prefix='.' + path.name + '.')
    errors = []
    linked = False
    try:
        with os.fdopen(descriptor, 'wb') as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        require(Path(name).read_bytes() == data, 'fixture-publication-readback')
        os.link(name, path)
        linked = True
    except BaseException as error:
        errors.append(error)
    try:
        os.unlink(name)
    except BaseException as error:
        errors.append(error)
    if linked:
        directory = None
        try:
            directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
            os.fsync(directory)
        except BaseException as error:
            errors.append(error)
        finally:
            if directory is not None:
                try:
                    os.close(directory)
                except BaseException as error:
                    errors.append(error)
    if errors:
        failure = errors[0] if len(errors) == 1 else BaseExceptionGroup(
            'Fixture publication/cleanup failed', errors)
        failure.fixture_publication = {
            'path': str(path), 'sha256': hashlib.sha256(data).hexdigest(),
            'link_succeeded': linked,
            'scope': 'Observed publication link, not current visibility or rollback',
            'durability': 'Unqualified because publication or cleanup failed'}
        raise failure


def publish_observed(data, destination):
    """Publish and read back one captured byte buffer without rereading its source.

    Create destination parents, then use exclusive atomic publication. IO and
    publication failures propagate. This binds the archive to the observation,
    not to the source's later state or an immutable source lifetime.
    """
    destination = Path(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    publish(destination, data)
    require(digest(destination) == hashlib.sha256(data).hexdigest(),
            'fixture-copy-readback-mismatch')


def copy_exclusive(source, destination):
    """Read once, atomically publish and verify copied bytes; never overwrite."""
    publish_observed(Path(source).read_bytes(), destination)


def _binding(path):
    """Return an evidence binding for one tool file: resolved path plus sha256."""
    return {'path': str(Path(path).resolve()), 'sha256': digest(path)}


def _binding_observed(binding):
    """Read a bound file's digest; preserve original IO causes on failure."""
    return digest(binding['path'])


def load_config(path, expected_sha256):
    """Load and fully validate the explicit caller recipe; nothing is defaulted.

    The exact recipe bytes must match expected_sha256. Unknown fields, duplicate
    keys, nonfinite numbers and malformed paths refuse before anything runs.
    """
    require(isinstance(expected_sha256, str) and SHA256.fullmatch(expected_sha256),
            'fixture-config-sha-format')
    raw = Path(path).read_bytes()
    require(hashlib.sha256(raw).hexdigest() == expected_sha256,
            'fixture-config-hash-mismatch')
    config = _loads(raw)
    require(isinstance(config, dict) and set(config) == CONFIG_KEYS,
            'fixture-config-schema')
    require(type(config['schema_version']) is int and config['schema_version'] == 1,
            'fixture-config-version')
    for key in ('root', 'frozen', 'workspace_root', 'target', 'output'):
        require(isinstance(config[key], str), 'fixture-config-path-type')
        config[key] = str(build_storage.canonical(config[key]))
    for key in BINDING_KEYS:
        binding = config[key]
        require(isinstance(binding, dict) and set(binding) == {'path', 'sha256'}
                and isinstance(binding['path'], str)
                and isinstance(binding['sha256'], str),
                f'fixture-config-binding:{key}')
        require(SHA256.fullmatch(binding['sha256']) is not None,
                f'fixture-config-sha-format:{key}')
        binding['path'] = str(build_storage.canonical(binding['path']))
    manifest = config['manifest_path']
    require(isinstance(manifest, str) and manifest and '\\' not in manifest
            and not manifest.startswith('/'), 'fixture-config-manifest-path')
    parts = PurePosixPath(manifest).parts
    require('..' not in parts and PurePosixPath(manifest).name == 'Cargo.toml',
            'fixture-config-manifest-path')
    require(isinstance(config['package'], str)
            and NAME_CHARS.fullmatch(config['package']) is not None,
            'fixture-config-package')
    require(isinstance(config['test'], str)
            and NAME_CHARS.fullmatch(config['test']) is not None,
            'fixture-config-test')
    features = config['features']
    require(isinstance(features, list) and features
            and len(set(features)) == len(features)
            and all(isinstance(item, str)
                    and FEATURE_CHARS.fullmatch(item) is not None
                    for item in features), 'fixture-config-features')
    return config


def validate_paths(config):
    """Check the filesystem placement of every declared path before verification.

    Refuses missing directories, symlinked frozen/workspace/target locations, a
    manifest outside the declared workspace, a lock outside the frozen tree,
    target/output overlapping the frozen tree or each other, a non-fresh or
    badly placed output (evidence must live outside root/frozen/target), and
    policy bindings that do not point at the persistent policies inside root.
    """
    root = Path(config['root'])
    frozen = Path(config['frozen'])
    workspace = Path(config['workspace_root'])
    target = Path(config['target'])
    output = Path(config['output'])
    require(root.is_dir(), 'fixture-root-missing')
    require(frozen.is_dir() and Path(os.path.realpath(frozen)) == frozen,
            'fixture-frozen-placement')
    require(workspace.is_dir() and workspace.is_relative_to(frozen)
            and Path(os.path.realpath(workspace)) == workspace,
            'fixture-workspace-placement')
    manifest = frozen / config['manifest_path']
    require(manifest.is_file() and manifest.is_relative_to(workspace),
            'fixture-manifest-placement')
    require(target.is_dir() and target != root and target != frozen
            and not target.is_relative_to(frozen)
            and not frozen.is_relative_to(target)
            and Path(os.path.realpath(target)) == target,
            'fixture-target-placement')
    require(output.parent.is_dir(), 'fixture-output-parent-missing')
    require(not output.exists()
            and not output.is_relative_to(root) and not root.is_relative_to(output)
            and not output.is_relative_to(frozen)
            and not frozen.is_relative_to(output)
            and not output.is_relative_to(target)
            and not target.is_relative_to(output), 'fixture-output-placement')
    require(config['external_targets_policy']['path']
            == str(root / '.cargo' / 'external-targets.json'),
            'fixture-external-policy-placement')
    require(config['space_policy']['path']
            == str(root / '.cargo' / 'storage-policy.json'),
            'fixture-space-policy-placement')
    require(Path(config['cargo_lock']['path']) == workspace / 'Cargo.lock',
            'fixture-lock-placement')
    for key in BINDING_KEYS:
        require(Path(config[key]['path']).is_file(),
                f'fixture-binding-file-missing:{key}')


def load_source_map(value):
    """Validate the strict plain relative-path -> sha256 source map structure."""
    require(isinstance(value, dict) and value, 'fixture-source-map-empty')
    for key, expected in value.items():
        require(isinstance(key, str) and key and key != '.'
                and '\\' not in key and '\0' not in key
                and not key.startswith('/'), 'fixture-source-map-key')
        require('..' not in PurePosixPath(key).parts, 'fixture-source-map-key')
        require(isinstance(expected, str)
                and SHA256.fullmatch(expected) is not None,
                'fixture-source-map-hash')
    return value


def load_registry_map(value):
    """Validate the strict absolute-path -> sha256 registry map structure."""
    require(isinstance(value, dict) and value, 'fixture-registry-map-empty')
    for key, expected in value.items():
        require(isinstance(key, str) and key.startswith('/')
                and '\0' not in key and key == os.path.normpath(key),
                'fixture-registry-map-key')
        require(isinstance(expected, str)
                and SHA256.fullmatch(expected) is not None,
                'fixture-registry-map-hash')
    return value


def _read_bound_json(config, key):
    """Read one bound JSON file once, verifying its declared hash before parsing."""
    raw = Path(config[key]['path']).read_bytes()
    require(hashlib.sha256(raw).hexdigest() == config[key]['sha256'],
            f'fixture-binding-hash-mismatch:{key}')
    return _loads(raw)


def verify_source_entries(frozen, source_map):
    """Return content/containment drifts; original IO failures propagate.

    An IO error stops this map comparison and leaves it unqualified. The caller
    independently attempts the other map/binding/storage postchecks.
    """
    drifts = []
    for key, expected in source_map.items():
        resolved = (frozen / key).resolve()
        if not resolved.is_relative_to(frozen):
            drifts.append({'path': key, 'reason': 'escape'})
            continue
        observed = digest(resolved)
        if observed != expected:
            drifts.append({'path': key, 'expected': expected, 'observed': observed})
    return drifts


def verify_registry_entries(registry_map):
    """Return observed registry drifts; retain original IO failure types/causes."""
    drifts = []
    for key, expected in registry_map.items():
        observed = digest(key)
        if observed != expected:
            drifts.append({'path': key, 'expected': expected, 'observed': observed})
    return drifts


def _verify_bindings(config):
    """Recheck binding hashes; report content drift and preserve original IO errors."""
    drifts = {}
    for key in BINDING_KEYS:
        observed = _binding_observed(config[key])
        if observed != config[key]['sha256']:
            drifts[key] = observed
    return drifts


def parse_messages(raw):
    """Split cargo JSON stdout into (artifact, build-finished) record lists.

    Cargo emits one JSON object per line under --message-format=json; any
    unparsable or non-object line refuses so partial output is never silently
    accepted. Other reasons (for example build-script-executed) are ignored
    records, as they carry no selected-test qualification fact.
    """
    artifacts = []
    finished = []
    for line in raw.decode('utf-8').splitlines():
        if not line.strip():
            continue
        try:
            value = _loads(line)
        except ValueError as error:
            raise ValueError('fixture-stdout-unparsable-line') from error
        require(isinstance(value, dict), 'fixture-stdout-unexpected-record')
        if value.get('reason') == 'compiler-artifact':
            artifacts.append(value)
        elif value.get('reason') == 'build-finished':
            finished.append(value)
    return artifacts, finished


def audit_depfiles(*, target, frozen, workspace_root, source_map, registry_map,
                   output):
    """Audit observed Rust depfile inputs and publish classified evidence.

    Each captured byte buffer supplies its hash, parsing and raw archive; no
    second source read is used to archive that observation. Every *.d under the admitted target's debug/deps is read once, copied raw
    into the fresh output (raw-depfiles/), and its first rule line (backslash
    continuations joined) is classified per declared input: relative paths
    resolve canonically against the declared Cargo workspace root, including
    parent components; classification still requires containment and binding. A wrongly
    declared base stays an explicit missing-entry failure and is never repaired
    by trying other roots; existing files under the frozen tree must be bound
    with a matching hash in the provided source map; other files must be
    prebound registry inputs; files under the admitted target are recorded as
    generated outputs with limited depfile-observation provenance and preserved
    under raw-generated/. Missing, unbound, drifted and unclassified entries
    are published first and then refuse qualification.
    """
    deps = target / 'debug' / 'deps'
    require(deps.is_dir(), 'fixture-deps-dir-missing')
    depfiles = sorted(deps.glob('*.d'))
    require(depfiles, 'fixture-no-depfiles')
    raw_depfiles = output / 'raw-depfiles'
    raw_generated = output / 'raw-generated'
    depfile_hashes = {}
    workspace = {}
    registry = {}
    generated = {}
    missing = []
    unmatched = {}
    for depfile in depfiles:
        raw = depfile.read_bytes()
        depfile_hashes[str(depfile)] = hashlib.sha256(raw).hexdigest()
        publish_observed(raw, raw_depfiles / depfile.name)
        try:
            text = raw.decode('utf-8')
        except UnicodeDecodeError:
            unmatched[str(depfile)] = 'depfile-not-utf8'
            continue
        first = text.replace('\\\n', ' ').split('\n', 1)[0]
        if ': ' not in first:
            unmatched[str(depfile)] = 'depfile-unparseable'
            continue
        for name in shlex.split(first.split(': ', 1)[1]):
            stated = Path(name)
            if stated.is_absolute():
                resolved = Path(os.path.realpath(stated))
            else:
                resolved = Path(os.path.realpath(workspace_root / stated))
            if not resolved.is_file():
                missing.append({'depfile': str(depfile), 'entry': name,
                                'resolved': str(resolved)})
                continue
            observed_bytes = resolved.read_bytes()
            observed = hashlib.sha256(observed_bytes).hexdigest()
            if resolved.is_relative_to(frozen):
                key = resolved.relative_to(frozen).as_posix()
                expected = source_map.get(key)
                if expected is None:
                    unmatched[str(resolved)] = 'workspace-source-unbound'
                elif expected != observed:
                    unmatched[str(resolved)] = 'workspace-source-hash-mismatch'
                else:
                    workspace[str(resolved)] = observed
            elif str(resolved) in registry_map:
                if registry_map[str(resolved)] == observed:
                    registry[str(resolved)] = observed
                else:
                    unmatched[str(resolved)] = 'registry-hash-mismatch'
            elif resolved.is_relative_to(target):
                key = str(resolved)
                if key in generated:
                    require(generated[key] == observed, 'fixture-generated-drift')
                else:
                    publish_observed(observed_bytes,
                                     raw_generated / resolved.relative_to(target))
                    generated[key] = observed
            else:
                unmatched[str(resolved)] = 'unclassified-external'
    declared = not missing and not unmatched
    classes = {'workspace': workspace, 'registry': registry, 'generated': generated}
    publish(output / 'depfile-missing.json', json.dumps(missing, indent=2) + '\n')
    publish(output / 'depfile-unmatched.json',
            json.dumps(unmatched, indent=2) + '\n')
    publish(output / 'actual-source-dependencies.json', json.dumps(
        {**workspace, **registry, **generated}, indent=2, sort_keys=True) + '\n')
    publish(output / 'dependency-classes.json',
            json.dumps(classes, indent=2, sort_keys=True) + '\n')
    proof = {'workspace_all_declared': declared,
             'depfiles': depfile_hashes,
             'resolution_base': str(workspace_root),
             'counts': {'depfiles': len(depfile_hashes), 'workspace': len(workspace),
                        'registry': len(registry), 'generated': len(generated),
                        'missing': len(missing), 'unmatched': len(unmatched)},
             'raw_depfiles_dir': str(raw_depfiles),
             'raw_generated_dir': str(raw_generated),
             'provenance': ('Observed cargo depfile inputs only; generated entries '
                            'are depfile-observed outputs under the admitted '
                            'target, never claimed prebound')}
    publish(output / 'depfile-proof.json', json.dumps(proof, indent=2) + '\n')
    require(declared, 'fixture-depfile-audit-failed')
    return proof


def retain_elf(*, artifact, config, output, source_inputs_sha256):
    """Copy the matched fixture executable into fresh evidence and bind receipts.

    The copy is exclusive, readback-verified and owner-executable (0700); the receipt records the actual
    artifact features/profile/target, the body_executed=False fact, the source
    and registry map and lock linkage, and the source-inputs hash. The original
    stays in the admitted target cache; nothing is moved, renamed or deleted.
    """
    source = Path(artifact['executable'])
    destination = output / 'retained-fixture-binaries' / source.name
    copy_exclusive(source, destination)
    destination.chmod(0o700)
    with destination.open('rb') as stream:
        os.fsync(stream.fileno())
    observed = digest(destination)
    expected = artifact.get('executable_sha256')
    if expected:
        require(expected == observed, 'fixture-elf-artifact-hash-mismatch')
    receipt = {'path': str(destination),
               'sha256': observed,
               'features': artifact['features'],
               'profile': artifact['profile'],
               'source_inputs_sha256': source_inputs_sha256,
               'source_manifest': dict(config['source_map']),
               'registry_manifest': dict(config['registry_map']),
               'cargo_lock': dict(config['cargo_lock']),
               'target': artifact['target'],
               'body_executed': False,
               'recipe': {'package': config['package'], 'test': config['test'],
                          'features': config['features']}}
    publish(output / 'retained-elf.json', json.dumps(receipt, indent=2) + '\n')
    return receipt


def observed_child_exit(result, native_result, primary):
    """Retain a known child exit even when the execution helper raises.

    Prefer the returned process, then its published result, then the exception's
    native outcome when receipt publication failed. Missing/non-integer values
    stay unknown. This observation does not qualify the controller or compile.
    """
    outcome = getattr(primary, 'native_outcome', None)
    subprocess_result = outcome.get('subprocess') if isinstance(outcome, dict) else None
    candidates = [getattr(result, 'returncode', None),
                  native_result.get('returncode') if isinstance(native_result, dict) else None,
                  subprocess_result.get('returncode') if isinstance(subprocess_result, dict) else None]
    return next((value for value in candidates if type(value) is int), None)


def _require_receipts(output):
    """Require the helper's own result and admission-after receipts; parse result."""
    for name in (EXECUTION_NAME + '-result.json',
                 EXECUTION_NAME + '-admission-after.json'):
        require((output / name).is_file(), f'fixture-compile-receipt-missing:{name}')
    return read_json(output / (EXECUTION_NAME + '-result.json'))


def qualify(config_path, config_sha256, *, admission=None, execute=None, space=None):
    """Run one compile-only fixture qualification; raise on any failed stage.

    admission defaults to build_storage.admitted, execute to
    native_build_outcome.execute and space to cargo_space_guard.check_space.
    These callable seams exist solely for inert regression tests and are never
    exposed through the CLI, which always uses the real helpers. Sequence:
    strict config load; placement checks; binding and full input verification;
    held storage admission and mount check; persistent space policy check; fresh
    exclusive output creation; held admission across child, receipts, postchecks
    and ELF copy; combined error
    retention (never a successful empty result). Returns the summary document
    on full success.
    """
    admission = build_storage.admitted if admission is None else admission
    execute = native_build_outcome.execute if execute is None else execute
    if space is None:
        space = lambda paths, minimum: cargo_space_guard.check_space(paths, minimum)
    config = load_config(config_path, config_sha256)
    validate_paths(config)
    root = Path(config['root'])
    frozen = Path(config['frozen'])
    workspace = Path(config['workspace_root'])
    target = Path(config['target'])
    output = Path(config['output'])
    for key in ('cargo_lock', 'external_targets_policy', 'space_policy'):
        require(_binding_observed(config[key]) == config[key]['sha256'],
                f'fixture-binding-hash-mismatch:{key}')
    source_map = load_source_map(_read_bound_json(config, 'source_map'))
    registry_map = load_registry_map(_read_bound_json(config, 'registry_map'))
    source_drifts = verify_source_entries(frozen, source_map)
    require(not source_drifts, 'fixture-source-input-hash-mismatch')
    registry_drifts = verify_registry_entries(registry_map)
    require(not registry_drifts, 'fixture-registry-input-hash-mismatch')
    for key in ENV_REFUSALS:
        require(key not in os.environ, f'fixture-environment-refused:{key}')
    space_policy = cargo_space_guard.load_policy(config['space_policy']['path'])
    minimum = space_policy['minimum_free_bytes']
    argv = ['cargo', 'test', '--manifest-path', config['manifest_path'],
            '--offline', '--locked', '-p', config['package'],
            '--features', ','.join(config['features']), '--test', config['test'],
            '--no-run', '--message-format=json']
    tools = {'adapter': _binding(Path(__file__)),
             'build_storage': _binding(Path(build_storage.__file__)),
             'native_build_outcome': _binding(Path(native_build_outcome.__file__)),
             'cargo_space_guard': _binding(Path(cargo_space_guard.__file__))}
    with admission(config['root'], config['target']) as guard:
        before_proof = guard.check()
        require(before_proof['mode'] == 'registered-external',
                'fixture-registered-output-admission-required')
        mount = Path(before_proof['mount']['target'])
        require(output.parent.resolve().is_relative_to(mount)
                and output.parent.stat().st_dev == before_proof['anchor_device'],
                'fixture-output-volume-mismatch')
        require(not verify_source_entries(frozen, source_map),
                'fixture-source-input-hash-mismatch')
        require(not verify_registry_entries(registry_map),
                'fixture-registry-input-hash-mismatch')
        require(not _verify_bindings(config), 'fixture-binding-drift-before-child')
        space_before = space([root, output.parent, target], minimum)
        os.mkdir(output, 0o700)
        parent_fd = os.open(output.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
        post = []

        def step(function):
            """Run one postcheck step, retaining its failure without masking others."""
            try:
                return function()
            except BaseException as error:
                post.append(error)
                return None

        invocation = {'schema_version': 1,
                      'config': config,
                      'config_sha256': config_sha256,
                      'argv': argv,
                      'cwd': str(frozen),
                      'environment': {'refused_keys': list(ENV_REFUSALS),
                                      'overrides': {'CARGO_TARGET_DIR': str(target)},
                                      'otherwise_inherited': True},
                      'admission_tools': tools,
                      'source_map': {**config['source_map'],
                                     'entries': len(source_map)},
                      'registry_map': {**config['registry_map'],
                                       'entries': len(registry_map)},
                      'cargo_lock': config['cargo_lock'],
                      'external_targets_policy': config['external_targets_policy'],
                      'space_policy': {**config['space_policy'],
                                       'minimum_free_bytes': minimum,
                                       'observations': space_before},
                      'storage_before': before_proof,
                      'network': 'Cargo offline mode; no process-network sandbox assertion',
                      'watchdog': None,
                      'retry': None,
                      'scope': SCOPE}
        step(lambda: publish(output / 'invocation.json',
                             json.dumps(invocation, indent=2) + '\n'))
        result = None
        primary = None
        if not post:
            try:
                result = execute(argv, out=output, name=EXECUTION_NAME,
                                 admission=guard, publish=publish, digest=digest,
                                 failure_code='fixture-compile-failed',
                                 cwd=str(frozen),
                                 env=dict(os.environ,
                                          CARGO_TARGET_DIR=str(target)))
            except BaseException as error:
                primary = error
        native_result = step(lambda: _require_receipts(output))
        parsed = step(lambda: parse_messages(
            (output / (EXECUTION_NAME + '.stdout')).read_bytes()))
        artifacts, finished = parsed if parsed is not None else ([], [])
        step(lambda: publish(output / 'artifacts.json',
                             json.dumps(artifacts, indent=2) + '\n'))
        source_after = step(lambda: verify_source_entries(frozen, source_map))
        registry_after = step(lambda: verify_registry_entries(registry_map))
        bindings_after = step(lambda: _verify_bindings(config))
        space_after = step(lambda: space([root, output, target], minimum))
        after_proof = step(lambda: guard.check())
        source_stable = source_after == []
        registry_stable = registry_after == []
        if source_after or registry_after or bindings_after:
            step(lambda: publish(output / 'stability-drift.json', json.dumps(
                {'source': source_after, 'registry': registry_after,
                 'bindings': bindings_after}, indent=2) + '\n'))
        if source_after:
            post.append(ValueError('fixture-source-drift-after-child'))
        if registry_after:
            post.append(ValueError('fixture-registry-drift-after-child'))
        if bindings_after:
            post.append(ValueError('fixture-binding-drift-after-child'))
        record = {'schema_version': 1,
                  'status': 'FIXTURE_COMPILE_EVIDENCE',
                  'argv': argv,
                  'cwd': str(frozen),
                  'package': config['package'],
                  'test': config['test'],
                  'features': config['features'],
                  'exit': observed_child_exit(result, native_result, primary),
                  'source_stable': source_stable,
                  'registry_stable': registry_stable,
                  'build_finished': finished,
                  'artifact_count': len(artifacts),
                  'source_inputs_sha256': config['source_map']['sha256'],
                  'source_map': config['source_map'],
                  'registry_map': config['registry_map'],
                  'cargo_lock': config['cargo_lock'],
                  'external_targets_policy': config['external_targets_policy'],
                  'space_policy': {**config['space_policy'],
                                   'minimum_free_bytes': minimum},
                  'storage_before': before_proof,
                  'storage_after': after_proof,
                  'space_after': space_after,
                  'native_result': native_result,
                  'workspace_root': str(workspace),
                  'target': str(target),
                  'output': str(output),
                  'network': 'Cargo offline mode; no process-network sandbox assertion',
                  'watchdog': None,
                  'retry': None,
                  'scope': SCOPE}
        step(lambda: publish(output / 'result.json',
                             json.dumps(record, indent=2) + '\n'))
        step(lambda: publish(output / 'source-inputs.json',
                             Path(config['source_map']['path']).read_bytes()))
        step(lambda: publish(output / 'registry-inputs.json',
                             Path(config['registry_map']['path']).read_bytes()))
        freeze = {'frozen': str(frozen), 'fixture_target': str(target),
                  'root': str(root), 'workspace_root': str(workspace),
                  'manifest_path': config['manifest_path'],
                  'package': config['package'], 'test': config['test'],
                  'features': config['features'], 'count': len(source_map),
                  'source_inputs_sha256': config['source_map']['sha256'],
                  'source_map': config['source_map'],
                  'registry_map': config['registry_map'], 'scope': SCOPE}
        step(lambda: publish(output / 'freeze.json',
                             json.dumps(freeze, indent=2) + '\n'))
        receipt = None
        proof = None
        if primary is None and result is not None and result.returncode == 0 \
                and not post:
            try:
                require(isinstance(native_result, dict)
                        and native_result.get('returncode') == result.returncode
                        and native_result.get('execution_attempted') is True
                        and native_result.get('issues') == [],
                        'fixture-native-receipt-result')
                after_receipt = read_json(output / (EXECUTION_NAME + '-admission-after.json'))
                require(after_receipt.get('subprocess') == native_result
                        and after_receipt.get('admission_after', {}).get('checked') is True
                        and after_receipt.get('admission_after', {}).get('ok') is True
                        and after_receipt.get('receipt_publication', {}).get('ok') is True,
                        'fixture-native-receipt-admission')
                for channel in ('stdout', 'stderr'):
                    require(digest(output / (EXECUTION_NAME + '.' + channel))
                            == native_result.get(channel + '_sha256'),
                            'fixture-native-receipt-channel')
                require(finished == [{'reason': 'build-finished', 'success': True}],
                        'fixture-build-finished-not-unique')
                matches = [item for item in artifacts
                           if isinstance(item, dict) and item.get('executable')
                           and isinstance(item.get('target'), dict)
                           and item['target'].get('name') == config['test']]
                require(len(matches) == 1, 'fixture-artifact-not-unique')
                artifact = matches[0]
                require(artifact['target'].get('kind') == ['test'],
                        'fixture-artifact-kind')
                require(isinstance(artifact.get('features'), list)
                        and sorted(artifact['features'])
                        == sorted(config['features']),
                        'fixture-artifact-features-mismatch')
                profile = artifact.get('profile')
                require(isinstance(profile, dict) and profile.get('test') is True,
                        'fixture-artifact-profile-not-test')
                executable = Path(artifact['executable'])
                require(executable.is_absolute() and executable.is_file()
                        and executable.resolve().is_relative_to(target),
                        'fixture-artifact-executable-outside-target')
                proof = audit_depfiles(target=target, frozen=frozen,
                                       workspace_root=workspace,
                                       source_map=source_map,
                                       registry_map=registry_map, output=output)
                receipt = retain_elf(artifact=artifact, config=config,
                                     output=output,
                                     source_inputs_sha256=config['source_map']['sha256'])
            except BaseException as error:
                post.append(error)
        if primary is not None or post:
            if primary is not None:
                if not post:
                    raise primary
                combined = BaseExceptionGroup(
                    'Fixture qualification failed: compile and postcheck causes '
                    'retained', [primary, *post])
                if hasattr(primary, 'native_outcome'):
                    combined.native_outcome = primary.native_outcome
                raise combined
            if len(post) == 1:
                raise post[0]
            raise BaseExceptionGroup('Fixture postchecks failed', post)
        return {'status': 'FIXTURE_PROFILE_QUALIFIED',
                'output': str(output),
                'retained_elf': receipt,
                'depfile_proof': {'workspace_all_declared':
                                  proof['workspace_all_declared'],
                                  'counts': proof['counts']},
                'artifacts': len(artifacts),
                'source_inputs_sha256': config['source_map']['sha256']}


def failure_document(error):
    """Render original causes and observed publication/native outcomes.

    Traverse retained groups and chains without cycles. A successful link is
    historical evidence of publication, even if sync/cleanup later failed; it
    never implies current visibility, durability or rollback. No private process
    channels or environment contents are copied into this diagnostic.
    """
    document = native_build_outcome.failure_document(error)
    pending = [error]
    seen = set()
    publications = []
    outcomes = []
    seen_publications = set()
    seen_outcomes = set()
    while pending:
        current = pending.pop()
        if id(current) in seen:
            continue
        seen.add(id(current))
        observation = getattr(current, 'fixture_publication', None)
        if observation is not None and id(observation) not in seen_publications:
            seen_publications.add(id(observation))
            publications.append(observation)
        outcome = getattr(current, 'native_outcome', None)
        if outcome is not None and id(outcome) not in seen_outcomes:
            seen_outcomes.add(id(outcome))
            outcomes.append(outcome)
        pending.extend(getattr(current, 'exceptions', ()))
        for linked in (current.__cause__, current.__context__):
            if linked is not None:
                pending.append(linked)
    if publications:
        document['publication_observations'] = publications
    if outcomes:
        document['native_outcomes'] = outcomes
    return document


def main():
    """Read the one explicit config and qualify; print JSON summary or failure.

    No execution-authority, injection, retry or override flags exist; the only
    inputs are --config and --config-sha256. Failures render through the
    existing outcome API with every retained cause and exit nonzero.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--config-sha256', required=True)
    arguments = parser.parse_args()
    try:
        summary = qualify(arguments.config, arguments.config_sha256)
    except (KeyboardInterrupt, SystemExit):
        raise
    except BaseException as error:
        json.dump(failure_document(error), sys.stderr, indent=2)
        sys.stderr.write('\n')
        return 1
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
