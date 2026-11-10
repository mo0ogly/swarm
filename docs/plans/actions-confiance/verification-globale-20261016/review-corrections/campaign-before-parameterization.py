"""One executable for existing public recipes. No global acceptance verdict."""
import argparse,datetime,hashlib,json,os,signal,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--resume',action='store_true');p.add_argument('--report-name',default='public-campaign.json');p.add_argument('--evidence-dir',type=Path,required=True);p.add_argument('--prepare-only',action='store_true');p.add_argument('--binary',type=Path,required=True);p.add_argument('--timeout-seconds',type=int,required=True);a=p.parse_args();assert a.timeout_seconds>0
root=Path(__file__).resolve().parents[1];out=a.evidence_dir.resolve();binary=a.binary.resolve();out.mkdir(parents=True,exist_ok=True)
sha=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
binary_sha=sha(binary);stages=[]
candidate=json.loads((root/'docs/plans/actions-confiance/verification-sensitive-20261016/secret-data-verification.json').read_text())
assert candidate['binary_sha256']==binary_sha,'binary differs from qualified local candidate'
def command(name,argv,proof):stages.append((name,argv,proof))
common=['--binary',str(binary),'--timeout-seconds',str(a.timeout_seconds)]
for name,script,extra,report in [
 ('connectors','actions_confiance_connectors.py',[],'connector-public.json'),
 ('evidence','actions_confiance_evidence.py',['--archive','--checkpoint'],'evidence-public.json'),
 ('laboratory','actions_confiance_laboratory.py',[],'laboratory-public.json'),
 ('secrets','actions_confiance_remote.py',['--secret-broker'],'remote.json'),
 ('secret-data','actions_confiance_secret_data.py',[],'remote.json'),
 ('helper-image','actions_confiance_secret_image.py',['--secret-broker'],'image-replacement.json')]:
 command(name,['python3','tests/'+script,*common,'--evidence-dir',str(out/name),*extra],out/name/report)
for name,script in [('pilot','actions_confiance_t2.py'),('processes','actions_confiance_t5.py')]:
 command(name,['python3','tests/'+script,*common,'--criterion','0','--root',str(root),'--evidence-dir',str(out/name)],None)
command('controls',['python3','tests/actions_confiance_controls.py','--root',str(root),'--evidence-dir',str(out/'controls'),'--timeout-seconds',str(a.timeout_seconds)],out/'controls/mapping-public.json')
inputs_path=root/'docs/plans/actions-confiance/verification-sensitive-20261016/secret-data-final-inputs.json'
inputs={entry['path']:entry['sha256'] for entry in json.loads(inputs_path.read_text())}
for rel,digest in inputs.items():assert sha(root/rel)==digest,'compiled input changed: '+rel
for path in [*root.glob('tests/actions_confiance_*'),root/'tests/i18n_test.cjs',root/'scripts/i18n/build.cjs',root/'scripts/processes/build.cjs',root/'package.json',root/'package-lock.json',Path(__file__),inputs_path]:
 if path.is_file():inputs[str(path.relative_to(root))]=sha(path)
snapshot=out/'campaign-inputs.json'
if snapshot.exists():
 assert a.resume,'existing evidence requires explicit --resume'
 assert json.loads(snapshot.read_text())==inputs,'frozen campaign inputs changed'
else:
 assert not a.resume,'cannot resume without original snapshot'
 snapshot.write_text(json.dumps(inputs,indent=2)+'\n')
if a.prepare_only:
 destination=out/'campaign-prepared.json';assert not destination.exists()
 destination.write_text(json.dumps(dict(binary=str(binary),binary_sha256=binary_sha,input_count=len(inputs),stages=[dict(name=n,argv=v) for n,v,_ in stages],executed=False),indent=2)+'\n')
 print(json.dumps(dict(prepared=True,stages=len(stages),executed=False)));raise SystemExit(0)
