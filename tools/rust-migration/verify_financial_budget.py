#!/usr/bin/env python3
"""Compare actual Go/native financial codec, four ledgers and planning reservation.

Only private memory databases and explicit CLI fixtures; no live root, provider,
spending, identity or complete claim. Fresh evidence, original output, no watchdog.
"""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import shutil

from recorded_process import capture
from verify_planning_budgets import fresh

ROOT=Path(__file__).resolve().parents[2]


def frames():
    """Return finite named contract cases, including valid witnesses before faults.

    Limits/amounts/states here are test data, never installation/operator defaults.
    Identity and actual costs are outside the SQL numeric-admission observation.
    """
    base=dict(request=dict(schema_version=1,event_id='budget',expected_revision=2,budget=dict(limit_usd=5,reserve_per_launch_usd=1,estimate_source='Fixture <&> café',reference_date='2024-02-29',actor='FORGED',updated='FORGED')),reserve=True)
    cases=[]
    def add(name,value):
        """Keep complete independent frame copies in original observable order."""
        cases.append(dict(name=name,frame=copy.deepcopy(value)))
    add('base',base)
    for name,limit,reserve in [('integer',10,2),('decimal',0.3,0.1),('tiny',1e-7,1e-8),('large',1e21,1e20),('zero-unlimited',0,0)]:
        value=copy.deepcopy(base);value['request']['budget'].update(limit_usd=limit,reserve_per_launch_usd=reserve);add(name,value)
    value=copy.deepcopy(base);value['absent']=True;add('absent-policy',value)
    for channel in ('agents','assist','prep','planning'):
        for amount in (3,4,5):
            add(channel+str(amount)+'-witness',base)
            value=copy.deepcopy(base);value[channel]=[dict(amount=amount,state='finished' if channel=='planning' else 'reserved')];add(channel+str(amount),value)
        value=copy.deepcopy(base);value[channel]=[dict(amount=100,state='released')];add(channel+'-released',value)
    value=copy.deepcopy(base)
    for channel in ('agents','assist','prep','planning'):value[channel]=[dict(amount=1,state='finished' if channel=='planning' else 'estimated')]
    add('four-ledgers-exact',value)
    value=copy.deepcopy(base);value['request']['budget'].update(limit_usd=0.3,reserve_per_launch_usd=0.1);value['agents']=[dict(amount=0.2,state='reserved')];add('binary-float-no-epsilon',value)
    for name in ('negative','missing-source','unicode-space','bad-date','zero-reserve','wrong-schema','wrong-revision','unknown','unknown-budget','wrong-type'):
        add(name+'-witness',base);value=copy.deepcopy(base)
        if name=='negative':value['request']['budget']['limit_usd']=-1
        if name=='missing-source':value['request']['budget']['estimate_source']=''
        if name=='unicode-space':value['request']['budget']['estimate_source']='\u00a0'
        if name=='bad-date':value['request']['budget']['reference_date']='2023-02-29'
        if name=='zero-reserve':value['request']['budget']['reserve_per_launch_usd']=0
        if name=='wrong-schema':value['request']['schema_version']=0
        if name=='wrong-revision':value['request']['expected_revision']=0
        if name=='unknown':value['request']['unknown']=True
        if name=='unknown-budget':value['request']['budget']['unknown']=True
        if name=='wrong-type':value['request']['budget']['limit_usd']='PRIVATE'
        add(name,value);add(name+'-recovery',base)
    return cases


def verify(output):
    """Build separated probes/production, compare full observations and intent bytes.

    SQL setup/output failures are nonzero, not expected refusals. All paths are
    private developer fixtures. Result hashes bind actual binaries, not deployment.
    """
    output.mkdir(mode=0o700)
    env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'rust/target/r25'),CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',RUSTDOCFLAGS='-D warnings')
    cargo=['--manifest-path','rust/Cargo.toml','--locked','--offline'];commands=[]
    def run(label,args,data=None,extra=None):
        """Retain complete command/output/cause without retry or an inherited timeout."""
        args=list(map(str,args));result=capture(args,cwd=ROOT,env=dict(env,**(extra or {})),data=data,stdout_path=output/(label+'.stdout'),stderr_path=output/(label+'.stderr'))
        record=dict(label=label,args=args,exit=result.returncode);commands.append(record);fresh(output/(label+'.command.json'),record);print(label,result.returncode,flush=True)
        if result.returncode:raise RuntimeError(label+': failed; original outputs retained')
        return result.stdout
    fresh(output/'harness.py',Path(__file__).read_bytes())
    run('fixture-build',['cargo','build',*cargo,'-p','swarm-engine','--features','test-fixtures','--example','financial_budget_probe'])
    probe=output/'native-financial-fixture';shutil.copyfile(ROOT/'rust/target/r25/debug/examples/financial_budget_probe',probe);probe.chmod(0o700)
    run('production-build',['cargo','build',*cargo,'-p','swarm-cli','--bin','swarm-rust'])
    production=output/'native-production-cli';shutil.copyfile(ROOT/'rust/target/r25/debug/swarm-rust',production);production.chmod(0o700)
    tree=run('production-features',['cargo','tree',*cargo,'-p','swarm-cli','--format','{p} features=[{f}]','--edges','normal'])
    if b'test-fixtures' in tree:raise ValueError('production-includes-fixtures')
    go=output/'go-planning';run('go-build',['go','test','-c','-o',go,'./internal/engine'])
    private=output/'fixture';private.mkdir(mode=0o700);fresh(private/'.rust-migration-fixture',b'Owned financial fixture\n')
    cases=frames();fresh(output/'cases.json',cases)
    frame=output/'go-frame.json';observations=output/'go-observations.json';fresh(frame,dict(Root=str(private),Output=str(observations),Frames=[c['frame']for c in cases]))
    raw=run('go-observe',[go,'-test.run','^TestRustMigrationFinancialBudgetProbe$','-test.v','-test.timeout=0'],extra={'SWARM_FINANCIAL_BUDGET_PROBE':str(frame)})
    if b'--- PASS: TestRustMigrationFinancialBudgetProbe' not in raw or b'SKIP' in raw:raise ValueError('go-observer-not-executed')
    requests=b''.join((json.dumps(c['frame'])+'\n').encode()for c in cases);fresh(output/'native-frames.jsonl',requests)
    native=[json.loads(line)for line in run('native-observe',[probe],requests).splitlines()];expected=json.loads(observations.read_bytes())
    if len(native)!=len(cases)or len(expected)!=len(cases):raise ValueError('observation-count-mismatch')
    for case,go_value,native_value in zip(cases,expected,native):
        if go_value!=native_value:raise ValueError(case['name']+': Go/native observation differs')
    assert expected[0]['code']=='ok'and expected[0]['planning_count']==1
    assert dict(zip([c['name']for c in cases],native))['binary-float-no-epsilon']['code']=='refused'
    hashes={p.name:hashlib.sha256(p.read_bytes()).hexdigest()for p in (go,probe,production)}
    fresh(output/'result.json',dict(status='PASS',count=len(cases),checks=[dict(name=c['name'],observation=v)for c,v in zip(cases,native)],binaries=hashes,commands=commands,scope='Codec and SQL estimated admission/reservation fixtures, not authenticated claim or actual spending'))
    print('PASS',len(cases),'Go/native financial comparisons',flush=True)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args();verify(args.output.resolve())
