#!/usr/bin/env python3
"""Exercise native managed planning from actual committed copy to two todo tasks.

Two inert planner processes delegate and create tasks; no worker, managed source
exploration, SQL seeding, acceptance, paid model or global watchdog. Actual copy
absence must refuse before spending or a provider marker; original restoration
resumes the same owning child scope. Context binds candidate/capabilities without
source/storage paths. Historical status observes the two initial tasks under saved automatic consent,
without a conductor or copy creation. Every failure keeps original output.
"""
import argparse
import json
from pathlib import Path
import traceback
import rust_agent_model_route as fixture
import rust_managed_repository_bootstrap as repository
import rust_planning_rejection_recovery as feedback
from rust_planning_worker_public_flow import proposal
from rust_planning_bootstrap import fresh_work,request
from rust_planning_budget import ledger
from rust_planning_activation import claim_request
from rust_planning_call_preview import preview_request
from rust_planning_decision import decision
from rust_mission_report_relay import rows


def provider_markers(root):
    """Read owned fixture witnesses in historical cwd or engine-labelled I0 leaves.
    No file content promotes an action; callers still assert exact count, immutable
    source and public decisions. This does not establish engine confinement alone.
    """
    return set(root.glob('model-received-*.json')) | set(root.glob('.swarm/biba-i0/planning/*/model-received-*.json'))


