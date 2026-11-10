#!/usr/bin/env python3
"""Replay six genuine originals across their creator's actual process exit.

Two valid verdicts and four failed calls; every recovery is a separate SDK
process with no provider exec/adoption/refund/acceptance or global watchdog.
"""
import argparse,json,subprocess,sys,traceback
from pathlib import Path
import rust_reviewer_claim_candidate as sdk


def main():
    """Retain exact per-case channels and require explicit owner/source proofs.

    Each inner case qualifies original and recovery SDK independently and retains
    the original receipts/context. Zero tests, missing evidence or any failed
    case prevents completion; partial progress and original causes remain saved.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',required=True,type=Path);parser.add_argument('--output',required=True,type=Path);parser.add_argument('--sdk-source-root',type=Path)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False);completed=[]
    sdk.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        for case in ['complete','changes-requested','duration','source','output-error','domain-invalid']:
            target=out/case;argv=[sys.executable,str(Path(__file__).with_name('rust_review_departed_owner.py')),'--binary',str(args.binary.resolve()),'--output',str(target),'--controller-case',case]
            if args.sdk_source_root:argv+=['--sdk-source-root',str(args.sdk_source_root.resolve())]
            sdk.f.publish(out/(case+'.command.json'),dict(argv=argv,global_watchdog=None))
            with (out/(case+'.stdout')).open('xb') as stdout,(out/(case+'.stderr')).open('xb') as stderr:result=subprocess.run(argv,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
            if result.returncode:raise ValueError('departed-owner-case-failed:'+case)
            result=json.loads((target/'result.json').read_text());proof=json.loads((target/'owned-review-departed-recovery.json').read_text())
            expected='passed' if case=='complete' else 'changes_requested' if case=='changes-requested' else 'error'
            valid=expected!='error'
            required=['owner_confirmed_departed','original_owner_preserved','known_wait_required','foreign_host_refused','live_tuple_refused','unknown_wait_refused','sql_rollback_verified','changed_public_refused','changed_private_refused','replay_without_exec','provider_invocation_unchanged','financial_estimate_held']
            if result['status']!='PASS' or result['provider_calls']!=1 or result['owner_adopted'] or result['task_accepted'] or result['settlement'] or not all(proof[k] for k in required) or proof['verdict_applicable']!=valid or proof['review_state']!=expected or proof['model_received'] or proof['decision_acknowledged']:raise ValueError('departed-owner-proof-invalid:'+case)
            if proof['source_guard_for_verdict']!=valid or proof['source_inapplicable_for_failed_call']==valid:raise ValueError('departed-source-mode-invalid:'+case)
            completed.append(case);sdk.f.publish(out/'progress.json',dict(completed=completed,expected=6));print('PASS departed owner '+case,flush=True)
        sdk.f.publish(out/'result.json',dict(status='PASS',cases=completed,recovered_originals=6,verdicts=2,failed_calls=4,provider_execs_total=6,recovery_provider_execs=0,owner_adoptions=0,task_accepted=False,financial_estimates_held=True,production_wiring=False,global_watchdog=None))
        print('PASS departed owner recovery 6 cases',flush=True)
    except BaseException:
        sdk.f.publish(out/'failure.json',dict(status='FAIL',completed=completed,cause=traceback.format_exc()))
        raise


if __name__=='__main__':main()
