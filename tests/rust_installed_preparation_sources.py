#!/usr/bin/env python3
"""Read-only local delivery proof: process, embedded assets, versions and state.

Explicit URL/Root/work identifiers select the authorized local instance. Session
credentials remain in memory, never in retained channels. Snapshot and verify
mutate only their private evidence directories, never the application Root.
No global watchdog; no fixture/provider execution or acceptance is inferred.
"""
import argparse,hashlib,json,subprocess,sys,tarfile,urllib.error,urllib.parse,urllib.request
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools'))
import swarm_local
from swarm_version import session_request
sys.path.insert(0,str(Path(__file__).resolve().parent))
from rust_agent_model_route import publish


def provider_capability_redacted(value):
    """Validate metadata-only fixed API or per-level CLI observations.

    Known mode fields are protocol metadata; unavailable routes carry only safe
    reasons and empty model/hash fields. This read grants no execution and does
    not authenticate the selected provider or prove remote availability.
    """
    if not isinstance(value,dict) or not {'name','model','digest'}.issubset(value) or not set(value).issubset({'name','model','digest','levels'}):
        return False
    if not all(isinstance(value[k],str) for k in ['name','model','digest']):
        return False
    if 'levels' not in value:
        return bool(value['name'] and value['model'] and value['digest'])
    levels=value['levels']
    if not isinstance(levels,list) or len(levels)!=4 or {m.get('level') for m in levels if isinstance(m,dict)}!={'auto','simple','standard','exigeant'}:
        return False
    for mode in levels:
        if not isinstance(mode,dict) or not {'level','available','model','digest','policy_hash'}.issubset(mode) or not set(mode).issubset({'level','available','model','digest','policy_hash','reason'}) or type(mode['available']) is not bool:
            return False
        if not all(isinstance(mode[k],str) for k in ['model','digest','policy_hash']):
            return False
        if mode['available']:
            if not all(mode[k] for k in ['model','digest','policy_hash']):return False
        elif any(mode[k] for k in ['model','digest','policy_hash']) or not isinstance(mode.get('reason'),str) or not mode['reason']:
            return False
    auto=next(m for m in levels if m['level']=='auto')
    return any(m['available'] for m in levels) and value['model']==auto['model'] and value['digest']==auto['digest']

