#!/usr/bin/env python3
"""Proposed owned-child low-FD discriminator; not executed in its preparation.

Consume already-built, source/compiler-bound fixture drivers and close-only shim.
Never compile, enforce, launch a provider, touch a live Root or retry a close.
A healthy observed hook is required before each fresh injected child. An injected
errno after successful real close is synthetic, never an OS close-failure proof.
Compiler/effective-input qualification is supplied by the separately owned build
receipts; verifying their hashes here authenticates neither author nor execution.
"""
import argparse
import base64
import errno
import tarfile
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
from typing import NamedTuple
from fixture_area import fixture_root


def require(value, code):
    """Refuse incomplete/malformed evidence with a safe static diagnostic."""
    if not value:
        raise ValueError(code)


def digest(path):
    """Read complete explicitly supplied bytes, without interpreting/executing them."""
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def publish(path, value):
    """Sync/read back adjacent bytes, then publish only this fresh proof file."""
    raw = value if isinstance(value, bytes) else (json.dumps(value, indent=2)+'\n').encode()
    fd, temporary = tempfile.mkstemp(prefix='.'+path.name, dir=path.parent)
    with os.fdopen(fd, 'wb') as out:
        out.write(raw); out.flush(); os.fsync(out.fileno())
    require(Path(temporary).read_bytes() == raw, 'proof-readback-failed')
    os.replace(temporary, path)
    fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def load(path):
    """Reject duplicate JSON keys; a descriptor is input, never execution authority."""
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, 'descriptor-duplicate-key')
            result[key] = value
        return result
    return json.loads(path.read_bytes(), object_pairs_hook=unique)


def bound(item):
    """Hash an exact regular, non-live-state file; no recursive path discovery."""
    require(isinstance(item, dict) and set(item) == {'path', 'sha256'}, 'bound-file-fields')
    require(isinstance(item['path'], str) and Path(item['path']).is_absolute(), 'bound-file-absolute-path-required')
    path = Path(item['path']).resolve(strict=True)
    require(not any(part in {'.git', '.swarm'} for part in path.parts), 'live-state-input-refused')
    require(stat.S_ISREG(path.stat().st_mode), 'bound-file-not-regular')
    require(digest(path) == item['sha256'], 'bound-file-changed')
    return path


