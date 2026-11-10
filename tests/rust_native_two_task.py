#!/usr/bin/env python3
"""Exercise two sequential public Rust worker cycles without manual preparation.

Two scoped tasks share an explicitly saved parent profile and workspace. Each
HTTP-owned worker survives its server, completes after nonce release, and retains
its own reservation, FIFO release and completion original. Planning is paused;
reports are fixture markers, not submission, review or acceptance. No paid model,
private SQL write, installation, retry or global watchdog is used.
"""
import argparse
import json
import os
from pathlib import Path
import select
import socket
import traceback
from urllib.parse import urlencode
import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as planning
import rust_mission_watch_worker as worker
import rust_mission_watch_public as public
import rust_native_parent_death as parent_death
import rust_native_http_cli as native
from rust_native_http_mission import request
from rust_mission_report_relay import rows


def archive_markers(workspace, output, nonce):
    """Retain exact owned provider files before allowing the next fixture process.

    Require the expected nonce and report/input digest binding, copy and re-read
    all originals before unlinking only these three known fixture paths. Unknown
    files are retained. This resets test markers, never product state or usage;
    any IO failure propagates with already retained originals left available.
    """
    output.mkdir(mode=0o700)
    names=['model-received.json','handoff.md','release-worker.json']
    raw={name:(workspace/name).read_bytes() for name in names}
    observed=json.loads(raw['model-received.json'])
    assert json.loads(raw['release-worker.json'])==dict(nonce=nonce,release=True)
    assert observed['stdin_sha256'].encode() in raw['handoff.md']
    for name,data in raw.items():
        fixture.publish(output/name,data)
        assert (output/name).read_bytes()==data
    fixture.publish(output/'inventory.json',{name:fixture.digest(data) for name,data in raw.items()})
    for name,data in raw.items():
        assert (workspace/name).read_bytes()==data
        (workspace/name).unlink()


def assert_resources(owned, agents):
    """Bind real task attempts, reservations and FIFO terminal events to each Agent.

    Read public Work/Agents and read-only fixture SQL; never manufacture terminal
    facts. Each supplied distinct task must be blocked with one actual attempt,
    each Agent completed, its admission estimate retained and turn released once.
    Missing, duplicate or inconsistent rows refuse; database errors propagate.
    """
    work=owned.ok(['work','show',owned.work])['work']
    tasks={task['id']:task for task in work['tasks']}
    assert set(tasks)=={'task','task-2'}
    assert len({a['id'] for a in agents})==len(agents)
    assert len({a['task_id'] for a in agents})==len(agents)
    for original in agents:
        agent=owned.ok(['agent','show',original['id']])['agent']
        assert agent['status']=='completed' and agent['task_id']==original['task_id']
        task=tasks[agent['task_id']]
        assert task['status']=='blocked' and len(task['attempts'])==1
        assert task['attempts'][0]['id']==agent['attempt_id']
        assert rows(owned,'SELECT state,amount FROM reservations WHERE agent_id=? AND work_id=?',(agent['id'],owned.work))==[('estimated',1.0)]
        turn=rows(owned,'SELECT seq,state FROM workspace_turns WHERE work_id=? AND task_id=? AND agent_id=?',(owned.work,agent['task_id'],agent['id']))
        assert len(turn)==1 and turn[0][1]=='released'
        assert str(turn[0][0])==agent['native_mission']['workspace_turn_seq']
        assert rows(owned,'SELECT COUNT(*) FROM workspace_turn_events WHERE turn_seq=? AND agent_id=? AND state=?',(turn[0][0],agent['id'],'released'))==[(1,)]
        journal=owned.root/'.swarm'/('native-completion-'+fixture.digest(agent['id'].encode())+'.json')
        value=json.loads(journal.read_bytes())
        assert value['schema_version']==1
        assert all(value['expected'][key]==agent[key] for key in ['id','work_id','task_id','attempt_id','native_mission'])
        assert value['details']['kernel_reaped'] is True
        assert value['details']['workspace_release_confirmed'] is True
        assert value['incomplete_output'] is False
    return work


