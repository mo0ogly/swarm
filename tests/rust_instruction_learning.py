#!/usr/bin/env python3
"""Verify attributed instruction learning through the normal CLI/API/cockpit.

Owned inert Roots only; no live database mutation or model call. Original command
bytes and refused/recovered states survive. No global watchdog, retry or business
acceptance is inferred. Browser credentials are stdin-only, never retained.
"""
import argparse,copy,hashlib,json,subprocess,traceback
from pathlib import Path
import rust_agent_model_route as f
from rust_planning_decision import start,decision
from rust_planning_bootstrap import fresh_work,request
from rust_planning_activation import claim_request,apply_policy
from rust_native_http_cli import Server


def check(name,value):
    """Emit a named assertion only after its meaningful state predicate succeeds."""
    assert value,name
    print('PASS '+name,flush=True)


def learned(owned,label,controls=False):
    """Create a public future task and its attributed addition in one leased decision."""
    if controls:
        owned.work=fresh_work(owned,label+'-create')
        enabled=owned.ok(['planning','enable',owned.work],request(event=label+'-enable',checks={'req-1':[dict(id='original-check',command=['git','diff','--check'],criteria=[1],justification='Verify the authorized original requirement')]}))
        claimed=owned.ok(['planning','claim',owned.work],claim_request(enabled,label+'-claim'))
    else:claimed=start(owned,label)
    operations=[dict(kind='task',id='future',title='Future controlled task',requirements=['req-1'],deliverable='Evidence',criteria=['Evidence'],next='Original operator instructions'),dict(kind='revise-instructions',id='future',next='Remember the verified control')]
    value=decision(claimed,label+'-decide',operations)
    work=owned.ok(['planning','decide',owned.work],value)
    task=work['tasks'][0];revision=task['instruction_revisions'][0]
    check(label+'-base-attribution-hashes',task['instruction_base']==('Original operator instructions'+('\nPrécisions de réalisation proposées : Evidence' if controls else '')) and task['next']==revision['addition']+'\n\n'+task['instruction_base'] and revision['holder']==value['holder'] and revision['input_events']==value['input_events'] and revision['id']=='instruction-'+hashlib.sha256((value['event_id']+'|future').encode()).hexdigest()[:24] and revision['before_sha256']==hashlib.sha256(task['instruction_base'].encode()).hexdigest() and revision['after_sha256']==hashlib.sha256(task['next'].encode()).hexdigest())
    check(label+'-validation-contract-preserved',task['status']=='todo' and ((task['plan_checks']['plan-validation']=='validation' and task.get('validation_policy') is None and task['criteria']==['Evidence']) if not controls else task['validation_policy']['mode']=='automatic' and task['criteria']==['Configuration is persisted without model execution']))
    return work,value


def rejection(work,label,**patch):
    """Use the actual Work/revision/revision ID; no guessed authority or scope."""
    value=dict(schema_version=1,event_id=label,expected_revision=work['revision'],task_id='future',instruction_revision_id=work['tasks'][0]['instruction_revisions'][0]['id'],reason='Operator declines this unused learning')
    value.update(patch);return value



