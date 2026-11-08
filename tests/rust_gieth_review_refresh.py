#!/usr/bin/env python3
"""Verify stale untracked inputs refuse, then renew actual controls/review publicly.

Two owned normal-profile fixtures, inert providers, no SQL writes or paid models.
Every original and negative result is retained. No global watchdog or retry; a
security failure never becomes PASS through output wording or restored evidence.
"""
import argparse,json,os,subprocess,traceback
from pathlib import Path
import rust_gieth_effective_dependency as prior



def api_refusals(binary,root,work,refresh_request,out):
    """Prove valid refresh replay before malformed/revision refusals after R2.

    Uses the actual fixture's archived refresh request, never invented positive
    SQL or a new call. Retains all channels and requires unchanged Work/finance.
    Unknown fields must hit the typed schema guard, not another task-state error.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    before=prior.shown(binary,root,out,'before',work);finance=prior.native.finance(root,work)
    code,value=prior.call(binary,root,out,'valid-replay',['task','review-refresh',work],refresh_request)
    assert code==0 and value['replayed'] and value['work']==before
    cases=[('bad-revision',dict(schema_version=1,event_id='extra-wrong-revision',expected_revision=before['revision']+1,task_id='task'),'review-refresh-revision-conflict'),('unknown-field',dict(schema_version=1,event_id='extra-unknown',expected_revision=before['revision'],task_id='task',allow=True),'task-validation-request-invalid')]
    for label,request,reason in cases:
        code,value=prior.call(binary,root,out,label,['task','review-refresh',work],request)
        assert code!=0 and reason in json.dumps(value),value
        assert prior.shown(binary,root,out,label+'-after',work)==before and prior.native.finance(root,work)==finance
    prior.fixture.publish(out/'result.json',dict(status='PASS',positive_replay=True,revision_refused=True,unknown_field_refused=True,work_finance_unchanged=True,provider_started=False))
    print('GI_REFRESH_API_REFUSALS_PASS 3 cases',flush=True)



def legacy_refresh(binary,old_binary,old_profile,out):
    """Produce real historical frame-v1 evidence, then requalify it publicly.

    The historical binary is bound to its retained profile, not to current
    sources. A real V1 review/control fixture precedes new-engine refusal.
    Archive only through refresh, preserving originals/calls/attempts; new
    controls must create a bound frame without inventing historical coverage.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    proof=json.loads(old_profile.read_text());assert proof['no_fixture_feature'] and proof['sha256']==prior.digest(old_binary.read_bytes())
    prior.fixture.publish(out/'historical-producer.json',dict(binary=str(old_binary),sha256=proof['sha256'],profile=str(old_profile),scope='retained historical producer, not current-source qualification'))
    root,work,workspace,checkpoint=prior.prepared_v1(old_binary,old_profile,out/'legacy')
    state=prior.shown(old_binary,root,out,'old-work',work);task=next(t for t in state['tasks'] if t['id']=='task');review=task['independent_review'];finance=prior.native.finance(root,work)
    request=dict(schema_version=1,event_id='legacy-publish-probe',expected_revision=state['revision'],task_id='task')
    code,value=prior.call(old_binary,root,out,'old-valid-preview',['task','publication-preview',work],request);assert code==0 and value['current_candidate_verified']
    frames={str(p):p.read_bytes() for p in (root/'.swarm').glob('native-*.receipt')}
    code,value=prior.call(binary,root,out,'new-requires-binding',['task','publish',work],request)
    assert code!=0 and 'managed-input-inventory-required' in json.dumps(value),value
    assert prior.shown(binary,root,out,'new-unchanged',work)==state
    request['event_id']='legacy-refresh'
    code,value=prior.call(binary,root,out,'refresh',['task','review-refresh',work],request)
    assert code==0 and not value['provider_started'] and not value['task_accepted'],value
    new=value['work'];t=next(t for t in new['tasks'] if t['id']=='task')
    assert review in t['previous_reviews'] and not t.get('independent_review') and not t.get('automatic_validation')
    assert t['attempts']==task['attempts'] and prior.native.finance(root,work)==finance
    request=dict(schema_version=1,event_id='legacy-new-controls',expected_revision=new['revision'],task_id='task')
    code,value=prior.call(binary,root,out,'new-controls',['task','validate',work],request)
    assert code==0 and value['controls_passed'] and not value['task_accepted'],value
    assert all(Path(p).read_bytes()==raw for p,raw in frames.items())
    prior.fixture.publish(out/'result.json',dict(status='PASS',legacy_originals_preserved=True,old_valid_witness=True,new_binding_required=True,explicit_refresh=True,new_controls=True,prior_consumption_preserved=True,root=str(root),work=work,paid_provider=False,global_watchdog=None))
    print('GI_LEGACY_REQUALIFICATION_PASS',flush=True)


