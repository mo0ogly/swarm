#!/usr/bin/env python3
"""Verify committed-only managed bootstrap through the real public Rust CLI.

Owned Git source includes uncommitted changes deliberately excluded from the
copy. Replay precedes source IO. An owned negative file-publication fault retains the
prepared copy without publishing Work; malformed/divergent proofs refuse, then
restoration recovers the pending original. No provider, acceptance, live database
or global watchdog. Fixture commits use the checkout's configured Git identity.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import traceback
import rust_agent_model_route as fixture
from rust_planning_bootstrap import fresh_work,request
import rust_native_http_cli as native


def git(root,*args):
    """Run owned fixture Git commands, retaining failure bytes in CalledProcessError.

    No shell or verification deadline; test commits use configured identity passed
    by the caller. This helper is never used against the user's working checkout
    except for read-only configuration discovery.
    """
    environment={k:v for k,v in os.environ.items() if not k.startswith('GIT_')}
    return subprocess.run(['git','--no-pager','-C',str(root),*args],env=environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True).stdout


def inventory(root):
    """Hash actual regular source files, including Git index/HEAD/config, without writes."""
    return {str(p.relative_to(root)):fixture.digest(p.read_bytes()) for p in sorted(root.rglob('*')) if p.is_file() and not p.is_symlink()}


def run(binary,profile,out):
    """Require copy/replay/refusal/recovery assertions on fresh public Works.

    Source and manifests are preserved on failure. Direct writes only inject
    negative proof/publication faults into this fixture, restored before public recovery.
    Output is not model execution, immutable storage authentication or acceptance.
    """
    out.mkdir(parents=True,mode=0o700,exist_ok=False)
    qualified=json.loads(profile.read_bytes());assert qualified['no_fixture_feature'] and fixture.digest(binary.read_bytes())==qualified['sha256']
    f=fixture.Fixture(binary,out/'setup','managed-bootstrap')
    fixture.publish(out/'fixture.json',dict(root=str(f.root),binary_sha256=qualified['sha256']))
    source=f.root/'source';source.mkdir(mode=0o700)
    git(source,'init','--quiet')
    checkout=Path(__file__).resolve().parents[1]
    name=git(checkout,'config','user.name').decode().strip();email=git(checkout,'config','user.email').decode().strip()
    assert name and email,'configured Git identity required'
    fixture.publish(source/'tracked.txt',b'committed fixture bytes\n');git(source,'add','tracked.txt')
    git(source,'-c','user.name='+name,'-c','user.email='+email,'-c','core.hooksPath=/dev/null','-c','commit.gpgsign=false','commit','--quiet','-m','Owned committed bootstrap fixture')
    base=git(source,'rev-parse','HEAD').decode().strip()
    fixture.publish(source/'tracked.txt',b'PRIVATE uncommitted bytes excluded\n');fixture.publish(source/'untracked.txt',b'PRIVATE untracked excluded\n')
    before=inventory(source);checks=[]
    def passed(label):
        """Emit and retain a named assertion only after observing all its conditions."""
        checks.append(label);print('PASS '+label,flush=True)
    value=request(event='managed-config',repository=dict(path='source',committed_only=True))
    f.work=fresh_work(f,'managed-create')
    configured=f.ok(['planning','enable',f.work],value)
    repo=configured['planning']['repository'];storage=Path(repo['storage']);bare=storage/'repository.git'
    assert repo['base_commit']==repo['candidate_commit']==base
    assert git(bare,'show',base+':tracked.txt')==b'committed fixture bytes\n'
    assert git(bare,'ls-tree','--name-only',base).decode().splitlines()==['tracked.txt']
    assert inventory(source)==before
    assert (storage/'repository.json').read_bytes()==(storage/'repository.json.pending').read_bytes()
    assert f.ok(['work','show',f.work])['work']['planning']['repository']==repo
    assert not (f.root/'model-received.json').exists()
    passed('managed-committed-copy-excludes-dirty-and-preserves-source')
    source.rename(f.root/'source-hidden')
    try:assert f.ok(['planning','enable',f.work],value)==configured
    finally:(f.root/'source-hidden').rename(source)
    passed('managed-recorded-replay-precedes-source-io')
    f.work=fresh_work(f,'managed-dirty-refusal');state=f.snapshot()
    code,response=f.call(['planning','enable',f.work],request(event='unsupported-snapshot',repository=dict(path='source',committed_only=True,include_dirty=['tracked.txt'])))
    assert code!=0 and 'managed-dirty-snapshot-unported' in json.dumps(response)
    assert f.snapshot()==state and inventory(source)==before
    passed('managed-dirty-snapshot-refuses-without-work-mutation')
    f.work=fresh_work(f,'managed-recovery-create');value=request(event='managed-recovery',repository=dict(path='source',committed_only=True,include_dirty=[]))
    state=f.snapshot()
    storage=f.root/'.swarm/managed'/f.work;manifest=storage/'repository.json';pending=storage/'repository.json.pending'
    # Real Git runs unchanged; only its last successful setup command creates an
    # owned empty directory that denies final manifest publication after bare move.
    real_git=shutil.which('git');assert real_git
    marker=out/'publication-fault-reached.json'
    program="""#!/usr/bin/python3
