"""Consume a completed owned before-planning fixture with one candidate worker.
No planning rerun, live Root, SQL seeding, model/account or task acceptance.
The public worker must retain prompt/cost behavior while denying private and
outside-workspace writes, including an actual descendant. Preserve all channels.
"""
import sys,os,json
from pathlib import Path
sys.path.insert(0,str(Path('tests').resolve()))
import rust_agent_model_route as route
base=Path(__file__).resolve().parent
build=json.loads((base/'candidate-fioclex/result.json').read_text())
binary=Path(build['binary']);out=base/'managed-fioclex';out.mkdir(exist_ok=False)
f=route.Fixture.__new__(route.Fixture)
f.binary=binary;f.output=out;f.label='managed-fioclex';f.calls=[]
f.root=Path(json.loads((base/'managed-positive-setup/fixture.json').read_text())['root'])
f.work=json.loads((base/'managed-positive-setup/result.json').read_text())['planned_work']
f.environment={**os.environ,'CODEX_HOME':str(f.root/'codex-home')}
current=f.ok(['work','show',f.work])['work']
current=f.ok(['budget','apply',f.work],dict(schema_version=1,event_id='biba-fioclex-budget',expected_revision=current['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=1,estimate_source='Owned inert Biba fixture',reference_date='2026-10-28')))
f.ok(['task','add',f.work],dict(schema_version=1,event_id='biba-fioclex-task',expected_revision=current['revision'],id='biba-protected',title='Owned protected worker',deliverable='Synthetic confinement facts',criteria=['No acceptance']))
current=f.ok(['work','show',f.work])['work']
private=f.root/'.swarm'
probe='''import os,json
facts={}
for name,path,flags in PROBES:
 try:
  fd=os.open(path,flags,0o600);os.close(fd);facts[name]=True
 except OSError:facts[name]=False
with open('biba-facts.json','x') as file:json.dump(facts,file)
pid=os.fork()
if pid==0:
 try:
  fd=os.open(CHILD,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600);os.close(fd);os._exit(11)
 except OSError:os._exit(0)
_,status=os.waitpid(pid,0)
assert os.waitstatus_to_exitcode(status)==0
'''
probes=[('private_read',str(private/'engine-settings.json'),os.O_RDONLY),('database_write',str(private/'state.db'),os.O_RDWR),('private_write',str(private/'biba-canary-after'),os.O_WRONLY|os.O_CREAT|os.O_EXCL),('official_write',str(f.root/'biba-official-after'),os.O_WRONLY|os.O_CREAT|os.O_EXCL)]
probe=probe.replace('PROBES',repr(probes)).replace('CHILD',repr(str(private/'biba-child-after')))
program=route.PROGRAM.replace('import sys, os, json, hashlib',probe+'\nimport sys, os, json, hashlib',1)
route.publish(f.root/'codex',program.encode());(f.root/'codex').chmod(0o700)
f.request=dict(schema_version=1,event_id='biba-fioclex',expected_revision=current['revision'],task_id='biba-protected',provider='fixture',capture_output=True)
agent=f.prepare()['agent'];workspace=Path(agent['workspace'])
assert workspace==Path(current['planning']['repository']['storage'])/'copies/biba-protected-1'
assert f.reservation()==('reserved',1.0)
ran=f.ok(['agent','run','biba-fioclex']);route.publish(out/'run.json',ran)
assert ran['agent']['status']=='completed',ran
facts=json.loads((workspace/'biba-facts.json').read_text());assert not any(facts.values()),facts
observed=json.loads((workspace/'model-received.json').read_text())
assert observed['stdin_sha256']==route.digest(agent['prompt'].encode())
assert f.reservation()==('estimated',1.0)
work=f.ok(['work','show',f.work])['work'];task=next(t for t in work['tasks'] if t['id']=='biba-protected')
assert task['status']=='blocked' and len(task['attempts'])==1
route.publish(out/'result.json',dict(status='PASS',facts=facts,workspace=str(workspace),binary_sha256=route.digest(binary.read_bytes()),descendant_denied=True,prompt_exact=True,accepted=False,scope='One owned managed fixture; before planning then candidate worker, no planning qualification'))
print('PASS managed-output-private-root-and-descendant-denied',flush=True)
