import datetime,hashlib,json,subprocess
from pathlib import Path
root=Path.cwd();base=root/'docs/plans/actions-confiance/verification-globale-20261016';out=base/'candidate-current'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
campaign=json.loads((out/'public-campaign.json').read_text());native=json.loads((out/'current-contracts-verification.json').read_text());browser=json.loads((out/'identity-browser-current.json').read_text())
assert all(r['exit_code']==0 for r in campaign['stages'])
assert native['distinct_cases']==407 and browser['inputs_unchanged']
inputs=campaign['input_sha256'];assert all(sha(root/p)==v for p,v in inputs.items())
assert sha(root/'bin/swarm-actions-confiance-secret-data-qualified')==campaign['binary_sha256']
selected=[
out/'public-campaign.json',out/'campaign-inputs.json',out/'current-contracts-verification.json',out/'current-contracts-process.json',out/'current-contracts-race.jsonl',out/'identity-browser-current.json',out/'final-manifest.jsonl',
root/'docs/plans/actions-confiance/verification-sensitive-20261016/secret-data-verification.json',
root/'docs/plans/actions-confiance/verification-ebpf-20261016/observe-verification.json',
root/'docs/plans/actions-confiance/verification-sensitive-20261016/image-verification.json',
base/'kernel-positive-launch.md',base/'run-kernel-positive.py',base/'kernel-positive-prepared/prepared.json',base/'kernel-positive-prepared/preparation-verified.json',base/'kernel-positive-prepared/unchanged-environment-guard.json',
root/'docs/architecture/actions-confiance-contract.json',root/'docs/actions-confiance-qualification.md',root/'docs/en/actions-confiance-qualification.md',root/'docs/engine-cursor-source-manifest.json',
root/'tools/verification/refresh_engine_source_manifest.py',Path(__file__).resolve()]
for stage in campaign['stages']:selected.append(root/stage['result_path'])
for stage in browser['stages']:
 path=(root/Path(stage['argv'][3])).resolve()
 selected.extend(path.glob('*ui.json'));selected.append(path/'process.json')
proofs={str(p.relative_to(root)):sha(p) for p in selected}
requirements=json.loads((root/'docs/architecture/actions-confiance-contract.json').read_text())['requirements']
manifest_events=[json.loads(line) for line in (out/'final-manifest.jsonl').read_text().splitlines()]
assert any(x.get('Action')=='pass' and x.get('Test')=='TestGraphDeliveryD03QualificationManifest' for x in manifest_events)
assert not any(x.get('Action')=='fail' for x in manifest_events)
for stage in campaign['stages']:assert sha(root/stage['result_path'])==stage['result_sha256']
freshness={}
for p in selected:
 if p.name in ['secret-data-verification.json','observe-verification.json','image-verification.json']:
  d=json.loads(p.read_text());old=d.get('input_sha256',{})
  if isinstance(old,dict):freshness[str(p.relative_to(root))]=dict(changed=[rel for rel,digest in old.items() if not (root/rel).is_file() or sha(root/rel)!=digest],whole_receipt_independently_accepted=False)
packet=dict(schema_version=1,prepared_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),status='READY_FOR_INDEPENDENT_REVIEW_WITH_OPEN_R20',binary_sha256=campaign['binary_sha256'],binary='bin/swarm-actions-confiance-secret-data-qualified',base_commit=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),branch=subprocess.check_output(['git','branch','--show-current'],text=True).strip(),dirty_diff_sha256=hashlib.sha256(subprocess.check_output(['git','diff','--binary'])).hexdigest(),source_input_sha256=inputs,proof_sha256=proofs,historical_receipt_input_freshness=freshness,public_stages=[dict(name=x['stage'],observed_results=x['observed_results'],exit_code=x['exit_code']) for x in campaign['stages']],native=dict(cases=407,main=111,subcases=296,one_execution=True),identity_browser_checks=sum(x['checks'] for x in browser['stages']),requirements=[dict(id=r['id'],observable=r['observable'],status='OPEN' if r['id'] in ['R11','R12','R20'] else 'LOCAL_EVIDENCE_AVAILABLE_NOT_ACCEPTED') for r in requirements],open=[dict(id='R20',owner='Codex plus authorized Linux environment',condition='native positive owned fixtures, foreign exclusion, loss/restart/cleanup, paired overhead'),dict(id='R11',owner='human authorization and independent reviewer',condition='separate context receives this exact candidate and returns attributable verdict'),dict(id='R12',owner='Codex',condition='complete-system qualification then final retrospective')],independent_review=False,global_acceptance=False,final_retex=False,limits=['untracked feature sources included by exact input hashes; tracked diff alone is incomplete','unrelated dirty work preserved; no authorship claimed','public stages use separate roots, one binary; scenario/check units are not summed','identity supplementary conductor ran in a tool cell; recipe hashes were frozen before/after; stage argv/process/result receipts retained','no BPF retry, new Linux privilege, provider account, payment, native mission, deployment or delegation'])
(out/'review-packet.json').write_text(json.dumps(packet,indent=2)+'\n')
print(json.dumps(dict(status=packet['status'],source_inputs=len(inputs),proofs=len(proofs),public_stages=len(campaign['stages']),native_cases=407,identity_browser_checks=packet['identity_browser_checks'],open=[r['id'] for r in packet['open']])))

