#!/usr/bin/env python3
"""Real 80x24 PTY, native typing/paste, fixture provider; never a real model."""
import os,pty,select,signal,struct,fcntl,termios,time,json,tempfile,subprocess,sys,pathlib,hashlib,datetime
binary=os.path.abspath(sys.argv[1]);out=pathlib.Path(sys.argv[2]);out.mkdir(parents=True,exist_ok=True);root=pathlib.Path(tempfile.mkdtemp(prefix='swarm-prepare-pty-'))
def cli(args,data=None):
 return json.loads(subprocess.check_output([binary,'--root',str(root),*args,*(['--input','-'] if data else [])],input=json.dumps(data).encode() if data else None))
cli(['init'])
for name in ['.claude/skills/apex/SKILL.md','tools/agent-workflows/CONTRACT.md']:
 p=root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text('Préparation uniquement.')
plan={'version':1,'objective':'Recette terminal','assumptions':[],'questions':[],'tasks':[{'id':'T1','title':'Rédiger','role':'worker','scope':'docs','deliverable':'docs/rapport.md','criteria':['Document relu'],'depends':[],'proof':'Rapport','entry':'Brief adopté','validation':'Revue','delivery':'Preuves fraîches','stop':'Arrêt sur blocage','max_attempts':2,'max_tool_calls':10}]}
fake=root/'claude';fake.write_text('''#!/usr/bin/python3
import sys,json,time
prompt=sys.stdin.read()
if 'INTERROMPRE_CET_ECHANGE' in prompt: time.sleep(3)
schema=json.loads(sys.argv[sys.argv.index('--json-schema')+1])
answer={'message':'Voici la proposition demandée.','brief':'# Brief\\nObjectif : cadrer le projet et vérifier le plan.'}
if 'plan' in schema['properties']:answer={'message':'Plan proposé.','plan':json.dumps(PLAN)}
print(json.dumps({'type':'result','result':json.dumps(answer)}))
'''.replace('PLAN',repr(plan)));fake.chmod(0o700);(root/'.swarm/providers.json').write_text(json.dumps({'schema_version':1,'providers':{'recette':{'command':str(fake)}}}))
transcript=bytearray();children=[]
def start(args):
 pid,fd=pty.fork()
 if pid==0:os.execv(binary,[binary,'--root',str(root),*args])
 children.append(pid);fcntl.ioctl(fd,termios.TIOCSWINSZ,struct.pack('HHHH',24,80,0,0));return pid,fd
def expect(fd,needle,start=0,timeout=12):
 end=time.monotonic()+timeout;wanted=needle.encode()
 while time.monotonic()<end:
  if wanted in transcript[start:]:return
  if select.select([fd],[],[],.1)[0]:
   try:b=os.read(fd,65536)
   except OSError:break
   if not b:break
   transcript.extend(b)
 raise AssertionError('Absent: '+needle+'\n'+transcript[start:].decode(errors='replace'))
def send(fd,text,expected):
 start=len(transcript);os.write(fd,text.encode()+b'\r');expect(fd,expected,start)
def turn():return cli(['prepare','dialogue',prep])[-1]
def wait_status(turn_id,status,timeout=6):
 end=time.monotonic()+timeout
 while time.monotonic()<end:
  current=next(item for item in cli(['prepare','dialogue',prep]) if item['id']==turn_id)
  if current['status']==status:return current
  time.sleep(.1)
 raise AssertionError('État absent pour '+turn_id+' : '+status)
