#!/usr/bin/env python3
"""Filtered log HTTP/CLI parity and saved read limits across existing consumers.

Owned inert Agent preparation only, no provider run or live mission DB. A retained
fixture models prior log retention, not a real cleanup or consumption reset.
"""
import argparse,json,http.client,sqlite3,subprocess,re,urllib.parse,traceback
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest
from rust_agent_exchanges import logical
from rust_readonly_agent_fixture import prepare_historical_agent
from rust_native_http_cli import Server,clean,assert_head_wire

def run(binary,go,output):
    """Assert real Go/native values, all four saved policy consumers and recovery.

    Original fixture rows are retained before modelling a retention gap. No
    global watchdog, silent retry, provider, launch or acceptance operation.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,output,'log-query');agent=prepare_historical_agent(f);db=f.root/'.swarm/state.db';source=f.root/'.swarm/log-read-policy.json';calls=[];checks=[];s=None;g=None;token=None;csrf=None;primary=None;expected=[];utf_expected={};utf_cli={}
    with sqlite3.connect(db) as c:
        original=list(c.execute('SELECT seq,agent_id,at,kind,message FROM agent_logs WHERE agent_id=? ORDER BY seq',(agent,)));publish(output/'fixture-before-retention.json',original);c.execute('DELETE FROM agent_logs WHERE agent_id=?',(agent,))
        for seq,kind,message in [(11000,'message','fixture-log <img src=x onerror=window.logInjected=true>'),(13000,'error','fixture-log TRACE 100% _ tag'),(17000,'message','fixture-log trace Mixed')]:c.execute('INSERT INTO agent_logs(seq,agent_id,at,kind,message) VALUES(?,?,?,?,?)',(seq,agent,'2026-01-01T00:00:00Z',kind,message))
        for seq in [12000,16000,18000]:c.execute('INSERT INTO terminal_events(seq,agent_id,data,cols,rows) VALUES(?,?,?,?,?)',(seq,agent,b'fixture terminal',80,24))
    cases=[{},dict(after=11000,q='TRACE'),dict(after=1,q='never-present'),dict(after=1,kind='error'),dict(after=17000),dict(q='%'),dict(q="' OR 1=1 --")]
    def path(q):return '/api/v1/logs?'+urllib.parse.urlencode(dict(work=f.work,agent=agent,**q))
    def wire(host,port,cookie,url):
        c=http.client.HTTPConnection(host,port,timeout=None)
        try:c.request('GET',url,headers={'Cookie':cookie});r=c.getresponse();raw=r.read();assert r.status==200,(r.status,raw);return json.loads(raw)
        finally:c.close()
    def request(url,body=None,status=200,method=None,authorized=True):
        h={'Origin':'http://'+s.address}
        if authorized:h['Cookie']=s.cookie
        if body is not None:h.update({'Content-Type':'application/json','X-Swarm-CSRF':csrf})
        raw=body if isinstance(body,bytes) else json.dumps(body).encode() if body is not None else None;c=http.client.HTTPConnection(s.host,s.port,timeout=None)
        try:c.request(method or ('POST' if body is not None else 'GET'),url,raw,h);r=c.getresponse();raw=r.read();actual=r.status;ct=r.getheader('Content-Type','')
        finally:c.close()
        value=json.loads(raw) if raw and ct.startswith('application/json') else raw.decode();calls.append(dict(path=url,status=actual,value=clean(value)));publish(output/'http.json',calls);assert actual==status,(url,actual,clean(value));return value
    def core(v):return {k:v[k] for k in ['entries','next_cursor','retained_from','retention_gap','more']}
    def intent(event,revision,value):return dict(schema_version=1,event_id=event,expected_revision=revision,values=dict(search_page_rows=value,history_page_rows=value,live_log_page_rows=value,terminal_page_rows=value),reason='Owned log pagination fixture')
    def live():return request('/api/v1/live/agent?'+urllib.parse.urlencode(dict(work=f.work,agent=agent,logs_after=0,terminal_after=0)))
    try:
        g=subprocess.Popen([str(go),'web','127.0.0.1:0','--root',str(f.root)],env=f.environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE);line=g.stdout.readline().decode();m=re.fullmatch(r'Cockpit local : http://(127[.]0[.]0[.]1:[0-9]+)/session/([A-Za-z0-9_-]+)',line.strip());assert m;address,token=m.groups();host,port=address.rsplit(':',1)
        for i,q in enumerate(cases):v=wire(host,int(port),'swarm_session='+token,path(q));expected.append(v);publish(output/('go-'+str(i)+'.json'),v)
        assert [e['seq'] for e in expected[0]['entries']]==[11000,13000,17000];assert expected[2]['entries']==[] and expected[2]['retention_gap'] and expected[2]['next_cursor']==1 and expected[2]['retained_from']==11000;assert len(expected[5]['entries'])==1 and expected[6]['entries']==[]
        for storage in ['blob','text']:
            with sqlite3.connect(db) as c:c.execute('UPDATE agent_logs SET at='+('?' if storage=='blob' else 'CAST(? AS TEXT)')+' WHERE seq=11000',(bytes([0xe2,0x82]),))
            utf_expected[storage]=wire(host,int(port),'swarm_session='+token,path({}));publish(output/('go-utf-'+storage+'.json'),utf_expected[storage]);r=subprocess.run([str(go),'agent','logs',agent,'-1','--root',str(f.root),'--json'],env=f.environment,capture_output=True);publish(output/('go-cli-utf-'+storage+'.stdout'),r.stdout);publish(output/('go-cli-utf-'+storage+'.stderr'),r.stderr);assert r.returncode==0;utf_cli[storage]=json.loads(r.stdout)
        with sqlite3.connect(db) as c:c.execute('UPDATE agent_logs SET at=? WHERE seq=11000',('2026-01-01T00:00:00Z',))
        g.terminate();_,err=g.communicate();assert token.encode() not in err;publish(output/'go-stop.json',dict(exit=g.returncode,stderr_sha256=digest(err)));g=None;before=logical(f.root)
        s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');csrf=request('/api/v1/native-session')['csrf_token']
        for q,v in zip(cases,expected):
            actual=request(path(q));assert core(actual)==v;assert actual['exact']==dict(next_cursor=str(v['next_cursor']),retained_from=str(v['retained_from']),sequences=[str(e['seq']) for e in v['entries']])
        assert core(f.ok(['logs','query',f.work,agent],dict(schema_version=1,after=1,query='never-present',kind='')))==expected[2];assert logical(f.root)==before;checks.append('actual-go-search-kind-literal-query-empty-retention-gap-and-cli-parity')
        policy=request('/api/v1/log-read-policy');assert policy['revision']==0 and policy['values']==json.loads(Path('config/log-read-policy.json').read_bytes()) and not source.exists();assert_head_wire(s.host,s.port,path({}),s.cookie);request(path({}),status=403,authorized=False);request(path({}),{},405);request(path({}),b'{}',400,method='GET')
        for tail in ['&after=01','&after=bad','&after=1&after=2','&unused=1']:request(path({})+tail,status=400)
        request('/api/v1/logs?work=unknown&agent='+agent,status=404);checks.append('default-source-wire-head-auth-method-body-query-refusals')
        r=intent('first',0,1);preview=request('/api/v1/log-read-policy/preview',r);assert preview['preview'] and not preview['applied'] and not source.exists();saved=request('/api/v1/log-read-policy/apply',{**r,'preview_token':preview['preview_token']});assert saved['applied'];raw=source.read_bytes();assert f.ok(['run-limits','logs','show'])['values']==r['values'];p=request(path({}));assert [e['seq'] for e in p['entries']]==[11000] and p['more'];p=request(path(dict(after=p['next_cursor'])));assert [e['seq'] for e in p['entries']]==[13000] and p['more'];p=request(path(dict(after=p['next_cursor'])));assert [e['seq'] for e in p['entries']]==[17000] and not p['more'];checks.append('preview-without-write-persisted-search-pages-exact-more')
        assert [e['seq'] for e in f.ok(['agent','logs',agent,'0'])]==[17000];assert [e['seq'] for e in f.ok(['agent','logs',agent,'-1'])]==[11000];assert [e['seq'] for e in f.ok(['agent','logs',agent,'11000'])]==[13000];v=live();assert v['logs']['next_exact']=='11000' and v['terminal']['next_exact']=='12000';assert len(v['logs']['items'])==len(v['terminal']['items'])==1 and v['logs']['more'] and v['terminal']['more'];assert logical(f.root)==before;checks.append('same-saved-settings-real-history-cli-live-logs-and-terminal-consumers')
        for value in [0,-1,9007199254740992]:request('/api/v1/log-read-policy/apply',intent('invalid',1,value),400);assert source.read_bytes()==raw
        missing=intent('missing',1,1);del missing['values']['terminal_page_rows'];request('/api/v1/log-read-policy/apply',missing,400);request('/api/v1/log-read-policy/apply',intent('stale',0,1),409);assert source.read_bytes()==raw;checks.append('required-nullable-choices-representation-zero-and-stale-refuse-without-mutation')
        assert f.ok(['run-limits','logs','apply'],intent('unlimited',1,None))['applied'];assert core(request(path({})))==expected[0];assert len(f.ok(['agent','logs',agent,'-1']))==3;v=live();assert len(v['logs']['items'])==len(v['terminal']['items'])==3 and not v['logs']['more'] and not v['terminal']['more'];checks.append('explicit-unlimited-all-four-consumers-no-hidden-ceiling')
        for storage in ['blob','text']:
            with sqlite3.connect(db) as c:c.execute('UPDATE agent_logs SET at='+('?' if storage=='blob' else 'CAST(? AS TEXT)')+' WHERE seq=11000',(bytes([0xe2,0x82]),))
            assert core(request(path({})))==utf_expected[storage] and f.ok(['agent','logs',agent,'-1'])==utf_cli[storage];live_page=live();assert live_page['logs']['items'][0]['at']==utf_cli[storage][0]['at']
        with sqlite3.connect(db) as c:c.execute('UPDATE agent_logs SET at=? WHERE seq=11000',('2026-01-01T00:00:00Z',))
        raw=source.read_bytes();f.settings_change(agent_provider_config_max_bytes='0');request('/api/v1/log-read-policy',status=500);request(path({}),status=500);f.settings_change(agent_provider_config_max_bytes=None);assert request('/api/v1/log-read-policy')['revision']==2;publish(source,b'broken');request(path({}),status=500);publish(source,raw);assert core(request(path({})))==expected[0];checks.append('shared-saved-file-cap-corrupt-source-refusal-and-recovery')
        with sqlite3.connect(db) as c:
            try:c.execute('UPDATE agent_logs SET message=NULL WHERE seq=11000');raise AssertionError('NOT NULL expected')
            except sqlite3.IntegrityError:pass
        assert core(request(path({})))==expected[0] and logical(f.root)==before and not list(f.root.rglob('model-received.json'));checks.append('invalid-source-storage-refusal-no-mutation-no-provider')
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
    if primary or errors:raise BaseExceptionGroup('Log query flow and cleanup',([primary] if primary else [])+errors)
    assert len(checks)==8;publish(output/'result.json',dict(status='PASS',checks=checks,binary_sha256=digest(binary.read_bytes()),go_sha256=digest(go.read_bytes()),provider_started=False,live_database=False));print('LOG_QUERY_PUBLIC_VERIFIED 8',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.go_binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
