#!/usr/bin/env python3
"""Build an explicit native candidate with observed Git facts, without Go/install.

Root and Cargo target must be owned and stable. The default launcher is unchanged.
Git HEAD/status are observations before/after compilation, not atomic source or
actor authentication. Effective compiler inputs are independently archived by the
native profile collector. No retry, network, global timeout, restart or publication.
"""
from pathlib import Path
import argparse,datetime,hashlib,importlib.util,json,os,re,subprocess,sys

spec=importlib.util.spec_from_file_location('native_profile',Path(__file__).resolve().parents[1]/'verification/verify_native_profile.py')
profile=importlib.util.module_from_spec(spec);spec.loader.exec_module(profile)


def release(value):
    """Validate legacy v-prefixed SemVer; devel is the explicit untagged label.

    This validates syntax only and never invents a published release or Git tag.
    """
    if value=='devel':return True
    if not re.fullmatch(r'v(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?',value):return False
    before=value.split('+',1)[0]
    return '-' not in before or all(not (len(s)>1 and s.startswith('0') and s.isdigit()) for s in before.split('-',1)[1].split('.'))


def git(root,out,name,arguments):
    """Run one fixed local Git observation; retain original channels on failure.

    External filters/hooks/fsmonitor/global config and optional index writes are
    disabled. No caller command interpolation, remote access or timeout is added.
    Private paths in errors stay in owned proof files, never public exceptions.
    """
    args=['git','--no-optional-locks','-c','core.hooksPath=/dev/null','-c','core.fsmonitor=false','-c','core.untrackedCache=false','-C',str(root),*arguments]
    env={**os.environ,'GIT_CONFIG_NOSYSTEM':'1','GIT_CONFIG_GLOBAL':os.devnull}
    # Ambient Git routing can override -C. Refuse rather than label another tree.
    forbidden=[key for key in env if key in {'GIT_DIR','GIT_WORK_TREE','GIT_COMMON_DIR','GIT_INDEX_FILE','GIT_OBJECT_DIRECTORY','GIT_ALTERNATE_OBJECT_DIRECTORIES','GIT_CONFIG_COUNT','GIT_CONFIG_PARAMETERS'}]
    profile.require(not forbidden,'native-build-ambient-git-routing')
    result=subprocess.run(args,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    profile.publish(out/(name+'.stdout'),result.stdout);profile.publish(out/(name+'.stderr'),result.stderr)
    profile.require(result.returncode==0,'native-build-git-'+name+'-failed')
    return result.stdout


def observe(root,out,prefix):
    """Observe exact HEAD and status hash; names stay out of the public manifest.

    SHA-1 is the legacy wire contract, not a security authentication claim. A
    dirty flag covers Git-observed changes, not ignored files or all build inputs.
    """
    top=git(root,out,prefix+'-root',['rev-parse','--show-toplevel']).decode('utf-8').strip()
    profile.require(Path(top).resolve()==root,'native-build-root-is-not-git-toplevel')
    head=git(root,out,prefix+'-head',['rev-parse','--verify','HEAD']).decode('ascii').strip()
    profile.require(re.fullmatch('[0-9a-f]{40}',head) is not None,'native-build-unsupported-git-object')
    status=git(root,out,prefix+'-status',['status','--porcelain=v1','--untracked-files=normal'])
    return dict(commit=head,modified=bool(status),status_sha256=hashlib.sha256(status).hexdigest())


def build(root,out,target,version):
    """Freeze a qualified default-profile candidate and verify consumed metadata.

    Explicit version is only a label. Actual Git/date facts cannot be overridden
    by ambient metadata. A failed build/probe/stability check retains evidence and
    leaves an uninstalled, unqualified candidate; no rollback is inferred. Output
    must be fresh and target exclusive. Only the immutable frozen executable is
    exercised. Runtime env forgery is a fixture, not an injected build declaration.
    """
    profile.require(release(version),'native-build-version-invalid')
    profile.require(target.is_relative_to(root),'native-build-target-outside-root')
    out.mkdir(parents=False,exist_ok=False,mode=0o700)
    profile.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        before=observe(root,out,'before')
        metadata=dict(schema_version=1,version=version,commit=before['commit'],modified=before['modified'],build_date=datetime.datetime.now(datetime.timezone.utc).isoformat(timespec='seconds').replace('+00:00','Z'))
        raw=json.dumps(metadata,sort_keys=True,separators=(',',':'))
        profile.publish(out/'build-metadata.json',raw+'\n')
        env={**os.environ,'SWARM_NATIVE_BUILD_METADATA':raw}
        args=[sys.executable,str(Path(profile.__file__)), '--root',str(root),'--target-dir',str(target),'--output',str(out/'profile')]
        profile.publish(out/'invocation.json',json.dumps(dict(argv=args,root=str(root),metadata_sha256=hashlib.sha256(raw.encode()).hexdigest(),global_watchdog=None),indent=2)+'\n')
        with (out/'collector.stdout').open('xb') as stdout,(out/'collector.stderr').open('xb') as stderr:
            result=subprocess.run(args,env=env,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
        profile.require(result.returncode==0,'native-build-profile-failed')
        qualified=json.loads((out/'profile/result.json').read_bytes())
        profile.require(qualified['native_build_metadata']==dict(present=True,sha256=hashlib.sha256(raw.encode()).hexdigest()),'native-build-metadata-binding')
        binary=root/qualified['binary'];profile.require(profile.digest(binary)==qualified['sha256'],'native-build-frozen-binary-mismatch')
        cold=out/'cold';cold.mkdir()
        forged={**env,'SWARM_NATIVE_BUILD_METADATA':'PRIVATE_RUNTIME_FORGERY'}
        command=[str(binary),'version','--json','--root',str(cold/'absent-root')]
        reply=subprocess.run(command,cwd=cold,env=forged,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        profile.publish(out/'version.stdout',reply.stdout);profile.publish(out/'version.stderr',reply.stderr)
        profile.require(reply.returncode==0,'native-build-version-failed')
        actual=json.loads(reply.stdout);expected={key:value for key,value in metadata.items() if key!='schema_version'};expected['provenance']='injected'
        profile.require(actual['schema_version']==1 and actual['binary']==expected,'native-build-consumed-metadata-mismatch')
        profile.require(not any(cold.iterdir()),'native-build-cold-root-mutated')
        after=observe(root,out,'after');profile.require(before==after,'native-build-git-observation-changed')
        proof=dict(status='VERIFIED_NATIVE_CANDIDATE',binary=str(binary),sha256=qualified['sha256'],metadata=metadata,metadata_sha256=hashlib.sha256(raw.encode()).hexdigest(),git_observations_stable=True,profile=str(out/'profile/result.json'),runtime_forgery_ignored=True,cold_root_not_created=True,installed=False,published=False,global_watchdog=None)
        profile.publish(out/'result.json',json.dumps(proof,indent=2)+'\n');return proof
    except BaseException as error:
        try:profile.publish(out/'failure.json',json.dumps(dict(status='FAIL',type=type(error).__name__,candidate_unqualified=True,installed=False))+'\n')
        except BaseException as secondary:raise BaseExceptionGroup('Native build and evidence publication failed',[error,secondary])
        raise


def main():
    """Build only explicitly selected source/output/target, defaulting to devel."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--root',type=Path,default=Path.cwd());p.add_argument('--output',type=Path,required=True);p.add_argument('--target-dir',type=Path,required=True);p.add_argument('--version',default='devel');a=p.parse_args()
    print(json.dumps(build(a.root.resolve(),a.output.resolve(),a.target_dir.resolve(),a.version)),flush=True)


if __name__=='__main__':main()
