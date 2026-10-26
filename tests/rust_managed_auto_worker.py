#!/usr/bin/env python3
"""Exercise two managed automatic workers from the real native web conductor.

Two scoped tasks have distinct attributed copies from the committed source. Each
HTTP-owned worker survives its server, completes after nonce release, and retains
its own reservation, FIFO release and completion original. Planning is paused;
reports are fixture markers, submitted only through immutable retention and the
owner inbox, without review or acceptance. No paid model, private SQL write,
installation, retry or global watchdog is used. The result commit is retained;
candidate publication and acceptance remain separate.
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
import rust_managed_planning as planning
import rust_managed_repository_bootstrap as repository
import rust_mission_watch_worker as worker
import rust_mission_watch_public as public
import rust_native_parent_death as parent_death
import rust_native_http_cli as native
from rust_native_http_mission import request
from rust_mission_report_relay import rows


def assert_resources(owned, agents, *, expected_task_status='blocked'):
    """Bind real task attempts, reservations and FIFO terminal events to each Agent.

    Read public Work/Agents and read-only fixture SQL; never manufacture terminal
    facts. Each distinct task must have its explicit expected phase status
    (blocked after completion, submitted after handoff) with one actual attempt,
    each Agent completed, its admission estimate retained and turn released once.
    An explicitly planned dependent task may remain todo with no attempt; its
    dependency must be exactly task and its admission is outside this fixture.
    Missing, duplicate or inconsistent rows refuse; database errors propagate.
    """
    work=owned.ok(['work','show',owned.work])['work']
    tasks={task['id']:task for task in work['tasks']}
    assert set(tasks) in ({'task','task-2'},{'task','task-2','dependent'})
    if 'dependent' in tasks:
        assert tasks['dependent']['status']=='todo' and not tasks['dependent'].get('attempts') and tasks['dependent']['depends']==['task']
    assert len({a['id'] for a in agents})==len(agents)
    assert len({a['task_id'] for a in agents})==len(agents)
    for original in agents:
        agent=owned.ok(['agent','show',original['id']])['agent']
        assert agent['status']=='completed' and agent['task_id']==original['task_id']
        task=tasks[agent['task_id']]
        assert task['status']==expected_task_status and len(task['attempts'])==1
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


def report_continuation(owned, agents, output, *, publication_refusal=False, git_callbacks=False):
    """Observe immutable result handoff through the actual mission watch after completion.

    Each attributed copy receives an actual task report. Only public mission
    resume/pause/watch controls change fixture state; original attempts, reports,
    spending and task decisions remain retained. No provider/retry is expected.
    Optional filesystem refusal retains the Git ref before recovering with the
    worker clone parked. It is not an ENOSPC or crash-durability assertion.
    EOF, submission, admission or cleanup errors fail with original channels.
    """
    output.mkdir(mode=0o700)
    for agent in agents:
        docs=Path(agent['workspace'])/'docs';docs.mkdir(exist_ok=True)
        fixture.publish(docs/(agent['task_id']+'.md'),b'Owned completed managed report, not integrated or accepted.\n')
    callback_marker=None
    if git_callbacks:
        from rust_managed_result_recovery import install_git_callback
        callback_marker=install_git_callback(owned,agents,output/'git-callback')
    if publication_refusal:
        from rust_managed_result_recovery import publication_recovery
        publication_recovery(owned,agents,output/'publication')
    else:
        from rust_managed_result_recovery import watch_reports
        seen=watch_reports(owned,{a['id'] for a in agents},output/'normal')
        assert seen=={a['id'] for a in agents}
    work=assert_resources(owned,agents,expected_task_status='submitted')
    assert all(t['status']=='submitted' for t in work['tasks'] if t['id'] in {'task','task-2'})
    inbox=work['planning']['inbox']
    for agent in agents:
        handoffs=[x for x in inbox if x['kind']=='handoff' and x['task']==agent['task_id']]
        assert len(handoffs)==1,handoffs
        assert handoffs[0]['scope']==next(t for t in work['tasks'] if t['id']==agent['task_id'])['scope_id']
        retained=Path(work['planning']['repository']['storage'])/'results'/agent['id']
        manifest=json.loads((retained/'result.json').read_bytes())
        report=(retained/'report.md').read_bytes()
        assert handoffs[0]['handoff']['sha256']==fixture.digest(report)==manifest['report_sha256']
        assert not next(t for t in work['tasks'] if t['id']==agent['task_id']).get('independent_review')
    print('PASS managed-auto-immutable-results-reach-owner-once-without-acceptance',flush=True)
    if callback_marker is not None:
        assert not callback_marker.exists(),'repository-configured callback ran during engine retention'
        print('PASS managed-auto-retains-raw-source-without-repository-callback',flush=True)


def run(binary, profile, out, *, project_subdir="", publication_refusal=False, git_callbacks=False, controls=None,source_entries=None,pending_dependent=False):
    """Run two authentic server admissions with public pause/reopen/resume barriers.

    A Linux subreaper caller owns every cleanup child. Saved child task budget2
    and mission slots1 are explicit fixture inputs. A held provider precedes each
    pause; stopping its server must release the lease without killing its grant.
    Optional controls is passed to actual planning authorization; None keeps the
    historical fixture contract. Optional source_entries is committed by the
    planning fixture before workers, never injected into an existing candidate
    or mission database. Reopening paused state must not admit anything until
    public resume. Failures
    retain original traceback plus independent cleanup errors, with no retry.
    pending_dependent preplans a third dependent task, never launched here; the
    explicit fixture task budget grows to three without changing product policy.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    qualified=json.loads(profile.read_bytes())
    assert qualified['no_fixture_feature'] and fixture.digest(binary.read_bytes())==qualified['sha256']
    prepared,f=planning.run(binary,profile,out/'planning',return_fixture=True,project_subdir=project_subdir,controls=controls,source_entries=source_entries,pending_dependent=pending_dependent)
    f.work=prepared['planned_work'];root=f.root;workspace=None
    current=f.ok(['work','show',f.work])['work']
    f.ok(['budget','apply',f.work],dict(schema_version=1,event_id='managed-auto-budget',expected_revision=current['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=1,estimate_source='Owned managed automatic fixture',reference_date='2026-10-24')))
    source=repository.inventory(root/'source')
    storage=Path(current['planning']['repository']['storage'])
    assert not (storage/'copies').exists()
    current=f.ok(['work','show',f.work])['work']
    f.ok(['mission','pause',f.work],dict(schema_version=1,event_id='managed-auto-initial-pause',expected_revision=current['revision'],reason='Observe paused startup before first managed departure'))
    current=f.ok(['work','show',f.work])['work']
    f.ok(['planning','pause',f.work],dict(schema_version=1,event_id='managed-auto-planning-pause',expected_revision=current['revision'],reason='Isolate public worker cycles from planner roles'))
    # A JSON origin label cannot manufacture the engine's borrowed lease fence.
    w=f.ok(['work','show',f.work])['work'];before=f.snapshot()
    code,wire=f.call(['agent','prepare',f.work],dict(schema_version=1,event_id='forged-managed-origin',expected_revision=w['revision'],task_id='task',provider='fixture',capture_output=True,origin='conducteur',conductor_id='forged-conductor'))
    assert code!=0 and 'agent-launch-request-branch-unported' in json.dumps(wire)
    assert f.snapshot()==before and not (storage/'copies').exists()
    print('PASS managed-auto-public-origin-label-refuses-without-authority',flush=True)
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
            fixture.publish(root/'codex',worker.provider_program(nonce).replace("control.connect('../watch-provider.sock')","control.connect("+repr(str(root/'watch-provider.sock'))+")").encode());(root/'codex').chmod(0o700)
            server=native.Server(binary,root,out,'turn-'+str(turn),environment=f.environment)
            identity=worker.observation(server.process.pid);server_fd=worker.attach(identity);fds.append(server_fd)
            csrf=server.request('/api/v1/native-session')['csrf_token']
            assert len(f.ok(['mission','status-native',f.work])['agents'])==turn
            if turn:
                assert request(server,'GET',path('show'))['paused'] is True
                assert_resources(f,agents)
                passed('managed-auto-paused-reopen-retains-first-completion')
            current=f.ok(['work','show',f.work])['work']
            body=dict(schema_version=1,event_id='managed-auto-launch-'+str(turn),expected_revision=current['revision'],reason='Owned sequential worker cycle')
            if turn==0:assert not (storage/'copies').exists()
            request(server,'POST',path('resume'),body,csrf)
            print('STAGE awaiting-managed-auto-provider-'+str(turn),flush=True)
            ready,_,_=select.select([listener,server_fd],[],[])
            assert listener in ready and server_fd not in ready,'server exited before provider'
            connection,_=listener.accept()
            supervisor,supervisor_fd,provider,provider_fd,agent_id=public.attach_provider(connection,identity,binary,root)
            fds.extend([supervisor_fd,provider_fd]);reaped=False
            entry=worker.frame(connection);assert entry['nonce']==nonce and entry['pid']==provider['pid']
            agent=worker.wait_running(f,agent_id,server_fd,supervisor_fd)
            assert agent['task_id'] in {'task','task-2'} and agent['task_id'] not in {a['task_id'] for a in agents}
            workspace=Path(agent['workspace'])
            copy=storage/'copies'/(agent['task_id']+'-1')
            assert workspace==copy/project_subdir
            assert workspace.is_dir() and (workspace/'tracked.txt').read_bytes()==b'Committed planning input\n'
            assert rows(f,'SELECT work_id,task_id,path,state FROM managed_attempts WHERE agent_id=?',(agent['id'],))==[(f.work,agent['task_id'],str(copy),'ready')]
            assert repository.inventory(root/'source')==source
            if turn==0:
                other='task-2' if agent['task_id']=='task' else 'task'
                waiting=rows(f,'SELECT workspace,state FROM workspace_turns WHERE work_id=? AND task_id=?',(f.work,other))
                assert waiting==[(str(storage/'copies'/(other+'-1')/project_subdir),'waiting')],waiting
                assert not (storage/'copies'/(other+'-1')).exists()
                passed('managed-auto-waiting-FIFO-does-not-create-second-copy')
            worker.verify_current_root_binding(agent['native_mission'],root)
            assert len(f.ok(['mission','status-native',f.work])['agents'])==turn+1
            current=f.ok(['work','show',f.work])['work']
            request(server,'POST',path('pause'),dict(schema_version=1,event_id='managed-auto-pause-'+str(turn),expected_revision=current['revision'],reason='Keep next task pending until explicit resume'),csrf)
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
                remaining=[t for t in work['tasks'] if t['id'] in {'task','task-2'} and t['id']!=agent['task_id']]
                assert len(remaining)==1 and remaining[0]['status']=='todo' and not remaining[0].get('attempts')
            fixture.publish(out/('turn-'+str(turn)+'-completed.json'),dict(agent=agent,work=work))
            connection.close();connection=None
            # Each managed task owns a different copy; keep all producer originals.
            assert (workspace/'handoff.md').is_file() and (workspace/'release-worker.json').is_file()
            if turn==0:
                manifest=storage/('native-worker-'+agent['id']+'.json');original=manifest.read_bytes()
                before=f.snapshot();invalid=json.loads(original);invalid['untrusted_extra']=True
                fixture.publish(manifest,invalid)
                try:
                    code,wire=f.call(['mission','status',f.work])
                    assert code!=0 and 'managed-worker-proof-invalid' in json.dumps(wire)
                    assert f.snapshot()==before
                finally:fixture.publish(manifest,original)
                passed('managed-auto-original-proof-refusal-preserves-terminal-state')
            status=f.ok(['mission','status',f.work]);item=next(t for t in status['tasks'] if t['id']==agent['task_id'])
            assert item['attempt']['agent_id']==agent['id'] and item['result']['state']=='completed_unproven'
            assert status['validated']==0 and repository.inventory(root/'source')==source
            passed('managed-auto-cycle-'+str(turn+1)+'-settles-once-with-durable-original')
        assert len(agents)==2
        assert agents[0]['attempt_id']!=agents[1]['attempt_id']
        assert agents[0]['native_mission']['generation']<agents[1]['native_mission']['generation']
        assert agents[0]['native_mission']['parent_profile_sha256']==agents[1]['native_mission']['parent_profile_sha256']
        assert len(f.ok(['mission','status-native',f.work])['agents'])==2
        assert all(t['status']=='blocked' for t in assert_resources(f,agents)['tasks'] if t['id'] in {'task','task-2'})
        server=native.Server(binary,root,out,'final-paused',environment=f.environment)
        assert request(server,'GET',path('show'))['paused'] is True
        assert len(f.ok(['mission','status-native',f.work])['agents'])==2
        status=request(server,'GET',path('status'));fixture.publish(out/'http-final-status.json',status)
        server.stop();native.finish(server,None);server=None
        assert_resources(f,agents)
        passed('managed-auto-final-reopen-has-no-third-agent-or-fake-acceptance')
        report_continuation(f,agents,out/'report-continuation',publication_refusal=publication_refusal,git_callbacks=git_callbacks)
        assert fixture.digest(binary.read_bytes())==qualified['sha256']
    except BaseException as error:
        primary=error
        worker.retain_failure(errors,lambda:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(error)))))
    finally:
        if server is not None:worker.retain_failure(errors,lambda:native.finish(server,None))
        if primary is not None:
            release=workspace/'release-worker.json' if workspace is not None else None
            if nonce is not None and release is not None and not release.exists():worker.retain_failure(errors,lambda:fixture.publish(release,dict(nonce=nonce,release=True)))
            if supervisor is not None and not reaped:worker.retain_failure(errors,lambda:public.reap_cleanup_helper(supervisor))
            worker.retain_failure(errors,lambda:public.cleanup_owned_children(binary,root,out))
        for item in [connection,listener]:
            if item is not None:worker.retain_failure(errors,item.close)
        for fd in fds:worker.retain_failure(errors,lambda fd=fd:os.close(fd))
    if primary is not None or errors:raise BaseExceptionGroup('Managed automatic journey and cleanup',([primary] if primary else [])+errors)
    result=dict(status='PASS',assertions=checks,agent_ids=[a['id'] for a in agents],task_ids=[a['task_id'] for a in agents],binary_sha256=qualified['sha256'],accepted=False,installed=False,global_watchdog=None,paid_provider=False)
    fixture.publish(out/'result.json',result);print('NATIVE_MANAGED_AUTO_PASS',flush=True)
    return result


def main():
    """Run against explicit qualified binary/profile in a fresh owned subreaper Root."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--subproject',action='store_true')
    parser.add_argument('--publication-refusal',action='store_true')
    parser.add_argument('--git-callbacks',action='store_true')
    args=parser.parse_args()
    worker.subreaper_scope(lambda:run(args.binary.resolve(),args.profile.resolve(),args.output.resolve(),project_subdir='app' if args.subproject else '',publication_refusal=args.publication_refusal,git_callbacks=args.git_callbacks),lambda error:None,parent_death.subreaper)


if __name__=='__main__':main()
