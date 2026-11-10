"""One executable for existing public recipes. No global acceptance verdict."""
import argparse,datetime,hashlib,json,os,signal,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--timeout-seconds',type=int,required=True);a=p.parse_args();assert a.timeout_seconds>0
root=Path(__file__).resolve().parents[4];out=Path(__file__).resolve().parent;binary=a.binary.resolve()
sha=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
binary_sha=sha(binary);stages=[]
def command(name,argv,proof):stages.append((name,argv,proof))
common=['--binary',str(binary),'--timeout-seconds',str(a.timeout_seconds)]
for name,script,extra,report in [
 ('connectors','actions_confiance_connectors.py',[],'connector-public.json'),
 ('evidence','actions_confiance_evidence.py',['--archive','--checkpoint'],'evidence-public.json'),
 ('laboratory','actions_confiance_laboratory.py',[],'laboratory-public.json'),
 ('secrets','actions_confiance_remote.py',['--secret-broker'],'remote.json')]:
 command(name,['python3','tests/'+script,*common,'--evidence-dir',str(out/name),*extra],out/name/report)
for name,script in [('pilot','actions_confiance_t2.py'),('processes','actions_confiance_t5.py')]:
 command(name,['python3','tests/'+script,*common,'--criterion','0','--root',str(root),'--evidence-dir',str(out/name)],None)
command('controls',['python3','tests/actions_confiance_controls.py','--root',str(root),'--evidence-dir',str(out/'controls'),'--timeout-seconds',str(a.timeout_seconds)],out/'controls/mapping-public.json')
inputs_path=root/'docs/plans/actions-confiance/verification-ebpf-20261016/attempt-source-frozen-inputs.json'
inputs=json.loads(inputs_path.read_text())
for extra in [*root.glob('*.go'),*root.glob('cmd/swarm/*.go')]:inputs[str(extra.relative_to(root))]=sha(extra)
for rel,digest in inputs.items():assert sha(root/rel)==digest,rel
receipts=[]
for name,argv,proof in stages:
 assert sha(binary)==binary_sha
 stdout=out/(name+'.stdout');stderr=out/(name+'.stderr')
 fd1=os.open(stdout,os.O_CREAT|os.O_TRUNC|os.O_WRONLY,0o600);fd2=os.open(stderr,os.O_CREAT|os.O_TRUNC|os.O_WRONLY,0o600)
 started=time.time();child=subprocess.Popen(argv,cwd=root,stdout=fd1,stderr=fd2,start_new_session=True);os.close(fd1);os.close(fd2)
 stat=Path(f'/proc/{child.pid}/stat').read_text();stamp=stat[stat.rfind(')')+2:].split()[19]
 receipt=dict(stage=name,argv=argv,pid=child.pid,start_ticks=stamp,started_unix=started,status='running',binary_sha256=binary_sha)
 dest=out/(name+'-process.json');dest.write_text(json.dumps(receipt,indent=2)+'\n')
 code=child.wait()  # Each existing recipe owns its declared watchdog and cleanup.
 receipt.update(status='watchdog_failed',exit_code=child.returncode,ended_unix=time.time());dest.write_text(json.dumps(receipt,indent=2)+'\n');raise
 receipt.update(status='passed' if code==0 else 'failed',exit_code=code,ended_unix=time.time(),stdout_sha256=sha(stdout),stderr_sha256=sha(stderr))
 dest.write_text(json.dumps(receipt,indent=2)+'\n');assert code==0,name+': private diagnostics retained'
 result=json.loads((proof or stdout).read_text());count=len(result.get('checks',result.get('cases',[])));assert count>0,name+': zero result'
 assert not result.get('binary_sha256') or result['binary_sha256']==binary_sha
 receipt['observed_results']=count;receipt['result_path']=str((proof or stdout).relative_to(root));receipt['result_sha256']=sha(proof or stdout);dest.write_text(json.dumps(receipt,indent=2)+'\n')
 for rel,digest in inputs.items():assert sha(root/rel)==digest,(name,rel)
 receipts.append(receipt);print(json.dumps(dict(stage=name,exit_code=code,observed_results=count)),flush=True)
report=dict(at=datetime.datetime.now(datetime.timezone.utc).isoformat(),status='FUNCTIONAL_CAMPAIGN_PARTIAL',binary_sha256=binary_sha,stages=receipts,input_sha256=inputs,native_kernel_qualified=False,independent_review=False,global_acceptance=False,final_retex=False,open_requirements=['R20','R11 global acceptance','R12 final RETEX'])
(out/'public-campaign.json').write_text(json.dumps(report,indent=2)+'\n')
