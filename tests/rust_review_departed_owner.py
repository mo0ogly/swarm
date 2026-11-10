#!/usr/bin/env python3
"""Recover an actual retained reviewer original after its creator process exits.

A first SDK runs one inert provider without publication and exits; a second SDK
must confirm same-host owner departure and apply the retained original without
new exec, adoption, task acceptance or settlement. Optional certificate mode
checks genuine legacy publication or legacy rollback followed by v2 publication.
No global watchdog or Go.
"""
import argparse,json,os,subprocess,sys,traceback
from pathlib import Path
import rust_reviewer_claim_candidate as sdk


def main():
    """Preserve both original execution and subsequent recovery source/profile.

    Require named test counts and original result evidence. A failed test retains
    its qualification and public before/after Work snapshots; it remains failure.
    Only an existing token-bound fixture from this invocation is read or changed.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--sdk-source-root',type=Path)
    parser.add_argument('--controller-case',choices=('complete','changes-requested','duration','source','output-error','domain-invalid'),default='complete')
    parser.add_argument('--certificate-mode',choices=('legacy','upgrade'),help='Actual publication wire compatibility on the same protected transaction')
    parser.add_argument('--stale-original',action='store_true',help='Known original/report-change non-verdict fixture only')
    parser.add_argument('--stale-variant',choices=('report','settings','paused','authority'),default='report',help='Explicit owned stale dependency; only with --stale-original')
    parser.add_argument('--budget-result',choices=('verdict','stale'),help='Owned public reserve change after original; no new provider permission')
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    sdk.f.publish(out/'method.py',Path(__file__).read_bytes());root=None
    try:
        if sum(map(bool,[args.stale_original,args.certificate_mode,args.budget_result]))>1:raise ValueError('exclusive-recovery-scenario-required')
        original=out/'original'
        argv=[sys.executable,str(Path(__file__).with_name('rust_reviewer_claim_candidate.py')),'--binary',str(args.binary.resolve()),'--output',str(original),'--execute-reviewer','--final-reserved-call','--controller-case',args.controller_case,'--require-raw-capture']
        if args.controller_case=='complete':argv+=['--stream-handshake']
        if args.sdk_source_root:argv+=['--sdk-source-root',str(args.sdk_source_root.resolve())]
        sdk.f.publish(out/'original.command.json',dict(argv=argv,global_watchdog=None))
        with (out/'original.stdout').open('xb') as stdout,(out/'original.stderr').open('xb') as stderr:
            actual=subprocess.run(argv,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
        if actual.returncode:raise ValueError('departed-owner-original-failed')
        proof=json.loads((original/'result.json').read_text());manifest=json.loads((original/'fixture.json').read_text());root=Path(manifest['root'])
        if proof['status']!='PASS' or proof['business_result_published'] or (root/'.swarm/owned-review-claim-fixture').read_text()!=manifest['token']:raise ValueError('departed-original-fixture-invalid')
        repository=Path(__file__).resolve().parents[1];source=sdk.sdk_source_inputs(repository,args.sdk_source_root)
        sys.path.insert(0,str(repository/'tools/verification'));import verify_native_profile as profile
        def work(label):
            """Read only public Work with original channels, preserving read failure."""
            command=[str(args.binary.resolve()),'work','show',manifest['work'],'--root',str(root),'--json'];actual=subprocess.run(command,capture_output=True,stdin=subprocess.DEVNULL)
            sdk.f.publish(out/(label+'.command.json'),dict(argv=command));sdk.f.publish(out/(label+'.stdout'),actual.stdout);sdk.f.publish(out/(label+'.stderr'),actual.stderr)
            if actual.returncode:raise ValueError('departed-work-read-failed:'+label)
            return json.loads(actual.stdout)['work']
        sdk.f.publish(out/'fixture.json',(original/'fixture.json').read_bytes())
        before=work('before-recovery')
        recovery=out/'recovery';recovery.mkdir()
        env={**os.environ,'CARGO_TARGET_DIR':str((repository/'rust/target/r97').resolve()),'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0','CODEX_HOME':str(root/'codex-home'),'SWARM_OWNED_REVIEW_CLAIM_FIXTURE':str(original/'fixture.json'),'SWARM_OWNED_REVIEW_PUBLIC_BINARY':str(args.binary.resolve()),'SWARM_OWNED_REVIEW_STALE_VARIANT':args.stale_variant,'SWARM_OWNED_REVIEW_BUDGET_RESULT':args.budget_result or ''}
        names=['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_TARGET','RUSTUP_TOOLCHAIN','CARGO_HOME'];profile.require(not any(env.get(k) for k in names),'sdk-ambient-compiler-override')
        bound=profile.candidates(source);sdk.f.publish(recovery/'sdk.sources.before.json',bound)
        test_name='reviewer_budget_result_owned_fixture' if args.budget_result else 'reviewer_stale_owned_fixture' if args.stale_original else ('public_certificate_'+args.certificate_mode+'_owned_fixture') if args.certificate_mode else 'reviewer_recover_departed_owned_public_fixture'
        argv=['cargo','test','--manifest-path',str(source/'rust/Cargo.toml'),'--offline','--locked','-p','swarm-engine','--lib','--message-format=json',test_name,'--','--ignored','--nocapture']
        sdk.f.publish(recovery/'command.json',dict(argv=argv,source=str(source),global_watchdog=None))
        with (recovery/'sdk.stdout').open('xb') as stdout,(recovery/'sdk.stderr').open('xb') as stderr:
            actual=subprocess.run(argv,env=env,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
        primary=RuntimeError(f'departed-review-recovery-failed:{actual.returncode}') if actual.returncode else None
        try:sdk.qualify_sdk(source,recovery,bound)
        except BaseException as secondary:
            if primary is not None:raise BaseExceptionGroup('Recovery and source qualification failed',[primary,secondary])
            raise
        try:after=work('after-recovery')
        except BaseException as secondary:
            if primary is not None:raise BaseExceptionGroup('Recovery and public readback failed',[primary,secondary])
            raise
        if primary is not None:raise primary
        text=(recovery/'sdk.stdout').read_text()
        profile.require('1 passed; 0 failed' in text and ('REVIEW_BUDGET_RESULT_CHECKED' if args.budget_result else 'STALE_REVIEW_ORIGINAL_RETIRED' if args.stale_original else 'PUBLIC_CERTIFICATE_COMPAT_CHECKED' if args.certificate_mode else 'DEPARTED_REVIEW_ORIGINAL_RECOVERED') in text,'departed-review-named-test-missing')
        profile.require(after['revision']>=before['revision']+1 if args.stale_original or args.budget_result else after['revision']==before['revision']+1,'departed-review-revision-invalid')
        profile.require(after['planning']['reviewer']['calls']==before['planning']['reviewer']['calls'],'departed-review-charged-again')
        retained=sdk.retain_process_observations(root,out)
        if args.budget_result:
            proof=json.loads((out/'owned-review-budget-result.json').read_text())
            applicable=args.budget_result=='verdict'
            profile.require(proof['variant']==args.budget_result and proof['state']==('passed' if applicable else 'stale') and proof['verdict_applicable']==applicable and after['revision']==proof['before_publication_revision']+1,'budget-result-state-invalid')
            profile.require(all(proof[k] for k in ['start_still_refused','historical_reservation_preserved','foreign_reservation_refused','missing_reservation_refused','observed_or_corrupt_usage_refused','transaction_required','provider_invocation_unchanged','replay_verified']) and not proof['task_accepted'] and not proof['settled'] and not proof['model_received'],'budget-result-proof-missing')
            for name in ['owned-review-budget-result.json','owned-review-budget-context.json','owned-review-public-certificate.json','owned-review-private-receipt.frame']:
                profile.require(retained.get(name)==sdk.f.digest((out/name).read_bytes()),'budget-result-original-not-retained')
            sdk.f.publish(out/'result.json',dict(status='PASS',variant=args.budget_result,review_state=proof['state'],provider_calls=1,recovery_execs=0,task_accepted=False,root=str(root),global_watchdog=None))
        elif args.stale_original:
            proof=json.loads((out/'owned-review-stale-recovery.json').read_text())
            profile.require(after['revision']==proof['before_publication_revision']+1 and proof['variant']==args.stale_variant,'stale-review-public-revision-invalid')
            profile.require(proof['state']=='stale' and not proof['verdict_applicable'] and not proof['task_accepted'] and not proof['model_received'] and not proof['decision_acknowledged'],'stale-review-not-explicit')
            profile.require(all(proof[k] for k in ['original_candidate_preserved','provider_invocation_unchanged','replay_verified','current_report_not_loaded','unchanged_refused','unreadable_refused','missing_report_refused','sql_rollback_verified','public_changed_refused','private_changed_refused','finance_estimate_held']),'stale-review-proof-missing')
            for name in ['owned-review-stale-recovery.json','owned-review-stale-context.json','owned-review-public-certificate.json','owned-review-private-receipt.frame']:
                profile.require(retained.get(name)==sdk.f.digest((out/name).read_bytes()),'stale-original-not-retained')
            sdk.f.publish(out/'result.json',dict(status='PASS',review_state='stale',provider_calls=1,recovery_execs=0,task_accepted=False,root=str(root),global_watchdog=None))
        elif args.certificate_mode:
            proof=json.loads((out/'owned-review-certificate-compat.json').read_text())
            upgrade=args.certificate_mode=='upgrade'
            profile.require(proof['legacy_wire']==(not upgrade) and proof['upgraded_after_rollback']==upgrade and proof['schema_version']==(2 if upgrade else 1),'certificate-mode-wrong')
            profile.require(proof['context_prepared'] and proof['replay_verified'] and proof['provider_execs']==1 and proof['recovery_execs']==0 and not proof['task_accepted'] and not proof['model_received'],'certificate-compat-proof-missing')
            if upgrade:profile.require(proof['old_candidate_preserved'],'certificate-old-candidate-lost')
            for name in ['owned-review-certificate-compat.json','owned-review-public-certificate.json','owned-review-private-receipt.frame']:
                profile.require(retained.get(name)==sdk.f.digest((out/name).read_bytes()),'certificate-original-not-retained')
            sdk.f.publish(out/'result.json',dict(status='PASS',certificate_mode=args.certificate_mode,original_sdk_waited=True,provider_calls=1,recovery_execs=0,task_accepted=False,settlement=False,root=str(root),global_watchdog=None))
        else:
            proof=json.loads((out/'owned-review-departed-recovery.json').read_text())
            required=['owner_confirmed_departed','original_owner_preserved','known_wait_required','foreign_host_refused','live_tuple_refused','unknown_wait_refused','sql_rollback_verified','changed_public_refused','changed_private_refused','replay_without_exec','provider_invocation_unchanged','financial_estimate_held']
            profile.require(all(proof[k] for k in required) and proof['applying_os_pid']!=proof['original_os_pid'] and not proof['task_accepted'],'departed-recovery-proof-missing')
            for name in ['owned-review-departed-recovery.json','owned-review-departed-context.json','owned-review-public-certificate.json','owned-review-private-receipt.frame']:
                profile.require(retained.get(name)==sdk.f.digest((out/name).read_bytes()),'departed-recovery-original-not-retained')
            sdk.f.publish(out/'result.json',dict(status='PASS' ,original_sdk_waited=True,original_result_published=False,recovery_tests=1,provider_calls=1,owner_adopted=False,task_accepted=False,settlement=False,controller_case=args.controller_case,verdict_applicable=proof['verdict_applicable'],review_state=proof['review_state'],root=str(root),global_watchdog=None))
        print('PASS departed reviewer original recovery',flush=True)
    except BaseException:
        sdk.f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()))
        raise


if __name__=='__main__':main()
