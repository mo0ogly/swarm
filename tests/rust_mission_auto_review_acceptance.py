#!/usr/bin/env python3
"""Automatic native journey through final review, then explicit fixture acceptance.

Reuse maintained status read-only/acceptance/freshness probes on the actual
worker's report. No acceptance inferred from reviewer, authenticated human,
live Root, paid model, SQL write, retry or global verification watchdog.
"""
import argparse,json,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_mission_controls as controls
import rust_status_review as status
import rust_mission_watch_worker as worker
import rust_native_parent_death as parent_death
check=worker.check

def run(binary,out):
    """Generate actual automatic evidence; require public status before/after
    explicit task.update acceptance, stale report refusal and exact restoration.
    Finance/invocation unchanged and status DB nonmutation use existing guards.
    """
    proof=out/'source';proof.mkdir();source=controls.run(binary,proof,True)
    check(source['task_accepted'] is False,'automatic source already accepted task')
    observed=status.observe(binary,out,proof,Path(source['root']),source['work'],report_relative='watch-worker/docs/task.md')
    return dict(status='PASS',root=source['root'],work=source['work'],automatic_review=True,explicit_fixture_acceptance=True,reads=observed['reads'],stale_and_recovery=True,authenticated_human=False,installed=False,global_G3_complete=False)

def main():
    """Require qualified default profile bytes and owned cleanup; preserve every
    original failure and distinguish fixture acceptance from real authorization.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:p.add_argument('--'+flag,type=Path,required=True)
    a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True);profile=json.loads(a.profile.read_bytes());check(profile['no_fixture_feature'] and profile['sha256']==fixture.digest(binary.read_bytes()),'automatic acceptance profile mismatch')
    result=worker.subreaper_scope(lambda:run(binary,out),lambda e:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(e)))),parent_death.subreaper)
    fixture.publish(out/'result.json',result);print('PASS automatic review public status, explicit fixture acceptance and stale report recovery',flush=True)
if __name__=='__main__':main()