receipts=[]
for name,argv,proof in stages:
 assert sha(binary)==binary_sha
 stdout=out/(name+'.stdout');stderr=out/(name+'.stderr');dest=out/(name+'-process.json')
 if dest.exists():
  assert a.resume,name+': existing execution must be explicitly resumed'
  recovered=out/(name+'-resumed-process.json')
  if recovered.exists():dest=recovered
  receipt=json.loads(dest.read_text());assert receipt['binary_sha256']==binary_sha and receipt['argv']==argv and receipt['exit_code']==0
  try:
   stat=Path(f"/proc/{receipt['pid']}/stat").read_text();stamp=stat[stat.rfind(')')+2:].split()[19]
   assert stamp!=receipt['start_ticks'],name+': original process is still alive'
  except (FileNotFoundError,ProcessLookupError):pass
  assert (proof or stdout).is_file(),name+': no retained result to adopt'
  if 'result_sha256' in receipt:assert sha(proof or stdout)==receipt['result_sha256'],name+': retained result changed'
  else:
   receipt=dict(receipt,status='passed',recovered_from=str(dest.relative_to(root)),recovery_reason='successful execution adopted after conductor interruption',stdout_sha256=sha(stdout),stderr_sha256=sha(stderr))
   dest=out/(name+'-resumed-process.json');assert not dest.exists(),'resumption already recorded'
  code=0
 else:
  fd1=os.open(stdout,os.O_CREAT|os.O_EXCL|os.O_WRONLY,0o600);fd2=os.open(stderr,os.O_CREAT|os.O_EXCL|os.O_WRONLY,0o600)
  started=time.time();child=subprocess.Popen(argv,cwd=root,stdout=fd1,stderr=fd2,start_new_session=True);os.close(fd1);os.close(fd2)
  stat=Path(f'/proc/{child.pid}/stat').read_text();stamp=stat[stat.rfind(')')+2:].split()[19]
  receipt=dict(stage=name,argv=argv,pid=child.pid,start_ticks=stamp,started_unix=started,status='running',binary_sha256=binary_sha)
  dest.write_text(json.dumps(receipt,indent=2)+'\n')
  code=child.wait()
  receipt.update(status='passed' if code==0 else 'failed',exit_code=code,ended_unix=time.time(),stdout_sha256=sha(stdout),stderr_sha256=sha(stderr))
  dest.write_text(json.dumps(receipt,indent=2)+'\n');assert code==0,name+': private diagnostics retained'
 result=json.loads((proof or stdout).read_text());count=len(result.get('checks',result.get('cases',[])));
 if name=='helper-image':
  guards=dict(substitute_refused=result.get('substituted_image_executed') is False,material_not_captured=result.get('fixture_material_captured') is False,exact_hmac=result.get('real_attestation_matches') is True,sealed_after_replacement=result.get('sealed_helper_after_path_replacement') is True)
  assert all(guards.values()),'helper image proof invalid';count=len(guards)
 assert count>0,name+': zero result'
 assert not result.get('binary_sha256') or result['binary_sha256']==binary_sha
 assert not result.get('source_binary_sha256') or result['source_binary_sha256']==binary_sha
 receipt['observed_results']=count;receipt['result_path']=str((proof or stdout).relative_to(root));receipt['result_sha256']=sha(proof or stdout);dest.write_text(json.dumps(receipt,indent=2)+'\n')
 for rel,digest in inputs.items():assert sha(root/rel)==digest,(name,rel)
 receipts.append(receipt);print(json.dumps(dict(stage=name,exit_code=code,observed_results=count)),flush=True)
report=dict(at=datetime.datetime.now(datetime.timezone.utc).isoformat(),status='FUNCTIONAL_CAMPAIGN_PARTIAL',binary_sha256=binary_sha,stages=receipts,input_sha256=inputs,native_kernel_qualified=False,independent_review=False,global_acceptance=False,final_retex=False,open_requirements=['R20','R11 global acceptance','R12 final RETEX'])
destination=out/a.report_name;assert destination.parent==out and not destination.exists(),'new report path required; preserve prior report'
report['functional_recipes_rerun']=not a.resume
destination.write_text(json.dumps(report,indent=2)+'\n')
