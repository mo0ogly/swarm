#!/usr/bin/env python3
"""Run the first attributed managed manual worker, blocked and never accepted.

Use actual native planning then a real inert local worker. CLI admission, replay,
completion and queued cancellation run on isolated owned Roots, no paid provider,
SQL seeding, install or whole-test watchdog. Source and copy evidence are retained.
"""
import argparse,json,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_managed_planning as planning
from rust_mission_report_relay import rows


def run(binary,profile,out,*,faults=False,detached_git=False,subproject=False):
    """Bind source/candidate/copy/original intent and actual process settlement.

    The planning fixture supplies two existing todo tasks. The first executes;
    the second is cancelled before any provider. Every failure preserves Root,
    original CLI channels and copied sources; no test accepts or integrates a task.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    subdir='app' if subproject else ''
    result,f=planning.run(binary,profile,out/'planning',return_fixture=True,project_subdir=subdir)
    f.work=result['planned_work'];root=f.root
    current=f.ok(['work','show',f.work])['work']
    current=f.ok(['budget','apply',f.work],dict(schema_version=1,event_id='managed-worker-budget',expected_revision=current['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=1,estimate_source='Owned inert worker fixture',reference_date='2026-10-24')))
    git_path='../.git' if subproject else '.git'
    program=fixture.PROGRAM.replace('raw=sys.stdin.buffer.read()',"raw=sys.stdin.buffer.read()\nos.rename("+repr(git_path)+","+repr(git_path+'.worker-retained')+")") if detached_git else fixture.PROGRAM
    fixture.publish(root/'codex',program.encode());(root/'codex').chmod(0o700)
    source=planning.repository.inventory(root/'source');base=current['planning']['repository']['candidate_commit']
    f.request=dict(schema_version=1,event_id='session',expected_revision=current['revision'],task_id='task',provider='fixture',capture_output=True)
    original=f.request.copy();prepared=f.prepare();agent=prepared['agent'];workspace=Path(agent['workspace'])
    def status(label,expected):
        """Read real status without mutation; require identity, state and no validation."""
        before=f.snapshot();observed=f.ok(['mission','status',f.work])
        fixture.publish(out/('status-'+label+'.json'),observed)
        item=next(t for t in observed['tasks'] if t['id']=='task')
        assert item['attempt']['agent_id']=='session' and item['attempt']['process_state']==expected,item
        assert observed['validated']==0 and f.snapshot()==before
        assert any(call['id']=='agent/session' and call['state']==expected for call in observed['spending']['calls'])
        if expected=='queued':assert item['result']['report_state']=='unobserved',item
        return observed
    status('queued','queued')
    print('PASS managed-status-queued-identity-without-mutation',flush=True)
    assert workspace==Path(current['planning']['repository']['storage'])/'copies/task-1'/subdir
    assert workspace.is_dir() and (workspace/'tracked.txt').read_bytes()==b'Committed planning input\n'
    assert planning.repository.git(workspace,'rev-parse','HEAD').decode().strip()==base
    assert f.reservation()==('reserved',1.0) and not (workspace/'model-received.json').exists()
    assert f.ok(['agent','prepare',f.work],original)['agent']==agent
    print('PASS managed-manual-attributed-copy-and-replay',flush=True)
    if faults:
        before=f.snapshot();manifest=Path(current['planning']['repository']['storage'])/'native-worker-session.json';original_manifest=manifest.read_bytes()
        invalid=json.loads(original_manifest);invalid['unknown_field']=True;fixture.publish(manifest,invalid)
        try:
            code,wire=f.call(['mission','status',f.work]);assert code!=0 and 'managed-worker-proof-invalid' in json.dumps(wire)
            assert f.snapshot()==before
            print('PASS managed-status-original-proof-refusal-without-mutation',flush=True)
            code,wire=f.call(['agent','run','session']);assert code!=0 and 'managed-worker-proof-invalid' in json.dumps(wire)
            assert f.snapshot()==before and not (workspace/'model-received.json').exists()
        finally:fixture.publish(manifest,original_manifest)
        parked=workspace.with_name(workspace.name+'.retained');workspace.rename(parked)
        try:
            code,wire=f.call(['agent','run','session']);assert code!=0 and ('managed-worker-workspace-path' if subproject else 'managed-worker-copy-open') in json.dumps(wire)
            assert f.snapshot()==before and not (parked/'model-received.json').exists()
        finally:parked.rename(workspace)
        print('PASS managed-manual-proof-copy-refusals-preserve-same-queued-agent',flush=True)
    ran=f.ok(['agent','run','session']);assert ran['agent']['status']=='completed'
    observed=json.loads((workspace/'model-received.json').read_bytes());assert observed['stdin_sha256']==fixture.digest(agent['prompt'].encode())
    assert f.reservation()==('estimated',1.0)
    work=f.ok(['work','show',f.work])['work'];task=next(t for t in work['tasks'] if t['id']=='task')
    assert task['status']=='blocked' and len(task['attempts'])==1
    assert planning.repository.inventory(root/'source')==source and workspace.is_dir()
    (root/'docs').mkdir(exist_ok=True)
    fixture.publish(root/'docs/task.md',b'Unrelated Root report must never be attributed to managed copy.\n')
    parked=workspace.with_name(workspace.name+'.status-retained');workspace.rename(parked)
    try:
        ended=status('completed-copy-absent','completed')
        item=next(t for t in ended['tasks'] if t['id']=='task')
        assert item['state']=='intervention' and item['result']['state']=='completed_unproven' and item['result']['report_state']=='absent_or_unattributed',item
    finally:parked.rename(workspace)
    print('PASS managed-status-completed-without-mutable-copy-or-false-report',flush=True)
    print('PASS managed-manual-real-worker-completes-blocked-with-retained-copy',flush=True)
    second=dict(schema_version=1,event_id='cancel-managed',expected_revision=work['revision'],task_id='task-2',provider='fixture',capture_output=True)
    queued=f.ok(['agent','prepare',f.work],second)['agent'];copy=Path(queued['workspace']);assert not (copy/'model-received.json').exists()
    retained=copy.with_name(copy.name+'.retained')
    if faults:copy.rename(retained)
    try:
        f.ok(['agent','stop','cancel-managed']);f.ok(['agent','reconcile','cancel-managed'])
        cancelled=status('cancelled-copy-absent','completed')
        item=next(t for t in cancelled['tasks'] if t['id']=='task-2')
        assert item['attempt']['agent_id']=='cancel-managed' and item['attempt']['process_state']=='interrupted',item
    finally:
        if faults:retained.rename(copy)
    final=f.ok(['agent','show','cancel-managed'])['agent'];assert final['status']=='interrupted' and final['desired']=='stop'
    assert rows(f,'SELECT state FROM reservations WHERE agent_id=?',('cancel-managed',))==[('released',)]
    assert copy.is_dir() and not (copy/'model-received.json').exists() and planning.repository.inventory(root/'source')==source
    last=f.ok(['work','show',f.work])['work'];cancelled_task=next(t for t in last['tasks'] if t['id']=='task-2')
    assert cancelled_task['status']=='blocked' and len(cancelled_task['attempts'])==1
    ended=status('cancelled','completed')
    item=next(t for t in ended['tasks'] if t['id']=='task-2')
    assert item['attempt']['agent_id']=='cancel-managed' and item['attempt']['process_state']=='interrupted',item
    print('PASS managed-status-cancelled-identity-and-settlement',flush=True)
    print('PASS managed-manual-queued-cancellation-releases-without-provider',flush=True)
    proof=dict(status='PASS',root=str(root),work=f.work,completed='session',cancelled='cancel-managed',accepted=False,installed=False,refusal_recovery=faults,worker_git_renamed=detached_git,project_subdir=subdir,global_watchdog=None)
    fixture.publish(out/'result.json',proof);return proof


def main():
    """Use explicit immutable normal binary/profile; preserve complete failures."""
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--faults',action='store_true');p.add_argument('--detached-git',action='store_true');p.add_argument('--subproject',action='store_true')
    a=p.parse_args()
    try:run(a.binary.resolve(),a.profile.resolve(),a.output.resolve(),faults=a.faults,detached_git=a.detached_git,subproject=a.subproject)
    except BaseException as e:
        if a.output.is_dir():fixture.publish(a.output/'failure.json',dict(cause=''.join(traceback.format_exception(e))))
        raise
if __name__=='__main__':main()
