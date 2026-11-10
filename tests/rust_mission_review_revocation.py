#!/usr/bin/env python3
"""Actual permitted reviewer held before reply, then public consent revocation.

A healthy held reviewer precedes the fault with the same preconditions. No live
Root, SQL writes, fake receipts, retry, paid provider or global watchdog. Existing
native source polling and owned TERM/wait retain uncertain/failed publication.
"""
import argparse,json,os,select,socket,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_mission_report_relay as relay
import rust_mission_watch_worker as worker
import rust_mission_watch_public as public
import rust_native_parent_death as parent_death
check=worker.check

def scenario(binary,out,revoke):
    """Bind the actual reviewer peer to its canonical permission and owner,
    release a valid quote or stop publicly, then assert journal/Work/funding.
    Owned cleanup preserves all primary/secondary failures. An unfinished job
    never becomes successful merely because its process was terminated.
    """
    out.mkdir();owned,producer,report=relay.prepare(binary,out,controls=True,review_gate=True)
    before=relay.work(owned);calls=before['planning']['reviewer'].get('calls',0)
    finance_before=relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);listener.bind(str(owned.root/'reviewer-probe.sock'));listener.listen(1)
    peer=childfd=None;observed={};errors=[];events=None
    try:
        def started(process,watchfd):
            nonlocal peer,childfd
            ready,_,_=select.select([listener,watchfd],[],[]);check(listener in ready and watchfd not in ready,'watch exited before reviewer entry')
            peer,_=listener.accept();identity,childfd=worker.owned_provider_peer(peer,worker.observation(process.pid))
            ready,_,_=select.select([peer,childfd,watchfd],[],[]);check(peer in ready and peer.recv(5)==b'ENTRY','reviewer exited before entry frame')
            rows=relay.rows(owned,'SELECT id,permission,phase,child_pid,owner_pid,work_revision FROM native_review_jobs WHERE work_id=?',(owned.work,))
            check(len(rows)==1 and rows[0][1:5]==(1,'permitted',identity['pid'],process.pid),'actual reviewer not bound to permitted canonical job')
            observed.update(peer=identity,job=rows[0]);fixture.publish(out/'actual-entry.json',observed)
            if revoke:
                public.mission_control(owned,'stop','revoke-real-reviewer-consent')
                ready,_,_=select.select([childfd,watchfd],[],[]);check(childfd in ready,'watch exited before owned reviewer termination')
            else:peer.sendall(b'G')
        events=relay.cycle(owned,out/'watch',started=started,refusal=revoke,completed_event='mission-review-recorded')
    except BaseException as e:errors.append(e)
    finally:
        for resource in [peer,listener]:
            if resource is not None:
                try:resource.close()
                except BaseException as e:errors.append(e)
        if childfd is not None:
            try:os.close(childfd)
            except BaseException as e:errors.append(e)
    if errors:raise BaseExceptionGroup('reviewer rendezvous/revocation/cleanup',errors)
    current=relay.work(owned);task=next(t for t in current['tasks'] if t['id']=='task')
    check(task['status']!='accepted','reviewer fabricated task acceptance')
    receipts=list((owned.root/'.swarm').glob('native-review-*.receipt'));check(len(receipts)==1,'missing original reviewer receipt')
    magic,extent,digest,body=receipts[0].read_bytes().split(b'\n',3)
    check(magic==b'SR1' and len(extent)==20 and int(extent)==len(body) and fixture.digest(body)==digest.decode(),'reviewer receipt frame integrity failed')
    receipt=json.loads(body);check(receipt['job']['child_pid']==observed['peer']['pid'] and receipt['job']['permission']==1,'journal not bound to real reviewer')
    check(receipt['waited'] and receipt['grant_attempted'] and receipt['channel_closed'],'reviewer lost actual process lifecycle')
    finance_after=relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))
    check(all(row in finance_after for row in finance_before) and len([r for r in finance_after if r[0]==observed['job'][0] and r[1]=='reviewer'])==1,'reviewer reset or repeated funding')
    check(current['planning']['reviewer']['calls']==calls+1,'reviewer call was lost or repeated')
    if revoke:
        failure=json.loads((out/'watch/watch.stderr').read_bytes());check('planning-mission-paused' in failure['failure']['causes'],'wrong reviewer revocation reason')
        check(failure['recorded_work']['id']==owned.work and failure['recorded_work']['revision']>=observed['job'][5],'revocation lost known reviewer claim')
        check(receipt['failed'] and receipt['signal']==15 and receipt['exit_code'] is None and receipt['original_reply'] is None,'reviewer revocation lost TERM/failed/incomplete reply facts')
        check(not any(e.get('event')=='mission-review-recorded' and e.get('verdict_applicable') for e in events) and task['independent_review']['state']!='passed','revoked reviewer published favorable result')
        jobs=relay.rows(owned,'SELECT phase FROM native_review_jobs WHERE id=?',(observed['job'][0],));check(len(jobs)==1 and jobs[0][0]!='recorded','revoked job silently became recorded success')
    else:
        check(not receipt['failed'] and receipt['exit_code']==0 and receipt['signal'] is None and task['independent_review']['state']=='passed','held healthy reviewer did not pass')
        check(len([e for e in events if e.get('event')=='mission-review-recorded' and e.get('verdict_applicable')])==1,'missing actual held reviewer result')
    fixture.publish(out/'public-work.json',current);fixture.publish(out/'finance-before.json',finance_before);fixture.publish(out/'finance-after.json',finance_after)
    return dict(root=str(owned.root),work=owned.work,actual_permitted_peer=True,public_revocation=revoke,task_accepted=False)

def main():
    """Require profile bytes and a healthy same-precondition witness before
    revocation, restoring owned subreaper and retaining original failures.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:p.add_argument('--'+flag,type=Path,required=True)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True);binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes());check(profile['no_fixture_feature'] and profile['sha256']==fixture.digest(binary.read_bytes()),'reviewer revocation binary/profile mismatch')
    def run():return dict(status='PASS',positive_control=scenario(binary,out/'positive',False),revocation=scenario(binary,out/'revoked',True),installed=False,global_watchdog=None)
    result=worker.subreaper_scope(run,lambda e:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(e)))),parent_death.subreaper)
    fixture.publish(out/'result.json',result);print('PASS actual permitted reviewer terminates after public consent revocation without favorable verdict',flush=True)
if __name__=='__main__':main()
