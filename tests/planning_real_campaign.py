"""Three real, isolated brief→planner→worker→verified Git deliveries; no manual acceptance."""
import json,pathlib,subprocess,sys,tempfile,time,uuid
binary=str(pathlib.Path(sys.argv[1]).resolve());out=pathlib.Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=True)
provider=json.loads(pathlib.Path(sys.argv[3]).read_text())['providers']['claude']
provider['args']=['-p','--output-format','stream-json','--verbose','--allowedTools','Read,Write,Edit,Bash(python3:*),Bash(git:*),Bash(mkdir:*)']
results=[]
for run in range(1,4):
 root=pathlib.Path(tempfile.mkdtemp(prefix='swarm-planning-real-'));source=root/'project';source.mkdir()
 def git(*args):
  return subprocess.run(['git','-c','user.name=Recipe','-c','user.email=recipe@localhost',*args],cwd=source,text=True,capture_output=True,check=True).stdout.strip()
 git('init');(source/'number.py').write_text('def answer():\n    return 0\n');(source/'test_number.py').write_text('import unittest\nfrom number import answer\nclass Check(unittest.TestCase):\n    def test_answer(self): self.assertEqual(answer(),42)\n');git('add','.');git('commit','-m','baseline');base=git('rev-parse','HEAD')
 def cli(args,data=None):
  p=subprocess.run([binary,'--root',str(root),'--json',*args]+(['--input','-'] if data is not None else []),input=json.dumps(data) if data is not None else None,text=True,capture_output=True,timeout=30)
  if p.returncode:raise RuntimeError(p.stdout+p.stderr)
  v=json.loads(p.stdout) if p.stdout.strip() else None
  return v.get('work',v) if isinstance(v,dict) and 'agents' not in v else v
 def change(args,w,**fields):return cli(args,dict(schema_version=1,event_id=uuid.uuid4().hex,expected_revision=w.get('revision',0),**fields))
 cli(['init']);(root/'.swarm/providers.json').write_text(json.dumps({'schema_version':1,'providers':{'claude':provider}}))
 w=change(['work','create'],{},title='Recette architecture réelle '+str(run),objective='Corriger number.py pour que answer() retourne 42. Créer exactement une tâche worker, puis clore le périmètre quand le résultat est accepté. Ne pas modifier test_number.py. Le worker rédige docs/<id-de-sa-tache>.md pour expliquer le changement et le test. Aucun autre besoin.',scope='Projet de recette isolé',criteria=['answer() retourne 42 et le test unittest passe'],next='Confier à une tâche puis vérifier')
 w=change(['planning','enable',w['id']],w,provider='claude',max_tasks=3,max_decisions=10,max_activations=12,repository={'path':str(source),'committed_only':True},checks={'req-1':[{'id':'answer-check','command':['python3','-B','-m','unittest','test_number.py'],'criteria':[1],'justification':'Le test vérifie answer()==42.','timeout_seconds':15}]})
 cli(['mission','start',w['id']],{'provider':'claude','role':'worker','workspace':str(source),'capture_output':True,'timeout_seconds':120})
 log=open(out/f'run-{run}.log','w');server=subprocess.Popen([binary,'--root',str(root),'web','127.0.0.1:0'],stdout=log,stderr=log)
 result={'run':run,'root':str(root),'work':w['id'],'base':base,'status':'FAIL'}
 try:
  deadline=time.monotonic()+240
  while time.monotonic()<deadline:
   current=cli(['work','show',w['id']]);p=current['planning']
   if p.get('failure'):raise AssertionError(p['failure'])
   if p['scopes'][0]['state']=='closed':break
   time.sleep(1)
  else:raise AssertionError('Délai atteint : '+json.dumps([(t['id'],t['status'],t.get('blocker')) for t in current['tasks']]))
  assert current['tasks'] and all(t['status']=='accepted' for t in current['tasks'])
  assert git('rev-parse','HEAD')==base and (source/'number.py').read_text()=='def answer():\n    return 0\n'
  bundle=out/f'run-{run}.bundle';cli(['planning','bundle',w['id'],str(bundle)])
  delivered=out/f'delivery-{run}';subprocess.run(['git','clone',str(bundle),str(delivered)],capture_output=True,check=True)
  subprocess.run(['python3','-B','-m','unittest','test_number.py'],cwd=delivered,capture_output=True,check=True)
  result.update(status='PASS',candidate=p['repository']['candidate_commit'],activations=p['activations'],decisions=p['decisions'],tasks=len(current['tasks']))
 except Exception as e:result['error']=str(e)
 finally:
  cli(['mission','stop',w['id']]);server.terminate();server.wait(timeout=15);log.close()
  for x in cli(['agent','list',w['id']])['agents']:
   a=x.get('agent',x)
   if a['status'] in ['queued','running','starting','stopping']:cli(['agent','stop',a['id']])
  (out/f'run-{run}-work.json').write_text(json.dumps(cli(['work','show',w['id']]),ensure_ascii=False,indent=2))
 results.append(result);(out/'campaign.json').write_text(json.dumps(results,ensure_ascii=False,indent=2));print(json.dumps(result,ensure_ascii=False),flush=True)
 if result['status']!='PASS':break
sys.exit(0 if len(results)==3 and all(r['status']=='PASS' for r in results) else 1)
