#!/usr/bin/env python3
"""Completed manual native worker consumed by automatic mission reviewer.

The existing public planner setup is reused. Before worker/planner/reviewer native
preflight, one combined inert executable is fixed for all three roles; it is never
rewritten during either execution. Report submission and controls use public
operations, then only mission watch may admit review. No SQL writes or fake
receipts, legacy provenance upgrade, paid provider or global watchdog.
"""
import argparse,ast,json,os,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as flow
import rust_manual_mission as manual
import rust_mission_watch_planning as planning
import rust_mission_report_relay as relay
import rust_mission_watch_worker as worker
import rust_native_parent_death as parent_death
check=worker.check

def provider_program():
    """Freeze three inert roles before preflight, selected by actual wire input.

    Reviewer uses the existing explicit contract marker. Planner consumes only
    the final scope context appended by planning_run_material::prompt and emits
    an empty-operation acknowledgement of exactly its delivered events. It never
    reads a stale proposal file or mutates Work. Other input uses the real manual
    worker fixture. Malformed planner context raises, rather than falling back to
    a successful worker. These markers select fixture code, not product rights.
    """
    tree=ast.parse(Path(planning.__file__).read_text())
    prefixes=[n.value for n in ast.walk(tree) if isinstance(n,ast.Constant) and isinstance(n.value,str) and n.value.startswith('import io,json,hashlib,os,sys\n')]
    check(len(prefixes)==1,'reviewer fixture prefix boundary changed')
    original="proposal=json.load(open('planning-proposal.json'))"
    check(flow.feedback.PROGRAM.count(original)==1,'planner fixture boundary changed')
    replacement="""context=json.loads(raw.rsplit(b'\\n\\n',1)[1])
assert isinstance(context,dict) and isinstance(context['scope']['id'],str)
assert isinstance(context['events'],list)
assert all(isinstance(e['id'],str) and e['scope']==context['scope']['id'] for e in context['events'])
proposal=dict(input_events=[e['id'] for e in context['events']],reason='Owned inert current delivery acknowledgement; no task acceptance',operations=[])"""
    planner=flow.feedback.PROGRAM.replace(original,replacement)
    dispatch=("if "+repr('Tu es le planificateur du périmètre reçu.'.encode())+" in _fixture_raw:\n"
        +" exec(compile("+repr(planner)+",'<owned-manual-planner>','exec'))\n"
        +"else:\n exec(compile("+repr(manual.PROGRAM)+",'<owned-manual-worker>','exec'))\n")
    program='#!/usr/bin/python3\n'+prefixes[0].replace('REVIEW_FAULT','False').replace('REVIEW_GATE','False')+dispatch
    compile(program,'owned-manual-review-provider','exec')
    return program


def prepare(binary,out,manual_controls=True):
    """Prepare actual manual worker/report/controls and explicit mission consent.
    Return an inactive owned Root before any automatic review admission; this
    preparation is not PASS or review authority. Executable bytes stay fixed.
    manual_controls retains the earlier explicit control witness; false delegates
    the first actual batch to watch, without seeding a validation receipt.
    """
    setup=out/'setup';setup.mkdir();ready,owned=flow.flow(binary,setup,before_worker=True,return_fixture=True)
    program=provider_program()
    fixture.publish(owned.root/'codex',program.encode());os.chmod(owned.root/'codex',0o700);program_sha=fixture.digest(program.encode())
    preview=owned.prepare(preview=True);owned.request['model_policy_hash']=preview['agent']['model_route']['policy_hash'];prepared=owned.prepare();owned.run_and_assert(prepared);manual.submit(owned)
    if manual_controls:
        current=relay.work(owned);done=owned.ok(['task','validate',owned.work],dict(schema_version=1,event_id='manual-source-controls',expected_revision=current['revision'],task_id='task'));check(done['controls_passed'],'manual source controls failed')
    agent=owned.ok(['agent','show',prepared['agent']['id']])['agent'];check(agent.get('native_mission') is None and agent['status']=='completed','source is not genuinely manual completed worker')
    current=relay.work(owned);calls=current['planning']['reviewer'].get('calls',0);finance_before=relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))
    profile={key:agent[key] for key in ['provider','role','level'] if key in agent}
    profile.update(workspace=str(Path(agent['workspace']).relative_to(owned.root)),capture_output=True)
    owned.ok(['mission','start',owned.work],dict(schema_version=1,event_id='manual-source-start',expected_revision=current['revision'],reason='Explicit automatic review of completed native manual source',slots=1,profile=profile))
    fixture.publish(out/'fixture.json',dict(root=str(owned.root),work=owned.work,binary_sha256=fixture.digest(binary.read_bytes())))
    return owned,agent,calls,finance_before,program_sha


