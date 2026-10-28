"""Observe SIGINT or SIGTERM during an actual held role without a global watchdog.

A traced healthy role precedes the signal case. strace follows only poll and
signal-return syscalls, never private read/write payloads. -D keeps the actual
watch as our child; kernel peer and canonical permission checks remain required.
The optional role progress check requires two completed main-thread timed
receiver waits after actual granted entry, bracketing a further loop iteration, with provider still held. Only futex/poll
metadata is traced; no payload. It is Linux/source-bound, not scheduler performance
or proof of all threads. Each role uses a real held peer/canonical slot. The baseline may defer
stop until role return; this probe records that fact,
releases its own gate and reaps. It is not full Go asynchronous parity evidence.
"""
from pathlib import Path
import argparse,json,os,re,select,shutil,signal,socket,subprocess,traceback
import rust_agent_model_route as fixture
import rust_mission_watch_planning as planning
import rust_mission_report_relay as relay
import rust_mission_watch_worker as worker
import rust_native_parent_death as parent_death
import rust_mission_watch_public as public
check=worker.check


def records(path):
    """Read complete public JSONL observations only; incomplete last line waits."""
    return [json.loads(line) for line in path.read_bytes().splitlines(keepends=True) if line.endswith(b'\n')]


def deferred_trace(text):
    """Recognize delivered main-thread SIGINT then resumed provider poll.

    A signal line alone is not handler completion. Require rt_sigreturn and a
    subsequent ppoll invocation; this is a syscall-order witness, not a latency
    threshold or proof of all threads' state. Only the original synchronous
    baseline uses this branch; a responsive run must wait for actual watch exit
    while its provider is held, never stop the test at this intermediate poll.
    """
    marker=text.find('--- SIGINT ')
    if marker<0:return False
    returned=text.find('rt_sigreturn(',marker)
    return returned>=0 and text.find('ppoll(',returned)>=0


def completed_main_waits(text, offset):
    """Recognize complete timed receiver-wait lines after the entry offset.

    Caller supplies the actual MAIN pid trace. Two completed waits bracket a
    further iteration in this source-bound conductor. A trace alone does not
    authenticate which high-level function called futex. Partial/old lines and
    other futex operations are excluded; no deadline or retry is introduced.
    """
    return [m.group() for m in re.finditer(r'futex\([^\n]*FUTEX_WAIT_BITSET_PRIVATE[^\n]*\{tv_sec=[^\n]*ETIMEDOUT[^\n]*\n', text) if m.end() > offset]


