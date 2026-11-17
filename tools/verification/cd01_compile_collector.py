#!/usr/bin/env python3
"""CD01 one-shot compile-only collector (proposal; root/operator review required
before execution). It builds the swarm-engine library test binary without running
any test body and publishes honest compile-only evidence.

This collector is specific to the declared snapshot context (root is the local,
owned R1a6a4785... plus17reviewed postimages snapshot, declared externally and not
re-verified here). Existing SDK collectors tied to a different Git/source/review
state must not be reused unchanged and are not used here.

What it does (one shot; no retry, no pause, no timeout, no watchdog, no cap):
  * verifies the declared snapshot root (existing owned directory, world-writable
    refused, .git/.swarm absent) and the explicit CLI paths,
  * verifies EVERY entry of the original snapshot --inventory against current root
    bytes before Cargo (regular files by sha256 over real non-symlink files,
    symlinks by exact link text); equality is claimed only over indexed entries,
    never over the whole tree after arbitrary writes,
  * creates a NEW evidence output leaf under the already mounted evidence parent
    plus a NEW output/tmp, and holds an evidence directory FD for the whole run
    (close failures are preserved, never masked),
  * byte-pins the three exact supplied helper modules (verify_native_profile.py is
    loaded from the selected snapshot by explicit importlib; it co-loads the
    co-located build_storage.py and native_build_outcome.py), publishes their bytes
    and the collector source into the evidence output,
  * refuses unless the explicitly selected external Cargo target is ALREADY
    registered (refusal comes from storage.admitted); it NEVER registers, never
    edits registration policy, repository source or quota; the root independently
    authorizes registration and build beforehand,
  * binds profile.candidates(root), .cargo config inputs and collector/helper
    bytes, archives the declared workspace candidates BEFORE Cargo and rereads and
    verifies the published archive,
  * records the additive registration files (.cargo/external-targets.json and its
    lock) separately from the original inventory,
  * observes rustc -vV and cargo --version read-only through the same outcome
    helper under distinct receipt names; they are never counted as tests,
  * runs exactly one cargo argv: cargo test --manifest-path ROOT/rust/Cargo.toml
    -p swarm-engine --lib --no-run --locked --offline --message-format=json with
    default features (no feature flags), CARGO_TARGET_DIR on the admitted target,
    TMPDIR under the evidence output, CARGO_INCREMENTAL=0, dev/test debug=0; the
    child environment is an explicit allowlist (PATH/HOME plus declared build
    variables); no arbitrary ambient environment dump is made; inherited Rust
    flags/wrappers/toolchain/target overrides are refused FIRST and only after
    that refusal the collector constructs its own declared RUSTC_WRAPPER
    instrumentation (an env override of the workspace [build] rustc-wrapper)
    plus the two explicit producer config variables CD01_CAPTURE_DIR and
    CD01_SPACEGUARD_PATH; workspace .cargo/config bytes and discovery are
    otherwise preserved (no --config, CARGO_HOME or CLI HOME override); the
    cargo argv stays unchanged, which is NOT a claim of identical rustc argv
    or codegen,
  * stages the explicitly supplied reviewed rustc INTENT producer (validated,
    byte-pinned against the explicit expected SHA, copied executable 0700 into
    the evidence output with readback verification, archived and reread) and
    pins the explicitly supplied root admission guard in place by exact path
    and bytes (both shebangs recorded as observed; interpreter resolution is
    never automatically qualified),
  * creates its own NEW 0700 rustc-intent capture directory under the output;
    after Cargo it parses ONLY the producer's records there (lossless ASCII
    JSON, valid hex bytes, exact cd01.rustc_intent.v1 field set, outcome
    UNKNOWN, intent.json/intent.partial same-inode, one invocation directory
    per record with same-PID recurrence allowed), classifies metadata probes
    versus compile candidates from actual decoded arguments (probes are never
    counted as compiles), requires a positive --crate-name swarm_engine
    --test intent matching the selected artifact, and refuses any invalid
    record while always preserving raw partial/failure files,
  * requires the already-registered admitted Cargo target to be a FRESH empty
    tree before Cargo (the explicit technical no-reuse prerequisite; this is
    never a fake "cargo invoked zero compilers" proof; no registration change
    and no target cleanup is ever performed),
  * prebinds, before Cargo, the exact Cargo.lock-selected crates.io registry
    source directories and cache archives under the child HOME (archive
    SHA-256 must equal the locked checksum; missing, symlinked or unsupported
    sources refuse without fallback; no .cargo-checksum.json sidecar
    assumption; digests only, never contents) together with Cargo.lock itself,
    and re-observes them after Cargo,
  * pins the Cargo config discovery hierarchy for the child (cwd, ancestors
    and HOME config; absence is recorded as a fact while unreadable or
    unsupported entries refuse; credential files are never read) and includes
    producer, guard, collector, source and config byte pins in the per-child
    FrozenInputs check,
  * records the observed rustc/cargo PATH resolution with final-name symlink
    hop, final stat metadata and file SHA-256 for the selected (host) target;
    rustup proxy versus actual target binary stays UNKNOWN unless resolved and
    this is not a full toolchain proof,
  * checks the never-lowered 20 GiB free-bytes guard on the host checkout, the
    cargo target volume and the evidence volume before and after Cargo (fresh
    statvfs observations; historic free values and ENOSPC/CI061b-style pauses are
    never reused; no retry on ENOSPC),
  * after success selects exactly one swarm_engine compiler-artifact whose target
    kind includes lib, whose profile.test is true, whose features are exactly []
    and whose executable exists; refuses fixture features on ANY event; retains
    the Cargo JSON, the full event feature inventory, exact argv and env-safe
    declarations; freezes the lib test binary into the evidence output (original
    and frozen sha256 retained; never installed, never executed),
  * re-verifies declared candidate equality and the original inventory after
    Cargo, and publishes result.json inside the held storage admission context
    while both the evidence and target directory FDs are still held for normal result publication. The CLI's outer evidence hold
    encloses failure publication after target cleanup; target-close errors do not
    imply target descriptors remain open.

Material limits: compile-only; selected_tests_executed is 0 and
COMPILE_ONLY_WORKSPACE_CANDIDATES_OBSERVED counts prebound workspace candidate
inputs, never PASS tests; effective dependency, registry, generated-input and
toolchain qualification is NOT performed and the full effective input set remains
unknown; no cd01-paired-libtest admission record is created (the full compiler
graph and later independent qualification remain required); held FDs prevent only
ordinary unmount - this is no sandbox against privileged/lazy detach or same-UID
filesystem mutation; the numeric child result is cached in resident memory before
any fallible publication and every failure.json (or stderr fallback) preserves
native_outcome, all cleanup causes and the cached numeric results, and source
stability is re-observed even on a failed build; output is fresh, no overwrites,
no retries; consumers must require process exit status 0 before treating
result.json as evidence; output/tmp is retained as evidence, not deleted.
The result status is always CD01_COMPILE_ONLY_UNQUALIFIED with
full_effective_inputs_unknown=true and 0 tests; every rustc intent receipt is
pre-exec with outcome UNKNOWN and no compiler outcome is ever derived from
intent; registry prebind/postbind equality does not observe the compiler's
read window; resolved toolchain paths record observations, never toolchain
qualification; the wrapper chain records intent only and cannot observe the
primary compiler exit.

Example:
    python3 tools/verification/cd01_compile_collector.py --root /checkout \
        --output /mnt/evidence/cd01-new-leaf \
        --target /mnt/swarm-build/cd01-new-target \
        --inventory /mnt/evidence/snapshot-inventory.json \
        --rustc-intent-producer /reviewed/packet/cd01_capture_rustc_wrapper.py \
        --rustc-intent-producer-sha256 1c06c44cf57628414296c269f98667f5fa5e8a4ce33fc7e423a64d40527a3932 \
        --root-guard /checkout/tools/verification/cargo_space_guard.py \
        --root-guard-sha256 09b7d45c78c64282dee6265be2d26cea6066505963465e2a50a856f1fe8d5916
"""

import argparse
import contextlib
import hashlib
import importlib.util
import json
import os
import stat
import sys
sys.dont_write_bytecode = True
import tarfile
import tempfile
import tomllib
from pathlib import Path, PurePosixPath

if sys.version_info < (3, 11):
    raise RuntimeError('cd01 collector requires Python 3.11 or later')

MIN_FREE_BYTES = 20 * 1024 * 1024 * 1024
SWARM_TARGET_UUID = '144f255c-3c71-43aa-9ae6-c46a6e5b8a9f'
CARGO_PACKAGE = 'swarm-engine'
SELECTED_ARTIFACT_NAME = 'swarm_engine'
FIXTURE_FEATURES = ('test-fixtures', 'test-util')
AMBIENT_OVERRIDE_NAMES = (
    'RUSTFLAGS',
    'CARGO_ENCODED_RUSTFLAGS',
    'RUSTC_WRAPPER',
    'RUSTC_WORKSPACE_WRAPPER',
    'CARGO_BUILD_TARGET',
    'RUSTUP_TOOLCHAIN',
    'CARGO_HOME',
)
REGISTRATION_ADDITIVE_FILES = ('.cargo/external-targets.json', '.cargo/external-targets.lock')
HELPER_DIRECTORY = 'tools/verification'
HELPER_FILES = ('verify_native_profile.py', 'build_storage.py', 'native_build_outcome.py')
HELPER_DIGESTS = {
    'verify_native_profile.py': 'ab99a496c3785b2adad565da41ac0015a7927d43bf8f69525cffb288a9f732c9',
    'build_storage.py': '61f585756e404803dc3073c6ff1c294ca89a4216b08f635c460f9c44af0da6d8',
    'native_build_outcome.py': '27b2bf5ebc57647876c00a338685cde71eaacea89d4bf1b55894fa33da5722ba',
}
HEX_DIGITS = frozenset('0123456789abcdef')
INTENT_SCHEMA = 'cd01.rustc_intent.v1'
INTENT_RECORD_FIELDS = frozenset(('capture', 'compiler_arguments_hex', 'compiler_path_hex', 'cwd_hex', 'delegation', 'outcome', 'outcome_note', 'pid', 'privacy', 'record_kind', 'record_location', 'schema', 'spaceguard_path_hex'))
INTENT_DIR_PREFIX = 'cd01-intent-'
CAPTURE_DIR_ENV_NAME = 'CD01_CAPTURE_DIR'
SPACEGUARD_ENV_NAME = 'CD01_SPACEGUARD_PATH'
REGISTRY_SOURCE_URL = 'registry+https://github.com/rust-lang/crates.io-index'
# Registry namespace is discovered from explicit child HOME; no host-specific index literal.
PROBE_ARGUMENT_EXACT = frozenset(('-vV', '--version'))
PROBE_ARGUMENT_PREFIX = '--print'
_LOCK_NAME_CHARS = frozenset(
    'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-')
_LOCK_VERSION_CHARS = frozenset(
    'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.+-')


