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
import traceback
from urllib.parse import urlencode
import rust_agent_model_route as fixture
import rust_managed_file_provider as file_provider
import rust_managed_planning as planning
import rust_managed_repository_bootstrap as repository
import rust_mission_watch_worker as worker
import rust_mission_watch_public as public
import rust_native_parent_death as parent_death
import rust_native_http_cli as native
from rust_native_http_mission import request
from rust_mission_report_relay import rows
from rust_mission_watch_worker import check as _kernel_check


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


class ManagedAutoTerminalRefusal(RuntimeError):
    """Secret-safe structured refusal for a newly launched terminal managed agent.

    Raised when a public observation proves a newly launched managed agent is
    terminal (failed/completed per the product lifecycle contract; queued,
    starting, running and stopping are active membership and never refuse). The
    rendered message stays minimal — agent/task
    identifier, status, exit code and public failure code only — so a secret
    planted in the public agent's prompt, context, args or capture never reaches
    str, repr or a rendered traceback. The owned ``payload`` attribute keeps the
    complete original public payload (identity, exit code, native execution
    result, structured failure with the real cause such as EPERM on
    socket(AF_UNIX), and the full public agent) for exact programmatic access:
    nothing is filtered out, nothing is fabricated, and no confinement conclusion
    is implied. Renderers must use the message, never the payload.
    """

    def __init__(self,payload):
        self.payload=payload
        failure=payload.get('failure')
        code=failure.get('code') if isinstance(failure,dict) else None
        super().__init__('managed-auto-provider-terminal-refusal: '+json.dumps(dict(
            agent_id=payload.get('public_agent',{}).get('id') if isinstance(payload.get('public_agent'),dict) else payload.get('public_agent'),
            task_id=payload.get('public_agent',{}).get('task_id') if isinstance(payload.get('public_agent'),dict) else None,
            status=payload.get('public_agent',{}).get('status') if isinstance(payload.get('public_agent'),dict) else None,
            exit_code=payload.get('public_agent',{}).get('exit_code') if isinstance(payload.get('public_agent'),dict) else None,
            failure_code=code,
        ),sort_keys=True,default=str))


class ObservationRefusal(RuntimeError):
    """Secret-safe typed refusal for an unknown or malformed public payload.

    Bounded to the managed-auto observation paths: an agent show payload
    without a status, or with a lifecycle status outside the product contract
    (``agent_contract.rs`` ``is_active`` = queued|starting|running|stopping;
    real terminals are failed/completed only). Unlike
    :class:`ManagedAutoTerminalRefusal` it never classifies the payload
    terminal and renders NO payload field at all — the message is a static
    safe reason, since even the status value itself could carry a secret —
    while the owned ``payload`` attribute keeps the complete original public
    payload for exact programmatic access: nothing filtered, nothing
    fabricated. Renderers must use the message, never the payload. The
    refusal conserves the observed phase cause: never an empty success, never
    a fabricated terminal classification, never a payload dump into message,
    repr or traceback.
    """

    def __init__(self,payload,reason):
        self.payload=payload
        self.reason=reason
        super().__init__('managed-auto-observation-refusal: '+reason)


