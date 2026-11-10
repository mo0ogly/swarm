#!/usr/bin/env python3
"""Actual Go/native finite activity source window, then native unlimited recovery.

501 synthetic controls cross the actual Go500 source policy. No provider, live
Root, hidden retry or global deadline. Full cursor byte identity is checked.
"""
import argparse,json,sqlite3,subprocess,re,http.client,urllib.parse,traceback
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest
from rust_agent_exchanges import logical
from rust_native_http_cli import Server

def run(binary,go,output):
    """Compare three bounded pages and recover the two excluded source rows.

    Fixture data and expected counts are explicit; increasing/removing the saved
    native display cap grants no permission or agent execution. Errors and both
    owned server cleanup failures remain observable.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,output,'activity-window');s=None;g=None;primary=None;token=None;expected=[]
    with sqlite3.connect(f.root/'.swarm/state.db') as c:
        assert c.execute('SELECT count(*) FROM cockpit_events WHERE work_id=?',(f.work,)).fetchone()[0]==1
        c.executemany('INSERT INTO cockpit_events(seq,work_id,at,kind,message) VALUES(?,?,?,?,?)',[(1000+i,f.work,'9999-01-01T00:00:00Z','dispatch','owned row '+str(i)) for i in range(501)])
    def read(host,port,cookie,before):
        path='/api/v1/activity?'+urllib.parse.urlencode(dict(work=f.work,limit=200,before=before));c=http.client.HTTPConnection(host,port,timeout=None)
        try:c.request('GET',path,headers={'Cookie':cookie});r=c.getresponse();raw=r.read();assert r.status==200,(r.status,raw);return json.loads(raw)
        finally:c.close()
    def core(v):return {k:v[k] for k in ['entries','next_cursor','more','history_truncated']}
    try:
        g=subprocess.Popen([str(go),'web','127.0.0.1:0','--root',str(f.root)],env=f.environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE);line=g.stdout.readline().decode();m=re.fullmatch(r'Cockpit local : http://(127[.]0[.]0[.]1:[0-9]+)/session/([A-Za-z0-9_-]+)',line.strip());assert m;address,token=m.groups();host,port=address.rsplit(':',1);cursor=''
        for page in range(3):
            value=read(host,int(port),'swarm_session='+token,cursor);expected.append(value);cursor=value['next_cursor'];publish(output/('go-page-'+str(page)+'.json'),value)
        assert all(v['history_truncated'] for v in expected) and [v['more'] for v in expected]==[True,True,False]
        g.terminate();_,err=g.communicate();assert token.encode() not in err;publish(output/'go-stop.json',dict(exit=g.returncode,stderr_sha256=digest(err)));g=None
        state=logical(f.root);s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');actual=[];cursor=''
        for page in range(3):
            value=read(s.host,s.port,s.cookie,cursor);actual.append(core(value));cursor=value['next_cursor'];publish(output/('native-page-'+str(page)+'.json'),value)
        assert actual==expected;bounded=[e for p in actual for e in p['entries']];assert len({e['rank'] for e in bounded})==len(bounded) and not any(e['rank']=='c000000001000' for e in bounded);assert logical(f.root)==state
        r=dict(schema_version=1,event_id='unlimited-window',expected_revision=0,values=dict(page_default_rows=10000,page_max_rows=None,source_max_rows=None),reason='Explicit fixture recovery of older source history');assert f.ok(['run-limits','activity','apply'],r)['applied'];full=f.ok(['activity','show',f.work]);assert not full['more'] and not full['history_truncated'] and len(full['entries'])==len(bounded)+2 and any(e['rank']=='c000000001000' for e in full['entries']);assert logical(f.root)==state and not list(f.root.rglob('model-received.json'))
    except BaseException as e:primary=e
    errors=[]
    if s:
        try:s.cleanup()
        except BaseException as e:errors.append(e)
    if g:
        try:
            if g.poll() is None:g.terminate()
            _,err=g.communicate();assert token is None or token.encode() not in err
        except BaseException as e:errors.append(e)
    if primary or errors:raise BaseExceptionGroup('Activity window and cleanup',([primary] if primary else [])+errors)
    checks=['go-native-three-window-pages-exact','no-cursor-duplicates-and-visible-truncation','saved-unlimited-recovers-two-excluded-rows-without-db-mutation'];publish(output/'result.json',dict(status='PASS',checks=checks,binary_sha256=digest(binary.read_bytes()),go_sha256=digest(go.read_bytes())));print('ACTIVITY_WINDOW_VERIFIED 3',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.go_binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
