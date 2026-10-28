#!/usr/bin/env python3
"""Close child and parent through actual owned inert native model processes.

Originals are retained before assertions. No positive SQL writes, provider
charges, global watchdog, human authentication or migration acceptance claim.
"""
import argparse,json,os,traceback,socket,selectors,select,struct,subprocess
from pathlib import Path
from types import SimpleNamespace
import rust_native_review_acceptance as acceptance
import rust_planning_worker_public_flow as flow
from rust_native_review_decision import rows


def held_decision(binary,root,work,out,scope,code):
    """Refuse an external close while an actual registered model is held alive.

    A socket/SO_PEERCRED/pidfd witnesses the real owned provider. Release and
    wait the same provider before requiring its own fenced close. No temporal
    watchdog, SQL mutation, foreign signalling or synthetic positive receipt.
    """
    environment={**os.environ,'CODEX_HOME':str(root/'codex-home')}
    before=acceptance.public.preview.shown(binary,root,out,'held-before-claim',work)
    current=next(s for s in before['planning']['scopes'] if s['id']==scope);claim='held-close-claim'
    request=dict(schema_version=1,event_id=claim,expected_revision=before['revision'],scope=scope,scope_revision=current['revision'],holder='owned-closure-planner',lease_seconds=120)
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
            external=dict(schema_version=1,event_id='external-close-active',expected_revision=baseline['revision'],scope=scope,scope_revision=current['revision'],holder=current['holder'],generation=current['generation'],reason='External attempt while actual native model remains alive',input_events=proposal['input_events'],operations=proposal['operations'])
            # All mutations use the same still-valid lease and current evidence.
            # Each malformed proposal must preserve business/finance/process state.
            variants=[('incomplete-inputs',dict(input_events=proposal['input_events'][:-1]),'planning-closure-input-pending'),
                ('unused-task-budget',dict(operations=[{**proposal['operations'][0],'max_tasks':1}]),'planning-closure-field-not-applicable'),
                ('mixed-close',dict(operations=proposal['operations']*2),'planning-closure-exclusive-operation')]
            assert len(proposal['input_events'])>1
            for label,patch,reason in variants:
                bad={**external,**patch,'event_id':'held-'+label}
                denied,value=acceptance.public.native.invoke(binary,root,out,'held-'+label,['planning','decide',work],bad,environment=environment)
                assert denied!=0 and reason in value['failure']['causes'],value
                assert acceptance.public.preview.shown(binary,root,out,'held-'+label+'-after',work)==baseline and acceptance.public.finance(root,work)==finance
            denied,value=acceptance.public.native.invoke(binary,root,out,'held-external-denied',['planning','decide',work],external,environment=environment)
            assert denied!=0 and 'planning-closure-native-run-active' in value['failure']['causes'],value
            assert acceptance.public.preview.shown(binary,root,out,'held-live-after',work)==baseline and acceptance.public.finance(root,work)==finance and rows(root,'SELECT phase FROM native_planning_runs WHERE claim_id=?',(claim,))==[('permitted',)]
            assert process.poll() is None and not select.select([childfd],[],[],0)[0]
            connection.sendall(b'1');connection.close();connection=None;status=process.wait();assert select.select([childfd],[],[],0)[0]
        raw=(out/('held-run.stdout' if status==0 else 'held-run.stderr')).read_bytes();answer=(status,dict(run=json.loads(raw)) if status==0 else json.loads(raw),code)
        acceptance.public.f.publish(out/'held-observation.json',dict(owner_pid=process.pid,child_pid=peer,external_close_refused_without_mutation=True,actual_child_pidfd_ready_after_wait=True,global_watchdog=None))
    except BaseException as error:errors.append(error)
    finally:
        if connection is not None:
            try:connection.sendall(b'1');connection.close()
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
    if errors:raise BaseExceptionGroup('Held closure and cleanup failed',errors)
    return answer


