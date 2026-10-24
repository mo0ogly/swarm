"""Native root/settings qualification. No global watchdog and no live root use."""
from collections import Counter
import datetime,hashlib,json,os,shutil,sqlite3,subprocess,sys,tempfile,time
from pathlib import Path
from qualification_inputs import collect
from qualify_validation_run import qualify as qualify_controls
from qualify_score import qualify as qualify_scores
from qualify_gate_store import qualify as qualify_gates
from qualify_project_context import qualify as qualify_context
ROOT=Path(__file__).resolve().parents[2]
def main():
    output=Path(sys.argv[1]).resolve()
    if output.exists():raise RuntimeError('Fresh evidence directory required')
    output.mkdir(parents=True)
    work=Path(tempfile.mkdtemp(prefix='swarm-root-',dir=ROOT/'.cache'))
    scratch=Path(tempfile.mkdtemp(prefix='.swarm-rust-root-',dir=Path.home().resolve()))
    for path in [scratch,*scratch.parents]:
        s=path.stat();assert s.st_uid in [0,os.geteuid()]and not s.st_mode&0o22,'unsafe fixture ancestry'
    env=dict(os.environ,TMPDIR=str(scratch),GOTMPDIR=str(scratch),CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(work/'target')))
    commands=[]
    def save(name,value):
        path=output/name;fd,tmp=tempfile.mkstemp(dir=path.parent,prefix='.'+path.name+'.')
        with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
        os.replace(tmp,path)
    def execute(label,args,data=None,expected=0):
        start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic()
        p=subprocess.run(args,cwd=ROOT,env=env,input=data,capture_output=True)
        if data is not None:(output/(label+'.stdin')).write_bytes(data)
        for ext,raw in [('stdout',p.stdout),('stderr',p.stderr)]:(output/(label+'.'+ext)).write_bytes(raw)
        commands.append(dict(label=label,command=args,start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));save('commands.json',commands);print(label,'exit',p.returncode,flush=True)
        if expected is not None and p.returncode!=expected:raise RuntimeError(label+': original failure retained')
        return p
    def run(label,args,data=None):return execute(label,args,data).stdout
    try:
        reference_source=run('validation-reference-source',['go','run','./tools/rust-migration/validation-reference'])
        prepared=work/'validation-reference';prepared.mkdir();(prepared/'main.go').write_bytes(reference_source)
        score_source=run('score-reference-source',['go','run','./tools/rust-migration/score-reference'])
        score_prepared=work/'score-reference';score_prepared.mkdir();(score_prepared/'main.go').write_bytes(score_source)
        project_source=run('project-reference-source',['go','run','./tools/rust-migration/project-reference'])
        project_prepared=work/'project-reference';project_prepared.mkdir();(project_prepared/'main.go').write_bytes(project_source)
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,['./cmd/swarm','./internal/engine','./'+str(prepared.relative_to(ROOT)),'./'+str(score_prepared.relative_to(ROOT)),'./'+str(project_prepared.relative_to(ROOT))],allow_root=True,extra_files=[prepared/'main.go',score_prepared/'main.go',project_prepared/'main.go'])
        run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
        tests={}
        for label,flags in [('workspace-all',['--workspace','--all-features']),('engine-default',['-p','swarm-engine'])]:
            args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline',*flags,'--all-targets']
            raw=run(label,args).decode();listed=run(label+'-inventory',args+['--','--list']).decode()
            actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'))
            expected=Counter(l.removesuffix(': test')for l in listed.splitlines()if l.endswith(': test'))
            assert actual and actual==expected,'named tests/inventory mismatch';tests[label]=list(actual.elements())
        selected=['TestIdempotencyAndRevision','TestConcurrentWriters','TestInspectionCLIRefusesImplicitStorageUpgrade','TestInspectionDoesNotCreateMissingStore']
        raw=run('go-reference-tests',['go','test','-timeout=0','-count=1','-v','./internal/engine','-run','^('+'|'.join(selected)+')$']).decode()
        for name in selected:assert '--- PASS: '+name+' 'in raw,name+': not executed'
        race=run('go-writer-race',['go','test','-race','-timeout=0','-count=1','-v','./internal/engine','-run','^TestConcurrentWriters$']).decode();assert '--- PASS: TestConcurrentWriters 'in race
        run('workspace-compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets'])
        run('clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets','--','-D','warnings'])
        run('go-cli-build',['go','build','-o',str(work/'go-cli'),'./cmd/swarm'])
        run('native-cli-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-cli'])
        run('root-probe-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example','root_store_probe'])
        native=Path(env['CARGO_TARGET_DIR'])/'debug/swarm-rust';probe=Path(env['CARGO_TARGET_DIR'])/'debug/examples/root_store_probe';cli=work/'go-cli'
        reference=work/'go-validation-reference';run('validation-reference-build',['go','build','-o',str(reference),str(prepared/'main.go')])
        run('validation-reference-vet',['go','vet','./tools/rust-migration/validation-reference','./'+str(prepared.relative_to(ROOT))])
        assert reference_source==run('validation-reference-after',['go','run','./tools/rust-migration/validation-reference'])
        run('validation-probe-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example','validation_run_probe'])
        validation_probe=Path(env['CARGO_TARGET_DIR'])/'debug/examples/validation_run_probe'
        controls=qualify_controls(ROOT,output/'validation-controls',native,validation_probe,reference)
        score_reference=work/'go-score-reference';run('score-reference-build',['go','build','-o',str(score_reference),str(score_prepared/'main.go')])
        run('score-reference-vet',['go','vet','./tools/rust-migration/score-reference','./'+str(score_prepared.relative_to(ROOT))])
        assert score_source==run('score-reference-after',['go','run','./tools/rust-migration/score-reference'])
        scores=qualify_scores(ROOT,output/'score',native,score_reference)
        run('gate-probe-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example','gate_store_probe'])
        gate_probe=Path(env['CARGO_TARGET_DIR'])/'debug/examples/gate_store_probe'
        gates=qualify_gates(ROOT,output/'gate-store',native,gate_probe,cli,probe)
        project_reference=work/'go-project-reference';run('project-reference-build',['go','build','-o',str(project_reference),str(project_prepared/'main.go')])
        run('project-reference-vet',['go','vet','./tools/rust-migration/project-reference','./'+str(project_prepared.relative_to(ROOT))])
        assert project_source==run('project-reference-after',['go','run','./tools/rust-migration/project-reference'])
        run('project-guard-probe-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example','project_context_probe'])
        project_probe=Path(env['CARGO_TARGET_DIR'])/'debug/examples/project_context_probe'
        project_context=qualify_context(ROOT,output/'project-context',native,project_reference,project_probe)
        roots={name:scratch/name for name in ['go','native']}
        for name,path in roots.items():
            path.mkdir(mode=0o700);run('root-init-'+name,[str(cli if name=='go'else native),'--root',str(path),'--json','init'])
            (path/'.rust-migration-fixture').write_text('Owned native root qualification fixture.\n')
        def schema(path):
            with sqlite3.connect(path/'.swarm/state.db')as db:
                objects=db.execute("SELECT type,name,tbl_name,sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name").fetchall()
                rows={name:db.execute('SELECT count(*)FROM "'+name.replace('"','""')+'"').fetchone()[0]for typ,name,_,_ in objects if typ=='table'}
                return dict(version=db.execute('PRAGMA user_version').fetchone()[0],objects=objects,initial_rows=rows)
        g,n=map(schema,roots.values());assert g==n and g['version']==37 and len(g['objects'])==76 and len(g['initial_rows'])==49 and not any(g['initial_rows'].values());save('schema-parity.json',g)
        # Actual native schema equality is authoritative; derivation formatting is recorded separately.
        save('schema-derivation-check.json',dict(actual_schema_equal=True,objects=76,tables=49,source_sha256=hashlib.sha256((ROOT/'rust/crates/swarm-engine/src/initial_schema_37.sql').read_bytes()).hexdigest()))
        root=roots['native']
        def command(label,args):return json.loads(run(label,[str(native),'--root',str(root),'--json',*args]))
        doc=command('settings-show',['engine-settings','show']);assert doc['revision']=='1'
        def frame(label,kind,workid,request):
            return json.loads(run(label,[str(probe),str(root)],json.dumps(dict(work=workid,kind=kind,request=request)).encode()+b'\n'))
        def request(event,revision,**fields):return dict(schema_version=1,event_id=event,expected_revision=revision,**fields)
        created=frame('native-work-create','work.create','',request('root-create',0,title='Native root',objective='Actual root store',scope='Owned fixture',criteria=['Proof']))
        assert created['code']=='ok'and created['settings_revision']=='1';wid=created['work']['id']
        # Go may read/mutate the fresh native schema while settings still match legacy defaults.
        observed=json.loads(run('go-read-native-root',[str(cli),'--root',str(root),'--json','work','show',wid]));assert observed['work']==created['work']
        changefile=work/'go-change.json';changefile.write_text(json.dumps(request('go-root-update',1,title='Go also reads the native schema')))
        observed=json.loads(run('go-mutate-native-root',[str(cli),'--root',str(root),'--json','work','update',wid,'--input',str(changefile)]));assert observed['work']['revision']==2
        assert list((root/'.swarm/views').glob('*.md')),'native views layout must support Go rendering'
        added=frame('native-task-add','task.add',wid,request('root-task',2,id='task',title='Task',deliverable='report',criteria=['Proof']));assert added['code']=='ok'
        edit=request('root-task-cap',3,id='task',max_attempts=4)
        assert frame('native-task-over-cap','task.update',wid,edit)['code']!='ok'
        settingspath=root/'.swarm/engine-settings.json';before_settings=settingspath.read_bytes();values=doc['values'].copy();values['task_max_attempts']='5'
        change=dict(schema_version=doc['schema_version'],event_id='root-policy',expected_revision='1',values=values,reason='Operator changes definition cap');path=work/'settings-change.json';path.write_text(json.dumps(change))
        preview=command('settings-preview',['engine-settings','preview','--input',str(path)]);assert settingspath.read_bytes()==before_settings
        execute('settings-without-token',[str(native),'--root',str(root),'--json','engine-settings','apply','--input',str(path)],expected=2);assert settingspath.read_bytes()==before_settings
        change['preview_token']=preview['preview_token'];path.write_text(json.dumps(change));applied=command('settings-apply',['engine-settings','apply','--input',str(path)]);assert not applied['replayed']and applied['document']['revision']=='2'
        reopened=command('settings-reopen',['engine-settings','show']);assert reopened['values']['task_max_attempts']=='5'
        edited=frame('native-task-cap-now-admitted','task.update',wid,edit);assert edited['code']=='ok'and edited['settings_revision']=='2'and edited['work']['tasks'][0]['plan_max_attempts']==4
        assert command('settings-replay',['engine-settings','apply','--input',str(path)])['replayed'];assert command('settings-replay-reopen',['engine-settings','show'])['history']==reopened['history']
        changed=dict(change,reason='Conflicting intent');path.write_text(json.dumps(changed));before_settings=settingspath.read_bytes();execute('settings-event-conflict',[str(native),'--root',str(root),'--json','engine-settings','apply','--input',str(path)],expected=2);assert settingspath.read_bytes()==before_settings
        # Reinitialization cannot restore installation defaults over operator changes.
        command('native-init-idempotent',['init']);assert command('native-init-preserves-policy',['engine-settings','show'])==reopened
        for name,binary in [('native-cli',native),('native-root-probe',probe),('go-cli',cli),('native-validation-probe',validation_probe),('go-validation-reference',reference),('go-score-reference',score_reference),('native-gate-probe',gate_probe),('go-project-reference',project_reference),('native-project-context-probe',project_probe)]:target=output/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600)
        close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
        save('result.json',dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3aj actual Root row review consistency and current reserved policy; no hierarchical acceptance authority',validation_controls=controls,score=scores,gates=gates,project_context=project_context,tests=tests,go_selected_tests=selected,go_race_test='TestConcurrentWriters',schema_objects=76,empty_tables=49,native_init_without_go=True,go_reads_and_mutates_native_schema=True,settings_cli_preview_save_reopen_replay=True,actual_work_engine_settings_effect=True,native_gate_recording=True,root_observed_gate_policy=True,root_row_review_observation=True,review_observation_scope="data-consistency-only",manual_task_acceptance=True,hierarchical_task_acceptance=False,inputs=len(before),registry_packages=len(registry),source_stable=True,fixture_root=str(work),scratch=str(scratch),global_watchdog=None,admin_settings_wired=False,old_storage_migrations=False,public_native_work_cli=False,http_server=False,independent_review=False,deployment=False))
        print('PASS native root/settings; PARTIAL full migration',flush=True)
    except BaseException as error:
        save('failure.json',dict(status='FAIL',type=type(error).__name__,cause=str(error),fixture_root=str(work),scratch=str(scratch),global_watchdog=None));raise
if __name__=='__main__':main()
