#!/usr/bin/env python3
"""Verify public native provider/reviewer bootstrap on an owned root.

No private SQL seeding, model/provider process, scope claim or automatic dispatch.
The inherited fixture prepares only an inert local executable and model catalogue;
this recipe creates a separate fresh Work through the supported CLI.
"""
import argparse,json,traceback,shutil,os
from pathlib import Path
import rust_agent_model_route as m


def fresh_work(f,event):
    """Create a new public Work without tasks; no historical adoption is implied."""
    return f.ok(['work','create'],dict(schema_version=1,event_id=event,expected_revision=0,title='Public planning bootstrap',objective='Scoped fixture planning',scope='Owned local bootstrap',criteria=['Configuration is persisted without model execution']))['work']['id']


def request(event='enable',**patch):
    """An explicit operator request with independent positive activation/review budgets."""
    value=dict(schema_version=1,event_id=event,expected_revision=1,provider='fixture',level='standard',max_activations=3,max_tasks=10,max_decisions=10,max_review_calls=2);value.update(patch);return value



def refused(f,value,expected):
    """Require the intended safe refusal with Work/events/agent/reservation intact."""
    before=f.ok(['work','show',f.work]);snapshot=f.snapshot()
    code,response=f.call(['planning','enable',f.work],value)
    assert code!=0 and response['failure']['code']==expected,(expected,response)
    assert 'PRIVATE' not in json.dumps(response)
    assert f.ok(['work','show',f.work])==before and f.snapshot()==snapshot
    return dict(code=expected,mutation=False)


def negative_cases(f):
    """Use a healthy public bootstrap before each changed single fault."""
    cases=[];source=f.providers.read_bytes();program=f.root/'codex'
    specs=[
        ('unknown-provider',dict(provider='missing-provider'),'agent-provider-not-configured',None),
        ('unsafe-name',dict(provider='../PRIVATE'),'planning-provider-name-invalid',None),
        ('negative-review-budget',dict(max_review_calls=-1),'invalid_review_budget',None),
        ('review-budget-without-provider',dict(provider=''),'invalid_review_budget',None),
        ('zero-activation-budget',dict(max_activations=0),'planning-control-invalid',None),
        ('stale-policy',dict(model_policy_hash='0'*64),'agent-model-policy-hash-changed',None),
        ('missing-catalogue',{},'agent-provider-catalogue-read','missing'),
        ('malformed-catalogue',{},'agent-provider-catalogue-invalid','malformed'),
        ('nonexecutable',{},'planning-provider-assistant-unavailable','mode'),
    ]
    for label,patch,expected,fault in specs:
        f.work=fresh_work(f,'witness-'+label);f.ok(['planning','enable',f.work],request(event='witness-plan-'+label))
        f.work=fresh_work(f,'refusal-'+label);mode=program.stat().st_mode&0o777
        try:
            if fault=='missing':f.providers.unlink()
            if fault=='malformed':m.publish(f.providers,b'PRIVATE-CATALOGUE malformed')
            if fault=='mode':os.chmod(program,0o600)
            cases.append(dict(case=label,**refused(f,request(event='bad-'+label,**patch),expected)))
        finally:
            if fault in ['missing','malformed']:m.publish(f.providers,source)
            if fault=='mode':os.chmod(program,mode)
        assert not (f.root/'model-received.json').exists()
    return cases


def legacy_adoption(binary,legacy,output):
    """A real v22 root refuses defaults adoption until the public preview/apply.

    No runtime settings file is rewritten directly. Preserve all old choices and
    history, then retry the same refused Work event with explicit new defaults.
    """
    f=m.Fixture(legacy,output,'legacy');f.binary=binary
    try:
        doc=f.ok(['engine-settings','show']);assert doc['schema_version']==22
        f.work=fresh_work(f,'legacy-bootstrap-create');r=request(event='legacy-bootstrap')
        refused(f,r,'settings-reviewer-creation-not-configured')
        defaults=f.ok(['engine-settings','describe'])['defaults'];values=dict(doc['values'])
        names=['reviewer_default_delivery_files','reviewer_default_context_files','reviewer_default_source_bytes','reviewer_default_context_bytes']
        values.update({k:defaults[k] for k in names});values['reviewer_default_context_files']='0'
        change=dict(schema_version=23,event_id='adopt-reviewer-defaults',expected_revision=doc['revision'],values=values,reason='Explicit adoption of initial reviewer configuration',preview_token='')
        previous=(f.root/'.swarm/engine-settings.json').read_bytes();preview=f.ok(['engine-settings','preview'],change)
        assert (f.root/'.swarm/engine-settings.json').read_bytes()==previous
        change['preview_token']=preview['preview_token'];f.ok(['engine-settings','apply'],change)
        adopted=f.ok(['engine-settings','show']);assert adopted['schema_version']==23 and adopted['history'][:len(doc['history'])]==doc['history']
        configured=f.ok(['planning','enable',f.work],r);assert configured['planning']['reviewer']['max_context_files']==0
        assert not (f.root/'model-received.json').exists()
        m.publish(output/'legacy-adoption.json',dict(status='PASS',original_schema=22,new_schema=23,context_files=0,history_preserved=True,provider_process_started=False))
        shutil.rmtree(f.root)
    except BaseException:
        m.publish(output/'legacy-failure-root.json',dict(root=str(f.root)));raise

