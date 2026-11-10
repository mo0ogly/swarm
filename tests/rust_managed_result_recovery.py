"""Owned filesystem-publication refusal and immutable managed-result recovery.

The Git wrapper preserves the real command, then creates one explicitly owned
empty directory at a future report-file path. This tests a publication refusal,
not ENOSPC or crash durability. No production Root, private SQL or provider call.
"""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import rust_agent_model_route as fixture
import rust_mission_watch_worker as worker


def publication_fault_git(real_git: Path, target: Path, ref: str, marker: Path) -> bytes:
    """Generate an isolated PATH wrapper after a successful exact update-ref.

    Preserve Git argv, environment, channels and status; no timeout or retry.
    Unexpected injection errors fail visibly. Existing targets are never replaced
    or deleted. Caller owns the fixture paths and the explicit later recovery.
    """
    return f'''#!{sys.executable}
import json, os, subprocess, sys
from pathlib import Path
real_git={str(real_git)!r}
target=Path({str(target)!r})
ref={ref!r}
marker=Path({str(marker)!r})
code=subprocess.run([real_git,*sys.argv[1:]]).returncode
args=sys.argv[1:]
if code==0 and len(args)>=4 and args[-4]=='update-ref' and args[-3]==ref and not marker.exists():
    target.parent.mkdir(mode=0o700,parents=True,exist_ok=True)
    target.mkdir(mode=0o700)
    with marker.open('x') as stream:
        json.dump(dict(git_success=True,ref=ref,target=str(target)),stream)
        stream.flush();os.fsync(stream.fileno())
sys.exit(code)
'''.encode()


def install_git_callback(owned, agents, output):
    """Install an executable adversarial filter only in completed fixture copies.

    First execute the owned callback as a positive fixture control, retain that
    witness, then remove its marker before configuring copies. The engine must
    retain exact reports without executing this repository-configured program.
    Only the owned tracked source gets a future fixture mtime to force Git’s
    racy-index rehash; actual report timestamps remain fresh and unchanged.
    No real mission/configuration, private SQL or provider is changed.
    """
    import rust_managed_repository_bootstrap as repository
    output.mkdir(mode=0o700)
    marker=output/'unexpected-callback';program=output/'callback.py'
    fixture.publish(program,('#!'+sys.executable+'\nimport sys,os,json\nfrom pathlib import Path\nPath('+repr(str(marker))+').write_text("callback executed")\nwith Path('+repr(str(output/'callback-invocations.jsonl'))+').open("a") as log: log.write(json.dumps({"cwd":os.getcwd(),"parent":Path("/proc/"+str(os.getppid())+"/cmdline").read_bytes().decode(errors="replace").split("\\u0000")})+"\\n")\nsys.stdout.buffer.write(sys.stdin.buffer.read())\n').encode());program.chmod(0o700)
    control=subprocess.run([str(program)],input=b'positive callback control\n',capture_output=True,check=True)
    assert control.stdout==b'positive callback control\n' and marker.read_text()=='callback executed'
    marker.rename(output/'callback-control-witness')
    for agent in agents:
        workspace=Path(agent['workspace']);attributes=workspace/'.gitattributes'
        assert not attributes.exists(),'fixture already supplies attributes; preserve originals'
        fixture.publish(attributes,b'docs/*.md filter=fixture\ntracked.txt filter=fixture\n')
        repository.git(workspace,'config','filter.fixture.clean',str(program))
        repository.git(workspace,'config','filter.fixture.required','true')
        # Force Git's racy-index path even under different scheduling. This is
        # a fixture file timestamp, never a watchdog, timeout or product policy.
        # Reports retain their actual clock/freshness; only tracked source moves.
        tracked=workspace/'tracked.txt'
        assert tracked.is_file() and not tracked.is_symlink()
        import time
        future=time.time()+24*60*60
        os.utime(tracked,(future,future),follow_symlinks=False)
    return marker


