#!/usr/bin/env python3
"""Real PTY attachment, detach and reattach; no model calls."""
import fcntl,json,os,pathlib,pty,select,struct,subprocess,sys,tempfile,termios,time,uuid
binary=str(pathlib.Path(sys.argv[1]).resolve());out=pathlib.Path(sys.argv[2]);out.mkdir(parents=True,exist_ok=True)
root=tempfile.mkdtemp(prefix='swarm-dialogue-')
def cli(*args,payload=None):
 argv=[binary,'--root',root,'--json',*args]
 if payload is not None:argv+=['--input','-']
 return json.loads(subprocess.check_output(argv,input=json.dumps(payload).encode() if payload is not None else None))
def mutate(args,rev,**fields):return cli(*args,payload=dict(schema_version=1,event_id=str(uuid.uuid4()),expected_revision=rev,**fields))['work']
cli('init');provider=pathlib.Path(root)/'codex';provider.write_text("""#!/usr/bin/python3
import sys,json,time
question=sys.stdin.read()
with open('provider-calls','a') as f:f.write('call\\n')
time.sleep(.3)
if 'resume' in sys.argv:assert sys.argv[sys.argv.index('resume')+1]=='test-session'
print(json.dumps({'type':'thread.started','thread_id':'test-session'}))
print(json.dumps({'type':'item.started','item':{'id':'item_0','type':'command_execution','command':question[:40]}}))
print(json.dumps({'type':'item.completed','item':{'id':'item_0','type':'command_execution','status':'completed'}}))
print(json.dumps({'type':'item.completed','item':{'id':'message','type':'agent_message','text':'REPONSE_CONTROLEE'}}))
print(json.dumps({'type':'turn.completed','usage':{'input_tokens':10,'output_tokens':5}}))
""");provider.chmod(0o700)
(pathlib.Path(root)/'.swarm/providers.json').write_text(json.dumps(dict(schema_version=1,providers=dict(fixture=dict(command=str(provider),args=['exec','--json','--sandbox','workspace-write','-'])))))
w=mutate(['work','create'],0,title='PTY test',objective='Attach',scope='fixture',criteria=['reply'],next='attach')
w=mutate(['task','add',w['id']],w['revision'],id='t1',title='TTY',deliverable='reply',criteria=['reply'],next='attach');cli('autonomy',w['id'],'manuel');w=cli('work','show',w['id'])['work']
a=cli('agent','start',w['id'],payload=dict(schema_version=1,event_id=str(uuid.uuid4()),expected_revision=w['revision'],task_id='t1',provider='fixture',workspace=root,mode='dialogue',timeout_seconds=40,limits=dict(max_tool_calls=2)))
a=a.get('agent',a);agent=a['id'];checks=[]
def attach():
 master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',24,80,0,0));p=subprocess.Popen([binary,'--root',root,'agent','attach',agent],stdin=slave,stdout=slave,stderr=slave);os.close(slave);return p,master
def read_until(fd,needle):
 data=b'';deadline=time.time()+8
 while time.time()<deadline:
  if select.select([fd],[],[],.1)[0]:
   try:data+=os.read(fd,65536)
   except OSError:break
  if needle in data:return data
 raise AssertionError((needle,data[-1500:]))
try:
 for _ in range(100):
  if cli('agent','show',agent)['agent']['status']=='running':break
  time.sleep(.05)
 p,fd=attach();read_until(fd,b'Vous >');first=cli('agent','show',agent)['agent'];assert first['status']=='running';time.sleep(1.2);first=cli('agent','show',agent)['agent'];assert first['progress']['tool_calls']==1;assert first['usage']['input_tokens']==10;checks.append('Premier échange terminé sans libérer la tentative ; outils et jetons reçus')
 os.write(fd,b'\x1d');p.wait(timeout=5);os.close(fd);p,fd=attach();read_until(fd,b'Vous >');os.write(fd,b'Second echange\nTroisieme interdit\nQuatrieme interdit\n');p.wait(timeout=8);os.close(fd)
 final=cli('agent','show',agent)['agent'];assert final['status']=='interrupted',final;assert final['progress']['tool_calls']==2,final;assert (pathlib.Path(root)/'provider-calls').read_text().count('call')==2; assert final['usage']['input_tokens']==20,final;checks.append('Reprise de la même conversation ; item_0 recompte au second échange ; arrêt sur plafond cumulé de deux outils')
 (out/'pty.json').write_text(json.dumps(dict(status='PASS',checks=checks),indent=2)+'\n')
finally:
 if cli('agent','show',agent)['agent']['status'] in ['running','starting','queued']:cli('agent','stop',agent)
