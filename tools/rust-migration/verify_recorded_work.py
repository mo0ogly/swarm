"""Source-bound R3e SQL reader qualification, reusing unchanged Go/Rust proofs."""
from collections import Counter
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import shutil
import sys
import tempfile
import time
from qualification_inputs import collect
from qualify_recorded_work import prepare,qualify
ROOT=Path(__file__).resolve().parents[2]
if __name__=='__main__':
    OUTPUT=Path(sys.argv[1]).resolve()
    if OUTPUT.exists()and any(OUTPUT.iterdir()):raise SystemExit('Fresh evidence directory required')
    OUTPUT.mkdir(parents=True,exist_ok=True)
    (ROOT/'.cache').mkdir(exist_ok=True)
    WORK=Path(tempfile.mkdtemp(prefix='swarm-work-read-',dir=ROOT/'.cache'))
else:
    WORK=Path(tempfile.gettempdir()) # No artifact allocation on import.
ENV=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(WORK/'target')));commands=[]
def save(name,value):
    p=OUTPUT/name;fd,tmp=tempfile.mkstemp(dir=p.parent,prefix='.'+p.name+'.')
    with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
    os.replace(tmp,p)
def execute(label,args,data=None,expected=0):
    started=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic()
    p=subprocess.run(args,cwd=ROOT,env=ENV,input=data,capture_output=True)
    if data is not None:(OUTPUT/(label+'.stdin')).write_bytes(data)
    for ext,raw in [('stdout',p.stdout),('stderr',p.stderr)]:(OUTPUT/(label+'.'+ext)).write_bytes(raw)
    commands.append(dict(label=label,command=args,start_utc=started,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));save('commands.json',commands)
    print(label,'exit',p.returncode,flush=True)
    if expected is not None and p.returncode!=expected:raise RuntimeError(label+': original failure retained')
    return p
def run(label,args,data=None):return execute(label,args,data).stdout

def verify_reuse_environment(baseline,fresh,old,current):
    assert json.loads((baseline/'build-environment.json').read_text())==json.loads((fresh/'build-environment.json').read_text()),'build environment changed: invalidate reused proof'
    for filename in ['rust-version.stdout','go-version.stdout','cc-version.stdout']:
        assert (baseline/filename).read_bytes()==(fresh/filename).read_bytes(),filename+': toolchain changed'
    for filename in ['rust/Cargo.toml','rust/Cargo.lock']:
        assert old[filename]==current[filename],filename+': workspace/dependencies changed'

def main():
    prepared=prepare(ROOT,OUTPUT,run,save)
    packages=['./cmd/swarm','./internal/engine','./tools/rust-migration/work-row-reference','./tools/rust-migration/work-row-fixture','./'+str(prepared.relative_to(ROOT))]
    before,snapshot,registry,close_graphs=collect(ROOT,OUTPUT,ENV,run,packages,allow_root=True)
    baseline=ROOT/'docs/plans/migration-rust/evidence/20261018-01/qualified-02'
    old=json.loads((baseline/'inputs-before.json').read_text())
    verify_reuse_environment(baseline,OUTPUT,old,before)
    # Preserve Go data-plane proofs only if their root, dependencies, tests and
    # embeds are all unchanged. New fixture/tool inputs are independently bound.
    def go_inputs(values):return{k:v for k,v in values.items()if not k.startswith(('rust/','tools/rust-migration/','.cache/'))}
    assert go_inputs(old)==go_inputs(before),'Go input closure changed: requalify affected reference'
    shared=[k for k in old if k.startswith('rust/crates/')and not k.startswith('rust/crates/swarm-engine/')]
    assert all(old[k]==before[k]for k in shared),'leaf input changed: requalify affected leaf'
    unchanged_codec=[k for k in old if k.startswith('rust/crates/swarm-engine/src/')and Path(k).name in ['models.rs','model_wire.rs','work.rs','request.rs','identity_plan.rs','identity_plan_rules.rs','identity_plan_upgrade.rs','identity_plan_heads.sql','identity_plan_upgrade.sql','identity_plan_upgrade_finish.sql']]
    assert unchanged_codec and all(old[k]==before[k]for k in unchanged_codec),'shared codec/identity input changed: broaden tests'
    save('reused-proof.json',dict(baseline=str(baseline.relative_to(ROOT)),go_inputs=len(go_inputs(before)),leaf_inputs=len(shared),unchanged_engine_contracts=unchanged_codec,limits='Reuse scoped to identical implementations and previous inputs; new SQL adapter, fixture and callers checked afresh'))
    run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
    tests={}
    for label,flags in [('engine-all',['--all-features']),('engine-default',[])]:
        args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine',*flags,'--all-targets']
        raw=run(label,args).decode();listed=run(label+'-inventory',args+['--','--list']).decode()
        actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'))
        expected=Counter(l.removesuffix(': test')for l in listed.splitlines()if l.endswith(': test'))
        assert actual and actual==expected;tests[label]=list(actual.elements())
    run('workspace-compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets'])
    run('engine-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--all-features','--all-targets','--','-D','warnings'])
    run('go-vet',['go','vet',*packages])
    run('native-probe-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example','recorded_work_probe'])
    run('fixture-cli-build',['go','build','-o',str(WORK/'go-cli-bin'),'./cmd/swarm'])
    run('fixture-setter-build',['go','build','-o',str(WORK/'go-row-setter-bin'),'./tools/rust-migration/work-row-fixture'])
    examples=Path(ENV['CARGO_TARGET_DIR'])/'debug/examples'
    result=qualify(ROOT,OUTPUT,WORK,ENV,run,execute,save,prepared,examples)
    close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
    report=dict(status='PARTIAL'if result['go_output_failure']=='OPEN'else'PASS',native_component_status='PASS',scope='R3e caller-owned SQL Work readers only',readers=result,tests=tests,source_stable=True,input_graph_closed=True,inputs=len(before),registry_packages=len(registry),fixture_root=str(WORK),scratch_device=os.stat(WORK).st_dev,scratch_free_bytes=shutil.disk_usage(WORK).free,global_watchdog=None,public_native_work_command=False,full_store=False,deployment=False,independent_review=False)
    save('result.json',report);print(json.dumps({k:report[k]for k in ['status','native_component_status','readers','inputs']}))
if __name__=='__main__':
    try:main()
    except Exception as error:save('failure.json',dict(type=type(error).__name__,cause=str(error),completed_commands=len(commands)));raise
