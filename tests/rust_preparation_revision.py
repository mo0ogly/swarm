#!/usr/bin/env python3
"""Compared plan revision via real CLI/API and browser on owned Roots.

Keep original refusals, full SQL fingerprints and fixture-only state injections.
No provider, live Root, global watchdog or acceptance of synthetic history.
"""
import argparse,json,sqlite3,subprocess
from pathlib import Path
from rust_preparation_reads import Driver
from rust_preparation_plan import plan,task
from rust_agent_model_route import publish
import rust_native_http_cli as native

def main():
    """Exercise explicit comparison, dual CAS, pause, locks, replay and recovery."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-unported',action='store_true');ap.add_argument('--backend-only',action='store_true');a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);d=Driver(Path(a.binary).resolve(),out);checks=[]
    def check(name,value):
        """Persist meaningful named state assertions, including original failures."""
        assert value,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def command(p,action,event,**extra):
        """Bind every explicit edit to the observed preparation revision."""
        return d.ok(['prepare',action,p['id']],dict(version=1,action=action,preparation_id=p['id'],event_id=event,expected_revision=p['revision'],**extra))
    def fixture(label):
        """Prepare, materialize and release an old plan using public operations."""
        p=d.ok(['prepare','create'],dict(version=1,action='create',event_id=label+'-create',expected_revision=0,title='Revision '+label,text='Need'))
        p=command(p,'save',label+'-brief',document='brief',text='Brief');p=command(p,'adopt-brief',label+'-adopt',sha256=p['documents']['brief']['sha256'])
        old=plan();old['tasks']=[task('a'),task('b',['a']),task('c')]
        p=command(p,'save',label+'-old',document='plan',text=json.dumps(old));p=command(p,'validate-plan',label+'-old-valid',sha256=p['documents']['plan']['sha256']);p=command(p,'create-missions',label+'-convert',sha256=p['documents']['plan']['sha256'],expected_work_revision=0)
        w=d.ok(['work','show',p['work_id']])['work'];p=command(p,'release-plan',label+'-release',sha256=p['conversion']['plan_hash'],expected_work_revision=w['revision']);w=d.ok(['work','show',p['work_id']])['work']
        new=plan();new['tasks']=[task('a'),task('b',['a']),task('d',['b'])];new['tasks'][0]['title']='Revised <b>literal</b>'
        p=command(p,'save',label+'-new',document='plan',text=json.dumps(new));p=command(p,'validate-plan',label+'-new-valid',sha256=p['documents']['plan']['sha256']);return dict(preparation=p,old_work=w,new_spec=new)
    def intent(case,event):
        """Select the validated new draft and both actual reviewed revisions."""
        p=case['preparation'];return dict(version=1,action='revise-missions',event_id=event,preparation_id=p['id'],expected_revision=p['revision'],expected_work_revision=case['old_work']['revision'],sha256=p['documents']['plan']['sha256'])
    def inspect(p,case,label):
        """Require historical plan/tasks, new held subset, pause and no execution."""
        old=case['old_work'];w=d.ok(['work','show',old['id']])['work'];ids=p['conversion']['task_ids'];tasks={t['id'].rsplit('-',1)[-1]:t for t in w['tasks']}
        check(label+'-revision-and-contract',w['revision']==old['revision']+1 and tasks['a']['title']=='Revised <b>literal</b>' and tasks['b']['status']=='todo' and tasks['d']['depends']==[tasks['b']['id']])
        check(label+'-retired-history-preserved',tasks['c']['status']=='abandoned' and tasks['c']['next']==old['tasks'][2]['next'] and tasks['c']['criteria']==old['tasks'][2]['criteria'] and w['plans'][:-1]==old['plans'] and w['plans'][-1]['spec']==case['new_spec'])
        check(label+'-new-locks-no-release',not p['conversion'].get('released_at') and ids==[tasks[k]['id'] for k in ['a','b','d']] and all(tasks[k]['launch_held'] for k in ['a','b','d']))
        with sqlite3.connect('file:'+str(d.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
            locks=db.execute('SELECT task_id,released FROM preparation_launch_locks WHERE work_id=? ORDER BY task_id',(w['id'],)).fetchall();paused=db.execute('SELECT paused FROM cockpit_controls WHERE work_id=?',(w['id'],)).fetchone();agents=db.execute('SELECT count(*) FROM agents').fetchone()[0];event=db.execute('SELECT kind,revision FROM events WHERE work_id=? ORDER BY revision DESC LIMIT 1',(w['id'],)).fetchone()
        check(label+'-atomic-pause-and-retained-lock',locks==sorted([(tasks[k]['id'],0) for k in ['a','b','d']]+[(tasks['c']['id'],1)]) and paused==(1,) and agents==0 and event==('preparation.revise-missions',w['revision']))
        mission=d.ok(['mission','show',w['id']]);check(label+'-public-mission-paused',mission['paused'] is True and mission['revision']==w['revision']);return w
    d.ok(['init']);d.settings(preparation_write_enabled='1');case=fixture('cli');p=case['preparation'];r=intent(case,'revise-cli');before=d.database();view=d.ok(['prepare','release-review',p['id']]);check('revision-positive-review-readonly',view['work']==case['old_work'] and before==d.database())
    if a.expect_unported:
        rc,_,err=d.call(['prepare','revise-missions',p['id']],r);check('revision-baseline-refuses',rc!=0 and before==d.database());print('PREPARATION_REVISION_BASELINE_PASS '+str(len(checks)));return
    check('revision-classification-order',[(c['id'],c['kind']) for c in view['revision']['changes']]==[('a','modifiée'),('b','à revérifier (dépendance modifiée)'),('d','ajoutée'),('c','retirée (historique conservé)')] and view['revision']['plan_hash']==r['sha256'])
    for label,extra in [('work-cas',dict(expected_work_revision=r['expected_work_revision']+1)),('preparation-cas',dict(expected_revision=p['revision']-1)),('hash',dict(sha256='wrong'))]:
        before=d.database();rc,_,err=d.call(['prepare','revise-missions',p['id']],{**r,**extra});check('revision-'+label+'-no-write',rc!=0 and before==d.database() and json.loads(err)['failure']['code'] in ['preparation-work-revision-conflict','preparation-revision-conflict','preparation-stale-document'])
    # Active task/review corruption belongs to the Rust pure-contract test.
    # The authoritative identity SQL guard intentionally rejects unregistered
    # Python writers; never install a fake UDF or disable its trigger here.
    p=d.ok(['prepare','revise-missions',p['id']],r);inspect(p,case,'revision-cli');before=d.database();check('revision-exact-event-replay',d.ok(['prepare','revise-missions',p['id']],r)==p and before==d.database())
    current=d.ok(['work','show',p['work_id']])['work'];p=command(p,'release-plan','revision-explicit-release',sha256=p['conversion']['plan_hash'],expected_work_revision=current['revision']);mission=d.ok(['mission','show',p['work_id']]);check('revision-release-keeps-explicit-cockpit-pause',mission['paused'] is True and bool(p['conversion']['released_at']))
    if a.backend_only:publish(out/'result.json',dict(status='PASS',checks=checks));print('PREPARATION_REVISION_BACKEND_PASS '+str(len(checks)));return
    http=fixture('http');cases=[dict(lang=lang,theme=theme,**fixture(lang+'-'+theme)) for lang in ['fr','en'] for theme in ['etat','sombre']]
    server=native.Server(d.binary,d.root,out,'preparation-revision');server.capture_async(out/'observer.raw');primary=None
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token'];hp=http['preparation'];before=d.database();check('revision-http-read-equal',server.request('/api/v1/preparations/release-review?id='+hp['id'],observe=False)==d.ok(['prepare','release-review',hp['id']]) and before==d.database());wanted=intent(http,'revise-http');server.request('/api/v1/preparations/command',wanted,csrf='wrong',status=403,observe=False);check('revision-csrf-no-write',before==d.database());hp=server.request('/api/v1/preparations/command',wanted,csrf=csrf,observe=False);inspect(hp,http,'revision-http')
        cfg=dict(base='http://'+server.address,cookie=server.cookie,output=str(out),cases=cases,binary=str(d.binary),root=str(d.root))
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:result=subprocess.run(['node','tests/rust_preparation_revision_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        check('revision-browser',result.returncode==0);print((out/'browser.stdout').read_text(),end='');browser=json.loads((out/'browser-result.json').read_text());check('revision-browser-nonempty',browser['status']=='PASS' and bool(browser['checks']))
        for c in json.loads((out/'browser-cases.json').read_text()):inspect(d.ok(['prepare','show',c['preparation']['id']]),c,c['lang']+'-'+c['theme'])
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    publish(out/'result.json',dict(status='PASS',checks=checks,browser_checks=len(browser['checks']),provider_calls=0));print('PREPARATION_REVISION_PASS '+str(len(checks)))
if __name__=='__main__':main()
