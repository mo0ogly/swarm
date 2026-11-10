from pathlib import Path
from datetime import datetime, timezone
import json, hashlib, subprocess, os, time
r=json.loads(Path('/tmp/swarm-T2-own-scope-budget-workspace.json').read_text());root=Path(r['workspace'])
argv=['go','test','./internal/engine','-run','^TestPlanningScopeOwnActivationBudgetDiscriminant$','-count=1','-timeout=2m','-v']
start=datetime.now(timezone.utc).isoformat();tick=time.monotonic();env=os.environ.copy();env['GOTMPDIR']='/home/fpizzi/.cache/swarm-operator-replan-20261011/qualification-tmp';env['TMPDIR']='/tmp'
out=Path('/tmp/swarm-T2-own-scope-budget.out')
with out.open('wb') as stream: result=subprocess.run(argv,cwd=root,env=env,stdout=stream,stderr=subprocess.STDOUT,timeout=150)
drift=[k for k,v in r['original_go_inputs'].items() if hashlib.sha256((root/k).read_bytes()).hexdigest()!=v]
receipt={'argv':argv,'started':start,'finished':datetime.now(timezone.utc).isoformat(),'exit':result.returncode,'wall_seconds':time.monotonic()-tick,'workspace':str(root),'original_go_sources':len(r['original_go_inputs']),'original_go_drift':drift,'output_sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'extra_harness_sha256':hashlib.sha256((root/'internal/engine/planning_scope_activation_discriminant_test.go').read_bytes()).hexdigest(),'limits':'Local public planning handler fixture, no real model, native database or candidate edit. Extra harness separate from historical full suite.'}
Path('/tmp/swarm-T2-own-scope-budget-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt));raise SystemExit(result.returncode or bool(drift))
