"""Source-bound pure managed-cohort contract parity; no Root, provider or review execution."""
import datetime,hashlib,json,os,shutil,subprocess,sys,tempfile,time
from collections import Counter
from pathlib import Path
from qualification_inputs import collect
ROOT=Path(__file__).resolve().parents[2]
def main():
 """Qualify fixture contracts in a fresh output area; preserve sources/results and raise on failures or changed inputs. No deployment or provider call is authorized."""
 output=Path(sys.argv[1]).resolve()
 if output.exists():raise RuntimeError('Fresh evidence directory required')
 output.mkdir(parents=True);work=Path(tempfile.mkdtemp(prefix='managed-contract-',dir=ROOT/'.cache'));commands=[]
 env=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(work/'target')))
 def save(name,value):
  """Atomically persist one JSON report in the evidence area; propagate I/O failures."""
  path=output/name;fd,tmp=tempfile.mkstemp(dir=path.parent,prefix='.'+path.name+'.')
  with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
  os.replace(tmp,path)
 def run(label,args,data=None,expected=0):
  """Run a check without a watchdog; retain exact arguments, input and output, returning stdout or raising for an unexpected exit."""
  start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic();p=subprocess.run([os.fspath(a)if isinstance(a,os.PathLike)else a for a in args],cwd=ROOT,env=env,input=data,capture_output=True)
  for suffix,raw in [('stdout',p.stdout),('stderr',p.stderr)]: (output/(label+'.'+suffix)).write_bytes(raw)
  if data is not None:(output/(label+'.stdin')).write_bytes(data)
  commands.append(dict(label=label,command=[dict(bytes_hex=a.hex())if isinstance(a,bytes)else str(a)for a in args],start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));save('commands.json',commands);print(label,'exit',p.returncode,flush=True)
  if p.returncode!=expected:raise RuntimeError(label+': original failure retained')
  return p.stdout
 try:
  raw=run('reference-source',['go','run','./tools/rust-migration/managed-contract-reference']);source=work/'main.go';source.write_bytes(raw);package='./'+str(work.relative_to(ROOT))
  before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,['./internal/engine','./tools/rust-migration/managed-contract-reference',package],allow_root=True,extra_files=[source])
  run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
  run('compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets'])
  run('clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets','--','-D','warnings'])
  reference=work/'go-reference';run('reference-build',['go','build','-o',reference,source]);run('reference-vet',['go','vet','./tools/rust-migration/managed-contract-reference',package]);assert raw==run('reference-after',['go','run','./tools/rust-migration/managed-contract-reference'])
  args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--test','review_contract'];tests=run('targeted-tests',args).decode();listing=run('targeted-inventory',args+['--','--list']).decode()
  names=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in tests.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));assert names and names==Counter(l.removesuffix(': test')for l in listing.splitlines()if l.endswith(': test'))
  run('probe-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example','managed_review_contract_probe']);native=Path(env['CARGO_TARGET_DIR'])/'debug/examples/managed_review_contract_probe'
  base=dict(id='work',tasks=[dict(id='selected',title='Task',criteria=['a','b'],attempts=[dict(id='old'),dict(id='latest')]),dict(id='prior',status='accepted',title='Prior'),dict(id='ignored',status='todo')])
  cases=[('empty','{}',True),('null','null',True),('empty-tasks','{"tasks":[]}',True),('base',json.dumps(base),True)]
  def changed(label,change):
   """Append one valid Work variation to the parity vectors without altering the baseline."""
   value=json.loads(json.dumps(base));change(value,value['tasks']);cases.append((label,json.dumps(value),True))
  changed('ignored-title',lambda w,t:t[2].update(title='Changed ignored'))
  changed('old-attempt',lambda w,t:t[0]['attempts'][0].update(id='Changed old'))
  changed('metadata',lambda w,t:w.update(id='other',revision=9,planning=dict(repository=dict(candidate_commit='other'))))
  changed('selected-state',lambda w,t:t[0].update(status='blocked',owner='other'))
  changed('latest-attempt',lambda w,t:t[0]['attempts'][-1].update(id='Changed latest'))
  changed('next-attempt',lambda w,t:t[0]['attempts'].append(dict(id='next')))
  changed('title',lambda w,t:t[0].update(title='Changed'))
  changed('prior-title',lambda w,t:t[1].update(title='Changed'))
  changed('prior-state',lambda w,t:t[1].update(status='Accepted'))
  changed('policy-empty',lambda w,t:t[1].update(validation_policy={}))
  changed('policy-control',lambda w,t:t[1].update(validation_policy=dict(mode='automatic',controls=[dict(id='check',command=['cargo','test'],timeout_seconds=7)])))
  changed('criteria-empty',lambda w,t:t[0].update(criteria=[]))
  changed('criteria-order',lambda w,t:t[0]['criteria'].reverse())
  changed('memory',lambda w,t:t[0].update(memory_selections=[dict(work='w',id='m',version=1,sha256='a'*64)]))
  changed('order',lambda w,t:t.reverse())
  changed('new-accepted',lambda w,t:t.append(dict(id='new',status='accepted')))
  changed('new-ignored',lambda w,t:t.append(dict(id='new',status='todo')))
  changed('duplicate-selected',lambda w,t:t.append(dict(id='selected',status='todo')))
  changed('unicode',lambda w,t:t[0].update(title='É<>&\u2028'))
  changed('irrelevant-opaque',lambda w,t:t[2].update(gate=dict(document=dict(raw='placeholder'))))
  label,raw,valid=cases[-1];cases[-1]=(label,raw.replace('"placeholder"','1e999'),valid)
  for label,raw in [('tasks-type','{"tasks":"wrong"}'),('task-id-type','{"tasks":[{"id":1}]}'),('attempt-type','{"tasks":[{"attempts":[{"id":1}]}]}'),('criteria-type','{"tasks":[{"criteria":1}]}'),('policy-type','{"tasks":[{"validation_policy":[]}]}'),('array','[]'),('trailing','{}{}'),('syntax','{')]:cases.append((label,raw,False))
  results={}
  for label,raw,valid in cases:
   data=raw.encode();expected=0 if valid else 2;go=run(label+'-go',[reference,'selected'],data,expected);rust=run(label+'-native',[native,'selected'],data,expected)
   if valid:
    g=json.loads(go);n=json.loads(rust);assert g==n and set(n)=={'contract'}and len(n['contract'])==64;results[label]=n['contract']
   else:assert not go and not rust
  assert results['empty']==results['null']==results['empty-tasks']
  for label in ['ignored-title','old-attempt','metadata','selected-state','new-ignored','irrelevant-opaque']:assert results[label]==results['base'],label
  for label in ['latest-attempt','next-attempt','title','prior-title','prior-state','policy-empty','policy-control','criteria-empty','criteria-order','memory','order','new-accepted','duplicate-selected','unicode']:assert results[label]!=results['base'],label
  save('vectors.json',[dict(name=n,raw=r,valid=v,contract=results.get(n))for n,r,v in cases])
  argument_checks=['missing','extra','invalid-utf8']
  for label,args in [('missing',[native]),('extra',[native,'selected','extra']),('invalid-utf8',[native,b'\xff'])]:
   control=run(label+'-argument-valid-control',[native,'selected'],b'{}');assert json.loads(control)['contract']==results['empty']
   assert not run(label+'-argument-native',args,b'{}',2)
  for name,binary in [('go-reference',reference),('native-managed-contract-probe',native)]:target=output/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600)
  close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
  run('diff-check',['git','diff','--check'])
  save('result.json',dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3am pure managed cohort contract; no candidate/authority verification',tests=list(names.elements()),fixture_argument_checks=argument_checks,exact_non_utf8_argument_recorded=True,cases=len(cases),valid=sum(v for _,_,v in cases),invalid=sum(not v for _,_,v in cases),inputs=len(before),registry_packages=len(registry),source_stable=True,global_watchdog=None,fixture_root=str(work),provider_calls=False,root_mutation=False,independent_review=False,deployment=False))
  print('PASS pure managed cohort contract; PARTIAL full migration',flush=True)
 except BaseException as error:save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__,fixture_root=str(work)));raise
if __name__=='__main__':main()
