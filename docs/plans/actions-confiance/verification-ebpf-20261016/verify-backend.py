"""Static/build checks for the fixed backend, not native R20 acceptance."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

paths=[*ROOT.glob('internal/engine/*.go'),*ROOT.glob('*.go'),ROOT/'go.mod',ROOT/'go.sum',ROOT/'build.sh']
resources=json.loads(subprocess.check_output(['go','list','-json','.'],cwd=ROOT,text=True))
paths.extend(ROOT/p for p in resources.get('EmbedFiles',[]))
inputs={str(p.relative_to(ROOT)):sha(p) for p in paths}
stages=[
    ('build',['sh','build.sh','bin/swarm-actions-confiance-kernel-backend']),
    ('vet',['go','vet','./internal/engine']),
    ('modules',['go','mod','verify']),
    ('contexts',['python3','tools/agent-workflows/check.py']),
    ('mapping',['python3','tools/actions-confiance/controls.py','--root',str(ROOT)]),
    ('diff',['git','diff','--check']),
]
for name,cmd in stages:
    start=time.time();log=OUT/('backend-'+name+'.out')
    with log.open('wb') as f:
        r=subprocess.run(cmd,cwd=ROOT,env=dict(os.environ,CGO_ENABLED='0'),stdout=f,stderr=subprocess.STDOUT,timeout=600)
    v=dict(command=cmd,started_unix=start,ended_unix=time.time(),exit_code=r.returncode,
           inputs=inputs,sources_unchanged=all(sha(ROOT/p)==h for p,h in inputs.items()),output_sha256=sha(log))
    if name=='build' and r.returncode==0:
        v['binary_sha256']=sha(ROOT/'bin/swarm-actions-confiance-kernel-backend')
    (OUT/('backend-'+name+'.json')).write_text(json.dumps(v,indent=2)+'\n')
    print(json.dumps(dict(stage=name,exit_code=r.returncode,sources_unchanged=v['sources_unchanged'])),flush=True)
    if r.returncode or not v['sources_unchanged']:
        raise SystemExit(1)
