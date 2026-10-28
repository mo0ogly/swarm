#!/usr/bin/env python3
"""Shared mission pause guard reaches both validation and fresh native review.

Owned public worker/control evidence and one inert reviewer only. No SQL seed,
paid model, synthesized permission, global watchdog or human acceptance.
"""
import argparse,json,os,traceback
from pathlib import Path
import rust_agent_model_route as f
import rust_native_reviewer as public
import rust_native_review_inputs as inputs
import rust_native_review_preview as preview
from rust_native_review_decision import rows


def main():
    """Healthy evidence precedes public pause and refusals without new jobs/calls.

    Global pause is an operational admission control, distinct from changing
    planning validation authority. Unpause must preserve unchanged report/control
    originals and permit fresh review under current state, without worker retry,
    revalidation, false consent or acceptance. Retain real native reviewer chain.
    The actual reviewer uses this owned root's announced model catalogue through
    an explicit CODEX_HOME; it must never inherit a host catalogue as evidence.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:p.add_argument('--'+name,required=True,type=Path)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);root=None;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes());assert profile['no_fixture_feature'] and f.digest(binary.read_bytes())==profile['sha256']
        root,work,request=inputs.source(binary,out/'source');f.publish(out/'fixture.json',dict(root=str(root),work=work,binary_sha256=profile['sha256']))
        f.publish(root/'fixture-review-reply.json',dict(reason='Owned inert pause recovery witness',criteria=[dict(index=1,verdict='pass',evidence='Observed fixture only; not authenticated model or task acceptance.')]))
        f.publish(root/'codex',public.PROGRAM);(root/'codex').chmod(0o700)
        healthy=preview.preview(binary,root,out,'healthy-review',work,request);control_id=healthy['validation']['job_id'];report=(root/'handoff.md').read_bytes();witness=(root/'validation-executed.json').read_bytes()
        protected={str(p):f.digest(p.read_bytes()) for p in (root/'.swarm').glob('native-validation-*.receipt')};assert protected
        def shown(label):return preview.shown(binary,root,out,label,work)
        def state(label):
            """Read actual optional registry presence; absence is None, not a job."""
            exists=rows(root,"SELECT name FROM sqlite_schema WHERE type='table' AND name='native_review_jobs'",())
            jobs=rows(root,'SELECT * FROM native_review_jobs ORDER BY id',()) if exists else None
            return (shown(label),public.finance(root,work),rows(root,'SELECT * FROM events ORDER BY id',()),rows(root,'SELECT * FROM cockpit_events ORDER BY seq',()),exists,jobs)
        current=shown('before-pause');code,paused=inputs.native.invoke(binary,root,out,'public-pause',['mission','pause',work],dict(schema_version=1,event_id='pause-before-review',expected_revision=current['revision'],reason='Stop new calls; retain current candidate and controls'))
        assert code==0,paused
        baseline=state('paused-baseline');calls=paused['planning']['reviewer']['calls'];assert not (root/'reviewer-invoked.json').exists()
        for action in ['review','validate']:
            body=dict(request,event_id='paused-'+action,expected_revision=baseline[0]['revision']);code,refused=inputs.native.invoke(binary,root,out,'paused-'+action,['task',action,work],body)
            assert code!=0 and 'planning-mission-paused' in refused['failure']['causes'],refused
            assert state('after-refused-'+action)==baseline and not (root/'reviewer-invoked.json').exists()
        code,unpaused=inputs.native.invoke(binary,root,out,'public-unpause',['mission','unpause',work],dict(schema_version=1,event_id='unpause-review',expected_revision=baseline[0]['revision'],reason='Reopen manual admission without changing control authority or consent'))
        assert code==0,unpaused
        request=dict(request,event_id='fresh-review-after-unpause',expected_revision=unpaused['revision'])
        fresh=preview.preview(binary,root,out,'fresh-review-preview',work,request);assert fresh['validation']['job_id']==control_id and fresh['validation']['receipt_sha256']==healthy['validation']['receipt_sha256']
        assert (root/'handoff.md').read_bytes()==report and (root/'validation-executed.json').read_bytes()==witness and protected=={str(p):f.digest(p.read_bytes()) for p in (root/'.swarm').glob('native-validation-*.receipt')}
        money=public.finance(root,work);code,result=inputs.native.invoke(binary,root,out,'actual-fresh-review',['task','review',work],request,environment={**os.environ,'CODEX_HOME':str(root/'codex-home')});assert code==0 and result['review_recorded'] and result['verdict_applicable'] and not result['task_accepted'],result
        final=shown('after-fresh-review');task=next(t for t in final['tasks'] if t['id']=='task');assert task['status']=='submitted' and len(task['attempts'])==1 and final['planning']['reviewer']['calls']==calls+1
        review=task['independent_review'];public.retain(root,out,work,review,profile['sha256'])
        assert all(row in public.finance(root,work) for row in money) and (root/'handoff.md').read_bytes()==report and (root/'validation-executed.json').read_bytes()==witness
        code,view=inputs.native.invoke(binary,root,out,'actual-control-view',['mission','show',work]);assert code==0 and view['paused'] is False and view['autonomy']=='manuel' and view['consent_enabled'] is None
        f.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,binary_sha256=profile['sha256'],initial_healthy_review_preview=True,paused_review_and_validation_refused_without_mutation=True,unchanged_controls_and_report_preserved=True,fresh_actual_review_after_unpause=True,review_calls_consumed_once=True,worker_attempts_preserved=True,task_not_accepted=True,no_true_consent=True,global_watchdog=None,installed=False,authenticated_actor=False))
        print('PASS shared mission pause guards fresh review and validation; unchanged controls recover after unpause',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
