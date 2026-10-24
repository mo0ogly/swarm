#!/usr/bin/env python3
"""Require public recovery after an isolated real native completion write refusal.

Both healthy/fault runs use the same inert local provider and owned Unix-socket
barrier. Wait for actual native persistence of declared usage before injecting a
named final-Work-update trigger: Root schema checks must not mask the fault before
execution. Restore only that fixture trigger before public reconcile. No model,
account, live mission, Go process, retry or global watchdog. Observation cadence
comes from the saved supervisor poll setting; it is not a verification deadline.
"""
import argparse,errno,json,os,selectors,socket,sqlite3,struct,subprocess,time,traceback
from pathlib import Path
import rust_agent_model_route as m

GATE='completion-gate.sock'
PROGRAM=m.PROGRAM+"\nimport socket\nwith socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as gate:\n    gate.connect('completion-gate.sock')\n    gate.recv(1)\n"
SQL="CREATE TRIGGER fixture_completion_refusal BEFORE UPDATE ON works WHEN json_extract(NEW.body,'$.tasks[0].status')='blocked' BEGIN SELECT RAISE(ABORT,'Owned completion refusal'); END"


def observe(f):
    """Read public lifecycle/reservation after the owned fixture schema is restored."""
    work=f.ok(['work','show',f.work])['work'];agent=f.ok(['agent','show','session'])['agent']
    return dict(work_revision=work['revision'],task_status=work['tasks'][0]['status'],attempts=work['tasks'][0].get('attempts'),agent_status=agent['status'],native_result_present=agent.get('native_execution_result') is not None,agent_ended=agent.get('ended'),supervisor_pid=agent.get('supervisor_pid'),child_pid=agent.get('child_pid'),reservation=f.reservation())


def actual_agent(f):
    """Observe only owned live metadata; never mutate runtime rows or consumption."""
    db=sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True)
    try:
        row=db.execute('SELECT body FROM agents WHERE id=?',('session',)).fetchone();assert row
        return json.loads(row[0])
    finally:db.close()


def restore_trigger(f):
    """Remove only our exact injected DDL after retaining its identity; no live root."""
    db=sqlite3.connect(f.root/'.swarm/state.db')
    try:
        saved=db.execute("SELECT sql FROM sqlite_schema WHERE type='trigger' AND name='fixture_completion_refusal'").fetchone();assert saved and saved[0]==SQL,saved
        db.execute('DROP TRIGGER fixture_completion_refusal');db.commit()
    finally:db.close()


