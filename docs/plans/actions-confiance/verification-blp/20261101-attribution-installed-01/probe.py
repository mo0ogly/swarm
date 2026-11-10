"""Read-only verification of the already-installed qualified BLP increment.

No lifecycle write, mission mutation, provider call or credential output. Use
protected metadata plus a checked owned-process observation rather than waiting
behind a concurrent lifecycle build. Browser profile stays in ignored evidence.
"""
import hashlib,json,os,subprocess,sys,tarfile,urllib.request
from pathlib import Path
sys.path.insert(0,'tools');sys.path.insert(0,'tests')
from swarm_version import session_request
from swarm_local import owned
from fixture_area import fixture_area
out=Path('docs/plans/actions-confiance/verification-blp/20261101-attribution-installed-01').resolve()
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
sha=value.hexdigest();assert sha=='4164953c841c2b22476bd552a12496aac2b7f95e9b26de04d986c3238e2ed83c'
assert version['binary']['version']=='v0.1.1-dev.292'
profile=Path('.cache/local-delivery/rust-3r0eves6/build/profile');inputs=json.loads((profile/'after.json').read_bytes())
sources=['rust/crates/swarm-engine/src/trust/confidentiality.rs','rust/crates/swarm-engine/src/trust/state.rs','rust/crates/swarm-engine/src/root_store/trust_store.rs','rust/crates/swarm-engine/src/root_store/reader_guard.rs','rust/crates/swarm-engine/src/trust/error.rs','rust/crates/swarm-confidentiality/src/lib.rs','rust/crates/swarm-web/src/service/trust.rs']
assert all(inputs[p]==hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in sources)
with tarfile.open(profile/'effective-sources.tar.gz') as archive:notes=archive.extractfile('version_history.json').read()
assert hashlib.sha256(notes).hexdigest()==inputs['version_history.json']
assert version['history']['local_changes']==json.loads(notes)['local_changes']
(out/'compiled-version-history.json').write_bytes(notes)
(out/'profile-inputs.json').write_bytes((profile/'after.json').read_bytes())
(out/'version.json').write_text(json.dumps(version,ensure_ascii=False,indent=2)+'\n')
status=dict(running=True,ready=True,pid=state['pid'],root=str(root),url='http://'+address+'/',binary=version['binary'],binary_sha256=sha)
(out/'status.json').write_text(json.dumps(status,indent=2)+'\n')
(out/'evidence').mkdir(mode=0o700);browser_out=out/'evidence/browser'
assert subprocess.run(['git','check-ignore','-q',str(browser_out/'browser/Default/Cookies')]).returncode==0
argv=['node','tests/paea_local_delivery_test.cjs'];env=dict(os.environ,TMPDIR=str(fixture_area()),SWARM_PAEA_LIVE_STATUS=str(out/'status.json'),SWARM_PAEA_LIVE_EVIDENCE=str(browser_out),SWARM_PAEA_LIVE_SOURCE_INVENTORY=str(out/'profile-inputs.json'),SWARM_PAEA_LIVE_NOTES=str(out/'compiled-version-history.json'))
run=subprocess.run(argv,capture_output=True,env=env)
(out/'browser.stdout').write_bytes(run.stdout);(out/'browser.stderr').write_bytes(run.stderr)
assert run.returncode==0,'browser verification failed; see retained diagnostics'
assert statepath.read_bytes()==original and owned(state),'service changed during readonly verification'
result=dict(status='PASS',version=version['binary']['version'],binary_sha256=sha,runtime_sources_match=sources,profile=str(profile),service_restarted=False,global_watchdog=None,scope='installed BLP increment and read-only PAEA/version/assets; no global BLP conformance')
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print('BLP_INSTALLED_PASS '+result['version']+' '+sha)
