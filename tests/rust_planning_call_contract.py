#!/usr/bin/env python3
"""Reject changed owned task contracts before native planner preparation.

All mutations use public CLI on an owned inactive Root. Bound inputs remain
immutable and are observed read-only; no provider, SQL seed or watchdog is used.
"""
import argparse,traceback
from pathlib import Path
import rust_agent_model_route as fixture
from rust_planning_activation import configured,claim_request
from rust_planning_decision import decision
from rust_planning_call_preview import preview_request,observe
from rust_planning_call_inputs import bindings


def main():
    """Observe a healthy claim, edit its owned task, require pure stale refusal.

    Restoring the original task contract may restore readiness under the same
    unexpired claim; this must not rewrite inputs or reset ledger consumption.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(x):fixture.digest(x.read_bytes()) for x in [Path(__file__),Path('tests/rust_planning_call_preview.py'),Path('tests/rust_planning_decision.py'),Path('tests/rust_planning_activation.py'),Path('tests/rust_agent_model_route.py')]}))
    try:
        owned=fixture.Fixture(binary,out,'contract-preview');enabled=configured(owned,'contract-preview');claimed=owned.ok(['planning','claim',owned.work],claim_request(enabled,'contract-first-claim'))
        made=owned.ok(['planning','decide',owned.work],decision(claimed,'contract-create-task',[dict(kind='task',id='contract-task',title='Original owned contract',requirements=['req-1'],deliverable='Evidence',criteria=['Preserve requirement'],next='Observe only')]))
        made=owned.ok(['task','update',owned.work],dict(schema_version=1,event_id='contract-initial-blocker',expected_revision=made['revision'],id='contract-task',blocker='Original blocker'))['work']
        resumed=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='contract-resume',expected_revision=made['revision'],reason='Observe current owned task contract'));scope=next(s for s in resumed['planning']['scopes'] if s['id']=='root');claimed=owned.ok(['planning','claim',owned.work],claim_request(resumed,'contract-second-claim',scope_revision=scope['revision']));value=preview_request(claimed,'contract-second-claim');observe(owned,value);original=bindings(owned)
        current=claimed
        for key,original_text in [('next','Observe only'),('blocker','Original blocker')]:
            value=preview_request(current,'contract-second-claim');observe(owned,value)
            changed=owned.ok(['task','update',owned.work],dict(schema_version=1,event_id='contract-'+key+'-change',expected_revision=current['revision'],id='contract-task',**{key:'Changed owned instruction'}))['work']
            assert next(t for t in changed['tasks'] if t['id']=='contract-task')[key]=='Changed owned instruction';assert bindings(owned)==original
            observe(owned,preview_request(changed,'contract-second-claim'),'planning-call-scope-contract-changed')
            current=owned.ok(['task','update',owned.work],dict(schema_version=1,event_id='contract-'+key+'-restore',expected_revision=changed['revision'],id='contract-task',**{key:original_text}))['work'];observe(owned,preview_request(current,'contract-second-claim'));assert bindings(owned)==original
        fixture.publish(out/'result.json',dict(status='PASS',public_owned_next_and_blocker_edits_refused=True,restore_without_binding_rewrite=True,provider_started=False))
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
