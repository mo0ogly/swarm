"""Retained ordinary controls lack an I0 context and must refuse before exec.
Park only the owned synthetic before-canary, retaining its bytes and hash.
This is a capability refusal, not a successful migration of ordinary work.
"""
import sys,json,os
from pathlib import Path
sys.path.insert(0,str(Path('tests').resolve()))
import rust_native_task_validation as native
base=Path(__file__).resolve().parent;out=base/'legacy-control-refusal';out.mkdir(exist_ok=False)
build=json.loads((base/'candidate-controls-joined/result.json').read_text());binary=Path(build['binary']);cases=[]
for label in ['valid','fault']:
 proof=json.loads((base/'validation-before'/label/(label+'-fixture.json')).read_text());root=Path(proof['root']);work=proof['work'];directory=out/label;directory.mkdir()
 canary=root/'.swarm/biba-validation-canary'
 if canary.exists():
  original=canary.read_bytes();parked=canary.with_name(canary.name+'.before-retained');assert not parked.exists();canary.rename(parked)
  native.fixture.publish(directory/'parked-before.json',dict(path=str(parked),sha256=native.fixture.digest(original)))
 env={**os.environ,'CODEX_HOME':str(root/'codex-home')}
 code,current=native.invoke(binary,root,directory,'show',['work','show',work],environment=env);assert code==0
 request=dict(schema_version=1,event_id='biba-legacy-refusal',expected_revision=current['work']['revision'],task_id='task')
 code,wire=native.invoke(binary,root,directory,'validate',['task','validate',work],request,environment=env)
 assert code!=0 and 'task-validation-i0-context-required' in json.dumps(wire),wire
 assert not canary.exists()
 cases.append(dict(label=label,refused=True,private_canary_absent=True))
native.fixture.publish(out/'result.json',dict(status='PASS',cases=cases,binary_sha256=build['sha256'],scope='Unsupported Root-cwd control context refuses; ordinary-context migration remains open',accepted=False))
print('PASS ordinary-control-i0-context-required-without-private-write',flush=True)