def _observe_new_agents(owned,work,previous_agents):
    """One public observation round of the newly launched managed agents.

    Reads public mission status-native, then public agent show for every newly
    launched agent: an identifier absent from previous turns and — when the entry
    records a generation — a generation strictly greater than the previous
    maximum, so completed agents of earlier turns never trigger a false refusal.
    Only real public payloads are used; a status-native entry without an id is
    refused with its payload, never guessed. Returns None when no newly launched
    agent is terminal, or a pending BUSY refusal (structured root-database-identity
    Prepare / sqlite-code:5) carrying the public reason; any other public read
    error raises with that reason. Lifecycle membership follows the product
    contract (agent_contract.rs ``is_active`` = queued|starting|running|
    stopping): queued, starting, running and stopping keep observing — a queued
    agent (PIDs 0, empty stamps, null exit code, the exact state validated by
    agent_execution_preflight.rs) is NEVER terminal, correcting the real
    2026-11-05 false refusal. A new agent whose public status is a real
    terminal one (failed/completed) raises a structured terminal refusal
    carrying its full public agent payload — exit code and native execution
    result as recorded by the product, public data only, never environment or
    session secrets — with the real task/status identity, not a bare 'server
    exited' summary, raised as the typed :class:`ManagedAutoTerminalRefusal`
    whose message is minimal and secret-safe while its owned ``payload``
    attribute keeps the complete original public payload exactly. Any other
    (unknown or malformed) status — including an agent show payload without a
    status at all — conserves the observed phase cause as the typed
    :class:`ObservationRefusal`: a static safe reason message rendering NO
    payload field (the status value itself could carry a secret) while the
    owned ``payload`` attribute keeps the complete original public payload —
    never a consumed success, never a fabricated terminal classification, and
    never a payload dump into message, repr or traceback.
    """
    code,value=owned.call(['mission','status-native',work])
    if code:
        failure=value.get('failure',{}) if isinstance(value,dict) else {}
        labels=set(failure.get('causes',[]))
        if failure.get('code')=='root-database-identity' and labels=={'root-database-identity','database-identity-operation:Prepare','sqlite-code:5'}:
            return RuntimeError('public status-native read refused (BUSY): '+json.dumps(failure,sort_keys=True))
        raise RuntimeError('public status-native read refused: '+json.dumps(failure,sort_keys=True))
    previous_ids={a['id'] for a in previous_agents}
    baseline=max((a['native_mission']['generation'] for a in previous_agents if isinstance(a.get('native_mission'),dict) and 'generation' in a['native_mission']),default=-1)
    for entry in value['agents']:
        if not isinstance(entry,dict) or 'id' not in entry:
            raise RuntimeError('status-native agent entry without public id: '+json.dumps(entry,sort_keys=True,default=str))
        if entry['id'] in previous_ids:continue
        mission=entry.get('native_mission')
        if isinstance(mission,dict) and isinstance(mission.get('generation'),int) and mission['generation']<=baseline:continue
        code,detail=owned.call(['agent','show',entry['id']])
        if code:
            failure=detail.get('failure',{}) if isinstance(detail,dict) else {}
            labels=set(failure.get('causes',[]))
            if failure.get('code')=='root-database-identity' and labels=={'root-database-identity','database-identity-operation:Prepare','sqlite-code:5'}:
                return RuntimeError('public agent show read refused (BUSY): '+json.dumps(failure,sort_keys=True))
            raise RuntimeError('public agent show read refused: '+json.dumps(failure,sort_keys=True))
        agent=detail['agent']
        if not isinstance(agent,dict) or 'status' not in agent:
            raise ObservationRefusal(detail,'public agent show payload without status')
        if agent['status'] in ('queued','starting','running','stopping'):continue
        if agent['status'] in ('failed','completed'):
            raise ManagedAutoTerminalRefusal(dict(agent_id=agent.get('id'),task_id=agent.get('task_id'),status=agent['status'],exit_code=agent.get('exit_code'),native_execution_result=agent.get('native_execution_result'),failure=agent.get('failure'),public_agent=agent))
        raise ObservationRefusal(agent,'public agent show payload with unknown lifecycle status')
    return None


def _close_watch_fd(fd):
    """Close the helper's own inotify watch descriptor (production: os.close).

    Module-level so the targeted regression can inject a refusing close with a
    valid witness (a genuinely watched descriptor) before the guarded call; no
    product behavior depends on this seam. Raises exactly like os.close and is
    called on every exit path of await_provider_turn.
    """
    os.close(fd)


