"""Bind the receiver checks to their actual inputs; R20 remains partial."""
import datetime
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def load(name):
    return json.loads((OUT/name).read_text())

receipts = ['ingress-race-final.json', 'ingress-build.json', 'ingress-vet.json',
            'ingress-contexts.json', 'ingress-mapping-integrity.json',
            'ingress-diff.json', 'ingress-manifest.json']
for name in receipts:
    value = load(name)
    assert value['exit_code'] == 0, name
    for path, digest in value['inputs'].items():
        assert sha(ROOT/path) == digest, (name, path)
raw = (OUT/'ingress-race-final.out').read_text()
names = re.findall(r'^\s*--- PASS: (\S+)', raw, re.M)
assert len(names) == 37 and len(set(names)) == 37
assert len([n for n in names if '/' not in n]) == 9
assert '--- SKIP: TestKernelObservationPeerHelper' in raw
assert load('ingress-manifest.json')['named_passed'] == ['TestGraphDeliveryD03QualificationManifest']
assert 'FAIL' in (OUT/'ingress-agent-before.out').read_text()
assert 'undefined: kernelObservationLease' in (OUT/'ingress-before.out').read_text()
assert load('map-capability-result.json')['available'] is False
inputs = [*ROOT.glob('internal/engine/kernel_observation*.go'),
          ROOT/'docs/actions-confiance-T7.md', ROOT/'docs/en/actions-confiance-T7.md',
          ROOT/'docs/architecture/actions-confiance-contract.json',
          ROOT/'tools/verification/refresh_engine_source_manifest.py']
proofs = {str(p.relative_to(ROOT)): sha(p) for p in OUT.iterdir()
          if p.is_file() and p.name != 'ingress-verification.json'}
result = dict(
    verified_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
    scope='internal kernel-observation-v1 Unix receiver',
    requirement='R20', status='PARTIAL', owner='Codex',
    local_receiver_verified=True, native_kernel_collection=False,
    runtime_qualified=False, independent_review=False, globally_accepted=False,
    tests=dict(meaningful_cases=37, main=9, subcases=28, named_passed=names,
               helper_parent_skipped=True, source='owned Unix subprocess fixtures'),
    checks=receipts, source_sha256={str(p.relative_to(ROOT)):sha(p) for p in inputs},
    proofs=proofs,
    limitations=['No live launcher integration or collector backend',
                 'SO_PEERCRED identifies the connector, not kernel origin',
                 'EPERM remains observed; no privilege change attempted',
                 'Kernel filter, cleanup, overhead and native recipe remain open',
                 'Existing R19 preview was not replaced'],
    next='Fixed owned launcher/backend, fresh source integration and public unavailable contract; then native qualification and RETEX')
(OUT/'ingress-verification.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps({k:result[k] for k in ['status','local_receiver_verified','native_kernel_collection']}))
