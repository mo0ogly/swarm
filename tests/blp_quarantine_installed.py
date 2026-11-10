"""Read-only verification of installed preparation confidentiality increments.

No lifecycle write, mission mutation, provider call or credential output. Use
protected metadata plus a checked owned-process observation rather than waiting
behind a concurrent lifecycle build. Browser profile stays in ignored evidence. Explicit --metadata-only-authority
leaves the real protected UI criterion OPEN and never downgrades its gate.
"""
import hashlib,json,os,subprocess,sys,tarfile,urllib.request
from pathlib import Path
sys.path.insert(0,'tools');sys.path.insert(0,'tests')
from swarm_version import session_request
from swarm_local import owned
from fixture_area import fixture_area
import argparse
ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',type=Path,required=True);ap.add_argument('--sha256',required=True);ap.add_argument('--source-producer',action='store_true',help='Also pin captured-source attribution runtime inputs and CLI help');ap.add_argument('--admission',action='store_true',help='Also pin immutable admission runtime inputs and CLI help');ap.add_argument('--metadata-only-authority',action='store_true',help='Explicit partial verification in real Authority mode; require UI denial and leave live UI criterion open');args=ap.parse_args()
out=args.output.resolve()
out.mkdir(mode=0o700,exist_ok=False);(out/'probe.py').write_bytes(Path(__file__).read_bytes())
root=Path.cwd().resolve();config=json.loads(Path('deploy/local-web.json').read_bytes());address=config['address']
assert address.startswith('127.0.0.1:') and Path(config['root']).resolve()==root
statepath=root/'.swarm/local-web'/hashlib.sha256(address.encode()).hexdigest()[:16]/'server.json'
original=statepath.read_bytes();state=json.loads(original);assert owned(state)
request=session_request(address,root,'/api/v1/native-version')
with urllib.request.urlopen(request) as response:version=json.load(response)
value=hashlib.sha256()
with Path('/proc',str(state['pid']),'exe').open('rb') as image:
 for block in iter(lambda:image.read(1024*1024),b''):value.update(block)
sha=value.hexdigest();prepared=json.loads(Path('deploy/local-build.json').read_bytes());assert sha==prepared['binary_sha256']==args.sha256
assert version['binary']['version']==prepared['version']
profile=Path(prepared['evidence'])/'build/profile';inputs=json.loads((profile/'after.json').read_bytes())
sources=['rust/crates/swarm-engine/src/root_store/preparation_confidentiality.rs','rust/crates/swarm-engine/src/local_storage.rs','rust/crates/swarm-engine/src/root_store/trust_enrollment.rs','rust/crates/swarm-engine/src/root_store/preparation_call_cli.rs','rust/crates/swarm-cli/src/preparation_worker.rs','web/trust-confidentiality.js','web/trust-native.html','rust/crates/swarm-web/src/service.rs','rust/crates/swarm-engine/src/root_store/preparation_call.rs','rust/crates/swarm-engine/src/root_store/preparation_prompt.rs','rust/crates/swarm-engine/src/trust/authentication.rs','rust/crates/swarm-engine/src/trust/confidentiality.rs','rust/crates/swarm-engine/src/trust/state.rs','rust/crates/swarm-engine/src/root_store/trust_store.rs','rust/crates/swarm-engine/src/root_store/reader_guard.rs','rust/crates/swarm-engine/src/trust/error.rs','rust/crates/swarm-confidentiality/src/lib.rs','rust/crates/swarm-web/src/service/trust.rs','rust/crates/swarm-cli/src/trust.rs','rust/crates/swarm-cli/src/lib.rs']
if args.source_producer:sources.append('rust/crates/swarm-engine/src/root_store/preparation_attribution.rs')
if args.admission:sources.append('rust/crates/swarm-engine/src/root_store/preparation_admission.rs')
assert all(inputs[p]==hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in sources)
with tarfile.open(profile/'effective-sources.tar.gz') as archive:notes=archive.extractfile('version_history.json').read()
assert hashlib.sha256(notes).hexdigest()==inputs['version_history.json']
assert version['history']['local_changes']==json.loads(notes)['local_changes']
if args.metadata_only_authority:
 mode_run=subprocess.run([str(Path('/proc',str(state['pid']),'exe')),'engine-settings','show','--root',str(root),'--json'],capture_output=True,check=True)
 mode=json.loads(mode_run.stdout)['values']['web_access_mode'];assert mode=='authority'
 refused=[]
 for route in ['/trust-actions.html','/trust-confidentiality.js','/i18n-en.js']:
  try:
   with urllib.request.urlopen(session_request(address,root,route)) as response:response.read()
  except urllib.error.HTTPError as error:
   assert error.code==403;refused.append(dict(route=route,status=403))
  else:raise AssertionError('Authority unexpectedly accepted the owner cookie on protected UI')
 (out/'authority-gate.json').write_text(json.dumps(dict(mode=mode,owner_key_ui_refusals=refused,live_ui_verified=False),indent=2)+'\n')
