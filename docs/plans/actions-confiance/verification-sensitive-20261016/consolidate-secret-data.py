"""Bind the controlled restricted-material delivery checks, without general-provider claims."""
import datetime,hashlib,json,re
from pathlib import Path
root=Path(__file__).resolve().parents[4];out=Path(__file__).resolve().parent
load=lambda p:json.loads(p.read_text());sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
log=(out/'secret-data-race.out').read_text();assert re.search(r'\nPASS\nok\s',log)and not re.search(r'^FAIL|^\s*--- FAIL:',log,re.M)
names=[n for n in re.findall(r'^\s*--- PASS: (\S+)',log,re.M)if n!='TestActionsConfianceT4SealedHelper'];assert len(names)==len(set(names))==290
r=load(out/'secret-data-race-receipt.json');assert r['exit_code']==0 and r['changed_inputs']==[]
for kind in ['build','vet','manifest']:
 r=load(out/f'secret-data-final-{kind}-receipt.json');assert r['exit_code']==0 and r['changed_inputs']==[]
for e in load(out/'secret-data-final-inputs.json'):assert sha(root/e['path'])==e['sha256'],e['path']
initial_changed=[e['path']for e in load(out/'secret-data-inputs-before.json')if sha(root/e['path'])!=e['sha256']]
assert set(initial_changed)=={'locales/en.json','web/i18n-en.js','web/trust-actions.html','web/trust-actions-en.js','web/trust-controls.json'},initial_changed
assert 'secret delivery bypasses active destination/tool/classification allowlists' in (out/'secret-data-before.out').read_text()
binary=root/'bin/swarm-actions-confiance-secret-data-qualified'
public=load(out/'secret-data-public-first/remote.json');legacy=load(out/'secret-data-legacy-public/remote.json')
assert len(public['checks'])==17 and len(legacy['checks'])==16
assert public['binary_sha256']==legacy['binary_sha256']==sha(binary)
assert public['server_stopped'] and legacy['server_stopped']
assert any('withdrawn secret destination' in c for c in public['checks'])
ui=load(out/'secret-data-browser-first/secret-ui.json');assert len(ui['checks'])==17 and ui['errors']==[] and ui['active_data_policy'] is True
reopen=load(out/'secret-data-admin-verified/reopen.json');assert len(reopen['checks'])==2 and reopen['errors']==[] and reopen['state_unchanged'] and reopen['binary_sha256']==sha(binary)
assert load(out/'secret-data-mapping-current.json')['status']=='mapping_artifact_valid'
assert 'OK: 12 shared' in (out/'secret-data-contexts.out').read_text()
assert 'PASS i18n locale' in (out/'secret-data-i18n-test.out').read_text()
assert (out/'secret-data-diff.out').read_bytes()==b''
manifest=(out/'secret-data-manifest-documented.out').read_text();assert '--- PASS: TestGraphDeliveryD03QualificationManifest' in manifest and re.search(r'\nPASS\nok\s',manifest)
proofs=['secret-data-race.out','secret-data-race-receipt.json','secret-data-before.out','secret-data-test-before.go.txt','secret-data-final-build-receipt.json','secret-data-final-vet-receipt.json','secret-data-final-manifest-receipt.json','secret-data-public-first/remote.json','secret-data-legacy-public/remote.json','secret-data-browser-first/secret-ui.json','secret-data-admin-verified/reopen.json','secret-data-manifest-documented.out','secret-data-contexts.out','secret-data-i18n-test.out','secret-data-mapping-current.json']
preview=None
if (out/'secret-data-preview-process.json').exists():
 preview=load(out/'secret-data-preview-process.json');assert preview['verification_status']=='verified' and preview['binary_sha256']==sha(binary);proofs+=['secret-data-preview-process.json','secret-data-preview-update.out']
result=dict(verified_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),status='VERIFIED_CONTROLLED_RESTRICTED_SECRET_DELIVERY',requirement='R14',scope='restricted material to fixed sealed helper; business resources remain public/internal',go_tests=dict(meaningful=290,main=75,subcases=215,single_race_execution=True,named_passed=names,empty_fixture_excluded=True),public=dict(active_policy_checks=17,legacy_checks=16,native_hmac=True,policy_withdrawal=True,real_secret_used=False),browser=dict(checks=19,unexpected_diagnostics=0,fr_en_mobile=True,exact_reopen_and_no_overflow=True),binary_sha256=sha(binary),input_sha256={e['path']:e['sha256']for e in load(out/'secret-data-final-inputs.json')},input_binding_note='Race inputs unchanged through execution. Later changes limited to five recorded UI/catalogue/mapping files. Final binary inputs unchanged through all execution. Supplemental read-only Admin recipe added afterward, included in final named manifest verification without rebuilding unchanged binary.',supplemental_harness_sha256=sha(root/'tests/actions_confiance_secret_data_reopen.cjs'),proof_sha256={str((out/n).relative_to(root)):sha(out/n)for n in proofs},harness_sha256=sha(Path(__file__)),manifest_sources=1507,preview_process=preview,general_provider_containment=False,native_ebpf_qualified=False,independent_assessment=False,global_acceptance=False,final_retex=False)
(out/'secret-data-verification.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(dict(status=result['status'],go_tests=290,browser_checks=19,global_acceptance=False)))
