"""Ordinary review data consistency; supplied Work is fixture data, never approval."""
import copy,datetime,hashlib,json,os,subprocess,tempfile,time
from pathlib import Path

def qualify(root,output,native,probe,reference,contract_reference):
 if output.exists():raise RuntimeError('Fresh review evidence directory required')
 output.mkdir(parents=True);fixture=Path(tempfile.mkdtemp(prefix='.swarm-review-evidence-',dir=Path.home()));fixture.chmod(0o700);commands=[];checks=[]
 def save(name,value):
  path=output/name;fd,tmp=tempfile.mkstemp(dir=path.parent,prefix='.'+path.name+'.')
  with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
  os.replace(tmp,path)
 def run(label,args,data=None,expected=0):
  start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic();p=subprocess.run(list(map(str,args)),cwd=root,input=data,capture_output=True)
  for suffix,raw in [('stdout',p.stdout),('stderr',p.stderr)]: (output/(label+'.'+suffix)).write_bytes(raw)
  if data is not None:(output/(label+'.stdin')).write_bytes(data)
  commands.append(dict(label=label,args=list(map(str,args)),exit=p.returncode,expected_exit=expected,start_utc=start,seconds=time.monotonic()-clock));save('commands.json',commands)
  if p.returncode!=expected:raise RuntimeError(label+': original failure retained')
  return p.stdout
 run('init',[native,'--root',fixture,'--json','init']);(fixture/'.rust-migration-fixture').write_text('Owned supplied-Work evidence fixture.\n')
 files={'report.md':b'Review report','delivery.md':b'Delivery','instructions.md':b'Ordinary instructions'}
 for name,raw in files.items():(fixture/name).write_bytes(raw)
 task=dict(id='task',title='Task',deliverable='delivery.md',criteria=['Proof'],attempts=[dict(id='attempt',status='completed')])
 contract=json.loads(run('contract-control',[contract_reference],json.dumps(task).encode()))['contract']
 task['independent_review']=dict(id='review',attempt='attempt',contract=contract,state='passed',report='report.md',sha256=hashlib.sha256(files['report.md']).hexdigest(),report_artifacts={'delivery.md':hashlib.sha256(files['delivery.md']).hexdigest()})
 work=dict(schema_version=1,id='fixture-work',revision=1,planning=dict(reviewer=dict(provider='fixture-not-called')),tasks=[task])
 def compare(label,value=work,expected=0,configured=True):
  raw=json.dumps(value,separators=(',',':')).encode();settings=(fixture/'.swarm/engine-settings.json').read_bytes();db=(fixture/'.swarm/state.db').read_bytes();source_files={p:p.read_bytes()for p in fixture.iterdir()if p.is_file()}
  frame=b'{"Root":'+json.dumps(str(fixture)).encode()+b',"Review":true,"TaskID":"task","Work":'+raw+b'}'
  g=run(label+'-go',[reference],frame,expected);n=run(label+'-native',[probe,fixture,'task'],raw,expected)
  if expected==0:assert json.loads(g)==json.loads(n)==dict(consistent=True,configured=configured)
  else:assert not g and not n
  assert (fixture/'.swarm/engine-settings.json').read_bytes()==settings and (fixture/'.swarm/state.db').read_bytes()==db
  assert all(p.read_bytes()==body for p,body in source_files.items());checks.append(label)
 compare('valid-control')
 faults=[]
 for name in ['reviewer','review','state','no-attempt','new-attempt','criteria','brief','memory','report-digest','artifact-digest','managed']:
  value=copy.deepcopy(work);t=value['tasks'][0];r=t['independent_review']
  if name=='reviewer':value['planning']['reviewer']=None
  elif name=='review':t['independent_review']=None
  elif name=='state':r['state']='running'
  elif name=='no-attempt':t['attempts']=[]
  elif name=='new-attempt':t['attempts'].append(dict(id='next-attempt'))
  elif name=='criteria':t['criteria']=['Changed']
  elif name=='brief':t['plan_brief_hash']='new'
  elif name=='memory':t['memory_selections']=[dict(work='w',id='memory',version=1,sha256='a'*64)]
  elif name=='report-digest':r['sha256']='0'*64
  elif name=='artifact-digest':r['report_artifacts']['delivery.md']='0'*64
  elif name=='managed':value['planning']['repository']={}
  faults.append((name,value))
 for name,value in faults:compare(name+'-control');compare(name,value,2);compare(name+'-recovery')
 for name in ['report.md','delivery.md']:
  file=fixture/name;label=name.split('.')[0];compare(label+'-file-control');file.write_bytes(b'Changed');compare(label+'-changed',expected=2);file.write_bytes(files[name]);compare(label+'-restored');file.unlink();compare(label+'-missing',expected=2);file.write_bytes(files[name]);compare(label+'-missing-recovery');file.unlink();file.mkdir();compare(label+'-directory',expected=2);file.rmdir();file.write_bytes(files[name]);compare(label+'-directory-recovery')
  if hasattr(os,'mkfifo'):
   compare(label+'-fifo-control');file.unlink();os.mkfifo(file);compare(label+'-fifo',expected=2);file.unlink();file.write_bytes(files[name]);compare(label+'-fifo-recovery')
 outside=fixture.parent/(fixture.name+'-outside.md');outside.write_bytes(b'Owned outside-file control');value=copy.deepcopy(work);value['tasks'][0]['independent_review']['report']=str(fixture/'report.md');compare('absolute-report-control');compare('absolute-report',value,2);compare('absolute-report-recovery');value['tasks'][0]['independent_review']['report']='../'+outside.name;value['tasks'][0]['independent_review']['sha256']=hashlib.sha256(outside.read_bytes()).hexdigest();compare('outside-report-control');compare('outside-report',value,2);compare('outside-report-recovery')
 manual=copy.deepcopy(work);manual['planning']=None;compare('manual-unconfigured',manual,configured=False)
 compare('before-new-context-control')
 profile=dict(version=1,name='Review project',instructions=[dict(path='instructions.md',roles=['reviewer'])]);profile_file=fixture/'swarm.project.json';profile_file.write_text(json.dumps(profile))
 compare('new-context-without-receipt',expected=2);ctx=json.loads(run('current-context',[native,'project-context','reviewer','--root',fixture,'--json']))['context'];current=copy.deepcopy(work);current['tasks'][0]['independent_review']['workflow']=dict(project=ctx);compare('current-context-valid',current)
 source=fixture/'instructions.md';source.write_bytes(b'Changed instructions');compare('source-context-changed',current,2);source.write_bytes(files['instructions.md']);compare('source-context-recovery',current);original=profile_file.read_bytes();profile_file.write_bytes(original+b'\n');compare('profile-context-changed',current,2);profile_file.write_bytes(original);compare('profile-context-recovery',current)
 doc=json.loads(run('settings-current',[native,'engine-settings','show','--root',fixture,'--json']));request=output/'change.json'
 for label,cap,expected in [('small','1',2),('restored',None,0)]:
  change=dict(schema_version=doc['schema_version'],event_id='review-context-'+label,expected_revision=doc['revision'],values=copy.deepcopy(doc['values']),reason='Actual configured review context observation');change['values']['project_context_max_bytes']=cap;request.write_text(json.dumps(change));before=(fixture/'.swarm/engine-settings.json').read_bytes();preview=json.loads(run(label+'-preview',[native,'engine-settings','preview','--root',fixture,'--input',request,'--json']));assert (fixture/'.swarm/engine-settings.json').read_bytes()==before;change['preview_token']=preview['preview_token'];request.write_text(json.dumps(change));doc=json.loads(run(label+'-apply',[native,'engine-settings','apply','--root',fixture,'--input',request,'--json']))['document'];run(label+'-actual-policy',[probe,fixture,'task'],json.dumps(current).encode(),expected)
 compare('configured-policy-restored',current)
 result=dict(status='PASS',scope='ordinary current review consistency on supplied fixture Work; not Root-row authority, independent review or managed review',checks=checks,commands=len(commands),fixture=str(fixture),outside_fixture=str(outside),managed_branch_refuses_without_port=True,actual_saved_context_policy_consumed=True,provider_calls=False,global_watchdog=None)
 save('result.json',result);return result
