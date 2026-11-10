#!/usr/bin/env python3
"""Configure a pristine locked preparation with explicit CLI/human team choices.

Owned inert provider/catalogue fixtures only, no model execution or live Root.
Public CLI/API/browser paths retain original commands, refusals and recovery.
"""
import argparse,hashlib,json,sqlite3,subprocess
from pathlib import Path
import rust_agent_model_route as fixture
from rust_preparation_plan import plan,task
import rust_native_http_cli as native


def main():
    """Require atomic configuration and immutable replay without unlocking tasks."""
    parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);parser.add_argument('--expect-unported',action='store_true');a=parser.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'team');f.settings_change(preparation_write_enabled='1');checks=[]
    def check(name,value):
        """Retain a named observed assertion; no empty successful fallback."""
        assert value,name;assert name not in checks;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def database():
        """Compare the complete fixture SQL dump, never a running mission DB."""
        with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
    def cmd(p,action,event,**extra):
        """Use exact returned preparation revisions and explicit event identities."""
        return f.ok(['prepare',action,p['id']],dict(version=1,action=action,preparation_id=p['id'],event_id=event,expected_revision=p['revision'],**extra))
    def prepared(label,multi=False):
        """Prepare/adopt/validate/convert via public writers, with no team or start."""
        p=f.ok(['prepare','create'],dict(version=1,action='create',event_id=label+'-create',expected_revision=0,title='Team '+label,text='Need'));p=cmd(p,'save',label+'-brief',document='brief',text='Brief');p=cmd(p,'adopt-brief',label+'-adopt',sha256=p['documents']['brief']['sha256']);spec=plan()
        if multi:spec['tasks']=[task('first'),task('second',['first'])]
        p=cmd(p,'save',label+'-plan',document='plan',text=json.dumps(spec));p=cmd(p,'validate-plan',label+'-validate',sha256=p['documents']['plan']['sha256']);return cmd(p,'create-missions',label+'-convert',sha256=p['documents']['plan']['sha256'],expected_work_revision=0)
    def work(p):
        """Read authoritative Work through CLI, including policy/provenance fields."""
        return f.ok(['work','show',p['work_id']])['work']
    def configuration():
        """Explicit fixture choices; 8/7 are test data, never runtime defaults."""
        return dict(provider='fixture',worker_provider='fixture',level='standard',workspace='.',validation='human',max_tasks=8,max_calls=7)
    def intent(p,event):
        """Bind team selection to both revisions and the converted current plan."""
        return dict(version=1,action='configure-team',preparation_id=p['id'],event_id=event,expected_revision=p['revision'],expected_work_revision=work(p)['revision'],sha256=p['conversion']['plan_hash'],organization=configuration())
    def refused(r,label,code):
        """Require the exact failure and no SQL/Work/event/receipt side effects."""
        before=database();rc,wire=f.call(['prepare','configure-team',r['preparation_id']],r);check(label,rc!=0 and wire['failure']['code']==code and database()==before)
    p=prepared('primary',True);r=intent(p,'configure-primary')
    if a.expect_unported:
        before=database();rc,wire=f.call(['prepare','configure-team',p['id']],r);check('team-baseline-unported',rc!=0 and database()==before);return
    configured=f.ok(['prepare','configure-team',p['id']],r);w=work(configured);planning=w['planning'];reviewer=planning['reviewer'];values=f.ok(['engine-settings','show'])['values']
    canonical=dict(worker_provider='fixture',provider='fixture',level='standard',workspace='.',validation='human',max_tasks=8,max_calls=7);check('team-canonical-provenance',configured['conversion']['organization_sha256']==hashlib.sha256(json.dumps(canonical,separators=(',',':')).encode()).hexdigest())
    check('team-work-and-provenance',w['revision']==2 and configured['revision']==p['revision']+1 and len(configured['conversion']['organization_sha256'])==64 and configured['conversion']['spec']==p['conversion']['spec'])
    check('team-requirements-and-human-policy',w['criteria']==['proof','proof'] and [t['requirements'] for t in w['tasks']]==[['req-1'],['req-2']] and all(t['scope_id']=='root' and t['launch_held'] and t['validation_policy']['mode']=='human' and t['validation_policy']['authorized_at'] and t['validation_policy']['actor'] for t in w['tasks']))
    check('team-explicit-budgets',planning['max_tasks']==8 and planning['max_activations']==7 and planning['max_decisions']==7 and reviewer['max_calls']==7 and planning['activations']==0 and planning['decisions']==0 and reviewer.get('calls',0)==0)
    check('team-provider-and-profile',planning['provider']=='fixture' and planning['provider_digest']==reviewer['provider_digest'] and w['launch_profile']['provider']=='fixture' and w['launch_profile']['workspace']==str(f.root.resolve()) and w['launch_profile']['role']=='worker' and reviewer['model_route']['model']=='fixture-standard')
    check('team-current-reviewer-defaults',all(reviewer[key]==int(values[name]) for key,name in [('max_files','reviewer_default_delivery_files'),('max_context_files','reviewer_default_context_files'),('max_source_bytes','reviewer_default_source_bytes'),('max_context_bytes','reviewer_default_context_bytes')]))
    with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
        locks=db.execute('SELECT released FROM preparation_launch_locks WHERE work_id=?',(w['id'],)).fetchall();agents=db.execute('SELECT count(*) FROM agents WHERE work_id=?',(w['id'],)).fetchone()[0];kind=db.execute('SELECT kind FROM events WHERE work_id=? ORDER BY revision DESC LIMIT 1',(w['id'],)).fetchone()[0]
    check('team-locks-held-no-agents',locks==[(0,),(0,)] and agents==0 and kind=='preparation.configure-team' and not (f.root/'model-received.json').exists())
    before=database();source=f.providers.read_bytes();f.providers.unlink()
    try:check('team-replay-before-provider-io',f.ok(['prepare','configure-team',p['id']],r)==configured and database()==before)
    finally:fixture.publish(f.providers,source)
    refused(intent(configured,'configure-again'),'team-double-refused','preparation-organization-already-configured');refused({**r,'organization':{**r['organization'],'max_calls':9}},'team-event-conflict','preparation-event-conflict')
    original_default=values['reviewer_default_delivery_files'];changed_default=int(original_default)+1;f.settings_change(reviewer_default_delivery_files=str(changed_default) if isinstance(original_default,str) else changed_default)
    negative=prepared('negative');wanted=intent(negative,'configure-negative')
    for label,patch,code in [('work-cas',{'expected_work_revision':2},'preparation-work-revision-conflict'),('hash',{'sha256':'other'},'preparation-stale-plan'),('missing-org',{'organization':None},'preparation-request-invalid'),('missing-work-cas',{'expected_work_revision':None},'preparation-request-invalid')]:refused({**wanted,**patch},'team-refuse-'+label,code)
    for label,patch,code in [('zero-calls',{'max_calls':0},'preparation-request-invalid'),('too-few-tasks',{'max_tasks':0},'preparation-request-invalid'),('automatic',{'validation':'automatic'},'preparation-organization-mode-unported'),('unknown-field',{'repository':{}},'preparation-request-invalid'),('unknown-worker',{'worker_provider':'missing'},'agent-provider-not-configured'),('stale-policy',{'model_policy_hash':'0'*64},'agent-model-policy-hash-changed'),('outside-workspace',{'workspace':'/tmp'},'agent-workspace-outside-project-or-not-directory')]:refused({**wanted,'organization':{**wanted['organization'],**patch}},'team-refuse-'+label,code)
    recovered=f.ok(['prepare','configure-team',negative['id']],wanted);check('team-refusal-recovery',work(recovered)['planning']['reviewer']['max_calls']==7);check('team-changed-reviewer-default-consumed',work(recovered)['planning']['reviewer']['max_files']==changed_default);f.settings_change(reviewer_default_delivery_files=original_default);check('team-default-snapshots-retained',work(recovered)['planning']['reviewer']['max_files']==changed_default and work(configured)['planning']['reviewer']['max_files']==int(original_default))
    edited=prepared('edited');ew=work(edited);f.ok(['task','update',ew['id']],dict(schema_version=1,event_id='edited-title',expected_revision=ew['revision'],id=ew['tasks'][0]['id'],title='Changed contract'));refused(intent(edited,'configure-edited'),'team-refuse-task-drift','preparation-organization-contract-changed')
    released=cmd(configured,'release-plan','release-after-team',sha256=configured['conversion']['plan_hash'],expected_work_revision=w['revision']);check('team-separate-release',bool(released['conversion']['released_at']) and all(not t.get('launch_held',False) for t in work(released)['tasks']))
    http=prepared('http');cases=[dict(lang=lang,theme=theme,preparation=prepared('ui-'+lang+'-'+theme)) for lang in ['fr','en'] for theme in ['etat','sombre']];server=native.Server(f.binary,f.root,out,'preparation-team',environment=f.environment);server.capture_async(out/'observer.raw');primary=None
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token'];check('team-http-capability','configure-team' in server.request('/api/v1/native-capabilities',observe=False)['preparations']);request=intent(http,'http-team');before=database();server.request('/api/v1/preparations/command',request,csrf='wrong',status=403,observe=False);check('team-csrf-no-write',before==database())
        for label,patch,status,code in [('unknown-worker',{'worker_provider':'missing'},400,'agent-provider-not-configured'),('outside-workspace',{'workspace':'/tmp'},400,'agent-workspace-outside-project-or-not-directory'),('stale-model',{'model_policy_hash':'0'*64},409,'agent-model-policy-hash-changed')]:
            before=database();reply=server.request('/api/v1/preparations/command',{**request,'organization':{**request['organization'],**patch}},csrf=csrf,status=status,observe=False);check('team-http-'+label,reply['failure']['code']==code and before==database())
        http=server.request('/api/v1/preparations/command',request,csrf=csrf,observe=False);check('team-http-cli-receipt',f.ok(['prepare','show',http['id']])==http and bool(http['conversion']['organization_sha256']))
        cfg=dict(base='http://'+server.address,cookie=server.cookie,output=str(out),cases=cases)
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:result=subprocess.run(['node','tests/rust_preparation_team_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        check('team-browser',result.returncode==0);browser=json.loads((out/'browser-result.json').read_text());assert browser['status']=='PASS' and browser['checks'];print((out/'browser.stdout').read_text(),end='')
        for case in cases:
            current=f.ok(['prepare','show',case['preparation']['id']]);check('team-ui-recorded-'+case['lang']+'-'+case['theme'],bool(current['conversion']['organization_sha256']) and not current['conversion'].get('released_at'))
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    check('team-never-executed-provider',not (f.root/'model-received.json').exists())
    fixture.publish(out/'result.json',dict(status='PASS',checks=checks,browser_checks=len(browser['checks']),provider_calls=0));print('PREPARATION_TEAM_PASS '+str(len(checks)),flush=True)


if __name__=='__main__':main()
