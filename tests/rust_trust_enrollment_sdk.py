"""Link the additional SDK regression to retained default production artifacts.
Records current library bytes/profile provenance; no retroactive library hash.
No Cargo lock, engine rebuild, source copy, provider or global watchdog.
"""
from pathlib import Path
import json,hashlib,subprocess,os,argparse
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--profile',required=True);parser.add_argument('--fixture-deps',required=True);parser.add_argument('--output',required=True)
args=parser.parse_args();out=Path(args.output).resolve();out.mkdir()
profile=Path(args.profile).resolve();fixture_deps=Path(args.fixture_deps).resolve()
source_paths=[Path(__file__),Path('rust/crates/swarm-engine/tests/trust_enrollment.rs'),Path('rust/crates/swarm-enrollment-authority/tests/common/authenticated_fixture.rs'),Path('rust/Cargo.lock')]
sources={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in source_paths}
rows=[json.loads(s) for s in (profile/'cargo.stdout').read_text().splitlines() if s.startswith('{')]
names=['swarm_engine','swarm_enrollment_authority','swarm_enrollment_bootstrap','swarm_enrollment_state','swarm_enrollment_config','swarm_identity_contract','swarm_wire_json','serde_json','rusqlite','rustix','base64','ed25519_dalek','sha2','time']
argv=['rustc','--edition=2021','--test','rust/crates/swarm-engine/tests/trust_enrollment.rs','-C','debuginfo=0','-o',str(out/'probe')];inputs={}
for name in names:
 selected=[r for r in rows if r.get('reason')=='compiler-artifact' and r['target']['name']==name and 'lib' in r['target']['kind'] and (name!='sha2' or 'sha2@0.10.9' in r['package_id']) and (name!='base64' or 'base64@0.22.1' in r['package_id'])]
 assert len(selected)==1,(name,len(selected))
 row=selected[0];assert 'test-fixtures' not in row['features']
 filename=next(f for f in row['filenames'] if f.endswith('.rlib'));p=Path(filename);inputs[filename]=hashlib.sha256(p.read_bytes()).hexdigest();argv+=['--extern',name+'='+filename]
temp=list(fixture_deps.glob('libtempfile-*.rlib'));assert len(temp)==1,temp
inputs[str(temp[0])]=hashlib.sha256(temp[0].read_bytes()).hexdigest();argv+=['--extern','tempfile='+str(temp[0]),'-L','dependency='+str(Path(next(f for r in rows if r.get('reason')=='compiler-artifact' and r['target']['name']=='swarm_engine' for f in r['filenames'] if f.endswith('.rlib'))).parent),'-L','dependency='+str(fixture_deps)]
(out/'invocation.json').write_text(json.dumps({'argv':argv,'libraries':inputs,'sources_before':sources,'historical_library_hashes_available':False,'global_watchdog':None},indent=2)+'\n')
r=subprocess.run(argv,stdout=subprocess.PIPE,stderr=subprocess.PIPE);(out/'compile.stdout').write_bytes(r.stdout);(out/'compile.stderr').write_bytes(r.stderr);assert r.returncode==0,'compile refused; read original stderr'
for p,h in inputs.items():assert hashlib.sha256(Path(p).read_bytes()).hexdigest()==h,p
r=subprocess.run([str(out/'probe'),'enrolled_approver_cannot_bypass_exact_financial_validation','--exact','--nocapture'],stdout=subprocess.PIPE,stderr=subprocess.PIPE);(out/'run.stdout').write_bytes(r.stdout);(out/'run.stderr').write_bytes(r.stderr);assert r.returncode==0,r.stdout.decode();assert b'1 passed; 0 failed' in r.stdout
for p,h in sources.items():assert hashlib.sha256(Path(p).read_bytes()).hexdigest()==h,p
(out/'result.json').write_text(json.dumps({'status':'SDK_APPROVAL_PASS','named_tests':1,'sources_before_after':sources,'probe_sha256':hashlib.sha256((out/'probe').read_bytes()).hexdigest(),'global_acceptance':False,'no_historical_library_hash_claim':True},indent=2)+'\n');print('SDK_APPROVAL_PASS 1 named test')