def main():
    """Compare current installed observations with an explicit pre-restart record."""
    ap=argparse.ArgumentParser();ap.add_argument('--root',required=True);ap.add_argument('--url',required=True);ap.add_argument('--output',required=True);ap.add_argument('--work',action='append',default=[]);ap.add_argument('--before');ap.add_argument('--candidate');ap.add_argument('--asset',action='append',default=[]);ap.add_argument('--note-fr');ap.add_argument('--note-en');ap.add_argument('--templates',action='store_true');ap.add_argument('--project-profile',action='store_true');ap.add_argument('--release-review',action='store_true');ap.add_argument('--dialogue',action='store_true');ap.add_argument('--calls',action='store_true');ap.add_argument('--profile-config',action='store_true');a=ap.parse_args();root=Path(a.root).resolve();out=Path(a.output).resolve();out.mkdir(mode=0o700,parents=True,exist_ok=False);address=urllib.parse.urlsplit(a.url).netloc;state=json.loads((root/'.swarm/local-web'/hashlib.sha256(address.encode()).hexdigest()[:16]/'server.json').read_text());assert swarm_local.owned(state),'Local instance ownership missing';checks=[]
    def check(name,value):
        """Retain named real observations immediately without exposing credentials."""
        assert value,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def get(path):
        """Protected read only, retaining the actual transport cause on failure."""
        with urllib.request.urlopen(session_request(address,root,path),timeout=None) as r:return r.read()
    version=json.loads(get('/api/v1/native-version'));settings=json.loads(get('/api/v1/native-settings'));sha=hashlib.sha256(Path('/proc/'+str(state['pid'])+'/exe').read_bytes()).hexdigest();missions={}
    for work in a.work:
        value=json.loads(subprocess.check_output([str(root/'bin/swarm'),'mission','show',work,'--root',str(root),'--json']));missions[work]={k:value[k] for k in ['revision','paused','autonomy','slots','consent_enabled','archived']}
    observed=dict(pid=state['pid'],sha256=sha,version=version,settings=settings,missions=missions);publish(out/'snapshot.json',observed)
    if not a.before:print('INSTALLED_PREPARATION_SOURCES_SNAPSHOT');return
    assert a.candidate,'Candidate evidence required for verification';before=json.loads(Path(a.before).read_text());candidate=json.loads(Path(a.candidate).read_text());check('installed-process-matches-qualified-sha',sha==candidate['sha256']);expected={**{k:v for k,v in candidate['metadata'].items() if k!='schema_version'},'provenance':'injected'};check('installed-version-matches-qualified-metadata',version['binary']==expected);check('installed-bilingual-source-note',any(i['fr'].startswith('Préparation Rust : découverte et aperçu des sources') and i['en'].startswith('Rust preparation: discover and preview sources') for i in version['history']['local_changes']['items']));check('installed-settings-preserved',settings==before['settings']);check('installed-missions-preserved',missions==before['missions'])
    if a.note_fr or a.note_en:
        assert a.note_fr and a.note_en,'Both note languages required'
        check('installed-feature-bilingual-note',any(i['fr'].startswith(a.note_fr) and i['en'].startswith(a.note_en) for i in version['history']['local_changes']['items']))
    extras={}
    for entry in a.asset:
        url,separator,name=entry.partition('=');assert separator and url.startswith('/') and not name.startswith('/') and '..' not in Path(name).parts,'Invalid asset expectation';extras[url]=name
    profile=Path(candidate['profile']).parent
    with tarfile.open(profile/'effective-sources.tar.gz') as archive:
        for url,name in {'/prepare.html':'web/prepare.html','/preparation-native.js':'web/preparation-native.js','/preparation-native-sources.js':'web/preparation-native-sources.js','/prephase.css':'web/prephase.css','/i18n-en.js':'web/i18n-en.js','/native-runtime/settings.js':'frontend/engine-settings/admin.js',**extras}.items():
            raw=get(url);expected_raw=archive.extractfile(name).read()
            if name=='web/prepare.html':
                # Native renderer intentionally replaces the legacy head and
                # makes create/title/need inert until policy has been read.
                body=expected_raw.decode().split('<body ',1)[1]
                for ident,attribute in [('create','disabled'),('new-title','readonly'),('new-need','readonly')]:
                    marker='id="'+ident+'"';assert body.count(marker)==1;body=body.replace(marker,marker+' '+attribute,1)
                served=raw.decode();check('installed-rendered-template-body',served.split('<body ',1)[1]==body and 'src="/preparation-native.js"' in served and 'src="/prephase.js"' not in served)
            else:check('installed-served-'+name,hashlib.sha256(raw).digest()==hashlib.sha256(expected_raw).digest())
    if a.release_review:
        preparations=json.loads(get('/api/v1/preparations'))['preparations'];p=next((p for p in preparations if p.get('conversion') and p.get('work_id')==p['conversion']['work_id']),None)
        assert p is not None,'No existing converted preparation for readonly installation proof'
        path='/api/v1/preparations/release-review?id='+urllib.parse.quote(p['id'],safe='');review=json.loads(get(path));cli=json.loads(subprocess.check_output([candidate['binary'],'prepare','release-review',p['id'],'--root',str(root),'--json']))
        check('installed-release-review-cli-http-equal',review==cli)
        check('installed-release-review-real-association',review['preparation']['id']==p['id'] and review['work']['id']==p['conversion']['work_id'])
        publish(out/'release-review.json',review)
    if a.calls:
        # Read only: parity and redaction are observed without enabling a policy
        # or contacting a configured API. Historical policy absence stays explicit.
        try:capabilities=json.loads(get('/api/v1/preparations/capabilities'));status=200
        except urllib.error.HTTPError as error:
            status=error.code;capabilities=json.load(error)
        process=subprocess.run([candidate['binary'],'prepare','capabilities','--root',str(root),'--json'],capture_output=True)
        publish(out/'capabilities-cli.stdout',process.stdout);publish(out/'capabilities-cli.stderr',process.stderr)
        publish(out/'capabilities-http.json',dict(status=status,body=capabilities))
        if status==200:
            check('installed-call-capabilities-cli-http-equal',process.returncode==0 and json.loads(process.stdout)==capabilities)
            check('installed-call-capabilities-redacted',{'enabled','policy','providers'}.issubset(capabilities) and set(capabilities).issubset({'enabled','policy','providers','unavailable_providers'}) and isinstance(capabilities['enabled'],bool) and isinstance(capabilities['providers'],list) and all(provider_capability_redacted(p) for p in capabilities['providers']) and isinstance(capabilities.get('unavailable_providers',[]),list) and all(set(p)=={'name','reason'} and all(isinstance(v,str) for v in p.values()) for p in capabilities.get('unavailable_providers',[])))
        else:
            check('installed-call-capabilities-explicit-refusal',status in (409,501) and process.returncode!=0 and json.loads(process.stderr)['failure']['code']==capabilities['failure']['code'])
    profile_catalogue=None
    if a.profile_config:
        profile_catalogue=json.loads(get('/api/v1/project-profile/catalog'))
        process=subprocess.run([candidate['binary'],'project-profile','list','--root',str(root),'--json'],capture_output=True)
        publish(out/'profile-catalogue-cli.stdout',process.stdout);publish(out/'profile-catalogue-cli.stderr',process.stderr);publish(out/'profile-catalogue-http.json',profile_catalogue)
        check('installed-profile-catalogue-cli-http-equal',process.returncode==0 and json.loads(process.stdout)==profile_catalogue)
        check('installed-profile-catalogue-redacted',isinstance(profile_catalogue.get('choices'),list) and len(profile_catalogue['choices'])==6 and 'cause' not in profile_catalogue and all('cause' not in choice and 'prompt' not in choice and set(choice['profile']).issubset({'version','name','source','instructions'}) for choice in profile_catalogue['choices']))
    dialogue_observation=None
    if a.dialogue:
        preparations=json.loads(get('/api/v1/preparations'))['preparations'];assert preparations,'No existing preparation for readonly history installation proof';selected=preparations[0]['id'];path='/api/v1/preparations/dialogue?id='+urllib.parse.quote(selected,safe='')
        try:history=json.loads(get(path));status=200
        except urllib.error.HTTPError as error:
            status=error.code;history=json.load(error);assert status==501 and history['failure']['code']=='preparation-dialogue-active-unported','Unexpected history refusal'
        process=subprocess.run([candidate['binary'],'prepare','dialogue',selected,'--root',str(root),'--json'],capture_output=True);publish(out/'dialogue-cli.stdout',process.stdout);publish(out/'dialogue-cli.stderr',process.stderr)
        if status==200:check('installed-dialogue-cli-http-equal',process.returncode==0 and json.loads(process.stdout)==history and all('prompt' not in t for t in history))
        else:check('installed-dialogue-cli-http-refusal-equal',process.returncode!=0 and json.loads(process.stderr)['failure']['code']==history['failure']['code'])
        dialogue_observation=dict(id=selected,status=status,count=len(history) if status==200 else None);publish(out/'dialogue-http.json',history)
    profile_observation=None
    if a.project_profile:
        profile_observation=json.loads(get('/api/v1/project-profile'))
        check('installed-project-profile-explicit-arrays',isinstance(profile_observation.get('enabled'),bool) and isinstance(profile_observation.get('roles'),list) and isinstance(profile_observation.get('claude_settings_detected'),list))
        publish(out/'project-profile.json',profile_observation)
    cfg=dict(dialogue_observation=dialogue_observation,profile_observation=profile_observation,base='http://'+address,cookie=session_request(address,root,'/').get_header('Cookie'),expected=version,output=str(out),templates=a.templates,template_catalogue_configured='preparation_plan_max_bytes' in settings['values'])
    with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:r=subprocess.run(['node',str(root/'tests/rust_installed_version_read.cjs')],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
    check('installed-version-dialog-fr-en',r.returncode==0)
    with (out/'page.stdout').open('wb') as stdout,(out/'page.stderr').open('wb') as stderr:r=subprocess.run(['node',str(root/'tests/rust_installed_preparation_sources_ui.cjs')],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
    check('installed-preparation-readonly-page',r.returncode==0)
    if a.profile_config:check('installed-profile-catalogue-no-implicit-save',json.loads(get('/api/v1/project-profile/catalog'))==profile_catalogue)
    check('installed-process-still-owned',swarm_local.owned(state));publish(out/'result.json',dict(status='PASS',checks=checks,version=version['binary'],sha256=sha,read_only=True));print('INSTALLED_PREPARATION_SOURCES_PASS '+str(len(checks)))
def locked_main():
    """Hold the managed service lifecycle lock across this read-only proof.

    Open the existing coordination file read-only: no Root file is created or
    written. This prevents supported concurrent restarts from mixing binaries
    during asset/browser checks; it grants no authority or source-edit freeze.
    CLI/API writers can still change application data, which comparisons detect.
    IO/browser errors propagate and the context releases the owned advisory lock.
    """
    import fcntl
    parser=argparse.ArgumentParser(add_help=False);parser.add_argument('--root',required=True);parser.add_argument('--url',required=True)
    selected,_=parser.parse_known_args();address=urllib.parse.urlsplit(selected.url).netloc
    path=Path(selected.root).resolve()/'.swarm/local-web'/hashlib.sha256(address.encode()).hexdigest()[:16]/'lifecycle.lock'
    with path.open('rb') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        main()
if __name__=='__main__':locked_main()
