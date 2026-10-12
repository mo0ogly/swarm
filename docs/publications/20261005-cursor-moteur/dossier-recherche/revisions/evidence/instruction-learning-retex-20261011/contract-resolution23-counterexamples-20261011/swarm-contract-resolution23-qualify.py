from pathlib import Path
from datetime import datetime, timezone
import hashlib
import json
import os
import subprocess
import time

receipt_path = Path('/tmp/swarm-contract-resolution23-workspace.json')
workspace_receipt = json.loads(receipt_path.read_text())
root = Path(workspace_receipt['workspace'])
manifest_path = root / 'docs/engine-cursor-source-manifest.json'
manifest = json.loads(manifest_path.read_text())
inputs = {}
for part in manifest['input_parts']:
    path = root / part['path']
    assert hashlib.sha256(path.read_bytes()).hexdigest() == part['sha256']
    for row in json.loads(path.read_text()):
        inputs[row['path']] = row['sha256']
    inputs[part['path']] = part['sha256']
inputs['docs/engine-cursor-source-manifest.json'] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
for row in workspace_receipt['method_support_inputs']:
    inputs[row['path']] = hashlib.sha256((root / row['path']).read_bytes()).hexdigest()
excluded = {'.git','.swarm','node_modules','bin','__pycache__'}
links = []
for path in sorted(root.rglob('*')):
    relative = path.relative_to(root)
    if any(part in excluded for part in relative.parts):
        continue
    if path.is_symlink():
        links.append({'path':relative.as_posix(),'target':os.readlink(path)})
    elif path.is_file():
        inputs[relative.as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
frozen = {'workspace':str(root),'candidate_digest':manifest['candidate_digest'],'manifest_sha256':inputs['docs/engine-cursor-source-manifest.json'],'inputs':inputs,'canonical_links':links,'construction':'Complete qualified parent workspace, current candidate changes preserved, all build/test/method support frozen.'}
Path('/tmp/swarm-contract-resolution23-frozen.json').write_text(json.dumps(frozen,indent=2)+'\n')

def drift():
    bad = []
    for name, digest in inputs.items():
        path = root / name
        if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            bad.append(name)
    for row in frozen['canonical_links']:
        path = root / row['path']
        if not path.is_symlink() or os.readlink(path) != row['target']:
            bad.append(row['path'])
    return bad

env = os.environ.copy()
env['GOTMPDIR'] = '/home/fpizzi/.cache/swarm-operator-replan-20261011/qualification-tmp'
env['TMPDIR'] = '/tmp'
commands = [
    ('full', ['go','test','./...','-count=1','-timeout=12m']),
    ('vet', ['go','vet','./...']),
    ('methods', ['python3','tools/agent-workflows/check.py']),
    ('build', ['go','build','-o','/tmp/swarm-qualified23-build','./cmd/swarm']),
]
all_results = []
for name, argv in commands:
    assert not drift(), drift()
    started = datetime.now(timezone.utc).isoformat()
    tick = time.monotonic()
    output_path = Path('/tmp/swarm-contract-resolution23-'+name+'.out')
    result_path = Path('/tmp/swarm-contract-resolution23-'+name+'-receipt.json')
    result_path.write_text(json.dumps({'state':'running','started':started,'argv':argv,'workspace':str(root),'candidate_digest':manifest['candidate_digest'],'frozen_paths':len(inputs)},indent=2)+'\n')
    with output_path.open('wb') as stream:
        run = subprocess.run(argv,cwd=root,env=env,stdout=stream,stderr=subprocess.STDOUT,timeout=960)
    record = {'state':'finished','started':started,'finished':datetime.now(timezone.utc).isoformat(),'argv':argv,'workspace':str(root),'candidate_digest':manifest['candidate_digest'],'manifest_sha256':frozen['manifest_sha256'],'frozen_paths':len(inputs),'wall_seconds':time.monotonic()-tick,'exit':run.returncode,'output':str(output_path),'output_sha256':hashlib.sha256(output_path.read_bytes()).hexdigest(),'drift':drift(),'limits':'Isolated source/build/method inputs, no native database or real model call. Not native acceptance.'}
    if name == 'build' and run.returncode == 0:
        record['binary_sha256'] = hashlib.sha256(Path('/tmp/swarm-qualified23-build').read_bytes()).hexdigest()
    result_path.write_text(json.dumps(record,indent=2)+'\n')
    all_results.append(record)
    print(json.dumps({'step':name,'exit':run.returncode,'wall_seconds':record['wall_seconds'],'drift':record['drift']},ensure_ascii=False),flush=True)
    if run.returncode != 0 or record['drift']:
        raise SystemExit(1)
Path('/tmp/swarm-contract-resolution23-qualified-receipt.json').write_text(json.dumps({'at':datetime.now(timezone.utc).isoformat(),'workspace':str(root),'candidate_digest':manifest['candidate_digest'],'checks':all_results,'result':'PASS local qualification','limits':'Primary/runtime unchanged; public installation, reception, decision and effect remain required.'},indent=2)+'\n')
