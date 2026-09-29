"""Explicitly paid parallel-provider qualification; never run by default CI.

Usage: python3 tests/parallel_real_campaign.py BINARY NEW_OUTPUT PROVIDERS_JSON
Two tasks are assigned through the public CLI before launch. Real Claude workers,
reviews and planner closure run afterwards, with no operator recovery. Limits:
6 planning activations (including setup), 4 reviews, 480 seconds. Provider limits
are preserved. Refuse an existing output directory; keep failed artifacts.
"""
import json,subprocess,time,uuid,sys,datetime,shutil
from pathlib import Path
source_engine=Path(sys.argv[1]).resolve();destination=Path(sys.argv[2]).resolve();provider_file=Path(sys.argv[3]).resolve()
from organized_fixture import add_owned_tasks, planning
out=destination;out.mkdir(mode=0o700);root=out/'store';root.mkdir(exist_ok=True);project=root/'project';project.mkdir(exist_ok=True);binary=out/'swarm';shutil.copy2(source_engine,binary)
def cmd(args,data=None):
 p=subprocess.run([str(binary),'--root',str(root),'--json',*args]+(['--input','-'] if data is not None else []),input=json.dumps(data) if data is not None else None,text=True,capture_output=True,timeout=60)
 with (out/'cli.jsonl').open('a') as f:f.write(json.dumps({'at':time.time(),'args':args,'exit':p.returncode})+'\n')
 if p.returncode:raise RuntimeError(p.stderr[:2000])
 return json.loads(p.stdout) if p.stdout.strip() else None
def change(args,w,**kw):
 x=cmd(args,dict(schema_version=1,event_id=uuid.uuid4().hex,expected_revision=w.get('revision',0),**kw));return x.get('work',x)
def git(*args):return subprocess.check_output(['git','-c','user.name=Trial','-c','user.email=trial@localhost',*args],cwd=project,stderr=subprocess.DEVNULL,text=True).strip()
git('init');(project/'README.md').write_text('Two independent modules: alpha.py answer() returns 11; beta.py answer() returns 22. Each worker edits only its assigned module and docs/<task-id>.* reports. No other files.');git('add','.');
if not (project/'.git/refs/heads/master').exists() and not (project/'.git/refs/heads/main').exists():git('commit','-m','Qualification baseline')
base_sha=git('rev-parse','HEAD')
cmd(['init']);config=json.loads(provider_file.read_text());(root/'.swarm/providers.json').write_text(json.dumps(config));(root/'.swarm/providers.json').chmod(0o600)
existing=cmd(['work','list'])
w=existing[0] if existing else change(['work','create'],{},title='Qualification — two real parallel agents',objective='Deux tâches déjà définies : ne pas en créer. Observer leurs résultats, puis fermer le périmètre racine quand les deux résultats sont acceptés. Aucun autre besoin.',scope='Disposable Python modules, two isolated copies, real Claude providers',criteria=['alpha.answer() returns 11','beta.answer() returns 22'],next='Exécuter les deux tâches puis fermer.')
checks={};tasks=[]
for n,(module,value) in enumerate([('alpha',11),('beta',22)],1):
 check=f"import runpy; d=runpy.run_path('{module}.py'); assert d['answer']()=={value}"
 checks[f'req-{n}']=[dict(id='exact',command=['python3','-B','-c',check],criteria=[1],justification='Execute the assigned module and verify its exact result',timeout_seconds=10)]
 tasks.append(dict(id=module,title=f'Implement {module}.py',requirements=[f'req-{n}'],deliverable=f'{module}.py and docs/{module}.md',criteria=[f'answer() returns {value}'],next=f'Write {module}.py with answer() returning integer {value}. Only modify this module and docs/{module}.* reports. Verify via python3. Do not touch the other module. Stop when done.'))
