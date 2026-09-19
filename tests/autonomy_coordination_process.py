"""Isolated nominal A9: two deterministic producers, exchanges, consumer, controls."""
import hashlib, json, pathlib, subprocess, sys, tempfile, time, uuid
binary=str(pathlib.Path(sys.argv[1]).resolve());out=pathlib.Path(sys.argv[2]);out.mkdir(parents=True,exist_ok=True)
scenario=sys.argv[3] if len(sys.argv)>3 else 'nominal'
assert scenario in ['nominal','shared','replay','correction','transient','environment','dual','restart']
root=pathlib.Path(tempfile.mkdtemp(prefix='swarm-a9-'));calls=[]
def cli(args,data=None):
 p=subprocess.run([binary,'--root',str(root),'--json',*args]+(['--input','-'] if data is not None else []),input=json.dumps(data) if data is not None else None,text=True,capture_output=True,timeout=15)
 calls.append({'args':args,'mutation':data is not None,'at':time.time(),'code':p.returncode})
 if p.returncode:raise RuntimeError(p.stdout+p.stderr)
 return json.loads(p.stdout) if p.stdout.strip() else None
def mutate(args,w,**fields):
 return cli(args,dict(schema_version=1,event_id=uuid.uuid4().hex,expected_revision=w.get('revision',0),**fields))['work']
cli(['init']);(root/'docs').mkdir()
provider=root/'provider.py'
provider.write_text(r'''import hashlib,json,pathlib,re,subprocess,sys,time,uuid
root=pathlib.Path(sys.argv[1]);binary=sys.argv[2];scenario=sys.argv[3];prompt=sys.stdin.read()
w,task,agent,attempt=re.search(r"Coordination structurée : mission ([^,]+), tâche ([^,]+), agent ([^,]+), tentative ([^.]+)[.]",prompt).groups()
def cli(args,data=None):
 p=subprocess.run([binary,'--root',str(root),'--json',*args]+(['--input','-'] if data is not None else []),input=json.dumps(data) if data is not None else None,text=True,capture_output=True)
 if p.returncode:raise RuntimeError(p.stdout+p.stderr)
 return json.loads(p.stdout)
start=time.time()
if task=='p1' and scenario in ['transient','environment']:
 marker=root/'injected-process-fault'
 if not marker.exists():
  marker.write_text('injected')
  error='temporary failure: connection reset' if scenario=='transient' else "bwrap: Can't bind mount /oldroot/ on /newroot/: No such device"
  print(json.dumps({'type':'item.started','item':{'id':'fault','type':'command_execution','command':'recipe-probe','status':'in_progress'}}),flush=True)
  print(json.dumps({'type':'item.completed','item':{'id':'fault','type':'command_execution','command':'recipe-probe','aggregated_output':error,'exit_code':1,'status':'failed'}}),flush=True)
  raise SystemExit(1)
if task in ['p1','p2']:
 time.sleep(2)
 content=task+'\n'
 if scenario=='correction' and task=='p1':
  marker=root/'first-fault'
  if not marker.exists():marker.write_text('injected');content='wrong\n'
else:
 exchanges=cli(['exchange','list',w])
 work=cli(['work','show',w])['work'];current={t['id']:t['attempts'][-1]['id'] for t in work['tasks']}
 stale=[x for x in exchanges if current[x['source_task_id']]!=x['source_attempt_id']]
 for x in stale:
  try:cli(['exchange','consume',w],dict(schema_version=1,event_id=uuid.uuid4().hex,exchange_id=x['id'],agent_id=agent,task_id=task,attempt_id=attempt))
  except RuntimeError as e:assert 'obsolète' in str(e)
  else:raise AssertionError('ancienne remise acceptée')
 exchanges=[x for x in exchanges if current[x['source_task_id']]==x['source_attempt_id']];assert len(exchanges)==2
 for x in exchanges:
  cli(['exchange','consume',w],dict(schema_version=1,event_id=uuid.uuid4().hex,exchange_id=x['id'],agent_id=agent,task_id=task,attempt_id=attempt,acknowledgement='Empreintes contrôlées par le moteur'))
 content=''.join((root/'docs'/f'{p}-handoff.md').read_text() for p in ['p1','p2'])
rel='docs/'+task+'-handoff.md';(root/rel).write_text(content)
if task in ['p1','p2']:
 request=dict(schema_version=1,event_id=uuid.uuid4().hex,kind='handoff',agent_id=agent,task_id=task,attempt_id=attempt,recipient_task_id='consumer',recipient_role='worker',result_state='completed',artifacts=[dict(path=rel,sha256=hashlib.sha256((root/rel).read_bytes()).hexdigest())])
 first=cli(['exchange','send',w],request)
 if scenario=='replay':
  replay=cli(['exchange','send',w],request);assert replay['created'] is False and replay['exchange']['id']==first['exchange']['id']
(root/(task+'-interval.json')).write_text(json.dumps([start,time.time()]))
print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':'Rapport et échanges de recette produits.'}}),flush=True)
''')
(root/'.swarm/providers.json').write_text(json.dumps({'schema_version':1,'providers':{'recette':{'command':sys.executable,'args':[str(provider),str(root),binary,scenario],'env_allow':[]}}}))
w=mutate(['work','create'],{},title='A9 coordination nominale isolée',objective='Producteurs, échanges, consommateur, contrôleur',scope='recette déterministe',criteria=['coordination sans intervention'],next='lancer')
for tid,deps in [('p1',[]),('p2',[]),('consumer',['p1','p2'])]:
 (root/tid).mkdir()
 w=mutate(['task','add',w['id']],w,id=tid,title=tid,deliverable='docs/'+tid+'-handoff.md',criteria=['contenu exact'],depends=deps,next='Produire le rapport demandé')
 expected=tid+'\n' if tid!='consumer' else 'p1\np2\n'
 check="from pathlib import Path; assert Path('docs/"+tid+"-handoff.md').read_text() == "+repr(expected)
 w=mutate(['task','update',w['id']],w,id=tid,max_attempts=2,validation_policy={'mode':'automatic','controls':[{'id':'exact','command':['python3','-c',check],'criteria':[1],'justification':'Comparer le contenu complet au résultat déterministe attendu.','timeout_seconds':5}]})
 cli(['profile',w['id'],tid],{'provider':'recette','role':'worker','workspace':str(root if scenario=='shared' else root/tid),'capture_output':True,'timeout_seconds':30})
 w=cli(['work','show',w['id']])['work']
