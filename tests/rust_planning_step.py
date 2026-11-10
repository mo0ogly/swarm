#!/usr/bin/env python3
"""Exercise native single-activation orchestration with owned inert processes.

No provider account, live Root, private SQL mutation or global watchdog is used.
An expired lease never proves termination. Fixture SIGSTOP is limited to the
actual owned CLI; every failure resumes it and retains original evidence.
"""
import argparse,json,os,socket,selectors,struct,subprocess,signal,time,traceback,sys
from pathlib import Path
from datetime import datetime,timezone
import rust_agent_model_route as fixture
import rust_planning_native_run as run
from rust_planning_activation import configured,claim_request,apply_policy
from rust_planning_call_preview import preview_request
from rust_planning_budget import ledger
from rust_planning_decision import decision


def first_step(binary,out):
    """Use a changed saved default for one real automatic claim/run, then idle.

    Initial pending input is explicit fixture data. Only the engine may choose
    holder/generation/claim IDs. A second idle step spends no activation/inference.
    """
    owned=fixture.Fixture(binary,out,'first-step');fixture.publish(owned.root/'codex',run.PROGRAM.encode());os.chmod(owned.root/'codex',0o700)
    apply_policy(owned,'step-default',dict(planning_default_lease_seconds='7'))
    enabled=configured(owned,'step');pending=[e['id'] for e in enabled['planning']['inbox'] if e['scope']=='root' and not e.get('decision')];assert len(pending)==1
    fixture.publish(owned.root/'planning-proposal.json',dict(input_events=pending,reason='Owned automatic activation',operations=[dict(kind='task',id='step-worker',title='Automatic native proposal',requirements=['req-1'],deliverable='Future report',criteria=['Future validation'])]))
    value=owned.ok(['planning','step',owned.work]);assert value['status']=='applied' and value['lease_seconds']==7 and value['selected_scope']=='root',value
    assert value['run']['decision_applied'] and value['accepted'] is False
    current=owned.ok(['work','show',owned.work])['work'];assert current['planning']['activations']==1 and current['planning']['decisions']==1
    marker=(owned.root/'model-received.json').read_bytes();money=ledger(owned);before=owned.snapshot()
    idle=owned.ok(['planning','step',owned.work]);assert idle['status']=='idle' and idle['accepted'] is False
    assert owned.snapshot()==before and ledger(owned)==money and (owned.root/'model-received.json').read_bytes()==marker
    return dict(root=str(owned.root),status='PASS',default_lease_used=7,provider_executions=1,idle_spend=False)


def fifo_scope(binary,out):
    """A child input older than root resume wins despite root-first scope order.

    All scopes/events are produced by public operations, not database seeding.
    The saved activation budget is unchanged and no worker is executed/accepted.
    """
    owned=fixture.Fixture(binary,out,'fifo-scope');fixture.publish(owned.root/'codex',run.PROGRAM.encode());os.chmod(owned.root/'codex',0o700)
    enabled=configured(owned,'fifo');claimed=owned.ok(['planning','claim',owned.work],claim_request(enabled,'fifo-manual-root'))
    delegated=owned.ok(['planning','decide',owned.work],decision(claimed,'fifo-delegate',[dict(kind='delegate',id='child',title='Owned child',requirements=['req-1'],next='Scoped work',max_tasks=2,max_activations=2)]))
    resumed=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='fifo-newer-root-input',expected_revision=delegated['revision'],reason='Later root instruction'))
    assert [scope['id'] for scope in resumed['planning']['scopes']]==['root','child']
    pending=[event for event in resumed['planning']['inbox'] if not event.get('decision')]
    assert pending[0]['scope']=='child' and pending[-1]['scope']=='root'
    ids=[event['id'] for event in pending if event['scope']=='child']
    fixture.publish(owned.root/'planning-proposal.json',dict(input_events=ids,reason='FIFO child activation',operations=[]))
    value=owned.ok(['planning','step',owned.work]);assert value['selected_scope']=='child' and value['input_event']==ids[0],value
    current=owned.ok(['work','show',owned.work])['work'];assert current['planning']['activations']==2 and current['planning']['decisions']==2
    assert any(event['scope']=='root' and not event.get('decision') for event in current['planning']['inbox'])
    return dict(root=str(owned.root),status='PASS',scope_order=['root','child'],selected_scope='child',older_input_selected=True)


def received(owned):
    """Observe every immutable inert-process witness without overwriting history.

    Files belong only to this multi-call fixture. PID is a local filename label,
    not agent identity or authority; reads/encoding failures propagate.
    """
    return {p.name:fixture.digest(p.read_bytes()) for p in sorted(owned.root.glob('model-received-*.json'))}