def scenario(binary,out,interrupt,probe_deferred=False,stop_signal=signal.SIGINT,role='planner',require_progress=False):
    """Own one traced watch, bind real permitted peer, then release or interrupt.

    Inotify/pidfds observe progress; no invented timeout or SQL writer. Original
    channels and trace remain. Finally release only our held provider and signal/
    wait our actual watch. Every cleanup cause survives, including a failed gate
    release; no process or funding reset is used to manufacture success.
    """
    if role=='planner':owned,nonce,_=planning.setup(binary,out/'setup')
    else:
        owned,_,_=relay.prepare(binary,out/'setup',controls=True,control_gate=role=='controls',review_gate=role=='reviewer');nonce=None
    socket_name={'planner':'watch-planner.sock','controls':'control-probe.sock','reviewer':'reviewer-probe.sock'}[role]
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);listener.bind(str(owned.root/socket_name));listener.listen(1)
    process=watchfd=childfd=notify=peer=None;streams=[];errors=[];observed={};tracers=[]
    try:
        streams=[worker.spool(out/'watch.stdout'),worker.spool(out/'watch.stderr')];notify=worker.watch_directory(out,modifications=True)
        argv=['strace','-D','-ff','-qq','-o',str(out/'trace'),'-e','trace=ppoll,poll,pselect6,rt_sigreturn'+(',futex' if require_progress else ''),'-e','signal=SIGINT,SIGTERM',str(binary),'mission','watch',owned.work,'--root',str(owned.root),'--json']
        fixture.publish(out/'command.json',dict(argv=argv,global_watchdog=None))
        process=subprocess.Popen(argv,env=owned.environment,stdin=subprocess.DEVNULL,stdout=streams[0],stderr=streams[1]);watchfd=os.pidfd_open(process.pid)
        ready,_,_=select.select([listener,watchfd],[],[]);check(listener in ready and watchfd not in ready,'traced watch exited before actual planner')
        peer,_=listener.accept()
        if role=='planner':
            identity,childfd=planning.planner_peer(peer,worker.observation(process.pid));frame=worker.frame(peer)
            check(frame.get('nonce')==nonce and frame.get('pid')==identity['pid'],'planner peer/nonce mismatch under tracer')
            job=relay.rows(owned,'SELECT claim_id,permission,phase,child_pid,owner_pid FROM native_planning_runs WHERE work_id=? AND child_pid=?',(owned.work,identity['pid']))
            current=relay.work(owned);held=[scope for scope in current['planning']['scopes'] if scope.get('holder')]
            check(len(held)==1 and held[0]['id']=='root','unexpected held scope');planning.planning.proposal(owned,current,'root',[])
        else:
            identity,childfd=worker.owned_provider_peer(peer,worker.observation(process.pid));entry=b''
            while len(entry)<5:
                ready,_,_=select.select([peer,childfd,watchfd],[],[]);check(peer in ready,'owned role exited before ENTRY frame')
                chunk=peer.recv(5-len(entry));check(bool(chunk),'owned role entry EOF');entry+=chunk
            check(entry==b'ENTRY','wrong role entry frame')
            if role=='reviewer':job=relay.rows(owned,'SELECT id,permission,phase,child_pid,owner_pid FROM native_review_jobs WHERE work_id=?',(owned.work,))
            else:job=relay.rows(owned,'SELECT j.id,s.permission,s.phase,s.child_pid,j.owner_pid FROM native_validation_jobs j JOIN native_validation_steps s ON s.job_id=j.id WHERE j.work_id=?',(owned.work,))
        check(len(job)==1 and job[0][1:]==(1,'permitted',identity['pid'],process.pid),'canonical role permission mismatch')
        finance_before=relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))
        for text in Path('/proc/self/task',str(os.getpid()),'children').read_text().split():
            pid=int(text)
            if pid==process.pid:continue
            executable=Path('/proc',str(pid),'exe')
            try:
                actual=Path('/proc',str(pid),'cmdline').read_bytes().split(b'\0')[:-1]
                if executable.resolve()==Path(shutil.which('strace')).resolve() and actual==[os.fsencode(v) for v in argv]:
                    stamp=worker.observation(pid);check(stamp['ppid']==os.getpid(),'tracer not our adopted child');tracers.append((pid,worker.attach(stamp)))
            except FileNotFoundError:continue
        check(len(tracers)==1,'actual owned tracer not identified')
        observed.update(peer=identity,job=job[0],role=role,provider_held=True,tracer_pid=tracers[0][0])
        fixture.publish(out/'actual-entry.json',observed)
        if interrupt and require_progress:
            # Two main timed completions bracket another saved-cadence loop iteration
            # AFTER granted provider entry; never release G or invent a timeout.
            trace=out/('trace.'+str(process.pid))
            start=len(trace.read_bytes()) if trace.exists() else 0
            while True:
                text=trace.read_text() if trace.exists() else ''
                waits=completed_main_waits(text,start)
                events=records(out/'watch.stdout')
                if len(waits)>=2 and any(e.get('event')=='mission-watch-blocked' and e.get('synchronization_committed') is True for e in events):
                    check(not select.select([watchfd,childfd],[],[],0)[0],'progress witness raced role departure')
                    observed['two_main_timed_waits_after_granted_entry']=True
                    fixture.publish(out/'conductor-progress.json',dict(main_pid=process.pid,trace_offset=start,completed_waits=waits[-2:],provider_held=True,dispatch_sync_observed=True,interpretation='two timed main waits bracket a further iteration in the qualified conductor source; trace alone is not a callstack attestation',global_watchdog=None))
                    break
                ready,_,_=select.select([notify,watchfd,childfd],[],[])
                check(watchfd not in ready and childfd not in ready,'role/watch exited before conductor progress')
                if notify in ready:os.read(notify,65536)
        if interrupt:signal.pidfd_send_signal(watchfd,stop_signal)
        else:peer.sendall(b'G');peer.close();peer=None
        while True:
            events=records(out/'watch.stdout')
            healthy={'planner':lambda e:e.get('event')=='mission-planning-step' and e.get('decision_applied'),'reviewer':lambda e:e.get('event')=='mission-review-recorded' and e.get('verdict_applicable'),'controls':lambda e:e.get('event')=='mission-controls-recorded' and e.get('controls_passed')}[role]
            if not interrupt and any(healthy(e) for e in events) and (role!='controls' or any(e.get('event')=='mission-watch-blocked' for e in events)):
                observed['healthy_planner_applied']=role=='planner';observed['healthy_role_recorded']=True;break
            trace=out/('trace.'+str(process.pid))
            if interrupt and probe_deferred and trace.exists() and deferred_trace(trace.read_text()):
                ready,_,_=select.select([watchfd,childfd],[],[],0)
                observed.update(signal_caught_then_foreground_poll=True,watch_and_provider_still_owned=not ready)
                check(not ready,'syscall deferred witness raced completed process')
                break
            ready,_,_=select.select([notify,watchfd],[],[])
            if watchfd in ready:
                observed['watch_exited_while_provider_held']=interrupt;break
            if notify in ready:os.read(notify,65536)
    except BaseException as error:errors.append(error)
    finally:
        if errors and process is not None and process.poll() is None:
            try:
                public.mission_control(owned,'stop','signal-probe-owned-error-cleanup');signal.pidfd_send_signal(watchfd,signal.SIGINT)
            except BaseException as error:errors.append(error)
        if peer is not None:
            try:
                ready,_,_=select.select([childfd],[],[],0)
                if childfd not in ready:peer.sendall(b'G')
            except BaseException as error:errors.append(error)
        if process is not None:
            try:
                if process.poll() is None and not interrupt:signal.pidfd_send_signal(watchfd,signal.SIGINT)
                observed['watch_exit']=process.wait()
                for pid,fd in tracers:
                    waited,status=os.waitpid(pid,0);check(waited==pid and os.waitstatus_to_exitcode(status)==0,'owned tracer failed');os.close(fd)
                tracers.clear()
            except BaseException as error:errors.append(error)
        for pid,fd in tracers:
            try:os.close(fd)
            except BaseException as error:errors.append(error)
        for resource in [peer,listener,*streams]:
            if resource is not None:
                try:resource.close()
                except BaseException as error:errors.append(error)
        for fd in [notify,childfd,watchfd]:
            if fd is not None:
                try:os.close(fd)
                except BaseException as error:errors.append(error)
    if errors:raise BaseExceptionGroup('watch signal probe and owned cleanup',errors)
    current=relay.work(owned);events=records(out/'watch.stdout');fixture.publish(out/'public-work.json',current)
    finance_after=relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))
    check(all(row in finance_after for row in finance_before),'stop reset original planning funding')
    fixture.publish(out/'finance-before.json',finance_before);fixture.publish(out/'finance-after.json',finance_after)
    if interrupt and observed.get('watch_exited_while_provider_held'):
        event={'planner':'mission-planning-interrupted','controls':'mission-controls-interrupted','reviewer':'mission-review-interrupted'}[role]
        check(any(e.get('event')==event and e.get('failed_call_recorded') for e in events),'durable interrupted call event absent')
        pattern={'planner':'native-planning-*.receipt','reviewer':'native-review-*.receipt','controls':'native-validation-step-*.receipt'}[role]
        receipts=list((owned.root/'.swarm').glob(pattern));check(len(receipts)==1,'interrupted role original receipt absent')
        magic,extent,digest,body=receipts[0].read_bytes().split(b'\n',3)
        check(magic==b'SR1' and int(extent)==len(body) and fixture.digest(body)==digest.decode(),'interruption original framing invalid')
        receipt=json.loads(body);check(receipt['signal']==15 and receipt['grant_attempted'] is True,'interruption TERM/permission facts absent')
        if role=='planner':
            check(not any(e.get('event')=='mission-planning-step' and e.get('decision_applied') for e in events),'interrupted planner applied a decision')
            check(receipt['output_complete'] is False and receipt['failure']=='planning-execution-interrupted','planner interruption failure absent')
        else:
            check(receipt['waited'] and receipt['channel_closed'] and receipt['failed'],'role cleanup/wait/failure absent')
            task=next(t for t in current['tasks'] if t['id']=='task');check(task['status']!='accepted','signal fabricated task acceptance')
            if role=='reviewer':check(task['independent_review']['state']=='error' and receipt['original_reply'] is None,'interrupted reviewer published verdict')
            else:check(not any(e.get('event')=='mission-controls-recorded' and e.get('controls_passed') for e in events),'interrupted controls passed')
        controls=public.status_native(owned)['controls'];check(controls['paused'] is False and controls['consent_enabled'] is True,'technical watch stop changed mission consent')
        observed['failed_call_and_TERM_receipt_verified']=True
    observed.update(root=str(owned.root),work=owned.work,planner_applied=any(e.get('event')=='mission-planning-step' and e.get('decision_applied') for e in events))
    fixture.publish(out/'observed.json',observed);return observed