def run(f):
    """Read the actual saved bootstrap and replay after removing only fixture config.

    Canonical provider digest binds the source configuration; route metadata is a
    request, never authenticated model/account/price or acceptance. Reviewer source
    limits snapshot creation defaults and do not prove runtime review enforcement.
    """
    f.work=fresh_work(f,'bootstrap-create');r=request();result=f.ok(['planning','enable',f.work],r)
    planning=result['planning'];reviewer=planning['reviewer']
    assert planning['provider']=='fixture' and planning['reviewer_required'] is True
    assert planning['provider_digest']==reviewer['provider_digest'] and len(planning['provider_digest'])==64
    assert reviewer['max_calls']==2 and reviewer.get('calls',0)==0 and reviewer['authorized']
    for route in [planning['model_route'],reviewer['model_route']]:assert route['level']=='standard' and route['model']=='fixture-standard' and route['effort']=='fixture-medium'
    values=f.ok(['engine-settings','show'])['values']
    for field,setting in [('max_files','reviewer_default_delivery_files'),('max_context_files','reviewer_default_context_files'),('max_source_bytes','reviewer_default_source_bytes'),('max_context_bytes','reviewer_default_context_bytes')]:
        assert reviewer[field]==int(values[setting]),(field,reviewer,values)
    assert not (f.root/'model-received.json').exists(),'Bootstrap executed a provider'
    before=f.ok(['work','show',f.work]);source=f.providers.read_bytes();f.providers.unlink()
    replay=f.ok(['planning','enable',f.work],r);assert replay==result and f.ok(['work','show',f.work])==before
    m.publish(f.providers,source)
    # Configuration is not an execution: preserve the active provider refusal.
    cooldown=f.root/'.swarm/provider-cooldowns';cooldown.mkdir(mode=0o700)
    file=cooldown/'fixture.json';m.publish(file,dict(provider='fixture',observed_at='2026-10-20T00:00:00Z',source='fixture',signal='backoff'));saved=file.read_bytes()
    f.work=fresh_work(f,'quota-config-create')
    configured=f.ok(['planning','enable',f.work],request(event='quota-config',max_review_calls=0))
    assert configured['planning']['reviewer']['max_calls']==3
    assert file.read_bytes()==saved and not (f.root/'model-received.json').exists()
    return dict(provider='fixture',reviewer_calls=0,review_budget=2,provider_process_started=False,replay_after_source_removal=True,requested_model='fixture-standard',runtime_review_caps_enforced=False,configuration_during_cooldown=True,inherited_review_budget=3)


def main():
    """Retain source/binary identity, raw commands and original failure roots."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--legacy-binary',type=Path);a=p.parse_args();output=a.output.resolve();output.mkdir(parents=True,exist_ok=False)
    sources=[Path(__file__).resolve(),Path(m.__file__).resolve(),a.binary.resolve()];before={str(p):m.digest(p.read_bytes()) for p in sources};m.publish(output/'inputs.json',before)
    if a.legacy_binary is not None:
        before[str(a.legacy_binary.resolve())]=m.digest(a.legacy_binary.read_bytes());sources.append(a.legacy_binary.resolve());m.publish(output/'inputs.json',before)
    for p in sources[:2]:m.publish(output/('source-'+p.name),p.read_bytes())
    f=None
    try:
        f=m.Fixture(a.binary.resolve(),output,'bootstrap');result=run(f)
        result['refusals']=negative_cases(f)
        if a.legacy_binary is not None:legacy_adoption(a.binary.resolve(),a.legacy_binary.resolve(),output)
        assert before=={str(p):m.digest(p.read_bytes()) for p in sources}
        m.publish(output/'result.json',dict(status='PASS',result=result,source_stable=True,global_watchdog=None));shutil.rmtree(f.root);print('PASS public native planning bootstrap without provider execution',flush=True)
    except BaseException as error:
        m.publish(output/'result.json',dict(status='FAIL',type=type(error).__name__,cause=str(error),root=str(f.root) if f else None,source_stable=before=={str(p):m.digest(p.read_bytes()) for p in sources}));traceback.print_exc();raise

if __name__=='__main__':main()
