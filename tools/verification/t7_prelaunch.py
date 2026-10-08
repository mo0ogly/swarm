#!/usr/bin/env python3
"""T7 reproducible checks: isolated browser fixtures, fingerprints, no live DB."""
import hashlib,json,pathlib,subprocess,sys,tempfile
ROOT=pathlib.Path(__file__).resolve().parents[2]
FILES=('web/mission.js','web/prephase-conversion.js','web/prephase-editor.js','web/prepare.html','web/prephase.js','web/i18n-en.js','locales/en.json','tests/prelaunch_diagnostic_ui.cjs')
mode=sys.argv[1] if len(sys.argv)>1 else 'all'
assert mode in ('entry','browser','validation','delivery','all')
out=ROOT/'test-results/t7-engine'
fingerprints={f:hashlib.sha256((ROOT/f).read_bytes()).hexdigest() for f in FILES}
def run(args):
 print('COMMAND',args,flush=True)
 subprocess.run(args,cwd=ROOT,check=True,timeout=180)
if mode in ('entry','all'):
 assert (ROOT/'node_modules/puppeteer').is_dir()
 print('PASS prerequisites')
if mode in ('browser','all'):
 with tempfile.TemporaryDirectory(prefix='swarm-t7-check-') as d:
  binary=str(pathlib.Path(d)/'swarm')
  run(['go','build','-o',binary,'./cmd/swarm'])
  run(['node','tests/prelaunch_diagnostic_ui.cjs',binary,str(out)])
 run(['node','tests/i18n_test.cjs'])
 assert fingerprints=={f:hashlib.sha256((ROOT/f).read_bytes()).hexdigest() for f in FILES},'source changed during check'
 (out/'sources.json').write_text(json.dumps(fingerprints,indent=2,sort_keys=True))
if mode in ('validation','delivery','all'):
 assert json.loads((out/'sources.json').read_text())==fingerprints,'stale evidence'
 result=json.loads((out/'result.json').read_text())
 assert result['status']=='PASS' and not result['errors'] and len(result['checks'])>=10
 for prefix in ('summary-missing','summary-ready','summary-en','diagnostic-copy','diagnostic-copy-en','preflight-details-fr','preflight-details-en'):
  for theme in ('etat','sombre'):
   assert (out/(prefix+'-'+theme+'.png')).stat().st_size>1000
 print('PASS fresh source-bound browser checks and fourteen screenshots; clipboard OS boundary mocked')
if mode in ('delivery','all'):
 report=(ROOT/'docs/plan-843bb3ce22-T7.md').read_text()
 assert all(x in report for x in ('REQ-SUM-01','REQ-DIAG-01','req-22','127','429'))
 print('PASS report presence; qualitative independent review remains required')
print('PASS T7',mode)
