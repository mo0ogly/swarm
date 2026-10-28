"""Run the actual Go/Rust bootstrap command; no mission/provider/service changes."""
import copy
import hashlib
import json
import os
from pathlib import Path
import stat
from bootstrap_scenarios import document,canonical

def run(go,rust,work,output,env,command):
 root=work/'cli-fixture';root.mkdir(mode=0o700)
 for name in ['bootstrap','configuration','witness']:(root/name).mkdir(mode=0o700)
 path=root/'bootstrap/bootstrap.json';d=document(str(root/'configuration'),str(root/'witness'));d['service_uid']=os.geteuid();d['controller_uid']=4294967295^os.geteuid();d['pins']['bootstrap_principal']['subject']='fixture <human>\u2028x'
 raw=canonical(d);path.write_bytes(raw);path.chmod(0o600);digest=hashlib.sha256(raw).hexdigest();cases=[];differences=[]
 def snapshot():return {str(p.relative_to(root)):dict(mode=stat.S_IMODE(p.lstat().st_mode),links=p.lstat().st_nlink,sha256=hashlib.sha256(p.read_bytes()).hexdigest()if p.is_file()else None)for p in root.rglob('*')}
 def invoke(name,args,language='fr',expected=0,exact=True):
  before=snapshot();results=[]
  for tag,binary in [('go',go),('rust',rust)]:
   stdout,stderr,code=command(name+'-'+tag,[str(binary),*args],cwd=root,extra_env=dict(SWARM_LANG=language),expected=expected);assert code==expected;results.append((stdout,stderr,code))
  assert snapshot()==before,name+': command mutated fixture';g,r=results
  assert g[2]==r[2]
  if expected==0:
   assert g[0]==r[0]and g[1]==r[1]==b'',name+': successful response divergence'
   value=json.loads(r[0]);assert value['schema_version']==1 and value['bootstrap_digest']==digest and value['bootstrap']==d and value['storage_verified']is True
   assert all(value[k]is False for k in ['human_authentication_verified','confinement_verified','service_started'])
  else:
   assert g[0]==r[0]==b'' and g[1]and r[1]
   if exact:assert g[1]==r[1],name+': diagnostic divergence'
   else:
    gg=json.loads(g[1]);rr=json.loads(r[1]);assert gg['schema_version']==rr['schema_version']==1 and gg['failure']['code']==rr['failure']['code']=='command_failed'and not gg['failure']['retryable']and not rr['failure']['retryable'];differences.append(dict(case=name,reason='Rust public diagnostic omits detailed protected filesystem cause; original retained by RunOutcome'))
  cases.append(name);return results
 valid=['identity','bootstrap-check',digest,'--input',str(path),'--json']
 for language in ['fr','en']:
  invoke('valid-'+language,valid,language)
  invoke('valid-without-json-flag-'+language,valid[:-1],language)
  invoke('valid-last-option-wins-'+language,valid+['--root','other','--root','.','--phase','other','--phase','delivery'],language)
  invoke('valid-explicit-lang-'+language,valid+['--lang',language], 'xx')
 for name,args in [
  ('identity-only',['identity']),('missing-input',['identity','bootstrap-check',digest]),('stdin-refused',['identity','bootstrap-check',digest,'--input','-']),
  ('invalid-operation',['identity','activate',digest,'--input',str(path)]),
  ('server-refused',valid+['--server','http://127.0.0.1']),('empty-server-refused',valid+['--server','']),('project-refused',valid+['--root','other']),
  ('output-refused',valid+['--output','out.json']),('task-refused',valid+['--task','task']),('phase-refused',valid+['--phase','other']),
  ('unknown-option',valid+['--unknown']),('missing-option-value',valid+['--input']),('input-value-is-json-flag',['identity','bootstrap-check',digest,'--input','--json']),
  ('bad-language',valid+['--lang','xx']),('missing-language',valid+['--lang'])]:
  invoke('valid-before-'+name,valid)
  invoke(name,args,expected=2)
 for name,args in [('missing-input-value',valid+['--input']),('usage',['identity']),('bad-language',valid+['--lang','xx'])]:invoke('english-'+name,args,'en',2)
 for value in ['', 'A'*64, 'b'*64]:
  invoke('pin-valid-control-'+repr(value),valid)
  args=valid.copy();args[2]=value;invoke('pin-refusal-'+repr(value),args,expected=2)
 # Empty and noncanonical declarations have independently pinned malformed bytes.
 for name,data in [('empty-document',b'{}'),('noncanonical',raw+b'\n'),('private-bad-type',b'{"schema_version":"private-raw-value"}')]:
  invoke('valid-before-'+name,valid);path.write_bytes(data);args=valid.copy();args[2]=hashlib.sha256(data).hexdigest();response=invoke(name,args,expected=2)
  assert b'private-raw-value'not in response[1][1];path.write_bytes(raw)
 for target in [path,root/'configuration',root/'witness',root/'bootstrap']:
  invoke('valid-before-permissions-'+target.name,valid);old=stat.S_IMODE(target.stat().st_mode);target.chmod(0o644 if target==path else 0o755)
  invoke('permissions-'+target.name,valid,expected=2,exact=target==path);target.chmod(old);invoke('recovered-permissions-'+target.name,valid)
 # An actual failing output device exercises kernel ENOSPC without filling disk.
 invoke('valid-before-output-device-failure',valid)
 for tag,binary in [('go',go),('rust',rust)]:
  stdout,stderr,code=command('full-output-'+tag,[str(binary),*valid],cwd=root,extra_env=dict(SWARM_LANG='fr'),expected=2,full_output=True)
  result=json.loads(stderr);assert result['failure']['code']=='storage_unavailable'and result['failure']['retryable']is False
 invoke('valid-after-output-device-failure',valid)
 # Both help variants declare the actual partial development scope. They are not
 # compared with Go's complete help, since the engine is still being ported.
 for language,word in [('fr',b'migration partielle'),('en',b'partial migration')]:
  stdout,stderr,status=command('native-help-'+language,[str(rust),'--help'],cwd=root,extra_env=dict(SWARM_LANG=language),expected=0);assert word in stdout and not stderr
 before=snapshot();stdout,stderr,status=command('native-unsupported',[''+str(rust),'mission','status','anything','--json'],cwd=root,expected=2);assert not stdout and json.loads(stderr)['failure']['code']=='command_failed';assert snapshot()==before
 stdout,stderr,status=command('native-empty-path',[str(rust),*valid],cwd=root,extra_env=dict(PATH='',SWARM_LANG='en'),expected=0);assert json.loads(stdout)['storage_verified']and not stderr
 trace=output/'native-execve.trace';stdout,stderr,status=command('native-execve',['/usr/bin/strace','-f','-e','trace=execve,execveat','-o',str(trace),str(rust),*valid],cwd=root,extra_env=dict(PATH='',SWARM_LANG='fr'),expected=0)
 assert json.loads(stdout)['storage_verified']and not stderr
 executed=[line for line in trace.read_text().splitlines()if 'execve('in line or 'execveat('in line];assert len(executed)==1 and str(rust)in executed[0],executed
 assert not(root/'.swarm').exists()
 for name in ['configuration','witness']:assert not list((root/name).iterdir())
 return dict(paired_cases=len(cases),cases=cases,diagnostic_differences=differences,full_output_device_handled=True,native_execve_calls=len(executed),no_go_runtime_call_observed=True,empty_path_native_success=True,project_created=False,human_or_service_claimed=False)
