#!/usr/bin/env python3
"""Exercise native planning input readiness without granting a process start.

Public mutations create claims. Readiness must be pure; native entry availability
is not execution/identity authority. Reject stale fences/budgets/sources without private SQL edits.
"""
import argparse,sqlite3,traceback
from pathlib import Path
import rust_agent_model_route as fixture
from rust_planning_activation import configured,claim_request,apply_policy
from rust_planning_call_inputs import bindings
from rust_planning_budget import budget,ledger


def preview_request(work,claim):
    """Declare current fences; matching declarations alone establish no identity."""
    scope=next(s for s in work['planning']['scopes'] if s['id']=='root')
    return dict(schema_version=1,expected_revision=work['revision'],claim_id=claim,scope='root',scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'])


def observe(owned,value,expected=None):
    """Require pure success/refusal with original command output retained."""
    before=owned.snapshot();money=ledger(owned);inputs=bindings(owned);settings=(owned.root/'.swarm/engine-settings.json').read_bytes();code,result=owned.call(['planning','call-preview',owned.work],value)
    if expected is None:
        assert code==0,result;assert result['prepared_inputs_verified'] and result['process_started'] is False and result['execution_available'] is True
    else:
        assert code!=0 and result['failure']['code']==expected,result
        assert 'PRIVATE' not in str(result)
    assert owned.snapshot()==before and ledger(owned)==money and bindings(owned)==inputs and (owned.root/'.swarm/engine-settings.json').read_bytes()==settings
    assert not (owned.root/'model-received.json').exists();return result


def binding_schema(owned):
    """Observe auxiliary schema on a read-only owned fixture, without creation.

    Read and close errors retain both causes; absence is returned as an exact
    observed row list for legacy refusal proof, never as successful readiness.
    """
    db=sqlite3.connect('file:'+str(owned.root/'.swarm/state.db')+'?mode=ro',uri=True);primary=None;rows=None
    try:rows=db.execute("SELECT type,name,sql FROM sqlite_schema WHERE tbl_name='native_planning_claims' ORDER BY type,name").fetchall()
    except BaseException as e:primary=e
    try:db.close()
    except BaseException as e:
        if primary:raise BaseExceptionGroup('Schema observation and close failed',[primary,e])
        raise
    if primary:raise primary
    return rows


def legacy_refusal(binary,legacy,out):
    """Create a real pre-binding claim, then require pure native refusal.

    No private SQL seed, fabricated migration or missing-table fallback is used.
    A healthy current-generation claim was verified by the main caller first.
    """
    owned=fixture.Fixture(legacy,out,'legacy-preview');enabled=configured(owned,'old-preview');claimed=owned.ok(['planning','claim',owned.work],claim_request(enabled,'old-preview-claim'));assert binding_schema(owned)==[]
    owned.binary=binary;before=owned.snapshot();money=ledger(owned);settings=(owned.root/'.swarm/engine-settings.json').read_bytes();owned.denied(['planning','call-preview',owned.work],preview_request(claimed,'old-preview-claim'),'planning-call-binding-required')
    assert owned.snapshot()==before and ledger(owned)==money and binding_schema(owned)==[] and (owned.root/'.swarm/engine-settings.json').read_bytes()==settings
    assert not (owned.root/'model-received.json').exists()


def main():
    """Read current owned claim, renew without replacing evidence, reject drift."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--legacy-binary',type=Path);a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(x):fixture.digest(x.read_bytes()) for x in [Path(__file__),Path('tests/rust_planning_activation.py'),Path('tests/rust_planning_call_inputs.py'),Path('tests/rust_planning_budget.py'),Path('tests/rust_planning_decision.py'),Path('tests/rust_planning_bootstrap.py'),Path('tests/rust_agent_model_route.py')]}))
    try:
        owned=fixture.Fixture(binary,out,'call-preview');enabled=configured(owned,'preview');claimed=owned.ok(['planning','claim',owned.work],claim_request(enabled,'preview-claim'));value=preview_request(claimed,'preview-claim');summary=observe(owned,value);scope=next(s for s in claimed['planning']['scopes'] if s['id']=='root');assert summary['context_sha256']==scope['delivery']['context_sha256']
        for label,changes,expected in [('unknown-field',{'PRIVATE_UNTRUSTED':'PRIVATE value'},'planning-call-request-format'),('schema',dict(schema_version=99),'planning-call-request-invalid'),('holder',dict(holder='other-holder'),'stale_lease'),('generation',dict(generation=99),'stale_lease'),('revision',dict(expected_revision=99),'revision_conflict'),('claim',dict(claim_id='unknown-claim'),'planning-call-binding-required')]:
            observe(owned,value);observe(owned,{**value,**changes},expected)
        renewed=owned.ok(['planning','lease-renew',owned.work],dict(schema_version=1,event_id='preview-renew',expected_revision=claimed['revision'],scope='root',scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'],lease_seconds=120));value=preview_request(renewed,'preview-claim');summary=observe(owned,value);assert summary['original_claim_revision']==claimed['revision'] and summary['work_revision']==renewed['revision']
        # Reobserve a valid changed provider, then recover original exact source.
        observe(owned,value);original=owned.providers.read_bytes();changed_provider=__import__('json').loads(original);changed_provider['providers']['fixture']['args'].append('--changed-preview-source');fixture.publish(owned.providers,changed_provider);observe(owned,value,'planning-claim-provider-changed');fixture.publish(owned.providers,original);observe(owned,value)
        # A valid project workflow profile changes the frozen source, not the lease.
        rules=owned.root/'AGENTS.md';profile=owned.root/'swarm.project.json';assert not rules.exists() and not profile.exists();observe(owned,value);fixture.publish(rules,b'Updated local planning contract.\n');fixture.publish(profile,dict(version=1,name='Owned preview profile',source='custom',instructions=[dict(path='AGENTS.md',roles=['planner'])]));current=owned.ok(['agent-workflow','planner']);assert current['workflow']['sha256']!=summary['workflow_sha256'];observe(owned,value,'planning-call-workflow-changed');profile.unlink();rules.unlink();observe(owned,value)
        for key in ['planning_context_max_events','planning_context_max_bytes']:
            observe(owned,value);saved=owned.ok(['engine-settings','show'])['values'][key];apply_policy(owned,'preview-disable-'+key.replace('_','-'),{key:'0'});observe(owned,value,'planning-call-current-context-policy');apply_policy(owned,'preview-restore-'+key.replace('_','-'),{key:saved});observe(owned,value)
        if a.legacy_binary:
            legacy=a.legacy_binary.resolve();fixture.publish(out/'legacy-binary.json',dict(path=str(legacy),sha256=fixture.digest(legacy.read_bytes())));legacy_refusal(binary,legacy,out)
        changed=budget(owned,'preview-finite-policy',2);value=preview_request(changed,'preview-claim');observe(owned,value,'budget-existing-planning-estimate-changed')
        fixture.publish(out/'result.json',dict(status='PASS',preview_pure=True,fences_refused=True,current_saved_context_caps_refused_and_recovered=True,legacy_missing_binding_refused_without_schema_creation=bool(a.legacy_binary),source_drift_refused_and_recovered=True,renewal_preserves_original_binding=True,current_budget_change_refused=True,provider_started=False,scope='Input readiness only; no executable process or authenticated agent permission'))
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
