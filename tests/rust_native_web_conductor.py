#!/usr/bin/env python3
"""Exercise server-only mission dispatch on a qualified normal Rust binary.

Owned public planning setup runs two inert local processes. HTTP start then admits
one independently granted worker without a separate mission watch. Kernel sockets,
pidfds, parentage and real CLI/HTTP state prove dispatch, nonblocking controls,
lease contention, server shutdown/reopen and surviving worker completion. No paid
provider, private SQL write, Go, installation, retry or global watchdog is used.
"""
import argparse
import json
import os
from pathlib import Path
import select
import selectors
import socket
import traceback
from urllib.parse import urlencode
import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as planning
import rust_mission_watch_planning as planner
from rust_mission_report_relay import rows
import rust_mission_watch_worker as worker
import rust_mission_watch_public as public
import rust_native_parent_death as parent_death
import rust_native_http_cli as native
from rust_native_http_mission import request, foreign_inode


def run(binary, profile, out):
    """Run one fresh server-owned journey, preserving every original and cleanup error.

    The inert worker waits for an authenticated nonce release, never a timeout.
    A positive live worker precedes pause/stop/contention checks. Server stop must
    leave that worker alive; only this owned fixture's cleanup may release/signal it.
    Reopen tests revoked consent, not an unsupported managed-process recovery claim.
    """
    out.mkdir(parents=True, mode=0o700, exist_ok=False)
    qualified=json.loads(profile.read_bytes())
    assert qualified['no_fixture_feature'] and fixture.digest(binary.read_bytes())==qualified['sha256']
    ready,f=planning.flow(binary,out/'planning',before_worker=True,return_fixture=True)
    nonce=fixture.digest(str(f.root).encode());root=f.root
    fixture.publish(root/'codex',worker.provider_program(nonce).encode());(root/'codex').chmod(0o700)
    (root/'watch-worker').mkdir(mode=0o700)
    fixture.publish(out/'fixture.json',dict(root=str(root),work=f.work,setup_inert_planning=True,paid_provider=False))
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
    server=connection=supervisor=provider=None
    server_fd=supervisor_fd=provider_fd=None
    reaped=False;checks=[];errors=[];primary=None
    path=lambda action:'/api/v1/mission?'+urlencode(dict(work=f.work,action=action))
    def passed(name):
        """Retain named successful assertions, never infer PASS from process exit."""
        checks.append(name);print('PASS '+name,flush=True)
    try:
        listener.bind(str(root/'watch-provider.sock'));os.chmod(root/'watch-provider.sock',0o600);listener.listen(1)
        server=native.Server(binary,root,out,'web',environment=f.environment)
        server_identity=worker.observation(server.process.pid);server_fd=worker.attach(server_identity)
        csrf=server.request('/api/v1/native-session')['csrf_token']
        before=f.ok(['mission','status-native',f.work]);assert before['agents']==[] and before['supervision'] is None
        for action in ['show','order','queue','status']:
            assert request(server,'GET',path(action)) is not None
        passed('protected-mission-reads-before-start')
        current=f.ok(['work','show',f.work])['work']
        start=dict(schema_version=1,event_id='http-conductor-start',expected_revision=current['revision'],reason='Owned inert server conductor',slots=1,
            profile=dict(provider='fixture',role='worker',workspace='watch-worker',level='standard',capture_output=True))
        request(server,'POST',path('start'),start,csrf)
        print('STAGE awaiting-server-owned-provider',flush=True)
        with selectors.DefaultSelector() as selector:
            selector.register(listener,selectors.EVENT_READ,'provider');selector.register(server_fd,selectors.EVENT_READ,'server')
            while connection is None:
                for key,_ in selector.select():
                    assert key.data=='provider','server exited before worker entry'
                    connection,_=listener.accept()
        supervisor,supervisor_fd,provider,provider_fd,agent_id=public.attach_provider(connection,server_identity,binary,root)
        entry=worker.frame(connection);assert entry['nonce']==nonce and entry['pid']==provider['pid']
        agent=worker.wait_running(f,agent_id,server_fd,supervisor_fd)
        assert agent['status']=='running' and agent['supervisor_pid']==supervisor['pid']
        worker.verify_current_root_binding(agent['native_mission'],root)
        lease=worker.supervision(f);assert lease['stopped_at']==''
        fixture.publish(out/'running.json',dict(agent=agent_id,provider=entry,supervisor=supervisor,lease=lease))
        passed('http-start-alone-launches-real-inert-worker')
        observed=request(server,'GET',path('status'));assert agent_id in json.dumps(observed)
        assert len(f.ok(['mission','status-native',f.work])['agents'])==1
        passed('http-status-responds-during-active-session')
        code,denied=f.call(['mission','watch',f.work]);assert code!=0 and 'mission-lease-already-held' in json.dumps(denied)
        assert worker.supervision(f)==lease and len(f.ok(['mission','status-native',f.work])['agents'])==1
        fixture.publish(out/'competing-watch-refusal.json',denied)
        passed('competing-cli-watch-refused-without-duplicate-agent')
        other=None;other_error=None
        try:
            other=native.Server(binary,root,out,'contending-web',environment=f.environment)
            assert request(other,'GET',path('show'))['consent_enabled'] is True
            other.stop()
        except BaseException as error:
            other_error=error;raise
        finally:
            if other is not None:native.finish(other,other_error)
        assert worker.supervision(f)==lease and server.process.poll() is None
        assert len(f.ok(['mission','status-native',f.work])['agents'])==1
        passed('second-web-server-remains-usable-without-owning-or-releasing-first-lease')
        stable_work=f.ok(['work','show',f.work])['work']
        invalid=dict(start,event_id='invalid-server-start',expected_revision=stable_work['revision'],unexpected=True)
        request(server,'POST',path('start'),invalid,csrf,expected=400)
        request(server,'POST',path('stop'),dict(schema_version=1,event_id='csrf-refusal',expected_revision=stable_work['revision'],reason='Owned refusal'),csrf='invalid',expected=403)
        assert f.ok(['work','show',f.work])['work']==stable_work
        assert request(server,'GET',path('show'))['consent_enabled'] is True
        passed('invalid-and-unauthorized-controls-refuse-without-work-mutation-and-read-recovers')
        def control(action,event,**extra):
            """Use each fresh Work revision through the authenticated HTTP writer."""
            value=f.ok(['work','show',f.work])['work']
            return request(server,'POST',path(action),dict(schema_version=1,event_id=event,expected_revision=value['revision'],reason='Owned server control',**extra),csrf)
        control('pause','server-pause');assert request(server,'GET',path('show'))['paused'] is True
        control('priority','server-priority',task_id='task',priority='7')
        control('resume','server-resume');assert request(server,'GET',path('show'))['paused'] is False
        control('stop','server-stop');assert request(server,'GET',path('show'))['consent_enabled'] is False
        control('unpause','server-unpause');view=request(server,'GET',path('show'));assert view['paused'] is False and view['consent_enabled'] is False
        assert len(f.ok(['mission','status-native',f.work])['agents'])==1
        passed('all-six-http-writers-remain-responsive-and-stop-revokes-consent')
        connection.sendall(b'Q');assert worker.frame(connection)['nonce']==nonce
        server.stop();native.finish(server,None);server=None
        assert worker.supervision(f)['stopped_at']!=''
        assert worker.observation(supervisor['pid'])['ppid']==os.getpid()
        connection.sendall(b'Q');assert worker.frame(connection)['pid']==provider['pid']
        passed('server-shutdown-releases-lease-and-granted-worker-survives')
        server=native.Server(binary,root,out,'reopened',environment=f.environment)
        view=request(server,'GET',path('show'));assert view['consent_enabled'] is False
        assert len(f.ok(['mission','status-native',f.work])['agents'])==1
        server.stop();native.finish(server,None);server=None
        passed('reopen-preserves-revocation-and-no-duplicate-agent')
        fixture.publish(root/'watch-worker/release-worker.json',dict(nonce=nonce,release=True))
        select.select([supervisor_fd],[],[])
        waited,status=os.waitpid(supervisor['pid'],0);reaped=True
        assert waited==supervisor['pid'] and os.WIFEXITED(status) and os.WEXITSTATUS(status)==0
        final=f.ok(['agent','show',agent_id])['agent'];assert final['status']=='completed'
        task=next(t for t in f.ok(['work','show',f.work])['work']['tasks'] if t['id']=='task');assert task['status']=='blocked'
        passed('independent-worker-completes-after-server-without-fake-acceptance')
        server=native.Server(binary,root,out,'identity-witness',environment=f.environment)
        assert request(server,'GET',path('show'))['consent_enabled'] is False
        with foreign_inode(root/'.swarm/state.db'):
            assert server.process.wait()!=0,'foreign datastore replacement did not stop coordinator'
        native.finish(server,None)
        assert 'web-root-binding-changed' in server.stderr.decode()
        server=None
        passed('foreign-datastore-stops-coordinator-and-preserves-original')
        server=native.Server(binary,root,out,'identity-recovered',environment=f.environment)
        assert request(server,'GET',path('show'))['consent_enabled'] is False
        assert f.ok(['agent','show',agent_id])['agent']['status']=='completed'
        server.stop();native.finish(server,None);server=None
        passed('restored-original-reopens-with-retained-completion-and-revocation')
        assert fixture.digest(binary.read_bytes())==qualified['sha256']
    except BaseException as error:
        primary=error
        worker.retain_failure(errors,lambda:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(error)))))
    finally:
        if server is not None:
            worker.retain_failure(errors,lambda:native.finish(server,primary))
        if primary is not None:
            release=root/'watch-worker/release-worker.json'
            if not release.exists():worker.retain_failure(errors,lambda:fixture.publish(release,dict(nonce=nonce,release=True)))
            if supervisor is not None and not reaped:
                worker.retain_failure(errors,lambda:public.reap_cleanup_helper(supervisor))
            worker.retain_failure(errors,lambda:public.cleanup_owned_children(binary,root,out))
        for item in [connection,listener]:
            if item is not None:worker.retain_failure(errors,item.close)
        for fd in [server_fd,supervisor_fd,provider_fd]:
            if fd is not None:worker.retain_failure(errors,lambda fd=fd:os.close(fd))
    if primary is not None or errors:raise BaseExceptionGroup('Public web conductor and cleanup',([primary] if primary else [])+errors)
    result=dict(status='PASS',assertions=checks,server_only=True,paid_provider=False,setup_inert_planning=True,installed=False,global_watchdog=None,binary_sha256=qualified['sha256'])
    fixture.publish(out/'worker-result.json',result)
    print('WEB_WORKER_JOURNEY_PASS',flush=True)
    return result


