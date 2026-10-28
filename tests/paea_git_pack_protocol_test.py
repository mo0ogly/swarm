#!/usr/bin/env python3
"""Qualify real Git pack streaming without a protected reference update.

Explicit owned native cumulative fixture + qualified normal CLI/profile required.
Export writes directly to an owned evidence file; no in-memory pack capture.
All refusals follow a valid control; originals and failure inventories retained.
This script is a protocol experiment, not an implemented effect service.
"""
import argparse,hashlib,json,os,shutil,subprocess,traceback
from pathlib import Path
from rust_native_task_validation import invoke
from rust_native_managed_reviewer import inspection


def sha(path):
    """Hash a file incrementally; never read a whole pack into memory."""
    h=hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda:stream.read(65536),b''):h.update(block)
    return h.hexdigest()


def run(binary,profile,fixture,out):
    """Exercise actual pack/commit identity, no-ref import and refusal/recovery.

    Export candidate is from the real normal native reader, never a positive SQL
    seed. Bare destination directories are newly owned fixtures. Failed imports
    may write object files: retain their exact inventory; no rollback is inferred.
    The script runs no business grant, private-UID or independent acceptance flow.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    f=inspection.native.fixture
    cfg=json.loads(profile.read_bytes())
    assert cfg['no_fixture_feature'] and cfg['all_core_features_empty'] and cfg['all_core_test_false']
    assert cfg['sha256']==sha(binary)
    source=json.loads((fixture/'result.json').read_bytes())
    assert source['status']=='PASS' and source['cumulative_publication'] and not source['paid_provider']
    root=Path(source['root']);work=source['work']
    assert any(json.loads(p.read_bytes()).get('root')==str(root) for p in fixture.rglob('owned-root-*.json'))
    code,value=invoke(binary,root,out,'actual-current-work',['work','show',work],environment=dict(os.environ,CODEX_HOME=str(root/'codex-home')))
    assert code==0,value
    before=value['work'];repo=before['planning']['repository'];candidate=repo['candidate_commit'];assert candidate==source['candidate_commit']
    origin=Path(repo['storage'])/'repository.git';assert origin.is_dir()
    git=Path(shutil.which('git')).resolve();git_sha=sha(git);version=subprocess.run([str(git),'--version'],capture_output=True,check=True).stdout.decode().strip()
    environment={k:v for k,v in os.environ.items() if not k.startswith('GIT_')}
    environment.update(GIT_CONFIG_NOSYSTEM='1',GIT_CONFIG_GLOBAL='/dev/null',GIT_TERMINAL_PROMPT='0')
    prefix=[str(git),'--no-pager','--no-optional-locks','-c','core.hooksPath=/dev/null','-c','core.fsmonitor=false','-c','core.fsync=all','-c','init.templateDir=']
    checks=[]
    def command(label,cwd,args,stdin=None,stdout=None):
        """Retain argv, actual channels/status and hashes; no timeout or shell."""
        argv=prefix+['--git-dir','.']+args
        f.publish(out/(label+'.command.json'),dict(argv=argv,cwd=str(cwd),global_watchdog=None))
        result=subprocess.run(argv,cwd=cwd,env=environment,stdin=stdin,stdout=stdout if stdout is not None else subprocess.PIPE,stderr=subprocess.PIPE)
        if stdout is None:f.publish(out/(label+'.stdout'),result.stdout)
        f.publish(out/(label+'.stderr'),result.stderr)
        f.publish(out/(label+'.exit.json'),dict(exit=result.returncode))
        return result
    def bare(label,format='sha1'):
        """Create only a private fresh Git fixture in the evidence directory."""
        path=out/label;path.mkdir(mode=0o700)
        r=subprocess.run(prefix+['init','--bare','--object-format='+format,'.'],cwd=path,env=environment,capture_output=True)
        f.publish(out/(label+'-init.stdout'),r.stdout);f.publish(out/(label+'-init.stderr'),r.stderr);assert r.returncode==0
        return path
    def refs(label,path):
        """Observe full ref content; never replace a failed read by empty refs."""
        r=command(label,path,['for-each-ref','--format=%(refname) %(objectname)']);assert r.returncode==0
        return r.stdout
    def inventory(path):
        """Retain actual regular-file hashes/modes after partial import failures."""
        result={}
        for file in sorted(path.rglob('*')):
            if file.is_file() and not file.is_symlink():result[str(file.relative_to(path))]=dict(bytes=file.stat().st_size,sha256=sha(file),mode=oct(file.stat().st_mode&0o777))
        return result
    def imported(label,path,pack):
        """Give the child its own stdin file; inspect actual status and artifacts."""
        with pack.open('rb') as stream:r=command(label,path,['index-pack','--stdin','--fix-thin'],stdin=stream)
        f.publish(out/(label+'-inventory.json'),inventory(path));return r
    def present(label,path,oid):
        """Check exact commit presence; an error means absence only in this fixture."""
        r=command(label,path,['cat-file','-t',oid]);return r.returncode==0 and r.stdout==b'commit\n'
    pack=out/'candidate.pack';oid_input=out/'candidate.oid';f.publish(oid_input,(candidate+'\n').encode())
    with oid_input.open('rb') as input_stream,pack.open('xb') as output:
        r=command('valid-export',origin,['pack-objects','--revs','--stdout'],stdin=input_stream,stdout=output)
        output.flush();os.fsync(output.fileno())
    assert r.returncode==0 and pack.stat().st_size>0
    pack_sha=sha(pack);dest=bare('compatible');initial=refs('initial-refs',dest);assert initial==b''
    assert imported('valid-import',dest,pack).returncode==0;assert present('actual-commit',dest,candidate);assert refs('after-import-refs',dest)==initial
    verified=command('connectivity',dest,['fsck','--connectivity-only',candidate]);assert verified.returncode==0
    checks.append('disk-streamed-pack-imports-actual-candidate-with-no-refs')
    # Operator-only fixture seed; the import itself must not move this target.
    seeded=command('seed-existing-target',dest,['update-ref','refs/heads/release',repo['base_commit']]);assert seeded.returncode==0
    before_refs=refs('seeded-refs',dest);assert imported('repeat-import',dest,pack).returncode==0
    assert refs('repeat-refs',dest)==before_refs and present('repeat-commit',dest,candidate)
    checks.append('repeat-import-preserves-existing-protected-target')
    incompatible=bare('incompatible','sha256');assert imported('mismatch-import',incompatible,pack).returncode!=0;assert refs('mismatch-refs',incompatible)==b''
    assert imported('mismatch-recovery-control',dest,pack).returncode==0;checks.append('object-format-mismatch-refuses-with-failure-inventory')
    for name,offset in [('truncated',None),('corrupt',20)]:
        # Applicable valid control immediately precedes each actual pack fault.
        assert imported(name+'-control',dest,pack).returncode==0
        damaged=out/(name+'.pack')
        with pack.open('rb') as src,damaged.open('xb') as dst:shutil.copyfileobj(src,dst)
        with damaged.open('r+b') as stream:
            if offset is None:stream.truncate(pack.stat().st_size-1)
            else:stream.seek(offset);b=stream.read(1);stream.seek(offset);stream.write(bytes([b[0]^1]))
            stream.flush();os.fsync(stream.fileno())
        target=bare(name+'-target');assert imported(name+'-import',target,damaged).returncode!=0;assert refs(name+'-refs',target)==b''
        assert imported(name+'-recovery',target,pack).returncode==0;assert present(name+'-recovered-commit',target,candidate)
        checks.append(name+'-pack-refuses-and-valid-pack-recovers')
    missing=out/'missing.oid';f.publish(missing,('3'*len(candidate)+'\n').encode());partial=out/'missing-output.pack'
    with missing.open('rb') as stream,partial.open('xb') as output:r=command('missing-oid-export',origin,['pack-objects','--revs','--stdout'],stdin=stream,stdout=output)
    assert r.returncode!=0;checks.append('missing-oid-export-refuses-with-partial-output-retained')
    assert sha(pack)==pack_sha and sha(git)==git_sha
    code,value=invoke(binary,root,out,'source-work-after',['work','show',work],environment=dict(os.environ,CODEX_HOME=str(root/'codex-home')));assert code==0 and value['work']==before
    f.publish(out/'result.json',dict(status='PASS',checks=checks,pack_sha256=pack_sha,pack_bytes=pack.stat().st_size,candidate=candidate,git_sha256=git_sha,git_version=version,source_work_unchanged=True,fixture_only=True,native_transfer_implemented=False,current_authority=False,global_watchdog=None))
    print('PAEA_GIT_PACK_PROTOCOL_PASS',len(checks),'named assertions',flush=True)


def main():
    """Require explicit paths, bind executable/helper bytes, preserve failures."""
    parser=argparse.ArgumentParser(description=__doc__)
    keys={'binary':'SWARM_PAEA_PACK_CLI','profile':'SWARM_PAEA_PACK_PROFILE','fixture':'SWARM_PAEA_PACK_FIXTURE','output':'SWARM_PAEA_PACK_EVIDENCE'}
    for name,key in keys.items():
        if key in os.environ and not os.environ[key]:parser.error(key+' must be nonempty')
        parser.add_argument('--'+name,type=Path,default=os.environ.get(key),required=key not in os.environ)
    a=parser.parse_args();f=inspection.native.fixture;inputs=dict(binary_sha256=sha(a.binary),profile_sha256=sha(a.profile),helpers=inspection.native.helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        run(a.binary.resolve(),a.profile.resolve(),a.fixture.resolve(),a.output.resolve())
        assert sha(a.binary)==inputs['binary_sha256'] and sha(a.profile)==inputs['profile_sha256'] and all(sha(Path(k))==v for k,v in inputs['helpers'].items())
        f.publish(a.output/'inputs.json',inputs)
    except BaseException:
        if a.output.exists():f.publish(a.output/'inputs.json',inputs);f.publish(a.output/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
