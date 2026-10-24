"""Qualify the actual development CLI against Go, including root go:embed inputs."""
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
from cli_bootstrap_scenarios import run as scenarios
ROOT=Path(__file__).resolve().parents[2];OUTPUT=Path(sys.argv[1]).resolve();REFERENCE=Path(sys.argv[2]).resolve()
if OUTPUT.exists()and any(OUTPUT.iterdir()):raise SystemExit('Fresh evidence directory required')
OUTPUT.mkdir(parents=True,exist_ok=True);WORK=Path(tempfile.mkdtemp(prefix='swarm-rust-cli-'));ENV=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(WORK/'target')));commands=[]
def run(label,args,data=None,expected=0):
 start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic();p=subprocess.run(args,cwd=ROOT,env=ENV,input=data,capture_output=True)
 if data is not None:(OUTPUT/(label+'.stdin')).write_bytes(data)
 for ext,b in [('stdout',p.stdout),('stderr',p.stderr)]:(OUTPUT/(label+'.'+ext)).write_bytes(b)
 commands.append(dict(label=label,command=args,start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));(OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');print(label,'exit',p.returncode,flush=True)
 if p.returncode!=expected:raise RuntimeError(label+': original failure retained')
 return p.stdout

def execute(label,args,cwd,extra_env=None,expected=0,full_output=False):
 clock=time.monotonic();selected=dict(ENV,**(extra_env or {}));sink=open('/dev/full','wb')if full_output else subprocess.PIPE
 try:p=subprocess.run(args,cwd=cwd,env=selected,stdin=subprocess.DEVNULL,stdout=sink,stderr=subprocess.PIPE)
 finally:
  if full_output:sink.close()
 stdout=p.stdout if not full_output else b''
 for ext,b in [('stdout',stdout),('stderr',p.stderr)]:(OUTPUT/(label+'.'+ext)).write_bytes(b)
 commands.append(dict(label=label,command=args,cwd=str(cwd),selected_environment=extra_env or{},stdout_device='/dev/full'if full_output else'pipe',seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));(OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n')
 if p.returncode!=expected:raise RuntimeError(label+': unexpected exit; original output retained')
 print(label,'exit',p.returncode,flush=True);return stdout,p.stderr,p.returncode

def qualify():
 before,snapshot,registry,close_graphs=collect(ROOT,OUTPUT,ENV,run,['./cmd/swarm','./internal/engine','./internal/enrollmentbootstrap'],allow_root=True)
 old=json.loads((REFERENCE/'inputs-after.json').read_text());assert json.loads((REFERENCE/'result.json').read_text())['status']=='PASS';keys=[k for k in old if k.startswith('rust/crates/')];assert keys and all(before.get(k)==old[k]for k in keys),'previous native sources changed'
 for label in ['rust-version','cc-version']:assert (REFERENCE/(label+'.stdout')).read_bytes()==(OUTPUT/(label+'.stdout')).read_bytes()
 assert json.loads((REFERENCE/'build-environment.json').read_text())==json.loads((OUTPUT/'build-environment.json').read_text())
 import tarfile
 with tarfile.open(REFERENCE/'sources.tar.gz')as archive:previous=tomllib.loads(archive.extractfile('rust/Cargo.lock').read().decode())
 def external(lock):return {(p['name'],p['version']):p['checksum']for p in lock['package']if p.get('source')}
 assert external(previous)==external(tomllib.loads((ROOT/'rust/Cargo.lock').read_text()))
 (OUTPUT/'reused-native.json').write_text(json.dumps(dict(reference=str(REFERENCE),checked_rust_inputs=len(keys),executed=False,source_registry_flags_tools_unchanged=True,scope='seven libraries; this run checks the new consumer'),indent=2)+'\n')
 run('go-cli-tests',['go','test','-json','-count=1','-timeout=0','-run','^TestAgentIdentityBootstrapCLIReadOnlyWithoutProject$','./internal/engine'])
 run('go-vet',['go','vet','./cmd/swarm','./internal/engine'])
 run('rust-format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
 run('rust-tests',['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-cli','--all-targets'])
 run('rust-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-cli','--all-targets','--','-D','warnings'])
 tree=run('production-features',['cargo','tree','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-cli','--edges','normal,build,features']);assert b'test-fixtures'not in tree
 go=WORK/'swarm-go';rust=Path(ENV['CARGO_TARGET_DIR'])/'debug/swarm-rust';run('go-build',['go','build','-o',str(go),'./cmd/swarm']);run('rust-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-cli','--bin','swarm-rust'])
 result=scenarios(go,rust,WORK,OUTPUT,ENV,execute);(OUTPUT/'scenarios.json').write_text(json.dumps(result,indent=2)+'\n')
 close_graphs();after=snapshot();(OUTPUT/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');assert before==after,'candidate changed during qualification'
 go_names=[e['Test']for e in map(json.loads,(OUTPUT/'go-cli-tests.stdout').read_text().splitlines())if e.get('Action')=='pass'and'Test'in e];assert go_names==['TestAgentIdentityBootstrapCLIReadOnlyWithoutProject']
 rust_names=[l for l in(OUTPUT/'rust-tests.stdout').read_text().splitlines()if l.startswith('test ')and l.endswith(' ... ok')];assert len(rust_names)==8,'named Rust tests missing'
 (OUTPUT/'binaries').mkdir();binaries={}
 for name,p in [('swarm-go',go),('swarm-rust',rust)]:shutil.copyfile(p,OUTPUT/'binaries'/name);binaries[name]=hashlib.sha256(p.read_bytes()).hexdigest()
 report=dict(status='PASS',scope='R5a development identity bootstrap-check only; full CLI/engine/server and cutover remain open',inputs=len(before),root_embedded_files=len(json.loads((OUTPUT/'root-embedded-inputs.json').read_text())['normal']['EmbedFiles']),registry_packages=len(registry),source_stable=True,input_graph_closed=True,go_named_passes=go_names,rust_named_passes=rust_names,scenarios=result,binary_sha256=binaries,temporary_fixture_root=str(WORK),cargo_target_directory=ENV['CARGO_TARGET_DIR'],global_watchdog='none; Go -timeout=0',deployment=False,independent_review=False)
 (OUTPUT/'result.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k]for k in ['status','inputs','root_embedded_files']}))
try:qualify()
except Exception as e:(OUTPUT/'failure.json').write_text(json.dumps(dict(type=type(e).__name__,cause=str(e),stage_count=len(commands)),indent=2)+'\n');raise
