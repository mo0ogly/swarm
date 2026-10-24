#!/usr/bin/env python3
"""Direct cycle actions refuse while the actual owned native planner is alive.

Inert local provider, normal production profile, no paid call or private seed.
The lease duration is an explicit fixture input, not a verification watchdog.
"""
import argparse,json,os,traceback,socket,selectors,select,struct,subprocess
from pathlib import Path
import rust_native_review_acceptance as acceptance
import rust_planning_worker_public_flow as flow
from rust_native_review_decision import rows
from rust_planning_bootstrap import fresh_work,request


def held_judge(binary,root,work,out,scope,code):
    """Witness direct-command rejection against real native process ownership.

    Current root claim/preview and a SO_PEERCRED/pidfd handshake establish the
    valid live provider before either injection. Both commands preserve Work,
    finance and the permitted row. Release the owned provider, wait and require
    its original leased judge operation to apply. No SQL writes or watchdog.
    Primary and cleanup failures are retained together; no foreign signalling.
    """
    environment={**os.environ,'CODEX_HOME':str(root/'codex-home')}
    before=acceptance.public.preview.shown(binary,root,out,'held-before-claim',work)
    current=next(s for s in before['planning']['scopes'] if s['id']==scope);claim='held-judge-claim'
    request=dict(schema_version=1,event_id=claim,expected_revision=before['revision'],scope=scope,scope_revision=current['revision'],holder='owned-judge-planner',lease_seconds=120)
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
            for action in ['judge','renew']:
                external=dict(schema_version=1,event_id='external-'+action,expected_revision=baseline['revision'],scope='root',scope_revision=current['revision'],judge_cycle=baseline['planning']['judge']['cycle'],reason='Direct action while actual native root model owns the run')
                if action=='judge':external['judge_verdict']='continue'
                denied,value=acceptance.public.native.invoke(binary,root,out,'held-'+action+'-denied',['planning',action,work],external,environment=environment)
                assert denied!=0 and 'planning-judge-native-owner-active' in value['failure']['causes'],value
                assert acceptance.public.preview.shown(binary,root,out,'held-'+action+'-after',work)==baseline and acceptance.public.finance(root,work)==finance
                assert rows(root,'SELECT phase FROM native_planning_runs WHERE claim_id=?',(claim,))==[('permitted',)]
                assert process.poll() is None and not select.select([childfd],[],[],0)[0]
            connection.sendall(b'1');connection.close();connection=None;status=process.wait();assert select.select([childfd],[],[],0)[0]
        raw=(out/('held-run.stdout' if status==0 else 'held-run.stderr')).read_bytes();answer=(status,dict(run=json.loads(raw)) if status==0 else json.loads(raw),code)
        acceptance.public.f.publish(out/'held-observation.json',dict(owner_pid=process.pid,child_pid=peer,direct_judge_and_renew_refused_without_mutation=True,actual_child_pidfd_ready_after_wait=True,global_watchdog=None))
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
    if errors:raise BaseExceptionGroup('Held judge and cleanup failed',errors)
    return answer

def main():
    """Create fresh public state, observe physical refusal, then original success.

    The retained SR1 original, received prompt and real PIDs bind the successful
    leased decision after release. Neither direct rejection may charge a call,
    acknowledge an input or invalidate the held provider's legitimate result.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:p.add_argument('--'+name,required=True,type=Path)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None
    acceptance.public.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes())
        assert profile['no_fixture_feature'] and acceptance.public.f.digest(binary.read_bytes())==profile['sha256']
        owned=acceptance.public.f.Fixture(binary,out/'source','judge-held')
        owned.work=fresh_work(owned,'held-create');work=owned.ok(['planning','enable',owned.work],request(event='held-enable',max_activations=2,max_decisions=2,max_judge_cycles=2))
        flow.proposal(owned,work,'root',[dict(kind='judge',judge_cycle=1,judge_verdict='continue')])
        status,value,code=held_judge(binary,owned.root,owned.work,out,'root',flow.feedback.PROGRAM.encode())
        raw=(owned.root/'.swarm'/('native-planning-'+acceptance.public.f.digest(b'held-judge-claim')+'.receipt')).read_bytes()
        acceptance.public.f.publish(out/'original.frame',raw);acceptance.public.f.publish(out/'inert-provider.py',code)
        magic,extent,sha,body=raw.split(b'\n',3);assert magic==b'SR1' and len(body)==int(extent) and acceptance.public.f.digest(body)==sha.decode()
        original=json.loads(body);pid=original['run']['child_pid'];prompt=(owned.root/('fixture-prompt-'+str(pid)+'.bin')).read_bytes();witness=json.loads((owned.root/('model-received-'+str(pid)+'.json')).read_bytes())
        acceptance.public.f.publish(out/'received-prompt.bin',prompt);acceptance.public.f.publish(out/'provider-witness.json',witness)
        assert witness['pid']==pid and witness['prompt_sha256']==acceptance.public.f.digest(prompt)==original['run']['prompt_sha256']
        assert status==0 and value['run']['decision_applied'],value
        final=owned.ok(['work','show',owned.work])['work'];assert final['planning']['judge']['verdict']=='continue' and final['planning']['judge']['cycle']==1 and final['planning']['activations']==1 and final['planning']['decisions']==1
        acceptance.public.f.publish(out/'result.json',dict(status='PASS',root=str(owned.root),work=owned.work,binary_sha256=profile['sha256'],live_direct_judge_and_renew_refused=True,original_leased_result_applied_after_release=True,physical_provider_witness=True,global_watchdog=None,installed=False,authenticated_actor=False))
        print('PASS direct judge and renew preserve live native ownership and original result',flush=True)
    except BaseException:
        acceptance.public.f.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
