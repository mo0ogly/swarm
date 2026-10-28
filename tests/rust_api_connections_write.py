#!/usr/bin/env python3
"""Verify native connection preview/apply on an owned inert Root, no remote call.

Fake key values are fixture data. Raw response proof excludes keys/tokens; source
snapshots are private fixture evidence, not production credentials or acceptance.
"""
import argparse,json,copy,traceback,subprocess
from pathlib import Path
from http.client import HTTPConnection
import rust_agent_model_route as retained
from rust_native_http_cli import Server,clean

def run(binary,output):
    """Require source-bound mutation and actual private file effects, not200 alone.

    The Root/server belong to this fixture; original failures and owned cleanup
    causes survive. No live mission, provider, reset, retry or global watchdog.
    """
    output.mkdir(parents=True,exist_ok=False);f=retained.Fixture(binary,output,'connections-write');before=f.snapshot();file=f.root/'.swarm/ai-connections.json';s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');csrf=None;calls=[];checks=[];primary=None
    def request(body=None,*,status=200,path='/api/v1/providers/connections',expected_code=None):
        """Use actual session/Origin/CSRF, retain safe redacted output only."""
        headers={'Cookie':s.cookie,'Origin':'http://'+s.address}
        if body is not None:headers.update({'X-Swarm-CSRF':csrf,'Content-Type':'application/json'})
        c=HTTPConnection(s.host,s.port,timeout=None)
        try:c.request('POST' if body is not None else 'GET',path,None if body is None else json.dumps(body).encode(),headers);r=c.getresponse();raw=r.read();actual=r.status
        finally:c.close()
        value=json.loads(raw);assert 'PRIVATE_CONNECTION_' not in raw.decode();calls.append({'status':actual,'path':path,'method':'POST' if body is not None else 'GET','response':clean(value)});retained.publish(output/'http.json',calls);assert actual==status,(actual,clean(value))
        if expected_code is not None:assert value.get('failure',{}).get('code')==expected_code,clean(value)
        return value
    def current():
        """Read current redacted catalogue through the actual protected endpoint."""
        return request()
    def unchanged():
        """Compare Work/Agent/reservation state and no provider child evidence."""
        assert f.snapshot()==before and not list(f.root.rglob('model-received.json'))
    def proposal(connection,replace=False):
        """Bind fixture intent to the currently observed source digest."""
        return {'version':1,'expected_digest':current()['digest'],'connection':connection,'replace_key':replace,'preview':True}
    def preview(change):
        """Require preview state and exact source preservation before each fault."""
        original=file.read_bytes() if file.exists() else None;lock=f.root/'.swarm/providers.lock';had_lock=lock.exists();v=request(change);assert v['preview'] and not v['applied'] and not v['saved'] and len(v['preview_token'])==64
        assert (file.read_bytes() if file.exists() else None)==original and lock.exists()==had_lock;unchanged();return v
    def apply(change):
        """Apply one current preview and assert saved source digest/private state."""
        original=file.read_bytes() if file.exists() else None
        v=preview(change);r={**change,'preview':False,'preview_token':v['preview_token']};done=request(r);assert done['saved'] and done['applied'] and not done['preview'];assert current()['digest']==done['after_digest']==retained.digest(file.read_bytes())
        assert file.stat().st_mode & 0o777==0o600
        if done['changed'] and original is not None:
            reference=done['backup_reference'];assert '/' not in reference and reference.startswith('api-connections-before-')
            backup=file.parent/reference;assert backup.read_bytes()==original and backup.stat().st_mode & 0o777==0o600
        else:assert done['backup_reference'] is None
        unchanged();return done
    try:
        csrf=request(path='/api/v1/native-session')['csrf_token'];assert current()['connections']==[]
        connection={'id':'local','label':'Owned API','base_url':'https://example.invalid/v1','model':'fixture-model','key':'PRIVATE_CONNECTION_FIRST','disabled':False}
        change=proposal(connection,True);v=preview(change)
        request({**change,'preview':False},status=400,expected_code='api-connection-preview-required');assert not file.exists()
        # Inherited Go intent: false ignores submitted key even on first create.
        apply(proposal(connection,False));assert not current()['connections'][0]['has_key']
        done=apply(proposal(connection,True));assert done['changed'] and json.loads(file.read_bytes())['local']['key']=='PRIVATE_CONNECTION_FIRST';assert current()['connections'][0]['has_key'];checks.append('preview-no-mutation-and-first-atomic-private-save')
        keep={**connection,'label':'Updated','key':'PRIVATE_CONNECTION_IGNORED'};apply(proposal(keep));assert json.loads(file.read_bytes())['local']['key']=='PRIVATE_CONNECTION_FIRST'
        destination={**keep,'base_url':'https://other.invalid/v1'};original=file.read_bytes();request(proposal(destination),status=400,expected_code='api-connection-key-destination-replacement-required');assert file.read_bytes()==original
        destination['key']='PRIVATE_CONNECTION_SECOND';apply(proposal(destination,True));assert json.loads(file.read_bytes())['local']['key']=='PRIVATE_CONNECTION_SECOND'
        destination['key']='';apply(proposal(destination,True));assert not current()['connections'][0]['has_key'];checks.append('keep-replace-remove-and-destination-rule')
        original=file.read_bytes();change=proposal(destination);v=preview(change);request({**change,'expected_digest':'0'*64,'preview':False,'preview_token':v['preview_token']},status=409,expected_code='revision_conflict');assert file.read_bytes()==original
        request({**change,'preview':False,'preview_token':'0'*64},status=400,expected_code='api-connection-preview-required');assert file.read_bytes()==original;checks.append('stale-source-and-token-refuse-without-write')
        stable=proposal(destination);v=preview(stable);count=len(list(file.parent.glob('api-connections-before-*.json')));done=apply(stable);assert not done['changed'] and len(list(file.parent.glob('api-connections-before-*.json')))==count;assert file.read_bytes()==original;checks.append('no-op-sync-without-extra-snapshot')

        # Validate the retained effective credential, not only submitted text.
        original=file.read_bytes();preview(proposal(destination))
        try:
            bad_key=json.loads(original);bad_key['local']['key']='PRIVATE_CONNECTION_STORED\r\nInjected: yes';retained.publish(file,bad_key)
            bad_bytes=file.read_bytes();request(proposal(destination),status=500,expected_code='api-connection-stored-key-invalid');assert file.read_bytes()==bad_bytes;unchanged()
            apply(proposal(destination,True));assert not current()['connections'][0]['has_key']
        finally:retained.publish(file,original)
        checks.append('retained-key-header-validation-and-explicit-removal-recovery')
        # Valid witness precedes every malformed/denied proposal.
        invalid=[{'version':2},{'unknown':True},{'connection':None}]
        for update in [{'id':'api-bad'},{'id':'not valid'},{'label':' '},{'model':'not valid'},{'base_url':'ftp://example.invalid/v1'},{'base_url':'https://user:PRIVATE_CONNECTION_INLINE@example.invalid/v1'},{'base_url':'https://example.invalid/v1?x=1'},{'base_url':'https://example.invalid/v1#fragment'},{'key':'PRIVATE_CONNECTION_BAD\r\nInjected: yes'}]:
            invalid.append({'connection':{**destination,**update}})
        original=file.read_bytes()
        for bad in invalid:
            control=proposal(destination);preview(control);request({**control,**bad},status=400);assert file.read_bytes()==original;unchanged()
        control=proposal(destination);preview(control)
        try:
            retained.publish(file,b'PRIVATE_CONNECTION_BROKEN stored source')
            request({**control,'expected_digest':retained.digest(file.read_bytes())},status=500);unchanged()
        finally:retained.publish(file,original)
        preview(proposal(destination));checks.append('invalid-request-and-stored-source-refusal-recovery')
        # Token must fence settings and CLI namespace, not only connection digest.
        original_cli=f.providers.read_bytes();change=proposal(destination);v=preview(change)
        changed_cli=json.loads(original_cli);changed_cli['providers']['extra']=copy.deepcopy(f.provider)
        try:
            retained.publish(f.providers,changed_cli);request({**change,'preview':False,'preview_token':v['preview_token']},status=400,expected_code='api-connection-preview-required');assert file.read_bytes()==original
        finally:retained.publish(f.providers,original_cli)
        collision=json.loads(original_cli);collision['providers']['api-local']=copy.deepcopy(f.provider)
        try:
            preview(proposal(destination));retained.publish(f.providers,collision)
            request(proposal({**destination,'disabled':True}),status=400,expected_code='api-connection-cli-name-conflict');assert file.read_bytes()==original
        finally:retained.publish(f.providers,original_cli)
        old_cap=f.ok(['engine-settings','show'])['values']['agent_provider_config_max_bytes']
        cap=max(len(original_cli),len(original))+1
        if str(cap)==old_cap:cap+=1
        change=proposal(destination);v=preview(change)
        try:
            f.settings_change(agent_provider_config_max_bytes=str(cap));request({**change,'preview':False,'preview_token':v['preview_token']},status=400,expected_code='api-connection-preview-required');assert file.read_bytes()==original
            preview(proposal(destination));request(proposal({**destination,'key':'K'*(cap+1)},True),status=400,expected_code='api-connection-file-outside-saved-policy');assert file.read_bytes()==original
        finally:f.settings_change(agent_provider_config_max_bytes=old_cap)
        apply(proposal(destination));checks.append('settings-cli-freshness-collision-and-file-cap')
        # No inherited key8192 or count32 business ceiling is introduced.
        large_key='K'*9000
        try:
            f.settings_change(agent_provider_config_max_bytes=str(len(original_cli)+len(original)+len(large_key)*2))
            apply(proposal({**destination,'key':large_key},True));assert json.loads(file.read_bytes())['local']['key']==large_key
            apply(proposal(destination,True))
        finally:f.settings_change(agent_provider_config_max_bytes=old_cap)
        saved=file.read_bytes()
        try:
            seeded={f'c{i:02}':{**destination,'id':f'c{i:02}','key':''} for i in range(32)};retained.publish(file,seeded);assert len(current()['connections'])==32
            apply(proposal({**destination,'id':'c32'},True));assert len(current()['connections'])==33
            retained.publish(output/'above-32-saved.json',file.read_bytes())
        finally:retained.publish(file,saved)
        unchanged();checks.append('saved-policy-not-hidden-key-or-connection-count-ceiling')
        # Public CLI uses the same producer, preview binding and actual publisher.
        assert f.ok(['providers','connections','show'])==current()
        cli_change=proposal({**destination,'label':'CLI applied'})
        cli_preview=f.ok(['providers','connections','preview'],cli_change);http_preview=preview(cli_change);assert cli_preview['preview_token']==http_preview['preview_token']
        cli_result=f.ok(['providers','connections','apply'],{**cli_change,'preview':False,'preview_token':cli_preview['preview_token']});assert cli_result['saved'] and json.loads(file.read_bytes())['local']['label']=='CLI applied'
        broken=proposal({**destination,'label':'Applied before output failed','key':'PRIVATE_CONNECTION_OUTPUT_KEY'},True);pv=preview(broken);broken.update(preview=False,preview_token=pv['preview_token']);input_path=output/'broken-output.input.json';retained.publish(input_path,broken)
        argv=[str(binary),'providers','connections','apply','--root',str(f.root),'--input',str(input_path),'--json','--lang','en'];retained.publish(output/'broken-output.argv.json',argv)
        child=subprocess.Popen(argv,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=f.environment);child.stdout.close();stderr=child.stderr.read();child.stderr.close();code=child.wait();retained.publish(output/'broken-output.stderr',stderr);retained.publish(output/'broken-output.exit.json',{'code':code})
        assert code==2 and json.loads(file.read_bytes())['local']['label']=='Applied before output failed'
        assert json.loads(file.read_bytes())['local']['key']=='PRIVATE_CONNECTION_OUTPUT_KEY'
        failure=json.loads(stderr);assert failure['configuration_may_be_recorded'] is True and 'PRIVATE_CONNECTION_' not in stderr.decode()
        assert current()['connections'][0]['label']=='Applied before output failed';unchanged();checks.append('cli-http-parity-and-structured-recorded-state-after-output-failure')
    except BaseException as error:primary=error
    try:s.cleanup()
    except BaseException as cleanup:
        if primary:raise BaseExceptionGroup('Connection writer and cleanup failed',[primary,cleanup])
        raise
    if primary:raise primary
    assert len(checks)==9 and not s.capture_errors and s.process.returncode==0
    retained.publish(output/'result.json',{'status':'PASS','checks':checks,'provider_started':False,'sha256':retained.digest(binary.read_bytes())});print('API_CONNECTIONS_WRITE_VERIFIED '+str(len(checks)),flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():retained.publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