def compare_go(binary,out,mode,work,value,actual,operation=None):
    """Compare actual task/planning JSON to extracted historical Go mutators.

    Clock is supplied from Rust's observed timestamp. Transaction revision/time
    and SQL authority are verified by public tests separately; no allowlist hides
    task or inbox differences. No Go caller identity or SQL replay is inferred.
    """
    if binary is None:return
    at=actual['tasks'][0]['instruction_revisions'][0]['at'] if mode=='revise' else actual['tasks'][0]['instruction_revisions'][0]['rejected_at']
    payload=dict(Mode=mode,Work=work,Request=value,Operation=operation or {},At=at)
    result=subprocess.run([str(binary)],input=json.dumps(payload).encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    f.publish(out/('go-'+mode+'.input.json'),payload);f.publish(out/('go-'+mode+'.stdout'),result.stdout);f.publish(out/('go-'+mode+'.stderr'),result.stderr)
    assert result.returncode==0,result.stderr
    oracle=json.loads(result.stdout);check('Go-Rust-'+mode+'-task-equality',not oracle['Refused'] and oracle['Work']['tasks']==actual['tasks'])
    if mode=='reject':check('Go-Rust-reject-planning-equality',oracle['Work']['planning']==actual['planning'])


def refusals(owned,work,reference=None,out=None):
    """A successful learning witness precedes refusals; each must leave SQL/Work intact."""
    for label,patch,expected in [
        ('empty-reason',dict(reason=' '),'instruction-operator-rejection-invalid'),
        ('reason-bytes',dict(reason='é'*2001),'instruction-operator-rejection-invalid'),
        ('ai-holder',dict(holder='planner'),'instruction-operator-rejection-invalid'),
        ('ai-input',dict(input_events=['bootstrap']),'instruction-operator-rejection-invalid'),
        ('ai-operation',dict(operations=[dict(kind='task',id='unwanted')]),'instruction-operator-rejection-invalid'),
        ('wrong-scope',dict(scope='outside'),'instruction-scope-changed'),
        ('wrong-task',dict(task_id='absent'),'instruction-task-not-found'),
        ('wrong-addition',dict(instruction_revision_id='absent'),'instruction-revision-not-found'),
        ('stale',dict(expected_revision=work['revision']-1),'revision_conflict'),
    ]:
        before=owned.snapshot();code,value=owned.call(['planning','reject-instructions',owned.work],rejection(work,label,**patch))
        check('reject-'+label+'-atomic',code!=0 and value['failure']['code']==expected and owned.ok(['work','show',owned.work])['work']==work and owned.snapshot()==before)
        if reference is not None and label!='stale':
            payload=dict(Mode='reject',Work=work,Request=rejection(work,label,**patch),At=work['updated'])
            result=subprocess.run([str(reference)],input=json.dumps(payload).encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            f.publish(out/('go-refusal-'+label+'.input.json'),payload);f.publish(out/('go-refusal-'+label+'.stdout'),result.stdout);f.publish(out/('go-refusal-'+label+'.stderr'),result.stderr)
            assert result.returncode==0,result.stderr
            oracle=json.loads(result.stdout);check('Go-Rust-refusal-'+label,oracle['Refused'] and oracle['Work']==work)


def native_learning(binary,out):
    """Receive actual native prompts: create → learn → operator reject → acknowledge.

    Three inert local planner processes return explicit fixture proposals. Exact
    stdin witnesses prove receipt of the operation contract and rejection event,
    not real model reasoning, human identity or autonomous engine acceptance.
    """
    import os
    from rust_planning_rejection_recovery import PROGRAM,pending
    owned=f.Fixture(binary,out,'native-learning')
    f.publish(owned.root/'codex',PROGRAM.encode());os.chmod(owned.root/'codex',0o700)
    owned.work=fresh_work(owned,'native-learning-create')
    w=owned.ok(['planning','enable',owned.work],request(event='native-learning-enable'))
    operations=[dict(kind='task',id='future',title='Future native task',requirements=['req-1'],deliverable='Evidence',criteria=['Evidence'],next='Original native instructions')]
    f.publish(owned.root/'planning-proposal.json',dict(input_events=pending(w),reason='Native fixture creates future task',operations=operations))
    created=owned.ok(['planning','step',owned.work]);check('native-planner-create-applied',created['run']['decision_applied'] and not created['accepted']);w=created['run']['work']
    w=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='native-learning-feedback',expected_revision=w['revision'],reason='Verified native feedback for a future task'))
    before=set(owned.root.glob('fixture-prompt-*.bin'))
    f.publish(owned.root/'planning-proposal.json',dict(input_events=pending(w),reason='Learn from the received feedback',operations=[dict(kind='revise-instructions',id='future',next='Native attributed learning observation')]))
    learned=owned.ok(['planning','step',owned.work]);w=learned['run']['work'];fresh=set(owned.root.glob('fixture-prompt-*.bin'))-before
    check('native-learning-decision-applied',learned['run']['decision_applied'] and w['tasks'][0]['next']=='Native attributed learning observation\n\nOriginal native instructions')
    check('native-learning-contract-actually-received',len(fresh)==1 and b'revise-instructions' in next(iter(fresh)).read_bytes() and b'instruction_rejected' in next(iter(fresh)).read_bytes())
    rejected=owned.ok(['planning','reject-instructions',owned.work],rejection(w,'native-operator-rejection',reason='NATIVE-OPERATOR-DECLINES-THIS-LEARNING'))
    before=set(owned.root.glob('fixture-prompt-*.bin'))
    f.publish(owned.root/'planning-proposal.json',dict(input_events=pending(rejected),reason='Acknowledge the human rejection without reintroducing learning',operations=[]))
    acknowledged=owned.ok(['planning','step',owned.work]);w=acknowledged['run']['work'];fresh=set(owned.root.glob('fixture-prompt-*.bin'))-before
    check('native-rejection-actually-received',len(fresh)==1 and b'NATIVE-OPERATOR-DECLINES-THIS-LEARNING' in next(iter(fresh)).read_bytes() and b'native-operator-rejection' in next(iter(fresh)).read_bytes())
    check('native-rejection-acknowledged-preserves-base',acknowledged['run']['decision_applied'] and not acknowledged['accepted'] and w['tasks'][0]['next']=='Original native instructions' and next(e for e in w['planning']['inbox'] if e['id']=='native-operator-rejection')['decision'])
    f.publish(out/'native-result.json',dict(status='PASS',inert_planner_processes=3,real_model=False,global_acceptance=False,root=str(owned.root)))


