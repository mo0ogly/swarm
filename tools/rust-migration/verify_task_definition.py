"""Source/fixture-bound task definition, policy and instructions qualification, without a watchdog."""
from collections import Counter
import datetime,json,os,shutil,subprocess,sys,tempfile,time
from pathlib import Path
from qualification_inputs import collect
from reuse_go_reference import reuse
from verify_go_reference_reuse import check as check_reuse
from recorded_command_vectors import vectors as prefix_vectors
from task_definition_vectors import vectors
from qualify_task_definition import qualify
from verify_local_path import tree
ROOT=Path(__file__).resolve().parents[2]
def main():
    output=Path(sys.argv[1]).resolve()
    if output.exists()and any(output.iterdir()):raise RuntimeError('Fresh evidence directory required')
    output.mkdir(parents=True,exist_ok=True);work=Path(tempfile.mkdtemp(prefix='swarm-task-definitions-',dir=ROOT/'.cache'));scratch=Path(tempfile.mkdtemp(prefix='.swarm-rust-verification-',dir=Path.home().resolve()))
    for p in [scratch,*scratch.parents]:s=p.stat();assert s.st_uid in [0,os.geteuid()]and not s.st_mode&0o22,'unsafe fixture ancestry'
    env=dict(os.environ,TMPDIR=str(scratch),GOTMPDIR=str(scratch),CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(work/'target')));commands=[]
    def save(name,value):
        p=output/name;fd,tmp=tempfile.mkstemp(dir=p.parent,prefix='.'+p.name+'.')
        with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
        os.replace(tmp,p)
    def execute(label,args,data=None,expected=0):
        start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic();p=subprocess.run(args,cwd=ROOT,env=env,input=data,capture_output=True)
        if data is not None:(output/(label+'.stdin')).write_bytes(data)
        for ext,raw in [('stdout',p.stdout),('stderr',p.stderr)]:(output/(label+'.'+ext)).write_bytes(raw)
        commands.append(dict(label=label,command=args,start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));save('commands.json',commands);print(label,'exit',p.returncode,flush=True)
        if expected is not None and p.returncode!=expected:raise RuntimeError(label+': original failure retained')
        return p
    def run(label,args,data=None):return execute(label,args,data).stdout
    try:
        save('fixture-environment.json',dict(scratch=str(scratch),ancestors=[dict(path=str(p),mode=oct(p.stat().st_mode&0o7777),uid=p.stat().st_uid)for p in [scratch,*scratch.parents]]))
        prepared=Path(tempfile.mkdtemp(prefix='swarm-r3m-private-',dir=ROOT/'.cache'));source=run('task-source',['go','run','./tools/rust-migration/task-definition-reference']);(prepared/'main.go').write_bytes(source)
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,['./cmd/swarm','./internal/engine','./tools/rust-migration/task-definition-reference','./'+str(prepared.relative_to(ROOT))],allow_root=True)
        baseline=ROOT/'docs/plans/migration-rust/evidence/20261018-04/qualified-02';prefix=prefix_vectors();prefix_data=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in prefix);old=json.loads((baseline/'inputs-before.json').read_text());original_prefix,reused=reuse(baseline,output,old,before,prefix_data);save('reused-go-proof.json',reused);save('reuse-guard-tests.json',check_reuse(baseline,output,old,before,prefix_data,work))
        previous=ROOT/'docs/plans/migration-rust/evidence/20261018-09/qualified-02';prior=json.loads((previous/'inputs-before.json').read_text());assert prior==json.loads((previous/'inputs-after.json').read_text());assert json.loads((previous/'result.json').read_text())['native_component_status']=='PASS'
        changed={'rust/crates/swarm-engine/'+p for p in ['src/lib.rs','Cargo.toml','src/mutation_guard.rs']};unchanged=[k for k in prior if k.startswith('rust/')and k not in changed];assert all(before[k]==prior[k]for k in unchanged),'other native contract changed; broaden qualification';save('reused-native-proof.json',dict(baseline=str(previous.relative_to(ROOT)),unchanged=unchanged,changed_shared_contract='Shared task definition predicate visibility only; new task/policy/instruction modules',limits='Fresh engine and related corpora; unchanged wire/config/leaf proofs reused'))
        run('archive-helper-check',['python3','tools/rust-migration/check_archive.py',str(output/'archive-helper-check')]);run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check']);tests={}
        for label,flags in [('engine-all',['--all-features']),('engine-default',[])]:
            args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine',*flags,'--all-targets'];raw=run(label,args).decode();listed=run(label+'-inventory',args+['--','--list']).decode();actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));expected=Counter(l.removesuffix(': test')for l in listed.splitlines()if l.endswith(': test'));assert actual and actual==expected;tests[label]=list(actual.elements())
        selected=['TestTaskDefinitionEditPersistsAndReplays','TestTaskDefinitionRejectsInvalidGraphAtomically','TestTaskDefinitionInvalidatesGateAndRebuildsChecks','TestTaskDefinitionArchivesOnlyChangedReviewContract','TestTaskDefinitionRejectsActiveIndependentReview','TestTaskDefinitionLimitsAndMetadata','TestInstructionRevisionOperatorReplacementRetainsLearning','TestInstructionRevisionLegacyOperatorPrefixReconciles','TestInstructionRevisionAmbiguousOperatorEditIsAtomic','TestValidationPolicyRejectsShellAndUnboundedControls']
        raw=run('go-task-tests',['go','test','-timeout=0','-count=1','-v','./internal/engine','-run','^('+'|'.join(selected)+')$']).decode();assert '[no tests to run]'not in raw
        for name in selected:assert '--- PASS: '+name+' 'in raw,name+': not executed'
        run('workspace-compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets']);run('clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--all-features','--all-targets','--','-D','warnings']);run('go-new-vet',['go','vet','./tools/rust-migration/task-definition-reference','./'+str(prepared.relative_to(ROOT))])
        for name in ['task_definition_probe','recorded_command_probe','local_path_probe']:run(name+'-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example',name])
        examples=Path(env['CARGO_TARGET_DIR'])/'debug/examples';reference=work/'go-task-definitions';run('task-reference-build',['go','build','-o',str(reference),str(prepared/'main.go')]);assert source==run('task-source-after',['go','run','./tools/rust-migration/task-definition-reference'])
        cases=vectors();data=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in cases);go=[json.loads(l)for l in run('task-corpus-go',[str(reference)],data).splitlines()];native=[json.loads(l)for l in run('task-corpus-rust',[str(examples/'task_definition_probe')],data).splitlines()];assert len(go)==len(native)==len(cases)
        differences=[];counterexamples=[]
        for v,g,n in zip(cases,go,native):
            if v['classification']=='parity':
                if g!=n:differences.append(dict(input=v,go=g,rust=n))
            else:assert g['code']==n['code']=='format'and g['body']!=n['body'];counterexamples.append(dict(input=v,go=g,rust=n))
            if v['control']:assert g['code']==n['code']=='ok'
        save('unexpected-differences.json',differences);save('go-task-memory-counterexamples.json',counterexamples);assert not differences and counterexamples
        path_cases=(previous/'path-corpus-rust.stdin').read_bytes();assert before['tools/rust-migration/local_path_vectors.py']==prior['tools/rust-migration/local_path_vectors.py'];path_root=Path(json.loads((previous/'result.json').read_text())['fixture_root'])/'path-fixture';path_snapshot=json.loads((previous/'filesystem-after.json').read_text());assert tree(path_root)==path_snapshot;assert [json.loads(l)for l in run('path-regressions',[str(examples/'local_path_probe')],path_cases).splitlines()]==[json.loads(l)for l in(previous/'path-corpus-rust.stdout').read_bytes().splitlines()];assert tree(path_root)==path_snapshot
        assert [json.loads(l)for l in run('prefix-regressions',[str(examples/'recorded_command_probe')],prefix_data).splitlines()]==[json.loads(l)for l in original_prefix.splitlines()]
        run('go-cli-build',['go','build','-o',str(work/'go-cli'),'./cmd/swarm']);public=qualify(ROOT,output,work,env,run,execute,save,examples/'task_definition_probe')
        for name,binary in [('native-task-probe',examples/'task_definition_probe'),('go-task-reference',reference)]:target=output/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600)
        close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
        report=dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3m pure task definitions, explicit policy limits and attributed instructions; no status acceptance or Store authority',cases=len(cases),parity_cases=len(cases)-len(counterexamples),memory_counterexamples=len(counterexamples),valid_controls=sum(v['control']for v in cases),tests=tests,go_selected_tests=selected,regressions=dict(paths=179,prefix=324),public_flow=public,inputs=len(before),registry_packages=len(registry),source_stable=True,fixture_root=str(work),scratch=str(scratch),global_watchdog=None,admin_settings_wired=False,public_native_mutation=False,independent_review=False,deployment=False)
        save('result.json',report);print(json.dumps({k:report[k]for k in ['status','native_component_status','cases','inputs']}))
    except Exception as e:save('failure.json',dict(type=type(e).__name__,cause=str(e),completed_commands=len(commands)));raise
if __name__=='__main__':main()