def held_planner(binary,out):
    """Observe a real permitted planner before stopping its owning HTTP server.

    No proposal is released. HTTP remains responsive while the foreground child
    is held; local ControllerShutdown must TERM/wait and publish the actual failed
    receipt before lease close. This tests server cancellation, separately from OS
    signal CLI recipes; it grants no model, task acceptance or managed recovery.
    """
    out.mkdir(parents=True,mode=0o700,exist_ok=False)
    f,nonce,_=planner.setup(binary,out/'setup')
    fixture.publish(out/'fixture.json',dict(root=str(f.root),work=f.work))
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
    server=peer=None;child_fd=server_fd=None;primary=None;errors=[]
    try:
        listener.bind(str(f.root/'watch-planner.sock'));os.chmod(f.root/'watch-planner.sock',0o600);listener.listen(1)
        server=native.Server(binary,f.root,out,'held-planner',environment=f.environment)
        identity=worker.observation(server.process.pid);server_fd=worker.attach(identity)
        ready,_,_=select.select([listener,server_fd],[],[]);assert listener in ready and server_fd not in ready
        peer,_=listener.accept();child,child_fd=planner.planner_peer(peer,identity)
        assert worker.frame(peer)['nonce']==nonce
        job=rows(f,'SELECT claim_id,permission,phase,child_pid,owner_pid FROM native_planning_runs WHERE work_id=? AND child_pid=?',(f.work,child['pid']))
        assert len(job)==1 and job[0][1:]==(1,'permitted',child['pid'],identity['pid'])
        path='/api/v1/mission?'+urlencode(dict(work=f.work,action='show'))
        assert request(server,'GET',path)['consent_enabled'] is True
        assert not select.select([child_fd],[],[],0)[0]
        server.stop();native.finish(server,None);server=None
        assert select.select([child_fd],[],[],0)[0] and worker.supervision(f)['stopped_at']!=''
        receipts=list((f.root/'.swarm').glob('native-planning-*.receipt'));assert len(receipts)==1
        magic,extent,digest,body=receipts[0].read_bytes().split(b'\n',3)
        assert magic==b'SR1' and int(extent)==len(body) and fixture.digest(body)==digest.decode()
        receipt=json.loads(body)
        assert receipt['signal']==15 and receipt['grant_attempted'] is True
        assert receipt['output_complete'] is False and receipt['failure']=='planning-execution-interrupted'
        work=f.ok(['work','show',f.work])['work'];assert not work.get('tasks')
        assert f.ok(['mission','show',f.work])['consent_enabled'] is True
        fixture.publish(out/'result.json',dict(status='PASS',http_responsive_while_role_held=True,actual_TERM_failed_receipt=True,no_decision_applied=True,local_controller_shutdown=True,installed=False,global_watchdog=None))
        print('PASS web-owned-held-planner-shutdown-retains-actual-TERM-failed-receipt',flush=True)
    except BaseException as error:
        primary=error
        worker.retain_failure(errors,lambda:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(error)))))
    finally:
        if server is not None:worker.retain_failure(errors,lambda:native.finish(server,None))
        for item in [peer,listener]:
            if item is not None:worker.retain_failure(errors,item.close)
        for fd in [child_fd,server_fd]:
            if fd is not None:worker.retain_failure(errors,lambda fd=fd:os.close(fd))
    if primary is not None or errors:raise BaseExceptionGroup('Held web planner and cleanup',([primary] if primary else [])+errors)


def main():
    """Require qualified inputs and scope Linux subreaper only to this owned recipe."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args()
    def cases():
        """Keep ordinary worker journey and held-role shutdown in independent Roots."""
        result=run(args.binary.resolve(),args.profile.resolve(),args.output.resolve())
        held_planner(args.binary.resolve(),args.output.resolve()/'held-planner')
        result['held_planner_local_shutdown_verified']=True
        fixture.publish(args.output.resolve()/'result.json',result)
        print('NATIVE_WEB_CONDUCTOR_PASS',flush=True)
    worker.subreaper_scope(cases,lambda error:None,parent_death.subreaper)


if __name__=='__main__':main()
