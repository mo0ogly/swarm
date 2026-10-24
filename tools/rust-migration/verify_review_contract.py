"""Source-bound pure review-contract parity; no Root, provider or review execution."""
import datetime,hashlib,json,os,shutil,subprocess,sys,tempfile,time
from collections import Counter
from pathlib import Path
from qualification_inputs import collect
ROOT=Path(__file__).resolve().parents[2]
def main():
 output=Path(sys.argv[1]).resolve()
 if output.exists():raise RuntimeError('Fresh evidence directory required')
 output.mkdir(parents=True);work=Path(tempfile.mkdtemp(prefix='review-contract-',dir=ROOT/'.cache'));commands=[]
 env=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(work/'target')))
 def save(name,value):
  path=output/name;fd,tmp=tempfile.mkstemp(dir=path.parent,prefix='.'+path.name+'.')
  with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
  os.replace(tmp,path)
 def run(label,args,data=None,expected=0):
  start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic();p=subprocess.run(list(map(str,args)),cwd=ROOT,env=env,input=data,capture_output=True)
  for suffix,raw in [('stdout',p.stdout),('stderr',p.stderr)]: (output/(label+'.'+suffix)).write_bytes(raw)
  if data is not None:(output/(label+'.stdin')).write_bytes(data)
  commands.append(dict(label=label,command=list(map(str,args)),start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));save('commands.json',commands);print(label,'exit',p.returncode,flush=True)
  if p.returncode!=expected:raise RuntimeError(label+': original failure retained')
  return p.stdout
 try:
  raw=run('reference-source',['go','run','./tools/rust-migration/review-contract-reference']);source=work/'main.go';source.write_bytes(raw);package='./'+str(work.relative_to(ROOT))
  before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,['./internal/engine','./tools/rust-migration/review-contract-reference',package],allow_root=True,extra_files=[source])
  run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
  args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--test','review_contract'];tests=run('targeted-tests',args).decode();listing=run('targeted-inventory',args+['--','--list']).decode()
  names=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in tests.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));assert names and names==Counter(l.removesuffix(': test')for l in listing.splitlines()if l.endswith(': test'))
  run('compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets'])
  run('clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets','--','-D','warnings'])
  reference=work/'go-reference';run('reference-build',['go','build','-o',reference,source]);run('reference-vet',['go','vet','./tools/rust-migration/review-contract-reference',package]);assert raw==run('reference-after',['go','run','./tools/rust-migration/review-contract-reference'])
  run('probe-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example','review_contract_probe']);native=Path(env['CARGO_TARGET_DIR'])/'debug/examples/review_contract_probe'
  cases=[('empty','{}',True),('null','null',True),('title','{"title":"Changed"}',True),('delivery','{"deliverable":"report.md"}',True),('criteria-empty','{"criteria":[]}',True),('depends-empty','{"depends":[]}',True),('brief','{"plan_brief_hash":"new"}',True),('memory-empty','{"memory_selections":[]}',True),('irrelevant','{"id":"another","status":"accepted","owner":"operator"}',True),('opaque-irrelevant','{"gate":{"document":{"n":1e999,"zero":-0}}}',True),('unicode','{"title":"É<>&\\u2028\\ud800","criteria":["a","b"]}',True),('duplicate-null','{"title":"old","TITLE":"new","title":null}',True)]
  base=dict(title='Task',deliverable='docs/delivery.md',criteria=['a','b'],depends=['x','y'],plan_brief_hash='brief',memory_selections=[dict(work='w',id='memory',version=1,sha256='a'*64)])
  cases.append(('base',json.dumps(base),True))
  for field in ['work','id','version','sha256']:
   value=json.loads(json.dumps(base));value['memory_selections'][0][field]=2 if field=='version'else'changed';cases.append(('memory-'+field,json.dumps(value),True))
  for field in ['criteria','depends']:
   value=json.loads(json.dumps(base));value[field].reverse();cases.append(('order-'+field,json.dumps(value),True))
  value=json.loads(json.dumps(base));value['memory_selections'].append(dict(work='other',id='m2',version=2,sha256='b'*64));cases.append(('memory-pair',json.dumps(value),True));value['memory_selections'].reverse();cases.append(('memory-order',json.dumps(value),True))
  for label,raw in [('criteria-type','{"criteria":"wrong"}'),('depends-type','{"depends":2}'),('memory-version','{"memory_selections":[{"version":1.5}]}'),('memory-overflow','{"memory_selections":[{"version":9223372036854775808}]}'),('array','[]'),('trailing','{}{}'),('syntax','{')]:cases.append((label,raw,False))
  results={}
  for label,raw,valid in cases:
   data=raw.encode();expected=0 if valid else 2;go=run(label+'-go',[reference],data,expected);rust=run(label+'-native',[native],data,expected)
   if valid:
    g=json.loads(go);n=json.loads(rust);assert g==n and set(n)=={'contract'}and len(n['contract'])==64;results[label]=n['contract']
   else:assert not go and not rust
  assert results['empty']==results['null']==results['memory-empty']==results['irrelevant']==results['opaque-irrelevant']
  for field in ['title','delivery','criteria-empty','depends-empty','brief']:assert results[field]!=results['empty']
  for field in ['work','id','version','sha256']:assert results['memory-'+field]!=results['base']
  assert results['order-criteria']!=results['base']and results['order-depends']!=results['base']and results['memory-pair']!=results['memory-order']
  save('vectors.json',[dict(name=n,raw=r,valid=v,contract=results.get(n))for n,r,v in cases])
  for name,binary in [('go-reference',reference),('native-review-contract-probe',native)]:target=output/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600)
  close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
  run('diff-check',['git','diff','--check'])
  save('result.json',dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3ah pure review instruction contract; no current identity/review authority or acceptance',tests=list(names.elements()),cases=len(cases),valid=sum(v for _,_,v in cases),invalid=sum(not v for _,_,v in cases),inputs=len(before),registry_packages=len(registry),source_stable=True,global_watchdog=None,fixture_root=str(work),provider_calls=False,root_mutation=False,independent_review=False,deployment=False))
  print('PASS pure review contract; PARTIAL full migration',flush=True)
 except BaseException as error:save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__,fixture_root=str(work)));raise
if __name__=='__main__':main()
