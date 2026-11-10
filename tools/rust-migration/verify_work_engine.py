"""Source/fixture-bound native WorkEngine and system observations qualification, without a watchdog."""
from collections import Counter
import datetime,json,os,shutil,subprocess,sys,tempfile,time
from pathlib import Path
from qualification_inputs import collect
from recorded_command_vectors import vectors as prefix_vectors
from environment_vectors import vectors
from qualify_work_engine import qualify
from qualify_environment import qualify as qualify_git
ROOT=Path(__file__).resolve().parents[2]
def reuse_rust_checks(baseline,fresh,current):
    """Reuse completed native checks from an interrupted overall qualification.

    This never promotes the failed overall result. Actual compilation inputs,
    resolved features, toolchains and executed test inventories must all match.
    """
    import hashlib
    old=json.loads((baseline/'inputs-before.json').read_text())
    native={k:v for k,v in old.items()if k.startswith(('rust/','registry/'))}
    assert native and native=={k:v for k,v in current.items()if k.startswith(('rust/','registry/'))},'native source or dependency changed'
    for name in ['cargo-metadata.stdout','build-environment.json']:
        assert json.loads((baseline/name).read_text())==json.loads((fresh/name).read_text()),name+': compilation graph/settings changed'
    for name in ['rust-version.stdout','cc-version.stdout']:
        assert (baseline/name).read_bytes()==(fresh/name).read_bytes(),name+': native toolchain changed'
    commands={v['label']:v for v in json.loads((baseline/'commands.json').read_text())};tests={};proofs={}
    for label in ['workspace-all','engine-default']:
        args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline',*(['--workspace','--all-features']if label=='workspace-all'else['-p','swarm-engine']),'--all-targets']
        for name,expected_args in [(label,args),(label+'-inventory',args+['--','--list'])]:
            command=commands[name];assert command['exit']==command['expected_exit']==0 and command['command']==expected_args,name+': check incomplete or changed'
            for extension in ['stdout','stderr']:
                file=name+'.'+extension;proofs[file]=hashlib.sha256((baseline/file).read_bytes()).hexdigest()
        raw=(baseline/(label+'.stdout')).read_text();inventory=(baseline/(label+'-inventory.stdout')).read_text()
        actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));expected=Counter(l.removesuffix(': test')for l in inventory.splitlines()if l.endswith(': test'));assert actual and actual==expected;tests[label]=list(actual.elements())
    return tests,dict(status='PASS',scope='Completed native tests only; original overall failure retained',baseline=str(baseline),native_inputs=len(native),proof_files_sha256=proofs)