def require(condition, reason):
    """Refuse explicitly with the given reason, also under python -O.

    Every collector refusal flows through here; there are no bare assertions and
    no successful empty fallbacks. Side effects: none. Limits: reason strings
    never embed environment values or credentials.
    """
    if not condition:
        raise ValueError(reason)


def digest(path):
    """Hash the complete current bytes of one file (integrity, not authenticity).

    Errors (absence, IO) propagate to the caller's refusal path. Limit: proves
    observed bytes only, never origin or authority.
    """
    with open(path, 'rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def _unique_pairs(items):
    """Reject duplicate JSON keys instead of silently replacing inventory entries.

    Used as json object_pairs_hook. Errors: duplicate keys refuse. Side effects:
    none.
    """
    result = {}
    for key, value in items:
        require(key not in result, 'cd01-inventory-duplicate-key')
        result[key] = value
    return result


def _verify_root_snapshot(root):
    """Verify the declared snapshot root before anything else runs.

    Requires an existing directory owned by the current user, not world-writable,
    with .git and .swarm absent. Errors: each violated property refuses with a
    distinct reason. Side effects: none. Limits: 'local' is approximated by
    same-host ownership plus later device-identity checks against the held
    volumes; no remote-filesystem detection is attempted, and the truncated
    R1a6a4785.../plus17reviewed review label is declared external context that is
    recorded but not re-verified here.
    """
    require(root.is_dir(), 'cd01-root-not-directory')
    info = root.stat()
    require(info.st_uid == os.getuid(), 'cd01-root-not-owned')
    require(not info.st_mode & 0o002, 'cd01-root-world-writable')
    for name in ('.git', '.swarm'):
        require(not os.path.lexists(root / name), 'cd01-root-' + name + '-present')


def _check_paths(root, output, target, inventory_path):
    """Validate explicit CLI paths before any filesystem mutation.

    Requires a canonical absolute output and target, a NEW (non-existing) output
    leaf under an existing canonical evidence parent, a NEW non-existing external
    target outside the snapshot, no output/target containment overlap, and an
    existing snapshot manifest and inventory file. Errors: each violation refuses
    with the offending role. Side effects: none (pure stat checks). Limits: the
    'already mounted' property of the evidence parent is declared by the operator
    and later enforced only by holding its directory FD with a stable identity.
    """
    require(inventory_path.is_file(), 'cd01-inventory-absent')
    for label, path in (('output', output), ('target', target)):
        require(path.is_absolute() and str(path) == os.path.normpath(str(path))
                and not str(path).startswith('//'), 'cd01-path-not-canonical:' + label)
    require(not os.path.lexists(output), 'cd01-output-not-new')
    require(not os.path.lexists(target), 'cd01-target-not-new')
    evidence_parent = output.parent
    require(evidence_parent == evidence_parent.resolve(), 'cd01-evidence-parent-symlink')
    require(evidence_parent.is_dir(), 'cd01-evidence-parent-absent')
    require(target.resolve() == target, 'cd01-target-path-symlink')
    require(not output.resolve().is_relative_to(root), 'cd01-output-inside-root')
    require(not target.is_relative_to(root), 'cd01-target-inside-root')
    require(not output.is_relative_to(target), 'cd01-output-inside-target')
    require(not target.is_relative_to(output), 'cd01-target-inside-output')
    require((root / 'rust' / 'Cargo.toml').is_file(), 'cd01-manifest-absent')


def _path_identity(path):
    """Return (device, inode) of an existing path; a path string alone is not identity."""
    info = os.stat(path)
    return (info.st_dev, info.st_ino)


def _fd_identity(fd):
    """Return (device, inode) of an open descriptor; identity survives renames."""
    info = os.fstat(fd)
    return (info.st_dev, info.st_ino)


@contextlib.contextmanager
def _held_directory(path):
    """Open and hold one directory FD for the whole evidence lifetime.

    The FD keeps the evidence volume busy against ordinary unmount while the
    child runs and the final proof is published. Errors: open failures propagate;
    a close failure with no in-flight exception re-raises, and a close failure
    during an in-flight failure is grouped with it so no cleanup cause is lost.
    Limits: ordinary-unmount hold only; no protection against privileged/lazy
    detach or same-UID mutation (no sandbox).
    """
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        yield fd
    finally:
        primary = sys.exception()
        try:
            os.close(fd)
        except BaseException as cleanup:
            if primary is None:
                raise
            raise BaseExceptionGroup('cd01 evidence hold and close failed',
                                     [primary, cleanup]) from None


def _free_bytes(path):
    """Observe bytes available to the current unprivileged user on path's volume.

    Uses statvfs (f_bavail * f_frsize). Errors propagate. Limit: an observation,
    not a reservation; concurrent writers may consume space afterwards and no
    retry or pause ever follows an ENOSPC.
    """
    status = os.statvfs(path)
    return status.f_bavail * status.f_frsize


def _checked_free_bytes(observations):
    """Require the unchanged 20 GiB guard on every observed volume.

    The guard constant is identical before and after Cargo and is never lowered.
    Returns observed byte counts for the evidence record. Errors: any volume
    below the guard refuses ('cd01-insufficient-free-bytes'); no retry, no pause,
    no ENOSPC recovery is attempted.
    """
    observed = {}
    for label, path in observations.items():
        free = _free_bytes(path)
        require(free >= MIN_FREE_BYTES, 'cd01-insufficient-free-bytes:' + label + ':' + str(free))
        observed[label] = free
    return observed


def _free_observation_paths(root, target, out):
    """Return the guarded volumes (host checkout, cargo target, evidence) that exist."""
    observations = {'host-checkout-root': root}
    if os.path.lexists(target):
        observations['cargo-target'] = target
    if out is not None and os.path.lexists(out):
        observations['evidence-output'] = out
    return observations


def _load_profile_module(root, out):
    """Load verify_native_profile.py from the selected snapshot by explicit importlib.

    All three supplied helper modules are byte-pinned against the review-packet
    digests before and after import (the helper co-loads the co-located
    build_storage.py and native_build_outcome.py). Helper copies and the collector
    source are published into the evidence output for replay. Errors: missing or
    mismatched helper bytes, invalid spec, import failure or post-import byte
    drift refuse. Side effects: writes helper-*.py and collector-source.py into
    the output. Limits: pinning proves executed bytes match the supplied packet,
    nothing more.
    """
    base = root / HELPER_DIRECTORY
    digests = {}
    for filename in HELPER_FILES:
        path = base / filename
        require(path.is_file(), 'cd01-helper-absent:' + filename)
        observed = digest(path)
        require(observed == HELPER_DIGESTS[filename], 'cd01-helper-bytes-mismatch:' + filename)
        digests[filename] = observed
    spec = importlib.util.spec_from_file_location('cd01_verify_native_profile',
                                                  base / 'verify_native_profile.py')
    require(spec is not None and spec.loader is not None, 'cd01-profile-module-spec-invalid')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    for filename in HELPER_FILES:
        require(digest(base / filename) == digests[filename], 'cd01-helper-bytes-drifted:' + filename)
    for filename in HELPER_FILES:
        module.publish(out / ('helper-' + filename), (base / filename).read_bytes())
    require('__file__' in globals(), 'cd01-collector-source-path-unknown')
    collector_bytes = Path(__file__).read_bytes()
    collector_sha = hashlib.sha256(collector_bytes).hexdigest()
    module.publish(out / 'collector-source.py', collector_bytes)
    return module, digests, collector_sha


def _verify_inventory(root, inventory_path, profile):
    """Verify every original snapshot inventory entry against current root bytes.

    Regular entries must be real non-symlink regular files whose complete bytes
    hash to the recorded sha256; symlink entries must be symlinks whose exact link
    text matches. Errors: unreadable/invalid inventory, duplicate keys, unsafe
    relative paths, and missing or changed entries all refuse. Side effects:
    read-only. Limits: equality is claimed ONLY over indexed entries; the additive
    registration files are recorded separately and no whole-tree inventory
    equality is claimed after arbitrary writes.
    """
    raw = inventory_path.read_bytes()
    try:
        document = json.loads(raw.decode('utf-8'), object_pairs_hook=_unique_pairs)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError('cd01-inventory-unreadable') from error
    require(isinstance(document, dict), 'cd01-inventory-not-object')
    require(len(document) > 0, 'cd01-inventory-empty')
    regular = 0
    symlinks = 0
    for relative, value in sorted(document.items()):
        require(isinstance(relative, str) and relative, 'cd01-inventory-key-invalid')
        posix = PurePosixPath(relative)
        require(not posix.is_absolute()
                and all(part not in ('', '..', '.') for part in posix.parts),
                'cd01-inventory-path-unsafe:' + relative)
        full = root / Path(*posix.parts)
        try:
            info = os.lstat(full)
        except OSError as error:
            raise ValueError('cd01-inventory-entry-absent:' + relative) from error
        if isinstance(value, str):
            require(len(value) == 64 and set(value) <= HEX_DIGITS,
                    'cd01-inventory-digest-invalid:' + relative)
            require(stat.S_ISREG(info.st_mode), 'cd01-inventory-entry-not-regular:' + relative)
            require(digest(full) == value, 'cd01-inventory-bytes-changed:' + relative)
            regular += 1
        elif isinstance(value, dict):
            require(set(value) == {'symlink'} and isinstance(value['symlink'], str)
                    and value['symlink'], 'cd01-inventory-symlink-entry-invalid:' + relative)
            require(stat.S_ISLNK(info.st_mode), 'cd01-inventory-entry-not-symlink:' + relative)
            require(os.readlink(full) == value['symlink'],
                    'cd01-inventory-symlink-changed:' + relative)
            symlinks += 1
        else:
            raise ValueError('cd01-inventory-entry-type-invalid:' + relative)
    return dict(entries=len(document), regular=regular, symlinks=symlinks,
                inventory_sha256=digest(inventory_path),
                scope='indexed-entries-only; additive registration files recorded '
                      'separately; no whole-tree equality claimed')


def _verify_tar_members(path, manifest):
    """Reread a candidate archive and require the exact member set and byte hashes.

    Errors: member set mismatch, absent members or byte mismatch refuse. Side
    effects: read-only; nothing is ever extracted. Limits: integrity check only.
    """
    with tarfile.open(path, 'r:gz') as archive:
        names = archive.getnames()
        require(len(names) == len(manifest) and set(names) == set(manifest),
                'cd01-archive-members:' + path.name)
        for name, sha in manifest.items():
            member = archive.extractfile(name)
            require(member is not None, 'cd01-archive-member-absent:' + name)
            require(hashlib.sha256(member.read()).hexdigest() == sha,
                    'cd01-archive-member-bytes:' + name)


def _archive_declared(profile, root, out, manifest):
    """Archive the declared workspace candidates BEFORE Cargo and reread the archive.

    Builds a temporary tar.gz of every prebound candidate, verifies its member
    set and hashes, publishes it atomically, then rereads and re-verifies the
    published archive. Errors: a candidate that changed before archiving, is not
    a regular file, or archive/member/hash mismatches refuse. Side effects:
    writes declared-candidates.tar.gz and declared-candidates.json into the
    output. Limits: declared candidates are not proof of the effective compile
    set; no effective-dependency claim is made.
    """
    descriptor, temporary = tempfile.mkstemp(dir=out, prefix='.declared-candidates-')
    os.close(descriptor)
    temporary = Path(temporary)
    try:
        with tarfile.open(temporary, 'w:gz') as archive:
            for name, sha in sorted(manifest.items()):
                require(stat.S_ISREG(os.lstat(root / name).st_mode),
                        'cd01-candidate-not-regular:' + name)
                require(profile.digest(root / name) == sha,
                        'cd01-candidate-changed-before-archive:' + name)
                archive.add(root / name, arcname=name, recursive=False)
        _verify_tar_members(temporary, manifest)
        payload = temporary.read_bytes()
    finally:
        primary = sys.exception()
        try:
            temporary.unlink(missing_ok=True)
        except BaseException as cleanup:
            if primary is not None:
                raise BaseExceptionGroup('archive and temporary cleanup failed', [primary, cleanup])
            raise
    published = out / 'declared-candidates.tar.gz'
    profile.publish(published, payload)
    _verify_tar_members(published, manifest)
    proof = dict(inputs=len(manifest), sha256=profile.digest(published),
                 members_verified=True, reread_published=True, timing='before-cargo')
    profile.publish(out / 'declared-candidates.json', json.dumps(proof, indent=2) + '\n')
    return proof


def _child_environment(target, tmpdir, capture_dir, guard_path, wrapper_path):
    """Build the explicit inherited-name allowlist and declared wrapper instrumentation.

    Only PATH and HOME are inherited (both required); CARGO_TARGET_DIR (admitted
    target), CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0,
    CARGO_PROFILE_TEST_DEBUG=0, TMPDIR (new evidence-output tmp), the two explicit
    producer config variables CD01_CAPTURE_DIR (our owned 0700 capture directory)
    and CD01_SPACEGUARD_PATH (the pinned root guard) are added, and RUSTC_WRAPPER
    is constructed by this collector pointing at the staged reviewed producer
    copy. Inherited Rust flags, wrappers, toolchain and target overrides are
    refused FIRST; only after that refusal is RUSTC_WRAPPER set — a declared
    instrumentation override of the workspace [build] rustc-wrapper, recorded as
    such, with the cargo argv unchanged and no claim of identical rustc argv or
    codegen. No CC/CFLAGS variable and no PATH shadowing is introduced in this
    stage. Returns (env, safe declaration record). Errors: missing PATH/HOME or
    any present ambient override refuses. Limits: no other environment value is
    copied into the child or log; this is no generic argv/descendant secrecy guarantee;
    workspace .cargo/config discovery otherwise remains Cargo's own, untouched.
    """
    present = [name for name in AMBIENT_OVERRIDE_NAMES if name in os.environ]
    require(not present, 'cd01-ambient-compiler-override:' + ','.join(present))
    passthrough = {}
    for name in ('PATH', 'HOME'):
        value = os.environ.get(name)
        require(isinstance(value, str) and value != '', 'cd01-child-environment-missing:' + name)
        passthrough[name] = value
    explicit = {'CARGO_TARGET_DIR': str(target), 'CARGO_INCREMENTAL': '0',
                'CARGO_PROFILE_DEV_DEBUG': '0', 'CARGO_PROFILE_TEST_DEBUG': '0',
                'TMPDIR': str(tmpdir),
                CAPTURE_DIR_ENV_NAME: str(capture_dir),
                SPACEGUARD_ENV_NAME: str(guard_path),
                'RUSTC_WRAPPER': str(wrapper_path)}
    env = {**passthrough, **explicit}
    declaration = dict(passthrough=passthrough, explicit=explicit, allowlist_only=True,
                       inherited_names_excluded=len([n for n in os.environ if n not in env]),
                       credential_policy='child receives only the listed names; '
                                         'no other environ value is copied or logged',
                       rustc_wrapper_declared_override=dict(
                           constructed_after_ambient_refusal=True,
                           overrides_workspace_config_rustc_wrapper=True,
                           cargo_argv_unchanged=True,
                           rustc_argv_or_codegen_identity_claim='none',
                           no_cc_cflags_or_path_shadow=True))
    return env, declaration


def _cargo_argv(root):
    """Return the single permitted cargo argv for this collector.

    cargo test with an absolute --manifest-path, -p swarm-engine, --lib --no-run
    (compile only, no test body), --locked --offline and JSON messages; default
    features are kept (no feature flags at all). Side effects: none.
    """
    return ['cargo', 'test', '--manifest-path', str(root / 'rust' / 'Cargo.toml'),
            '-p', CARGO_PACKAGE, '--lib', '--no-run', '--locked', '--offline',
            '--message-format=json']


def _cached_publisher(profile, state):
    """Cache helper-produced numeric outcomes before fallible disk publication.

    Retain all named phases; publication failure does not undo process completion.
    Only the trusted shared helper's result schema is consumed here.
    """
    def publish(path, data):
        if path.name.endswith('-result.json'):
            value = json.loads(data)
            names = {'rustc-vv-result.json': 'rustc_vv',
                     'cargo-version-result.json': 'cargo_version',
                     'cargo-result.json': 'cargo_test_compile'}
            if path.name in names:
                state['returncodes'][names[path.name]] = value['returncode']
                state.setdefault('resident_process_results', {})[path.name] = value
        profile.publish(path, data)
    return publish


def _native_outcomes(error):
    """Collect native outcome carriers across groups, causes and contexts.

    Cycle suppression affects traversal only; phase observations are not merged
    or overwritten. This exposes known results, not successful publication.
    """
    result, seen = [], set()
    def visit(value):
        if value is None or id(value) in seen:
            return
        seen.add(id(value))
        if hasattr(value, 'native_outcome'):
            result.append(value.native_outcome)
        for child in getattr(value, 'exceptions', ()):
            visit(child)
        visit(value.__cause__)
        visit(value.__context__)
    visit(error)
    return result


def _error_tree(error, seen=None):
    """Preserve every typed primary/cleanup cause in terminal diagnostics.

    Cycle suppression avoids recursive groups; outcomes remain separately listed.
    Only collector/helper errors (never credentials) belong in this diagnostic.
    A collector_failure carrier preserves cached process facts after a nonnative
    postcheck plus diagnostic failure; it is not fabricated native_outcome data.
    """
    seen = set() if seen is None else seen
    if id(error) in seen:
        return {'type': type(error).__name__, 'cycle': True}
    seen.add(id(error))
    value = {'type': type(error).__name__, 'message': str(error)}
    if isinstance(getattr(error, 'collector_failure', None), dict):
        value['collector_failure'] = error.collector_failure
    if isinstance(error, OSError):
        value['errno'] = error.errno
    if getattr(error, 'exceptions', None):
        value['exceptions'] = [_error_tree(child, seen) for child in error.exceptions]
    if error.__cause__ is not None:
        value['cause'] = _error_tree(error.__cause__, seen)
    elif error.__context__ is not None and not error.__suppress_context__:
        value['context'] = _error_tree(error.__context__, seen)
    return value


def _registered_evidence_mount(storage, root, target):
    """Consume the existing validated external-target registration before writes.

    Read-only: the pinned storage helper validates the owned policy and returns
    its byte hash. Require the exact selected target and the already-authorized
    SWARM UUID/ext4 identity; return its persisted mount path and provenance.
    Missing, foreign or unreadable policy refuses without registering/fallback.
    This selects configuration only, not proof of a mounted device or capacity.
    """
    policy, policy_sha = storage.load_policy(root)
    matches = [e for e in policy['targets'] if e['path'] == str(target)]
    require(len(matches) == 1, 'cd01-target-registration-missing-or-ambiguous')
    entry = matches[0]
    require(entry['uuid'] == SWARM_TARGET_UUID and entry['fstype'] == 'ext4',
            'cd01-registered-evidence-identity')
    return Path(entry['mountpoint']), dict(policy_sha256=policy_sha, entry=dict(entry))


def _preflight_evidence_volume(args):
    """Verify the selected evidence/target mount before the first evidence write.

    Uses the pinned shared mount observer without creating a directory or policy.
    Returns mount facts for the outer CLI volume hold. Refusal creates no leaf.
    """
    root = args.root.resolve()
    path = root / HELPER_DIRECTORY / 'build_storage.py'
    require(digest(path) == HELPER_DIGESTS['build_storage.py'], 'cd01-storage-helper-pin')
    spec = importlib.util.spec_from_file_location('cd01_preflight_storage', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    mount, registration = _registered_evidence_mount(module, root, args.target)
    for path in (args.output, args.target):
        require(path.is_absolute() and path == path.resolve() and path.is_relative_to(mount),
                'cd01-output-or-target-outside-admitted-volume')
    facts = module.observe_mount(mount)
    require(facts['uuid'] == SWARM_TARGET_UUID and facts['fstype'] == 'ext4',
            'cd01-evidence-volume-identity')
    _checked_free_bytes({'host-checkout-root': root, 'target-volume': mount,
                         'evidence-parent': args.output.parent})
    return module, facts


def _observe_toolchain(profile, out, admission, env, root, state):
    """Run read-only rustc -vV and cargo --version under the shared outcome helper.

    Distinct receipt names ('rustc-vv', 'cargo-version') keep them separate from
    the single cargo test invocation; they are observations only and are never
    counted as tests. Each numeric returncode is cached into resident memory
    immediately. Errors: nonzero exit or custody failure refuses via the helper
    (no fallback, no retry). Side effects: writes <name>.stdout/.stderr/
    -result.json/-admission-after.json into the output.
    """
    observations = {}
    for name, argv in (('rustc-vv', ['rustc', '-vV']), ('cargo-version', ['cargo', '--version'])):
        result = profile.outcomes.execute(argv, out=out, name=name, admission=admission,
                                          publish=_cached_publisher(profile, state), digest=profile.digest,
                                          failure_code='cd01-toolchain-observation-failed:' + name,
                                          cwd=str(root), env=env)
        state['returncodes'][name.replace('-', '_')] = result.returncode
        require(result.returncode == 0, 'cd01-toolchain-observation-failed:' + name)
        observations[name] = dict(argv=argv, returncode=result.returncode, counted_as_test=False)
    state['facts']['toolchain_observed'] = True
    state['progress'].append('toolchain-observed')
    return observations


def _parse_cargo_events(out):
    """Parse the retained cargo stdout (the exact Cargo JSON stream) into events.

    Blank lines are ignored; any malformed JSON line refuses. Errors: unreadable
    or malformed channels refuse. Side effects: reads out/cargo.stdout only.
    Limits: parsing proves nothing about toolchain behavior; the raw retained
    file remains the record.
    """
    text = (out / 'cargo.stdout').read_text(encoding='utf-8')
    events = []
    for index, line in enumerate(text.splitlines()):
        if not line.strip():
            continue
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError as error:
            raise ValueError('cd01-cargo-event-invalid:' + str(index)) from error
    require(bool(events), 'cd01-cargo-events-empty')
    return events


def _select_libtest(events, state):
    """Enforce fixture-feature absence on every event and select the lib test artifact.

    Requires a successful build-finished event, at least one compiler-artifact,
    no fixture features ('test-fixtures'/'test-util') in ANY event carrying a
    feature list, and exactly one swarm_engine compiler-artifact whose target
    kind includes 'lib', whose profile.test is true, whose features are exactly
    [] and whose executable exists as a file. Returns (event, executable_path,
    feature_inventory, artifact_count). Errors: each violated property refuses
    with its reason and count.
    """
    require(any(event.get('reason') == 'build-finished' and event.get('success') is True
                for event in events), 'cd01-build-finished-missing')
    artifacts = [event for event in events if event.get('reason') == 'compiler-artifact']
    require(bool(artifacts), 'cd01-no-compiler-artifacts')
    for index, event in enumerate(events):
        features = event.get('features')
        if isinstance(features, list):
            require(not (set(features) & set(FIXTURE_FEATURES)),
                    'cd01-fixture-feature-on-event:' + str(index))
    feature_inventory = [dict(package=event.get('package_id'),
                              target=(event.get('target') or {}).get('name'),
                              kind=(event.get('target') or {}).get('kind'),
                              features=event.get('features'),
                              profile_test=(event.get('profile') or {}).get('test'))
                         for event in artifacts]
    selected = [event for event in artifacts
                if (event.get('target') or {}).get('name') == SELECTED_ARTIFACT_NAME
                and 'lib' in ((event.get('target') or {}).get('kind') or [])
                and (event.get('profile') or {}).get('test') is True]
    require(len(selected) == 1, 'cd01-libtest-selection-count:' + str(len(selected)))
    event = selected[0]
    require(event.get('features') == [], 'cd01-libtest-features-not-empty')
    executable = event.get('executable')
    require(isinstance(executable, str) and bool(executable), 'cd01-libtest-executable-missing')
    path = Path(executable)
    require(path.is_file(), 'cd01-libtest-executable-absent')
    state['facts']['build_finished_observed'] = True
    state['facts']['libtest_selected'] = True
    state['progress'].append('libtest-selected')
    return event, path, feature_inventory, len(artifacts)


def _freeze_binary(profile, out, executable, state):
    """Freeze the compiled swarm_engine lib test binary into the evidence output.

    Copies the exact executable bytes atomically into the output, re-hashes both
    the original and the frozen copy and requires equality. Errors: read, hash or
    publish failures refuse. Side effects: writes frozen-swarm_engine-libtest into
    the output. Limits: never installed, never registered, never executed; the
    compile-only record makes no runtime capability claim.
    """
    raw = executable.read_bytes()
    original_sha = hashlib.sha256(raw).hexdigest()
    frozen_path = out / 'frozen-swarm_engine-libtest'
    profile.publish(frozen_path, raw)
    frozen_path.chmod(0o700)
    frozen_sha = profile.digest(frozen_path)
    require(frozen_sha == original_sha, 'cd01-frozen-libtest-hash-mismatch')
    require(profile.digest(executable) == original_sha, 'cd01-original-libtest-changed')
    state['facts']['frozen_libtest_sha256'] = frozen_sha
    state['progress'].append('libtest-frozen')
    return dict(original_sha256=original_sha, frozen_path=str(frozen_path),
                frozen_sha256=frozen_sha)


def _registration_additives(root):
    """Record the two known additive registration files separately from the snapshot.

    Supported target registration adds .cargo/external-targets.json and its lock;
    both are recorded (presence, regular-ness, sha256 when regular) but are not
    part of the original inventory and no whole-tree inventory equality is claimed
    over them. Errors: unreadable additive bytes refuse explicitly. Side effects:
    none (read-only).
    """
    record = {}
    for relative in REGISTRATION_ADDITIVE_FILES:
        path = root / relative
        if not os.path.lexists(path):
            record[relative] = dict(present=False)
            continue
        info = os.lstat(path)
        entry = dict(present=True, regular=stat.S_ISREG(info.st_mode))
        if entry['regular']:
            entry['sha256'] = digest(path)
        record[relative] = entry
    return record


def _require_hex64(value, label):
    """Require a 64-character hex SHA-256-shaped string for an explicit pin.

    Errors: wrong type, length or non-hex characters refuse with the label.
    Side effects: none. Limits: shape validation only; no digest computed here.
    """
    require(isinstance(value, str) and len(value) == 64 and set(value) <= HEX_DIGITS,
            'cd01-sha256-invalid:' + label)


def _observed_shebang(path):
    """Record the first-line shebang bytes of a script, if any, as hex.

    Returns the raw first line hex-encoded when the file starts with '#!',
    else None. Errors: read failures propagate. Side effects: none. Limits:
    the shebang is OBSERVED ONLY; interpreter resolution through env at exec
    time is never automatically qualified here.
    """
    with open(path, 'rb') as stream:
        first = stream.readline()
    if first.startswith(b'#!'):
        return first.hex()
    return None


def _stage_rustc_intent_producer(profile, out, target, producer_path, expected_sha):
    """Validate, pin and stage the reviewed rustc INTENT producer as executable.

    Requires the explicit producer path to be an absolute non-symlink regular
    file outside the cargo target whose complete bytes hash to the explicitly
    supplied expected SHA-256; publishes an exact byte copy into the evidence
    output, marks it 0700 executable, and re-reads the copy requiring digest
    equality (readback). The observed shebang is recorded without qualifying
    the interpreter. Errors: each violated property refuses with a distinct
    reason. Side effects: writes rustc-intent-producer.py into the output.
    Limits: the pin proves the intended copy equals the expected source bytes;
    it proves neither origin, authority nor runtime behavior.
    """
    _require_hex64(expected_sha, 'rustc-intent-producer-sha256')
    require(producer_path.is_absolute(), 'cd01-producer-path-not-absolute')
    require(not producer_path.is_relative_to(target), 'cd01-producer-inside-target')
    try:
        info = os.lstat(producer_path)
    except OSError as error:
        raise ValueError('cd01-producer-unreadable') from error
    require(stat.S_ISREG(info.st_mode), 'cd01-producer-not-regular')
    observed = digest(producer_path)
    require(observed == expected_sha, 'cd01-producer-bytes-mismatch')
    copy_path = out / 'rustc-intent-producer.py'
    profile.publish(copy_path, producer_path.read_bytes())
    copy_path.chmod(0o700)
    require(os.access(copy_path, os.X_OK), 'cd01-producer-copy-not-executable')
    require(digest(copy_path) == expected_sha, 'cd01-producer-readback-mismatch')
    return dict(path=str(copy_path), source=str(producer_path), sha256=observed,
                source_mode=stat.S_IMODE(info.st_mode),
                copy_mode=stat.S_IMODE(copy_path.stat().st_mode),
                shebang_hex=_observed_shebang(copy_path), interpreter_qualified=False,
                interpreter_note='shebang recorded as observed; env shebang interpreter '
                                 'resolution is not qualified here')


def _pin_root_guard(root, guard_path, expected_sha):
    """Pin the exact root-selected admission guard in place by path and bytes.

    Requires the guard to be an absolute non-symlink regular file inside the
    verified snapshot root, executable, hashing to the explicitly supplied
    expected SHA-256. The guard file is used in place (never copied or
    rewritten) so delegation preserves its exact profile and code at the root
    path; the observed shebang is recorded without qualifying the interpreter.
    Errors: each violated property refuses distinctly. Side effects: none
    (read-only). Limits: byte pin proves observed bytes only; guard behavior
    is delegated to, never re-implemented or re-verified here.
    """
    _require_hex64(expected_sha, 'root-guard-sha256')
    require(guard_path.is_absolute(), 'cd01-guard-path-not-absolute')
    require(guard_path == root / HELPER_DIRECTORY / 'cargo_space_guard.py',
            'cd01-guard-not-exact-root-location')
    try:
        info = os.lstat(guard_path)
    except OSError as error:
        raise ValueError('cd01-guard-unreadable') from error
    require(stat.S_ISREG(info.st_mode), 'cd01-guard-not-regular')
    observed = digest(guard_path)
    require(observed == expected_sha, 'cd01-guard-bytes-mismatch')
    require(os.access(guard_path, os.X_OK), 'cd01-guard-not-executable')
    return dict(path=str(guard_path), sha256=observed, mode=stat.S_IMODE(info.st_mode),
                shebang_hex=_observed_shebang(guard_path), interpreter_qualified=False,
                interpreter_note='shebang recorded as observed; env shebang interpreter '
                                 'resolution is not qualified here')


def _archive_instruments(profile, out, producer_copy, guard_path):
    """Archive/re-read pinned instruments while retaining every owned candidate.

    Writes a private temporary tar and the published verified copy. All partial
    or complete temporary candidates remain on any failure, including verification
    and publication errors. The temporary is also retained after success; no
    cleanup/retention policy is implemented here. Errors propagate with contexts;
    this is custody of bytes, never behavioral or interpreter qualification.
    """
    manifest = {'rustc-intent-producer.py': digest(producer_copy),
                'root-guard.py': digest(guard_path)}
    descriptor, temporary = tempfile.mkstemp(dir=out, prefix='.instruments-')
    os.close(descriptor)
    temporary = Path(temporary)
    with tarfile.open(temporary, 'w:gz') as archive:
        archive.add(producer_copy, arcname='rustc-intent-producer.py', recursive=False)
        archive.add(guard_path, arcname='root-guard.py', recursive=False)
    with temporary.open('rb') as stream:
        os.fsync(stream.fileno())
    _verify_tar_members(temporary, manifest)
    payload = temporary.read_bytes()
    published = out / 'rustc-intent-instruments.tar.gz'
    profile.publish(published, payload)
    _verify_tar_members(published, manifest)
    return dict(members=manifest, sha256=profile.digest(published),
                reread_published=True, retained_temporary=str(temporary))


def _create_capture_dir(out):
    """Create the collector-owned NEW 0700 rustc-intent capture directory.

    The directory is a new leaf under the evidence output, created exclusively;
    the producer later creates one exclusive invocation directory per rustc
    invocation inside it and never cleans anything up. Errors: existing target
    path or creation failure refuses. Side effects: one mkdir. Limits: private
    mode is lease hygiene for our own records; the producer's own trust
    statement governs same-UID writers.
    """
    path = out / 'rustc-intent-capture'
    try:
        path.mkdir(mode=0o700)
    except FileExistsError as error:
        raise ValueError('cd01-capture-dir-not-new') from error
    except OSError as error:
        raise ValueError('cd01-capture-dir-create-failed') from error
    return dict(path=str(path), identity=_path_identity(path),
                mode=stat.S_IMODE(path.stat().st_mode))


def _discover_cargo_configs(root, home):
    """Observe both Cargo config names in cwd/ancestors/home without deciding precedence.

    Returns deterministic per-filename presence and byte pins, including absence
    only on FileNotFoundError. Other probe/read errors and symlinks refuse; no
    credentials are opened. Read-only under caller lease, not a merge/read-window
    proof. Cargo remains responsible for its own precedence and interpretation.
    """
    require(isinstance(home, str) and Path(home).is_absolute(), 'cd01-home-not-absolute')
    root = Path(root)
    require(root.is_absolute(), 'cd01-config-cwd-not-absolute')
    directories = [(root, 'cwd-ancestors')]
    while directories[-1][0].parent != directories[-1][0]:
        directories.append((directories[-1][0].parent, 'cwd-ancestors'))
    directories.append((Path(home), 'home'))
    entries = []
    for directory, scope in directories:
        for name in ('config', 'config.toml'):
            candidate = directory / '.cargo' / name
            entry = dict(scope=scope, directory=str(directory), path=str(candidate))
            try:
                info = os.lstat(candidate)
            except FileNotFoundError:
                entries.append(dict(entry, present=False))
                continue
            require(stat.S_ISREG(info.st_mode), 'cd01-config-not-regular')
            entries.append(dict(entry, present=True, sha256=digest(candidate), size=info.st_size))
    return entries


def _read_cargo_lock(root):
    """Hash and parse the same complete Cargo.lock bytes under the caller lease.

    Requires a real regular file and nonempty package list; I/O, UTF8/TOML or
    shape errors refuse with causes retained. No writes, extra HOME reads or
    atomic read-window claim. Returns digest and packages from one byte buffer.
    """
    path = root / 'rust' / 'Cargo.lock'
    require(stat.S_ISREG(os.lstat(path).st_mode), 'cd01-cargo-lock-not-regular')
    raw = path.read_bytes()
    document = tomllib.loads(raw.decode('utf-8'))
    packages = document.get('package')
    require(isinstance(packages, list) and bool(packages), 'cd01-cargo-lock-packages-invalid')
    return hashlib.sha256(raw).hexdigest(), packages


def _hash_regular_tree(directory, stem):
    """Hash every regular file under a real crate directory, refusing anything else.

    Recursive scandir walk that never follows symlinks: any symlinked or
    unsupported entry (fifo, socket, device) refuses with the crate stem and
    relative path. Returns {relative-path: sha256}. Errors: unreadable entries
    refuse. Side effects: read-only. Limits: digests only, never contents; a
    hostile concurrent writer is out of scope.
    """
    files = {}

    def walk(current, prefix):
        """Traverse one leased directory; mutate only the result map/list, refuse I/O."""
        with os.scandir(current) as entries:
            for entry in entries:
                relative = prefix + entry.name
                if entry.is_symlink():
                    raise ValueError('cd01-registry-tree-symlink:' + stem + ':' + relative)
                if entry.is_dir(follow_symlinks=False):
                    walk(entry.path, relative + '/')
                elif entry.is_file(follow_symlinks=False):
                    files[relative] = digest(entry.path)
                else:
                    raise ValueError('cd01-registry-tree-unsupported:' + stem + ':' + relative)

    walk(directory, '')
    return files


def _prebind_registry_sources(root, home, lock_sha, packages):
    """Hash lock-selected source trees and checksum-matching archives before Cargo.

    Discover exactly one complete matching src/cache namespace under the explicit
    child HOME. Missing, ambiguous, unsupported, symlinked or checksum-mismatched
    inputs refuse, never select a fallback. Local packages are skipped. The
    namespace is an observed layout, not proof of Cargo's runtime resolution.
    Hashes bind current bytes under the lease, not compiler reads or unpacked
    source equivalence with archive members. No files are written/extracted.
    """
    require(isinstance(home, str) and Path(home).is_absolute(), 'cd01-home-not-absolute')
    base = Path(home) / '.cargo' / 'registry'
    selected = []
    local = 0
    seen = set()
    for package in packages:
        require(isinstance(package, dict), 'cd01-cargo-lock-package-invalid')
        name, version = package.get('name'), package.get('version')
        require(isinstance(name, str) and bool(name) and set(name) <= _LOCK_NAME_CHARS,
                'cd01-lock-name-unsafe')
        require(isinstance(version, str) and bool(version) and set(version) <= _LOCK_VERSION_CHARS,
                'cd01-lock-version-unsafe')
        if package.get('source') is None:
            local += 1
            continue
        require(package['source'] == REGISTRY_SOURCE_URL, 'cd01-registry-source-unsupported')
        stem = name + '-' + version
        require(stem not in seen, 'cd01-registry-duplicate-locked-package')
        seen.add(stem)
        checksum = package.get('checksum')
        _require_hex64(checksum, 'cd01-lock-checksum')
        selected.append((stem, checksum))
    require(bool(selected), 'cd01-registry-packages-none')
    roots = []
    with os.scandir(base / 'src') as directories:
        for entry in directories:
            require(not entry.is_symlink(), 'cd01-registry-namespace-symlink')
            if not entry.is_dir(follow_symlinks=False):
                continue
            src = Path(entry.path)
            cache = base / 'cache' / entry.name
            try:
                cache_info = os.lstat(cache)
            except FileNotFoundError:
                continue
            require(stat.S_ISDIR(cache_info.st_mode), 'cd01-registry-cache-namespace-not-directory')
            complete = True
            for stem, checksum in selected:
                try:
                    src_info = os.lstat(src / stem)
                    arc_info = os.lstat(cache / (stem + '.crate'))
                except FileNotFoundError:
                    complete = False
                    break
                require(stat.S_ISDIR(src_info.st_mode) and stat.S_ISREG(arc_info.st_mode),
                        'cd01-registry-package-type-unsupported')
            if complete:
                roots.append((entry.name, src, cache))
    require(len(roots) == 1, 'cd01-registry-layout-missing-or-ambiguous')
    namespace, src_root, cache_root = roots[0]
    entries = {}
    for stem, checksum in selected:
        src_dir, archive = src_root / stem, cache_root / (stem + '.crate')
        archive_sha = digest(archive)
        require(archive_sha == checksum, 'cd01-registry-archive-checksum-mismatch')
        files = _hash_regular_tree(src_dir, stem)
        tree = hashlib.sha256()
        for relative in sorted(files):
            tree.update(os.fsencode(relative) + b'\x00' + files[relative].encode('ascii') + b'\n')
        entries[stem] = dict(source=REGISTRY_SOURCE_URL, checksum=checksum,
                             source_dir=str(src_dir), source_file_count=len(files),
                             source_files=files, source_tree_sha256=tree.hexdigest(),
                             archive=str(archive), archive_sha256=archive_sha)
    return dict(cargo_lock_sha256=lock_sha, registry_index_dirname=namespace,
                home_registry_base=str(base), packages_total=len(packages),
                packages_local_skipped=local, packages_prebound=len(entries), entries=entries,
                sidecar_note='No sidecar assumed; actual source bytes and locked archive bytes '
                             'are pinned separately, not claimed equivalent', timing='before-cargo')


def _compare_registry_prebind(root, home, prebind):
    """Re-observe every prebound registry input and Cargo.lock after Cargo.

    Requires the Cargo.lock digest and package count to be unchanged and every
    crate source tree and cache archive to re-verify to exactly the prebound
    digests (archive checksums are re-checked against the lock). Errors: any
    drift, absence or re-read failure refuses. Side effects: read-only.
    Limits: equality is over the prebound set only; this is a post-build
    re-observation, not a compiler read-window proof.
    """
    lock_sha, packages = _read_cargo_lock(root)
    require(lock_sha == prebind['cargo_lock_sha256'], 'cd01-cargo-lock-changed')
    require(len(packages) == prebind['packages_total'],
            'cd01-cargo-lock-package-count-changed')
    reobserved = _prebind_registry_sources(root, home, lock_sha, packages)
    require(reobserved['entries'] == prebind['entries'], 'cd01-registry-prebind-drift')
    return dict(ok=True, cargo_lock_sha256=lock_sha,
                packages_prebound=reobserved['packages_prebound'],
                files_reobserved=sum(entry['source_file_count']
                                     for entry in reobserved['entries'].values()),
                timing='after-cargo')


def _require_fresh_target_tree(target):
    """Require the already-registered admitted target to hold no prior outputs.

    Walks the held target directory after admission and before Cargo and
    refuses if ANY entry exists (file, symlink, directory, device): a fresh
    target must be an empty tree. No registration change and no cleanup is
    ever performed. Errors: absent directory or any pre-existing entry refuses
    with the offending relative paths. Side effects: read-only. Limits: this
    is the explicit technical no-reuse prerequisite only; an empty target is
    NOT proof that Cargo invoked zero rustc processes — actual invocations are
    observed separately as wrapper intent receipts.
    """
    require(target.is_dir(), 'cd01-target-dir-absent-after-admission')
    leftovers = []

    def walk(current, prefix):
        """Traverse one leased directory; mutate only the result map/list, refuse I/O."""
        with os.scandir(current) as entries:
            for entry in entries:
                relative = prefix + entry.name
                leftovers.append(relative)
                if entry.is_dir(follow_symlinks=False):
                    walk(entry.path, relative + '/')

    walk(target, '')
    require(not leftovers, 'cd01-target-prior-entries:' + ','.join(leftovers))
    return dict(fresh_empty=True, prior_entries=0, timing='before-cargo',
                note='technical no-reuse prerequisite; not a cargo-invocation-count proof')


def _resolve_tool_path(name, path_value, root):
    """Resolve one tool name through the child PATH and record the symlink chain.

    PATH search semantics for the child environment (an empty PATH element
    means the child cwd, i.e. the snapshot root): the first directory holding
    an executable file of that name is selected, then final-name symlinks
    are walked with identity-based cycle detection. Parent-directory resolution
    assumes the caller lease. Records those observed hops, the
    final real path, existing stat metadata and the file SHA-256. Errors:
    missing PATH value, a symlink loop or no resolvable executable refuses.
    Side effects: read-only. Limits: observation of the resolved path and
    bytes only; rustup proxy versus actual target toolchain binary stays
    UNKNOWN unless resolved; not a full toolchain qualification.
    """
    require(isinstance(path_value, str) and path_value != '', 'cd01-tool-path-unset:' + name)
    for element in path_value.split(':'):
        directory = Path(element) if element else Path(root)
        if not directory.is_absolute():
            directory = Path(root) / directory
        candidate = Path(directory) / name
        try:
            first = os.lstat(candidate)
        except (FileNotFoundError, NotADirectoryError):
            continue
        if not (stat.S_ISREG(first.st_mode) or stat.S_ISLNK(first.st_mode)):
            continue
        if not os.access(candidate, os.X_OK):
            continue
        hops = []
        identities = set()
        current = candidate
        regular = False
        while True:
            info = os.lstat(current)
            identity = (info.st_dev, info.st_ino)
            if identity in identities:
                raise ValueError('cd01-tool-symlink-loop:' + name)
            identities.add(identity)
            hops.append(str(current))
            if not stat.S_ISLNK(info.st_mode):
                regular = stat.S_ISREG(info.st_mode)
                break
            target_text = os.readlink(current)
            current = Path(target_text) if os.path.isabs(target_text) \
                else current.parent / target_text
        if not regular:
            continue
        require(os.access(current, os.X_OK), 'cd01-tool-not-executable:' + name)
        final = os.stat(current)
        return dict(name=name, hops=hops, resolved=str(current),
                    mode=stat.S_IMODE(final.st_mode), uid=final.st_uid,
                    size=final.st_size, mtime_ns=final.st_mtime_ns, sha256=digest(current),
                    proxy_vs_actual_target_binary='UNKNOWN-unless-resolved',
                    full_toolchain_proof=False)
    raise ValueError('cd01-tool-path-unresolved:' + name)


def _observe_toolchain_paths(env, root):
    """Record observed PATH tools without inferring the effective Cargo target.

    Resolves both names through the child's inherited PATH. The compiler path
    Cargo actually supplies to the wrapper is recorded per intent receipt;
    equality of path names with the resolved rustc is never claimed, and rustup
    proxy versus actual target binary stays UNKNOWN when unresolved. Errors:
    resolution refusals propagate. Side effects: read-only. Limits: recorded
    observations and file digests, not a full toolchain proof; no dynamic
    library or toolchain graph is inspected.
    """
    return dict(rustc=_resolve_tool_path('rustc', env['PATH'], root),
                cargo=_resolve_tool_path('cargo', env['PATH'], root),
                selected_target='UNKNOWN; actual rustc arguments and Cargo configuration require review',
                equal_pathname_claim='none; proxy versus actual target path stays '
                                     'UNKNOWN unless resolved',
                note='resolved paths, symlink chains, existing stat metadata and file '
                     'SHA-256 recorded; not a full toolchain proof')


def _valid_hex(value):
    """Check lowercase reversible hex, allowing the valid empty argument encoding."""
    return isinstance(value, str) and len(value) % 2 == 0 and set(value) <= HEX_DIGITS


def _parse_intent_records(capture):
    """Validate current producer-v1 records without interpreting them as execution.

    Capture identity must remain stable. Each invocation directory must contain
    regular hardlinked partial/final ASCII JSON with exactly the reviewed v1
    fields; duplicate keys, failure records, invalid hex/NUL bytes, wrong schema,
    nonpositive PID or inconsistent delegation refuse. Empty arguments/lists are
    valid. Same-PID recurrence is allowed. Raw files remain untouched; return
    hashes, exact byte fields and scoped intent classification, never outcomes.
    The caller separately binds intended guard to its selected configuration.
    """
    path = Path(capture['path'])
    require(_path_identity(path) == capture['identity'], 'cd01-capture-dir-drift')
    children = sorted(os.listdir(path))
    require(bool(children), 'cd01-capture-empty')
    records = []
    for entry in children:
        require(entry.startswith(INTENT_DIR_PREFIX), 'cd01-capture-unexpected-entry')
        pid_text, separator, nonce = entry[len(INTENT_DIR_PREFIX):].partition('-')
        require(separator and pid_text.isascii() and pid_text.isdigit()
                and len(nonce) == 32 and set(nonce) <= HEX_DIGITS,
                'cd01-capture-entry-name-invalid')
        directory = path / entry
        require(stat.S_ISDIR(os.lstat(directory).st_mode), 'cd01-capture-entry-not-directory')
        require(set(os.listdir(directory)) == {'intent.partial', 'intent.json'},
                'cd01-intent-members-incomplete-failed-or-foreign')
        partial, final = directory / 'intent.partial', directory / 'intent.json'
        a, b = os.lstat(partial), os.lstat(final)
        require(stat.S_ISREG(a.st_mode) and stat.S_ISREG(b.st_mode)
                and (a.st_dev, a.st_ino) == (b.st_dev, b.st_ino) and b.st_nlink >= 2,
                'cd01-intent-hardlink-invalid')
        raw = final.read_bytes()
        value = json.loads(raw.decode('ascii'), object_pairs_hook=_unique_pairs)
        require(isinstance(value, dict) and set(value) == INTENT_RECORD_FIELDS,
                'cd01-intent-record-fields')
        require(value['schema'] == INTENT_SCHEMA and value['record_kind'] == 'pre_exec_intent'
                and value['outcome'] == 'UNKNOWN', 'cd01-intent-record-schema')
        pid = value['pid']
        require(type(pid) is int and pid > 0 and str(pid) == pid_text, 'cd01-intent-record-pid')
        for key in ('cwd_hex', 'compiler_path_hex', 'spaceguard_path_hex'):
            require(_valid_hex(value[key]) and bool(value[key])
                    and b'\x00' not in bytes.fromhex(value[key]), 'cd01-intent-path-hex')
        arguments = value['compiler_arguments_hex']
        require(isinstance(arguments, list) and all(_valid_hex(x)
                and b'\x00' not in bytes.fromhex(x) for x in arguments), 'cd01-intent-argument-hex')
        delegation = value['delegation']
        expected_delegation = {'mode', 'executable_hex', 'argv_hex', 'argv0_note', 'environment_note'}
        require(isinstance(delegation, dict) and set(delegation) == expected_delegation,
                'cd01-intent-delegation-fields')
        require(delegation['executable_hex'] == value['spaceguard_path_hex']
                and delegation['argv_hex'] == [value['spaceguard_path_hex'], value['compiler_path_hex'], *arguments],
                'cd01-intent-delegation-inconsistent')
        require(all(isinstance(value[k], dict) for k in ('record_location', 'capture', 'privacy'))
                and isinstance(value['outcome_note'], str), 'cd01-intent-metadata-type')
        decoded = [os.fsdecode(bytes.fromhex(x)) for x in arguments]
        probe = any(x in PROBE_ARGUMENT_EXACT or x.startswith(PROBE_ARGUMENT_PREFIX) for x in decoded)
        crate = None
        if not probe:
            for index, arg in enumerate(decoded):
                if arg == '--crate-name' and index + 1 < len(decoded):
                    crate = decoded[index + 1]
                    break
                if arg.startswith('--crate-name='):
                    crate = arg.split('=', 1)[1]
                    break
        libtest = not probe and crate == SELECTED_ARTIFACT_NAME and '--test' in decoded
        kind = ('metadata-probe' if probe else 'libtest-compile-candidate' if libtest
                else 'compile-candidate' if crate is not None else 'unclassified')
        records.append(dict(record_dir=entry, pid=pid, intent_json_sha256=hashlib.sha256(raw).hexdigest(),
                            compiler_path_hex=value['compiler_path_hex'], guard_path_hex=value['spaceguard_path_hex'],
                            cwd_hex=value['cwd_hex'], compiler_arguments_hex=arguments,
                            argument_count=len(arguments), intent_class=kind, crate_name=crate,
                            has_test_flag='--test' in decoded, outcome='UNKNOWN', counted_as_test=False))
    return dict(capture_dir=str(path), records=records, failure_records=[],
                counts=dict(total=len(records), metadata_probes=sum(x['intent_class']=='metadata-probe' for x in records),
                            compile_candidates=sum(x['intent_class']=='compile-candidate' for x in records),
                            libtest_candidates=sum(x['intent_class']=='libtest-compile-candidate' for x in records),
                            unclassified=sum(x['intent_class']=='unclassified' for x in records)),
                note='Pre-exec intent only, not compiler execution/outcome, source/output binding or full graph')


def _match_libtest_intent(intent, state, event):
    """Observe a positive name/test intent for the separately selected artifact.

    Requires the actual Cargo event target name to equal the declared selected
    libtest name, and a matching intent candidate. It proves only name/test flag
    correspondence; source/output and per-compiler outcome binding stay UNKNOWN.
    Mutates only in-memory facts/progress; missing candidates refuse, no writes.
    """
    name = (event.get('target') or {}).get('name')
    require(name == SELECTED_ARTIFACT_NAME, 'cd01-selected-artifact-name')
    selected = [r for r in intent['records'] if r['intent_class']=='libtest-compile-candidate'
                and r['crate_name']==name]
    require(bool(selected), 'cd01-libtest-intent-absent')
    state['facts']['libtest_intent_name_candidate_present'] = True
    state['progress'].append('libtest-intent-name-candidate-observed')
    return dict(candidate_records=len(selected), selected_artifact_name=name,
                compiler_paths_hex=sorted({r['compiler_path_hex'] for r in selected}),
                binding_status='NAME_AND_TEST_INTENT_ONLY_SOURCE_AND_OUTPUT_BINDING_UNKNOWN',
                note='Cargo primary and frozen image remain separate; no compiler outcome from intent')


def _publish_failure(out, state, error, profile, root, target):
    """Publish the final failure record without losing any known outcome.

    Preserves the helper's native_outcome and every cleanup cause (grouped
    exceptions are described recursively), the numeric child results cached in
    resident memory, all observed progress facts, and best-effort source
    stability observations (declared candidates and the original inventory) taken
    even on a failed build. If failure.json itself cannot be written, the full
    payload is dumped to stderr as the fallback. If that final channel fails,
    raise a group retaining original/reporting causes and the complete payload
    in collector_failure; the terminal _error_tree consumer exposes its cached
    numeric facts even for nonnative postcheck errors. This in-memory carrier
    is not a durable delivery guarantee. Failed free-space observations keep
    labeled typed causes next to
    their unknown numeric values. No retry or claim of forced durable receipt.
    """
    if profile is not None:
        describe = profile.outcomes.describe
        document = profile.outcomes.failure_document(error)
    else:
        def describe(value, seen=None):
            return {'type': type(value).__name__, 'message': str(value)}
        document = dict(type=type(error).__name__, reason=str(error), causes=describe(error))
    payload = dict(status='FAIL', schema='cd01-compile-only-collector/1',
                   selected_tests_executed=0, error=document,
                   cached_numeric_results=dict(state['returncodes']),
                   resident_process_results=state.get('resident_process_results', {}),
                   all_native_outcomes=_native_outcomes(error),
                   observed_facts=dict(state['facts']), progress=list(state['progress']),
                   output_leaf=state.get('out'), free_bytes_guard=MIN_FREE_BYTES,
                   scope='CD01 compile-only run unqualified; no test executed; no retry; '
                         'inspect retained channels; failure record supersedes any earlier file')
    if profile is not None and state.get('before_candidates') is not None:
        try:
            after = profile.candidates(root)
            before = state['before_candidates']
            added = sorted(set(after) - set(before))
            removed = sorted(set(before) - set(after))
            changed = sorted(name for name in set(after) & set(before) if after[name] != before[name])
            payload['source_candidate_postcheck'] = dict(
                ok=not (added or removed or changed), added=added, removed=removed, changed=changed)
        except BaseException as post_error:
            payload['source_candidate_postcheck'] = dict(ok=None, error=describe(post_error))
    if profile is not None and state.get('inventory_path') is not None:
        try:
            _verify_inventory(root, Path(state['inventory_path']), profile)
            payload['inventory_postcheck'] = dict(ok=True)
        except BaseException as post_error:
            payload['inventory_postcheck'] = dict(ok=False, error=describe(post_error))
    free = {}
    free_errors = {}
    for label, path in (('host-checkout-root', root),
                        ('cargo-target', target if os.path.lexists(target) else None),
                        ('evidence-output', out)):
        if path is None:
            free[label] = None
            continue
        try:
            free[label] = _free_bytes(path)
        except BaseException as observation_error:
            free[label] = None
            free_errors[label] = _error_tree(observation_error)
    payload['free_bytes_after'] = free
    payload['free_bytes_after_errors'] = free_errors
    published = False
    publication_error = None
    if out is not None:
        text = json.dumps(payload, indent=2) + '\n'
        try:
            if profile is not None:
                profile.publish(out / 'failure.json', text)
            else:
                with open(out / 'failure.json', 'xb') as stream:
                    stream.write(text.encode('utf-8'))
                    stream.flush()
                    os.fsync(stream.fileno())
            published = True
        except BaseException as secondary:
            publication_error = secondary
            payload['failure_publication'] = dict(ok=False, error=describe(secondary))
    if not published:
        payload.setdefault('failure_publication', dict(ok=False, reason='output-unavailable'))
        try:
            sys.stderr.write(json.dumps(payload, indent=2) + '\n')
            sys.stderr.flush()
        except BaseException as reporting_error:
            payload['stderr_publication'] = dict(ok=False, error=_error_tree(reporting_error))
            failure = BaseExceptionGroup(
                'Collector failure and terminal diagnostic publication failure',
                [error, *([publication_error] if publication_error is not None else []),
                 reporting_error])
            failure.collector_failure = payload
            raise failure from None


def run(args):
    """Execute the one-shot CD01 compile-only collection end to end.

    Side effects: creates the NEW evidence output leaf, its tmp/ directory and
    its own 0700 rustc-intent capture directory, writes evidence files, stages
    and pins the reviewed intent producer plus the pinned root guard, prebinds
    Cargo.lock, the lock-selected registry sources/archives and the config
    discovery hierarchy, makes three primary tool/build calls (rustc -vV,
    cargo --version, one cargo test compile) under custody of the supplied
    outcome helper — the cargo child delegates every rustc invocation through
    the intended RUSTC_WRAPPER chain; runtime binding remains scoped —
    then parses and matches the producer's intent receipts; shared mount
    observations additionally invoke findmnt. Holds
    the evidence directory FD plus the admitted target
    FDs through child execution and final proof publication. Errors: any refusal,
    child failure, postcheck failure, invalid intent record or publication
    failure publishes failure.json
    (or the stderr fallback) and re-raises under the CLI outer evidence hold; nothing is retried and no timeout,
    pause or watchdog exists. Limits: see the module docstring; the returned
    proof carries status CD01_COMPILE_ONLY_UNQUALIFIED and is evidence only
    when the process exits 0.
    """
    root = args.root.resolve()
    output = Path(args.output)
    target = Path(args.target)
    inventory_path = Path(args.inventory)
    state = {'returncodes': {'rustc_vv': None, 'cargo_version': None, 'cargo_test_compile': None},
             'progress': [], 'facts': {}, 'inventory_path': str(inventory_path),
             'before_candidates': None, 'out': None}
    profile = None
    out = None
    try:
        _verify_root_snapshot(root)
        state['progress'].append('root-snapshot-verified')
        _check_paths(root, output, target, inventory_path)
        evidence_parent = output.parent
        with _held_directory(evidence_parent) as evidence_fd:
            evidence_identity = _fd_identity(evidence_fd)
            require(_path_identity(evidence_parent) == evidence_identity,
                    'cd01-evidence-parent-changed')
            try:
                output.mkdir(mode=0o700)
            except FileExistsError as error:
                raise ValueError('cd01-output-not-new') from error
            except OSError as error:
                raise ValueError('cd01-output-create-failed') from error
            out = output
            state['out'] = str(out)
            state['facts']['output_created_new'] = True
            out_identity = _path_identity(out)
            require(out_identity[0] == evidence_identity[0], 'cd01-output-device-mismatch')
            tmpdir = out / 'tmp'
            try:
                tmpdir.mkdir(mode=0o700)
            except OSError as error:
                raise ValueError('cd01-tmp-create-failed') from error
            profile, helper_digests, collector_sha = _load_profile_module(root, out)
            state['progress'].append('helpers-loaded-and-pinned')
            inventory_summary = _verify_inventory(root, inventory_path, profile)
            state['facts']['inventory_verified'] = True
            state['progress'].append('inventory-verified')
            profile.publish(out / 'inventory-verification.json',
                            json.dumps(inventory_summary, indent=2) + '\n')
            with profile.storage.admitted(root, target) as admission:
                admission_proof = admission.check()
                require(admission_proof.get('mode') == 'registered-external',
                        'cd01-target-not-registered-external')
                mount = admission_proof.get('mount') or {}
                require(mount.get('uuid') == SWARM_TARGET_UUID, 'cd01-target-uuid-mismatch')
                require(mount.get('fstype') == 'ext4', 'cd01-target-fstype-not-ext4')
                require('rw' in str(mount.get('options', '')).split(','),
                        'cd01-target-mount-not-rw')
                state['progress'].append('target-admitted')
                profile.publish(out / 'storage-admission.json',
                                json.dumps(admission_proof, indent=2) + '\n')
                free_pre = _checked_free_bytes(_free_observation_paths(root, target, out))
                state['facts']['free_bytes_pre'] = dict(free_pre)
                before = profile.candidates(root)
                state['before_candidates'] = dict(before)
                profile.publish(out / 'before-candidates.json',
                                json.dumps(before, indent=2) + '\n')
                config_inputs = profile.config_manifest(before)
                state['progress'].append('candidates-bound')
                archive_proof = _archive_declared(profile, root, out, before)
                state['facts']['declared_candidates_archive_sha256'] = archive_proof['sha256']
                state['progress'].append('candidates-archived')
                target_freshness = _require_fresh_target_tree(target)
                state['facts']['target_fresh_empty'] = True
                state['progress'].append('target-fresh-verified')
                home = os.environ.get('HOME')
                require(isinstance(home, str) and home != '', 'cd01-home-unset')
                lock_sha, lock_packages = _read_cargo_lock(root)
                registry_prebind = _prebind_registry_sources(root, home, lock_sha, lock_packages)
                profile.publish(out / 'registry-prebind.json',
                                json.dumps(registry_prebind, indent=2) + '\n')
                state['facts']['registry_packages_prebound'] = registry_prebind['packages_prebound']
                state['progress'].append('registry-prebound')
                config_pins = _discover_cargo_configs(root, home)
                state['progress'].append('config-hierarchy-pinned')
                guard_path = Path(args.root_guard)
                guard = _pin_root_guard(root, guard_path, args.root_guard_sha256)
                state['facts']['root_guard_sha256'] = guard['sha256']
                state['progress'].append('root-guard-pinned')
                producer = _stage_rustc_intent_producer(profile, out, target,
                                                        Path(args.rustc_intent_producer),
                                                        args.rustc_intent_producer_sha256)
                producer_copy = Path(producer['path'])
                instruments = _archive_instruments(profile, out, producer_copy, guard_path)
                state['facts']['instruments_archive_sha256'] = instruments['sha256']
                state['facts']['producer_sha256'] = producer['sha256']
                state['progress'].append('producer-staged-pinned-archived')
                capture = _create_capture_dir(out)
                capture_path = Path(capture['path'])
                state['progress'].append('intent-capture-created')
                env, env_declaration = _child_environment(
                    target, tmpdir, capture_path, guard_path, producer_copy)
                tool_paths = _observe_toolchain_paths(env, root)
                argv = _cargo_argv(root)
                target_guard = admission
                class FrozenInputs:
                    """Join held target identity to prebound source/config/producer inputs.

                    Every child uses the same pre/post checker over the snapshot
                    candidates, the inventory, the collector source, the staged
                    producer copy, the pinned root guard, the discovered config
                    hierarchy, the capture-directory identity and the evidence
                    paths (registry and Cargo.lock prebinds are re-observed once
                    after Cargo, not per child). This detects drift; it does not
                    confine a hostile concurrent writer.
                    """
                    def check(self):
                        """Refuse target, evidence, source, config, producer or guard drift."""
                        facts = target_guard.check()
                        require(_path_identity(out) == out_identity, 'cd01-output-drift')
                        require(_path_identity(evidence_parent) == evidence_identity,
                                'cd01-evidence-parent-drift')
                        require(out_identity[0] == facts['anchor_device'],
                                'cd01-evidence-target-device-mismatch')
                        require(profile.candidates(root) == before, 'cd01-candidates-drift')
                        require(_verify_inventory(root, inventory_path, profile) == inventory_summary,
                                'cd01-inventory-drift')
                        require(digest(Path(__file__)) == collector_sha, 'cd01-collector-drift')
                        require(digest(producer_copy) == producer['sha256'],
                                'cd01-producer-copy-drift')
                        require(digest(guard_path) == guard['sha256'], 'cd01-root-guard-drift')
                        require(_discover_cargo_configs(root, home) == config_pins,
                                'cd01-config-hierarchy-drift')
                        require(_path_identity(capture_path) == capture['identity'],
                                'cd01-capture-dir-drift')
                        require(_observe_toolchain_paths(env, root) == tool_paths,
                                'cd01-selected-tool-path-drift')
                        require(digest(Path(producer['source'])) == producer['sha256'],
                                'cd01-producer-original-drift')
                        return facts
                admission = FrozenInputs()
                toolchain = _observe_toolchain(profile, out, admission, env, root, state)
                free_pre_cargo = _checked_free_bytes(_free_observation_paths(root, target, out))
                invocation = dict(
                    argv=argv, cwd=str(root), target_dir=str(target),
                    one_cargo_compile_argv=True, compile_only=True, no_run=True,
                    default_features=True, no_feature_flags=True,
                    rustc_argv_codegen_identity='not-claimed; the unchanged cargo argv '
                        'does not imply identical rustc argv or codegen',
                    child_environment=env_declaration,
                    ambient_override_names_checked=list(AMBIENT_OVERRIDE_NAMES),
                    ambient_overrides_present=[],
                    rustc_wrapper_instrumentation=dict(
                        producer=producer, guard=guard, capture=capture,
                        instruments_archive=instruments,
                        env_config_names=[CAPTURE_DIR_ENV_NAME, SPACEGUARD_ENV_NAME],
                        delegation_chain='cargo -> RUSTC_WRAPPER (reviewed intent producer) '
                            '-> pinned root guard -> compiler; the producer delegates '
                            'before any compiler start and cannot observe the compiler exit'),
                    storage_admission=admission_proof,
                    inventory=inventory_summary,
                    declared_candidates=dict(count=len(before), archive_sha256=archive_proof['sha256']),
                    cargo_config_inputs=config_inputs,
                    config_discovery_pins=config_pins,
                    cargo_lock_sha256=registry_prebind['cargo_lock_sha256'],
                    registry_prebind=registry_prebind,
                    target_freshness=target_freshness,
                    toolchain_paths=tool_paths,
                    collector_sha256=collector_sha, helper_sha256=helper_digests,
                    toolchain_observations=toolchain,
                    free_bytes=dict(guard=MIN_FREE_BYTES, pre=free_pre, pre_cargo=free_pre_cargo),
                    workspace_cargo_config_semantics='config bytes preserved: no --config, '
                        'CARGO_HOME or CLI HOME override; the env RUSTC_WRAPPER override is '
                        'declared instrumentation recorded in rustc_wrapper_instrumentation; '
                        'checkout .cargo/config bytes prebound and archived; discovery '
                        'hierarchy pinned in config_discovery_pins',
                    additive_registration_files=_registration_additives(root),
                    no_cc_cflags_or_path_shadow=True,
                    retries=0, timeouts=None, global_watchdog=None)
                profile.publish(out / 'invocation.json', json.dumps(invocation, indent=2) + '\n')
                result = profile.outcomes.execute(
                    argv, out=out, name='cargo', admission=admission, publish=_cached_publisher(profile, state),
                    digest=profile.digest, failure_code='cd01-cargo-test-compile-failed',
                    cwd=str(root), env=env)
                state['returncodes']['cargo_test_compile'] = result.returncode
                require(result.returncode == 0, 'cd01-cargo-test-compile-failed')
                state['progress'].append('cargo-test-compile-returncode-0')
                events = _parse_cargo_events(out)
                event, executable, feature_inventory, artifacts_count = _select_libtest(events, state)
                frozen = _freeze_binary(profile, out, executable, state)
                registry_post = _compare_registry_prebind(root, home, registry_prebind)
                profile.publish(out / 'registry-postcheck.json',
                                json.dumps(registry_post, indent=2) + '\n')
                require(_discover_cargo_configs(root, home) == config_pins,
                        'cd01-config-hierarchy-changed')
                require(digest(guard_path) == guard['sha256'], 'cd01-root-guard-changed')
                require(digest(producer_copy) == producer['sha256'],
                        'cd01-producer-copy-changed')
                intent = _parse_intent_records(capture)
                require(all(r['guard_path_hex'] == os.fsencode(guard_path).hex()
                            for r in intent['records']), 'cd01-intent-root-guard-mismatch')
                intent_match = _match_libtest_intent(intent, state, event)
                profile.publish(out / 'rustc-intent-manifest.json',
                                json.dumps(dict(intent=intent, libtest_intent_candidate=intent_match),
                                           indent=2) + '\n')
                state['facts']['rustc_intent_records'] = intent['counts']['total']
                state['progress'].append('rustc-intent-parsed-and-matched')
                additives = _registration_additives(root)
                after = profile.candidates(root)
                require(after == state['before_candidates'], 'cd01-source-candidates-changed')
                state['facts']['source_candidates_stable'] = True
                inventory_post = _verify_inventory(root, inventory_path, profile)
                require(inventory_post == inventory_summary, 'cd01-inventory-changed')
                state['facts']['inventory_stable_post'] = True
                state['progress'].append('postchecks-passed')
                free_post = _checked_free_bytes(_free_observation_paths(root, target, out))
                require(_path_identity(out) == out_identity, 'cd01-output-changed')
                require(_fd_identity(evidence_fd) == evidence_identity,
                        'cd01-evidence-volume-changed')
                admission_final = admission.check()
                selected_libtest = dict(
                    name=SELECTED_ARTIFACT_NAME,
                    kind=(event.get('target') or {}).get('kind'),
                    profile_test=(event.get('profile') or {}).get('test'),
                    features=event.get('features'), executable=str(executable),
                    original_sha256=frozen['original_sha256'],
                    frozen_path=frozen['frozen_path'], frozen_sha256=frozen['frozen_sha256'],
                    installed=False)
                artifacts_summary = dict(
                    artifacts_observed=artifacts_count, no_fixture_features_on_any_event=True,
                    full_event_feature_inventory=feature_inventory,
                    selected_event=event, selected_libtest=selected_libtest)
                profile.publish(out / 'cargo-artifacts-summary.json',
                                json.dumps(artifacts_summary, indent=2) + '\n')
                profile.publish(out / 'free-space.json', json.dumps(
                    dict(guard=MIN_FREE_BYTES, pre=free_pre, pre_cargo=free_pre_cargo,
                         post=free_post), indent=2) + '\n')
                result_doc = dict(
                    status='CD01_COMPILE_ONLY_UNQUALIFIED',
                    schema='cd01-compile-only-collector/1',
                    full_effective_inputs_unknown=True,
                    compile_only=True, selected_tests_executed=0, tests_executed=0,
                    COMPILE_ONLY_WORKSPACE_CANDIDATES_OBSERVED=len(before),
                    summary=dict(selected_tests_executed=0,
                                 COMPILE_ONLY_WORKSPACE_CANDIDATES_OBSERVED=len(before),
                                 note='Lib test binary compiled with --no-run and never executed; '
                                      'observed candidates are prebound workspace inputs, not PASS tests.'),
                    exact_argv=argv, env_safe_declarations=env_declaration,
                    subprocess_numeric_results=dict(state['returncodes']),
                    storage_admission=admission_final,
                    snapshot=dict(root=str(root), git_absent=True, swarm_dir_absent=True,
                                  owned_by_current_uid=True,
                                  declared_context='R1a6a4785... plus17reviewed postimages '
                                                   '(declared externally; not re-verified here)',
                                  inventory=inventory_summary),
                    inventory_postcheck=inventory_post,
                    inventory_stable_after_cargo=True,
                    source_candidates_stable=True,
                    declared_candidates=dict(count=len(before), archive=archive_proof),
                    cargo_config_inputs=config_inputs,
                    collector_sha256=collector_sha, helper_sha256=helper_digests,
                    toolchain_observations=toolchain, cargo_artifacts=artifacts_summary,
                    rustc_intent=dict(producer=producer, guard=guard,
                                      capture_dir=capture['path'], manifest=intent,
                                      libtest_intent_candidate=intent_match,
                                      instruments_archive=instruments),
                    registry_prebind=registry_prebind, registry_postcheck=registry_post,
                    config_discovery_pins=config_pins,
                    cargo_lock_sha256=registry_prebind['cargo_lock_sha256'],
                    target_freshness=target_freshness, toolchain_paths=tool_paths,
                    frozen_libtest=frozen, additive_registration_files=additives,
                    free_bytes=dict(guard=MIN_FREE_BYTES, pre=free_pre,
                                    pre_cargo=free_pre_cargo, post=free_post),
                    ambient_overrides_present=[],
                    no_fixture_features_on_any_event=True,
                    limits=dict(effective_dependency_qualification='not-performed',
                                unknown_full_effective_inputs=True,
                                registry_or_generated_or_toolchain_qualification='not-performed',
                                registry_prebind_scope='lock-selected source dirs and archives '
                                    'prebound and re-observed; the compiler read window over '
                                    'them stays unobserved and post-hashes are not a '
                                    'read-window proof',
                                rustc_intent_receipts='pre-exec only; every record outcome is '
                                    'UNKNOWN; metadata probes are not counted as compiles; no '
                                    'compiler outcome is derived from intent; cargo primary 0 '
                                    'and the frozen artifact remain separate facts',
                                rustc_argv_codegen_identity='not-claimed; the unchanged cargo '
                                    'argv does not imply identical rustc argv or codegen',
                                toolchain_paths='resolved proxy/symlink chains, existing stat '
                                    'metadata and file digests recorded; rustup proxy versus '
                                    'actual target binary stays UNKNOWN unless resolved; not '
                                    'a full toolchain proof',
                                cd01_paired_libtest_admission_record='not-created; full compiler '
                                    'graph and later independent qualification remain required',
                                installed=False, frozen_binary_location='evidence output only',
                                held_fds='evidence directory + admitted target directories '
                                    'through child execution and final publication; '
                                    'ordinary-unmount hold only, no sandbox'),
                    retries=0, timeouts=None, global_watchdog=None)
                profile.publish(out / 'result.json', json.dumps(result_doc, indent=2) + '\n')
                state['progress'].append('result-published')
                free_final = _checked_free_bytes(_free_observation_paths(root, target, out))
                profile.publish(out / 'free-space-final.json', json.dumps(
                    dict(guard=MIN_FREE_BYTES, after_publication=free_final), indent=2) + '\n')
                require(_path_identity(out) == out_identity, 'cd01-output-changed-after-publication')
                require(_fd_identity(evidence_fd) == evidence_identity,
                        'cd01-evidence-volume-changed-after-publication')
                state['facts']['result_published'] = True
            return result_doc
    except BaseException as error:
        _publish_failure(out, state, error, profile, root, target)
        raise


def _parse_args(argv=None):
    """Parse the eight required explicit CLI arguments; no silent defaults.

    --root snapshot checkout, --output NEW evidence leaf, --target NEW external
    Cargo target, --inventory original snapshot JSON, --rustc-intent-producer
    the reviewed producer source, --rustc-intent-producer-sha256 its expected
    SHA-256, --root-guard the exact root admission guard path and
    --root-guard-sha256 its expected SHA-256 (both pins are technical
    allocation inputs supplied explicitly, never operator policy). Errors:
    missing arguments refuse via argparse; path-form problems refuse later in
    _check_paths or the staging and pinning functions. Side effects: none.
    """
    parser = argparse.ArgumentParser(
        description='CD01 one-shot compile-only collector (see module docstring for limits)')
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--target', type=Path, required=True)
    parser.add_argument('--inventory', type=Path, required=True)
    parser.add_argument('--rustc-intent-producer', type=Path, required=True)
    parser.add_argument('--rustc-intent-producer-sha256', required=True)
    parser.add_argument('--root-guard', type=Path, required=True)
    parser.add_argument('--root-guard-sha256', required=True)
    return parser.parse_args(argv)


def main():
    """Run the CLI under an independently verified outer evidence-volume hold.

    The hold encloses run(), its failure publication after target cleanup, and
    terminal stdout/stderr publication. Target holds enclose normal result
    publication; a target-close error is recorded under the outer volume hold.
    An outer hold-close error is terminal stderr uncertainty, never test PASS.
    """
    args = _parse_args()
    proof = None
    try:
        storage, facts = _preflight_evidence_volume(args)
        parent = args.output.parent
        with _held_directory(parent) as outer_fd:
            info = os.fstat(outer_fd)
            require(f'{os.major(info.st_dev)}:{os.minor(info.st_dev)}' == facts['maj:min'],
                    'cd01-evidence-open-device')
            require(_path_identity(parent) == _fd_identity(outer_fd), 'cd01-evidence-parent-drift')
            _checked_free_bytes({'host-checkout-root': args.root.resolve(),
                                 'held-evidence-parent': parent})
            try:
                proof = run(args)
                require(storage.observe_mount(Path(facts['target'])) == facts,
                        'cd01-evidence-mount-drift-after-run')
                sys.stdout.write(json.dumps(proof, indent=2) + '\n')
                sys.stdout.flush()
            except BaseException as error:
                sys.stderr.write(json.dumps({'error': _error_tree(error),
                    'all_native_outcomes': _native_outcomes(error),
                    'completed_proof_before_error': proof}) + '\n')
                sys.stderr.flush()
                raise
    except BaseException as error:
        sys.stderr.write(json.dumps({'terminal_error': _error_tree(error),
            'all_native_outcomes': _native_outcomes(error),
            'completed_proof_before_error': proof}) + '\n')
        sys.stderr.flush()
        raise SystemExit(1)


if __name__ == '__main__':
    main()
