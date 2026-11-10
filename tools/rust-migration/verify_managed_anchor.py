"""Source-bound candidate/file bindings only; never a complete managed review."""
import copy, datetime, hashlib, json, os, shutil, subprocess, sys, tempfile, time
from collections import Counter
from pathlib import Path
from qualification_inputs import collect, rust_assets
ROOT=Path(__file__).resolve().parents[2]
def main():
 """Qualify fixture contracts in a fresh output area; preserve sources/results and raise on failures or changed inputs. No deployment or provider call is authorized."""
 output=Path(sys.argv[1]).resolve()
 if output.exists():raise RuntimeError('Fresh evidence directory required')
 output.mkdir(parents=True);work=Path(tempfile.mkdtemp(prefix='managed-anchor-',dir=ROOT/'.cache'));commands=[]
 scratch=Path(tempfile.mkdtemp(prefix='.swarm-managed-anchor-',dir=Path.home().resolve()))
 env=dict(os.environ,TMPDIR=str(scratch),GOTMPDIR=str(scratch),CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(work/'target')))
 def save(name,value):
  """Atomically persist one JSON report in the evidence area; propagate I/O failures."""
  path=output/name;fd,tmp=tempfile.mkstemp(dir=path.parent,prefix='.'+path.name+'.')
  with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
  os.replace(tmp,path)
 def run(label,args,data=None,expected=0):
  """Run a check without a watchdog; retain exact arguments, input and output, returning stdout or raising for an unexpected exit."""
  start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic();p=subprocess.run(list(map(str,args)),cwd=ROOT,env=env,input=data,capture_output=True)
  for ext,raw in [('stdout',p.stdout),('stderr',p.stderr)]:(output/(label+'.'+ext)).write_bytes(raw)
  if data is not None:(output/(label+'.stdin')).write_bytes(data)
  commands.append(dict(label=label,command=list(map(str,args)),start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));save('commands.json',commands);print(label,'exit',p.returncode,flush=True)
  if p.returncode!=expected:raise RuntimeError(label+': original failure retained')
  return p.stdout
 try:
  source=work/'main.go';raw=run('reference-source',['go','run','./tools/rust-migration/managed-anchor-reference']);source.write_bytes(raw);package='./'+str(work.relative_to(ROOT))
  before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,['./internal/engine','./tools/rust-migration/managed-anchor-reference',package],allow_root=True,extra_files=[source])
  selected=['TestManagedReviewEvidenceAliasesKeepEveryDigestBinding','TestManagedIndependentReviewBindsReportsContractAndCandidate','TestManagedIndependentReviewPublishesSameSHAAndSeparateProcess','TestReviewMethodRestartAliasesKeepReceiptBinding','TestReviewMethodRestartRequiresExplicitCorrectionAndKeepsHistory','TestReviewMethodRestartRejectsTamperedOriginalsAndSemanticRefusal']
  text=run('go-regressions',['go','test','-timeout=0','-count=1','-v','./internal/engine','-run','^('+'|'.join(selected)+')$']).decode()
  for name in selected:assert '--- PASS: '+name+' 'in text,name+': not executed'
  reference=work/'go-reference';run('reference-build',['go','build','-o',reference,source]);run('reference-vet',['go','vet','./tools/rust-migration/managed-anchor-reference',package]);assert raw==run('reference-after',['go','run','./tools/rust-migration/managed-anchor-reference'])
  reuse=Path(sys.argv[2]).resolve()if len(sys.argv)>2 else None
  native_verification=dict(mode='executed',executed_again=True)
  if reuse is None:
   run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
   tests={}
   for label,flags in [('workspace-all',['--workspace','--all-features']),('engine-default',['-p','swarm-engine'])]:
    args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline',*flags,'--all-targets'];text=run(label,args).decode();listing=run(label+'-inventory',args+['--','--list']).decode()
    actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in text.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));expected=Counter(l.removesuffix(': test')for l in listing.splitlines()if l.endswith(': test'))
    assert actual and actual==expected and actual['root_managed_anchor_reads_actual_row_but_never_enables_acceptance']==1 and actual['managed_anchor_never_collapses_aliased_file_bindings']==1;tests[label]=list(actual.elements())
   run('compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets'])
   run('clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets','--','-D','warnings'])
  else:
   rust_sources={p.resolve()for p in(ROOT/'rust').rglob('*.rs')if'target'not in p.relative_to(ROOT/'rust').parts}
   assert all(p.is_relative_to(ROOT/'rust')for p in rust_assets(ROOT,rust_sources))
   prior=json.loads((reuse/'inputs-before.json').read_text());relevant=lambda entries:{k:v for k,v in entries.items()if k.startswith(('rust/','registry/'))}
   assert relevant(prior)and relevant(prior)==relevant(before),'native input membership or bytes changed'
   for name in ['build-environment.json','cargo-metadata.stdout']:assert json.loads((reuse/name).read_text())==json.loads((output/name).read_text()),'native build configuration changed'
   for name in ['rust-version.stdout','cc-version.stdout']:assert(reuse/name).read_bytes()==(output/name).read_bytes(),'native compiler changed'
   labels=['format','workspace-all','workspace-all-inventory','engine-default','engine-default-inventory','compile','clippy']
   execution=reuse;seen=set()
   while True:
    assert execution not in seen,'cyclic native proof reuse';seen.add(execution)
    old_inputs=json.loads((execution/'inputs-before.json').read_text());assert relevant(old_inputs)==relevant(before),'reused native execution inputs changed'
    for name in ['build-environment.json','cargo-metadata.stdout']:assert json.loads((execution/name).read_text())==json.loads((output/name).read_text()),'reused execution configuration changed'
    for name in ['rust-version.stdout','cc-version.stdout']:assert(execution/name).read_bytes()==(output/name).read_bytes(),'reused execution compiler changed'
    previous={c['label']:c for c in json.loads((execution/'commands.json').read_text())}
    if all(label in previous for label in labels):break
    receipt=json.loads((execution/'result.json').read_text());assert receipt['native_component_status']=='PASS'and receipt['native_verification']['mode']=='reused','missing native execution provenance'
    execution=Path(receipt['native_verification']['source']).resolve()
   for label in labels:
    for ext in ['stdout','stderr']:assert(reuse/(label+'.'+ext)).read_bytes()==(execution/(label+'.'+ext)).read_bytes(),'reused output differs from actual execution'
   for label in labels:
    command=previous[label];assert command['exit']==command['expected_exit']==0
    for ext in ['stdout','stderr']:shutil.copyfile(reuse/(label+'.'+ext),output/(label+'.'+ext))
   tests={}
   for label,flags in [('workspace-all',['--workspace','--all-features']),('engine-default',['-p','swarm-engine'])]:
    args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline',*flags,'--all-targets'];assert previous[label]['command']==args and previous[label+'-inventory']['command']==args+['--','--list']
    text=(output/(label+'.stdout')).read_text();listing=(output/(label+'-inventory.stdout')).read_text();actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in text.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));expected=Counter(l.removesuffix(': test')for l in listing.splitlines()if l.endswith(': test'))
    assert actual and actual==expected and actual['managed_anchor_never_collapses_aliased_file_bindings']==1 and actual['root_managed_anchor_reads_actual_row_but_never_enables_acceptance']==1;tests[label]=list(actual.elements())
   assert previous['compile']['command']==['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets']
   assert previous['clippy']['command']==['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets','--','-D','warnings']
   assert previous['format']['command']==['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check']
   native_verification=dict(mode='reused',executed_again=False,source=str(reuse),native_inputs=len(relevant(before)),execution_source=str(execution),reuse_chain=[str(p)for p in seen],labels=labels,compiler_graph_environment_and_literal_assets_checked=True)
  run('probe-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example','managed_review_anchor_probe']);native=Path(env['CARGO_TARGET_DIR'])/'debug/examples/managed_review_anchor_probe'
  fixture=scratch/'files';fixture.mkdir(mode=0o700);(fixture/'.rust-migration-fixture').write_text('Owned fixture; no Git/review authority.\n')
  files={'receipt.json':b'{"candidate_commit":"fixture"}','context.json':b'{"candidate_commit":"fixture"}','report.md':b'Review report\n'}
  for name,data in files.items():(fixture/name).write_bytes(data)
  sha=lambda data:hashlib.sha256(data).hexdigest()
  review=dict(id='review',candidate_commit='fixture',receipt='receipt.json',receipt_sha256=sha(files['receipt.json']),context='context.json',context_sha256=sha(files['context.json']),report='report.md',sha256=sha(files['report.md']))
  base=dict(planning=dict(repository=dict(candidate_commit='fixture')),tasks=[dict(id='task',automatic_validation=dict(candidate_sha='fixture'),independent_review=review)])
  cases=[('valid',base,True)]
  def fault(label,change):
   """Append one invalid supplied-data scenario while keeping the valid baseline unchanged."""
   value=copy.deepcopy(base);change(value,value['tasks'][0],value['tasks'][0]['independent_review']);cases.append((label,value,False))
  fault('missing-review',lambda w,t,r:t.pop('independent_review'))
  fault('missing-automatic',lambda w,t,r:t.pop('automatic_validation'))
  fault('empty-candidate',lambda w,t,r:r.update(candidate_commit=''))
  fault('wrong-candidate',lambda w,t,r:r.update(candidate_commit='other'))
  fault('wrong-repository',lambda w,t,r:w['planning']['repository'].update(candidate_commit='other'))
  fault('wrong-automatic',lambda w,t,r:t['automatic_validation'].update(candidate_sha='other'))
  for path,digest in [('receipt','receipt_sha256'),('context','context_sha256'),('report','sha256')]:
   for value in ['',sha(b'Changed')]:fault(path+'-digest-'+('empty'if not value else'wrong'),lambda w,t,r,d=digest,v=value:r.update({d:v}))
   fault(path+'-missing',lambda w,t,r,k=path:r.update({k:'missing'}))
   fault(path+'-absolute',lambda w,t,r,k=path:r.update({k:str(fixture/r[k])}))
  outside=scratch/'outside.md';outside.write_bytes(files['report.md']);(fixture/'outside.md').symlink_to(outside)
  os.mkfifo(fixture/'fifo');(fixture/'directory').mkdir()
  for path in ['outside.md','fifo','directory']:fault('report-'+path,lambda w,t,r,p=path:r.update(report=p))
  alias=copy.deepcopy(base);r=alias['tasks'][0]['independent_review'];r.update(context=r['receipt'],report=r['receipt'],context_sha256=r['receipt_sha256'],sha256=r['receipt_sha256']);cases.append(('aliased-valid',alias,True))
  for field in ['receipt_sha256','context_sha256','sha256']:
   for bad in ['',sha(b'Changed')]:
    value=copy.deepcopy(alias);value['tasks'][0]['independent_review'][field]=bad;cases.append(('alias-'+field+'-'+('empty'if not bad else'wrong'),value,False))
  # A valid control precedes each fault, with the same filesystem prerequisites.
  results=[]
  for label,value,valid in cases:
   for kind,binary in [('go',reference),('native',native)]:
    control=run(label+'-'+kind+'-control',[binary,fixture],json.dumps(base).encode());assert json.loads(control)==dict(bindings_consistent=True)
    observed=run(label+'-'+kind,[binary,fixture],json.dumps(value).encode(),0 if valid else 2)
    if valid:assert json.loads(observed)==dict(bindings_consistent=True)
    else:assert not observed
   assert all((fixture/name).read_bytes()==data for name,data in files.items())
   results.append(dict(name=label,valid=valid))
  save('vectors.json',results)
  for name,binary in [('go-reference',reference),('native-managed-anchor-probe',native)]:target=output/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600)
  close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
  run('diff-check',['git','diff','--check'])
  save('result.json',dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3al candidate/file bindings and explicit method restart alias integrity',native_verification=native_verification,tests=tests,go_selected_tests=selected,cases=len(cases),valid=sum(v for _,_,v in cases),invalid=sum(not v for _,_,v in cases),comparison_commands=4*len(cases),actual_root_row_observation=True,git_report_verified=False,protocol_verified=False,authority_verified=False,inputs=len(before),registry_packages=len(registry),source_stable=True,global_watchdog=None,fixture_root=str(work),scratch=str(scratch),provider_calls=False,deployment=False))
  print('PASS candidate/file bindings and regression fix; PARTIAL managed review/migration',flush=True)
 except BaseException as error:save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__,fixture_root=str(work)));raise
if __name__=='__main__':main()
