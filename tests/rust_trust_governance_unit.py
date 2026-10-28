#!/usr/bin/env python3
"""Run the eleven named governance regressions, separate from production profile.

No global watchdog or provider. Raw output/counts establish execution only;
the production artifact and browser require their separate public recipe.
"""
import os,subprocess,sys
from pathlib import Path

def main():
    """Retain Cargo output and require every selected target test to pass."""
    env=os.environ.copy();env['CARGO_TARGET_DIR']=str(Path('.cache/rust-native-version846-target').resolve());env.update(CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0')
    argv=['cargo','test','--manifest-path','rust/Cargo.toml','-p','swarm-engine','--test','trust_governance','--','--nocapture']
    r=subprocess.run(argv,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE);sys.stdout.buffer.write(r.stdout);sys.stderr.buffer.write(r.stderr)
    names=['governance_policy_persists_and_preview_is_read_only','governance_public_signatures_checkpoints_and_revocation','governance_invalid_policy_refuses_and_omission_preserves','governance_audit_links_pending_and_completed_transaction','governance_effect_conflicts_preserve_business_state','governance_review_alias_cannot_bypass_separation','governance_review_revocation_preserves_historical_audit','governance_reconcile_survives_later_group_conflict','governance_review_new_missing_target_is_recorded','governance_review_new_foreign_root_revoke_is_independent','governance_enrollment_mapping_has_no_local_token_fallback']
    text=r.stdout.decode();assert r.returncode==0 and 'test result: ok. 11 passed;' in text
    for name in names:assert 'test '+name+' ... ok' in text,name
    print('GOVERNANCE_UNIT_PASS 11 named tests',flush=True)
if __name__=='__main__':main()