def _attach_canonical_agent(owned,work,previous_agents,server_pid,server_fd,binary,root):
    """Wait for the newly launched canonical agent and attach its kernel identity.

    Replaces the former AF_UNIX bootstrap (listener + await_provider_turn +
    public.attach_provider) which can never complete now that the file-protocol
    provider connects no socket. Public reads only (``mission status-native``
    then ``agent show``) under an inotify subscription on root/'.swarm' taken
    BEFORE the first read; re-reads happen only after real database activity or
    the conductor pidfd becoming readable ('server exited before provider') —
    no timeout, temporal polling, retry or arbitrary cap. Previous-turn agent
    identifiers are ignored, and a generation no greater than the previous
    maximum is ignored, so completed agents of earlier turns never trigger a
    false refusal. A structured SQLite BUSY (root-database-identity Prepare /
    sqlite-code:5) is kept as a pending refusal carrying its public reason and
    is cleared only by a fresh read after real activity — never an empty
    success. A newly launched agent that is terminal raises
    :class:`ManagedAutoTerminalRefusal` (minimal secret-safe message, owned
    ``payload`` attribute keeps the exact public agent payload). When an agent
    is recorded ``running`` with usable child PIDs and stamps, its kernel
    identity is fully validated and attached by :func:`_attach_agent_kernel`;
    the return (agent, supervisor, supervisor_fd, child, child_fd) happens only
    after every validation passes AND after the controlled watch closure with
    its causes verified: a refusing watch close after a successful attachment
    never becomes a lost-error success — both taken pidfds are then explicitly
    released and their close errors conserved as secondary causes of the same
    group, so no descriptor is abandoned and no success is returned past a
    refusal (the return-inside-try fault already corrected in
    await_provider_turn is structurally impossible here). The caller owns the
    returned pidfds and closes them; a partial refusal never abandons a taken
    pidfd. A pending BUSY refusal is cleared by the next fresh valid public
    read (only a newer BUSY replaces it), never kept after recovery. No
    SO_PEERCRED is claimed anywhere: the public payload and every report file
    only ever repeat an identity already established from real /proc facts.
    """
    fd=worker.watch_directory(owned.root/'.swarm',modifications=True)
    errors=[];pending=None;result=None
    try:
        while result is None:
            candidate=_running_new_agent(owned,work,previous_agents)
            if isinstance(candidate,RuntimeError):pending=candidate
            else:
                pending=None
                if candidate is not None:
                    result=(candidate,)+_attach_agent_kernel(candidate,binary,root,server_pid)
                    continue
            ready,_,_=select.select([fd,server_fd],[],[])
            if server_fd in ready:raise RuntimeError('server exited before provider (conductor terminal readable)')
            if fd in ready:os.read(fd,65536)
    except BaseException as error:
        if pending is not None and pending is not error:errors.append(pending)
        errors.append(error)
    finally:
        try:_close_watch_fd(fd)
        except BaseException as close_error:
            errors.append(close_error)
            if result is not None:
                for descriptor in (result[2],result[4]):
                    try:os.close(descriptor)
                    except BaseException as pidfd_error:errors.append(pidfd_error)
                result=None
    if errors:raise BaseExceptionGroup('managed-auto canonical agent observation/close',errors)
    return result


