"""Qualify native public Work views and mutations against the actual Go CLI.

Private initialized roots only; no live service, providers or acceptance.
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
from qualification_inputs import collect, graph
from recorded_process import capture

ROOT = Path(__file__).resolve().parents[2]



def reuse_go_checks(prior, output, current, pattern):
    """Reuse named Go checks only with identical actual Go/root/embed inputs.

    Prior may have failed later UI checks; the Go test/inventory commands must
    have completed successfully with every selected top-level test passing. Both
    normal/race compiler graphs, Go environment and build settings must match.
    Current is the fresh shared collector snapshot. Returns names and writes an
    attributed proof; raises on any changed input or incomplete evidence, never
    silently reruns, skips or labels a failed package PASS. No runtime operation.
    """
    old = json.loads((prior / 'inputs-before.json').read_bytes())
    assert prior.is_relative_to(ROOT), 'Go evidence must belong to this checkout'
    assert json.loads((prior/'go-env.stdout').read_bytes()) == json.loads((output/'go-env.stdout').read_bytes())
    for label in ('go-version','cc-version'):
        assert (prior/(label+'.stdout')).read_bytes() == (output/(label+'.stdout')).read_bytes()
    for label in ('go-inputs-normal', 'go-inputs-race'):
        assert graph((prior/(label+'.stdout')).read_bytes()) == graph((output/(label+'.stdout')).read_bytes()), 'Go effective graph changed'
    assert json.loads((prior / 'build-environment.json').read_bytes()) == json.loads((output / 'build-environment.json').read_bytes()), 'build environment changed'
    embedded = json.loads((prior / 'root-embedded-inputs.json').read_bytes())
    assert embedded == json.loads((output / 'root-embedded-inputs.json').read_bytes())
    roots = {'go.mod', 'go.sum'}
    for group in embedded.values():
        for kind in ('GoFiles', 'CompiledGoFiles', 'EmbedFiles'):
            roots.update(group[kind])
    def selected(inputs):
        """Include actual root embeds, Go/test files and complete native Go graph data."""
        return {k: v for k, v in inputs.items() if k in roots or k.endswith('.go') or k.startswith(('internal/', 'cmd/', 'go-toolchain/', 'external-go/'))}
    assert selected(old) == selected(current), 'Go source/test/embed bytes changed'
    commands = {v['label']: v for v in json.loads((prior / 'commands.json').read_bytes())}
    for label in ('go-targeted', 'go-inventory'):
        assert commands[label]['exit'] == commands[label]['expected_exit'] == 0
    assert commands['go-targeted']['command'] == ['go','test','-timeout=0','-count=1','-json','./internal/engine','-run',pattern]
    assert commands['go-inventory']['command'] == ['go','test','-timeout=0','./internal/engine','-list',pattern]
    raw = (prior / 'go-targeted.stdout').read_text()
    rows = [json.loads(line) for line in raw.splitlines()]
    assert not any(r.get('Action') in ('fail', 'skip') for r in rows), 'Go failure/skip cannot be reused'
    names = [r['Test'] for r in rows if r.get('Action') == 'pass' and 'Test' in r and '/' not in r['Test']]
    listed = {line for line in (prior / 'go-inventory.stdout').read_text().splitlines() if line.startswith('Test')}
    assert names and set(names) == listed
    proof = dict(scope='Go named checks only, unchanged graph/test/embed bytes; prior UI failure remains FAIL',directory=str(prior),go_tests=len(names),names=names,input_count=len(selected(current)),files={label+'.'+ext:hashlib.sha256((prior/(label+'.'+ext)).read_bytes()).hexdigest() for label in ('go-targeted','go-inventory') for ext in ('stdout','stderr')},global_watchdog=None)
    path = output / 'reused-go-checks.json'
    with path.open('x') as stream:
        json.dump(proof, stream, indent=2);stream.write('\n')
    return names


def main():
    """Qualify public Work/gate paths or the full criteria/phase-policy increment.

    Requires a fresh evidence output and the shared explicit review baseline.
    Optional modes select affected test scopes; source review at close remains
    mandatory in every mode. SWARM_MIGRATION_SCOPE labels the explicit increment
    without altering test selection or product policy. Commands use owned fixtures, no global watchdog,
    provider or deployment. Failed prerequisites/behavior/review preserve outputs
    and raise; a component pass never establishes full migration acceptance.
    """
    if not __debug__ or len(sys.argv) not in (2,3) or len(sys.argv)==3 and sys.argv[2] not in ('gate','criteria','phases'):
        raise RuntimeError('active assertions and one fresh evidence directory required')
    phase_mode=len(sys.argv)==3 and sys.argv[2]=='phases'
    criteria_mode=len(sys.argv)==3 and sys.argv[2] in ('criteria','phases')
    gate_mode=len(sys.argv)==3
    output = Path(sys.argv[1]).resolve()
    output.mkdir(parents=True, exist_ok=False)
    work = Path(tempfile.mkdtemp(prefix='work-view-', dir=ROOT / '.cache'))
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
            ROOT, output, env, run, ['./cmd/swarm', './internal/engine'], allow_root=True, extra_files=[ROOT/p for p in ['frontend/engine-settings/admin.js','frontend/engine-settings/admin.css','tests/rust_engine_settings_admin_ui.cjs','tests/helpers/browser_verification_runtime.cjs','locales/en.json','web/i18n-en.js','package-lock.json']])
        cargo = ['--manifest-path', 'rust/Cargo.toml', '--locked', '--offline']
        run('revision', ['git', 'rev-parse', 'HEAD'])
        run('format', ['cargo', 'fmt', '--manifest-path', 'rust/Cargo.toml', '--all', '--', '--check'])
        selection=['-p','swarm-cli']if gate_mode and not criteria_mode else['--workspace']
        run('compile', ['cargo', 'check', *cargo, *selection, '--all-features', '--all-targets'])
        run('clippy', ['cargo', 'clippy', *cargo, *selection, '--all-features', '--all-targets', '--', '-D', 'warnings'])
        tests = ['cargo', 'test', *cargo, *selection, '--all-features', '--all-targets']
        actual_raw = run('workspace-tests', tests).decode()
        inventory = run('workspace-inventory', tests + ['--', '--list']).decode()
        actual = Counter(line[5:-7] for line in actual_raw.splitlines()
                         if line.startswith('test ') and line.endswith(' ... ok'))
        listed = Counter(line[:-6] for line in inventory.splitlines() if line.endswith(': test'))
        assert actual and actual == listed, (actual-listed, listed-actual)
        save('workspace-test-names.json', list(actual.elements()))
        go_names=[]
        if criteria_mode:
            run('review-guard-tests',['python3','-m','unittest','discover','-s','tools/rust-migration','-p','test_review_increment.py','-v'])
            # Shared creation and operation-field contracts affect these Go callers.
            # Named cases are inventoried; no global watchdog or provider calls.
            pattern='^(TestWorkCreation.*|TestWorkDefinitionRevision|TestRequestOperationFields.*|TestTaskCreation.*|TestTaskDefinition.*|TestDeclaredGateFixture.*|TestAcceptanceFreshnessAndAttempts|TestReopenedDependencyInvalidatesAcceptance|TestAutomaticValidationAcceptsAndUnblocksChainWithoutOperatorStep|TestAutomaticValidationFailureBlocksOnlyDependentBranch|TestRecoveredControl.*|TestControlRuntimeBudget.*|TestGatePreview.*|TestPostDelivery.*|TestRetexTaskExportAndVerifiedGuide|TestProjectMemory.*|TestParcoursInterruptionsParRegime)$'
            env['SWARM_MEMORY_RECIPE_BUNDLE']=str(work/'memory-browser-fixture.json')
            save('fixture-export-precondition.json',dict(scope='explicit isolated browser fixture export; no live state',path=env['SWARM_MEMORY_RECIPE_BUNDLE']))
            prior_go=env.get('SWARM_MIGRATION_GO_EVIDENCE')
            if prior_go:
                go_names=reuse_go_checks(Path(prior_go).resolve(),output,before,pattern)
            else:
                got=run('go-targeted',['go','test','-timeout=0','-count=1','-json','./internal/engine','-run',pattern]).decode()
                go_names=[r['Test']for line in got.splitlines()if (r:=json.loads(line)).get('Action')=='pass' and 'Test'in r and '/'not in r['Test']]
                listed_go=run('go-inventory',['go','test','-timeout=0','./internal/engine','-list',pattern]).decode()
                assert go_names and set(go_names)=={line for line in listed_go.splitlines()if line.startswith('Test')}
            save('go-test-names.json',go_names)
        reference = work / 'go-cli'
        run('go-build', ['go', 'build', '-o', reference, './cmd/swarm'])
        run('cli-build', ['cargo', 'build', *cargo, '-p', 'swarm-cli'])
        run('fixture-probe-build', ['cargo', 'build', *cargo, '-p', 'swarm-engine',
                                  '--features', 'test-fixtures', '--example', 'root_store_probe'])
        if criteria_mode:
            run('all-probes-build',['cargo','build',*cargo,'-p','swarm-engine','--features','test-fixtures','--examples'])
        target = Path(env['CARGO_TARGET_DIR']) / 'debug'
        native = target / 'swarm-rust'
        probe = target / 'examples/root_store_probe'
        fixture = Path(tempfile.mkdtemp(prefix='.swarm-rust-view-', dir=Path.home()))
        fixture.chmod(0o700)
        (fixture / '.rust-migration-fixture').touch()
        run('git-init', ['git', '-C', fixture, 'init', '--quiet'])
        (fixture/'tracked.txt').write_text('Fixture only\n')
        run('git-add', ['git', '-C', fixture, 'add', 'tracked.txt'])
        run('git-commit', ['git', '-C', fixture, '-c', 'user.name=Fixture', '-c',
                           'user.email=fixture@localhost', 'commit', '--quiet', '-m', 'Fixture'])
        run('native-init', [native, 'init', '--root', fixture, '--json'])
        comparisons = []
        def compare(label, selected=None):
            """Compare actual public projections; all reads preserve DB/settings/views."""
            database = fixture / '.swarm/state.db'
            settings = fixture / '.swarm/engine-settings.json'
            original, policy = database.read_bytes(), settings.read_bytes()
            files={p.name:p.read_bytes()for p in (fixture/'.swarm/views').iterdir()}
            for language in ['fr', 'en']:
                operations=[('list-json',['work','list','--json']),('list-text',['work','list'])]
                if selected:
                    operations += [('show-json',['work','show',selected,'--json']),
                                   ('show-text',['work','show',selected])]
                for operation,args in operations:
                    args += ['--lang',language,'--root',fixture]
                    go = run(label+'-'+language+'-'+operation+'-go', [reference, *args])
                    rust = run(label+'-'+language+'-'+operation+'-native', [native, *args])
                    if operation.endswith('json'):
                        left,right=json.loads(go),json.loads(rust)
                        assert left==right, (label,operation,language,left,right)
                    else:
                        assert go==rust,(label,operation,language,go,rust)
                    assert database.read_bytes()==original and settings.read_bytes()==policy
                    assert {p.name:p.read_bytes()for p in (fixture/'.swarm/views').iterdir()}==files
                    comparisons.append(dict(name=label,language=language,operation=operation,unchanged=True))
        compare('empty')
        selected=None
        revision=0
        steps=[
            ('create',['work','create'],dict(title='Premier <&> projet',objective='Objective',scope='Private fixture',criteria=['Proof'])),
            ('update',['work','update'],dict(title='Mission \"projet\" \u2028 fin')),
            ('task-add',['task','add'],dict(id='task-a',title='Task',owner='human',deliverable='Result',criteria=['Proof'])),
            ('task-update',['task','update'],dict(id='task-a',status='blocked',blocker='Waiting',next='Resolve')),
            ('ooda',['ooda'],dict(observation='Fact',orientation='Context',decision='Proceed',owner='human',next='Implement',result='Recorded')),
            ('checkpoint',['checkpoint'],dict(summary='Checkpoint saved',next='Continue')),
        ]
        for label,args,fields in steps:
            request=dict(schema_version=1,event_id='public-'+label,expected_revision=revision,**fields)
            path=work/(label+'.json');path.write_text(json.dumps(request))
            if selected:args += [selected]
            args += ['--root',fixture,'--input',path,'--json']
            result=json.loads(run(label+'-mutation',[native,*args]))
            assert result['work']['revision']==revision+1
            selected=result['work']['id'];revision=result['work']['revision']
            replay=json.loads(run(label+'-replay',[native,*args]))
            assert result==replay
            compare(label,selected)
        gate_comparisons=[]
        if gate_mode:
            # Generate required IDs from the actual stored task producer.
            request=dict(schema_version=1,event_id='gate-criteria',expected_revision=revision,
                         id='task-a',criteria=['Actual gate criterion'])
            path=work/'gate-criteria.json';path.write_text(json.dumps(request))
            result=json.loads(run('gate-criteria',[native,'task','update',selected,'--input',path,'--root',fixture,'--json']))
            revision=result['work']['revision'];task=result['work']['tasks'][0]
            checks=[dict(id=key,gate=phase,mandatory=True,severity='minor',penalty=1,max_penalty=100)
                    for key,phase in task['plan_checks'].items()]
            checks.append(dict(id='fixture-blocking',domain='quality',gate='delivery',mandatory=True,severity='minor',penalty=1,max_penalty=100))
            (fixture/'proof.txt').write_bytes(b'proof')
            document=dict(method_version='2',scope_id='task-a',artifacts={'proof.txt':hashlib.sha256(b'proof').hexdigest()},
                          domains={'quality':1},checks=checks,results=[dict(id=c['id'],status='PASS',count=0,evidence=['proof.txt'])for c in checks])
            gate_request=dict(schema_version=1,event_id='gate-pass',expected_revision=revision,task_id='task-a',phase='delivery',name='Gate <&> fixture',document=document)
            path=work/'gate-pass.json';path.write_text(json.dumps(gate_request))
            args=['gate',selected,'--root',fixture,'--input',path,'--json']
            native_pass=json.loads(run('gate-pass-native',[native,*args]));revision+=1
            assert native_pass['work']['revision']==revision and native_pass['current_gates']['task-a']['evaluation']['allowed']
            database=fixture/'.swarm/state.db'
            original=database.read_bytes()
            for language in ['fr','en']:
                go=json.loads(run('gate-pass-go-replay-'+language,[reference,*args,'--lang',language]))
                rust=json.loads(run('gate-pass-native-replay-'+language,[native,*args,'--lang',language]))
                assert go==rust==native_pass and database.read_bytes()==original
                gate_comparisons.append(dict(name='pass-replay',language=language,equal=True))
            for defect in ['task-mismatch','caller-policy']:
                # Fresh valid witnesses with identical stored revisions before each injection.
                run('control-'+defect+'-native',[native,*args]);run('control-'+defect+'-go',[reference,*args])
                invalid=json.loads(json.dumps(gate_request));invalid['event_id']='fault-'+defect;invalid['expected_revision']=revision
                if defect=='caller-policy':invalid['policy']={'generation':0}
                fault=work/(defect+'.json');fault.write_text(json.dumps(invalid))
                options=['gate',selected,'--root',fixture,'--input',fault,'--json']
                if defect=='task-mismatch':options+=['--task','different']
                before_fault=database.read_bytes()
                for name,binary in [('native',native),('go',reference)]:
                    assert run('refuse-'+defect+'-'+name,[binary,*options],expected=2)==b''
                    assert database.read_bytes()==before_fault
                run('recover-'+defect+'-native',[native,*args]);run('recover-'+defect+'-go',[reference,*args])
            gate_request['event_id']='gate-denied';gate_request['expected_revision']=revision
            next(r for r in gate_request['document']['results']if r['id']=='fixture-blocking').update(status='FAIL',count=1)
            denied_path=work/'gate-denied.json';denied_path.write_text(json.dumps(gate_request))
            denied_args=['gate',selected,'--root',fixture,'--input',denied_path,'--json']
            denied=json.loads(run('gate-denied-native',[native,*denied_args],expected=1));revision+=1
            assert denied['work']['revision']==revision and not denied['current_gates']['task-a']['evaluation']['allowed']
            original=database.read_bytes()
            for language in ['fr','en']:
                go=json.loads(run('gate-denied-go-replay-'+language,[reference,*denied_args,'--lang',language],expected=1))
                rust=json.loads(run('gate-denied-native-replay-'+language,[native,*denied_args,'--lang',language],expected=1))
                assert go==rust==denied and database.read_bytes()==original
                gate_comparisons.append(dict(name='denied-replay',language=language,equal=True))
            compare('after-gates',selected)
            save('gate-comparisons.json',gate_comparisons)
        for language in ['fr','en']:
            result=json.loads(run('resume-'+language,[native,'resume',selected,'--root',fixture,'--json','--lang',language]))
            assert (fixture/'.swarm/views'/str(selected+'.md')).read_text()==result['resume_markdown']
        if not gate_mode:
            # Identical before/after diagnostic vectors; no arbitrary timing cutoff.
            run('projection-probe-build',['cargo','build',*cargo,'-p','swarm-engine','--features','test-fixtures','--example','work_projection_probe'])
            projection=target/'examples/work_projection_probe'
            measures=[]
            history=work/'scale-fixture';history.mkdir(mode=0o700)
            (history/'.rust-migration-fixture').touch()
            connection=sqlite3.connect(history/'history.db')
            connection.execute('CREATE TABLE events(work_id TEXT,kind TEXT,revision INTEGER,payload BLOB)')
            connection.commit();connection.close()
            prior=ROOT/'docs/plans/migration-rust/evidence/20261018-51'
            for count in [100,500,1000]:
                old=prior/('scale-before-'+str(count)+'.stdin')
                if not old.exists():
                    old=prior/('scale-before-'+str(count)+'.input')
                data=old.read_bytes();start=time.monotonic()
                observed=run('scale-after-'+str(count),[projection,'pure',history],data)
                elapsed=time.monotonic()-start
                original=(prior/('scale-before-'+str(count)+'.stdout')).read_bytes()
                assert observed==original, 'bulk freshness changed projection'
                measures.append(dict(tasks=count,seconds=elapsed,identical_output=True))
            save('scale-after.json',dict(scope='debug diagnostic only; no production performance claim',measurements=measures))
        admin_checks=[]
        if criteria_mode:
            run('browser-runtime-guards',['node','tests/browser_verification_runtime_test.cjs'])
            run('admin',['node','tests/rust_engine_settings_admin_ui.cjs',native,probe,output/'admin',target/'examples/project_context_probe',target/'examples/git_report_probe',target/'examples/baseline_files_probe',target/'examples/review_reply_probe',target/'examples/work_projection_probe'])
            from verify_admin_programs import verify_runtime_archive
            save('browser-runtime-archive.json',verify_runtime_archive(output/'admin'))
            admin=json.loads((output/'admin/result.json').read_text())
            assert admin['status']=='PASS'and len(admin['checks'])==4 and not admin['errors']
            admin_checks=admin['checks']
            assert all(c['saved_admin_criterion_limit_changes_creation']and c['criterion_creation_finite_null_and_recovery']for c in admin_checks)
            if phase_mode:
                assert all(c['saved_admin_manual_phases_change_new_tasks']and c['manual_phases_empty_and_existing_contract_preserved']for c in admin_checks)
        save('public-comparisons.json',comparisons)
        run('go-vet', ['go', 'vet', './cmd/swarm', './internal/engine'])
        run('rustdoc', ['cargo', 'doc', *cargo, *selection, '--all-features', '--no-deps'])
        run('doctests',['cargo','test',*cargo,*selection,'--all-features','--doc'])
        run('i18n-build-check',['node','scripts/i18n/build.cjs','--check'])
        run('i18n',['npm','run','test:i18n'])
        run('methods', ['python3', 'tools/agent-workflows/check.py'])
        run('diff-check', ['git', 'diff', '--check'])
        binaries = {}
        for name, source in [('go-cli', reference), ('native-cli', native), ('root-fixture-probe', probe)]:
            destination = output / (name+'.binary')
            shutil.copyfile(source, destination)
            destination.chmod(0o600)
            binaries[name] = hashlib.sha256(destination.read_bytes()).hexdigest()
        save('binaries.json', binaries)
        close_graphs()
        after = snapshot()
        assert before == after, 'effective inputs changed during verification'
        save('inputs-after.json', after)
        save('result.json', dict(status='PARTIAL', native_component_status='PASS',
             scope=env.get('SWARM_MIGRATION_SCOPE') or ('R3ax54 persisted manual gate phase defaults and contract preservation'if phase_mode else'R3aw53 task criterion binding, persisted limits and safe reasons'if criteria_mode else'R5f52 public gate and basic Work routing'if gate_mode else'R5e51 public Work views'),
             go_tests=len(go_names),go_checks_reused_from=env.get('SWARM_MIGRATION_GO_EVIDENCE'),admin_checks=admin_checks,
             native_tests=sum(actual.values()),test_scope='swarm-cli all targets'if gate_mode and not criteria_mode else'workspace all targets', public_comparisons=len(comparisons),gate_comparisons=len(gate_comparisons),
             database_failure_and_recovery=True, output_error_and_recovery=True,
             inputs=len(before), registry_packages=len(registry), source_stable=True,
             global_watchdog=None, fixture_root=str(fixture), provider_calls=False,
             independent_review=False, deployment=False))
        print('PASS public Work views; PARTIAL migration', flush=True)
    except BaseException as error:
        save('failure.json', dict(status='FAIL', cause=str(error), type=type(error).__name__))
        raise


if __name__ == '__main__':
    main()
