import datetime,hashlib,json,subprocess
from pathlib import Path
repo=Path('/home/fpizzi/Documents/ChatGPT/swarm');p=Path(__file__).resolve().parent
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
def read(n):return json.loads((p/n).read_text())
change=read('evidence-disabled-revoke-source-change.json');service=repo/change['path'];oldservice=(p/'evidence-service-disabled-revoke-before.txt').read_bytes();assert hashlib.sha256(oldservice).hexdigest()==change['before_sha256'];assert oldservice.decode().replace(change['only_replacement']['before'],change['only_replacement']['after'],1)==service.read_text();assert sha(service)==change['after_sha256']
testpath='internal/engine/trust_evidence_test.go';oldtest=(repo/testpath).read_text().split('\nfunc TestActionsConfianceT6EvidenceRevokeWhileDisabled',1)[0].rstrip()+'\n';oldtesthash=hashlib.sha256(oldtest.encode()).hexdigest()
preconcurrent=(repo/testpath).read_text().split('\nfunc TestActionsConfianceT6EvidenceConcurrentPrepare',1)[0].rstrip()+'\n';preconcurrenthash=hashlib.sha256(preconcurrent.encode()).hexdigest()
names=[];groups=['evidence-regression-t2','evidence-regression-t3','evidence-regression-t4','evidence-regression-t5','evidence-revoke-fixed-race','evidence-concurrent-final-race']
for group in groups:
 v=read(group+'.json');assert v['exit_code']==0 and v['sources_unchanged'] and v['named_passed_cases'];assert not set(names)&set(v['named_passed_cases']);names+=v['named_passed_cases']
 for f,h in v['input_sha256'].items():
  if group not in ['evidence-revoke-fixed-race','evidence-concurrent-final-race']and f==change['path']:assert h==change['before_sha256']
  elif group not in ['evidence-revoke-fixed-race','evidence-concurrent-final-race']and f==testpath:assert h==oldtesthash
  elif group=='evidence-revoke-fixed-race'and f==testpath:assert h==preconcurrenthash
  else:assert sha(repo/f)==h,(group,f)
for stage in ['build','vet','i18n','contexts','diff']:
 v=read('evidence-labelled-'+stage+'.json');assert v['exit_code']==0 and v['sources_unchanged'];assert all((f==testpath and h==preconcurrenthash) or sha(repo/f)==h for f,h in v['input_sha256'].items())
binary=repo/'bin/swarm-actions-confiance-evidence-labelled';build=read('evidence-labelled-build.json');assert sha(binary)==build['binary_sha256']
for stage in ['evidence-public-labelled','evidence-browser-labelled','evidence-pilot-labelled']:
 v=read(stage+'.json');assert v['exit_code']==0 and v['sources_unchanged']
public=read('evidence-public-labelled/evidence-public.json');ui=read('evidence-browser-labelled/evidence-ui.json');pilot=read('evidence-pilot-labelled.out');assert len(public['checks'])==11 and public['binary_sha256']==sha(binary);assert len(ui['checks'])==32 and not ui['unexpected_diagnostics'] and ui['expected_denials']==4;assert len(pilot['cases'])==7 and pilot['binary_sha256']==sha(binary)
manifest=read('evidence-manifest-final-race.json');assert manifest['exit_code']==0 and manifest['sources_unchanged']and manifest['named_passed_cases']==['TestGraphDeliveryD03QualificationManifest']
rows=[];snapshot=json.loads((repo/'docs/engine-cursor-source-manifest.json').read_text())
for part in snapshot['input_parts']:
 assert sha(repo/part['path'])==part['sha256'];rows+=json.loads((repo/part['path']).read_text())
assert all(sha(repo/row['path'])==row['sha256']for row in rows)
preview=read('evidence-preview-process.json');assert preview['verification_status']=='verified';proc=Path('/proc')/str(preview['pid']);assert(proc/'stat').read_text().split(') ',1)[1].split()[19]==preview['proc_start_ticks'];assert[x.decode()for x in(proc/'cmdline').read_bytes().split(b'\0')if x]==preview['argv'];assert sha(Path(preview['argv'][0]))==preview['binary_sha256']==sha(binary);assert sha(Path(preview['fixture_root'])/'.swarm/trust-actions.json')==preview['state_sha256_unchanged']
r=subprocess.run(['git','diff','--check'],cwd=repo,capture_output=True);(p/'evidence-diff-current.out').write_bytes(r.stdout+r.stderr);assert r.returncode==0
proofs={str(f.relative_to(repo)):sha(f)for f in p.rglob('*')if f.is_file()and f.name!='evidence-verification.json'}
inputs={row['path']:row['sha256']for row in rows if 'trust_'in row['path']or 'evidence'in row['path']or row['path']in['internal/engine/store.go','internal/engine/main.go','internal/engine/web_server.go','resources.go','go.mod','go.sum','build.sh','package.json','locales/trust-actions-en.json']}
result={'version':1,'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'local_R16_signing_slice_verified_not_independent_acceptance','binary':str(binary),'binary_sha256':sha(binary),'go':{'groups':groups,'distinct_cases':len(names),'main':sum('/'not in n for n in names),'sub':sum('/'in n for n in names),'single_execution':False,'reuse':'326 prior cases retain all relevant source branches; only new disabled key-revoke gate and appended guard changed, exact preimage/diff verified. All22 R16 cases executed after that fix; a final additional test-only concurrency case is executed separately. The appended test is excluded from production compilation; exact pre-append test bytes and all production/build inputs are checked.'},'public':{'evidence_cli_checks':len(public['checks']),'pilot_scenarios':len(pilot['cases'])},'browser':{'evidence':len(ui['checks']),'pilot':pilot['cases'][-1]['assertions'],'responsive_contexts':20,'unexpected_diagnostics':[],'expected_evidence_denials':4},'source_manifest_paths':len(rows),'preview':preview,'input_sha256':inputs,'proof_sha256':proofs,'limits':['ephemeral protected software signing key; no hardware vault, swap or privileged trace qualification','public pin acquired separately; old offline pins do not learn later revocations','no outside-root witness/archival/retention delivery yet; R16 remains partial','R17 laboratory/R18 connector/R19 mapping and T7/T8/RETEX remain open','self-review only; no independent/global release acceptance']}
(p/'evidence-verification.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps({'go':result['go'],'public':result['public'],'browser':result['browser'],'sources':len(rows),'inputs':len(inputs),'proofs':len(proofs)}))
