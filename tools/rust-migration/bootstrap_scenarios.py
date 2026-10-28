"""Read-only bootstrap parity over canonical bytes and protected fixture files."""
import copy
import hashlib
import itertools
import json
import os
from pathlib import Path
import stat
import subprocess

def canonical(value):
 return json.dumps(value,separators=(',',':'),ensure_ascii=False).replace('&','\\u0026').replace('<','\\u003c').replace('>','\\u003e').replace('\u2028','\\u2028').replace('\u2029','\\u2029').encode()
def document(config='/service/config',witness='/service/witness'):
 return dict(schema_version=1,pins=dict(installation_digest='a'*64,authority_public_key='BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc',bootstrap_principal=dict(principal_id='operator',principal_kind='human',issuer='https://issuer.example',subject='fixture-human')),service_uid=1000,controller_uid=1001,configuration_directory=config,witness_directory=witness)
def vectors():
 result=[]
 def raw(name,data):result.append(dict(name=name,operation='decode',raw=data.hex()))
 def typed(name,d):
  result.append(dict(name=name+'-encode',operation='encode',document=d));raw(name+'-decode',canonical(d))
 d=document();typed('valid',d);raw('empty',b'');raw('null',b'null');raw('array',b'[]');raw('trailing',canonical(d)+b'{}');raw('whitespace',b' '+canonical(d))
 for order in itertools.permutations(d):raw('order-'+','.join(order),canonical({k:d[k]for k in order}))
 for field in d:
  for value in [None,False,True,[],{},'',0,1,1.0,-1,4294967295,4294967296,18446744073709551616]:
   x=copy.deepcopy(d);x[field]=value;raw('type-'+field+'-'+repr(value),canonical(x))
  x=copy.deepcopy(d);del x[field];raw('missing-'+field,canonical(x))
  raw('folded-'+field,canonical(d).replace(('"'+field+'"').encode(),('"'+field.upper()+'"').encode(),1))
  raw('duplicate-'+field,b'{'+canonical({field:d[field]})[1:-1]+b','+canonical(d)[1:])
 for field in ['configuration_directory','witness_directory']:
  for path in ['', '/', 'relative','/service/./config','/service/../config','/service/config/','/service//config','/service/config','/service/config/nested','/service/config-other']:
   x=copy.deepcopy(d);x[field]=path;typed('path-'+field+'-'+repr(path),x)
  for code in range(256):
   x=copy.deepcopy(d);x[field]+='/x'+chr(code)+'x';typed('path-byte-'+field+'-'+str(code),x)
 for field in ['principal_id','principal_kind','issuer','subject']:
  for code in range(256):
   x=copy.deepcopy(d);x['pins']['bootstrap_principal'][field]='x'+chr(code)+'x';typed('principal-byte-'+field+'-'+str(code),x)
 for field in ['installation_digest','authority_public_key']:
  for value in ['', 'a'*64,'A'*64,'BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=','bad',' ']:
   x=copy.deepcopy(d);x['pins'][field]=value;typed('pins-'+field+'-'+value,x)
 for uid in [0,1,1000,1001,4294967295]:
  for controller in [0,1,1000,1001,4294967295]:
   x=copy.deepcopy(d);x['service_uid']=uid;x['controller_uid']=controller;typed('uids-'+str(uid)+'-'+str(controller),x)
 for depth in [1,100,9999,10000]:raw('depth-'+str(depth),b'{"unknown":'+b'['*depth+b'0'+b']'*depth+b'}')
 for payload in [b'\xff',b'\xed\xa0\x80',b'\\ud800',b'\\udfff']:
  raw('invalid-string-'+payload.hex(),canonical(d).replace(b'fixture-human',payload))
 return result

