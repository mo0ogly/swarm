#!/usr/bin/env python3
"""Explicit snapshot release through CLI/API/UI on an owned isolated Root.

Keep original channels, named assertions and fault/recovery evidence. No provider,
live Root, global watchdog, hidden policy cap or external acceptance is exercised.
"""
import argparse,json,sqlite3,subprocess
from pathlib import Path
from rust_preparation_reads import Driver
from rust_preparation_plan import plan,task as plan_task
from rust_agent_model_route import publish
import rust_native_http_cli as native


def main():
    """Create real preparations publicly, then observe release, refusals and replay."""
    parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);parser.add_argument('--expect-unported',action='store_true');a=parser.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);d=Driver(Path(a.binary).resolve(),out);checks=[]
    def check(name,value):
        """Persist each meaningful assertion, never infer PASS from an exit alone."""
        assert value,name;assert name not in checks;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def command(p,action,event,**extra):
        """Bind an explicit intent to the preparation revision returned by the writer."""
        r=dict(version=1,action=action,preparation_id=p['id'],event_id=event,expected_revision=p['revision'],**extra)
        return d.ok(['prepare',action,p['id']],r)
    def prepared(label,spec=None):
        """Use public creation, brief adoption, verdict and locked conversion."""
        p=d.ok(['prepare','create'],dict(version=1,action='create',event_id=label+'-create',expected_revision=0,title='Release '+label,text='Need'))
        p=command(p,'save',label+'-brief',document='brief',text='Brief');p=command(p,'adopt-brief',label+'-adopt',sha256=p['documents']['brief']['sha256'])
        p=command(p,'save',label+'-plan',document='plan',text=json.dumps(plan() if spec is None else spec));p=command(p,'validate-plan',label+'-verify',sha256=p['documents']['plan']['sha256'])
        return command(p,'create-missions',label+'-convert',sha256=p['documents']['plan']['sha256'],expected_work_revision=0)
    def work(p):
        """Read persisted Work through the actual CLI, including current task edits."""
        return d.ok(['work','show',p['work_id']])['work']
    def intent(p,event):
        """Select the conversion snapshot, never the current draft document hash."""
        return dict(version=1,action='release-plan',preparation_id=p['id'],event_id=event,expected_revision=p['revision'],expected_work_revision=work(p)['revision'],sha256=p['conversion']['plan_hash'])
    def refused(r,label,code,status=2):
        """Assert failure code and exact SQL dump preservation, then retain originals."""
        before=d.database();rc,_,err=d.call(['prepare','release-plan',r['preparation_id']],r);check(label,rc==status and json.loads(err)['failure']['code']==code and d.database()==before)
    def inspect(p,label,previous):
        """Observe lock/event/Work changes without assuming authorization is execution."""
        w=work(p);check(label+'-work-cas',w['revision']==previous['revision']+1 and all(not t.get('launch_held',False) for t in w['tasks']))
        changed=json.loads(json.dumps(previous));changed.update(revision=w['revision'],updated=w['updated']);
        for task in changed['tasks']:task.pop('launch_held',None)
        if changed.get('planning'):changed['planning']['paused']=False
        check(label+'-only-authorized-fields-changed',w==changed and bool(p['conversion']['released_at']))
        with sqlite3.connect('file:'+str(d.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
            locks=db.execute('SELECT task_id,preparation_id,released FROM preparation_launch_locks WHERE work_id=? ORDER BY task_id',(w['id'],)).fetchall();events=db.execute('SELECT kind,revision FROM events WHERE work_id=? ORDER BY revision',(w['id'],)).fetchall();agents=db.execute('SELECT count(*) FROM agents WHERE work_id=?',(w['id'],)).fetchone()[0]
        check(label+'-locks-event-no-dispatch',locks==[(t,p['id'],1) for t in sorted(p['conversion']['task_ids'])] and events[-1]==('preparation.release-plan',w['revision']) and agents==0)
    d.ok(['init']);d.settings(preparation_write_enabled='1');p=prepared('primary');r=intent(p,'release-primary')
    if a.expect_unported:
        before=d.database();rc,_,err=d.call(['prepare','release-plan',p['id']],r);check('release-baseline-unported',rc!=0 and d.database()==before);publish(out/'result.json',dict(status='EXPECTED_UNPORTED',checks=checks));return
    for lang in ['fr','en']:
        result=subprocess.run([str(d.binary),'--help','--lang',lang],capture_output=True);check('release-help-'+lang,result.returncode==0 and b'|release-plan' in result.stdout)
    for label,extra,code,status in [('preparation-cas',{'expected_revision':p['revision']-1},'preparation-revision-conflict',3),('work-cas',{'expected_work_revision':r['expected_work_revision']+1},'preparation-work-revision-conflict',3),('wrong-hash',{'sha256':'other'},'preparation-stale-plan',2),('no-work-cas',{'expected_work_revision':None},'preparation-request-invalid',2),('zero-work-cas',{'expected_work_revision':0},'preparation-request-invalid',2),('foreign-fields',{'text':'ignored?'},'preparation-request-invalid',2),('organization',{'organization':{}},'preparation-request-invalid',2)]:refused({**r,**extra},'release-refuse-'+label,code,status)
    without=d.ok(['prepare','create'],dict(version=1,action='create',event_id='release-no-conversion',expected_revision=0,title='No conversion',text='Need'));refused({**r,'preparation_id':without['id'],'expected_revision':without['revision']},'release-no-conversion-refused','preparation-stale-plan')
    previous=work(p);verdict=p['verdict'];released=d.ok(['prepare','release-plan',p['id']],r);inspect(released,'release',previous);check('release-verdict-preserved',released['verdict']==verdict and released['plan_ready'])
    before=d.database();check('release-replay-no-write',d.ok(['prepare','release-plan',p['id']],r)==released and d.database()==before);refused({**r,'sha256':'different'},'release-event-conflict','preparation-event-conflict',3);refused(intent(released,'release-again'),'release-double-refused','preparation-already-released')
    p=prepared('draft');original=p['conversion'];draft=plan();draft['objective']='New draft objective';p=command(p,'save','new-draft',document='plan',text=json.dumps(draft));previous=work(p);r=intent(p,'release-old-snapshot');p=d.ok(['prepare','release-plan',p['id']],r);inspect(p,'release-old-snapshot',previous);check('release-keeps-new-draft',p['conversion']['spec']==original['spec'] and p['documents']['plan']['text']==json.dumps(draft) and not p['plan_ready'])
    p=prepared('edited');previous=work(p);task=previous['tasks'][0];updated=d.ok(['task','update',p['work_id']],dict(schema_version=1,event_id='edit-before-release',expected_revision=previous['revision'],id=task['id'],title='Reviewed updated title',next='Reviewed updated instruction'))['work'];r=intent(p,'release-edited');p=d.ok(['prepare','release-plan',p['id']],r);inspect(p,'release-edited',updated)
    p=prepared('locks');r=intent(p,'release-locks');previous=work(p)
    for label,sql in [('foreign',"UPDATE preparation_launch_locks SET preparation_id=? WHERE work_id=?"),('premature',"UPDATE preparation_launch_locks SET released=1 WHERE preparation_id=? AND work_id=?")]:
        # The successful release above is the valid witness; fault only this fixture.
        other=released['id'] if label=='foreign' else p['id'];d.write_fixture(sql,(other,p['work_id']));refused(r,'release-lock-'+label,'preparation-lock-write-mismatch');d.write_fixture('UPDATE preparation_launch_locks SET preparation_id=?,released=0 WHERE work_id=?',(p['id'],p['work_id']))
    missing=p['conversion']['task_ids'][0];d.write_fixture('DELETE FROM preparation_launch_locks WHERE work_id=? AND task_id=?',(p['work_id'],missing));refused(r,'release-lock-missing','preparation-lock-write-mismatch');d.write_fixture('INSERT INTO preparation_launch_locks(work_id,task_id,preparation_id,released) VALUES(?,?,?,0)',(p['work_id'],missing,p['id']))
    d.write_fixture("CREATE TRIGGER fixture_release_late AFTER INSERT ON preparation_commands WHEN json_extract(CAST(NEW.request AS TEXT),'$.action')='release-plan' BEGIN UPDATE preparation_launch_locks SET released=0 WHERE preparation_id=NEW.preparation_id; END",());refused(r,'release-injected-schema-refused','root-schema-mismatch');d.write_fixture('DROP TRIGGER fixture_release_late',());p=d.ok(['prepare','release-plan',p['id']],r);inspect(p,'release-lock-recovery',previous)
    multi=plan();multi['tasks']=[plan_task('first'),plan_task('second',['first'])]
    witness=prepared('multi-valid',multi);previous=work(witness);witness=d.ok(['prepare','release-plan',witness['id']],intent(witness,'release-multi-valid'));inspect(witness,'release-multi-valid',previous)
    broken=prepared('multi-rollback',multi);previous=work(broken);wanted=intent(broken,'release-multi-rollback');second=broken['conversion']['task_ids'][1];d.write_fixture('UPDATE preparation_launch_locks SET preparation_id=? WHERE work_id=? AND task_id=?',(released['id'],broken['work_id'],second));refused(wanted,'release-partial-lock-rollback','preparation-lock-write-mismatch');d.write_fixture('UPDATE preparation_launch_locks SET preparation_id=? WHERE work_id=? AND task_id=?',(broken['id'],broken['work_id'],second));broken=d.ok(['prepare','release-plan',broken['id']],wanted);inspect(broken,'release-multi-recovery',previous)
    scope=prepared('scope');sw=work(scope);d.ok(['task','add',sw['id']],dict(schema_version=1,event_id='extra-task',expected_revision=sw['revision'],id='outside-conversion',title='Extra task',deliverable='Separate result',criteria=['Observable result'],next='Review separately',owner='worker'));before_release=work(scope);scope=d.ok(['prepare','release-plan',scope['id']],intent(scope,'release-extra-task'));after_release=work(scope);check('release-added-task-preserved',after_release['tasks'][-1]==before_release['tasks'][-1] and all(not t.get('launch_held',False) for t in after_release['tasks'] if t['id'] in scope['conversion']['task_ids']) and bool(scope['conversion']['released_at']))
    http=prepared('http');cases=[dict(lang=lang,theme=theme,preparation=prepared('ui-'+lang+'-'+theme)) for lang in ['fr','en'] for theme in ['etat','sombre']]
    server=native.Server(d.binary,d.root,out,'preparation-release');server.capture_async(out/'observer.raw');primary=None
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token'];caps=server.request('/api/v1/native-capabilities',observe=False);check('release-http-capability','release-plan' in caps['preparations']);before=d.database();review=server.request('/api/v1/preparations/release-review?id='+http['id'],observe=False);check('release-review-read-only',review['preparation']==http and review['work']==work(http) and d.database()==before)
        wanted=intent(http,'http-release');before=d.database();server.request('/api/v1/preparations/command',{**wanted,'expected_work_revision':wanted['expected_work_revision']+1},csrf=csrf,status=409,observe=False);check('release-http-work-cas-no-write',before==d.database());server.request('/api/v1/preparations/command',wanted,csrf='wrong',status=403,observe=False);check('release-csrf-no-write',before==d.database());server.request('/api/v1/preparations/release-review?id='+http['id']+'&document=plan',status=400,observe=False)
        previous=work(http);http=server.request('/api/v1/preparations/command',wanted,csrf=csrf,observe=False);inspect(http,'release-http',previous);check('release-http-cli-receipt',d.ok(['prepare','show',http['id']])==http)
        cfg=dict(base='http://'+server.address,cookie=server.cookie,output=str(out),cases=cases)
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:result=subprocess.run(['node','tests/rust_preparation_release_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        check('release-browser',result.returncode==0);browser=json.loads((out/'browser-result.json').read_text());assert browser['status']=='PASS' and browser['checks'];print((out/'browser.stdout').read_text(),end='')
        for case in cases:
            current=d.ok(['prepare','show',case['preparation']['id']]);check('release-ui-recorded-'+case['lang']+'-'+case['theme'],bool(current['conversion']['released_at']) and all(not t.get('launch_held',False) for t in work(current)['tasks']))
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    publish(out/'result.json',dict(status='PASS',checks=checks,browser_checks=len(browser['checks']),provider_calls=0));print('PREPARATION_RELEASE_PASS '+str(len(checks)),flush=True)


if __name__=='__main__':main()
