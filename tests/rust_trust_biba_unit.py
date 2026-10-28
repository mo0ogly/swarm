#!/usr/bin/env python3
"""Named Rust Biba guards with no global watchdog or fixture-enabled production.

Cargo tests are separate from the shipped executable. Output/count assertions
establish execution, never production-profile identity or independent acceptance.
"""
import json,os,subprocess
from pathlib import Path

def main():
    env=os.environ.copy();env['CARGO_TARGET_DIR']=str(Path('.cache/rust-native-version846-target').resolve());env.update(CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0')
    groups=[('monitor',['-p','swarm-engine','--lib','biba::tests'],8),('identity',['-p','swarm-engine','--lib','trust_types::tests'],5),('proof',['-p','swarm-engine','--lib','trust::proof::tests'],1),('root',['-p','swarm-engine','--test','trust_biba'],14),('http-cookie',['-p','swarm-web','--lib','service::tests::native_cookie_policy_keeps_the_server_snapshot_when_saved_values_change'],1),('http-publication',['-p','swarm-web','--lib','service::tests::root_recorded_state_survives_http_close_and_json_response'],1)]
    for name,args,count in groups:
        argv=['cargo','test','--manifest-path','rust/Cargo.toml',*args,'--','--nocapture'];print('RUN '+name,flush=True)
        result=subprocess.run(argv,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        print(result.stdout.decode(),end='',flush=True)
        import sys
        print(result.stderr.decode(),end='',file=sys.stderr,flush=True)
        assert result.returncode==0 and f'test result: ok. {count} passed;' in result.stdout.decode(),name
        print('PASS BIBA_UNIT_'+name,flush=True)
    print('BIBA_UNIT_PASS 30 named tests in six groups',flush=True)
if __name__=='__main__':main()
