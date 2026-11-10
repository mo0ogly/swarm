#!/usr/bin/env python3
"""Actual scoped worker failure and explicit same-task native retry, no review.

Owned inert programs, public creation only, no SQL seed or paid provider. Keep
actual completions and failed Roots; a message/PID never proves a kernel wait.
"""
import argparse,json,os,traceback
from pathlib import Path
from contextlib import contextmanager
import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as flow
from rust_native_production_retry import negative_frame
from rust_planning_bootstrap import request as enable_request
from rust_native_review_decision import rows


@contextmanager
def missing_original(path,original):
    """Remove only our archived exact original and join refusal/restore causes.

    Caller retains bytes in its immutable evidence before entry. An unexpected
    replacement is never overwritten. This negative fixture creates no receipt,
    ownership, success state or permission; normal observation must fail closed.
    """
    assert path.read_bytes()==original
    path.unlink();errors=[]
    try:yield
    except BaseException as error:errors.append(error)
    try:
        if path.exists():raise ValueError('Unexpected completion appeared; refusing blind restoration')
        fixture.publish(path,original)
        assert path.read_bytes()==original
    except BaseException as error:errors.append(error)
    if errors:raise BaseExceptionGroup('Missing original probe and restoration failed',errors)


def setup(binary,out,exit_code):
    """Create a real Root/task/first process with an explicit saved budget.

    Exit0 is the control; exit17 changes only the fixture program's final exit.
    The first report is a raw artifact, never a submitted/accepted task result.
    Return genuine public state and retain original completion/witness first.
    """
    owned=fixture.Fixture(binary,out,'worker-failure-retry');fixture.publish(owned.root/'codex',flow.feedback.PROGRAM.encode());(owned.root/'codex').chmod(0o700)
    value=owned.ok(['work','create'],dict(schema_version=1,event_id='failure-work',expected_revision=0,title='Owned worker failure recovery',objective='Preserve actual attempts and retry only after observed native termination',scope='Owned process evidence only',criteria=['Preserve history and emit the current actual fixture input digest']))['work'];owned.work=value['id']
    checks={'req-1':[dict(id='real-report',command=['python3','-c',"from pathlib import Path; import re; assert re.search(r'Input SHA256: [0-9a-f]{64}',Path('result.md').read_text())"],criteria=[1],inputs=['result.md'],justification='Verify actual report input digest after future submission')]}
    enabled=owned.ok(['planning','enable',owned.work],enable_request(event='failure-enable',max_tasks=1,checks=checks))
    flow.proposal(owned,enabled,'root',[dict(kind='task',id='task',title='Owned recovery task',next='Produce a report from the current received input',requirements=['req-1'],deliverable='result.md',criteria=['Return current input evidence'])]);created=owned.ok(['planning','step',owned.work]);assert created['run']['decision_applied']
    current=created['run']['work'];owned.ok(['budget','apply',owned.work],dict(schema_version=1,event_id='failure-budget',expected_revision=current['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=1,estimate_source='Owned inert worker failure fixture, not measured pricing',reference_date='2026-10-22')))
    program=flow.mission.PROGRAM.replace("'model-received.json'","'first-worker-received.json'").replace("os.replace(name,'handoff.md')","os.replace(name,'first-report.md')")+'\nsys.exit('+str(exit_code)+')\n'
    fixture.publish(owned.root/'codex',program.encode());(owned.root/'codex').chmod(0o700)
    before=owned.ok(['work','show',owned.work])['work'];r=dict(schema_version=1,event_id='first-worker',expected_revision=before['revision'],task_id='task',provider='fixture',capture_output=True)
    prepared=owned.ok(['agent','preview',owned.work],r);r['model_policy_hash']=prepared['agent']['model_route']['policy_hash'];prepared=owned.ok(['agent','prepare',owned.work],r)
    status,result=owned.call(['agent','run','first-worker']);current=owned.ok(['work','show',owned.work])['work'];actual=rows(owned.root,'SELECT status FROM agents WHERE id=?',('first-worker',));expected='completed' if exit_code==0 else 'failed';assert actual==[(expected,)] and current['tasks'][0]['attempts'][-1]['status']==expected
    completion=owned.root/'.swarm'/('native-completion-'+fixture.digest(b'first-worker')+'.json');raw=completion.read_bytes();original=json.loads(raw);assert original['expected']['id']=='first-worker' and original['exit_code']==exit_code and original['failed']==(exit_code!=0) and not original['incomplete_output'] and original['details']['kernel_reaped'] and original['details']['output_complete']
    witness=json.loads((owned.root/'first-worker-received.json').read_text());assert witness['pid']==original['expected']['child_pid'] and witness['ppid']==original['expected']['supervisor_pid'] and witness['stdin_sha256']==fixture.digest(prepared['agent']['prompt'].encode())
    fixture.publish(out/'original-completion.json',raw);fixture.publish(out/'first-worker-program.py',program.encode());fixture.publish(out/'first-worker-prompt.bin',prepared['agent']['prompt'].encode());fixture.publish(out/'first-worker-witness.json',(owned.root/'first-worker-received.json').read_bytes());fixture.publish(out/'first-report.md',(owned.root/'first-report.md').read_bytes())
    fixture.publish(out/'fixture.json',dict(root=str(owned.root),work=owned.work,first_exit=exit_code,binary_sha256=fixture.digest(binary.read_bytes()),run_cli_exit=status))
    return owned,current


def main():
    """Require healthy actual completion before the changed exit-status case.

    Preserve the real failed worker and native planner original before requiring
    retry and a second worker/report on the same task. Fault mode requires a full
    healthy retry first; corrupt only an owned completion, restore exact bytes
    and run the same queued agent. No task acceptance/actor/autonomy is inferred.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--failure',action='store_true');p.add_argument('--faults',action='store_true');p.add_argument('--control',type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None;fixture.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes());assert profile['all_core_features_empty'] and profile['all_core_test_false'] and fixture.digest(binary.read_bytes())==profile['sha256']
        owned,work=setup(binary,out/'source',17 if a.failure else 0);task=work['tasks'][0];assert task['status']=='blocked' and not task.get('independent_review') and len(task['attempts'])==1
        signature=dict(settings=fixture.digest(json.dumps(owned.ok(['engine-settings','show'])['values'],sort_keys=True).encode()),contract=fixture.digest(json.dumps({k:task.get(k) for k in ['title','deliverable','criteria','depends','plan_max_attempts']},sort_keys=True).encode()))
        if not a.failure:
            fixture.publish(out/'result.json',dict(status='PASS_INITIAL_WORKER_CONTROL',root=str(owned.root),work=owned.work,exit_code=0,actual_worker=True,signature=signature,binary_sha256=fixture.digest(binary.read_bytes())));print('PASS healthy initial worker before failure injection',flush=True);return
        assert a.control is not None;control=json.loads(a.control.read_bytes());assert control['status'] in ['PASS_INITIAL_WORKER_CONTROL','PASS'] and control['actual_worker'] and control['signature']==signature and control['binary_sha256']==fixture.digest(binary.read_bytes())
        assert any(e['kind']=='attempt_ended' and e['task']=='task' and e['attempt']==task['attempts'][0]['id'] and not e.get('decision') for e in work['planning']['inbox'])
        fixture.publish(owned.root/'codex',flow.feedback.PROGRAM.encode());(owned.root/'codex').chmod(0o700)
        operation=dict(kind='retry',id='task',next='Inspect the actual failed native completion, correct the same task and emit a new result.md with current input evidence')
        flow.proposal(owned,work,'root',[operation])
        if a.faults:
            assert control.get('actual_second_worker') and not control.get('faults') and control.get('full_binary_sha256')==fixture.digest(binary.read_bytes()),'Full healthy retry required before injection'
            completion=owned.root/'.swarm'/('native-completion-'+fixture.digest(b'first-worker')+'.json');original=completion.read_bytes()
            old_runs=rows(owned.root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? ORDER BY claim_id',(owned.work,));old_money=rows(owned.root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(owned.work,));old_prompts=set(owned.root.glob('fixture-prompt-*.bin'))
            unknown=json.loads(original);unknown['details']['kernel_reaped']=False
            gate=json.loads(original);gate['details']['private_gate_eof']=False
            projection=json.loads(original);projection['exit_code']=18
            cases=[('unknown-wait',json.dumps(unknown).encode(),'agent-completion-wait-not-confirmed'),('unknown-gate',json.dumps(gate).encode(),'agent-completion-gate-not-confirmed'),('changed-terminal',json.dumps(projection).encode(),'agent-completion-current-state-changed'),('invalid-original',b'{','agent-completion-journal-invalid')]
            for name,changed,reason in cases:
                fixture.publish(out/(name+'-negative.json'),changed)
                with negative_frame(completion,original,changed):
                    status,refused=owned.call(['planning','step',owned.work]);assert status!=0 and reason in refused['failure']['causes'],refused
                    assert owned.ok(['work','show',owned.work])['work']==work
                    assert rows(owned.root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? ORDER BY claim_id',(owned.work,))==old_runs
                    assert rows(owned.root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(owned.work,))==old_money and set(owned.root.glob('fixture-prompt-*.bin'))==old_prompts
            with missing_original(completion,original):
                status,refused=owned.call(['planning','step',owned.work]);assert status!=0 and 'agent-completion-journal-missing' in refused['failure']['causes'],refused
                assert owned.ok(['work','show',owned.work])['work']==work
                assert rows(owned.root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? ORDER BY claim_id',(owned.work,))==old_runs
                assert rows(owned.root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(owned.work,))==old_money and set(owned.root.glob('fixture-prompt-*.bin'))==old_prompts
        status,value=owned.call(['planning','step',owned.work])
        actual=rows(owned.root,'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? ORDER BY prepared_revision,claim_id',(owned.work,));claim=actual[-1][0];path=owned.root/'.swarm'/('native-planning-'+fixture.digest(claim.encode())+'.receipt');raw=path.read_bytes();magic,extent,digest,body=raw.split(b'\n',3);assert magic==b'SR1' and len(body)==int(extent) and fixture.digest(body)==digest.decode();fixture.publish(out/'planner-original.frame',raw);fixture.publish(out/'planner-program.py',flow.feedback.PROGRAM.encode());fixture.publish(out/'planner-proposal.json',(owned.root/'planning-proposal.json').read_bytes());fixture.publish(out/'planner-observation.json',dict(root=str(owned.root),work=owned.work,rows=actual))
        assert status==0 and value['run']['decision_applied'],dict(exit_code=status,failure=value.get('failure'))
        after=owned.ok(['work','show',owned.work])['work'];assert after['tasks'][0]['planning_retry'] and after['tasks'][0]['attempts']==task['attempts'] and len(after['tasks'])==1
        # The actual supervisor received safe metadata bound to the exact
        # original. Neither a message nor the fixture's desired exit grants retry.
        planner=json.loads(body)['run'];pid=planner['child_pid'];prompt=(owned.root/('fixture-prompt-'+str(pid)+'.bin')).read_bytes()
        original=(out/'source/original-completion.json').read_bytes()
        assert b'worker_completions' in prompt and b'first-worker' in prompt and fixture.digest(original).encode() in prompt
        fixture.publish(out/'planner-prompt.bin',prompt)
        instruction=operation['next'];first_report=(owned.root/'first-report.md').read_bytes()
        program=flow.mission.PROGRAM.replace("'model-received.json'","'second-worker-received.json'").replace("os.replace(name,'handoff.md')","os.replace(name,'second-report.md')")
        fixture.publish(owned.root/'codex',program.encode());(owned.root/'codex').chmod(0o700)
        request=dict(schema_version=1,event_id='second-worker',expected_revision=after['revision'],task_id='task',provider='fixture',capture_output=True)
        preview=owned.ok(['agent','preview',owned.work],request);request['model_policy_hash']=preview['agent']['model_route']['policy_hash']
        prepared=owned.ok(['agent','prepare',owned.work],request);queued=owned.ok(['work','show',owned.work])['work']
        assert len(queued['tasks'])==1 and len(queued['tasks'][0]['attempts'])==2 and queued['tasks'][0]['attempts'][0]==task['attempts'][0]
        assert instruction in prepared['agent']['prompt'] and not queued['tasks'][0].get('independent_review')
        completion=owned.root/'.swarm'/('native-completion-'+fixture.digest(b'first-worker')+'.json')
        if a.faults:
            assert control.get('actual_second_worker') and control.get('full_binary_sha256')==fixture.digest(binary.read_bytes()),'Full healthy retry required before injecting receipt damage'
            damaged=json.loads(original);damaged['details']['kernel_reaped']=False;changed=json.dumps(damaged,separators=(',',':')).encode()
            finance=rows(owned.root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(owned.work,))
            with negative_frame(completion,original,changed):
                status,refused=owned.call(['agent','run','second-worker'])
                assert status!=0 and 'agent-completion-wait-not-confirmed' in refused['failure']['causes'],refused
                assert owned.ok(['work','show',owned.work])['work']==queued and rows(owned.root,'SELECT status FROM agents WHERE id=?',('second-worker',))==[('queued',)]
                assert rows(owned.root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(owned.work,))==finance
                assert not (owned.root/'second-worker-received.json').exists()
            fixture.publish(out/'negative-completion.json',changed)
        status,ran=owned.call(['agent','run','second-worker']);assert status==0 and ran['agent']['status']=='completed' and not ran['task_accepted'],ran
        second=owned.root/'.swarm'/('native-completion-'+fixture.digest(b'second-worker')+'.json');value=json.loads(second.read_bytes());assert value['exit_code']==0 and not value['failed'] and value['details']['kernel_reaped'] and value['details']['output_complete']
        witness=json.loads((owned.root/'second-worker-received.json').read_bytes());assert witness['pid']==value['expected']['child_pid'] and witness['ppid']==value['expected']['supervisor_pid'] and witness['stdin_sha256']==fixture.digest(prepared['agent']['prompt'].encode())
        report=(owned.root/'second-report.md').read_bytes();assert witness['stdin_sha256'].encode() in report and (owned.root/'first-report.md').read_bytes()==first_report and completion.read_bytes()==original
        for name,data in [('second-completion.json',second.read_bytes()),('second-worker-program.py',program.encode()),('second-worker-prompt.bin',prepared['agent']['prompt'].encode()),('second-worker-witness.json',(owned.root/'second-worker-received.json').read_bytes()),('second-report.md',report)]:fixture.publish(out/name,data)
        current=owned.ok(['work','show',owned.work])['work'];submitted=owned.ok(['task','submit',owned.work],dict(schema_version=1,event_id='second-submit',expected_revision=current['revision'],id='task',agent_id='second-worker',report_path='second-report.md',report_sha256=fixture.digest(report)))
        final=owned.ok(['work','show',owned.work])['work'];assert final['tasks'][0]['status']=='submitted' and len(final['tasks'][0]['attempts'])==2 and final['tasks'][0]['attempts'][0]==task['attempts'][0]
        money=rows(owned.root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(owned.work,))
        replay=owned.ok(['planning','reconcile',claim]);assert replay['replayed'] and replay['decision_applied']
        assert owned.ok(['work','show',owned.work])['work']==final and rows(owned.root,'SELECT agent_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id',(owned.work,))==money
        fixture.publish(out/'result.json',dict(status='PASS',root=str(owned.root),work=owned.work,signature=signature,binary_sha256=fixture.digest(binary.read_bytes()),full_binary_sha256=fixture.digest(binary.read_bytes()),first_worker_failed=True,actual_worker=True,actual_planner=True,actual_second_worker=True,faults=a.faults,unknown_wait_refused=a.faults,pre_model_faults=5 if a.faults else 0,same_queued_agent_after_restore=a.faults,first_original_preserved=True,report_submitted=True,task_accepted=False,tasks=1,attempts=2,real_model=False,authenticated_actor=False,global_watchdog=None));print('PASS actual failed worker, native retry and second report with retained history',flush=True)
    except BaseException:fixture.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
