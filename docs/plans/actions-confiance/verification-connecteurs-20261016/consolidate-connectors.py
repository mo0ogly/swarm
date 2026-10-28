import datetime,hashlib,json
from pathlib import Path
repo=Path('/home/fpizzi/Documents/ChatGPT/swarm');proof=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda name:json.loads((proof/name).read_text())
native=[];names=set()
for name in ['connector-qualified3-race','connector-regression-shared','connector-regression-r16','connector-regression-laboratory']:
 v=read(name+'.json');assert v['exit_code']==0 and v['sources_unchanged']
 current=[n for n in v['named_passed_cases']if n!='TestActionsConfianceT6LaboratoryHelper'];assert current
 for p,h in v['input_sha256'].items():
  if p.endswith('.go')or p in ['go.mod','go.sum']:assert sha(repo/p)==h,(name,p)
 names.update(current);native.append({'receipt':name+'.json','cases':len(current),'main_cases':sum('/'not in n for n in current),'go_inputs_unchanged':True})
assert len(names)==327
binary=repo/'bin/swarm-actions-confiance-connectors-labelled';binary_hash=sha(binary)
for name in ['connector-public-final','connector-browser-final','connector-pilot-final']:
 v=read(name+'.json');assert v['exit_code']==0 and v['sources_unchanged']and v['binary_sha256']==binary_hash
 assert all(sha(repo/p)==h for p,h in v['input_sha256'].items())
public=read('connector-public-final/connector-public.json');browser=read('connector-browser-final/connector-ui.json');pilot=read('connector-pilot-final.out')
assert len(public['checks'])==11 and len(browser['checks'])==33 and browser['unexpected_diagnostics']==[]
assert public['binary_sha256']==binary_hash and pilot['binary_sha256']==binary_hash and len(pilot['cases'])==7 and all(c['status']=='pass'for c in pilot['cases'])
pilot_browser=next(c['observed']for c in pilot['cases']if c['id']=='public-fr-en-responsive-browser-recipe');assert len(pilot_browser['checks'])==23 and pilot_browser['diagnostics']==[]
for name in ['build','vet','i18n','contexts','diff']:
 v=read('connector-labelled-'+name+'.json');assert v['exit_code']==0 and v['sources_unchanged'];assert all(sha(repo/p)==h for p,h in v['input_sha256'].items())
manifest=read('connector-manifest-final-race.json');assert manifest['exit_code']==0 and manifest['sources_unchanged']and 'TestGraphDeliveryD03QualificationManifest'in manifest['named_passed_cases']
snapshot=json.loads((repo/'docs/engine-cursor-source-manifest.json').read_text());rows=[]
for part in snapshot['input_parts']:rows.extend(json.loads((repo/(part['path']if isinstance(part,dict)else part)).read_text()))
assert len(rows)==1455 and all(sha(repo/r['path'])==r['sha256']for r in rows)
preview=read('connector-preview-process.json');assert preview['verification_status']=='verified'and preview['binary_sha256']==binary_hash and preview['connector_catalog_verified']and preview['state_sha256_unchanged']=='d3b01d34c5c750ca91cbe1591ccd1c978ecb3a52a6416ffbbb0d610cfddd48c1'
paths=list((repo/'internal/engine').glob('trust*.go'))+[repo/p for p in ['web/trust-actions.js','web/trust-connectors.js','web/trust-actions.html','web/trust-actions.css','locales/trust-actions-en.json','web/trust-actions-en.js','tests/actions_confiance_connectors.py','tests/actions_confiance_connectors_ui.cjs','docs/actions-confiance-T6.md','docs/en/actions-confiance-T6.md','docs/actions-confiance-T4.md','docs/en/actions-confiance-T4.md','docs/architecture/actions-confiance-contract.json','tools/verification/refresh_engine_source_manifest.py']]
inputs={str(p.relative_to(repo)):sha(p)for p in paths}
evidence={str(p.relative_to(proof)):sha(p)for p in proof.rglob('*')if p.is_file()and '__pycache__'not in p.parts and p.name!='connector-verification.json'}
result={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'R18 two fixed simulated connectors; no real provider or independent/global acceptance','native_groups':native,'distinct_go_cases':len(names),'main_cases':sum('/'not in n for n in names),'sub_cases':sum('/'in n for n in names),'single_combined_native_execution':False,'native_reuse':'Go and module inputs unchanged after frontend-only amount encoding and labels; final public recipes use rebuilt labelled binary','public_connector_checks':11,'public_pilot_scenarios':7,'browser_checks':56,'browser_breakdown':{'connectors':33,'pilot':23},'unexpected_browser_diagnostics':[],'manifest_paths':len(rows),'binary':str(binary.relative_to(repo)),'binary_sha256':binary_hash,'preview_receipt':'connector-preview-process.json','review':'same-session self-review only','limitations':['both effects simulated in local state','no provider containment or sensitive provider qualification','no independent acceptance','R19, eBPF, global qualification and RETEX remain'],'input_sha256':inputs,'evidence_sha256':evidence}
(proof/'connector-verification.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:result[k]for k in ['distinct_go_cases','main_cases','sub_cases','public_connector_checks','public_pilot_scenarios','browser_checks','manifest_paths','binary_sha256']}))
