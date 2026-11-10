#!/usr/bin/env python3
"""Actual public supervisor result after a future-reserve change, no new exec.

Use the existing real held-provider/SQL-lock recipe. Its normal control recovers
one original first; second case changes only budget through supported CLI between
original owner exit and reconcile. No fake positive receipt, paid model or global
watchdog. Product lease120 seconds is an explicit existing fixture input, never
an inherited whole-test timeout or a permission to revive an expired lease.
"""
import argparse,json,sys,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_planning_native_run as native
from rust_planning_budget import ledger


def retain_originals(root,case,label,binary_sha256,destination):
    """Copy complete owned original frame/program/model observation before PASS.

    Require the actual case marker Root/binary before opening any source. This
    prevents accidental cross-Root collection, not malicious same-UID forgery.
    Preserve full bytes with hashes and distinguish framed digest from optional
    recorded body digest. Never seed/reconstruct a positive engine receipt or
    claim that this is an independently authenticated invoice/identity/restore.
    """
    marker=json.loads((case/(label+'-fixture.json')).read_text())
    if Path(marker['root']).resolve()!=root.resolve() or marker['binary_sha256']!=binary_sha256 or marker['scope']!='owned local fixture, no account/model authentication':raise ValueError('planner-retention-fixture-mismatch')
    claim=label+'-claim';destination.mkdir(exist_ok=False);sources={'original.frame':root/'.swarm'/('native-planning-'+fixture.digest(claim.encode())+'.receipt'),'model-received.json':root/'model-received.json','planning-proposal.json':root/'planning-proposal.json','inert-provider.py':root/'codex'};hashes={}
    for name,path in sources.items():
        if path.is_symlink() or not path.is_file():raise ValueError('planner-original-file-required:'+name)
        raw=path.read_bytes();fixture.publish(destination/name,raw)
        if (destination/name).read_bytes()!=raw or path.read_bytes()!=raw:raise ValueError('planner-original-copy-drift:'+name)
        hashes[name]=fixture.digest(raw)
    class Owned:
        pass
    owned=Owned();owned.root=root;row=native.native_row(owned,claim)
    fixture.publish(destination/'retention.json',dict(status='PASS',root=str(root),claim=claim,files=hashes,native_row=row,recorded_body_sha256=row[4],frame_sha256=hashes['original.frame'],scope='Owned actual frame copies, not signed provenance, invoice or historical Root restoration'))
    return hashes


