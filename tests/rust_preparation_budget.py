#!/usr/bin/env python3
"""Exercise preparation envelope configuration in an isolated Root via public CLI/API.

No provider execution, live data or consumption reset. Amounts are explicit fixture inputs.
"""
import argparse,hashlib,json,sqlite3,subprocess
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native
from rust_preparation_plan import plan

def main():
    """Require CAS, retained metadata, invalid refusals and exact event recovery."""
    parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);parser.add_argument('--expect-unported',action='store_true');a=parser.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'prep-budget');f.settings_change(preparation_write_enabled='1');checks=[]
    def check(name,value):
        """Persist named observed assertions, including before/after SQL witnesses."""
        assert value,name;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def dump():
        """Fingerprint fixture SQL only; never use a running mission datastore."""
        with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
    def create(event):
        """Create a manual draft through the supported writer."""
        return f.ok(['prepare','create'],dict(version=1,action='create',event_id=event,expected_revision=0,title='Budget fixture',text='Need'))
    def intent(p,event,budget):
        """Bind the explicit envelope to the returned preparation revision."""
        return dict(version=1,action='budget',preparation_id=p['id'],event_id=event,expected_revision=p['revision'],budget=budget)
    budget=dict(limit_usd=12,reserve_per_launch_usd=0.5,estimate_source='Observed fixture',reference_date='2026-10-26',actor='forged',updated='forged');p=create('budget-main');r=intent(p,'budget-config',budget)
    if a.expect_unported:
        before=dump();rc,wire=f.call(['prepare','budget',p['id']],r);check('budget-baseline-unported',rc!=0 and before==dump());return
    # Synthetic historical rows, owned fixture only: no provider execution claim.
    with sqlite3.connect(f.root/'.swarm/state.db') as db:
        for label,state,amount in [('held','reserved',0.5),('spent','estimated',2.5),('released','released',1.0)]:
            db.execute('INSERT INTO preparation_turns(id,preparation_id,status,body) VALUES(?,?,?,?)',(label,p['id'],'answered',json.dumps(dict(id=label,preparation_id=p['id'],status='answered'))))
            db.execute('INSERT INTO preparation_reservations(turn_id,preparation_id,work_id,amount,work_amount,state) VALUES(?,?,?,?,?,?)',(label,p['id'],None,amount,0,state))
    def history():
        """Snapshot actual stored fixture commitments independently of envelope."""
        with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:return [db.execute('SELECT * FROM '+table+' ORDER BY 1').fetchall() for table in ['preparation_turns','preparation_reservations']]
    original_history=history()
    configured=f.ok(['prepare','budget',p['id']],r);b=configured['budget'];check('budget-explicit-persisted',b['limit_usd']==12 and b['reserve_per_launch_usd']==0.5 and b['estimate_source']==budget['estimate_source'] and b['reference_date']==budget['reference_date']);check('budget-engine-metadata',b['actor'] and b['actor']!='forged' and b['updated'] and b['updated']!='forged');check('budget-documents-preserved',configured['documents']==p['documents'] and configured['revision']==p['revision']+1);before=dump();check('budget-exact-replay',f.ok(['prepare','budget',p['id']],r)==configured and dump()==before)
    for label,change,code in [('negative',{'limit_usd':-1},'preparation-budget-invalid'),('zero-reserve',{'reserve_per_launch_usd':0},'preparation-budget-invalid'),('missing-source',{'estimate_source':''},'preparation-budget-invalid'),('bad-date',{'reference_date':'2026-02-30'},'preparation-budget-invalid'),('unknown',{'surprise':1},'preparation-request-invalid')]:
        request=intent(configured,label,{**budget,**change});before=dump();rc,wire=f.call(['prepare','budget',p['id']],request);check('budget-refuse-'+label,rc!=0 and wire['failure']['code']==code and before==dump())
    before=dump();missing=intent(configured,'missing-budget',None);rc,wire=f.call(['prepare','budget',p['id']],missing);check('budget-missing-refused',rc!=0 and wire['failure']['code']=='preparation-request-invalid' and dump()==before)
    before=dump();rc,wire=f.call(['prepare','budget',p['id']],{**r,'budget':{**budget,'limit_usd':13}});check('budget-event-conflict',rc!=0 and wire['failure']['code']=='preparation-event-conflict' and before==dump());rc,wire=f.call(['prepare','budget',p['id']],{**r,'event_id':'stale'});check('budget-stale-cas',rc!=0 and wire['failure']['code']=='preparation-revision-conflict' and before==dump());unlimited=f.ok(['prepare','budget',p['id']],intent(configured,'unlimited',dict(limit_usd=0,reserve_per_launch_usd=0,estimate_source='',reference_date='')));check('budget-unlimited-explicit',unlimited['budget']['limit_usd']==0 and unlimited['budget']['reserve_per_launch_usd']==0);check('budget-historical-replay',f.ok(['prepare','budget',p['id']],r)['receipt_historical'] is True)
    planned=create('budget-plan')
    def plan_command(action,event,**extra):
        """Apply the next source-bound plan transition through the real writer."""
        nonlocal planned
        planned=f.ok(['prepare',action,planned['id']],dict(version=1,action=action,preparation_id=planned['id'],event_id=event,expected_revision=planned['revision'],**extra))
    plan_command('save','budget-brief',document='brief',text='Brief');plan_command('adopt-brief','budget-adopt',sha256=planned['documents']['brief']['sha256']);plan_command('save','budget-plan-save',document='plan',text=json.dumps(plan()));plan_command('validate-plan','budget-plan-check',sha256=planned['documents']['plan']['sha256']);previous_plan=planned
    planned=f.ok(['prepare','budget',planned['id']],intent(planned,'budget-retains-plan',budget));check('budget-current-verdict-preserved',planned['plan_ready'] and planned['verdict']==previous_plan['verdict'] and planned['documents']==previous_plan['documents'])
    http=create('budget-http');cases=[dict(lang=lang,theme=theme,preparation=create('ui-'+lang+'-'+theme)) for lang in ['fr','en'] for theme in ['etat','sombre']];server=native.Server(f.binary,f.root,out,'preparation-budget',environment=f.environment);server.capture_async(out/'observer.raw');primary=None
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token'];check('budget-http-capability','budget' in server.request('/api/v1/native-capabilities',observe=False)['preparations']);request=intent(http,'http-budget',budget);before=dump();server.request('/api/v1/preparations/command',request,csrf='wrong',status=403,observe=False);check('budget-csrf-no-write',before==dump());reply=server.request('/api/v1/preparations/command',{**request,'budget':{**budget,'limit_usd':-1}},csrf=csrf,status=400,observe=False);check('budget-http-invalid',reply['failure']['code']=='preparation-budget-invalid' and before==dump());saved=server.request('/api/v1/preparations/command',request,csrf=csrf,observe=False);check('budget-http-cli-shared',f.ok(['prepare','show',http['id']])==saved)
        cfg=dict(base='http://'+server.address,cookie=server.cookie,output=str(out),cases=cases)
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:result=subprocess.run(['node','tests/rust_preparation_budget_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        check('budget-browser',result.returncode==0);print((out/'browser.stdout').read_text(),end='')
        for case in cases:check('budget-ui-persisted-'+case['lang']+'-'+case['theme'],f.ok(['prepare','show',case['preparation']['id']])['budget']['limit_usd']==9)
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    lower=f.ok(['prepare','budget',p['id']],intent(unlimited,'lower-than-commitment',dict(limit_usd=1,reserve_per_launch_usd=0.25,estimate_source='Explicit reduced envelope',reference_date='2026-10-26')));check('budget-lower-preserves-commitments',lower['budget']['limit_usd']==1 and history()==original_history)
    check('budget-consumption-history-preserved',history()==original_history)
    check('budget-never-executed-provider',not (f.root/'model-received.json').exists());print('PREPARATION_BUDGET_PASS '+str(len(checks)))
if __name__=='__main__':main()
