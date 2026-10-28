#!/usr/bin/env python3
"""Verify Go-compatible provider-wait policy on a private inert native Root.

No provider, running mission, budget reset or global watchdog. Source defaults are
actual versioned data, not duplicated numeric assumptions in the fixture.
"""
import argparse,json,traceback,subprocess
from pathlib import Path
from http.client import HTTPConnection
import rust_agent_model_route as retained
from rust_native_http_cli import Server,clean

def run(binary,output):
    """Require actual source/history and rejection/replay/restart behavior.

    Original failures survive; owned process cleanup retains secondary causes.
    Direct source faults apply to fixture-only JSON and restore original bytes.
    """
    output.mkdir(parents=True,exist_ok=False);f=retained.Fixture(binary,output,'provider-wait');before=f.snapshot();file=f.root/'.swarm/provider-wait.json';assert not file.exists();s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');csrf=None;calls=[];checks=[];primary=None
    def request(path='/api/v1/provider-wait',body=None,status=200,code=None,method=None):
        """Use real cookie/Origin/CSRF, preserve safe response and exact status."""
        headers={'Cookie':s.cookie,'Origin':'http://'+s.address}
        if body is not None:headers.update({'Content-Type':'application/json','X-Swarm-CSRF':csrf})
        c=HTTPConnection(s.host,s.port,timeout=None)
        try:c.request(method or ('GET' if body is None else 'POST'),path,None if body is None else json.dumps(body).encode(),headers);r=c.getresponse();raw=r.read();actual=r.status
        finally:c.close()
        value=json.loads(raw);calls.append({'path':path,'status':actual,'response':clean(value)});retained.publish(output/'http.json',calls);assert actual==status,(actual,clean(value));
        if code is not None:assert value['failure']['code']==code,(code,clean(value))
        return value
    def unchanged():
        """No policy operation changes Work/Agent/reservations or starts a model."""
        assert f.snapshot()==before and not list(f.root.rglob('model-received.json'))
    try:
        csrf=request('/api/v1/native-session')['csrf_token'];assert 'values' in request('/api/v1/native-settings')
        defaults=json.loads(Path('config/provider-wait.json').read_bytes());view=request();assert view['schema_version']==1 and view['revision']==0 and view['values']==defaults and view['history']==[]
        assert not file.exists() and not (file.parent/'provider-wait.lock').exists();unchanged();checks.append('versioned-defaults-no-file-no-policy-lock')
        change={'schema_version':1,'event_id':'wait-first','expected_revision':0,'values':{**defaults,'max_duration_seconds':1},'reason':'Owned provider wait policy test'}
        pv=request('/api/v1/provider-wait/preview',change);assert not pv['applied'] and pv['config']['revision']==1 and pv['config']['values']==change['values'];assert not file.exists() and not (file.parent/'provider-wait.lock').exists()
        request('/api/v1/provider-wait/apply',change,status=400);assert not file.exists()
        applied=request('/api/v1/provider-wait/apply',{**change,'preview_token':pv['preview_token']});assert applied['applied'] and applied['config']['revision']==1
        original=file.read_bytes();stored=json.loads(original);assert stored['values']==change['values'] and stored['history'][0]['change']==change and 'preview_token' not in original.decode();assert file.stat().st_mode&0o777==0o600;unchanged();checks.append('preview-bound-atomic-save-original-change-history')
        replay=request('/api/v1/provider-wait/apply',{**change,'preview_token':pv['preview_token']});assert replay['replayed'] and replay['config']['revision']==1 and file.read_bytes()==original
        request('/api/v1/provider-wait/preview',{**change,'reason':'Different reuse'},status=409);assert file.read_bytes()==original
        request('/api/v1/provider-wait/preview',{**change,'event_id':'stale'},status=409);assert file.read_bytes()==original;unchanged();checks.append('exact-replay-conflicts-preserve-history')
        assert f.ok(['run-limits','provider-wait','show'])['values']==change['values'];assert len(f.ok(['run-limits','provider-wait','history']))==1
        fresh={**change,'event_id':'wait-unlimited','expected_revision':1,'values':{**defaults,'max_duration_seconds':0}}
        p=f.ok(['run-limits','provider-wait','preview'],fresh);result=f.ok(['run-limits','provider-wait','apply'],{**fresh,'preview_token':p['preview_token']});assert result['applied'] and result['config']['revision']==2
        assert request()['values']['max_duration_seconds']==0;unchanged();checks.append('same-cli-api-authority-unlimited-and-persistence')
        current=file.read_bytes();now=request();candidate={**fresh,'event_id':'wait-third','expected_revision':2}
        for bad in [{'schema_version':2},{'schema_version':1.0},{'values':{**defaults,'max_duration_seconds':'1'}},{'values':{**defaults,'silence_seconds':-1}},{'values':{**defaults,'planning_lease_seconds':0}},{'event_id':'../invalid'},{'reason':'  '},{'values':{**defaults,'max_duration_seconds':9223372037}}]:
            request('/api/v1/provider-wait/preview',{**candidate,**bad},status=400);assert file.read_bytes()==current
        assert request()['revision']==2;unchanged();checks.append('invalid-input-and-duration-representation-no-mutation')
        # Fixture-only source fault follows a valid control; original bytes restored.
        retained.publish(file,b'{"schema_version":1,"revision":0,"values":null,"history":[]}')
        request(status=500,code='provider-wait-source-invalid');retained.publish(file,current);assert request()['revision']==2
        # Go null/case/duplicate scalar semantics from the actual shared codec:
        # later null does not clear an existing value struct, history null remains.
        nullable=('{"schema_version":0,"SCHEMA_VERSION":1,"revision":0,"values":'+json.dumps(defaults)+',"VALUES":null,"history":null}').encode()
        retained.publish(file,nullable);legacy=request();assert legacy['revision']==0 and legacy['values']==defaults and legacy['history'] is None and file.read_bytes()==nullable
        retained.publish(file,current);assert request()['revision']==2;unchanged();checks.append('legacy-null-history-case-folding-and-duplicate-null-merge')
        altered=json.loads(current);altered['history'][1]['revision']=altered['history'][0]['revision'];retained.publish(file,altered);request(status=500,code='provider-wait-source-invalid');retained.publish(file,current)
        # A historical lease remains accepted after a new saved minimum excludes it.
        settings=f.ok(['engine-settings','show']);values=settings['values'];values['planning_scope_lease_min_seconds']=str(defaults['planning_lease_seconds']+1);values['planning_default_lease_seconds']=str(max(int(values['planning_default_lease_seconds']),defaults['planning_lease_seconds']+1))
        maximum=values['planning_scope_lease_max_seconds'];values['planning_scope_lease_max_seconds']=None if maximum is None else str(max(int(maximum),defaults['planning_lease_seconds']+1))
        setting_change={'schema_version':settings['schema_version'],'event_id':'wait-lease-bound','expected_revision':settings['revision'],'values':values,'reason':'Owned wait lease bounds test'}
        sp=f.ok(['engine-settings','preview'],setting_change);f.ok(['engine-settings','apply'],{**setting_change,'preview_token':sp['preview_token']})
        assert request()['revision']==2;assert request('/api/v1/provider-wait/apply',fresh)['replayed'];assert file.read_bytes()==current
        request('/api/v1/provider-wait/preview',candidate,status=400,code='provider-wait-lease-outside-policy');assert file.read_bytes()==current;unchanged();checks.append('stored-errors-recovery-and-current-bound-with-historical-replay')
        # CLI output failure AFTER exact replay must keep known state in JSON.
        out,err=output/'closed.stdout',output/'closed.stderr';input_file=output/'closed-input.json';retained.publish(input_file,fresh)
        child=subprocess.Popen([str(binary),'run-limits','provider-wait','apply','--root',str(f.root),'--input',str(input_file),'--json'],env=f.environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE);child.stdout.close();raw=child.stderr.read();child.stderr.close();exit_code=child.wait();retained.publish(err,raw);retained.publish(output/'closed-exit.json',{'exit':exit_code})
        assert exit_code==2 and json.loads(raw)['configuration_may_be_recorded'] is True;assert file.read_bytes()==current
        unchanged();checks.append('late-output-failure-retains-replayed-configuration')
        request('/api/v1/provider-wait',{},status=405);request('/api/v1/provider-wait/preview',status=405);request('/api/v1/provider-wait?unexpected=1',status=400);request(body={},method='GET',status=400);assert file.read_bytes()==current
        checks.append('method-query-body-refusals-without-mutation')
        # The actual Go-compatible lock refuses application, not read/preview.
        import fcntl,os
        next_change={**candidate,'event_id':'wait-lock-recovery','values':{**defaults,'planning_lease_seconds':defaults['planning_lease_seconds']+1},'reason':'Owned lock contention recovery'}
        lock_path=file.parent/'provider-wait.lock'
        with lock_path.open('r+b') as held:
            fcntl.flock(held,fcntl.LOCK_EX|fcntl.LOCK_NB)
            control=request();assert control['revision']==2
            pv=request('/api/v1/provider-wait/preview',next_change)
            request('/api/v1/provider-wait/apply',{**next_change,'preview_token':pv['preview_token']},status=500)
            assert file.read_bytes()==current
            fcntl.flock(held,fcntl.LOCK_UN)
        result=request('/api/v1/provider-wait/apply',{**next_change,'preview_token':pv['preview_token']});assert result['config']['revision']==3 and result['applied']
        assert (file.parent/result['backup_reference']).read_bytes()==current
        current=file.read_bytes();assert request('/api/v1/provider-wait/apply',change)['config']['revision']==3 and file.read_bytes()==current;unchanged()
        checks.append('real-policy-lock-contention-recovery-snapshot-and-oldest-replay')
        # Saved byte cap controls source reads and projected history, without a
        # copied file/history count limit. Only owned fixture settings are changed.
        settings=f.ok(['engine-settings','show']);original_values=settings['values'].copy();values=original_values.copy();values['agent_provider_config_max_bytes']=str(len(current))
        def set_values(values,event):
            control=f.ok(['engine-settings','show']);r={'schema_version':control['schema_version'],'event_id':event,'expected_revision':control['revision'],'values':values,'reason':'Owned provider file bound test'};pv=f.ok(['engine-settings','preview'],r);return f.ok(['engine-settings','apply'],{**r,'preview_token':pv['preview_token']})
        set_values(values,'wait-source-cap')
        assert request()['revision']==3
        oversized={**next_change,'event_id':'wait-history-too-large','expected_revision':3,'reason':'r'*len(current)}
        request('/api/v1/provider-wait/preview',oversized,status=400,code='provider-wait-file-outside-saved-policy');assert file.read_bytes()==current
        values['agent_provider_config_max_bytes']='0';set_values(values,'wait-source-cap-zero');request(status=500);assert file.read_bytes()==current
        set_values(original_values,'wait-source-cap-restore');assert request()['revision']==3;unchanged();checks.append('saved-source-and-projected-history-caps-refusal-recovery')
        # New HTTP process consumes the exact same saved Root without replaying
        # applications. CLI/read process persistence was already checked above.
        s.cleanup();assert not s.capture_errors and s.process.returncode==0
        s=Server(binary,f.root,output,'http-restarted',environment=f.environment);s.capture_async(output/'stream-restarted.jsonl');csrf=request('/api/v1/native-session')['csrf_token']
        assert request()['revision']==3 and file.read_bytes()==current;unchanged();checks.append('owned-http-restart-preserves-values-history-and-inert-state')


    except BaseException as error:primary=error
    try:s.cleanup()
    except BaseException as cleanup:
        if primary:raise BaseExceptionGroup('Wait policy and owned cleanup failed',[primary,cleanup])
        raise
    if primary:raise primary
    assert len(checks)==12 and not s.capture_errors and s.process.returncode==0
    retained.publish(output/'result.json',{'status':'PASS','checks':checks,'provider_started':False,'sha256':retained.digest(binary.read_bytes())});print('PROVIDER_WAIT_POLICY_VERIFIED 12',flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():retained.publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
