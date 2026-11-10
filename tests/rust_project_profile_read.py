#!/usr/bin/env python3
"""Read effective project metadata without importing native settings contents.

Owned inactive Root only; profile/source mutations are synthetic fixtures, never
live state. Refusal/recovery keeps SQL unchanged and never launches a provider.
"""
import argparse,hashlib,http.client,json,os,sqlite3
from pathlib import Path
import rust_agent_model_route as fixture
from rust_project_memory_source import seed_lesson, seed_work
import rust_native_http_cli as native

def main():
    """Require role/source integrity, metadata-only detection and explicit errors."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-unported',action='store_true');a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'project-profile-read');checks=[]
    def check(name,value):
        """Retain named observations immediately; no failed/empty fallback counts."""
        assert value,name;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def sql():
        with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
    before=sql();code,value=f.call(['project-profile','show'])
    if a.expect_unported:
        check('profile-baseline-cli-refuses',code!=0 and before==sql());s=native.Server(f.binary,f.root,out,'project-profile-baseline',environment=f.environment);primary=None
        try:s.request('/api/v1/project-profile',status=404,observe=False);check('profile-baseline-http-refuses',before==sql())
        except BaseException as e:primary=e
        finally:native.finish(s,primary)
        print('PROJECT_PROFILE_READ_BASELINE 2');return
    check('profile-absent-explicit',code==0 and value==dict(enabled=False,roles=[],claude_settings_detected=[]));private=f.root/'.claude';private.mkdir();settings=private/'settings.json';settings.write_bytes(b'\xff\x00NEVER_IMPORT_NATIVE_SETTINGS_CONTENT');settings.chmod(0)
    def read():return f.ok(['project-profile','show'])
    v=read();check('profile-settings-metadata-not-content',v['claude_settings_detected']==['.claude/settings.json'] and 'NEVER_IMPORT' not in json.dumps(v));roles=['preparation','planner','subplanner','worker','reviewer'];source='Keep the assigned task boundary. Unique private instruction fixture.';doc=dict(version=1,name='Observed profile',source='custom',instructions=[dict(path='AGENTS.md',roles=roles)]);fixture.publish(f.root/'AGENTS.md',source.encode());fixture.publish(f.root/'swarm.project.json',doc);raw=(f.root/'swarm.project.json').read_bytes();v=read();check('profile-five-ordered-roles',v['enabled'] and v['name']==doc['name'] and [r['role'] for r in v['roles']]==roles);check('profile-source-hashes-and-counts',all(r['profile_sha256']==hashlib.sha256(raw).hexdigest() and r['sources']==[dict(path='AGENTS.md',sha256=hashlib.sha256(source.encode()).hexdigest(),bytes=len(source.encode()))] for r in v['roles']));check('profile-no-prompt-or-source-text',source not in json.dumps(v) and all('prompt' not in r for r in v['roles']));check('profile-check-show-identical',f.ok(['project-profile','check'])==v)
    for role in roles:check('profile-existing-reader-'+role,f.ok(['project-context',role])['context']==next(r for r in v['roles'] if r['role']==role))
    # Recognizable credentials in metadata reach the same model prompt as source
    # text. Exercise the real public producer, using only an attributed fake key.
    fake='sk-012345678901234567890123456789';fake_path=fake+'.md';fixture.publish(f.root/fake_path,source.encode());metadata_results=[]
    for label,candidate in [('name',{**doc,'name':'Fixture '+fake}),('path',{**doc,'instructions':[dict(path=fake_path,roles=roles)]})]:
        check('profile-metadata-'+label+'-positive-control',read()==v)
        fixture.publish(f.root/'swarm.project.json',candidate);b=sql();rc,result=f.call(['project-context','preparation'])
        fixture.publish(out/('metadata-'+label+'-observation.json'),dict(exit=rc,fixture_token_in_prompt=rc==0 and fake in result.get('prompt',''),fixture_token_in_error=rc!=0 and fake in json.dumps(result),sql_unchanged=b==sql()))
        metadata_results.append((label,rc!=0 and fake not in json.dumps(result) and b==sql()));fixture.publish(f.root/'swarm.project.json',raw)
        check('profile-metadata-'+label+'-recovery',read()==v)
    opaque='OPAQUE_FIXTURE_VALUE_0123456789'
    for label,target,link in [('native-alias',private/'glm-settings.json','symlink'),('private-alias',f.root/'.swarm/stored-fixture.md','symlink'),('private-hardlink',f.root/'.swarm/stored-fixture.md','hardlink')]:
        check('profile-metadata-'+label+'-positive-control',read()==v)
        assert not target.exists();fixture.publish(target,dict(key=opaque,permissions=dict(allow=['FIXTURE_NATIVE_PERMISSION'])))
        alias=f.root/'ordinary-alias.md';assert not alias.exists()
        if link=='hardlink':os.link(target,alias)
        else:alias.symlink_to(target)
        candidate={**doc,'instructions':[dict(path='ordinary-alias.md',roles=roles)]};fixture.publish(f.root/'swarm.project.json',candidate);b=sql();rc,result=f.call(['project-context','preparation'])
        fixture.publish(out/('metadata-'+label+'-observation.json'),dict(exit=rc,fixture_token_in_prompt=rc==0 and opaque in result.get('prompt',''),fixture_token_in_error=rc!=0 and opaque in json.dumps(result),sql_unchanged=b==sql()))
        metadata_results.append((label,rc!=0 and opaque not in json.dumps(result) and b==sql()));fixture.publish(f.root/'swarm.project.json',raw);alias.unlink();target.unlink()
        check('profile-metadata-'+label+'-recovery',read()==v)
    for label,refused in metadata_results:check('profile-metadata-'+label+'-secret-refuses',refused)
    for name,invalid in [('malformed-profile',b'{'),('unknown-field',json.dumps({**doc,'unknown':1}).encode()),('missing-role',json.dumps({**doc,'instructions':[dict(path='AGENTS.md',roles=['worker'])]}).encode())]:
        read();fixture.publish(f.root/'swarm.project.json',invalid);b=sql();code,error=f.call(['project-profile','show']);check('profile-refuses-'+name,code!=0 and b==sql() and error['failure']['code']=='project-context-invalid');fixture.publish(f.root/'swarm.project.json',raw);check('profile-recovers-'+name,read()==v)
    secret='password: abcdefghijklmnopqrstuvwxyz';fixture.publish(f.root/'AGENTS.md',secret.encode());b=sql();code,error=f.call(['project-profile','show']);check('profile-secret-refuses-without-content',code!=0 and secret not in json.dumps(error) and b==sql());fixture.publish(f.root/'AGENTS.md',source.encode());check('profile-secret-recovery',read()==v)
    old=f.ok(['engine-settings','show'])['values']['project_context_max_bytes'];f.settings_change(project_context_max_bytes='0');code,_=f.call(['project-profile','show']);check('profile-saved-cap-consumed',code!=0);f.settings_change(project_context_max_bytes=old);check('profile-cap-recovery',read()==v)
    if os.geteuid()!=0:
        read();private.chmod(0)
        try:code,error=f.call(['project-profile','show'])
        finally:private.chmod(0o700)
        check('profile-metadata-permission-error-not-absence',code!=0 and error['failure']['code']=='path');check('profile-metadata-permission-recovery',read()==v)
    else:raise AssertionError('Permission fixture requires an unprivileged user; not executed')
    local=private/'settings.local.json';local.symlink_to(settings);check('profile-contained-link-metadata',read()['claude_settings_detected']==['.claude/settings.json','.claude/settings.local.json']);local.unlink();outside=out/'outside-settings';outside.write_text('NEVER_IMPORT_OUTSIDE');local.symlink_to(outside);check('profile-outside-settings-not-admitted',read()['claude_settings_detected']==['.claude/settings.json']);local.unlink();os.mkfifo(local);check('profile-fifo-not-opened',read()['claude_settings_detected']==['.claude/settings.json']);local.unlink()
    s=native.Server(f.binary,f.root,out,'project-profile-read',environment=f.environment);s.capture_async(out/'observer.raw');primary=None
    try:
        check('profile-http-cli-equal',s.request('/api/v1/project-profile',observe=False)==read())
        connection=http.client.HTTPConnection(s.host,s.port,timeout=None)
        try:
            connection.request('HEAD','/api/v1/project-profile',headers={'Cookie':s.cookie});response=connection.getresponse();payload=response.read();check('profile-HEAD-no-body',response.status==200 and payload==b'' and response.getheader('Content-Type').startswith('application/json'))
        finally:connection.close()
        s.request('/api/v1/project-profile',authorized=False,status=403,observe=False);check('profile-unauthorized-refuses',before==sql())
        csrf=s.request('/api/v1/native-session',observe=False)['csrf_token'];s.request('/api/v1/project-profile',body={},csrf=csrf,status=405,observe=False);check('profile-write-method-refuses',before==sql());guide=private/'field-guide';guide.mkdir();memory_work_before=seed_lesson(f.root,f.work);(guide/'index.md').write_text('[Managed fixture](g.md)');fixture.publish(f.root/'swarm.project.json',{**doc,'instructions':[dict(path='.claude/field-guide/index.md',roles=roles)]});blocked=s.request('/api/v1/project-profile',status=500,observe=False);check('profile-memory-guard-preserved',blocked['failure'].get('reason')=='project-source-memory-managed' and 'causes' not in blocked['failure']);
        seed_work(f.root,f.work,memory_work_before)
        fixture.publish(f.root/'swarm.project.json',raw);check('profile-memory-guard-recovery',s.request('/api/v1/project-profile',observe=False)==v);fixture.publish(f.root/'swarm.project.json',{**doc,'PRIVATE_UNTRUSTED_FIELD_SENTINEL':'PRIVATE_REQUEST_CONTENT'});invalid=s.request('/api/v1/project-profile',status=500,observe=False);check('profile-safe-public-reason-no-raw-cause',invalid['failure'].get('reason')=='profile-json' and 'causes' not in invalid['failure'] and 'PRIVATE_' not in json.dumps(invalid));fixture.publish(f.root/'swarm.project.json',raw);check('profile-public-reason-recovery',s.request('/api/v1/project-profile',observe=False)==v);s.request('/api/v1/project-profile?unknown=1',status=400,observe=False);check('profile-invalid-query-no-write',sql()==before)
    except BaseException as e:primary=e
    finally:native.finish(s,primary)
    settings.chmod(0o600) # Restore this owned synthetic file for retained replay evidence.
    check('profile-no-sql-mutation',before==sql());check('profile-no-provider-execution',not (f.root/'model-received.json').exists());print('PROJECT_PROFILE_READ_PASS '+str(len(checks)))
if __name__=='__main__':main()