def _running_new_agent(owned,work,previous_agents):
    """Return one attachment candidate for a newly launched agent, or None.

    Same identity rules as :func:`_observe_new_agents` for ignoring previous
    turns and the same product lifecycle membership (agent_contract.rs
    ``is_active`` = queued|starting|running|stopping): ``queued`` (PIDs 0,
    empty stamps, null exit code — the exact state
    agent_execution_preflight.rs validates, and the real 2026-11-05 false
    refusal), ``starting`` and ``stopping`` keep observing and return None,
    never a terminal classification. Returns the full public agent payload of
    a ``running`` entry only when ``supervisor_pid``/``child_pid`` are positive
    integers AND ``supervisor_identity``/``child_identity`` are non-empty
    decimal stamp strings — otherwise None and the causal activity continues.
    A real terminal status (failed/completed) raises the typed terminal
    refusal; any other (unknown or malformed) status — including a payload
    without a status at all — conserves the observed phase cause as the typed
    :class:`ObservationRefusal` (static safe reason message, complete public
    payload kept in the owned ``payload`` attribute, never a payload dump into
    message, repr or traceback). A structured
    BUSY is returned as a pending RuntimeError carrying its public reason —
    the caller keeps it pending and never mistakes it for an agent — and is
    cleared only by a fresh read after real database activity.
    """
    code,value=owned.call(['mission','status-native',work])
    if code:
        failure=value.get('failure',{}) if isinstance(value,dict) else {}
        labels=set(failure.get('causes',[]))
        if failure.get('code')=='root-database-identity' and labels=={'root-database-identity','database-identity-operation:Prepare','sqlite-code:5'}:
            return RuntimeError('public status-native read refused (BUSY): '+json.dumps(failure,sort_keys=True))
        raise RuntimeError('public status-native read refused: '+json.dumps(failure,sort_keys=True))
    previous_ids={a['id'] for a in previous_agents}
    baseline=max((a['native_mission']['generation'] for a in previous_agents if isinstance(a.get('native_mission'),dict) and 'generation' in a['native_mission']),default=-1)
    for entry in value['agents']:
        if not isinstance(entry,dict) or 'id' not in entry:
            raise RuntimeError('status-native agent entry without public id: '+json.dumps(entry,sort_keys=True,default=str))
        if entry['id'] in previous_ids:continue
        mission=entry.get('native_mission')
        if isinstance(mission,dict) and isinstance(mission.get('generation'),int) and mission['generation']<=baseline:continue
        code,detail=owned.call(['agent','show',entry['id']])
        if code:
            failure=detail.get('failure',{}) if isinstance(detail,dict) else {}
            labels=set(failure.get('causes',[]))
            if failure.get('code')=='root-database-identity' and labels=={'root-database-identity','database-identity-operation:Prepare','sqlite-code:5'}:
                return RuntimeError('public agent show read refused (BUSY): '+json.dumps(failure,sort_keys=True))
            raise RuntimeError('public agent show read refused: '+json.dumps(failure,sort_keys=True))
        agent=detail['agent']
        if not isinstance(agent,dict) or 'status' not in agent:
            raise ObservationRefusal(detail,'public agent show payload without status')
        if agent['status'] in ('queued','starting','stopping'):continue
        if agent['status']=='running':
            pids=(agent.get('supervisor_pid'),agent.get('child_pid'))
            stamps=(agent.get('supervisor_identity'),agent.get('child_identity'))
            if all(isinstance(value,int) and not isinstance(value,bool) and value>0 for value in pids) and all(isinstance(value,str) and value.isdecimal() for value in stamps):return agent
            continue
        if agent['status'] in ('failed','completed'):
            raise ManagedAutoTerminalRefusal(dict(agent_id=agent.get('id'),task_id=agent.get('task_id'),status=agent['status'],exit_code=agent.get('exit_code'),native_execution_result=agent.get('native_execution_result'),failure=agent.get('failure'),public_agent=agent))
        raise ObservationRefusal(agent,'public agent show payload with unknown lifecycle status')
    return None


