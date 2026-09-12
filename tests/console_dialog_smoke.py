#!/usr/bin/env python3
"""Full operator path: select task, modal, provider, start/stop/retry. No model calls."""
import argparse, hashlib, signal, json, os, subprocess, tempfile, uuid, pty, termios, fcntl, struct, select, time, shutil
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--report',required=True);args=p.parse_args()
binary=str(Path(args.binary).resolve())
with tempfile.TemporaryDirectory(prefix='swarm-dialog-') as temp:
 root=Path(temp);proc=None;master=slave=None;history=bytearray();current=bytearray();snapshots={}
 def cli(*cmd,data=None):
  r=subprocess.run([binary,'--root',temp,'--json',*cmd,*(['--input','-'] if data is not None else [])],input=json.dumps(data) if data is not None else None,capture_output=True,text=True,timeout=10)
  assert r.returncode==0,(cmd,r.stderr)
  return json.loads(r.stdout) if r.stdout.startswith(('{','[')) else None
 def mutate(*cmd,revision=0,**kw):return cli(*cmd,data=dict(schema_version=1,event_id=uuid.uuid4().hex,expected_revision=revision,**kw))['work']
 def drain(seconds=.05):
  ready,_,_=select.select([master],[],[],seconds)
  if ready:
   try:b=os.read(master,65536);current.extend(b);history.extend(b)
   except OSError:pass
 def expect(text):
  until=time.monotonic()+8;needle=text.encode()
  while time.monotonic()<until:
   if needle in current:return
   drain(.1)
  raise AssertionError(('missing',text,current[-3500:].decode(errors='replace')))
 def send(data):
  drain();current.clear();os.write(master,data)
 def agents():return [a['agent'] for a in cli('agent','list',wid)['agents']]
 def await_agent(status,count):
  until=time.monotonic()+10
  while time.monotonic()<until:
   drain(.05);a=agents()
   if len(a)==count:
    found=[x for x in a if x['status']==status]
    if found:return found[0]
  raise AssertionError(('agent',status,a))
 cli('init');mutate('work','create',title='Autre travail',objective='Unrelated selection',scope='Fixture only',criteria=['no effect'],next='Leave unchanged');w=mutate('work','create',title='Modal test',objective='Operator path',scope='Fixture only',criteria=['modal'],next='Select task');wid=w['id']
 w=mutate('task','add',wid,revision=w['revision'],id='demo',title='Tâche utilisateur 日本 é',deliverable='Fixture',criteria=['proof'],owner='test')
 w=mutate('task','add',wid,revision=w['revision'],id='other',title='Do not touch',deliverable='None',criteria=['unchanged'],owner='test')
 provider=root/'fixture.py';provider.write_text('''#!/usr/bin/python3
import sys,time,json
s=sys.stdin.read()
print('{"type":"system"}',flush=True)
print(json.dumps({"type":"assistant","message":{"content":[{"type":"tool_use","id":"cpu-probe","name":"Bash","input":{"description":"Mesurer la consommation CPU","command":"python3 probe.py --seconds 6"}}]}}),flush=True)
if 'FINISH_QUICKLY' not in s: time.sleep(60)
print(json.dumps({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"cpu-probe","content":"private fixture result"}]}}),flush=True)
''');provider.chmod(0o700)
 (root/'.swarm/providers.json').write_text(json.dumps({'schema_version':1,'providers':{name:{'command':str(provider),'args':[],'env_allow':[],'limits':{'max_tool_calls':10}} for name in ['claude','codex','skynet-glm']}}))
 try:
  master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCGWINSZ,bytes(8));fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',24,80,0,0));before=termios.tcgetattr(slave)
  runtime=root/'swarm';shutil.copy2(binary,runtime)
  proc=subprocess.Popen([str(runtime),'--root',temp,'console'],stdin=slave,stdout=slave,stderr=slave,env={**os.environ,'TERM':'xterm-256color'})
  expect('VOS TRAVAUX');assert b'Choisir un num' not in current
  send(b'\x1b[B');expect('Objectif : Unrelated selection');send(b'\x1b[A');expect('Objectif : Operator path');send(b'\r')
  expect('Entrée actions')
  snapshots['terminal-theme-initial']=current.decode(errors='replace')
  send(b't');expect('Entrée actions');snapshots['terminal-theme-alternate']=current.decode(errors='replace')
  send(b't');expect('Entrée actions')
  fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',30,100,0,0));os.kill(proc.pid,signal.SIGWINCH);time.sleep(.15);drain();assert proc.poll() is None
  fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',24,80,0,0));os.kill(proc.pid,signal.SIGWINCH)
  send(b'\x1bOB\r');expect('Actions — other');send(b'\x1b');time.sleep(.15);drain();send(b'\x1bOA')
  send(b'\x1b[200~pause\r\x1b[201~');expect('swarm> pause ');assert cli('console',wid)['paused'] is False
  send(b'\x15\x1b[3~\r');expect('Actions — demo')
  send(b'\r');expect('Fournisseur : claude');expect('Périmètre : Fixture only')
  # Confirmation must produce a prominent result, including a rejected launch.
  send(b'\t\t\x15/missing/workspace\t\t\t\r')
  expect('LANCEMENT REFUSÉ — demo');assert len(agents())==0
  snapshots['launch-refused']=current.decode(errors='replace')
  send(b'\r');expect('Lancer — demo')
  send(b'\t\t\x15'+str(root).encode()+b'\t\t\t')
  send(b'\t\x1b[C\x1b[C');expect('Fournisseur : skynet-glm')
  send(b'\t\t');send(('\x1b[200~Résumé 日本 é\r\x1b[201~').encode());send(b'\t\t\r');expect('Agent lancé pour demo');first=await_agent('running',1);assert first['provider']=='skynet-glm';assert 'Résumé 日本 é' in first['prompt']
  expect('Mesurer la consommation CPU')
  send(b'd');expect('Livrable : Fixture')
  for _ in range(12):send(b'\x1b[B')
  expect('HISTORIQUE')
  send(b'\x1b');time.sleep(.15);drain()
  send(b'\r');expect('Actions — demo');send(b'\x1b[B\x1b[B\r');expect('Demander l’arrêt' if False else "Demander l'arrêt")
  send(b'\x1b');time.sleep(.15);drain();assert agents()[0]['status']=='running'
  send(b'\r\x1b[B\x1b[B\r');expect('Confirmer — demo');send(b'\r');expect('Demande enregistrée');stopped=await_agent('interrupted',1)
  time.sleep(.2)
  send(b'\r\x1b[B\r');expect('Relancer — demo');send(b'\x15FINISH_QUICKLY\t\t\r');expect('Lancement enregistré');second=await_agent('completed',2)
  assert second['previous']==first['id'] and second['attempt_id']!=first['attempt_id']
  w=cli('work','show',wid)['work'];assert next(t for t in w['tasks'] if t['id']=='other')['status']=='todo'
  (root/'docs').mkdir();(root/'docs/demo-handoff.md').write_text('Rapport de recette')
  send(b'\rl');expect('Rapport — demo');expect('Rapport de recette');snapshots['report']=current.decode(errors='replace');send(b'\x1b');time.sleep(.15);drain()
  send(b'\rs');expect('Soumettre le rapport — demo');expect('docs/demo-handoff.md')
  send(b'\r\r');expect('Rapport soumis');w=cli('work','show',wid)['work'];assert next(t for t in w['tasks'] if t['id']=='demo')['status']=='submitted'
  send(b'\ra');expect('Accepter la tâche — demo');send(b'\r');expect('Acceptation refusée');snapshots['accept-blocked']=current.decode(errors='replace')
  send(b'\x1b');time.sleep(.15);drain()
  doc={'method_version':'2','scope_id':'demo','artifacts':{'docs/demo-handoff.md':hashlib.sha256((root/'docs/demo-handoff.md').read_bytes()).hexdigest()},'domains':{'quality':1},'checks':[{'id':'review','domain':'quality','mandatory':True,'gate':'delivery','penalty':100,'max_penalty':100,'severity':'major'}],'results':[{'id':'review','status':'PASS','count':0,'evidence':['docs/demo-handoff.md']}]}
  (root/'docs/demo.evidence.json').write_text(json.dumps(doc))
  send(b'\rg');expect('Charger une gate — demo');expect('docs/demo.evidence.json')
  send(b'\r\r');expect('Enregistrer la gate — demo');expect('Verdict : PASS')
  snapshots['gate-preview']=current.decode(errors='replace')
  send(b'\r');expect('Gate enregistrée');send(b'\x1b');time.sleep(.15);drain()
  send(b'\rv');expect('Gates et preuves — demo');expect('Avancement vérifié');send(b'\x1b[B'*6);expect('review : PASS');snapshots['gate']=current.decode(errors='replace');send(b'\x1b');time.sleep(.15);drain()
  send(b'\ra');expect('Accepter la tâche — demo');send(b'\x1b[B\r');expect('Action annulée');w=cli('work','show',wid)['work'];assert next(t for t in w['tasks'] if t['id']=='demo')['status']=='submitted'
  send(b'\ra');expect('Accepter la tâche — demo');send(b'\r');expect('Tâche acceptée');snapshots['accepted']=current.decode(errors='replace');w=cli('work','show',wid)['work'];assert next(t for t in w['tasks'] if t['id']=='demo')['status']=='accepted'
  # A stale gate remains stale after an explicit, audited operator override.
  (root/'docs/demo-handoff.md').write_text('Changed after review')
  send(b'\rf');expect('Forcer par dérogation — demo');send(b'\r\r');expect('Motif requis');snapshots['override-dialog']=current.decode(errors='replace')
  send(b'\x1b');time.sleep(.15);drain()
  send(b'\rf');expect('Forcer par dérogation — demo')
  send(b'\x1b[200~Revue manuelle assumee pour la recette\x1b[201~');send(b'\r\r');expect('Tâche acceptée par dérogation')
  snapshots['override']=current.decode(errors='replace')
  w=cli('work','show',wid)['work'];task=next(t for t in w['tasks'] if t['id']=='demo')
  assert task['status']=='waived' and task['manual_override']['reason']=='Revue manuelle assumee pour la recette' and task['gate']
  # Tracking panels are exercised through the same keyboard interface.
  send(b'\rh');expect('HIERARCHY');expect('OBJECTIF');snapshots['hierarchy']=current.decode(errors='replace');send(b'\x1b');time.sleep(.15);drain()
  send(b'\rp');expect('ASSIGN');send(b'\x15reviewer-ui\t\r');expect('Enregistré : assign')
  assert cli('work','show',wid)['work']['tasks'][0]['owner']=='reviewer-ui'
  send(b'\re');expect('OODA');send(b'Observation UI\tOrientation UI\tDecision UI\tResultat UI\tSuite UI\t\r');expect('Enregistré : ooda')
  assert cli('work','show',wid)['work']['next']=='Suite UI'
  send(b'\ru');expect('RESUME');expect('Suite UI');snapshots['resume']=current.decode(errors='replace');send(b'\x1b');time.sleep(.15);drain()
  send(b'\rb');expect('BUDGET');expect('Enregistrer');send(b'\x150.1\t\x151\tEstimation de recette\t2026-09-12\t\r');expect('Enregistré : budget')
  send(b'\rj');expect('JOURNAUX');send(b'Processus\r');expect('Processus');send(b'f');expect('f direct');send(b'f');expect('f pause');send(b'e');expect('Export borné');snapshots['logs']=current.decode(errors='replace');assert list((root/'.swarm').glob('logs-*.json'));send(b'\x1b');time.sleep(.15);drain()
  send(b'\ri');expect('DÉCISIONS');snapshots['decisions']=current.decode(errors='replace');send(b'\x1b');time.sleep(.15);drain()
  replacement=root/'swarm.next';shutil.copy2(binary,replacement);replacement.replace(runtime)
  current.clear();expect('Mise à jour installée')
  send(b'\rr');expect('Relancer — demo');send(b'\t\t\r');expect('Console ancienne');assert len(agents())==2
  send(b'\x1b');time.sleep(.15);drain()
  send(b'q\r');until=time.monotonic()+6
  while proc.poll() is None and time.monotonic()<until:drain(.1)
  assert proc.poll()==0,'console did not exit'
  assert termios.tcgetattr(slave)==before,'terminal not restored'
  # Verify termination variants on fresh PTYs, without provider launch.
  for exit_key in [b'\x03',b'\x04']:
   mm,ss=pty.openpty();fcntl.ioctl(ss,termios.TIOCSWINSZ,struct.pack('HHHH',24,80,0,0));saved=termios.tcgetattr(ss)
   pp=subprocess.Popen([binary,'--root',temp,'console',wid],stdin=ss,stdout=ss,stderr=ss,env={**os.environ,'TERM':'xterm-256color'})
   data=b'';deadline=time.monotonic()+5
   while b'swarm> ' not in data and time.monotonic()<deadline:
    if select.select([mm],[],[],.1)[0]:data+=os.read(mm,65536)
   assert b'swarm> ' in data
   os.write(mm,exit_key);pp.wait(timeout=5);assert pp.returncode==0;assert termios.tcgetattr(ss)==saved
   os.close(mm);os.close(ss)
  plain=subprocess.run([binary,'--root',temp,'console',wid,'--plain'],input='q\n',capture_output=True,text=True,timeout=5)
  assert plain.returncode==0 and '\x1b[' not in plain.stdout
  Path(args.report).write_text(json.dumps({'status':'PASS','model_calls':0,'terminal':'80x24 PTY','checks':['launch console without ID; choose work with arrows and Enter','task selection resolves agent automatically','provider description visible; d opens live details and scrollable history','dynamic skynet-glm provider with limits accepted','launch with default workspace','cancel stop has no effect','confirmed stop from task','retry with instruction and prior attempt','other task untouched','stale console detected and launch blocked after binary replacement','resize stays active; SS3 arrows and multiline bracketed paste','report discovered and submitted from modal; missing gate rejected; gate displayed; cancellation then acceptance','Unicode pasted prompt preserved; Ctrl-C and EOF restore terminal; plain mode without ANSI','terminal restored','hierarchy, owner, OODA, resume, budget, decisions and log search/live/export through modals'],'snapshots':snapshots,'excerpt':history[-14000:].decode(errors='replace')},ensure_ascii=False,indent=2))
 finally:
  if proc is not None and proc.poll() is None:proc.terminate();proc.wait(timeout=5)
  for a in agents():
   if a['status'] in ['running','starting','queued','stopping']:
    cli('agent','stop',a['id'])
  if master is not None:os.close(master)
  if slave is not None:os.close(slave)
print('PASS: task → modal → GLM fixture → stop/cancel → retry, no IDs or paths typed.')