def run(go,rust,work,output,env,command):
 root=work/'bootstrap-fixtures';root.mkdir(mode=0o700)
 for name in ['bootstrap','configuration','witness']:(root/name).mkdir(mode=0o700)
 vv=vectors();payload=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in vv)
 observed=[]
 for label,binary in [('bootstrap-go',go),('bootstrap-rust',rust)]:
  raw=command(label,[str(binary),str(root)],payload);rr=[json.loads(line)for line in raw.splitlines()];assert len(rr)==len(vv)
  observed.append(rr)
 mismatch=[dict(input=v,go=g,rust=r)for v,g,r in zip(vv,*observed)if g!=r]
 (output/'bootstrap-mismatches.json').write_text(json.dumps(mismatch,indent=2)+'\n');assert not mismatch,'bootstrap format divergence'
 assert observed[0][0]['code']=='ok'and any(v['code']=='document'for v in observed[0])
 # Same read-only files for both implementations, with valid controls before
 # each fault. Keep every request/result and prove directory contents unchanged.
 path=root/'bootstrap/bootstrap.json';d=document(str(root/'configuration'),str(root/'witness'));d['service_uid']=os.geteuid();d['controller_uid']=4294967295^os.geteuid();base=canonical(d)
 def write(raw):path.write_bytes(raw);path.chmod(0o600)
 def snapshot():
  return {str(p.relative_to(root)):dict(mode=stat.S_IMODE(p.lstat().st_mode),links=p.lstat().st_nlink,bytes=hashlib.sha256(p.read_bytes()).hexdigest()if p.is_file()and not p.is_symlink()else None,symlink=os.readlink(p)if p.is_symlink()else None)for p in root.rglob('*')if not stat.S_ISFIFO(p.lstat().st_mode)}
 clients=[];logs=[];cases=[]
 try:
  for label,binary in [('bootstrap-load-go',go),('bootstrap-load-rust',rust)]:
   log=(output/(label+'.jsonl')).open('w');err=(output/(label+'.stderr')).open('wb');p=subprocess.Popen([str(binary),str(root)],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=err);clients.append((p,err));logs.append(log)
  def load(name,expected='ok',source=path,digest=None):
   request=dict(name=name,operation='load',path=str(source),digest=digest if digest is not None else hashlib.sha256(path.read_bytes()).hexdigest())
   before=snapshot();result=[]
   for (p,_),log in zip(clients,logs):
    log.write(json.dumps(dict(input=request))+'\n');log.flush();p.stdin.write(json.dumps(request).encode()+b'\n');p.stdin.flush();raw=p.stdout.readline();assert raw,'fixture process exited';v=json.loads(raw);log.write(json.dumps(dict(output=v))+'\n');log.flush();result.append(v)
   assert result[0]==result[1],(name,result);assert result[0]['code']==expected,(name,result)
   if expected!='ok':assert result[0]['document']is None and result[0]['zero'] and result[0]['raw']==''
   assert before==snapshot(),name+': read-only load mutated files';cases.append(name)
   return result[0]
  write(base);assert load('valid')['document']==d
  for digest in ['', 'A'*64, 'b'*64]:load('invalid-pin-'+digest,'pin',digest=digest)
  load('bad-pin-before-bad-path','pin',source='relative',digest='')
  for field in ['configuration','witness','bootstrap']:
   load('valid-before-directory-'+field);(root/field).chmod(0o755);load('unsafe-directory-'+field,'path');(root/field).chmod(0o700);load('restored-directory-'+field)
  for variant in ['missing','file-mode','symlink','hardlink','fifo','config-symlink','service-uid','bootstrap-nested','noncanonical','directory-as-file','relative']:
   write(base);load('valid-before-'+variant);source=path;data=base
   if variant=='missing':path.unlink()
   elif variant=='file-mode':path.chmod(0o644)
   elif variant=='symlink':source=path.with_suffix('.link');source.symlink_to(path)
   elif variant=='hardlink':os.link(path,path.with_suffix('.hard'))
   elif variant=='fifo':path.unlink();os.mkfifo(path,0o600)
   elif variant=='config-symlink':
    source_link=root/'configuration-link';source_link.symlink_to(root/'configuration');x=copy.deepcopy(d);x['configuration_directory']=str(source_link);data=canonical(x);write(data)
   elif variant=='service-uid':x=copy.deepcopy(d);x['service_uid'],x['controller_uid']=x['controller_uid'],x['service_uid'];data=canonical(x);write(data)
   elif variant=='bootstrap-nested':x=copy.deepcopy(d);x['configuration_directory']=str(root/'bootstrap');data=canonical(x);write(data)
   elif variant=='noncanonical':data=base+b'\n';write(data)
   elif variant=='directory-as-file':source=path.parent
   elif variant=='relative':source='relative/bootstrap.json'
   load('refuse-'+variant,'document'if variant=='noncanonical'else'path',source=source,digest=hashlib.sha256(data).hexdigest())
   if variant=='symlink':source.unlink()
   elif variant=='hardlink':path.with_suffix('.hard').unlink()
   elif variant=='fifo':path.unlink()
   elif variant=='config-symlink':(root/'configuration-link').unlink()
   write(base);load('recovered-'+variant)
  replacement=copy.deepcopy(d);replacement['pins']['bootstrap_principal']['subject']='replacement';write(canonical(replacement));load('canonical-replacement-cannot-change-pin','pin',digest=hashlib.sha256(base).hexdigest());write(base);load('final-valid')
 finally:
  for p,err in clients:
   if p.poll()is None:p.stdin.close()
   p.wait();err.close()
  for log in logs:log.close()
 return dict(format_cases=len(vv),format_valid=sum(v['code']=='ok'for v in observed[0]),load_operations=len(cases),load_cases=cases,read_only=True,same_files_both_languages=True)
