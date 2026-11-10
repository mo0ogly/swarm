#!/usr/bin/env python3
"""Route actual native review protocol failure without a new producer attempt.

Owned inert reviewer/worker/planner only; original receipts come from real CLI
execution. No SQL seed, fabricated grant, paid provider or global watchdog.
"""
import argparse,json,os,traceback
from pathlib import Path
from types import SimpleNamespace
import rust_native_review_acceptance as acceptance
import rust_planning_worker_public_flow as flow
from rust_native_review_decision import rows
from rust_native_production_retry import negative_frame


def main():
    """Require a healthy actual reviewer before the changed protocol response.

    Preserve real review/worker/control evidence before checking routing. The
    diagnostic case requires the review-recovery reason with unchanged Work and
    finance. Retry mode retains the real native planner original before requiring
    its applied decision; that does not itself prove a second review or acceptance.
    Fault mode requires a healthy same-binary decision control before any owned
    original/control/report corruption, asserts no pre-model mutation/call, then
    restores exact bytes and preserves all primary/cleanup failures. Recorded
    parent/child/input digests are compared to the actual provider witness. A
    malformed JSONL/capture scenario must remain unavailable for reviewer retry;
    a similar public error state cannot substitute for the typed original class.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--profile',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--control',type=Path);p.add_argument('--error',action='store_true');p.add_argument('--failure-case',choices=['domain-invalid','malformed-output','interrupted'],default='domain-invalid');p.add_argument('--retry',action='store_true');p.add_argument('--faults',action='store_true');p.add_argument('--decision-control',type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);root=None;f=acceptance.public.f;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();root,work=acceptance.prepare(binary,a.profile.resolve(),out/'source',review_case=a.failure_case if a.error else 'passed');before=acceptance.public.preview.shown(binary,root,out,'before',work);task=before['tasks'][0];review=task['independent_review'];assert review['state']==('error' if a.error else 'passed')
        signature=dict(contract=f.digest(json.dumps({k:task.get(k) for k in ['title','deliverable','criteria','depends','plan_max_attempts']},sort_keys=True).encode()))
        # Settings timestamps/event identities are part of their document, so
        # compare actual effective values for separate owned Root controls.
        signature['settings']=f.digest(json.dumps(json.loads((root/'.swarm/engine-settings.json').read_bytes())['values'],sort_keys=True).encode())
        f.publish(out/'original-review.json',review)
        if not a.error:
            f.publish(out/'result.json',dict(status='PASS_CONTROL',signature=signature,binary_sha256=f.digest(binary.read_bytes()),root=str(root),work=work,actual_review=True));print('PASS healthy native reviewer before protocol failure',flush=True);return
        assert a.control is not None;control=json.loads(a.control.read_bytes());assert control['status']=='PASS_CONTROL' and control['actual_review'] and control['signature']==signature and control['binary_sha256']==f.digest(binary.read_bytes())
        finance=acceptance.public.finance(root,work);worker_money=rows(root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(work,));environment={**os.environ,'CODEX_HOME':str(root/'codex-home')}
        request=dict(schema_version=1,event_id='forbidden-producer',expected_revision=before['revision'],task_id='task',provider='fixture',capture_output=True)
        status,value=acceptance.public.native.invoke(binary,root,out,'worker-refused',['agent','prepare',work],request,environment=environment)
        assert status!=0 and 'scoped-worker-review-recovery-required' in value['failure']['causes'],value
        assert acceptance.public.preview.shown(binary,root,out,'after-worker-refusal',work)==before and acceptance.public.finance(root,work)==finance and rows(root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(work,))==worker_money and rows(root,'SELECT id FROM agents WHERE id=?',('forbidden-producer',))==[]
        if not a.retry:
            f.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,protocol_error_routed=True,producer_refused=True,producer_attempts=len(task['attempts']),task_accepted=False));print('PASS protocol failure routes to review recovery with no new worker',flush=True);return
        f.publish(root/'codex',flow.feedback.PROGRAM.encode());(root/'codex').chmod(0o700)
        op=dict(kind='review-retry',id='task',next=('Resume the reviewer after its known terminal interruption; preserve the unchanged producer attempt, report and original controls' if a.failure_case=='interrupted' else 'Correct the reviewer response schema after inspecting its actual original error; preserve the producer attempt and report'))
        flow.proposal(SimpleNamespace(root=root),before,'child',[op])
        if a.faults:
            assert a.decision_control is not None;healthy=json.loads(a.decision_control.read_bytes())
            assert healthy['status']=='PASS_DECISION_ONLY' and healthy['signature']==signature and healthy['binary_sha256']==f.digest(binary.read_bytes()) and not healthy.get('faults'),healthy
            original_path=root/'.swarm'/('native-review-'+f.digest(review['id'].encode())+'.receipt');original_bytes=original_path.read_bytes()
            controls=list((root/'.swarm').glob('native-validation-step-*.receipt'));assert len(controls)==1
            cases=[('private-original',original_path,'storage-operation:Read'),('control-original',controls[0],'storage-operation:Read'),('report',root/'handoff.md','review-artifact-stale')]
            prior_runs=rows(root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? ORDER BY claim_id',(work,));prompts=set(root.glob('fixture-prompt-*.bin'))
            for name,path,reason in cases:
                original=path.read_bytes();changed=bytearray(original);changed[-1]^=1;f.publish(out/(name+'-original.bytes'),original)
                with negative_frame(path,original,bytes(changed)):
                    code,refused=acceptance.public.native.invoke(binary,root,out,name+'-refused',['planning','step',work],environment=environment)
                    assert code!=0 and reason in refused['failure']['causes'],refused
                    assert rows(root,'SELECT revision FROM works WHERE id=?',(work,))==[(before['revision'],)] and acceptance.public.finance(root,work)==finance and rows(root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? ORDER BY claim_id',(work,))==prior_runs and set(root.glob('fixture-prompt-*.bin'))==prompts
                assert acceptance.public.preview.shown(binary,root,out,name+'-restored',work)==before
            assert original_path.read_bytes()==original_bytes
        status,result=acceptance.public.native.invoke(binary,root,out,'planner-review-retry',['planning','step',work],environment=environment)
        actual=rows(root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? AND scope_id=? ORDER BY prepared_revision,claim_id',(work,'child'));claim=actual[-1][0];frame=root/'.swarm'/('native-planning-'+f.digest(claim.encode())+'.receipt');raw=frame.read_bytes();magic,extent,digest,body=raw.split(b'\n',3);assert magic==b'SR1' and len(body)==int(extent) and f.digest(body)==digest.decode();f.publish(out/'planner-original.frame',raw);f.publish(out/'planner-program.py',flow.feedback.PROGRAM.encode());f.publish(out/'proposal.json',(root/'planning-proposal.json').read_bytes());f.publish(out/'planner-observation.json',dict(root=str(root),work=work,rows=actual))
        if a.failure_case=='malformed-output':
            assert status!=0 and 'planning-decision-input-unavailable' in result['failure']['causes'],result
            after=acceptance.public.preview.shown(binary,root,out,'nonprotocol-refused',work);current=after['tasks'][0]
            assert current['independent_review']==review and current['attempts']==task['attempts'] and current['next']==task['next'] and len(after['tasks'])==1
            assert rows(root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(work,))==worker_money
            f.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,failed_capture_protocol_retry_refused=True,second_review=False,new_producer_attempt=False,task_accepted=False));print('PASS unqualified failed capture cannot authorize review retry',flush=True);return
        assert status==0 and result['run']['decision_applied'],dict(exit_code=status,failure=result.get('failure'))
        after=acceptance.public.preview.shown(binary,root,out,'after-decision',work);current=after['tasks'][0]
        assert current['status']==task['status']=='submitted' and current['independent_review']==review and current['attempts']==task['attempts'] and current['automatic_validation']==task['automatic_validation'] and current['next']==op['next'] and not current.get('planning_retry')
        assert len(after['tasks'])==1 and after['planning']['reviewer']==before['planning']['reviewer']
        assert rows(root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(work,))==worker_money
        money=acceptance.public.finance(root,work);assert len(money)==len(finance)+1 and all(row in money for row in finance)
        run=json.loads(body)['run'];pid=run['child_pid'];prompt=(root/('fixture-prompt-'+str(pid)+'.bin')).read_bytes();original=root/'.swarm'/('native-review-'+f.digest(review['id'].encode())+'.receipt')
        review_frame=original.read_bytes();magic,extent,body_sha,review_body=review_frame.split(b'\n',3);assert magic==b'SR1' and len(review_body)==int(extent) and f.digest(review_body)==body_sha.decode()
        failure_code=b'review_fragment_interrupted' if a.failure_case=='interrupted' else b'review_fragment_protocol'
        assert b'\"review_retry_unavailable\":false' in prompt and b'review_failure_routing' in prompt and b'review_protocol_originals' in prompt and failure_code in prompt and review['id'].encode() in prompt and body_sha in prompt and b'PRIVATE invalid domain fixture' not in prompt and b'review-retry (id,next' in prompt and b'close (champs vides' in prompt
        witness_path=root/('model-received-'+str(pid)+'.json');witness=json.loads(witness_path.read_bytes())
        assert witness['pid']==run['child_pid'] and witness['ppid']==run['parent_pid'] and witness['prompt_sha256']==f.digest(prompt)==run['prompt_sha256']
        f.publish(out/'planner-prompt.bin',prompt);f.publish(out/'planner-witness.json',witness_path.read_bytes());f.publish(out/'review-original.frame',original.read_bytes())
        money_before_replay=acceptance.public.finance(root,work);code,replayed=acceptance.public.native.invoke(binary,root,out,'decision-replay',['planning','reconcile',claim]);assert code==0 and replayed['replayed'] and replayed['decision_applied']
        assert acceptance.public.preview.shown(binary,root,out,'after-replay',work)==after and acceptance.public.finance(root,work)==money_before_replay
        f.publish(out/'result.json',dict(status='PASS_DECISION_ONLY',root=str(root),work=work,signature=signature,binary_sha256=f.digest(binary.read_bytes()),faults=a.faults,pre_model_faults=3 if a.faults else 0,actual_planner=True,original_protocol_incident_verified=True,original_body_sha256=body_sha.decode(),original_frame_sha256=f.digest(review_frame),same_producer_attempt=True,original_controls_preserved=True,original_review_preserved=True,old_spending_preserved=True,correction_recorded=True,replayed_without_exec=True,second_review=False,task_accepted=False));print('PASS native review retry decision only',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