def run(binary, profile, out):
    """Run two authentic server admissions with public pause/reopen/resume barriers.

    A Linux subreaper caller owns every cleanup child. Saved child task budget2
    and mission slots1 are explicit fixture inputs. A held provider precedes each
    pause; stopping its server must release the lease without killing its grant.
    Reopening paused state must not admit anything until public resume. Failures
    retain original traceback plus independent cleanup errors, with no retry.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    qualified=json.loads(profile.read_bytes())
    assert qualified['no_fixture_feature'] and fixture.digest(binary.read_bytes())==qualified['sha256']
    _,f=planning.flow(binary,out/'planning',before_worker=True,return_fixture=True,extra_worker=True,child_task_budget=2)
    root=f.root;workspace=root/'watch-worker';workspace.mkdir(mode=0o700)
    current=f.ok(['work','show',f.work])['work']
    f.ok(['planning','pause',f.work],dict(schema_version=1,event_id='two-task-planning-pause',expected_revision=current['revision'],reason='Isolate public worker cycles from planner roles'))
    fixture.publish(out/'fixture.json',dict(root=str(root),work=f.work,paid_provider=False,private_sql_writes=False))
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
    server=connection=supervisor=None;fds=[];agents=[];checks=[];errors=[];primary=None;reaped=True;nonce=None
    path=lambda action:'/api/v1/mission?'+urlencode(dict(work=f.work,action=action))
    def passed(name):
        """Emit and retain each actual assertion, not an exit-only success."""
        checks.append(name);print('PASS '+name,flush=True)
    try:
        listener.bind(str(root/'watch-provider.sock'));os.chmod(root/'watch-provider.sock',0o600);listener.listen(1)
        for turn in range(2):
            nonce=fixture.digest((str(root)+':'+str(turn)).encode())
            fixture.publish(root/'codex',worker.provider_program(nonce).encode());(root/'codex').chmod(0o700)
            server=native.Server(binary,root,out,'turn-'+str(turn),environment=f.environment)
            identity=worker.observation(server.process.pid);server_fd=worker.attach(identity);fds.append(server_fd)
            csrf=server.request('/api/v1/native-session')['csrf_token']
            assert len(f.ok(['mission','status-native',f.work])['agents'])==turn
            if turn:
                assert request(server,'GET',path('show'))['paused'] is True
                assert_resources(f,agents)
                passed('two-task-paused-reopen-retains-first-completion')
            current=f.ok(['work','show',f.work])['work']
            body=dict(schema_version=1,event_id='two-task-launch-'+str(turn),expected_revision=current['revision'],reason='Owned sequential worker cycle')
            if turn==0:body.update(slots=1,profile=dict(provider='fixture',role='worker',workspace='watch-worker',level='standard',capture_output=True))
            request(server,'POST',path('start' if turn==0 else 'resume'),body,csrf)
            print('STAGE awaiting-two-task-provider-'+str(turn),flush=True)
            ready,_,_=select.select([listener,server_fd],[],[])
            assert listener in ready and server_fd not in ready,'server exited before provider'
            connection,_=listener.accept()
            supervisor,supervisor_fd,provider,provider_fd,agent_id=public.attach_provider(connection,identity,binary,root)
            fds.extend([supervisor_fd,provider_fd]);reaped=False
            entry=worker.frame(connection);assert entry['nonce']==nonce and entry['pid']==provider['pid']
            agent=worker.wait_running(f,agent_id,server_fd,supervisor_fd)
            assert agent['task_id'] in {'task','task-2'} and agent['task_id'] not in {a['task_id'] for a in agents}
            worker.verify_current_root_binding(agent['native_mission'],root)
            assert len(f.ok(['mission','status-native',f.work])['agents'])==turn+1
            current=f.ok(['work','show',f.work])['work']
            request(server,'POST',path('pause'),dict(schema_version=1,event_id='two-task-pause-'+str(turn),expected_revision=current['revision'],reason='Keep next task pending until explicit resume'),csrf)
            assert request(server,'GET',path('show'))['paused'] is True
            connection.sendall(b'Q');assert worker.frame(connection)['nonce']==nonce
            server.stop();native.finish(server,None);server=None
            assert worker.observation(supervisor['pid'])['ppid']==os.getpid()
            assert worker.supervision(f)['stopped_at']!=''
            fixture.publish(workspace/'release-worker.json',dict(nonce=nonce,release=True))
            select.select([supervisor_fd],[],[])
            waited,status=os.waitpid(supervisor['pid'],0);reaped=True
            assert waited==supervisor['pid'] and os.WIFEXITED(status) and os.WEXITSTATUS(status)==0
            agents.append(agent);work=assert_resources(f,agents)
            if turn==0:
                remaining=[t for t in work['tasks'] if t['id']!=agent['task_id']]
                assert len(remaining)==1 and remaining[0]['status']=='todo' and not remaining[0].get('attempts')
            fixture.publish(out/('turn-'+str(turn)+'-completed.json'),dict(agent=agent,work=work))
            connection.close();connection=None
            archive_markers(workspace,out/('provider-'+str(turn)),nonce)
            passed('two-task-cycle-'+str(turn+1)+'-settles-once-with-durable-original')
        assert len(agents)==2
        assert agents[0]['attempt_id']!=agents[1]['attempt_id']
        assert agents[0]['native_mission']['generation']<agents[1]['native_mission']['generation']
        assert agents[0]['native_mission']['parent_profile_sha256']==agents[1]['native_mission']['parent_profile_sha256']
        assert len(f.ok(['mission','status-native',f.work])['agents'])==2
        assert all(t['status']=='blocked' for t in assert_resources(f,agents)['tasks'])
        server=native.Server(binary,root,out,'final-paused',environment=f.environment)
        assert request(server,'GET',path('show'))['paused'] is True
        assert len(f.ok(['mission','status-native',f.work])['agents'])==2
        status=request(server,'GET',path('status'));fixture.publish(out/'http-final-status.json',status)
        server.stop();native.finish(server,None);server=None
        assert_resources(f,agents)
        passed('two-task-final-reopen-has-no-third-agent-or-fake-acceptance')
        assert fixture.digest(binary.read_bytes())==qualified['sha256']
    except BaseException as error:
        primary=error
        worker.retain_failure(errors,lambda:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(error)))))
    finally:
        if server is not None:worker.retain_failure(errors,lambda:native.finish(server,None))
        if primary is not None:
            release=workspace/'release-worker.json'
            if nonce is not None and not release.exists():worker.retain_failure(errors,lambda:fixture.publish(release,dict(nonce=nonce,release=True)))
            if supervisor is not None and not reaped:worker.retain_failure(errors,lambda:public.reap_cleanup_helper(supervisor))
            worker.retain_failure(errors,lambda:public.cleanup_owned_children(binary,root,out))
        for item in [connection,listener]:
            if item is not None:worker.retain_failure(errors,item.close)
        for fd in fds:worker.retain_failure(errors,lambda fd=fd:os.close(fd))
    if primary is not None or errors:raise BaseExceptionGroup('Two-task journey and cleanup',([primary] if primary else [])+errors)
    result=dict(status='PASS',assertions=checks,agent_ids=[a['id'] for a in agents],task_ids=[a['task_id'] for a in agents],binary_sha256=qualified['sha256'],accepted=False,installed=False,global_watchdog=None,paid_provider=False)
    fixture.publish(out/'result.json',result);print('NATIVE_TWO_TASK_PASS',flush=True)
    return result


def main():
    """Run against explicit qualified binary/profile in a fresh owned subreaper Root."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args()
    worker.subreaper_scope(lambda:run(args.binary.resolve(),args.profile.resolve(),args.output.resolve()),lambda error:None,parent_death.subreaper)


if __name__=='__main__':main()
