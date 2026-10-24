#!/usr/bin/env python3
"""Public parsed-proposal rejection/replan with owned inert native processes.

Healthy activation precedes each fault. No paid account, private SQL writes,
mission Root, process watchdog or proof deletion is used. The retained prompt
contains fixture material only; it proves delivery, not model/agent authenticity.
"""
import argparse,json,os,sys,traceback,hashlib
from pathlib import Path
import rust_agent_model_route as fixture
import rust_planning_native_run as native
from rust_planning_bootstrap import fresh_work,request
from rust_planning_budget import ledger
from rust_planning_activation import apply_policy

PROGRAM=native.PROGRAM.replace("open('model-received.json','x'","open('model-received-'+str(os.getpid())+'.json','x'").replace("args=sys.argv[1:]", "with open('fixture-prompt-'+str(os.getpid())+'.bin','xb') as f:\n f.write(raw);f.flush();os.fsync(f.fileno())\nargs=sys.argv[1:]")

def pending(work):
    """Use actual unacknowledged root inputs; never invent delivery identifiers."""
    return [e['id'] for e in work['planning']['inbox'] if e['scope']=='root' and not e.get('decision')]

def publish_proposal(owned,work,bad=False,operations=None):
    """Publish explicit fixture output, separating original bad text from repair."""
    proposal=dict(input_events=pending(work),reason='UNTRUSTED-ORIGINAL-REJECTION-PROPOSAL' if bad else 'Corrected owned proposal',operations=[dict(kind='task',id='replanned-worker',title='Owned future worker',requirements=['req-999' if bad else 'req-1'],deliverable='Future report',criteria=['Future evidence'])])
    if operations is not None:proposal['operations']=operations
    fixture.publish(owned.root/'planning-proposal.json',proposal)
    return proposal

def setup(binary,out,label):
    """Use public configuration with an explicit six-activation fixture budget.

    One healthy empty decision and public resume witness the same native route
    before rejection. Every prompt/process witness is create-only and retained.
    """
    owned=fixture.Fixture(binary,out,label);fixture.publish(owned.root/'codex',PROGRAM.encode());os.chmod(owned.root/'codex',0o700)
    owned.work=fresh_work(owned,label+'-create');work=owned.ok(['planning','enable',owned.work],request(event=label+'-enable',max_activations=6))
    fixture.publish(owned.root/'planning-proposal.json',dict(input_events=pending(work),reason='Healthy control before rejection',operations=[]))
    healthy=owned.ok(['planning','step',owned.work]);assert healthy['run']['decision_applied']
    work=healthy['run']['work'];work=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id=label+'-input',expected_revision=work['revision'],reason='New explicit fixture input'))
    return owned,work

def reject(owned,work,operations=None,reason_code="planning-operation-requirements-invalid"):
    """Require durable diagnostic delivery without any proposed task/decision.

    Read the exact contained diagnostic and digest; its proposal is untrusted
    data. A receipt digest is correlation, never a signature or permission.
    """
    proposal=publish_proposal(owned,work,True,operations);old_decisions=work['planning']['decisions']
    code,wire=owned.call(['planning','step',owned.work]);assert code!=0 and reason_code in str(wire),wire
    current=owned.ok(['work','show',owned.work])['work'];assert not current.get('tasks') and current['planning']['decisions']==old_decisions
    events=[e for e in current['planning']['inbox'] if e['kind']=='planning_rejected' and not e.get('decision')]
    assert events,'business refusal diagnostic missing'
    event=events[-1];reference=event['handoff'];path=(owned.root/reference['path']).resolve();path.relative_to(owned.root.resolve());raw=path.read_bytes();assert hashlib.sha256(raw).hexdigest()==reference['sha256']
    diagnostic=json.loads(raw);assert diagnostic['schema_version']==1 and diagnostic['reason_code']==reason_code
    assert json.loads(diagnostic['proposal'])==proposal and len(diagnostic['original_receipt_sha256'])==64
    assert event['rejection_key'] and all(not e.get('decision') for e in current['planning']['inbox'] if e['id'] in proposal['input_events'])
    return current,event,reference

def corrected(binary,out):
    """Prove receipt→diagnostic→new prompt→corrected decision→actual task effect."""
    owned,work=setup(binary,out,'corrected');current,event,reference=reject(owned,work)
    before=set(owned.root.glob('fixture-prompt-*.bin'));publish_proposal(owned,current)
    result=owned.ok(['planning','step',owned.work]);work=result['run']['work'];assert result['run']['decision_applied'] and not result['accepted']
    assert work['planning']['decisions']==2 and work['planning']['activations']==3
    assert len(work['tasks'])==1 and work['tasks'][0]['id']=='replanned-worker' and work['tasks'][0]['scope_id']=='root'
    assert next(e for e in work['planning']['inbox'] if e['id']==event['id'])['decision']
    fresh=set(owned.root.glob('fixture-prompt-*.bin'))-before;assert len(fresh)==1;prompt=fresh.pop().read_bytes()
    assert reference['sha256'].encode() in prompt and b'UNTRUSTED-ORIGINAL-REJECTION-PROPOSAL' in prompt and b'req-999' in prompt
    money=ledger(owned);snapshot=owned.snapshot();assert owned.ok(['planning','step',owned.work])['status']=='idle';assert ledger(owned)==money and owned.snapshot()==snapshot
    return dict(root=str(owned.root),status='PASS',diagnostic_received=True,corrected_task_applied=True,global_acceptance=False)