def run(binary,profile,out,resume_from=None,rollback_sdk=None):
    """Exercise accepted control, typed refusals, explicit refresh and R2 acceptance.

    V2-dangerous must remain refused. A distinct V2-safe changes effective allowed
    targets while preserving the control's denial of B; only fresh controls and
    R2 on those exact bytes permit acceptance. Restores originals after injected
    missing-input faults; no source/receipt/SQL rewrite manufactures new evidence.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    inventory=prior.qualify(binary,profile,out)
    if resume_from is None:
        prior.prepared_v1(binary,profile,out/'control')
        prior.publication_handoff(out/'control',out/'control-handoff')
        prior.publication.run(binary,profile,out/'positive-publication',reuse=out/'control-handoff')
        print('GI_FIXED_V1_ACCEPTED',flush=True)
        root,work,workspace,checkpoint=prior.prepared_v1(binary,profile,out/'mutation')
    else:
        proof=json.loads((resume_from/'positive-publication/result.json').read_text());assert proof['status']=='PASS' and proof['task_accepted']
        setup=json.loads((resume_from/'mutation/review/inspection/setup.json').read_text())
        root,work,workspace,checkpoint=Path(setup['root']),setup['work'],Path(setup['workspace']),setup['candidate']
        prior.fixture.publish(out/'resumed-from.json',dict(original=str(resume_from),positive_witness=proof))
        print('GI_FIXED_V1_ACCEPTED',flush=True)
    def call(label,args,request=None,success=True):
        """Retain CLI channels and require the declared success/refusal."""
        code,value=prior.call(binary,root,out,label,args,request)
        assert (code==0)==success,(label,value)
        return value
    def show(label):
        """Require canonical Work from a fresh public process."""
        return prior.shown(binary,root,out,label,work)
    def request(label):
        """Create a fresh task/revision envelope; no client authority or hashes."""
        return dict(schema_version=1,event_id=label,expected_revision=show(label+'-work')['revision'],task_id='task')
    if resume_from is None:
        before=show('before');task=next(t for t in before['tasks'] if t['id']=='task');r1=task['independent_review']
        frames={str(p):p.read_bytes() for p in (root/'.swarm').glob('native-*.receipt')}
        finance=prior.native.finance(root,work);tracked=prior.tracked_inventory(workspace,checkpoint)
        def refused(label,action,reason):
            """Require the actual dependency guard and no Work/financial mutation."""
            current=show(label+'-before')
            value=call(label,['task',action,work],request(label),False)
            assert reason in json.dumps(value),value
            assert show(label+'-after')==current and prior.native.finance(root,work)==finance
            assert all(Path(p).read_bytes()==raw for p,raw in frames.items())
            return value
        refused('fresh-refresh','review-refresh','review-refresh-original-still-applicable')
        policy=workspace/'effective-security.json'
        call('valid-before-missing',['task','publication-preview',work],request('valid-before-missing'))
        retained=policy.with_suffix('.retained');policy.rename(retained)
        try:refused('missing','publish','managed-input-inventory-read')
        finally:retained.rename(policy)
        call('missing-restored',['task','publication-preview',work],request('missing-restored'))
        prior.fixture.publish(policy,prior.V2)
        observed=subprocess.run(['python3','security-consumer.py'],cwd=workspace,capture_output=True)
        prior.fixture.publish(out/'dangerous-consumer.stdout',observed.stdout)
        prior.fixture.publish(out/'dangerous-consumer.stderr',observed.stderr)
        assert observed.returncode==42 and json.loads(observed.stdout)['decision']=='allow'
        refused('dangerous-v2','publish','managed-input-inventory-changed')
        prior.fixture.publish(policy,prior.V1)
        call('dangerous-restored',['task','publication-preview',work],request('dangerous-restored'))
        safe=b'{"allowed_targets":["fixture-C"]}\n'
        prior.fixture.publish(policy,safe)
        refused('safe-v2-old-r1','publish','managed-input-inventory-changed')
        assert prior.tracked_inventory(workspace,checkpoint)==tracked
        refresh=request('refresh-r1')
        if rollback_sdk:
            envelope=out/'rollback-request.json';prior.fixture.publish(envelope,refresh)
            command=[str(rollback_sdk),'--ignored','--exact','root_store::task_validation::review_refresh::tests::refresh_rollback_preserves_original_review_and_recorded_state','--nocapture']
            observed=subprocess.run(command,capture_output=True,env=dict(os.environ,SWARM_GIETH_REFRESH_ROOT=str(root),SWARM_GIETH_REFRESH_WORK=work,SWARM_GIETH_REFRESH_REQUEST=str(envelope)))
            prior.fixture.publish(out/'rollback.command.json',dict(argv=command,global_watchdog=None))
            prior.fixture.publish(out/'rollback.stdout',observed.stdout);prior.fixture.publish(out/'rollback.stderr',observed.stderr)
            assert observed.returncode==0 and b'1 passed; 0 failed' in observed.stdout and b'GI_REFRESH_ROLLBACK_PRESERVED' in observed.stdout
            assert show('rollback-unchanged')==before and prior.native.finance(root,work)==finance
        value=call('refresh-r1',['task','review-refresh',work],refresh)
        assert not value['provider_started'] and not value['task_accepted'] and value['review_id']==r1['id']
        state=show('refreshed');t=next(t for t in state['tasks'] if t['id']=='task')
        assert not t.get('independent_review') and not t.get('automatic_validation') and r1 in t['previous_reviews']
        assert t['attempts']==task['attempts'] and prior.native.finance(root,work)==finance
        replay=call('refresh-replay',['task','review-refresh',work],refresh)
        assert replay['replayed'] and replay['work']==state and prior.native.finance(root,work)==finance
        call('no-controls',['task','publish',work],request('no-controls'),False)
        controls=call('new-controls',['task','validate',work],request('new-controls'))
        assert controls['controls_passed'] and not controls['task_accepted']
        new=show('new-controls-work');nt=next(t for t in new['tasks'] if t['id']=='task')
        assert all(json.loads(c['review_output'])['policy_sha256']==prior.digest(safe) for c in nt['automatic_validation']['controls'])
    else:
        before=json.loads((resume_from/'before.stdout').read_text())['work'];task=next(t for t in before['tasks'] if t['id']=='task');r1=task['independent_review']
        frames={str(p):p.read_bytes() for p in (root/'.swarm').glob('native-*.receipt')}
        tracked=prior.tracked_inventory(workspace,checkpoint);finance=prior.native.finance(root,work)
        safe=b'{"allowed_targets":["fixture-C"]}\n'
        assert (workspace/'effective-security.json').read_bytes()==safe
        resumed=show('resume-work');t=next(t for t in resumed['tasks'] if t['id']=='task')
        assert r1 in t['previous_reviews'] and not t.get('independent_review')
        assert all(json.loads(c['review_output'])['policy_sha256']==prior.digest(safe) for c in t['automatic_validation']['controls'])
    prior.native.run(binary,profile,out/'r2',reuse=out/'mutation/inspection-seed')
    state=show('r2-work');t=next(t for t in state['tasks'] if t['id']=='task');r2=t['independent_review']
    assert r2['id']!=r1['id'] and r1 in t['previous_reviews'] and t['status']=='submitted'
    assert state['planning']['reviewer']['calls']==before['planning']['reviewer']['calls']+1
    assert t['attempts']==task['attempts'] and prior.tracked_inventory(workspace,checkpoint)==tracked
    assert all(Path(p).read_bytes()==raw for p,raw in frames.items())
    publish=request('publish-r2');value=call('publish-r2',['task','publish',work],publish)
    assert value['task_accepted'] and not value['provider_started']
    accepted=show('accepted');assert next(t for t in accepted['tasks'] if t['id']=='task')['status']=='accepted'
    final_finance=prior.native.finance(root,work)
    replay=call('publish-r2-replay',['task','publish',work],publish)
    assert replay['replayed'] and replay['work']==accepted and prior.native.finance(root,work)==final_finance
    api_refusals(binary,root,work,json.loads(((resume_from or out)/'refresh-r1.input.json').read_text()),out/'api-refusals')
    assert all(prior.digest(Path(n).read_bytes())==sha for n,sha in inventory.items())
    prior.fixture.publish(out/'result.json',dict(status='PASS',rollback_sdk_exercised=bool(rollback_sdk and resume_from is None),valid_v1_accepted=True,
        dangerous_v2_refused_under_r1=True,safe_v2_refused_under_r1=True,
        initial_tracked_entries=len(tracked),tracked_unchanged=True,r1_preserved=True,
        refresh_replay_without_call=True,new_controls_on_safe_v2=True,
        r2_id=r2['id'],r1_id=r1['id'],safe_v2_accepted_under_r2=True,
        publication_replayed=True,worker_attempts_preserved=True,root=str(root),work=work,
        paid_provider=False,independent_security_audit=False,global_watchdog=None))
    print('GI_REFRESH_R1_REFUSED_R2_ACCEPTED',flush=True)


def main():
    """Bind all static helper sources and preserve original failures without retry."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--resume-from',type=Path)
    parser.add_argument('--refresh-sdk',type=Path)
    a=parser.parse_args();out=a.output.resolve();sources=prior.inspection.native.helper_inputs(Path(__file__))
    try:
        run(a.binary.resolve(),a.profile.resolve(),out,a.resume_from.resolve() if a.resume_from else None,a.refresh_sdk.resolve() if a.refresh_sdk else None)
        assert all(prior.digest(Path(n).read_bytes())==sha for n,sha in sources.items())
        prior.fixture.publish(out/'helper-inputs.json',sources)
    except BaseException:
        if out.exists():
            prior.fixture.publish(out/'helper-inputs.json',sources)
            prior.fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
