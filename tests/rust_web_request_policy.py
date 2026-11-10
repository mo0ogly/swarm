#!/usr/bin/env python3
"""Public browser-policy CLI/HTTP with isolated Root and actual saved cap.

No account/provider/mission execution or global deadline. Private-only source
faults follow valid controls and retain originals. Browser behavior is verified
by a separate actual served-UI recipe; this script does not infer cancellation.
"""
from pathlib import Path
import argparse,json,http.client,traceback
from rust_agent_model_route import Fixture,publish,digest
from rust_native_http_cli import Server,clean

def run(binary,output):
    """Retain exact command/status/state proofs and require checked cleanup.

    No new timer, implicit retry, Go source mutation or live database fixture.
    Positive effects and rejected requests are distinguished from unknown replies.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,output,'web-policy');before=f.snapshot();source=f.root/'.swarm/web-requests.json';checks=[];calls=[];s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');primary=None;csrf=None
    def request(path,body=None,status=200,method=None,authorized=True):
        headers={'Origin':'http://'+s.address}
        if authorized:headers['Cookie']=s.cookie
        if body is not None:headers.update({'Content-Type':'application/json','X-Swarm-CSRF':csrf})
        raw=None if body is None else body if isinstance(body,bytes) else json.dumps(body).encode()
        c=http.client.HTTPConnection(s.host,s.port,timeout=None)
        try:c.request(method or ('GET' if body is None else 'POST'),path,raw,headers);r=c.getresponse();raw=r.read();actual=r.status;content_type=r.getheader('Content-Type','')
        finally:c.close()
        value=json.loads(raw) if raw and content_type.startswith('application/json') else raw.decode()
        calls.append(dict(path=path,status=actual,method=method or ('GET' if body is None else 'POST'),content_type=content_type,value=clean(value)))
        publish(output/'http.json',calls);assert actual==status,(path,actual,clean(value));return value
    def change(event,revision):return dict(schema_version=1,event_id=event,expected_revision=revision,values=dict(general_ms=20,snapshot_ms=30,connection_test_ms=40),reason='Owned browser policy fixture')
    def unchanged():assert f.snapshot()==before and not list(f.root.rglob('model-received.json'))
    try:
        csrf=request('/api/v1/native-session')['csrf_token'];view=request('/api/v1/web-requests')
        assert view['revision']==0 and view['history']==[] and view['values']==dict(general_ms=15000,snapshot_ms=60000,connection_test_ms=65000) and view['source']=='versioned_defaults'
        script=request('/api/v1/web-requests/script.js');assert script.startswith('globalThis.SwarmWebRequestConfig = ') and json.loads(script.split(' = ',1)[1].removesuffix(';\n'))==dict(revision=0,values=view['values'])
        assert request('/api/v1/web-requests/script.js',method='HEAD')==''
        assert f.ok(['run-limits','web-requests','history'])==[];assert not source.exists()
        checks.append('versioned-defaults-script-real-mime-head-cli-history')
        request('/api/v1/web-requests/script.js',{},405);request('/api/v1/web-requests',b'{}',400,method='GET')
        checks.append('dynamic-script-auth-and-method-body-gates-before-source')
        r=change('first',0);pv=request('/api/v1/web-requests/preview',r);assert pv['preview'] and not pv['applied'] and pv['config']['revision']==1 and not source.exists() and not (f.root/'.swarm/web-requests.lock').exists()
        saved=request('/api/v1/web-requests/apply',{**r,'preview_token':pv['preview_token']});assert saved['applied'] and saved['config']['revision']==1
        assert f.ok(['run-limits','web-requests','show'])['values']==r['values'];first=source.read_bytes();unchanged();checks.append('read-only-preview-owned-atomic-apply-cli-readback')
        request('/api/v1/web-requests/apply',change('stale',0),409);request('/api/v1/web-requests/apply',{**r,'reason':'Conflicting recorded event'},409)
        bad=change('negative',1);bad['values']['general_ms']=-1;request('/api/v1/web-requests/apply',bad,400)
        bad=change('partial',1);del bad['values']['snapshot_ms'];request('/api/v1/web-requests/apply',bad,400)
        raw=b'{"schema_version":1,"event_id":"duplicate","expected_revision":1,"reason":"Owned duplicate guard","values":{"general_ms":1},"values":{"general_ms":2,"snapshot_ms":3,"connection_test_ms":4}}';request('/api/v1/web-requests/apply',raw,400)
        assert source.read_bytes()==first and request('/api/v1/web-requests')['revision']==1;unchanged();checks.append('stale-event-negative-partial-earlier-duplicate-refuse-without-mutation')
        second=change('second',1);second['values']['general_ms']=0;assert f.ok(['run-limits','web-requests','apply'],second)['config']['revision']==2
        before_replay=source.read_bytes();replay=request('/api/v1/web-requests/apply',{**r,'preview_token':pv['preview_token']});assert replay['replayed'] and replay['config']['revision']==2 and source.read_bytes()==before_replay
        hist=f.ok(['run-limits','web-requests','history']);assert [row['revision'] for row in hist]==[1,2]
        assert json.loads(request('/api/v1/web-requests/script.js').split(' = ',1)[1].removesuffix(';\n'))['values']['general_ms']==0
        checks.append('legacy-cas-exact-replay-current-config-old-token-unchanged-source-script-current')
        third=change('third',2);p=request('/api/v1/web-requests/preview',third);settings=f.ok(['engine-settings','show'])['values'];cap=settings['agent_provider_config_max_bytes'];f.settings_change(agent_provider_config_max_bytes=None)
        current=source.read_bytes();request('/api/v1/web-requests/apply',{**third,'preview_token':p['preview_token']},409);assert source.read_bytes()==current
        assert request('/api/v1/web-requests/apply',third)['config']['revision']==3
        f.settings_change(agent_provider_config_max_bytes='0');request('/api/v1/web-requests',status=500);assert source.read_bytes()!=b''
        request('/api/v1/web-requests/script.js',status=403,authorized=False)
        f.settings_change(agent_provider_config_max_bytes=None);assert request('/api/v1/web-requests')['revision']==3;unchanged();checks.append('settings-token-stale-explicit-cas-saved-read-cap-refusal-and-recovery')
        # Valid public CAS write beyond Go's old file/reason caps; framing remains
        # native16MiB, this input intentionally stays below that technical bound.
        large=change('large-reason',3);large['reason']='x'*(1048576+4096)
        result=f.ok(['run-limits','web-requests','apply'],large);assert result['config']['revision']==4 and source.stat().st_size>1048576
        assert f.ok(['run-limits','web-requests','show'])['revision']==4
        f.settings_change(agent_provider_config_max_bytes='1048576');code,error=f.call(['run-limits','web-requests','show']);assert code!=0
        f.settings_change(agent_provider_config_max_bytes=None);assert f.ok(['run-limits','web-requests','show'])['revision']==4
        checks.append('no-hidden-go-history-or-reason-cap-shared-saved-byte-limit-real-consumer')
        original=source.read_bytes();publish(output/'valid-source-before-fault.json',original)
        publish(source,b'not-json');request('/api/v1/web-requests',status=500);assert source.read_bytes()==b'not-json'
        publish(source,original);assert request('/api/v1/web-requests')['revision']==4
        # Historical Change is typed metadata, not a second validity gate.
        data=json.loads(original);data['history'][0]['change']=None;publish(source,data)
        assert request('/api/v1/web-requests')['history'][0]['change']['schema_version']==0
        publish(source,original);assert request('/api/v1/web-requests')['revision']==4
        unchanged();checks.append('valid-control-source-json-error-recovery-no-historical-change-gate')
    except BaseException as error:primary=error
    cleanup=None
    try:s.cleanup()
    except BaseException as error:cleanup=error
    if primary or cleanup:raise BaseExceptionGroup('Web policy or owned cleanup failed',([primary] if primary else [])+([cleanup] if cleanup else []))
    assert len(checks)==8 and not s.capture_errors
    publish(output/'result.json',dict(status='PASS',checks=checks,binary_sha256=digest(binary.read_bytes()),provider_started=False,live_database_mutated=False));print('WEB_REQUEST_POLICY_PUBLIC_VERIFIED',len(checks),flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
