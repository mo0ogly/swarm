#!/usr/bin/env python3
"""Public Rust planner delegation -> worker -> handoff -> planner observation.

No Go-shaped seed, private SQL writes, paid provider, automatic conductor or
acceptance is used. Actual inert processes produce proposals and a report;
source/binary inputs and original failures are retained without global timeout.
"""
import argparse,json,os,sys,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_planning_rejection_recovery as feedback
import rust_manual_mission as mission
from rust_planning_bootstrap import request

def pending(work,scope):
    """Select real pending events for the intended scope, not fabricated IDs."""
    return [e['id'] for e in work['planning']['inbox'] if e['scope']==scope and not e.get('decision')]

def proposal(owned,work,scope,operations):
    """Supply owned inert model output; the engine still validates delivery/fences."""
    fixture.publish(owned.root/'planning-proposal.json',dict(input_events=pending(work,scope),reason='Explicit public integration fixture',operations=operations))

def flow(binary,out,authorized=True,control_script=None,control_inputs=None,extra_controls=(),deliverable="handoff.md",share_control_output=None,fixture_files=(),child_task_budget=1,child_activation_budget=3,before_worker=False,extra_worker=False,return_fixture=False):
    """Require public provenance, child isolation, real submission and next prompt.

    Explicit fixture budgets are persisted public inputs, not product policy or
    measured prices. The authorized case supplies an explicit handoff control;
    its timeout comes from saved policy and the command is not executed by this
    recipe. Afterwards the missing-control variant must refuse without mutation.
    Optional control_inputs declares fixture input paths; control_script replaces
    only the explicitly authorized fixture command before enable. deliverable and
    share_control_output override explicit fixture declarations, not product defaults.
    fixture_files creates explicit owned input bytes before authorization. extra_controls
    appends explicit fixture definitions in operator order; none execute here.
    child_task_budget is the explicit delegate-proposal fixture value, not a product
    default; the normal one-worker witness uses1. child_activation_budget is
    likewise an explicit saved delegation fixture input (legacy witness3), not
    a retry cap imposed by the recipe. Manual worker invocation
    and inert proposals prove no autonomous orchestration, model authenticity,
    completed controls or independent review. before_worker retains a genuine
    ready task setup without admitting a worker; its result is READY, not PASS.
    return_fixture requires before_worker and returns (ready, actual Fixture),
    preserving its private environment/command history without reconstructing it.
    This test context is not an identity, capability or launch grant.
    extra_worker proposes a second independent task under the explicit supplied
    child task budget; it never silently raises that budget.
    """
    if return_fixture and not before_worker:raise ValueError('return_fixture requires before_worker')
    owned=fixture.Fixture(binary,out,'public-worker-flow' if authorized else 'missing-controls')
    for name,raw in fixture_files:
        target=owned.root/name;assert target.resolve().is_relative_to(owned.root.resolve())
        target.parent.mkdir(parents=True,exist_ok=True);fixture.publish(target,raw)
    fixture.publish(owned.root/'codex',feedback.PROGRAM.encode());os.chmod(owned.root/'codex',0o700)
    work=owned.ok(['work','create'],dict(schema_version=1,event_id='public-flow-create',expected_revision=0,title='ROOT-PRIVATE-TITLE',objective='ROOT-PRIVATE-OBJECTIVE',scope='ROOT-PRIVATE-BRIEF',criteria=['OWN-REQUIREMENT']))['work'];owned.work=work['id']
    control_code=control_script if control_script is not None else "from pathlib import Path; import re; text=Path('handoff.md').read_text(); assert re.search(r'Input SHA256: [0-9a-f]{64}',text)"
    controls={'req-1':[dict(id='actual-handoff',command=['python3','-c',control_code],criteria=[1],inputs=['handoff.md'] if control_inputs is None else control_inputs,review_output=(control_script is not None) if share_control_output is None else share_control_output,justification='Check actual owned fixture handoff input digest')]}
    controls['req-1'].extend(dict(c) for c in extra_controls)
    work=owned.ok(['planning','enable',owned.work],request(event='public-flow-enable',max_activations=6,checks=controls if authorized else None))
    proposal(owned,work,'root',[dict(kind='delegate',id='child',title='OWN-SCOPE',next='Produce only the owned requirement',requirements=['req-1'],max_tasks=child_task_budget,max_activations=child_activation_budget)])
    first=owned.ok(['planning','step',owned.work]);assert first['run']['decision_applied'];work=first['run']['work'];assert not work.get('tasks')
    child=next(s for s in work['planning']['scopes'] if s['id']=='child');assert child['parent']=='root' and child['requirements']==['req-1']
    before=set(owned.root.glob('fixture-prompt-*.bin'))
    operations=[dict(kind='task',id='task',title='OWN-TASK',next='Produce the actual local handoff',requirements=['req-1'],deliverable=deliverable,criteria=['OWN-CRITERION'])]
    if extra_worker:operations.append(dict(kind='task',id='task-2',title='SECOND-OWN-TASK',next='Produce the second owned handoff',requirements=['req-1'],deliverable='second/handoff.md',criteria=['OWN-CRITERION']))
    proposal(owned,work,'child',operations)
    second=owned.ok(['planning','step',owned.work]);assert second['run']['decision_applied'];work=second['run']['work'];task=work['tasks'][0];assert task['scope_id']=='child' and task['plan_role']=='worker'
    prompts=set(owned.root.glob('fixture-prompt-*.bin'))-before;assert len(prompts)==1;child_prompt=next(iter(prompts)).read_bytes();assert b'OWN-SCOPE' in child_prompt and b'ROOT-PRIVATE-TITLE' not in child_prompt and b'ROOT-PRIVATE-OBJECTIVE' not in child_prompt
    work=owned.ok(['budget','apply',owned.work],dict(schema_version=1,event_id='public-worker-budget',expected_revision=work['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=1,estimate_source='Owned inert integration fixture, not measured pricing',reference_date='2026-10-21')))
    fixture.publish(owned.root/'codex',mission.PROGRAM.encode());os.chmod(owned.root/'codex',0o700)
    owned.request=dict(schema_version=1,event_id='session',expected_revision=work['revision'],task_id='task',provider='fixture',capture_output=True)
    if before_worker:
        assert authorized and not (owned.root/'model-received.json').exists()
        ready=dict(status='READY_FOR_NATIVE_WORKER',root=str(owned.root),work_id=owned.work,revision=work['revision'],worker_started=False,accepted=False)
        return (ready,owned) if return_fixture else ready
    if not authorized:
        before=owned.snapshot();code,wire=owned.call(['agent','preview',owned.work],owned.request)
        assert code!=0 and wire['failure']['code']=='organization_incomplete',wire
        assert 'organization-requirement-controls-missing' in wire['failure']['causes'],wire
        assert owned.snapshot()==before and owned.reservation() is None and not (owned.root/'model-received.json').exists()
        return dict(status='PASS',root=str(owned.root),missing_controls_refused=True,no_worker_start=True,no_mutation=True)
    preview=owned.prepare(preview=True);assert preview['preview_only'] and not preview['process_started'] and owned.reservation() is None
    owned.request['model_policy_hash']=preview['agent']['model_route']['policy_hash'];prepared=owned.prepare();prompt=prepared['agent']['prompt']
    assert all(marker in prompt for marker in ['OWN-SCOPE','OWN-TASK','OWN-CRITERION','OWN-REQUIREMENT'])
    assert all(marker not in prompt for marker in ['ROOT-PRIVATE-TITLE','ROOT-PRIVATE-OBJECTIVE','ROOT-PRIVATE-BRIEF'])
    actual=owned.run_and_assert(prepared);report=(owned.root/'handoff.md').read_bytes();assert actual['stdin_sha256'].encode() in report
    mission.submit(owned);work=owned.ok(['work','show',owned.work])['work'];assert work['tasks'][0]['status']=='submitted' and len(work['tasks'][0]['attempts'])==1
    events=[e for e in work['planning']['inbox'] if e['kind'] in ['attempt_ended','handoff']];assert len(events)==2 and all(e['scope']=='child' and e['task']=='task' and e['attempt']==prepared['agent']['attempt_id'] for e in events)
    handoff=next(e for e in events if e['kind']=='handoff');assert handoff['handoff']==dict(path='handoff.md',sha256=fixture.digest(report))
    fixture.publish(owned.root/'codex',feedback.PROGRAM.encode());os.chmod(owned.root/'codex',0o700);before=set(owned.root.glob('fixture-prompt-*.bin'));proposal(owned,work,'child',[])
    third=owned.ok(['planning','step',owned.work]);assert third['run']['decision_applied'];current=third['run']['work'];assert current['tasks'][0]['status']=='submitted' and current['planning']['reviewer'].get('calls',0)==0
    fresh=set(owned.root.glob('fixture-prompt-*.bin'))-before;assert len(fresh)==1;received=next(iter(fresh)).read_bytes()
    with feedback.native.database(owned) as db:
        row=db.execute('SELECT context_json FROM native_planning_claims WHERE claim_id=?',[third['run']['claim_id']]).fetchone();assert row
    assert received.endswith(row[0].encode());context=json.loads(row[0]);assert context['scope']['id']=='child'
    actual_reports=[r for r in context['reports'] if r['event_id']==handoff['id']];assert len(actual_reports)==1 and actual_reports[0]['text']==report.decode() and actual_reports[0]['sha256']==fixture.digest(report)
    assert all(e['decision'] for e in current['planning']['inbox'] if e['id'] in {event['id'] for event in events})
    return dict(status='PASS',root=str(owned.root),work_id=owned.work,revision=current['revision'],agent_id='session',attempt_id=prepared['agent']['attempt_id'],public_delegation=True,child_prompt_isolated=True,real_worker_and_report=True,handoff_received_by_planner=True,private_sql_writes=False,go_seed=False,autonomous_conductor=False,independent_review=False,accepted=False)

def main():
    """Retain exact helpers/default binary and fresh fixture; never delete failures."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(exist_ok=False);binary=a.binary.resolve()
    modules=[sys.modules[f.__module__] for f in [feedback.fresh_work,feedback.request,feedback.ledger,feedback.apply_policy]];sources=list(dict.fromkeys([Path(__file__),Path(fixture.__file__),Path(feedback.__file__),Path(feedback.native.__file__),Path(mission.__file__)]+[Path(m.__file__) for m in modules]));inputs=dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(p):fixture.digest(p.read_bytes()) for p in sources});fixture.publish(out/'inputs.json',inputs)
    try:
        checks=[flow(binary,out),flow(binary,out,False)];assert fixture.digest(binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(n).read_bytes())==h for n,h in inputs['sources'].items());result=dict(status='PASS',checks=checks,paid_provider=False,global_watchdog=None);fixture.publish(out/'result.json',result);print(json.dumps(result))
    except BaseException:fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise

if __name__=='__main__':main()