import json,subprocess,sys
from pathlib import Path
code=subprocess.run([REAL_GIT,*sys.argv[1:]]).returncode
if code==0 and sys.argv[-3:]==['symbolic-ref','HEAD','refs/heads/swarm-result']:
    Path(TARGET).mkdir(mode=0o700)
    Path(MARKER).write_text(json.dumps({'actual_git_succeeded':True,'target':TARGET}))
sys.exit(code)
""".replace('REAL_GIT',repr(real_git)).replace('TARGET',repr(str(manifest))).replace('MARKER',repr(str(marker)))
    fixture.publish(f.root/'git',program.encode());(f.root/'git').chmod(0o700)
    previous_path=f.environment.get('PATH')
    f.environment['PATH']=str(f.root)+os.pathsep+(previous_path or os.defpath)
    try:
        code,response=f.call(['planning','enable',f.work],value)
        fixture.publish(out/'publication-refusal.json',dict(exit=code,response=response))
        assert code!=0 and 'managed-manifest-publish' in json.dumps(response)
        assert marker.is_file() and f.snapshot()==state and manifest.is_dir()
        assert (storage/'repository.git').is_dir()
    finally:
        if previous_path is None:f.environment.pop('PATH',None)
        else:f.environment['PATH']=previous_path
        (f.root/'git').unlink()
        if manifest.is_dir():manifest.rmdir()
    original=pending.read_bytes()
    fixture.publish(out/'retained-manifest.json',original)
    for label,raw in [('malformed',b'PRIVATE malformed'),('binding',json.dumps(dict(json.loads(original),source='PRIVATE-outside')).encode())]:
        fixture.publish(manifest,raw)
        try:
            code,response=f.call(['planning','enable',f.work],value)
            fixture.publish(out/(label+'-refusal.json'),dict(exit=code,response=response))
            assert code!=0 and f.snapshot()==state and manifest.read_bytes()==raw and pending.read_bytes()==original
            assert 'PRIVATE' not in json.dumps(response)
        finally:fixture.publish(manifest,original)
    passed('managed-malformed-and-divergent-originals-refuse-unchanged')
    manifest.unlink()
    for label,patch in [('binding',dict(source='PRIVATE-outside')),('commit',dict(base_commit='0'*40,candidate_commit='0'*40))]:
        changed=json.dumps(dict(json.loads(original),**patch)).encode();fixture.publish(pending,changed)
        try:
            code,response=f.call(['planning','enable',f.work],value)
            fixture.publish(out/(label+'-pending-refusal.json'),dict(exit=code,response=response))
            assert code!=0 and f.snapshot()==state and pending.read_bytes()==changed
            assert not manifest.exists() and 'PRIVATE' not in json.dumps(response)
        finally:fixture.publish(pending,original)
    pending.unlink()
    try:
        code,response=f.call(['planning','enable',f.work],value)
        assert code!=0 and 'managed-orphan-repository' in json.dumps(response) and f.snapshot()==state
    finally:fixture.publish(pending,original)
    bare=storage/'repository.git';bare.rename(storage/'repository.retained')
    try:
        code,response=f.call(['planning','enable',f.work],value)
        assert code!=0 and 'managed-repository-open' in json.dumps(response) and f.snapshot()==state
        assert pending.read_bytes()==original and not bare.exists()
    finally:(storage/'repository.retained').rename(bare)
    passed('managed-binding-missing-object-and-orphans-refuse-with-originals')
    recovered=f.ok(['planning','enable',f.work],value)
    assert recovered['planning']['repository']==json.loads(original)
    assert manifest.read_bytes()==original and pending.read_bytes()==original
    assert inventory(source)==before
    assert not (f.root/'model-received.json').exists()
    passed('managed-pending-original-recovers-after-publication-refusal')
    # A subproject uses the same top-level immutable commit, through protected HTTP.
    (source/'subproject').mkdir(mode=0o700)
    f.work=fresh_work(f,'managed-http-create');server=None;primary=None
    try:
        server=native.Server(binary,f.root,out,'managed-http',environment=f.environment)
        csrf=server.request('/api/v1/native-session')['csrf_token']
        url='/api/v1/planning?work='+f.work
        http_work=server.request(url+'&action=enable',request(event='managed-http-enable',repository=dict(path='source/subproject',committed_only=True)),csrf)
        repo=http_work['planning']['repository']
        assert repo['project_subdir']=='subproject' and repo['base_commit']==repo['candidate_commit']==base
        assert server.request(url)==f.ok(['work','show',f.work])['work']
        assert inventory(source)==before and not (f.root/'model-received.json').exists()
        server.stop()
    except BaseException as error:primary=error
    finally:
        if server is not None:native.finish(server,primary)
        elif primary is not None:raise primary
    passed('managed-protected-http-subproject-matches-cli')
    result=dict(status='PASS',assertions=checks,binary_sha256=qualified['sha256'],paid_provider=False,accepted=False,installed=False,global_watchdog=None)
    fixture.publish(out/'result.json',result);print('MANAGED_REPOSITORY_BOOTSTRAP_PASS',flush=True)
    return result


def main():
    """Bind explicit normal binary/profile and retain original failures in fresh output."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args()
    try:run(args.binary.resolve(),args.profile.resolve(),args.output.resolve())
    except BaseException as error:
        if args.output.is_dir():fixture.publish(args.output/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(error))))
        raise


if __name__=='__main__':main()
