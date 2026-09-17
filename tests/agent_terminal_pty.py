#!/usr/bin/env python3
"""Real PTY attachment, detach and reattach; no model calls."""
import fcntl,json,os,pathlib,pty,select,struct,subprocess,sys,tempfile,termios,time,uuid
binary=str(pathlib.Path(sys.argv[1]).resolve());out=pathlib.Path(sys.argv[2]);out.mkdir(parents=True,exist_ok=True)
root=tempfile.mkdtemp(prefix='swarm-attach-')
def cli(*args,payload=None):
 argv=[binary,'--root',root,'--json',*args]
 if payload is not None:argv+=['--input','-']
 return json.loads(subprocess.check_output(argv,input=json.dumps(payload).encode() if payload is not None else None))
def mutate(args,rev,**fields):return cli(*args,payload=dict(schema_version=1,event_id=str(uuid.uuid4()),expected_revision=rev,**fields))['work']
cli('init');provider=pathlib.Path(root)/'fixture.py';provider.write_text('import sys\nassert sys.stdin.isatty()\nprint("READY",flush=True)\nfor line in sys.stdin:\n if line.strip()=="/exit":break\n print("REPLY:"+line.strip(),flush=True)\n')
(pathlib.Path(root)/'.swarm/providers.json').write_text(json.dumps(dict(schema_version=1,providers=dict(fixture=dict(command='/usr/bin/python3',args=[str(provider)],interactive_args=[str(provider)])))))
w=mutate(['work','create'],0,title='PTY test',objective='Attach',scope='fixture',criteria=['reply'],next='attach')
w=mutate(['task','add',w['id']],w['revision'],id='t1',title='TTY',deliverable='reply',criteria=['reply'],next='attach');cli('autonomy',w['id'],'manuel');w=cli('work','show',w['id'])['work']
a=cli('agent','start',w['id'],payload=dict(schema_version=1,event_id=str(uuid.uuid4()),expected_revision=w['revision'],task_id='t1',provider='fixture',workspace=root,mode='terminal',timeout_seconds=40))
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
 p,fd=attach();read_until(fd,b'READY');os.write(fd,'Bonjour é\n'.encode());read_until(fd,'REPLY:Bonjour é'.encode());checks.append('TTY réel, Unicode et retour fournisseur')
 os.write(fd,b'\x1d');p.wait(timeout=5);assert p.returncode==0;os.close(fd);assert cli('agent','show',agent)['agent']['status']=='running';checks.append('Ctrl+] détache sans arrêter')
 p,fd=attach();read_until(fd,'REPLY:Bonjour é'.encode());os.write(fd,b'/exit\n');p.wait(timeout=8);os.close(fd);assert p.returncode==0;assert cli('agent','show',agent)['agent']['status']=='completed';checks.append('rattachement et fin sur la même tentative')
 (out/'pty.json').write_text(json.dumps(dict(status='PASS',checks=checks),indent=2)+'\n')
finally:
 if cli('agent','show',agent)['agent']['status'] in ['running','starting','queued']:cli('agent','stop',agent)
