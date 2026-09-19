"""Deterministic capacity recipe: twenty scripted tasks, three real fake-worker processes."""
import json,pathlib,subprocess,sys,tempfile,time,uuid
binary=str(pathlib.Path(sys.argv[1]).resolve());out=pathlib.Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=True);root=pathlib.Path(tempfile.mkdtemp(prefix='swarm-capacity-'));project=root/'project';project.mkdir()
def cli(args,data=None):
 p=subprocess.run([binary,'--root',str(root),'--json',*args]+(['--input','-'] if data else []),input=json.dumps(data) if data else None,text=True,capture_output=True,timeout=30)
 if p.returncode:raise RuntimeError(p.stdout+p.stderr)
 v=json.loads(p.stdout) if p.stdout.strip() else None
 return v.get('work',v) if isinstance(v,dict) and 'agents' not in v else v
def change(args,w,**fields):return cli(args,dict(schema_version=1,event_id=uuid.uuid4().hex,expected_revision=w.get('revision',0),**fields))
def git(*args):return subprocess.run(['git','-c','user.name=Recipe','-c','user.email=recipe@local',*args],cwd=project,capture_output=True,text=True,check=True).stdout.strip()
git('init');(project/'baseline.txt').write_text('baseline');git('add','.');git('commit','-m','baseline');base=git('rev-parse','HEAD');cli(['init'])
worker=root/'worker.py';worker.write_text('import sys,re,pathlib,time\nprompt=sys.stdin.read()\ntask=re.search(r"Tâche (t[0-9]+) :",prompt).group(1)\npathlib.Path("docs").mkdir(exist_ok=True)\npathlib.Path("docs/"+task+".md").write_text("Rapport factice "+task)\npathlib.Path(task+".txt").write_text(task)\ntime.sleep(4)\n')
(root/'.swarm/providers.json').write_text(json.dumps({'schema_version':1,'providers':{'fixture':{'command':'/usr/bin/python3','args':[str(worker)],'env_allow':[]}}}))
w=change(['work','create'],{},title='Capacité déterministe vingt tâches',objective='Tester trois copies simultanées',scope='fournisseurs factices uniquement',criteria=[f't{i}.txt contient t{i}' for i in range(20)],next='lancer')
checks={f'req-{i+1}':[{'id':f'check-{i}','command':['python3','-B','-c',f'from pathlib import Path;assert Path("t{i}.txt").read_text()=="t{i}"'],'criteria':[1],'justification':'Compare le fichier à sa valeur attendue.','timeout_seconds':5}] for i in range(20)}
w=change(['planning','enable',w['id']],w,max_tasks=20,max_decisions=5,max_activations=5,repository={'path':str(project),'committed_only':True},checks=checks)
w=change(['planning','claim',w['id']],w,scope='root',scope_revision=1,holder='scripted-recipe',lease_seconds=60);scope=w['planning']['scopes'][0]
w=change(['planning','decide',w['id']],w,scope='root',scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'],input_events=[e['id'] for e in w['planning']['inbox']],reason='Plan scripté de charge ; aucune IA',operations=[{'kind':'task','id':f't{i}','title':f'Tâche {i}','requirements':[f'req-{i+1}'],'deliverable':f'docs/t{i}.md','criteria':['valeur exacte'],'next':'Écrire le fichier'} for i in range(20)])
cli(['autonomy',w['id'],'autonome','3']);cli(['mission','start',w['id']],{'provider':'fixture','role':'worker','workspace':str(project),'timeout_seconds':30})
log=open(out/'server.log','w');server=subprocess.Popen([binary,'--root',str(root),'web','127.0.0.1:0'],stdout=log,stderr=log);start=time.monotonic();max_active=0;result={'status':'FAIL','root':str(root),'work':w['id'],'provider':'script Python factice ; aucune IA'}
try:
 while time.monotonic()-start<180:
  current=cli(['work','show',w['id']]);agents=cli(['agent','list',w['id']])['agents'];max_active=max(max_active,sum(x['agent']['status'] in ('running','starting') for x in agents))
  if all(t['status']=='accepted' for t in current['tasks']):break
  time.sleep(.2)
 else:raise AssertionError([(t['id'],t['status'],t.get('blocker')) for t in current['tasks']])
 assert len(agents)==20 and max_active==3
 assert git('rev-parse','HEAD')==base
 bundle=out/'delivery.bundle';cli(['planning','bundle',w['id'],str(bundle)])
 result.update(status='PASS',tasks=20,attempts=len(agents),max_simultaneous_workers=max_active,duration_seconds=round(time.monotonic()-start,1))
except Exception as e:result['error']=str(e)
finally:
 cli(['mission','stop',w['id']]);server.terminate();server.wait(timeout=15);log.close()
 for x in cli(['agent','list',w['id']])['agents']:
  if x['agent']['status'] in ('running','starting','queued','stopping'):cli(['agent','stop',x['agent']['id']])
 (out/'result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2));print(json.dumps(result,ensure_ascii=False),flush=True)
sys.exit(0 if result['status']=='PASS' else 1)
