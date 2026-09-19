"""Public CLI recipe: two real fake-provider processes, no human approval between tasks."""
import json, pathlib, subprocess, sys, tempfile, time, uuid
binary=str(pathlib.Path(sys.argv[1]).resolve())
out=pathlib.Path(sys.argv[2]);out.mkdir(parents=True,exist_ok=True)
root=pathlib.Path(tempfile.mkdtemp(prefix='swarm-auto-validation-'))
def cli(args,data=None):
 p=subprocess.run([binary,'--root',str(root),'--json',*args]+(['--input','-'] if data is not None else []),input=json.dumps(data) if data is not None else None,text=True,capture_output=True,timeout=15)
 if p.returncode:raise RuntimeError(p.stdout+p.stderr)
 return json.loads(p.stdout) if p.stdout.strip() else None
def mutate(args,w,**fields):
 x=cli(args,dict(schema_version=1,event_id=uuid.uuid4().hex,expected_revision=w.get('revision',0),**fields));return x.get('work',x)
cli(['init'])
provider=root/'provider.py'
provider.write_text('''import json,pathlib,re,sys
prompt=sys.stdin.read()
report=re.search(r"Livrable : (docs/[a-z0-9-]+[.]md)",prompt).group(1)
p=pathlib.Path(report);p.parent.mkdir(exist_ok=True);p.write_text("preuve locale\\n")
print(json.dumps({"type":"item.completed","item":{"type":"agent_message","text":"Rapport de recette produit."}}),flush=True)
''')
(root/'.swarm/providers.json').write_text(json.dumps({'schema_version':1,'providers':{'recette':{'command':sys.executable,'args':[str(provider)],'env_allow':[]}}}))
w=mutate(['work','create'],{},title='Chaîne automatique isolée',objective='Deux tâches sans clic intermédiaire',scope='recette locale',criteria=['deux rapports validés'],next='lancer')
for tid,deps in [('t1',[]),('t2',['t1'])]:
 w=mutate(['task','add',w['id']],w,id=tid,title=tid,owner='recette',deliverable='docs/'+tid+'.md',criteria=['rapport local exact'],depends=deps,next='Produire uniquement le livrable demandé')
 w=mutate(['task','update',w['id']],w,id=tid,validation_policy={'mode':'automatic','controls':[{'id':'report-check','command':['python3','-c',"from pathlib import Path; assert Path('docs/"+tid+".md').read_text() == 'preuve locale\\n'"],'criteria':[1],'justification':'Le contrôle compare le rapport complet au contenu exact attendu.','timeout_seconds':5}]})
cli(['mission','start',w['id']],{'provider':'recette','role':'worker','workspace':str(root),'capture_output':True,'timeout_seconds':30})
log=open(out/'server.log','w');server=subprocess.Popen([binary,'--root',str(root),'web'],stdout=log,stderr=log)
try:
 deadline=time.monotonic()+40
 while time.monotonic()<deadline:
  current=cli(['work','show',w['id']]);current=current.get('work',current)
  if all(t['status']=='accepted' for t in current['tasks']):break
  time.sleep(.4)
 else:raise AssertionError([(t['id'],t['status'],t.get('blocker')) for t in current['tasks']])
 agents=[x.get('agent',x) for x in cli(['agent','list',w['id']])['agents']]
 assert len(agents)==2 and all(a['status']=='completed' for a in agents)
 for t in current['tasks']:
  receipt=t['automatic_validation'];assert receipt['state']=='accepted' and receipt['attempt_id']
  assert receipt['producer_agent_id'] != receipt['controller'] and receipt['controller'].startswith('controller://')
  assert (root/receipt['receipt']).is_file()
 result={'status':'PASS','root':str(root),'checks':['deux processus réels de fournisseur factice','politique par tâche enregistrée avant lancement','chaîne acceptée sans clic intermédiaire','deux tentatives seulement','reçus liés aux tentatives et consultables'],'tasks':[{k:t[k] for k in ['id','status','automatic_validation']} for t in current['tasks']]}
 (out/'recipe.json').write_text(json.dumps(result,ensure_ascii=False,indent=2));print(json.dumps({'status':'PASS','checks':result['checks']},ensure_ascii=False))
finally:
 cli(['mission','stop',w['id']]);server.terminate();server.wait(timeout=10);log.close()
 for x in cli(['agent','list',w['id']])['agents']:
  a=x.get('agent',x)
  if a['status'] in ['running','starting','queued']:cli(['agent','stop',a['id']])
