#!/usr/bin/env python3
"""Root operator judge/renew are separate from leased/model decisions.

Public native CLI only on owned fixtures. No model/provider call, private SQL
seed, fabricated delivery, reset, accepted candidate or whole-test watchdog.
"""
import argparse,json,traceback
from pathlib import Path
import rust_agent_model_route as f
from rust_planning_activation import apply_policy,claim_request
from rust_planning_bootstrap import fresh_work,request
import rust_native_review_acceptance as review


def main():
    """Verify no input acknowledgement or claim/counter charge for direct actions.

    Actual current revisions/cycles are echoed, old history stays readable after
    lowering admission policy, and replay bypasses neither history nor later
    consumption. A real operator claim witnesses logical-owner protection;
    public pause/resume releases it without a refund or private owner edit.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:p.add_argument('--'+name,required=True,type=Path)
    args=p.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=args.binary.resolve();profile=json.loads(args.profile.read_bytes());assert profile['no_fixture_feature'] and profile['sha256']==f.digest(binary.read_bytes())
        owned=f.Fixture(binary,out/'source','judge-direct');owned.work=fresh_work(owned,'operator-create');apply_policy(owned,'operator-reason-high',dict(planning_decision_max_reason_bytes='6000'))
        initial=owned.ok(['planning','enable',owned.work],request(event='operator-enable',max_judge_cycles=2));inbox=initial['planning']['inbox'];money=review.public.finance(owned.root,owned.work);markers={str(p):f.digest(p.read_bytes()) for p in owned.root.glob('fixture-prompt-*.bin')}
        refused=[]
        def call(label,action,expected=None,**patch):
            """Observe fresh state/fences and targeted refusal with exact preservation.

            Role/cycle/foreign-field errors must not hide behind a shared unrelated
            rejection. Same-schema success is the control before negative calls.
            """
            before=owned.ok(['work','show',owned.work])['work'];scope=next(s for s in before['planning']['scopes'] if s['id']=='root');body=dict(schema_version=1,event_id=label,expected_revision=before['revision'],scope='root',scope_revision=scope['revision'],judge_cycle=before['planning']['judge']['cycle'],reason='Operator cycle checkpoint')
            body.update(patch);old_money=review.public.finance(owned.root,owned.work);code,value=owned.call(['planning',action,owned.work],body)
            if expected:
                assert code!=0 and expected in value['failure']['causes'],value
                assert owned.ok(['work','show',owned.work])['work']==before and review.public.finance(owned.root,owned.work)==old_money
                refused.append(expected);return before,body
            assert code==0,value
            assert value['planning']['activations']==before['planning']['activations'] and value['planning']['decisions']==before['planning']['decisions']
            assert value['planning']['inbox']==before['planning']['inbox'] and review.public.finance(owned.root,owned.work)==old_money
            assert next(s for s in value['planning']['scopes'] if s['id']=='root')['revision']==scope['revision']+1
            return value,body
        long_reason='é'*2500
        first,first_request=call('operator-judge-first','judge',judge_verdict='continue',reason=long_reason,expected_candidate='supplied-reference-not-attested')
        assert first['planning']['judge']['reason']==long_reason and first['planning']['judge']['history'][0]['evidence_ref']=='supplied-reference-not-attested'
        call('operator-already','judge','planning-judge-already-decided',judge_verdict='continue')
        second,_=call('operator-renew-first','renew',reason='');assert second['planning']['judge']['cycle']==2 and not second['planning']['judge'].get('verdict')
        apply_policy(owned,'operator-reason-low',dict(planning_decision_max_reason_bytes='200'));assert owned.ok(['work','show',owned.work])['work']==second
        call('operator-reason-refused','judge','planning-decision-reason-limit',judge_verdict='continue',reason=long_reason)
        call('operator-inputs-refused','judge','planning-judge-unacknowledged-inputs',judge_verdict='continue',input_events=[inbox[0]['id']])
        call('operator-renew-inputs-refused','renew','planning-judge-unacknowledged-inputs',input_events=[inbox[0]['id']])
        call('operator-fields-refused','judge','planning-judge-operator-field-not-applicable',judge_verdict='continue',operations=[dict(kind='task',id='forbidden')])
        call('operator-holder-refused','judge','planning-judge-operator-field-not-applicable',judge_verdict='continue',holder='declared-holder-does-not-grant-authority')
        call('operator-scope-refused','judge','planning-judge-root-required',judge_verdict='continue',scope='child')
        call('operator-revision-refused','judge','revision_conflict',judge_verdict='continue',scope_revision=1)
        call('operator-cycle-refused','judge','planning-judge-stale-cycle',judge_verdict='continue',judge_cycle=1)
        call('operator-end-refused','judge','planning-judge-end-managed-evidence-unported',judge_verdict='end',expected_candidate='unattested-candidate')
        call('operator-renew-first-refused','renew','planning-judge-continue-required')
        current=owned.ok(['work','show',owned.work])['work'];scope=next(s for s in current['planning']['scopes'] if s['id']=='root');claimed=owned.ok(['planning','claim',owned.work],claim_request(current,'operator-owner-claim',scope_revision=scope['revision']))
        call('operator-busy-refused','judge','planning-judge-owner-active',judge_verdict='continue')
        paused=owned.ok(['planning','pause',owned.work],dict(schema_version=1,event_id='operator-pause',expected_revision=claimed['revision'],reason='Release only owned logical claim before direct cycle action'))
        call('operator-paused-refused','judge','planning-paused',judge_verdict='continue')
        current=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='operator-resume',expected_revision=paused['revision'],reason='Preserve claim history; permit current operator control'))
        current,_=call('operator-judge-second','judge',judge_verdict='continue')
        call('operator-quota-refused','renew','planning-judge-cycle-budget-exhausted')
        quota=owned.ok(['quotas','show',owned.work]);limits=dict(quota['limits'],judge_cycles=3);owned.ok(['quotas','apply',owned.work],dict(schema_version=1,event_id='operator-quota-increase',expected_revision=quota['revision'],limits=limits,reason='Allow the next explicit global cycle; preserve consumed2'))
        final,_=call('operator-renew-second','renew');assert final['planning']['judge']['cycle']==3 and final['planning']['judge']['history'][0]['reason']==long_reason and len(final['planning']['judge']['history'])==2
        assert final['planning']['activations']==1 and final['planning']['decisions']==0 and not final.get('tasks') and all(not e.get('decision') for e in final['planning']['inbox'])
        before=owned.snapshot();replayed=owned.ok(['planning','judge',owned.work],first_request);assert replayed==final and owned.snapshot()==before
        assert markers=={str(p):f.digest(p.read_bytes()) for p in owned.root.glob('fixture-prompt-*.bin')} and all(row in review.public.finance(owned.root,owned.work) for row in money)
        f.publish(out/'result.json',dict(status='PASS',root=str(owned.root),work=owned.work,standalone_cycle_actions=True,no_native_model=True,pending_inputs_preserved=True,no_synthetic_claim=True,operator_claim_preserved=True,historical_reason_readable_under_lower_cap=True,replayed_old_long_reason_under_current_cap=True,refusals=refused,global_watchdog=None,authenticated_actor=False,mission_accepted=False));print('PASS standalone native operator judge/renew without model or implicit acknowledgement',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
