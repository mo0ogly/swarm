#!/usr/bin/env python3
"""Compare actual Go/native price DTOs, catalogue defaults and explicit quotes.

Only private Store data and pure native fixtures; no live root, provider,
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
    """Finite sourced-contract fixtures, with valid witnesses around refusals.

    Numeric values and counts are synthetic data, never bundled provider prices.
    Explicit expected outcomes stop a common refusal from masquerading as parity.
    """
    rate=dict(version='rate-fixture',provider='fixture',model='example-v1',currency='USD',input_per_million=2,output_per_million=5,cache_read_per_million=None,cache_write_per_million=None,source='Synthetic <&> café \u2028',reference_date='2024-02-29',actor='fixture',updated='2026-10-19T00:00:00Z')
    catalogue=dict(schema_version=1,digest='UNTRUSTED',rates=[rate])
    request=dict(schema_version=1,expected_digest='fixture',rate=rate)
    base=dict(mode='change',request=request,catalogue=catalogue)
    cases=[]
    def add(name,frame,expected='ok'):
        """Retain independent frames and their explicit semantic expectation."""
        cases.append(dict(name=name,frame=copy.deepcopy(frame),expected=expected))
    add('valid-change',base)
    for name,amount in [('zero',0),('tiny',1e-7),('decimal',0.1),('large',1e10),('largest-finite',1.7976931348623157e308)]:
        value=copy.deepcopy(base);value['request']['rate']['input_per_million']=amount;add(name,value)
    for name in ['negative','source-space','source-too-long','date','model','model-too-long','provider','currency','all-unknown','schema','unknown','rate-unknown','rate-type']:
        add(name+'-witness',base);value=copy.deepcopy(base);r=value['request']['rate']
        if name=='negative':r['input_per_million']=-1
        if name=='source-space':r['source']='\u00a0'
        if name=='source-too-long':r['source']='x'*2001
        if name=='date':r['reference_date']='2023-02-29'
        if name=='model':r['model']='invalid model'
        if name=='model-too-long':r['model']='x'*161
        if name=='provider':r['provider']='invalid provider'
        if name=='currency':r['currency']='usd'
        if name=='all-unknown':
            for key in ['input_per_million','output_per_million','cache_read_per_million','cache_write_per_million']:r[key]=None
        if name=='schema':value['request']['schema_version']=0
        if name=='unknown':value['request']['PRIVATE_unknown']=True
        if name=='rate-unknown':r['PRIVATE_unknown']=True
        if name=='rate-type':r['input_per_million']='PRIVATE'
        add(name,value,'refused');add(name+'-recovery',base)
    quote=dict(mode='quote',request=dict(rate_version='rate-fixture',non_cached_input_tokens=1000000,output_tokens=1000000,cache_read_tokens=0,cache_write_tokens=0),catalogue=catalogue)
    add('quote-known',quote)
    value=copy.deepcopy(quote);value['request'].update(non_cached_input_tokens=3,output_tokens=0);value['catalogue']['rates'][0]['input_per_million']=0.01;add('quote-legacy-finite-rounding',value)
    value=copy.deepcopy(quote);value['request'].update(non_cached_input_tokens=1000000,output_tokens=0);value['catalogue']['rates'][0]['input_per_million']=1.7976931348623157e308;add('quote-overflowing-product-finite-result',value)
    value=copy.deepcopy(quote);value['request']['cache_read_tokens']=1;add('quote-missing-cache',value)
    value['catalogue']['rates'][0]['cache_read_per_million']=0;add('quote-explicit-free-cache',value)
    value=copy.deepcopy(quote);value['request']['non_cached_input_tokens']=1000000000001;value['catalogue']['rates'][0]['input_per_million']=1e10;add('quote-no-business-ceilings',value)
    value=copy.deepcopy(quote)
    for key in ['non_cached_input_tokens','output_tokens','cache_read_tokens','cache_write_tokens']:value['request'][key]=0
    add('quote-zero-volumes',value)
    for name in ['negative-input','negative-output','negative-cache','missing-input','missing-output','wrong-version','overflow','wrong-token-type','unknown-quote-key']:
        add(name+'-witness',quote);value=copy.deepcopy(quote);q=value['request']
        if name=='negative-input':q['non_cached_input_tokens']=-1
        if name=='negative-output':q['output_tokens']=-1
        if name=='negative-cache':q['cache_read_tokens']=-1
        if name=='missing-input':q.pop('non_cached_input_tokens')
        if name=='missing-output':q['output_tokens']=None
        if name=='wrong-version':q['rate_version']='unknown'
        if name=='overflow':q['non_cached_input_tokens']=9223372036854775807;value['catalogue']['rates'][0]['input_per_million']=1.7976931348623157e308
        if name=='wrong-token-type':q['output_tokens']=1.5
        if name=='unknown-quote-key':q['PRIVATE_unknown']=True
        add(name,value,'refused');add(name+'-recovery',quote)
    for name,retained in [('retained',catalogue),('empty-initial',{}),('null-initial',None),('schema-null',dict(schema_version=None)),('rates-null',dict(schema_version=None,rates=None)),('empty-rates',dict(schema_version=1,rates=[]))]:
        add('catalogue-'+name,dict(mode='catalogue',request={},catalogue=retained))
    for name in ['negative-stored','stored-schema','stored-unknown','stored-source']:
        add(name+'-witness',dict(mode='catalogue',request={},catalogue=catalogue));value=copy.deepcopy(catalogue)
        if name=='negative-stored':value['rates'][0]['input_per_million']=-1
        if name=='stored-schema':value['schema_version']=2
        if name=='stored-unknown':value['PRIVATE_unknown']=True
        if name=='stored-source':value['rates'][0]['source']=''
        add(name,dict(mode='catalogue',request={},catalogue=value),'refused')
        add(name+'-recovery',dict(mode='catalogue',request={},catalogue=catalogue))
    for name,request in [('null',None),('empty',{}),('zero',quote['request']),('negative',dict(non_cached_input_tokens=-1,output_tokens=0))]:
        add('codec-'+name,dict(mode='quote-codec',request=request,catalogue=catalogue))
    return cases


def verify(output):
    """Build separated probes/production, compare full observations and intent bytes.

    Fixture setup/output failures are nonzero, not expected refusals. All paths are
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
    run('fixture-build',['cargo','build',*cargo,'-p','swarm-engine','--features','test-fixtures','--example','pricing_probe'])
    probe=output/'native-pricing-fixture';shutil.copyfile(ROOT/'rust/target/r25/debug/examples/pricing_probe',probe);probe.chmod(0o700)
    run('production-build',['cargo','build',*cargo,'-p','swarm-cli','--bin','swarm-rust'])
    production=output/'native-production-cli';shutil.copyfile(ROOT/'rust/target/r25/debug/swarm-rust',production);production.chmod(0o700)
    tree=run('production-features',['cargo','tree',*cargo,'-p','swarm-cli','--format','{p} features=[{f}]','--edges','normal'])
    if b'test-fixtures' in tree:raise ValueError('production-includes-fixtures')
    go=output/'go-planning';run('go-build',['go','test','-c','-o',go,'./internal/engine'])
    private=output/'fixture';private.mkdir(mode=0o700);fresh(private/'.rust-migration-fixture',b'Owned pricing fixture\n')
    cases=frames();fresh(output/'cases.json',cases)
    frame=output/'go-frame.json';observations=output/'go-observations.json';fresh(frame,('{\"Root\":'+json.dumps(str(private))+',\"Output\":'+json.dumps(str(observations))+',\"Frames\":['+','.join(json.dumps(c['frame'])for c in cases)+']}').encode())
    raw=run('go-observe',[go,'-test.run','^TestRustMigrationPricingProbe$','-test.v','-test.timeout=0'],extra={'SWARM_PRICING_PROBE':str(frame)})
    if b'--- PASS: TestRustMigrationPricingProbe' not in raw or b'SKIP' in raw:raise ValueError('go-observer-not-executed')
    requests=b''.join((json.dumps(c['frame'])+'\n').encode()for c in cases);fresh(output/'native-frames.jsonl',requests)
    native=[json.loads(line)for line in run('native-observe',[probe],requests).splitlines()];expected=json.loads(observations.read_bytes())
    if len(native)!=len(cases)or len(expected)!=len(cases):raise ValueError('observation-count-mismatch')
    for case,go_value,native_value in zip(cases,expected,native):
        if go_value!=native_value:raise ValueError(case['name']+': Go/native observation differs')
    for case,observed in zip(cases,native):
        if observed['code']!=case['expected']:raise ValueError(case['name']+': expectation not exercised')
    by_name=dict(zip([c['name']for c in cases],native))
    if json.loads(by_name['quote-known']['intent'])['estimated_amount']!=7:raise ValueError('known-quote-missing')
    if json.loads(by_name['quote-legacy-finite-rounding']['intent'])['estimated_amount']!=3e-8:raise ValueError('legacy-rounding-changed')
    if json.loads(by_name['quote-overflowing-product-finite-result']['intent'])['estimated_amount']!=1.7976931348623157e308:raise ValueError('avoidable-overflow-not-rescaled')
    if json.loads(by_name['quote-missing-cache']['intent'])['estimated_amount'] is not None:raise ValueError('unknown-became-free')
    hashes={p.name:hashlib.sha256(p.read_bytes()).hexdigest()for p in (go,probe,production)}
    fresh(output/'result.json',dict(status='PASS',count=len(cases),checks=[dict(name=c['name'],observation=v)for c,v in zip(cases,native)],binaries=hashes,commands=commands,scope='Price DTO/catalogue/quote fixtures, not HTTP, authenticated actor, provider billing, reservation or deployment'))
    print('PASS',len(cases),'Go/native pricing comparisons',flush=True)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args();verify(args.output.resolve())
