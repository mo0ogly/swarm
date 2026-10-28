"""Qualify supplied-text quote matching and the actual Go reply-parser regression.

This is a component check, with no provider, deployment or managed acceptance.
"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
from collections import Counter
from qualification_inputs import collect
from recorded_process import capture

ROOT = Path(__file__).resolve().parents[2]
GO_TESTS = (
    'TestManagedReviewQuoteKeepsMatchingFenceBoundary',
    'TestManagedReviewQuoteResumesAfterMatchingFence',
    'TestReviewReplyFieldBoundsUseActualUTF8Bytes',
    'TestFragmentMarkdownQuoteFormattingOnly',
    'TestManagedReviewEvidencePreservesArtifactKindsAndBoundaries',
    'TestReferencedCitationUsesExistingLiteralAndScopeGuards',
)


def vectors():
    """Describe UTF-8 matching boundaries; each case gets its own literal control."""
    cases = []

    def add(name, content, quote, present, kind='source', path='proof.md'):
        """Retain original input and expected business result, independently of exit code."""
        cases.append(dict(name=name, frame=dict(name=path, kind=kind, content=content,
                          evidence=quote, minimum_normalized_bytes=8), present=present))

    for kind in ('source', 'diff'):
        for i, content in enumerate(('```go\n~~~\nreturn `raw` text\n```',
                                     '````go\n```\nreturn `raw` text\n````',
                                     '```go\n```not-a-close\nreturn `raw` text\n```')):
            if kind == 'diff':
                content = '+' + content.replace('\n', '\n+')
            add(f'{kind}-invalid-close-{i}', content, 'return raw text', False, kind)
        for i, (opening, closing) in enumerate((('```go', '```'), ('~~~~go', '~~~~~'),
                                                ('````go', '`````'))):
            content = f'{opening}\nreturn `raw` text\n{closing}\nValid `prose` after\nboundary.'
            if kind == 'diff':
                content = '+' + content.replace('\n', '\n+')
            add(f'{kind}-matching-close-{i}', content, 'Valid prose after boundary.', True, kind)
            add(f'{kind}-code-literal-only-{i}', content, 'return raw text', False, kind)
    for i, space in enumerate(('\t', '\r', '\u0085', '\u00a0', '\u1680', '\u200a',
                                '\u2028', '\u202f', '\u205f', '\u3000')):
        add(f'go-space-{i}', f'Original{space}`exact`{space}evidence', 'Original exact evidence', True)
    for i, space in enumerate(('\u001c', '\u200b', '\ufeff')):
        add(f'non-go-space-{i}', f'Original{space}`exact`{space}evidence', 'Original exact evidence', False)
    for i, content in enumerate(('+Original `exact`\n-evidence',
                                  '+Original `exact`\n@@ -1 +1 @@\n+evidence',
                                  '+Original `exact`\n+\n+evidence',
                                  '+Original `exact`\n+++ next.md\n+evidence')):
        add(f'diff-run-{i}', content, 'Original exact evidence', False, 'diff')
    for i, (path, kind) in enumerate((('proof.go', 'source'), ('proof.MD', 'source'),
                                      ('proof.md', 'context'), ('proof.md', 'control_output'))):
        add(f'permission-{i}', 'Original `exact`\nevidence', 'Original exact evidence', False, kind, path)
    add('inline-wrap', 'Original `exact`\nevidence', 'Original exact evidence', True)
    add('paragraph', 'Original `exact`\n\nevidence', 'Original exact evidence', False)
    add('short-normalized', '`1234567`', '1234 567', False)
    add('unicode-eight-bytes', 'éé`éé`', 'éééé', True)
    add('empty-literal-not-reply-validator', 'original', '', True)
    add('literal-non-markdown', 'literal', 'literal', True, 'context', 'source.rs')
    add('long-literal-not-reply-validator', 'e' * 4001, 'e' * 4001, True)
    add('large-prose', 'prefix\n' * 4096 + 'Original `exact`\nevidence', 'Original exact evidence', True)
    return cases


def main():
    """Archive exact inputs and named checks in a fresh directory; refuse any failed prerequisite."""
    if len(sys.argv) != 2:
        raise RuntimeError('One fresh evidence directory required')
    output = Path(sys.argv[1]).resolve()
    output.mkdir(parents=True, exist_ok=False)
    work = Path(tempfile.mkdtemp(prefix='review-quote-', dir=ROOT / '.cache'))
    env = dict(os.environ)
    env.setdefault('CARGO_TARGET_DIR', str(work / 'target'))
    commands = []

    def save(name, value):
        """Publish only completed JSON records by fsync and same-directory replacement."""
        path = output / name
        fd, tmp = tempfile.mkstemp(dir=path.parent, prefix='.' + path.name + '.')
        with os.fdopen(fd, 'w') as stream:
            json.dump(value, stream, indent=2)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(tmp, path)

    def run(label, args, data=None, expected=0):
        """Capture checked subprocess bytes with no arbitrary global verification watchdog."""
        args = [str(arg) for arg in args]
        started = datetime.datetime.now(datetime.timezone.utc).isoformat()
        clock = time.monotonic()
        process = capture(args, cwd=ROOT, env=env, data=data,
                          stdout_path=output/(label+'.stdout'), stderr_path=output/(label+'.stderr'))
        if data is not None:
            (output / (label + '.stdin')).write_bytes(data)
        commands.append(dict(label=label, command=args, start_utc=started,
                             seconds=time.monotonic() - clock, exit=process.returncode,
                             expected_exit=expected))
        save('commands.json', commands)
        print(label, 'exit', process.returncode, flush=True)
        if process.returncode != expected:
            raise RuntimeError(label + ': original failure retained')
        return process.stdout

    try:
        cargo = ['--manifest-path', 'rust/Cargo.toml', '--locked', '--offline']
        raw = run('reference-source', ['go', 'run', './tools/rust-migration/review-quote-reference'])
        source = work / 'main.go'
        source.write_bytes(raw)
        package = './' + str(work.relative_to(ROOT))
        before, snapshot, registry, close_graphs = collect(
            ROOT, output, env, run, ['./internal/engine',
            './tools/rust-migration/review-quote-reference', package],
            allow_root=True, extra_files=[source])
        run('revision', ['git', 'rev-parse', 'HEAD'])
        run('format', ['cargo', 'fmt', '--manifest-path', 'rust/Cargo.toml', '--all', '--', '--check'])
        run('compile', ['cargo', 'check', *cargo, '-p', 'swarm-engine', '--all-features', '--all-targets'])
        run('clippy', ['cargo', 'clippy', *cargo, '-p', 'swarm-engine', '--all-features', '--all-targets', '--', '-D', 'warnings'])
        tests = ['cargo', 'test', *cargo, '-p', 'swarm-engine', '--lib', '--test', 'review_quote',
                 '--test', 'review_contract', '--test', 'managed_review_context', '--test', 'request']
        actual_raw = run('native-tests', tests).decode()
        listed_raw = run('native-inventory', tests + ['--', '--list']).decode()
        actual = Counter(line[5:-7] for line in actual_raw.splitlines()
                         if line.startswith('test ') and line.endswith(' ... ok'))
        listed = Counter(line[:-6] for line in listed_raw.splitlines() if line.endswith(': test'))
        assert actual and actual == listed, (actual - listed, listed - actual)
        save('native-test-names.json', list(actual.elements()))
        go = run('go-regressions', ['go', 'test', './internal/engine', '-run',
                 '^(' + '|'.join(GO_TESTS) + ')$', '-count=1', '-timeout=0', '-v']).decode()
        passed = {line.split()[2] for line in go.splitlines() if line.startswith('--- PASS: ')}
        assert passed == set(GO_TESTS) and '--- SKIP:' not in go, passed
        save('go-test-names.json', sorted(passed))
        reference = work / 'go-reference'
        run('reference-build', ['go', 'build', '-o', reference, source])
        run('go-vet', ['go', 'vet', './internal/engine', './tools/rust-migration/review-quote-reference', package])
        assert raw == run('reference-after', ['go', 'run', './tools/rust-migration/review-quote-reference'])
        run('probe-build', ['cargo', 'build', *cargo, '-p', 'swarm-engine', '--features',
                           'test-fixtures', '--example', 'review_quote_probe'])
        native = Path(env['CARGO_TARGET_DIR']) / 'debug/examples/review_quote_probe'
        cases = vectors()
        for case in cases:
            frame = case['frame']
            witness = dict(frame, evidence=frame['content'])
            for phase, value, present in (('control', witness, True), ('case', frame, case['present']),
                                           ('recovery', witness, True)):
                data = json.dumps(value).encode()
                go_result = json.loads(run(case['name'] + '-' + phase + '-go', [reference], data))
                rust_result = json.loads(run(case['name'] + '-' + phase + '-native', [native], data))
                assert go_result == rust_result == dict(present=present), (case['name'], phase)
        save('vectors.json', cases)
        for label, data, args in (('syntax', b'{', []), ('wrong-type', b'[]', []),
                                   ('missing-fields', b'{}', []), ('arguments', b'{}', ['unexpected'])):
            assert not run('native-refusal-' + label, [native, *args], data, expected=2)
            control = dict(name='proof.go', kind='source', content='literal', evidence='literal',
                           minimum_normalized_bytes=8)
            assert json.loads(run('native-recovery-' + label, [native], json.dumps(control).encode())) == dict(present=True)
        run('rustdoc', ['cargo', 'doc', *cargo, '-p', 'swarm-engine', '--no-deps'])
        doctest = run('doctest', ['cargo', 'test', *cargo, '-p', 'swarm-engine', '--doc']).decode()
        assert '1 passed; 0 failed' in doctest
        run('methods', ['python3', 'tools/agent-workflows/check.py'])
        run('diff-check', ['git', 'diff', '--check'])
        binaries = {}
        for name, path in (('go-reference', reference), ('native-quote-probe', native)):
            target = output / (name + '.binary')
            shutil.copyfile(path, target)
            target.chmod(0o600)
            binaries[name] = hashlib.sha256(target.read_bytes()).hexdigest()
        save('binaries.json', binaries)
        close_graphs()
        after = snapshot()
        assert before == after, 'effective input bytes changed during qualification'
        save('inputs-after.json', after)
        save('result.json', dict(status='PARTIAL', native_component_status='PASS',
             overall_migration='PARTIAL', scope='R3at44 original quote matching and actual Go parser fence repair',
             native_tests=sum(actual.values()), go_top_level_tests=len(passed),
             quote_vectors=len(cases), valid_control_and_recovery_each=True,
             native_transport_refusal_recovery=4, inputs=len(before), registry_packages=len(registry),
             source_stable=True, global_watchdog=None, provider_calls=False,
             independent_review=False, deployment=False, full_reply_parser_ported=False))
        print('PASS quote component; PARTIAL migration', flush=True)
    except BaseException as error:
        save('failure.json', dict(status='FAIL', cause=str(error), type=type(error).__name__))
        raise


if __name__ == '__main__':
    main()
