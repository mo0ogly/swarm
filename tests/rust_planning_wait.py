#!/usr/bin/env python3
"""Replay persisted planner wait policy without starting a provider.

Only inactive owned Roots and public CLI mutations are used. Validate a good
edit before each fault, preserve exact state/history after refusal, and optionally
adopt an actual native26 document. This is not a Go provider-wait file importer,
real timer, process-control, remote cancellation or model-billing qualification.
"""
import argparse,json,traceback
from datetime import datetime
from pathlib import Path
import rust_agent_model_route as fixture
from rust_planning_activation import apply_policy,configured,claim_request
from rust_planning_call_inputs import bindings

FIELDS=['planning_provider_silence_seconds','planning_provider_max_duration_seconds','planning_default_lease_seconds','planning_lease_renew_after_percent']


def invalid(owned,event,changes,reason,remove=None):
    """Require the specific classified safe reason and refusal without mutation.

    The caller establishes a valid edit first. Broad error-family matching alone
    cannot hide a stale schema/revision prerequisite or an unrelated refusal.
    """
    doc=owned.ok(['engine-settings','show']);values=dict(doc['values']);values.update(changes)
    if remove:values.pop(remove)
    raw=(owned.root/'.swarm/engine-settings.json').read_bytes();before=owned.snapshot()
    code,response=owned.call(['engine-settings','preview'],dict(schema_version=doc['schema_version'],event_id=event,expected_revision=doc['revision'],values=values,reason='Explicit invalid wait-policy fixture',preview_token=''))
    assert (owned.root/'.swarm/engine-settings.json').read_bytes()==raw and owned.snapshot()==before
    assert code!=0 and response['failure']['code']==reason,response
    diagnostic=json.dumps(response['failure'])
    assert reason in diagnostic,('Missing safe validation reason',reason,response)
    assert 'PRIVATE_WAIT_VALUE' not in diagnostic
    return dict(case=event,reason=reason,unchanged=True)


def adoption(binary,legacy,out):
    """Use actual older native installation history, never amputated current data.

    Explicit settings preview/apply preserves every older entry and materializes
    the complete four-field group. It does not adopt Go provider-wait.json.
    """
    owned=fixture.Fixture(legacy,out,'wait-native26')
    old=owned.ok(['engine-settings','show']);assert old['schema_version']==26
    assert not set(FIELDS).intersection(old['values'])
    original=(owned.root/'.swarm/engine-settings.json').read_bytes();owned.binary=binary
    assert owned.ok(['engine-settings','show'])==old
    assert (owned.root/'.swarm/engine-settings.json').read_bytes()==original
    defaults=owned.ok(['engine-settings','describe'])['defaults']
    apply_policy(owned,'explicit-wait27-adoption',{key:defaults[key] for key in FIELDS},target_schema=27)
    new=owned.ok(['engine-settings','show']);assert new['schema_version']==27
    assert new['history'][:len(old['history'])]==old['history'] and len(new['history'])==len(old['history'])+1
    assert all(new['values'][key]==defaults[key] for key in FIELDS)


def main():
    """Persist/reopen controls, diagnose exact bad fields, preserve explicit leases.

    A configured default never replaces a missing/zero explicit CLI claim lease.
    Every invocation and binary/source hash is retained; failures keep their Root.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--legacy-binary',type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None
    sources=[Path(__file__),*(Path('tests')/n for n in ['rust_agent_model_route.py','rust_planning_activation.py','rust_planning_bootstrap.py','rust_planning_call_inputs.py','rust_planning_decision.py'])]
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(f):fixture.digest(f.read_bytes()) for f in sources}))
    try:
        owned=fixture.Fixture(binary,out,'planning-wait');doc=owned.ok(['engine-settings','show']);assert doc['schema_version']==27
        cases=[]
        faults=[('renewal',dict(planning_lease_renew_after_percent='100'),'settings-planning-wait-renewal-invalid',None),('default-zero',dict(planning_default_lease_seconds='0'),'settings-planning-wait-lease-invalid',None),('default-outside',dict(planning_default_lease_seconds='301'),'planning-lease-outside-policy',None),('missing',{},'settings-planning-wait-not-configured','planning_provider_max_duration_seconds'),('format-private',dict(planning_provider_silence_seconds='PRIVATE_WAIT_VALUE'),'settings-request-invalid',None)]
        for i,(label,changes,reason,remove) in enumerate(faults):
            apply_policy(owned,'wait-valid-'+str(i),dict(planning_lease_renew_after_percent='25' if i%2==0 else '50'))
            cases.append(invalid(owned,'wait-invalid-'+label,changes,reason,remove))
        apply_policy(owned,'wait-saved-controls',dict(planning_provider_silence_seconds='0',planning_provider_max_duration_seconds='0',planning_default_lease_seconds='8',planning_lease_renew_after_percent='25',planning_scope_lease_min_seconds='7',planning_scope_lease_max_seconds='9'))
        saved=owned.ok(['engine-settings','show']);assert [saved['values'][k] for k in FIELDS]==['0','0','8','25']
        enabled=configured(owned,'wait-explicit-claim');before=owned.snapshot()
        owned.denied(['planning','claim',owned.work],claim_request(enabled,'wait-no-implicit-lease',lease_seconds=0),'planning-lease-outside-policy');assert owned.snapshot()==before
        claimed=owned.ok(['planning','claim',owned.work],claim_request(enabled,'wait-explicit-lease',lease_seconds=9));scope=next(s for s in claimed['planning']['scopes'] if s['id']=='root')
        assert scope['holder']=='fixture-planner' and claimed['planning']['activations']==1
        binding=next(r for r in bindings(owned) if r['claim_id']=='wait-explicit-lease')
        # prepare_claim uses one authoritative instant for last_model.at and lease.
        # Binding publication happens later and must not redefine that instant.
        assert binding['claim_id']=='wait-explicit-lease'
        assert (datetime.fromisoformat(scope['lease_until'])-datetime.fromisoformat(scope['last_model']['at'])).total_seconds()==9
        assert owned.ok(['engine-settings','show'])==saved
        if a.legacy_binary:
            legacy=a.legacy_binary.resolve();fixture.publish(out/'legacy-binary.json',dict(path=str(legacy),sha256=fixture.digest(legacy.read_bytes())));adoption(binary,legacy,out)
        assert not (owned.root/'model-received.json').exists()
        fixture.publish(out/'result.json',dict(status='PASS',cases=cases,complete_policy_saved_and_reopened=True,no_implicit_explicit_claim_duration=True,native26_adoption_history_preserved=bool(a.legacy_binary),provider_started=False,runtime_waiting_verified=False,scope='Policy validation/history only; no Go wait import, timer/process/remote billing proof'))
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
