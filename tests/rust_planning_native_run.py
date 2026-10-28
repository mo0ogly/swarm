#!/usr/bin/env python3
"""Replay a real native planner CLI on an inactive owned Root and inert provider.

This requires actual process observation and a Work decision, not preview success.
It neither contacts a provider account nor launches/accepts the proposed worker.
Original command responses, source/binary hashes and any failed Root are retained.
No shell, temporal watchdog, SQL mutation, budget reset or paid retry is used.
"""
import argparse,json,os,sqlite3,traceback,socket,selectors,select,struct,subprocess,time
from contextlib import contextmanager
from datetime import datetime,timezone
from pathlib import Path
import rust_agent_model_route as fixture
from rust_planning_activation import configured,claim_request,apply_policy
from rust_planning_call_preview import preview_request
from rust_planning_budget import ledger

PROGRAM=r'''#!/usr/bin/python3
import sys,os,json,hashlib
raw=sys.stdin.buffer.read()
args=sys.argv[1:]
with open('model-received.json','x',encoding='utf8') as f:
 json.dump(dict(argv=args,pid=os.getpid(),ppid=os.getppid(),prompt_sha256=hashlib.sha256(raw).hexdigest(),prompt_bytes=len(raw)),f);f.flush();os.fsync(f.fileno())
proposal=json.load(open('planning-proposal.json'))
print(json.dumps(dict(type='result',structured_output=proposal,usage=dict(input_tokens=7,output_tokens=8))),flush=True)
'''


def read_usage(owned,claim):
    """Read normalized fixture usage without private seeding or live database IO.

    SQLite/close errors remain failures; unknown cost must remain SQLNULL.
    Immutable JSON evidence is distinct from these operational scalar columns.
    """
    db=sqlite3.connect('file:'+str(owned.root/'.swarm/state.db')+'?mode=ro',uri=True)
    primary=None
    try:
        fields=db.execute('PRAGMA table_info(native_planning_usage)').fetchall()
        assert fields and all(row[2].upper()!='BLOB' for row in fields)
        rows=db.execute('SELECT input_tokens,output_tokens,provider_reported_cost_usd FROM native_planning_usage WHERE claim_id=?',[claim]).fetchall()
        assert rows==[(7,8,None)],rows
    except BaseException as error:primary=error
    try:db.close()
    except BaseException as error:
        if primary:raise BaseExceptionGroup('Usage observation and close failed',[primary,error])
        raise
    if primary:raise primary



def happy(binary,out):
    """Require one real inert provider, one bound decision and preserved reserves.

    The existing pure preview is the pre-start control. Explicit call repetition
    must refuse without inference; recovery is a separate supported operation.
    No JSON-supplied actor, clock, event ID or acceptance is part of this request.
    """
    owned=fixture.Fixture(binary,out,'native-planner')
    fixture.publish(owned.root/'codex',PROGRAM.encode());os.chmod(owned.root/'codex',0o700)
    enabled=configured(owned,'native-plan');claim='native-plan-claim'
    claimed=owned.ok(['planning','claim',owned.work],claim_request(enabled,claim,lease_seconds=120))
    scope=next(s for s in claimed['planning']['scopes'] if s['id']=='root')
    proposal=dict(input_events=scope['delivery']['events'],reason='Owned inert provider proposal',operations=[dict(kind='task',id='native-worker',title='Observe a real planner decision',requirements=['req-1'],deliverable='Explicit future report',criteria=['Future validation required'])])
    fixture.publish(owned.root/'planning-proposal.json',proposal)
    value=preview_request(claimed,claim);ready=owned.ok(['planning','call-preview',owned.work],value)
    assert ready['prepared_inputs_verified'] and ready['process_started'] is False
    assert not (owned.root/'model-received.json').exists()
    money=ledger(owned);response=owned.ok(['planning','run',owned.work],value)
    assert response['decision_applied'] is True and response['accepted'] is False,response
    current=owned.ok(['work','show',owned.work])['work'];assert current['planning']['activations']==1 and current['planning']['decisions']==1
    task=next(t for t in current['tasks'] if t['id']=='native-worker');assert not task.get('attempts')
    assert ledger(owned)==money,'Inferred/unknown cost changed original reservations'
    marker=owned.root/'model-received.json';first=marker.read_bytes();observed=json.loads(first)
    assert observed['prompt_bytes']>ready['context_bytes']+ready['workflow_bytes']
    assert '--sandbox' in observed['argv'] and observed['argv'][observed['argv'].index('--sandbox')+1]=='read-only'
    assert not any('dangerously' in arg for arg in observed['argv'])
    read_usage(owned,claim)
    before=owned.snapshot();owned.denied(['planning','run',owned.work],value,'planning-run-already-recorded')
    assert owned.snapshot()==before and marker.read_bytes()==first and ledger(owned)==money
    replay=owned.ok(['planning','reconcile',claim]);assert replay['replayed'] and replay['decision_applied'] and replay['work']==current
    assert owned.snapshot()==before and marker.read_bytes()==first and ledger(owned)==money
    return dict(root=str(owned.root),work=owned.work,claim=claim,decision_applied=True,accepted=False,provider_executions=1,unknown_cost_is_null=True,reservations_unchanged=True)


