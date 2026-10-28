"""Bounded checks for the internal Unix receiver, never kernel qualification."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

inputs = {str(p.relative_to(ROOT)): sha(p) for p in
          [*ROOT.glob('internal/engine/*.go'), *ROOT.glob('*.go'), ROOT/'go.mod', ROOT/'go.sum']}
env = dict(os.environ, CGO_ENABLED='0')
stages = [
    ('build', ['sh', 'build.sh', 'bin/swarm-actions-confiance-observation-contract']),
    ('vet', ['go', 'vet', './internal/engine']),
    ('contexts', ['python3', 'tools/agent-workflows/check.py']),
    ('mapping-integrity', ['python3', 'tools/actions-confiance/controls.py', '--root', str(ROOT)]),
    ('diff', ['git', 'diff', '--check']),
]
for name, command in stages:
    start = time.time()
    log = OUT / ('ingress-' + name + '.out')
    with log.open('wb') as handle:
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=handle,
                                stderr=subprocess.STDOUT, timeout=600)
    unchanged = all(sha(ROOT / p) == digest for p, digest in inputs.items())
    receipt = dict(command=command, started_unix=start, ended_unix=time.time(),
                   exit_code=result.returncode, inputs=inputs,
                   sources_unchanged=unchanged, output_sha256=sha(log))
    if name == 'build' and result.returncode == 0:
        receipt['binary_sha256'] = sha(ROOT/'bin/swarm-actions-confiance-observation-contract')
    (OUT / ('ingress-' + name + '.json')).write_text(json.dumps(receipt, indent=2)+'\n')
    print(json.dumps(dict(stage=name, exit_code=result.returncode, unchanged=unchanged)), flush=True)
    if result.returncode or not unchanged:
        raise SystemExit(1)
