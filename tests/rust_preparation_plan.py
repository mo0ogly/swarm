#!/usr/bin/env python3
"""Plan verdicts via Rust CLI/protected HTTP on an isolated owned Root.

Strict Go-shaped plans, source freshness, graphs, product references and persisted
Admin caps are exercised without conversion, provider calls or live DB mutation.
Every refusal requires exact safe code and unchanged SQL. No global watchdog.
"""
import argparse
import copy
import json
import subprocess
from pathlib import Path
from rust_preparation_reads import Driver
from rust_agent_model_route import publish
import rust_native_http_cli as native


def task(identifier='t1',depends=None):
    """Return a complete fixture task; no task is executed or materialized."""
    return dict(id=identifier,title='Task',role='worker',scope='scope',deliverable='delivery',depends=[] if depends is None else depends,criteria=['proof'],proof='evidence',entry='entry',validation='validation',delivery='delivery',stop='stop',max_attempts=1,max_tool_calls=1)


def plan():
    """Complete non-product ActionPlan fixture using protocol version1."""
    return dict(version=1,objective='A valid plan',assumptions=[],questions=[],tasks=[task()])


def product_plan():
    """Minimal covered product story, with task references and no dependency."""
    value=plan();value['product']=dict(mode='existing',journeys=[dict(id='j1',title='Journey',goal='Goal',story_ids=['s1-example'])],stories=[dict(id='s1-example',title='Story',user='User',value='Value',criteria=['Proof'],complexity=3,depends=[],task_ids=['t1'])]);return value


