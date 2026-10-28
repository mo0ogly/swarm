#!/usr/bin/env python3
"""Recover complete actual slot originals after combined-receipt publication fails.

Owned isolated ordinary/managed public worker flows, read-only SQL and real native
commands. A valid batch precedes a held-control filesystem fault. Missing/unknown
slots and changed source refuse without mutation; exact restoration permits
reconciliation without commands. A retained failed pre-fix fixture can be resumed
with its original request. No fabricated acceptance, provider, global watchdog or
SQL write. Normal binary/profile and helper closure are fingerprinted.
"""
import argparse,json,os,select,socket,subprocess,traceback
from pathlib import Path
import rust_native_managed_validation as flow
from rust_native_task_validation import invoke


def run(binary,profile,out,*,ordinary=False,resume=None):
    """Exercise actual journals/recovery/retirement, preserving original failures.

    Resume reuses only the original failed job/root, never reruns its commands or
    rewrites its SQL. Incomplete/unknown evidence remains an explicit refusal.
    """
    out.mkdir(parents=True,exist_ok=False);fixture=flow.fixture
    p=json.loads(profile.read_bytes());assert p['no_fixture_feature'] and p['sha256']==fixture.digest(binary.read_bytes())
    if resume:
        state=json.loads((resume/'pending.json').read_bytes());root=Path(state['root']);work=state['work'];workspace=Path(state['workspace']);request=state['request'];job=state['job'];ordinary=state['ordinary'];origin=resume
    else:
        origin=out
        if ordinary:
            import rust_planning_worker_public_flow as public
            setup=public.flow(binary,out/'workers',control_script=flow.FIRST)
            root=Path(setup['root']);work=setup['work_id'];workspace=root
            (root/'tracked.txt').write_bytes(b'Committed planning input\n')
        else:
            flow.managed.worker.subreaper_scope(lambda:flow.managed.run(binary,profile,out/'workers',project_subdir='app',controls=flow.CONTROLS),lambda error:None,flow.managed.parent_death.subreaper)
            setup=json.loads((out/'workers/fixture.json').read_bytes());root=Path(setup['root']);work=setup['work']
    environment=dict(os.environ);environment['CODEX_HOME']=str(root/'codex-home');checks=[]
    def call(label,args,value=None):return invoke(binary,root,out,label,args,value,environment=environment)
    def ok(label,args,value=None):
        code,reply=call(label,args,value);assert code==0,(label,reply);return reply
    def show(label):return ok(label,['work','show',work])['work']
    def request_for(label):return dict(schema_version=1,event_id=label+'-'+out.name,expected_revision=show(label+'-before')['revision'],task_id='task')
    def markers():return {p.name:fixture.digest(p.read_bytes()) for p in workspace.glob('native-*.json')}
    if not resume:
        if not ordinary:
            w=show('resume-state');mission=ok('mission-state',['mission','show',work])
            if mission['paused']:ok('resume-mission',['mission','resume',work],dict(schema_version=1,event_id='partial-mission',expected_revision=w['revision'],reason='Explicit isolated controls'))
            w=show('resume-plan-state')
            if w['planning'].get('paused'):ok('resume-planning',['planning','resume',work],dict(schema_version=1,event_id='partial-planning',expected_revision=w['revision'],reason='Explicit isolated controls'))
            prepared=ok('candidate',['task','validation-candidate',work],request_for('candidate'))
            repo=show('repository')['planning']['repository'];candidates=[json.loads(f.read_bytes()) for f in (Path(repo['storage'])/'candidates').glob('*/candidate.json') if json.loads(f.read_bytes())['task']=='task'];assert len(candidates)==1;workspace=Path(candidates[0]['workspace'])
        positive=ok('valid-before-fault',['task','validate',work],request_for('valid'))
        assert positive['controls_passed'] and not positive['task_accepted'];checks.append('actual-valid-batch-before-fault')

    def held(label):
        """Admit a real held child, then deny only its future combined receipt.

        Listener procfd closes before spawn; socket+pidfd expose early exit.
        Owned channels/children are settled on every fixture failure.
        """
        request=request_for(label);listener=socket.socket(socket.AF_UNIX);connection=None;process=None;pidfd=None;primary=None;cleanup=[];sock=workspace/'native-control.sock'
        stdout=(out/(label+'.stdout')).open('xb');stderr=(out/(label+'.stderr')).open('xb')
        try:
            fd=os.open(workspace,os.O_RDONLY|os.O_DIRECTORY|os.O_CLOEXEC|os.O_NOFOLLOW)
            try:listener.bind('/proc/'+str(os.getpid())+'/fd/'+str(fd)+'/native-control.sock')
            finally:os.close(fd)
            listener.listen(1);input_file=out/(label+'.input.json');fixture.publish(input_file,request)
            argv=[str(binary),'task','validate',work,'--root',str(root),'--input',str(input_file),'--json'];fixture.publish(out/(label+'.command.json'),dict(argv=argv,global_watchdog=None))
            process=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=environment);pidfd=os.pidfd_open(process.pid)
            ready,_,_=select.select([listener,pidfd],[],[]);assert listener in ready,('controller-exited',process.poll())
            connection,_=listener.accept();assert connection.recv(1)==b'R'
            jobs=flow.rows(root,"SELECT id FROM native_validation_jobs WHERE work_id=? AND task_id='task' AND phase='active'",(work,));assert len(jobs)==1;job=jobs[0][0]
            assert flow.rows(root,'SELECT count(*) FROM native_validation_steps WHERE job_id=? AND permission=1',(job,))==[(1,)]
            target=root/'.swarm'/('native-validation-job-'+fixture.digest(job.encode())+'.receipt');assert not target.exists();target.mkdir(mode=0o700)
            connection.sendall(b'G');connection.close();connection=None;assert process.wait()!=0
        except BaseException as error:primary=error
        finally:
            if connection is not None:
                flow.managed.worker.retain_failure(cleanup,lambda:connection.sendall(b'G'));flow.managed.worker.retain_failure(cleanup,connection.close)
            if process is not None and process.poll() is None:flow.managed.worker.retain_failure(cleanup,process.wait)
            if pidfd is not None:flow.managed.worker.retain_failure(cleanup,lambda:os.close(pidfd))
            for v in [listener,stdout,stderr]:flow.managed.worker.retain_failure(cleanup,v.close)
            if sock.exists():flow.managed.worker.retain_failure(cleanup,sock.unlink)
        if primary or cleanup:raise BaseExceptionGroup('Partial receipt fixture',([primary] if primary else [])+cleanup)
        return request,job,target

    if not resume:
        request,job,target=held('combined-refusal');count=1 if ordinary else 3
        slots=flow.rows(root,'SELECT id FROM native_validation_steps WHERE job_id=? ORDER BY ordinal',(job,));assert len(slots)==count
        for (sid,) in slots:
            _,_,slot=flow.original(root,'step',sid);assert slot['waited'] and slot['result']['passed']
        unchanged=show('before-combined-shape-refusal');code,_=call('existing-invalid-combined',['task','validation-reconcile',work],request);assert code!=0 and target.is_dir() and show('after-combined-shape-refusal')==unchanged
        target.rmdir();state=dict(root=str(root),work=work,workspace=str(workspace),request=request,job=job,ordinary=ordinary,markers=markers(),slots=[v[0] for v in slots]);fixture.publish(out/'pending.json',state)
    state=json.loads((origin/'pending.json').read_bytes());assert state['job']==job;before=markers();assert before==state['markers']
    current=show('before-reconstruction')
    # A complete restored set is required. Temporarily absent/unknown originals
    # must never become empty successful observations or replacement commands.
    path,raw,slot=flow.original(root,'step',state['slots'][0]);parked=path.with_suffix('.partial-retained');path.rename(parked)
    try:
        code,_=call('missing-slot-refusal',['task','validation-reconcile',work],request);assert code!=0 and show('missing-slot-unchanged')==current and markers()==before
    finally:parked.rename(path)
    unknown=json.loads(json.dumps(slot));unknown['waited']=False;unknown['exit_code']=None;unknown['signal']=None
    payload=json.dumps(unknown,separators=(',',':'),ensure_ascii=False).encode();frame=b'SR1\n'+str(len(payload)).zfill(20).encode()+b'\n'+fixture.digest(payload).encode()+b'\n'+payload
    path.write_bytes(frame)
    try:
        code,wire=call('unknown-wait-refusal',['task','validation-reconcile',work],request);assert code!=0 and 'task-validation-process-reap-unknown' in json.dumps(wire),(code,wire)
        assert show('unknown-wait-unchanged')==current and markers()==before
    finally:path.write_bytes(raw)
    tracked=workspace/'tracked.txt';saved=tracked.read_bytes();tracked.write_bytes(b'changed source\n')
    try:
        # Ordinary declared input is the report; managed raw tracked source is
        # the intentionally changed candidate input. Only claim the latter case.
        if not ordinary:
            code,_=call('changed-source-refusal',['task','validation-reconcile',work],request);assert code!=0 and show('source-unchanged')==current and markers()==before
    finally:tracked.write_bytes(saved)
    recovered=ok('recover-complete-slots',['task','validation-reconcile',work],request)
    assert recovered['controls_passed'] and recovered['job_id']==job and not recovered['task_accepted'] and recovered['replayed'] and markers()==before
    _,body,receipt=flow.original(root,'job',job);_,rebuilt,assembly=flow.original(root,'reconstruction',job);assert body==rebuilt and receipt==assembly
    for sid in state['slots']:flow.original(root,'step',sid)
    repeated=ok('replay',['task','validate',work],request);assert repeated['replayed'] and repeated['job_id']==job and markers()==before
    checks+=['invalid-existing-combined-not-replaced','missing-slot-refuses-without-mutation','unknown-wait-refuses-without-mutation','complete-originals-reconstruct-without-commands','reconstruction-provenance-exact','terminal-replay-without-commands']
    if not ordinary:checks.append('changed-source-refuses-and-exact-restore-recovers')
    # Separate actual complete originals may be abandoned without acceptance.
    request2,job2,target=held('retire-partial');target.rmdir();before=markers()
    retired=ok('retire-reconstructed',['task','validation-retire',work],request2);assert retired['retired'] and not retired['controls_passed'] and not retired['task_accepted'] and markers()==before
    repeated=ok('retire-replay',['task','validation-retire',work],request2);assert repeated['retired'] and repeated['replayed'] and markers()==before
    checks.append('complete-slot-retirement-and-replay-without-commands')
    result=dict(status='PASS',checks=checks,ordinary=ordinary,root=str(root),work=work,job=job,task_accepted=False,reexecuted=False)
    fixture.publish(out/'result.json',result);print('PARTIAL_VALIDATION_PASS '+str(len(checks))+' assertions');return result


def main():
    """Bind exact normal inputs and preserve failures; never retry implicitly."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--resume-from',type=Path);parser.add_argument('--ordinary',action='store_true');args=parser.parse_args();binary=args.binary.resolve();profile=args.profile.resolve();out=args.output.resolve()
    inputs=dict(binary=str(binary),sha256=flow.fixture.digest(binary.read_bytes()),profile_sha256=flow.fixture.digest(profile.read_bytes()),helpers=flow.helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        run(binary,profile,out,ordinary=args.ordinary,resume=args.resume_from.resolve() if args.resume_from else None)
        assert flow.fixture.digest(binary.read_bytes())==inputs['sha256'] and flow.fixture.digest(profile.read_bytes())==inputs['profile_sha256'] and all(flow.fixture.digest(Path(n).read_bytes())==h for n,h in inputs['helpers'].items())
        flow.fixture.publish(out/'inputs.json',inputs)
    except BaseException:
        if out.is_dir():flow.fixture.publish(out/'inputs.json',inputs);flow.fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
