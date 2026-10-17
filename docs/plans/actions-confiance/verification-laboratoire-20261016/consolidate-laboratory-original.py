import datetime,hashlib,json,subprocess
from pathlib import Path
repo=Path('/home/fpizzi/Documents/ChatGPT/swarm');p=Path(__file__).resolve().parent
sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
read=lambda n:json.loads((p/n).read_text())
groups=['laboratory-qualified-race','laboratory-regression-t2','laboratory-regression-r16'];names=[]
for group in groups:
 v=read(group+'.json');assert v['exit_code']==0 and v['sources_unchanged']and v['named_passed_cases'];assert all(sha(repo/f)==h for f,h in v['input_sha256'].items()),group
 current=[n for n in v['named_passed_cases']if n!='TestActionsConfianceT6LaboratoryHelper'];assert not set(current)&set(names);names+=current
assert len(names)==191
for stage in ['build','vet','i18n','contexts','diff']:
 v=read('laboratory-readable-'+stage+'.json');assert v['exit_code']==0 and v['sources_unchanged'];assert all(sha(repo/f)==h for f,h in v['input_sha256'].items()),stage
binary=repo/'bin/swarm-actions-confiance-laboratory-readable';assert sha(binary)==read('laboratory-readable-build.json')['binary_sha256']
for stage in ['laboratory-public-readable','laboratory-browser-readable','laboratory-pilot-readable']:
 v=read(stage+'.json');assert v['exit_code']==0 and v['sources_unchanged'];assert all(sha(repo/f)==h for f,h in v['input_sha256'].items()),stage
public=read('laboratory-public-readable/laboratory-public.json');ui=read('laboratory-browser-readable/laboratory-ui.json');pilot=read('laboratory-pilot-readable.out');assert len(public['checks'])==8 and public['binary_sha256']==sha(binary);assert len(ui['checks'])==32 and ui['expected_denials']==2 and not ui['unexpected_diagnostics'];assert len(pilot['cases'])==7 and pilot['binary_sha256']==sha(binary)and pilot['cases'][-1]['assertions']==23
manifest=read('laboratory-manifest-readable-race.json');assert manifest['exit_code']==0 and manifest['sources_unchanged'] and manifest['named_passed_cases']==['TestGraphDeliveryD03QualificationManifest']
rows=[];snapshot=json.loads((repo/'docs/engine-cursor-source-manifest.json').read_text())
for part in snapshot['input_parts']:
 assert sha(repo/part['path'])==part['sha256'];rows+=json.loads((repo/part['path']).read_text())
assert all(sha(repo[r['path']])==r['sha256']for r in rows)
preview=read('laboratory-readable-preview-process.json');proc=Path('/proc')/str(preview['pid']);assert preview['verification_status']=='verified';assert(proc/'stat').read_text().split(') ',1)[1].split()[19]==preview['proc_start_ticks'];assert[x.decode()for x in(proc/'cmdline').read_bytes().split(b'\0')if x]==preview['argv'];assert sha(Path(preview['argv'][0]))==sha(binary);assert sha(Path(preview['fixture_root'])/'.swarm/trust-actions.json')==preview['state_sha256_unchanged']
r=subprocess.run(['git','diff','--check'],cwd=repo,capture_output=True);(p/'laboratory-diff-current.out').write_bytes(r.stdout+r.stderr);assert r.returncode==0
proofs={str(f.relative_to(repo)):sha(f)for f in p.rglob('*')if f.is_file()and f.name!='laboratory-verification.json'}
inputs={r['path']:r['sha256']for r in rows if 'trust_'in r['path']or 'laborat'in r['path']or r['path']in['internal/engine/main.go','internal/engine/web_server.go','resources.go','go.mod','go.sum','build.sh','package.json','locales/trust-actions-en.json']}
result={'version':1,'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'local_R17_fixed_laboratory_verified_not_independent_acceptance','binary':str(binary),'binary_sha256':sha(binary),'go':{'groups':groups,'significant_cases':191,'main':sum('/'not in n for n in names),'sub':sum('/'in n for n in names),'laboratory':18,'pilot':126,'public_evidence':47,'single_execution':False,'excluded_empty_helper':'TestActionsConfianceT6LaboratoryHelper','reuse':'All Go inputs match exact current bytes. Later UI-only changes provide a readable summary; public recipes use the rebuilt binary. Earlier overlapping groups are not added.'},'public':{'laboratory_cli_api_checks':8,'pilot_scenarios':7},'browser':{'laboratory':32,'pilot':23,'responsive_contexts':20,'unexpected_diagnostics':[],'expected_laboratory_denials':2},'source_manifest_paths':len(rows),'preview':preview,'input_sha256':inputs,'proof_sha256':proofs,'limits':['closed fixed corpus with five scenarios and fourteen OS guards; no arbitrary code or provider confinement','fixture inputs/policies/time are separate from production business data and operator execution policy','hard parent death stops child but can leave an orphaned simulation root; no global cleanup scan','real providers, sensitive effects, eBPF observation, R18/R19 and T8/RETEX remain open','self-review only; no independent/global acceptance']}
(p/'laboratory-verification.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps({'go':result['go'],'public':result['public'],'browser':result['browser'],'sources':len(rows),'inputs':len(inputs),'proofs':len(proofs)}))
