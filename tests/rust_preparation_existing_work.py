#!/usr/bin/env python3
"""Append a prepared plan to an owned existing Work through CLI/API/UI.

Preserve unrelated tasks/plans/SQL history; CAS failures do not mutate. Explicit
release affects only the new tasks. Fixture only, no provider/global watchdog.
"""
import argparse,hashlib,json,sqlite3,subprocess
from pathlib import Path
from rust_preparation_reads import Driver
from rust_preparation_plan import plan
from rust_agent_model_route import publish
import rust_native_http_cli as native

def main():
    """Retain named state assertions, original channels and owned server cleanup."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-unported',action='store_true');ap.add_argument('--backend-only',action='store_true');a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);d=Driver(Path(a.binary).resolve(),out);checks=[]
    def check(name,value):
        """Persist every meaningful observation; failures keep prior evidence."""
        assert value,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def command(p,action,event,**extra):
        """Bind an explicit intention to the observed preparation revision."""
        return d.ok(['prepare',action,p['id']],dict(version=1,action=action,preparation_id=p['id'],event_id=event,expected_revision=p['revision'],**extra))
    def fixture(label,target=None):
        """Use only public operations for target, old task and validated proposal."""
        if target is None:
            w=d.ok(['work','create'],dict(schema_version=1,event_id=label+'-work',expected_revision=0,title='Existing '+label,objective='Preserve objective',scope='Preserve scope',criteria=['Preserve criterion']))['work']
            d.ok(['task','add',w['id']],dict(schema_version=1,event_id=label+'-old',expected_revision=w['revision'],id='old-task',title='Old task',deliverable='Old artifact',criteria=['Old evidence'],next='Keep this instruction'))
            w=d.ok(['work','show',w['id']])['work']
        else:w=target
        p=d.ok(['prepare','create'],dict(version=1,action='create',event_id=label+'-create',expected_revision=0,title='Append '+label,text='Need',work_id=w['id']))
        p=command(p,'save',label+'-brief',document='brief',text='Brief');p=command(p,'adopt-brief',label+'-adopt',sha256=p['documents']['brief']['sha256']);p=command(p,'save',label+'-plan',document='plan',text=json.dumps(plan()));p=command(p,'validate-plan',label+'-validate',sha256=p['documents']['plan']['sha256'])
        return p,w
    def intent(p,w,event):
        """Bind both observed revisions and the current validated plan digest."""
        return dict(version=1,action='create-missions',preparation_id=p['id'],event_id=event,expected_revision=p['revision'],expected_work_revision=w['revision'],sha256=p['documents']['plan']['sha256'])
    def inspect(p,old,label):
        """Compare unrelated fields, appended provenance and exact held subset."""
        w=d.ok(['work','show',old['id']])['work'];ids=p['conversion']['task_ids'];check(label+'-same-work',w['id']==old['id'] and w['revision']==old['revision']+1)
        ignored={'revision','updated','tasks','plans'};check(label+'-old-fields-preserved',{k:v for k,v in w.items() if k not in ignored}=={k:v for k,v in old.items() if k not in ignored} and w['tasks'][:len(old['tasks'])]==old['tasks'] and w.get('plans',[])[:-1]==old.get('plans',[]))
        check(label+'-new-locked-provenance',[t['id'] for t in w['tasks'][len(old['tasks']):]]==ids and all(t['launch_held'] for t in w['tasks'][len(old['tasks']):]) and w['plans'][-1]['task_ids']==ids)
        with sqlite3.connect('file:'+str(d.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
            locks=db.execute('SELECT task_id,preparation_id,released FROM preparation_launch_locks WHERE work_id=? ORDER BY task_id',(w['id'],)).fetchall();agents=db.execute('SELECT count(*) FROM agents').fetchone()[0]
        prior=[(i,approved['source'],int(not next(t for t in old['tasks'] if t['id']==i).get('launch_held',False))) for approved in old.get('plans',[]) for i in approved.get('task_ids',[])];check(label+'-locks-no-agent',locks==sorted(prior+[(i,p['id'],0) for i in ids]) and agents==0);return w
    d.ok(['init']);d.settings(preparation_write_enabled='1');p,w=fixture('cli');r=intent(p,w,'append-cli');before=d.database()
    if a.expect_unported:
        rc,_,err=d.call(['prepare','create-missions',p['id']],r);check('existing-work-baseline-refuses',rc!=0 and json.loads(err)['failure']['code'] in ['preparation-request-invalid','preparation-existing-work-unported'] and before==d.database());print('EXISTING_WORK_BASELINE_PASS '+str(len(checks)));return
    review=d.ok(['prepare','release-review',p['id']]);check('existing-review-readonly',review['work']==w and review['preparation']==d.ok(['prepare','show',p['id']]) and before==d.database())
    for label,extra in [('stale-target',{'expected_work_revision':0}),('stale-preparation',{'expected_revision':p['revision']-1}),('stale-hash',{'sha256':'wrong'})]:
        before=d.database();rc,_,err=d.call(['prepare','create-missions',p['id']],{**r,**extra});check(label+'-no-write',rc!=0 and before==d.database() and json.loads(err)['failure']['code'] in ['preparation-work-revision-conflict','preparation-revision-conflict','preparation-stale-document'])
    p=d.ok(['prepare','create-missions',p['id']],r);current=inspect(p,w,'cli');before=d.database();check('existing-exact-replay',d.ok(['prepare','create-missions',p['id']],r)==p and before==d.database())
    second,prior=fixture('second',current);second=d.ok(['prepare','create-missions',second['id']],intent(second,prior,'second-append'));current=inspect(second,prior,'second-plan');old_before_release=current['tasks'];
    p=command(p,'release-plan','subset-release',sha256=p['conversion']['plan_hash'],expected_work_revision=current['revision']);released=d.ok(['work','show',w['id']])['work'];check('existing-release-subset-only',[t for t in released['tasks'] if t['id'] not in p['conversion']['task_ids']]==[t for t in old_before_release if t['id'] not in p['conversion']['task_ids']] and all(not t.get('launch_held',False) for t in released['tasks'] if t['id'] in p['conversion']['task_ids']) and bool(p['conversion']['released_at']))
    collision,target=fixture('collision');task_id='plan-'+hashlib.sha256(collision['id'].encode()).hexdigest()[:10]+'-'+json.loads(collision['documents']['plan']['text'])['tasks'][0]['id']
    d.ok(['task','add',target['id']],dict(schema_version=1,event_id='collision-existing-task',expected_revision=target['revision'],id=task_id,title='Keep collision',deliverable='Existing artifact',criteria=['Preserved']))
    target=d.ok(['work','show',target['id']])['work'];before=d.database();rc,_,err=d.call(['prepare','create-missions',collision['id']],intent(collision,target,'collision-append'));check('existing-task-collision-no-write',rc!=0 and json.loads(err)['failure']['code']=='preparation-task-invalid' and before==d.database())
    if a.backend_only:publish(out/'result.json',dict(status='PASS',checks=checks));print('EXISTING_WORK_BACKEND_PASS '+str(len(checks)));return
    hp,hw=fixture('http');cases=[]
    for lang in ['fr','en']:
        for theme in ['etat','sombre']:
            cp,cw=fixture(lang+'-'+theme);cases.append(dict(lang=lang,theme=theme,preparation=cp,work_revision=cw['revision'],old_work=cw))
    server=native.Server(d.binary,d.root,out,'existing-work');server.capture_async(out/'observer.raw');primary=None
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token'];before=d.database();view=server.request('/api/v1/preparations/release-review?id='+hp['id'],observe=False);check('existing-http-review-readonly',view==d.ok(['prepare','release-review',hp['id']]) and before==d.database())
        wanted=intent(hp,hw,'http-append');server.request('/api/v1/preparations/command',wanted,csrf='wrong',status=403,observe=False);check('existing-csrf-no-write',before==d.database());hp=server.request('/api/v1/preparations/command',wanted,csrf=csrf,observe=False);inspect(hp,hw,'http')
        cfg=dict(base='http://'+server.address,cookie=server.cookie,output=str(out),cases=cases)
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:result=subprocess.run(['node','tests/rust_preparation_conversion_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        check('existing-browser',result.returncode==0);print((out/'browser.stdout').read_text(),end='');browser=json.loads((out/'browser-result.json').read_text());check('existing-browser-nonempty',browser['status']=='PASS' and bool(browser['checks']))
        for case in cases:
            case['preparation']=d.ok(['prepare','show',case['preparation']['id']]);work=inspect(case['preparation'],case['old_work'],case['lang']+'-'+case['theme']);case['work_revision']=work['revision']
        release_out=out/'release-browser';release_out.mkdir();cfg['output']=str(release_out)
        with (out/'release-browser.stdout').open('wb') as stdout,(out/'release-browser.stderr').open('wb') as stderr:result=subprocess.run(['node','tests/rust_preparation_release_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        check('existing-subset-release-browser',result.returncode==0);print((out/'release-browser.stdout').read_text(),end='');release_browser=json.loads((release_out/'browser-result.json').read_text());check('existing-subset-release-browser-nonempty',release_browser['status']=='PASS' and bool(release_browser['checks']))
        for case in cases:
            work=d.ok(['work','show',case['old_work']['id']])['work'];ids=case['preparation']['conversion']['task_ids'];check(case['lang']+'-'+case['theme']+'-release-old-task-preserved',work['tasks'][0]==case['old_work']['tasks'][0] and all(not t.get('launch_held',False) for t in work['tasks'] if t['id'] in ids))
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    publish(out/'result.json',dict(status='PASS',checks=checks,browser_checks=len(browser['checks']),provider_calls=0));print('EXISTING_WORK_PASS '+str(len(checks)))
if __name__=='__main__':main()
