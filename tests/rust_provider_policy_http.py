#!/usr/bin/env python3
"""Exercise protected provider-policy HTTP and CLI on an owned inert Root.

No provider launches, real credentials, live missions, arbitrary watchdog or
SQL mutations. Policies/digest are public configuration, never availability.
Retain exact CLI channels and redacted HTTP observations; failed fixtures survive.
"""
import argparse,copy,http.client,json,sys,traceback
from contextlib import contextmanager
from pathlib import Path
import rust_agent_model_route as retained
from rust_native_http_cli import Server,clean


@contextmanager
def restoration(cleanup):
    """Restore an owned fixture and preserve both original and cleanup failures.

    No retry or rollback is inferred if cleanup fails; both causes are raised.
    """
    primary=None
    try:yield
    except BaseException as error:primary=error
    try:cleanup()
    except BaseException as error:
        if primary:raise BaseExceptionGroup('Fixture fault and restoration failed',[primary,error])
        raise
    if primary:raise primary


def run(binary,output):
    """Verify read/preview/apply/refusal/recovery and future model routing.

    Require a normal supplied binary; build-profile qualification is external.
    File faults touch only this born fixture and restore originals in finally.
    Public commands alone mutate Work/settings/configuration. Propagate primary
    and cleanup errors separately; no retry or forced success is provided.
    """
    output.mkdir(parents=True,exist_ok=False)
    f=retained.Fixture(binary,output,'policies')
    retained.publish(f.cache,{'models':[
        {'slug':'fixture-simple','visibility':'list'},
        {'slug':'fixture-standard','visibility':'list','supported_reasoning_levels':[{'effort':'fixture-medium'}]},
        {'slug':'fixture-frontier','visibility':'list'}]})
    before=f.snapshot();route=f.prepare(True)['agent']['model_route']
    portable=f.ok(['providers','policies','export']);assert set(portable)=={'version','policies'}
    original=f.providers.read_bytes();saved=f.ok(['providers','config','show'])['configuration']
    s=Server(binary,f.root,output,'http',environment=f.environment)
    observations=[];checks=[]
    s.capture_async(output/'http-stream.jsonl')
    def request(method='GET',body=None,path='/api/v1/providers/policies',status=200,authorized=True,csrf=True):
        """Send actual bytes and require status/content; record no private tokens.

        Use the owned stream observer independently to avoid pipe backpressure.
        HEAD has no entity. Cookie/CSRF failures happen before Root dispatch.
        """
        headers={'Origin':'http://'+s.address}
        if authorized:headers['Cookie']=s.cookie
        if csrf and method not in ('GET','HEAD'):headers['X-Swarm-CSRF']=token
        raw=body if isinstance(body,bytes) else None if body is None else json.dumps(body,separators=(',',':')).encode()
        if raw is not None:headers['Content-Type']='application/json'
        c=http.client.HTTPConnection(s.host,s.port,timeout=None)
        try:
            c.request(method,path,raw,headers);response=c.getresponse();data=response.read();actual=response.status
        finally:c.close()
        value=None if method=='HEAD' else json.loads(data) if data.startswith(b'{') else data.decode()
        observations.append(dict(method=method,path=path,status=actual,body=clean(value)))
        retained.publish(output/'http-observations.json',observations)
        assert actual==status,(method,path,actual,clean(value))
        assert str(f.root/'codex') not in data.decode() and 'fixture-old' not in data.decode()
        return value
    def unchanged(raw):
        """Reject writes, provider launches, reservations and Work drift explicitly."""
        assert f.providers.read_bytes()==raw and f.snapshot()==before
        assert not list(f.root.rglob('model-received.json'))
    def positive():
        """A valid protected saved view is the control before each input fault."""
        view=request();assert view['policies']==f.ok(['providers','policies','export'])['policies']
        assert view['digest']==retained.digest(f.providers.read_bytes())
        return view
    primary=None
    try:
        token=request(path='/api/v1/native-session')['csrf_token']
        view=positive();assert set(view)=={'version','digest','policies'}
        assert request(method='HEAD') is None;unchanged(original)
        checks.append('source-bound-get-head-and-unchanged-cli-portable-export')
        policy=copy.deepcopy(portable['policies']);policy['fixture']['work_level']='simple'
        change=dict(version=1,expected_digest=view['digest'],policies=policy,preview=True)
        files=set((f.root/'.swarm').rglob('*'))
        preview=request('POST',change);assert preview['preview'] and not preview['applied'] and preview['changed']
        assert not preview['provider_execution_started'] and set((f.root/'.swarm').rglob('*'))==files
        unchanged(original)
        # Inherited duplicate/case/null merge must select preview, not a writer.
        wire=json.dumps(change).replace('"preview": true','"preview": false, "PREVIEW": true, "preview": null').encode()
        merged=request('POST',wire);assert merged['preview'] and not merged['applied'];unchanged(original)
        checks.append('preview-and-go-duplicate-case-null-semantics-without-writes')
        invalid_policy=copy.deepcopy(policy);invalid_policy['fixture']['levels']={}
        for patch in [dict(policies=None),dict(policies={}),dict(policies={'fixture':None}),dict(key='PRIVATE'),dict(preview='true'),dict(policies=invalid_policy)]:
            positive();bad=dict(change,**patch);http_wire=request('POST',bad,status=400)
            code,cli=f.call(['providers','policies','preview'],bad);assert code and cli['failure']['code']==http_wire['failure']['code']
            unchanged(original)
        positive();request('POST',dict(change,policies={'unknown':policy['fixture']}),status=400);unchanged(original)
        checks.append('strict-input-cli-parity-and-unknown-provider-refusal')
        for method,path,body,status,auth,csrf in [
            ('GET','/api/v1/providers/policies?extra=1',None,400,True,True),
            ('GET','/api/v1/providers/policies',b'{}',400,True,True),
            ('PUT','/api/v1/providers/policies',b'{}',405,True,True),
            ('GET','/api/v1/providers/policies',None,403,False,True),
            ('POST','/api/v1/providers/policies',change,403,True,False)]:
            positive();request(method,body,path,status,auth,csrf);unchanged(original)
        checks.append('method-query-body-session-and-csrf-refusals')
        apply=dict(change,preview=False,preview_token=preview['preview_token'])
        positive();request('POST',dict(apply,preview_token='wrong'),status=409);unchanged(original)
        positive();request('POST',dict(apply,expected_digest='0'*64),status=409);unchanged(original)
        applied=request('POST',apply);assert applied['applied'] and applied['changed'] and not applied['provider_execution_started']
        current=f.providers.read_bytes();stored=json.loads(current)
        old=saved['providers']['fixture'];new=stored['providers']['fixture']
        assert {k:v for k,v in old.items() if k!='model_policy'}=={k:v for k,v in new.items() if k!='model_policy'}
        assert (f.root/'.swarm'/applied['backup_reference']).read_bytes()==original
        view=positive();assert view['policies']==policy and view['digest']==applied['after_digest']
        request('POST',apply,status=409);unchanged(current)
        checks.append('token-digest-cas-persistent-apply-exact-backup-and-command-preservation')
        next_route=f.prepare(True)['agent']['model_route']
        assert next_route['level']=='simple' and next_route['model']=='fixture-simple'
        assert next_route['policy_hash']!=route['policy_hash'];unchanged(current)
        checks.append('next-agent-preview-consumes-saved-policy-without-admission')
        same=dict(version=1,expected_digest=view['digest'],policies=policy,preview=True)
        backups=set((f.root/'.swarm').glob('providers-before-*.json'))
        noop_preview=request('POST',same);assert not noop_preview['changed'] and noop_preview['preview']
        same.update(preview=False,preview_token=noop_preview['preview_token'])
        noop=request('POST',same)
        assert noop['applied'] and not noop['changed'] and noop['backup_reference'] is None
        assert noop['before_digest']==noop['after_digest']==view['digest']
        assert set((f.root/'.swarm').glob('providers-before-*.json'))==backups;unchanged(current)
        checks.append('no-op-application-reconfirms-without-new-bytes-or-backup')
        # Actual file corruption belongs to the server, not the caller.
        positive()
        with restoration(lambda: retained.publish(f.providers,current)):
            retained.publish(f.providers,b'PRIVATE malformed source')
            bad=request(status=500);assert bad['failure']['code']!='web-route-unported'
        positive();unchanged(current)
        positive()
        with restoration(lambda: f.providers.with_suffix('.missing').rename(f.providers)):
            f.providers.rename(f.providers.with_suffix('.missing'))
            request(status=500)
        positive();unchanged(current)
        checks.append('stored-corruption-missing-source-safe-errors-and-recovery')
    except BaseException as error:primary=error
    try:s.cleanup()
    except BaseException as cleanup:
        if primary:raise BaseExceptionGroup('Policy journey and owned cleanup failed',[primary,cleanup])
        raise
    if primary:raise primary
    assert len(checks)==8 and not s.capture_errors and s.process.returncode==0
    result=dict(status='PASS',checks=checks,root=str(f.root),binary=str(binary),binary_sha256=retained.digest(binary.read_bytes()),provider_started=False,mission_accepted=False,initial_model=route['model'],next_model=next_route['model'])
    retained.publish(output/'result.json',result);return result


def main():
    """Run one fresh journey; preserve failed originals and a safe failure marker."""
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    try:
        result=run(a.binary.resolve(),a.output.resolve());print(json.dumps(result));return 0
    except BaseException:
        if a.output.exists():retained.publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise

if __name__=='__main__':sys.exit(main())