def _attach_agent_kernel(agent,binary,root,server_pid):
    """Attach and validate the kernel identity of one recorded running agent.

    The public agent payload is only a report; every right comes from real
    /proc facts. ``agent['supervisor_pid']`` must be this owned server's
    helper child and ``agent['child_pid']`` its provider child: both PIDs are
    observed by :func:`rust_mission_watch_worker.observation` (UID and live
    state checked), the parent chain child→supervisor→server is verified,
    ``supervisor_identity``/``child_identity`` start-tick strings are compared
    exactly to the observed stamps, the supervisor argv must be EXACTLY
    ``[binary, _mission_worker_entry, agent_id, --root, root, --json]`` with
    the public agent id, and the real ``/proc/<supervisor>/exe`` hash must
    match the provided binary image. Both pidfds are taken by
    :func:`rust_mission_watch_worker.attach` (stamp checked before AND after
    pidfd_open); on any refusal after the first attach the first descriptor is
    closed with the primary cause conserved, never abandoned. Returns
    (supervisor, supervisor_fd, child, child_fd); no file ever authorizes a
    PID and no SO_PEERCRED is claimed on any channel here.
    """
    supervisor=worker.observation(agent['supervisor_pid'])
    child=worker.observation(agent['child_pid'])
    _kernel_check(supervisor['ppid']==server_pid,'public supervisor is not the owned server helper child')
    _kernel_check(child['ppid']==supervisor['pid'],'public provider parent is not the supervisor')
    _kernel_check(str(supervisor['stamp'])==str(agent['supervisor_identity']),'public supervisor start stamp mismatch')
    _kernel_check(str(child['stamp'])==str(agent['child_identity']),'public child start stamp mismatch')
    args=Path('/proc',str(supervisor['pid']),'cmdline').read_bytes().split(b'\0')[:-1]
    _kernel_check(len(args)==6 and args[:2]==[os.fsencode(binary),b'_mission_worker_entry']
          and args[2]==os.fsencode(agent['id'])
          and args[3:]==[b'--root',os.fsencode(root),b'--json'],
          'public supervisor argv differs from helper/Root')
    _kernel_check(fixture.digest(Path('/proc',str(supervisor['pid']),'exe').read_bytes())
          ==fixture.digest(binary.read_bytes()),'public supervisor binary mismatch')
    supervisor_fd=worker.attach(supervisor)
    try:
        child_fd=worker.attach(child)
    except BaseException as primary:
        worker.close_descriptor(supervisor_fd,primary)
        raise
    return supervisor,supervisor_fd,child,child_fd


