#!/usr/bin/env python3
"""Graph objectives public CLI/HTTP on an inert private Root, no renderer/provider.

Exercise saved shared cap, metadata size parity, atomic history, source refusal
and recovery. No arbitrary global watchdog; original failures remain retained.
"""
import argparse,json,http.client,traceback
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest
from rust_native_http_cli import Server,clean

def run(binary,output):
    """Retain named actual assertions, original replies and checked owned cleanup.

    Only fixture files are faulted after valid controls; no live datastore, model,
    benchmark, budget reset, implicit retry or acceptance operation is invoked.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,output,'graph-objectives');before=f.snapshot();source=f.root/'.swarm/graph-performance.json';checks=[];calls=[];s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');primary=None;csrf=None
    def request(path,body=None,status=200,method=None,authorized=True):
        headers={'Origin':'http://'+s.address}
        if authorized:headers['Cookie']=s.cookie
        if body is not None:headers.update({'Content-Type':'application/json','X-Swarm-CSRF':csrf})
        raw=None if body is None else body if isinstance(body,bytes) else json.dumps(body).encode();c=http.client.HTTPConnection(s.host,s.port,timeout=None)
        try:c.request(method or ('GET' if body is None else 'POST'),path,raw,headers);r=c.getresponse();raw=r.read();actual=r.status;ct=r.getheader('Content-Type','')
        finally:c.close()
        value=json.loads(raw) if raw and ct.startswith('application/json') else raw.decode();calls.append(dict(path=path,status=actual,method=method or ('GET' if body is None else 'POST'),value=clean(value)));publish(output/'http.json',calls);assert actual==status,(path,actual,clean(value));return value
    def change(event,revision):return dict(schema_version=1,event_id=event,expected_revision=revision,values=dict(keyboard_p95_ms=60001,samples=101,loads=[dict(cards=10001,render_p95_ms=60001)]),reason='Owned graph objective fixture')
    def unchanged():assert f.snapshot()==before and not list(f.root.rglob('model-received.json'))
    try:
        csrf=request('/api/v1/native-session')['csrf_token'];v=request('/api/v1/graph-performance');assert v['revision']==0 and v['history']==[] and v['values']==json.loads(Path('config/graph-performance.json').read_bytes());assert f.ok(['run-limits','performance','history'])==[] and not source.exists();assert request('/api/v1/graph-performance',method='HEAD')==''
        request('/api/v1/graph-performance',{},405);request('/api/v1/graph-performance',b'{}',400,method='GET');request('/api/v1/graph-performance',status=403,authorized=False);checks.append('actual-versioned-defaults-cli-head-method-auth-no-source-adoption')
        r=change('first',0);p=request('/api/v1/graph-performance/preview',r);assert p['preview'] and not p['applied'] and p['config']['revision']==1 and not source.exists() and not (f.root/'.swarm/graph-performance.lock').exists();saved=request('/api/v1/graph-performance/apply',{**r,'preview_token':p['preview_token']});assert saved['applied'] and saved['config']['values']==r['values'] and f.ok(['run-limits','performance','show'])['revision']==1;first=source.read_bytes();unchanged();checks.append('preview-readonly-atomic-save-no-historical-small-load-sample-target-caps')
        for key,value in [('samples',0),('keyboard_p95_ms',0),('loads',[]),('loads',[dict(cards=1,render_p95_ms=1)]),('loads',[dict(cards=2,render_p95_ms=1)]*2),('samples',9007199254740992)]:
            bad=change('invalid',1);bad['values'][key]=value;request('/api/v1/graph-performance/apply',bad,400);assert source.read_bytes()==first
        request('/api/v1/graph-performance/apply',change('stale',0),409);request('/api/v1/graph-performance/apply',{**r,'reason':'changed event'},409);assert source.read_bytes()==first;checks.append('recipe-invariants-representation-and-conflicts-refuse-without-mutation')
        second=change('second',1);second['values']['samples']=2;assert f.ok(['run-limits','performance','apply'],second)['config']['revision']==2;raw=source.read_bytes();assert request('/api/v1/graph-performance/apply',{**r,'preview_token':p['preview_token']})['config']['revision']==2 and source.read_bytes()==raw;checks.append('explicit-cli-cas-exact-replay-current-no-extra-history')
        third=change('third',2);p=request('/api/v1/graph-performance/preview',third);f.settings_change(agent_provider_config_max_bytes=None);request('/api/v1/graph-performance/apply',{**third,'preview_token':p['preview_token']},409);assert source.read_bytes()==raw
        assert request('/api/v1/graph-performance/apply',third)['config']['revision']==3;f.settings_change(agent_provider_config_max_bytes='0');request('/api/v1/graph-performance',status=500);f.settings_change(agent_provider_config_max_bytes=None);assert request('/api/v1/graph-performance')['revision']==3;checks.append('saved-cap-real-consumer-settings-token-staleness-and-recovery')
        r=change('large-reason',3);r['reason']='x'*(1048576+4096);assert f.ok(['run-limits','performance','apply'],r)['config']['revision']==4 and source.stat().st_size>1048576;original=source.read_bytes();f.settings_change(agent_provider_config_max_bytes='1048576');assert f.call(['run-limits','performance','show'])[0]!=0;f.settings_change(agent_provider_config_max_bytes=None);assert f.ok(['run-limits','performance','show'])['revision']==4;checks.append('shared-policy-no-hidden-file-or-reason-ceiling')
        publish(output/'valid-source-before-fault.json',original);publish(source,b'not-json');request('/api/v1/graph-performance',status=500);assert source.read_bytes()==b'not-json';publish(source,original);assert request('/api/v1/graph-performance')['revision']==4;data=json.loads(original);data['history'][0]['change']=None;publish(source,data);assert request('/api/v1/graph-performance')['history'][0]['change']['schema_version']==0;publish(source,original);assert request('/api/v1/graph-performance')['revision']==4;unchanged();checks.append('valid-control-malformed-source-and-typed-historical-metadata-recovery')
    except BaseException as e:primary=e
    cleanup=None
    try:s.cleanup()
    except BaseException as e:cleanup=e
    if primary or cleanup:raise BaseExceptionGroup('Graph objective flow/cleanup failed',([primary] if primary else [])+([cleanup] if cleanup else []))
    assert len(checks)==7 and not s.capture_errors;publish(output/'result.json',dict(status='PASS',checks=checks,binary_sha256=digest(binary.read_bytes()),provider_started=False,live_database_mutated=False));print('GRAPH_PERFORMANCE_PUBLIC_VERIFIED 7',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
