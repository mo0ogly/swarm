#!/usr/bin/env python3
"""Replay eight original-based reviewer publication/refusal cases on owned Roots.

Actual inert processes and immutable SDK inputs; no paid model, Go, production
installation, global watchdog or task acceptance. Financial estimates stay held.
"""
import argparse,json,subprocess,sys,traceback
from pathlib import Path
import rust_reviewer_claim_candidate as sdk


def main():
    """Require exact coverage and evidence, retaining every original channel.

    Two verdicts and two zero-capture variants publish once with one scoped
    inbox event; source/reply/SQL/public/private corruption tests precede their
    verified replay. Four actual failed calls remain inapplicable in this publication-only
    recipe; explicit failure reconciliation has its own replay. All subprocesses are sequential, without watchdog.
    Fresh exclusive output preserves partial completion on any failure.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--sdk-source-root',type=Path)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    cases=[(n,n,None) for n in ['complete','changes-requested','duration','source','output-error','domain-invalid']]
    cases += [('zero-bytes','complete',{'agent_capture_max_bytes':'0','agent_capture_max_lines':None}),('zero-lines','complete',{'agent_capture_max_bytes':None,'agent_capture_max_lines':'0'})]
    completed=[];published=[]
    sdk.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        for name,case,policy in cases:
            target=out/name
            argv=[sys.executable,str(Path(__file__).with_name('rust_reviewer_claim_candidate.py')),'--binary',str(args.binary.resolve()),'--output',str(target),'--execute-reviewer','--final-reserved-call','--controller-case',case,'--require-raw-capture','--publish-review']
            if case=='complete':argv.append('--stream-handshake')
            if policy is not None:argv+=['--capture-policy',json.dumps(policy)]
            if args.sdk_source_root:argv+=['--sdk-source-root',str(args.sdk_source_root.resolve())]
            sdk.f.publish(out/(name+'.command.json'),dict(argv=argv,global_watchdog=None))
            with (out/(name+'.stdout')).open('xb') as stdout,(out/(name+'.stderr')).open('xb') as stderr:
                result=subprocess.run(argv,stdout=stdout,stderr=stderr,stdin=subprocess.DEVNULL)
            if result.returncode:raise ValueError('review-publication-case-failed:'+name)
            actual=json.loads((target/'result.json').read_text());proof=json.loads((target/'owned-review-publication.json').read_text())
            expected=case in ('complete','changes-requested')
            if actual['status']!='PASS' or actual['business_result_published']!=expected or actual['result_published']!=expected or proof['business_result_published']!=expected or proof['task_accepted']:raise ValueError('publication-outcome-contradiction:'+name)
            if expected:
                required=['source_changed_refused','invalid_reply_refused','sql_rollback_verified','changed_public_original_refused','replay_without_call','changed_private_replay_refused']
                if not all(proof[key] for key in required) or proof['inbox_events']!=1:raise ValueError('publication-fence-proof-missing:'+name)
                if proof['task_status']!=('blocked' if case=='changes-requested' else 'submitted'):raise ValueError('publication-task-state:'+name)
                certificate=json.loads((target/'owned-review-public-certificate.json').read_text())
                if set(certificate)!={'schema_version','review','work_id','task_id','source_sha256','validation_receipt_sha256'}:raise ValueError('public-certificate-fields:'+name)
                consumer=json.loads((target/'owned-review-consumer-faults.json').read_text())
                context=json.loads((target/'owned-review-supervisor-context.json').read_text())
                if not all(consumer[key] for key in ['context_materialized','original_report_present','public_changed_refused','public_oversized_refused','private_changed_refused','report_changed_refused','forged_inbox_refused','work_unchanged']):raise ValueError('consumer-proof-missing:'+name)
                if consumer['model_received'] or consumer['decision_acknowledged'] or not context['reviews']:raise ValueError('consumer-authority-claim:'+name)
                published.append(name)
            elif not proof['failed_original_refused']:raise ValueError('failed-original-not-refused:'+name)
            completed.append(name);sdk.f.publish(out/'progress.json',dict(completed=completed,expected=len(cases),published=published))
            print('PASS publication '+name,flush=True)
        if len(completed)!=8 or len(published)!=4:raise ValueError('publication-coverage-incomplete')
        sdk.f.publish(out/'result.json',dict(status='PASS',cases=completed,published=published,published_count=4,refused_failed_originals=4,task_accepted=False,production_wiring=False,financial_reservations_held=True,global_watchdog=None))
        print('PASS reviewer publication 8 cases',flush=True)
    except BaseException:
        sdk.f.publish(out/'failure.json',dict(status='FAIL',completed=completed,expected=len(cases),cause=traceback.format_exc()))
        raise


if __name__=='__main__':main()
