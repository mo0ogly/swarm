#!/usr/bin/env python3
"""Run five existing financial start guards against qualified normal library bytes.

No process/provider/Root business fixture or global watchdog. This integration
suite uses its original in-memory financial fixture and actual reserve function.
The normal engine library and test executable are distinguished by Cargo facts.
"""
import argparse,json,os,subprocess,sys,traceback
from pathlib import Path
import rust_reviewer_claim_candidate as sdk


def main():
    """Require all five named start cases, unchanged compiler inputs and archive.

    Explicit immutable source selection includes the integration source. Compiler
    and source qualification failures remain separate causes; a zero/missing test
    count or fixture features never qualify as success. No paid/provider action.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--sdk-source-root',type=Path);p.add_argument('--output',required=True,type=Path);args=p.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False);sdk.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        repository=Path(__file__).resolve().parents[1];source=sdk.sdk_source_inputs(repository,args.sdk_source_root);sys.path.insert(0,str(repository/'tools/verification'));import verify_native_profile as profile
        before=profile.candidates(source);test='rust/crates/swarm-engine/tests/financial_budget.rs';before[test]=sdk.f.digest((source/test).read_bytes());sdk.f.publish(out/'sdk.sources.before.json',before)
        env={**os.environ,'CARGO_TARGET_DIR':str((repository/'rust/target/r97').resolve()),'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0'}
        profile.require(not any(env.get(k) for k in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_TARGET','RUSTUP_TOOLCHAIN','CARGO_HOME']),'sdk-ambient-compiler-override')
        argv=['cargo','test','--manifest-path',str(source/'rust/Cargo.toml'),'--offline','--locked','-p','swarm-engine','--test','financial_budget','--message-format=json','reserved_planning_start_','--','--nocapture'];sdk.f.publish(out/'command.json',dict(argv=argv,source=str(source),global_watchdog=None))
        with (out/'sdk.stdout').open('xb') as stdout,(out/'sdk.stderr').open('xb') as stderr:r=subprocess.run(argv,env=env,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
        primary=RuntimeError('financial-start-tests-failed:'+str(r.returncode)) if r.returncode else None
        try:sdk.qualify_sdk(source,out,before,test_target='financial_budget')
        except BaseException as secondary:
            if primary is not None:raise BaseExceptionGroup('Financial test and qualification failed',[primary,secondary])
            raise
        if primary is not None:raise primary
        text=(out/'sdk.stdout').read_text();profile.require('5 passed; 0 failed' in text and text.count('test reserved_planning_start_')==5,'financial-start-named-count-invalid')
        sdk.f.publish(out/'result.json',dict(status='PASS',named_tests=5,test_target='financial_budget',normal_engine_library=True,provider_execs=0,global_watchdog=None));print('PASS five qualified financial start guards',flush=True)
    except BaseException:
        sdk.f.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':main()
