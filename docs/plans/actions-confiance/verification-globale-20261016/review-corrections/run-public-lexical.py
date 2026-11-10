from pathlib import Path
import subprocess,json,time,hashlib,os
root=Path.cwd();out=Path(__file__).parent;binary=root/'bin/swarm-actions-confiance-proof-exact';sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();baseline=json.loads((out/'build-lexical-inputs.json').read_text());assert all(sha(root/k)==v for k,v in baseline.items())
commands={
 'campaign':['python3','tests/actions_confiance_campaign.py','--binary',str(binary),'--candidate-receipt',str(out/'build-lexical-process.json'),'--build-inputs',str(out/'build-lexical-inputs.json'),'--evidence-dir',str(out/'public-campaign-lexical'),'--timeout-seconds','180'],
 'effect-proof':['python3','tests/actions_confiance_effect_proof.py','--binary',str(binary),'--evidence-dir',str(out/'public-effect-proof-lexical'),'--timeout-seconds','180','--legacy-receipt',str(out/'independent-receipt.json')],
 'evidence-browser':['node','tests/actions_confiance_evidence_ui.cjs',str(binary),str(out/'public-evidence-browser-confirmed'),'--effect-proof']}
children=[]
for name,argv in commands.items():
 f=out/(name+'-lexical-process.json');assert not f.exists();a=open(out/(name+'-lexical.stdout'),'xb');b=open(out/(name+'-lexical.stderr'),'xb');os.chmod(a.name,0o600);os.chmod(b.name,0o600);child=subprocess.Popen(argv,cwd=root,stdout=a,stderr=b,start_new_session=True);a.close();b.close();st=Path(f'/proc/{child.pid}/stat').read_text().rsplit(') ',1)[1].split()[19];d=dict(argv=argv,pid=child.pid,start_ticks=st,started_unix=time.time(),status='running',binary_sha256=sha(binary));f.write_text(json.dumps(d,indent=2)+'\n');children.append((name,child,f,d));print(name,child.pid,flush=True)
while children:
 for item in list(children):
  name,child,f,d=item;code=child.poll()
  if code is None:continue
  changed=[k for k,v in baseline.items() if sha(root/k)!=v];d.update(exit_code=code,status='passed' if code==0 and not changed else 'failed',ended_unix=time.time(),inputs_unchanged=not changed,changed_inputs=changed);f.write_text(json.dumps(d,indent=2)+'\n');children.remove(item);print(json.dumps(dict(name=name,**d)),flush=True)
 time.sleep(.5)
