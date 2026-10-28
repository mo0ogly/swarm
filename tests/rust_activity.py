#!/usr/bin/env python3
"""Activity read/policy: protected native HTTP/CLI and real Go reference.

Only inert owned SQL fixtures; no provider, mission execution or live datastore.
No global watchdog. Original replies and named assertions retain failures.
"""
import argparse,json,http.client,sqlite3,subprocess,re,urllib.parse,traceback
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest
from rust_agent_exchanges import logical
from rust_native_http_cli import Server,clean,assert_head_wire

def run(binary,go,output):
    """Verify read parity, saved choices, refusal without mutation and recovery.

    SQL rows are synthetic history, never receipts of agent execution. Native
    policy metadata and stricter invalid input/source refusal are explicit
    extensions. Owned Go startup may migrate its fixture before read snapshots.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,output,'activity');db=f.root/'.swarm/state.db';source=f.root/'.swarm/activity-policy.json';checks=[];calls=[];s=None;g=None;primary=None;token=None;csrf=None
    with sqlite3.connect(db) as c:
        revision=c.execute('SELECT max(revision) FROM events WHERE work_id=?',(f.work,)).fetchone()[0]+1;poly=json.dumps(dict(title='Recorded review',criteria=[dict(criterion='structured review fixture',passed=True)])).encode();c.execute('INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?,?,?,?,?,?,?)',('fixture-polymorphic-review',f.work,revision,'review.managed.result','2026-01-01T00:00:00Z',poly,b'{}'))
        for seq,kind,message in [(800,'dispatch','<img src=x onerror=window.activityInjected=true>'),(801,'decision','human decision'),(802,'assist-action','{"id":"fixture","turn":"fixture","step":1,"status":"recorded","message":"Recorded advice"}')]:
            c.execute('INSERT INTO cockpit_events(seq,work_id,at,kind,message) VALUES(?,?,?,?,?)',(seq,f.work,'9999-01-01T00:00:00Z',kind,message))
    def request(path,body=None,status=200,method=None,authorized=True):
        headers={'Origin':'http://'+s.address}
        if authorized:headers['Cookie']=s.cookie
        if body is not None:headers.update({'Content-Type':'application/json','X-Swarm-CSRF':csrf})
        raw=body if isinstance(body,bytes) else json.dumps(body).encode() if body is not None else None;c=http.client.HTTPConnection(s.host,s.port,timeout=None)
        try:c.request(method or ('GET' if body is None else 'POST'),path,raw,headers);r=c.getresponse();raw=r.read();actual=r.status;ct=r.getheader('Content-Type','')
        finally:c.close()
        value=json.loads(raw) if raw and ct.startswith('application/json') else raw.decode();calls.append(dict(path=path,status=actual,value=clean(value)));publish(output/'http.json',calls);assert actual==status,(path,actual,clean(value));return value
    def page(**q):return '/api/v1/activity?'+urllib.parse.urlencode({'work':f.work,**q})
    def go_page(**q):
        c=http.client.HTTPConnection(gh,int(gp),timeout=None)
        try:c.request('GET',page(**q),headers={'Cookie':'swarm_session='+token});r=c.getresponse();raw=r.read();assert r.status==200;v=json.loads(raw);publish(output/('go-'+digest(json.dumps(q,sort_keys=True).encode())+'.json'),v);return v
        finally:c.close()
    def intent(event,rev,default=2,maximum=None,source_max=None):return dict(schema_version=1,event_id=event,expected_revision=rev,reason='Owned activity fixture',values=dict(page_default_rows=default,page_max_rows=maximum,source_max_rows=source_max))
    def core(v):return {k:v[k] for k in ['entries','next_cursor','more','history_truncated']}
    try:
        g=subprocess.Popen([str(go),'web','127.0.0.1:0','--root',str(f.root)],env=f.environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE);line=g.stdout.readline().decode();m=re.fullmatch(r'Cockpit local : http://(127\.0\.0\.1:[0-9]+)/session/([^\s]+)\n',line);assert m;address,token=m.groups();gh,gp=address.rsplit(':',1)
        go_initial=go_page();go_first=go_page(limit=2);go_second=go_page(limit=2,before=go_first['next_cursor']);go_decisions=go_page(decisions=1)
        g.terminate();rest,err=g.communicate();assert token.encode() not in err;publish(output/'go-stop.json',dict(exit=g.returncode,stderr_sha256=digest(err)));g=None
        s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');csrf=request('/api/v1/native-session')['csrf_token'];before=logical(f.root);initial=request(page());assert core(initial)==go_initial;assert initial['entries'][0]['kind']=='assist-action';assert core(f.ok(['activity','show',f.work]))==core(initial);assert not source.exists();checks.append('go-native-cli-default-page-and-recorded-origin-parity')
        first=request(page(limit=2));assert core(first)==go_first and len(first['entries'])==2 and first['more'];second=request(page(limit=2,before=first['next_cursor']));assert core(second)==go_second;assert not set(e['rank'] for e in first['entries'])&set(e['rank'] for e in second['entries']);assert core(request(page(decisions=1)))==go_decisions;assert logical(f.root)==before;checks.append('equal-timestamp-exclusive-full-cursor-decisions-no-mutation')
        assert_head_wire(s.host,s.port,page(),s.cookie);request(page(),status=403,authorized=False);request(page(),{},405);request(page(),b'{}',400,method='GET')
        for q in ['work='+f.work+'&limit=-1','work='+f.work+'&limit=01','work='+f.work+'&decisions=true','work='+f.work+'&work='+f.work,'work='+f.work+'&extra=1']:request('/api/v1/activity?'+q,status=400)
        request('/api/v1/activity?work=missing-work',status=404);checks.append('auth-wire-head-strict-query-method-body-and-unknown-work-refusals')
        r=intent('first',0);p=request('/api/v1/activity-policy/preview',r);assert p['preview'] and not p['applied'] and not source.exists();applied=request('/api/v1/activity-policy/apply',{**r,'preview_token':p['preview_token']});assert applied['applied'] and applied['config']['revision']==1;assert f.ok(['run-limits','activity','show'])['values']==r['values'];assert len(request(page())['entries'])==2;raw=source.read_bytes();assert logical(f.root)==before;checks.append('preview-readonly-persisted-default-real-pagination-and-cli-readback')
        for values in [{'page_default_rows':1,'source_max_rows':None},{'page_default_rows':0,'page_max_rows':None,'source_max_rows':None},{'page_default_rows':2,'page_max_rows':1,'source_max_rows':None},{'page_default_rows':1,'page_max_rows':None,'source_max_rows':-1},{'page_default_rows':1,'page_max_rows':None,'source_max_rows':9007199254740992}]:request('/api/v1/activity-policy/apply',{**intent('bad',1),'values':values},400);assert source.read_bytes()==raw
        malformed=json.dumps(intent('duplicate',1)).replace('"page_default_rows": 2','"page_default_rows": 2,"page_default_rows": 1').encode();request('/api/v1/activity-policy/apply',malformed,400);request('/api/v1/activity-policy/apply',intent('stale',0),409);assert source.read_bytes()==raw;checks.append('required-nullable-choices-duplicates-representation-cas-refuse-unchanged')
        assert f.ok(['run-limits','activity','apply'],intent('zero-source',1,1,None,0))['config']['revision']==2;v=request(page());assert v['entries']==[] and v['history_truncated'] and not v['more'];assert f.ok(['run-limits','activity','apply'],intent('clamped',2,1,1,None))['config']['revision']==3;v=request(page(limit=10000));assert len(v['entries'])==1 and v['read_policy']['effective_page_rows']=='1';assert f.ok(['run-limits','activity','apply'],intent('unlimited',3,10001,None,None))['config']['revision']==4;assert core(request(page()))==core(initial);checks.append('source-zero-visible-truncation-saved-page-clamp-unlimited-recovery')
        raw=source.read_bytes();f.settings_change(agent_provider_config_max_bytes='0');request('/api/v1/activity-policy',status=500);request(page(),status=500);f.settings_change(agent_provider_config_max_bytes=None);assert request('/api/v1/activity-policy')['revision']==4;publish(output/'source-before-fault.json',raw);publish(source,b'broken');request(page(),status=500);publish(source,raw);assert core(request(page()))==core(initial);checks.append('shared-saved-byte-cap-corrupt-source-refusal-and-recovery')
        with sqlite3.connect(db) as c:c.execute('UPDATE cockpit_events SET message=? WHERE seq=802',('{"step":"invalid"}',))
        request(page(),status=500)
        with sqlite3.connect(db) as c:c.execute('UPDATE cockpit_events SET message=? WHERE seq=802',('{"id":"fixture","turn":"fixture","step":1,"status":"recorded","message":"Recorded advice"}',))
        assert core(request(page()))==core(initial)
        with sqlite3.connect(db) as c:c.execute('UPDATE events SET payload=? WHERE id=?',(b'{"title":false,"criteria":[{}]}','fixture-polymorphic-review'))
        request(page(),status=500)
        with sqlite3.connect(db) as c:c.execute('UPDATE events SET payload=? WHERE id=?',(poly,'fixture-polymorphic-review'))
        assert core(request(page()))==core(initial) and logical(f.root)==before and not list(f.root.rglob('model-received.json'));checks.append('polymorphic-review-parity-consumed-type-refusal-and-recovery-no-provider')
    except BaseException as e:primary=e
    errors=[]
    if s:
        try:s.cleanup()
        except BaseException as e:errors.append(e)
    if g:
        try:
            if g.poll() is None:g.terminate()
            rest,err=g.communicate();assert token is None or token.encode() not in err;publish(output/'go-stop.json',dict(exit=g.returncode,stderr_sha256=digest(err)))
        except BaseException as e:errors.append(e)
    if primary or errors:raise BaseExceptionGroup('Activity flow and cleanup failed',([primary] if primary else [])+errors)
    assert len(checks)==8;publish(output/'result.json',dict(status='PASS',checks=checks,binary_sha256=digest(binary.read_bytes()),go_sha256=digest(go.read_bytes()),provider_started=False));print('ACTIVITY_PUBLIC_VERIFIED 8',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.go_binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
