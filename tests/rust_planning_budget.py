#!/usr/bin/env python3
"""Replay public planning financial reservations on an owned inactive Root.

Budget policy is adopted through the CLI; SQLite is read-only for ledger proof.
No provider process, model charge, live mission, reset or global watchdog occurs.
"""
import argparse,sqlite3,traceback
from pathlib import Path
import rust_agent_model_route as fixture
from rust_planning_activation import configured,claim_request
from rust_planning_decision import decision


def ledger(owned):
    """Read exact planning-call reservations without altering private fixture SQL.

    Parameterized identifiers, original read/close causes and connection closure
    are retained; this is observation, not execution or financial settlement.
    """
    db=sqlite3.connect('file:'+str(owned.root/'.swarm/state.db')+'?mode=ro',uri=True);primary=None;rows=None
    try:rows=db.execute('SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,)).fetchall()
    except BaseException as error:primary=error
    try:db.close()
    except BaseException as error:
        if primary:raise BaseExceptionGroup('Planning ledger read and close failed',[primary,error])
        raise
    if primary:raise primary
    return rows


def budget(owned,event,limit):
    """Adopt an explicit fixture envelope without resetting consumed reservations."""
    work=owned.ok(['planning','show',owned.work])
    owned.ok(['budget','apply',owned.work],dict(schema_version=1,event_id=event,expected_revision=work['revision'],budget=dict(limit_usd=limit,reserve_per_launch_usd=1,estimate_source='Owned planning financial fixture',reference_date='2026-10-20')))
    return owned.ok(['planning','show',owned.work])


def main():
    """Require atomic reservation, exhaustion, explicit repair and idempotent replay."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(x):fixture.digest(x.read_bytes()) for x in [Path(__file__),Path('tests/rust_planning_activation.py'),Path('tests/rust_planning_decision.py'),Path('tests/rust_planning_bootstrap.py'),Path('tests/rust_agent_model_route.py')]}));owned=None
    try:
        owned=fixture.Fixture(binary,out,'planning-budget');configured(owned,'budget-plan');candidate=budget(owned,'first-envelope',1)
        value=claim_request(candidate,'budget-first-claim');first=owned.ok(['planning','claim',owned.work],value);assert first['planning']['activations']==1;first_ledger=ledger(owned);fixture.publish(out/'first-ledger.json',first_ledger)
        ack=owned.ok(['planning','decide',owned.work],decision(first,'budget-first-observation'))
        resumed=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='budget-resume',expected_revision=ack['revision'],reason='Observe next activation under the same finite envelope'));scope=next(s for s in resumed['planning']['scopes'] if s['id']=='root');value=claim_request(resumed,'budget-second-claim',scope_revision=scope['revision'])
        before=owned.ok(['work','show',owned.work]);snapshot=owned.snapshot();code,result=owned.call(['planning','claim',owned.work],value)
        assert code!=0 and result['failure']['code']=='budget_exhausted',dict(response=result,first_ledger=first_ledger,second_ledger=ledger(owned))
        assert first_ledger==[('budget-first-claim','root',1.0,'estimated')];assert owned.ok(['work','show',owned.work])==before and owned.snapshot()==snapshot and ledger(owned)==first_ledger
        repaired=budget(owned,'explicit-larger-envelope',2);assert ledger(owned)==first_ledger;value['expected_revision']=repaired['revision'];second=owned.ok(['planning','claim',owned.work],value);assert second['planning']['activations']==2
        expected=[('budget-first-claim','root',1.0,'estimated'),('budget-second-claim','root',1.0,'estimated')];assert ledger(owned)==expected;assert owned.ok(['planning','claim',owned.work],value)==second and ledger(owned)==expected
        assert not (owned.root/'model-received.json').exists();fixture.publish(out/'result.json',dict(status='PASS',finite_envelope_refusal_without_mutation=True,explicit_budget_repair_preserves_reservations=True,idempotent_replay=True,ledger=expected,provider_started=False,scope='Conservative planning estimates; no actual provider usage/invoice or autonomous launch proved'))
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
