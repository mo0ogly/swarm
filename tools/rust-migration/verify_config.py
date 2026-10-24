"""Qualify configuration formats and native dependent guards; no provider or mission operations."""
import datetime
import hashlib
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
ROOT=Path(__file__).resolve().parents[2]
OUTPUT=Path(sys.argv[1]).resolve()
REFERENCE=Path(sys.argv[2]).resolve()
GO_REFERENCE=Path(sys.argv[3]).resolve() if len(sys.argv)>3 else None
if OUTPUT.exists() and any(OUTPUT.iterdir()):raise SystemExit('Fresh evidence directory required')
OUTPUT.mkdir(parents=True,exist_ok=True)
WORK=Path(tempfile.mkdtemp(prefix='swarm-rust-config-'))
ENV=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(WORK/'target')))
commands=[]
def run(label,args,data=None):
    started=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic()
    if data is not None:(OUTPUT/(label+'.stdin')).write_bytes(data)
    if GO_REFERENCE and label in {'go-tests','go-vet'}:
        original=json.loads((GO_REFERENCE/'commands.json').read_text())
        command=next(c for c in original if c['label']==label)
        assert command['command']==args and command['exit']==0,'reference command differs or failed'
        for ext in ['stdout','stderr']:shutil.copyfile(GO_REFERENCE/(label+'.'+ext),OUTPUT/(label+'.'+ext))
        commands.append(dict(label=label,command=args,executed=False,reference=str(GO_REFERENCE),exit=0,seconds=0))
        (OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');print(label,'reused PASS',flush=True)
        return (OUTPUT/(label+'.stdout')).read_bytes()
    result=subprocess.run(args,cwd=ROOT,env=ENV,input=data,capture_output=True)
    (OUTPUT/(label+'.stdout')).write_bytes(result.stdout);(OUTPUT/(label+'.stderr')).write_bytes(result.stderr)
    commands.append(dict(label=label,command=args,start_utc=started,seconds=time.monotonic()-clock,exit=result.returncode))
    (OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');print(label,'exit',result.returncode,flush=True)
    if result.returncode:raise RuntimeError(label+': original failure retained')
    return result.stdout

def compare(label,raw,out,keys):
    vectors=[json.loads(l)for l in raw.splitlines()];results=[json.loads(l)for l in out.splitlines()]
    assert vectors and len(vectors)==len(results) and len({v['name']for v in vectors})==len(vectors),'missing, extra or duplicate fixtures'
    mismatches=[dict(vector=v,rust=r)for v,r in zip(vectors,results)if any(v[k]!=r[k]for k in keys)]
    (OUTPUT/(label+'-mismatches.json')).write_text(json.dumps(mismatches,indent=2)+'\n');assert not mismatches,'Go/Rust '+label+' divergence'
    return len(vectors),{mode:sum(v['mode']==mode for v in vectors)for mode in sorted({v['mode']for v in vectors})}

def qualify():
    packages=['./internal/enrollmentconfig','./internal/enrollmentauthority','./internal/enrollmentrequest','./tools/rust-migration/config-oracle','./tools/rust-migration/authority-oracle']
    before,snapshot,registry,close_graphs=collect(ROOT,OUTPUT,ENV,run,packages)
    # Reuse the frozen Go authority corpus only after all original Go source,
    # generated/dependency inputs and captured Go environment match this graph.
    old=json.loads((REFERENCE/'inputs-before.json').read_text());old_result=json.loads((REFERENCE/'result.json').read_text());assert old_result['status']=='PASS'
    go_keys=[k for k in old if k in ['go.mod','go.sum'] or k.startswith(('go-toolchain/','external-go/')) or (k.startswith(('internal/','tools/')) and k.endswith('.go'))]
    assert go_keys and all(k in before and old[k]==before[k]for k in go_keys),'reference Go input differs'
    for label in ['go-env','go-version']:
        assert (REFERENCE/(label+'.stdout')).read_bytes()==(OUTPUT/(label+'.stdout')).read_bytes(),'reference Go environment differs'
    authority_raw=(REFERENCE/'go-vectors.stdout').read_bytes()
    (OUTPUT/'reused-authority-go.json').write_text(json.dumps(dict(reference=str(REFERENCE),executed=False,checked_go_inputs=len(go_keys),corpus_sha256=hashlib.sha256(authority_raw).hexdigest()),indent=2)+'\n')
    if GO_REFERENCE:
        from qualification_inputs import graph
        previous=json.loads((GO_REFERENCE/'inputs-before.json').read_text())
        keys=[k for k in previous if k in ['go.mod','go.sum'] or k.startswith(('go-toolchain/','external-go/')) or (k.startswith(('internal/','tools/')) and k.endswith('.go'))]
        assert keys and all(k in before and previous[k]==before[k]for k in keys),'reused Go check input differs'
        for label in ['go-env','go-version']:
            assert (GO_REFERENCE/(label+'.stdout')).read_bytes()==(OUTPUT/(label+'.stdout')).read_bytes(),'reused Go environment differs'
        for label in ['go-inputs-normal','go-inputs-race']:
            assert graph((GO_REFERENCE/(label+'.stdout')).read_bytes())==graph((OUTPUT/(label+'.stdout')).read_bytes()),'reused Go graph differs'
        (OUTPUT/'reused-go-checks.json').write_text(json.dumps(dict(reference=str(GO_REFERENCE),checked_go_inputs=len(keys),executed=False),indent=2)+'\n')
    run('go-tests',['go','test','-json','-count=1','-timeout=0','-run','^TestConfig','./internal/enrollmentconfig'])
    run('go-vet',['go','vet','./internal/enrollmentconfig','./tools/rust-migration/config-oracle'])
    run('rust-format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
    for label,flags in [('rust-tests',['--all-features']),('rust-default',[])]:
        run(label,['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-identity-contract','-p','swarm-wire-json','-p','swarm-enrollment-proof','-p','swarm-enrollment-state','-p','swarm-enrollment-authority','-p','swarm-enrollment-config',*flags,'--all-targets'])
    run('rust-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-identity-contract','-p','swarm-wire-json','-p','swarm-enrollment-proof','-p','swarm-enrollment-state','-p','swarm-enrollment-authority','-p','swarm-enrollment-config','--all-features','--all-targets','--','-D','warnings'])
    go=WORK/'config-oracle';target=Path(ENV['CARGO_TARGET_DIR']);probe=target/'debug/examples/config_probe';authority=target/'debug/examples/authority_probe'
    run('go-build',['go','build','-o',str(go),'./tools/rust-migration/config-oracle'])
    run('rust-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-enrollment-config','--example','config_probe'])
    raw=run('go-vectors',[str(go)]);out=run('rust-vectors',[str(probe)],raw)
    cases,modes=compare('config',raw,out,['name','code','output'])
    run('authority-rust-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-enrollment-authority','--features','test-fixtures','--example','authority_probe'])
    out=run('authority-rust-vectors',[str(authority)],authority_raw)
    authority_cases,_=compare('authority',authority_raw,out,['name','code','output','digest'])
    close_graphs();after=snapshot();(OUTPUT/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');assert before==after,'qualification input changed'
    go_pass=[e['Test']for e in map(json.loads,(OUTPUT/'go-tests.stdout').read_text().splitlines())if e.get('Action')=='pass'and'Test'in e]
    expected={name for p in (ROOT/'internal/enrollmentconfig').glob('*_test.go')for name in re.findall(r'^func (TestConfig\w+)\(t \*testing.T\)',p.read_text(),re.M)}
    assert expected and {n for n in go_pass if '/'not in n}==expected,'Go test collection differs'
    def passed(label):return [s for s in (OUTPUT/(label+'.stdout')).read_text().splitlines()if s.startswith('test ')and s.endswith(' ... ok')]
    assert len(passed('rust-tests'))==87 and len(passed('rust-default'))==78,'Rust test collection differs'
    (OUTPUT/'binaries').mkdir();binaries={}
    for name,p in [('go_oracle',go),('rust_probe',probe),('authority_rust_probe',authority)]:shutil.copyfile(p,OUTPUT/'binaries'/name);binaries[name]=hashlib.sha256(p.read_bytes()).hexdigest()
    result=dict(status='PASS',scope='R2.4 formats, native dependent guards and authority recheck; storage interoperability requires verify_config_store',inputs=len(before),registry_packages=len(registry),source_stable=True,input_graph_closed=True,go_named_passes=go_pass,rust_named_passes=passed('rust-tests'),rust_default_named_passes=passed('rust-default'),protocol_cases=cases,modes=modes,authority_recheck_cases=authority_cases,binary_sha256=binaries,temporary_fixture_root=str(WORK),cargo_target_directory=ENV['CARGO_TARGET_DIR'],global_watchdog='none; Go -timeout=0',deployment=False,independent_review=False)
    (OUTPUT/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:result[k]for k in ['status','inputs','protocol_cases','authority_recheck_cases']}))
try:qualify()
except Exception as error:
    (OUTPUT/'failure.json').write_text(json.dumps(dict(type=type(error).__name__,error=str(error),stage_count=len(commands)),indent=2)+'\n');raise