def await_provider_turn(owned,work,listener,server_fd,previous_agents):
    """Wait causally for the provider control connection or a real terminal refusal.

    Replaces the former bare select on [listener, server_fd], which waited forever
    when both newly launched providers had already failed before connecting (real
    incident 2026-11-05: the Biba kernel refuses socket(AF_UNIX) by contract, both
    canonical agents reached terminal failed with exit 1 and kernel_reaped, and no
    conductor ever connected). Contract, reused from rust_mission_watch_worker:
    the inotify subscription on root/'.swarm' (modifications=True) is taken BEFORE
    the first public read; public state is read once up front, then re-read only
    after observed database activity, a readable listener or conductor exit — no
    global timeout, temporal polling, cap, retry or text-based wait. A structured
    SQLite BUSY is recorded as a refusal with its public reason and is cleared
    only by actual database activity, never by an empty success or a pending
    connection. Termination and error have priority over consuming a connection:
    a newly launched agent no longer starting/running raises the structured
    terminal refusal before any pending provider connection is accepted — and
    after real watch activity the public state is re-observed BEFORE accepting
    a simultaneously ready connection, so a terminal surfaced since the earlier
    probe is still refused and the pending connection is left unconsumed. The
    conductor socket becoming readable refuses immediately ('server exited
    before provider'). The watch descriptor is always closed in a controlled
    finally: a close refusal is appended to the wait's causes, and the success
    return happens only AFTER that closure and the cause check, so a close
    error is never dropped by the return path — and no accepted socket is ever
    returned or left dangling when the closure refuses (an accepted connection
    from that failed attempt is closed with the error conserved in the group).
    Our own close target is injectable via the module-level ``_close_watch_fd``
    for the targeted regression guard; production stays os.close. Secondary
    causes are preserved alongside the primary; failure.json writing and child
    cleanup stay on the caller's existing public path. Limits: the
    probe/accept window is causal, not atomic — a provider failing inside it is caught by the downstream
    handshake and wait_running refusals; with no event at all (silent conductor,
    no connection) the causal wait continues, since no global timeout is allowed.
    """
    fd=worker.watch_directory(owned.root/'.swarm',modifications=True)
    errors=[];pending=None;connection=None
    try:
        while True:
            observed=_observe_new_agents(owned,work,previous_agents)
            if observed is None:pending=None
            else:
                pending=observed
                ready,_,_=select.select([fd,server_fd],[],[])
                if server_fd in ready:raise RuntimeError('server exited before provider (conductor socket readable)')
                if fd in ready:os.read(fd,65536)
                continue
            ready,_,_=select.select([fd,listener,server_fd],[],[])
            if server_fd in ready:raise RuntimeError('server exited before provider (conductor socket readable)')
            if fd in ready:os.read(fd,65536)
            if listener in ready:
                if fd in ready:
                    # Guard: watch activity and a ready connection arrived in the
                    # same select. Termination keeps priority over consuming the
                    # connection: re-observe publicly first; on a refusal the
                    # pending connection stays unconsumed in the backlog and is
                    # rechecked only after the next real activity.
                    observed=_observe_new_agents(owned,work,previous_agents)
                    if observed is not None:
                        pending=observed
                        continue
                connection,_=listener.accept()
                break
    except BaseException as error:
        if pending is not None and pending is not error:errors.append(pending)
        errors.append(error)
        connection=None
    finally:
        if connection is not None:
            # Owned typed handback: the accepted socket must never leak while a
            # watch-close refusal still has to stop the wait. If closing the
            # watch fails, the connection is closed here and both causes reach
            # the caller as a group instead of a lost-error success return.
            try:_close_watch_fd(fd)
            except BaseException as error:
                errors.append(error)
                try:connection.close()
                except BaseException as close_error:errors.append(close_error)
                connection=None
        else:
            try:_close_watch_fd(fd)
            except BaseException as error:errors.append(error)
    if errors:raise BaseExceptionGroup('managed-auto provider terminal observation/close',errors)
    return connection


