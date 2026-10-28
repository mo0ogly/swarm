"""Qualify the bounded backend checks without claiming native R20 success."""
import datetime
import hashlib
import json
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parents[4]
OUT=Path(__file__).resolve().parent
def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()
def load(name):
    return json.loads((OUT/name).read_text())

receipts=['backend-race-final.json','backend-build.json','backend-vet.json',
          'backend-modules.json','backend-contexts.json','backend-mapping.json',
          'backend-diff.json','backend-manifest.json']
for name in receipts:
    v=load(name)
    assert v['exit_code']==0,name
    for path,digest in v['inputs'].items():
        assert sha(ROOT/path)==digest,(name,path)
raw=(OUT/'backend-race-final.out').read_text()
names=re.findall(r'^\s*--- PASS: (\S+)',raw,re.M)
assert len(names)==59 and len(set(names))==59
assert sum('/' not in n for n in names)==15
assert 'TestKernelBackendNativeAvailability' in names
assert load('backend-manifest.json')['named_passed']==['TestGraphDeliveryD03QualificationManifest']
native=load('backend-native-availability.json')
assert native['available'] is False and native['permission_denied'] is True
assert native['native_event_coverage_verified'] is False
assert native['public_cli_exit_code_observed'] is False
assert 'kernel backend ring:' in native['error']
assert 'undefined: decodeKernelBackendRecord' in (OUT/'backend-before.out').read_text()
assert load('backend-race-final.json')['sources_unchanged'] is True
paths=[*ROOT.glob('internal/engine/kernel_backend*.go'),*ROOT.glob('internal/engine/kernel_observation*.go'),ROOT/'go.mod',ROOT/'go.sum',ROOT/'docs/actions-confiance-T7.md',ROOT/'docs/en/actions-confiance-T7.md',ROOT/'docs/architecture/actions-confiance-contract.json']
v=dict(verified_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
       requirement='R20',status='PARTIAL',owner='Codex',
       scope='internal fixed Linux amd64 backend; registered main thread only',
       tests=dict(cases=59,main=15,subcases=44,named_passed=names,
                  native_diagnostic='permission denial before attachment',
                  kernel_event_coverage=False,previous_37_ingress_cases_included=True),
       checks=receipts,native=native,independent_review=False,
       runtime_qualified=False,global_acceptance=False,
       source_sha256={str(p.relative_to(ROOT)):sha(p) for p in paths},
       proofs={str(p.relative_to(ROOT)):sha(p) for p in OUT.iterdir()
               if p.is_file() and p.name!='backend-verification.json'},
       remaining=['fixed separate launcher and Unix integration',
                  'persisted Admin/API/CLI policy before activation',
                  'public unavailable result/code3',
                  'kernel verifier, foreign exclusion, loss, cleanup, restart and overhead recipe'],
       existing_preview_restarted=False,linux_privileges_changed=False)
(OUT/'backend-verification.json').write_text(json.dumps(v,indent=2)+'\n')
print(json.dumps(dict(status=v['status'],cases=59,native_available=False)))
