"""Verify current explicit observer evidence without manufacturing global acceptance."""
import datetime,hashlib,json,re
from pathlib import Path
root=Path(__file__).resolve().parents[4];out=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
load=lambda p:json.loads(p.read_text())
def passed(name):
 raw=(out/name).read_text();assert re.search(r'\nPASS\nok\s',raw),name
 assert not re.search(r'^FAIL|^\s*--- FAIL:',raw,re.M),name
 names=re.findall(r'^\s*--- PASS: (\S+)',raw,re.M);assert names and len(names)==len(set(names));return names
names=passed('observe-race.out')+passed('observe-effects.out')+passed('observe-public-first.out')
assert len(names)==len(set(names))==297
assert sum('/'not in n for n in names)==80
for kind in ['race','effects','build','vet']:
 r=load(out/f'observe-{kind}-receipt.json');assert r['exit_code']==0 and r['changed_inputs']==[]
for kind in ['build','manifest','pilot']:
 r=load(out/f'observe-final-{kind}-receipt.json');assert r['exit_code']==0 and r['changed_inputs']==[]
passed('observe-final-manifest.out')
for entry in load(out/'observe-final-inputs-before.json'):assert sha(root/entry['path'])==entry['sha256'],entry['path']
old_changed=[e['path']for e in load(out/'observe-inputs-before.json')if sha(root/e['path'])!=e['sha256']]
assert old_changed==['web/trust-controls.json'],old_changed
binary=root/'bin/swarm-actions-confiance-observe'
initial=out/'observe-browser-first/observe-ui.json';browser=load(out/'observe-browser-final/observe-ui.json')
assert browser['binary_sha256']==sha(binary) and len(browser['checks'])==33 and browser['errors']==[]
assert browser['response_fixtures'] is True and browser['native_kernel_events'] is False
pilot=load(out/'observe-final-pilot.out');assert pilot['binary_sha256']==sha(binary) and len(pilot['cases'])==7
assert load(out/'observe-mapping-after.json')['matrix_version']==3
assert 'file digest mismatch' in (out/'observe-mapping-before.json').read_text()
assert 'OK: 12 shared' in (out/'observe-contexts.out').read_text()
assert 'PASS i18n locale' in (out/'observe-i18n-test.out').read_text()
assert (out/'observe-diff-check.out').read_bytes()==b''
public=load(out/'observe-public-unavailable.json');assert public['available'] is False and public['events']==[] and public['cleanup_verified'] is True and public['complete'] is False
proofs=['observe-race.out','observe-effects.out','observe-public-first.out','observe-public-unavailable.json','observe-cleanup-before.out','observe-client-before.out','observe-client-after.out','observe-browser-first/observe-ui.json','observe-browser-final/observe-ui.json','observe-final-pilot.out','observe-mapping-before.json','observe-mapping-after.json','observe-final-manifest.out','observe-contexts.out','observe-i18n-test.out','observe-final-i18n-check.out']
for p in out.glob('observe-*-receipt.json'):proofs.append(p.name)
paths=sorted({e['path']for e in load(out/'observe-final-inputs-before.json')if 'observe' in e['path'] or e['path'] in ['internal/engine/trust_actions.go','internal/engine/trust_actions_cli.go','internal/engine/trust_actions_http.go','internal/engine/trust_remote.go','internal/engine/main.go','internal/engine/kernel_fixed_helper_linux.go','internal/engine/kernel_backend_resources_linux.go','internal/engine/trust_kernel_check_child.go','internal/engine/trust_kernel_check_native_linux_amd64.go','docs/actions-confiance-T7.md','docs/en/actions-confiance-T7.md']})
preview=load(out/'observe-preview-process.json');assert preview['verification_status']=='verified' and preview['binary_sha256']==sha(binary) and preview['kernel_observation']['enabled'] is False
proofs+=['observe-preview-process.json','observe-preview-update.out']
result=dict(verified_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),requirement='R20',status='PARTIAL',scope='explicit I3 registered-main-thread collector; fixed helper; unavailable native environment',go_tests=dict(distinct=297,main=80,subcases=217,groups=[141,155,1],single_execution=False,named_passed=names,skipped_helpers_excluded=True),public_observer=dict(path=str((out/'observe-public-first.out').relative_to(root)),remote_cli_exit=3,http_observed=True,owned_registry_source=True,native_positive=False,business_state_unchanged=True),browser=dict(checks=33,response_fixtures=True,unexpected_diagnostics=0,binary_sha256=sha(binary)),pilot=dict(scenarios=7,browser_checks=len(load(out/'observe-final-pilot/browser/ui.json')['checks'])),binary_sha256=sha(binary),earlier_binary_sha256=load(initial)['binary_sha256'],reuse_note='Go sources and observation UI unchanged after race/public execution; final build changes only contextual matrix and architectural links. Earlier public binary path was recorded; its hash is attributed through the initial build/browser, not claimed freshly captured during the public call.',input_sha256={p:sha(root/p)for p in paths},final_input_snapshot=dict(path=str((out/'observe-final-inputs-before.json').relative_to(root)),sha256=sha(out/'observe-final-inputs-before.json')),proof_sha256={str((out/p).relative_to(root)):sha(out/p)for p in sorted(set(proofs))},harness_sha256={str(p.relative_to(root)):sha(p)for p in [Path(__file__),out/'run-observe-checks.py',out/'run-observe-final-checks.py',out/'run-observe-pilot.py',out/'refresh-observe-effects.py']},preview=dict(path=str((out/'observe-preview-process.json').relative_to(root)),sha256=sha(out/'observe-preview-process.json'),business_bytes_unchanged=True,browser_reload_observed=False),manifest_sources=1500,native_kernel_qualified=False,global_acceptance=False,independent_assessment=False,retrospective_final=False,open=['positive kernel verifier and attached probes','real foreign-thread/PID exclusion','native losses/recovery/overhead','independent review and global acceptance','final RETEX'])
(out/'observe-verification.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(dict(status='PARTIAL',go_cases=297,browser_checks=result['browser']['checks']+result['pilot']['browser_checks'],native_kernel_qualified=False)))