def main():
    """Exercise replay, scoped refusals, rejection event, freeze and protected UI."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--browser',action='store_true');p.add_argument('--go-reference',type=Path);a=p.parse_args()
    out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);binary=a.binary.resolve();server=None
    f.publish(out/'inputs.json',dict(binary=str(binary),sha256=f.digest(binary.read_bytes()),global_watchdog=None))
    try:
        owned=f.Fixture(binary,out,'instruction');witness,_=learned(owned,'rejection-witness');positive=owned.ok(['planning','reject-instructions',owned.work],rejection(witness,'rejection-positive'));check('rejection-valid-before-faults',positive['tasks'][0]['instruction_revisions'][0]['state']=='rejected')
        work,value=learned(owned,'initial')
        original=copy.deepcopy(work);task=original['tasks'][0];task['next']=task.pop('instruction_base');task.pop('instruction_revisions');
        compare_go(a.go_reference,out,'revise',original,value,work,value['operations'][1])
        check('decision-replay-exact',owned.ok(['planning','decide',owned.work],value)==work)
        refusals(owned,work,a.go_reference,out)
        value=rejection(work,'operator-reject');rejected=owned.ok(['planning','reject-instructions',owned.work],value)
        compare_go(a.go_reference,out,'reject',work,value,rejected)
        check('rejected-base-history-inbox',rejected['tasks'][0]['next']==work['tasks'][0]['instruction_base'] and rejected['tasks'][0]['instruction_revisions'][0]['state']=='rejected' and rejected['tasks'][0]['instruction_revisions'][0]['rejected_reason']==value['reason'] and rejected['planning']['inbox'][-1]['kind']=='instruction_rejected' and rejected['planning']['inbox'][-1]['id']==value['event_id'])
        check('rejection-replay-exact',owned.ok(['planning','reject-instructions',owned.work],value)==rejected)
        before=owned.snapshot();code,wire=owned.call(['planning','reject-instructions',owned.work],rejection(rejected,'second-reject'));check('second-rejection-refuses',code!=0 and wire['failure']['code']=='instruction-revision-already-used-or-rejected' and owned.snapshot()==before)
        # Pending rejection is actually delivered to the next supervisor context.
        scope=next(s for s in rejected['planning']['scopes'] if s['id']=='root')
        claimed=owned.ok(['planning','claim',owned.work],dict(schema_version=1,event_id='rejection-claim',expected_revision=rejected['revision'],scope='root',scope_revision=scope['revision'],holder='next-planner',lease_seconds=60))
        delivery=next(s for s in claimed['planning']['scopes'] if s['id']=='root')['delivery'];check('rejection-delivered-to-supervisor','operator-reject' in delivery['events'])
        policy=owned.ok(['engine-settings','show'])['values']['planning_decision_max_instruction_bytes']
        apply_policy(owned,'learning-policy-zero',dict(planning_decision_max_instruction_bytes='0'))
        before=owned.snapshot();code,wire=owned.call(['planning','decide',owned.work],decision(claimed,'learning-policy-decision',[dict(kind='revise-instructions',id='future',next='Policy-limited learning')]))
        check('saved-instruction-policy-zero-refuses',code!=0 and wire['failure']['code']=='planning-operation-instruction-limit' and owned.snapshot()==before)
        apply_policy(owned,'learning-policy-restore',dict(planning_decision_max_instruction_bytes=policy))
        restored=owned.ok(['planning','decide',owned.work],decision(claimed,'learning-policy-decision',[dict(kind='revise-instructions',id='future',next='Policy-limited learning')]))
        check('saved-instruction-policy-restored-recovery',restored['tasks'][0]['instruction_revisions'][-1]['addition']=='Policy-limited learning')
        # Public resume supplies a new input for the separate duplicate/refusal case.
        restored=owned.ok(['planning','resume',owned.work],dict(schema_version=1,event_id='duplicate-input',expected_revision=restored['revision'],reason='Verify refusal of a previously rejected addition'))
        scope=next(s for s in restored['planning']['scopes'] if s['id']=='root')
        claimed=owned.ok(['planning','claim',owned.work],dict(schema_version=1,event_id='duplicate-claim',expected_revision=restored['revision'],scope='root',scope_revision=scope['revision'],holder='duplicate-planner',lease_seconds=60))
        before=owned.snapshot();code,wire=owned.call(['planning','decide',owned.work],decision(claimed,'duplicate-decision',[dict(kind='revise-instructions',id='future',next='Remember the verified control')]))
        check('rejected-addition-cannot-be-reintroduced',code!=0 and wire['failure']['code']=='instruction-addition-already-recorded' and owned.snapshot()==before)
        # Same refused decision event recovers with a different, attributed learning.
        recovered=owned.ok(['planning','decide',owned.work],decision(claimed,'duplicate-decision',[dict(kind='revise-instructions',id='future',next='A distinct verified observation')]))
        check('refused-decision-recovery',len(recovered['tasks'][0]['instruction_revisions'])==3)
        # Reservation freezes revisions before any process is spawned.
        recovered,_=learned(owned,'reservation',controls=True)
        budgeted=owned.ok(['budget','apply',owned.work],dict(schema_version=1,event_id='learning-budget',expected_revision=recovered['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=1,estimate_source='Inert instruction fixture',reference_date='2026-10-25')))
        prepared=owned.ok(['agent','prepare',owned.work],dict(schema_version=1,event_id='learning-attempt',expected_revision=budgeted['revision'],task_id='future',provider='fixture',capture_output=True))
        frozen=owned.ok(['work','show',owned.work])['work'];check('reservation-freezes-applied-learning',bool(frozen['tasks'][0]['instruction_revisions'][0].get('used_by_attempt')))
        before=owned.snapshot();r=rejection(frozen,'used-reject',instruction_revision_id=frozen['tasks'][0]['instruction_revisions'][0]['id']);code,wire=owned.call(['planning','reject-instructions',owned.work],r)
        check('reserved-learning-refuses-rejection',code!=0 and owned.snapshot()==before and owned.ok(['work','show',owned.work])['work']==frozen)
        # Each theme/language receives an independent unused revision for a real POST.
        ui_works=[]
        for label in ['fr-etat','fr-sombre','en-etat','en-sombre','lost-response','stale-browser','concurrent-refresh']:
            w,_=learned(owned,'ui-'+label,controls=True);ui_works.append(w['id'])
        server=Server(binary,owned.root,out,'instruction-web',environment=owned.environment)
        server.capture_async(out/'web-events.private.jsonl')
        session=server.request('/api/v1/native-session',observe=False);csrf=session['csrf_token']
        owned.work=ui_works[0];w=owned.ok(['work','show',owned.work])['work'];before=owned.snapshot()
        server.request('/api/v1/planning?work='+owned.work+'&action=reject-instructions',rejection(w,'csrf-refusal'),authorized=True,status=403,observe=False)
        check('HTTP-CSRF-refuses-no-mutation',owned.snapshot()==before)
        api=server.request('/api/v1/planning?work='+owned.work,observe=False);check('HTTP-current-history',api==w)
        # Actual served bytes, including the newly embedded widget and translation.
        import http.client
        for route,source in [('/cockpit-native-instructions.js','web/cockpit-native-instructions.js'),('/cockpit-native.js','web/cockpit-native.js'),('/i18n-en.js','web/i18n-en.js')]:
            c=http.client.HTTPConnection(server.host,server.port,timeout=None)
            try:
                c.request('GET',route,headers={'Cookie':server.cookie});response=c.getresponse();body=response.read();check('served-bytes-'+route,response.status==200 and body==Path(source).read_bytes())
            finally:c.close()
        if a.browser:
            cfg=dict(address=server.address,key=server.key,root=str(owned.root),output=str(out),works=ui_works)
            with (out/'browser.stdout').open('xb') as browser_out,(out/'browser.stderr').open('xb') as browser_error:
                result=subprocess.run(['node','tests/rust_instruction_learning_ui.cjs'],input=json.dumps(cfg).encode(),stdout=browser_out,stderr=browser_error)
            check('browser-journey',result.returncode==0)
        # Reproduce stdout backpressure using the observed pipe capacity. Each
        # extra protected read awaits actual stream progress, without a deadline
        # or fixed request count. This is a fixture witness, not product policy.
        if a.browser and server.capture_capacity is not None:
            while server.capture_bytes<=server.capture_capacity:
                before=server.capture_bytes
                server.request('/api/v1/native-session',observe=False)
                server.wait_capture_progress(before)
        f.publish(out/'capture-observation.json',dict(bytes=server.capture_bytes,pipe_capacity=server.capture_capacity,errors=len(server.capture_errors)))
        check('no-model-process',not (owned.root/'model-received.json').exists())
        native_learning(binary,out/'native-planner')
        f.publish(out/'result.json',dict(status='PASS',browser=a.browser,provider_called=False,binary_sha256=f.digest(binary.read_bytes())))
    except BaseException:
        f.publish(out/'failure.txt',traceback.format_exc().encode());raise
    finally:
        if server is not None:
            server.cleanup()
            check('web-event-capture-valid',not server.capture_errors and server.capture_bytes>0)
            if a.browser and server.capture_capacity is not None:check('web-event-capture-exceeds-original-pipe',server.capture_bytes>server.capture_capacity)

if __name__=='__main__':main()
