"""Read-only exact local activity delivery proof; live records are hashed, never reported; no mission/provider writes.
Optional fourth argument binds a retained build manifest to the observed server
when another owner prepares a newer candidate. All SHA/version checks still run.
"""
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
def work_signature(works):
    """Compare complete actual Work data without retaining private live content.
    Canonical digest/count prove unchanged public values only, not DB isolation.
    """
    assert isinstance(works,list) and all(isinstance(w,dict) and isinstance(w.get('id'),str) for w in works)
    return {'count':len(works),'sha256':digest(json.dumps(works,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode())}
def snapshot():
    """Observe actual OS-owned process and public state without changing Root."""
    state=json.loads((project/'.swarm/local-web'/hashlib.sha256(address.encode()).hexdigest()[:16]/'server.json').read_bytes());assert swarm_local.owned(state)
    return {'pid':state['pid'],'sha256':digest(Path('/proc/'+str(state['pid'])+'/exe').read_bytes()),'version':json.loads(get('/api/v1/native-version')),'works':work_signature(json.loads(get('/api/v1/works')))}
now=snapshot();publish(out/'snapshot.json',now)
if sys.argv[1]=='snapshot':print('INSTALLED_ACTIVITY_SNAPSHOT');sys.exit(0)
before=json.loads(Path(sys.argv[3]).read_bytes());build=json.loads((Path(sys.argv[4]) if len(sys.argv)>4 else root/'deploy/local-build.json').read_bytes());candidate=json.loads((Path(build['evidence'])/'build/result.json').read_bytes());checks=[]
def check(name,condition):
    """Retain exact named observed assertions, never credential bytes."""
    assert condition,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)
check('actual-process-matches-prepared-sha',now['sha256']==candidate['sha256']==build['binary_sha256'])
expected={**{k:v for k,v in candidate['metadata'].items() if k!='schema_version'},'provenance':'injected'};check('actual-version-matches-metadata',now['version']['binary']==expected)
check('works-unchanged',before['works']==now['works']);notes=now['version']['history']['local_changes'];check('activity-notes-fr-en',any('Fil d’activité natif' in v['fr'] and 'Native activity feed' in v['en'] for v in notes['items']))
with tarfile.open(Path(candidate['profile']).parent/'effective-sources.tar.gz') as archive:
 compiled=json.loads(archive.extractfile('version_history.json').read());check('served-notes-match-compiled',notes==compiled['local_changes']);publish(out/'compiled-version-history.json',compiled)
 for url,name in {'/native-runtime/activity-policy.js':'web/native-activity-policy.js','/native-runtime/cockpit.js':'web/native-cockpit.js','/native-runtime/app.css':'web/native-runtime.css','/native-runtime/graph-performance.js':'web/native-graph-performance.js','/native-runtime/policy-receipt.js':'web/native-policy-receipt.js','/native-runtime/provider-wait.js':'web/native-provider-wait.js','/native-runtime/run-limits.js':'web/native-run-limits.js','/native-runtime/web-requests-admin.js':'web/native-web-requests.js','/native-runtime/web-requests.js':'web/web-requests.js','/native-runtime/app.js':'web/native-runtime.js','/native-runtime/admin.css':'frontend/engine-settings/admin.css','/native-runtime/english.js':'web/i18n-en.js'}.items():check('served-'+name,digest(get(url))==digest(archive.extractfile(name).read()))
works=json.loads(get('/api/v1/works'));check('actual-work-control-nonempty',bool(works));work=works[0]['id'];view=json.loads(get('/api/v1/activity?work='+work));cli=json.loads(subprocess.check_output([candidate['binary'],'activity','show',work,'--root',str(project),'--json']));check('actual-activity-cli-http-equal',view==cli);check('actual-activity-page-shaped',isinstance(view['entries'],list) and isinstance(view['more'],bool) and isinstance(view['history_truncated'],bool));publish(out/'activity-readback-signature.json',{'rows':len(view['entries']),'sha256':digest(json.dumps(view,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()),'live_record_content_retained':False})
policy=json.loads(get('/api/v1/activity-policy'));check('actual-policy-cli-http-equal',policy==json.loads(subprocess.check_output([candidate['binary'],'run-limits','activity','show','--root',str(project),'--json'])));publish(out/'policy-readback.json',policy)
cookie=session_request(address,project,'/').get_header('Cookie');payload={'base':'http://'+address,'cookie':cookie,'expected':now['version'],'output':str(out)}
with (out/'ui.stdout').open('xb') as stdout,(out/'ui.stderr').open('xb') as stderr:r=subprocess.run(['node','tests/rust_installed_version_read.cjs'],input=json.dumps(payload).encode(),stdout=stdout,stderr=stderr)
check('actual-version-dialog-fr-en-14-assertions',r.returncode==0 and len(json.loads((out/'installed-ui.json').read_bytes())['checks'])==14)
check('works-still-unchanged',work_signature(json.loads(get('/api/v1/works')))==now['works']);publish(out/'result.json',{'status':'PASS','checks':checks,'binary':expected,'sha256':now['sha256'],'read_only':True,'provider_started':False});print('INSTALLED_ACTIVITY_VERIFIED',len(checks))
