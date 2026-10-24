#!/usr/bin/env python3
"""A failed real control must not replay the same completed worker's report.

Same socket-held passing control precedes a fresh failing check. Public planning
pause isolates the next relay pass; no fake review/SQL write/global watchdog.
"""
import argparse,json,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_mission_controls_revocation as controls
import rust_mission_report_relay as relay
import rust_mission_watch_worker as worker
import rust_native_parent_death as parent_death
check=worker.check


def run(binary,out,reviewer):
    """Retain real passing then failed native execution before resumed watch.
    The second pass must preserve blocked Work, old receipt and funding exactly.
    """
    valid=controls.scenario(binary,out/'positive',False,expect_reviewer=reviewer)
    failed,owned=controls.scenario(binary,out/'failed',False,fail_control=True,return_fixture=True)
    current=relay.work(owned)
    owned.ok(['planning','pause',owned.work],dict(schema_version=1,event_id='isolate-failed-report-relay',expected_revision=current['revision'],reason='Keep failed quality result blocked; isolate relay from planner'))
    before=relay.work(owned);finance=relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))
    fixture.publish(out/'before-resume.json',before)
    events=relay.cycle(owned,out/'resumed')
    after=relay.work(owned);fixture.publish(out/'after-resume.json',after)
    check(not any(e.get('event')=='mission-report-submitted' for e in events),'same completed worker report was resubmitted after failed quality control')
    check(after==before and next(t for t in after['tasks'] if t['id']=='task')['status']=='blocked','resumed relay unblocked or changed failed-quality task')
    check(relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))==finance,'replay changed planning/reviewer funding')
    return dict(status='PASS',positive=valid,failed=failed,same_worker_report_not_repeated=True,task_accepted=False,installed=False)


def main():
    """Require exact normal binary and retain the original faulty execution."""
    p=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:p.add_argument('--'+flag,required=True,type=Path)
    p.add_argument('--reviewer',action='store_true');a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True);binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes())
    check(profile['no_fixture_feature'] and profile['sha256']==fixture.digest(binary.read_bytes()),'report-once binary/profile mismatch')
    result=worker.subreaper_scope(lambda:run(binary,out,a.reviewer),lambda e:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(e)))),parent_death.subreaper)
    fixture.publish(out/'result.json',result);print('PASS failed native control does not resubmit the same completed worker report',flush=True)


if __name__=='__main__':main()
