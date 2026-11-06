#!/usr/bin/env python3
"""Run the named data-provenance regressions, separate from production profile.

No global watchdog or provider. Raw output/counts establish execution only;
the production artifact requires its separate public recipe. The seven
integration tests own isolated roots under the repository fixture area
(`.swarm-fixtures/trust-data-policy/`), never `/tmp`; a failed assertion
retains its root for diagnosis. The suite pins the Go-parity admission
contract: preview projection, denial without capture, exact classified path
with recovery, strict typed validation, approve/commit re-admission,
persistence across reopen and resource classification enforcement.
"""
import os,subprocess,sys
from pathlib import Path

def main():
    """Retain Cargo output and require every selected target test to pass."""
    env=os.environ.copy();env['CARGO_TARGET_DIR']=str(Path('.cache/rust-trust-data-policy-unit-target').resolve());env.update(CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0')
    argv=['cargo','test','--manifest-path','rust/Cargo.toml','-p','swarm-engine','--test','trust_data_policy','--','--nocapture']
    r=subprocess.run(argv,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE);sys.stdout.buffer.write(r.stdout);sys.stderr.buffer.write(r.stderr)
    names=['data_policy_public_contract','data_policy_admission_rejects_without_capture','data_policy_exact_action_and_recovery','data_policy_strict_validation','data_policy_checks_approval_and_first_effect','data_policy_persistence_and_disabled','data_policy_resource_classification']
    text=r.stdout.decode();assert r.returncode==0 and 'test result: ok. 7 passed;' in text
    for name in names:assert 'test '+name+' ... ok' in text,name
    print('DATA_POLICY_UNIT_PASS 7 named tests',flush=True)
if __name__=='__main__':main()
