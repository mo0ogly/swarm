"""Compare actual Go ordinary-source context with native configured readers.
Managed memory and full reviewer authority are excluded, never declared valid.
"""
import hashlib,json,subprocess,tempfile,copy
from pathlib import Path

def qualify(root,output,native,reference,guard_probe=None):
 if output.exists():raise RuntimeError('Fresh project context evidence required')
 output.mkdir(parents=True);fixture=Path(tempfile.mkdtemp(prefix='.swarm-project-context-',dir=Path.home()));commands=[]
 def run(label,args,data=None):
  p=subprocess.run(list(map(str,args)),input=data,capture_output=True,cwd=root)
  for suffix,raw in [('stdout',p.stdout),('stderr',p.stderr)]: (output/(label+'.'+suffix)).write_bytes(raw)
  if data is not None:(output/(label+'.stdin')).write_bytes(data)
  commands.append(dict(label=label,exit=p.returncode));return p
 p=run('native-init',[native,'--root',fixture,'--json','init']);assert p.returncode==0,p.stderr
 profile=dict(version=1,name='Project <&>\u2028',instructions=[dict(path='guide.md',roles=['preparation','planner','subplanner','worker','reviewer'])]);(fixture/'guide.md').write_text('Ordinary project rules')
 def compare(label,p=profile,role='reviewer',reject=False,raw=None):
  data=raw if raw is not None else json.dumps(p,ensure_ascii=False,separators=(',',':')).encode();file=fixture/'swarm.project.json';file.write_bytes(data)
  g=run(label+'-go',[reference],json.dumps(dict(Root=str(fixture),Role=role)).encode());n=run(label+'-native',[native,'project-context',role,'--root',fixture,'--json']);assert g.returncode==n.returncode,(label,g.stderr,n.stderr)
  if reject:assert n.returncode==2 and not n.stdout
  else:
   assert n.returncode==0,(label,g.stderr,n.stderr);go=json.loads(g.stdout);native_result=json.loads(n.stdout);assert native_result['context']==go['context'],(label,'context/digest mismatch')
   # Intentional truthful clarification: static native text describes recognizable
   # secret detection as a heuristic. Manifest/source bytes remain equivalent.
   assert native_result['prompt'].split('\n',1)[0]==go['prompt'].split('\n',1)[0]
   assert native_result['prompt'].split('\nSOURCE PROJET ',1)[1]==go['prompt'].split('\nSOURCE PROJET ',1)[1]
  assert file.read_bytes()==data
 for role in ['preparation','planner','subplanner','worker','reviewer']:compare('role-'+role,role=role)
 faults=[]
 for field,value in [('version',2),('name',' '),('name','x'*101),('source','unknown')]:p=copy.deepcopy(profile);p[field]=value;faults.append((field+'-'+str(len(faults)),p,None))
 p=copy.deepcopy(profile);p['instructions']=[];faults.append(('empty',p,None))
 p=copy.deepcopy(profile);p['instructions'].append(p['instructions'][0]);faults.append(('duplicate-source',p,None))
 for field,value in [('path','../outside.md'),('path','guide.json'),('roles',['unknown']),('roles',['reviewer','reviewer'])]:p=copy.deepcopy(profile);p['instructions'][0][field]=value;faults.append((field+'-'+str(len(faults)),p,None))
 p=copy.deepcopy(profile);p['instructions'][0]['unknown']='blocked';faults.append(('unknown-nested',p,None))
 for label,p,raw in faults:compare(label+'-control');compare(label,p,reject=True,raw=raw);compare(label+'-recovery')
 for role,cap in [('reviewer',16000),('worker',32000)]:
  overhead=len('\nSOURCE PROJET guide.md\n\n'.encode());(fixture/'guide.md').write_text('x'*(cap-overhead));compare(role+'-exact',role=role);(fixture/'guide.md').write_text('x'*(cap-overhead+1));compare(role+'-overflow',role=role,reject=True);(fixture/'guide.md').write_text('Ordinary project rules');compare(role+'-restored',role=role)
 # Exercise all Go recognizable-secret patterns through actual context loading.
 examples=['sk-'+'a'*20,'ſK-'+'a'*20,'sk-ant-'+'a'*16,'-----BEGIN PRIVATE KEY-----','-----begin rsa private key-----','API_KEY = "'+'a'*20,'auth-token:\t'+'a'*20,'PASSWORD='+'a'*20,'API_KEY='+'a'*20,'API_KEY='+'a'*19,'password\v='+'a'*20,'ordinary rules']
 for i,text in enumerate(examples):
  compare('secret-'+str(i)+'-control');g=run('secret-'+str(i)+'-pattern',[reference],json.dumps(dict(Secret=True,Text=text)).encode());assert g.returncode==0;secret=json.loads(g.stdout)['secret'];(fixture/'guide.md').write_text(text);compare('secret-'+str(i),reject=secret);(fixture/'guide.md').write_text('Ordinary project rules')
 guard_commands=[]
 if guard_probe is not None:
  (fixture/'.rust-migration-fixture').write_text('Owned project context guard fixture.\n')
  compare('guard-baseline')
  def observed(label,role='reviewer'):
   g=run(label+'-go',[reference],json.dumps(dict(Root=str(fixture),Role=role)).encode());n=run(label+'-native',[native,'project-context',role,'--root',fixture,'--json']);assert g.returncode==n.returncode==0
   value=json.loads(g.stdout)['context'];assert value==json.loads(n.stdout)['context'];return value
  def guard(label,previous,expected=0,role='reviewer'):
   files={p:p.read_bytes()for p in [fixture/'swarm.project.json',fixture/'guide.md',fixture/'.swarm/engine-settings.json']if p.exists()}
   g=run(label+'-guard-go',[reference],json.dumps(dict(Root=str(fixture),Role=role,Guard=True,Previous=previous)).encode());n=run(label+'-guard-native',[guard_probe,fixture],json.dumps(dict(role=role,previous=previous)).encode())
   assert g.returncode==n.returncode==expected,(label,g.stderr,n.stdout,n.stderr)
   if expected==0:assert json.loads(g.stdout)==json.loads(n.stdout)==dict(fresh=True)
   assert all(p.read_bytes()==raw for p,raw in files.items());guard_commands.append(label)
  previous=observed('guard-observe');guard('guard-valid',previous)
  guard('guard-missing-old',None,2);guard('guard-missing-old-recovery',previous)
  profile_file=fixture/'swarm.project.json';profile_raw=profile_file.read_bytes();source_file=fixture/'guide.md';source_raw=source_file.read_bytes()
  for label,file,changed in [('source',source_file,b'Changed rules'),('profile',profile_file,profile_raw+b'\n')]:
   guard(label+'-valid-control',previous);file.write_bytes(changed);guard(label+'-changed',previous,2);fresh=observed(label+'-new-observation');guard(label+'-new-observation-valid',fresh);file.write_bytes(source_raw if file==source_file else profile_raw);guard(label+'-restored',previous)
  worker=observed('worker-observe','worker');guard('role-valid-control',worker,role='worker');guard('role-changed',previous,2,role='worker');guard('role-restored',worker,role='worker');guard('reviewer-restored',previous)
  for label,file in [('profile-removed',profile_file),('source-removed',source_file)]:
   guard(label+'-valid-control',previous);saved=file.read_bytes();file.unlink();guard(label,previous,2)
   if file==profile_file:guard('no-profile-no-previous',None)
   file.write_bytes(saved);guard(label+'-recovery',previous)
  guard('invalid-profile-control',previous);profile_file.write_bytes(b'{invalid');guard('invalid-profile',previous,2);profile_file.write_bytes(profile_raw);guard('invalid-profile-recovery',previous)
 # Real persisted operator values change the actual reader without fallback.
 compare('policy-control');shown=run('settings-show',[native,'engine-settings','show','--root',fixture,'--json']);assert shown.returncode==0;document=json.loads(shown.stdout);change=dict(schema_version=document['schema_version'],event_id='context-cap',expected_revision=document['revision'],values=copy.deepcopy(document['values']),reason='Actual context reader operator policy');change['values']['project_context_max_bytes']='1';request=output/'change.json';request.write_text(json.dumps(change))
 before=(fixture/'.swarm/engine-settings.json').read_bytes();p=run('preview',[native,'engine-settings','preview','--root',fixture,'--input',request,'--json']);assert p.returncode==0;assert (fixture/'.swarm/engine-settings.json').read_bytes()==before;change['preview_token']=json.loads(p.stdout)['preview_token'];request.write_text(json.dumps(change));assert run('apply',[native,'engine-settings','apply','--root',fixture,'--input',request,'--json']).returncode==0
 p=run('configured-cap-consumed',[native,'project-context','reviewer','--root',fixture,'--json']);assert p.returncode==2 and not p.stdout
 if guard_probe is not None:
  settings=(fixture/'.swarm/engine-settings.json').read_bytes();p=run('guard-configured-cap-refuses',[guard_probe,fixture],json.dumps(dict(role='reviewer',previous=previous)).encode());assert p.returncode==2;assert json.loads(p.stdout)['code']=='project-context-invalid';assert (fixture/'.swarm/engine-settings.json').read_bytes()==settings
  current=json.loads(run('guard-settings-current',[native,'engine-settings','show','--root',fixture,'--json']).stdout);change=dict(schema_version=current['schema_version'],event_id='guard-restore-context-cap',expected_revision=current['revision'],values=copy.deepcopy(current['values']),reason='Restore actual context reader');change['values']['project_context_max_bytes']=None;request.write_text(json.dumps(change));p=run('guard-settings-preview',[native,'engine-settings','preview','--root',fixture,'--input',request,'--json']);assert p.returncode==0;change['preview_token']=json.loads(p.stdout)['preview_token'];request.write_text(json.dumps(change));assert run('guard-settings-apply',[native,'engine-settings','apply','--root',fixture,'--input',request,'--json']).returncode==0;guard('guard-cap-restored',previous)

 result=dict(status='PASS',scope='ordinary project source reader/default-cap parity and native configured effect; memory/reviewer authority excluded',fixture=str(fixture),commands=commands,freshness_guard=guard_probe is not None,freshness_cases=guard_commands,global_watchdog=None,source_reference='actual Go declarations plus explicitly scoped ordinary-source adapter',prompt_security_claim_clarified=True)
 (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');return result
