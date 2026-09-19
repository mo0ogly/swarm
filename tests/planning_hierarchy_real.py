"""Real recursive responsibility and two independent coding branches."""
import json,pathlib,subprocess,sys,tempfile,time,uuid
binary=str(pathlib.Path(sys.argv[1]).resolve());out=pathlib.Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=True)
root=pathlib.Path(tempfile.mkdtemp(prefix='swarm-hierarchy-real-'));project=root/'project';project.mkdir()
provider=json.loads(pathlib.Path(sys.argv[3]).read_text())['providers']['claude'];provider['args']=['-p','--output-format','stream-json','--verbose','--allowedTools','Read,Write,Edit,Bash(python3:*),Bash(git:*),Bash(mkdir:*)']
def cli(args,data=None):
 p=subprocess.run([binary,'--root',str(root),'--json',*args]+(['--input','-'] if data is not None else []),input=json.dumps(data) if data is not None else None,text=True,capture_output=True,timeout=30)
 if p.returncode:raise RuntimeError(p.stdout+p.stderr)
 v=json.loads(p.stdout) if p.stdout.strip() else None
 return v.get('work',v) if isinstance(v,dict) and 'agents' not in v else v
def change(args,w,**fields):return cli(args,dict(schema_version=1,event_id=uuid.uuid4().hex,expected_revision=w.get('revision',0),**fields))
def git(*args):return subprocess.run(['git','-c','user.name=Recipe','-c','user.email=recipe@localhost',*args],cwd=project,capture_output=True,text=True,check=True).stdout.strip()
git('init');(project/'.gitignore').write_text('__pycache__/\n');(project/'alpha.py').write_text('def answer():\n    return 0\n');(project/'beta.py').write_text('def answer():\n    return 0\n');git('add','.');git('commit','-m','base');base=git('rev-parse','HEAD')
cli(['init']);(root/'.swarm/providers.json').write_text(json.dumps({'schema_version':1,'providers':{'claude':provider}}))
objective='Démontrer une hiérarchie à trois niveaux avec deux branches indépendantes. La racine conserve req-2 et crée une tâche worker pour beta.py, qui doit retourner 7. La racine délègue req-1 au périmètre alpha-owner avec cette consigne complète : alpha-owner doit déléguer req-1 à alpha-leaf, qui crée une tâche worker pour alpha.py afin que answer() retourne 42. Aucun autre changement de code. Chaque worker rédige docs/<task-id>.md puis termine. Chaque responsable clôt son périmètre seulement après acceptation de ses tâches et clôture de ses enfants. Aucun préalable croisé entre les branches. Les objectifs des délégations doivent contenir leurs consignes, fichiers, valeur attendue et règle de clôture.'
w=change(['work','create'],{},title='Recette hiérarchie réelle',objective=objective,scope='Deux modules isolés',criteria=['alpha.answer() retourne 42','beta.answer() retourne 7'],next='Déléguer et réaliser')
checks={}
for i,(mod,val) in enumerate([('alpha',42),('beta',7)],1):checks['req-'+str(i)]=[{'id':mod,'command':['python3','-B','-c',f'import {mod}; assert {mod}.answer()=={val}'],'criteria':[1],'justification':f'Compare la valeur renvoyée par {mod} à {val}.','timeout_seconds':15}]
w=change(['planning','enable',w['id']],w,provider='claude',max_tasks=6,max_decisions=16,max_activations=20,repository={'path':str(project),'committed_only':True},checks=checks)
cli(['mission','start',w['id']],{'provider':'claude','role':'worker','workspace':str(project),'capture_output':True,'timeout_seconds':150})
log=open(out/'server.log','w');server=subprocess.Popen([binary,'--root',str(root),'web','127.0.0.1:0'],stdout=log,stderr=log)
result={'status':'FAIL','root':str(root),'work':w['id'],'max_simultaneous_workers':0};start=time.monotonic()
try:
 while time.monotonic()-start<480:
  current=cli(['work','show',w['id']]);p=current['planning'];agents=cli(['agent','list',w['id']])['agents'];result['max_simultaneous_workers']=max(result['max_simultaneous_workers'],sum(x['agent']['status'] in ('running','starting') for x in agents))
  if p.get('failure'):raise AssertionError(p['failure'])
  if p['scopes'][0]['state']=='closed':break
  time.sleep(1)
 else:raise AssertionError('Délai atteint')
 assert len(p['scopes'])==3 and all(s['state']=='closed' for s in p['scopes'])
 assert len(current['tasks'])==2 and all(t['status']=='accepted' for t in current['tasks'])
 assert git('rev-parse','HEAD')==base
 bundle=out/'delivery.bundle';cli(['planning','bundle',w['id'],str(bundle)]);delivered=out/'delivery';subprocess.run(['git','clone',str(bundle),str(delivered)],capture_output=True,check=True);subprocess.run(['python3','-B','-c','import alpha,beta;assert alpha.answer()==42 and beta.answer()==7'],cwd=delivered,capture_output=True,check=True)
 result.update(status='PASS',scopes=3,tasks=2,decisions=p['decisions'],activations=p['activations'],candidate=p['repository']['candidate_commit'])
except Exception as e:result['error']=str(e)
finally:
 cli(['mission','stop',w['id']]);server.terminate();server.wait(timeout=15);log.close()
 for x in cli(['agent','list',w['id']])['agents']:
  a=x['agent']
  if a['status'] in ('queued','running','starting','stopping'):cli(['agent','stop',a['id']])
 (out/'work.json').write_text(json.dumps(cli(['work','show',w['id']]),ensure_ascii=False,indent=2));result['duration_seconds']=round(time.monotonic()-start,1);(out/'result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2));print(json.dumps(result,ensure_ascii=False),flush=True)
sys.exit(0 if result['status']=='PASS' else 1)