def stopped_and_refused(binary,out):
    """A valid step witnesses pause/refusal guards and exact no-spend repetition.

    Pause/resume are public operator changes. The invalid proposal is retained;
    the saved activation cap still blocks spending after delivered feedback.
    All errors must preserve known claim state and the original safe cause.
    """
    owned=fixture.Fixture(binary,out,'stopped-refused');fixture.publish(owned.root/'codex',run.PROGRAM.replace("open('model-received.json','x'","open('model-received-'+str(os.getpid())+'.json','x'").encode());os.chmod(owned.root/'codex',0o700)
    enabled=configured(owned,'stopped');ids=[event['id'] for event in enabled['planning']['inbox'] if not event.get('decision')]
    fixture.publish(owned.root/'planning-proposal.json',dict(input_events=ids,reason='Valid control before pause',operations=[]))
    healthy=owned.ok(['planning','step',owned.work]);assert healthy['run']['decision_applied'] and len(received(owned))==1
    current=healthy['run']['work'];resumed=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='stopped-input',expected_revision=current['revision'],reason='New explicit input'))
    paused=owned.ok(['planning','pause',owned.work],dict(schema_version=1,event_id='stopped-pause',expected_revision=resumed['revision']))
    before=owned.snapshot();money=ledger(owned);marker=received(owned)
    code,wire=owned.call(['planning','step',owned.work]);assert code!=0 and wire['failure']['code']=='planning_paused',wire
    assert owned.snapshot()==before and ledger(owned)==money and received(owned)==marker
    resumed=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='stopped-recovery',expected_revision=paused['revision'],reason='Explicit recovery'))
    ids=[event['id'] for event in resumed['planning']['inbox'] if not event.get('decision')]
    fixture.publish(owned.root/'planning-proposal.json',dict(input_events=ids,reason='Valid control after pause',operations=[]))
    recovered=owned.ok(['planning','step',owned.work]);assert recovered['run']['decision_applied'] and len(received(owned))==2
    current=recovered['run']['work'];resumed=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='refused-input',expected_revision=current['revision'],reason='Exercise refused proposal'))
    ids=[event['id'] for event in resumed['planning']['inbox'] if not event.get('decision')]
    fixture.publish(owned.root/'planning-proposal.json',dict(input_events=ids,reason='Invalid requirement fixture',operations=[dict(kind='task',id='refused-worker',title='Invalid owned task',requirements=['req-999'],deliverable='Future report',criteria=['Evidence'])]))
    code,wire=owned.call(['planning','step',owned.work]);assert code!=0 and 'planning-operation-requirements-invalid' in str(wire),wire
    assert 'work-state-preserved' in str(wire) and len(received(owned))==3,wire
    current=owned.ok(['work','show',owned.work])['work'];assert current['planning']['activations']==3 and current['planning']['decisions']==2 and not current.get('tasks')
    before=owned.snapshot();money=ledger(owned);marker=received(owned)
    code,wire=owned.call(['planning','step',owned.work]);assert code!=0 and wire['failure']['code']=='planning-activation-budget-exhausted',wire
    assert owned.snapshot()==before and ledger(owned)==money and received(owned)==marker
    return dict(root=str(owned.root),status='PASS',pause_recovered=True,saved_activation_cap_blocked=True,unchanged_spend=True,causal_replan_proved=False)


def expiry_ns(value):
    """Convert the engine's UTC RFC3339 lease without rounding nanoseconds down.

    This observes the existing product expiry, not a verification watchdog. A
    non-UTC/unexpected fixture timestamp refuses instead of guessing a duration.
    """
    assert value.endswith('Z');head=value[:-1];whole,separator,fraction=head.partition('.')
    assert not separator or fraction.isdigit() and len(fraction)<=9
    seconds=int(datetime.fromisoformat(whole+'+00:00').timestamp())
    return seconds*1_000_000_000+(int(fraction.ljust(9,'0')) if separator else 0)


