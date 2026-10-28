#!/usr/bin/env python3
"""Replay public claimed decisions on inactive owned Roots with no model process.

Original CLI responses, source/binary identities and failed Roots are retained.
This verifies operator-controlled transactions, not agent authentication or autonomy.
"""
import argparse,json,traceback
from pathlib import Path
import rust_agent_model_route as fixture
from rust_planning_activation import configured,claim_request,apply_policy
from rust_planning_bootstrap import fresh_work,request


def start(owned,label):
    """Create/enable/claim a fresh public Work; no SQL state is injected."""
    enabled=configured(owned,label)
    return owned.ok(['planning','claim',owned.work],claim_request(enabled,label+'-claim'))


def decision(work,label,operations=None):
    """Echo the actual persisted activation fences and delivered event identifiers."""
    scope=next(s for s in work['planning']['scopes'] if s['id']=='root')
    return dict(schema_version=1,event_id=label,expected_revision=work['revision'],scope='root',scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'],input_events=scope['delivery']['events'],operations=operations or [],reason='Owned public decision witness')



def legacy_adoption(binary,legacy,out):
    """Require explicit complete v24→v25 adoption preserving old history/Work.

    A pending activation survives a settings refusal; the same uncommitted
    decision may succeed after public preview/apply. No SQL/runtime file edits.
    """
    owned=fixture.Fixture(legacy,out,'legacy-decision');candidate=start(owned,'legacy-decision');owned.binary=binary
    value=decision(candidate,'legacy-decision-ack');before=owned.ok(['work','show',owned.work]);snapshot=owned.snapshot();code,result=owned.call(['planning','decide',owned.work],value);assert code!=0 and result['failure']['code']=='settings-planning-decisions-not-configured';assert owned.ok(['work','show',owned.work])==before and owned.snapshot()==snapshot
    old=owned.ok(['engine-settings','show']);assert old['schema_version']==24;catalog=owned.ok(['engine-settings','describe']);names=[f['name'] for f in catalog['fields'] if f['introduced_schema_version']==25];assert len(names)==8
    saved=apply_policy(owned,'adopt-v25-decision-policy',{name:catalog['defaults'][name] for name in names},target_schema=25);assert saved['schema_version']==25 and saved['history'][:-1]==old['history']
    before=owned.ok(['work','show',owned.work]);snapshot=owned.snapshot();code,refusal=owned.call(['planning','decide',owned.work],{**value,'operations':[dict(kind='delegate',id='undelivered-child',title='New capability not delivered',requirements=['req-1'])]});assert code!=0 and refusal['failure']['code']=='planning-delegation-not-delivered';assert owned.ok(['work','show',owned.work])==before and owned.snapshot()==snapshot
    decided=owned.ok(['planning','decide',owned.work],value);assert decided['planning']['activations']==1 and decided['planning']['decisions']==1;assert not (owned.root/'model-received.json').exists()
    fixture.publish(out/'legacy-adoption.json',dict(status='PASS',old_version=24,new_version=25,history_preserved=True,same_event_recovered=True,provider_started=False))


def main():
    """Require public task/delegation and atomic refusal/recovery on a normal CLI."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--legacy-binary',type=Path);a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(x):fixture.digest(x.read_bytes()) for x in [Path(__file__),Path('tests/rust_planning_activation.py'),Path('tests/rust_planning_bootstrap.py'),Path('tests/rust_agent_model_route.py')]}))
    owned=None
    try:
        owned=fixture.Fixture(binary,out,'decision');claimed=start(owned,'decision')
        original_scope=next(s for s in claimed['planning']['scopes'] if s['id']=='root');renewal=dict(schema_version=1,event_id='live-renewal',expected_revision=claimed['revision'],scope='root',scope_revision=1,holder=original_scope['holder'],generation=original_scope['generation'],lease_seconds=120)
        renewed=owned.ok(['planning','lease-renew',owned.work],renewal);scope=next(s for s in renewed['planning']['scopes'] if s['id']=='root');assert renewed['planning']['activations']==1 and renewed['planning']['decisions']==0;assert {k:v for k,v in scope.items() if k!='lease_until'}=={k:v for k,v in original_scope.items() if k!='lease_until'};assert scope['lease_until']>original_scope['lease_until'];assert owned.ok(['planning','lease-renew',owned.work],renewal)==renewed;claimed=renewed
        value=decision(claimed,'task-decision',[dict(kind='task',id='controlled-worker',title='Owned worker',requirements=['req-1'],deliverable='Evidence',criteria=['Preserve requirement'],next='Observe only')])
        decided=owned.ok(['planning','decide',owned.work],value)
        task=next(t for t in decided['tasks'] if t['id']=='controlled-worker');assert task['scope_id']=='root' and task['plan_role']=='worker'
        scope=next(s for s in decided['planning']['scopes'] if s['id']=='root');assert not scope.get('holder') and scope['generation']==2 and scope['revision']==2
        assert decided['planning']['decisions']==1 and scope['delivery']['decision']=='task-decision'
        assert owned.ok(['planning','decide',owned.work],value)==decided
        before=owned.ok(['work','show',owned.work]);snapshot=owned.snapshot();code,conflict=owned.call(['planning','decide',owned.work],{**value,'reason':'Conflicting committed intent'});assert code!=0 and conflict['failure']['code']=='event_conflict';assert owned.ok(['work','show',owned.work])==before and owned.snapshot()==snapshot
        cases=[]
        # A consumed inbox differs from a policy-disabled batch; neither spends.
        before=owned.ok(['work','show',owned.work]);snapshot=owned.snapshot();code,no_pending=owned.call(['planning','claim',owned.work],{**claim_request(decided,'empty-inbox-claim'),'scope_revision':2});assert code!=0 and no_pending['failure']['code']=='planning-no-pending-events';assert owned.ok(['work','show',owned.work])==before and owned.snapshot()==snapshot
        # Every injected request fault starts with a separate valid decision.
        faults=[('holder',dict(holder='other-holder'),'stale_lease'),('generation',dict(generation=99),'stale_lease'),('scope-revision',dict(scope_revision=99),'revision_conflict'),('missing-input',dict(input_events=[]),'planning-decision-inputs-required'),('undelivered',dict(input_events=['unseen']),'planning-decision-input-not-delivered'),('duplicate',None,'planning-decision-input-not-delivered'),('empty-reason',dict(reason=' '),'planning-decision-envelope-invalid'),('unapplied-budget',dict(max_tasks=5),'planning-decision-field-not-applicable'),('unported',dict(operations=[dict(kind='close',id='root',title='No shortcut',requirements=['req-1'])]),'planning-operation-unported')]
        for label,patch,expected in faults:
            healthy=start(owned,label+'-healthy');owned.ok(['planning','decide',owned.work],decision(healthy,label+'-healthy-decision'))
            candidate=start(owned,label+'-fault');value=decision(candidate,label+'-decision');value.update(patch or dict(input_events=value['input_events']*2));before=owned.ok(['work','show',owned.work]);snapshot=owned.snapshot()
            code,result=owned.call(['planning','decide',owned.work],value);assert code!=0 and result['failure']['code']==expected,(label,result)
            assert owned.ok(['work','show',owned.work])==before and owned.snapshot()==snapshot
            owned.ok(['planning','decide',owned.work],decision(candidate,label+'-decision'));cases.append(dict(case=label,code=expected,unchanged=True,recovered=True))
        task_op=dict(kind='task',id='transaction-worker',title='Owned worker',requirements=['req-1'],deliverable='Evidence',criteria=['Evidence'])
        operation_faults=[
            ('wrong-requirement',[{**task_op,'requirements':['req-99']}],'planning-operation-requirements-invalid'),
            ('foreign-dependency',[{**task_op,'depends':['unseen-task']}],'planning-task-dependency-outside-scope'),
            ('missing-criteria',[{**task_op,'criteria':[]}],'planning-task-definition-invalid'),
            ('duplicate-task',[task_op,task_op],'planning-task-definition-invalid'),
            ('second-invalid',[task_op,{**task_op,'id':'second-worker','requirements':['req-99']}],'planning-operation-requirements-invalid'),
            ('child-budget',[dict(kind='delegate',id='over-budget-child',title='Owned child',requirements=['req-1'],max_tasks=11)],'planning-child-budget-invalid'),
        ]
        for label,operations,expected in operation_faults:
            healthy=start(owned,label+'-healthy');owned.ok(['planning','decide',owned.work],decision(healthy,label+'-healthy-decision',[task_op]))
            candidate=start(owned,label+'-fault');value=decision(candidate,label+'-decision',operations);before=owned.ok(['work','show',owned.work]);snapshot=owned.snapshot();code,result=owned.call(['planning','decide',owned.work],value)
            assert code!=0 and result['failure']['code']==expected,(label,result);assert owned.ok(['work','show',owned.work])==before and owned.snapshot()==snapshot
            owned.ok(['planning','decide',owned.work],decision(candidate,label+'-decision',[task_op]));cases.append(dict(case=label,code=expected,unchanged=True,recovered=True))
        # Delegation transfers ownership; child may create only its own worker.
        candidate=start(owned,'delegate');value=decision(candidate,'delegate-decision',[dict(kind='delegate',id='child',title='Owned child',requirements=['req-1'],next='Scope only',max_tasks=2,max_activations=2)])
        delegated=owned.ok(['planning','decide',owned.work],value);root=next(s for s in delegated['planning']['scopes'] if s['id']=='root');child=next(s for s in delegated['planning']['scopes'] if s['id']=='child');assert not root.get('requirements') and child['requirements']==['req-1'] and child['parent']=='root'
        child_claim=owned.ok(['planning','claim',owned.work],dict(schema_version=1,event_id='child-claim',expected_revision=delegated['revision'],scope='child',scope_revision=1,holder='child-holder',lease_seconds=60));child=next(s for s in child_claim['planning']['scopes'] if s['id']=='child')
        child_value=dict(schema_version=1,event_id='child-decision',expected_revision=child_claim['revision'],scope='child',scope_revision=child['revision'],holder=child['holder'],generation=child['generation'],input_events=child['delivery']['events'],reason='Own requirement',operations=[dict(kind='task',id='child-worker',title='Child worker',requirements=['req-1'],deliverable='Owned report',criteria=['Owned evidence'])])
        child_done=owned.ok(['planning','decide',owned.work],child_value);assert child_done['tasks'][0]['scope_id']=='child'
        # Saved zero operations rejects a real task atomically; null repairs same event.
        candidate=start(owned,'saved-policy');value=decision(candidate,'policy-decision',[dict(kind='task',id='policy-worker',title='Policy worker',requirements=['req-1'],deliverable='Report',criteria=['Evidence'])]);apply_policy(owned,'disable-operations',dict(planning_decision_max_operations='0'));before=owned.ok(['work','show',owned.work]);snapshot=owned.snapshot();code,result=owned.call(['planning','decide',owned.work],value);assert code!=0 and result['failure']['code']=='planning-decision-operation-limit';assert owned.ok(['work','show',owned.work])==before and owned.snapshot()==snapshot
        apply_policy(owned,'enable-operations',dict(planning_decision_max_operations=None));owned.ok(['planning','decide',owned.work],value)
        # Original requirement controls, not model-suggested criteria, authorize task.
        owned.work=fresh_work(owned,'checks-create');enabled=owned.ok(['planning','enable',owned.work],request(event='checks-enable',checks={'req-1':[dict(id='explicit-control',command=['git','diff','--check'],criteria=[1],justification='Verify original authorized requirement',timeout_seconds=30)]}));candidate=owned.ok(['planning','claim',owned.work],claim_request(enabled,'checks-claim'))
        checked=owned.ok(['planning','decide',owned.work],decision(candidate,'checks-decision',[dict(kind='task',id='checked-worker',title='Checked worker',requirements=['req-1'],deliverable='Verified report',criteria=['MODEL-SUGGESTION'])]));task=checked['tasks'][0];policy=task['validation_policy'];assert policy['mode']=='automatic' and policy.get('authorized_at') and policy.get('actor');assert task['criteria']==['Configuration is persisted without model execution'] and 'MODEL-SUGGESTION' not in task['criteria'];assert policy['controls'][0]['criteria']==[1]
        assert not (owned.root/'model-received.json').exists()
        if a.legacy_binary:legacy_adoption(binary,a.legacy_binary.resolve(),out)
        fixture.publish(out/'result.json',dict(status='PASS',scope='Public operator task/delegation decisions; cryptographic agent identity and automatic conductor not proved',negative_cases=cases,delegation=True,scoped_child_task=True,live_lease_renewal_without_new_activation=True,saved_policy_recovery=True,inherited_controls=True,provider_started=False))
    except BaseException as e:
        fixture.publish(out/'failure.json',dict(status='FAIL',cause=type(e).__name__,message=str(e),root=str(owned.root) if owned else None));traceback.print_exc();raise
    finally:
        if owned:fixture.publish(out/'commands.json',owned.calls)


if __name__=='__main__':main()
