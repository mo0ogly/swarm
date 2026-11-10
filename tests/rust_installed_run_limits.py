"""Read-only exact local run-limits delivery proof; no mission/provider writes."""
import sys,json,hashlib,tarfile,subprocess,urllib.request
from pathlib import Path
sys.path.insert(0,'tools');sys.path.insert(0,'tests')
from swarm_version import session_request
import swarm_local
from rust_agent_model_route import publish,digest
root=Path.cwd();cfg=json.loads((root/'deploy/local-web.json').read_bytes());address=cfg['address'];project=Path(cfg['root']);out=Path(sys.argv[2]);out.mkdir(parents=True,exist_ok=False)
def get(path):
    """Protected GET only; cookie private, no response discarded as success."""
    with urllib.request.urlopen(session_request(address,project,path),timeout=None) as r:return r.read()
def snapshot():
    """Observe actual OS-owned process and public state without changing Root."""
    state=json.loads((project/'.swarm/local-web'/hashlib.sha256(address.encode()).hexdigest()[:16]/'server.json').read_bytes());assert swarm_local.owned(state)
    return {'pid':state['pid'],'sha256':digest(Path('/proc/'+str(state['pid'])+'/exe').read_bytes()),'version':json.loads(get('/api/v1/native-version')),'works':json.loads(get('/api/v1/works'))}
now=snapshot();publish(out/'snapshot.json',now)
if sys.argv[1]=='snapshot':print('INSTALLED_RUN_LIMITS_SNAPSHOT');sys.exit(0)
before=json.loads(Path(sys.argv[3]).read_bytes());build=json.loads((root/'deploy/local-build.json').read_bytes());candidate=json.loads((Path(build['evidence'])/'build/result.json').read_bytes());checks=[]
def check(name,condition):
    """Retain exact named observed assertions, never credential bytes."""
    assert condition,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)
check('actual-process-matches-prepared-sha',now['sha256']==candidate['sha256']==build['binary_sha256'])
expected={**{k:v for k,v in candidate['metadata'].items() if k!='schema_version'},'provenance':'injected'};check('actual-version-matches-metadata',now['version']['binary']==expected)
check('works-unchanged',before['works']==now['works']);notes=now['version']['history']['local_changes'];check('run-limits-notes-fr-en',any('Limites d’exécution Rust' in v['fr'] and 'Rust execution limits' in v['en'] for v in notes['items']))
with tarfile.open(Path(candidate['profile']).parent/'effective-sources.tar.gz') as archive:
 compiled=json.loads(archive.extractfile('version_history.json').read());check('served-notes-match-compiled',notes==compiled['local_changes']);publish(out/'compiled-version-history.json',compiled)
 for url,name in {'/native-runtime/run-limits.js':'web/native-run-limits.js','/native-runtime/app.js':'web/native-runtime.js','/native-runtime/admin.css':'frontend/engine-settings/admin.css','/native-runtime/english.js':'web/i18n-en.js'}.items():check('served-'+name,digest(get(url))==digest(archive.extractfile(name).read()))
view=json.loads(get('/api/v1/run-limits?scope=project'));cli=json.loads(subprocess.check_output([candidate['binary'],'run-limits','show','project','-','-','--root',str(project),'--json']));check('real-root-cli-http-equal',view['entry']==cli);check('actual-admin-exact-fields',view['exact']['entry']['revision']==str(cli['revision']) and all(isinstance(view['exact']['entry']['values'][name],str) for name in ['observation_mode','silence_seconds','tool_seconds','max_tool_calls','max_repeated_calls','max_consecutive_errors']));publish(out/'run-limits-readback.json',view)
cookie=session_request(address,project,'/').get_header('Cookie');payload={'base':'http://'+address,'cookie':cookie,'expected':now['version'],'output':str(out)}
with (out/'ui.stdout').open('xb') as stdout,(out/'ui.stderr').open('xb') as stderr:r=subprocess.run(['node','tests/rust_installed_version_read.cjs'],input=json.dumps(payload).encode(),stdout=stdout,stderr=stderr)
check('actual-version-dialog-fr-en-14-assertions',r.returncode==0 and len(json.loads((out/'installed-ui.json').read_bytes())['checks'])==14)
check('works-still-unchanged',json.loads(get('/api/v1/works'))==now['works']);publish(out/'result.json',{'status':'PASS','checks':checks,'binary':expected,'sha256':now['sha256'],'read_only':True,'provider_started':False});print('INSTALLED_RUN_LIMITS_VERIFIED',len(checks))
