#!/usr/bin/env python3
"""Exercise native provider Admin inventory on one owned inert Root.

Five named obligations; no remote provider, native mission, budget reset, SQL
mutation, retry or global watchdog. Command path is intentional Admin data;
private args/env/credentials are not response fields. Original failures survive.
"""
import argparse,copy,json,os,sys,traceback,socket
from http.client import HTTPConnection
from pathlib import Path
import rust_agent_model_route as retained
from rust_native_http_cli import Server,clean
from rust_provider_policy_http import restoration

OBLIGATIONS={
    'same-source-digest-saved-policy-and-actual-local-model-options',
    'missing-policy-and-custom-inventory-never-invent-model-defaults',
    'command-health-and-helper-causes-are-visible-retained-and-recoverable',
    'stored-catalogue-source-policy-errors-refuse-and-recover',
    'query-body-method-session-and-head-contract',
}


def run(binary,output):
    """Verify actual protected inventory, source preservation and diagnostics.

    Normal build qualification remains external. Public CLI writes fixture config
    only; file/mode faults restore original bytes and permission. Typed source
    errors remain errors, while expected command unavailability is an attributed
    row and observer cause. No successful metadata read grants execution.
    """
    output.mkdir(parents=True,exist_ok=False);f=retained.Fixture(binary,output,'provider-admin')
    f.provider['args'].insert(-1,'PRIVATE_ADMIN_ARGUMENT');f.save_provider()
    cache={'models':[
        {'slug':'fixture-standard','visibility':'list','supported_reasoning_levels':[{'effort':'fixture-medium'}]},
        {'slug':'fixture-standard','visibility':'list','supported_reasoning_levels':[{'effort':'fixture-high'}]},
        {'slug':'fixture-simple','visibility':'list'},
        {'slug':'fixture-frontier','visibility':'list'},
        {'slug':'hidden-fixture','visibility':'hidden'},
        {'slug':'not valid','visibility':'list'}]};retained.publish(f.cache,cache)
    before=f.snapshot();source=f.providers.read_bytes();cache_before=f.cache.read_bytes();initial=f.ok(['providers','config','show'])
    s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');records=[];checks=[];primary=None
    def request(*,path='/api/v1/providers/admin',method='GET',body=None,status=200,authorized=True):
        """Require protected HTTP content and retain only redacted observations."""
        headers={'Origin':'http://'+s.address}
        if authorized:headers['Cookie']=s.cookie
        if method not in ['GET','HEAD']:headers['X-Swarm-CSRF']=csrf
        raw=body if isinstance(body,bytes) else None if body is None else json.dumps(body).encode()
        if raw is not None:headers['Content-Type']='application/json'
        c=HTTPConnection(s.host,s.port,timeout=None)
        try:c.request(method,path,raw,headers);response=c.getresponse();data=response.read();actual=response.status
        finally:c.close()
        value=None if method=='HEAD' else json.loads(data) if data.startswith(b'{') else data.decode()
        records.append({'path':path,'method':method,'status':actual,'response':clean(value)});retained.publish(output/'http.json',records)
        assert actual==status,(path,actual,clean(value))
        assert 'PRIVATE_ADMIN_ARGUMENT' not in data.decode()
        return value
    def unchanged():
        """Assert logical Work/Agent/reservation state and exact saved source/cache."""
        assert f.snapshot()==before and f.providers.read_bytes()==source and f.cache.read_bytes()==cache_before
        assert not list(f.root.rglob('model-received.json'))
    def positive():
        """Get the valid inventory before each refusal and assert source binding."""
        view=request();assert view['version']==1 and view['digest']==retained.digest(f.providers.read_bytes())
        assert view['provider_execution_started'] is False and view['providers'];unchanged();return view
    def save(configuration):
        """Use real preview/apply for fixture configuration, retaining exact state."""
        nonlocal source
        change,_=retained.configuration_proposal(f,configuration=configuration);f.ok(['providers','config','apply'],change);source=f.providers.read_bytes();unchanged()
    try:
        csrf=request(path='/api/v1/native-session')['csrf_token']
        view=positive();row=view['providers']['fixture'];assert row['adapter']=='codex' and row['command']==f.provider['command']
        assert row['saved'] and row['policy']==f.provider['model_policy'] and row['assistant_unavailable']=='' and row['health_issue'] is None
        assert view['export']==f.ok(['providers','policies','export'])
        assert [m['id'] for m in row['models']]==['fixture-frontier','fixture-old','fixture-simple','fixture-standard']
        by_id={m['id']:m for m in row['models']};assert by_id['fixture-standard']['efforts']==['fixture-high','fixture-medium']
        assert by_id['fixture-standard']['source']=='local_cache_unverified' and by_id['fixture-old']['source']=='configured_argv_unverified'
        assert set(row).isdisjoint({'args','env_allow','key','configuration'});unchanged()
        checks.append('same-source-digest-saved-policy-and-actual-local-model-options')
        original=f.ok(['providers','config','show'])['configuration'];configuration=copy.deepcopy(original)
        missing=copy.deepcopy(configuration['providers']['fixture']);missing.pop('model_policy');configuration['providers']['missing-policy']=missing
        custom=f.root/'plain-custom';retained.publish(custom,b"#!/usr/bin/python3\nfrom pathlib import Path\nPath(__file__).with_name('model-received.json').write_text('UNEXPECTED')\nraise SystemExit(97)\n");os.chmod(custom,0o700)
        configuration['providers']['custom']={'command':str(custom),'args':[]}

        save(configuration);view=positive()
        for name in ['missing-policy','custom']:assert view['providers'][name]['policy'] is None and not view['providers'][name]['saved']
        assert set(view['export']['policies'])=={'fixture'}
        assert view['providers']['custom']['models']==[] and view['providers']['custom']['model_options_state']=='no_local_model_declaration'
        assert view['providers']['custom']['health_issue'] is None and view['providers']['custom']['assistant_unavailable']=='assistant-provider-adapter-unported'
        save(original);positive();checks.append('missing-policy-and-custom-inventory-never-invent-model-defaults')
        command=f.root/'codex';mode=command.stat().st_mode & 0o777
        positive()
        with restoration(lambda: os.chmod(command,mode)):
            os.chmod(command,0o600);view=positive();row=view['providers']['fixture']
            assert row['health_issue']=='provider-admin-command-not-executable-file' and row['assistant_unavailable']=='assistant-provider-executable-unavailable'
        positive()
        absent=command.with_suffix('.absent')
        with restoration(lambda: absent.rename(command)):
            command.rename(absent);view=positive();row=view['providers']['fixture']
            assert row['health_issue']=='provider-admin-command-metadata' and row['assistant_unavailable']=='assistant-provider-executable-unavailable'
        assert command.stat().st_mode & 0o777==mode;positive();unchanged()
        relative=copy.deepcopy(original);relative['providers']['fixture']['command']='codex';save(relative)
        row=positive()['providers']['fixture'];assert row['health_issue']=='provider-admin-command-absolute-required'
        assert row['assistant_unavailable']=='assistant-provider-absolute-command-required'
        save(original);positive();unchanged()
        checks.append('command-health-and-helper-causes-are-visible-retained-and-recoverable')
        positive()
        with restoration(lambda: retained.publish(f.cache,cache_before)):
            retained.publish(f.cache,b'PRIVATE malformed cache');bad=request(status=500);assert bad['failure']['code']=='agent-model-catalogue-invalid'
        positive()
        with restoration(lambda: retained.publish(f.providers,source)):
            retained.publish(f.providers,b'PRIVATE malformed source');request(status=500)
        positive()
        with restoration(lambda: retained.publish(f.providers,source)):
            bad=copy.deepcopy(json.loads(source));bad['providers']['fixture']['model_policy']['levels']={};retained.publish(f.providers,bad)
            value=request(status=500);assert value['failure']['code']=='agent-model-policy-invalid'
        positive();unchanged();checks.append('stored-catalogue-source-policy-errors-refuse-and-recover')
        for params in [dict(path='/api/v1/providers/admin?extra=1',status=400),dict(body=b'{}',status=400),dict(method='POST',body={},status=405),dict(authorized=False,status=403)]:
            positive();request(**params);unchanged()
        assert request(method='HEAD') is None
        # http.client deliberately suppresses HEAD bodies; inspect raw bytes too.
        with socket.create_connection((s.host,s.port),timeout=None) as raw_head:
            raw_head.sendall(('HEAD /api/v1/providers/admin HTTP/1.1\r\nHost: '+s.address+'\r\nCookie: '+s.cookie+'\r\nConnection: close\r\n\r\n').encode())
            chunks=[]
            while True:
                chunk=raw_head.recv(65536)
                if not chunk:break
                chunks.append(chunk)
        headers,body=b''.join(chunks).split(b'\r\n\r\n',1)
        assert headers.startswith(b'HTTP/1.1 200') and body==b''
        retained.publish(output/'raw-head.json',{'status':200,'body_bytes':len(body)})
        unchanged();checks.append('query-body-method-session-and-head-contract')
    except BaseException as error:primary=error
    try:s.cleanup()
    except BaseException as cleanup:
        if primary:raise BaseExceptionGroup('Provider Admin and owned cleanup failed',[primary,cleanup])
        raise
    if primary:raise primary
    assert set(checks)==OBLIGATIONS and len(checks)==len(set(checks)) and not s.capture_errors and s.process.returncode==0
    outcomes=[r['outcome'] for r in s.records if r.get('event')=='web-outcome']
    # Unavailable-file observations must reach actual transport diagnostics, not
    # disappear inside a successful inventory body. Protocol producer is retained.
    retained.publish(output/'outcome-observations.json',outcomes)
    codes=[code for outcome in outcomes if 'observation_causes' in outcome for code in outcome['observation_causes']]
    assert 'provider-admin-command-metadata' in codes and 'assistant-provider-executable-unavailable' in codes
    assert 'provider-admin-command-absolute-required' in codes and 'assistant-provider-adapter-unported' in codes
    result={'status':'PASS','checks':checks,'root':str(f.root),'binary':str(binary),'sha256':retained.digest(binary.read_bytes()),'provider_started':False,'mission_accepted':False,'observation_causes_retained':True}
    retained.publish(output/'result.json',result);return result


def main():
    """Require fresh evidence, retaining original failures without global timeout."""
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    try:print(json.dumps(run(a.binary.resolve(),a.output.resolve())))
    except BaseException:
        if a.output.exists():retained.publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise

if __name__=='__main__':main()
