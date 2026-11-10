#!/usr/bin/env python3
"""Reap failures, deliver observed diagnostics, and recover through public CLI.

Each owned inert-provider Root starts with a healthy call. No paid account,
private SQL mutation, global timeout, cost reset, or raw-stream claim is made.
The failure receipt contains no complete proposal; a diagnostic must say so.
"""
import argparse,json,os,sys,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_planning_rejection_recovery as feedback
from rust_planning_budget import ledger

FINAL="print(json.dumps(dict(type='result',structured_output=proposal,usage=dict(input_tokens=7,output_tokens=8))),flush=True)"
assert feedback.PROGRAM.count(FINAL)==1
FAULT=feedback.PROGRAM.replace(FINAL,"if open('failure-mode').read().strip()=='malformed':\n print('not-a-provider-event',flush=True)\nelse:\n "+FINAL+"\n sys.exit(9)")

def scenario(binary,out,mode,repeat=False):
    """Require durable input, no task/ACK, corrected effect and pure recovery.

    Original return/cause and receipt hash remain observed evidence; diagnostic
    content is untrusted, and a missing proposal must stay null. Reconciliation
    repeats no provider call and must not mutate the corrected Work. Equivalent
    repeated failures wait for a change before another claim or reservation.
    """
    owned,work=feedback.setup(binary,out,mode+('-repeat' if repeat else ''))
    fixture.publish(owned.root/'codex',FAULT.encode());os.chmod(owned.root/'codex',0o700)
    fixture.publish(owned.root/'failure-mode',mode.encode());feedback.publish_proposal(owned,work)
    money=ledger(owned);code,wire=owned.call(['planning','step',owned.work]);assert code!=0,wire
    current=owned.ok(['work','show',owned.work])['work'];assert not current.get('tasks') and current['planning']['decisions']==work['planning']['decisions']
    after=ledger(owned);assert all(row in after for row in money) and len(after)==len(money)+1;money=after
    events=[e for e in current['planning']['inbox'] if e['kind']=='planning_rejected' and not e.get('decision')]
    assert events,'reaped failure diagnostic missing'
    event=events[-1];reference=event['handoff'];raw=(owned.root/reference['path']).read_bytes();assert fixture.digest(raw)==reference['sha256'];diagnostic=json.loads(raw)
    assert diagnostic['proposal'] is None and diagnostic['operation_inventory']==[]
    assert diagnostic['reason_code'] in str(wire) and len(diagnostic['original_receipt_sha256'])==64
    assert all(not e.get('decision') for e in current['planning']['inbox'] if e['id'] in feedback.pending(work))
    claim=diagnostic['claim_id'];before=owned.snapshot();prompts=set(owned.root.glob('fixture-prompt-*.bin'))
    code,replay=owned.call(['planning','reconcile',claim]);assert code!=0 and 'planning-receipt-not-successful' in str(replay),replay
    assert owned.snapshot()==before and prompts==set(owned.root.glob('fixture-prompt-*.bin')) and ledger(owned)==money
    if repeat:
        feedback.publish_proposal(owned,current);code,second=owned.call(['planning','step',owned.work]);assert code!=0,second
        current=owned.ok(['work','show',owned.work])['work'];scope=next(s for s in current['planning']['scopes'] if s['id']=='root');assert scope.get('rejection_wait')
        snapshot=owned.snapshot();money=ledger(owned);prompts=set(owned.root.glob('fixture-prompt-*.bin'))
        code,blocked=owned.call(['planning','step',owned.work]);assert code!=0 and blocked['failure']['code']=='rejection_requires_change',blocked
        assert owned.snapshot()==snapshot and ledger(owned)==money and prompts==set(owned.root.glob('fixture-prompt-*.bin'))
        return dict(mode=mode,status='PASS',root=str(owned.root),equivalent_failure_blocked=True,no_repeat_spend=True)
    fixture.publish(owned.root/'codex',feedback.PROGRAM.encode());os.chmod(owned.root/'codex',0o700);feedback.publish_proposal(owned,current)
    result=owned.ok(['planning','step',owned.work]);assert result['run']['work']['tasks'][0]['id']=='replanned-worker'
    fresh=set(owned.root.glob('fixture-prompt-*.bin'))-prompts;assert len(fresh)==1;prompt=next(iter(fresh)).read_bytes();assert diagnostic['reason_code'].encode() in prompt and reference['sha256'].encode() in prompt
    after=ledger(owned);assert all(row in after for row in money) and len(after)==len(money)+1
    return dict(mode=mode,status='PASS',root=str(owned.root),diagnostic_received=True,missing_proposal_explicit=True,corrected_task_applied=True,reconcile_no_inference=True)

