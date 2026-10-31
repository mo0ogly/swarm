#!/usr/bin/env python3
"""Replay one stale lease/generation case across a public holder reopen.

One named scenario on an owned inactive Root with an inert local provider:
valid lease witness through a real permitted run, actual product lease expiry
observed without arbitrary sleeps, old tokens refused BEFORE any new durable
effect (exact refusal codes, unchanged Work/ledger/snapshot), public
pause/resume reopen (generation bump, holder/until cleared) with the stale
token refused again, then a legitimate new acquisition succeeds exactly once.
The product contract fixes this order: the run owner self-refuses (its own
live() fence) as soon as the reopen clears its lease, and a reopened scope
keeps no lease to expire, so expiry is witnessed on the valid original lease
first (owner frozen), then the public reopen follows. Public CLI only; no
live mission database, no SQL mutation, no agent-clock ordering, no global
watchdog. Original command outputs, source/binary hashes and any failed Root
are retained. Local refusal evidence never establishes authenticated
identity, acceptance or settlement.
"""
import argparse,json,os,selectors,signal,socket,struct,subprocess,sys,time,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_planning_native_run as run
import rust_planning_step as step
from rust_planning_activation import configured,claim_request,apply_policy
from rust_planning_call_preview import preview_request
from rust_planning_budget import ledger