def active_claim_guard(binary,out):
    """Correlate a real provider, freeze only its owner, then observe lease expiry.

    Exact original claim replay is the valid admission control. A new claim must
    refuse without spending while the old registered process remains active.
    SIGCONT/connection/descriptor/wait failures remain joined with the original.
    """
    owned=fixture.Fixture(binary,out,'active-claim-guard')
    barrier="import socket\nwith socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as gate:\n gate.connect('step-fixture.sock')\n gate.sendall(b'R')\n assert gate.recv(1)==b'1'\n"
    program=run.PROGRAM.replace("proposal=json.load(open('planning-proposal.json'))",barrier+"proposal=json.load(open('planning-proposal.json'))")
    fixture.publish(owned.root/'codex',program.encode());os.chmod(owned.root/'codex',0o700)
    apply_policy(owned,'guard-settings',dict(planning_scope_lease_min_seconds='1',planning_default_lease_seconds='2',planning_lease_renew_after_percent='0',planning_provider_silence_seconds='0',planning_provider_max_duration_seconds='0'))
    enabled=configured(owned,'guard');original=claim_request(enabled,'guard-original',lease_seconds=2);w=owned.ok(['planning','claim',owned.work],original);scope=next(s for s in w['planning']['scopes'] if s['id']=='root')
    fixture.publish(owned.root/'planning-proposal.json',dict(input_events=scope['delivery']['events'],reason='Owned blocked model',operations=[]))
    value=preview_request(w,'guard-original');assert owned.ok(['planning','call-preview',owned.work],value)['prepared_inputs_verified']
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);connection=None;selector=selectors.DefaultSelector();pidfd=None;process=None;stopped=False;primary=None;result=None
    stdout=(out/'guard-run.stdout').open('xb');stderr=(out/'guard-run.stderr').open('xb');source=out/'guard-run.input.json';fixture.publish(source,value)
    try:
        listener.bind(str(owned.root/'step-fixture.sock'));os.chmod(owned.root/'step-fixture.sock',0o600);listener.listen(1);listener.setblocking(False)
        argv=[str(binary),'planning','run',owned.work,'--root',str(owned.root),'--input',str(source),'--json'];process=subprocess.Popen(argv,env=owned.environment,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
        fixture.publish(out/'guard-owner.json',dict(argv=argv,pid=process.pid,root=str(owned.root),global_watchdog=None))
        pidfd=os.pidfd_open(process.pid);selector.register(pidfd,selectors.EVENT_READ,'exit');selector.register(listener,selectors.EVENT_READ,'provider')
        while connection is None:
            events=selector.select()
            if any(k.data=='exit' for k,_ in events):raise AssertionError('Owner exited before actual provider handshake')
            connection,_=listener.accept()
        selector.unregister(listener);peer,uid,_=struct.unpack('3i',connection.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert uid==os.getuid() and connection.recv(1)==b'R'
        assert run.native_row(owned,'guard-original')[:4]==(process.pid,peer,1,'permitted')
        replay_money=ledger(owned);replay_snapshot=owned.snapshot()
        replay=owned.ok(['planning','claim',owned.work],original);assert replay==w and ledger(owned)==replay_money and owned.snapshot()==replay_snapshot
        process.send_signal(signal.SIGSTOP);pid,status=os.waitpid(process.pid,os.WUNTRACED);assert pid==process.pid and os.WIFSTOPPED(status);stopped=True
        deadline=expiry_ns(scope['lease_until'])
        while (remaining:=deadline-time.time_ns())>0:assert not selector.select(remaining/1_000_000_000),'Stopped owner exited before its original lease expiry'
        current=owned.ok(['work','show',owned.work])['work'];before=owned.snapshot();money=ledger(owned)
        code,wire=owned.call(['planning','claim',owned.work],claim_request(current,'guard-new',holder='another-fixture-holder',lease_seconds=2))
        assert code!=0,('New claim admitted while actual prior provider still live',wire)
        assert wire['failure']['code']=='planning-claim-call-active',wire
        assert owned.ok(['work','show',owned.work])['work']==current and owned.snapshot()==before and ledger(owned)==money
        result=dict(root=str(owned.root),status='PASS',actual_provider_pid=peer,expired_lease=True,new_claim_spend=False)
    except BaseException as error:primary=error
    cleanup=[]
    for operation in [lambda:process.send_signal(signal.SIGCONT) if stopped and process is not None else None,lambda:connection.close() if connection is not None else None,listener.close,selector.close,lambda:os.close(pidfd) if pidfd is not None else None,lambda:process.wait() if process is not None else None,stdout.close,stderr.close]:
        try:operation()
        except BaseException as error:cleanup.append(error)
    if primary or cleanup:
        errors=([primary] if primary is not None else [])+cleanup
        if len(errors)==1:raise errors[0]
        raise BaseExceptionGroup('Owned admission scenario and cleanup failed',errors)
    return result


def main():
    """Retain real failures/source/default-binary identity, never erase a fixture."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--case',choices=['step','guard','fifo','state','all'],default='all');a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    sources=list(dict.fromkeys([Path(__file__),Path(fixture.__file__),Path(run.__file__)]+[Path(sys.modules[f.__module__].__file__) for f in [configured,claim_request,apply_policy,preview_request,ledger,decision]]));inputs=dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(p):fixture.digest(p.read_bytes()) for p in sources});fixture.publish(out/'inputs.json',inputs)
    try:
        checks=[]
        if a.case in ('step','all'):checks.append(first_step(binary,out))
        if a.case in ('guard','all'):checks.append(active_claim_guard(binary,out))
        if a.case in ('fifo','all'):checks.append(fifo_scope(binary,out))
        if a.case in ('state','all'):checks.append(stopped_and_refused(binary,out))
        assert fixture.digest(binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(n).read_bytes())==h for n,h in inputs['sources'].items())
        result=dict(status='PASS',checks=checks,provider_account_used=False,global_watchdog=None);fixture.publish(out/'result.json',result);print(json.dumps(result))
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':main()
