#!/usr/bin/env python3
"""Go/native readonly lifecycle inventory; synthetic inactive fixture metadata only.
No archived-file integrity, restoration, lifecycle mutation or provider is proven.
"""
import argparse,json,subprocess,re,urllib.parse,sqlite3,traceback
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest
from rust_native_http_cli import Server
from rust_agent_exchanges import logical
from rust_planning_history import clean
import http.client

def seed(f):
    """Insert explicitly synthetic archive/trash observations on an owned Root.
    No archive/snapshot is claimed valid; Work and event records are unchanged.
    """
    with sqlite3.connect(f.root/'.swarm/state.db') as db:
        db.execute('INSERT INTO mission_lifecycle VALUES(?,?,?,?)',(f.work,'','synthetic-unverified', '0'*64))
        db.execute('INSERT INTO mission_trash VALUES(?,?,?,?,?)',('synthetic-trash','<img> fixture',9223372036854775807,'fixture-time',b'not-a-valid-snapshot'))

def run(binary,go,out,unported=False):
    """Compare real Go and native observations; preserve all owned cleanup errors.
    SQL corruption is confined to synthetic fixtures; no public result is forged.
    """
    out.mkdir(parents=True,exist_ok=False);publish(out/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,out,'lifecycle');gf=Fixture(binary,out/'go','lifecycle-go');s=None;g=None;primary=None;checks=[];calls=[]
    def wire(address,cookie,path,status=200,method='GET',body=None,csrf=None):
        """Check exact protected status/value and retain only redacted fixture data."""
        host,port=address.rsplit(':',1);c=http.client.HTTPConnection(host,int(port),timeout=None);h={'Origin':'http://'+address}
        if cookie:h['Cookie']=cookie
        if csrf:h['X-Swarm-CSRF']=csrf
        try:c.request(method,path,body,h);r=c.getresponse();raw=r.read();actual=r.status
        finally:c.close()
        try:value=json.loads(raw) if raw else None
        except (ValueError,UnicodeError):value=raw.decode(errors='replace')
        calls.append(dict(path=path,status=actual,method=method,value=clean(value)));publish(out/'http.json',calls);assert actual==status,(actual,value);return value
    try:
        g=subprocess.Popen([str(go),'web','127.0.0.1:0','--root',str(gf.root)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=gf.environment);m=re.fullmatch(r'Cockpit local : http://(127[.]0[.]0[.]1:[0-9]+)/session/([A-Za-z0-9_-]+)',g.stdout.readline().decode().strip());assert m;address,key=m.groups();path='/api/v1/lifecycle/missions';a=wire(address,'swarm_session='+key,path);assert len(a)==1 and a[0]['state']=='active' and 'changed_at' not in a[0];seed(gf);reference=wire(address,'swarm_session='+key,path);assert [v['state'] for v in reference]==['archived','trash'] and 'changed_at' not in reference[0] and reference[1]['revision']==9223372036854775807;checks.append('actual-go-active-archived-trash-empty-time-high-revision')
        s=Server(binary,f.root,out,'http',environment=f.environment);s.capture_async(out/'stream.jsonl');before=logical(f.root)
        if unported:wire(s.address,s.cookie,path,404);assert logical(f.root)==before;checks.append('native-route-absence')
        else:
            first=wire(s.address,s.cookie,path);assert first==f.ok(['lifecycle','list']) and first[0]['work_id']==f.work and first[0]['state']=='active';seed(f);before=logical(f.root);expected=[{**v,'work_id':f.work if v['work_id']==gf.work else v['work_id']} for v in reference];assert wire(s.address,s.cookie,path)==f.ok(['lifecycle','list'])==expected;assert logical(f.root)==before;checks.append('native-cli-go-values-no-mutation')
            assert wire(s.address,s.cookie,path,method='HEAD') is None;csrf=wire(s.address,s.cookie,'/api/v1/native-session')['csrf_token'];wire(s.address,s.cookie,path,405,'POST',b'',csrf);wire(s.address,s.cookie,path+'?work='+f.work,400);wire(s.address,s.cookie,path,400,body=b'{}');wire(s.address,None,path,403);assert logical(f.root)==before;checks.append('method-query-body-auth-no-mutation')
            # Valid exact control precedes each isolated invalid Work body.
            assert wire(s.address,s.cookie,path)==expected
            with sqlite3.connect(f.root/'.swarm/state.db') as db:original=db.execute('SELECT body FROM works WHERE id=?',(f.work,)).fetchone()[0];db.execute('UPDATE works SET body=? WHERE id=?',(b'{',f.work))
            broken=logical(f.root);value=wire(s.address,s.cookie,path,500);code,error=f.call(['lifecycle','list']);assert code!=0 and logical(f.root)==broken
            with sqlite3.connect(f.root/'.swarm/state.db') as db:db.execute('UPDATE works SET body=? WHERE id=?',(original,f.work))
            assert wire(s.address,s.cookie,path)==expected;checks.append('malformed-work-refusal-no-empty-success-and-recovery')
        assert not list(f.root.rglob('model-received.json')) and not list(gf.root.rglob('model-received.json'))
    except BaseException as e:primary=e
    errors=[]
    if g is not None:
        try:g.terminate();g.communicate()
        except BaseException as e:errors.append(e)
    if s is not None:
        try:s.cleanup()
        except BaseException as e:errors.append(e)
    if primary:errors.insert(0,primary)
    if errors:raise BaseExceptionGroup('Lifecycle inventory and owned cleanup',errors)
    assert not s.capture_errors
    publish(out/'result.json',dict(status='PASS',checks=checks,binary_sha256=digest(binary.read_bytes()),go_sha256=digest(go.read_bytes()),synthetic_metadata_not_applied_lifecycle=True,provider_requested=False));print('LIFECYCLE_INVENTORY_UNPORTED_WITNESS' if unported else 'LIFECYCLE_INVENTORY_PUBLIC_VERIFIED 4',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--go-binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--expect-unported',action='store_true');a=p.parse_args()
    try:run(a.binary.resolve(),a.go_binary.resolve(),a.output.resolve(),a.expect_unported)
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
