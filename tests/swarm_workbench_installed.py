"""Bind the integrated design to the owned local process and protected assets.

Only GET reads and browser presentation changes are allowed. No live mission,
provider, policy, private token or active Work content enters retained evidence.
"""
import argparse,hashlib,json,os,subprocess,sys,tarfile,urllib.request
from pathlib import Path
sys.path.insert(0,'tools')
from swarm_version import session_request
import swarm_local


def main():
    """Snapshot before delivery or verify its actual image, notes and UI after it.

    Requires the configured local project and its private owned session file.
    All primary errors propagate; no retry, alternate credential or empty success.
    Browser cookies travel on stdin only. Evidence stores public-CLI hashes of
    live data; this never claims business HTTP access from a probe cookie.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode',choices=['before','after']);parser.add_argument('--output',required=True);parser.add_argument('--before')
    args=parser.parse_args();out=Path(args.output);out.mkdir(parents=True,exist_ok=False)
    source=Path.cwd();cfg=json.loads((source/'deploy/local-web.json').read_bytes());address=cfg['address'];root=Path(cfg['root'])
    def get(path):
        """Read one protected local resource, retaining neither credential nor body."""
        with urllib.request.urlopen(session_request(address,root,path),timeout=None) as response:return response.read()
    state=json.loads((root/'.swarm/local-web'/hashlib.sha256(address.encode()).hexdigest()[:16]/'server.json').read_bytes());assert swarm_local.owned(state),'Local process identity changed'
    version=json.loads(get('/api/v1/native-version'))
    def state_reads():
        """Hash supported read-only CLI output with the actual live image.

        The delivery cookie is restricted to probes in authority mode. These
        hashes do not establish an authenticated business-API/browser session.
        No private Work body is written into evidence or printed.
        """
        image='/proc/'+str(state['pid'])+'/exe'
        commands={'CLI work list':['work','list'],'CLI engine-settings show':['engine-settings','show']}
        return {name:hashlib.sha256(subprocess.check_output([image,'--root',str(root),*command,'--json'])).hexdigest() for name,command in commands.items()}
    reads=state_reads()
    snapshot=dict(pid=state['pid'],binary_sha256=hashlib.sha256(Path('/proc/'+str(state['pid'])+'/exe').read_bytes()).hexdigest(),binary=version['binary'],live_reads=reads)
    (out/'snapshot.json').write_text(json.dumps(snapshot,indent=2))
    if args.mode=='before':print('INSTALLED_DESIGN_SNAPSHOT');return
    assert args.before,'Before snapshot is required';before=json.loads(Path(args.before).read_text());build=json.loads((source/'deploy/local-build.json').read_bytes());checks=[]
    def check(name,condition):
        """Retain each named assertion before raising on a failed live observation."""
        checks.append(dict(name=name,pass_=bool(condition)));(out/'checks.json').write_text(json.dumps(checks,indent=2));assert condition,name;print('PASS '+name,flush=True)
    check('actual-process-is-prepared-image',snapshot['binary_sha256']==build['binary_sha256'])
    check('live-data-and-settings-unchanged',before['live_reads']==reads)
    proof=json.loads((Path(build['evidence'])/'build/result.json').read_text());profile=json.loads(Path(proof['profile']).read_text())
    check('production-profile-no-fixtures',profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and all(profile[k] is True for k in ['all_core_features_empty','all_core_test_false','no_fixture_feature','actual_dependencies_stable']))
    with tarfile.open(Path(proof['profile']).parent/'effective-sources.tar.gz') as archive:
        check('served-notes-equal-compiled-notes',version['history']['local_changes']==json.loads(archive.extractfile('version_history.json').read())['local_changes'])
        for url,name in {'/swarm-workbench.js':'web/swarm-workbench.js','/swarm-workbench.css':'web/swarm-workbench.css','/trust-workbench.js':'web/trust-workbench.js','/cockpit-native.js':'web/cockpit-native.js','/cockpit-native-read.js':'web/cockpit-native-read.js','/swarm-design.css':'web/swarm-design.css','/swarm-shell.js':'web/swarm-shell.js','/trust-actions.html':'web/trust-native.html','/native-runtime/run-limits.js':'web/native-run-limits.js','/i18n-en.js':'web/i18n-en.js'}.items():check('actual-protected-bytes-'+url,hashlib.sha256(get(url)).hexdigest()==hashlib.sha256(archive.extractfile(name).read()).hexdigest())
    payload={'base':'http://'+address,'cookie':session_request(address,root,'/').get_header('Cookie'),'expected':version,'output':str(out)}
    for script,marker in [('tests/rust_installed_version_read.cjs','version'),('tests/swarm_workbench_installed_ui.cjs','design')]:
        with (out/(marker+'.stdout')).open('xb') as stdout,(out/(marker+'.stderr')).open('xb') as stderr:result=subprocess.run(['node',script],input=json.dumps(payload).encode(),stdout=stdout,stderr=stderr)
        check('actual-browser-'+marker,result.returncode==0)
    check('live-data-still-unchanged',reads==state_reads())
    (out/'result.json').write_text(json.dumps(dict(status='PASS',checks=checks,binary=snapshot['binary'],binary_sha256=snapshot['binary_sha256'],read_only=True,global_acceptance=False),indent=2))
    print('INSTALLED_WORKBENCH_VERIFIED',len(checks))


if __name__=='__main__':main()
