#!/usr/bin/env python3
"""Recover a real automatic worker's failed SQL completion through mission watch.

The healthy public journey precedes the same held local provider under a stopped
web owner. Only an isolated fixture DDL trigger refuses final settlement; its
original durable receipt and kernel exit are retained. Missing/invalid originals
must not mutate Agent/resources; restored original is recovered under a new lease.
No paid provider, live Root, positive runtime SQL seed, retry or global watchdog.
"""
import argparse
import json
import os
from pathlib import Path
import select
import signal
import socket
import sqlite3
import subprocess
import traceback
from urllib.parse import urlencode
import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as planning
import rust_mission_watch_public as public
import rust_mission_watch_registered as registered
import rust_mission_watch_worker as worker
import rust_native_parent_death as parent_death
import rust_native_http_cli as native
import rust_native_completion_recovery as manual
import rust_workspace_fifo_completion as completion
from rust_mission_report_relay import rows
from rust_native_http_mission import request


def snapshot(f, agent):
    """Read owned canonical settlement resources, excluding advancing lease heartbeat.

    Read-only SQL observes exact bytes without Root bootstrap altering or rejecting
    injected fixture schemas. This does not replace later public CLI/HTTP checks.
    SQL/close failures propagate through the established rows helper.
    """
    return dict(agent=rows(f,'SELECT CAST(body AS TEXT) FROM agents WHERE id=?',(agent,)),
                work=rows(f,'SELECT CAST(body AS TEXT) FROM works WHERE id=?',(f.work,)),
                reservation=rows(f,'SELECT work_id,amount,state FROM reservations WHERE agent_id=?',(agent,)),
                turns=rows(f,'SELECT * FROM workspace_turns WHERE work_id=? AND agent_id=?',(f.work,agent)),
                turn_events=rows(f,'SELECT * FROM workspace_turn_events WHERE work_id=? AND agent_id=? ORDER BY event_seq',(f.work,agent)),
                funding=rows(f,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(f.work,)))