def main():
    """Retain all channels and named checks; always stop the owned HTTP server."""
    parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);a=parser.parse_args()
    out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);d=Driver(Path(a.binary).resolve(),out);checks=[]

    def check(name,value):
        """Persist distinct assertions after observing actual values/state."""
        assert value,name;assert name not in checks,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)

    def save(p,event,kind,text):
        """Use an observed preparation revision for a public document write."""
        return d.ok(['prepare','save',pid],dict(version=1,action='save',preparation_id=pid,event_id='save-'+event,expected_revision=p['revision'],document=kind,text=text))

    def request(p,event):
        """Validate the exact stored plan hash; never invent a current revision."""
        return dict(version=1,action='validate-plan',preparation_id=pid,event_id=event,expected_revision=p['revision'],sha256=p['documents']['plan']['sha256'])

    def refused(value,name,family='preparation-invalid-plan',exit_code=2):
        """Require safe exact family, no success output and no persisted mutation."""
        before=d.database();code,stdout,stderr=d.call(['prepare','validate-plan',pid],value)
        check(name,code==exit_code and not stdout and json.loads(stderr)['failure']['code']==family and d.database()==before)
        return json.loads(stderr)

    def draft(p,event,value):
        """Save literal text or compact fixture JSON through the public writer."""
        return save(p,event,'plan',value if isinstance(value,str) else json.dumps(value,separators=(',',':')))

    d.ok(['init']);check('plan-schema43',d.ok(['engine-settings','show'])['schema_version']==43)
    p=d.ok(['prepare','create'],dict(version=1,action='create',event_id='plan-create',expected_revision=0,text='need'));pid=p['id']
    p=save(p,'brief','brief','brief');p=d.ok(['prepare','adopt-brief',pid],dict(version=1,action='adopt-brief',preparation_id=pid,event_id='adopt',expected_revision=p['revision'],sha256=p['documents']['brief']['sha256']))
    p=draft(p,'initial-plan',plan());wanted=request(p,'validate-first');history=d.ok(['prepare','history',pid,'plan'])
    refused({**wanted,'sha256':'wrong'},'plan-hash-refused','preparation-stale-document')
    refused({**wanted,'expected_revision':p['revision']-1},'plan-cas-refused','preparation-revision-conflict',3)
    for label,extra in [('text',{'text':'unexpected'}),('document',{'document':'plan'}),('method',{'method':'debug'}),('decisions',{'decisions':[]}),('title',{'title':'unexpected'})]:
        refused({**wanted,'event_id':'foreign-'+label,**extra},'plan-foreign-'+label,'preparation-request-invalid')
    good=d.ok(['prepare','validate-plan',pid],wanted)
    check('plan-records-exact-verdict',good['plan_ready'] and good['verdict']['plan_hash']==p['documents']['plan']['sha256'] and good['verdict']['brief_hash']==p['adopted_brief']['sha256'] and good['verdict']['method_hash']==p['method_hash'] and good['revision']==p['revision']+1)
    check('plan-preserves-documents-history',good['documents']==p['documents'] and d.ok(['prepare','history',pid,'plan'])==history)
    before=d.database();check('plan-replay-no-write',d.ok(['prepare','validate-plan',pid],wanted)==good and d.database()==before)
    refused({**wanted,'sha256':'other'},'plan-event-conflict','preparation-event-conflict',3)
    p=good
    cases=[('json-malformed','{'),('json-extra-object',json.dumps(plan())+' {}')]
    for label,updates in [('unknown-field',{'unknown':True}),('version',{'version':2}),('objective',{'objective':' '}),('assumptions-null',{'assumptions':None}),('questions-null',{'questions':None}),('empty-tasks',{'tasks':[]}),('unanswered',{'questions':[dict(question='Why?',answer=' ')]}),('blank-question',{'questions':[dict(question='',answer='yes')]}),('empty-assumption',{'assumptions':[' ']}),('task-count',{'tasks':[task('t'+str(i)) for i in range(9)]})]:
        cases.append((label,{**plan(),**updates}))
    for label,updates in [('role',{'role':'reviewer'}),('id-length',{'id':'x'*33}),('id-invalid',{'id':'bad/id'}),('title',{'title':''}),('criteria-null',{'criteria':None}),('criteria-empty',{'criteria':[]}),('criterion-blank',{'criteria':[' ']}),('criteria-cap',{'criteria':['p']*13}),('depends-null',{'depends':None}),('unknown-dependency',{'depends':['missing']}),('self-cycle',{'depends':['t1']}),('attempts-zero',{'max_attempts':0}),('attempts-cap',{'max_attempts':4}),('tools-zero',{'max_tool_calls':0}),('tools-cap',{'max_tool_calls':101}),('phase',{'phase':'unknown'})]:
        cases.append(('task-'+label,{**plan(),'tasks':[{**task(),**updates}]}))
    cases.extend([('duplicate-task',{**plan(),'tasks':[task(),task()]}),('duplicate-edge',{**plan(),'tasks':[task('t1'),task('t2',['t1','t1'])]}),('task-cycle',{**plan(),'tasks':[task('t1',['t2']),task('t2',['t1'])]})])
    for label,value in cases:
        p=draft(p,'save-invalid-'+label,value);error=refused(request(p,'invalid-'+label),'plan-invalid-'+label)
        check('plan-safe-error-'+label,'DO_NOT_LEAK' not in json.dumps(error)) if label=='json-malformed' else None
    p=draft(p,'valid-forward-graph',{**plan(),'tasks':[task('t2',['t1']),task('t1')]});p=d.ok(['prepare','validate-plan',pid],request(p,'forward-graph'));check('plan-forward-dependency-valid',p['plan_ready'])
    p=draft(p,'base-recovery',plan());p=d.ok(['prepare','validate-plan',pid],request(p,'base-recovery'));check('plan-invalid-json-recovery',p['plan_ready'])
    before=d.database();old=d.ok(['prepare','validate-plan',pid],wanted);check('plan-historical-verdict-not-ready',old['receipt_historical'] and not old['plan_ready'] and d.database()==before)
    local=d.root/'.claude/skills/apex/SKILL.md';local.parent.mkdir(parents=True);publish(local,Path('.claude/skills/apex/SKILL.md').read_bytes()+b'\nLocal method fixture.\n')
    refused(request(p,'method-drift'),'plan-method-drift','preparation-stale-method')
    p=d.ok(['prepare','method',pid],dict(version=1,action='method',preparation_id=pid,event_id='method-refresh',expected_revision=p['revision'],method='apex'))
    refused(request(p,'old-plan-method'),'plan-old-method-provenance','preparation-stale-plan')
    p=draft(p,'method-rebased-plan',plan());p=d.ok(['prepare','validate-plan',pid],request(p,'method-recovery'));check('plan-method-recovery',p['plan_ready'])
    p=save(p,'need-drift','besoin','changed need');refused(request(p,'need-drift'),'plan-brief-need-drift','preparation-stale-brief')
    p=save(p,'brief-rebased','brief','changed brief');p=d.ok(['prepare','adopt-brief',pid],dict(version=1,action='adopt-brief',preparation_id=pid,event_id='adopt-rebased',expected_revision=p['revision'],sha256=p['documents']['brief']['sha256']))
    refused(request(p,'old-plan-brief'),'plan-old-brief-provenance','preparation-stale-plan')
    p=draft(p,'brief-rebased-plan',plan());p=d.ok(['prepare','validate-plan',pid],request(p,'brief-recovery'));check('plan-brief-recovery',p['plan_ready'])
    # Every operator cap has refusal and same-event recovery; no hidden Go cap.
    for field,value in [('preparation_plan_max_bytes',plan()),('preparation_plan_max_tasks',plan()),('preparation_plan_max_assumptions',{**plan(),'assumptions':['observed']}),('preparation_plan_max_questions',{**plan(),'questions':[dict(question='Why?',answer='because')]})]:
        p=draft(p,'save-cap-'+field,value);intent=request(p,'cap-'+field);d.settings(**{field:'0'});refused(intent,'plan-saved-zero-'+field);d.settings(**{field:None});p=d.ok(['prepare','validate-plan',pid],intent);check('plan-cap-recovery-'+field,p['plan_ready'])
    d.settings(preparation_document_max_bytes=None)
    large={**plan(),'objective':'x'*33000,'tasks':[task('n'+str(i),[] if i==0 else ['n'+str(i-1)]) for i in range(9)],'assumptions':['a']*17,'questions':[dict(question='q',answer='a')]*17}
    p=draft(p,'explicit-unlimited',large);p=d.ok(['prepare','validate-plan',pid],request(p,'explicit-unlimited'));check('plan-explicit-unlimited-consumed',p['plan_ready'])
    d.settings(task_max_attempts='4',task_max_tool_calls='101',task_max_criteria='13')
    p=draft(p,'raised-task-policy',{**plan(),'tasks':[{**task(),'max_attempts':4,'max_tool_calls':101,'criteria':['p']*13}]});p=d.ok(['prepare','validate-plan',pid],request(p,'raised-task-policy'));check('plan-existing-task-admin-policy-consumed',p['plan_ready'])
    d.settings(task_max_attempts='3',task_max_tool_calls='100',task_max_criteria='12')
    refused(request(p,'task-policy-lowered'),'plan-current-task-policy-refuses')
    # Readiness is the Go hash-freshness projection, not execution permission or
    # a proof that a past verdict was rerun under changed operator settings.
    check('plan-verdict-retained-after-policy-change',bool(d.ok(['prepare','show',pid]).get('verdict')))
    p=d.ok(['prepare','method',pid],dict(version=1,action='method',preparation_id=pid,event_id='product-method',expected_revision=p['revision'],method='ks-product'))
    p=draft(p,'missing-product',plan());refused(request(p,'missing-product'),'plan-product-method-requires-product')
    product=product_plan();p=draft(p,'valid-product',product);p=d.ok(['prepare','validate-plan',pid],request(p,'valid-product'));check('plan-product-valid',p['plan_ready'])
    products=[]
    for label,updates in [('mode',{'mode':'unknown'}),('journeys',{'journeys':[]}),('stories',{'stories':[]})]:
        v=copy.deepcopy(product);v['product'].update(updates);products.append((label,v))
    for label,updates in [('id',{'id':'bad'}),('complexity',{'complexity':5}),('risk',{'complexity':4,'risk':''}),('criterion',{'criteria':[' ']}),('depends-null',{'depends':None}),('tasks-null',{'task_ids':None}),('task-ref',{'task_ids':['missing']}),('task-duplicate',{'task_ids':['t1','t1']}),('self-cycle',{'depends':['s1-example']})]:
        v=copy.deepcopy(product);v['product']['stories'][0].update(updates);products.append(('story-'+label,v))
    for label,updates in [('id',{'id':'bad/id'}),('story-ref',{'story_ids':['missing']}),('story-duplicate',{'story_ids':['s1-example','s1-example']}),('empty-story-refs',{'story_ids':[]})]:
        v=copy.deepcopy(product);v['product']['journeys'][0].update(updates);products.append(('journey-'+label,v))
    v=copy.deepcopy(product);v['product']['stories'].append(copy.deepcopy(v['product']['stories'][0]));products.append(('duplicate-story',v))
    v=copy.deepcopy(product);v['product']['journeys'].append(copy.deepcopy(v['product']['journeys'][0]));products.append(('duplicate-journey',v))
    v=copy.deepcopy(product);second=copy.deepcopy(v['product']['stories'][0]);second['id']='s2-example';v['product']['stories'].append(second);products.append(('uncovered-story',v))
    v=copy.deepcopy(v);v['product']['journeys'][0]['story_ids'].append('s2-example');v['product']['stories'][0]['depends']=['s2-example'];v['product']['stories'][1]['depends']=['s1-example'];products.append(('story-cycle',v))
    for label,v in products:p=draft(p,'product-invalid-'+label,v);refused(request(p,'product-invalid-'+label),'plan-product-invalid-'+label)
    p=draft(p,'product-recovered',product);p=d.ok(['prepare','validate-plan',pid],request(p,'product-recovered'));check('plan-product-recovery',p['plan_ready'])
    optional=copy.deepcopy(product);optional['product']['stories'][0].update(complexity=4,risk='Explicit fixture risk',task_ids=[])
    p=draft(p,'product-risk-and-unplanned-story',optional);p=d.ok(['prepare','validate-plan',pid],request(p,'risk-and-unplanned-story'));check('plan-product-risk-and-empty-task-refs-valid',p['plan_ready'])
    # Real stdout failure follows a valid same-precondition verdict witness.
    intent=request(p,'verdict-output');path=out/'output.json';publish(path,intent)
    with open('/dev/full','wb',buffering=0) as full:r=subprocess.run([str(d.binary),'prepare','validate-plan',pid,'--root',str(d.root),'--json','--input',str(path)],stdout=full,stderr=subprocess.PIPE)
    publish(out/'output.stderr',r.stderr);check('plan-output-known-receipt',r.returncode==2 and json.loads(r.stderr)['recorded_preparation']==dict(id=pid,revision=p['revision']+1))
    p=d.ok(['prepare','validate-plan',pid],intent);check('plan-output-replay',p['plan_ready'])
    d.settings(preparation_write_enabled='0');refused(intent,'plan-disabled-denies-replay','preparation-write-disabled',4);d.settings(preparation_write_enabled='1')
    server=native.Server(d.binary,d.root,out,'plan');server.capture_async(out/'observer.raw');primary=None
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token'];check('plan-http-capability','validate-plan' in server.request('/api/v1/native-capabilities',observe=False)['preparations'])
        api=request(p,'plan-api');before=d.database();server.request('/api/v1/preparations/command',api,authorized=False,status=403,observe=False);server.request('/api/v1/preparations/command',api,status=403,observe=False);check('plan-http-protected',d.database()==before)
        failure=server.request('/api/v1/preparations/command',{**api,'sha256':'wrong'},csrf=csrf,status=409,observe=False);check('plan-http-stale-document',failure['failure']['code']=='preparation-stale-document' and d.database()==before)
        response=server.request('/api/v1/preparations/command',api,csrf=csrf,observe=False);check('plan-http-cli-same-verdict',response==d.ok(['prepare','show',pid]) and response['plan_ready'])
        before=d.database();check('plan-http-replay',server.request('/api/v1/preparations/command',api,csrf=csrf,observe=False)==response and d.database()==before)
        p=draft(response,'api-invalid-json','{"PRIVATE_PLAN_MARKER":true}')
        failure=server.request('/api/v1/preparations/command',request(p,'api-invalid'),csrf=csrf,status=400,observe=False);check('plan-http-invalid-safe',failure['failure']['code']=='preparation-invalid-plan' and 'PRIVATE_PLAN_MARKER' not in json.dumps(failure))
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    publish(out/'result.json',dict(status='PASS',checks=checks,provider_calls=0));print('PREPARATION_PLAN_PASS '+str(len(checks)),flush=True)


if __name__=='__main__':main()