def cargo_record_hash(record):
    """Identify the exact parsed Cargo record, not a guessed filename suffix."""
    return hashlib.sha256(json.dumps(record, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


class LibraryCopy(NamedTuple):
    """Typed common view of explicitly supported producer receipts.

    Paths remain declarations until mapped_library binds original Cargo selection
    and exact frozen bytes. This view conveys no producer authentication.
    """
    original: str
    frozen: str
    sha256: str


def copy_fields(original, frozen, sha256):
    """Validate common path/hash types without reading a mutable Cargo cache.

    Return a typed view or a static refusal. Absolute paths and a SHA256 are
    protocol representation invariants, not configurable operator policy.
    """
    require(all(isinstance(p, str) and Path(p).is_absolute() for p in (original, frozen)), 'copy-path-fields-invalid')
    require(isinstance(sha256, str) and len(sha256) == 64 and all(c in '0123456789abcdef' for c in sha256), 'copy-hash-field-invalid')
    return LibraryCopy(original, frozen, sha256)


def nested_copy(mapping, record, linked):
    """Validate the observed component producer's nested mapping against Cargo.

    Check typed metadata, exact profile/features/freshness, filename and frozen
    bytes. A second retained copy may have a different path but must have the
    same bytes as the actual extern. Read frozen evidence only, never originals.
    """
    fields = {'package_id', 'target', 'profile', 'features', 'cargo_fresh', 'original', 'original_filename', 'frozen', 'sha256', 'bytes'}
    require(isinstance(mapping, dict) and set(mapping) == fields, 'nested-copy-fields')
    view = copy_fields(mapping['original'], mapping['frozen'], mapping['sha256'])
    require(isinstance(mapping['package_id'], str) and isinstance(mapping['target'], str) and isinstance(mapping['original_filename'], str), 'nested-copy-metadata-types')
    require(isinstance(mapping['profile'], dict) and isinstance(mapping['features'], list) and all(isinstance(x, str) for x in mapping['features']) and type(mapping['cargo_fresh']) is bool and type(mapping['bytes']) is int and mapping['bytes'] >= 0, 'nested-copy-metadata-types')
    require(mapping['package_id'] == record['package_id'] and mapping['target'] == record['target']['name'] and cargo_record_hash(mapping['profile']) == cargo_record_hash(record['profile']) and mapping['features'] == record['features'] and mapping['cargo_fresh'] is record['fresh'], 'nested-copy-record-mismatch')
    require(view.original in record['filenames'] and view.original.endswith('.rlib') and mapping['original_filename'] == Path(view.original).name, 'nested-copy-original-mismatch')
    frozen = bound({'path': view.frozen, 'sha256': view.sha256})
    require(mapping['bytes'] == frozen.stat().st_size == linked.stat().st_size and view.sha256 == digest(linked), 'nested-copy-linked-bytes-mismatch')
    return view


def producer_copy(role, receipt, record, files, cargo_stdout, revision, libc_record):
    """Normalize only the three observed receipt protocols, then bind common data.

    Biba supports the flat normal-library receipt and nested normal-component
    receipt. Libc supports its retained-cache receipt. Status selects a protocol,
    never a mission/revision exception. No synthetic producer receipt or fallback
    is accepted; malformed/mixed shapes refuse before any child can start.
    """
    require(isinstance(receipt, dict) and isinstance(receipt.get('status'), str), 'copy-receipt-fields-incomplete')
    status = receipt['status']
    if role == 'biba_rlib':
        require(isinstance(receipt.get('artifact'), dict) and cargo_record_hash(receipt['artifact']) == cargo_record_hash(record), 'biba-copy-record-mismatch')
        inventory = receipt.get('effective_inputs')
        require(isinstance(inventory, dict), 'biba-copy-source-inventory-required')
        require(inventory.get('rust/crates/swarm-biba-linux/src/lib.rs') == digest(files['crate_lib']) and inventory.get('rust/crates/swarm-biba-linux/src/kernel.rs') == digest(files['crate_kernel']), 'biba-copy-source-mismatch')
        if status == 'QUALIFIED_NORMAL_BIBA_LIBRARY_ONLY':
            require({'original_library', 'frozen_library', 'sha256'} <= set(receipt) and not ({'head', 'libc_frozen_mapping'} & set(receipt)), 'flat-copy-shape-invalid')
            return copy_fields(receipt['original_library'], receipt['frozen_library'], receipt['sha256'])
        require(status == 'QUALIFIED_BASELINE8D_NORMAL_BIBA_LIBRARY_COMPONENT_ONLY', 'biba-producer-receipt-kind')
        require({'head', 'frozen_library', 'libc_frozen_mapping'} <= set(receipt) and not ({'original_library', 'sha256'} & set(receipt)), 'nested-copy-shape-invalid')
        require(isinstance(receipt['head'], str) and receipt['head'] == revision, 'biba-copy-revision-mismatch')
        view = nested_copy(receipt['frozen_library'], record, files['biba_rlib'])
        nested_copy(receipt['libc_frozen_mapping'], libc_record, files['libc_rlib'])
        return view
    require(role == 'libc_rlib' and status == 'RETAINED_CACHED_LIBC_DEPENDENCY_NOT_REBUILT', 'libc-producer-receipt-kind')
    require({'original_library', 'frozen_library', 'sha256'} <= set(receipt) and not ({'head', 'libc_frozen_mapping'} & set(receipt)), 'flat-copy-shape-invalid')
    require(receipt.get('artifact_package') == record['package_id'] and receipt.get('artifact_profile') == record['profile'] and receipt.get('features') == record['features'], 'libc-copy-record-mismatch')
    require(receipt.get('actual_cargo_stdout_sha256') == digest(cargo_stdout), 'libc-cargo-channel-mismatch')
    return copy_fields(receipt['original_library'], receipt['frozen_library'], receipt['sha256'])


def mapped_library(role, record, binding, files, cargo_stdout, revision, libc_record):
    """Bind a selected Cargo filename to the exact extern via a typed copy view.

    Known producer declarations do not authenticate historical compilation or
    registry/toolchain provenance. Unknown/mixed/incomplete mappings refuse;
    mutable original paths need not remain readable. Return the bound receipt.
    """
    require(isinstance(binding, dict) and set(binding) == {'cargo_filename', 'cargo_record_sha256', 'copy_receipt'}, 'cargo-binding-fields')
    source = binding['cargo_filename']
    require(isinstance(source, str) and source in record['filenames'] and source.endswith('.rlib') and Path(source).is_absolute(), 'cargo-artifact-filename-not-selected')
    require(binding['cargo_record_sha256'] == cargo_record_hash(record), 'cargo-record-binding-mismatch')
    receipt_path = bound(binding['copy_receipt']); receipt = load(receipt_path)
    view = producer_copy(role, receipt, record, files, cargo_stdout, revision, libc_record)
    require(view.original == source and Path(view.frozen).resolve() == files[role], 'cargo-original-frozen-path-mismatch')
    require(view.sha256 == digest(files[role]), 'cargo-frozen-bytes-mismatch')
    return receipt_path


def inputs(path):
    """Check matrix revisions, source roles, compiler closure and Cargo-copy links.

    The versioned matrix is an independently reviewed counterexample input,
    not an operator policy. Root owns component qualification/authenticity.
    Neither unequal labels nor individually valid hashes justify a mixed rlib.
    """
    doc = load(path)
    require(set(doc) == {'schema_version', 'baseline', 'candidate', 'shim'}, 'inputs-fields')
    require(type(doc['schema_version']) is int and doc['schema_version'] == 2, 'inputs-version')
    matrix_path = Path(__file__).with_name('fixtures')/'biba_low_fd_matrix.json'
    matrix = load(matrix_path)
    require(set(matrix) == {'schema_version', 'baseline', 'candidate', 'libc'} and type(matrix['schema_version']) is int and matrix['schema_version'] == 1, 'matrix-fields')
    checked = {}
    for label in ['baseline', 'candidate', 'shim']:
        item = doc[label]
        require(set(item) == {'artifact', 'receipt'}, 'artifact-fields')
        artifact = bound(item['artifact']); receipt_path = bound(item['receipt']); receipt = load(receipt_path)
        require(set(receipt) == {'schema_version', 'kind', 'revision', 'artifact_sha256', 'compiler_argv', 'returncode', 'profile', 'files', 'cargo_bindings'}, 'receipt-fields')
        require(type(receipt['schema_version']) is int and receipt['schema_version'] == 2, 'receipt-version')
        require(type(receipt['returncode']) is int and receipt['returncode'] == 0, 'compiler-not-successful')
        require(receipt['artifact_sha256'] == digest(artifact), 'receipt-artifact-mismatch')
        compiler = receipt['compiler_argv']
        require(isinstance(compiler, list) and compiler and all(isinstance(x, str) for x in compiler), 'compiler-argv-required')
        require(receipt['kind'] == ('close-shim' if label == 'shim' else 'public-biba-low-fd-driver'), 'artifact-kind')
        profile = receipt['profile']
        require(isinstance(profile, dict) and set(profile) == {'fixture_binary', 'dependency_features', 'dependency_test'}, 'fixture-profile-fields')
        require(profile['fixture_binary'] is True and profile['dependency_test'] is False and profile['dependency_features'] == [], 'fixture-profile-required')
        roles = receipt['files']; required = {'source', 'compiler_stdout', 'compiler_stderr', 'compiler_identity'}
        if label != 'shim':
            required |= {'source_archive', 'effective_inventory', 'cargo_artifacts', 'depfiles', 'biba_rlib', 'libc_rlib', 'crate_lib', 'crate_kernel', 'driver_source'}
        require(isinstance(roles, dict) and set(roles) == required, 'compiler-source-closure-incomplete')
        files = {role: bound(value) for role, value in roles.items()}
        require('-o' in compiler and compiler[compiler.index('-o')+1:] and Path(compiler[compiler.index('-o')+1]).resolve() == artifact, 'compiler-output-link-missing')
        copies = {}
        if label != 'shim':
            require(receipt['revision'] == matrix[label]['revision'], 'allocated-revision-mismatch')
            require(digest(files['crate_lib']) == matrix[label]['crate_lib_sha256'] and digest(files['crate_kernel']) == matrix[label]['crate_kernel_sha256'], 'allocated-source-mismatch')
            require(str(files['driver_source']) in compiler and 'swarm_biba_linux='+str(files['biba_rlib']) in compiler and 'libc='+str(files['libc_rlib']) in compiler, 'compiler-input-link-missing')
            messages = [json.loads(line) for line in files['cargo_artifacts'].read_text().splitlines() if line.startswith('{')]
            emitted = [m for m in messages if m.get('reason') == 'compiler-artifact']
            require(not any(set(m.get('features', [])) & {'test-fixtures', 'test-util'} for m in emitted), 'fixture-feature-in-dependency-graph')
            biba = [m for m in emitted if m['target']['name'] == 'swarm_biba_linux' and m['target']['kind'] == ['lib']]
            libc = [m for m in emitted if m['target']['name'] == 'libc' and m['target']['kind'] == ['lib'] and m['package_id'].rsplit('#', 1)[-1] == matrix['libc']['package']]
            require(len(biba) == 1 and biba[0]['features'] == [] and biba[0]['profile']['test'] is False, 'biba-library-artifact-profile')
            require(len(libc) == 1 and sorted(libc[0]['features']) == matrix['libc']['features'] and libc[0]['profile']['test'] is False, 'libc-library-artifact-profile')
            require(isinstance(receipt['cargo_bindings'], dict) and set(receipt['cargo_bindings']) == {'biba_rlib', 'libc_rlib'}, 'cargo-bindings-required')
            for role, record in [('biba_rlib', biba[0]), ('libc_rlib', libc[0])]:
                copies[role] = mapped_library(role, record, receipt['cargo_bindings'][role], files, files['cargo_artifacts'], receipt['revision'], libc[0])
            require(digest(files['driver_source']) == digest(Path(__file__).with_name('rust_biba_low_fd_driver.rs')) == digest(files['source']), 'driver-source-mismatch')
            inventory = load(files['effective_inventory']); require(isinstance(inventory, dict) and inventory, 'effective-inventory-required'); observed = {}
            with tarfile.open(files['source_archive'], 'r:gz') as archive:
                for member in archive:
                    name = Path(member.name)
                    require(member.isfile() and not name.is_absolute() and '..' not in name.parts and member.name not in observed, 'source-archive-member-invalid')
                    observed[member.name] = hashlib.sha256(archive.extractfile(member).read()).hexdigest()
            require(observed == inventory, 'effective-source-archive-mismatch')
            for role, name in [('crate_lib', 'rust/crates/swarm-biba-linux/src/lib.rs'), ('crate_kernel', 'rust/crates/swarm-biba-linux/src/kernel.rs'), ('driver_source', 'tests/rust_biba_low_fd_driver.rs')]:
                require(inventory.get(name) == digest(files[role]), 'compiler-source-role-mismatch')
        else:
            require(receipt['cargo_bindings'] == {}, 'shim-cargo-binding-invalid')
            require(str(files['source']) in compiler, 'shim-compiler-link-missing')
            require(digest(files['source']) == digest(Path(__file__).with_name('fixtures')/'biba_ruleset_close.c'), 'shim-source-mismatch')
        checked[label] = dict(path=artifact, receipt=receipt, files=files, descriptor=item, copy_receipts=copies, matrix=matrix_path)
    return doc, checked


def parent_stdio():
    """Observe this runner's stdio identities; never close/reassign them."""
    return [(os.fstat(fd).st_dev, os.fstat(fd).st_ino, stat.S_IFMT(os.fstat(fd).st_mode)) for fd in range(3)]


def parse_report(path):
    """Read only the driver's closed TSV vocabulary, not arbitrary error text."""
    values = {'causes': [], 'retained': [], 'driver_errors': []}
    for raw in path.read_text().splitlines():
        row = raw.split('\t'); kind = row[0]
        if kind == 'stdio':
            require(row == ['stdio', 'free', '0', '1', '2'], 'stdio-not-free'); values['stdio_free'] = True
        elif kind in {'prepare', 'cleanup', 'restored'}:
            require(len(row) == 2 and kind not in values, 'report-phase-invalid'); values[kind] = row[1]
        elif kind == 'cause':
            require(len(row) == 4 and row[1] in {'prepare', 'cleanup'}, 'cause-fields')
            require(row[2] and all(c.isascii() and (c.isalnum() or c in '-_') for c in row[2]), 'cause-stage-invalid')
            values['causes'].append([row[1], row[2], None if row[3] == 'none' else int(row[3])])
        elif kind == 'retained':
            require(len(row) == 2, 'retained-fields'); values['retained'].append(int(row[1]))
        elif kind == 'driver-error':
            values['driver_errors'].append(row[1:])
        else:
            raise ValueError('report-kind-invalid')
    require(values.get('stdio_free') and values.get('restored') == 'yes' and not values['driver_errors'], 'child-ownership-or-restoration-incomplete')
    require(values.get('prepare') in {'ok', 'refused'}, 'prepare-observation-missing')
    return values


def parse_events(path):
    """Validate observed close records, including the real vs delivered result."""
    result = []
    for raw in path.read_text().splitlines():
        row = raw.split('\t'); require(len(row) == 7 and row[0] == 'close', 'close-event-fields')
        phase, fd, rc, actual_errno, delivered, synthetic = map(int, row[1:])
        require(phase in {1, 2} and fd >= 0 and synthetic in {0, 1}, 'close-event-invalid')
        result.append(dict(phase=phase, fd=fd, real_rc=rc, real_errno=actual_errno, delivered_errno=delivered, synthetic=bool(synthetic)))
    return result


def observed_witness(case):
    """Require real interposition/low-allocation evidence before any fault child."""
    report, events = case['report'], case['events']
    require(report['prepare'] == 'ok' and report.get('cleanup') == 'ok' and not report['causes'], 'healthy-prepare-required')
    require(len(report['retained']) == 1 and report['retained'][0] > 2, 'healthy-high-custody-required')
    require(len(events) == 2, 'healthy-close-interposition-not-observed')
    low, high = events
    require(low['phase'] == 1 and low['fd'] <= 2 and high['phase'] == 2 and high['fd'] == report['retained'][0], 'healthy-ruleset-low-high-not-observed')
    require(all(e['real_rc'] == 0 and e['real_errno'] == 0 and e['delivered_errno'] == 0 and not e['synthetic'] for e in events), 'healthy-close-not-clean')
    return case['binding']


def safe_causes(error, seen=None):
    """Preserve exception topology/errno/outcome without private OSError text."""
    seen = set() if seen is None else seen
    if id(error) in seen:
        return {'type': type(error).__name__, 'cycle': True}
    seen.add(id(error)); result = {'type': type(error).__name__}
    if isinstance(error, OSError):
        result['errno'] = error.errno
    if isinstance(error, ValueError) and all(c.isascii() and (c.isalnum() or c in '-_') for c in str(error)):
        result['code'] = str(error)
    if hasattr(error, 'low_fd_outcome'):
        result['completed_child'] = error.low_fd_outcome
    if isinstance(error, BaseExceptionGroup):
        result['exceptions'] = [safe_causes(e, seen) for e in error.exceptions]
    if error.__cause__ is not None:
        result['cause'] = safe_causes(error.__cause__, seen)
    elif error.__context__ is not None and not error.__suppress_context__:
        result['context'] = safe_causes(error.__context__, seen)
    return result


def completed_child(argv, env, channels, out, run_process=None, publish_file=None):
    """Observe completion BEFORE independently publishing outcome and both streams.

    Paired raw bytes are private fallback payloads in process/failure receipts;
    publication status is explicit, never inferred from observing bytes. Each
    filename is attempted once. Distinct failure publication is not a retry.
    No timeout/retry/compilation; injected callbacks support proposed source guards.
    """
    run_process = subprocess.run if run_process is None else run_process
    publish_file = publish if publish_file is None else publish_file
    errors = []; child = None
    try:
        child = run_process(argv, env=env, pass_fds=tuple(channels), stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    except BaseException as error:
        errors.append(error)
    if child is None and not errors:
        errors.append(ValueError('child-completion-missing'))
    outcome = dict(execution_attempted=True, completed=child is not None,
        returncode=None if child is None else child.returncode,
        signal=None if child is None or child.returncode >= 0 else -child.returncode,
        channels={}, publications={})
    if child is not None:
        for name, raw in [('stdout', child.stdout), ('stderr', child.stderr)]:
            outcome['channels'][name] = dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest(), raw_base64=base64.b64encode(raw).decode('ascii'))
        if child.returncode != 0:
            errors.insert(0, ValueError('child-returned-nonzero'))
    writes = [('process.json', outcome)]
    if child is not None:
        writes += [('stdout', child.stdout), ('stderr', child.stderr)]
    for name, payload in writes:
        state = {'attempted': True, 'ok': None}; outcome['publications'][name] = state
        try:
            publish_file(out/name, payload); state['ok'] = True
        except BaseException as error:
            state['ok'] = False; state['cause'] = safe_causes(error); errors.append(error)
    outcome['paired_file_retention_complete'] = child is not None and all(outcome['publications'].get(n, {}).get('ok') for n in ['process.json', 'stdout', 'stderr'])
    if errors:
        details = dict(status='FAIL_OR_INCOMPLETE', outcome=outcome, issues=[safe_causes(e) for e in errors])
        try:
            publish_file(out/'retention-failure.json', details)
        except BaseException as error:
            errors.append(error)
        failure = BaseExceptionGroup('Completed child or evidence retention failed', errors)
        failure.low_fd_outcome = outcome
        raise failure
    return child


def invoke(label, mode, artifact, shim, out, permit=None):
    """Run one new owned child with private channels and a minimal explicit env.

    The runner never signals another process or closes parent stdio. No timeout,
    retries or output truncation. Failure attempts paired retention and returns no completed assertion;
    per-file status, not observed bytes, defines what was published. A faulty case requires a just-observed matching witness.
    """
    binding = dict(driver_sha256=digest(artifact), shim_sha256=digest(shim))
    require(mode == 'control' or permit == binding, 'fault-without-matching-observed-witness')
    out.mkdir(mode=0o700)
    root = fixture_root('swarm-biba-low-fd-'); (root/'.swarm').mkdir(mode=0o700); workspace = root/'out'; workspace.mkdir(mode=0o700)
    before = parent_stdio(); files = []; primary = None
    try:
        for name in ['report.tsv', 'events.tsv']:
            fd = os.open(out/name, os.O_RDWR | os.O_CREAT | os.O_EXCL, 0o600); files.append(fd)
        report_fd, event_fd = files
        require(report_fd > 2 and event_fd > 2 and report_fd != event_fd, 'fixture-channels-overlap-stdio')
        env = {'LANG': 'C', 'LC_ALL': 'C', 'LD_PRELOAD': str(shim), 'SWARM_BIBA_CLOSE_EVENTS_FD': str(event_fd), 'SWARM_BIBA_CLOSE_MODE': mode}
        argv = [str(artifact), str(root), str(workspace), str(report_fd), str(event_fd)]
        publish(out/'invocation.json', dict(label=label, mode=mode, argv=argv, environment=env, binding=binding, parent_stdio=before, global_watchdog=None, scope='Owned child, no enforce/provider/service'))
        child = completed_child(argv, env, files, out)
        for fd in files:
            os.fsync(fd)
        require(parent_stdio() == before, 'parent-stdio-identity-changed')
        require(child.returncode == 0 and not child.stdout and not child.stderr, 'child-incomplete-or-diagnostic')
        value = dict(report=parse_report(out/'report.tsv'), events=parse_events(out/'events.tsv'), binding=binding, fixture_root=str(root))
        publish(out/'observations.json', value)
        return value
    except BaseException as error:
        primary = error
        raise
    finally:
        cleanup_errors = []
        for fd in files:
            try:
                os.close(fd)
            except BaseException as error:
                cleanup_errors.append(error)
        if cleanup_errors:
            raise BaseExceptionGroup('Fixture channels close failed', ([primary] if primary else []) + cleanup_errors)


def check_injected(case, mode, baseline):
    """Assert a behavior-level RED/GREEN, never accept a compilation/setup error."""
    events, report = case['events'], case['report']
    require(len(events) == 2 and len({e['fd'] for e in events}) == 2, 'owned-close-count-invalid')
    low, high = events
    require(low['phase'] == 1 and low['fd'] <= 2 and high['fd'] > 2, 'injected-ruleset-low-high-required')
    require(all(e['real_rc'] == 0 and e['real_errno'] == 0 for e in events), 'actual-close-error-not-synthetic-witness')
    require(low['synthetic'] and low['delivered_errno'] == errno.EIO, 'low-EIO-not-observed')
    require(high['synthetic'] == (mode == 'both') and high['delivered_errno'] == (errno.EBADF if mode == 'both' else 0), 'duplicate-close-observation-invalid')
    if baseline:
        require(report['prepare'] == 'ok' and high['phase'] == 2, 'baseline-ignored-close-RED-not-reproduced')
        require(report.get('cleanup') == ('refused' if mode == 'both' else 'ok'), 'baseline-explicit-cleanup-outcome')
        require(report['causes'] == ([['cleanup', 'biba-close', errno.EBADF]] if mode == 'both' else []), 'baseline-cleanup-cause-lost')
        return dict(status='EXPECTED_BEHAVIORAL_RED_OBSERVED', assertion='prepare_must_refuse_low_ruleset_close_error', actual=False)
    require(report['prepare'] == 'refused' and high['phase'] == 1 and not report['retained'], 'candidate-did-not-refuse-before-custody')
    expected = [['prepare', 'biba-close', errno.EIO]] + ([['prepare', 'biba-close', errno.EBADF]] if mode == 'both' else [])
    require(report['causes'] == expected, 'candidate-primary-cleanup-causes-lost')
    return dict(status='CANDIDATE_GREEN_OBSERVED', assertion='prepare_must_refuse_low_ruleset_close_error', actual=True)


def run(artifacts, out):
    """Execute only this declared differential matrix after explicit allocation.

    Per image: healthy witness before original-only fault; fresh healthy witness
    before dual fault; fresh recovery. Ten fresh owned children, not an operator
    concurrency/retry policy. Never run old full-boundary campaigns or build.
    """
    os.umask(0o077); out.mkdir(parents=False, exist_ok=False, mode=0o700)
    try:
        doc, checked = inputs(artifacts); publish(out/'artifact-inputs.json', doc)
        bindings = {label: dict(artifact=digest(v['path']), receipt=digest(bound(v['descriptor']['receipt'])), files={k: digest(p) for k, p in v['files'].items()}, copies={k: digest(p) for k, p in v['copy_receipts'].items()}, matrix=digest(v['matrix'])) for label, v in checked.items()}
        publish(out/'bindings.before.json', bindings)
        publish(out/'expected-matrix.json', checked['baseline']['matrix'].read_bytes())
        for label, item in checked.items():
            copied = out/('retained-'+label); copied.mkdir(mode=0o700)
            publish(copied/'compiler-receipt.json', bound(item['descriptor']['receipt']).read_bytes())
            for role, path in item['files'].items():
                publish(copied/role, path.read_bytes())
            for role, path in item['copy_receipts'].items():
                publish(copied/(role+'-copy-receipt.json'), path.read_bytes())
        cases = []
        for label in ['baseline', 'candidate']:
            artifact = checked[label]['path']; shim = checked['shim']['path']
            for mode in ['original', 'both']:
                control = invoke(label, 'control', artifact, shim, out/(label+'-'+mode+'-control'))
                permit = observed_witness(control)
                print('BIBA_LOW_FD_REAL_CONTROL '+label+' '+mode, flush=True)
                value = invoke(label, mode, artifact, shim, out/(label+'-'+mode), permit)
                result = check_injected(value, mode, label == 'baseline'); publish(out/(label+'-'+mode+'-assertion.json'), result); cases.append(result)
                print('BIBA_LOW_FD_EXPECTED_BASELINE_RED' if label == 'baseline' else 'BIBA_LOW_FD_CANDIDATE_GREEN', flush=True)
            recovery = invoke(label, 'control', artifact, shim, out/(label+'-fresh-recovery')); observed_witness(recovery)
            print('BIBA_LOW_FD_FRESH_RECOVERY '+label, flush=True)
        _, after = inputs(artifacts)
        after_bindings = {label: dict(artifact=digest(v['path']), receipt=digest(bound(v['descriptor']['receipt'])), files={k: digest(p) for k, p in v['files'].items()}, copies={k: digest(p) for k, p in v['copy_receipts'].items()}, matrix=digest(v['matrix'])) for label, v in after.items()}
        require(doc == load(artifacts) and bindings == after_bindings, 'fixture-inputs-changed-during-run')
        publish(out/'bindings.after.json', after_bindings)
        publish(out/'result.json', dict(status='RED_GREEN_OBSERVED_FIXTURE_ONLY', cases=cases, injected_after_successful_real_close=True, actual_OS_close_failure_proven=False, engine_diagnostic_flow=False, product_qualified=False, installed=False, global_watchdog=None))
    except BaseException as error:
        try:
            publish(out/'failure.json', dict(status='FAIL_OR_INCOMPLETE', causes=safe_causes(error), retention_scope='Only individually successful file publications; inspect nested outcome/publication status'))
        except BaseException as secondary:
            raise BaseExceptionGroup('Fixture and failure-publication failed', [error, secondary])
        raise


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifacts', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--from-environment', action='store_true')
    args = parser.parse_args()
    if args.from_environment:
        require(args.artifacts is None and args.output is None, 'ambiguous-fixture-paths')
        require(os.environ.get('SWARM_BIBA_LOW_FD_INPUTS') and os.environ.get('SWARM_BIBA_LOW_FD_OUTPUT'), 'fixture-paths-required')
        args.artifacts = Path(os.environ['SWARM_BIBA_LOW_FD_INPUTS']); args.output = Path(os.environ['SWARM_BIBA_LOW_FD_OUTPUT'])
    require(args.artifacts is not None and args.output is not None, 'fixture-paths-required')
    run(args.artifacts.resolve(), args.output.resolve())