def main():
    """Preserve control, exact policy change and original failure/recovery proofs.

    Normal executable profile/bytes are required. Existing held recipe retains
    original command channels, real owner/child/wait and unchanged reservations.
    The public preview must still refuse a new start at the old reserve; public
    reconciliation must apply the one original without relaxing lease/actions.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--output',required=True,type=Path);args=p.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    fixture.publish(out/'method.py',Path(__file__).read_bytes());fixture.publish(out/'held-method.py',Path(native.__file__).read_bytes())
    try:
        binary=args.binary.resolve();profile=json.loads(args.profile.read_text());assert profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and profile['all_core_test_false'] and profile['all_core_features_empty'] and profile['no_fixture_feature'] and fixture.digest(binary.read_bytes())==profile['sha256']
        control=out/'control';control.mkdir();good=native.held(binary,control,'budget-result-control',True,False);fixture.publish(out/'control.json',good)
        def change(owned,claim):
            """One actual public budget apply after original, preserving every ledger row."""
            rows=ledger(owned);row=next(r for r in rows if r[0]==claim);reserve=row[2]+1;limit=reserve*100
            before=owned.ok(['work','show',owned.work])['work'];owned.ok(['budget','apply',owned.work],dict(schema_version=1,event_id='known-original-budget-change',expected_revision=before['revision'],budget=dict(limit_usd=limit,reserve_per_launch_usd=reserve,estimate_source='Explicit known planner original fixture',reference_date='2026-10-22')))
            assert ledger(owned)==rows;after=owned.ok(['work','show',owned.work])['work'];assert after.get('tasks')==before.get('tasks') and after['planning']['scopes']==before['planning']['scopes']
            scope=next(s for s in after['planning']['scopes'] if s['id']=='root');request=dict(schema_version=1,expected_revision=after['revision'],claim_id=claim,scope='root',scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'])
            code,wire=owned.call(['planning','call-preview',owned.work],request);assert code!=0 and 'budget-existing-planning-estimate-changed' in str(wire),wire
            fixture.publish(owned.output/'budget-change.json',dict(root=str(owned.root),work=owned.work,claim=claim,old_estimate=row[2],future_reserve=reserve,ledger=rows,new_start_refused=True,source_scopes_unchanged=True,provider_original_exists=(owned.root/'model-received.json').exists()))
        changed=out/'changed';changed.mkdir();result=native.held(binary,changed,'budget-result-changed',True,False,after_original=change);fixture.publish(out/'changed.json',result)
        assert good['decision_applied'] and result['decision_applied'] and good['provider_executions']==result['provider_executions']==1 and not result['accepted']
        paused=out/'paused';paused.mkdir();held={}
        def pause(owned,claim):
            """Known result plus new budget still owes the actual live stop fence."""
            change(owned,claim);before=owned.ok(['work','show',owned.work])['work'];owned.ok(['planning','pause',owned.work],dict(schema_version=1,event_id='known-original-pause',expected_revision=before['revision'],reason='Verify result finance does not bypass stop fence'));held.update(owned=owned,claim=claim,original=(owned.root/'model-received.json').read_bytes(),ledger=ledger(owned))
        try:native.held(binary,paused,'budget-result-paused',True,False,after_original=pause)
        except AssertionError as rejection:
            # Accept only the expected public reconcile failure tuple after full
            # held cleanup; setup/SQL/provider/assertion/cleanup failures remain FAIL.
            if len(rejection.args)!=1 or not isinstance(rejection.args[0],tuple) or len(rejection.args[0])!=4:raise
            label,argv,status,wire=rejection.args[0]
            if label!='budget-result-paused' or argv!=['planning','reconcile',held.get('claim')] or status==0:raise
            failure=wire.get('failure',{});code=failure.get('code');causes=failure.get('causes',[])
            if code!='planning-decision-stopped' and not (code=='work-state-preserved' and 'planning-decision-stopped' in causes):raise
            fixture.publish(out/'paused-original-refusal.json',wire)
        else:raise AssertionError('known original result bypassed actual planning pause')
        owned=held['owned'];current=owned.ok(['work','show',owned.work])['work'];assert current['planning']['decisions']==0 and not any(t['id']=='held-worker' for t in current.get('tasks',[]));assert ledger(owned)==held['ledger'] and (owned.root/'model-received.json').read_bytes()==held['original']
        fixture.publish(out/'paused.json',dict(root=str(owned.root),work=owned.work,claim=held['claim'],refused=True,decision_applied=False,reservations_preserved=True,provider_executions=1,model_original_sha256=fixture.digest(held['original']),task_accepted=False))
        roots=[(Path(good['root']),control,'budget-result-control'),(Path(result['root']),changed,'budget-result-changed'),(owned.root,paused,'budget-result-paused')]
        retained={}
        for index,(root,case,label) in enumerate(roots):
            retained[label]=retain_originals(root,case,label,profile['sha256'],case/'retained-originals')
            if index==0:
                wrong=out/'foreign-retention-must-not-exist'
                try:retain_originals(root,changed,'budget-result-changed',profile['sha256'],wrong)
                except ValueError as error:
                    if str(error)!='planner-retention-fixture-mismatch':raise
                else:raise AssertionError('foreign Root marker accepted')
                assert not wrong.exists()
        fixture.publish(out/'result.json',dict(status='PASS',originals_retained=retained,foreign_retention_refused=True,control=good,changed=result,original_execs_per_case=1,total_original_execs=3,paused_result_refused=True,recovery_execs=0,normal_binary_sha256=profile['sha256'],new_start_still_refused=True,decision_applied=True,task_accepted=False,settlement=False,global_watchdog=None));print('PASS normal planner result after budget change',flush=True)
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':main()