def run(binary,profile,out,*,return_fixture=False,project_subdir="",controls=None,source_entries=None,pending_dependent=False):
    """Plan under public budgets/fences on a prepared real Git commit, never launch.

    Source commit uses the checkout's configured identity. Missing-copy injection
    only renames an owned directory and restores the original before retry. Work,
    financial ledger and provider markers must remain unchanged on refusal.
    Optional controls supplies explicit preauthorized fixture controls without
    changing the default handoff contract. Optional return_fixture exposes this same owned Root for a dependent public
    journey; its original default return remains the result alone.
    pending_dependent adds an explicitly budgeted third task depending on task,
    through the public planner before workers; it grants no launch or acceptance.
    The inert proposal input uses its explicit owned absolute path; provider
    output witnesses are read from the registered I0 leaf or historical cwd.
    Reports here prove actual local processes and context receipt, not model or
    agent authenticity, successful delivery, controls, review or acceptance.
    Optional source_entries supplies explicit committed fixture paths with bytes
    or ('symlink', literal_target), before any worker. It never patches a candidate
    or SQL and refuses escaping source names. None preserves the original tree.
    """
    out.mkdir(parents=True,mode=0o700,exist_ok=False)
    qualified=json.loads(profile.read_bytes());assert qualified['no_fixture_feature'] and fixture.digest(binary.read_bytes())==qualified['sha256']
    f=fixture.Fixture(binary,out/'setup','managed-planning')
    fixture.publish(out/'fixture.json',dict(root=str(f.root),binary_sha256=qualified['sha256']))
    source=f.root/'source';source.mkdir(mode=0o700);repository.git(source,'init','--quiet')
    checkout=Path(__file__).resolve().parents[1]
    name=repository.git(checkout,'config','user.name').decode().strip();email=repository.git(checkout,'config','user.email').decode().strip();assert name and email
    project=source/project_subdir
    if project_subdir:project.mkdir(mode=0o700)
    fixture.publish(project/'tracked.txt',b'Committed planning input\n')
    names=['tracked.txt']
    for name,value in (source_entries or {}).items():
        relative=Path(name)
        assert not relative.is_absolute() and relative.parts and all(p not in ['..','.git'] for p in relative.parts)
        path=project/relative;path.parent.mkdir(parents=True,exist_ok=True)
        if isinstance(value,tuple):
            assert len(value)==2 and value[0]=='symlink';path.symlink_to(value[1])
        else:fixture.publish(path,value)
        names.append(name)
    repository.git(source,'add','--',*[str((project/name).relative_to(source)) for name in names])
    repository.git(source,'-c','user.name='+name,'-c','user.email='+email,'-c','core.hooksPath=/dev/null','-c','commit.gpgsign=false','commit','--quiet','-m','Owned managed planning fixture')
    base=repository.git(source,'rev-parse','HEAD').decode().strip();fixture.publish(project/'tracked.txt',b'PRIVATE dirty source excluded\n')
    original=repository.inventory(source)
    program=feedback.PROGRAM.replace("open('planning-proposal.json')",'open('+repr(str(f.root/'planning-proposal.json'))+')')
    fixture.publish(f.root/'codex',program.encode());(f.root/'codex').chmod(0o700)
    f.work=fresh_work(f,'managed-planning-create')
    checks={'req-1':[dict(id='actual-handoff',command=['python3','-c',"from pathlib import Path; assert Path('handoff.md').read_text().strip()"],criteria=[1],inputs=['handoff.md'],justification='Check owned handoff content') ]}
    if controls is not None: checks={'req-1':controls}
    task_budget=3 if pending_dependent else 2
    work=f.ok(['planning','enable',f.work],request(event='managed-planning-enable',max_activations=6,max_tasks=task_budget,repository=dict(path=str(project.relative_to(f.root)),committed_only=True),checks=checks))
    proposal(f,work,'root',[dict(kind='delegate',id='child',title='Owned child scope',next='Plan the owned requirement',requirements=['req-1'],max_tasks=task_budget,max_activations=3)])
    first=f.ok(['planning','step',f.work]);assert first['run']['decision_applied']
    work=first['run']['work'];assert not work.get('tasks')
    assert any(s['id']=='child' and s['parent']=='root' for s in work['planning']['scopes'])
    checks_passed=[]
    def passed(label):
        """Publish named checks after their actual process/state conditions hold."""
        checks_passed.append(label);print('PASS '+label,flush=True)
    passed('managed-native-planner-delegates-without-worker')
    state=f.snapshot();money=ledger(f);markers=provider_markers(f.root)
    assert len(markers)==1
    bare=Path(work['planning']['repository']['storage'])/'repository.git'
    bare.rename(bare.with_name('repository.retained'))
    try:
        code,response=f.call(['planning','step',f.work])
        fixture.publish(out/'missing-copy-refusal.json',dict(exit=code,response=response))
        assert code!=0 and 'managed-repository-open' in json.dumps(response)
        assert f.snapshot()==state and ledger(f)==money and provider_markers(f.root)==markers
    finally:bare.with_name('repository.retained').rename(bare)
    passed('managed-missing-copy-refuses-without-spend-or-provider')
    operations=[dict(kind='task',id=task,title='Owned '+task,next='Produce handoff',requirements=['req-1'],deliverable='handoff.md',criteria=['Owned handoff']) for task in ['task','task-2']]
    if pending_dependent:
        operations.append(dict(kind='task',id='dependent',title='Owned dependent worker',next='Inspect published predecessor',requirements=['req-1'],deliverable='handoff.md',criteria=['Owned handoff'],depends=['task']))
    proposal(f,work,'child',operations)
    second=f.ok(['planning','step',f.work]);assert second['run']['decision_applied']
    work=second['run']['work'];tasks=work['tasks']
    assert {t['id'] for t in tasks}==({'task','task-2','dependent'} if pending_dependent else {'task','task-2'})
    assert all(t['scope_id']=='child' and t['status']=='todo' and not t.get('attempts') for t in tasks)
    assert len(provider_markers(f.root))==2 and repository.inventory(source)==original
    passed('managed-restored-copy-resumes-child-and-creates-three-todo-tasks' if pending_dependent else 'managed-restored-copy-resumes-child-and-creates-two-todo-tasks')
    protected_claims=[]
    for result in [first,second]:
        claim=result['run']['claim_id'];leaf=f.root/'.swarm/biba-i0/planning'/claim
        if leaf.exists():
            assert leaf.is_dir() and len(list(leaf.glob('model-received-*.json')))==1
            protected_claims.append(claim)
        actual=rows(f,'SELECT context_json FROM native_planning_claims WHERE claim_id=?',(result['run']['claim_id'],));assert len(actual)==1
        context=json.loads(actual[0][0]);meta=context['repository']
        assert meta==dict(candidate_commit=base,project_subdir=project_subdir,operations=['delegate','task','revise-instructions'])
        assert 'PRIVATE' not in json.dumps(context)
    if protected_claims:
        assert len(protected_claims)==2 and not list(f.root.glob('model-received-*.json'))
        passed('managed-planners-write-only-attributed-i0-leaves')
    passed('managed-candidate-capabilities-delivered-without-source-paths')
    before=f.snapshot();money=ledger(f)
    code,response=f.call(['agent','preview',f.work],dict(schema_version=1,event_id='managed-worker-preview',expected_revision=work['revision'],task_id='task',provider='fixture',capture_output=True))
    fixture.publish(out/'worker-still-refused.json',dict(exit=code,response=response))
    assert code!=0 and 'agent-launch-work-branch-unported' in json.dumps(response)
    assert f.snapshot()==before and ledger(f)==money and len(provider_markers(f.root))==2
    passed('managed-worker-launch-still-refuses-without-mutation')
    # Saving consent through CLI never starts a conductor. Historical status
    # must project the future managed copy, without creating it or spending.
    work=f.ok(['mission','start',f.work],dict(schema_version=1,event_id='managed-status-start',expected_revision=work['revision'],reason='Observe prepared managed tasks only',slots=1,profile=dict(provider='fixture',role='worker',workspace='source',level='standard',capture_output=True)))
    before=f.snapshot();money=ledger(f);markers=provider_markers(f.root)
    status=f.ok(['mission','status',f.work])
    fixture.publish(out/'managed-status.json',status)
    assert {t['id'] for t in status['tasks']}==({'task','task-2','dependent'} if pending_dependent else {'task','task-2'})
    assert all(t['state']=='waiting' and ('Attend Owned task' == t['reason'] if t['id']=='dependent' else 'aucun conducteur actif' in t['reason']) for t in status['tasks'])
    assert status['total']==task_budget and status['running']==0 and status['validated']==0
    assert not (Path(work['planning']['repository']['storage'])/'copies').exists()
    assert f.snapshot()==before and ledger(f)==money and provider_markers(f.root)==markers
    assert rows(f,'SELECT COUNT(*) FROM agents WHERE work_id=?',(f.work,))==[(0,)]
    passed('managed-status-three-unstarted-tasks-without-copy-spend-or-worker' if pending_dependent else 'managed-status-two-unstarted-tasks-without-copy-spend-or-worker')
    # A distinct owned Work exercises prepermission observation without inference.
    planned_work=f.work;f.work=fresh_work(f,'managed-preview-create')
    enabled=f.ok(['planning','enable',f.work],request(event='managed-preview-enable',repository=dict(path=str(project.relative_to(f.root)),committed_only=True)))
    claimed=f.ok(['planning','claim',f.work],claim_request(enabled,'managed-preview-claim'))
    value=preview_request(claimed,'managed-preview-claim')
    before=f.snapshot();money=ledger(f);markers=provider_markers(f.root)
    healthy=f.ok(['planning','call-preview',f.work],value)
    assert healthy['prepared_inputs_verified'] and healthy['process_started'] is False
    assert f.snapshot()==before and ledger(f)==money and provider_markers(f.root)==markers
    bare=Path(enabled['planning']['repository']['storage'])/'repository.git';bare.rename(bare.with_name('repository.retained'))
    try:
        code,response=f.call(['planning','call-preview',f.work],value)
        fixture.publish(out/'prepermission-missing-copy.json',dict(exit=code,response=response))
        assert code!=0 and 'managed-repository-open' in json.dumps(response)
        assert f.snapshot()==before and ledger(f)==money and provider_markers(f.root)==markers
    finally:bare.with_name('repository.retained').rename(bare)
    assert f.ok(['planning','call-preview',f.work],value)['prepared_inputs_verified']
    passed('managed-prepermission-reobserves-copy-without-mutation-or-inference')
    code,response=f.call(['planning','decide',f.work],decision(claimed,'managed-close-refusal',operations=[dict(kind='close')]))
    fixture.publish(out/'unported-operation-refusal.json',dict(exit=code,response=response))
    assert code!=0 and 'planning-decision-managed-operation-unported' in json.dumps(response)
    assert f.snapshot()==before and ledger(f)==money and provider_markers(f.root)==markers
    acknowledged=f.ok(['planning','decide',f.work],decision(claimed,'managed-ack'))
    assert not acknowledged.get('tasks') and acknowledged['planning']['decisions']==1
    passed('managed-unported-decision-refuses-and-acknowledgement-recovers')
    result=dict(status='PASS',assertions=checks_passed,binary_sha256=qualified['sha256'],actual_inert_planners=2,protected_planner_claims=protected_claims,manual_claims_without_inference=1,planned_work=planned_work,worker_started=False,accepted=False,installed=False,global_watchdog=None)
    fixture.publish(out/'result.json',result);print('MANAGED_PLANNING_PASS',flush=True)
    return (result,f) if return_fixture else result


def main():
    """Use exact explicit normal profile and preserve fresh original failure output."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args()
    try:run(args.binary.resolve(),args.profile.resolve(),args.output.resolve())
    except BaseException as error:
        if args.output.is_dir():fixture.publish(args.output/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(error))))
        raise


if __name__=='__main__':main()
