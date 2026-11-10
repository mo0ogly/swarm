"""Atomic storage qualification, explicit Go integrity failures, no full Store."""
from collections import Counter
import datetime
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tarfile
import time
from qualification_inputs import collect
from reuse_go_reference import reuse
from verify_go_reference_reuse import check as check_reuse
from recorded_change_vectors import vectors
from recorded_command_vectors import vectors as prefix_vectors
from qualify_recorded_change import qualify
ROOT=Path(__file__).resolve().parents[2]
def main():
    output=Path(sys.argv[1]).resolve()
    if output.exists()and any(output.iterdir()):raise RuntimeError('Fresh evidence directory required')
    output.mkdir(parents=True,exist_ok=True);(ROOT/'.cache').mkdir(exist_ok=True);work=Path(tempfile.mkdtemp(prefix='swarm-change-',dir=ROOT/'.cache'));scratch=work/'tmp';scratch.mkdir(mode=0o700)
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
        prepared=Path(tempfile.mkdtemp(prefix='swarm-r3h-private-',dir=ROOT/'.cache'));source=run('tail-source',['go','run','./tools/rust-migration/change-reference']);(prepared/'main.go').write_bytes(source)
        packages=['./cmd/swarm','./internal/engine','./tools/rust-migration/change-reference','./tools/rust-migration/work-row-fixture','./'+str(prepared.relative_to(ROOT))]
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,packages,allow_root=True)
        baseline=ROOT/'docs/plans/migration-rust/evidence/20261018-04/qualified-02';old=json.loads((baseline/'inputs-before.json').read_text())
        prefix=prefix_vectors();prefix_data=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in prefix)
        original_go,reused=reuse(baseline,output,old,before,prefix_data);save('reused-go-proof.json',reused);save('reuse-guard-tests.json',check_reuse(baseline,output,old,before,prefix_data,work));print('Go proof reuse: positive/stale/refusal/recovery PASS',flush=True)
        unchanged=[k for k in old if k.startswith('rust/crates/')and(not k.startswith('rust/crates/swarm-engine/')or(k.startswith('rust/crates/swarm-engine/src/')and Path(k).name not in ['lib.rs','recorded_command.rs','identity_plan_upgrade.rs']))]
        upgrade='rust/crates/swarm-engine/src/identity_plan_upgrade.rs'
        with tarfile.open(baseline/'sources.tar.gz')as archive:previous=archive.extractfile(upgrade).read().decode()
        expected_upgrade=previous.replace('    Schema,','    Schema,\n    InactiveTransaction,',1).replace('            Self::Schema => "identity_plan_projection_schema",','            Self::Schema => "identity_plan_projection_schema",\n            Self::InactiveTransaction => "identity_plan_transaction_inactive",').replace('    let mut savepoint = transaction.savepoint_with_name(SAVEPOINT)?;','    if transaction.is_autocommit() {\n        return Err(IdentityPlanUpgradeError::InactiveTransaction);\n    }\n    let mut savepoint = transaction.savepoint_with_name(SAVEPOINT)?;')
        assert (ROOT/upgrade).read_text()==expected_upgrade,'conversion changed beyond active-transaction precondition; broaden qualification'
        assert all(old[k]==before[k]for k in unchanged),'unchanged leaf/codec/event/Work/identity inputs changed';save('reused-native-proof.json',dict(baseline=str(baseline.relative_to(ROOT)),unchanged=unchanged,changed_shared_contract='recorded_command exact key/active transaction; identity conversion active transaction guard, fresh tests and real-Go DB conversion'))
        run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check']);tests={}
        for label,flags in [('engine-all',['--all-features']),('engine-default',[])]:
            args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine',*flags,'--all-targets'];raw=run(label,args).decode();listed=run(label+'-inventory',args+['--','--list']).decode()
            actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));expected=Counter(l.removesuffix(': test')for l in listed.splitlines()if l.endswith(': test'));assert actual and actual==expected;tests[label]=list(actual.elements())
        run('workspace-compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets']);run('engine-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--all-features','--all-targets','--','-D','warnings'])
        run('new-go-vet',['go','vet','./tools/rust-migration/change-reference','./'+str(prepared.relative_to(ROOT))])
        for name in ['recorded_change_probe','recorded_command_probe','identity_plan_upgrade_probe']:run(name+'-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example',name])
        examples=Path(env['CARGO_TARGET_DIR'])/'debug/examples';native_prefix=run('prefix-regression-rust',[str(examples/'recorded_command_probe')],prefix_data);assert [json.loads(l)for l in native_prefix.splitlines()]==[json.loads(l)for l in original_go.splitlines()]
        reference=work/'go-tail';run('tail-build',['go','build','-o',str(reference),str(prepared/'main.go')]);assert source==run('tail-source-check',['go','run','./tools/rust-migration/change-reference'])
        cases=vectors();data=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in cases)
        go=[json.loads(l)for l in run('tail-corpus-go',[str(reference)],data).splitlines()];native=[json.loads(l)for l in run('tail-corpus-rust',[str(examples/'recorded_change_probe')],data).splitlines()];assert len(go)==len(native)==len(cases)and cases
        differences=[];counterexamples=[]
        for v,g,n in zip(cases,go,native):
            if v['classification']=='integrity':assert g['code']=='ok'and n['code']=='reject'and g!=n;counterexamples.append(dict(input=v,go=g,rust=n))
            elif g!=n:differences.append(dict(input=v,go=g,rust=n))
            if v['control']:assert g['code']==n['code']=='ok'
        save('unexpected-differences.json',differences);save('go-integrity-counterexamples.json',counterexamples);assert not differences and counterexamples
        for name,package in [('go-cli','./cmd/swarm'),('go-row-setter','./tools/rust-migration/work-row-fixture')]:run(name+'-build',['go','build','-o',str(work/name),package])
        public=qualify(ROOT,output,work,env,run,execute,save,examples/'recorded_change_probe')
        for name,binary in [('native-change-probe',examples/'recorded_change_probe'),('go-tail-reference',reference)]:target=output/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600)
        close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
        report=dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3h atomic Work/event persistence in caller-owned transaction, not business mutation',cases=len(cases),parity_cases=sum(v['classification']=='parity'for v in cases),valid_controls=sum(v['control']for v in cases),go_integrity_counterexamples=len(counterexamples),prefix_regression_cases=len(prefix),go_original_checks_reused=reused['checks'],tests=tests,public_flow=public,source_stable=True,inputs=len(before),registry_packages=len(registry),fixture_root=str(work),scratch_free_bytes=shutil.disk_usage(work).free,global_watchdog=None,full_store=False,public_native_command=False,independent_review=False,deployment=False)
        save('result.json',report);print(json.dumps({k:report[k]for k in ['status','native_component_status','cases','go_integrity_counterexamples','inputs']}))
    except Exception as e:save('failure.json',dict(type=type(e).__name__,cause=str(e),completed_commands=len(commands)));raise
if __name__=='__main__':main()