def watch(f, binary, out, label, expected=None):
    """Run one owned watch until explicit recovery/wait evidence or actual failure.

    Complete JSONL frames and inotify/pidfd drive progress without elapsed-time
    decisions. An ordinary active-context refusal is a discriminating old-code
    counterexample, never a reason to wait indefinitely. Stop only our Popen child;
    preserve primary and all signal/wait/close failures. No automatic retry.
    """
    out.mkdir(mode=0o700);process=None;notify=descriptor=None;channels=[];errors=[];events=[];code=None
    try:
        notify=worker.watch_directory(out,modifications=True)
        stdout=worker.spool(out/'watch.stdout');channels.append(stdout)
        stderr=worker.spool(out/'watch.stderr');channels.append(stderr)
        argv=[str(binary),'mission','watch',f.work,'--root',str(f.root),'--json']
        fixture.publish(out/'argv.json',dict(argv=argv,global_watchdog=None,label=label))
        process=subprocess.Popen(argv,env=f.environment,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
        descriptor=os.pidfd_open(process.pid)
        while True:
            events=registered.records(out/'watch.stdout')
            if expected and any(e.get('event')==expected[0] and (expected[1] is None or e.get('reason')==expected[1]) for e in events):
                signal.pidfd_send_signal(descriptor,signal.SIGINT);code=process.wait();assert code==0;break
            if expected and any(e.get('event')=='mission-watch-blocked' and (e.get('reason')=='deferred-active-worker-context' or e.get('synchronization_committed') is True) for e in events):
                raise AssertionError('original completion was deferred instead of recovered')
            if select.select([descriptor],[],[],0)[0]:
                code=process.wait();assert expected is None and code!=0,(label,code,events);break
            ready,_,_=select.select([notify,descriptor],[],[])
            if notify in ready:os.read(notify,65536)
        fixture.publish(out/'result.json',dict(exit=code,events=events,expected=expected,global_watchdog=None))
    except BaseException as error:errors.append(error)
    finally:
        if process is not None:
            try:
                if process.poll() is None:
                    if descriptor is not None:signal.pidfd_send_signal(descriptor,signal.SIGINT)
                    else:process.send_signal(signal.SIGINT)
                process.wait()
            except BaseException as error:errors.append(error)
        for channel in channels:worker.retain_failure(errors,channel.close)
        for fd in [notify,descriptor]:
            if fd is not None:worker.retain_failure(errors,lambda fd=fd:os.close(fd))
    if errors:raise BaseExceptionGroup('recovery watch and cleanup',errors)
    return events,(out/'watch.stderr').read_bytes()


def fault(binary, out, baseline_probe=False):
    """Create failure only after a real automatic grant, then prove lease recovery.

    Server stop leaves the independent worker alive. Fixture trigger refuses its
    actual completion transaction after journal fsync. Removing that exact trigger
    restores only our injected schema. All malformed journal variants start from
    a verified original and retain unchanged Agent/Work/reservation/provider bytes.
    """
    out.mkdir(mode=0o700)
    _,f=planning.flow(binary,out/'planning',before_worker=True,return_fixture=True)
    root=f.root;nonce=fixture.digest(str(root).encode())
    fixture.publish(out/'fixture.json',dict(root=str(root),work=f.work))
    fixture.publish(root/'codex',worker.provider_program(nonce).encode());(root/'codex').chmod(0o700)
    (root/'watch-worker').mkdir(mode=0o700)
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);connection=server=None
    supervisor=provider=None;supervisor_fd=provider_fd=server_fd=None;reaped=False;injected=False;errors=[];primary=None
    try:
        listener.bind(str(root/'watch-provider.sock'));os.chmod(root/'watch-provider.sock',0o600);listener.listen(1)
        server=native.Server(binary,root,out,'owner',environment=f.environment)
        identity=worker.observation(server.process.pid);server_fd=worker.attach(identity)
        csrf=server.request('/api/v1/native-session')['csrf_token']
        current=f.ok(['work','show',f.work])['work']
        path='/api/v1/mission?'+urlencode(dict(work=f.work,action='start'))
        request(server,'POST',path,dict(schema_version=1,event_id='recovery-start',expected_revision=current['revision'],reason='Owned recovery proof',slots=1,profile=dict(provider='fixture',role='worker',workspace='watch-worker',level='standard',capture_output=True)),csrf)
        ready,_,_=select.select([listener,server_fd],[],[]);assert listener in ready and server_fd not in ready
        connection,_=listener.accept()
        supervisor,supervisor_fd,provider,provider_fd,agent=public.attach_provider(connection,identity,binary,root)
        assert worker.frame(connection)['nonce']==nonce
        active=worker.wait_running(f,agent,server_fd,supervisor_fd)
        worker.verify_current_root_binding(active['native_mission'],root)
        assert completion.reservation(f,agent)==['reserved',1.0]
        server.stop();native.finish(server,None);server=None
        assert worker.observation(supervisor['pid'])['ppid']==os.getpid()
        connection.sendall(b'Q');assert worker.frame(connection)['pid']==provider['pid']
        fixture.publish(out/'granted-original.json',dict(agent=agent,supervisor=supervisor,provider=provider))
        db=sqlite3.connect(root/'.swarm/state.db')
        try:db.execute(manual.SQL);db.commit();injected=True
        finally:db.close()
        fixture.publish(root/'watch-worker/release-worker.json',dict(nonce=nonce,release=True))
        select.select([supervisor_fd],[],[])
        waited,status=os.waitpid(supervisor['pid'],0);reaped=True
        assert waited==supervisor['pid'] and os.WIFEXITED(status) and os.WEXITSTATUS(status)!=0
        journals=list((root/'.swarm').glob('native-completion-*.json'));assert len(journals)==1
        journal=journals[0];original=journal.read_bytes();value=json.loads(original)
        assert value['expected']['id']==agent and value['may_have_started'] is True
        assert value['details']['kernel_reaped'] is True and value['details']['workspace_release_confirmed'] is True
        fixture.publish(out/'original-completion.json',original)
        fixture.publish(out/'completion-refused.json',dict(kernel_exit=os.WEXITSTATUS(status),state=snapshot(f,agent),trigger=manual.SQL))
        manual.restore_trigger(f);injected=False
        current=f.ok(['work','show',f.work])['work']
        f.ok(['planning','pause',f.work],dict(schema_version=1,event_id='recovery-isolate-planning',expected_revision=current['revision'],reason='Isolate completion recovery from later roles'))
        before=snapshot(f,agent);provider_marker=root/'watch-worker/model-received.json';marker=provider_marker.read_bytes()
        assert json.loads(before['agent'][0][0])['status']=='running'
        if baseline_probe:
            watch(f,binary,out/'baseline-recovery','baseline',('mission-worker-completion-recovered',None))
            raise AssertionError('Baseline unexpectedly supports completion recovery')
        journal.unlink() # owned negative fixture; exact original retained above
        events,_=watch(f,binary,out/'missing','missing',('mission-watch-blocked','completion-original-pending'))
        assert snapshot(f,agent)==before and provider_marker.read_bytes()==marker
        fixture.publish(journal,original)
        print('PASS recovery-missing-original-waits-without-settlement',flush=True)
        rejects=[]
        for label,change in [('kernel-not-reaped',lambda j:j['details'].update(kernel_reaped=False)),('incomplete-output',lambda j:j.update(incomplete_output=True)),('wrong-agent',lambda j:j['expected'].update(id='unrelated')),('unknown-version',lambda j:j.update(schema_version=2))]:
            candidate=json.loads(original);change(candidate)
            fixture.publish(journal,json.dumps(candidate,separators=(',',':'),ensure_ascii=False).encode())
            events,error=watch(f,binary,out/label,label)
            assert error and snapshot(f,agent)==before and provider_marker.read_bytes()==marker,label
            rejects.append(dict(case=label,error=json.loads(error)))
            fixture.publish(journal,original)
            print('PASS recovery-rejects-'+label,flush=True)
        events,_=watch(f,binary,out/'recovered','restored',('mission-worker-completion-recovered',None))
        recovered=next(e for e in events if e.get('event')=='mission-worker-completion-recovered')
        assert recovered['agent_id']==agent and recovered['terminal_verified'] is True and recovered['provider_started_by_recovery'] is False
        final=f.ok(['agent','show',agent])['agent'];assert final['status']=='completed'
        work=f.ok(['work','show',f.work])['work'];task=next(t for t in work['tasks'] if t['id']=='task')
        previous=next(t for t in json.loads(before['work'][0][0])['tasks'] if t['id']=='task')
        assert task['status'] in ['blocked','submitted']
        assert [a['id'] for a in task['attempts']]==[a['id'] for a in previous['attempts']]
        assert completion.reservation(f,agent)==['estimated',1.0]
        turn=completion.one_turn(f.ok(['mission','queue',f.work]),f.work,agent)
        assert turn['state']=='released'
        releases=rows(f,"SELECT state FROM workspace_turn_events WHERE work_id=? AND agent_id=? AND state='released'",(f.work,agent))
        assert releases==[('released',)]
        assert len(f.ok(['mission','status-native',f.work])['agents'])==1
        assert journal.read_bytes()==original and provider_marker.read_bytes()==marker
        links=rows(f,"SELECT r.generation,e.kind,e.revision,r.work_revision FROM native_mission_work_receipts r JOIN events e ON e.work_id=r.work_id AND e.id=r.event_id WHERE r.work_id=? AND e.id LIKE 'finish-%'",(f.work,))
        assert len(links)==1 and links[0][0]>active['native_mission']['generation'] and links[0][1]=='task.update' and links[0][2]==links[0][3]
        repeat=f.ok(['agent','reconcile',agent]);assert repeat['completion_recovered'] is True and repeat['cancelled_before_start'] is False
        server=native.Server(binary,root,out,'reopened',environment=f.environment)
        view=request(server,'GET','/api/v1/mission?'+urlencode(dict(work=f.work,action='show')))
        assert view['consent_enabled'] is True
        limited=request(server,'GET','/api/v1/mission?'+urlencode(dict(work=f.work,action='status')),expected=500)
        assert limited['failure']['code']=='mission-status-planning-unported'
        assert agent in json.dumps(f.ok(['mission','status-native',f.work]))
        server.stop();native.finish(server,None);server=None
        assert provider_marker.read_bytes()==marker and len(f.ok(['mission','status-native',f.work])['agents'])==1
        fixture.publish(out/'result.json',dict(status='PASS',agent=agent,rejected=rejects,recovery_event=recovered,lease_event_links=links,http_historical_status_limitation='mission-status-planning-unported',no_provider_rerun=True,one_agent=True,paid_provider=False,global_watchdog=None))
        print('PASS recovery-restored-original-settles-under-new-lease-without-rerun',flush=True)
    except BaseException as error:
        primary=error
        worker.retain_failure(errors,lambda:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(error)))))
    finally:
        if injected:worker.retain_failure(errors,lambda:manual.restore_trigger(f))
        if server is not None:worker.retain_failure(errors,lambda:native.finish(server,None))
        if primary is not None:
            release=root/'watch-worker/release-worker.json'
            if not release.exists():worker.retain_failure(errors,lambda:fixture.publish(release,dict(nonce=nonce,release=True)))
            if supervisor is not None and not reaped:worker.retain_failure(errors,lambda:public.reap_cleanup_helper(supervisor))
            worker.retain_failure(errors,lambda:public.cleanup_owned_children(binary,root,out))
        for item in [connection,listener]:
            if item is not None:worker.retain_failure(errors,item.close)
        for fd in [server_fd,supervisor_fd,provider_fd]:
            if fd is not None:worker.retain_failure(errors,lambda fd=fd:os.close(fd))
    if primary is not None or errors:raise BaseExceptionGroup('worker recovery and cleanup',([primary] if primary else [])+errors)


def main():
    """Bind qualified immutable binary; healthy control precedes injected refusal."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--baseline-probe',action='store_true',help='Discriminate restored-original recovery on the retained old binary')
    args=parser.parse_args();binary=args.binary.resolve();out=args.output.resolve();out.mkdir(mode=0o700,exist_ok=False)
    profile=json.loads(args.profile.read_bytes());assert profile['no_fixture_feature'] and profile['sha256']==fixture.digest(binary.read_bytes())
    def cases():
        """Use separate Roots for healthy completion and retained completion refusal."""
        public.run_case(binary,out/'healthy','sigint')
        print('PASS recovery-healthy-worker-control',flush=True)
        fault(binary,out/'fault',args.baseline_probe)
        fixture.publish(out/'result.json',dict(status='PASS',healthy_control=True,completion_recovery=True,global_watchdog=None,installed=False,binary_sha256=profile['sha256']))
        print('MISSION_WORKER_RECOVERY_PASS',flush=True)
    worker.subreaper_scope(cases,lambda e:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(e)))),parent_death.subreaper)


if __name__=='__main__':main()
