"""Controlled malformed-row injection on a private fixture; never a live database.
The malformed state is not claimed reachable via ordinary public operations.
"""
import hashlib,json,sqlite3,subprocess,tempfile
from pathlib import Path

def qualify(root,output,binary,injector,expect_panic=False):
 if output.exists():raise RuntimeError('Fresh attempt-integrity evidence directory required')
 output.mkdir(parents=True);fixture=Path(tempfile.mkdtemp(prefix='.swarm-attempt-integrity-',dir=Path.home()));commands=[]
 def call(label,parts,request=None,expected=0):
  args=[str(binary),'--root',str(fixture),'--json','--lang','fr',*parts]
  if request is not None:
   file=output/(label+'.input.json');file.write_text(json.dumps(request));args+=['--input',str(file)]
  p=subprocess.run(args,capture_output=True,cwd=root)
  for suffix,raw in [('stdout',p.stdout),('stderr',p.stderr)]: (output/(label+'.'+suffix)).write_bytes(raw)
  commands.append(dict(label=label,exit=p.returncode,expected=expected));assert p.returncode==expected,(label,p.returncode,p.stderr)
  return p
 def apply(label,kind,wid,revision,**fields):
  p=call(label,[*kind,*([wid]if wid else[])],dict(schema_version=1,event_id=label,expected_revision=revision,**fields));return json.loads(p.stdout)['work']
 call('init',['init']);(fixture/'.rust-attempt-integrity-fixture').write_text('Owned attempt-integrity fault injection fixture.\n');w=apply('create',['work','create'],'',0,title='Attempt integrity',objective='Controlled malformed fixture',scope='Private test',criteria=['Proof']);wid=w['id']
 w=apply('add',['task','add'],wid,w['revision'],id='task',title='Task',deliverable='Report',criteria=['Proof'])
 w=apply('control-start',['task','update'],wid,w['revision'],id='task',status='running');assert len(w['tasks'][0]['attempts'])==1
 w=apply('valid-control',['task','update'],wid,w['revision'],id='task',status='blocked',blocker='Known fixture issue',outcome='failed');assert w['tasks'][0]['attempts'][0]['ended']and w['tasks'][0]['status']=='blocked'
 w=apply('prepare-fault',['task','update'],wid,w['revision'],id='task',status='running');assert len(w['tasks'][0]['attempts'])==2
 database=fixture/'.swarm/state.db'
 with sqlite3.connect(database)as db:
  assert db.execute('PRAGMA user_version').fetchone()==(37,)
  original=db.execute('SELECT body FROM works WHERE id=?',(wid,)).fetchone()[0];assert isinstance(original,bytes)
  document=json.loads(original);assert document==w and document['tasks'][0]['status']=='running'
  document['tasks'][0]['attempts']=[];fault=json.dumps(document,ensure_ascii=False,separators=(',',':')).encode()
 def inject(label,body):
  frame=json.dumps(dict(work=wid,body_hex=body.hex())).encode();p=subprocess.run([str(injector),str(fixture)],input=frame,capture_output=True,cwd=root)
  for suffix,data in [('stdin',frame),('stdout',p.stdout),('stderr',p.stderr)]: (output/(label+'.'+suffix)).write_bytes(data)
  assert p.returncode==0,(label,p.stderr)
 inject('fixture-injection',fault)
 (output/'work-before-fault.json').write_bytes(original);(output/'injected-work.json').write_bytes(fault)
 request=dict(schema_version=1,event_id='closure-refusal',expected_revision=w['revision'],id='task',status='blocked',blocker='Known fixture issue',outcome='failed')
 result=call('malformed-close',['task','update',wid],request,2)
 assert not result.stdout
 if expect_panic:assert b'panic:'in result.stderr and b'index out of range'in result.stderr
 else:
  error=json.loads(result.stderr);assert error['schema_version']==1 and error['failure']['code']=='attempt_state_invalid'and 'tentative courante absente'in error['error']
 with sqlite3.connect(database)as db:
  assert db.execute('SELECT body FROM works WHERE id=?',(wid,)).fetchone()[0]==fault
  assert db.execute('SELECT count(*)FROM events WHERE id=?',('closure-refusal',)).fetchone()==(0,)
  # Restore the exact pre-injection fixture bytes, not a business recovery API.
 inject('restore-exact-fixture',original)
 recovered=apply('restored-fixture-recovery',['task','update'],wid,w['revision'],id='task',status='blocked',blocker='Known fixture issue',outcome='failed');assert recovered['tasks'][0]['status']=='blocked'and recovered['tasks'][0]['attempts'][-1]['ended']
 summary=dict(status='REPRODUCED'if expect_panic else'PASS',scope=('public CLI panics on malformed fixture data'if expect_panic else'public CLI refuses malformed fixture data with stable code')+'; fixture restoration is not production recovery',fixture=str(fixture),injected_state='running with empty attempts',ordinary_public_reachability=False,mutation_on_refusal=False,original_body_sha256=hashlib.sha256(original).hexdigest(),fault_body_sha256=hashlib.sha256(fault).hexdigest(),commands=commands,global_watchdog=None,runtime_delivery=False)
 (output/'result.json').write_text(json.dumps(summary,indent=2)+'\n');return summary
