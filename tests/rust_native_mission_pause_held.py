#!/usr/bin/env python3
"""Global pause reaches a genuinely permitted native planner and permits recovery.

Owned inert provider/Root only. No positive SQL/receipt seed, fixture signal,
provider charge, global watchdog or physical completion inferred from lease time.
"""
import argparse,json,os,traceback,socket,selectors,select,struct,subprocess
from pathlib import Path
import rust_native_review_acceptance as acceptance
import rust_planning_worker_public_flow as flow
from rust_native_review_decision import rows
from rust_planning_bootstrap import fresh_work,request
from rust_planning_activation import claim_request
from rust_planning_call_preview import preview_request


def held_mission_pause(binary,root,work,out,scope,code):
    """Pause during a genuine permitted model, then observe owned termination.

    A valid claim/preview and real SO_PEERCRED/pidfd/registry witness precede
    the public global pause. The engine's own freshness supervision must stop
    the blocked provider: this helper sends no signal or successful release.
    Wait without a global watchdog and retain both primary/cleanup failures.
    """
    environment={**os.environ,'CODEX_HOME':str(root/'codex-home')}
    before=acceptance.public.preview.shown(binary,root,out,'held-before-claim',work)
    current=next(s for s in before['planning']['scopes'] if s['id']==scope);claim='held-mission-claim'
    request=dict(schema_version=1,event_id=claim,expected_revision=before['revision'],scope=scope,scope_revision=current['revision'],holder='owned-mission-planner',lease_seconds=120)
    status,value=acceptance.public.native.invoke(binary,root,out,'held-claim',['planning','claim',work],request,environment=environment);assert status==0,value
    claimed=acceptance.public.preview.shown(binary,root,out,'held-claimed',work);current=next(s for s in claimed['planning']['scopes'] if s['id']==scope)
    request=dict(schema_version=1,expected_revision=claimed['revision'],claim_id=claim,scope=scope,scope_revision=current['revision'],holder=current['holder'],generation=current['generation'])
    proposal=json.loads((root/'planning-proposal.json').read_text());proposal['input_events']=current['delivery']['events'];acceptance.public.f.publish(root/'planning-proposal.json',proposal)
    status,preview=acceptance.public.native.invoke(binary,root,out,'held-preview',['planning','call-preview',work],request,environment=environment);assert status==0 and preview['prepared_inputs_verified'],preview
    barrier="import socket\nwith socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as gate:\n gate.connect('closure-fixture.sock')\n gate.sendall(b'R')\n assert gate.recv(1)==b'1'\n"
    marker=b"proposal=json.load(open('planning-proposal.json'))";assert marker in code
    code=code.replace(marker,barrier.encode()+marker);acceptance.public.f.publish(root/'codex',code);(root/'codex').chmod(0o700)
    acceptance.public.f.publish(out/'held-run.input.json',request)
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);selector=selectors.DefaultSelector();connection=None;process=None;pidfd=None;childfd=None;errors=[];answer=None
    try:
        listener.bind(str(root/'closure-fixture.sock'));listener.listen(1);listener.setblocking(False)
        with (out/'held-run.stdout').open('xb') as stdout,(out/'held-run.stderr').open('xb') as stderr:
            argv=[str(binary),'planning','run',work,'--root',str(root),'--input',str(out/'held-run.input.json'),'--json'];acceptance.public.f.publish(out/'held-run.command.json',dict(argv=argv,global_watchdog=None))
            process=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env={**os.environ,'CODEX_HOME':str(root/'codex-home')})
            pidfd=os.pidfd_open(process.pid);selector.register(pidfd,selectors.EVENT_READ,'exit');selector.register(listener,selectors.EVENT_READ,'provider')
            while connection is None:
                ready=selector.select()
                if any(key.data=='exit' for key,_ in ready):raise AssertionError('Owned planner exited before provider handshake')
                connection,_=listener.accept()
            peer,uid,_=struct.unpack('3i',connection.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert uid==os.getuid() and connection.recv(1)==b'R';childfd=os.pidfd_open(peer)
            actual=rows(root,'SELECT owner_pid,child_pid,permission,phase FROM native_planning_runs WHERE claim_id=?',(claim,));assert actual==[(process.pid,peer,1,'permitted')],actual
            baseline=acceptance.public.preview.shown(binary,root,out,'held-live-before',work);finance=acceptance.public.finance(root,work)
            current=next(s for s in baseline['planning']['scopes'] if s['id']=='root')
            pause=dict(schema_version=1,event_id='global-pause-during-model',expected_revision=baseline['revision'],reason='Revoke only actual owned planner; preserve original and spending')
            code_status,paused=acceptance.public.native.invoke(binary,root,out,'global-pause',['mission','pause',work],pause,environment=environment)
            assert code_status==0,paused
            scope_after=next(s for s in paused['planning']['scopes'] if s['id']=='root')
            assert scope_after['generation']==current['generation']+1 and not scope_after.get('holder')
            assert paused['planning']['activations']==baseline['planning']['activations'] and paused['planning']['decisions']==baseline['planning']['decisions'] and acceptance.public.finance(root,work)==finance
            acceptance.public.f.publish(out/'awaiting-owned-stop.json',dict(owner_pid=process.pid,child_pid=peer,global_pause_recorded=True,no_fixture_release_or_signal=True,global_watchdog=None))
            status=process.wait();assert select.select([childfd],[],[],0)[0]
            connection.close();connection=None
        raw=(out/('held-run.stdout' if status==0 else 'held-run.stderr')).read_bytes();answer=(status,dict(run=json.loads(raw)) if status==0 else json.loads(raw),code)
        acceptance.public.f.publish(out/'held-observation.json',dict(owner_pid=process.pid,child_pid=peer,public_global_pause_recorded=True,no_fixture_release_or_signal=True,actual_child_pidfd_ready_after_wait=True,global_watchdog=None))
    except BaseException as error:errors.append(error)
    finally:
        if connection is not None:
            try:connection.close()
            except BaseException as error:errors.append(error)
        if process is not None and process.poll() is None:
            try:process.wait()
            except BaseException as error:errors.append(error)
        for fd in [childfd,pidfd]:
            if fd is not None:
                try:os.close(fd)
                except BaseException as error:errors.append(error)
        for resource in [selector,listener]:
            try:resource.close()
            except BaseException as error:errors.append(error)
    if errors:raise BaseExceptionGroup('Held mission pause and cleanup failed',errors)
    return answer

def main():
    """Retain the actual failed original before requiring no business verdict.

    Known native wait/registered PIDs/current controls protect recovery. Unpause
    preserves consent; genuine failure reconciliation may record refusal but
    never success. A new real claim/preview must work without refunding the old
    estimate or rewriting its receipt. No new model call is made for recovery.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:p.add_argument('--'+name,required=True,type=Path)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None;f=acceptance.public.f;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes());assert profile['no_fixture_feature'] and f.digest(binary.read_bytes())==profile['sha256']
        owned=f.Fixture(binary,out/'source','mission-held');owned.work=fresh_work(owned,'held-create');work=owned.ok(['planning','enable',owned.work],request(event='held-enable',max_activations=3,max_decisions=3,max_judge_cycles=2))
        flow.proposal(owned,work,'root',[dict(kind='judge',judge_cycle=1,judge_verdict='continue')])
        status,value,code=held_mission_pause(binary,owned.root,owned.work,out,'root',flow.feedback.PROGRAM.encode())
        receipt=owned.root/'.swarm'/('native-planning-'+f.digest(b'held-mission-claim')+'.receipt');raw=receipt.read_bytes();f.publish(out/'original.frame',raw);f.publish(out/'inert-provider.py',code)
        magic,extent,sha,body=raw.split(b'\n',3);assert magic==b'SR1' and len(body)==int(extent) and f.digest(body)==sha.decode()
        original=json.loads(body);f.publish(out/'post-wait-observation.json',dict(exit=status,response=value,rows=rows(owned.root,'SELECT claim_id,phase,permission,receipt_sha256,refusal_code FROM native_planning_runs WHERE work_id=?',(owned.work,))))
        assert original['exit_code'] is not None or original['signal'] is not None
        assert status!=0 and original['output_complete'] is False and original['failure'] is not None,value
        assert any('stale_lease' in cause['labels'] for cause in original['failure_causes']),original['failure_causes']
        pid=original['run']['child_pid'];prompt=(owned.root/('fixture-prompt-'+str(pid)+'.bin')).read_bytes();witness=json.loads((owned.root/('model-received-'+str(pid)+'.json')).read_bytes());f.publish(out/'received-prompt.bin',prompt);f.publish(out/'provider-witness.json',witness)
        assert witness['pid']==pid and witness['prompt_sha256']==f.digest(prompt)==original['run']['prompt_sha256']
        final=owned.ok(['work','show',owned.work])['work'];assert not final['planning']['judge'].get('verdict') and final['planning']['decisions']==0 and not final.get('tasks') and owned.ok(['mission','show',owned.work])['paused']
        old_money=acceptance.public.finance(owned.root,owned.work);unpaused=owned.ok(['mission','unpause',owned.work],dict(schema_version=1,event_id='held-unpause',expected_revision=final['revision'],reason='Observe original known wait before requesting a fresh claim'))
        state=rows(owned.root,'SELECT phase FROM native_planning_runs WHERE claim_id=?',('held-mission-claim',))[0][0]
        if state!='refused':
            refused,diagnostic=owned.call(['planning','reconcile','held-mission-claim']);assert refused!=0 and 'planning-receipt-not-successful' in diagnostic['failure']['causes'],diagnostic
        assert rows(owned.root,'SELECT phase FROM native_planning_runs WHERE claim_id=?',('held-mission-claim',))==[('refused',)]
        assert receipt.read_bytes()==raw and acceptance.public.finance(owned.root,owned.work)==old_money
        current=owned.ok(['work','show',owned.work])['work'];scope=next(s for s in current['planning']['scopes'] if s['id']=='root');fresh_id='post-pause-current-claim';fresh=owned.ok(['planning','claim',owned.work],claim_request(current,fresh_id,scope_revision=scope['revision']));assert owned.ok(['planning','call-preview',owned.work],preview_request(fresh,fresh_id))['prepared_inputs_verified']
        assert fresh['planning']['activations']==2 and fresh['planning']['decisions']==0 and not fresh['planning']['judge'].get('verdict') and receipt.read_bytes()==raw and all(row in acceptance.public.finance(owned.root,owned.work) for row in old_money)
        f.publish(out/'result.json',dict(status='PASS',root=str(owned.root),work=owned.work,binary_sha256=profile['sha256'],actual_live_provider_revoked_by_public_mission_pause=True,no_fixture_signal_or_release=True,known_wait_and_original_preserved=True,logical_verdict_not_applied=True,actual_refused_physical_row=True,public_unpause_fresh_claim_preview=True,no_refund=True,new_model_calls_after_failure=0,global_watchdog=None,installed=False,authenticated_actor=False))
        print('PASS actual global mission pause stops owned native planner and permits evidence-bound recovery',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
