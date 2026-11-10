#!/usr/bin/env python3
"""Replay start atomicity using an already qualified verification-only SDK caller.

Public normal worker/control setup only; no private positive SQL, paid provider,
live mission, receipt invention or watchdog. SDK and normal profile SHA must bind
the supplied executables to the prior feature/source qualification.
"""
import argparse,json,os,subprocess,traceback
from pathlib import Path
import rust_agent_model_route as f
import rust_native_review_inputs as inputs
import rust_native_reviewer as review

def main():
    """Retain actual rollback, same-request recovery, channels and original hashes.

    The SDK injects only its negative trigger AFTER a valid Root.open and a valid
    start, requires a SQLite trigger constraint and equality of Work/control/event
    state, then retries successfully. This is API coverage, not a CLI fault-hook
    or shipped executable. Keep failures/Roots and distinguish local acceptance.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for key in ['binary','profile','sdk-binary','sdk-profile','output']:p.add_argument('--'+key,required=True,type=Path)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);root=None;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();sdk=a.sdk_binary.resolve();normal=json.loads(a.profile.read_bytes());profile=json.loads(a.sdk_profile.read_bytes())
        assert normal['no_fixture_feature'] and normal['sha256']==f.digest(binary.read_bytes())
        assert profile['engine_features_empty'] and profile['shared_workspace_features_match_normal'] and not profile['test_profile']
        assert profile['binary_sha256']==f.digest(sdk.read_bytes()) and profile['normal_profile_sha256']==normal['sha256']
        f.publish(out/'profiles.json',dict(normal=normal,sdk=profile))
        root,work,_=inputs.source(binary,out/'source')
        f.publish(root/'.swarm/verification-mission-start.json',dict(root=str(root),work=work))
        request=dict(schema_version=1,event_id='sdk-start',expected_revision=0,reason='Owned API atomicity fixture',profile=dict(provider='fixture',role='worker',workspace='.',level='standard',capture_output=True))
        f.publish(out/'request.json',dict(root=str(root),work=work,request=request))
        before=review.finance(root,work);originals={str(q.relative_to(root)):f.digest(q.read_bytes()) for q in [root/'handoff.md',root/'validation-executed.json',*(root/'.swarm').glob('native-validation-*.receipt')]}
        command=[str(sdk),str(out/'request.json')];f.publish(out/'command.json',dict(argv=command,explicit_fixture_cache={'CODEX_HOME':str(root/'codex-home')},global_watchdog=None))
        run=subprocess.run(command,env={**os.environ,'CODEX_HOME':str(root/'codex-home')},stdin=subprocess.DEVNULL,capture_output=True)
        f.publish(out/'stdout',run.stdout);f.publish(out/'stderr',run.stderr);f.publish(out/'exit.json',dict(returncode=run.returncode));assert run.returncode==0
        result=json.loads(run.stdout);assert result['status']=='PASS' and result['actual_sql_trigger_constraint'] and result['profile_controls_consent_and_events_rolled_back'] and result['same_request_recovered']
        assert review.finance(root,work)==before and all(f.digest((root/n).read_bytes())==h for n,h in originals.items())
        result.update(root=str(root),originals=originals,finance_unchanged=True,normal_binary_sha256=normal['sha256'],sdk_binary_sha256=profile['binary_sha256'],global_watchdog=None,installed=False)
        f.publish(out/'result.json',result);print('PASS Root API mission start atomicity and same-request recovery with unchanged originals/finance',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()));raise

if __name__=='__main__':main()
