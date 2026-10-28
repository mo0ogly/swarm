"""Qualify native read-only bootstrap; preserve exact sources and failures."""
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
import tomllib
from qualification_inputs import collect
from bootstrap_scenarios import run as scenarios
ROOT=Path(__file__).resolve().parents[2]
OUTPUT=Path(sys.argv[1]).resolve();REFERENCE=Path(sys.argv[2]).resolve()
GO_REUSE=Path(sys.argv[3]).resolve()if len(sys.argv)>3 else None
if OUTPUT.exists()and any(OUTPUT.iterdir()):raise SystemExit('Fresh evidence directory required')
OUTPUT.mkdir(parents=True,exist_ok=True);WORK=Path(tempfile.mkdtemp(prefix='swarm-rust-bootstrap-'))
ENV=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(WORK/'target')));commands=[]
def run(label,args,data=None,expected=0):
 start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic()
 if data is not None:(OUTPUT/(label+'.stdin')).write_bytes(data)
 if GO_REUSE and label in {'go-tests','go-vet'}:
  original=next(c for c in json.loads((GO_REUSE/'commands.json').read_text())if c['label']==label)
  assert original['exit']==original['expected_exit']==expected==0 and original['command']==args
  for ext in ['stdout','stderr']:shutil.copyfile(GO_REUSE/(label+'.'+ext),OUTPUT/(label+'.'+ext))
  commands.append(dict(label=label,command=args,exit=0,expected_exit=0,executed=False,reference=str(GO_REUSE)));(OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');print(label,'reused PASS',flush=True)
  return (OUTPUT/(label+'.stdout')).read_bytes()
 result=subprocess.run(args,cwd=ROOT,env=ENV,input=data,capture_output=True)
 for ext,payload in [('stdout',result.stdout),('stderr',result.stderr)]:(OUTPUT/(label+'.'+ext)).write_bytes(payload)
 commands.append(dict(label=label,command=args,start_utc=start,seconds=time.monotonic()-clock,exit=result.returncode,expected_exit=expected));(OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');print(label,'exit',result.returncode,flush=True)
 if result.returncode!=expected:raise RuntimeError(label+': original failure retained')
 return result.stdout

def reuse_core(before):
 old=json.loads((REFERENCE/'inputs-after.json').read_text());report=json.loads((REFERENCE/'result.json').read_text());assert report['status']=='PASS'
 keys=[k for k in old if k.startswith('rust/crates/')];assert keys and all(before.get(k)==old[k]for k in keys),'previous Rust components changed'
 # No registry revision may drift when adding a local package.
 def locked(path):return {(p['name'],p['version']):p['checksum']for p in tomllib.loads(path.read_text())['package']if p.get('source')}
 import tarfile
 with tarfile.open(REFERENCE/'sources.tar.gz')as archive:previous=tomllib.loads(archive.extractfile('rust/Cargo.lock').read().decode())
 previous={(p['name'],p['version']):p['checksum']for p in previous['package']if p.get('source')};assert previous==locked(ROOT/'rust/Cargo.lock')
 for name in ['rust-version','cc-version']:assert (REFERENCE/(name+'.stdout')).read_bytes()==(OUTPUT/(name+'.stdout')).read_bytes()
 assert json.loads((REFERENCE/'build-environment.json').read_text())==json.loads((OUTPUT/'build-environment.json').read_text()),'build flags changed'
 return dict(reference=str(REFERENCE),executed=False,checked_rust_inputs=len(keys),registry_lock_unchanged=True,rust_tests=len(report['rust_named_passes']),rust_default_tests=len(report['rust_default_named_passes']))

def default_guard():
 client=WORK/'default-client';(client/'src').mkdir(parents=True)
 (client/'Cargo.toml').write_text('[package]\nname="bootstrap-default-client"\nversion="0.0.0"\nedition="2021"\n[dependencies]\nswarm-enrollment-bootstrap={path='+json.dumps(str(ROOT/'rust/crates/swarm-enrollment-bootstrap'))+'}\n')
 shutil.copyfile(ROOT/'rust/Cargo.lock',client/'Cargo.lock')
 control='fn main(){let _=swarm_enrollment_bootstrap::load;}\n';denied='fn main(){let _=swarm_enrollment_bootstrap::load_with_faults;}\n'
 (client/'src/main.rs').write_text(control);run('default-client-lock',['cargo','generate-lockfile','--manifest-path',str(client/'Cargo.toml'),'--offline']);run('default-client-control',['cargo','check','--manifest-path',str(client/'Cargo.toml'),'--locked','--offline'])
 (client/'src/main.rs').write_text(denied);run('default-client-refusal',['cargo','check','--manifest-path',str(client/'Cargo.toml'),'--locked','--offline'],expected=101)
 err=(OUTPUT/'default-client-refusal.stderr').read_bytes();assert b'E0425'in err and b'load_with_faults'in err
 for name,data in [('default-control.rs',control),('default-refused.rs',denied)]:(OUTPUT/name).write_text(data)
 for name in ['Cargo.toml','Cargo.lock']:shutil.copyfile(client/name,OUTPUT/('default-'+name))
 main={(p['name'],p['version']):p.get('checksum')for p in tomllib.loads((ROOT/'rust/Cargo.lock').read_text())['package']}
 assert all(main.get((p['name'],p['version']))==p['checksum']for p in tomllib.loads((client/'Cargo.lock').read_text())['package']if p.get('source'))
 return dict(valid_consumer=True,fixture_api_absent=True,refusal_code='E0425')

def qualify():
 before,snapshot,registry,close_graphs=collect(ROOT,OUTPUT,ENV,run,['./internal/enrollmentbootstrap','./tools/rust-migration/bootstrap-oracle'])
 if GO_REUSE:
  from qualification_inputs import graph
  old=json.loads((GO_REUSE/'inputs-before.json').read_text());keys=[k for k in old if k in ['go.mod','go.sum']or k.startswith(('go-toolchain/','external-go/'))or(k.startswith(('internal/','tools/'))and k.endswith('.go'))]
  assert keys and all(before.get(k)==old[k]for k in keys),'Go reuse inputs changed'
  for label in ['go-env','go-version']:assert (GO_REUSE/(label+'.stdout')).read_bytes()==(OUTPUT/(label+'.stdout')).read_bytes()
  for label in ['go-inputs-normal','go-inputs-race']:assert graph((GO_REUSE/(label+'.stdout')).read_bytes())==graph((OUTPUT/(label+'.stdout')).read_bytes())
  (OUTPUT/'reused-go.json').write_text(json.dumps(dict(reference=str(GO_REUSE),checked_go_inputs=len(keys),overall_previous_status='incomplete: clippy failed; only individual successful Go checks reused'),indent=2)+'\n')
 reused=reuse_core(before);(OUTPUT/'reused-core.json').write_text(json.dumps(reused,indent=2)+'\n')
 run('go-tests',['go','test','-json','-count=1','-timeout=0','./internal/enrollmentbootstrap']);run('go-vet',['go','vet','./internal/enrollmentbootstrap','./tools/rust-migration/bootstrap-oracle'])
 run('rust-format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
 for label,flags in [('rust-tests',['--all-features']),('rust-default',[])]:run(label,['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-enrollment-bootstrap',*flags,'--all-targets'])
 run('rust-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-enrollment-bootstrap','--all-targets','--all-features','--','-D','warnings'])
 go=WORK/'bootstrap-oracle';rust=Path(ENV['CARGO_TARGET_DIR'])/'debug/examples/bootstrap_probe';run('go-build',['go','build','-o',str(go),'./tools/rust-migration/bootstrap-oracle']);run('rust-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-enrollment-bootstrap','--features','test-fixtures','--example','bootstrap_probe'])
 result=scenarios(go,rust,WORK,OUTPUT,ENV,run);guard=default_guard()
 close_graphs();after=snapshot();(OUTPUT/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');assert before==after,'qualification inputs changed'
 names=[e['Test']for e in map(json.loads,(OUTPUT/'go-tests.stdout').read_text().splitlines())if e.get('Action')=='pass'and'Test'in e]
 expected={n for p in (ROOT/'internal/enrollmentbootstrap').glob('*_test.go')for n in re.findall(r'^func (Test\w+)\(t \*testing.T\)',p.read_text(),re.M)};assert expected and {n for n in names if '/'not in n}==expected
 def passed(label):return [s for s in (OUTPUT/(label+'.stdout')).read_text().splitlines()if s.startswith('test ')and s.endswith(' ... ok')]
 assert len(passed('rust-tests'))==5 and len(passed('rust-default'))==4,'named Rust tests missing'
 (OUTPUT/'binaries').mkdir();binaries={}
 for name,p in [('go-bootstrap-oracle',go),('rust-bootstrap-probe',rust)]:shutil.copyfile(p,OUTPUT/'binaries'/name);binaries[name]=hashlib.sha256(p.read_bytes()).hexdigest()
 final=dict(status='PASS',scope='R2.5 native bootstrap declaration and read-only protected load; not human authentication, confinement or service startup',inputs=len(before),registry_packages=len(registry),source_stable=True,input_graph_closed=True,go_named_passes=names,rust_named_passes=passed('rust-tests'),rust_default_named_passes=passed('rust-default'),reused_core=reused,scenarios=result,default_guard=guard,binary_sha256=binaries,temporary_fixture_root=str(WORK),cargo_target_directory=ENV['CARGO_TARGET_DIR'],global_watchdog='none; Go -timeout=0',deployment=False,independent_review=False)
 (OUTPUT/'result.json').write_text(json.dumps(final,indent=2)+'\n');print(json.dumps(dict(status='PASS',inputs=len(before),scenarios=result),ensure_ascii=False))
try:qualify()
except Exception as e:(OUTPUT/'failure.json').write_text(json.dumps(dict(type=type(e).__name__,cause=str(e),stage_count=len(commands)),indent=2)+'\n');raise
