#!/usr/bin/env python3
"""Real resumed watch worker→report→saved native controls, without acceptance.

Uses a fresh owned Root and inert actual provider. The saved covering command is
selected before preflight. No manual control admission (only exact terminal readback), private SQL writes,
live Root, provider billing or global verification watchdog. Mission watch retains
its independent heartbeat and completes the publication/dispatch pass before a
further planner activation. Native receipt/actual control result are asserted;
automatic reviewer is required with --reviewer; acceptance remains outside this proof.
"""
import argparse
import json
import subprocess
from pathlib import Path
import traceback
import rust_agent_model_route as fixture
import rust_mission_report_relay as relay
import rust_mission_watch_worker as worker
import rust_native_parent_death as parent_death
check=worker.check


def run(binary,out,expect_reviewer=False):
    """Observe actual automatic controls and unchanged producer/cost, never infer
    execution from an HTTP status or event alone. Full receipt bytes and Work are
    retained; the actual public task status must remain submitted or blocked.
    After watch exits, exact terminal replay must read back the original result
    without a fresh owning lease and without starting a new control process.
    With reviewer required, replay its exact original claim after watch exit and
    require unchanged Work, invocation witness and funding without a new call.
    """
    owned,producer,report=relay.prepare(binary,out,controls=True)
    agent=owned.ok(['agent','show',producer['agent']])['agent']
    costs=owned.ok(['costs','show',owned.work])
    finance_before=relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))
    reviewer_before=relay.work(owned)['planning']['reviewer'].get('calls',0)
    fixture.publish(out/'fixture.json',dict(root=str(owned.root),work=owned.work,binary_sha256=fixture.digest(binary.read_bytes())))
    events=relay.cycle(owned,out/'controls-watch',completed_event='mission-review-recorded' if expect_reviewer else 'mission-controls-recorded')
    check(not any(c['argv'][1:3] in (['task','submit'],['task','validate']) for c in owned.calls),'fixture manually submitted or ran controls')
    records=[r for r in events if r.get('event')=='mission-controls-recorded']
    reports=[r for r in events if r.get('event')=='mission-report-submitted']
    check(len(reports)==1 and reports[0]['agent_id']==agent['id'] and reports[0]['report']=='watch-worker/docs/task.md' and reports[0]['task_accepted'] is False,'missing exact canonical automatic report relay')
    check(len(records)==1 and records[0]['controls_passed'] is True and records[0]['task_accepted'] is False and records[0]['automatic_reviewer'] is False,'missing actual successful automatic controls event')
    check(reports[0]['revision']<records[0]['revision'] and events.index(reports[0])<events.index(records[0]),'controls were not recorded after the actual report relay')
    current=relay.work(owned);task=next(t for t in current['tasks'] if t['id']=='task')
    validation=task.get('automatic_validation')
    check(validation is not None and task['status'] in ('submitted','blocked') and validation['state'] in ('pending_review','pending_human'),'controls accepted task or lost required review')
    controls=validation['controls']
    check(len(controls)==1 and controls[0]['executed'] and controls[0]['passed'] and controls[0]['exit_code']==0,'native control did not actually execute/pass')
    receipt=owned.root/validation['receipt'];raw=receipt.read_bytes();raw_receipt=raw;public=json.loads(raw)
    check(public['job_id']==records[0]['job_id'] and not public['failures'] and public['producer_agent_id']==agent['id'],'receipt does not bind actual job/producer')
    check(validation['artifacts'][validation['receipt']]==fixture.digest(raw),'public receipt bytes differ from saved digest')
    check(validation['artifacts']['watch-worker/docs/task.md']==fixture.digest(report.read_bytes()),'report bytes not bound into actual controls')
    check(owned.ok(['agent','show',agent['id']])['agent']==agent,'controls rewrote original worker')
    check(owned.ok(['costs','show',owned.work])==costs,'controls/review changed worker reported-cost history')
    check(task['status']!='accepted','controls/review fabricated acceptance')
    reviews=[r for r in events if r.get('event')=='mission-review-recorded']
    if expect_reviewer:
        check(len(reviews)==1 and reviews[0]['review_recorded'] and reviews[0]['verdict_applicable'] and not reviews[0]['task_accepted'],'missing actual automatic reviewer result')
        check(events.index(records[0])<events.index(reviews[0]) and records[0]['revision']<reviews[0]['revision'],'review preceded actual controls')
        review=task['independent_review'];check(review['id']==reviews[0]['review_id'] and review['state']=='passed' and review['producer']==agent['id'],'automatic review does not bind actual producer')
        witness=json.loads((owned.root/'reviewer-invoked.json').read_bytes())
        prompt=(owned.root/'reviewer-input.bin').read_bytes()
        check(fixture.digest(prompt)==witness['stdin_sha256'] and b'ROOT-PRIVATE-TITLE' not in prompt and b'ROOT-PRIVATE-OBJECTIVE' not in prompt,'reviewer prompt bytes differ or leak enclosing private brief')
        jobs=relay.rows(owned,'SELECT prompt_sha256,producer_id,attempt_id,permission,phase,owner_pid FROM native_review_jobs WHERE id=?',(review['id'],))
        check(len(jobs)==1 and jobs[0][0]==witness['stdin_sha256'] and jobs[0][1]==agent['id'] and jobs[0][2]==agent['attempt_id'] and jobs[0][3:5]==(1,'recorded') and jobs[0][5]==witness['ppid'],'actual reviewer invocation not bound to canonical job')
        finance_after=relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))
        check(all(row in finance_after for row in finance_before) and len([r for r in finance_after if r[1]=='reviewer' and r[0]==review['id']])==1,'review lost prior funding or reserved more than once')
        check(current['planning']['reviewer']['calls']==reviewer_before+1,'review call count not consumed exactly once')
        fixture.publish(out/'finance-before.json',finance_before);fixture.publish(out/'finance-after.json',finance_after)
        # Only original canonical review intention, after conductor exit; public
        # readback may not create a fresh grant/call or adopt current consent.
        original=relay.rows(owned,"SELECT request FROM events WHERE work_id=? AND kind='review.claim'",(owned.work,))
        check(len(original)==1,'ambiguous original reviewer intention')
        raw_intent=original[0][0];raw_intent=raw_intent.encode() if isinstance(raw_intent,str) else raw_intent
        intent_path=out/'original-review-request.json';fixture.publish(intent_path,raw_intent)
        command=[str(binary),'task','review',owned.work,'--root',str(owned.root),'--input',str(intent_path),'--json']
        fixture.publish(out/'review-replay.command.json',dict(argv=command,readback_only=True))
        replayed=subprocess.run(command,env=owned.environment,capture_output=True)
        fixture.publish(out/'review-replay.stdout',replayed.stdout);fixture.publish(out/'review-replay.stderr',replayed.stderr)
        check(replayed.returncode==0,'automatic reviewer historical result cannot replay without fresh owning lease')
        shown_review=json.loads(replayed.stdout)
        check(shown_review['replayed'] and shown_review['review_id']==review['id'] and shown_review['verdict_applicable'] and not shown_review['task_accepted'],'review replay lost original verdict or fabricated acceptance')
        check(relay.work(owned)==current,'review replay changed Work')
        check(json.loads((owned.root/'reviewer-invoked.json').read_bytes())==witness and relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))==finance_after,'review replay executed or changed funding')

    else:check(not reviews and not task.get('independent_review'),'unexpected reviewer in controls-only legacy fixture')
    # Read the isolated fixture's original canonical intent, never synthesize a
    # new command identity. This read-only SQL observation grants no execution.
    rows=relay.rows(owned,"SELECT request FROM events WHERE work_id=? AND kind='task.validation-start'",(owned.work,))
    check(len(rows)==1,'ambiguous original controls intention')
    raw=rows[0][0];raw=raw.encode() if isinstance(raw,str) else raw
    request_path=out/'original-control-request.json';fixture.publish(request_path,raw)
    command=[str(binary),'task','validate',owned.work,'--root',str(owned.root),'--input',str(request_path),'--json']
    fixture.publish(out/'replay.command.json',dict(argv=command,readback_only=True))
    replay=subprocess.run(command,env=owned.environment,capture_output=True)
    fixture.publish(out/'replay.stdout',replay.stdout);fixture.publish(out/'replay.stderr',replay.stderr)
    check(replay.returncode==0,'terminal automatic control receipt cannot be replayed without fresh owning lease')
    shown=json.loads(replay.stdout)
    check(shown['replayed'] and shown['controls_passed'] and shown['task_accepted'] is False and shown['job_id']==public['job_id'],'replay did not return original terminal receipt')
    check(relay.work(owned)==current and receipt.read_bytes()==raw_receipt,'replay changed Work or original public receipt')
    fixture.publish(out/'public-work.json',current);fixture.publish(out/'public-receipt.json',public)
    return dict(status='PASS',root=str(owned.root),work=owned.work,actual_worker=True,automatic_report=True,actual_native_controls=True,no_manual_control_admission=True,terminal_replay_without_new_authority=True,task_accepted=False,automatic_reviewer=expect_reviewer,installed=False)


def main():
    """Bind public execution to verified native default-profile binary bytes;
    retain original failures and use the existing owned-child cleanup boundary.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:parser.add_argument('--'+flag,required=True,type=Path)
    parser.add_argument('--reviewer',action='store_true',help='Require the current automatic reviewer stage; absent retains the older controls-only reference contract')
    args=parser.parse_args();binary=args.binary.resolve();out=args.output.resolve();out.mkdir(parents=True)
    profile=json.loads(args.profile.read_bytes())
    check(profile['no_fixture_feature'] and profile['sha256']==fixture.digest(binary.read_bytes()),'controls binary/profile mismatch')
    result=worker.subreaper_scope(lambda:run(binary,out,args.reviewer),lambda e:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(e)))),parent_death.subreaper)
    fixture.publish(out/'result.json',result)
    print('PASS public watch relays actual worker report and executes saved native controls'+(' and automatic reviewer' if args.reviewer else '')+' without acceptance',flush=True)


if __name__=='__main__':main()
