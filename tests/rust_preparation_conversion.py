#!/usr/bin/env python3
"""Locked new-work conversion via real CLI/API/UI on an owned Root.

Assert exact replay, fresh policies, all-or-nothing locks/events and refusal
recovery. SQL injection is restricted to a fixture after a valid witness. No
provider/global watchdog or live Root is used. Backend-only is diagnostic only.
"""
import argparse,json,sqlite3,subprocess
from pathlib import Path
from rust_preparation_reads import Driver
from rust_preparation_plan import plan,task,product_plan
from rust_agent_model_route import publish
import rust_native_http_cli as native


def main():
    """Keep named assertions/original channels and always drain owned servers."""
    parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);parser.add_argument('--backend-only',action='store_true');a=parser.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);d=Driver(Path(a.binary).resolve(),out);checks=[]
    def check(name,value):
        """Persist meaningful state assertions separately from command exits."""
        assert value,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def prepared(label,spec=None,work=''):
        """Use public edits/adoption/verdict; never fabricate preparation state."""
        p=d.ok(['prepare','create'],dict(version=1,action='create',event_id=label+'-create',expected_revision=0,title='Conversion '+label,text='Need',work_id=work))
        for kind,text in [('brief','Brief'),('plan',json.dumps(spec or plan()))]:
            p=d.ok(['prepare','save',p['id']],dict(version=1,action='save',preparation_id=p['id'],event_id=label+'-'+kind,expected_revision=p['revision'],document=kind,text=text))
            if kind=='brief':p=d.ok(['prepare','adopt-brief',p['id']],dict(version=1,action='adopt-brief',preparation_id=p['id'],event_id=label+'-adopt',expected_revision=p['revision'],sha256=p['documents']['brief']['sha256']))
        return d.ok(['prepare','validate-plan',p['id']],dict(version=1,action='validate-plan',preparation_id=p['id'],event_id=label+'-validate',expected_revision=p['revision'],sha256=p['documents']['plan']['sha256']))
    def intent(p,event):
        """Bind conversion to exact preparation and new-work revision zero."""
        return dict(version=1,action='create-missions',preparation_id=p['id'],event_id=event,expected_revision=p['revision'],expected_work_revision=0,sha256=p['documents']['plan']['sha256'])
    def refused(r,label,code,status=2,reason=None):
        """Every rejection must retain the complete fixture SQL dump."""
        before=d.database();rc,_,err=d.call(['prepare','create-missions',r['preparation_id']],r);check(label,rc==status and json.loads(err)['failure']['code']==code and (reason is None or reason in json.loads(err)['failure']['causes']) and d.database()==before)
    def inspect(p,label):
        """Observe persisted Work, exact task mapping, locks and absence of agents."""
        w=d.ok(['work','show',p['work_id']])['work'];c=p['conversion'];ids=c['task_ids'];check(label+'-work-provenance',w['id']==c['work_id'] and w['revision']==1 and w['plans'][-1]['task_ids']==ids and w['plans'][-1]['spec']==c['spec'])
        check(label+'-tasks-held',len(w['tasks'])==len(ids) and all(t['launch_held'] and t['status']=='todo' and not t['attempts'] and t['plan_max_attempts']==1 and t['plan_tool_limit']==1 for t in w['tasks']))
        with sqlite3.connect('file:'+str(d.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
            locks=db.execute('SELECT task_id,preparation_id,released FROM preparation_launch_locks WHERE work_id=? ORDER BY task_id',(w['id'],)).fetchall();agents=db.execute('SELECT count(*) FROM agents WHERE work_id=?',(w['id'],)).fetchone()[0];events=db.execute('SELECT kind,revision FROM events WHERE work_id=?',(w['id'],)).fetchall()
        check(label+'-locks-and-event',locks==[(i,p['id'],0) for i in sorted(ids)] and events==[('preparation.create-missions',1)] and agents==0)
        return w
    for lang in ['fr','en']:
        result=subprocess.run([str(d.binary),'--help','--lang',lang],capture_output=True);check('conversion-help-'+lang,result.returncode==0 and b'prepare create|save|method|adopt-brief|validate-plan|answer-questions|create-missions' in result.stdout)
    d.ok(['init']);d.settings(preparation_write_enabled='1')
    spec=plan();spec['tasks']=[task('later',['first']),task('independent'),task('first')];p=prepared('ordered',spec);r=intent(p,'convert-ordered')
    for label,extra,code,status in [('cas',{'expected_revision':p['revision']-1},'preparation-revision-conflict',3),('hash',{'sha256':'wrong'},'preparation-stale-document',2),('work-revision',{'expected_work_revision':1},'preparation-request-invalid',2),('organization',{'organization':{}},'preparation-request-invalid',2)]:refused({**r,**extra},'conversion-refuse-'+label,code,status)
    converted=d.ok(['prepare','create-missions',p['id']],r);w=inspect(converted,'conversion');check('conversion-topological-order',w['tasks'][1]['depends']==[w['tasks'][0]['id']] and w['tasks'][0]['id'].endswith('-first'))
    before=d.database();code,_,err=d.call(['task','update',w['id']],dict(schema_version=1,event_id='locked-start',expected_revision=w['revision'],id=w['tasks'][0]['id'],status='running'));check('conversion-public-start-refused',code!=0 and d.database()==before and json.loads(err)['failure']['code']=='preparation-launch-locked');
    before=d.database();check('conversion-replay-no-second-write',d.ok(['prepare','create-missions',p['id']],r)==converted and d.database()==before)
    refused(intent(converted,'convert-again'),'conversion-second-event-refused','preparation-already-created');refused({**r,'sha256':'other'},'conversion-event-conflict','preparation-event-conflict',3)
    existing=prepared('existing',work=w['id']);refused(intent(existing,'existing-convert'),'conversion-existing-work-refused','preparation-existing-work-unported')
    q=prepared('freshness');wanted=intent(q,'fresh-convert');d.settings(preparation_plan_max_tasks='0');refused(wanted,'conversion-revalidate-plan-policy','preparation-invalid-plan');d.settings(preparation_plan_max_tasks='8');d.settings(preparation_document_max_bytes='8');refused(wanted,'conversion-revalidate-document-policy','preparation-request-invalid');d.settings(preparation_document_max_bytes='50000');d.settings(preparation_method_max_bytes='0');refused(wanted,'conversion-stale-method-policy','root-cleanup',reason='preparation-stale-plan');d.settings(preparation_method_max_bytes='131072');source=d.root/'.claude/skills/apex/SKILL.md';original=source.read_bytes() if source.exists() else None;source.parent.mkdir(parents=True,exist_ok=True);source.write_text('Changed local method fixture');refused(wanted,'conversion-stale-source-content','preparation-stale-plan');source.write_bytes(original) if original is not None else source.unlink()
    # Valid conversion above is the witness under identical schema/settings.
    d.write_fixture("CREATE TRIGGER fixture_conversion_lock AFTER INSERT ON preparation_launch_locks BEGIN UPDATE preparation_launch_locks SET released=1 WHERE work_id=NEW.work_id AND task_id=NEW.task_id; END",())
    refused(wanted,'conversion-trigger-schema-refusal','root-schema-mismatch');d.write_fixture('DROP TRIGGER fixture_conversion_lock',());q=d.ok(['prepare','create-missions',q['id']],wanted);inspect(q,'conversion-recovery')
    late=prepared('late-trigger');late_request=intent(late,'late-convert');d.write_fixture("CREATE TRIGGER fixture_conversion_late AFTER INSERT ON preparation_commands WHEN json_extract(CAST(NEW.request AS TEXT),'$.action')='create-missions' BEGIN UPDATE preparation_launch_locks SET released=1 WHERE preparation_id=NEW.preparation_id; END",());refused(late_request,'conversion-late-trigger-schema-refusal','root-schema-mismatch');d.write_fixture('DROP TRIGGER fixture_conversion_late',());late=d.ok(['prepare','create-missions',late['id']],late_request);inspect(late,'conversion-late-recovery')
    prod=prepared('product',product_plan());prod=d.ok(['prepare','create-missions',prod['id']],intent(prod,'product-convert'));pw=inspect(prod,'conversion-product');check('conversion-product-story-contract','Story s1-example' in pw['tasks'][0]['next'] and pw['tasks'][0]['plan_checks']['plan-entry']=='entry')
    if a.backend_only:publish(out/'result.json',dict(status='PASS',checks=checks,scope='backend diagnostic only'));print('PREPARATION_CONVERSION_BACKEND_PASS '+str(len(checks)));return
    http_p=prepared('http');ui=[dict(lang=lang,theme=theme,preparation=prepared('ui-'+lang+'-'+theme)) for lang in ['fr','en'] for theme in ['etat','sombre']]
    server=native.Server(d.binary,d.root,out,'preparation-conversion');server.capture_async(out/'observer.raw');primary=None
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token'];caps=server.request('/api/v1/native-capabilities',observe=False);check('conversion-http-capability','create-missions' in caps['preparations'])
        wanted=intent(http_p,'http-convert');before=d.database();server.request('/api/v1/preparations/command',wanted,csrf='wrong',status=403,observe=False);check('conversion-csrf-no-write',d.database()==before)
        http_p=server.request('/api/v1/preparations/command',wanted,csrf=csrf,observe=False);check('conversion-http-shared-writer',d.ok(['prepare','show',http_p['id']])==http_p);inspect(http_p,'conversion-http')
        cfg=dict(base='http://'+server.address,cookie=server.cookie,output=str(out),cases=ui)
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:result=subprocess.run(['node','tests/rust_preparation_conversion_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        check('conversion-browser',result.returncode==0)
        browser=json.loads((out/'browser-result.json').read_text());assert browser['status']=='PASS' and browser['checks'];print((out/'browser.stdout').read_text(),end='')
        for case in ui:inspect(d.ok(['prepare','show',case['preparation']['id']]),'conversion-ui-'+case['lang']+'-'+case['theme'])
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    publish(out/'result.json',dict(status='PASS',checks=checks,browser_checks=len(browser['checks']),provider_calls=0));print('PREPARATION_CONVERSION_PASS '+str(len(checks)),flush=True)


if __name__=='__main__':main()
