"""Qualify current ordinary review consistency, not independent review authority."""
from collections import Counter
import datetime,json,os,shutil,subprocess,sys,tempfile,time
from pathlib import Path
from qualification_inputs import collect, rust_assets
from qualify_project_context import qualify as qualify_context
from qualify_review_evidence import qualify as qualify_review
ROOT=Path(__file__).resolve().parents[2]
def main():
 output=Path(sys.argv[1]).resolve()
 if output.exists():raise RuntimeError('Fresh evidence directory required')
 output.mkdir(parents=True);work=Path(tempfile.mkdtemp(prefix='review-evidence-',dir=ROOT/'.cache'));commands=[]
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
  sources={};packages=['./internal/engine'];files=[]
  for name in ['project-reference','review-contract-reference']:
   tool='./tools/rust-migration/'+name;raw=run(name+'-source',['go','run',tool]);folder=work/name;folder.mkdir();source=folder/'main.go';source.write_bytes(raw);packages.extend([tool,'./'+str(folder.relative_to(ROOT))]);files.append(source);sources[name]=(raw,source,tool)
  before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,packages,allow_root=True,extra_files=files)
  reuse=Path(sys.argv[2]).resolve()if len(sys.argv)>2 else None
  native_verification=dict(mode='executed',executed_again=True)
  if reuse is None:
   run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
   args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--test','review_evidence','--test','review_contract','--test','gate_evaluation'];raw=run('targeted-tests',args).decode();listing=run('targeted-inventory',args+['--','--list']).decode();tests=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));assert tests and tests==Counter(l.removesuffix(': test')for l in listing.splitlines()if l.endswith(': test'))
   run('compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets'])
   run('clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets','--','-D','warnings'])
  else:
   # All literal Rust includes currently stay inside rust/. Refuse reuse rather
   # than silently missing a newly introduced outside input.
   rust_sources={p.resolve()for p in(ROOT/'rust').rglob('*.rs')if'target'not in p.relative_to(ROOT/'rust').parts}
   assert all(p.is_relative_to(ROOT/'rust')for p in rust_assets(ROOT,rust_sources))
   prior=json.loads((reuse/'inputs-before.json').read_text())
   relevant=lambda values:{k:v for k,v in values.items()if k.startswith(('rust/','registry/'))}
   assert relevant(prior)and relevant(prior)==relevant(before),'native input membership or bytes changed'
   for name in ['build-environment.json','cargo-metadata.stdout']:
    assert json.loads((reuse/name).read_text())==json.loads((output/name).read_text()),'native build configuration changed'
   for name in ['rust-version.stdout','cc-version.stdout']:
    assert (reuse/name).read_bytes()==(output/name).read_bytes(),'native compiler changed'
   labels=['format','targeted-tests','targeted-inventory','compile','clippy'];prior_commands={c['label']:c for c in json.loads((reuse/'commands.json').read_text())}
   for label in labels:
    command=prior_commands[label];assert command['exit']==command['expected_exit']==0
    for ext in ['stdout','stderr']:shutil.copyfile(reuse/(label+'.'+ext),output/(label+'.'+ext))
   raw=(output/'targeted-tests.stdout').read_text();listing=(output/'targeted-inventory.stdout').read_text();tests=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'));assert tests and tests==Counter(l.removesuffix(': test')for l in listing.splitlines()if l.endswith(': test'))
   assert prior_commands['targeted-tests']['command']==['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--test','review_evidence','--test','review_contract','--test','gate_evaluation']
   native_verification=dict(mode='reused',executed_again=False,source=str(reuse),native_inputs=len(relevant(before)),labels=labels,compiler_graph_environment_and_literal_assets_checked=True)
   save('native-verification-reuse.json',native_verification);print('PASS reuse unchanged native checks; fresh Go/reference checks follow',flush=True)
  references={}
  for name,(raw,source,tool)in sources.items():
   binary=work/name/'oracle';run(name+'-build',['go','build','-o',binary,source]);run(name+'-vet',['go','vet',tool,'./'+str(source.parent.relative_to(ROOT))]);assert raw==run(name+'-after',['go','run',tool]);references[name]=binary
  run('native-cli-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-cli'])
  run('probe-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example','review_evidence_probe','--example','project_context_probe'])
  target=Path(env['CARGO_TARGET_DIR'])/'debug';native=target/'swarm-rust';probe=target/'examples/review_evidence_probe';context_probe=target/'examples/project_context_probe'
  context=qualify_context(ROOT,output/'project-context',native,references['project-reference'],context_probe)
  review=qualify_review(ROOT,output/'review-evidence',native,probe,references['project-reference'],references['review-contract-reference'])
  for name,binary in [('native-cli',native),('native-review-evidence-probe',probe),('native-context-probe',context_probe),*references.items()]:destination=output/(name+'.binary');shutil.copyfile(binary,destination);destination.chmod(0o600)
  close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after);run('diff-check',['git','diff','--check'])
  save('result.json',dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3ai ordinary review consistency on supplied Work/default-current context policy; no Root-row authority',tests=list(tests.elements()),native_verification=native_verification,project_context=context,review_evidence=review,inputs=len(before),registry_packages=len(registry),source_stable=True,global_watchdog=None,root_row_review_guard=False,independent_review=False,managed_review=False,deployment=False,fixture_root=str(work)))
  print('PASS ordinary review consistency; PARTIAL full migration',flush=True)
 except BaseException as error:save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__,fixture_root=str(work)));raise
if __name__=='__main__':main()
