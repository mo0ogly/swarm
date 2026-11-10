from pathlib import Path
import json,hashlib,subprocess,os,time
root=Path.cwd();out=Path(__file__).parent;sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
paths=set(json.loads((out/'final-inputs-before.json').read_text()))
paths.update(str(p) for p in root.relative_to(root).glob('never-match'))
paths.update(['tests/actions_confiance_effect_proof.py','tests/actions_confiance_evidence_exact_test.cjs','tests/actions_confiance_campaign.py','docs/actions-confiance-qualification.md','docs/en/actions-confiance-qualification.md','tools/verification/refresh_engine_source_manifest.py','docs/engine-cursor-source-manifest.json'])
manifest=json.loads((root/'docs/engine-cursor-source-manifest.json').read_text());paths.update(x['path'] for x in manifest['input_parts'])
inputs={k:sha(root/k) for k in sorted(paths)};snap=out/'build-final-inputs.json';assert not snap.exists();snap.write_text(json.dumps(inputs,indent=2)+'\n')
binary='bin/swarm-actions-confiance-effect-proof-qualified';argv=['sh','build.sh',binary];f=out/'build-final-process.json';assert not f.exists();log=open(out/'build-final.out','xb');proc=subprocess.Popen(argv,cwd=root,stdout=log,stderr=subprocess.STDOUT,start_new_session=True);st=Path(f'/proc/{proc.pid}/stat').read_text().rsplit(') ',1)[1].split()[19]
d=dict(argv=argv,pid=proc.pid,start_ticks=st,started_unix=time.time(),status='running');f.write_text(json.dumps(d,indent=2)+'\n');print(proc.pid,flush=True)
code=proc.wait();changed=[k for k,v in inputs.items() if sha(root/k)!=v];d.update(exit_code=code,status='passed' if code==0 and not changed else 'failed',ended_unix=time.time(),inputs_unchanged=not changed,changed_inputs=changed,binary=binary,binary_sha256=sha(root/binary) if code==0 else None);f.write_text(json.dumps(d,indent=2)+'\n');print(json.dumps(d));assert d['status']=='passed'