if not w.get('planning'):w=change(['planning','enable',w['id']],w,provider='claude',max_tasks=2,max_decisions=12,max_activations=6,max_review_calls=4,repository={'path':str(project),'committed_only':True},checks=checks)
scope=w['planning']['scopes'][0]
if not w['tasks']:
 if scope.get('holder'):
  w=planning(cmd,w,'decide',scope='root',scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'],input_events=[e['id'] for e in w['planning']['inbox'] if e['scope']=='root' and not e.get('decision')],reason='Assign two independent qualification tasks before launch',operations=[dict(kind='task',**t) for t in tasks])
 else:w=add_owned_tasks(cmd,w,tasks)
profile=dict(provider='claude',role='worker',workspace=str(project),capture_output=True,timeout_seconds=240,limits=config['providers']['claude']['limits'])
cmd(['autonomy',w['id'],'autonome','2']);log=(out/'server.log').open('w');server=None;started=time.time();result={'status':'FAIL','work':w['id']}
try:
 cmd(['mission','start',w['id']],profile);server=subprocess.Popen([str(binary),'--root',str(root),'web','127.0.0.1:0'],stdout=log,stderr=log)
 while time.time()-started<480:
  w=cmd(['work','show',w['id']])['work'];agents=[x['agent'] for x in cmd(['agent','list',w['id']])['agents']]
  (out/'observed.json').write_text(json.dumps({'work':w,'agents':agents}))
  if w['planning']['scopes'][0]['state']=='closed':break
  if w['planning'].get('failure'):raise RuntimeError(json.dumps([(t['id'],t['status'],t.get('blocker')) for t in w['tasks']]))
  time.sleep(10)
 assert len(agents)==2, f'Expected two producers without redundant execution, observed {len(agents)}'
 assert all(t['status']=='accepted' for t in w['tasks']), 'not two accepted tasks'
 assert w['planning']['scopes'][0]['state']=='closed','scope not closed'
 dt=lambda v:datetime.datetime.fromisoformat(v.replace('Z','+00:00')).timestamp()
 assert max(dt(a['started']) for a in agents)<min(dt(a['ended']) for a in agents),'producers did not overlap'
 assert len(set(a['workspace'] for a in agents))==2,'workspaces shared'
 assert all(t['independent_review']['state']=='passed' and t['automatic_validation']['state']=='accepted' for t in w['tasks'])
 bundle=out/'delivery.bundle';cmd(['planning','bundle',w['id'],str(bundle)]);delivery=out/'delivery';subprocess.run(['git','clone',str(bundle),str(delivery)],capture_output=True,check=True)
 subprocess.run(['python3','-B','-c','import alpha,beta; assert alpha.answer()==11 and beta.answer()==22'],cwd=delivery,check=True)
 assert git('rev-parse','HEAD')==base_sha and not (project/'alpha.py').exists(),'source modified'
 result.update(status='PASS',elapsed_seconds=round(time.time()-started,1),parallel=True,isolated=True,workers=2,planning_calls=w['planning']['activations'],review_calls=w['planning']['reviewer']['calls'],candidate=w['planning']['repository']['candidate_commit'],tools=[a.get('progress',{}).get('tool_calls') for a in agents],worker_cost_usd=[a.get('usage',{}).get('provider_reported_cost_usd') for a in agents],interventions_after_launch=0,planning_setup='tasks assigned through public CLI before launch')
except Exception as e:result['error']=str(e)
finally:
 cmd(['mission','stop',w['id']]);agents=[x['agent'] for x in cmd(['agent','list',w['id']])['agents']]
 for a in agents:
  if a['status'] in ['queued','running','starting','stopping']:cmd(['agent','stop',a['id']])
 if server:
  deadline=time.time()+30
  while time.time()<deadline:
   current=cmd(['work','show',w['id']])['work'];rows=[x['agent'] for x in cmd(['agent','list',w['id']])['agents']]
   if not any(a['status'] in ['queued','running','starting','stopping'] for a in rows) and not any((t.get('independent_review') or {}).get('state')=='running' for t in current['tasks']):break
   time.sleep(1)
  else:raise RuntimeError('shutdown not confirmed; server retained')
  server.terminate();server.wait(timeout=30)
 log.close();(out/'result.json').write_text(json.dumps(result,indent=2));print(json.dumps(result),flush=True)

raise SystemExit(0 if result["status"]=="PASS" else 1)
