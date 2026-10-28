"""Internal source binding and one-binary functional campaign, not R20 acceptance."""
import datetime,hashlib,json,re
from pathlib import Path
root=Path(__file__).resolve().parents[4];out=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
load=lambda p:json.loads(p.read_text())
raw=(out/'attempt-source-race.out').read_text();assert re.search(r'\nPASS\nok\s',raw) and not re.search(r'^FAIL|^\s*--- FAIL:',raw,re.M)
names=re.findall(r'^\s*--- PASS: (\S+)',raw,re.M);assert len(names)==75 and len(set(names))==75
assert sum('/'not in n for n in names)==14
new=[n for n in names if n.startswith('TestKernelAttempt')];assert len(new)==38
assert 'TestKernelAttemptSourceActualSupervisor' in new
frozen=load(out/'attempt-source-frozen-inputs.json')
for path,digest in frozen.items():assert sha(root/path)==digest,path
for name in ['attempt-source-build.out','attempt-source-vet.out','attempt-source-diff.out']:assert (out/name).read_bytes()==b'',name
assert 'all modules verified' in (out/'attempt-source-modules.out').read_text()
assert 'OK: 12 shared' in (out/'attempt-source-contexts.out').read_text()
assert load(out/'attempt-source-mapping.out')['status']=='mapping_artifact_valid'
assert load(out/'attempt-source-contract.out')['requirements']==20
m=(out/'attempt-source-manifest-race.out').read_text();assert '--- PASS: TestGraphDeliveryD03QualificationManifest' in m and re.search(r'\nPASS\nok\s',m)
campaign=root/'docs/plans/actions-confiance/verification-globale-20261016';v=load(campaign/'public-campaign.json');binary=root/'bin/swarm-actions-confiance-attempt-source'
assert v['binary_sha256']==sha(binary) and len(v['stages'])==7 and v['global_acceptance'] is False
for stage in v['stages']:
 assert stage['exit_code']==0 and stage['status']=='passed' and stage['observed_results']>0
 assert sha(root/stage['result_path'])==stage['result_sha256']
 for rel,digest in v['input_sha256'].items():assert sha(root/rel)==digest,rel
recipes={str(p.relative_to(root)):sha(p) for p in (root/'tests').glob('actions_confiance_*') if p.is_file() and p.suffix in ('.py','.cjs')}
paths=['internal/engine/kernel_attempt_source_linux.go','internal/engine/kernel_attempt_source_linux_test.go','internal/engine/kernel_observation_linux.go','internal/engine/kernel_observation_transport_linux.go','docs/actions-confiance-T7.md','docs/en/actions-confiance-T7.md','docs/architecture/actions-confiance-contract.json']
resume=load(campaign/'public-campaign-resume-validation.json');assert resume['functional_recipes_rerun'] is False and len(resume['stages'])==7
result=dict(verified_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),requirement='R20',status='PARTIAL',scope='internal registry-derived batch source and receiver factory; not automatically attached',go_tests=dict(distinct=75,main=14,subcases=61,new_attempt_source_cases=38,existing_ingress_cases_replayed=37,named_passed=names),input_sha256={p:sha(root/p) for p in paths},compiled_inputs_sha256=frozen,recipe_current_sha256=recipes,recipe_binding_note='Existing recipes were not edited during execution; current digests captured after execution, not represented as a before/after recipe snapshot.',binary_sha256=sha(binary),public_campaign=dict(path=str((campaign/'public-campaign.json').relative_to(root)),sha256=sha(campaign/'public-campaign.json'),status=v['status'],stages=[dict(stage=s['stage'],observed_results=s['observed_results']) for s in v['stages']]),conductor_sources={str(p.relative_to(root)):sha(p) for p in [campaign/'run-public-before.py',campaign/'run-public-resumed.py',campaign/'run-public.py',Path(__file__)]},resume_validation=dict(path=str((campaign/'public-campaign-resume-validation.json').relative_to(root)),sha256=sha(campaign/'public-campaign-resume-validation.json'),functional_recipes_rerun=False),independent_review=False,native_kernel_qualified=False,global_acceptance=False,preview_restarted=False,linux_privileges_changed=False,remaining=['collector launcher and persisted opt-in policy must consume registry factory','positive real-kernel collection and exclusion/loss/restart/overhead','global independent review and final RETEX'],proofs={str(p.relative_to(root)):sha(p) for p in out.glob('attempt-source*') if p.is_file() and p.name not in ('attempt-source-verification.json','attempt-source-consolidation.out')})
(out/'attempt-source-verification.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(dict(status='PARTIAL',go_cases=75,new_source_cases=38,campaign_stages=7,native_kernel_qualified=False)))
