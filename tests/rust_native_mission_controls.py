#!/usr/bin/env python3
"""Global native mission pause/stop preserve consumption, consent and originals.

Owned normal-profile Root, actual CLI and read-only state inspection. No model
process, provider charge, positive SQL seed, source edit or global watchdog.
Logical claims are data; this recipe makes no physical-process recovery claim.
"""
import argparse,json,traceback
from pathlib import Path
import rust_agent_model_route as f
import rust_native_review_acceptance as review
from rust_planning_activation import apply_policy,configured,claim_request
from rust_planning_call_preview import preview_request
from rust_native_review_decision import rows


def main():
    """Use valid worker preview and logical planner binding before pause faults.

    Saved non-DDL initial slots17 are observed/reopened; lower policy does not
    invalidate existing17. Compare actual canonical/control events, Work/agent/
    reservation/finance, pending inputs and frozen claim context around refusals
    and replay. Unpause preserves manual mode/absent or false consent. Old logical
    claim refuses after unpause, while a real new claim/preview succeeds. A stop
    never refunds reservations or creates permission to launch automatically.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:p.add_argument('--'+name,required=True,type=Path)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes());assert profile['no_fixture_feature'] and profile['sha256']==f.digest(binary.read_bytes())
        owned=f.Fixture(binary,out/'source','mission-controls');apply_policy(owned,'mission-slots-policy',dict(mission_dispatch_default_slots='17',mission_dispatch_max_slots='33'))
        def snapshot():
            """Observe real state without writes or silently defaulting a missing row."""
            return (owned.snapshot(),review.public.finance(owned.root,owned.work),rows(owned.root,'SELECT * FROM events ORDER BY id',()),rows(owned.root,'SELECT * FROM cockpit_events ORDER BY seq',()),rows(owned.root,'SELECT * FROM cockpit_controls ORDER BY work_id',()))
        def control(label,action,**patch):
            """Derive actual Work revision; each mutation retains original CLI bytes."""
            work=owned.ok(['work','show',owned.work])['work'];request=dict(schema_version=1,event_id=label,expected_revision=work['revision'],reason='Explicit local departure control; no automatic consent')
            request.update(patch);return owned.ok(['mission',action,owned.work],request),request
        def refused(label,action,expected,**patch):
            """Invalid request must hit its intended boundary and preserve SQL/state."""
            work=owned.ok(['work','show',owned.work])['work'];request=dict(schema_version=1,event_id=label,expected_revision=work['revision'],reason='Do not manufacture state or consent');request.update(patch);before=snapshot();code,value=owned.call(['mission',action,owned.work],request)
            assert code!=0 and value['failure']['code']==expected,value
            assert snapshot()==before;return expected
        initial=owned.ok(['mission','show',owned.work]);assert initial['paused'] is None and initial['autonomy'] is None and initial['slots'] is None and initial['consent_enabled'] is None
        refused('unpause-absent','unpause','mission-controls-required')
        healthy=owned.prepare(preview=True);assert healthy['preview_only'] and owned.reservation() is None
        paused,_=control('worker-global-pause','pause');view=owned.ok(['mission','show',owned.work]);assert view['paused'] and view['autonomy']=='manuel' and view['slots']==17 and view['consent_enabled'] is None
        owned.request['expected_revision']=paused['revision'];before=snapshot();code,value=owned.call(['agent','preview',owned.work],owned.request);assert code!=0 and 'agent-launch-mission-paused' in value['failure']['causes'],value;assert snapshot()==before
        resumed,_=control('worker-global-unpause','unpause');owned.request['expected_revision']=resumed['revision'];healthy=owned.prepare(preview=True);assert healthy['preview_only'] and owned.reservation() is None
        view=owned.ok(['mission','show',owned.work]);assert view['paused'] is False and view['autonomy']=='manuel' and view['consent_enabled'] is None
        enabled=configured(owned,'logical-planner');claim='mission-original-claim';claimed=owned.ok(['planning','claim',owned.work],claim_request(enabled,claim));original=preview_request(claimed,claim);assert owned.ok(['planning','call-preview',owned.work],original)['prepared_inputs_verified']
        frozen=rows(owned.root,'SELECT context_json,context_sha256,generation FROM native_planning_claims WHERE claim_id=?',(claim,))[0];money=review.public.finance(owned.root,owned.work)
        old_scope=next(s for s in claimed['planning']['scopes'] if s['id']=='root');paused,pause_request=control('planner-global-pause','pause');scope=next(s for s in paused['planning']['scopes'] if s['id']=='root')
        assert scope['generation']==old_scope['generation']+1 and not scope.get('holder') and not scope.get('lease_until') and not paused['planning']['paused']
        assert paused['planning']['activations']==claimed['planning']['activations']==1 and paused['planning']['decisions']==claimed['planning']['decisions']==0 and paused['planning']['inbox']==claimed['planning']['inbox']
        assert rows(owned.root,'SELECT context_json,context_sha256,generation FROM native_planning_claims WHERE claim_id=?',(claim,))[0]==frozen and review.public.finance(owned.root,owned.work)==money
        before=snapshot();code,value=owned.call(['planning','claim',owned.work],claim_request(paused,'denied-new-claim',scope_revision=scope['revision']));assert code!=0 and 'planning-mission-paused' in value['failure']['causes'],value;assert snapshot()==before
        current,_=control('planner-global-unpause','unpause');old={**original,'expected_revision':current['revision']};before=snapshot();code,value=owned.call(['planning','call-preview',owned.work],old);assert code!=0 and 'stale_lease' in value['failure']['causes'],value;assert snapshot()==before
        scope=next(s for s in current['planning']['scopes'] if s['id']=='root');fresh_id='mission-fresh-claim';fresh=owned.ok(['planning','claim',owned.work],claim_request(current,fresh_id,scope_revision=scope['revision']));assert owned.ok(['planning','call-preview',owned.work],preview_request(fresh,fresh_id))['prepared_inputs_verified']
        assert fresh['planning']['activations']==2 and rows(owned.root,'SELECT context_json,context_sha256,generation FROM native_planning_claims WHERE claim_id=?',(claim,))[0]==frozen and all(row in review.public.finance(owned.root,owned.work) for row in money)
        stopped,_=control('planner-stop-consent','stop');view=owned.ok(['mission','show',owned.work]);assert view['paused'] and view['consent_enabled'] is False and view['autonomy']=='manuel'
        before=snapshot();assert owned.ok(['mission','pause',owned.work],pause_request)==stopped and snapshot()==before
        apply_policy(owned,'mission-slots-lower',dict(mission_dispatch_default_slots='3',mission_dispatch_max_slots='4'));control('pause-with-legacy-slots','pause');assert owned.ok(['mission','show',owned.work])['slots']==17
        unpaused,_=control('unpause-after-stop','unpause');view=owned.ok(['mission','show',owned.work]);assert view['paused'] is False and view['consent_enabled'] is False and view['autonomy']=='manuel' and view['slots']==17
        faults=[refused('stale-control','pause','revision_conflict',expected_revision=1),refused('unknown-consent','stop','mission-control-request-invalid',enabled=True),refused('unknown-actor','pause','mission-control-request-invalid',actor='declared'),refused('unknown-slots','pause','mission-control-request-invalid',slots=2),refused('wrong-schema','pause','mission-control-request-invalid',schema_version=0),refused('blank-reason','pause','mission-control-request-invalid',reason=' ')]
        for action in ['watch']:faults.append(refused('unported-'+action,action,'command_not_ported'))
        assert unpaused['planning']['activations']==2 and not unpaused.get('tasks') and not list(owned.root.glob('fixture-prompt-*.bin'))
        f.publish(out/'result.json',dict(status='PASS',root=str(owned.root),work=owned.work,binary_sha256=profile['sha256'],worker_preview_before_pause_and_after_unpause=True,new_worker_and_planner_admission_refused_while_paused=True,old_logical_claim_context_preserved_and_refused=True,new_logical_claim_preview_succeeds=True,no_refund_or_false_consent=True,old_slots_readable_after_lower_policy=True,replay_without_second_revocation=True,control_refusals=faults,no_model_process=True,physical_run_recovery_covered=False,automatic_conductor=False,global_watchdog=None,installed=False,authenticated_actor=False))
        print('PASS global native mission pause/unpause/stop preserve consent, claims and consumption',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
