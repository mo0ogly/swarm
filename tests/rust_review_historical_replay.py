#!/usr/bin/env python3
"""Replay retained terminal reviewer originals with a qualified newer normal binary.

Fixtures must retain their own marker and originals. Old/new binary digests are
explicitly distinct provenance, not authenticated identity. No new reviewer,
SQL seed, acceptance, settlement, paid provider or global watchdog.
"""
import argparse,json,traceback
from pathlib import Path
import rust_native_reviewer as public


def main():
    """Require exact frame/certificate/witness and Work/finance conservation.

    Validate each historical marker against its retained originals before opening
    Root SQL. Only terminal recorded outcomes are eligible; missing evidence is
    an error, never reconstructed or guessed. All channels/inputs are preserved.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--fixtures',required=True,nargs='+',type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);results=[]
    public.f.publish(out/'method.py',Path(__file__).read_bytes());binary=a.binary.resolve();sha=public.f.digest(binary.read_bytes());profile=json.loads(a.profile.read_text())
    try:
        assert profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and profile['all_core_test_false'] and profile['all_core_features_empty'] and profile['no_fixture_feature'] and profile['sha256']==sha
        for index,source in enumerate(a.fixtures):
            case=out/str(index);case.mkdir();marker=json.loads((source/'fixture.json').read_text());retained=json.loads((source/'originals/retention.json').read_text());root=Path(marker['root']);work=marker['work'];review_id=retained['review_id']
            if retained['root']!=str(root) or retained['work']!=work or len(marker['binary_sha256'])!=64:raise ValueError('historical-review-fixture-mismatch')
            frame=root/'.swarm'/('native-review-'+public.f.digest(review_id.encode())+'.receipt')
            assert public.f.digest(frame.read_bytes())==retained['files']['original.frame']
            assert public.f.digest((root/'reviewer-invoked.json').read_bytes())==retained['files']['invocation.json']
            before=public.preview.shown(binary,root,case,'before',work);review=next(t for t in before['tasks'] if t['independent_review']['id']==review_id)['independent_review'];state=review['state'];assert state in ('passed','changes_requested','error','stale')
            certificate=root/review['receipt'];assert public.f.digest(certificate.read_bytes())==retained['files']['certificate.json']
            paths=[frame,certificate,root/'reviewer-invoked.json'];raw={str(p):p.read_bytes() for p in paths};rows=public.finance(root,work)
            code,value=public.native.invoke(binary,root,case,'reconcile',['task','review-reconcile',review_id]);assert code==0 and value['replayed'] and value['verdict_applicable']==(state in ('passed','changes_requested')) and not value['task_accepted'],value
            assert public.preview.shown(binary,root,case,'after',work)==before and public.finance(root,work)==rows and all(Path(p).read_bytes()==v for p,v in raw.items())
            result=dict(status='PASS',fixture=str(source.resolve()),old_binary_sha256=marker['binary_sha256'],new_binary_sha256=sha,state=state,originals_unchanged={p:public.f.digest(v) for p,v in raw.items()},work_and_reservations_unchanged=True,recovery_execs=0);public.f.publish(case/'result.json',result);results.append(result)
        public.f.publish(out/'result.json',dict(status='PASS',cases=results,count=len(results),recovery_execs=0,task_accepted=False));print('PASS historical reviewer replay '+str(len(results))+' retained cases',flush=True)
    except BaseException:public.f.publish(out/'failure.json',dict(status='FAIL',completed=len(results),cause=traceback.format_exc()));raise


if __name__=='__main__':main()
