#!/usr/bin/env python3
"""Run managed controls through actual native claim, helper, journals and recovery.

Actual isolated managed workers/public immutable handoffs precede each journey.
Three saved controls execute in the retained candidate project, not Root or a
worker copy. A valid held-control witness precedes filesystem publication refusal;
restore and public reconciliation reuse originals without another command.
SQL is read-only; no accepted task, provider identity or independent review is
fabricated. Existing process/deadline settings remain authoritative; no global
watchdog, hidden retry or paid business provider is introduced.
"""
import argparse,hashlib,json,os,select,socket,sqlite3,subprocess,sys,traceback
from pathlib import Path
from types import SimpleNamespace
import rust_agent_model_route as fixture
import rust_managed_auto_worker as managed
import rust_mission_report_relay as relay
from rust_native_task_validation import invoke
from rust_native_managed_candidate import helper_inputs

HEADER = """from pathlib import Path
import os,json,hashlib,socket
ppid=os.getppid()
stamp=Path('/proc/'+str(ppid)+'/stat').read_text().rsplit(')',1)[1].split()[19]
tag=str(ppid)+'-'+stamp
assert Path('tracked.txt').read_bytes()==b'Committed planning input\\n'
"""
FIRST=HEADER+"""with Path('native-'+tag+'-first.json').open('x') as f:
 json.dump(dict(pid=os.getpid(),ppid=ppid,cwd=os.getcwd(),input_sha256=hashlib.sha256(Path('tracked.txt').read_bytes()).hexdigest()),f)
 f.flush();os.fsync(f.fileno())
if Path('native-control.sock').exists():
 with socket.socket(socket.AF_UNIX) as s:
  s.connect('native-control.sock');s.sendall(b'R');assert s.recv(1)==b'G'
print('ACTUAL-MANAGED-FIRST',flush=True)
"""
SECOND=HEADER+"""assert Path('native-'+tag+'-first.json').exists()
with Path('native-'+tag+'-second.json').open('x') as f:
 json.dump(dict(pid=os.getpid(),ppid=ppid,cwd=os.getcwd()),f);f.flush();os.fsync(f.fileno())
print('ACTUAL-MANAGED-SECOND',flush=True)
if Path('docs/task-2.md').exists(): raise SystemExit(17)
"""
THIRD=HEADER+"""assert Path('native-'+tag+'-second.json').exists()
with Path('native-'+tag+'-third.json').open('x') as f:
 json.dump(dict(pid=os.getpid(),ppid=ppid,cwd=os.getcwd()),f);f.flush();os.fsync(f.fileno())
print('ACTUAL-MANAGED-THIRD',flush=True)
"""
CONTROLS=[dict(id=name,command=['python3','-c',script],criteria=[1],inputs=['tracked.txt'],
    justification='Verify actual retained candidate input and native process order',review_output=True)
    for name,script in [('candidate-first',FIRST),('candidate-second',SECOND),('candidate-third',THIRD)]]


def rows(root,sql,args=()):
    """Observe isolated actual SQL without writes; close even when querying fails."""
    db=sqlite3.connect('file:'+str(root/'.swarm/state.db')+'?mode=ro',uri=True)
    try:return db.execute(sql,args).fetchall()
    finally:db.close()


def original(root,kind,identifier):
    """Check the actual SR1 format/hash and return immutable private JSON bytes.

    Magic/20-digit extent/64-hex digest are protocol representation, not policy.
    Decoding this fixture alone never proves a permit or an actual wait.
    """
    path=root/'.swarm'/('native-validation-'+kind+'-'+fixture.digest(identifier.encode())+'.receipt')
    raw=path.read_bytes();magic,length,digest,body=raw.split(b'\n',3)
    assert magic==b'SR1' and len(length)==20 and length.isdigit() and int(length)==len(body)
    assert len(digest)==64 and fixture.digest(body).encode()==digest
    return path,raw,json.loads(body)


