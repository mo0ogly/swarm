#!/usr/bin/env python3
"""Probe actual in-flight automatic planner after public stop then fresh start.

Positive witness is the full automatic journey696 on the same profile. This
fresh fixture holds the first real permitted planner at its proposal socket,
withdraws consent with public stop and restores it through fresh public start. No SQL writes, fake receipts, global watchdog or paid provider.
"""
import argparse,json,os,select,signal,socket,subprocess,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_mission_watch_planning as planning
import rust_mission_report_relay as relay
import rust_mission_watch_worker as worker
import rust_mission_watch_public as public
import rust_native_parent_death as parent_death
check=worker.check

def run(binary,out):
    """Bind real peer to permitted planner, change only public mission controls,
    then observe actual decision/refusal and retain owned process cleanup causes.
    No poll delay or new execution limit substitutes for observable progress.
    """
    owned,nonce,program=planning.setup(binary,out/'setup')
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);listener.bind(str(owned.root/'watch-planner.sock'));listener.listen(1)
    peer=childfd=watchfd=notify=process=None;streams=[];errors=[];events=[]
    try:
        streams=[worker.spool(out/'watch.stdout'),worker.spool(out/'watch.stderr')]
        notify=worker.watch_directory(out,modifications=True)
        argv=[str(binary),'mission','watch',owned.work,'--root',str(owned.root),'--json'];fixture.publish(out/'command.json',dict(argv=argv,global_watchdog=None))
        process=subprocess.Popen(argv,env=owned.environment,stdin=subprocess.DEVNULL,stdout=streams[0],stderr=streams[1]);watchfd=os.pidfd_open(process.pid)
        ready,_,_=select.select([listener,watchfd],[],[]);check(listener in ready and watchfd not in ready,'watch exited before actual planner entry')
        peer,_=listener.accept();identity,childfd=planning.planner_peer(peer,worker.observation(process.pid));frame=worker.frame(peer)
        check(frame.get('nonce')==nonce and frame.get('pid')==identity['pid'],'actual planner peer/nonce mismatch')
        consent_before=relay.rows(owned,"SELECT seq FROM cockpit_events WHERE work_id=? AND kind='mission-policy' ORDER BY rowid DESC LIMIT 1",(owned.work,))
        current=relay.work(owned);claim=planning.bound_proposal(owned,current,'root')
        rows=relay.rows(owned,'SELECT claim_id,holder,generation,permission,phase FROM native_planning_runs WHERE work_id=? AND child_pid=? AND owner_pid=?',(owned.work,identity['pid'],process.pid))
        check(len(rows)==1 and rows[0][1:4]==(claim['holder'],claim['generation'],1) and rows[0][4]=='permitted','planner not actually permitted')
        fixture.publish(out/'actual-entry.json',dict(peer=identity,job=rows,claim=claim))
        public.mission_control(owned,'stop','planner-consent-stop')
        current=relay.work(owned);saved=current['launch_profile']
        profile={k:saved[k] for k in ['provider','role','level','capture_output']};profile['workspace']=str(Path(saved['workspace']).relative_to(owned.root))
        owned.ok(['mission','start',owned.work],dict(schema_version=1,event_id='planner-consent-new-start',expected_revision=current['revision'],reason='Explicit new consent after held planner original admission',slots=1,profile=profile))
        status=public.status_native(owned);check(status['controls']['paused'] is False and status['controls']['consent_enabled'] is True,'public start failed to restore explicit consent')
        consent_after=relay.rows(owned,"SELECT seq FROM cockpit_events WHERE work_id=? AND kind='mission-policy' ORDER BY rowid DESC LIMIT 1",(owned.work,))
        check(len(consent_before)==len(consent_after)==1 and consent_after[0][0]>consent_before[0][0],'consent sequence did not change')
        fixture.publish(out/'withdrawn-controls.json',dict(status=status,before=consent_before,after=consent_after))
        peer.sendall(b'G');peer.close();peer=None
        while True:
            raw=(out/'watch.stdout').read_bytes();events=[json.loads(s) for s in raw.splitlines(keepends=True) if s.endswith(b'\n')]
            if any(e.get('event')=='mission-planning-step' and e.get('decision_applied') for e in events):break
            ready,_,_=select.select([notify,watchfd],[],[])
            if watchfd in ready:break
            if notify in ready:os.read(notify,65536)
    except BaseException as e:errors.append(e)
    finally:
        if process is not None:
            try:
                if process.poll() is None:
                    public.mission_control(owned,'stop','planner-probe-owned-cleanup-stop')
                    signal.pidfd_send_signal(watchfd,signal.SIGINT)
                fixture.publish(out/'exit.json',dict(exit=process.wait()))
            except BaseException as e:errors.append(e)
        for resource in [peer,listener,*streams]:
            if resource is not None:
                try:resource.close()
                except BaseException as e:errors.append(e)
        for fd in [childfd,watchfd,notify]:
            if fd is not None:
                try:os.close(fd)
                except BaseException as e:errors.append(e)
    if errors:raise BaseExceptionGroup('planner consent probe/cleanup',errors)
    raw=(out/'watch.stdout').read_bytes();events=[json.loads(s) for s in raw.splitlines() if s.strip()];current=relay.work(owned)
    fixture.publish(out/'public-work.json',current);fixture.publish(out/'fixture.json',dict(root=str(owned.root),work=owned.work,binary_sha256=fixture.digest(binary.read_bytes())))
    check(not any(e.get('event')=='mission-planning-step' and e.get('decision_applied') for e in events),'automatic planner applied a decision from an earlier consent after stop and new consent')
    check(not current.get('tasks') and len(current['planning']['scopes'])==1,'old-consent planner created delegated work')
    failure=json.loads((out/'watch.stderr').read_bytes());check('stale_lease' in failure['failure']['causes'] and failure['recorded_work']['id']==owned.work,'old scope was not refused with its known claim and stale lease cause')
    fixture.publish(out/'public-failure.json',failure)
    return dict(status='PASS',root=str(owned.root),work=owned.work,actual_permitted_planner=True,old_scope_lease_refused=True,task_accepted=False,installed=False)

def main():
    """Use the supplied qualified profile; retain original failure and restore
    owned subreaper without managing any live/shared Root or process.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:p.add_argument('--'+flag,type=Path,required=True)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True);binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes())
    check(profile['no_fixture_feature'] and profile['sha256']==fixture.digest(binary.read_bytes()),'planner consent binary/profile mismatch')
    result=worker.subreaper_scope(lambda:run(binary,out),lambda e:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(e)))),parent_death.subreaper)
    fixture.publish(out/'result.json',result);print('PASS in-flight automatic planner refuses withdrawn consent after consent epoch replacement',flush=True)
if __name__=='__main__':main()
