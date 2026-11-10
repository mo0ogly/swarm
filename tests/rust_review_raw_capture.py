#!/usr/bin/env python3
"""Replay nine private raw-capture and streaming guards on source-bound SDK code.

No provider, Go, database, acceptance, deadline/watchdog or installation. Default
uses the current checkout; --sdk-source-root requires an explicit retained source
fixture. SDK qualification and archive checks preserve actual test profile/input
scope; a concurrent used-source change refuses instead of granting a PASS.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import traceback
import rust_reviewer_claim_candidate as sdk


def main():
    """Publish exclusive evidence for actual named tests and their compiled inputs.

    Require compiler defaults without ambient overrides, prebind source bytes,
    assert nine selected tests ran, then qualify actual depfiles/features/archive.
    IO/build/test/qualification failures retain original channels and failure
    evidence. Output must not already exist; no original proof is overwritten.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--sdk-source-root',type=Path)
    args=parser.parse_args()
    repository=Path(__file__).resolve().parents[1]
    sys.path.insert(0,str(repository/'tools/verification'))
    import verify_native_profile as profile
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    try:
        source=sdk.sdk_source_inputs(repository,args.sdk_source_root)
        before=profile.candidates(source);sdk.f.publish(out/'sdk.sources.before.json',before)
        target=Path(os.environ.get('CARGO_TARGET_DIR',str(repository/'rust/target/r97'))).resolve()
        env={**os.environ,'CARGO_TARGET_DIR':str(target),'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0'}
        names=['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_TARGET','RUSTUP_TOOLCHAIN','CARGO_HOME']
        profile.require(not any(env.get(k) for k in names),'sdk-ambient-compiler-override')
        argv=['cargo','test','--manifest-path',str(source/'rust/Cargo.toml'),'--offline','--locked','-p','swarm-engine','--lib','--message-format=json','reviewer_raw_capture_','--','--nocapture']
        sdk.f.publish(out/'command.json',dict(argv=argv,source_root=str(source),global_watchdog=None))
        sdk.f.publish(out/'method.py',Path(__file__).read_bytes())
        with (out/'sdk.stdout').open('xb') as stdout,(out/'sdk.stderr').open('xb') as stderr:
            result=subprocess.run(argv,env=env,stdout=stdout,stderr=stderr,stdin=subprocess.DEVNULL)
        profile.require(result.returncode==0,'raw-capture-tests-failed')
        profile.require('9 passed; 0 failed' in (out/'sdk.stdout').read_text(),'raw-capture-tests-not-collected')
        sdk.qualify_sdk(source,out,before)
        sdk.f.publish(out/'result.json',dict(status='PASS',tests=9,shared_raw_policy=True,truncation_preserves_reply=True,capture_failure_invalidates_reply=True,parser_failure_retains_raw=True,source_root=str(source),production_wiring=False,global_watchdog=None))
        print('PASS raw capture streaming 9 tests',flush=True)
    except BaseException:
        sdk.f.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise


if __name__=='__main__':
    main()