@contextmanager
def database(owned,write_lock=False):
    """Observe fixture SQL or hold a write reservation without changing a row.

    The fault uses BEGIN IMMEDIATE only, always explicitly rolled back. Primary,
    rollback and close failures stay joined. This never opens a live mission DB.
    """
    path=owned.root/'.swarm/state.db'
    db=sqlite3.connect(str(path) if write_lock else 'file:'+str(path)+'?mode=ro',uri=not write_lock,timeout=0)
    errors=[];begun=False
    try:
        if write_lock:db.execute('BEGIN IMMEDIATE');begun=True
        yield db
    except BaseException as e:errors.append(e)
    if begun:
        try:db.rollback()
        except BaseException as e:errors.append(e)
    try:db.close()
    except BaseException as e:errors.append(e)
    if len(errors)==1:raise errors[0]
    if errors:raise BaseExceptionGroup('Fixture SQL operation and cleanup failed',errors)


def native_row(owned,claim):
    """Read the actual scalar owner/child/phase receipt without private mutation."""
    with database(owned) as db:
        row=db.execute('SELECT owner_pid,child_pid,permission,phase,receipt_sha256 FROM native_planning_runs WHERE claim_id=?',[claim]).fetchone()
        assert row;return row


def disabled_cases(binary,out):
    """Use valid preview before cap0 and a regular-file helper transport fault.

    Refusals must preserve Work/ledger/process absence and original input bytes.
    No malformed-input marker is interpreted as execution authority.
    """
    owned=fixture.Fixture(binary,out,'disabled-planner');fixture.publish(owned.root/'codex',PROGRAM.encode());os.chmod(owned.root/'codex',0o700)
    enabled=configured(owned,'disabled-plan');claim='disabled-claim';w=owned.ok(['planning','claim',owned.work],claim_request(enabled,claim,lease_seconds=120));request=preview_request(w,claim)
    assert owned.ok(['planning','call-preview',owned.work],request)['prepared_inputs_verified']
    source=owned.root/'regular-stdin';fixture.publish(source,b'PRIVATE_INPUT_MUST_REMAIN_UNCHANGED');before=source.read_bytes()
    with source.open('rb') as stream:
        run=subprocess.run([str(binary),'_planning_gate',claim,'--root',str(owned.root),'--json'],stdin=stream,capture_output=True,env=owned.environment)
    assert run.returncode!=0 and source.read_bytes()==before
    wire=json.loads(run.stderr);assert wire['failure']['code']=='planning-gate-channel-kind',wire
    assert b'PRIVATE_INPUT' not in run.stdout+run.stderr
    apply_policy(owned,'disable-response',dict(planning_response_max_bytes='0'))
    state=owned.ok(['work','show',owned.work]);money=ledger(owned);snapshot=owned.snapshot()
    owned.denied(['planning','run',owned.work],request,'planning-call-response-disabled')
    assert owned.ok(['work','show',owned.work])==state and owned.snapshot()==snapshot and ledger(owned)==money
    assert not (owned.root/'model-received.json').exists()
    return dict(cap0_processes=0,regular_stdin_unchanged=True,refusals_without_mutation=True)


