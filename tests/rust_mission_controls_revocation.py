#!/usr/bin/env python3
"""Actual control permission, public consent revocation and owned termination.

A socket-held successful control with the same preconditions precedes the fault.
No authority SQL writes, live Root, synthetic receipt, sleeps or global watchdog.
Read-only ledger observations bind the actual kernel peer to its permitted slot.
Revocation after real exec must retain uncertainty/receipts and cannot become PASS.
This proves ongoing revocation, not every before-grant race or OS confinement.
"""
import argparse
import json
import os
from pathlib import Path
import select
import socket
import traceback
import rust_agent_model_route as fixture
import rust_mission_report_relay as relay
import rust_mission_watch_worker as worker
import rust_mission_watch_public as public
import rust_native_parent_death as parent_death
check=worker.check


def ledger(owned):
    """Read the isolated fixture's actual slot/owner without modifying authority."""
    return relay.rows(owned,'SELECT j.id,j.phase,j.owner_pid,s.child_pid,s.permission,s.phase FROM native_validation_jobs j JOIN native_validation_steps s ON s.job_id=j.id WHERE j.work_id=?',(owned.work,))


def scenario(binary,out,revoke,expect_reviewer=False,fail_control=False,return_fixture=False):
    """Require real permitted peer before either release or public mission stop.

    Held descriptors and actual process exit drive observation without a timeout.
    Native watch owns child termination/reap; this parent only closes its peer and
    observation handles, preserving every primary and cleanup exception.
    fail_control deliberately denies the saved check handshake; return_fixture
    returns this inactive Root after cleanup for a resumed-watch regression.
    """
    out.mkdir();owned,producer,report=relay.prepare(binary,out,controls=True,control_gate=True)
    before_agent=owned.ok(['agent','show',producer['agent']])['agent'];costs=owned.ok(['costs','show',owned.work])
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);listener.bind(str(owned.root/'control-probe.sock'));listener.listen(1)
    peer=childfd=None;observed={};errors=[];events=None
    try:
        def started(process,watchfd):
            nonlocal peer,childfd
            ready,_,_=select.select([listener,watchfd],[],[])
            check(listener in ready and watchfd not in ready,'watch exited before actual control rendezvous')
            peer,_=listener.accept();identity,childfd=worker.owned_provider_peer(peer,worker.observation(process.pid))
            ready,_,_=select.select([peer,childfd,watchfd],[],[])
            check(peer in ready,'control exited before actual entry frame');check(peer.recv(5)==b'ENTRY','wrong control entry frame')
            rows=ledger(owned);check(len(rows)==1 and rows[0][2]==process.pid and rows[0][3]==identity['pid'] and rows[0][4]==1,'actual control peer not bound to permitted native slot')
            observed.update(identity=identity,permission_rows=rows)
            fixture.publish(out/'actual-entry.json',observed)
            if revoke:
                public.mission_control(owned,'stop','revoke-real-control-consent')
                ready,_,_=select.select([childfd,watchfd],[],[])
                check(childfd in ready,'watch exited before actual controlled child termination')
            else:peer.sendall(b'F' if fail_control else b'G')
        events=relay.cycle(owned,out/'watch',started=started,refusal=revoke or fail_control,completed_event='mission-review-recorded' if expect_reviewer else 'mission-controls-recorded')
    except BaseException as error:errors.append(error)
    finally:
        for resource in [peer,listener]:
            if resource is not None:
                try:resource.close()
                except BaseException as error:errors.append(error)
        if childfd is not None:
            try:os.close(childfd)
            except BaseException as error:errors.append(error)
    if errors:raise BaseExceptionGroup('native control rendezvous/revocation/cleanup',errors)
    current=relay.work(owned);task=next(t for t in current['tasks'] if t['id']=='task')
    check(owned.ok(['agent','show',before_agent['id']])['agent']==before_agent,'control changed original worker')
    check(owned.ok(['costs','show',owned.work])==costs,'control reset original consumption')
    check(task['status']!='accepted','control fabricated acceptance')
    if expect_reviewer and not revoke and not fail_control:
        check(task['independent_review']['state']=='passed' and len([e for e in events if e.get('event')=='mission-review-recorded'])==1,'held positive missing actual reviewer')
    else:check(not task.get('independent_review'),'failed/revoked control fabricated review')
    if revoke:
        failure=json.loads((out/'watch/watch.stderr').read_bytes())
        check('planning-mission-paused' in failure['failure']['causes'],'wrong revocation diagnostic')
        check(failure['recorded_work']['id']==owned.work,'lost known control claim after revocation')
        check(not task.get('automatic_validation') and not any(e.get('event')=='mission-controls-recorded' for e in events),'revoked controls published success')
        rows=ledger(owned);check(len(rows)==1 and rows[0][1] in ('active','unresolved'),'revoked command manufactured terminal success')
        receipts=list((owned.root/'.swarm').glob('native-validation-*.receipt'))
        check(receipts,'revocation lost original durable observations')
        steps=[]
        for path in receipts:
            magic,extent,digest,body=path.read_bytes().split(b'\n',3)
            check(magic==b'SR1' and len(extent)==20 and extent.isdigit() and int(extent)==len(body) and fixture.digest(body)==digest.decode(),'original native receipt frame integrity failed')
            if path.name.startswith('native-validation-step-'):steps.append(json.loads(body))
        check(len(steps)==1,'missing exact original step observation')
        step=steps[0]
        check(step['waited'] and step['grant_attempted'] and step['channel_closed'] and step['failed'] and step['signal']==15 and step['exit_code'] is None,'revoked control lost actual wait/TERM/failure facts')
        check(step['step']['child_pid']==observed['identity']['pid'] and step['step']['permission']==1,'durable step does not bind actual permitted peer')
        fixture.publish(out/'retained-journal-names.json',[str(p.relative_to(owned.root)) for p in receipts])
        fixture.publish(out/'actual-terminated-step.json',step)
    elif fail_control:
        failure=json.loads((out/'watch/watch.stderr').read_bytes())
        check('task-validation-controls-failed' in failure['failure']['causes'] and failure['recorded_work']['id']==owned.work,'wrong recorded failed-control diagnostic')
        validation=task['automatic_validation'];check(task['status']=='blocked' and validation['state']=='blocked' and validation['controls'][0]['executed'] and not validation['controls'][0]['passed'] and validation['controls'][0]['exit_code']!=0,'actual failed control did not durably block the task')
    else:
        validation=task['automatic_validation'];check(validation['controls'][0]['passed'] and validation['controls'][0]['review_output']=='NATIVE-CONTROL-RELEASED\n','held positive control did not execute/pass')
        check(len([e for e in events if e.get('event')=='mission-controls-recorded'])==1,'held positive missing recorded event')
    fixture.publish(out/'public-work.json',current)
    result=dict(root=str(owned.root),actual_permitted_peer=True,public_revocation=revoke,failed_control=fail_control,task_accepted=False,automatic_reviewer=expect_reviewer and not revoke and not fail_control)
    return (result,owned) if return_fixture else result


def main():
    """Validate default binary bytes, retain a valid control before fault and use
    the established subreaper restoration boundary without claiming global G3.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:parser.add_argument('--'+flag,required=True,type=Path)
    parser.add_argument('--reviewer',action='store_true',help='Require automatic review in the positive control only')
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True);binary=args.binary.resolve();profile=json.loads(args.profile.read_bytes())
    check(profile['no_fixture_feature'] and fixture.digest(binary.read_bytes())==profile['sha256'],'revocation binary/profile mismatch')
    def run():
        control=scenario(binary,out/'control',False,expect_reviewer=args.reviewer);fault=scenario(binary,out/'revoked',True)
        return dict(status='PASS',positive_control=control,revocation=fault,installed=False,global_watchdog=None)
    result=worker.subreaper_scope(run,lambda e:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(e)))),parent_death.subreaper)
    fixture.publish(out/'result.json',result);print('PASS actual permitted control stops after public consent revocation; durable uncertainty preserved',flush=True)


if __name__=='__main__':main()
