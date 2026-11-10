#!/usr/bin/env python3
"""Exercise redacted API connection inventory on an owned inert native Root.

Four named groups; no remote/provider/live mission/write API calls. Direct source
faults are confined to the owned fixture and restored; errors remain original.
"""
import argparse,json,traceback,socket
from pathlib import Path
from http.client import HTTPConnection
import rust_agent_model_route as retained
from rust_native_http_cli import Server,clean

def run(binary,output):
    """Read absent/null/valid/corrupt sources and test HTTP refusal/recovery.

    Private key is only a fixture canary, never an actual credential. Original
    source/Work/Agent/reservation state is compared; owned cleanup errors survive.
    """
    output.mkdir(parents=True,exist_ok=False);f=retained.Fixture(binary,output,'connections');before=f.snapshot();file=f.root/'.swarm/ai-connections.json'
    assert not file.exists();s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');checks=[];primary=None;records=[];csrf=None
    def request(path='/api/v1/providers/connections',method='GET',body=None,status=200,authorized=True):
        """Assert actual HTTP response and redact the fixture secret defensively."""
        headers={'Origin':'http://'+s.address}
        if authorized:headers['Cookie']=s.cookie
        if method not in ['GET','HEAD'] and csrf is not None:headers['X-Swarm-CSRF']=csrf
        c=HTTPConnection(s.host,s.port,timeout=None)
        try:c.request(method,path,body,headers);r=c.getresponse();raw=r.read();actual=r.status
        finally:c.close()
        value=json.loads(raw) if raw.startswith(b'{') else raw.decode();assert 'PRIVATE_CONNECTION_KEY' not in raw.decode();records.append({'path':path,'status':actual,'method':method,'value':clean(value)});retained.publish(output/'http.json',records);assert actual==status,(actual,value);return value
    def unchanged():
        """Require inert fixture state and no unexpectedly executed provider child."""
        assert f.snapshot()==before and not list(f.root.rglob('model-received.json'))
    try:
        csrf=request('/api/v1/native-session')['csrf_token']
        assert request('/api/v1/providers/admin')['version']==1
        empty=request();assert empty=={'version':1,'digest':retained.digest(b''),'connections':[]};assert not file.exists();unchanged();checks.append('absent-source-empty-with-actual-digest-no-file')
        data={'alpha':{'id':'alpha','label':'Owned local','base_url':'https://example.invalid/v1','model':'fixture-model','key':'PRIVATE_CONNECTION_KEY','disabled':False},'beta':{'id':'beta','label':'Disabled','base_url':'http://127.0.0.1:1','model':'fixture-disabled','disabled':True}}
        retained.publish(file,data);original=file.read_bytes();v=request();assert v['digest']==retained.digest(original) and v['version']==1
        assert [r['id'] for r in v['connections']]==['alpha','beta'];assert v['connections'][0]['provider']=='api-alpha' and v['connections'][0]['has_key'] is True and v['connections'][1]['has_key'] is False and v['connections'][1]['disabled'] is True
        assert all('key' not in row for row in v['connections']);assert file.read_bytes()==original;unchanged();checks.append('saved-fields-sorted-redacted-and-disabled-not-availability')
        for bad in [b'PRIVATE malformed',b'[]',b'{"alpha":{"key":42}}',b'{"alpha":{"unknown":true}}',b'{"alpha":{"id":"other"}}',b'{"alpha":null}']:
            assert request()['digest']==retained.digest(original)
            try:retained.publish(file,bad);request(status=500);assert file.read_bytes()==bad;unchanged()
            finally:retained.publish(file,original)
            assert request()['digest']==retained.digest(original)
        retained.publish(file,b'null');assert request()=={'version':1,'digest':retained.digest(b'null'),'connections':[]};retained.publish(file,original)
        original_cap=f.ok(['engine-settings','show'])['values']['agent_provider_config_max_bytes']
        try:
            f.settings_change(agent_provider_config_max_bytes=str(len(original)))
            assert request()['digest']==retained.digest(original)
            retained.publish(file,original+b' ');request(status=500)
            assert file.read_bytes()==original+b' ';unchanged()
            retained.publish(file,original);assert request()['digest']==retained.digest(original)
        finally:
            retained.publish(file,original);f.settings_change(agent_provider_config_max_bytes=original_cap)
        checks.append('invalid-stored-shape-identity-cap-refusal-and-null-recovery')
        for params in [dict(path='/api/v1/providers/connections?x=1',status=400),dict(body=b'{}',status=400),dict(method='PUT',status=405),dict(authorized=False,status=403)]:
            assert request()['digest']==retained.digest(original);request(**params);assert file.read_bytes()==original;unchanged()
        with socket.create_connection((s.host,s.port),timeout=None) as head:
            head.sendall(('HEAD /api/v1/providers/connections HTTP/1.1\r\nHost: '+s.address+'\r\nCookie: '+s.cookie+'\r\nConnection: close\r\n\r\n').encode());chunks=[]
            while True:
                chunk=head.recv(65536)
                if not chunk:break
                chunks.append(chunk)
        headers,body=b''.join(chunks).split(b'\r\n\r\n',1);assert headers.startswith(b'HTTP/1.1 200') and not body
        retained.publish(output/'raw-head.json',{'status':200,'body_bytes':len(body)})
        file.unlink();assert request()['connections']==[];unchanged();checks.append('query-body-method-session-head-refuse-and-recover')
    except BaseException as error:primary=error
    try:s.cleanup()
    except BaseException as cleanup:
        if primary:raise BaseExceptionGroup('Connection inventory and cleanup failed',[primary,cleanup])
        raise
    if primary:raise primary
    assert len(checks)==4 and s.process.returncode==0 and not s.capture_errors
    retained.publish(output/'result.json',{'status':'PASS','checks':checks,'sha256':retained.digest(binary.read_bytes()),'provider_started':False,'connection_write_api':False});print('API_CONNECTIONS_READ_VERIFIED 4',flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():retained.publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