def close(binary,root,work,out,scope,held=False,unknown_grant=False):
    """Propose one close from pending public inputs and retain actual run bytes.

    The engine remains responsible for business, physical completion and commit
    guards. Failure preserves its real original and diagnostic before assertion.
    """
    before=acceptance.public.preview.shown(binary,root,out,scope+'-before',work)
    code=flow.feedback.PROGRAM.encode();acceptance.public.f.publish(root/'codex',code);(root/'codex').chmod(0o700)
    flow.proposal(SimpleNamespace(root=root),before,scope,[dict(kind='close',title='All owned requirements have current accepted evidence')])
    control=None;original_control=None
    if unknown_grant:
        controls=list((root/'.swarm').glob('native-validation-step-*.receipt'));assert len(controls)==1
        control=controls[0];original_control=control.read_bytes();changed=bytearray(original_control);changed[-1]^=1
        acceptance.public.f.publish(control,bytes(changed))
    try:
        if held:status,value,code=held_decision(binary,root,work,out,scope,code)
        else:status,value=acceptance.public.native.invoke(binary,root,out,scope+'-close',['planning','step',work],environment={**os.environ,'CODEX_HOME':str(root/'codex-home')})
    finally:
        if control is not None:acceptance.public.f.publish(control,original_control)
    actual=rows(root,'SELECT claim_id,phase,prepared_revision,receipt_sha256 FROM native_planning_runs WHERE work_id=? AND scope_id=? ORDER BY prepared_revision,claim_id',(work,scope));claim=actual[-1][0]
    path=root/'.swarm'/('native-planning-'+acceptance.public.f.digest(claim.encode())+'.receipt');raw=path.read_bytes();magic,extent,digest,body=raw.split(b'\n',3)
    assert magic==b'SR1' and len(body)==int(extent) and acceptance.public.f.digest(body)==digest.decode()
    retained=out/(scope+'-originals');retained.mkdir();acceptance.public.f.publish(retained/'original.frame',raw);acceptance.public.f.publish(retained/'inert-program.py',code);acceptance.public.f.publish(retained/'proposal.json',(root/'planning-proposal.json').read_bytes());acceptance.public.f.publish(retained/'retention.json',dict(root=str(root),work=work,row=actual[-1],frame_sha256=acceptance.public.f.digest(raw),binary_sha256=acceptance.public.f.digest(binary.read_bytes())))
    if unknown_grant:
        assert status!=0 and 'planning-closure-evidence-invalid' in value['failure']['causes'],value
        assert actual[-1][1]=='permitted',actual
        baseline=acceptance.public.preview.shown(binary,root,out,'recovery-before',work);money=acceptance.public.finance(root,work)
        missing_wait=json.loads(body);assert missing_wait['exit_code']==0;missing_wait['exit_code']=None
        wait_body=json.dumps(missing_wait,separators=(',',':'),ensure_ascii=False).encode();wait_frame=b'SR1\n'+f'{len(wait_body):020d}'.encode()+b'\n'+acceptance.public.f.digest(wait_body).encode()+b'\n'+wait_body
        acceptance.public.f.publish(retained/'unknown-wait-negative.frame',wait_frame)
        try:
            acceptance.public.f.publish(path,wait_frame)
            refused,diagnostic=acceptance.public.native.invoke(binary,root,out,'recovery-unknown-wait',['planning','reconcile',claim],environment={**os.environ,'CODEX_HOME':str(root/'codex-home')})
            assert refused!=0 and 'planning-receipt-not-successful' in diagnostic['failure']['causes'],diagnostic
            assert acceptance.public.preview.shown(binary,root,out,'recovery-unknown-wait-after',work)==baseline and rows(root,'SELECT phase,receipt_sha256 FROM native_planning_runs WHERE claim_id=?',(claim,))==[('permitted',None)] and acceptance.public.finance(root,work)==money
        finally:acceptance.public.f.publish(path,raw)
        missing=json.loads(body);assert missing.pop('grant_attempted') is True
        changed=json.dumps(missing,separators=(',',':'),ensure_ascii=False).encode();unknown=b'SR1\n'+f'{len(changed):020d}'.encode()+b'\n'+acceptance.public.f.digest(changed).encode()+b'\n'+changed
        acceptance.public.f.publish(retained/'unknown-grant-negative.frame',unknown)
        try:
            acceptance.public.f.publish(path,unknown)
            refused,diagnostic=acceptance.public.native.invoke(binary,root,out,'recovery-unknown-grant',['planning','reconcile',claim],environment={**os.environ,'CODEX_HOME':str(root/'codex-home')})
            assert refused!=0 and 'planning-closure-native-run-active' in diagnostic['failure']['causes'],diagnostic
            after_unknown=acceptance.public.preview.shown(binary,root,out,'recovery-unknown-after',work)
            observed=rows(root,'SELECT phase,receipt_sha256 FROM native_planning_runs WHERE claim_id=?',(claim,))
            acceptance.public.f.publish(out/'unknown-grant-observation.json',dict(refused=refused!=0,work_unchanged=after_unknown==baseline,rows=observed))
            assert after_unknown==baseline and observed==[('permitted',None)] and acceptance.public.finance(root,work)==money,dict(work_unchanged=after_unknown==baseline,rows=observed)
        finally:acceptance.public.f.publish(path,raw)
        status,value=acceptance.public.native.invoke(binary,root,out,'recovery-original',['planning','reconcile',claim],environment={**os.environ,'CODEX_HOME':str(root/'codex-home')})
        assert status==0 and value['decision_applied'],value
        assert acceptance.public.finance(root,work)==money and path.read_bytes()==raw
        actual=rows(root,'SELECT claim_id,phase,prepared_revision,receipt_sha256 FROM native_planning_runs WHERE claim_id=?',(claim,));value=dict(run=value)
    record=json.loads(body);child_pid=record['run']['child_pid'];assert child_pid is not None
    for label,source in [('received-prompt.bin',root/('fixture-prompt-'+str(child_pid)+'.bin')),('provider-witness.json',root/('model-received-'+str(child_pid)+'.json'))]:
        data=source.read_bytes();acceptance.public.f.publish(retained/label,data)
    witness=json.loads((retained/'provider-witness.json').read_text());prompt=(retained/'received-prompt.bin').read_bytes()
    assert witness['pid']==child_pid and witness['ppid']==record['run']['owner_pid'] and witness['prompt_sha256']==acceptance.public.f.digest(prompt)
    assert status==0 and value['run']['decision_applied'],dict(exit_code=status,failure=value.get('failure'))
    after=acceptance.public.preview.shown(binary,root,out,scope+'-after',work)
    assert next(s for s in after['planning']['scopes'] if s['id']==scope)['state']=='closed'
    assert actual[-1][1]=='applied'
    finance=acceptance.public.finance(root,work);prompts=set(root.glob('fixture-prompt-*.bin'))
    replay,replayed=acceptance.public.native.invoke(binary,root,out,scope+'-historical-replay',['planning','reconcile',claim])
    assert replay==0 and replayed['replayed'] and replayed['decision_applied'],replayed
    assert acceptance.public.preview.shown(binary,root,out,scope+'-after-replay',work)==after and acceptance.public.finance(root,work)==finance and set(root.glob('fixture-prompt-*.bin'))==prompts
    return after


def main():
    """Create real native accepted evidence, then close descendants before root."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--held',action='store_true');p.add_argument('--unknown-grant',action='store_true');a=p.parse_args();out=a.output.resolve();out.mkdir(exist_ok=False,parents=True);root=None
    acceptance.public.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();source=out/'source';root,work=acceptance.prepare(binary,a.profile.resolve(),source)
        acceptance.accept(binary,root,work,source,'closure-predecessor-accepted',True)
        child=close(binary,root,work,out,'child',held=a.held)
        assert any(e['kind']=='scope_closed' and e['scope']=='root' and not e.get('decision') for e in child['planning']['inbox'])
        final=close(binary,root,work,out,'root',unknown_grant=a.unknown_grant);assert all(s['state']=='closed' for s in final['planning']['scopes'])
        acceptance.public.f.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,child_and_root_closed=True,live_external_close_refused=a.held,unknown_grant_preserved=a.unknown_grant,real_model=False,authenticated_human=False,global_watchdog=None));print('PASS native child and root closure',flush=True)
    except BaseException:acceptance.public.f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
