#!/usr/bin/env python3
"""Build, freeze and exercise the exact cockpit unit image against the Go corpus.

Reuse recorded delivery compiler options, not an implicit debug profile. The
image is cfg(test), with build-version injection explicitly absent, and must
never be installed. Actual local depfiles are prebound and archived; external
toolchain/C build inputs and production qualification remain separate.
"""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tarfile
import traceback
from current_cli_inventory import publish_new

ROOT = Path(__file__).resolve().parents[2]
TEST = 'root_store::cockpit_post_delivery::tests::retained_go_oracle_matches_sources_and_cost_text'
spec = importlib.util.spec_from_file_location('native_profile', ROOT / 'tools/verification/verify_native_profile.py')
profile = importlib.util.module_from_spec(spec)
spec.loader.exec_module(profile)


def publish(path, value):
    """Publish new evidence only; retain visible effects and IO failures."""
    raw = value if isinstance(value, bytes) else (json.dumps(value, indent=2) + '\n').encode()
    publish_new(path, raw)


def run(invocation, out):
    """Wait for the shared build lock, bind sources, freeze then run one test.

    Runtime corpus and method inputs are hashed before/after. No wall deadline,
    retry, provider, datastore or installation is added. Missing/stale inputs,
    Cargo/IO/test/close failures refuse; a named compile alone is not PASS.
    """
    settings = json.loads(invocation.read_bytes())
    options = settings['profile_environment']
    if set(options) != {'CARGO_PROFILE_DEV_DEBUG', 'CARGO_PROFILE_TEST_DEBUG', 'CARGO_INCREMENTAL'} or not all(isinstance(v, str) for v in options.values()):
        raise ValueError('delivery-compiler-options-invalid')
    target = Path(settings['target_dir']).resolve()
    if not target.is_relative_to(ROOT) or Path(settings['cwd']).resolve() != ROOT:
        raise ValueError('delivery-invocation-root-invalid')
    print('WAITING_PROJECT_BUILD_LOCK', flush=True)
    with (ROOT / 'deploy/.local-build.lock').open('a+b') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('BUILD_LOCK_ACQUIRED', flush=True)
        before = profile.candidates(ROOT)
        extras = [p for p in (ROOT / 'rust/crates').glob('*/tests/**/*') if p.is_file()]
        before.update(profile.manifest(ROOT, extras))
        corpus = ROOT / 'tools/rust-migration/fixtures/cockpit-post-delivery-go.json'
        methods = [Path(__file__).resolve(), invocation, corpus, ROOT / 'tools/verification/verify_native_profile.py', ROOT / 'tools/rust-migration/current_cli_inventory.py']
        runtime_before = {str(p): profile.digest(p) for p in methods}
        publish(out / 'before.json', before)
        publish(out / 'runtime.before.json', runtime_before)
        env = {**os.environ, **options, 'CARGO_TARGET_DIR': str(target)}
        env.pop('SWARM_NATIVE_BUILD_METADATA', None)
        names = ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_TARGET', 'RUSTUP_TOOLCHAIN', 'CARGO_HOME']
        presence = {name: bool(env.get(name)) for name in names}
        if any(presence.values()):
            raise ValueError('unit-ambient-compiler-override')
        argv = ['cargo', 'test', '--manifest-path', 'rust/Cargo.toml', '--offline', '--locked', '-p', 'swarm-engine', '--lib', '--no-run', '--message-format=json']
        publish(out / 'invocation.json', dict(argv=argv, profile_environment=options, target_dir=str(target), build_metadata='absent-test-only', ambient_override_presence=presence, global_watchdog=None))
        with (out / 'cargo.stdout').open('xb') as stdout, (out / 'cargo.stderr').open('xb') as stderr:
            built = subprocess.run(argv, cwd=ROOT, env=env, stdout=stdout, stderr=stderr, stdin=subprocess.DEVNULL)
        publish(out / 'cargo-exit.json', dict(returncode=built.returncode))
        if built.returncode:
            raise RuntimeError('cockpit-unit-build-failed')
        events = [json.loads(line) for line in (out / 'cargo.stdout').read_text().splitlines()]
        artifacts = [v for v in events if v.get('reason') == 'compiler-artifact']
        selected = [v for v in artifacts if v['target']['name'] == 'swarm_engine' and v['profile']['test'] and v.get('executable')]
        if len(selected) != 1 or selected[0]['features'] or not any(v.get('reason') == 'build-finished' and v.get('success') is True for v in events):
            raise ValueError('cockpit-unit-artifact-invalid')
        actual = {}
        previous = Path.cwd()
        try:
            # Observed Cargo unit depfiles use paths relative to the workspace.
            os.chdir(ROOT / 'rust')
            for artifact in artifacts:
                if not Path(artifact['manifest_path']).is_relative_to(ROOT / 'rust'):
                    continue
                if artifact['target']['kind'] != ['lib']:
                    raise ValueError('unit-local-target-kind-unhandled')
                if artifact['profile']['test']:
                    depfile = Path(artifact['executable']).with_suffix('.d')
                else:
                    library = next(Path(p) for p in artifact['filenames'] if p.endswith('.rlib'))
                    depfile = library.with_name(library.stem.removeprefix('lib') + '.d')
                actual.update(profile.dependencies(ROOT, depfile))
        finally:
            os.chdir(previous)
        profile.effective_unchanged(ROOT, before, actual)
        publish(out / 'actual.json', actual)
        profile.archive_effective_sources(ROOT, out, actual)
        original = Path(selected[0]['executable'])
        body = original.read_bytes()
        sha = hashlib.sha256(body).hexdigest()
        frozen = out / ('cockpit-unit-' + sha)
        publish(frozen, body)
        frozen.chmod(0o700)
        if profile.digest(original) != sha or profile.digest(frozen) != sha:
            raise ValueError('unit-image-changed-during-freeze')
        argv = [str(frozen), TEST, '--exact', '--nocapture']
        publish(out / 'test-argv.json', argv)
        with (out / 'test.stdout').open('xb') as stdout, (out / 'test.stderr').open('xb') as stderr:
            result = subprocess.run(argv, cwd=ROOT, env=env, stdout=stdout, stderr=stderr, stdin=subprocess.DEVNULL)
        publish(out / 'test-exit.json', dict(returncode=result.returncode))
        runtime_after = {str(p): profile.digest(p) for p in methods}
        publish(out / 'runtime.after.json', runtime_after)
        profile.effective_unchanged(ROOT, before, actual)
        if runtime_before != runtime_after or profile.digest(frozen) != sha:
            raise ValueError('unit-inputs-or-image-changed')
        output = (out / 'test.stdout').read_text()
        if result.returncode or 'test result: ok. 1 passed; 0 failed; 0 ignored' not in output:
            raise RuntimeError('named-cockpit-unit-not-passed')
        cases = json.loads(corpus.read_bytes())['cases']
        lines = {line for line in output.splitlines() if line.startswith('PASS GO_COCKPIT_')}
        expected = {'PASS GO_COCKPIT_' + ('SOURCE' if c['kind'] == 'source' else 'COST') + ' ' + c['name'] for c in cases}
        if not expected or lines != expected:
            raise ValueError('named-go-cases-incomplete')
        publish(out / 'result.json', dict(status='PASS', cases=len(cases), named_tests=1, sha256=sha, image=str(frozen), profile='cfg(test), matching recorded compiler options; not production', effective_local_sources=len(actual), installed=False, provider_started=False, global_watchdog=None))
        print('RUST_COCKPIT_GO_ORACLE_VERIFIED', len(cases), flush=True)


