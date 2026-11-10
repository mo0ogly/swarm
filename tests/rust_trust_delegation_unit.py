#!/usr/bin/env python3
"""Run the named delegation regressions, separate from production profile.

No global watchdog or provider. Raw output/counts establish execution only;
the production artifact requires its separate public recipe. The nine
integration tests own isolated roots under the repository fixture area
(`.swarm-fixtures/trust-delegation/`), never `/tmp`; a failed assertion
retains its root for diagnosis. The two `mandate_status_projection_*`
regressions pin the typed-error propagation of the operator projection
(stored timestamp / stored delegation depth never collapse to a label).
"""
import os,subprocess,sys
from pathlib import Path

def main():
    """Retain Cargo output and require every selected target test to pass."""
    env=os.environ.copy();env['CARGO_TARGET_DIR']=str(Path('.cache/rust-trust-delegation-unit-target').resolve());env.update(CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0')
    argv=['cargo','test','--manifest-path','rust/Cargo.toml','-p','swarm-engine','--test','trust_delegation','--','--nocapture']
    r=subprocess.run(argv,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE);sys.stdout.buffer.write(r.stdout);sys.stderr.buffer.write(r.stderr)
    names=['delegation_config_admission_persists_and_rejects_bad_shapes','delegation_depth_zero_and_authority_boundary','delegation_chain_request_approve_commit_and_depth_ceiling','delegation_ancestor_revocation_blocks_boundaries','delegation_rejects_amplification','delegation_default_expiry_bounded_by_parent','delegation_projection_reports_status','mandate_status_projection_propagates_stored_timestamp_error','mandate_status_projection_propagates_stored_depth_error']
    text=r.stdout.decode();assert r.returncode==0 and 'test result: ok. 9 passed;' in text
    for name in names:assert 'test '+name+' ... ok' in text,name
    print('DELEGATION_UNIT_PASS 9 named tests',flush=True)
if __name__=='__main__':main()