def main():
    """Require exact qualified profile and name expected baseline interpretation."""
    p=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:p.add_argument('--'+flag,type=Path,required=True)
    p.add_argument('--require-conductor-progress',action='store_true');p.add_argument('--role',choices=['planner','reviewer','controls'],default='planner');p.add_argument('--expect-deferred',action='store_true');p.add_argument('--signal',choices=['sigint','sigterm'],default='sigint');a=p.parse_args();assert not (a.expect_deferred and a.signal!='sigint');assert not (a.require_conductor_progress and a.expect_deferred);out=a.output.resolve();out.mkdir(exist_ok=False);binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes())
    check(profile['sha256']==fixture.digest(binary.read_bytes()) and profile['no_fixture_feature'],'signal probe binary/profile mismatch')
    fixture.publish(out/'probe-source.py',Path(__file__).read_bytes());fixture.publish(out/'probe-input.json',dict(binary_sha256=profile['sha256'],profile_sha256=fixture.digest(a.profile.read_bytes()),probe_source_sha256=fixture.digest(Path(__file__).read_bytes()),global_watchdog=None))
    def run():
        positive=scenario(binary,out/'positive',False,role=a.role);check(positive.get('healthy_role_recorded') and positive['watch_exit']==0,'traced positive planner failed')
        fault=scenario(binary,out/'signal',True,a.expect_deferred,signal.SIGINT if a.signal=='sigint' else signal.SIGTERM,role=a.role,require_progress=a.require_conductor_progress)
        if a.expect_deferred:check(fault.get('signal_caught_then_foreground_poll') and fault.get('watch_and_provider_still_owned') and fault['watch_exit']==0,'baseline does not exhibit deferred signal')
        else:check(fault.get('watch_exited_while_provider_held') and fault['watch_exit']==0,'watch SIGINT remained deferred')
        return dict(status='PASS',positive=positive,signal=fault,role=a.role,requested_signal=a.signal,expected_deferred=a.expect_deferred,global_watchdog=None,installed=False,conductor_progress_required=a.require_conductor_progress)
    result=worker.subreaper_scope(run,lambda error:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(error)))),parent_death.subreaper)
    fixture.publish(out/'result.json',result);print('WATCH_SIGNAL_PROBE_PASS',flush=True)
if __name__=='__main__':main()
