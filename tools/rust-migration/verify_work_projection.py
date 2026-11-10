"""Qualify Work projections and persisted v8 presentation, without acceptance claims.

Actual Go read models, private SQL/files and real Admin browser journeys.
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
    work = Path(tempfile.mkdtemp(prefix='work-projection-', dir=ROOT / '.cache'))
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
            ROOT, output, env, run, ['./internal/engine'], allow_root=True, extra_files=[ROOT/p for p in ['frontend/engine-settings/admin.js','frontend/engine-settings/admin.css','tests/rust_engine_settings_admin_ui.cjs','tests/helpers/browser_verification_runtime.cjs','locales/en.json','web/i18n-en.js','package-lock.json']])
        cargo = ['--manifest-path', 'rust/Cargo.toml', '--locked', '--offline']
        run('revision', ['git', 'rev-parse', 'HEAD'])
        run('format', ['cargo', 'fmt', '--manifest-path', 'rust/Cargo.toml', '--all', '--', '--check'])
        run('compile', ['cargo', 'check', *cargo, '--workspace', '--all-features', '--all-targets'])
        run('clippy', ['cargo', 'clippy', *cargo, '--workspace', '--all-features', '--all-targets', '--', '-D', 'warnings'])
        tests=['cargo','test',*cargo,'--workspace','--all-features','--all-targets']
        actual_raw=run('workspace-tests',tests).decode();inventory=run('workspace-inventory',tests+['--','--list']).decode()
        actual=Counter(line[5:-7]for line in actual_raw.splitlines()if line.startswith('test ')and line.endswith(' ... ok'))
        listed=Counter(line[:-6]for line in inventory.splitlines()if line.endswith(': test'))
        assert actual and actual==listed,(actual-listed,listed-actual)
        save('native-test-names.json',list(actual.elements()))
        reference=work/'go-projection';run('reference-build',['go','test','-c','-o',reference,'./internal/engine'])
        run('cli-build',['cargo','build',*cargo,'-p','swarm-cli'])
        run('probe-build',['cargo','build',*cargo,'-p','swarm-engine','--features','test-fixtures','--examples'])
        target=Path(env['CARGO_TARGET_DIR'])/'debug';native=target/'swarm-rust';probe=target/'examples/work_projection_probe'
        fixture=work/'fixture';fixture.mkdir(mode=0o700);(fixture/'.rust-migration-fixture').touch()
        env['SWARM_RUST_WORK_PROJECTION_FIXTURE']=str(fixture)
        dbpath=fixture/'history.db'
        import copy
        cases=[]
        base=dict(schema_version=1,id='work',revision=1,tasks=[dict(id='task',status='todo')])
        def add(name,mutate,history=None):
            """Keep complete inputs and routing/receipt data before comparing real readers."""
            w=copy.deepcopy(base);mutate(w);cases.append(dict(name=name,work=w,history=history or []))
        add('empty',lambda w:w.update(tasks=[]))
        for status in ['todo','running','blocked','submitted','accepted','waived','abandoned']:
            add('status-'+status,lambda w,s=status:w['tasks'][0].update(status=s))
        add('valid-waiver',lambda w:w['tasks'][0].update(status='waived',manual_override=dict(reason='Operator exception',at='date')))
        add('blank-waiver',lambda w:w['tasks'][0].update(status='waived',manual_override=dict(reason=' ')))
        add('partial',lambda w:w.update(tasks=[dict(id='a',status='waived',manual_override=dict(reason='Exception')),dict(id='b',status='abandoned')]))
        add('dependency-missing',lambda w:w['tasks'][0].update(status='waived',manual_override=dict(reason='Exception'),depends=['missing']))
        add('report-review-only',lambda w:w['tasks'][0].update(independent_review=dict(state='passed',attempt='review-attempt',reviewer='reviewer',started='start')))
        add('control-empty',lambda w:w['tasks'][0].update(automatic_validation=dict(controls=[])))
        add('controls-mixed',lambda w:w['tasks'][0].update(automatic_validation=dict(revision=2,controls=[dict(id='pass',executed=True,passed=True,wall_duration_ms=0),dict(id='unknown'),dict(id='fail',started_at='start',exit_code=3,wall_duration_ms=9)])))
        add('controls-passed-unknown',lambda w:w['tasks'][0].update(automatic_validation=dict(controls=[dict(id='pass',executed=True,passed=True),dict(id='unknown')])))
        add('controls-all-unknown',lambda w:w['tasks'][0].update(automatic_validation=dict(controls=[dict(id='unknown')])))
        add('command-fallback',lambda w:w['tasks'][0].update(automatic_validation=dict(policy=dict(controls=[dict(id='c',command=['cargo','test'])]),controls=[dict(id='c',executed=True,passed=True)])))
        historical=dict(attempt_id='attempt',receipt='old',revision=3,controls=[dict(id='historical',executed=True,passed=True,wall_duration_ms=100)])
        add('history',lambda w:w['tasks'][0].update(attempts=[dict(id='attempt')]),[historical])
        add('history-current-excluded',lambda w:w['tasks'][0].update(attempts=[dict(id='attempt')],automatic_validation=historical),[historical])
        add('history-foreign',lambda w:w['tasks'][0].update(attempts=[dict(id='other')]),[historical])
        add('history-deduplicated',lambda w:w['tasks'][0].update(attempts=[dict(id='attempt')]),[historical,historical])
        many=[dict(attempt_id='attempt',receipt='r'+str(i),controls=[dict(id='c'+str(i),wall_duration_ms=i)])for i in range(8)]
        add('history-top-five',lambda w:w['tasks'][0].update(attempts=[dict(id='attempt')]),many)
        def prepare(history):
            """Own a closed minimal SQL fixture; production roots remain untouched."""
            if dbpath.exists():dbpath.unlink()
            db=sqlite3.connect(dbpath)
            try:
                db.execute('CREATE TABLE events(work_id TEXT,kind TEXT,revision INTEGER,payload BLOB)')
                for i,r in enumerate(history):db.execute('INSERT INTO events VALUES(?,?,?,?)',('work','task.auto-validation',i,json.dumps(r).encode()))
                db.commit()
            finally:db.close()
        comparisons=[]
        for i,case in enumerate(cases):
            for phase,current in [('control',dict(work=base,history=[])),('case',case),('recovery',dict(work=base,history=[]))]:
                prepare(current['history'])
                for language in ['fr','en']:
                    frame=dict(work=current['work'],language=language);data=json.dumps(frame).encode();(fixture/'work-probe.json').write_bytes(data)
                    label=str(i)+'-'+phase+'-'+language;original=dbpath.read_bytes()
                    raw=run(label+'-go',[reference,'-test.run=^TestRustMigrationWorkProjectionProbe$','-test.v','-test.timeout=0']).decode()
                    assert '--- PASS: TestRustMigrationWorkProjectionProbe' in raw and '--- SKIP:' not in raw
                    marked=[s.removeprefix('RUST_WORK_PROJECTION_RESULT ')for s in raw.splitlines()if s.startswith('RUST_WORK_PROJECTION_RESULT ')]
                    assert len(marked)==1
                    go=json.loads(marked[0]);rust=json.loads(run(label+'-native',[probe,'pure',fixture],data))
                    assert go==rust,(case['name'],phase,language,go,rust)
                    assert dbpath.read_bytes()==original
                    comparisons.append(dict(name=case['name'],phase=phase,language=language,unchanged=True))
        save('vectors.json',cases);save('comparisons.json',comparisons)
        run('go-vet',['go','vet','./internal/engine'])
        run('rustdoc',['cargo','doc',*cargo,'-p','swarm-engine','--no-deps'])
        doc=run('doctest',['cargo','test',*cargo,'-p','swarm-engine','--doc']).decode();assert '1 passed; 0 failed' in doc
        run('i18n-build-check',['node','scripts/i18n/build.cjs','--check']);run('i18n-tests',['npm','run','test:i18n'])
        run('admin',['node','tests/rust_engine_settings_admin_ui.cjs',native,target/'examples/root_store_probe',output/'admin',target/'examples/project_context_probe',target/'examples/git_report_probe',target/'examples/baseline_files_probe',target/'examples/review_reply_probe',probe])
        admin=json.loads((output/'admin/result.json').read_text());fields=len(json.loads((ROOT/'rust/crates/swarm-engine/config/engine-settings-catalog.json').read_text())['fields'])
        assert admin['status']=='PASS' and len(admin['checks'])==4
        assert all(c['fields']==fields and c['saved_admin_work_projection_policies'] and c['work_history_is_declared_fixture_data']for c in admin['checks'])
        run('methods',['python3','tools/agent-workflows/check.py']);run('diff-check',['git','diff','--check'])
        binaries={}
        for name,source in [('go-projection',reference),('native-cli',native),('projection-probe',probe)]:
            destination=output/(name+'.binary');shutil.copyfile(source,destination);destination.chmod(0o600);binaries[name]=hashlib.sha256(destination.read_bytes()).hexdigest()
        save('binaries.json',binaries);close_graphs();after=snapshot();assert before==after,'effective inputs changed during qualification';save('inputs-after.json',after)
        save('result.json',dict(status='PARTIAL',native_component_status='PASS',scope='R5c49 Work projections and v8 presentation; public rendering/hierarchy pending',native_tests=sum(actual.values()),scenarios=len(cases),comparisons=len(comparisons),admin_journeys=4,admin_fields=fields,source_stable=True,inputs=len(before),registry_packages=len(registry),global_watchdog=None,provider_calls=False,independent_review=False,deployment=False))
        print('PASS Work projections/presentation; PARTIAL migration',flush=True)
    except BaseException as error:
        save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__));raise


if __name__=='__main__':main()
