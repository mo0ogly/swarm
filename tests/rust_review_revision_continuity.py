#!/usr/bin/env python3
"""Verify reviewer source continuity after an unrelated public mission checkpoint.

Real public worker/control originals plus one SDK claim and public checkpoint;
no reviewer/model, Go, installation, private SQL injection or global watchdog.
Default compiles current sources; explicit --sdk-source-root binds a retained
immutable input fixture. Failures preserve channels and partial original proofs.
"""
import argparse,json,os,stat,subprocess,sys,traceback
from pathlib import Path
import rust_reviewer_claim_candidate as sdk


def retain(root,out,token,required):
    """Copy only fixed regular fixture originals, then verify hashes/readback.

    The private token prevents accidental Root mixups, not hostile UID forgery.
    Refuse symlinks, missing required originals and copy errors without mutating
    the sources. Partial failed SDK runs may lack originals; their copies never
    constitute success. No result marker or engine receipt is manufactured.
    """
    marker=root/'.swarm/owned-review-claim-fixture'
    if not stat.S_ISREG(marker.lstat().st_mode) or marker.read_text()!=token:
        raise ValueError('owned-revision-marker-changed')
    names=['owned-review-revision-observation.json','owned-review-pause-observation.json']
    names += ['owned-review-'+op+'.'+suffix for op in ['checkpoint','pause','resume'] for suffix in ['input.json','command.json','stdout','stderr']]
    retained={}
    for name in names:
        source=root/name
        try:info=source.lstat()
        except FileNotFoundError:
            if required:raise ValueError('required-revision-original-missing:'+name)
            continue
        if not stat.S_ISREG(info.st_mode) or source.parent.resolve()!=root.resolve():
            raise ValueError('revision-original-not-contained-regular-file')
        data=source.read_bytes();sdk.f.publish(out/name,data)
        if (out/name).read_bytes()!=data:raise ValueError('revision-original-copy-mismatch')
        retained[name]=sdk.f.digest(data)
    sdk.f.publish(out/'original-retention.json',dict(status='COPIED_ORIGINALS',required=required,files=retained,scope='Fixture bytes only, not engine authenticity or acceptance'))
    return retained


def main():
    """Run exactly one named SDK witness, retaining actual profile on test failure.

    Token-check before copying any owned Root observations; never turn a missing
    original or zero collected tests into success. The claim consumes one fixture
    call/reservation, without resetting consumption or publishing a verdict.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--sdk-source-root',type=Path)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    repository=Path(__file__).resolve().parents[1];root=None;token=None;failure=None
    sdk.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=out/'binary';sdk.f.publish(binary,args.binary.resolve().read_bytes());binary.chmod(0o700)
        root,work,request=sdk.inputs.source(binary,out/'source')
        token=os.urandom(32).hex();sdk.f.publish(root/'.swarm/owned-review-claim-fixture',token.encode())
        request['event_id']='owned-review-revision-claim'
        manifest=out/'fixture.json';sdk.f.publish(manifest,dict(root=str(root),work=work,request=request,token=token))
        source=sdk.sdk_source_inputs(repository,args.sdk_source_root)
        sys.path.insert(0,str(repository/'tools/verification'));import verify_native_profile as profile
        env={**os.environ,'CARGO_TARGET_DIR':str((repository/'rust/target/r97').resolve()),'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0','CODEX_HOME':str(root/'codex-home'),'SWARM_OWNED_REVIEW_CLAIM_FIXTURE':str(manifest),'SWARM_OWNED_REVIEW_PUBLIC_BINARY':str(binary)}
        names=['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_TARGET','RUSTUP_TOOLCHAIN','CARGO_HOME']
        profile.require(not any(env.get(k) for k in names),'sdk-ambient-compiler-override')
        before=profile.candidates(source);sdk.f.publish(out/'sdk.sources.before.json',before)
        argv=['cargo','test','--manifest-path',str(source/'rust/Cargo.toml'),'--offline','--locked','-p','swarm-engine','--lib','--message-format=json','reviewer_unrelated_revision_owned_public_fixture','--','--ignored','--nocapture']
        sdk.f.publish(out/'command.json',dict(argv=argv,source=str(source),global_watchdog=None))
        with (out/'sdk.stdout').open('xb') as stdout,(out/'sdk.stderr').open('xb') as stderr:
            actual=subprocess.run(argv,env=env,stdout=stdout,stderr=stderr,stdin=subprocess.DEVNULL)
        sdk.qualify_sdk(source,out,before)
        profile.require(actual.returncode==0,'review-unrelated-revision-test-failed')
        profile.require('1 passed; 0 failed' in (out/'sdk.stdout').read_text() and 'REVIEW_UNRELATED_REVISION_CONTINUITY_CHECKED' in (out/'sdk.stdout').read_text(),'review-revision-named-test-missing')
        observation=json.loads((root/'owned-review-revision-observation.json').read_text())
        profile.require(observation['source_check_ok'] and observation['task_unchanged'] and observation['after_revision']==observation['before_revision']+1 and not observation['provider_started'],'review-revision-proof-invalid')
        pause=json.loads((root/'owned-review-pause-observation.json').read_text())
        profile.require(pause['paused_refused'] and pause['resumed_stale_refused'] and pause['task_unchanged'] and not pause['provider_started'],'review-pause-proof-invalid')
    except BaseException as error:
        failure=error
        try:
            sdk.f.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc(),root=str(root) if root else None))
        except BaseException as evidence_error:
            failure=BaseExceptionGroup('Revision check and failure retention failed',[error,evidence_error])
            raise failure
        raise
    finally:
        try:
            if root is not None and token is not None:
                retain(root,out,token,required=failure is None)
        except BaseException as retention_error:
            if failure is not None:raise BaseExceptionGroup('Revision check and original retention failed',[failure,retention_error])
            raise

    sdk.f.publish(out/'result.json',dict(status='PASS',tests=1,public_pause_and_stale_resume_refused=True,unrelated_checkpoint_usable=True,changed_report_refused_and_restored=True,provider_started=False,claim_recorded=True,financial_settlement=False,global_watchdog=None))
    print('PASS reviewer unrelated revision continuity',flush=True)

if __name__=='__main__':main()
