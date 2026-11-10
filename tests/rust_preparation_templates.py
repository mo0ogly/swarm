#!/usr/bin/env python3
"""Verify existing preparation templates/checks on an inactive owned Root.

No model, Go process, native mission or live state is executed. A projected plan
is a draft; missing decisions and false launch authorization remain observable.
"""
import argparse,hashlib,json,sqlite3
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native

def main():
    """Require current catalogue, bilingual projection, refusal and saved caps."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-unported',action='store_true');a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'preparation-templates');checks=[]
    def check(name,value):
        """Retain meaningful named observations; a refusal never counts as success."""
        assert value,name;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def database():
        with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
    before=database();code,value=f.call(['prepare','templates'])
    if a.expect_unported:
        check('templates-baseline-refuses',code!=0 and before==database());s=native.Server(f.binary,f.root,out,'template-baseline',environment=f.environment);primary=None
        try:s.request('/api/v1/preparations/templates',status=404,observe=False);check('templates-http-baseline-unported',database()==before)
        except BaseException as e:primary=e
        finally:native.finish(s,primary)
        print('PREPARATION_TEMPLATES_BASELINE 2');return
    check('templates-catalogue-valid-control',code==0 and isinstance(value,list) and len(value)==10);templates=value;portal=next(t for t in templates if t['id']=='client-portal');ordinary=next(t for t in templates if t['id']=='application');check('templates-existing-work-graph',len(portal['work_model']['increments'])==5 and all(len(i['plans']['fr']['tasks'])>0 for i in portal['work_model']['increments']));check('templates-frozen-role-framing',all(r['workflow']['role']==r['role'] and len(r['workflow']['sha256'])==64 for t in templates for r in t['team']));check('templates-alias-selection',f.ok(['prepare','template','ks-feature'])==ordinary)
    def assess(request):return f.ok(['prepare','template-check'],request)
    def intent(template,lang='fr',**changes):return dict(template_id=template['id'],language=lang,answers={q['id']:'Fixture '+q['id'] for q in template['questions']},**changes)
    empty=assess(dict(template_id='application'));check('templates-required-missing',not empty['need_complete'] and empty['answered']==0 and len(empty['missing'])==len([q for q in ordinary['questions'] if q['required']]) and not empty['launch_authorized'])
    for language in ['fr','en']:
        req=intent(portal,language);v=assess(req);check('templates-projection-'+language,v['need_complete'] and not v['launch_authorized'] and v['increment']=='foundations' and v['work_plan']['product']['mode']=='greenfield' and all('Fixture '+q['id'] in v['need'] for q in portal['questions']) and any(not q['answer'] for q in v['work_plan']['questions']));v=assess({**req,'intervention':'migrate','increment':'session'});check('templates-migration-projection-'+language,v['work_plan']['product']['mode']=='existing' and v['increment']=='session')
    valid=intent(ordinary)
    for name,patch in [('unknown-language',{'language':'xx'}),('unknown-answer',{'answers':{'unlisted':''}}),('too-long-answer',{'answers':{ordinary['questions'][0]['id']:'é'*501}}),('unknown-intervention',{'intervention':'unlisted'}),('no-graph-increment',{'increment':'foundations'}),('unknown-template',{'template_id':'unlisted'})]:
        assess(valid);b=database();code,_=f.call(['prepare','template-check'],{**valid,**patch});check('templates-refuses-'+name,code!=0 and database()==b)
    q1,q2=[q['id'] for q in ordinary['questions'][:2]]
    def raw_check(name,raw):
        """Retain ordered/duplicate raw members rather than sorting a test map."""
        path=out/(name+'.json');fixture.publish(path,raw.encode());return f.ok(['prepare','template-check','--input',str(path)])
    merged=raw_check('map-merge','{"TEMPLATE_ID":"application","ANSWERS":{'+json.dumps(q1)+':"first"},"answers":{'+json.dumps(q2)+':"second"}}');check('templates-go-map-alias-merge',merged['answered']==2 and 'first' in merged['need'] and 'second' in merged['need'])
    cleared=raw_check('map-null','{"template_id":"application","answers":{'+json.dumps(q1)+':"first",'+json.dumps(q1)+':null}}');check('templates-go-map-null-zero-value',cleared['answered']==0 and cleared['missing']==empty['missing']);check('templates-nil-answer-map',assess(dict(template_id='application',answers=None))==empty)
    q=ordinary['questions'][0]['id'];r=assess({**valid,'answers':{**valid['answers'],q:'[define missing]'}});check('templates-placeholder-missing',q in r['missing']);defaults=f.ok(['engine-settings','show'])['values'];f.settings_change(preparation_template_answer_max_bytes='1');code,_=f.call(['prepare','template-check'],valid);check('templates-answer-cap-consumed',code!=0);f.settings_change(preparation_template_answer_max_bytes=None);check('templates-answer-unlimited',assess(valid)['need_complete']);f.settings_change(preparation_template_answer_max_bytes=defaults['preparation_template_answer_max_bytes'],preparation_template_draft_max_bytes='0');code,_=f.call(['prepare','template-check'],valid);check('templates-draft-zero-consumed',code!=0);f.settings_change(preparation_template_draft_max_bytes=None);check('templates-draft-unlimited',assess(valid)['need_complete']);f.settings_change(preparation_template_draft_max_bytes=defaults['preparation_template_draft_max_bytes']);check('templates-projection-isolated',assess(intent(portal))==assess(intent(portal)) and f.ok(['prepare','templates'])==templates)
    s=native.Server(f.binary,f.root,out,'templates',environment=f.environment);s.capture_async(out/'observer.raw');primary=None
    try:
        check('templates-http-cli-catalogue',s.request('/api/v1/preparations/templates',observe=False)==templates);csrf=s.request('/api/v1/native-session',observe=False)['csrf_token'];b=database();s.request('/api/v1/preparations/template-check',valid,csrf='wrong',status=403,observe=False);check('templates-csrf-no-write',database()==b);check('templates-http-cli-check',s.request('/api/v1/preparations/template-check',valid,csrf=csrf,observe=False)==assess(valid));s.request('/api/v1/preparations/template-check',{**valid,'answers':{'unknown':''}},csrf=csrf,status=400,observe=False);check('templates-http-invalid-no-write',database()==b)
    except BaseException as e:primary=e
    finally:native.finish(s,primary)
    check('templates-no-sql-mutation',database()==before);check('templates-no-provider-execution',not (f.root/'model-received.json').exists());print('PREPARATION_TEMPLATES_PASS '+str(len(checks)))
if __name__=='__main__':main()