else:
 for route,source in [('/trust-actions.html','web/trust-native.html'),('/trust-confidentiality.js','web/trust-confidentiality.js'),('/i18n-en.js','web/i18n-en.js')]:
  with urllib.request.urlopen(session_request(address,root,route)) as response:raw=response.read()
  assert hashlib.sha256(raw).hexdigest()==inputs[source],route
(out/'compiled-version-history.json').write_bytes(notes)
(out/'qualified-profile.json').write_bytes((profile/'result.json').read_bytes())
(out/'profile-inputs.json').write_bytes((profile/'after.json').read_bytes())
(out/'version.json').write_text(json.dumps(version,ensure_ascii=False,indent=2)+'\n')
status=dict(running=True,ready=True,pid=state['pid'],root=str(root),url='http://'+address+'/',binary=version['binary'],binary_sha256=sha)
(out/'status.json').write_text(json.dumps(status,indent=2)+'\n')
if not args.metadata_only_authority:
 (out/'evidence').mkdir(mode=0o700);browser_out=out/'evidence/browser'
 assert subprocess.run(['git','check-ignore','-q',str(browser_out/'browser/Default/Cookies')]).returncode==0
 argv=['node','tests/paea_local_delivery_test.cjs'];env=dict(os.environ,TMPDIR=str(fixture_area()),SWARM_PAEA_LIVE_STATUS=str(out/'status.json'),SWARM_PAEA_LIVE_EVIDENCE=str(browser_out),SWARM_PAEA_LIVE_SOURCE_INVENTORY=str(out/'profile-inputs.json'),SWARM_PAEA_LIVE_NOTES=str(out/'compiled-version-history.json'))
 run=subprocess.run(argv,capture_output=True,env=env)
 (out/'browser.stdout').write_bytes(run.stdout);(out/'browser.stderr').write_bytes(run.stderr)
 assert run.returncode==0,'browser verification failed; see retained diagnostics'
assert statepath.read_bytes()==original and owned(state),'service changed during readonly verification'
result=dict(status='PARTIAL_AUTHORITY_UI_NOT_VERIFIED' if args.metadata_only_authority else 'PASS',version=version['binary']['version'],binary_sha256=sha,runtime_sources_match=sources,profile=str(profile),service_restarted_by_readonly_probe=False,global_watchdog=None,scope='active binary, compiled notes/runtime sources and owner-key UI403 in Authority mode; live protected UI not verified' if args.metadata_only_authority else 'installed quarantine/captured-source/admission candidate, protected asset bytes and read-only PAEA/version browser; no live classification, quarantine activation or global BLP conformance')
for lang in ['fr','en']:
 run=subprocess.run([str(Path('/proc',str(state['pid']),'exe')),'--help','--lang',lang],capture_output=True)
 if args.source_producer:assert b'blp-preparation-attribute' in run.stdout
 if args.admission:assert all(op in run.stdout for op in [b'blp-preparation-admit',b'blp-preparation-input-attribute',b'blp-preparation-admission-show'])
 (out/('help-'+lang+'.stdout')).write_bytes(run.stdout);(out/('help-'+lang+'.stderr')).write_bytes(run.stderr)
 assert run.returncode==0 and b'blp-preparation-check' in run.stdout and b'blp-object-attribute' in run.stdout and (b'filiation physique' in run.stdout if lang=='fr' else b'physical lineage' in run.stdout)
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(('BLP_ADMISSION_INSTALLED_METADATA_VERIFIED_UI_OPEN ' if args.metadata_only_authority else 'BLP_ADMISSION_INSTALLED_PASS ' if args.admission else 'BLP_SOURCE_PRODUCER_INSTALLED_PASS ' if args.source_producer else 'BLP_QUARANTINE_INSTALLED_PASS ')+result['version']+' '+sha)
