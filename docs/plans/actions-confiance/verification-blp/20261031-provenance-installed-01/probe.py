"""Read-only verification of this exact prepared local BLP increment.

Identifies the loaded image, protected version endpoint, compiled FR/EN notes
and existing Version dialog. Session material only crosses private subprocess
stdin; it is never retained or printed. No business operation or provider call.
"""
import hashlib,json,os,subprocess,sys,urllib.request
from pathlib import Path
sys.path.insert(0,'tools')
sys.path.insert(0,'tests')
from swarm_version import session_request
from fixture_area import fixture_area
out=Path('docs/plans/actions-confiance/verification-blp/20261031-provenance-installed-01').resolve()
out.mkdir(mode=0o700,exist_ok=False)
(out/'probe.py').write_bytes(Path(__file__).read_bytes())
status=subprocess.run(['./swarm.sh','status'],capture_output=True,check=True)
(out/'status.json').write_bytes(status.stdout)
s=json.loads(status.stdout)
b=json.loads(Path('deploy/local-build.json').read_bytes())
assert s['running'] and s['ready'] and s['binary']['version']=='v0.1.1-dev.284'
assert s['binary_sha256']==b['binary_sha256']=='874feeb290283e464711021f072e92e8973e6ac75be07b2d7994733eaaba954e'
assert hashlib.sha256(Path('/proc',str(s['pid']),'exe').read_bytes()).hexdigest()==b['binary_sha256']
address=s['url'].split('://',1)[1].rstrip('/')
request=session_request(address,Path(s['root']),'/api/v1/native-version')
with urllib.request.urlopen(request) as response: actual=json.load(response)
assert actual['binary']==s['binary']
notes=Path('version_history.json').read_bytes()
assert hashlib.sha256(notes).hexdigest()==b['effective_inputs']['version_history.json']
assert actual['history']['local_changes']==json.loads(notes)['local_changes']
(out/'version.json').write_text(json.dumps(actual,ensure_ascii=False,indent=2)+'\n')
cfg=dict(base=s['url'].rstrip('/'),cookie=request.get_header('Cookie'),expected=actual,output=str(out))
env=dict(os.environ);env['TMPDIR']=str(fixture_area())
ui=subprocess.run(['node','tests/rust_installed_version_read.cjs'],input=json.dumps(cfg).encode(),capture_output=True,env=env)
(out/'ui.stdout').write_bytes(ui.stdout);(out/'ui.stderr').write_bytes(ui.stderr)
assert ui.returncode==0, 'installed Version dialog check failed; see retained stderr'
result=dict(status='PASS',binary_sha256=b['binary_sha256'],version=s['binary']['version'],pid=s['pid'],checks=['loaded-image','protected-version','compiled-current-fr-en-notes','version-dialog-fr-en'],source_profile=b['evidence'],global_watchdog=None)
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(ui.stdout.decode(),end='')
print('BLP_INSTALLED_PASS '+s['binary']['version']+' '+b['binary_sha256'])
