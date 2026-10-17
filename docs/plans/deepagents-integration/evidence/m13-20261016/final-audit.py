from pathlib import Path
import subprocess,json,hashlib,re,difflib,datetime
root=Path(__file__).resolve().parents[5]; out=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
owned=['internal/engine/agent_dialogue.go','internal/engine/dialogue_checkpoint.go','internal/engine/dialogue_checkpoint_store.go','internal/engine/dialogue_boundary_integrity_test.go','locales/en.json','web/i18n-en.js','tests/dialogue_recovery_ui.cjs','docs/PROJECT-MEMORY.md','docs/en/PROJECT-MEMORY.md','docs/plans/deepagents-integration/PLAN.md','docs/plans/deepagents-integration/cas-evaluation.json','docs/plans/deepagents-integration/INCREMENT-MEMOIRE-09.md','docs/plans/deepagents-integration/RESULTATS-MEMOIRE-09.md']
checks=[]
for argv in [['git','diff','--check'],['gofmt','-l',*owned[:4]],['node','--check','tests/dialogue_recovery_ui.cjs'],['node','scripts/i18n/build.cjs','--check']]:
 r=subprocess.run(argv,cwd=root,capture_output=True,text=True); assert r.returncode==0,(argv,r.stdout,r.stderr)
 if argv[0]=='gofmt': assert not r.stdout.strip(),r.stdout
 checks.append({'argv':argv,'exit_code':r.returncode,'output':r.stdout.strip()})
for f in owned:
 r=subprocess.run(['git','diff','--no-index','--check','/dev/null',f],cwd=root,capture_output=True,text=True)
 assert r.returncode in (0,1) and not r.stdout+r.stderr,(f,r.stdout,r.stderr)
checks.append({'check':'owned file whitespace','files':len(owned),'result':'PASS'})
receipts={}
for name in ['boundary-final','boundary-race','build','static','i18n','web-unit','configuration','browser-flow']:
 d=json.loads((out/(name+'.json')).read_text()); assert d['exit_code']==0 and not d['changed_inputs'],name
 assert d['log_sha256']==sha(out/(name+'.log')),name
 inputs=json.loads((out/(name+'-inputs.json')).read_text())
 changed=[f for f,h in inputs.items() if not (root/f).is_file() or sha(root/f)!=h]; assert not changed,(name,changed)
 receipts[name]={'exit_code':0,'named_tests_ran':d['named_tests_ran'],'input_files':len(inputs),'log_sha256':d['log_sha256']}
assert len(receipts['boundary-final']['named_tests_ran'])==15
assert len(receipts['boundary-race']['named_tests_ran'])==6
assert set(receipts['boundary-race']['named_tests_ran'])<=set(receipts['boundary-final']['named_tests_ran'])
red=json.loads((out/'boundary-red.json').read_text()); assert red['exit_code']==1 and len(red['named_tests_ran'])==4
assert sha(out/'preimages/dialogue_boundary_integrity_red_test.go.txt')==json.loads((out/'boundary-red-inputs.json').read_text())['internal/engine/dialogue_boundary_integrity_test.go']
for old,preimage in [('browser','dialogue_recovery_integrity_first_browser.cjs.txt'),('browser-final','dialogue_recovery_integrity_second_browser.cjs.txt')]:
 d=json.loads((out/(old+'.json')).read_text()); assert d['exit_code']==1 and d['log_sha256']==sha(out/(old+'.log'))
 assert (out/'preimages'/preimage).is_file()
