#!/usr/bin/env python3
"""Verify configured-model resolution on protected HTTP and owned inert fixtures.

Six named obligations; no real provider, native mission, acceptance, hidden retry
or global watchdog. Shared public policy/configuration writers change fixtures
only. Metadata reads never reserve or execute; declarations remain unverified.
"""
import argparse,copy,json,sys,traceback
from http.client import HTTPConnection
from pathlib import Path
from urllib.parse import urlencode
import rust_agent_model_route as retained
from rust_native_http_cli import Server,clean
from rust_provider_policy_http import restoration

OBLIGATIONS={
    'work-page-all-purposes-and-explicit-tier-resolution',
    'strict-query-body-method-and-session-refusals',
    'automatic-page-default-stays-subject-to-saved-admin-policy',
    'current-public-policy-update-consumed-without-admission',
    'stored-policy-catalogue-missing-source-errors-and-recovery',
    'existing-custom-null-is-distinct-from-unknown-or-implicit-known-provider',
}


def run(binary,output):
    """Exercise actual Root-backed HTTP with one owned process and restored faults.

    A supplied normal build/profile is qualified separately. Retain meaningful
    responses and original CLI channels privately; never log credentials. Fixture
    source faults restore exact originals and preserve primary/cleanup causes.
    """
    output.mkdir(parents=True,exist_ok=False);f=retained.Fixture(binary,output,'model-read')
    f.provider['args'].insert(-1,'PRIVATE_MODEL_ARGUMENT');f.save_provider()
    retained.publish(f.cache,{'models':[
        {'slug':'fixture-simple','visibility':'list'},
        {'slug':'fixture-standard','visibility':'list','supported_reasoning_levels':[{'effort':'fixture-medium'}]},
        {'slug':'fixture-frontier','visibility':'list'}]})
    before=f.snapshot();source_before=f.providers.read_bytes();initial=f.prepare(True)['agent']['model_route']
    s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl')
    observations=[];checks=[];primary=None
    def request(path,*,method='GET',body=None,status=200,authorized=True):
        """Send real HTTP with exact owned cookie; redact tokens and require status.

        Observe asynchronously so HTTP callbacks cannot block on stdout. Common
        gates run before route semantics; HEAD must return no entity.
        """
        headers={'Origin':'http://'+s.address}
        if authorized:headers['Cookie']=s.cookie
        if method not in ['GET','HEAD']:headers['X-Swarm-CSRF']=csrf
        raw=body if isinstance(body,bytes) else None if body is None else json.dumps(body,separators=(',',':')).encode()
        if raw is not None:headers['Content-Type']='application/json'
        connection=HTTPConnection(s.host,s.port,timeout=None)
        try:
            connection.request(method,path,raw,headers);response=connection.getresponse();data=response.read();actual=response.status
        finally:connection.close()
        value=None if method=='HEAD' else json.loads(data) if data.startswith(b'{') else data.decode()
        observations.append({'path':path,'method':method,'status':actual,'response':clean(value)});retained.publish(output/'http.json',observations)
        assert actual==status,(path,actual,clean(value))
        assert 'PRIVATE_CONNECTION_SENTINEL' not in data.decode() and 'PRIVATE_MODEL_ARGUMENT' not in data.decode() and str(f.root/'codex') not in data.decode()
        return value
    def path(provider='fixture',purpose='work',level=None):
        """Encode one ordinary request; explicit None means inherited default."""
        values={'provider':provider,'purpose':purpose}
        if level is not None:values['level']=level
        return '/api/v1/providers/resolve?'+urlencode(values)
    def unchanged():
        """Assert exact source and logical Work/no Agent/reservation/process effects."""
        assert f.providers.read_bytes()==source_before and f.snapshot()==before
        assert not list(f.root.rglob('model-received.json'))
    def positive():
        """Use the actual valid work read as a control before every fault."""
        value=request(path());assert set(value)=={'route'} and value['route']['model'];unchanged();return value['route']
    try:
        csrf=request('/api/v1/native-session')['csrf_token']
        work=positive();assert work==initial
        for purpose,model in [('work','fixture-standard'),('page','fixture-simple'),('brainstorm','fixture-standard'),('preparation','fixture-standard'),('planning','fixture-standard')]:
            route=request(path(purpose=purpose))['route'];assert route['model']==model and '· '+purpose+' ·' in route['reason']
        assert request(path(level='auto'))['route']==work
        for level,model in [('simple','fixture-simple'),('standard','fixture-standard'),('exigeant','fixture-frontier')]:
            route=request(path(level=level))['route'];assert route['level']==level and route['model']==model
        assert request(path(),method='HEAD') is None;unchanged();checks.append('work-page-all-purposes-and-explicit-tier-resolution')
        for target in ['/api/v1/providers/resolve','/api/v1/providers/resolve?provider=fixture','/api/v1/providers/resolve?purpose=work',path()+'&provider=fixture',path()+'&extra=1',path(provider='unknown'),path(provider='fixture%FF'),path(purpose='unknown'),path(level='unknown'),'/api/v1/providers/resolve?provider=%FF&purpose=work']:
            positive();request(target,status=400);unchanged()
        for method,body,status,auth in [('GET',b'{}',400,True),('POST',{},405,True),('GET',None,403,False)]:
            positive();request(path(),method=method,body=body,status=status,authorized=auth);unchanged()
        checks.append('strict-query-body-method-and-session-refusals')
        saved_levels=f.ok(['engine-settings','show'])['values']['agent_model_automatic_levels']
        f.settings_change(agent_model_automatic_levels=['standard']);positive()
        denied=request(path(purpose='page'),status=400);assert denied['failure']['code']=='settings-model-automatic-level-denied'
        assert request(path(purpose='page',level='simple'))['route']['model']=='fixture-simple';unchanged()
        f.settings_change(agent_model_automatic_levels=saved_levels);positive()
        saved_grant=f.ok(['engine-settings','show'])['values']['agent_allow_unverified_provider_context']
        f.settings_change(agent_allow_unverified_provider_context='0');positive()
        f.denied(['agent','preview',f.work],f.request,'agent-provider-verified-context-required');unchanged()
        f.settings_change(agent_allow_unverified_provider_context=saved_grant);positive()
        stored_provider=copy.deepcopy(f.provider)
        with restoration(lambda: retained.publish(f.providers,source_before)):
            f.provider['model_policy']['page_level']='exigeant';f.save_provider()
            denied=request(path(purpose='page'),status=400);assert denied['failure']['code']=='settings-model-automatic-level-denied'
            assert request(path(purpose='page',level='exigeant'))['route']['model']=='fixture-frontier'
        f.provider=stored_provider;positive();checks.append('automatic-page-default-stays-subject-to-saved-admin-policy')
        policies=request('/api/v1/providers/policies');proposal=copy.deepcopy(policies['policies'])
        proposal['fixture']['work_level']='simple';proposal['fixture']['page_level']='standard'
        change={'version':1,'expected_digest':policies['digest'],'policies':proposal,'preview':True}
        preview=request('/api/v1/providers/policies',method='POST',body=change)
        assert preview['preview'] and preview['changed'];unchanged()
        change.update(preview=False,preview_token=preview['preview_token']);applied=request('/api/v1/providers/policies',method='POST',body=change)
        assert applied['applied'] and applied['changed'] and not applied['provider_execution_started']
        source_before=f.providers.read_bytes();new=positive();assert new['model']=='fixture-simple' and new['policy_hash']!=work['policy_hash']
        assert request(path(purpose='page'))['route']['model']=='fixture-standard'
        assert f.prepare(True)['agent']['model_route']==new;unchanged();checks.append('current-public-policy-update-consumed-without-admission')
        positive()
        with restoration(lambda: retained.publish(f.providers,source_before)):
            retained.publish(f.providers,b'PRIVATE malformed source');request(path(),status=500)
        positive()
        with restoration(lambda: f.providers.with_suffix('.missing').rename(f.providers)):
            f.providers.rename(f.providers.with_suffix('.missing'));request(path(),status=500)
        positive();cache=f.cache.read_bytes()
        with restoration(lambda: retained.publish(f.cache,cache)):
            retained.publish(f.cache,b'PRIVATE malformed cache');bad=request(path(),status=500);assert bad['failure']['code']=='agent-model-catalogue-invalid'
        positive();checks.append('stored-policy-catalogue-missing-source-errors-and-recovery')
        configuration=f.ok(['providers','config','show'])['configuration'];updated=copy.deepcopy(configuration)
        updated['providers']['custom']={'command':str(f.root/'plain-custom'),'args':[]}
        missing=copy.deepcopy(updated['providers']['fixture']);missing.pop('model_policy');updated['providers']['missing-policy']=missing
        updated['providers']['api-collision']=copy.deepcopy(updated['providers']['fixture'])
        change,_=retained.configuration_proposal(f,configuration=updated);f.ok(['providers','config','apply'],change);source_before=f.providers.read_bytes();positive()
        assert request(path(provider='custom'))=={'route':None};unchanged()
        positive();request(path(provider='custom',level='standard'),status=400);unchanged()
        positive();bad=request(path(provider='missing-policy'),status=500);assert bad['failure']['code']=='agent-model-saved-policy-required';unchanged()
        connections=f.root/'.swarm/ai-connections.json'
        original_connections=connections.read_bytes() if connections.exists() else None
        def restore_connections():
            """Restore only this fixture's prior namespace bytes or actual absence."""
            if original_connections is None:connections.unlink()
            else:retained.publish(connections,original_connections)
        with restoration(restore_connections):
            metadata={'collision':{'id':'collision','label':'inert namespace','base_url':'http://127.0.0.1:1','model':'fixture-api','key':'PRIVATE_CONNECTION_SENTINEL','disabled':True}}
            retained.publish(connections,metadata);disabled=connections.read_bytes();positive()
            assert request(path(provider='api-collision'))['route']['model']=='fixture-simple'
            f.prepare(True);unchanged()
            with restoration(lambda: retained.publish(connections,disabled)):
                metadata['collision']['disabled']=False;retained.publish(connections,metadata)
                f.denied(['agent','preview',f.work],f.request,'agent-provider-api-name-conflict')
                bad=request(path(),status=500);assert bad['failure']['code']=='agent-provider-api-name-conflict'
            positive();unchanged()
            metadata['collision']['disabled']=True
            metadata['only']={'id':'only','label':'inert selected API','base_url':'http://127.0.0.1:1','model':'fixture-api','key':'PRIVATE_CONNECTION_SENTINEL','disabled':False}
            retained.publish(connections,metadata);positive()
            api_request=dict(f.request,provider='api-only')
            f.denied(['agent','preview',f.work],api_request,'agent-api-provider-execution-unported')
            bad=request(path(provider='api-only'),status=501);assert bad['failure']['code']=='agent-api-provider-execution-unported';unchanged()
            metadata['only']['disabled']=True;retained.publish(connections,metadata);positive()
            request(path(provider='api-only'),status=400);unchanged()
        assert (connections.read_bytes() if connections.exists() else None)==original_connections
        change,_=retained.configuration_proposal(f,configuration=configuration);f.ok(['providers','config','apply'],change);source_before=f.providers.read_bytes();positive()
        checks.append('existing-custom-null-is-distinct-from-unknown-or-implicit-known-provider')
    except BaseException as error:primary=error
    try:s.cleanup()
    except BaseException as cleanup:
        if primary:raise BaseExceptionGroup('Model resolution and owned cleanup failed',[primary,cleanup])
        raise
    if primary:raise primary
    assert set(checks)==OBLIGATIONS and len(checks)==len(set(checks)) and not s.capture_errors and s.process.returncode==0
    result={'status':'PASS','checks':checks,'root':str(f.root),'binary':str(binary),'binary_sha256':retained.digest(binary.read_bytes()),'provider_started':False,'mission_accepted':False}
    retained.publish(output/'result.json',result);return result


def main():
    """Run one fresh fixture journey, retaining original failure diagnostics."""
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    try:print(json.dumps(run(a.binary.resolve(),a.output.resolve())));return 0
    except BaseException:
        if a.output.exists():retained.publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise

if __name__=='__main__':sys.exit(main())
