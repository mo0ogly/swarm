#!/usr/bin/env python3
"""Actual native planner acts on a verified review; stale dependency blocks exec.

Owned inert processes/Root only, no paid provider, SQL seed, global watchdog,
real mission, authenticated model/human or automatic overall acceptance claim.
"""
import argparse,json,os,sqlite3,traceback
from pathlib import Path
from types import SimpleNamespace
import rust_native_review_acceptance as acceptance
import rust_planning_worker_public_flow as flow


def rows(root,sql,parameters):
    """Observe owned SQL read-only and preserve independent read/close causes."""
    db=sqlite3.connect('file:'+str(root/'.swarm/state.db')+'?mode=ro',uri=True);primary=None;value=None
    try:value=db.execute(sql,parameters).fetchall()
    except BaseException as error:primary=error
    try:db.close()
    except BaseException as error:
        if primary is not None:raise BaseExceptionGroup('Fixture read and close failed',[primary,error])
        raise
    if primary is not None:raise primary
    return value


def main():
    """Retain actual planner original before requiring a business decision.

    A finite explicit child budget2 covers two tasks; it is not a new product
    default. Corrupt only the owned predecessor's control frame after a valid
    first worker/review/acceptance, require no new attempt/reservation, restore
    exact bytes, then run a real dependent worker and submit its own report.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--review-case',choices=['passed','changes-requested','domain-invalid'],default='passed');a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);binary=a.binary.resolve();root=None
    acceptance.public.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        source=out/'source';root,work=acceptance.prepare(binary,a.profile.resolve(),source,review_case=a.review_case,child_task_budget=2)
        if a.review_case=='passed':acceptance.accept(binary,root,work,source,'predecessor-accepted',True)
        before=acceptance.public.preview.shown(binary,root,out,'before-planner',work);original_invocation=(root/'reviewer-invoked.json').read_bytes();original_report=(root/'handoff.md').read_bytes();old_finance=acceptance.public.finance(root,work)
        code=flow.feedback.PROGRAM.encode();acceptance.public.f.publish(root/'codex',code);(root/'codex').chmod(0o700)
        operation=dict(kind='task',id='followup',title='Owned review follow-up',requirements=['req-1'],depends=['task'],deliverable='followup.md',criteria=['Emit a separate report while the original dependency remains current'],next='Observe the accepted predecessor, preserve handoff.md, emit followup.md')
        flow.proposal(SimpleNamespace(root=root),before,'child',[operation]);environment={**os.environ,'CODEX_HOME':str(root/'codex-home')};prompts=set(root.glob('fixture-prompt-*.bin'))
        if a.review_case=='passed':
            # Full current input proof is checked before a new claim/model call.
            control_frames=list((root/'.swarm').glob('native-validation-step-*.receipt'));assert len(control_frames)==1
            control_path=control_frames[0];control_original=control_path.read_bytes();bad=bytearray(control_original);bad[-1]^=1
            old_runs=rows(root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? ORDER BY claim_id',(work,))
            try:
                acceptance.public.f.publish(control_path,bytes(bad))
                stopped,diagnostic=acceptance.public.native.invoke(binary,root,out,'planner-stale-input',['planning','step',work],environment=environment)
                assert stopped!=0 and 'storage-operation:Read' in diagnostic['failure']['causes'],diagnostic
                # Full public rendering itself validates the accepted evidence and
                # may refuse while damaged. Read the revision, then compare full Work
                # after exact file restoration; never convert that refusal to empty Work.
                assert rows(root,'SELECT revision FROM works WHERE id=?',(work,))==[(before['revision'],)] and acceptance.public.finance(root,work)==old_finance
                assert rows(root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? ORDER BY claim_id',(work,))==old_runs and set(root.glob('fixture-prompt-*.bin'))==prompts
            finally:acceptance.public.f.publish(control_path,control_original)
            assert acceptance.public.preview.shown(binary,root,out,'after-stale-planner-restored',work)==before
        status,result=acceptance.public.native.invoke(binary,root,out,'planner-step',['planning','step',work],environment=environment)
        # Even an unported business decision may already have a real original.
        actual=rows(root,'SELECT claim_id,phase,prepared_revision,receipt_sha256 FROM native_planning_runs WHERE work_id=? AND scope_id=? ORDER BY prepared_revision,claim_id',(work,'child'));claim=actual[-1][0]
        frame=root/'.swarm'/('native-planning-'+acceptance.public.f.digest(claim.encode())+'.receipt');raw=frame.read_bytes();magic,extent,digest,body=raw.split(b'\n',3);assert magic==b'SR1' and len(body)==int(extent) and acceptance.public.f.digest(body)==digest.decode();retained=out/'planner-originals';retained.mkdir();acceptance.public.f.publish(retained/'original.frame',raw);acceptance.public.f.publish(retained/'inert-program.py',code);acceptance.public.f.publish(retained/'proposal.json',(root/'planning-proposal.json').read_bytes());acceptance.public.f.publish(retained/'retention.json',dict(root=str(root),work=work,claim=claim,row=actual[-1],frame_sha256=acceptance.public.f.digest(raw),body_sha256=digest.decode(),normal_binary_sha256=acceptance.public.f.digest(binary.read_bytes())))
        received=set(root.glob('fixture-prompt-*.bin'))-prompts;assert len(received)==1;prompt=next(iter(received)).read_bytes();acceptance.public.f.publish(retained/'received-prompt.bin',prompt)
        if a.review_case!='passed':
            assert status!=0,result
            # Known current protocol evidence is now available for reviewer-only
            # correction; a task creation still requires an applicable passed review.
            expected='planning-native-review-current-passed-required'
            assert expected in result['failure']['causes'],result
            after=acceptance.public.preview.shown(binary,root,out,'negative-after',work)
            assert not any(t['id']=='followup' for t in after['tasks']) and after['tasks'][0]['status']!='accepted'
            assert (root/'reviewer-invoked.json').read_bytes()==original_invocation and (root/'handoff.md').read_bytes()==original_report
            money=acceptance.public.finance(root,work);assert len(money)==len(old_finance)+1 and all(r in money for r in old_finance)
            acceptance.public.f.publish(out/'result.json',dict(status='PASS',review_case=a.review_case,native_planner_process_ran=True,business_operation_refused=True,refusal=expected,planner_original_retained=True,worker_started=False,original_reviewer_reexecuted=False,real_model=False,authenticated_human=False,global_watchdog=None));print('PASS non-favorable review active decision refused '+a.review_case,flush=True);return
        assert status==0 and result['run']['decision_applied'],dict(exit_code=status,failure=result.get('failure'),status=result.get('status'))
        after=acceptance.public.preview.shown(binary,root,out,'after-planner',work);task=next(t for t in after['tasks'] if t['id']=='followup');assert task['scope_id']=='child' and task['status']=='todo' and task['depends']==['task']
        review=after['tasks'][0]['independent_review'];assert review['id'].encode() in prompt and b'ROOT-PRIVATE-TITLE' not in prompt and after['tasks'][0]['status']=='accepted'
        consumed=[e for e in after['planning']['inbox'] if e['id']==review['id']];assert len(consumed)==1 and consumed[0].get('decision')
        money=acceptance.public.finance(root,work);assert len(money)==len(old_finance)+1 and all(r in money for r in old_finance)
        assert (root/'reviewer-invoked.json').read_bytes()==original_invocation and (root/'handoff.md').read_bytes()==original_report
        # New worker has its own output/witness, and cannot overwrite old evidence.
        worker=flow.mission.PROGRAM.replace("'model-received.json'","'followup-model-received.json'").replace("os.replace(name,'handoff.md')","os.replace(name,'followup.md')")
        assert worker!=flow.mission.PROGRAM;acceptance.public.f.publish(root/'codex',worker.encode());(root/'codex').chmod(0o700)
        prepared_request=dict(schema_version=1,event_id='followup-session',expected_revision=after['revision'],task_id='followup',provider='fixture',capture_output=True)
        # A valid preview witnesses the same input before the corruption.
        status,preview=acceptance.public.native.invoke(binary,root,out,'worker-valid-preview',['agent','preview',work],prepared_request,environment=environment);assert status==0 and preview['preview_only'],preview
        prepared_request['model_policy_hash']=preview['agent']['model_route']['policy_hash']
        controls=list((root/'.swarm').glob('native-validation-step-*.receipt'));assert len(controls)==1;path=controls[0];original=path.read_bytes();changed=bytearray(original);changed[-1]^=1
        reserve=lambda:rows(root,'SELECT state,amount FROM reservations WHERE agent_id=?',('followup-session',))
        assert reserve()==[]
        try:
            acceptance.public.f.publish(path,bytes(changed));status,refused=acceptance.public.native.invoke(binary,root,out,'worker-stale-refused',['agent','prepare',work],prepared_request,environment=environment)
            assert status!=0 and 'storage-operation:Read' in refused['failure']['causes'],refused
            assert rows(root,'SELECT revision FROM works WHERE id=?',(work,))==[(after['revision'],)] and reserve()==[] and not (root/'followup-model-received.json').exists() and acceptance.public.finance(root,work)==money
        finally:acceptance.public.f.publish(path,original)
        assert acceptance.public.preview.shown(binary,root,out,'after-worker-restored',work)==after
        status,prepared=acceptance.public.native.invoke(binary,root,out,'worker-prepare',['agent','prepare',work],prepared_request,environment=environment);assert status==0,prepared
        status,ran=acceptance.public.native.invoke(binary,root,out,'worker-run',['agent','run','followup-session'],environment=environment);assert status==0 and ran['agent']['status']=='completed' and not ran['task_accepted'],ran
        witness=json.loads((root/'followup-model-received.json').read_text());assert witness['pid']==ran['agent']['child_pid'] and witness['ppid']==ran['agent']['supervisor_pid'] and witness['stdin_sha256']==acceptance.public.f.digest(prepared['agent']['prompt'].encode())
        assert 'ROOT-PRIVATE-TITLE' not in prepared['agent']['prompt']
        current=acceptance.public.preview.shown(binary,root,out,'after-worker-run',work);report=(root/'followup.md').read_bytes();assert witness['stdin_sha256'].encode() in report
        submit=dict(schema_version=1,event_id='followup-submit',expected_revision=current['revision'],id='followup',agent_id='followup-session',report_path='followup.md',report_sha256=acceptance.public.f.digest(report))
        status,submitted=acceptance.public.native.invoke(binary,root,out,'followup-submit',['task','submit',work],submit,environment=environment);assert status==0,submitted
        final=acceptance.public.preview.shown(binary,root,out,'final',work);assert next(t for t in final['tasks'] if t['id']=='followup')['status']=='submitted' and final['tasks'][0]['status']=='accepted' and (root/'handoff.md').read_bytes()==original_report and (root/'reviewer-invoked.json').read_bytes()==original_invocation
        # Keep every new process original before PASS, not only in /tmp.
        archived=out/'followup-originals';archived.mkdir();paths={'completion.json':root/'.swarm'/('native-completion-'+acceptance.public.f.digest(b'followup-session')+'.json'),'invocation.json':root/'followup-model-received.json','report.md':root/'followup.md','inert-worker.py':root/'codex'};hashes={}
        for label,path in paths.items():
            assert path.is_file() and not path.is_symlink();data=path.read_bytes();acceptance.public.f.publish(archived/label,data);assert path.read_bytes()==data;hashes[label]=acceptance.public.f.digest(data)
        original=json.loads((archived/'completion.json').read_text());assert original['schema_version']==1 and original['expected']['id']=='followup-session' and original['exit_code']==0 and not original['failed'] and not original['interrupted'] and original['may_have_started']
        acceptance.public.f.publish(archived/'retention.json',dict(root=str(root),work=work,normal_binary_sha256=acceptance.public.f.digest(binary.read_bytes()),files=hashes,model_authenticated=False))
        acceptance.public.f.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,review_event_consumed=True,native_planner_process_decision_applied=True,inert_provider=True,real_model=False,new_worker_original_retained=True,followup_created=True,stale_dependency_refused_before_reservation=True,actual_followup_worker=True,own_report_submitted=True,followup_accepted=False,original_reviewer_reexecuted=False,authenticated_human=False,paid_provider=False,global_watchdog=None));print('PASS native review decision and dependency guarded worker',flush=True)
    except BaseException:acceptance.public.f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