meta=json.loads((out/'browser-flow-inputs-metadata.json').read_text()); binary=Path((out/'binary-path.txt').read_text().strip())
assert meta['script_sha256']==sha(root/'tests/dialogue_recovery_ui.cjs') and meta['binary_sha256']==sha(binary)
assert json.loads((out/'built-binary.json').read_text())['sha256']==sha(binary)
browser=json.loads((out/'browser-flow/result.json').read_text()); assert browser['status']=='PASS' and not browser['errors'] and not browser['failures'] and browser['model_calls']==0 and browser['provider_invocations']==1
assert sum(x.get('status')==400 for x in browser['expectedFailures'])==4
assert len(list((out/'browser-flow').glob('*.png')))==16
baseline=json.loads((out/'baseline.json').read_text()); runtime=json.loads(subprocess.check_output(['./swarm.sh','status'],cwd=root,text=True))
assert runtime['pid']==baseline['runtime']['pid'] and runtime['ready'] and runtime['running']
(out/'unchanged-live-runtime.json').write_text(json.dumps({'before':baseline['runtime'],'after':runtime,'deployed':False},indent=2)+'\n')
oldcases=json.loads((out/'preimages/docs__plans__deepagents-integration__cas-evaluation.json').read_text()); cases=json.loads((root/'docs/plans/deepagents-integration/cas-evaluation.json').read_text()); assert cases['cases']==oldcases['cases'] and cases['evaluations'][:-1]==oldcases['evaluations']
patch=''; links=[]
for f in owned:
 p=root/f; pre=out/'preimages'/f.replace('/','__'); before=pre.read_text().splitlines(True) if pre.exists() else []
 patch+=''.join(difflib.unified_diff(before,p.read_text().splitlines(True),fromfile='a/'+f if pre.exists() else '/dev/null',tofile='b/'+f))
 if p.suffix=='.md':
  assert ',+' not in p.read_text(),f
  if p.name=='RESULTATS-MEMOIRE-09.md': assert 'Vérification navigateur en cours' not in p.read_text()
  for link in re.findall(r'\]\(([^)]+)\)',p.read_text()):
   if '://' not in link and not link.startswith('#'):
    assert (p.parent/link.split('#')[0]).resolve().exists(),(f,link); links.append({'file':f,'link':link})
(out/'m13-changes.patch').write_text(patch)
(out/'method-observations.json').write_text(json.dumps({'scope':'Local M13; no delegation, real model or runtime deployment','incidents':[{'cause':'Reader omitted row-session agreement and semantic session/usage checks; end confirmation overwrote a changed session','original':'boundary-red.json/.log','owner':'Codex engine change','effect':'Shared reader refuses; compare-and-set blocks next exchange; negative and transition cases retained'},{'cause':'Browser recipe assumed a storage key and retained English URL','original':'browser.json/.log; browser/failure.json','changed_precondition':'Use actual lang query parameter and setTheme function','owner':'Codex recipe'},{'cause':'Snapshot polling canceled by navigation counted as unexpected failure','original':'browser-final.json/.log','changed_precondition':'Classify only same-fixture snapshot ERR_ABORTED during observed navigation; retain separately; all other failures block','owner':'Codex recipe','final_navigation_aborts':browser['navigationAborts']},{'cause':'File searches assumed absent paths; documentation patch introduced stray plus; audit rejected legitimate existing text','changed_precondition':'Read actual producers; reread corrected paragraph; narrow audit to current result status','owner':'Codex method'}]},ensure_ascii=False,indent=2)+'\n')
evidence={str(p.relative_to(out)):sha(p) for p in out.rglob('*') if p.is_file() and p.name!='final-audit.json'}
result={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'head':baseline['head'],'branch':baseline['branch'],'source_hashes':{f:sha(root/f) for f in owned},'checks':checks,'receipts':receipts,'distinct_go_tests':15,'race_tests_included':6,'local_browser_checks':len(browser['checks']),'links_checked':len(links),'runtime_unchanged':runtime,'real_model_calls':0,'deployed':False,'provider_restoration':'NOT IMPLEMENTED','evidence_hashes':evidence}
(out/'final-audit.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'checks':len(checks),'owned_files':len(owned),'go_tests':15,'race_tests_included':6,'browser_checks':len(browser['checks']),'evidence_files_hashed':len(evidence),'runtime_pid_unchanged':runtime['pid'],'source_drift':[]}))
