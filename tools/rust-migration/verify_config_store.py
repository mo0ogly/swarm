"""Qualify Linux Store on private fixtures; no provider, mission or service access."""
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
from store_scenarios import run as store_scenarios
from coordinator_scenarios import run as coordinator_scenarios
ROOT=Path(__file__).resolve().parents[2]
OUTPUT=Path(sys.argv[1]).resolve()
CONFIG_REFERENCE=Path(sys.argv[2]).resolve()
AUTHORITY_REFERENCE=Path(sys.argv[3]).resolve()
GO_REFERENCE=Path(sys.argv[4]).resolve()if len(sys.argv)>4 else None
if OUTPUT.exists()and any(OUTPUT.iterdir()):raise SystemExit('Fresh evidence directory required')
OUTPUT.mkdir(parents=True,exist_ok=True)
WORK=Path(tempfile.mkdtemp(prefix='swarm-rust-store-'))
ENV=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(WORK/'target')))
commands=[]
def run(label,args,data=None,expected=0):
    start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic()
    if data is not None:(OUTPUT/(label+'.stdin')).write_bytes(data)
    if GO_REFERENCE and label in {'go-tests','go-race'}:
        old=next(c for c in json.loads((GO_REFERENCE/'commands.json').read_text())if c['label']==label)
        assert old['command']==args and old['exit']==expected==0,'reused check differs or failed'
        for ext in ['stdout','stderr']:shutil.copyfile(GO_REFERENCE/(label+'.'+ext),OUTPUT/(label+'.'+ext))
        commands.append(dict(label=label,command=args,exit=0,expected_exit=0,executed=False,reference=str(GO_REFERENCE),seconds=0))
        (OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');print(label,'reused PASS',flush=True)
        return (OUTPUT/(label+'.stdout')).read_bytes()
    result=subprocess.run(args,cwd=ROOT,env=ENV,input=data,capture_output=True)
    (OUTPUT/(label+'.stdout')).write_bytes(result.stdout);(OUTPUT/(label+'.stderr')).write_bytes(result.stderr)
    commands.append(dict(label=label,command=args,start_utc=start,seconds=time.monotonic()-clock,exit=result.returncode,expected_exit=expected))
    (OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');print(label,'exit',result.returncode,flush=True)
    if result.returncode!=expected:raise RuntimeError(label+': original failure retained')
    return result.stdout

def reference(before,path,label):
    previous=json.loads((path/'inputs-before.json').read_text());assert json.loads((path/'result.json').read_text())['status']=='PASS'
    keys=[k for k in previous if k in ['go.mod','go.sum']or k.startswith(('go-toolchain/','external-go/'))or(k.startswith(('internal/','tools/'))and k.endswith('.go'))]
    assert keys and all(k in before and previous[k]==before[k]for k in keys),'reference Go sources differ'
    for env in ['go-env','go-version']:assert (path/(env+'.stdout')).read_bytes()==(OUTPUT/(env+'.stdout')).read_bytes(),'reference Go environment differs'
    raw=(path/'go-vectors.stdout').read_bytes()
    (OUTPUT/(label+'-go-reused.json')).write_text(json.dumps(dict(reference=str(path),executed=False,checked_go_inputs=len(keys),sha256=hashlib.sha256(raw).hexdigest()),indent=2)+'\n')
    return raw

def compare(label,raw,executable,keys):
    result=run(label,[str(executable)],raw);vv=[json.loads(l)for l in raw.splitlines()];rr=[json.loads(l)for l in result.splitlines()]
    assert vv and len(vv)==len(rr)and len({v['name']for v in vv})==len(vv),'missing/extra/duplicate fixtures'
    mismatch=[dict(vector=v,rust=r)for v,r in zip(vv,rr)if any(v[k]!=r[k]for k in keys)]
    (OUTPUT/(label+'-mismatches.json')).write_text(json.dumps(mismatch,indent=2)+'\n');assert not mismatch,label+': Go/Rust divergence'
    return len(vv)

def default_guard():
    client=WORK/'default-consumer';(client/'src').mkdir(parents=True)
    manifest='[package]\nname="store-default-consumer"\nversion="0.0.0"\nedition="2021"\n[dependencies]\nswarm-enrollment-config={path='+json.dumps(str(ROOT/'rust/crates/swarm-enrollment-config'))+'}\n'
    (client/'Cargo.toml').write_text(manifest);shutil.copyfile(ROOT/'rust/Cargo.lock',client/'Cargo.lock')
    control='fn main(){let _=swarm_enrollment_config::store::Store::close;let _=swarm_enrollment_config::store::Coordinator::close;}\n'
    denied='fn main(){let _=swarm_enrollment_config::store::Store::fixture_faults;}\n'
    (client/'src/main.rs').write_text(control)
    run('default-client-lock',['cargo','generate-lockfile','--manifest-path',str(client/'Cargo.toml'),'--offline'])
    run('default-client-control',['cargo','check','--manifest-path',str(client/'Cargo.toml'),'--locked','--offline'])
    (OUTPUT/'default-consumer-control.rs').write_text(control);(client/'src/main.rs').write_text(denied)
    run('default-client-refusal',['cargo','check','--manifest-path',str(client/'Cargo.toml'),'--locked','--offline'],expected=101)
    assert b'E0599'in(OUTPUT/'default-client-refusal.stderr').read_bytes()and b'fixture_faults'in(OUTPUT/'default-client-refusal.stderr').read_bytes(),'wrong default-feature failure'
    (client/'src/main.rs').write_text(control)
    run('default-coordinator-control',['cargo','check','--manifest-path',str(client/'Cargo.toml'),'--locked','--offline'])
    denied_coordinator='fn main(){let _=swarm_enrollment_config::store::Coordinator::fixture_hook;}\n'
    (client/'src/main.rs').write_text(denied_coordinator)
    run('default-coordinator-refusal',['cargo','check','--manifest-path',str(client/'Cargo.toml'),'--locked','--offline'],expected=101)
    assert b'E0599'in(OUTPUT/'default-coordinator-refusal.stderr').read_bytes()and b'fixture_hook'in(OUTPUT/'default-coordinator-refusal.stderr').read_bytes()
    (OUTPUT/'default-coordinator-refused.rs').write_text(denied_coordinator)
    (OUTPUT/'default-consumer-refused.rs').write_text(denied);shutil.copyfile(client/'Cargo.toml',OUTPUT/'default-consumer.toml');shutil.copyfile(client/'Cargo.lock',OUTPUT/'default-consumer.lock')
    import tomllib
    main={(p['name'],p['version']):p.get('checksum')for p in tomllib.loads((ROOT/'rust/Cargo.lock').read_text())['package']}
    used=[p for p in tomllib.loads((client/'Cargo.lock').read_text())['package']if p.get('source')]
    assert all(main.get((p['name'],p['version']))==p['checksum']for p in used),'default client resolves other registry inputs'
    return dict(control=True,fixture_api_absent=True,registry_packages=len(used))

def qualify():
    packages=['./internal/enrollmentconfig','./internal/enrollmentauthority','./internal/enrollmentrequest','./tools/rust-migration/config-store-oracle','./tools/rust-migration/config-oracle','./tools/rust-migration/authority-oracle']
    before,snapshot,registry,close_graphs=collect(ROOT,OUTPUT,ENV,run,packages)
    config_raw=reference(before,CONFIG_REFERENCE,'config');authority_raw=reference(before,AUTHORITY_REFERENCE,'authority')
    if GO_REFERENCE:
        from qualification_inputs import graph
        previous=json.loads((GO_REFERENCE/'inputs-before.json').read_text());assert json.loads((GO_REFERENCE/'result.json').read_text())['status']=='PASS'
        keys=[k for k in previous if k in ['go.mod','go.sum']or k.startswith(('go-toolchain/','external-go/'))or(k.startswith(('internal/','tools/'))and k.endswith('.go'))]
        keys=[k for k in keys if not k.startswith('tools/rust-migration/config-store-oracle/')]
        # Test/race commands compile enrollmentconfig and its dependencies, not
        # the changed fixture CLI. Vet of that CLI is executed again below.
        assert keys and all(k in before and previous[k]==before[k]for k in keys),'reused Go inputs differ'
        for label in ['go-env','go-version']:assert (GO_REFERENCE/(label+'.stdout')).read_bytes()==(OUTPUT/(label+'.stdout')).read_bytes(),'reused Go environment differs'
        for label in ['go-inputs-normal','go-inputs-race']:assert graph((GO_REFERENCE/(label+'.stdout')).read_bytes())==graph((OUTPUT/(label+'.stdout')).read_bytes()),'reused Go graph differs'
        (OUTPUT/'reused-go-checks.json').write_text(json.dumps(dict(reference=str(GO_REFERENCE),executed=False,checked_go_inputs=len(keys)),indent=2)+'\n')
    run('go-tests',['go','test','-json','-count=1','-timeout=0','-run','^Test(ConfigStore|Coordinated|Witness)','./internal/enrollmentconfig'])
    run('go-race',['go','test','-json','-race','-count=1','-timeout=0','-run','^Test(ConfigStoreConcurrentHandlesRejectBusyThenStale|CoordinatedSeparateDirectoriesAndConcurrentHandles)$','./internal/enrollmentconfig'])
    run('go-vet',['go','vet','./internal/enrollmentconfig','./tools/rust-migration/config-store-oracle'])
    run('rust-format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
    for label,flags in [('rust-tests',['--all-features']),('rust-default',[])]:run(label,['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-identity-contract','-p','swarm-wire-json','-p','swarm-enrollment-proof','-p','swarm-enrollment-state','-p','swarm-enrollment-authority','-p','swarm-enrollment-config',*flags,'--all-targets'])
    run('rust-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-identity-contract','-p','swarm-wire-json','-p','swarm-enrollment-proof','-p','swarm-enrollment-state','-p','swarm-enrollment-authority','-p','swarm-enrollment-config','--all-features','--all-targets','--','-D','warnings'])
    go=WORK/'config-store-oracle';target=Path(ENV['CARGO_TARGET_DIR']);native=target/'debug/examples/config_store_probe'
    run('go-build',['go','build','-o',str(go),'./tools/rust-migration/config-store-oracle'])
    run('rust-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-enrollment-config','--features','test-fixtures','--examples'])
    run('authority-rust-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-enrollment-authority','--features','test-fixtures','--example','authority_probe'])
    scenario=store_scenarios(go,native,WORK,OUTPUT,ENV);(OUTPUT/'store-scenarios.json').write_text(json.dumps(scenario,indent=2)+'\n')
    coordinated=coordinator_scenarios(go,native,WORK,OUTPUT,ENV);(OUTPUT/'coordinator-scenarios.json').write_text(json.dumps(coordinated,indent=2)+'\n')
    config_cases=compare('config-recheck',config_raw,target/'debug/examples/config_probe',['name','code','output'])
    authority_cases=compare('authority-recheck',authority_raw,target/'debug/examples/authority_probe',['name','code','output','digest'])
    default=default_guard();(OUTPUT/'default-guard.json').write_text(json.dumps(default,indent=2)+'\n')
    close_graphs();after=snapshot();(OUTPUT/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');assert before==after,'qualification source changed'
    go_pass=[e['Test']for e in map(json.loads,(OUTPUT/'go-tests.stdout').read_text().splitlines())if e.get('Action')=='pass'and'Test'in e]
    expected={name for p in (ROOT/'internal/enrollmentconfig').glob('*_test.go')for name in re.findall(r'^func (Test(?:ConfigStore|Coordinated|Witness)\w+)\(t \*testing.T\)',p.read_text(),re.M)}
    assert expected and {x for x in go_pass if '/'not in x}==expected,'Go tests not collected'
    race=[e['Test']for e in map(json.loads,(OUTPUT/'go-race.stdout').read_text().splitlines())if e.get('Action')=='pass'and'Test'in e];assert set(race)=={'TestConfigStoreConcurrentHandlesRejectBusyThenStale','TestCoordinatedSeparateDirectoriesAndConcurrentHandles'},'Go race test not collected'
    def passed(label):return [s for s in (OUTPUT/(label+'.stdout')).read_text().splitlines()if s.startswith('test ')and s.endswith(' ... ok')]
    assert len(passed('rust-tests'))==87 and len(passed('rust-default'))==78,'Rust tests not collected'
    (OUTPUT/'binaries').mkdir();binaries={}
    for name,p in [('go_store_oracle',go),('rust_store_probe',native),('rust_config_probe',target/'debug/examples/config_probe'),('rust_authority_probe',target/'debug/examples/authority_probe')]:shutil.copyfile(p,OUTPUT/'binaries'/name);binaries[name]=hashlib.sha256(p.read_bytes()).hexdigest()
    result=dict(status='PASS',scope='R2.4c protected Store and coordinated independent witness on Linux fixtures; human authentication and Rust engine still open',inputs=len(before),registry_packages=len(registry),source_stable=True,input_graph_closed=True,go_named_passes=go_pass,go_race_named_passes=race,rust_named_passes=passed('rust-tests'),rust_default_named_passes=passed('rust-default'),store_scenarios=scenario,coordinator_scenarios=coordinated,config_recheck_cases=config_cases,authority_recheck_cases=authority_cases,default_guard=default,binary_sha256=binaries,temporary_fixture_root=str(WORK),cargo_target_directory=ENV['CARGO_TARGET_DIR'],global_watchdog='none; Go -timeout=0',deployment=False,independent_review=False)
    (OUTPUT/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:result[k]for k in ['status','inputs','config_recheck_cases','authority_recheck_cases']}))
try:qualify()
except Exception as e:(OUTPUT/'failure.json').write_text(json.dumps(dict(type=type(e).__name__,cause=str(e),stage_count=len(commands)),indent=2)+'\n');raise
