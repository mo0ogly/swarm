#!/usr/bin/env python3
"""Runtime-health public API: isolated Root, no provider or mission execution.

The reference Go server has its own Root with no queued Agent; its startup
state changes are retained separately from the native read-only assertion. Preserve exact replies,
logical SQL state and original failures. No arbitrary global watchdog or retry.
"""
import argparse,json,http.client,subprocess,re,traceback,os
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest
from rust_agent_exchanges import logical
from rust_native_http_cli import Server,clean,assert_head_wire

def run(binary,go,output,unported=False):
    """Require valid controls before refusals; clean only our server processes.
    Go dynamic observation times/free space and compiled identity are explicit
    comparison exclusions, never ignored state or operator policy differences.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,output,'runtime-health');before=logical(f.root);go_dir=output/'go-fixture';go_dir.mkdir();gf=Fixture(binary,go_dir,'go-runtime-health');go_before=logical(gf.root);publish(output/'go-before.snapshot.txt',repr(go_before).encode());s=None;g=None;primary=None;calls=[];token=None;csrf=None;checks=[]
    def wire(address,cookie,path,status=200,method='GET',body=None):
        """Retain typed safe HTTP replies and exact status; close every connection.
        Origin/CSRF come only from this owned native server; no credentials saved.
        """
        host,port=address.rsplit(':',1);c=http.client.HTTPConnection(host,int(port),timeout=None)
        try:
            headers={'Cookie':cookie,'Origin':'http://'+address} if cookie else {}
            if csrf and method not in ['GET','HEAD']:headers['X-Swarm-CSRF']=csrf
            c.request(method,path,body,headers);r=c.getresponse();raw=r.read();actual=r.status
        finally:c.close()

        try:value=json.loads(raw) if raw else None
        except (json.JSONDecodeError,UnicodeDecodeError):value=raw.decode(errors='replace')
        calls.append(dict(path=path,method=method,status=actual,value=clean(value)));publish(output/'http.json',calls);assert actual==status,(actual,clean(value));return value
    try:
        g=subprocess.Popen([str(go),'web','127.0.0.1:0','--root',str(gf.root)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=gf.environment);line=g.stdout.readline().decode();m=re.fullmatch(r'Cockpit local : http://(127[.]0[.]0[.]1:[0-9]+)/session/([A-Za-z0-9_-]+)',line.strip());assert m;address,token=m.groups();reference=wire(address,'swarm_session='+token,'/api/v1/runtime-health');assert reference['state'] in ['ready','warning','blocked'] and isinstance(reference['launch_allowed'],bool) and len(reference['volumes'])==2;publish(output/'go-health.json',reference);g.terminate();_,stderr=g.communicate();assert token.encode() not in stderr;publish(output/'go-stop.json',dict(exit=g.returncode,stderr_sha256=digest(stderr)));g=None;publish(output/'go-after-startup.snapshot.txt',repr(logical(gf.root)).encode())
        s=Server(binary,f.root,output,'native',environment=f.environment);s.capture_async(output/'stream.jsonl');control=wire(s.address,s.cookie,'/api/v1/native-version');assert control['binary']['version'];view=wire(s.address,s.cookie,'/api/v1/runtime-health',404 if unported else 200)
        if not unported:
            health=lambda:wire(s.address,s.cookie,'/api/v1/runtime-health')
            assert isinstance(view['volumes'],list) and len(view['volumes'])==2
            assert set(view)==set(reference)|{'observation_errors'}
            assert all(set(row)==set(expected) for row,expected in zip(view['volumes'],reference['volumes']))
            assert all({'available_bytes','available_inodes','path','kind','state'}<=set(row) for row in view['volumes'])
            assert view['state']==reference['state'] and view['launch_allowed']==reference['launch_allowed']
            assert [v['kind'] for v in view['volumes']]==[v['kind'] for v in reference['volumes']]
            assert 'observation_errors' in view and isinstance(view['observation_errors'],list)
            cli=f.ok(['runtime-health']);assert cli['state']==view['state'] and cli['launch_allowed']==view['launch_allowed'];checks.append('actual-go-health-shape-state-and-native-cli-control')
            csrf=wire(s.address,s.cookie,'/api/v1/native-session')['csrf_token']
            assert_head_wire(s.host,s.port,'/api/v1/runtime-health',s.cookie)
            wire(s.address,None,'/api/v1/runtime-health',403)
            wire(s.address,s.cookie,'/api/v1/runtime-health',405,'POST',b'{}')
            wire(s.address,s.cookie,'/api/v1/runtime-health',400,'GET',b'{}')
            wire(s.address,s.cookie,'/api/v1/runtime-health?unused=1',400);checks.append('protected-auth-method-body-query-and-raw-head-refusals')
            fields=dict(runtime_storage_stop_bytes='0',runtime_storage_warning_bytes='0',runtime_storage_stop_inodes='0')
            f.settings_change(**fields);assert health()['state']=='ready'
            f.settings_change(**{**fields,'runtime_storage_warning_bytes':str((1<<63)-1)});assert health()['state']=='warning'
            f.settings_change(**{**fields,'runtime_storage_stop_bytes':str((1<<63)-1),'runtime_storage_warning_bytes':str((1<<63)-1)});blocked=health();assert blocked['state']=='blocked' and not blocked['launch_allowed']
            f.settings_change(**fields);s.cleanup();s=None;s=Server(binary,f.root,output,'reopened',environment=f.environment);s.capture_async(output/'reopened-stream.jsonl');assert health()['state']=='ready';checks.append('persisted-admin-values-real-classifier-and-reopen-recovery')
            settings=f.root/'.swarm/engine-settings.json';original=settings.read_bytes();publish(output/'settings-control.json',original)
            try:
                publish(settings,b'{');wire(s.address,s.cookie,'/api/v1/runtime-health',500);assert settings.read_bytes()==b'{'
            finally:publish(settings,original)
            assert health()['state']=='ready';checks.append('invalid-source-refuses-without-default-adoption-and-recovers')
            db=f.root/'.swarm/state.db';original_db=db.read_bytes();publish(output/'owned-sqlite-before-fault.bin',original_db);inode=db.stat().st_ino
            assert wire(s.address,s.cookie,'/api/v1/works')
            try:
                with db.open('r+b') as stream:stream.write(b'not an SQLite database'+bytes(100-22));stream.flush();os.fsync(stream.fileno())
                assert db.stat().st_ino==inode;wire(s.address,s.cookie,'/api/v1/works',500);assert health()['state']=='ready'
                code,value=f.call(['runtime-health']);assert code==2 and 'failure' in value
            finally:
                with db.open('r+b') as stream:stream.write(original_db);stream.truncate();stream.flush();os.fsync(stream.fileno())
            assert db.stat().st_ino==inode and db.read_bytes()==original_db and wire(s.address,s.cookie,'/api/v1/works');assert health()['state']=='ready';checks.append('already-bound-http-health-survives-real-sqlite-failure-cold-cli-refuses-and-restores')
            saved=f.root/'.swarm/state.db.owned-health-original';db.rename(saved)
            try:
                publish(db,original_db);assert db.stat().st_ino!=inode;wire(s.address,s.cookie,'/api/v1/runtime-health',500)
            finally:
                if db.exists():db.unlink()
                saved.rename(db)
            assert db.stat().st_ino==inode and health()['state']=='ready';checks.append('replacement-database-refuses-with-held-witness-and-original-recovery')
            s.cleanup();s=None;env={**f.environment,'TMPDIR':str(f.root/'missing-probe-directory')};s=Server(binary,f.root,output,'unknown-probe',environment=env);s.capture_async(output/'unknown-stream.jsonl');unknown=health();assert unknown['state']=='blocked' and not unknown['launch_allowed'] and unknown['observation_errors'];assert unknown['volumes'][1]['state']=='unknown';assert not (f.root/'missing-probe-directory').exists();checks.append('actual-missing-temp-probe-blocks-with-visible-safe-cause-no-empty-success')
        assert logical(f.root)==before and not list(f.root.rglob('model-received.json'))
    except BaseException as e:primary=e
    failures=[]
    if g is not None:
        try:g.terminate();g.communicate()
        except BaseException as e:failures.append(e)
    if s is not None:
        try:s.cleanup()
        except BaseException as e:failures.append(e)
    if primary:failures.insert(0,primary)
    if failures:raise BaseExceptionGroup('Runtime health and owned cleanup',failures)
    if s:assert not s.capture_errors
    publish(output/'result.json',dict(status='PASS',checks=checks,scope='positive Go/native controls and missing native route witness' if unported else 'actual health read, saved policy, SQL availability and physical identity on owned fixtures',binary_sha256=digest(binary.read_bytes()),go_sha256=digest(go.read_bytes()),provider_started=False,live_database=False));print('RUNTIME_HEALTH_UNPORTED_WITNESS' if unported else 'RUNTIME_HEALTH_PUBLIC_VERIFIED '+str(len(checks)),flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--expect-unported',action='store_true');a=p.parse_args()
    try:run(a.binary.resolve(),a.go_binary.resolve(),a.output.resolve(),a.expect_unported)
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
