#!/usr/bin/env python3
"""Real owned native supervisor retry preserves task, attempts and provenance.

Inert programs only, no paid model, SQL seed, fake positive receipt, global
watchdog, authenticated actor or autonomous conductor claim.
"""
import argparse,json,os,traceback,sqlite3
from pathlib import Path
from contextlib import contextmanager
from types import SimpleNamespace
import rust_native_review_acceptance as acceptance
import rust_planning_worker_public_flow as flow
from rust_native_review_decision import rows


def replace_event_request(root,work,event,expected,replacement):
    """Inject/restore one negative canonical-event corruption in this owned Root.

    Caller archives genuine original bytes first and restores them in finally.
    Compare-and-swap forbids overwriting an unexpected row. There is no positive
    state seed, permission/receipt creation, live mission or wait/retry policy;
    SQLite no-wait detects unexpected concurrent fixture ownership immediately.
    Primary/rollback/close failures remain joined and never produce empty success.
    """
    db=sqlite3.connect(root/'.swarm/state.db',timeout=0);errors=[]
    try:
        db.execute('BEGIN IMMEDIATE')
        cursor=db.execute('UPDATE events SET request=? WHERE id=? AND work_id=? AND kind=? AND request=?',(replacement,event,work,'planning.decide',expected));count=cursor.rowcount;cursor.close()
        if count!=1:raise ValueError('Owned canonical event changed unexpectedly')
        db.commit()
    except BaseException as error:
        errors.append(error)
        if db.in_transaction:
            try:db.rollback()
            except BaseException as error:errors.append(error)
    try:db.close()
    except BaseException as error:errors.append(error)
    if errors:raise BaseExceptionGroup('Negative event mutation and cleanup failed',errors)


@contextmanager
def negative_event(root,work,event,original,changed):
    """Restore only our exact negative row and join primary/cleanup failures."""
    errors=[]
    try:
        replace_event_request(root,work,event,original,changed)
        yield
    except BaseException as error:errors.append(error)
    try:
        actual=rows(root,'SELECT request FROM events WHERE id=? AND work_id=?',(event,work))
        if actual==[(changed,)]:replace_event_request(root,work,event,changed,original)
        elif actual!=[(original,)]:raise ValueError('Unexpected canonical event; refusing blind restoration')
    except BaseException as error:errors.append(error)
    if errors:raise BaseExceptionGroup('Canonical corruption probe and restoration failed',errors)


@contextmanager
def negative_frame(path,original,changed):
    """Restore the exact owned frame; preserve operation and restoration causes."""
    errors=[]
    try:
        acceptance.public.f.publish(path,changed)
        yield
    except BaseException as error:errors.append(error)
    try:
        actual=path.read_bytes()
        if actual==changed:acceptance.public.f.publish(path,original)
        elif actual!=original:raise ValueError('Unexpected frame; refusing blind restoration')
    except BaseException as error:errors.append(error)
    if errors:raise BaseExceptionGroup('Frame corruption probe and restoration failed',errors)