def held(binary,out,label,inject,renew,stop=None,hold_for_stop=False,after_original=None):
    """Own a real provider socket and CLI pidfd through release and recovery.

    Healthy and SQL-lock variants share explicit persisted retry policy. Lock is
    acquired only after actual registered child/peer correlation. Renewal waits
    beyond the original expiry at saved poll cadence without concurrent database
    readers, then requires an actual decision, original owner/child correlation
    and persisted renewal event. Other cases verify live-owner refusal. Reading
    during commit with zero busy policy could itself stop the provider. The
    contractual expiry is evidence, never a global test watchdog.
    The inert provider executes once; no acceptance or actual billing is inferred.
    Explicit two-second fixture stop policies are product inputs, not a test
    watchdog. Each held stop has a released healthy control under identical settings;
    actual child pidfd readiness is checked after owner wait. An optional trusted
    fixture callback is permitted only on SQL-lock recovery, after real original
    capture/owner exit and before public reconciliation. It grants no product
    callback, provider, lease or retry authority and must preserve original data.
    """
    if after_original is not None and (not inject or renew or stop is not None):raise ValueError('held-after-original-requires-isolated-sql-recovery')
    owned=fixture.Fixture(binary,out,label)
    barrier="import socket\nwith socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as gate:\n gate.connect('planning-fixture.sock')\n gate.sendall(b'R')\n assert gate.recv(1)==b'1'\n"
    program=PROGRAM.replace("proposal=json.load(open('planning-proposal.json'))",barrier+"proposal=json.load(open('planning-proposal.json'))")
    fixture.publish(owned.root/'codex',program.encode());os.chmod(owned.root/'codex',0o700)
    changes=dict(storage_busy_retries='0',storage_busy_timeout_ms='0')
    if renew:changes.update(planning_scope_lease_min_seconds='1',planning_default_lease_seconds='2',planning_lease_renew_after_percent='25')
    if stop is not None:
        assert stop in ('silence','maximum')
        changes.update(planning_provider_silence_seconds='2' if stop=='silence' else '0',planning_provider_max_duration_seconds='2' if stop=='maximum' else '0')
    apply_policy(owned,label+'-settings',changes)
    enabled=configured(owned,label);claim=label+'-claim';w=owned.ok(['planning','claim',owned.work],claim_request(enabled,claim,lease_seconds=2 if renew else 120));scope=next(s for s in w['planning']['scopes'] if s['id']=='root')
    proposal=dict(input_events=scope['delivery']['events'],reason='Owned held-provider proposal',operations=[dict(kind='task',id='held-worker',title='Held fixture task',requirements=['req-1'],deliverable='Future report',criteria=['Future validation'])])
    fixture.publish(owned.root/'planning-proposal.json',proposal);request=preview_request(w,claim)
    assert owned.ok(['planning','call-preview',owned.work],request)['prepared_inputs_verified']
    money=ledger(owned);before=owned.ok(['work','show',owned.work])['work'];path=owned.output/(label+'-run.input.json');fixture.publish(path,request)
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);connection=None;selector=selectors.DefaultSelector();pidfd=None;childfd=None;process=None;primary=None;result=None
    stdout=(out/(label+'-run.stdout')).open('xb');stderr=(out/(label+'-run.stderr')).open('xb')
    try:
        listener.bind(str(owned.root/'planning-fixture.sock'));os.chmod(owned.root/'planning-fixture.sock',0o600);listener.listen(1);listener.setblocking(False)
        argv=[str(binary),'planning','run',owned.work,'--root',str(owned.root),'--input',str(path),'--json']
        process=subprocess.Popen(argv,env=owned.environment,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
        fixture.publish(out/(label+'-owner.json'),dict(argv=argv,pid=process.pid,global_watchdog=None,sql_lock=inject,lease_renewal=renew))
        pidfd=os.pidfd_open(process.pid);selector.register(pidfd,selectors.EVENT_READ,'exit');selector.register(listener,selectors.EVENT_READ,'provider')
        while connection is None:
            events=selector.select()
            if any(key.data=='exit' for key,_ in events):raise AssertionError('CLI exited before owned provider handshake')
            connection,_=listener.accept()
        selector.unregister(listener)
        peer,uid,_=struct.unpack('3i',connection.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert uid==os.getuid() and connection.recv(1)==b'R'
        childfd=os.pidfd_open(peer)
        if not renew:
            row=native_row(owned,claim);assert row[:4]==(process.pid,peer,1,'permitted'),row
            owned.denied(['planning','reconcile',claim],None,'planning-reconcile-process-still-present')
        if renew:
            cadence=int(owned.ok(['engine-settings','show'])['values']['agent_supervision_poll_interval_ms'])/1000;assert cadence>0
            original_expiry=datetime.fromisoformat(scope['lease_until'].replace('Z','+00:00'))
            while (remaining:=(original_expiry-datetime.now(timezone.utc)).total_seconds())>0:
                assert not selector.select(min(cadence,remaining)),'Owner exited before original lease expiry'
            assert not selector.select(0),'Owner exited before provider release past original expiry'
        if hold_for_stop:
            assert stop is not None
            code=process.wait();assert code!=0
            connection.close();connection=None
        elif inject:
            with database(owned,True):
                connection.sendall(b'1');connection.close();connection=None;code=process.wait()
                assert code!=0,'SQL write reservation failed to exercise refusal'
        else:
            connection.sendall(b'1');connection.close();connection=None;code=process.wait();assert code==0
        stdout.flush();stderr.flush();os.fsync(stdout.fileno());os.fsync(stderr.fileno())
        assert select.select([childfd],[],[],0)[0]==[childfd],'Actual provider leader did not terminate'
        wire=json.loads((out/(label+('-run.stderr' if code else '-run.stdout'))).read_bytes());first=(owned.root/'model-received.json').read_bytes()
        if hold_for_stop:
            expected='planning-provider-silence' if stop=='silence' else 'planning-provider-maximum-duration'
            assert wire['failure']['code']=='work-state-preserved' and expected in str(wire),wire
            after=owned.ok(['work','show',owned.work])['work'];assert after['revision']==before['revision']+1 and not after.get('tasks')
            assert after['planning']['decisions']==0 and any(e['kind']=='planning_rejected' for e in after['planning']['inbox']) and ledger(owned)==money
            row=native_row(owned,claim);assert row[3]=='refused' and row[4]
            with database(owned) as db:assert db.execute('SELECT quality,input_tokens,output_tokens,provider_reported_cost_usd FROM native_planning_usage WHERE claim_id=?',[claim]).fetchone()==('incomplete',None,None,None)
            result=dict(root=str(owned.root),provider_executions=1,decision_applied=False,configured_stop=stop,leader_terminated=True,accepted=False)
        else:
            if inject:
                assert wire['failure']['code']=='root-storage-busy',wire
                assert owned.ok(['work','show',owned.work])['work']==before
                row=native_row(owned,claim);assert row[3]=='permitted' and row[4] is None
                with database(owned) as db:assert db.execute('SELECT count(*) FROM native_planning_usage WHERE claim_id=?',[claim]).fetchone()==(0,)
                if after_original is not None:after_original(owned,claim)
                response=owned.ok(['planning','reconcile',claim]);assert response['decision_applied'] and not response['replayed']
            current=owned.ok(['work','show',owned.work])['work'];assert current['planning']['activations']==1 and current['planning']['decisions']==1
            assert next(t for t in current['tasks'] if t['id']=='held-worker') and not next(t for t in current['tasks'] if t['id']=='held-worker').get('attempts')
            assert ledger(owned)==money;read_usage(owned,claim)
            if renew:
                row=native_row(owned,claim);assert row[:4]==(process.pid,peer,1,'applied') and row[4],row
                assert next(s for s in current['planning']['scopes'] if s['id']=='root')['generation']==scope['generation']+1
                with database(owned) as db:
                    events=db.execute("SELECT request FROM events WHERE work_id=? AND kind='planning.lease-renew'",[owned.work]).fetchall();assert events
                    assert all(json.loads(raw)['generation']==scope['generation'] and json.loads(raw)['holder']==scope['holder'] for (raw,) in events)
                    assert db.execute('SELECT generation FROM native_planning_runs WHERE claim_id=?',[claim]).fetchone()==(scope['generation'],)
            again=owned.ok(['planning','reconcile',claim]);assert again['replayed'] and again['work']==current
            assert (owned.root/'model-received.json').read_bytes()==first and ledger(owned)==money
            result=dict(root=str(owned.root),provider_executions=1,decision_applied=True,sql_lock_recovered=inject,same_claim_renewed=renew,past_initial_lease=renew,accepted=False)
    except BaseException as error:primary=error
    cleanup=[]
    for fn in [lambda:connection.close() if connection is not None else None,listener.close,selector.close,lambda:os.close(pidfd) if pidfd is not None else None,lambda:os.close(childfd) if childfd is not None else None,lambda:process.wait() if process is not None and process.poll() is None else None,stdout.close,stderr.close]:
        try:fn()
        except BaseException as e:cleanup.append(e)
    if primary or cleanup:
        errors=([primary] if primary is not None else [])+cleanup
        if len(errors)==1:raise errors[0]
        raise BaseExceptionGroup('Owned planning replay and cleanup failed',errors)
    return result


def failed_calls(binary,out):
    """Known-reaped transport and business refusals retire the call and deliver feedback.

    Healthy run is exercised first by the same replay. Valid JSON outside the
    scope must not create a Work decision; actual usage/held estimates remain
    distinct. A repeated request never re-infers, and error status stays nonzero.
    """
    results=[]
    for label,nonzero,requirement,expected in [('exit-refusal',True,'req-1','planning-provider-exit-unsuccessful'),('business-refusal',False,'req-999','planning-operation-requirements-invalid')]:
        owned=fixture.Fixture(binary,out,label);fixture.publish(owned.root/'codex',(PROGRAM+('\nsys.exit(9)\n' if nonzero else '')).encode());os.chmod(owned.root/'codex',0o700)
        enabled=configured(owned,label);claim=label+'-claim';w=owned.ok(['planning','claim',owned.work],claim_request(enabled,claim,lease_seconds=120));scope=next(s for s in w['planning']['scopes'] if s['id']=='root')
        fixture.publish(owned.root/'planning-proposal.json',dict(input_events=scope['delivery']['events'],reason='Owned rejected proposal',operations=[dict(kind='task',id='rejected-worker',title='Rejected fixture',requirements=[requirement],deliverable='Future report',criteria=['Future validation'])]))
        request=preview_request(w,claim);assert owned.ok(['planning','call-preview',owned.work],request)['prepared_inputs_verified']
        before=owned.ok(['work','show',owned.work]);money=ledger(owned)
        if nonzero:
            code,wire=owned.call(['planning','run',owned.work],request)
            assert code!=0 and wire['failure']['code']=='work-state-preserved' and expected in str(wire),wire
            after=owned.ok(['work','show',owned.work]);assert after['work']['revision']==before['work']['revision']+1 and not after['work'].get('tasks')
            assert after['work']['planning']['decisions']==0 and any(e['kind']=='planning_rejected' for e in after['work']['planning']['inbox']);before=after
        else:
            code,wire=owned.call(['planning','run',owned.work],request)
            assert code!=0 and wire['failure']['code']=='work-state-preserved' and expected in str(wire),wire
            after=owned.ok(['work','show',owned.work])
            assert after['work']['revision']==before['work']['revision']+1 and not after['work'].get('tasks')
            assert after['work']['planning']['decisions']==0
            assert any(e['kind']=='planning_rejected' and not e.get('decision') for e in after['work']['planning']['inbox'])
            before=after
        assert ledger(owned)==money
        row=native_row(owned,claim);assert row[3]=='refused' and row[4],('Known reaped refusal left execution ownership live',row)
        marker=(owned.root/'model-received.json').read_bytes()
        with database(owned) as db:
            usage=db.execute('SELECT quality,input_tokens,output_tokens,provider_reported_cost_usd FROM native_planning_usage WHERE claim_id=?',[claim]).fetchone()
            assert usage==(('incomplete',None,None,None) if nonzero else ('observed',7,8,None)),usage
        snapshot=owned.snapshot();owned.denied(['planning','run',owned.work],request,'planning-run-already-recorded')
        assert owned.snapshot()==snapshot and (owned.root/'model-received.json').read_bytes()==marker and ledger(owned)==money
        owned.denied(['planning','reconcile',claim],None,'planning-receipt-not-successful' if nonzero else 'planning-receipt-business-refused')
        assert owned.ok(['work','show',owned.work])==before and owned.snapshot()==snapshot and ledger(owned)==money
        results.append(dict(reason=expected,phase='refused',decisions=0,provider_executions=1,reservations_unchanged=True))
    return results


def main():
    """Retain exact source/default-binary identity and original failure evidence."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    sources=[Path(__file__),*(Path('tests')/n for n in ['rust_agent_model_route.py','rust_planning_activation.py','rust_planning_bootstrap.py','rust_planning_call_preview.py','rust_planning_budget.py'])]
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(p):fixture.digest(p.read_bytes()) for p in sources}))
    try:
        result=happy(binary,out); failed=failed_calls(binary,out); disabled=disabled_cases(binary,out); healthy=held(binary,out,"held-control",False,False); recovery=held(binary,out,"held-recovery",True,False); renewal=held(binary,out,"held-renewal",False,True); timers=[]
        for stop in ("silence","maximum"):
            control=held(binary,out,stop+"-control",False,False,stop,False);fault=held(binary,out,stop+"-fault",False,False,stop,True);timers.append(dict(control=control,fault=fault))
        recorded=json.loads((out/'inputs.json').read_bytes());assert fixture.digest(binary.read_bytes())==recorded['binary_sha256'];assert all(fixture.digest(Path(n).read_bytes())==v for n,v in recorded['sources'].items())
        fixture.publish(out/'result.json',dict(status='PASS',happy=result,timers=timers,failed=failed,disabled=disabled,held_control=healthy,recovery=recovery,renewal=renewal,scope='Owned inert-provider native run, refusal, same-claim renewal and SQL-lock recovery; global migration remains partial',real_provider=False,installed=False))
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':main()
