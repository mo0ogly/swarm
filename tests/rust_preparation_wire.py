#!/usr/bin/env python3
"""Replay nested Go JSON semantics and old canonical receipts on an owned Root.

Synthetic request formatting in the retained fixture is not a real Go execution.
Valid controls precede duplicate/null/alias cases; no account or live database.
"""
import argparse,json,sqlite3,hashlib
from pathlib import Path
import rust_agent_model_route as fixture

def main():
    """Require no accidental unlimited budget and semantic receipt replay."""
    parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);parser.add_argument('--expect-defects',action='store_true');a=parser.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'preparation-wire');f.settings_change(preparation_write_enabled='1');checks=[]
    def check(name,value):
        """Retain the exact observed case without reducing failures to empty output."""
        assert value,name;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def create(name):return f.ok(['prepare','create'],dict(version=1,action='create',event_id=name,expected_revision=0,text='Need',title='Wire fixture'))
    def raw_command(p,event,tail):
        """Keep raw nested member order/duplicates; the fixture JSON writer sorts maps."""
        value=dict(version=1,action='budget',preparation_id=p['id'],event_id=event,expected_revision=p['revision']);raw=json.dumps(value)[:-1]+','+tail+'}';file=out/(event+'.wire.json');fixture.publish(file,raw.encode());return f.call(['prepare','budget',p['id'],'--input',str(file)])
    source='"reserve_per_launch_usd":1,"estimate_source":"fixture","reference_date":"2026-10-26"'
    p=create('wire-control');status,control=raw_command(p,'control','"budget":{"limit_usd":10,'+source+'}');check('wire-valid-control',status==0 and control['budget']['limit_usd']==10)
    p=create('wire-null');status,value=raw_command(p,'null','"budget":{"limit_usd":10,"limit_usd":null,'+source+'}');check('wire-null-defect-observed' if a.expect_defects else 'wire-null-preserves-finite-limit',status==0 and value['budget']['limit_usd']==(0 if a.expect_defects else 10))
    p=create('wire-alias');status,value=raw_command(p,'alias','"budget":{"limit_usd":10,"LIMIT_USD":20,'+source+'}');check('wire-alias-defect-observed' if a.expect_defects else 'wire-alias-last-member-wins',status==0 and value['budget']['limit_usd']==(10 if a.expect_defects else 20))
    p=create('wire-merge');status,value=raw_command(p,'merge','"budget":{"limit_usd":10,'+source+'},"BUDGET":{"limit_usd":20}');check('wire-object-merge-defect-observed' if a.expect_defects else 'wire-object-members-merge',status!=0 if a.expect_defects else status==0 and value['budget']['limit_usd']==20 and value['budget']['reserve_per_launch_usd']==1)
    p=create('wire-replay');request=dict(version=1,action='budget',preparation_id=p['id'],event_id='retained',expected_revision=p['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=1,estimate_source='fixture',reference_date='2026-10-26'));saved=f.ok(['prepare','budget',p['id']],request)
    # Go-shaped canonical Budget has six fields, even if input omitted metadata.
    go_shaped={**request,'budget':{**request['budget'],'updated':'','actor':''}}
    with sqlite3.connect(f.root/'.swarm/state.db') as db:db.execute('UPDATE preparation_commands SET request=? WHERE preparation_id=? AND event_id=?',(json.dumps(go_shaped,separators=(',',':')),p['id'],'retained'))
    def dump():
        with sqlite3.connect(f.root/'.swarm/state.db') as db:return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
    before=dump();status,value=f.call(['prepare','budget',p['id']],request);check('wire-receipt-order-defect-observed' if a.expect_defects else 'wire-receipt-semantic-replay',status!=0 if a.expect_defects else status==0 and value==saved);check('wire-replay-no-sql-change',before==dump())
    print(('PREPARATION_WIRE_BASELINE_DEFECTS ' if a.expect_defects else 'PREPARATION_WIRE_PASS ')+str(len(checks)))
if __name__=='__main__':main()
