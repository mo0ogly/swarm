import json,subprocess,tempfile,uuid,hashlib,sys
from pathlib import Path
root=Path(tempfile.mkdtemp(prefix='swarm-web-recipe-'));binary=sys.argv[1]
def call(*args,data=None):
 p=subprocess.run([binary,'--root',str(root),'--json',*args,*(['--input','-'] if data is not None else [])],input=json.dumps(data) if data else None,text=True,capture_output=True)
 if p.returncode:raise RuntimeError(p.stderr)
 return json.loads(p.stdout)
def mutate(*args,revision=0,**kw):return call(*args,data=dict(schema_version=1,event_id=uuid.uuid4().hex,expected_revision=revision,**kw))['work']
call('init')
w=mutate('work','create',title='Recette du cockpit — parcours opérateur',objective='Vérifier les interactions réelles du terminal et du navigateur.',scope='Espace temporaire de recette ; aucun fichier Wattson modifié.',criteria=['Parcours UI complet'],next='Piloter la tâche de recette.')
w=mutate('task','add',w['id'],revision=w['revision'],id='UI-01',title='Vérifier le parcours complet',deliverable='Rapport de recette',criteria=['Lire, soumettre, évaluer et accepter depuis l’interface'],owner='codex')
w=mutate('task','add',w['id'],revision=w['revision'],id='UI-02',title='Vérifier la dépendance',deliverable='Rapport suivant',criteria=['La dépendance est respectée'],depends=['UI-01'])
(root/'docs').mkdir();(root/'docs/UI-01-handoff.md').write_text('# Recette du cockpit\nRapport de fixture pour les interactions UI.\nAucun résultat métier revendiqué.\n<img src=x onerror="window.injected=true">\n')
p='docs/UI-01-handoff.md'
evidence={'method_version':'2','scope_id':'UI-01','artifacts':{p:hashlib.sha256((root/p).read_bytes()).hexdigest()},'domains':{'quality':1},'checks':[{'id':'review','domain':'quality','mandatory':True,'gate':'delivery','penalty':100,'max_penalty':100,'severity':'major'}],'results':[{'id':'review','status':'PASS','count':0,'evidence':[p]}]}
(root/'docs/UI-01.evidence.json').write_text(json.dumps(evidence))
provider=root/'provider.py'
provider.write_text('#!/usr/bin/python3\nimport sys,json,time\nsys.stdin.read()\nprint(json.dumps({"type":"system"}),flush=True)\ntime.sleep(3)\nprint(json.dumps({"type":"result","usage":{"input_tokens":12,"output_tokens":8,"cache_read_input_tokens":19}}),flush=True)\n');provider.chmod(0o700)
(root/'.swarm/providers.json').write_text(json.dumps({'schema_version':1,'providers':{'recette':{'command':str(provider),'args':[],'env_allow':[]}}}))
print(root)
