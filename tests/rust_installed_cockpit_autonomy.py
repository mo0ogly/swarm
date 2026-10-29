#!/usr/bin/env python3
"""Verify exact installed autonomy delivery without changing live data or policy.

Explicit candidate receipt, protected asset bytes, saved consumer observations,
FR/EN version dialog and read-only Admin drafts are checked. Live record contents
are hashed rather than reported. No POST, provider, fixture or global watchdog.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import urllib.request
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from swarm_version import session_request
import swarm_local


def main():
    """Bind the owned process to a qualified receipt and verify read-only effects.

    Missing/changed process, metadata, assets, journal choice or observed mode
    refuses. Browser failures retain original channels. Root/HTTP/IO errors
    propagate; no setting adoption, retry or process interruption is attempted.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--candidate-record',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False,mode=0o700)
    source=Path(__file__).resolve().parents[1];cfg=json.loads((source/'deploy/local-web.json').read_bytes());root=Path(cfg['root']);address=cfg['address']
    sha=lambda data:hashlib.sha256(data).hexdigest()
    def get(path):
        """Protected GET with no deadline or fallback; return complete bytes."""
        with urllib.request.urlopen(session_request(address,root,path),timeout=None) as response:return response.read()
    def read(path):return json.loads(get(path))
    def signature(value):return sha(json.dumps(value,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode())
    candidate=json.loads(args.candidate_record.read_bytes());assert candidate['status']=='VERIFIED_NATIVE_CANDIDATE'
    state_path=root/'.swarm/local-web'/sha(address.encode())[:16]/'server.json';state=json.loads(state_path.read_bytes());assert swarm_local.owned(state)
    assert sha(Path('/proc/'+str(state['pid'])+'/exe').read_bytes())==candidate['sha256']
    version=read('/api/v1/native-version');assert all(version['binary'][k]==v for k,v in candidate['metadata'].items() if k!='schema_version')
    profile=Path(candidate['profile']).parent;qualification=json.loads((profile/'result.json').read_bytes());archive=profile/'effective-sources.tar.gz'
    assert qualification['sha256']==candidate['sha256'] and sha(archive.read_bytes())==qualification['effective_sources_archived']['sha256']
    with tarfile.open(archive) as retained:
        history=json.loads(retained.extractfile('version_history.json').read())
        assert version['history']['local_changes']==history['local_changes']
        assert any(i['fr'].startswith('Cockpit : autonomie initiale configurable') for i in history['local_changes']['items'])
        for route,name in {'/native-runtime/settings.js':'frontend/engine-settings/admin.js','/native-runtime/admin.css':'frontend/engine-settings/admin.css','/native-runtime/english.js':'web/i18n-en.js','/cockpit-native-panels.js':'web/cockpit-native-panels.js'}.items():assert sha(get(route))==sha(retained.extractfile(name).read())
    settings=read('/api/v1/native-settings');assert settings['values']['cockpit_default_autonomy']=='2'
    works=read('/api/v1/works');controls={}
    for work in works:
        control=read('/api/v1/mission?work='+work['id']);controls[work['id']]=control
        observed=read('/api/v1/cockpit-read?work='+work['id'])['autonomy_observation']
        assert observed['autonomy']==(control['autonomy'] or 'autonome')
        assert observed['source']==('mission' if control['autonomy'] is not None else 'installation')
        assert observed['consent_enabled']==(control['consent_enabled'] is True) and observed['launch_authorized'] is False
    cookie=session_request(address,root,'/').get_header('Cookie')
    for script in ['rust_installed_version_read.cjs','rust_installed_cockpit_autonomy_ui.cjs']:
        payload=json.dumps(dict(base='http://'+address,cookie=cookie,expected=version,output=str(out))).encode()
        with (out/(script+'.stdout')).open('xb') as stdout,(out/(script+'.stderr')).open('xb') as stderr:
            result=subprocess.run(['node',str(source/'tests'/script)],input=payload,stdout=stdout,stderr=stderr)
        assert result.returncode==0,('installed-browser-failed',script)
    assert read('/api/v1/native-settings')==settings and read('/api/v1/works')==works
    assert {work['id']:read('/api/v1/mission?work='+work['id']) for work in works}==controls
    assert swarm_local.owned(state) and sha(Path('/proc/'+str(state['pid'])+'/exe').read_bytes())==candidate['sha256']
    report=dict(status='PASS',pid=state['pid'],sha256=candidate['sha256'],version=version['binary'],works_count=len(works),works_sha256=signature(works),mission_controls_sha256=signature(controls),settings_sha256=signature(settings),protected_assets=4,read_only=True,provider_started=False,global_watchdog=None)
    (out/'result.json').write_text(json.dumps(report,indent=2)+'\n');print('INSTALLED_COCKPIT_AUTONOMY_VERIFIED',len(works))


if __name__=='__main__':main()