def main():
    output=Path(sys.argv[1]).resolve()
    if output.exists()and any(output.iterdir()):raise RuntimeError('Fresh evidence directory required')
    output.mkdir(parents=True,exist_ok=True);work=Path(tempfile.mkdtemp(prefix='swarm-work-engine-',dir=ROOT/'.cache'));scratch=Path(tempfile.mkdtemp(prefix='.swarm-rust-verification-',dir=Path.home().resolve()))
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
        prepared=Path(tempfile.mkdtemp(prefix='swarm-r3o-private-',dir=ROOT/'.cache'));source=run('environment-source',['go','run','./tools/rust-migration/environment-reference']);(prepared/'main.go').write_bytes(source)
        prefix_prepared=Path(tempfile.mkdtemp(prefix='swarm-r3o-prefix-',dir=ROOT/'.cache'));prefix_source=run('prefix-reference-source',['go','run','./tools/rust-migration/command-reference']);(prefix_prepared/'main.go').write_bytes(prefix_source)
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,['./cmd/swarm','./internal/engine','./tools/rust-migration/environment-reference','./tools/rust-migration/work-row-fixture','./tools/rust-migration/command-reference','./'+str(prepared.relative_to(ROOT)),'./'+str(prefix_prepared.relative_to(ROOT))],allow_root=True)
        # Dependencies/features changed: execute fresh Go reference checks, with
        # exact source/normal/race/root/embed collection. Never weaken reuse guards.
        prefix=prefix_vectors();prefix_data=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in prefix)
        run('archive-helper-check',['python3','tools/rust-migration/check_archive.py',str(output/'archive-helper-check')]);run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check']);tests={}
        if len(sys.argv)==3:
            tests,reused=reuse_rust_checks(Path(sys.argv[2]).resolve(),output,before);save('reused-completed-native-checks.json',reused)
        else:
            for label,flags in [('workspace-all',['--workspace','--all-features']),('engine-default',['-p','swarm-engine'])]:
                args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline',*flags,'--all-targets'];raw=run(label,args).decode();listed=run(label+'-inventory',args+['--','--list']).decode();actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));expected=Counter(l.removesuffix(': test')for l in listed.splitlines()if l.endswith(': test'));assert actual and actual==expected;tests[label]=list(actual.elements())
        selected=['TestIdempotencyAndRevision','TestConcurrentWriters','TestWorkRowCommonMutationRejectsBeforeCallbacks','TestWorkRowReplayReturnsCurrentConsistentState']
        raw=run('go-original-mutation-tests',['go','test','-timeout=0','-count=1','-v','./internal/engine','-run','^('+'|'.join(selected)+')$']).decode();assert '[no tests to run]'not in raw
        for name in selected:assert '--- PASS: '+name+' 'in raw,name+': not executed'
        race=run('go-concurrent-writer-race',['go','test','-race','-timeout=0','-count=1','-v','./internal/engine','-run','^TestConcurrentWriters$']).decode();assert '--- PASS: TestConcurrentWriters 'in race
        run('workspace-compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets']);run('clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--all-features','--all-targets','--','-D','warnings']);run('go-vet',['go','vet','./tools/rust-migration/environment-reference','./tools/rust-migration/work-row-fixture','./tools/rust-migration/command-reference','./'+str(prepared.relative_to(ROOT)),'./'+str(prefix_prepared.relative_to(ROOT))])
        for name in ['environment_probe','work_engine_probe','recorded_command_probe']:run(name+'-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example',name])
        examples=Path(env['CARGO_TARGET_DIR'])/'debug/examples';reference=work/'go-environment';run('environment-reference-build',['go','build','-o',str(reference),str(prepared/'main.go')]);assert source==run('environment-source-after',['go','run','./tools/rust-migration/environment-reference'])
        cases=vectors();data=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in cases);go=[json.loads(l)for l in run('environment-corpus-go',[str(reference)],data).splitlines()];native=[json.loads(l)for l in run('environment-corpus-rust',[str(examples/'environment_probe')],data).splitlines()];assert len(go)==len(native)==len(cases)
        differences=[];encoding_variations=[]
        for v,g,n in zip(cases,go,native):
            if g!=n:
                if v['Mode']=='text'and g['name']==n['name']and json.loads(bytes.fromhex(g['value']))==json.loads(bytes.fromhex(n['value'])):
                    try:bytes.fromhex(v['Bytes']).decode('utf-8')
                    except UnicodeDecodeError:encoding_variations.append(v['Name']);continue
                differences.append(dict(input=v,go=g,rust=n))
        save('unexpected-differences.json',differences);save('invalid-utf8-encoding-variations.json',dict(count=len(encoding_variations),names=encoding_variations,scope='Same decoded text; Go invalid-byte escape vs normalized UTF-8 U+FFFD. Raw originals retained, not byte parity.'));assert not differences
        prefix_reference=work/'go-prefix';run('prefix-reference-build',['go','build','-o',str(prefix_reference),str(prefix_prepared/'main.go')]);assert prefix_source==run('prefix-reference-source-after',['go','run','./tools/rust-migration/command-reference'])
        original_prefix=run('command-corpus-go',[str(prefix_reference)],prefix_data);assert [json.loads(l)for l in run('command-corpus-rust',[str(examples/'recorded_command_probe')],prefix_data).splitlines()]==[json.loads(l)for l in original_prefix.splitlines()]
        git=qualify_git(output,work,env,run,save,reference,examples/'environment_probe')
        actor=dict(Name='actual-operator',Mode='operator',Bytes='',Prefix='',Seconds=0,Nanos=0,Root='');data=json.dumps(actor).encode()+b'\n';assert json.loads(run('operator-go',[str(reference)],data))==json.loads(run('operator-rust',[str(examples/'environment_probe')],data))
        run('go-cli-build',['go','build','-o',str(work/'go-cli'),'./cmd/swarm']);public=qualify(ROOT,output,work,scratch,env,run,execute,save,examples/'work_engine_probe')
        for name,binary in [('native-environment-probe',examples/'environment_probe'),('native-work-engine-probe',examples/'work_engine_probe'),('go-environment-reference',reference),('go-prefix-reference',prefix_reference)]:target=output/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600)
        close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
        report=dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3o transaction-owning WorkEngine on supplied v37 connection; native environment; no public root migration/CLI/Admin',cases=len(cases),semantic_parity_cases=len(cases),byte_parity_cases=len(cases)-len(encoding_variations),invalid_utf8_json_encoding_variations=len(encoding_variations),tests=tests,go_selected_tests=selected,go_race_test='TestConcurrentWriters',prefix_regressions=324,public_flow=public,git_cases=len(git['cases']),inputs=len(before),registry_packages=len(registry),source_stable=True,fixture_root=str(work),scratch=str(scratch),global_watchdog=None,dependency_context_requalified='Fresh workspace tests, Go reference/race and oracle execution after new existing time/getrandom/nix edges/features',admin_settings_wired=False,full_store=False,public_native_mutation=False,independent_review=False,deployment=False)
        save('result.json',report);print(json.dumps({k:report[k]for k in ['status','native_component_status','cases','inputs']}))
    except Exception as e:save('failure.json',dict(type=type(e).__name__,cause=str(e),completed_commands=len(commands)));raise
if __name__=='__main__':main()
