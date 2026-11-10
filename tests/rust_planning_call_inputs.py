#!/usr/bin/env python3
"""Observe immutable native claim inputs through public CLI and read-only SQL.

No fixture SQL mutation, model call, call-start permission or watchdog is used.
Original commands and failed Roots remain retained for replay and diagnosis.
"""
import argparse,json,sqlite3,traceback
from pathlib import Path
import rust_agent_model_route as fixture
from rust_planning_activation import configured,claim_request
from rust_planning_decision import decision


def bindings(owned):
    """Read every complete owned input binding; preserve read and close causes."""
    db=sqlite3.connect('file:'+str(owned.root/'.swarm/state.db')+'?mode=ro',uri=True);db.row_factory=sqlite3.Row;primary=None;rows=None
    try:rows=[dict(r) for r in db.execute('SELECT * FROM native_planning_claims WHERE work_id=? ORDER BY generation',(owned.work,))]
    except BaseException as e:primary=e
    try:db.close()
    except BaseException as e:
        if primary:raise BaseExceptionGroup('Binding read and close failed',[primary,e])
        raise
    if primary:raise primary
    return rows


def main():
    """Require exact frozen bytes/correlation and append-only history on real claims."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(x):fixture.digest(x.read_bytes()) for x in [Path(__file__),Path('tests/rust_planning_activation.py'),Path('tests/rust_planning_decision.py'),Path('tests/rust_planning_bootstrap.py'),Path('tests/rust_agent_model_route.py')]}))
    try:
        owned=fixture.Fixture(binary,out,'claim-inputs');enabled=configured(owned,'claim-inputs');request=claim_request(enabled,'input-claim');claimed=owned.ok(['planning','claim',owned.work],request);scope=next(s for s in claimed['planning']['scopes'] if s['id']=='root');rows=bindings(owned);assert len(rows)==1;row=rows[0]
        assert (row['claim_id'],row['work_id'],row['scope_id'],row['generation'],row['scope_revision'],row['claim_revision'],row['holder'])==('input-claim',owned.work,'root',1,1,claimed['revision'],'fixture-planner')
        context=json.loads(row['context_json']);assert context['work']==owned.work and context['scope']['id']=='root' and context['scope']['revision']==1;assert [e['id'] for e in context['events']]==scope['delivery']['events'];assert fixture.digest(row['context_json'].encode())==row['context_sha256']==scope['delivery']['context_sha256'];assert fixture.digest(row['workflow_prompt'].encode())==row['workflow_prompt_sha256'];assert json.loads(row['workflow_json'])==scope['workflow'];assert json.loads(row['model_json'])==scope['last_model']
        renewed=owned.ok(['planning','lease-renew',owned.work],dict(schema_version=1,event_id='input-renew',expected_revision=claimed['revision'],scope='root',scope_revision=1,holder=scope['holder'],generation=1,lease_seconds=120));assert bindings(owned)==rows;assert owned.ok(['planning','claim',owned.work],request)==renewed and bindings(owned)==rows
        ack=owned.ok(['planning','decide',owned.work],decision(renewed,'input-ack'));resumed=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='input-resume',expected_revision=ack['revision'],reason='Observe next immutable activation'));scope=next(s for s in resumed['planning']['scopes'] if s['id']=='root');next_work=owned.ok(['planning','claim',owned.work],claim_request(resumed,'input-second-claim',scope_revision=scope['revision']));after=bindings(owned);assert len(after)==2 and after[0]==row;assert after[1]['generation']>row['generation'];assert after[1]['claim_id']=='input-second-claim';assert [e['id'] for e in json.loads(after[1]['context_json'])['events']]==['input-resume'];assert next_work['planning']['activations']==2;assert not (owned.root/'model-received.json').exists()
        fixture.publish(out/'result.json',dict(status='PASS',exact_context_workflow_model=True,renewal_and_replay_preserve_binding=True,next_claim_appends_without_overwrite=True,provider_started=False,scope='Immutable local evidence binding; no process identity or execution permission'))
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