def run(binary,profile,out,*,before_binary=None,workers_evidence=None,worker_binary=None,subproject=False,resume_from=None):
    """Verify positive execution before faults, final originals and exact recovery.

    Optional retained actual PASS workers may come from an explicitly identified
    prior normal executable; their real SQL/submission/ownership is still checked
    by the current Root consumer. No source copy or runtime-state rewrite occurs.
    resume_from replays a previously successful positive job using its exact
    public request and pre-execution policy observation instead of reexecuting.
    Each new control invocation is explicit, not a worker/provider retry.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False);worker_binary=worker_binary or binary
    qualified=json.loads(profile.read_bytes());assert qualified['no_fixture_feature'] and fixture.digest(binary.read_bytes())==qualified['sha256']
    workers=workers_evidence or out/'workers'
    if workers_evidence is None:
        assert worker_binary==binary
        managed.worker.subreaper_scope(lambda:managed.run(binary,profile,workers,project_subdir='app' if subproject else '',controls=CONTROLS),lambda error:None,managed.parent_death.subreaper)
    setup=json.loads((workers/'result.json').read_bytes());assert setup['status']=='PASS' and setup['binary_sha256']==fixture.digest(worker_binary.read_bytes())
    birth=json.loads((workers/'fixture.json').read_bytes());root=Path(birth['root']);work=birth['work']
    environment=dict(os.environ);environment['CODEX_HOME']=str(root/'codex-home');checks=[]
    def call(label,args,value=None,exe=None):return invoke(exe or binary,root,out,label,args,value,environment=environment)
    def ok(label,args,value=None):
        code,value=call(label,args,value);assert code==0,(label,value);return value
    def show(label):return ok(label,['work','show',work])['work']
    def passed(label):checks.append(label);print('PASS '+label,flush=True)
    current=show('before');mission=ok('mission',['mission','show',work])
    if mission['paused']:ok('mission-resume',['mission','resume',work],dict(schema_version=1,event_id='controls-resume-'+out.name,expected_revision=current['revision'],reason='Explicit isolated native controls without conductor'))
    current=show('before-planning')
    if current['planning'].get('paused',False):ok('planning-resume',['planning','resume',work],dict(schema_version=1,event_id='controls-plan-resume-'+out.name,expected_revision=current['revision'],reason='Exercise native controls of actual submitted managed work'))
    initial=show('initial');storage=Path(initial['planning']['repository']['storage'])
    original_work=json.loads((resume_from/'initial.stdout').read_bytes())['work'] if resume_from else initial
    assert original_work['id']==work
    authorized={t['id']:t['validation_policy'] for t in original_work['tasks']}
    assert all(len(policy['controls'])==3 for policy in authorized.values())
    def request(event,task='task'):
        w=show(event+'-work');return dict(schema_version=1,event_id=event,expected_revision=w['revision'],task_id=task)
    def prepare(label,task='task'):
        r=request(label,task);before=show(label+'-before');value=ok(label,['task','validation-candidate',work],r)
        assert value['cohort_task_count']==1 and value['control_count']==3 and not value['job_created']
        assert show(label+'-after')==before
        candidates=[f for f in (storage/'candidates').glob('*/candidate.json') if json.loads(f.read_bytes())['task']==task]
        assert len(candidates)==1
        candidate=json.loads(candidates[0].read_bytes());workspace=Path(candidate['workspace'])
        assert workspace.name==('app' if subproject else 'copy')
        return candidate,workspace
    candidate,workspace=prepare('candidate')
    if before_binary:
        w=show('before-old');r=request('old-managed-refusal');count=rows(root,'SELECT count(*) FROM native_validation_jobs') if rows(root,"SELECT name FROM sqlite_master WHERE name='native_validation_jobs'") else []
        code,wire=call('old-managed-refusal',['task','validate',work],r,exe=before_binary)
        assert code!=0 and 'task-validation-managed-repository-unported' in json.dumps(wire),wire
        assert show('after-old')==w and (rows(root,'SELECT count(*) FROM native_validation_jobs') if count else [])==count
        passed('actual-prior-binary-refuses-managed-controls-before-job')
    def held(event,publication_fault=False):
        """Run identical held-control preconditions, optionally deny one public file.

        Listener and every subprocess channel are owned and checked. On failure,
        release any admitted control and wait before leaving; never infer a reap
        from an EOF or hide a primary error behind cleanup.
        """
        r=request(event);path=workspace/'native-control.sock';listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);connection=None;process=None;target=None;pidfd=None
        stdout=(out/(event+'.stdout')).open('xb');stderr=(out/(event+'.stderr')).open('xb');errors=[];primary=None
        try:
            # AF_UNIX bounds the supplied address, not the resolved cwd path.
            # Bind via our temporary directory FD, then close it before spawning
            # so fixture custody cannot mask missing engine descriptor ownership.
            address_fd=os.open(workspace,os.O_RDONLY|os.O_DIRECTORY|os.O_CLOEXEC|os.O_NOFOLLOW)
            try:listener.bind('/proc/'+str(os.getpid())+'/fd/'+str(address_fd)+'/native-control.sock')
            finally:os.close(address_fd)
            listener.listen(1)
            input_file=out/(event+'.input.json');fixture.publish(input_file,r)
            argv=[str(binary),'task','validate',work,'--root',str(root),'--input',str(input_file),'--json']
            fixture.publish(out/(event+'.command.json'),dict(argv=argv,global_watchdog=None,fixture_cache=environment['CODEX_HOME']))
            process=subprocess.Popen(argv,env=environment,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
            pidfd=os.pidfd_open(process.pid)
            ready,_,_=select.select([listener,pidfd],[],[])
            assert listener in ready,('controller exited before control admission',process.poll())
            connection,_=listener.accept();assert connection.recv(1)==b'R'
            jobs=rows(root,"SELECT id FROM native_validation_jobs WHERE work_id=? AND task_id='task' AND phase='active'",(work,));assert len(jobs)==1;jid=jobs[0][0]
            assert rows(root,"SELECT count(*) FROM native_validation_steps WHERE job_id=? AND permission=1",(jid,))==[(1,)]
            if publication_fault:
                target=root/'.swarm'/('native-validation-public-'+fixture.digest(jid.encode())+'.json');assert not target.exists();target.mkdir(mode=0o700)
            connection.sendall(b'G');connection.close();connection=None
            process.wait();code=process.returncode
        except BaseException as error:primary=error
        finally:
            if connection is not None:
                managed.worker.retain_failure(errors,lambda:connection.sendall(b'G'))
                managed.worker.retain_failure(errors,connection.close)
            if process is not None and process.poll() is None:managed.worker.retain_failure(errors,process.wait)
            if pidfd is not None:managed.worker.retain_failure(errors,lambda:os.close(pidfd))
            for item in [listener,stdout,stderr]:managed.worker.retain_failure(errors,item.close)
            if path.exists():managed.worker.retain_failure(errors,path.unlink)
        if primary is not None or errors:raise BaseExceptionGroup('Native held-control journey and cleanup',([primary] if primary else [])+errors)
        reply=json.loads((out/(event+('.stdout' if code==0 else '.stderr'))).read_bytes())
        return r,jid,code,reply,target
    def inspect(jid,expected_state,task='task'):
        """Bind actual job/slot/permission/wait/input/cwd and original public data."""
        _,bodyraw,body=original(root,'job',jid);assert body['schema_version']==3 and 'effective_policy' not in body
        assert body['managed_candidate']['task']==task and len(body['steps'])==3
        assert body['source_origin']['candidate_binding_sha256']==body['managed_candidate']['binding_sha256']
        assert len({s[0]['step']['control_id'] for s in body['steps']})==3
        assert body['policy']==authorized[task]
        assert [s[0]['result']['id'] for s in body['steps']]==[c['id'] for c in authorized[task]['controls']]
        assert all(s[0]['step']['control_id']!=s[0]['result']['id'] for s in body['steps'])
        assert all(s[0]['grant_attempted'] and s[0]['waited'] and s[0]['channel_closed'] and s[0]['step']['permission']==1 for s in body['steps'])
        assert rows(root,'SELECT phase,control_count FROM native_validation_jobs WHERE id=?',(jid,))==[(expected_state,3)]
        current=show('inspect-'+jid);t=next(v for v in current['tasks'] if v['id']==task)
        assert t['status']!='accepted' and not t.get('independent_review')
        ws=Path(body['managed_candidate']['workspace']);markers=list(ws.glob('native-*-first.json'))
        own=[f for f in markers if json.loads(f.read_bytes())['ppid']==body['job']['owner_pid']];assert len(own)==1
        marker=json.loads(own[0].read_bytes());assert marker['pid']==body['steps'][0][0]['step']['child_pid'] and marker['cwd']==str(ws) and marker['cwd']!=str(root)
        if expected_state=='recorded':
            validation=t['automatic_validation'];assert validation['candidate_sha']==body['managed_candidate']['commit']
            assert len(validation['controls'])==3 and body['policy']==t['validation_policy']
            assert any(name.endswith('/tracked.txt') and name.startswith('.swarm/managed/') for name in validation['artifacts'])
            assert all(fixture.digest((root/name).read_bytes())==digest for name,digest in validation['artifacts'].items())
        return body,t
    if resume_from:
        r1=json.loads((resume_from/'positive.input.json').read_bytes())
        prior=json.loads((resume_from/'positive.stdout').read_bytes());j1=prior['job_id']
        reply=ok('positive-original-replay',['task','validate',work],r1)
        assert reply['job_id']==j1 and reply['replayed'] and reply['controls_passed']
    else:
        r1,j1,code,reply,_=held('positive');assert code==0 and reply['controls_passed'] and not reply['task_accepted'],reply
    body,t=inspect(j1,'recorded');assert t['status']=='submitted' and t['automatic_validation']['state']=='pending_review'
    passed('actual-three-controls-execute-in-pinned-candidate-with-schema3-origin-and-transport-slots')
    before={f.name:fixture.digest(f.read_bytes()) for f in workspace.glob('native-*.json')}
    r2,j2,code,reply,target=held('publication-refusal',True);assert code!=0 and j2!=j1
    body2,_=inspect(j2,'active');assert all(s[0]['result']['passed'] for s in body2['steps'])
    after={f.name:fixture.digest(f.read_bytes()) for f in workspace.glob('native-*.json')};assert len(after)==len(before)+3
    code,wire=call('active-exact-replay',['task','validate',work],r2);assert code!=0 and 'task-validation-reconciliation-required' in json.dumps(wire)
    assert {f.name:fixture.digest(f.read_bytes()) for f in workspace.glob('native-*.json')}==after
    target.rmdir()
    recovered=ok('recover-original',['task','validation-reconcile',work],r2);assert recovered['controls_passed'] and not recovered['task_accepted'] and recovered['job_id']==j2
    assert {f.name:fixture.digest(f.read_bytes()) for f in workspace.glob('native-*.json')}==after
    inspect(j2,'recorded');passed('publication-refusal-retains-complete-original-and-public-recovery-does-not-reexecute')
    frame,raw,_=original(root,'candidate',j2);parked=workspace.with_name(workspace.name+'.replay-retained');saved_frame=frame.with_suffix('.retained');workspace.rename(parked);frame.rename(saved_frame)
    try:
        replay=ok('terminal-replay-without-candidate',['task','validate',work],r2);assert replay['replayed'] and replay['job_id']==j2 and replay['controls_passed']
        assert {f.name:fixture.digest(f.read_bytes()) for f in parked.glob('native-*.json')}==after
    finally:saved_frame.rename(frame);parked.rename(workspace)
    assert frame.read_bytes()==raw
    passed('terminal-exact-replay-needs-neither-live-workspace-nor-admission-frame')
    w=show('preview-work');preview=ok('managed-preview',['task','validation-preview',work],dict(schema_version=1,event_id='preview-after-controls',expected_revision=w['revision'],task_id='task'))
    assert preview['source_sha256']==body2['source_origin']['source_sha256'] and preview['control_count']==3 and not preview['execution_authorized']
    reviewer_preview=ok('managed-readonly-review-preview',['task','review-preview',work],dict(schema_version=1,event_id='review-preview-after-controls',expected_revision=w['revision'],task_id='task'))
    assert reviewer_preview['native_control_evidence_verified'] and not reviewer_preview['invocation_authorized'] and not reviewer_preview['provider_started']
    code,wire=call('managed-final-review-guard',['task','review',work],dict(schema_version=1,event_id='review-after-controls',expected_revision=w['revision'],task_id='task'))
    assert code!=0 and 'review-managed-candidate-unported' in json.dumps(wire),wire
    assert show('review-unchanged')==w
    passed('managed-preview-is-readonly-and-incomplete-final-review-remains-refused')
    c2,ws2=prepare('candidate-failing-task','task-2')
    # Actual foreground watch supplies an owning MissionLease; no JSON label or
    # manual validation request grants the automatic permission chain.
    owned=SimpleNamespace(binary=binary,root=root,work=work,environment=environment)
    relay.cycle(owned,out/'automatic-failed-controls',refusal=True)
    error=(out/'automatic-failed-controls/watch.stderr').read_text()
    assert 'task-validation-controls-failed' in error,error
    jobs=rows(root,'SELECT id FROM native_validation_jobs WHERE work_id=? AND task_id=?',(work,'task-2'));assert len(jobs)==1
    bindings=rows(root,'SELECT generation,consent_sequence FROM native_mission_validation_bindings WHERE job_id=?',(jobs[0][0],))
    assert len(bindings)==1 and bindings[0][0]>0 and bindings[0][1]>0
    failed,t=inspect(jobs[0][0],'recorded','task-2');assert t['status']=='blocked' and t['automatic_validation']['state']=='blocked'
    assert failed['steps'][1][0]['result']['exit_code']==17 and failed['steps'][2][0]['result']['passed']
    assert rows(root,"SELECT count(*) FROM native_validation_jobs WHERE work_id=? AND phase='recorded'",(work,))==[(3,)]
    passed('automatic-leased-failed-command-is-retained-and-later-control-executes-without-acceptance')
    # A separate explicit job has the same valid execution preconditions, then
    # its observations are deliberately abandoned after publication refusal.
    r4,j4,code,wire,target=held('retire-observations',True);assert code!=0
    inspect(j4,'active')
    marker_hashes={f.name:fixture.digest(f.read_bytes()) for f in workspace.glob('native-*.json')}
    parked=workspace.with_name(workspace.name+'.retirement-retained');workspace.rename(parked)
    try:
        retired=ok('retire-original',['task','validation-retire',work],r4)
        assert retired['retired'] and not retired['controls_passed'] and not retired['task_accepted']
        repeated=ok('retire-original-replay',['task','validation-retire',work],r4)
        assert repeated['retired'] and repeated['replayed']
        assert {f.name:fixture.digest(f.read_bytes()) for f in parked.glob('native-*.json')}==marker_hashes
    finally:parked.rename(workspace)
    target.rmdir()
    code,wire=call('retired-job-cannot-run',['task','validate',work],r4)
    assert code!=0 and 'task-validation-original-refused' in json.dumps(wire),wire
    assert rows(root,'SELECT phase FROM native_validation_jobs WHERE id=?',(j4,))==[('refused',)]
    passed('explicit-retirement-preserves-originals-without-current-candidate-or-reexecution')
    proof=dict(status='PASS',assertions=checks,root=str(root),work=work,actual_managed_controls=True,
        recorded_jobs=3,retired_jobs=1,controls_per_job=3,actual_borrowed_lease=True,recovery_without_execution=True,replay_without_candidate=True,
        source_sha256=preview['source_sha256'],candidate_commit=candidate['commit'],private_sql_writes=False,
        accepted=False,independent_review=False,subproject=subproject,global_watchdog=None,
        worker_binary_sha256=setup['binary_sha256'],binary_sha256=fixture.digest(binary.read_bytes()))
    fixture.publish(out/'result.json',proof);print('NATIVE_MANAGED_VALIDATION_PASS',flush=True);return proof


def main():
    """Bind exact normal binary/profile/static helper inputs and retain failures."""
    p=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:p.add_argument('--'+flag,type=Path,required=True)
    for flag in ['before-binary','workers-evidence','worker-binary','resume-from']:p.add_argument('--'+flag,type=Path)
    p.add_argument('--subproject',action='store_true');a=p.parse_args();binary=a.binary.resolve();profile=a.profile.resolve();out=a.output.resolve()
    inputs=dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),profile=str(profile),profile_sha256=fixture.digest(profile.read_bytes()),sources=helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        run(binary,profile,out,before_binary=a.before_binary.resolve() if a.before_binary else None,
            workers_evidence=a.workers_evidence.resolve() if a.workers_evidence else None,
            worker_binary=a.worker_binary.resolve() if a.worker_binary else None,subproject=a.subproject,resume_from=a.resume_from.resolve() if a.resume_from else None)
        assert fixture.digest(binary.read_bytes())==inputs['binary_sha256'] and fixture.digest(profile.read_bytes())==inputs['profile_sha256']
        assert all(fixture.digest(Path(f).read_bytes())==h for f,h in inputs['sources'].items())
        fixture.publish(out/'inputs.json',inputs)
    except BaseException:
        if out.is_dir():fixture.publish(out/'inputs.json',inputs);fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
