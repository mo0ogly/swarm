#!/usr/bin/env python3
"""Replay explicit native mission start/resume configuration on an owned Root.

Real public worker/control setup precedes configuration, refusals and recovery.
No paid provider, SQL seed, automatic conductor or new model call is used by the
configuration commands. Retain failed roots and original channels; no watchdog.
"""
import argparse,json,os,sqlite3,traceback
from pathlib import Path
import rust_agent_model_route as f
import rust_native_review_inputs as inputs
import rust_native_review_preview as preview
import rust_native_reviewer as review
from rust_native_review_decision import rows

def main():
    """Verify start/replay/pause/resume/stop with persisted policy and no dispatch.

    All invalid variants follow a successful explicit start. Current saved caps
    apply to resume, without clamping existing slots or changing original consent.
    --expect-unported retains the old binary's missing command as a RED witness.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for key in ['binary','profile','output']:p.add_argument('--'+key,type=Path,required=True)
    p.add_argument('--expect-unported',action='store_true');a=p.parse_args()
    out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);root=None;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes());assert profile['no_fixture_feature'] and f.digest(binary.read_bytes())==profile['sha256']
        root,work,_=inputs.source(binary,out/'source');environment={**os.environ,'CODEX_HOME':str(root/'codex-home')}
        def call(label,action,request=None):
            """Invoke actual CLI using only this owned announced fixture catalogue."""
            return inputs.native.invoke(binary,root,out,label,['mission',action,work],request,environment=environment)
        def shown(label):return preview.shown(binary,root,out,label,work)
        def state(label):
            """Observe actual Work, controls/events/agents/finance without SQL writes."""
            return (shown(label),rows(root,'SELECT * FROM events ORDER BY id',()),rows(root,'SELECT * FROM cockpit_events ORDER BY seq',()),rows(root,'SELECT * FROM cockpit_controls ORDER BY work_id',()),rows(root,'SELECT * FROM agents ORDER BY id',()),review.finance(root,work))
        def request(label,**patch):
            """Bind each new audited command to its actually observed current revision."""
            return dict(schema_version=1,event_id=label,expected_revision=shown(label+'-revision')['revision'],reason='Explicit owned local mission configuration',**patch)
        def mutate(label,action,**patch):
            """Apply a fresh public command; errors retain exact channels and Root."""
            body=request(label,**patch);code,result=call(label,action,body);assert code==0,result;return result,body
        start_profile=dict(provider='fixture',role='worker',workspace='.',level='standard',capture_output=True)
        initial=request('explicit-start',profile=start_profile);before=state('red-before')
        if a.expect_unported:
            code,wire=call('start-unported','start',initial);assert code!=0 and wire['failure']['code']=='command_not_ported',wire
            assert state('red-after')==before
            f.publish(out/'result.json',dict(status='RED_CONFIRMED',root=str(root),missing='mission.start',valid_native_worker_and_control_setup=True,no_mutation=True,binary_sha256=profile['sha256']));return
        preview.saved_setting(binary,root,out,'max-slots','mission_dispatch_max_slots','33')
        preview.saved_setting(binary,root,out,'initial-slots','mission_dispatch_default_slots','17')
        finance=review.finance(root,work);agents=rows(root,'SELECT * FROM agents ORDER BY id',());report=(root/'handoff.md').read_bytes();control=(root/'validation-executed.json').read_bytes()
        started,initial=mutate('explicit-start','start',profile=start_profile)
        code,view=call('started-view','show');assert code==0 and view['consent_enabled'] is True and view['autonomy']=='autonome' and view['paused'] is False and view['slots']==17,view
        stored=started['launch_profile'];assert stored['workspace']==str(root) and stored['actor'] and stored['updated'];assert started['planning']['reviewer']['calls']==0
        saved=state('replay-before');assert call('exact-replay','start',initial)==(0,started) and state('replay-after')==saved
        # Extra triggers are refused by Root's schema guard on open. Capture
        # healthy state first, require that exact boundary, remove only our own
        # negative fixture trigger and verify ordinary admission recovers.
        failed=request('forced-schema-refusal',profile=start_profile)
        unchanged=state('schema-refusal-before')
        with sqlite3.connect(root/'.swarm/state.db') as db:
            db.execute("CREATE TRIGGER fixture_start_refusal BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT,'PRIVATE owned fixture canonical write refusal'); END")
        code,rejected=call('schema-refusal','start',failed)
        assert code!=0 and rejected['failure']['code']=='root-schema-mismatch' and 'PRIVATE' not in json.dumps(rejected),rejected
        with sqlite3.connect(root/'.swarm/state.db') as db:db.execute('DROP TRIGGER fixture_start_refusal')
        assert state('schema-refusal-recovered')==unchanged
        f.publish(out/'schema-refusal.json',dict(failure=rejected['failure'],state_unchanged=True,not_mid_transaction_rollback_evidence=True))
        recovered,_=mutate('after-schema-recovery','start',profile=start_profile)
        stored=recovered['launch_profile']
        mutate('pause-started','pause');resumed,_=mutate('resume-consented','resume');assert resumed['launch_profile']==stored
        code,view=call('resumed-view','show');assert code==0 and not view['paused'] and view['consent_enabled'] is True
        policy_rows=rows(root,"SELECT * FROM cockpit_events WHERE kind='mission-policy' ORDER BY seq",());assert len(policy_rows)==2
        def refused(label,action,expected,**patch):
            """Require the intended cause and exact state preservation before recovery."""
            body=request(label,**patch);saved=state(label+'-before');code,wire=call(label,action,body)
            assert code!=0 and expected in wire['failure']['causes'],wire
            assert state(label+'-after')==saved;return expected
        faults=[]
        for label,patch,expected in [('provider',dict(provider='missing'),'agent-provider-not-configured'),('workspace',dict(workspace='/'),'agent-workspace-outside-project-or-not-directory'),('role',dict(role='reviewer'),'mission-profile-invalid'),('actor',dict(actor='forged'),'mission-control-request-invalid'),('negative-timeout',dict(timeout_seconds=-1),'mission-profile-invalid')]:
            faults.append(refused('bad-'+label,'start',expected,profile={**start_profile,**patch}))
        faults.append(refused('too-many-slots','start','mission-dispatch-saved-slots-outside-policy',profile=start_profile,slots=34))
        faults.append(refused('resume-cannot-set-slots','resume','mission-control-request-invalid',slots=2))
        mutate('pause-before-tightening','pause');preview.saved_setting(binary,root,out,'lower-default','mission_dispatch_default_slots','3');preview.saved_setting(binary,root,out,'lower-cap','mission_dispatch_max_slots','4')
        faults.append(refused('resume-under-tightened-cap','resume','mission-dispatch-saved-slots-outside-policy'))
        assert call('old-slots-not-clamped','show')[1]['slots']==17
        preview.saved_setting(binary,root,out,'restore-cap','mission_dispatch_max_slots','33');mutate('resume-after-policy-recovery','resume')
        mutate('stop-consent','stop');faults.append(refused('resume-after-stop','resume','mission-consent-required'))
        mutate('unpause-is-not-consent','unpause');assert call('still-stopped','show')[1]['consent_enabled'] is False
        again,_=mutate('start-reuse-profile','start');assert again['launch_profile']['provider']==stored['provider'];assert call('consent-again','show')[1]['consent_enabled'] is True
        assert review.finance(root,work)==finance and rows(root,'SELECT * FROM agents ORDER BY id',())==agents
        assert (root/'handoff.md').read_bytes()==report and (root/'validation-executed.json').read_bytes()==control and again['planning']['reviewer']['calls']==0
        f.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,binary_sha256=profile['sha256'],explicit_start_and_existing_consent_resume=True,source_profile_reused=True,no_new_model_call_or_refund=True,slot_policy_persisted=True,no_clamping=True,exact_replay=True,altered_schema_refused_without_mutation=True,recovery_after_schema_refusal=True,refusals=faults,global_watchdog=None,automatic_conductor=False,installed=False,authenticated_actor=False))
        print('PASS mission start/resume configuration, explicit consent, replay and policy recovery without dispatch',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()));raise

if __name__=='__main__':main()