def main():
    """Retain actual planner original before requiring its retry application.

    A correction is untrusted proposal data; the engine must verify current
    original review/control/report and canonical decision before a new attempt.
    Owned negative event/frame faults have exact restoration; archive reports,
    prompts and completion before PASS. The new report stays unaccepted and
    a second review/control is not claimed by this recipe.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--faults',action='store_true');p.add_argument('--control',type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);root=None
    acceptance.public.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();root,work=acceptance.prepare(binary,a.profile.resolve(),out/'source',review_case='changes-requested')
        before=acceptance.public.preview.shown(binary,root,out,'before',work);task=before['tasks'][0];assert task['status']=='blocked' and not task.get('planning_retry')
        worker_money=rows(root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(work,))
        signature=dict(settings=acceptance.public.f.digest(json.dumps(json.loads((root/'.swarm/engine-settings.json').read_text())['values'],sort_keys=True).encode()),task_contract=acceptance.public.f.digest(json.dumps({k:task.get(k) for k in ['title','deliverable','criteria','depends','plan_max_attempts']},sort_keys=True).encode()),prior_worker_estimates=sorted([r[1],r[2]] for r in worker_money))
        if a.faults:
            assert a.control is not None,'Healthy actual retry control required before fault injection'
            control=json.loads(a.control.read_bytes());assert control['status']=='PASS' and control['actual_second_worker'] and not control['faults'] and control['binary_sha256']==acceptance.public.f.digest(binary.read_bytes()) and control['setup_signature']==signature,control
        original_review=task['independent_review'];original_report=(root/'handoff.md').read_bytes();original_invocation=(root/'reviewer-invoked.json').read_bytes();money=acceptance.public.finance(root,work)
        retained=out/'originals';retained.mkdir();acceptance.public.f.publish(retained/'first-handoff.md',original_report);acceptance.public.f.publish(retained/'first-reviewer-invocation.json',original_invocation)
        code=flow.feedback.PROGRAM.encode();acceptance.public.f.publish(root/'codex',code);(root/'codex').chmod(0o700)
        instruction='Apply correction from review '+original_review['id']+' to the same task; preserve handoff.md and emit a distinct corrected report in retry-report.md.'
        operation=dict(kind='retry',id='task',next=instruction);flow.proposal(SimpleNamespace(root=root),before,'child',[operation])
        status,value=acceptance.public.native.invoke(binary,root,out,'planner-retry',['planning','step',work],environment={**os.environ,'CODEX_HOME':str(root/'codex-home')})
        actual=rows(root,'SELECT claim_id,phase,prepared_revision,receipt_sha256 FROM native_planning_runs WHERE work_id=? AND scope_id=? ORDER BY prepared_revision,claim_id',(work,'child'));claim=actual[-1][0];path=root/'.swarm'/('native-planning-'+acceptance.public.f.digest(claim.encode())+'.receipt');raw=path.read_bytes();magic,extent,digest,body=raw.split(b'\n',3);assert magic==b'SR1' and len(body)==int(extent) and acceptance.public.f.digest(body)==digest.decode();acceptance.public.f.publish(retained/'planner-original.frame',raw);acceptance.public.f.publish(retained/'inert-planner.py',code);acceptance.public.f.publish(retained/'proposal.json',(root/'planning-proposal.json').read_bytes());acceptance.public.f.publish(retained/'retention.json',dict(root=str(root),work=work,row=actual[-1],frame_sha256=acceptance.public.f.digest(raw),binary_sha256=acceptance.public.f.digest(binary.read_bytes())))
        assert status==0 and value['run']['decision_applied'],dict(exit_code=status,failure=value.get('failure'))
        after=acceptance.public.preview.shown(binary,root,out,'after-decision',work);current=after['tasks'][0]
        assert len(after['tasks'])==1 and current['id']==task['id'] and current['status']=='blocked' and current['planning_retry'] and current['next']==instruction and current['independent_review']==original_review and current['attempts']==task['attempts']
        assert (root/'handoff.md').read_bytes()==original_report and (root/'reviewer-invoked.json').read_bytes()==original_invocation
        finance=acceptance.public.finance(root,work);assert len(finance)==len(money)+1 and all(row in finance for row in money)
        environment={**os.environ,'CODEX_HOME':str(root/'codex-home')}
        worker_code=flow.mission.PROGRAM.replace("'model-received.json'","'retry-model-received.json'").replace("os.replace(name,'handoff.md')","os.replace(name,'retry-report.md')")
        acceptance.public.f.publish(root/'codex',worker_code.encode());(root/'codex').chmod(0o700)
        request=dict(schema_version=1,event_id='retry-worker-session',expected_revision=after['revision'],task_id='task',provider='fixture',capture_output=True)
        # A valid preview witnesses provider readiness before each fault.
        status,preview=acceptance.public.native.invoke(binary,root,out,'valid-retry-preview',['agent','preview',work],request,environment=environment);assert status==0 and preview['preview_only'],preview
        request['model_policy_hash']=preview['agent']['model_route']['policy_hash']
        # The public scoped edit route is itself unsupported; require that
        # refusal, then target canonical integrity with a negative fixture fault.
        changed=dict(schema_version=1,event_id='retry-next-changed',expected_revision=after['revision'],id='task',status='blocked',blocker=current['blocker'],next='A different local instruction has no matching retry decision')
        status,value=acceptance.public.native.invoke(binary,root,out,'change-next',['task','update',work],changed,environment=environment)
        assert status!=0 and 'command_not_ported' in value['failure']['causes'],value
        assert acceptance.public.preview.shown(binary,root,out,'after-unsupported-edit',work)==after
        decision_event=json.loads(body)['run']['decision_event'];event_rows=rows(root,'SELECT request FROM events WHERE id=? AND work_id=?',(decision_event,work));assert len(event_rows)==1 and isinstance(event_rows[0][0],bytes)
        event_original=event_rows[0][0];mutated=json.loads(event_original);assert mutated['operations'][0]['next']==instruction;mutated['operations'][0]['next']='A corrupted canonical correction must not authorize this unchanged task'
        event_changed=json.dumps(mutated,separators=(',',':'),ensure_ascii=False).encode();acceptance.public.f.publish(retained/'canonical-decision-original.json',event_original);acceptance.public.f.publish(retained/'canonical-decision-negative.json',event_changed)
        if a.faults:
            with negative_event(root,work,decision_event,event_original,event_changed):
                status,refused=acceptance.public.native.invoke(binary,root,out,'changed-decision-refused',['agent','prepare',work],request,environment=environment)
                assert status!=0 and 'native-retry-decision-changed' in refused['failure']['causes'],refused
                assert acceptance.public.preview.shown(binary,root,out,'after-decision-refusal',work)==after and acceptance.public.finance(root,work)==finance and rows(root,'SELECT status FROM agents WHERE id=?',('retry-worker-session',))==[]
        ready=acceptance.public.preview.shown(binary,root,out,'ready',work);assert ready==after
        status,preview=acceptance.public.native.invoke(binary,root,out,'restored-retry-preview',['agent','preview',work],request,environment=environment);assert status==0 and preview['preview_only'],preview
        controls=list((root/'.swarm').glob('native-validation-step-*.receipt'));assert len(controls)==1
        path=controls[0];original=path.read_bytes();bad_bytes=bytearray(original);bad_bytes[-1]^=1
        if a.faults:
            with negative_frame(path,original,bytes(bad_bytes)):
                status,refused=acceptance.public.native.invoke(binary,root,out,'bad-original-before-prepare',['agent','prepare',work],request,environment=environment)
                assert status!=0 and 'storage-operation:Read' in refused['failure']['causes'],refused
                assert acceptance.public.preview.shown(binary,root,out,'after-original-refusal',work)==ready and acceptance.public.finance(root,work)==finance and rows(root,'SELECT status FROM agents WHERE id=?',('retry-worker-session',))==[]
        status,prepared=acceptance.public.native.invoke(binary,root,out,'prepare',['agent','prepare',work],request,environment=environment);assert status==0,prepared
        queued=acceptance.public.preview.shown(binary,root,out,'queued',work);qtask=queued['tasks'][0]
        assert qtask['status']=='running' and not qtask.get('planning_retry') and not qtask.get('independent_review') and qtask['previous_reviews'][-1]==original_review and len(qtask['attempts'])==2 and qtask['attempts'][0]==task['attempts'][0] and qtask['next']==instruction
        assert not qtask.get('automatic_validation') and not qtask.get('gate')
        prompt=prepared['agent']['prompt'];assert instruction in prompt and original_review['id'] in prompt and 'ROOT-PRIVATE-TITLE' not in prompt
        queued_finance=acceptance.public.finance(root,work);assert queued_finance==finance
        worker_queued=rows(root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(work,));assert len(worker_queued)==len(worker_money)+1 and all(r in worker_queued for r in worker_money) and any(r[0]=='retry-worker-session' and r[1]=='reserved' for r in worker_queued)
        # At actual preflight current validation is already cleared. The guard
        # MUST reread the prior original chain instead of treating it as absent.
        if a.faults:
            with negative_frame(path,original,bytes(bad_bytes)):
                status,refused=acceptance.public.native.invoke(binary,root,out,'bad-original-before-run',['agent','run','retry-worker-session'],environment=environment)
                assert status!=0 and 'storage-operation:Read' in refused['failure']['causes'],refused
                assert acceptance.public.preview.shown(binary,root,out,'after-preflight-refusal',work)==queued and acceptance.public.finance(root,work)==queued_finance and rows(root,'SELECT status FROM agents WHERE id=?',('retry-worker-session',))==[('queued',)]
                assert not (root/'retry-model-received.json').exists() and not (root/'retry-report.md').exists()
        status,ran=acceptance.public.native.invoke(binary,root,out,'run',['agent','run','retry-worker-session'],environment=environment);assert status==0 and ran['agent']['status']=='completed' and not ran['task_accepted'],ran
        witness=json.loads((root/'retry-model-received.json').read_text());assert witness['pid']==ran['agent']['child_pid'] and witness['ppid']==ran['agent']['supervisor_pid'] and witness['stdin_sha256']==acceptance.public.f.digest(prompt.encode())
        report=(root/'retry-report.md').read_bytes();assert witness['stdin_sha256'].encode() in report and (root/'handoff.md').read_bytes()==original_report and (root/'reviewer-invoked.json').read_bytes()==original_invocation
        completion=root/'.swarm'/('native-completion-'+acceptance.public.f.digest(b'retry-worker-session')+'.json');data=completion.read_bytes();value=json.loads(data);assert value['exit_code']==0 and not value['failed'] and not value['interrupted'] and value['expected']['id']=='retry-worker-session'
        first_agents=rows(root,'SELECT id FROM agents WHERE work_id=? AND task_id=? AND id!=?',(work,'task','retry-worker-session'));assert len(first_agents)==1
        first_completion=root/'.swarm'/('native-completion-'+acceptance.public.f.digest(first_agents[0][0].encode())+'.json');first_raw=first_completion.read_bytes();first_value=json.loads(first_raw);assert first_value['expected']['id']==first_agents[0][0] and first_value['exit_code']==0 and not first_value['failed']
        acceptance.public.f.publish(retained/'first-worker-completion.json',first_raw);acceptance.public.f.publish(retained/'first-worker-invocation.json',(root/'model-received.json').read_bytes())
        planner_pid=json.loads(body)['run']['child_pid'];acceptance.public.f.publish(retained/'planner-received-prompt.bin',(root/('fixture-prompt-'+str(planner_pid)+'.bin')).read_bytes());acceptance.public.f.publish(retained/'planner-invocation.json',(root/('model-received-'+str(planner_pid)+'.json')).read_bytes())
        for name,data in [('retry-worker-completion.json',data),('retry-report.md',report),('retry-worker-invocation.json',(root/'retry-model-received.json').read_bytes()),('inert-worker.py',worker_code.encode()),('retry-worker-prompt.bin',prompt.encode())]:acceptance.public.f.publish(retained/name,data)
        current=acceptance.public.preview.shown(binary,root,out,'after-worker',work)
        submit=dict(schema_version=1,event_id='retry-worker-submit',expected_revision=current['revision'],id='task',agent_id='retry-worker-session',report_path='retry-report.md',report_sha256=acceptance.public.f.digest(report))
        status,value=acceptance.public.native.invoke(binary,root,out,'submit',['task','submit',work],submit,environment=environment);assert status==0,value
        final=acceptance.public.preview.shown(binary,root,out,'final',work);assert len(final['tasks'])==1 and final['tasks'][0]['status']=='submitted' and len(final['tasks'][0]['attempts'])==2 and final['tasks'][0]['previous_reviews'][-1]==original_review and not final['tasks'][0].get('independent_review')
        final_finance=acceptance.public.finance(root,work);status,replay=acceptance.public.native.invoke(binary,root,out,'retry-decision-replay',['planning','reconcile',claim]);assert status==0 and replay['replayed'] and replay['decision_applied'],replay
        assert acceptance.public.preview.shown(binary,root,out,'after-replay',work)==final and acceptance.public.finance(root,work)==final_finance and (root/'handoff.md').read_bytes()==original_report
        acceptance.public.f.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,native_retry_decision=True,faults=a.faults,setup_signature=signature,binary_sha256=acceptance.public.f.digest(binary.read_bytes()),canonical_decision_corruption_refused=a.faults,unsupported_public_edit_refused=True,damaged_original_refused_before_prepare=a.faults,damaged_prior_original_refused_at_preflight=a.faults,same_agent_run_after_restore=a.faults,actual_second_worker=True,tasks=1,attempts=2,old_review_archived=True,old_report_preserved=True,new_report_submitted=True,second_review=False,task_accepted=False,real_model=False,authenticated_supervisor=False,global_watchdog=None));print('PASS native production retry with guarded second worker',flush=True)
    except BaseException:acceptance.public.f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
