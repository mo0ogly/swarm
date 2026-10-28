"""Qualify current validation-data guards against the actual Go implementation.

Ordinary private fixtures only; success proves no execution provenance or authority.
"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import sys
import tempfile
import time
from collections import Counter
from qualification_inputs import collect
from recorded_process import capture

ROOT = Path(__file__).resolve().parents[2]


def main():
    """Freeze effective inputs; require actual named tests, exact facts and recovery."""
    if not __debug__ or len(sys.argv) != 2:
        raise RuntimeError('active assertions and one fresh evidence directory required')
    output = Path(sys.argv[1]).resolve()
    output.mkdir(parents=True, exist_ok=False)
    work = Path(tempfile.mkdtemp(prefix='validation-evidence-', dir=ROOT / '.cache'))
    env = dict(os.environ)
    commands = []

    def save(name, value):
        """Publish completed records atomically, preserving an existing record on failure."""
        path = output / name
        fd, temporary = tempfile.mkstemp(dir=path.parent, prefix='.' + path.name)
        with os.fdopen(fd, 'w') as stream:
            json.dump(value, stream, indent=2)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)

    def run(label, args, data=None, expected=0):
        """Stream original outputs with no global watchdog; reject an unexpected exit."""
        args = list(map(str, args))
        started = datetime.datetime.now(datetime.timezone.utc).isoformat()
        start = time.monotonic()
        result = capture(args, cwd=ROOT, env=env, data=data,
                         stdout_path=output / (label + '.stdout'),
                         stderr_path=output / (label + '.stderr'))
        if data is not None:
            (output / (label + '.stdin')).write_bytes(data)
        commands.append(dict(label=label, command=args, start_utc=started,
                             seconds=time.monotonic()-start, exit=result.returncode,
                             expected_exit=expected))
        save('commands.json', commands)
        print(label, 'exit', result.returncode, flush=True)
        if result.returncode != expected:
            raise RuntimeError(label + ': original failure retained')
        return result.stdout

    try:
        before, snapshot, registry, close_graphs = collect(
            ROOT, output, env, run, ['./internal/engine'], allow_root=True)
        cargo = ['--manifest-path', 'rust/Cargo.toml', '--locked', '--offline']
        run('revision', ['git', 'rev-parse', 'HEAD'])
        run('format', ['cargo', 'fmt', '--manifest-path', 'rust/Cargo.toml', '--all', '--', '--check'])
        run('compile', ['cargo', 'check', *cargo, '--workspace', '--all-features', '--all-targets'])
        run('clippy', ['cargo', 'clippy', *cargo, '--workspace', '--all-features', '--all-targets', '--', '-D', 'warnings'])
        tests = ['cargo', 'test', *cargo, '-p', 'swarm-engine', '--lib', '--test', 'validation_evidence',
                 '--test', 'gate_contract', '--test', 'root_store']
        actual_raw = run('native-tests', tests).decode()
        inventory = run('native-inventory', tests + ['--', '--list']).decode()
        actual = Counter(line[5:-7] for line in actual_raw.splitlines()
                         if line.startswith('test ') and line.endswith(' ... ok'))
        listed = Counter(line[:-6] for line in inventory.splitlines() if line.endswith(': test'))
        assert actual and actual == listed, (actual-listed, listed-actual)
        save('native-test-names.json', list(actual.elements()))
        reference = work / 'go-validation'
        run('reference-build', ['go', 'test', '-c', '-o', reference, './internal/engine'])
        run('probe-build', ['cargo', 'build', *cargo, '-p', 'swarm-engine', '--features',
                            'test-fixtures', '--example', 'validation_evidence_probe'])
        native = Path(env['CARGO_TARGET_DIR']) / 'debug/examples/validation_evidence_probe'
        fixture = work / 'fixture'
        fixture.mkdir(mode=0o700)
        (fixture / '.rust-migration-fixture').touch()
        env['SWARM_RUST_VALIDATION_FIXTURE'] = str(fixture)
        receipt_file = fixture / 'receipt.json'
        original = b'original receipt bytes'
        receipt_file.write_bytes(original)
        digest = hashlib.sha256(original).hexdigest()
        control = dict(review_output=True, id='control', command=['cargo', 'test'],
                       criteria=None, justification='')
        policy = dict(mode='human', controls=[control])

        def policy_digest(policy):
            """Explicit Go producer shape: attributed fields omitted, required zero fields retained."""
            p = dict(mode=policy.get('mode',''), controls=policy.get('controls'))
            raw=json.dumps(p, separators=(',', ':'), ensure_ascii=False)
            for value, escaped in [('<','\\u003c'),('>','\\u003e'),('&','\\u0026'),('\u2028','\\u2028'),('\u2029','\\u2029')]:
                raw=raw.replace(value,escaped)
            return hashlib.sha256(raw.encode()).hexdigest()
        base = dict(id='task', attempts=[dict(id='attempt')], validation_policy=policy,
                    automatic_validation=dict(attempt_id='attempt', controller='controller://swarm-validation',
                    policy_digest=policy_digest(policy), receipt='receipt.json', state='pending_human',
                    controls=[dict(id='control', command=['cargo','test'], executed=True, passed=True,
                                   exit_code=0, started_at='start', finished_at='finish',
                                   review_output='proof', output_bytes=5)], artifacts={'receipt.json':digest}))
        import copy
        cases = []

        def add(name, mutate, ok):
            """Retain independent complete inputs and expected business result before execution."""
            task = copy.deepcopy(base)
            mutate(task)
            cases.append(dict(name=name, task=task, ok=ok))
        for state in ['pending_review','accepted','pending_human']:
            add('state-'+state,lambda t,s=state:t['automatic_validation'].update(state=s),True)
        for field,value in [('attempt_id','other'),('controller','other'),('policy_digest','wrong'),
                            ('state','failed'),('receipt',''),('artifacts',None),('controls',[])]:
            add('receipt-'+field,lambda t,k=field,v=value:t['automatic_validation'].update({k:v}),False)
        for field,value in [('id','other'),('command',None),('command',[]),('executed',False),
                            ('passed',False),('exit_code',1),('started_at',''),('finished_at',''),
                            ('output_bytes',-1),('output_bytes',4),('review_output_truncated',True)]:
            add('control-'+field+'-'+str(value),lambda t,k=field,v=value:t['automatic_validation']['controls'][0].update({k:v}),False)
        add('truncated-consistent',lambda t:t['automatic_validation']['controls'][0].update(output_bytes=9,review_output_truncated=True),True)
        add('empty-output',lambda t:t['automatic_validation']['controls'][0].update(output_bytes=0,review_output=''),True)
        add('utf8-over-limit',lambda t:t['automatic_validation']['controls'][0].update(output_bytes=65538,review_output='é'*32769),False)
        add('utf8-exact-limit',lambda t:t['automatic_validation']['controls'][0].update(output_bytes=65536,review_output='é'*32768),True)
        add('no-policy',lambda t:t.update(validation_policy=None),True)
        add('empty-policy',lambda t:t.update(validation_policy=dict(mode='human',controls=[])),True)
        add('no-receipt',lambda t:t.update(automatic_validation=None),False)
        add('wrong-order',lambda t:t['automatic_validation']['controls'].append(dict(t['automatic_validation']['controls'][0])),False)
        add('policy-attribution-only',lambda t:t['validation_policy'].update(actor='human <&>',authorized_at='date'),True)
        add('policy-command-changed',lambda t:t['validation_policy']['controls'][0].update(command=['other']),False)
        add('artifact-membership',lambda t:t['automatic_validation'].update(artifacts={}),False)
        add('artifact-escape',lambda t:t['automatic_validation']['artifacts'].update({'../private':'hash'}),False)
        for shared in [False,True]:
            def sharing(t, shared=shared):
                p=t['validation_policy'];p['controls'][0]['review_output']=False
                # Go omits the false optional field in the digest.
                shaped=copy.deepcopy(p);del shaped['controls'][0]['review_output']
                t['automatic_validation']['policy_digest']=policy_digest(shaped)
                if not shared:t['automatic_validation']['controls'][0].update(review_output='',output_bytes=0)
            add('sharing-disabled-'+str(shared),sharing,not shared)
        results = []

        def compare(label, task, ok):
            """Assert actual guard data, non-skipped Go transport, and unchanged artifact bytes."""
            frame=dict(task=task,max_review_output_bytes=65536)
            data=json.dumps(frame,ensure_ascii=False).encode()
            (fixture/'validation-probe.json').write_bytes(data)
            before_file=receipt_file.read_bytes() if receipt_file.exists() else None
            go_raw=run(label+'-go',[reference,'-test.run=^TestRustMigrationValidationEvidenceProbe$',
                                  '-test.v','-test.timeout=0']).decode()
            assert '--- PASS: TestRustMigrationValidationEvidenceProbe' in go_raw and '--- SKIP:' not in go_raw
            marked=[s.removeprefix('RUST_VALIDATION_RESULT ') for s in go_raw.splitlines() if s.startswith('RUST_VALIDATION_RESULT ')]
            assert len(marked)==1
            go=json.loads(marked[0]);rust=json.loads(run(label+'-native',[native,fixture],data))
            assert go==rust and rust['ok']==ok,(label,go,rust)
            after_file=receipt_file.read_bytes() if receipt_file.exists() else None
            assert before_file==after_file
            results.append(dict(name=label,ok=ok,artifact_unchanged=True))
        save('vectors.json',cases)
        for i,case in enumerate(cases):
            name=str(i)+'-'+case['name']
            compare(name+'-control',base,True)
            compare(name,case['task'],case['ok'])
            compare(name+'-recovery',base,True)
        for fault in ['changed','missing']:
            compare(fault+'-control',base,True)
            if fault=='changed':receipt_file.write_bytes(b'changed')
            else:receipt_file.rename(fixture/'retained.json')
            compare(fault+'-refusal',base,False)
            if fault=='changed':receipt_file.write_bytes(original)
            else:(fixture/'retained.json').rename(receipt_file)
            compare(fault+'-recovery',base,True)
        save('comparisons.json',results)
        go_tests=['TestUnconfiguredValidationReviewNeverIndexesHistoricalResults',
                  'TestReviewOutputRequiresExplicitAuthorization',
                  'TestReviewOutputExplicitlyMarksTruncatedObservation',
                  'TestReviewOutputFlowsThroughPersistedEngineReceipt',
                  'TestReviewOutputLiteralQuotationSourcesAreBoundedAndExact']
        go_raw=run('go-regressions',['go','test','./internal/engine','-run','^('+'|'.join(go_tests)+')$',
                                  '-count=1','-timeout=0','-v']).decode()
        passed={line.split()[2]for line in go_raw.splitlines()if line.startswith('--- PASS: ')}
        assert passed==set(go_tests) and '--- SKIP:' not in go_raw,passed
        save('go-test-names.json',sorted(passed))
        run('go-vet',['go','vet','./internal/engine'])
        run('rustdoc',['cargo','doc',*cargo,'-p','swarm-engine','--no-deps'])
        doc=run('doctest',['cargo','test',*cargo,'-p','swarm-engine','--doc']).decode()
        assert '1 passed; 0 failed' in doc
        run('methods',['python3','tools/agent-workflows/check.py'])
        run('diff-check',['git','diff','--check'])
        binaries={}
        for name,source in [('go-validation',reference),('native-validation',native)]:
            destination=output/(name+'.binary');shutil.copyfile(source,destination);destination.chmod(0o600)
            binaries[name]=hashlib.sha256(destination.read_bytes()).hexdigest()
        save('binaries.json',binaries)
        close_graphs();after=snapshot();assert before==after,'effective inputs changed during qualification'
        save('inputs-after.json',after)
        save('result.json',dict(status='PARTIAL',native_component_status='PASS',
             scope='R3av48 validation data guard, not full TaskEvidence/public Work rendering',
             native_tests=sum(actual.values()),go_top_level_tests=len(passed),scenarios=len(cases),comparisons=len(results),
             valid_control_and_recovery_each=True,artifact_failure_recovery=2,
             root_policy_save_reopen_consumer_verified=True,inputs=len(before),registry_packages=len(registry),
             source_stable=True,global_watchdog=None,provider_calls=False,independent_review=False,deployment=False))
        print('PASS validation-data guard; PARTIAL migration',flush=True)
    except BaseException as error:
        save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__))
        raise


if __name__ == '__main__':
    main()