cli(['autonomy',w['id'],'autonome','2'])
cli(['mission','start',w['id']],{'provider':'recette','role':'worker','workspace':str(root),'capture_output':True,'timeout_seconds':30})
start_index=len(calls);started=time.monotonic();log=open(out/'server.log','w');server=subprocess.Popen([binary,'--root',str(root),'web'],stdout=log,stderr=log)
second=None;injected=False
if scenario=='dual':second=subprocess.Popen([binary,'--root',str(root),'web'],stdout=log,stderr=log)
try:
 while time.monotonic()-started<50:
  current=cli(['work','show',w['id']])['work']
  if scenario=='restart' and not injected and sum(t['status']=='running' for t in current['tasks'])>=2:
   server.kill();server.wait(timeout=5);server=subprocess.Popen([binary,'--root',str(root),'web'],stdout=log,stderr=log);injected=True
  if all(t['status']=='accepted' for t in current['tasks']):break
  if scenario=='environment' and {t['id']:t['status'] for t in current['tasks']}=={'p1':'blocked','p2':'accepted','consumer':'todo'}:
   time.sleep(6);current=cli(['work','show',w['id']])['work'];break
  time.sleep(.3)
 else:raise AssertionError([(t['id'],t['status'],t.get('blocker')) for t in current['tasks']])
 exchanges=cli(['exchange','list',w['id']])
 agents=[x['agent'] for x in cli(['agent','list',w['id']])['agents']]
 intervals={t:json.loads((root/(t+'-interval.json')).read_text()) for t in ['p1','p2','consumer'] if (root/(t+'-interval.json')).exists()}
 if scenario=='environment':
  assert {t['id']:t['status'] for t in current['tasks']}=={'p1':'blocked','p2':'accepted','consumer':'todo'}
  assert len(agents)==2 and len([a for a in agents if a['task_id']=='p1'])==1
  mission=cli(['mission','status',w['id']]);(out/'mission.json').write_text(json.dumps(mission,ensure_ascii=False,indent=2))
 else:
  assert sum(x['state']=='consumed' for x in exchanges)==2
  if scenario=='correction':assert len(exchanges)==3 and sum(x['state']=='stale' for x in exchanges)==1
  else:assert len(exchanges)==2
  assert len(agents)==(4 if scenario in ['correction','transient'] else 3)
  overlap=max(intervals['p1'][0],intervals['p2'][0])<min(intervals['p1'][1],intervals['p2'][1])
  if scenario=='shared':assert not overlap,intervals
  elif scenario not in ['correction','transient']:assert overlap,intervals
  assert intervals['consumer'][0]>=max(intervals['p1'][1],intervals['p2'][1])
  assert all(t['automatic_validation']['controller']!=t['automatic_validation']['producer_agent_id'] for t in current['tasks'])
 assert not any(c['mutation'] for c in calls[start_index:])
 result={'status':'PASS','scenario':scenario,'provider':'deterministic, not an AI provider','root':str(root),'elapsed_seconds':time.monotonic()-started,'external_interventions_after_launch':0,'attempts':len(agents),'intervals':intervals,'exchanges':exchanges,'work':current,'host_calls':calls[start_index:],'injected_faults':['conductor crash and restart'] if injected else [],'conductor_processes':2 if scenario=='dual' else 1}
 (out/'recipe.json').write_text(json.dumps(result,ensure_ascii=False,indent=2));print(json.dumps({k:result[k] for k in ['status','provider','elapsed_seconds','external_interventions_after_launch','attempts']}))
finally:
 if second:second.terminate();second.wait(timeout=10)
 cli(['mission','stop',w['id']]);server.terminate();server.wait(timeout=10);log.close()
 for x in cli(['agent','list',w['id']])['agents']:
  if x['agent']['status'] in ['running','starting','queued']:cli(['agent','stop',x['agent']['id']])
 (out/'host-calls.json').write_text(json.dumps(calls,indent=2))
