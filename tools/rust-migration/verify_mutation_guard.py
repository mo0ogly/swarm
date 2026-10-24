"""Source-bound R3i qualification. No deployment, provider or global watchdog."""
from collections import Counter
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
from qualification_inputs import collect
from reuse_go_reference import reuse
from verify_go_reference_reuse import check as check_reuse
from mutation_guard_vectors import vectors
from recorded_command_vectors import vectors as prefix_vectors
from recorded_change_vectors import vectors as storage_vectors
from qualify_mutation_guard import qualify

ROOT=Path(__file__).resolve().parents[2]
def main():
    output=Path(sys.argv[1]).resolve()
    if output.exists()and any(output.iterdir()):raise RuntimeError('Fresh evidence directory required')
    output.mkdir(parents=True,exist_ok=True);work=Path(tempfile.mkdtemp(prefix='swarm-guards-',dir=ROOT/'.cache'));scratch=work/'tmp';scratch.mkdir(mode=0o700)
    env=dict(os.environ,TMPDIR=str(scratch),GOTMPDIR=str(scratch),CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(work/'target')));commands=[]
    def save(name,value):
        path=output/name;fd,tmp=tempfile.mkstemp(dir=path.parent,prefix='.'+path.name+'.')
        with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
        os.replace(tmp,path)
    def execute(label,args,data=None,expected=0):
        start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic();p=subprocess.run(args,cwd=ROOT,env=env,input=data,capture_output=True)
        if data is not None:(output/(label+'.stdin')).write_bytes(data)
        for ext,raw in [('stdout',p.stdout),('stderr',p.stderr)]:(output/(label+'.'+ext)).write_bytes(raw)
        commands.append(dict(label=label,command=args,start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));save('commands.json',commands);print(label,'exit',p.returncode,flush=True)
        if expected is not None and p.returncode!=expected:raise RuntimeError(label+': original failure retained')
        return p
    def run(label,args,data=None):return execute(label,args,data).stdout
    try:
        prepared=Path(tempfile.mkdtemp(prefix='swarm-r3i-private-',dir=ROOT/'.cache'));source=run('guard-source',['go','run','./tools/rust-migration/guard-reference']);(prepared/'main.go').write_bytes(source)
        packages=['./cmd/swarm','./internal/engine','./tools/rust-migration/guard-reference','./'+str(prepared.relative_to(ROOT))]
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,packages,allow_root=True)
        baseline=ROOT/'docs/plans/migration-rust/evidence/20261018-04/qualified-02';old=json.loads((baseline/'inputs-before.json').read_text());prefix=prefix_vectors();prefix_data=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in prefix)
        original_prefix,reused=reuse(baseline,output,old,before,prefix_data);save('reused-go-proof.json',reused);save('reuse-guard-tests.json',check_reuse(baseline,output,old,before,prefix_data,work))
        previous=ROOT/'docs/plans/migration-rust/evidence/20261018-05/qualified-04';previous_inputs=json.loads((previous/'inputs-before.json').read_text());assert previous_inputs==json.loads((previous/'inputs-after.json').read_text())
        previous_report=json.loads((previous/'result.json').read_text());assert previous_report['native_component_status']=='PASS'and previous_report['source_stable']
        changed={'rust/crates/swarm-engine/src/lib.rs','rust/crates/swarm-engine/src/sqlite_value.rs','rust/crates/swarm-engine/Cargo.toml'}
        unchanged=[k for k in previous_inputs if k.startswith('rust/')and k not in changed];assert all(before[k]==previous_inputs[k]for k in unchanged),'previous native contract changed; broaden qualification'
        scalar='rust/crates/swarm-engine/src/sqlite_value.rs'
        with tarfile.open(previous/'sources.tar.gz')as archive:old_scalar=archive.extractfile(scalar).read().decode()
        current_scalar=(ROOT/scalar).read_text()
        assert old_scalar[old_scalar.index('pub(crate) fn integer'):]==current_scalar[current_scalar.index('pub(crate) fn integer'):],'existing scalar conversions changed'
        save('reused-native-proof.json',dict(baseline=str(previous.relative_to(ROOT)),unchanged=unchanged,sqlite_existing_integer_string_bytes_unchanged=True,limits='Fresh engine tests, prefix and storage regressions; no full leaf/codec corpus rerun'))
        run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check']);tests={}
        for label,flags in [('engine-all',['--all-features']),('engine-default',[])]:
            args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine',*flags,'--all-targets'];raw=run(label,args).decode();listed=run(label+'-inventory',args+['--','--list']).decode()
            actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));expected=Counter(l.removesuffix(': test')for l in listed.splitlines()if l.endswith(': test'));assert actual and actual==expected;tests[label]=list(actual.elements())
        selected=['TestTaskDefinitionActiveAgentGuard','TestTaskDefinitionArchivesOnlyChangedReviewContract','TestHierarchicalContractRevisionPublicCLIReplayAndHistory','TestHierarchicalContractRevisionRejectsUnsafeRequestsAtomically','TestHierarchicalContractRevisionRejectsActiveReviewAndClosedScope','TestMissionPauseConsumersContractRevisionKeepsCause']
        raw=run('go-guard-original-tests',['go','test','-timeout=0','-count=1','-v','./internal/engine','-run','^('+'|'.join(selected)+')$']).decode();assert '[no tests to run]'not in raw
        for name in selected:assert '--- PASS: '+name+' 'in raw,name+': not executed'
        run('workspace-compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets']);run('engine-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--all-features','--all-targets','--','-D','warnings'])
        run('go-new-vet',['go','vet','./tools/rust-migration/guard-reference','./'+str(prepared.relative_to(ROOT))])
        for name in ['mutation_guard_probe','recorded_command_probe','recorded_change_probe']:run(name+'-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example',name])
        examples=Path(env['CARGO_TARGET_DIR'])/'debug/examples';ref=work/'go-guards';run('guard-build',['go','build','-o',str(ref),str(prepared/'main.go')]);assert source==run('guard-source-after',['go','run','./tools/rust-migration/guard-reference'])
        cases=vectors();data=b''.join(json.dumps(v,ensure_ascii=True,separators=(',',':')).encode()+b'\n'for v in cases);go=[json.loads(l)for l in run('guard-corpus-go',[str(ref)],data).splitlines()];native=[json.loads(l)for l in run('guard-corpus-rust',[str(examples/'mutation_guard_probe')],data).splitlines()];assert len(go)==len(native)==len(cases)
        differences=[dict(input=v,go=g,rust=n)for v,g,n in zip(cases,go,native)if g!=n or g['code']!=v['want']];save('unexpected-differences.json',differences);assert not differences
        got=run('prefix-regression-rust',[str(examples/'recorded_command_probe')],prefix_data);assert [json.loads(l)for l in got.splitlines()]==[json.loads(l)for l in original_prefix.splitlines()]
        storage=storage_vectors();storage_data=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in storage);assert storage_data==(previous/'tail-corpus-rust.stdin').read_bytes()
        for k in ['tools/rust-migration/recorded_change_vectors.py','tools/rust-migration/change-reference/main.go','tools/rust-migration/verify_recorded_change.py']:assert before[k]==previous_inputs[k]
        assert next(c for c in json.loads((previous/'commands.json').read_text())if c['label']=='tail-corpus-rust')['exit']==0
        got=run('storage-regression-rust',[str(examples/'recorded_change_probe')],storage_data);assert [json.loads(l)for l in got.splitlines()]==[json.loads(l)for l in(previous/'tail-corpus-rust.stdout').read_bytes().splitlines()]
        run('go-cli-build',['go','build','-o',str(work/'go-cli'),'./cmd/swarm']);public=qualify(ROOT,output,work,env,run,execute,save,examples/'mutation_guard_probe')
        for name,binary in [('native-guard-probe',examples/'mutation_guard_probe'),('go-guard-reference',ref)]:target=output/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600)
        close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
        report=dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3i guards before mutation closure, not full Store or permission',cases=len(cases),valid_controls=sum(v['control']for v in cases),tests=tests,go_selected_tests=selected,prefix_regressions=len(prefix),storage_regressions=len(storage),public_flow=public,inputs=len(before),registry_packages=len(registry),source_stable=True,global_watchdog=None,fixture_root=str(work),full_store=False,public_native_mutation=False,independent_review=False,deployment=False)
        save('result.json',report);print(json.dumps({k:report[k]for k in ['status','native_component_status','cases','inputs']}))
    except Exception as e:save('failure.json',dict(type=type(e).__name__,cause=str(e),completed_commands=len(commands)));raise
if __name__=='__main__':main()
