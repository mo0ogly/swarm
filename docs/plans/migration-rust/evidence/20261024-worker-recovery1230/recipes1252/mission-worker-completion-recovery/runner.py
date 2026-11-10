#!/usr/bin/env python3
"""Run permanent, source-bound fixture recipes and verify retained evidence.

Executes explicit authorized argv without an implicit shell, retry or global
watchdog. A recipe grants no rights, deployment, paid calls or live-state access.
PASS covers declared inputs/assertions only, not effective-graph qualification,
independent review, authenticity or acceptance. Environment capture excludes the
inherited environment; recipe authors must never put credentials in a manifest.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import time

REPOSITORY = Path(__file__).resolve().parents[2]


def digest(raw):
    """Hash complete bytes, never partial output or a declared digest alone."""
    return hashlib.sha256(raw).hexdigest()


def publish(path, raw):
    """Publish fresh evidence atomically; errors leave any existing file intact.

    Adjacent temp, fsync and exclusive link prevent partial overwrite/ENOSPC loss.
    Failure propagates; this grants no permission to replace historical evidence.
    """
    path = Path(path)
    if not isinstance(raw, bytes):
        raw = (json.dumps(raw, ensure_ascii=False, indent=2) + '\n').encode()
    fd, temporary = tempfile.mkstemp(dir=path.parent, prefix='.recipe-')
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        if Path(temporary).read_bytes() != raw:
            raise OSError('recipe-publication-bytes-differ')
        os.link(temporary, path)
        directory = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        Path(temporary).unlink(missing_ok=True)


def load_recipe(raw):
    """Validate v1 recipe structure, explicit assertions and environment overrides.

    Reject unknown fields, zero steps, bool-as-exit-code, empty assertions and
    reserved HOME/CODEX_HOME overrides. The manifest is data, not authorization;
    semantic coverage and absence of secrets remain the author's responsibility.
    """
    def unique_object(pairs):
        """Refuse ambiguous duplicate JSON keys, including nested step fields."""
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError('recipe-duplicate-key')
            value[key] = item
        return value
    value = json.loads(raw.decode('utf-8'), object_pairs_hook=unique_object)
    if not isinstance(value, dict) or set(value) != {'schema_version', 'id', 'inputs', 'environment', 'steps'}:
        raise ValueError('recipe-fields-invalid')
    if type(value['schema_version']) is not int or value['schema_version'] != 1:
        raise ValueError('recipe-version-invalid')
    if not isinstance(value['id'], str) or not value['id'].strip():
        raise ValueError('recipe-id-required')
    if not isinstance(value['inputs'], list) or not value['inputs']:
        raise ValueError('recipe-inputs-required')
    for pattern in value['inputs']:
        if not isinstance(pattern, str) or not pattern or '\0' in pattern or Path(pattern).is_absolute() or '..' in Path(pattern).parts:
            raise ValueError('recipe-input-pattern-invalid')
    if not isinstance(value['environment'], dict):
        raise ValueError('recipe-environment-invalid')
    for key, item in value['environment'].items():
        if not isinstance(key, str) or not key or '=' in key or '\0' in key or key.lower() in {'home', 'codex_home'} or not isinstance(item, str) or '\0' in item:
            raise ValueError('recipe-environment-override-invalid')
    if not isinstance(value['steps'], list) or not value['steps']:
        raise ValueError('recipe-steps-required')
    seen = set()
    for step in value['steps']:
        if not isinstance(step, dict) or set(step) != {'id', 'command', 'expected_exit', 'required_stdout', 'required_stderr'}:
            raise ValueError('recipe-step-fields-invalid')
        if not isinstance(step['id'], str) or not step['id'].strip() or step['id'] in seen:
            raise ValueError('recipe-step-id-invalid')
        seen.add(step['id'])
        args = step['command']
        if not isinstance(args, list) or not args or not all(isinstance(arg, str) and '\0' not in arg for arg in args) or not args[0]:
            raise ValueError('recipe-argv-invalid')
        if type(step['expected_exit']) is not int or step['expected_exit'] < 0:
            raise ValueError('recipe-expected-exit-invalid')
        assertions = []
        for key in ('required_stdout', 'required_stderr'):
            items = step[key]
            if not isinstance(items, list) or not all(isinstance(item, str) and item.strip() for item in items):
                raise ValueError('recipe-assertions-invalid')
            assertions.extend(items)
        if not assertions:
            raise ValueError('recipe-observable-assertion-required')
    return value


def snapshot(root, patterns, output, recipe_path):
    """Hash declared contained files, rejecting missing patterns or escaping links.

    Exclude this fresh evidence directory, root live state/Git, dependency/build
    caches. This inventories declared source only; it is not compiler dependency
    discovery. Added/removed/changed declared files are detected between steps.
    """
    selected = set()
    excluded = [output, root / '.swarm', root / '.git', root / 'node_modules', root / 'rust/target']
    for pattern in patterns:
        matches = set()
        for path in root.glob(pattern):
            if not path.is_file() or any(path.is_relative_to(folder) for folder in excluded) or '__pycache__' in path.parts:
                continue
            if not path.resolve().is_relative_to(root):
                raise ValueError('recipe-input-outside-root')
            matches.add(path)
        if not matches:
            raise ValueError('recipe-input-pattern-empty')
        selected.update(matches)
    if recipe_path.is_relative_to(root):
        selected.add(recipe_path)
    return {str(path.relative_to(root)): digest(path.read_bytes()) for path in sorted(selected)}


def archive_inputs(root, inputs, output):
    """Save exact declared source bytes; refuse a source change during archiving."""
    data = io.BytesIO()
    with tarfile.open(fileobj=data, mode='w:gz') as archive:
        for name, expected in inputs.items():
            raw = (root / name).read_bytes()
            if digest(raw) != expected:
                raise ValueError('recipe-source-changed-during-archive')
            info = tarfile.TarInfo(name)
            info.size = len(raw)
            archive.addfile(info, io.BytesIO(raw))
    publish(output / 'declared-inputs.tar.gz', data.getvalue())


def run_recipe(root, recipe_path, output):
    """Run an authorized recipe sequentially with EOF stdin and no global timeout.

    Fresh output only. Preserve original stdout/stderr, launcher identity, declared
    environment and source bytes. Unexpected exit/assertion/source change stops
    later steps. FAIL retains cause and evidence; interruption never becomes PASS.
    Preserve the invoked launcher name and symlink path (including relative PATH
    directories) because multicall tools/virtual environments consume argv0.
    The separately recorded resolved target/hash is provenance, not a replacement
    invocation. No retry, supervision, deployment or acceptance is created.
    """
    root, recipe_path, output = Path(root).resolve(), Path(recipe_path).resolve(), Path(output).resolve()
    if output.exists() or output in {root, root / '.swarm', root / '.git'} or output.is_relative_to(root / '.swarm') or output.is_relative_to(root / '.git'):
        raise ValueError('recipe-output-must-be-new-outside-live-state')
    original = recipe_path.read_bytes()
    recipe = load_recipe(original)
    before = snapshot(root, recipe['inputs'], output, recipe_path)
    output.mkdir(mode=0o700)
    tracked = []
    steps = []
    failure = None
    after = None
    def save(name, value):
        """Publish one owned immutable artifact and retain its hash for verification."""
        publish(output / name, value)
        tracked.append(name)
    try:
        save('recipe.original.json', original)
        save('runner.py', Path(__file__).read_bytes())
        save('inputs.before.json', before)
        archive_inputs(root, before, output)
        tracked.append('declared-inputs.tar.gz')
        environment = dict(os.environ)
        overrides = {key: item.replace('{root}', str(root)) for key, item in recipe['environment'].items()}
        environment.update(overrides)
        save('environment.json', {'overrides': overrides, 'scope': 'Inherited environment not logged; compiler graph/profile needs separate qualification'})
        for number, step in enumerate(recipe['steps'], 1):
            args = [arg.replace('{root}', str(root)) for arg in step['command']]
            search_path = os.pathsep.join(str(root / part) if not Path(part).is_absolute() else part for part in environment.get('PATH', os.defpath).split(os.pathsep))
            candidate = str(root / args[0]) if os.path.dirname(args[0]) else args[0]
            executable = shutil.which(candidate, path=search_path)
            if executable is None:
                raise FileNotFoundError('recipe-executable-unavailable')
            args[0] = str(Path(executable).absolute())
            launcher_hash = digest(Path(executable).read_bytes())
            prefix = f'step-{number:04}'
            stdout, stderr = output / (prefix + '.stdout'), output / (prefix + '.stderr')
            print(json.dumps({'state': 'RUN', 'step': step['id']}), flush=True)
            started = time.monotonic()
            with stdout.open('xb') as out, stderr.open('xb') as err:
                tracked.extend([stdout.name, stderr.name])
                process = subprocess.run(args, cwd=root, env=environment, input=b'', stdout=out, stderr=err)
                out.flush(); os.fsync(out.fileno())
                err.flush(); os.fsync(err.fileno())
            record = dict(id=step['id'], command=args, expected_exit=step['expected_exit'], exit=process.returncode, seconds=time.monotonic()-started, executable=str(Path(executable).resolve()), launcher_sha256=launcher_hash)
            steps.append(record)
            save(prefix + '.json', record)
            if process.returncode != step['expected_exit']:
                raise ValueError('recipe-unexpected-exit')
            if any(token.encode() not in stdout.read_bytes() for token in step['required_stdout']) or any(token.encode() not in stderr.read_bytes() for token in step['required_stderr']):
                raise ValueError('recipe-observable-assertion-missing')
            if digest(Path(executable).read_bytes()) != launcher_hash:
                raise ValueError('recipe-launcher-changed')
            after = snapshot(root, recipe['inputs'], output, recipe_path)
            if before != after or recipe_path.read_bytes() != original or (output/'runner.py').read_bytes() != Path(__file__).read_bytes():
                raise ValueError('recipe-declared-inputs-changed')
            print(json.dumps({'state': 'PASS', 'step': step['id']}), flush=True)
    except BaseException as error:
        failure = {'type': type(error).__name__, 'cause': str(error), 'errno': getattr(error, 'errno', None)}
    if after is None:
        try:
            after = snapshot(root, recipe['inputs'], output, recipe_path)
        except Exception as error:
            after = {'snapshot_failed': type(error).__name__}
    save('inputs.after.json', after)
    status = 'PASS' if failure is None else 'FAIL'
    artifacts = {name: digest((output/name).read_bytes()) for name in tracked}
    result = dict(schema_version=1, recipe=recipe['id'], status=status, scope='Declared recipe checks only; not independent/effective-graph qualification or acceptance', global_watchdog=None, source_stable=before==after, steps=steps, failure=failure, artifacts=artifacts)
    publish(output/'result.json', result)
    return result


def verify_evidence(output):
    """Verify retained hashes/source archive without rerunning or authenticating it.

    Missing/changed/escaping artifacts or incoherent PASS refuse. Recorded FAIL
    stays FAIL even when its files are intact. Self-consistency proves no trusted
    author, independent review, current checkout compatibility or deployment.
    """
    output = Path(output).resolve()
    result = json.loads((output/'result.json').read_bytes())
    if result.get('schema_version') != 1 or result.get('status') not in {'PASS', 'FAIL'}:
        raise ValueError('recipe-result-invalid')
    mandatory = {'recipe.original.json', 'runner.py', 'inputs.before.json', 'inputs.after.json', 'declared-inputs.tar.gz', 'environment.json'}
    for number, actual in enumerate(result['steps'], 1):
        mandatory.update(f'step-{number:04}' + suffix for suffix in ('.json', '.stdout', '.stderr'))
    if not mandatory.issubset(result['artifacts']):
        raise ValueError('recipe-artifact-table-incomplete')
    for name, expected in result['artifacts'].items():
        path = output/name
        if Path(name).is_absolute() or '..' in Path(name).parts or not path.resolve().is_relative_to(output) or digest(path.read_bytes()) != expected:
            raise ValueError('recipe-artifact-changed-or-invalid')
    recipe = load_recipe((output/'recipe.original.json').read_bytes())
    before = json.loads((output/'inputs.before.json').read_bytes())
    after = json.loads((output/'inputs.after.json').read_bytes())
    with tarfile.open(output/'declared-inputs.tar.gz', 'r:gz') as archive:
        if len(archive.getmembers()) != len(before) or set(archive.getnames()) != set(before) or any(not entry.isfile() for entry in archive.getmembers()):
            raise ValueError('recipe-source-archive-incomplete')
        for name, expected in before.items():
            if Path(name).is_absolute() or '..' in Path(name).parts or digest(archive.extractfile(name).read()) != expected:
                raise ValueError('recipe-source-archive-invalid')
    if result['status'] == 'PASS':
        if before != after or not result['source_stable'] or len(result['steps']) != len(recipe['steps']) or result['failure'] is not None:
            raise ValueError('recipe-pass-incoherent')
        for number, (declared, actual) in enumerate(zip(recipe['steps'], result['steps']), 1):
            if json.loads((output/f'step-{number:04}.json').read_bytes()) != actual or actual['exit'] != declared['expected_exit'] or actual['id'] != declared['id']:
                raise ValueError('recipe-step-incoherent')
            stdout = (output/f'step-{number:04}.stdout').read_bytes()
            stderr = (output/f'step-{number:04}.stderr').read_bytes()
            if any(s.encode() not in stdout for s in declared['required_stdout']) or any(s.encode() not in stderr for s in declared['required_stderr']):
                raise ValueError('recipe-step-assertion-incoherent')
    return {'integrity': 'VERIFIED', 'recorded_status': result['status'], 'recipe': recipe['id']}


def main():
    """Expose explicit run/verify; nonzero/incomplete evidence never exits as PASS."""
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    run = sub.add_parser('run')
    run.add_argument('--recipe', type=Path, required=True)
    run.add_argument('--output', type=Path, required=True)
    run.add_argument('--root', type=Path, default=REPOSITORY)
    verify = sub.add_parser('verify')
    verify.add_argument('--evidence', type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.action == 'run':
            value = run_recipe(args.root, args.recipe, args.output)
            print(json.dumps({'status': value['status'], 'evidence': str(args.output), 'steps': len(value['steps'])}))
            return 0 if value['status'] == 'PASS' else 1
        value = verify_evidence(args.evidence)
        print(json.dumps(value))
        return 0 if value['recorded_status'] == 'PASS' else 1
    except Exception as error:
        print(json.dumps({'status': 'REFUSED', 'cause': str(error), 'type': type(error).__name__}))
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