def report_publication_recovery(binary,out):
    """A saved cap refuses diagnostic publication; restore then reconcile, no call.

    Healthy control precedes the policy fault. Original receipts/held estimates
    survive; usage is not settled before a feedback commit. Restoration uses
    the exact observed saved value through Admin's public settings contract.
    """
    owned,work=feedback.setup(binary,out,'publication')
    cap=owned.ok(['engine-settings','show'])['values']['planning_report_max_bytes']
    feedback.apply_policy(owned,'disable-publication',dict(planning_report_max_bytes='0'))
    fixture.publish(owned.root/'codex',FAULT.encode());os.chmod(owned.root/'codex',0o700);fixture.publish(owned.root/'failure-mode',b'malformed');feedback.publish_proposal(owned,work)
    code,wire=owned.call(['planning','step',owned.work]);assert code!=0 and 'planning-rejection-report-limit' in str(wire),wire
    current=owned.ok(['work','show',owned.work])['work'];assert not current.get('tasks') and current['planning']['decisions']==work['planning']['decisions']
    assert not any(e['kind']=='planning_rejected' for e in current['planning']['inbox'])
    with feedback.native.database(owned) as db:
        rows=db.execute("SELECT claim_id,phase FROM native_planning_runs WHERE phase!='applied'").fetchall();assert len(rows)==1 and rows[0][1]=='permitted';claim=rows[0][0]
        assert db.execute('SELECT count(*) FROM native_planning_usage WHERE claim_id=?',[claim]).fetchone()==(0,)
    before=owned.snapshot();money=ledger(owned);prompts=set(owned.root.glob('fixture-prompt-*.bin'))
    code,wire=owned.call(['planning','reconcile',claim]);assert code!=0 and 'planning-rejection-report-limit' in str(wire),wire
    assert owned.snapshot()==before and ledger(owned)==money and prompts==set(owned.root.glob('fixture-prompt-*.bin'))
    feedback.apply_policy(owned,'restore-publication',dict(planning_report_max_bytes=cap))
    code,wire=owned.call(['planning','reconcile',claim]);assert code!=0 and 'planning-receipt-not-successful' in str(wire),wire
    current=owned.ok(['work','show',owned.work])['work'];assert any(e['kind']=='planning_rejected' for e in current['planning']['inbox']) and ledger(owned)==money and prompts==set(owned.root.glob('fixture-prompt-*.bin'))
    fixture.publish(owned.root/'codex',feedback.PROGRAM.encode());os.chmod(owned.root/'codex',0o700);feedback.publish_proposal(owned,current)
    result=owned.ok(['planning','step',owned.work]);assert result['run']['work']['tasks'][0]['id']=='replanned-worker'
    return dict(status='PASS',root=str(owned.root),publication_refused_without_work_feedback=True,original_receipt_recovered_without_call=True,corrected_task_applied=True)

def main():
    """Retain exact helper/binary inputs and original failure; never delete Roots."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--case',choices=['malformed','nonzero','all'],default='all');a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);binary=a.binary.resolve()
    sources=list(dict.fromkeys([Path(__file__),Path(fixture.__file__),Path(feedback.__file__),Path(feedback.native.__file__)]+[Path(sys.modules[f.__module__].__file__) for f in [feedback.fresh_work,feedback.request,ledger,feedback.apply_policy]]));inputs=dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(p):fixture.digest(p.read_bytes()) for p in sources});fixture.publish(out/'inputs.json',inputs)
    try:
        checks=[scenario(binary,out,m,repeat) for repeat in [False,True] for m in (['malformed','nonzero'] if a.case=='all' else [a.case])]
        if a.case=='all':checks.append(report_publication_recovery(binary,out))
        assert fixture.digest(binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(n).read_bytes())==h for n,h in inputs['sources'].items())
        result=dict(status='PASS',checks=checks,paid_provider=False,global_watchdog=None);fixture.publish(out/'result.json',result);print(json.dumps(result))
    except BaseException:fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise

if __name__=='__main__':main()