def repeated(binary,out):
    """Two equivalent refusals block unchanged state, without an attempt policy.

    The second call consumes newly delivered diagnostic input. Afterwards the
    rejection-state guard must refuse before claim/model/reservation spend.
    """
    owned,work=setup(binary,out,'repeated');first,_,_=reject(owned,work);second,event,_=reject(owned,first)
    scope=next(s for s in second['planning']['scopes'] if s['id']=='root');assert scope['rejection_wait']['event_id']==event['id']
    snapshot=owned.snapshot();money=ledger(owned);prompts=set(owned.root.glob('fixture-prompt-*.bin'))
    code,wire=owned.call(['planning','step',owned.work]);assert code!=0 and wire['failure']['code']=='rejection_requires_change',wire
    assert snapshot==owned.snapshot() and money==ledger(owned) and prompts==set(owned.root.glob('fixture-prompt-*.bin'))
    return dict(root=str(owned.root),status='PASS',equivalent_refusal_blocked=True,no_repeat_spend=True)

def protected_artifact(binary,out):
    """After valid feedback, tamper and cap0 refuse before claim/model spend.

    Restore the exact original bytes and saved policy through supported changes,
    then require the corrected task. No live data, SQL mutation or timeout is used.
    """
    owned,work=setup(binary,out,'protected');current,event,reference=reject(owned,work)
    path=owned.root/reference['path'];original=path.read_bytes();publish_proposal(owned,current)
    def no_start():
        """Assert exact rejection/no-spend/no-process against the current witness."""
        snapshot=owned.snapshot();money=ledger(owned);prompts=set(owned.root.glob('fixture-prompt-*.bin'))
        code,wire=owned.call(['planning','step',owned.work]);assert code!=0 and 'planning-context-report-invalid' in str(wire),wire
        assert owned.snapshot()==snapshot and ledger(owned)==money and prompts==set(owned.root.glob('fixture-prompt-*.bin'))
    fixture.publish(path,original+b' ');no_start();fixture.publish(path,original)
    saved=owned.ok(['engine-settings','show'])['values']['planning_report_max_bytes']
    apply_policy(owned,'disable-feedback-read',dict(planning_report_max_bytes='0'));no_start()
    apply_policy(owned,'restore-feedback-read',dict(planning_report_max_bytes=saved))
    result=owned.ok(['planning','step',owned.work]);assert result['run']['work']['tasks'][0]['id']=='replanned-worker'
    return dict(root=str(owned.root),status='PASS',tamper_refused=True,cap0_refused=True,recovered=True)

def optional_operation_id(binary,out):
    """Unsupported operations with omitted ID still produce truthful feedback.

    The typed codec's empty ID is descriptive data, not a missing authority or
    fabricated failed-operation index. The original untrusted JSON stays intact.
    """
    owned,work=setup(binary,out,'optional-id');current,event,reference=reject(owned,work,operations=[dict(kind='unsupported')],reason_code='planning-operation-unported')
    diagnostic=json.loads((owned.root/reference['path']).read_bytes());assert diagnostic['operation_inventory']==[['unsupported','']]
    publish_proposal(owned,current);result=owned.ok(['planning','step',owned.work]);assert result['run']['work']['tasks'][0]['id']=='replanned-worker'
    return dict(root=str(owned.root),status='PASS',missing_operation_id_preserved=True,corrected_task_applied=True)


def main():
    """Bind retained outputs to exact driver/helper/default executable bytes."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);binary=a.binary.resolve()
    sources=list(dict.fromkeys([Path(__file__),Path(fixture.__file__),Path(native.__file__)]+[Path(sys.modules[f.__module__].__file__) for f in [fresh_work,request,ledger,apply_policy]]));inputs=dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(p):fixture.digest(p.read_bytes()) for p in sources});fixture.publish(out/'inputs.json',inputs)
    try:
        checks=[corrected(binary,out),repeated(binary,out),protected_artifact(binary,out),optional_operation_id(binary,out)]
        assert fixture.digest(binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(n).read_bytes())==h for n,h in inputs['sources'].items())
        result=dict(status='PASS',checks=checks,paid_provider=False,global_watchdog=None);fixture.publish(out/'result.json',result);print(json.dumps(result))
    except BaseException:fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise

if __name__=='__main__':main()
