#!/usr/bin/env python3
"""Replay four actual failed reviewers through explicit non-verdict finalization.

Duration/source/output/domain originals, SQL rollback and recovery, protected
readback/replay and normal incident context. No paid model, Go, installation,
owner adoption, provider retry, acceptance, settlement or global watchdog.
"""
import argparse,json,subprocess,sys,traceback
from pathlib import Path
import rust_reviewer_claim_candidate as sdk


def main():
    """Require all four source-qualified lifecycle proofs on fresh owned Roots.

    Preserve original channels and partial progress. Each inner SDK must retain
    receipt/certificate/context and assert refusal without mutation before
    restoration. Completion requires original evidence, never only exit0.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--sdk-source-root',type=Path)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    sdk.f.publish(out/'method.py',Path(__file__).read_bytes());completed=[]
    try:
        for case in ['duration','source','output-error','domain-invalid']:
            target=out/case
            argv=[sys.executable,str(Path(__file__).with_name('rust_reviewer_claim_candidate.py')),'--binary',str(args.binary.resolve()),'--output',str(target),'--execute-reviewer','--final-reserved-call','--controller-case',case,'--require-raw-capture','--reconcile-failed']
            if args.sdk_source_root:argv+=['--sdk-source-root',str(args.sdk_source_root.resolve())]
            sdk.f.publish(out/(case+'.command.json'),dict(argv=argv,global_watchdog=None))
            with (out/(case+'.stdout')).open('xb') as stdout,(out/(case+'.stderr')).open('xb') as stderr:
                actual=subprocess.run(argv,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
            if actual.returncode:raise ValueError('review-failure-case-failed:'+case)
            result=json.loads((target/'result.json').read_text());proof=json.loads((target/'owned-review-failure-reconciliation.json').read_text());context=json.loads((target/'owned-review-failure-context.json').read_text());retained=json.loads((target/'process-evidence-retention.json').read_text())
            if result['status']!='PASS' or not result['failure_reconciled'] or not result['business_result_published'] or result['verdict_applicable']:raise ValueError('failure-outcome-contradiction:'+case)
            names=['failure_recorded','foreign_owner_refused','foreign_owner_child_waited','legacy_refused','unknown_wait_refused','sql_rollback_verified','changed_public_refused','private_changed_refused','malformed_digest_refused','forged_inbox_refused','normal_context_prepared','inapplicable_report_not_loaded','replay_without_exec','finance_estimate_held']
            if not all(proof[k] for k in names) or proof['verdict_applicable'] or proof['task_accepted'] or proof['model_received'] or proof['decision_acknowledged'] or proof['review_state']!='error' or proof['task_status']!='submitted' or proof['inbox_events']!=1:raise ValueError('failure-proof-missing:'+case)
            if not context['unavailable_reports'] or not any(r['state']=='error' for r in context['reviews']):raise ValueError('failure-context-not-explicit:'+case)
            for name in ['owned-review-failure-reconciliation.json','owned-review-failure-context.json','owned-review-public-certificate.json','owned-review-private-receipt.frame']:
                if retained['files'].get(name)!=sdk.f.digest((target/name).read_bytes()):raise ValueError('failure-original-not-retained:'+case+':'+name)
            completed.append(case);sdk.f.publish(out/'progress.json',dict(completed=completed,expected=4))
            print('PASS failed review reconciliation '+case,flush=True)
        sdk.f.publish(out/'result.json',dict(status='PASS',cases=completed,failed_calls_finalized=4,applicable_verdicts=0,task_accepted=False,financial_estimates_held=True,production_wiring=False,global_watchdog=None))
        print('PASS failed review reconciliation 4 cases',flush=True)
    except BaseException:
        sdk.f.publish(out/'failure.json',dict(status='FAIL',completed=completed,cause=traceback.format_exc()))
        raise


if __name__=='__main__':main()
