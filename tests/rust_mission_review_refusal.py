#!/usr/bin/env python3
"""Real reviewer invalid-quote refusal after a healthy full public journey.

The inert variant is fixed before preflight. No private SQL mutation, invented
receipt, live Root, provider billing, global timeout or automatic retry. Proves
safe diagnosis, actual final known revision and retained single-call funding.
"""
import argparse,json,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_mission_controls as controls
import rust_mission_report_relay as relay
import rust_mission_watch_worker as worker
import rust_native_parent_death as parent_death
check=worker.check

def fault(binary,out):
    """Execute the real malformed reviewer and compare its journal, safe public
    failure and committed Work; no stdout event or exit alone establishes PASS.
    """
    out.mkdir();owned,producer,report=relay.prepare(binary,out,controls=True,review_fault=True)
    before=relay.work(owned);calls=before['planning']['reviewer'].get('calls',0)
    finance_before=relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))
    events=relay.cycle(owned,out/'watch',refusal=True)
    failure=json.loads((out/'watch/watch.stderr').read_bytes());current=relay.work(owned)
    check('review-reply-quote-absent' in failure['failure']['causes'],'lost static protocol reason')
    check('PRIVATE-ABSENT' not in json.dumps(failure),'private reply appeared in public diagnosis')
    check(failure['recorded_work']==dict(id=owned.work,revision=current['revision']),'newer recorded review result hidden by an older stage')
    task=next(t for t in current['tasks'] if t['id']=='task')
    check(task['status']!='accepted' and task['automatic_validation']['controls'][0]['passed'],'refusal lost successful controls or accepted task')
    check(not any(e.get('event')=='mission-review-recorded' and e.get('verdict_applicable') for e in events),'invalid quote became applicable verdict')
    witness=json.loads((owned.root/'reviewer-invoked.json').read_bytes());prompt=(owned.root/'reviewer-input.bin').read_bytes()
    check(fixture.digest(prompt)==witness['stdin_sha256'] and b'PRIVATE-ABSENT-REVIEW-QUOTE' not in prompt,'invalid quote was actually an input original')
    jobs=relay.rows(owned,'SELECT id,phase,permission,published_revision,prompt_sha256,owner_pid FROM native_review_jobs WHERE work_id=?',(owned.work,))
    check(len(jobs)==1 and jobs[0][1:4]==('refused',1,current['revision']) and jobs[0][4]==witness['stdin_sha256'] and jobs[0][5]==witness['ppid'],'actual refused reviewer/result not bound to final Work')
    finance_after=relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))
    check(all(row in finance_after for row in finance_before) and len([r for r in finance_after if r[1]=='reviewer' and r[0]==jobs[0][0]])==1,'refusal lost or repeated reservation')
    check(current['planning']['reviewer']['calls']==calls+1,'refusal did not consume exactly one reviewer call')
    receipts=list((owned.root/'.swarm').glob('native-review-*.receipt'));check(len(receipts)==1,'missing original native reviewer receipt')
    magic,extent,digest,body=receipts[0].read_bytes().split(b'\n',3)
    check(magic==b'SR1' and len(extent)==20 and int(extent)==len(body) and fixture.digest(body)==digest.decode(),'original receipt integrity failed')
    receipt=json.loads(body)
    original=receipt['original_reply'];check(isinstance(original,list),'missing actual decoded original reply')
    reply=json.loads(bytes(original));check(reply==json.loads((owned.root/'fixture-review-reply.json').read_bytes()) and reply['criteria'][0]['evidence']=='PRIVATE-ABSENT-REVIEW-QUOTE','decoder did not retain the exact malformed quote reply')
    check(receipt['failed'] and receipt['waited'] and receipt['grant_attempted'] and receipt['channel_closed'],'refusal lost actual process observations')
    check(any('review-reply-quote-absent' in cause['labels'] for cause in receipt['causes']),'journal lost static protocol reason')
    fixture.publish(out/'public-work.json',current);fixture.publish(out/'public-failure.json',failure);fixture.publish(out/'finance-before.json',finance_before);fixture.publish(out/'finance-after.json',finance_after)
    return dict(root=str(owned.root),work=owned.work,actual_reviewer=True,static_reason=True,latest_known_revision=True,single_call=True,task_accepted=False)

def main():
    """Require actual default-profile binary then positive-before-fault proof;
    restore the owned subreaper and preserve all primary/cleanup failures.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:p.add_argument('--'+flag,type=Path,required=True)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True);binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes())
    check(profile['no_fixture_feature'] and profile['sha256']==fixture.digest(binary.read_bytes()),'review refusal binary/profile mismatch')
    def run():
        positive=out/'positive';positive.mkdir();control=controls.run(binary,positive,True)
        return dict(status='PASS',positive_control=control,refusal=fault(binary,out/'refused'),installed=False,global_watchdog=None)
    result=worker.subreaper_scope(run,lambda e:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(e)))),parent_death.subreaper)
    fixture.publish(out/'result.json',result);print('PASS malformed reviewer quote retains static refusal, latest known Work and one reservation',flush=True)
if __name__=='__main__':main()