def run_held(f,inject,before_release=None):
    """Run one actual provider; close owned socket on every failure to release it.

    A pidfd distinguishes CLI exit from missing progress. Socket peer credentials
    and recorded child PID must match. SQL fault is introduced only after actual
    permission/provider observation and persisted usage. No time cutoff is added.
    """
    assert hasattr(os,'pidfd_open'),'This Linux fixture requires pidfd observation'
    m.publish(f.root/'codex',PROGRAM.encode());os.chmod(f.root/'codex',0o700)
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);connection=None;selector=selectors.DefaultSelector();pidfd=None;process=None;injected=False
    out=(f.output/(f.label+'-run.stdout')).open('xb');err=(f.output/(f.label+'-run.stderr')).open('xb')
    argv=[str(f.binary),'agent','run','session','--root',str(f.root),'--json'];started=None
    try:
        listener.bind(str(f.root/GATE));os.chmod(f.root/GATE,0o600);listener.listen(1);listener.setblocking(False)
        cadence=int(f.ok(['engine-settings','show'])['values']['agent_supervision_poll_interval_ms'])/1000;assert cadence>0
        prepared=f.prepare();assert observe(f)['task_status']=='running'
        started=time.time();process=subprocess.Popen(argv,env=f.environment,stdin=subprocess.DEVNULL,stdout=out,stderr=err)
        m.publish(f.output/(f.label+'-owned-process.json'),dict(argv=argv,pid=process.pid,root=str(f.root),injected=inject,global_watchdog=None,poll_seconds=cadence))
        pidfd=os.pidfd_open(process.pid);selector.register(pidfd,selectors.EVENT_READ,'exit');selector.register(listener,selectors.EVENT_READ,'gate')
        while connection is None:
            events=selector.select()
            for key,_ in events:
                if key.data=='exit':raise AssertionError('CLI exited before fixture gate connection')
                connection,_=listener.accept()
        selector.unregister(listener)
        peer,uid,_=struct.unpack('3i',connection.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert uid==os.getuid()
        while True:
            agent=actual_agent(f)
            if agent.get('usage',{}).get('output_tokens')==8:
                assert agent['child_pid']==peer and agent['supervisor_pid']==process.pid
                assert agent['status']=='running';break
            assert not selector.select(cadence),'CLI exited before persisted provider facts'
        received=f.root/'model-received.json';actual=json.loads(received.read_bytes());assert actual['stdin_sha256']==m.digest(prepared['agent']['prompt'].encode())
        if inject:
            db=sqlite3.connect(f.root/'.swarm/state.db')
            try:db.execute(SQL);db.commit();injected=True
            finally:db.close()
        if before_release is not None:before_release()
        connection.sendall(b'1');connection.close();connection=None
        code=process.wait();out.flush();err.flush();os.fsync(out.fileno());os.fsync(err.fileno())
        raw=(f.output/(f.label+('-run.stdout' if code==0 else '-run.stderr'))).read_bytes();value=json.loads(raw)
        f.calls.append(dict(argv=argv,start=started,end=time.time(),exit=code));m.publish(f.output/(f.label+'-commands.json'),f.calls)
        return code,value,prepared
    finally:
        if connection is not None:connection.close()
        listener.close();selector.close()
        if pidfd is not None:os.close(pidfd)
        if process is not None and process.poll() is None:process.wait()
        out.close();err.close()
        if injected:restore_trigger(f)


def main():
    """Retain healthy/refusal/recovery outcomes and both fixture Roots, bound to immutable source/binary."""
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--binary',required=True,type=Path);parser.add_argument('--output',required=True,type=Path);a=parser.parse_args()
    binary=a.binary.resolve();output=a.output.resolve();output.mkdir(parents=True,exist_ok=False)
    sources=[Path(__file__).resolve(),Path(m.__file__).resolve(),binary];before={str(p):m.digest(p.read_bytes()) for p in sources};m.publish(output/'inputs.json',before)
    for p in sources[:-1]:m.publish(output/('source-'+p.name),p.read_bytes())
    healthy=None;fault=None
    try:
        healthy=m.Fixture(binary,output,'healthy');code,result,_=run_held(healthy,False);assert code==0 and result['agent']['status']=='completed'
        witness=observe(healthy);assert witness['native_result_present'] and witness['task_status']=='blocked' and tuple(witness['reservation'])==('estimated',1.0)
        m.publish(output/'healthy-completion-receipt.json',next((healthy.root/'.swarm').glob('native-completion-*.json')).read_bytes())
        m.publish(output/'healthy-witness.json',witness);print('PASS healthy native completion with actual socket/usage witness',flush=True)
        fault=m.Fixture(binary,output,'completion-refusal');code,error,_=run_held(fault,True);assert code!=0,'Completion refusal became success'
        observed=observe(fault);m.publish(output/'completion-refused.json',dict(exit=code,error=error,observed=observed,root=str(fault.root),provider_observed=True))
        received=fault.root/'model-received.json';original=received.read_bytes()
        journals=list((fault.root/'.swarm').glob('native-completion-*.json'));assert len(journals)==1
        journal=journals[0];saved=journal.read_bytes();m.publish(output/'native-completion-receipt.json',saved)
        fixture=json.loads(saved);baseline_state=fault.snapshot();rejections=[]
        for label,change in [
            ('wrong-root',lambda j:j.update(directory=[0,0])),
            ('wrong-agent',lambda j:j['expected'].update(id='unrelated')),
            ('stale-snapshot',lambda j:j['expected']['native_execution'].update(context_sha256='0'*64)),
            ('incomplete-output',lambda j:j.update(incomplete_output=True)),
            ('unknown-version',lambda j:j.update(schema_version=2)),
        ]:
            candidate=json.loads(saved)
            assert json.dumps(candidate,ensure_ascii=False,separators=(',',':')).encode()==saved,'Fixture encoder changed unrelated receipt bytes'
            change(candidate);m.publish(journal,json.dumps(candidate,ensure_ascii=False,separators=(',',':')).encode())
            code,rejected=fault.call(['agent','reconcile','session']);assert code!=0,(label,rejected)
            assert fault.snapshot()==baseline_state and received.read_bytes()==original,label
            rejections.append(dict(case=label,exit=code,response=rejected))
        journal.unlink() # owned fixture only; exact original is retained in evidence
        code,rejected=fault.call(['agent','reconcile','session']);assert code!=0 and rejected['failure']['code']=='agent-completion-journal-missing'
        assert fault.snapshot()==baseline_state and received.read_bytes()==original
        rejections.append(dict(case='missing-receipt',exit=code,response=rejected))
        m.publish(journal,saved)
        code,recovered=fault.call(['agent','reconcile','session'])
        m.publish(output/'rejected-receipts.json',rejections)
        m.publish(output/'recovery-response.json',dict(exit=code,response=recovered,root=str(fault.root)));assert code==0,('public completion recovery refused after fixture cause removed',recovered)
        after=observe(fault);assert received.read_bytes()==original,'Recovery reran the provider'
        repeat=fault.ok(['agent','reconcile','session']);assert repeat['agent']['status']=='completed'
        repeated=observe(fault)
        assert after['agent_status']=='completed' and after['task_status']=='blocked' and after['native_result_present'],after
        assert tuple(after['reservation'])==('estimated',1.0)
        assert repeated==after and repeat['completion_recovered'] is True and repeat['cancelled_before_start'] is False
        assert all(m.digest(p.read_bytes())==before[str(p)] for p in sources)
        m.publish(output/'result.json',dict(status='PASS',scope='Actual local native completion/public recovery, no model/acceptance',source_stable=True,witness=witness,recovered=after,rejected_receipts=len(rejections),idempotent=True,provider_rerun=False,global_watchdog=None))
        print('PASS native completion recovery; fixture Roots retained',flush=True)
    except BaseException as error:
        m.publish(output/'result.json',dict(status='FAIL',type=type(error).__name__,cause=str(error),healthy_root=str(healthy.root) if healthy else None,fault_root=str(fault.root) if fault else None,source_stable=all(m.digest(p.read_bytes())==before[str(p)] for p in sources),global_watchdog=None))
        traceback.print_exc();raise


if __name__=='__main__':main()
