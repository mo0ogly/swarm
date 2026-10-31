"""Current protected controls consume real retained prior fixture workers.
No provider rerun, SQL seed/write, real Root, account or task acceptance.
"""
import sys,os,json
from pathlib import Path
sys.path.insert(0,str(Path('tests').resolve()))
import rust_native_task_validation as native
base=Path(__file__).resolve().parent;out=base/'controls-after';out.mkdir(exist_ok=False)
build=json.loads((base/'candidate-controls-joined/result.json').read_text());binary=Path(build['binary'])
birth=json.loads((base/'control-workers-before/fixture.json').read_text());root=Path(birth['root']);work=birth['work']
environment={**os.environ,'CODEX_HOME':str(root/'codex-home')}
def call(label,args,value=None):
    """Retain exact public channels; refusal is never an empty success."""
    code,value=native.invoke(binary,root,out,label,args,value,environment=environment)
    assert code==0,(label,value)
    return value
w=call('show',['work','show',work])['work'];m=call('mission',['mission','show',work])
if m['paused']:call('mission-resume',['mission','resume',work],dict(schema_version=1,event_id='biba-controls-resume',expected_revision=w['revision'],reason='Only owned manual control fixture; no conductor launched'))
w=call('current',['work','show',work])['work']
if w['planning'].get('paused',False):call('planning-resume',['planning','resume',work],dict(schema_version=1,event_id='biba-controls-planning',expected_revision=w['revision'],reason='Authorize only manual control of retained fixture reports'))
w=call('source',['work','show',work])['work'];private_before=set((root/'.swarm').glob('biba-control-*'))
r=dict(schema_version=1,event_id='biba-control-candidate',expected_revision=w['revision'],task_id='task')
prepared=call('candidate',['task','validation-candidate',work],r);assert not prepared['process_started'] and not prepared['job_created']
w=call('after-candidate',['work','show',work])['work'];storage=Path(w['planning']['repository']['storage'])
found=[p for p in (storage/'candidates').glob('*/candidate.json') if json.loads(p.read_text())['task']=='task'];assert len(found)==1
candidate=json.loads(found[0].read_text());workspace=Path(candidate['workspace']);old=set(workspace.glob('biba-control-result-*.json'))
r=dict(schema_version=1,event_id='biba-protected-control',expected_revision=w['revision'],task_id='task')
reply=call('validate',['task','validate',work],r);assert reply['controls_passed'] and not reply['task_accepted']
new=set(workspace.glob('biba-control-result-*.json'))-old;assert len(new)==1
facts=json.loads(next(iter(new)).read_text());assert facts and not any(facts.values()),facts
assert set((root/'.swarm').glob('biba-control-*'))==private_before
replay=call('replay',['task','validate',work],r);assert replay['replayed'] and replay['job_id']==reply['job_id']
assert set(workspace.glob('biba-control-result-*.json'))-old==new
native.fixture.publish(out/'result.json',dict(status='PASS',facts=facts,one_actual_control=True,replay_without_execution=True,producer_binary=json.loads((base/'control-workers-before/result.json').read_text())['binary_sha256'],control_binary=build['sha256'],accepted=False,scope='Owned retained fixture only; no global qualification'))
print('PASS protected-managed-control-private-official-and-replay',flush=True)
