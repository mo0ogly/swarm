"""Source-bound reservation/replay prefix; no whole-Store acceptance."""
from collections import Counter
import datetime
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
from qualification_inputs import collect
from verify_recorded_work import verify_reuse_environment
from recorded_command_vectors import vectors
from qualify_recorded_command import qualify
ROOT=Path(__file__).resolve().parents[2]
def main():
    output=Path(sys.argv[1]).resolve()
    if output.exists()and any(output.iterdir()):raise RuntimeError('Fresh evidence directory required')
    output.mkdir(parents=True,exist_ok=True);(ROOT/'.cache').mkdir(exist_ok=True)
    work=Path(tempfile.mkdtemp(prefix='swarm-command-',dir=ROOT/'.cache'));scratch=work/'tmp';scratch.mkdir(mode=0o700)
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
        prepared=Path(tempfile.mkdtemp(prefix='swarm-r3g-private-',dir=ROOT/'.cache'))
        reference_source=run('command-reference-source',['go','run','./tools/rust-migration/command-reference']);(prepared/'main.go').write_bytes(reference_source)
        go_version=re.search(rb'const schemaVersion = (\d+)',reference_source);native_version=re.search(r'RECORDED_COMMAND_SCHEMA_VERSION: i64 = (\d+)',(ROOT/'rust/crates/swarm-engine/src/recorded_command.rs').read_text());assert go_version and native_version and int(go_version[1])==int(native_version[1])
        save('schema-contract.json',dict(actual_go_epoch=int(go_version[1]),native_epoch=int(native_version[1]),meaning='Global storage epoch, independent of projection version; not an execution policy'))
        packages=['./cmd/swarm','./internal/engine','./tools/rust-migration/command-reference','./'+str(prepared.relative_to(ROOT))]
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,packages,allow_root=True)
        baseline=ROOT/'docs/plans/migration-rust/evidence/20261018-03/qualified-03';old=json.loads((baseline/'inputs-before.json').read_text());verify_reuse_environment(baseline,output,old,before)
        def go_inputs(v):return{k:h for k,h in v.items()if not k.startswith(('rust/','tools/rust-migration/','.cache/'))}
        assert go_inputs(old)==go_inputs(before),'Go compiled/root/embed inputs changed'
        unchanged=[k for k in old if k.startswith('rust/crates/')and (not k.startswith('rust/crates/swarm-engine/')or(k.startswith('rust/crates/swarm-engine/src/')and Path(k).name!='lib.rs'))]
        assert all(old[k]==before[k]for k in unchanged),'previous native implementation changed'
        save('reused-proof.json',dict(baseline=str(baseline.relative_to(ROOT)),unchanged_native_inputs=unchanged,go_inputs=len(go_inputs(before)),limits='Prior leaf/codecs/Work/event/identity proofs reused only for identical source/environment; new transaction contract qualified independently'))
        run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check']);tests={}
        for label,flags in [('engine-all',['--all-features']),('engine-default',[])]:
            args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine',*flags,'--all-targets'];raw=run(label,args).decode();listed=run(label+'-inventory',args+['--','--list']).decode()
            actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));expected=Counter(l.removesuffix(': test')for l in listed.splitlines()if l.endswith(': test'));assert actual and actual==expected;tests[label]=list(actual.elements())
        run('workspace-compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets'])
        run('engine-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--all-features','--all-targets','--','-D','warnings'])
        selected=['TestIdempotencyAndRevision','TestConcurrentWriters','TestWorkRowCommonMutationRejectsBeforeCallbacks','TestWorkRowReplayReturnsCurrentConsistentState'];pattern='^('+'|'.join(selected)+')$'
        go_tests=run('go-original-mutation-tests',['go','test','-count=1','-timeout=0','-v','./internal/engine','-run',pattern]).decode()
        assert all('--- PASS: '+name+' 'in go_tests for name in selected)and '[no tests to run]'not in go_tests
        race=run('go-concurrent-writer-race',['go','test','-race','-count=1','-timeout=0','-v','./internal/engine','-run','^TestConcurrentWriters$']).decode();assert '--- PASS: TestConcurrentWriters 'in race
        run('go-vet',['go','vet',*packages]);run('probe-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example','recorded_command_probe'])
        reference=work/'go-command-reference';run('reference-build',['go','build','-o',str(reference),str(prepared/'main.go')]);assert run('command-reference-source-check',['go','run','./tools/rust-migration/command-reference'])==reference_source
        probe=Path(env['CARGO_TARGET_DIR'])/'debug/examples/recorded_command_probe';values=vectors();data=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in values)
        go=[json.loads(l)for l in run('command-corpus-go',[str(reference)],data).splitlines()];native=[json.loads(l)for l in run('command-corpus-rust',[str(probe)],data).splitlines()];assert len(go)==len(native)==len(values)and values
        diffs=[dict(input=v,go=g,rust=r)for v,g,r in zip(values,go,native)if g!=r];save('differences.json',diffs);assert not diffs
        assert all(v['code']in ['new','replay']for i,v in zip(values,go)if i['control'])
        run('public-cli-build',['go','build','-o',str(work/'go-cli'),'./cmd/swarm']);public=qualify(ROOT,output,work,env,run,execute,save,probe)
        for name,binary in [('go-command-reference',reference),('native-command-probe',probe)]:target=output/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600)
        close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
        report=dict(status='PASS',native_component_status='PASS',overall_migration='PARTIAL',scope='R3g writer reservation/storage epoch/idempotency prefix only',cases=len(values),controls=sum(v['control']for v in values),observations=dict(Counter(v['code']for v in go)),tests=tests,go_selected_tests=selected,go_race_test='TestConcurrentWriters',public_flow=public,source_stable=True,inputs=len(before),registry_packages=len(registry),fixture_root=str(work),scratch_device=os.stat(work).st_dev,scratch_free_bytes=shutil.disk_usage(work).free,global_watchdog=None,native_full_mutation=False,full_store=False,deployment=False,independent_review=False)
        save('result.json',report);print(json.dumps({k:report[k]for k in ['status','overall_migration','cases','inputs']}))
    except Exception as e:save('failure.json',dict(type=type(e).__name__,cause=str(e),completed_commands=len(commands)));raise
if __name__=='__main__':main()
