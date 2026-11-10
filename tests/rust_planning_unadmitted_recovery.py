#!/usr/bin/env python3
"""Native admission refusal on a real injected process-creation failure.

A healthy traced call precedes syscall injection on another owned Root. strace
is a fixture-only tracer; it does not instrument the shipped binary. No paid
provider, private SQL mutation, global watchdog or invented reap is used.
"""
import argparse,json,os,sys,subprocess,traceback,hashlib,shutil
from pathlib import Path
import rust_agent_model_route as fixture
import rust_planning_native_run as native
import rust_planning_rejection_recovery as feedback
from rust_planning_activation import configured,claim_request
from rust_planning_call_preview import preview_request
from rust_planning_budget import ledger

def traced(binary,out,label,inject):
    """Compare real normal execution with kernel fork/clone EAGAIN refusal.

    Require actual injected syscall trace, saved original CLI failure and no
    provider witness. A revoked unadmitted run is distinct from child termination.
    """
    owned=fixture.Fixture(binary,out,label);fixture.publish(owned.root/'codex',feedback.PROGRAM.encode());os.chmod(owned.root/'codex',0o700)
    enabled=configured(owned,label);claim=label+'-claim';work=owned.ok(['planning','claim',owned.work],claim_request(enabled,claim,lease_seconds=120));feedback.publish_proposal(owned,work)
    request=preview_request(work,claim);assert owned.ok(['planning','call-preview',owned.work],request)['prepared_inputs_verified']
    path=out/(label+'-input.json');fixture.publish(path,request);trace=out/(label+'-process.trace')
    command=[shutil.which('strace'),'-f','-qq','-o',str(trace),'-e','trace=process']
    if inject:
        for call in ['clone','clone3','fork','vfork']:command+=['-e','inject='+call+':error=EAGAIN']
    command += [str(binary),'planning','run',owned.work,'--root',str(owned.root),'--input',str(path),'--json']
    fixture.publish(out/(label+'-command.json'),dict(argv=command,global_watchdog=None));money=ledger(owned)
    result=subprocess.run(command,env=owned.environment,stdin=subprocess.DEVNULL,capture_output=True);fixture.publish(out/(label+'-stdout'),result.stdout);fixture.publish(out/(label+'-stderr'),result.stderr)
    wire=json.loads(result.stderr if result.returncode else result.stdout)
    if not inject:
        assert result.returncode==0 and wire['decision_applied'] and not wire['accepted'],wire
        assert len(list(owned.root.glob('model-received-*.json')))==1
        return dict(root=str(owned.root),healthy_traced_control=True)
    assert result.returncode!=0 and 'EAGAIN' in trace.read_text() and '(INJECTED)' in trace.read_text(),wire
    assert not list(owned.root.glob('model-received-*.json')) and not list(owned.root.glob('fixture-prompt-*.bin')) and ledger(owned)==money
    row=native.native_row(owned,claim);assert row[3]=='refused' and row[2]==0 and row[1] is None,'unadmitted call retains ownership'
    current=owned.ok(['work','show',owned.work])['work'];assert not current.get('tasks') and current['planning']['decisions']==0
    event=next(e for e in current['planning']['inbox'] if e['kind']=='planning_rejected');ref=event['handoff'];raw=(owned.root/ref['path']).read_bytes();assert fixture.digest(raw)==ref['sha256'];diagnostic=json.loads(raw)
    assert diagnostic['grant_attempted'] is False and diagnostic['proposal'] is None
    causes=diagnostic['failure_causes'];assert any(c['io_errno']==11 for c in causes) and any('process-operation:spawn' in c['labels'] for c in causes)
    before=owned.snapshot();code,replay=owned.call(['planning','reconcile',claim]);assert code!=0 and 'planning-receipt-not-successful' in str(replay),replay
    assert owned.snapshot()==before and ledger(owned)==money and not list(owned.root.glob('model-received-*.json'))
    feedback.publish_proposal(owned,current);corrected=owned.ok(['planning','step',owned.work]);assert corrected['run']['work']['tasks'][0]['id']=='replanned-worker'
    prompt=next(owned.root.glob('fixture-prompt-*.bin')).read_bytes();assert b'process-operation:spawn' in prompt
    return dict(root=str(owned.root),unadmitted_revoked=True,safe_spawn_cause_received=True,corrected_task_applied=True,child_reap_claimed=False)

def main():
    """Bind tracer, exact helpers and normal binary; retain failures and Roots."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(exist_ok=False);binary=a.binary.resolve();tracer=shutil.which('strace');assert tracer,'strace fixture prerequisite missing'
    sources=list(dict.fromkeys([Path(__file__),Path(fixture.__file__),Path(native.__file__),Path(feedback.__file__)]+[Path(sys.modules[f.__module__].__file__) for f in [configured,claim_request,preview_request,ledger,feedback.fresh_work]]));inputs=dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),tracer=tracer,tracer_sha256=fixture.digest(Path(tracer).read_bytes()),sources={str(p):fixture.digest(p.read_bytes()) for p in sources});fixture.publish(out/'inputs.json',inputs)
    try:
        checks=[traced(binary,out,'healthy',False),traced(binary,out,'spawn-refused',True)]
        assert fixture.digest(binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(n).read_bytes())==h for n,h in inputs['sources'].items())
        result=dict(status='PASS',checks=checks,paid_provider=False,global_watchdog=None);fixture.publish(out/'result.json',result);print(json.dumps(result))
    except BaseException:fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise

if __name__=='__main__':main()
