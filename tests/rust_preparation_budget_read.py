#!/usr/bin/env python3
"""Read historical preparation coverage through CLI/API on an owned fixture.

Synthetic turns/reservations are labeled test data, never actual provider execution.
Malformed rows must refuse without modifying their original state; repair restores reads.
"""
import argparse,hashlib,json,sqlite3
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native

def main():
    """Require exact estimates, unknown versus reported zero, policy and recovery."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-unported',action='store_true');a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'budget-read');f.settings_change(preparation_write_enabled='1');checks=[]
    def check(name,value):
        """Persist each meaningful assertion immediately, preserving failures."""
        assert value,name;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='reader-create',expected_revision=0,title='Budget history',text='Need'))
    def read():
        """Read through the same persisted projection as the protected API."""
        return f.ok(['prepare','budget',p['id']])
    def dump():
        """Fingerprint complete isolated SQL before refusals/read-only operations."""
        with sqlite3.connect(f.root/'.swarm/state.db') as db:return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
    if a.expect_unported:
        before=dump();code,_=f.call(['prepare','budget',p['id']]);check('reader-baseline-unported',code!=0 and dump()==before);return
    before=dump();v=read();check('reader-empty-unknown',v['turns']==0 and v['actual_cost_usd'] is None and v['reserved_usd']==0 and v['committed_estimate_usd']==0 and dump()==before)
    p=f.ok(['prepare','budget',p['id']],dict(version=1,action='budget',preparation_id=p['id'],event_id='reader-envelope',expected_revision=p['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=0.5,estimate_source='fixture',reference_date='2026-10-26')))
    def seed(label,usage,amount,state):
        """Insert only synthetic retained fixture data with matching row identities."""
        turn=dict(id=label,preparation_id=p['id'],status='answered',usage=usage)
        with sqlite3.connect(f.root/'.swarm/state.db') as db:
            db.execute('INSERT INTO preparation_turns(id,preparation_id,status,body) VALUES(?,?,?,?)',(label,p['id'],'answered',json.dumps(turn)));db.execute('INSERT INTO preparation_reservations(turn_id,preparation_id,work_id,amount,work_amount,state) VALUES(?,?,?,?,?,?)',(label,p['id'],None,amount,0,state))
        return turn
    seed('unknown',None,0.5,'reserved');check('reader-unknown-turn-cost',read()['actual_cost_usd'] is None and read()['turns']==1);zero=seed('zero',dict(input_tokens=2,output_tokens=3,provider_reported_cost_usd=0),2.5,'estimated');v=read();check('reader-reported-zero',v['actual_cost_usd']==0 and v['turns']==2 and v['usage_turns']==1 and v['cost_turns']==1)
    paid=seed('paid',dict(input_tokens=5,output_tokens=7,provider_reported_cost_usd=1.25),100,'released');v=read();check('reader-partial-coverage',v['actual_cost_usd']==1.25 and v['turns']==3 and v['usage_turns']==2 and v['cost_turns']==2 and v['input_tokens']==7 and v['output_tokens']==10);check('reader-reserved-estimated-separated',v['reserved_usd']==0.5 and v['committed_estimate_usd']==2.5 and v['remaining_usd']==7 and 'work_budget' not in v)
    catalogue=f.ok(['engine-settings','describe']);fields=catalogue['fields'] if isinstance(catalogue,dict) else catalogue;check('reader-admin-help-scope',any(v['name']=='work_budget_warning_percent' and 'préparation' in v['help'] for v in fields))
    original=f.ok(['engine-settings','show'])['values']['work_budget_warning_percent'];f.settings_change(work_budget_warning_percent='20' if isinstance(original,str) else 20);check('reader-saved-warning-consumed',read()['warning'] is True);f.settings_change(work_budget_warning_percent='90' if isinstance(original,str) else 90);check('reader-warning-change-consumed',read()['warning'] is False);f.settings_change(work_budget_warning_percent=None);check('reader-warning-disabled',read()['warning'] is False);f.settings_change(work_budget_warning_percent=original)
    def corrupt(label,sql,args,repair,repair_args,code):
        """A valid witness precedes each distinct fixture fault and exact repair."""
        read()
        with sqlite3.connect(f.root/'.swarm/state.db') as db:db.execute(sql,args)
        before=dump();status,reply=f.call(['prepare','budget',p['id']]);check(label,status!=0 and reply['failure']['code']==code and before==dump())
        with sqlite3.connect(f.root/'.swarm/state.db') as db:db.execute(repair,repair_args)
        check(label+'-recovery',read()['actual_cost_usd']==1.25)
    update='UPDATE preparation_turns SET body=? WHERE id=?'
    with sqlite3.connect(f.root/'.swarm/state.db') as db:db.execute(update,(json.dumps({**zero,'usage':dict(input_tokens=2,output_tokens=3)}),'zero'))
    check('reader-usage-without-cost-unknown',read()['actual_cost_usd']==1.25 and read()['cost_turns']==1 and read()['usage_turns']==2)
    with sqlite3.connect(f.root/'.swarm/state.db') as db:db.execute(update,(json.dumps(zero),'zero'))

    for label,usage,code in [('negative-tokens',{**paid['usage'],'input_tokens':-1},'preparation-budget-usage-invalid'),('negative-cost',{**paid['usage'],'provider_reported_cost_usd':-0.5},'preparation-budget-amount-invalid'),('token-overflow',{**paid['usage'],'input_tokens':9223372036854775807},'preparation-budget-count-overflow')]:
        corrupt('reader-'+label,update,(json.dumps({**paid,'usage':usage}),'paid'),update,(json.dumps(paid),'paid'),code)
    both="UPDATE preparation_turns SET body=CASE WHEN id='zero' THEN ? ELSE ? END WHERE id IN ('zero','paid')"
    corrupt('reader-cost-sum-overflow',both,(json.dumps({**zero,'usage':{**zero['usage'],'provider_reported_cost_usd':1.7e308}}),json.dumps({**paid,'usage':{**paid['usage'],'provider_reported_cost_usd':1.7e308}})),both,(json.dumps(zero),json.dumps(paid)),'preparation-budget-amount-invalid')
    corrupt('reader-row-identity',update,(json.dumps({**paid,'id':'other'}),'paid'),update,(json.dumps(paid),'paid'),'preparation-budget-turn-row-mismatch')
    corrupt('reader-malformed-turn',update,('{','paid'),update,(json.dumps(paid),'paid'),'preparation-budget-turn-invalid')
    corrupt('reader-negative-commitment','UPDATE preparation_reservations SET amount=? WHERE turn_id=?',(-0.5,'unknown'),'UPDATE preparation_reservations SET amount=? WHERE turn_id=?',(0.5,'unknown'),'preparation-budget-amount-invalid')
    linked=f.ok(['prepare','create'],dict(version=1,action='create',event_id='reader-linked',expected_revision=0,work_id=f.work,title='Linked budget',text='Need'))
    linked_view=f.ok(['prepare','budget',linked['id']]);work_view=f.ok(['budget','show',f.work]);check('reader-shared-work-view',linked_view['work_budget']==work_view['budget'])
    server=native.Server(f.binary,f.root,out,'preparation-budget-read',environment=f.environment);server.capture_async(out/'observer.raw');primary=None
    try:
        before=dump();http=server.request('/api/v1/preparations/budget?id='+p['id'],observe=False);check('reader-http-cli-equal',http==read() and before==dump());server.request('/api/v1/preparations/budget',status=400,observe=False);check('reader-invalid-query-no-write',before==dump())
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    check('reader-no-provider-execution',not (f.root/'model-received.json').exists());print('PREPARATION_BUDGET_READ_PASS '+str(len(checks)));print('PREPARATION_BUDGET_READ_EVIDENCE '+str(out))
if __name__=='__main__':main()