def watch_reports(owned, expected, output, *, environment=None, refusal=False):
    """Run one actual foreground watch; retain its frames and checked shutdown.

    Resume/pause only this fixture through public controls. A refusal must exit
    nonzero itself; successful handoffs stop via SIGINT after all expected IDs.
    Stderr streams directly into an owned evidence file from launch, so verbose
    diagnostics cannot block an unread pipe. Errors and cleanup causes both
    survive. No global watchdog or hidden retry;
    waiting/EOF cannot count as submission, quality or mission acceptance.
    """
    output.mkdir(mode=0o700)
    work=owned.ok(['work','show',owned.work])['work']
    owned.ok(['mission','resume',owned.work],dict(schema_version=1,event_id='resume-'+output.name,
        expected_revision=work['revision'],reason='Exercise owned immutable-result publication and recovery'))
    # A regular evidence file cannot fill an unread stderr pipe while the
    # parent waits for stdout. Preserve large diagnostics from process start.
    diagnostic=(output/'watch.stderr').open('xb')
    try:
        original=(output/'watch.stdout').open('xb')
    except BaseException as primary:
        errors=[];worker.retain_failure(errors,diagnostic.close)
        if errors:raise BaseExceptionGroup('Watch evidence open and close',[primary,*errors])
        raise
    try:
        process=subprocess.Popen([str(owned.binary),'--root',str(owned.root),'--json','mission','watch',owned.work],
            env=environment if environment is not None else owned.environment,stdout=subprocess.PIPE,stderr=diagnostic)
    except BaseException as primary:
        errors=[];worker.retain_failure(errors,original.close);worker.retain_failure(errors,diagnostic.close)
        if errors:raise BaseExceptionGroup('Watch launch and diagnostic close',[primary,*errors])
        raise
    records=[];seen=set();errors=[];primary=None
    try:
        while refusal or not expected.issubset(seen):
            raw=process.stdout.readline()
            if not raw:
                assert refusal,'watch exited before immutable handoffs'
                break
            original.write(raw)
            value=json.loads(raw);records.append(value)
            assert value['event']!='mission-worker-started',value
            if value['event']=='mission-report-submitted':
                assert value['agent_id'] not in seen,value
                seen.add(value['agent_id'])
    except BaseException as error:primary=error
    finally:
        if process.poll() is None:worker.retain_failure(errors,lambda:process.send_signal(signal.SIGINT))
        def drain():
            """Preserve stdout/stderr and verify the expected actual exit kind."""
            out,_=process.communicate()
            original.write(out);original.flush();os.fsync(original.fileno())
            fixture.publish(output/'events.json',records);fixture.publish(output/'tail.stdout',out)
            diagnostic.flush();os.fsync(diagnostic.fileno())
            assert process.returncode!=0 if refusal else process.returncode==0,process.returncode
        worker.retain_failure(errors,drain)
        worker.retain_failure(errors,original.close)
        worker.retain_failure(errors,diagnostic.close)
        def pause():
            """Leave only the owned fixture paused, without modifying its database."""
            work=owned.ok(['work','show',owned.work])['work']
            owned.ok(['mission','pause',owned.work],dict(schema_version=1,event_id='pause-'+output.name,
                expected_revision=work['revision'],reason='Preserve immutable-result fixture and original failures'))
        worker.retain_failure(errors,pause)
    if primary or errors:raise BaseExceptionGroup('Managed watch and cleanup',([primary] if primary else [])+errors)
    return seen


def publication_recovery(owned, agents, output):
    """Refuse one initial artifact publication, then recover the same Git result.

    A previously exercised normal journey is the valid control. The actual Git
    ref must exist before refusal. Remove only our empty injected directory and
    park (preserve) the worker clone; the next public watch must use that same
    commit/report without the copy or a new agent. No ENOSPC claim is made.
    """
    import shutil
    output.mkdir(mode=0o700)
    work=owned.ok(['work','show',owned.work])['work']
    storage=Path(work['planning']['repository']['storage']);agent=agents[0]
    candidate=work['planning']['repository']['candidate_commit']
    original_agents={a['id'] for a in owned.ok(['mission','status-native',owned.work])['agents']}
    assert original_agents=={a['id'] for a in agents}
    target=storage/'results'/agent['id']/'report.md';marker=output/'injected.json'
    ref='refs/swarm/attempts/'+agent['id']
    assert not target.exists() and not marker.exists()
    real_git=shutil.which('git',path=owned.environment.get('PATH'));assert real_git
    wrapper=output/'wrapper';wrapper.mkdir(mode=0o700)
    fixture.publish(wrapper/'git',publication_fault_git(Path(real_git),target,ref,marker));(wrapper/'git').chmod(0o700)
    environment=dict(owned.environment,PATH=str(wrapper)+os.pathsep+owned.environment.get('PATH',os.defpath))
    seen=watch_reports(owned,set(),output/'refusal',environment=environment,refusal=True)
    assert agent['id'] not in seen and json.loads(marker.read_bytes())['git_success'] is True
    assert target.is_dir() and not any(target.iterdir()) and not (target.parent/'result.json').exists()
    raw=(output/'refusal/watch.stderr').read_bytes()
    assert b'managed-result-retention-incomplete' in raw,raw
    command=[real_git,'--git-dir',str(storage/'repository.git')]
    frozen=subprocess.run([*command,'rev-parse','--verify',ref],check=True,capture_output=True).stdout.decode().strip()
    task=next(t for t in owned.ok(['work','show',owned.work])['work']['tasks'] if t['id']==agent['task_id'])
    assert task['status']=='blocked' and len(task['attempts'])==1
    proof=json.loads((storage/('native-worker-'+agent['id']+'.json')).read_bytes())
    clone=Path(proof['path']);parked=clone.with_name(clone.name+'.publication-retained')
    assert not parked.exists()
    target.rmdir();clone.rename(parked)
    primary=None;errors=[]
    try:
        remaining={a['id'] for a in agents}-seen
        recovered=watch_reports(owned,remaining,output/'recovered')
        assert recovered==remaining,recovered
        retained=json.loads((target.parent/'result.json').read_bytes())
        assert retained['commit']==frozen and fixture.digest(target.read_bytes())==retained['report_sha256']
        assert not clone.exists() and parked.is_dir()
    except BaseException as error:primary=error
    finally:
        def restore():
            """Restore only our original; refuse an unexpected replacement path."""
            if clone.exists() or clone.is_symlink():raise FileExistsError('managed-clone-restore-path-occupied')
            parked.rename(clone)
        worker.retain_failure(errors,restore)
    if primary or errors:raise BaseExceptionGroup('Managed recovery and original restoration',([primary] if primary else [])+errors)
    after=owned.ok(['work','show',owned.work])['work']
    assert after['planning']['repository']['candidate_commit']==candidate
    assert {a['id'] for a in owned.ok(['mission','status-native',owned.work])['agents']}==original_agents
    fixture.publish(output/'result.json',dict(status='PASS',commit=frozen,agent=agent['id'],
        fault='owned-empty-directory-publication-refusal',copy_absent_on_recovery=True,accepted=False))
    print('PASS managed-result-publication-refusal-retains-ref-and-recovers-without-copy',flush=True)
