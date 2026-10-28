"""Qualify R2.3 authority documents on isolated data; no provider or mission operations."""
import datetime
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
from qualification_inputs import collect as collect_inputs
ROOT=Path(__file__).resolve().parents[2]
OUTPUT=Path(sys.argv[1]).resolve()
if OUTPUT.exists() and any(OUTPUT.iterdir()):raise SystemExit('Fresh evidence directory required')
OUTPUT.mkdir(parents=True,exist_ok=True)
# Honor TMPDIR so builds and fixtures can use the volume with observed free space.
WORK=Path(tempfile.mkdtemp(prefix='swarm-rust-enrollment-'))
ENV=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(WORK/'target')))
commands=[]

def run(label,args,data=None,expected=0):
    start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic()
    if data is not None:(OUTPUT/(label+'.stdin')).write_bytes(data)
    result=subprocess.run(args,cwd=ROOT,env=ENV,input=data,capture_output=True)
    (OUTPUT/(label+'.stdout')).write_bytes(result.stdout);(OUTPUT/(label+'.stderr')).write_bytes(result.stderr)
    commands.append(dict(label=label,command=args,start_utc=start,end_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),seconds=time.monotonic()-clock,exit=result.returncode,expected_exit=expected))
    (OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');print(label,'exit',result.returncode,flush=True)
    if result.returncode!=expected:raise RuntimeError(label+': unexpected exit; originals preserved')
    return result.stdout

def collect():
    return collect_inputs(ROOT, OUTPUT, ENV, run, ['./internal/enrollmentauthority','./internal/enrollmentrequest','./tools/rust-migration/authority-oracle'])

def qualify():
    before,snapshot,registry,close_graphs=collect()
    run('go-tests',['go','test','-json','-count=1','-timeout=0','./internal/enrollmentauthority','./internal/enrollmentrequest'])
    run('go-race',['go','test','-json','-race','-count=1','-timeout=0','-run','^TestChallengeBindingAndConcurrentValueCopies$','./internal/enrollmentrequest'])
    run('go-vet',['go','vet','./internal/enrollmentauthority','./internal/enrollmentrequest','./tools/rust-migration/authority-oracle'])
    run('rust-format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
    run('rust-tests',['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-identity-contract','-p','swarm-wire-json','-p','swarm-enrollment-proof','-p','swarm-enrollment-state','-p','swarm-enrollment-authority','--all-features','--all-targets'])
    run('rust-default',['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-identity-contract','-p','swarm-wire-json','-p','swarm-enrollment-proof','-p','swarm-enrollment-state','-p','swarm-enrollment-authority','--all-targets'])
    run('rust-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-identity-contract','-p','swarm-wire-json','-p','swarm-enrollment-proof','-p','swarm-enrollment-state','-p','swarm-enrollment-authority','--all-features','--all-targets','--','-D','warnings'])
    go=WORK/'authority-oracle';rust=Path(ENV['CARGO_TARGET_DIR'])/'debug/examples/authority_probe'
    run('go-build',['go','build','-o',str(go),'./tools/rust-migration/authority-oracle'])
    run('rust-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-enrollment-authority','--features','test-fixtures','--example','authority_probe'])
    raw=run('go-vectors',[str(go)]);out=run('rust-vectors',[str(rust)],raw)
    vectors=[json.loads(l)for l in raw.splitlines()];results=[json.loads(l)for l in out.splitlines()]
    assert vectors and len(vectors)==len(results) and len({v['name']for v in vectors})==len(vectors),'missing, extra or duplicate fixtures'
    mismatches=[dict(vector=v,rust=r)for v,r in zip(vectors,results)if any(v[k]!=r[k]for k in ['name','code','output','digest'])]
    (OUTPUT/'mismatches.json').write_text(json.dumps(mismatches,indent=2)+'\n');assert not mismatches,'Go/Rust authority divergence'
    close_graphs();after=snapshot();(OUTPUT/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');assert before==after,'qualification input changed'
    go_pass=[e['Test']for e in map(json.loads,(OUTPUT/'go-tests.stdout').read_text().splitlines())if e.get('Action')=='pass'and'Test'in e]
    import re
    expected={name for folder in ['enrollmentauthority','enrollmentrequest'] for p in (ROOT/'internal'/folder).glob('*_test.go')for name in re.findall(r'^func (Test\w+)\(t \*testing.T\)',p.read_text(),re.M)}
    assert {n for n in go_pass if '/'not in n}==expected and expected,'Go tests not collected'
    race_pass=[e['Test']for e in map(json.loads,(OUTPUT/'go-race.stdout').read_text().splitlines())if e.get('Action')=='pass'and'Test'in e]
    assert race_pass==['TestChallengeBindingAndConcurrentValueCopies'],'Go race test not collected'
    def passed(label):return [s for s in (OUTPUT/(label+'.stdout')).read_text().splitlines()if s.startswith('test ')and s.endswith(' ... ok')]
    assert len(passed('rust-tests'))==53 and len(passed('rust-default'))==52,'Rust tests not collected; inspect named contracts'
    (OUTPUT/'binaries').mkdir();binaries={}
    for name,p in [('go_oracle',go),('rust_probe',rust)]:shutil.copyfile(p,OUTPUT/'binaries'/name);binaries[name]=hashlib.sha256(p.read_bytes()).hexdigest()
    modes={mode:sum(v['mode']==mode for v in vectors)for mode in sorted({v['mode']for v in vectors})}
    report=dict(status='PASS',scope='R2.3 authority documents and immutable publication descriptor on fixtures; runtime identity and full engine still open',inputs=len(before),registry_packages=len(registry),source_stable=True,input_graph_closed=True,go_named_passes=go_pass,go_race_named_passes=race_pass,rust_named_passes=passed('rust-tests'),rust_default_named_passes=passed('rust-default'),protocol_cases=len(vectors),modes=modes,binary_sha256=binaries,temporary_fixture_root=str(WORK),cargo_target_directory=ENV['CARGO_TARGET_DIR'],global_watchdog='none; Go -timeout=0',deployment=False,independent_review=False)
    (OUTPUT/'result.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k]for k in ['status','inputs','protocol_cases','modes']}))
try:qualify()
except Exception as error:
    (OUTPUT/'failure.json').write_text(json.dumps(dict(type=type(error).__name__,error=str(error),stage_count=len(commands)),indent=2)+'\n');raise
