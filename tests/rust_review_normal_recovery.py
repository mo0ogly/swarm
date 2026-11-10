#!/usr/bin/env python3
"""Normal CLI recovers actual unpublished SDK originals after owner departure.

The producer is explicitly a qualified libtest fixture, not the normal runtime.
Application/replay use the supplied qualified normal binary. No positive SQL
seeding, paid model, ownership adoption, acceptance, settlement or watchdog.
"""
import argparse,json,subprocess,sys,traceback
from pathlib import Path
import rust_native_reviewer as public
import rust_reviewer_claim_candidate as sdk


def main():
    """Require six result modes plus explicit stale retirement after a valid control.

    Each producer runs one actual inert reviewer, waits it and exits while its
    original/claim remains unpublished. Only then call normal Root recovery.
    Preserve private/public originals before PASS and assert unchanged finance,
    one original invocation and exact replay. Missing evidence remains an error.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--sdk-source-root',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);completed=[]
    sdk.f.publish(out/'method.py',Path(__file__).read_bytes());binary=a.binary.resolve();profile=json.loads(a.profile.read_text());sha=sdk.f.digest(binary.read_bytes())
    try:
        assert profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and profile['all_core_test_false'] and profile['all_core_features_empty'] and profile['no_fixture_feature'] and profile['sha256']==sha
        for name in ['complete','changes-requested','duration','source','output-error','domain-invalid','stale']:
            case=out/name;case.mkdir();source=case/'producer';variant='complete' if name=='stale' else name
            command=[sys.executable,str(Path(__file__).with_name('rust_reviewer_claim_candidate.py')),'--binary',str(binary),'--sdk-source-root',str(a.sdk_source_root.resolve()),'--output',str(source),'--execute-reviewer','--final-reserved-call','--controller-case',variant,'--require-private-receipt','--require-raw-capture']
            sdk.f.publish(case/'producer.command.json',dict(argv=command,global_watchdog=None,scope='Qualified libtest original producer; normal CLI result application'))
            with (case/'producer.stdout').open('xb') as stdout,(case/'producer.stderr').open('xb') as stderr:r=subprocess.run(command,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
            if r.returncode:raise ValueError('normal-recovery-producer-failed:'+name)
            manifest=json.loads((source/'fixture.json').read_text());root=Path(manifest['root']);work=manifest['work']
            if (root/'.swarm/owned-review-claim-fixture').read_text()!=manifest['token'] or not manifest['execute_reviewer']:raise ValueError('normal-recovery-owned-fixture-required')
            sdk.f.publish(case/'fixture.json',dict(root=str(root),work=work,binary_sha256=sha))
            before=public.preview.shown(binary,root,case,'before',work);review=before['tasks'][0]['independent_review'];assert review['state']=='running'
            review_id=review['id'];invoked=(root/'reviewer-invoked.json').read_bytes();rows=public.finance(root,work)
            if name=='stale':
                code,refused=public.native.invoke(binary,root,case,'fresh-retirement-refused',['task','review-retire-stale',review_id]);assert code!=0 and refused['failure']['code']=='review-stale-original-still-applicable',refused
                assert public.preview.shown(binary,root,case,'after-fresh-refusal',work)==before and public.finance(root,work)==rows
                original=(root/'handoff.md').read_bytes();sdk.f.publish(root/'handoff.md',original+b'\nOwned explicit stale dependency change\n')
            action='review-retire-stale' if name=='stale' else 'review-reconcile'
            code,result=public.native.invoke(binary,root,case,'apply',['task',action,review_id]);assert code==0,result
            valid=name in ('complete','changes-requested');assert result['review_recorded'] and result['verdict_applicable']==valid and not result['task_accepted'] and not result['replayed'],result
            after=public.preview.shown(binary,root,case,'after',work);task=after['tasks'][0];state='stale' if name=='stale' else 'passed' if name=='complete' else 'changes_requested' if name=='changes-requested' else 'error'
            assert task['independent_review']['state']==state and task['status']==('blocked' if name=='changes-requested' else 'submitted')
            assert after['planning']['reviewer']['calls']==before['planning']['reviewer']['calls'] and public.finance(root,work)==rows
            receipt=public.retain(root,case,work,task['independent_review'],sha);assert receipt['failed']==(not valid and name!='stale')
            code,replayed=public.native.invoke(binary,root,case,'replay',['task',action,review_id]);assert code==0 and replayed['replayed'] and replayed['verdict_applicable']==valid,replayed
            assert public.preview.shown(binary,root,case,'after-replay',work)==after and public.finance(root,work)==rows and (root/'reviewer-invoked.json').read_bytes()==invoked
            if name=='stale':
                sdk.f.publish(root/'handoff.md',original)
                code,historical=public.native.invoke(binary,root,case,'restored-replay',['task','review-reconcile',review_id]);assert code==0 and historical['replayed'] and not historical['verdict_applicable'],historical
                assert public.finance(root,work)==rows and (root/'reviewer-invoked.json').read_bytes()==invoked
            sdk.f.publish(case/'result.json',dict(status='PASS',state=state,original_owner_exited=True,libtest_original_producer=True,normal_cli_application=True,recovery_execs=0,one_original_exec=True,task_accepted=False));completed.append(name);sdk.f.publish(out/'progress.json',dict(completed=completed,expected=7));print('PASS normal recovery '+name,flush=True)
        sdk.f.publish(out/'result.json',dict(status='PASS',cases=completed,libtest_original_producer=True,normal_cli_application=True,original_execs=7,recovery_execs=0,task_accepted=False,settlement=False,authenticated_identity=False,global_watchdog=None));print('PASS normal reviewer recovery 7 cases',flush=True)
    except BaseException:sdk.f.publish(out/'failure.json',dict(status='FAIL',completed=completed,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