def run(binary,out,manual_controls=True):
    """Generate a real manual current producer, submit and validate its report,
    then require automatic reviewer to preserve attempts and original funding.
    No labels or serialized lease fields confer automatic authority.
    """
    owned,agent,calls,finance_before,program_sha=prepare(binary,out,manual_controls=manual_controls)
    events=relay.cycle(owned,out/'watch',completed_event='mission-review-recorded');current=relay.work(owned);task=next(t for t in current['tasks'] if t['id']=='task');review=task['independent_review']
    check(review['state']=='passed' and review['producer']==agent['id'] and task['status']!='accepted','manual-origin automatic review missing or fabricated acceptance')
    check(len([e for e in events if e.get('event')=='mission-review-recorded' and e.get('verdict_applicable')])==1,'missing actual automatic review event')
    if not manual_controls:
        check(not any(c['argv'][1:3]==['task','validate'] for c in owned.calls),'fixture manually admitted automatic control')
        validations=[e for e in events if e.get('event')=='mission-controls-recorded'];check(len(validations)==1 and validations[0]['controls_passed'] and not validations[0]['task_accepted'] and task['automatic_validation']['controls'][0]['executed'],'missing actual first automatic native controls')
        review_event=next(e for e in events if e.get('event')=='mission-review-recorded');check(events.index(validations[0])<events.index(review_event) and validations[0]['revision']<review_event['revision'],'review preceded actual first controls')
        validation=task['automatic_validation'];receipt=json.loads((owned.root/validation['receipt']).read_bytes());check(receipt['job_id']==validations[0]['job_id'] and receipt['producer_agent_id']==agent['id'] and validation['artifacts']['handoff.md']==fixture.digest((owned.root/'handoff.md').read_bytes()),'automatic controls not bound to original manual producer/report')
    check(owned.ok(['agent','show',agent['id']])['agent']==agent and fixture.digest((owned.root/'codex').read_bytes())==program_sha,'automatic review changed producer or executable')
    finance_after=relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,));check(all(r in finance_after for r in finance_before) and current['planning']['reviewer']['calls']==calls+1,'manual origin review lost prior funding or repeated debit')
    witness=json.loads((owned.root/'reviewer-invoked.json').read_bytes());jobs=relay.rows(owned,'SELECT producer_id,permission,phase,prompt_sha256,owner_pid FROM native_review_jobs WHERE id=?',(review['id'],));check(len(jobs)==1 and jobs[0]==(agent['id'],1,'recorded',witness['stdin_sha256'],witness['ppid']),'actual reviewer is not bound to canonical manual producer job')
    fixture.publish(out/'public-work.json',current);fixture.publish(out/'finance-before.json',finance_before);fixture.publish(out/'finance-after.json',finance_after)
    return dict(status='PASS',root=str(owned.root),work=owned.work,manual_native_producer=True,automatic_controls=not manual_controls,automatic_reviewer=True,task_accepted=False,installed=False)

def main():
    """Bind execution to the actual default profile, retain failure and restore
    owned subreaper. Missing manual provenance remains refusal, not acceptance.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:p.add_argument('--'+flag,type=Path,required=True)
    p.add_argument('--automatic-controls',action='store_true',help='Require watch to admit the first native controls without manual task.validate')
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True);binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes());check(profile['no_fixture_feature'] and profile['sha256']==fixture.digest(binary.read_bytes()),'manual-review profile mismatch')
    result=worker.subreaper_scope(lambda:run(binary,out,manual_controls=not a.automatic_controls),lambda e:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(e)))),parent_death.subreaper)
    fixture.publish(out/'result.json',result);print('PASS completed manual native worker receives automatic mission review without acceptance',flush=True)
if __name__=='__main__':main()
