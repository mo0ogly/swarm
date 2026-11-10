#!/usr/bin/env python3
"""Replay an actual older scoped context and its explicit public replacement.

Requires the retained pre124 normal binary as --legacy-binary, rather than
editing persisted evidence to fabricate an old version. Only owned inactive
Roots, public mutations and read-only binding/ledger observations are used.
No provider process, private SQL mutation or temporal watchdog is installed.
"""
import argparse,json,traceback
from pathlib import Path
import rust_agent_model_route as fixture
from rust_planning_activation import configured,claim_request
from rust_planning_decision import decision
from rust_planning_call_preview import preview_request,observe
from rust_planning_call_inputs import bindings
from rust_planning_budget import ledger


def main():
    """Verify legacy healthy control, pure refusal, and a newly charged claim.

    Resume explicitly fences the old owner without rewriting original bindings
    or refunding estimates. A new claim carries the current projection. Failure
    retains the original fixture, commands, hashes and traceback for diagnosis.
    This verifies context evolution, not planner execution or review acceptance.
    """
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary',required=True,type=Path)
    p.add_argument('--legacy-binary',required=True,type=Path)
    p.add_argument('--output',required=True,type=Path)
    a=p.parse_args();binary=a.binary.resolve();legacy=a.legacy_binary.resolve()
    out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None
    sources=[Path(__file__),*(Path('tests')/n for n in ['rust_planning_activation.py','rust_planning_bootstrap.py','rust_planning_decision.py','rust_planning_call_preview.py','rust_planning_call_inputs.py','rust_planning_budget.py','rust_agent_model_route.py'])]
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),legacy_binary=str(legacy),legacy_binary_sha256=fixture.digest(legacy.read_bytes()),sources={str(f):fixture.digest(f.read_bytes()) for f in sources}))
    try:
        owned=fixture.Fixture(legacy,out,'context-evolution')
        enabled=configured(owned,'evolution')
        first=owned.ok(['planning','claim',owned.work],claim_request(enabled,'evolution-first'))
        made=owned.ok(['planning','decide',owned.work],decision(first,'evolution-task',[dict(kind='task',id='owned-task',title='Owned task',requirements=['req-1'],deliverable='Evidence',criteria=['Observable proof'],next='Observe without provider')]))
        resumed=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='evolution-old-resume',expected_revision=made['revision'],reason='Observe legacy context'))
        scope=next(s for s in resumed['planning']['scopes'] if s['id']=='root')
        claimed=owned.ok(['planning','claim',owned.work],claim_request(resumed,'evolution-old',scope_revision=scope['revision']))
        observe(owned,preview_request(claimed,'evolution-old'))
        original={row['claim_id']:row for row in bindings(owned)};money=ledger(owned)
        fields={'attempts_used','attempts_max','instruction_revisions_count','review_coordination'}
        old_task=json.loads(original['evolution-old']['context_json'])['tasks'][0]
        assert not fields.intersection(old_task),'The supplied legacy binary already has the new projection'
        owned.binary=binary
        observe(owned,preview_request(claimed,'evolution-old'),'planning-call-scope-contract-changed')
        assert {r['claim_id']:r for r in bindings(owned)}==original and ledger(owned)==money
        resumed=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='evolution-current-resume',expected_revision=claimed['revision'],reason='Replace incomplete legacy context explicitly'))
        scope=next(s for s in resumed['planning']['scopes'] if s['id']=='root')
        current=owned.ok(['planning','claim',owned.work],claim_request(resumed,'evolution-current',scope_revision=scope['revision']))
        observe(owned,preview_request(current,'evolution-current'))
        rows={r['claim_id']:r for r in bindings(owned)}
        assert all(rows[k]==v for k,v in original.items()) and len(rows)==len(original)+1
        current_task=json.loads(rows['evolution-current']['context_json'])['tasks'][0]
        assert fields.issubset(current_task)
        task=next(t for t in current['tasks'] if t['id']=='owned-task')
        assert current_task['attempts_used']==len(task.get('attempts',[]))
        assert current_task['attempts_max']==task['plan_max_attempts']
        assert current_task['instruction_revisions_count']==len(task.get('instruction_revisions',[]))
        assert current_task['review_coordination']==task.get('review_coordination')
        assert all(row in ledger(owned) for row in money) and len(ledger(owned))==len(money)+1
        assert current['planning']['activations']==claimed['planning']['activations']+1
        assert not (owned.root/'model-received.json').exists()
        fixture.publish(out/'result.json',dict(status='PASS',actual_legacy_control=True,pure_missing_projection_refusal=True,explicit_resume_and_fresh_claim=True,immutable_old_bindings_and_reservations=True,new_activation_charged=True,provider_started=False))
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