def run(binary, profile, out, *, project_subdir="", publication_refusal=False, git_callbacks=False, controls=None,source_entries=None,pending_dependent=False,disjoint_results=False):
    """Run two authentic server admissions with public pause/reopen/resume barriers.

    A Linux subreaper caller owns every cleanup child. Saved child task budget2
    and mission slots1 are explicit fixture inputs. A held provider precedes each
    pause; stopping its server must release the lease without killing its grant.
    Optional controls is passed to actual planning authorization; None keeps the
    historical fixture contract. Optional source_entries is committed by the
    planning fixture before workers, never injected into an existing candidate
    or mission database. Reopening paused state must not admit anything until
    public resume. The provider control channel is the verified file rendezvous
    of rust_managed_file_provider and the bootstrap is socket-free: the
    conductor attaches the newly launched canonical agent through real public
    reads and /proc validation (:func:`_attach_canonical_agent`), then the
    generated inert program publishes ``ready-<turn>.json`` in its workspace,
    the conductor side answers with one identified probe, and the physical
    release file is published only after the real server stop/adoption. No
    AF_UNIX listener, connection or SO_PEERCRED exists on the run path.
    Failures retain original traceback plus independent cleanup errors, with
    no retry. Read-only fixture SQL (``rows``, zero timeout) is observed only
    outside the concurrent server-startup/write phases. Those writes can
    conflict with a zero-timeout readonly probe, which can legally refuse with
    "database is locked" (the actual 2026-11-05 observation), which says
    nothing about the asserted state. Each turn's ``managed_attempts``/
    ``workspace_turns`` metadata is therefore checked in the existing stable
    window after ``server.stop``/``native.finish`` and BEFORE the file
    rendezvous release: the provider's ready and probe publications were
    already causally acquired, the real server writes are finished, and both
    owner identities (helper pidfd, provider identity) are still held, so the
    admission rows are read after the known concurrent writer phase; the release then starts the
    provider settlement those rows witness. The turn-1 paused-reopen resource
    check runs before the second server starts — same settled rows, no live
    writer — while its public paused HTTP observation stays live.
    Post-release ``assert_resources`` calls and all public HTTP reads keep
    their positions: they witness the actual release and settlement.
    pending_dependent preplans a third dependent task, never launched
    here; the explicit fixture task budget grows to three without changing
    product policy. disjoint_results gives each inert worker its own
    model-received/handoff filenames at birth (the release name keeps the
    protocol naming release-<turn>.json for every turn), preserving actual
    immutable outputs for conflict-free cumulative tests.
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
    server=supervisor=None;fds=[];agents=[];checks=[];errors=[];primary=None;reaped=True;nonce=None;release_name="release-worker.json";rendezvous=None
    path=lambda action:'/api/v1/mission?'+urlencode(dict(work=f.work,action=action))
    def passed(name):
        """Emit and retain each actual assertion, not an exit-only success."""
        checks.append(name);print('PASS '+name,flush=True)
    try:
        for turn in range(2):
            turn_name='turn-'+str(turn)
            nonce=fixture.digest((str(root)+':'+str(turn)).encode())
            program=file_provider.program(nonce,turn_name)
            handoff_name='handoff.md';release_name='release-'+turn_name+'.json'
            if disjoint_results:
                handoff_name='handoff-'+str(turn)+'.md'
                program=program.replace('model-received.json','model-received-'+str(turn)+'.json').replace('handoff.md',handoff_name)
            fixture.publish(root/'codex',program.encode());(root/'codex').chmod(0o700)
            if turn:
                # Paused-reopen SQL observation: the first turn's resource rows
                # are settled (its cycle stopped the server, reaped the helper
                # and closed the rendezvous) and no second server is live yet,
                # so read-only fixture SQL has no writer to refuse with. The
                # public paused HTTP observation below stays on the live
                # reopened server; retention across the reopen is witnessed by
                # that public read and the final stable assert_resources.
                assert_resources(f,agents)
            server=native.Server(binary,root,out,turn_name,environment=f.environment)
            identity=worker.observation(server.process.pid);server_fd=worker.attach(identity);fds.append(server_fd)
            csrf=server.request('/api/v1/native-session')['csrf_token']
            assert len(f.ok(['mission','status-native',f.work])['agents'])==turn
            if turn:
                assert request(server,'GET',path('show'))['paused'] is True
                passed('managed-auto-paused-reopen-retains-first-completion')
            current=f.ok(['work','show',f.work])['work']
            body=dict(schema_version=1,event_id='managed-auto-launch-'+str(turn),expected_revision=current['revision'],reason='Owned sequential worker cycle')
            if turn==0:assert not (storage/'copies').exists()
            request(server,'POST',path('resume'),body,csrf)
            print('STAGE awaiting-managed-auto-provider-'+str(turn),flush=True)
            agent,supervisor,supervisor_fd,provider,provider_fd=_attach_canonical_agent(f,f.work,agents,server.process.pid,server_fd,binary,root)
            fds.extend([supervisor_fd,provider_fd]);reaped=False
            # The public payload only reported this identity; _attach_agent_kernel
            # re-established every field from real /proc facts (stamps, parent
            # chain, argv, exe hash) and holds caller-owned pidfds BEFORE any
            # file report is read. A ready/probe payload can only repeat, never
            # authorize, this identity.
            provider_identity=dict(pid=provider['pid'],ppid=provider['ppid'],sid=provider['sid'],pgid=provider['pgid'])
            assert agent['task_id'] in {'task','task-2'} and agent['task_id'] not in {a['task_id'] for a in agents}
            workspace=Path(agent['workspace'])
            assert workspace==storage/'copies'/(agent['task_id']+'-1')/project_subdir
            copy=storage/'copies'/(agent['task_id']+'-1')
            rendezvous=file_provider.FileRendezvous(workspace,nonce,turn_name,provider_identity,provider_fd,terminal_fds=(('conductor',server_fd),('helper',supervisor_fd)))
            assert workspace.is_dir() and (workspace/'tracked.txt').read_bytes()==b'Committed planning input\n'
            assert repository.inventory(root/'source')==source
            worker.verify_current_root_binding(agent['native_mission'],root)
            assert len(f.ok(['mission','status-native',f.work])['agents'])==turn+1
            current=f.ok(['work','show',f.work])['work']
            request(server,'POST',path('pause'),dict(schema_version=1,event_id='managed-auto-pause-'+str(turn),expected_revision=current['revision'],reason='Keep next task pending until explicit resume'),csrf)
            assert request(server,'GET',path('show'))['paused'] is True
            entry=rendezvous.ready();assert entry['nonce']==nonce and entry['pid']==provider['pid']
            reply=rendezvous.probe();assert reply['pid']==provider['pid'] and reply['ppid']==supervisor['pid']
            server.stop();native.finish(server,None);server=None
            assert worker.observation(supervisor['pid'])['ppid']==os.getpid()
            assert worker.supervision(f)['stopped_at']!=''
            # STABLE SQL OBSERVATION WINDOW (causality of this boundary):
            # ready and probe were acquired above, the owned server is really
            # stopped/finished and the caller still holds the helper pidfd and
            # the validated provider identity, while the provider itself stays
            # BLOCKED on release-<turn>.json. The known concurrent writer phase has ended; any unexpected
            # readonly-probe refusal remains a failure. This is where admission
            # metadata is read; the release below starts exactly the provider
            # settlement these rows witness, so they must be asserted before it.
            assert rows(f,'SELECT work_id,task_id,path,state FROM managed_attempts WHERE agent_id=?',(agent['id'],))==[(f.work,agent['task_id'],str(copy),'ready')]
            if turn==0:
                other='task-2' if agent['task_id']=='task' else 'task'
                waiting=rows(f,'SELECT workspace,state FROM workspace_turns WHERE work_id=? AND task_id=?',(f.work,other))
                assert waiting==[(str(storage/'copies'/(other+'-1')/project_subdir),'waiting')],waiting
                assert not (storage/'copies'/(other+'-1')).exists()
                passed('managed-auto-waiting-FIFO-does-not-create-second-copy')
            assert rendezvous.release()==dict(nonce=nonce,release=True)
            select.select([supervisor_fd],[],[])
            waited,status=os.waitpid(supervisor['pid'],0);reaped=True
            assert waited==supervisor['pid'] and os.WIFEXITED(status) and os.WEXITSTATUS(status)==0
            agents.append(agent);work=assert_resources(f,agents)
            if turn==0:
                remaining=[t for t in work['tasks'] if t['id'] in {'task','task-2'} and t['id']!=agent['task_id']]
                assert len(remaining)==1 and remaining[0]['status']=='todo' and not remaining[0].get('attempts')
            fixture.publish(out/('turn-'+str(turn)+'-completed.json'),dict(agent=agent,work=work))
            rendezvous.close();rendezvous=None
            # Each managed task owns a different copy; keep all producer originals.
            assert (workspace/handoff_name).is_file() and (workspace/release_name).is_file()
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
            release=workspace/release_name if workspace is not None else None
            if nonce is not None and release is not None and not release.exists():worker.retain_failure(errors,lambda:fixture.publish(release,dict(nonce=nonce,release=True)))
            if supervisor is not None and not reaped:worker.retain_failure(errors,lambda:public.reap_cleanup_helper(supervisor))
            worker.retain_failure(errors,lambda:public.cleanup_owned_children(binary,root,out))
        if rendezvous is not None:worker.retain_failure(errors,rendezvous.close)
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
