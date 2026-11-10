import datetime,hashlib,json,subprocess
from pathlib import Path
repo=Path('/home/fpizzi/Documents/ChatGPT/swarm');p=Path(__file__).resolve().parent
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
def read(n):return json.loads((p/n).read_text())
groups=['process-guards-typed-race','process-regression-t2','process-regression-admission-remote','process-regression-identity'];names=[]
for group in groups:
 v=read(group+'.json');assert v['exit_code']==0 and v['sources_unchanged'] and v['named_passed_cases'];assert not set(names)&set(v['named_passed_cases']);names+=v['named_passed_cases'];assert all(sha(repo/f)==h for f,h in v['input_sha256'].items())
for stage in ['build','vet','i18n','contexts','diff']:
 v=read('process-render-fixed-'+stage+'.json');assert v['exit_code']==0 and v['sources_unchanged'];assert all(sha(repo/f)==h for f,h in v['input_sha256'].items())
build=read('process-render-fixed-build.json');binary=repo/'bin/swarm-actions-confiance-processes';assert sha(binary)==build['binary_sha256']
for stage in ['process-public-final','process-public-t5-final']:
 v=read(stage+'.json');assert v['exit_code']==0 and v['sources_unchanged'];r=read(stage+'.out');assert r['binary_sha256']==sha(binary) and r['cases'] and all(c['status']=='pass' and isinstance(c['assertions'],int) and c['assertions']>0 and c['observed'] for c in r['cases'])
pilot=read('process-public-final.out');process=read('process-public-t5-final.out');ui=read('process-public-t5-final/browser/process-ui.json');assert len(ui['checks'])==38 and not ui['unexpected_diagnostics'] and ui['expected_denials']==2
manifest=read('process-manifest-current-race.json');assert manifest['exit_code']==0 and manifest['sources_unchanged'] and manifest['named_passed_cases']==['TestGraphDeliveryD03QualificationManifest']
snapshot=json.loads((repo/'docs/engine-cursor-source-manifest.json').read_text());rows=[]
for part in snapshot['input_parts']:
 assert sha(repo/part['path'])==part['sha256'];rows+=json.loads((repo/part['path']).read_text())
assert all(sha(repo/row['path'])==row['sha256'] for row in rows)
preview=read('process-preview-process.json');proc=Path('/proc')/str(preview['pid']);assert (proc/'stat').read_text().split(') ',1)[1].split()[19]==preview['proc_start_ticks'];assert [x.decode() for x in (proc/'cmdline').read_bytes().split(b'\0') if x]==preview['argv'];assert sha(Path(preview['argv'][0]))==preview['binary_sha256']==sha(binary);assert sha(Path(preview['fixture_root'])/'.swarm/trust-actions.json')==preview['state_sha256_unchanged']
r=subprocess.run(['git','diff','--check'],cwd=repo,capture_output=True);(p/'process-diff-current.out').write_bytes(r.stdout+r.stderr);assert r.returncode==0
proofs={str(f.relative_to(repo)):sha(f) for f in p.rglob('*') if f.is_file() and f.name!='process-verification.json'}
inputs={row['path']:row['sha256'] for row in rows if 'trust_' in row['path'] or 'process' in row['path'] or row['path'] in ['internal/engine/main.go','internal/engine/web_server.go','resources.go','go.mod','go.sum','build.sh','package.json','locales/en.json']}
result={'version':1,'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'local_slice_verified_not_independent_acceptance','binary':str(binary),'binary_sha256':sha(binary),'go':{'groups':groups,'distinct_cases':len(names),'main':sum('/' not in n for n in names),'sub':sum('/' in n for n in names),'single_execution':False},'public_cases':{'pilot':len(pilot['cases']),'process':len(process['cases'])},'browser':{'pilot':pilot['cases'][-1]['assertions'],'process':len(ui['checks']),'unexpected_diagnostics':[],'expected_process_denials':2,'responsive_contexts':20},'source_manifest_paths':len(rows),'preview':preview,'input_sha256':inputs,'proof_sha256':proofs,'limits':['local public/internal workflow tracking; no T2 payment approval/effect created','no version migration/retirement extension','operator cookie/CSRF plus existing identity credential; no new cross-origin SSO deployment','self-review only; global roadmap and RETEX remain open']}
(p/'process-verification.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps({'go':result['go'],'public':result['public_cases'],'browser':result['browser'],'sources':len(rows),'proofs':len(proofs)}))
