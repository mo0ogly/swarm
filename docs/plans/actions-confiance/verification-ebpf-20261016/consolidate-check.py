"""Evidence for a closed diagnostic; never a positive R20 qualification."""
import datetime,hashlib,json,re,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[4];out=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
load=lambda n:json.loads((out/n).read_text())
groups=[];allnames=set()
for name,expected in [('check-race.out',10),('check-regression-race.out',184),('check-connectors-race.out',29)]:
 p=out/name;s=p.read_text();assert re.search(r'\nPASS\nok\s',s) and not re.search(r'^FAIL|^\s*--- FAIL:',s,re.M)
 names=re.findall(r'^\s*--- PASS: (\S+)',s,re.M);assert len(names)==expected and not allnames.intersection(names)
 allnames.update(names);groups.append(dict(output=name,sha256=sha(p),passed_names=names))
assert len(allnames)==223
public=load('check-public-current/kernel-public.json');assert len(public['checks'])==6 and public['public_unavailable_exit']==3 and public['server_stopped']
ui=load('check-browser-visual/kernel-ui.json');assert len(ui['checks'])==32 and ui['errors']==[]
visual=load('check-admin-viewport/kernel-admin-visual.json');assert visual['errors']==[]
pilot=json.loads((out/'check-pilot.out').read_text());assert len(pilot['cases'])==7 and all(c['status']=='pass' for c in pilot['cases'])
assert next(c for c in pilot['cases'] if c['id']=='public-fr-en-responsive-browser-recipe')['assertions']==23
for filename in ['kernel-local-unavailable.json','kernel-remote-unavailable.json']:
 v=load('check-public-current/'+filename);assert v['available'] is False and v['events']==[] and v['cleanup_verified'] and v['reason']=='permission_denied'
for name in ['check-build.out','check-focus-build.out','check-final-build.out','check-vet.out','check-diff.out']:
 assert (out/name).read_bytes()==b'',name
assert 'OK: 12 shared' in (out/'check-contexts.out').read_text()
assert 'PASS i18n build check' in (out/'check-i18n.out').read_text()
assert load('check-contract.out')['requirements']==20
assert load('check-mapping-after.out')['status']=='mapping_artifact_valid'
s=(out/'check-manifest-race.out').read_text();assert '--- PASS: TestGraphDeliveryD03QualificationManifest' in s and re.search(r'\nPASS\nok\s',s)
resources=json.loads(subprocess.check_output(['go','list','-json','.'],cwd=root,text=True)).get('EmbedFiles',[])
paths=[*root.glob('internal/engine/kernel_*.go'),*root.glob('internal/engine/trust_kernel_*.go')]
paths += [root/p for p in ['internal/engine/trust_actions.go','internal/engine/trust_actions_cli.go','internal/engine/trust_actions_http.go','internal/engine/trust_remote.go','internal/engine/main.go','web/trust-actions.js','web/trust-actions.html','web/trust-actions.css','web/trust-kernel.js','web/trust-controls.json','docs/actions-confiance-T7.md','docs/en/actions-confiance-T7.md','docs/architecture/actions-confiance-contract.json','tests/actions_confiance_kernel.py','tests/actions_confiance_kernel_ui.cjs','go.mod','go.sum']]
v=dict(verified_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),requirement='R20',status='PARTIAL',scope='public closed owned-fixture diagnostic; no business-agent selection',tests=dict(distinct_go_cases=223,main=63,subcases=160,groups=groups,public_kernel_checks=6,pilot_scenarios=7,browser_kernel_checks=32,browser_pilot_checks=23,unexpected_browser_diagnostics=0),input_sha256={str(p.relative_to(root)):sha(p) for p in paths},binary_sha256=sha(root/'bin/swarm-actions-confiance-kernel-public'),browser_binary_sha256=ui['binary_sha256'],browser_reuse='Only the static controls matrix and architecture evidence-path document changed in the executable after browser qualification; kernel/pilot Go and UI assets unchanged. Admin viewport capture separately inspected.',public=public,linux_privileges_changed=False,native_positive_qualified=False,global_acceptance=False,independent_review=False,remaining=['authorized business-attempt collector integration','positive kernel/verifier validation','foreign exclusion and real loss/restart recipe','measured overhead with and without collection'],proofs={str(p.relative_to(root)):sha(p) for p in out.rglob('*') if p.is_file() and p.name not in ('check-verification.json','check-consolidation.out') and '__pycache__' not in p.parts})
(out/'check-verification.json').write_text(json.dumps(v,indent=2)+'\n')
print(json.dumps(dict(status=v['status'],go_cases=223,kernel_public=6,browser_kernel=32,browser_pilot=23,native_positive_qualified=False)))
