"""Actual Go CLI/native Root gate recording and cross-runtime replay on one fixture.
Only historical method 2 is comparable to Go. Native method 3 is checked separately.
"""
import copy,hashlib,json,subprocess,tempfile
from pathlib import Path

def qualify(root,output,native,probe,go_cli,root_probe=None):
 if output.exists():raise RuntimeError('Fresh gate store evidence directory required')
 output.mkdir(parents=True);fixture=Path(tempfile.mkdtemp(prefix='.swarm-gate-store-',dir=Path.home()));commands=[]
 def call(label,args,data=None,expected=0):
  p=subprocess.run(list(map(str,args)),input=data,capture_output=True,cwd=root)
  for suffix,raw in [('stdout',p.stdout),('stderr',p.stderr)]: (output/(label+'.'+suffix)).write_bytes(raw)
  if data is not None:(output/(label+'.stdin')).write_bytes(data)
  commands.append(dict(label=label,exit=p.returncode,expected_exit=expected));assert p.returncode==expected,(label,p.returncode,p.stderr)
  return json.loads(p.stdout)if p.stdout else None
 def cli(label,parts,request=None,expected=0):
  args=[go_cli,'--root',fixture,'--json',*parts]
  if request is not None:
   file=output/(label+'.input.json');file.write_bytes(request if isinstance(request,bytes)else json.dumps(request).encode());args+=['--input',file]
  return call(label,args,expected=expected)
 def encode_request(request):return json.dumps(request,ensure_ascii=False,separators=(',',':')).encode().replace(b'"metadata":[1.0,0]',b'"metadata":[1.0,-0,1e+09,1e999]')
 def record(label,request):return call(label,[probe,fixture,wid],request if isinstance(request,bytes)else encode_request(request))
 call('native-init',[native,'--root',fixture,'--json','init']);(fixture/'.rust-migration-fixture').write_text('Owned raw native gate fixture.\n');(fixture/'proof.txt').write_bytes(b'proof')
 w=cli('work-create',['work','create'],dict(schema_version=1,event_id='create',expected_revision=0,title='Gate',objective='Cross replay',scope='Fixture',criteria=['Proof']))['work'];wid=w['id']
 w=cli('task-add',['task','add',wid],dict(schema_version=1,event_id='add',expected_revision=w['revision'],id='task',title='Task',deliverable='Report',criteria=['Initial']))['work']
 w=cli('task-criteria',['task','update',wid],dict(schema_version=1,event_id='criteria',expected_revision=w['revision'],id='task',criteria=['Amended']))['work']
 checks=[dict(id=k,gate=v,mandatory=True,severity='minor',penalty=1,max_penalty=100)for k,v in w['tasks'][0]['plan_checks'].items()]
 checks.append(dict(id='quality',domain='quality',mandatory=False,gate='delivery',severity='minor',penalty=20,max_penalty=100,metadata=[1.0,0]))
 doc=dict(method_version='2',scope_id='task',domains={'quality':1},artifacts={'proof.txt':hashlib.sha256(b'proof').hexdigest()},checks=checks,results=[dict(id=c['id'],status='PASS',count=0,evidence=['proof.txt'])for c in checks])
 req=dict(schema_version=1,event_id='native-gate',expected_revision=w['revision'],task_id='task',phase='delivery',name='Gate <native>\u2028',document=doc)
 # Opaque numeric lexemes must survive normalization and event hashing unchanged.
 raw=encode_request(req)
 assert b'1e999'in raw
 first=record('native-record',raw);assert first['code']=='ok'and not first['replayed'];assert first['work']['revision']==w['revision']+1
 read=cli('go-read-native',['work','show',wid]);assert read['work']==first['work'];before_events=len(read['events'])
 replay=cli('go-replay-native',['gate',wid,'--task','task'],raw);assert replay['work']==first['work']and len(replay['events'])==before_events
 assert record('native-replay',raw)['replayed']
 # Go writes another receipt; Rust must replay it without observations/mutation.
 go_raw=raw.replace(b'"event_id":"native-gate"',b'"event_id":"go-gate"').replace(b'"expected_revision":'+str(w['revision']).encode(),b'"expected_revision":'+str(first['work']['revision']).encode())
 go_saved=cli('go-record',['gate',wid,'--task','task'],go_raw);replayed=record('native-replay-go',go_raw);assert replayed['code']=='ok'and replayed['replayed']and replayed['work']==go_saved['work']
 observed=call('native-inspect',[probe,fixture,wid,'task']);assert observed['evaluation']['ship_allowed']and observed['work_revision']==go_saved['work']['revision']
 stable=cli('before-refusals',['work','show',wid]);cases=[]
 for defect in ['required-alias','unknown-policy','missing-task','changed-intent']:
  control=copy.deepcopy(req);control['event_id']=defect+'-control';control['expected_revision']=stable['work']['revision']
  assert record(defect+'-control',control)['code']=='ok';stable=cli(defect+'-baseline',['work','show',wid])
  bad=copy.deepcopy(control);bad['event_id']=defect;bad['expected_revision']=stable['work']['revision']
  if defect=='required-alias':bad['document']['CHECKS']=copy.deepcopy(bad['document']['checks']);bad['document']['checks'][0]['mandatory']=False
  if defect=='unknown-policy':bad['policy']={'generation':0}
  if defect=='missing-task':bad['task_id']='missing'
  if defect=='changed-intent':bad=go_raw.replace(b'"name":'+json.dumps(req['name'],ensure_ascii=False).encode(),b'"name":"different"');assert bad!=go_raw and b'1e999'in bad
  result=record(defect,bad);assert result['code']=={'required-alias':'gate-evidence-invalid','unknown-policy':'format','missing-task':'gate-evidence-invalid','changed-intent':'event_conflict'}[defect]
  after=cli(defect+'-read',['work','show',wid]);assert after['work']==stable['work']and after['events']==stable['events']
  cases.append(dict(defect=defect,code=result['code'],no_mutation=True))
 # Real read checks stale artifacts, and recovery rereads the original bytes.
 (fixture/'proof.txt').write_bytes(b'changed');stale=call('stale-artifact',[probe,fixture,wid,'task']);assert stale['code']=='gate-evidence-invalid';(fixture/'proof.txt').write_bytes(b'proof');assert call('artifact-recovery',[probe,fixture,wid,'task'])['evaluation']['ship_allowed']
 if root_probe is not None:
  def mutate(label,request):return call(label,[root_probe,fixture],json.dumps(dict(work=wid,kind='task.update',request=request)).encode()+b'\n')
  current=cli('lifecycle-current',['work','show',wid])['work']
  invalid=dict(schema_version=1,event_id='invalid-submission',expected_revision=current['revision'],id='task',status='submitted',outcome='completed')
  assert mutate('lifecycle-invalid-transition',invalid)['code']=='invalid_transition'
  running=dict(schema_version=1,event_id='lifecycle-running',expected_revision=current['revision'],id='task',status='running')
  started=mutate('lifecycle-native-running',running);assert started['code']=='ok'and len(started['work']['tasks'][0]['attempts'])==1
  assert cli('lifecycle-go-replay-running',['task','update',wid],running)['work']==started['work']
  invalid=dict(schema_version=1,event_id='invalid-outcome',expected_revision=started['work']['revision'],id='task',status='submitted',outcome='failed')
  assert mutate('lifecycle-invalid-outcome',invalid)['code']=='format'
  submitted=dict(invalid,event_id='lifecycle-submitted',outcome='completed')
  submitted_work=cli('lifecycle-go-submitted',['task','update',wid],submitted)['work'];assert submitted_work['tasks'][0]['attempts'][0]['ended']
  assert mutate('lifecycle-native-replay-submitted',submitted)['replayed']
  receipt=copy.deepcopy(req);receipt['event_id']='lifecycle-gate';receipt['expected_revision']=submitted_work['revision']
  receipt_raw=encode_request(receipt);go_gate=cli('lifecycle-go-gate',['gate',wid,'--task','task'],receipt_raw)['work']
  accepted=dict(schema_version=1,event_id='lifecycle-accepted',expected_revision=go_gate['revision'],id='task',status='accepted')
  accepted_work=mutate('lifecycle-native-accepted',accepted);assert accepted_work['code']=='ok'and accepted_work['work']['tasks'][0]['status']=='accepted'
  assert cli('lifecycle-go-replay-accepted',['task','update',wid],accepted)['work']==accepted_work['work']
  assert mutate('lifecycle-native-replay-accepted',accepted)['replayed']
 summary=dict(status='PASS',scope='native Root gate plus actual Go CLI and cross-runtime replay; fixture transport, not public native API',fixture=str(fixture),cross_replay_both_directions=True,manual_lifecycle=root_probe is not None,opaque_numbers_preserved=True,artifacts_reread=True,cases=cases,commands=commands,global_watchdog=None,acceptance=False)
 (output/'result.json').write_text(json.dumps(summary,indent=2)+'\n');return summary
