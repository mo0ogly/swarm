"""Source-bound request/shared-projection qualification and affected consumers."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
from qualification_inputs import collect
from request_vectors import vectors
from store_scenarios import run as store_scenarios
from coordinator_scenarios import run as coordinator_scenarios
from bootstrap_scenarios import run as bootstrap_scenarios
from cli_bootstrap_scenarios import run as cli_scenarios
ROOT=Path(__file__).resolve().parents[2];OUTPUT=Path(sys.argv[1]).resolve();EVIDENCE=ROOT/'docs/plans/migration-rust/evidence'
if OUTPUT.exists()and any(OUTPUT.iterdir()):raise SystemExit('Fresh evidence directory required')
OUTPUT.mkdir(parents=True,exist_ok=True);WORK=Path(tempfile.mkdtemp(prefix='swarm-request-contract-'));ENV=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(WORK/'target')));commands=[]
REFERENCES=dict(proof=EVIDENCE/'20261017-02/qualified-01',journal=EVIDENCE/'20261017-03/qualified-01',authority=EVIDENCE/'20261017-05/qualified-01',config=EVIDENCE/'20261017-06/qualified-03',store=EVIDENCE/'20261017-08/qualified-02')
def run(label,args,data=None,expected=0):
 start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic();p=subprocess.run(args,cwd=ROOT,env=ENV,input=data,capture_output=True)
 if data is not None:(OUTPUT/(label+'.stdin')).write_bytes(data)
 for ext,b in [('stdout',p.stdout),('stderr',p.stderr)]:(OUTPUT/(label+'.'+ext)).write_bytes(b)
 commands.append(dict(label=label,command=args,start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));(OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');print(label,'exit',p.returncode,flush=True)
 if p.returncode!=expected:raise RuntimeError(label+': original failure retained')
 return p.stdout

def execute(label,args,cwd,extra_env=None,expected=0,full_output=False):
 clock=time.monotonic();sink=open('/dev/full','wb')if full_output else subprocess.PIPE
 try:p=subprocess.run(args,cwd=cwd,env=dict(ENV,**(extra_env or{})),stdin=subprocess.DEVNULL,stdout=sink,stderr=subprocess.PIPE)
 finally:
  if full_output:sink.close()
 out=p.stdout if not full_output else b''
 for ext,b in [('stdout',out),('stderr',p.stderr)]:(OUTPUT/(label+'.'+ext)).write_bytes(b)
 commands.append(dict(label=label,command=args,cwd=str(cwd),selected_environment=extra_env or{},seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));(OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n')
 if p.returncode!=expected:raise RuntimeError(label+': original failure retained')
 if len(commands)%20==0:print('public CLI commands',len(commands),flush=True)
 return out,p.stderr,p.returncode

def go_reference(before,name):
 path=REFERENCES[name];assert json.loads((path/'result.json').read_text())['status']=='PASS';old=json.loads((path/'inputs-before.json').read_text())
 keys=[k for k in old if k in ['go.mod','go.sum']or k.startswith(('go-toolchain/','external-go/'))or(k.startswith(('internal/','tools/'))and k.endswith('.go'))]
 assert keys and all(before.get(k)==old[k]for k in keys),name+': original Go inputs changed'
 old_env='go-environment.stdout'if name=='proof'else'go-env.stdout'
 assert json.loads((path/old_env).read_text())==json.loads((OUTPUT/'go-env.stdout').read_text())
 assert (path/'go-version.stdout').read_bytes()==(OUTPUT/'go-version.stdout').read_bytes()
 (OUTPUT/('reused-'+name+'.json')).write_text(json.dumps(dict(reference=str(path),checked_go_inputs=len(keys),executed=False),indent=2)+'\n')
 return path

def compare(label,raw,binary,keys):
 out=run(label,[str(binary)],raw);vv=[json.loads(l)for l in raw.splitlines()];rr=[json.loads(l)for l in out.splitlines()];assert vv and len(vv)==len(rr)and len({v['name']for v in vv})==len(vv)
 differences=[dict(vector=v,actual=r)for v,r in zip(vv,rr)if any(v[k]!=r[k]for k in keys)];(OUTPUT/(label+'-mismatches.json')).write_text(json.dumps(differences,indent=2)+'\n');assert not differences,label+': divergence'
 return len(vv)

def default_guard():
 client=WORK/'default-consumer';(client/'src').mkdir(parents=True)
 deps={'swarm-enrollment-config':'swarm-enrollment-config','swarm-enrollment-bootstrap':'swarm-enrollment-bootstrap','swarm-enrollment-proof':'swarm-enrollment-proof'}
 (client/'Cargo.toml').write_text('[package]\nname="shared-default-consumer"\nversion="0.0.0"\nedition="2021"\n[dependencies]\n'+''.join(name+'={path='+json.dumps(str(ROOT/'rust/crates'/folder))+'}\n'for name,folder in deps.items()))
 shutil.copyfile(ROOT/'rust/Cargo.lock',client/'Cargo.lock');control='fn main(){let _=swarm_enrollment_config::store::Store::close;let _=swarm_enrollment_config::store::Coordinator::close;let _=swarm_enrollment_bootstrap::load;let _=swarm_enrollment_proof::Verifier::new_challenge;}\n'
 (client/'src/main.rs').write_text(control);run('default-lock',['cargo','generate-lockfile','--offline','--manifest-path',str(client/'Cargo.toml')]);checks=[]
 for name,denied,code in [('store','swarm_enrollment_config::store::Store::fixture_faults','E0599'),('coordinator','swarm_enrollment_config::store::Coordinator::fixture_hook','E0599'),('bootstrap','swarm_enrollment_bootstrap::load_with_faults','E0425'),('proof','swarm_enrollment_proof::Verifier::challenge_with_reader','E0599')]:
  (client/'src/main.rs').write_text(control);run('default-'+name+'-control',['cargo','check','--manifest-path',str(client/'Cargo.toml'),'--offline','--locked']);source='fn main(){let _='+denied+';}\n';(client/'src/main.rs').write_text(source);run('default-'+name+'-refusal',['cargo','check','--manifest-path',str(client/'Cargo.toml'),'--offline','--locked'],expected=101)
  err=(OUTPUT/('default-'+name+'-refusal.stderr')).read_text();assert code in err and denied.split('::')[-1]in err;checks.append(name);(OUTPUT/('default-'+name+'-refused.rs')).write_text(source)
 for name in ['Cargo.toml','Cargo.lock']:shutil.copyfile(client/name,OUTPUT/('default-'+name))
 main={(p['name'],p['version']):p.get('checksum')for p in tomllib.loads((ROOT/'rust/Cargo.lock').read_text())['package']};assert all(main.get((p['name'],p['version']))==p['checksum']for p in tomllib.loads((client/'Cargo.lock').read_text())['package']if p.get('source'))
 return checks

def qualify():
 packages=['./cmd/swarm','./internal/engine','./internal/enrollmentconfig','./internal/enrollmentstate','./internal/enrollmentproof','./internal/enrollmentauthority','./internal/enrollmentrequest','./internal/enrollmentbootstrap',*['./tools/rust-migration/'+p for p in ['request-oracle','model-layout','proof-oracle','enrollment-oracle','config-store-oracle','config-oracle','authority-oracle','bootstrap-oracle']]]
 before,snapshot,registry,close_graphs=collect(ROOT,OUTPUT,ENV,run,packages,allow_root=True)
 refs={name:go_reference(before,name)for name in REFERENCES}
 # The shared integer/projection code changed. Rebuild all native consumers and
 # repeat their tests; no whole Go suite or live mission is used.
 run('go-model-tests',['go','test','-json','-count=1','-timeout=0','-run','^TestTaskDefinitionEditPersistsAndReplays$','./internal/engine'])
 run('go-reply-tests',['go','test','-json','-count=1','-timeout=0','-run','^TestActiveReply','./internal/enrollmentstate'])
 run('go-vet',['go','vet',*packages])
 run('rust-format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
 for label,flags in [('rust-tests',['--all-features']),('rust-default',[])]:run(label,['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace',*flags,'--all-targets'])
 run('rust-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets','--','-D','warnings'])
 layout=json.loads(run('go-model-layout',['go','run','./tools/rust-migration/model-layout','internal/engine/model.go','internal/engine/task_definition.go']));(WORK/'layout.json').write_text(json.dumps(layout));generated=WORK/'request.rs';run('regenerate-models',['python3','tools/rust-migration/generate_request_models.py',str(WORK/'layout.json'),str(generated)]);run('format-generated',['rustfmt','--edition','2021',str(generated)]);assert generated.read_bytes()==(ROOT/'rust/crates/swarm-engine/src/request.rs').read_bytes(),'generated DTO differs'
 target=Path(ENV['CARGO_TARGET_DIR']);examples=target/'debug/examples';run('rust-examples',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--examples'])
 binaries={}
 for label,pkg in [('go-request','request-oracle'),('go-store','config-store-oracle'),('go-bootstrap','bootstrap-oracle')]:
  file=WORK/(label+'-bin');run(label+'-build',['go','build','-o',str(file),'./tools/rust-migration/'+pkg]);binaries[label]=file
 vv=vectors(layout);raw=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in vv);go_raw=run('go-request-vectors',[str(binaries['go-request'])],raw);native_raw=run('rust-request-vectors',[str(examples/'request_probe')],raw);gg=[json.loads(l)for l in go_raw.splitlines()];rr=[json.loads(l)for l in native_raw.splitlines()];assert len(gg)==len(rr)==len(vv)
 differences=[dict(input=v,go=g,rust=r)for v,g,r in zip(vv,gg,rr)if g!=r];(OUTPUT/'request-mismatches.json').write_text(json.dumps(differences,indent=2)+'\n');assert not differences,'typed request divergence'
 assert all(g['code']=='ok'for v,g in zip(vv,gg)if v['name'].startswith('control-')),'invalid fault control';assert any(g['code']!='ok'for g in gg)
 config_count=compare('config-corpus',(refs['config']/'go-vectors.stdout').read_bytes(),examples/'config_probe',['name','code','output'])
 authority_count=compare('authority-corpus',(refs['authority']/'go-vectors.stdout').read_bytes(),examples/'authority_probe',['name','code','output','digest'])
 proof_raw=(refs['proof']/'go-vectors.stdout').read_bytes();proof_out=run('proof-corpus',[str(examples/'proof_probe')],proof_raw);pv=[json.loads(l)for l in proof_raw.splitlines()];pr=[json.loads(l)for l in proof_out.splitlines()];assert len(pv)==len(pr)==825
 mismatch=[dict(vector=v,actual=r)for v,r in zip(pv,pr)if r['name']!=v['name']or r['code']!=v['expected']or r['recovery']!=v['recovery']or r['replay']!=v['replay']or not all(r[k]for k in ['signature_equal','payload_equal','request_equal'])];(OUTPUT/'proof-mismatches.json').write_text(json.dumps(mismatch,indent=2)+'\n');assert not mismatch
 strings=json.loads(run('proof-json-table',[str(examples/'proof_probe'),'--json-table']));assert strings==json.loads((refs['proof']/'go-json.stdout').read_text())and strings['string_cases']==1243136
 reply_input=(refs['journal']/'0004-go-reply-schema.stdin').read_bytes();reply_root=WORK/'reply-fixtures';reply_root.mkdir(mode=0o700);reply_out=run('reply-corpus',[str(examples/'enrollment_probe'),str(reply_root)],reply_input);old_reply=[json.loads(l)for l in(refs['journal']/'0004-go-reply-schema.stdout').read_text().splitlines()];assert [json.loads(l)for l in reply_out.splitlines()]==old_reply
 stored=store_scenarios(binaries['go-store'],examples/'config_store_probe',WORK,OUTPUT,ENV);coordinated=coordinator_scenarios(binaries['go-store'],examples/'config_store_probe',WORK,OUTPUT,ENV);boot=bootstrap_scenarios(binaries['go-bootstrap'],examples/'bootstrap_probe',WORK,OUTPUT,ENV,run)
 # Build the production/default CLI after feature-enabled fixture builds.
 run('default-cli-features',['cargo','tree','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-cli','--edges','normal,build,features']);assert b'test-fixtures'not in(OUTPUT/'default-cli-features.stdout').read_bytes();run('default-cli-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-cli','--bin','swarm-rust']);go_cli=WORK/'swarm-go';run('go-cli-build',['go','build','-o',str(go_cli),'./cmd/swarm']);public=cli_scenarios(go_cli,target/'debug/swarm-rust',WORK,OUTPUT,ENV,execute)
 guards=default_guard();close_graphs();after=snapshot();(OUTPUT/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');assert before==after,'qualification inputs changed'
 def passed(label):return [l for l in(OUTPUT/(label+'.stdout')).read_text().splitlines()if l.startswith('test ')and l.endswith(' ... ok')]
 assert len(passed('rust-tests'))==106 and len(passed('rust-default'))==96,'named native tests missing'
 go_names=[e['Test']for e in map(json.loads,(OUTPUT/'go-model-tests.stdout').read_text().splitlines())if e.get('Action')=='pass'and'Test'in e];assert go_names==['TestTaskDefinitionEditPersistsAndReplays']
 (OUTPUT/'binaries').mkdir();binary_hashes={}
 selected={**binaries,'swarm-go':go_cli,'swarm-rust':target/'debug/swarm-rust',**{name:examples/name for name in ['request_probe','config_probe','authority_probe','proof_probe','enrollment_probe','config_store_probe','bootstrap_probe']}}
 for name,path in selected.items():shutil.copyfile(path,OUTPUT/'binaries'/name);binary_hashes[name]=hashlib.sha256(path.read_bytes()).hexdigest()
 report=dict(status='PASS',scope='R3a typed Request/validation DTOs and shared Go projection; Work/Task transactions and complete engine remain open',inputs=len(before),registry_packages=len(registry),source_stable=True,input_graph_closed=True,generated_fields=sum(len(x)for x in layout['types'].values()),rust_named_passes=passed('rust-tests'),rust_default_named_passes=passed('rust-default'),go_model_named_passes=go_names,request_cases=len(vv),request_valid=sum(g['code']=='ok'for g in gg),config_cases=config_count,authority_cases=authority_count,proof_cases=len(pv),string_cases=strings['string_cases'],reply_cases=len(old_reply),store=stored,coordinator=coordinated,bootstrap=boot,public_cli=public,default_feature_guards=guards,binary_sha256=binary_hashes,temporary_fixture_root=str(WORK),cargo_target_directory=ENV['CARGO_TARGET_DIR'],global_watchdog='none; Go -timeout=0',deployment=False,independent_review=False)
 (OUTPUT/'result.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k]for k in ['status','inputs','request_cases','config_cases','authority_cases','proof_cases','string_cases','reply_cases']}))
try:qualify()
except Exception as e:(OUTPUT/'failure.json').write_text(json.dumps(dict(type=type(e).__name__,cause=str(e),stage_count=len(commands)),indent=2)+'\n');raise
