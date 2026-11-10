#!/usr/bin/env python3
"""Two genuine results survive a changed future reserve without any new grant.

Verdict and stale/report-change originals, then normal public claim/preview.
Each case has one inert original exec, no replay exec/refund/acceptance/settlement.
Source-bound Rust SDK and normal profile remain distinct. No global watchdog.
"""
import argparse,json,subprocess,sys,traceback
from pathlib import Path
import rust_reviewer_claim_candidate as sdk


def main():
    """Require stable historical estimates and strict future-start refusal.

    Retain both command channels and exact source profiles before PASS; the SDK
    also checks foreign/missing/observed-corrupt reservation and transaction scope.
    Any failed prerequisite/case retains progress and aborts without a green mean.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--sdk-source-root',type=Path);p.add_argument('--output',required=True,type=Path);args=p.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False);completed=[];sdk.f.publish(out/'method.py',Path(__file__).read_bytes())
    def run(label,argv):
        """Execute one named step once, preserving actual channels and status."""
        sdk.f.publish(out/(label+'.command.json'),dict(argv=argv,global_watchdog=None))
        with (out/(label+'.stdout')).open('xb') as stdout,(out/(label+'.stderr')).open('xb') as stderr:r=subprocess.run(argv,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
        if r.returncode:raise ValueError('budget-result-step-failed:'+label+':'+str(r.returncode))
    try:
        for variant,state in [('verdict','passed'),('stale','stale')]:
            target=out/variant;argv=[sys.executable,str(Path(__file__).with_name('rust_review_departed_owner.py')),'--binary',str(args.binary.resolve()),'--output',str(target),'--budget-result',variant]
            if args.sdk_source_root:argv+=['--sdk-source-root',str(args.sdk_source_root.resolve())]
            run(variant,argv);result=json.loads((target/'result.json').read_text());proof=json.loads((target/'owned-review-budget-result.json').read_text())
            required=['start_still_refused','historical_reservation_preserved','foreign_reservation_refused','missing_reservation_refused','observed_or_corrupt_usage_refused','transaction_required','provider_invocation_unchanged','replay_verified']
            if result['status']!='PASS' or result['provider_calls']!=1 or result['recovery_execs']!=0 or not all(proof[k] for k in required) or proof['state']!=state or proof['task_accepted'] or proof['settled'] or proof['model_received']:raise ValueError('budget-result-proof-invalid:'+variant)
            run(variant+'-context',[sys.executable,str(Path(__file__).with_name('rust_review_public_context.py')),'--binary',str(args.binary.resolve()),'--profile',str(args.profile.resolve()),'--fixture',str(target/'fixture.json'),'--output',str(out/(variant+'-context')),'--expected-review-state',state])
            context=json.loads((out/(variant+'-context')/'result.json').read_text())
            if context['expected_review_state']!=state or not context['preview_pure'] or context['model_received'] or context['decision_acknowledged']:raise ValueError('budget-result-normal-context-invalid:'+variant)
            completed.append(variant);sdk.f.publish(out/'progress.json',dict(completed=completed,expected=2));print('PASS budget result '+variant,flush=True)
        sdk.f.publish(out/'result.json',dict(status='PASS',variants=completed,original_provider_execs=2,recovery_provider_execs=0,normal_public_contexts=2,historical_reservations_preserved=True,new_start_still_refused=True,task_accepted=False,settlement=False,production_wiring=False,global_watchdog=None));print('PASS budget result recovery 2 cases',flush=True)
    except BaseException:
        sdk.f.publish(out/'failure.json',dict(status='FAIL',completed=completed,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
