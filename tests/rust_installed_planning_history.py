"""Read-only exact local planning-history delivery proof; live records are hashed, never reported; no mission/provider writes.
Optional fourth argument binds a retained build manifest to the observed server
when another owner prepares a newer candidate. All SHA/version checks still run.
"""
import sys,json,hashlib,tarfile,subprocess,urllib.request,re
from pathlib import Path
sys.path.insert(0,'tools');sys.path.insert(0,'tests')
from swarm_version import session_request
import swarm_local
from rust_agent_model_route import publish,digest
root=Path.cwd();cfg=json.loads((root/'deploy/local-web.json').read_bytes());address=cfg['address'];project=Path(cfg['root']);out=Path(sys.argv[2]);out.mkdir(parents=True,exist_ok=False)
def get(path):
    """Protected GET only; cookie private, no response discarded as success.
    Harness HTML normalizes only its single verified CSP-bound nonce insertion."""
    with urllib.request.urlopen(session_request(address,project,path),timeout=None) as r:
        raw=r.read()
        if path=='/live-session.html':
            # Match the actual documented one-slot transform, not arbitrary HTML.
            matches=list(re.finditer(rb'<meta name="swarm-style-nonce" content="([a-zA-Z0-9_-]+)">',raw));assert len(matches)==1
            nonce=matches[0].group(1).decode('ascii');assert "'nonce-"+nonce+"'" in r.headers['Content-Security-Policy']
            raw=raw[:matches[0].start()]+b'<meta name="swarm-style-nonce" content="">'+raw[matches[0].end():]
        return raw
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
if sys.argv[1]=='snapshot':print('INSTALLED_PLANNING_HISTORY_SNAPSHOT');sys.exit(0)
before=json.loads(Path(sys.argv[3]).read_bytes());build=json.loads((Path(sys.argv[4]) if len(sys.argv)>4 else root/'deploy/local-build.json').read_bytes());candidate=json.loads((Path(build['evidence'])/'build/result.json').read_bytes());checks=[]
def check(name,condition):
    """Retain exact named observed assertions, never credential bytes."""
    assert condition,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)
check('actual-process-matches-prepared-sha',now['sha256']==candidate['sha256']==build['binary_sha256'])
expected={**{k:v for k,v in candidate['metadata'].items() if k!='schema_version'},'provenance':'injected'};check('actual-version-matches-metadata',now['version']['binary']==expected)
check('works-unchanged',before['works']==now['works']);notes=now['version']['history']['local_changes'];check('planning-history-notes-fr-en',any('Historique natif des décisions de planification' in v['fr'] and 'Native planning decision history' in v['en'] for v in notes['items']))
with tarfile.open(Path(candidate['profile']).parent/'effective-sources.tar.gz') as archive:
 compiled=json.loads(archive.extractfile('version_history.json').read());check('served-notes-match-compiled',notes==compiled['local_changes']);publish(out/'compiled-version-history.json',compiled)
 for url,name in {'/native-runtime/recorded-cursor.js':'web/recorded-cursor.js','/native-runtime/runtime-health.js':'web/native-runtime-health.js','/native-runtime.html':'web/native-runtime.html','/native-runtime/log-read-policy.js':'web/native-log-read-policy.js','/live/log-query.js':'web/native-log-query.js','/live/recorded-cursor.js':'web/recorded-cursor.js','/live/live-session.js':'web/live-session.js','/live-session.html':'web/live-session.html','/native-runtime/cockpit.js':'web/native-cockpit.js','/native-runtime/app.css':'web/native-runtime.css','/native-runtime/graph-performance.js':'web/native-graph-performance.js','/native-runtime/policy-receipt.js':'web/native-policy-receipt.js','/native-runtime/provider-wait.js':'web/native-provider-wait.js','/native-runtime/run-limits.js':'web/native-run-limits.js','/native-runtime/web-requests-admin.js':'web/native-web-requests.js','/native-runtime/web-requests.js':'web/web-requests.js','/native-runtime/app.js':'web/native-runtime.js','/native-runtime/admin.css':'frontend/engine-settings/admin.css','/native-runtime/english.js':'web/i18n-en.js'}.items():check('served-'+name,digest(get(url))==digest(archive.extractfile(name).read()))
works=json.loads(get('/api/v1/works'));check('actual-works-nonempty-for-readback',bool(works))
if works:
    # Actual IDs/content remain in memory only. Public show is readonly; no
    # mutation, fault, settings adoption or lifecycle action on a real Root.
    from urllib.parse import urlencode
    work=works[0]['id'];view=json.loads(get('/api/v1/planning-history?'+urlencode({'work':work})));cli=json.loads(subprocess.check_output([candidate['binary'],'planning','history',work,'--root',str(project),'--json']))
    check('actual-history-cli-http-equal',cli==view);check('actual-history-array',isinstance(view,list));check('actual-history-public-fields',all(set(e)=={'id','at','scope','reason','operations'} for e in view))
    publish(out/'history-readback-signature.json',{'entries':len(view),'sha256':digest(json.dumps(view,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()),'live_record_content_retained':False,'mutation_requested':False})

cookie=session_request(address,project,'/').get_header('Cookie');payload={'base':'http://'+address,'cookie':cookie,'expected':now['version'],'output':str(out)}
with (out/'ui.stdout').open('xb') as stdout,(out/'ui.stderr').open('xb') as stderr:r=subprocess.run(['node','tests/rust_installed_version_read.cjs'],input=json.dumps(payload).encode(),stdout=stdout,stderr=stderr)
check('actual-version-dialog-fr-en-14-assertions',r.returncode==0 and len(json.loads((out/'installed-ui.json').read_bytes())['checks'])==14)
check('works-still-unchanged',work_signature(json.loads(get('/api/v1/works')))==now['works']);publish(out/'result.json',{'status':'PASS','checks':checks,'binary':expected,'sha256':now['sha256'],'read_only':True,'provider_started':False});print('INSTALLED_PLANNING_HISTORY_VERIFIED',len(checks))
