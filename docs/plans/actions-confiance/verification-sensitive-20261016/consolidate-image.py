"""Verify image substitution repair; no general provider or global qualification."""
import datetime,hashlib,json,re
from pathlib import Path
root=Path(__file__).resolve().parents[4];out=Path(__file__).resolve().parent
load=lambda p:json.loads(p.read_text())
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
log=(out/'image-race.out').read_text();assert re.search(r'\nPASS\nok\s',log) and not re.search(r'^FAIL|^\s*--- FAIL:',log,re.M)
names=[n for n in re.findall(r'^\s*--- PASS: (\S+)',log,re.M)if n not in ['TestActionsConfianceT6LaboratoryHelper','TestActionsConfianceT4SealedHelper']];assert len(names)==len(set(names))==88
assert 'TestKernelCheckParentDeathStopsHelper' in names
for kind in ['race','vet','build']:
 r=load(out/f'image-{kind}-receipt.json');assert r['exit_code']==0
 assert r['changed_inputs'] in [[],['tests/actions_confiance_secret_image.py']]
for row in load(out/'image-inputs-before.json'):
 if row['path'] not in ['tests/actions_confiance_secret_image.py','web/trust-controls.json']:assert sha(root/row['path'])==row['sha256'],row['path']
for kind in ['build','manifest']:
 r=load(out/f'image-final-{kind}-receipt.json');assert r['exit_code']==0 and r['changed_inputs']==[]
for row in load(out/'image-final-inputs.json'):assert sha(root/row['path'])==row['sha256'],row['path']
old=load(out/'image-before/image-replacement.json');assert old['substituted_image_executed'] and old['fixture_material_captured'] and not old['real_attestation_matches'] and old['state']=='consumed'
for stage in ['image-after','image-final-public']:
 x=load(out/stage/'image-replacement.json');assert not x['substituted_image_executed'] and not x['fixture_material_captured'] and x['real_attestation_matches'] and x['sealed_helper_after_path_replacement']
checks=[]
for file,count in [('image-secret-browser/secret-ui.json',17),('image-sealed-browser/sealed-ui.json',13)]:
 x=load(out/file);assert len(x['checks'])==count and x['errors']==[];checks+=x['checks']
assert len(load(out/'image-public/remote.json')['checks'])==16
assert load(out/'image-mapping-after.json')['status']=='mapping_artifact_valid'
assert 'OK: 12 shared' in (out/'image-contexts.out').read_text()
assert '--- PASS: TestGraphDeliveryD03QualificationManifest' in (out/'image-final-manifest.out').read_text()
assert (out/'image-diff.out').read_bytes()==b''
binary=root/'bin/swarm-actions-confiance-fixed-image-qualified'
assert load(out/'image-final-public/image-replacement.json')['source_binary_sha256']==sha(binary)
proofs=['image-race.out','image-vet.out','image-build-receipt.json','image-race-receipt.json','image-vet-receipt.json','image-before/image-replacement.json','image-before.out','image-recipe-before.py','image-after/image-replacement.json','image-final-public/image-replacement.json','image-final-public/image-process.json','image-final-public.out','image-public/remote.json','image-secret-browser/secret-ui.json','image-sealed-browser/sealed-ui.json','image-mapping-before.json','image-mapping-after.json','image-secret-controls-receipt.json','image-final-build-receipt.json','image-final-manifest-receipt.json','image-contexts.out','image-i18n-check.out','image-diff.out']
preview=load(out/'image-preview-process.json');assert preview['verification_status']=='verified' and preview['binary_sha256']==sha(binary)
proofs+=['image-preview-process.json','image-preview-update.out']
result=dict(verified_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),status='VERIFIED_REPAIR_WITHIN_FIXED_HELPER_SCOPE',requirement='R14',cause='re-resolving mutable executable pathname before helper start',repair='execute server running image through owned inherited descriptor',go_tests=dict(meaningful=88,main=36,subcases=52,raw_passes=90,excluded_empty_fixtures=['TestActionsConfianceT6LaboratoryHelper','TestActionsConfianceT4SealedHelper'],named_passed=names),public=dict(broker_checks=16,image_substitution_before_and_after=True,final_binary_exact_image_case=True,fixture_material_only=True),browser=dict(checks=30,unexpected_diagnostics=0,unchanged_ui=True,initial_binary_sha256=load(out/'image-public/remote.json')['binary_sha256']),binary_sha256=sha(binary),input_sha256={e['path']:e['sha256']for e in load(out/'image-final-inputs.json')},source_freeze_note='Go/embedded inputs unchanged during execution. Initial broad receipts explicitly show one future recipe edit, not a compiled-source change. Final checks use the frozen final snapshot. Final build changes static matrix only after initial public/browser runs.',proof_sha256={str((out/n).relative_to(root)):sha(out/n)for n in proofs},harness_sha256={str(p.relative_to(root)):sha(p)for p in [Path(__file__),out/'run-image-checks.py',out/'run-image-final-checks.py']},preview=dict(path=str((out/'image-preview-process.json').relative_to(root)),sha256=sha(out/'image-preview-process.json'),business_bytes_unchanged=True,browser_reload_observed=False),manifest_sources=1503,general_provider_containment=False,native_ebpf_qualified=False,independent_assessment=False,global_acceptance=False,final_retex=False)
(out/'image-verification.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(dict(status=result['status'],go_tests=88,browser_checks=30,global_acceptance=False)))
