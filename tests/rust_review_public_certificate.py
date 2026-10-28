#!/usr/bin/env python3
"""Check public certificate byte contracts on source-bound library tests.

Pure wire fixtures only: no Root, provider, process authority, positive engine
receipt, installation or global watchdog. Actual Cargo/profile/source archives
remain attached even when the expected protocol assertion fails.
"""
import argparse,json,os,subprocess,sys,traceback
from pathlib import Path
import rust_reviewer_claim_candidate as sdk


def main():
    """Require both named wire tests and preserve compiler failure plus diagnostics.

    Default uses current sources; explicit immutable --sdk-source-root must match
    its complete manifest. Missing tests/source drift/secondary qualification
    failures never become success. No business state or policy is modified.
    """
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',required=True,type=Path);parser.add_argument('--sdk-source-root',type=Path)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    sdk.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        repository=Path(__file__).resolve().parents[1];source=sdk.sdk_source_inputs(repository,args.sdk_source_root)
        sys.path.insert(0,str(repository/'tools/verification'));import verify_native_profile as profile
        env={**os.environ,'CARGO_TARGET_DIR':str((repository/'rust/target/r97').resolve()),'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0'}
        keys=['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_TARGET','RUSTUP_TOOLCHAIN','CARGO_HOME'];profile.require(not any(env.get(k) for k in keys),'sdk-ambient-compiler-override')
        before=profile.candidates(source);sdk.f.publish(out/'sdk.sources.before.json',before)
        argv=['cargo','test','--manifest-path',str(source/'rust/Cargo.toml'),'--offline','--locked','-p','swarm-engine','--lib','--message-format=json','native_public_certificate_','--','--nocapture']
        sdk.f.publish(out/'command.json',dict(argv=argv,source=str(source),global_watchdog=None))
        with (out/'sdk.stdout').open('xb') as stdout,(out/'sdk.stderr').open('xb') as stderr:actual=subprocess.run(argv,env=env,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
        primary=RuntimeError(f'certificate-wire-test-failed:{actual.returncode}') if actual.returncode else None
        try:sdk.qualify_sdk(source,out,before)
        except BaseException as secondary:
            if primary is not None:raise BaseExceptionGroup('Wire test and source qualification failed',[primary,secondary])
            raise
        if primary is not None:raise primary
        profile.require('2 passed; 0 failed' in (out/'sdk.stdout').read_text(),'certificate-wire-test-not-collected')
        sdk.f.publish(out/'result.json',dict(status='PASS',tests=2,wire_only=True,root=False,engine_receipts=False,global_watchdog=None));print('PASS public certificate wire format',flush=True)
    except BaseException:
        sdk.f.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':main()