checks=[]
try:
 pid,fd=start(['prepare']);expect(fd,'préparer>')
 send(fd,'Un besoin avec accents éà, commun au web et au CLI.','Préparation :')
 prep=cli(['prepare','list'])['preparations'][0]['id'];assert cli(['prepare','show',prep])['documents']['besoin']['text'].startswith('Un besoin avec accents')
 send(fd,'/ia recette','IA sélectionnée');send(fd,'/contexte recent','dernier échange');send(fd,'Propose un brief.','Envoi enregistré');expect(fd,'Proposition de brief :');tid=turn()['id'];assert turn()['context_mode']=='recent'
 send(fd,'/appliquer '+tid,'/confirmer ou /annuler >')
 before=cli(['prepare','show',prep])['revision'];send(fd,'\x1b[200~/confirmer\x1b[201~','le collage ne confirme pas');assert cli(['prepare','show',prep])['revision']==before
 send(fd,'/confirmer','Enregistré');assert 'adopted_brief' not in cli(['prepare','show',prep]);send(fd,'/adopter','/confirmer ou /annuler >');send(fd,'/confirmer','Enregistré')
 checks.append('PTY 80x24 : besoin Unicode, dialogue, comparaison, collage ne confirme pas, adoption distincte')
 send(fd,'INTERROMPRE_CET_ECHANGE','Envoi enregistré');stopped=turn()['id'];send(fd,'/arreter','Arrêt :');wait_status(stopped,'interrupted');send(fd,'/reprendre','Envoi enregistré');resumed=turn()['id'];assert resumed!=stopped;send(fd,'/arreter','Arrêt :');wait_status(resumed,'interrupted')
 checks.append('Arrêt confirmé puis /reprendre crée une nouvelle tentative explicite')
 send(fd,'/plan','Envoi enregistré');expect(fd,'Proposition de plan :');tid=turn()['id'];send(fd,'/appliquer '+tid,'/confirmer ou /annuler >');send(fd,'/confirmer','Enregistré');send(fd,'/verifier','Plan vérifié');assert cli(['prepare','show',prep])['plan_ready']
 checks.append('Plan proposé, enregistré puis vérifié depuis le même terminal')
 send(fd,'/creer-missions','/confirmer ou /annuler >');send(fd,'\x1b[200~/confirmer\x1b[201~','le collage ne confirme pas');assert not cli(['prepare','show',prep]).get('conversion');send(fd,'/confirmer','autorisés : false.');created=cli(['prepare','show',prep]);assert len(created['conversion']['task_ids'])==1
 send(fd,'/autoriser-missions','/confirmer ou /annuler >');send(fd,'/annuler','Action abandonnée');assert not cli(['prepare','show',prep])['conversion'].get('released_at')
 script=cli(['prepare','create'],{'version':1,'event_id':'script-create','expected_revision':0,'action':'create','title':'Parité JSON','text':'Un besoin avec accents éà, commun au web et au CLI.'})
 script_events=0
 def mutate(action,**values):
  global script,script_events
  script_events+=1
  request={'version':1,'preparation_id':script['id'],'event_id':'script-'+str(script_events)+'-'+action,'expected_revision':script['revision'],'action':action};request.update(values);script=cli(['prepare',action,script['id']],request);return script
 mutate('save',document='brief',text='# Brief\nObjectif : cadrer le projet et vérifier le plan.')
 mutate('adopt-brief',sha256=script['documents']['brief']['sha256'])
 mutate('save',document='plan',text=json.dumps(plan,ensure_ascii=False))
 mutate('validate-plan',sha256=script['documents']['plan']['sha256'])
 review=cli(['prepare','conversion',script['id']]);mutate('create-missions',sha256=script['documents']['plan']['sha256'],expected_work_revision=review['work_revision'])
 assert script['plan_ready'] and len(script['conversion']['task_ids'])==len(created['conversion']['task_ids']) and not script['conversion'].get('released_at') and json.loads(script['documents']['plan']['text'])==json.loads(created['documents']['plan']['text'])
 checks.append('Script JSON versionné produit le même plan validé et les mêmes missions verrouillées que le REPL')
 send(fd,'/autoriser-missions','/confirmer ou /annuler >');send(fd,'/confirmer','autorisés : true.');assert cli(['prepare','show',prep])['conversion']['released_at'];checks.append('Création et autorisation séparées, collage ne confirme pas, annulation conserve le verrou')
 calls=len(cli(['prepare','dialogue',prep]));send(fd,'/multiligne besoin','Saisie multiligne');send(fd,'\x1b[200~Ligne une\n/plan\nLigne trois\x1b[201~','... ');send(fd,'/envoyer','Enregistré');assert cli(['prepare','show',prep])['documents']['besoin']['text']=='Ligne une\n/plan\nLigne trois';assert len(cli(['prepare','dialogue',prep]))==calls
 os.write(fd,b'texte abandonne\x03');expect(fd,'Saisie abandonnée.');send(fd,'/voir besoin','Ligne trois')
 fcntl.ioctl(fd,termios.TIOCSWINSZ,struct.pack('HHHH',30,100,0,0));send(fd,'/aide','Ctrl-C abandonne');os.write(fd,b'/quitter\r');os.waitpid(pid,0);children.remove(pid);assert termios.tcgetattr(fd)[3]&termios.ICANON;os.close(fd)
 checks.append('Collage multiligne littéral, Ctrl-C garde la session, redimensionnement et restauration du terminal')
 offset=len(transcript);pid,fd=start(['prepare','resume',prep,'--plain']);expect(fd,'préparer>',offset);send(fd,'/voir plan','Recette terminal');os.write(fd,b'\x04');os.waitpid(pid,0);children.remove(pid)
 while select.select([fd],[],[],.1)[0]:
  try:transcript.extend(os.read(fd,65536))
  except OSError:break
 os.close(fd);assert b'\x1b' not in transcript[offset:];checks.append('Reprise en --plain sans séquence ANSI ; EOF ferme la vue')
 result=subprocess.run([binary,'--root',str(root),'prepare','chat','--json'],input='',capture_output=True,text=True);assert result.returncode==2 and result.stdout=='';checks.append('stdin redirigé / --json : aucune invite interactive ni appel implicite')
 candidate_sha256=hashlib.sha256(pathlib.Path(binary).read_bytes()).hexdigest()
 evidence={'status':'PASS','checks':checks,'root':str(root),'provider':'factice','candidate_sha256':candidate_sha256,'generated_at':datetime.datetime.now(datetime.timezone.utc).isoformat()}
 (out/'pty.txt').write_bytes(transcript);(out/'pty.json').write_text(json.dumps(evidence,ensure_ascii=False,indent=2));print(json.dumps({'status':'PASS','checks':checks,'candidate_sha256':candidate_sha256},ensure_ascii=False))
finally:
 (out/'pty.txt').write_bytes(transcript)
 for pid in children:
  try:os.kill(pid,signal.SIGTERM);os.waitpid(pid,0)
  except ProcessLookupError:pass