def replay(record, out, image=None):
    """Run a retained exact unit image on the current corpus without Cargo.

    Verify its recorded SHA and retained source archive before execution. The
    optional image relocation must have identical bytes. The original checkout
    fixture locator is compiled into the unit. Runtime corpus changes are bound
    separately; no current source, production profile or authenticity is inferred.
    """
    record_bytes = record.read_bytes()
    record_sha = hashlib.sha256(record_bytes).hexdigest()
    proof = json.loads(record_bytes)
    if proof.get('status') != 'PASS' or proof.get('installed') is not False or not proof.get('profile', '').startswith('cfg(test)'):
        raise ValueError('retained-unit-record-invalid')
    archive = record.parent / 'effective-sources.tar.gz'
    archive_proof = json.loads((record.parent / 'effective-sources.json').read_bytes())
    sources = json.loads((record.parent / 'actual.json').read_bytes())
    if profile.digest(archive) != archive_proof['sha256']:
        raise ValueError('retained-unit-sources-changed')
    with tarfile.open(archive) as retained:
        if set(retained.getnames()) != set(sources):
            raise ValueError('retained-unit-source-members-changed')
        for member in retained:
            stream = retained.extractfile(member)
            if not member.isfile() or stream is None:
                raise ValueError('retained-unit-source-not-bytes')
            with stream:
                if hashlib.sha256(stream.read()).hexdigest() != sources[member.name]:
                    raise ValueError('retained-unit-source-bytes-changed')
    image = image or Path(proof['image'])
    if profile.digest(image) != proof['sha256'] or image.read_bytes()[:4] != b'\x7fELF':
        raise ValueError('retained-unit-image-changed')
    corpus = ROOT / 'tools/rust-migration/fixtures/cockpit-post-delivery-go.json'
    before = profile.digest(corpus)
    argv = [str(image), TEST, '--exact', '--nocapture']
    publish(out / 'invocation.json', dict(argv=argv, corpus_sha256=before, source_record=str(record), source_record_sha256=record_sha, image_sha256=proof['sha256'], cargo_invoked=False, global_watchdog=None))
    with (out / 'test.stdout').open('xb') as stdout, (out / 'test.stderr').open('xb') as stderr:
        result = subprocess.run(argv, cwd=ROOT, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
    publish(out / 'test-exit.json', dict(returncode=result.returncode))
    output = (out / 'test.stdout').read_text()
    cases = json.loads(corpus.read_bytes())['cases']
    expected = {'PASS GO_COCKPIT_' + ('SOURCE' if c['kind'] == 'source' else 'COST') + ' ' + c['name'] for c in cases}
    observed = {line for line in output.splitlines() if line.startswith('PASS GO_COCKPIT_')}
    if result.returncode or 'test result: ok. 1 passed; 0 failed; 0 ignored' not in output or not expected or observed != expected:
        raise RuntimeError('retained-cockpit-unit-not-passed')
    if profile.digest(corpus) != before or profile.digest(image) != proof['sha256'] or profile.digest(record) != record_sha:
        raise ValueError('retained-unit-runtime-input-changed')
    publish(out / 'result.json', dict(status='PASS', cases=len(cases), named_tests=1, sha256=proof['sha256'], source_record_sha256=record_sha, corpus_sha256=before, cargo_invoked=False, installed=False, provider_started=False, global_watchdog=None))
    print('RUST_COCKPIT_GO_ORACLE_REPLAYED', len(cases), flush=True)


def main():
    """Own a fresh private output before recovery; never append to old evidence."""
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--profile-invocation', type=Path)
    mode.add_argument('--replay-record', type=Path)
    parser.add_argument('--image', type=Path, help='Relocated retained image; replay only, identical SHA required')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.image and not args.replay_record:
        parser.error('--image requires --replay-record')
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False, mode=0o700)
    try:
        if args.replay_record:
            replay(args.replay_record.resolve(), out, args.image.resolve() if args.image else None)
        else:
            run(args.profile_invocation.resolve(), out)
    except BaseException as error:
        try:
            publish(out / 'failure.json', dict(status='FAIL', cause=''.join(traceback.format_exception(error))))
        except BaseException as retention:
            raise BaseExceptionGroup('unit failure and evidence retention', [error, retention])
        raise


if __name__ == '__main__':
    main()