def hold(binary,owned,out,request,claim):
    """Start one owned foreground run until the real provider handshake.

    Reuse the native-run program semantics: the inert provider waits on an
    owned UNIX socket; the actual child peer is correlated by SO_PEERCRED and
    the persisted row must be permitted with these exact pids. No watchdog or
    retry; caller owns cleanup via the returned handles.
    """
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);selector=selectors.DefaultSelector()
    held=dict(listener=listener,connection=None,selector=selector,pidfd=None,childfd=None,process=None,stopped=False,peer=None,argv=None,stdout=None,stderr=None)
    stdout=(out/(claim+'-run.stdout')).open('xb');stderr=(out/(claim+'-run.stderr')).open('xb');held['stdout']=stdout;held['stderr']=stderr
    try:
        listener.bind(str(owned.root/(claim+'-fixture.sock')));os.chmod(owned.root/(claim+'-fixture.sock'),0o600);listener.listen(1);listener.setblocking(False)
        argv=[str(binary),'planning','run',owned.work,'--root',str(owned.root),'--input',str(out/'lease-reopen-input.json'),'--json']
        process=subprocess.Popen(argv,env=owned.environment,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr);held['process']=process
        fixture.publish(out/(claim+'-owner.json'),dict(argv=argv,pid=process.pid,global_watchdog=None));held['argv']=argv
        pidfd=os.pidfd_open(process.pid);held['pidfd']=pidfd;selector.register(pidfd,selectors.EVENT_READ,'exit');selector.register(listener,selectors.EVENT_READ,'provider')
        while held['connection'] is None:
            events=selector.select()
            if any(key.data=='exit' for key,_ in events):raise AssertionError('CLI exited before owned provider handshake')
            held['connection'],_=listener.accept()
        selector.unregister(listener)
        peer,uid,_=struct.unpack('3i',held['connection'].getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert uid==os.getuid() and held['connection'].recv(1)==b'R'
        held['peer']=peer;childfd=os.pidfd_open(peer);held['childfd']=childfd
        row=run.native_row(owned,claim);assert row[:4]==(process.pid,peer,1,'permitted'),row
        return held
    except BaseException:
        cleanup(holds=[held],extra=[])
        raise


def cleanup(holds,extra):
    """Release every owned handle; joined causes stay failures, never success.

    A frozen owner is continued first so its wait() can complete; a peer that
    already left the barrier (its fence refused earlier) makes the release byte
    a broken pipe, which stays a joined observation, not a silent success.
    """
    errors=[]
    for hold in holds:
        connection=hold['connection']
        for operation in [lambda:hold['process'].send_signal(signal.SIGCONT) if hold.get('stopped') else None,lambda:connection.sendall(b'1') if connection is not None else None,lambda:connection.close() if connection is not None else None,hold['listener'].close,hold['selector'].close,lambda:os.close(hold['pidfd']) if hold['pidfd'] is not None else None,lambda:os.close(hold['childfd']) if hold['childfd'] is not None else None,lambda:hold['process'].wait() if hold['process'] is not None and hold['process'].poll() is None else None,hold['stdout'].close,hold['stderr'].close]:
            try:operation()
            except BrokenPipeError:pass
            except BaseException as error:errors.append(error)
    for operation in extra:
        try:operation()
        except BaseException as error:errors.append(error)
    if len(errors)==1:raise errors[0]
    if errors:raise BaseExceptionGroup('Owned lease-reopen cleanup failed',errors)


def refuse(owned,args,value,expected):
    """Require the exact refusal with unchanged durable state and no new spend."""
    before=owned.ok(['work','show',owned.work]);snapshot=owned.snapshot();money=ledger(owned)
    code,wire=owned.call(args,value)
    assert code!=0 and wire['failure']['code']==expected,wire
    assert 'PRIVATE' not in str(wire)
    assert owned.ok(['work','show',owned.work])==before and owned.snapshot()==snapshot and ledger(owned)==money
    return wire


def lease_reopen_case(binary,out):
    """The one named stale lease/generation scenario after holder reopen.

    Witness a valid lease with a real permitted run frozen at its barrier,
    observe the actual product lease expiry, refuse stale replays before any
    new durable effect, reopen the holder through public pause/resume, refuse
    the stale token again against the reopened generation, then acquire once.
    Every refusal is verified for cause and before/after state, not exit alone.
    The reopen follows the expiry because the run owner's live() fence refuses
    a cleared holder while paused state exists and a reopened scope keeps no
    lease to expire; the stale token is refused both before and after reopen.
    """
    owned=fixture.Fixture(binary,out,'lease-reopen')
    # Hold the foreground provider at the same owned UNIX-socket barrier as the
    # step guard: the composed program waits for the fixture release byte, so
    # the actual run owner stays alive through pause/resume and expiry.
    barrier="import socket\nwith socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as gate:\n gate.connect('lease-reopen-original-fixture.sock')\n gate.sendall(b'R')\n assert gate.recv(1)==b'1'\n"
    program=run.PROGRAM.replace("proposal=json.load(open('planning-proposal.json'))",barrier+"proposal=json.load(open('planning-proposal.json'))")
    assert barrier in program,'Barrier composition failed; provider would not hold'
    fixture.publish(owned.root/'codex',program.encode());os.chmod(owned.root/'codex',0o700)
    apply_policy(owned,'lease-reopen-settings',dict(planning_scope_lease_min_seconds='1',planning_default_lease_seconds='2',planning_lease_renew_after_percent='0',planning_provider_silence_seconds='0',planning_provider_max_duration_seconds='0'))
    enabled=configured(owned,'lease-reopen');original=claim_request(enabled,'lease-reopen-original',lease_seconds=2)
    claimed=owned.ok(['planning','claim',owned.work],original);scope=next(s for s in claimed['planning']['scopes'] if s['id']=='root')
    assert scope['holder']=='fixture-planner' and scope['generation']==1 and scope['lease_until']
    proposal=dict(input_events=scope['delivery']['events'],reason='Owned stale-lease witness proposal',operations=[dict(kind='task',id='lease-reopen-worker',title='Stale lease witness',requirements=['req-1'],deliverable='Future report',criteria=['Future validation'])])
    fixture.publish(owned.root/'planning-proposal.json',proposal)
    value=preview_request(claimed,'lease-reopen-original')
    assert owned.ok(['planning','call-preview',owned.work],value)['prepared_inputs_verified']
    fixture.publish(out/'lease-reopen-input.json',value)
    money=ledger(owned);before=owned.ok(['work','show',owned.work])['work']
    holds=[]
    try:
        held=hold(binary,owned,out,value,'lease-reopen-original');holds.append(held)
        marker=owned.root/'model-received.json';first=marker.read_bytes()
        # Freeze the run owner at its own poll boundary (step guard pattern) so
        # its live() fence cannot race the deliberate expiry observation.
        held['process'].send_signal(signal.SIGSTOP)
        pid,status=os.waitpid(held['process'].pid,os.WUNTRACED);assert pid==held['process'].pid and os.WIFSTOPPED(status)
        held['stopped']=True
        # Observe the actual product lease expiry of the original valid claim;
        # the frozen owner must stay alive past it. No sleep-based shortcut.
        assert not held['selector'].select(0),'Owner exited before its original lease expiry'
        remaining=(step.expiry_ns(scope['lease_until'])-time.time_ns())/1_000_000_000
        assert remaining>0,'Original lease already expired before the reopen wait'
        assert not held['selector'].select(remaining),'Owner exited before its original lease expiry'
        assert not held['selector'].select(0),'Owner exited before its original lease expiry'
        # Stale token refusals BEFORE any new durable effect, on the expired
        # original lease. Each refuses for the lease/record cause itself with
        # unchanged Work/ledger/snapshot, not exit code alone.
        refuse(owned,['planning','run',owned.work],value,'planning-run-already-recorded')
        stale_preview=dict(value,scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'])
        refuse(owned,['planning','call-preview',owned.work],stale_preview,'stale_lease')
        stale_renew=dict(schema_version=1,event_id='lease-reopen-stale-renew',expected_revision=owned.ok(['work','show',owned.work])['work']['revision'],scope='root',scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'],lease_seconds=2)
        refuse(owned,['planning','lease-renew',owned.work],stale_renew,'stale_lease')
        assert (owned.root/'model-received.json').read_bytes()==first
        # Public pause/resume reopen. The owner is stopped, so its own live()
        # fence cannot act on the cleared lease; it resumes only for cleanup.
        paused=owned.ok(['planning','pause',owned.work],dict(schema_version=1,event_id='lease-reopen-pause',expected_revision=claimed['revision']))
        resumed=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='lease-reopen-resume',expected_revision=paused['revision'],reason='Explicit holder reopen after operator pause'))
        reopened=next(s for s in resumed['planning']['scopes'] if s['id']=='root')
        # Each public control increments the generation once (pause and resume
        # alike; planning_control contract), and clears holder/until. The public
        # JSON omits the emptied fields entirely; absence is the cleared state.
        assert paused['revision']==claimed['revision']+1 and resumed['revision']==paused['revision']+1
        assert reopened['generation']==scope['generation']+2 and not reopened.get('holder') and not reopened.get('lease_until'),reopened
        # The stale token is refused again against the reopened generation: the
        # lease fence itself refuses the cleared holder (stale_lease,
        # planning_decision.rs:62) with the current revisions but the old
        # holder/generation, before any new effect.
        stale_decide=dict(schema_version=1,event_id='lease-reopen-stale-decide',expected_revision=resumed['revision'],scope='root',scope_revision=reopened['revision'],holder=scope['holder'],generation=scope['generation'],input_events=proposal['input_events'],operations=proposal['operations'],reason='Stale pre-reopen decision')
        refuse(owned,['planning','decide',owned.work],stale_decide,'stale_lease')
        # Release the frozen owner: its own live() fence now refuses the cleared
        # lease (stale_lease) and records the terminal refused disposition that
        # releases registry ownership (require_claim_available protocol). The
        # release wait is bounded by the saved poll cadence, never a watchdog.
        held['process'].send_signal(signal.SIGCONT);held['stopped']=False
        held['connection'].sendall(b'1')
        cadence=int(owned.ok(['engine-settings','show'])['values']['agent_supervision_poll_interval_ms'])/1000;assert cadence>0
        events=held['selector'].select(cadence*4)
        assert any(key.data=='exit' for key,_ in events) or held['process'].poll() is not None,'Owner did not reach its terminal disposition within the saved poll cadence'
        assert held['process'].wait()!=0,held['process'].returncode
        assert run.native_row(owned,'lease-reopen-original')[3]=='refused'
        # New legitimate acquisition succeeds exactly once on the reopened scope.
        fresh=owned.ok(['planning','claim',owned.work],claim_request(resumed,'lease-reopen-fresh',scope_revision=reopened['revision'],holder='lease-reopen-holder'))
        fresh_scope=next(s for s in fresh['planning']['scopes'] if s['id']=='root')
        assert fresh_scope['holder']=='lease-reopen-holder' and fresh_scope['generation']==reopened['generation']+1,fresh_scope
        current=owned.ok(['work','show',owned.work])['work']
        assert current['planning']['activations']==2 and current['planning']['decisions']==0,current['planning']
        after=ledger(owned);assert [row for row in after if row[0]!='lease-reopen-fresh']==money,'Original reservations changed by refusals or the new claim'
        assert len(after)==len(money)+1 and 'lease-reopen-fresh' in [row[0] for row in after],after
        busy=refuse(owned,['planning','claim',owned.work],claim_request(current,'lease-reopen-second',scope_revision=fresh_scope['revision'],holder='lease-reopen-holder',lease_seconds=2),'lease_busy')
        final=owned.ok(['work','show',owned.work])['work']
        assert final==current and (owned.root/'model-received.json').read_bytes()==first
        return dict(root=str(owned.root),status='PASS',provider_executions=1,witness_generation=scope['generation'],reopened_generation=reopened['generation'],fresh_generation=fresh_scope['generation'],stale_run_refused='planning-run-already-recorded',stale_preview_refused='stale_lease',stale_renew_refused='stale_lease',stale_decision_refused='stale_lease',second_claim_refused='lease_busy',new_acquisitions=1,reservations_unchanged=True)
    finally:
        cleanup(holds,extra=[])


def main():
    """Retain exact source/binary identity and original failure evidence.

    The frozen production binary must carry the qualified profile contract
    (no fixture feature, bound sha256); sources and binary digests are rechecked
    after the scenario so changed inputs invalidate the evidence instead of
    passing silently.
    """
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary',required=True,type=Path)
    p.add_argument('--profile',required=True,type=Path)
    p.add_argument('--output',required=True,type=Path)
    a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    profile=json.loads(a.profile.read_bytes())
    assert profile['no_fixture_feature'] is True and profile['sha256']==fixture.digest(binary.read_bytes()),'lease-reopen normal profile/hash mismatch'
    fixture.publish(out/'profile.json',profile)
    sources=[Path(__file__),Path('tests/rust_agent_model_route.py'),Path('tests/rust_planning_activation.py'),Path('tests/rust_planning_bootstrap.py'),Path('tests/rust_planning_call_preview.py'),Path('tests/rust_planning_budget.py'),Path('tests/rust_planning_native_run.py'),Path('tests/rust_planning_step.py'),Path('tests/rust_planning_decision.py')]
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(p2):fixture.digest(p2.read_bytes()) for p2 in sources}))
    try:
        result=lease_reopen_case(binary,out)
        recorded=json.loads((out/'inputs.json').read_bytes());assert fixture.digest(binary.read_bytes())==recorded['binary_sha256'];assert all(fixture.digest(Path(n).read_bytes())==v for n,v in recorded['sources'].items())
        fixture.publish(out/'result.json',dict(result,scope='Stale lease/generation after public holder reopen; planner/lifecycle consumers only, not Git/settlement effects',real_provider=False,installed=False,global_watchdog=None))
        print('PASS lease-reopen-stale-token-refused-before-new-effect')
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':
    sys.exit(main())
