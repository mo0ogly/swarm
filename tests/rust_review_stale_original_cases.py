#!/usr/bin/env python3
"""Four actual original lifecycles and normal public stale/non-verdict context.

Each fixture runs one inert provider and a later SDK; no recovery exec, paid
provider, Go, adoption, acceptance, settlement or global watchdog. Paused case
uses explicit public resume before testing public claim, preserving the result.
"""
import argparse,json,subprocess,sys,traceback
from pathlib import Path
import rust_reviewer_claim_candidate as sdk


def main():
    """Require all four typed dependency changes and retained failure guards.

    Original SDK/public profile qualification, exact financial state and current
    source refusals precede PASS. One case failure preserves channels/progress
    and aborts this recipe, never reports a green average or a guessed timeout.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--sdk-source-root',type=Path);p.add_argument('--output',required=True,type=Path)
    args=p.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False);completed=[];sdk.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        for variant,reason in [('report','report-changed'),('settings','settings-changed'),('paused','planning-inactive'),('authority','validation-source-changed')]:
            target=out/variant;argv=[sys.executable,str(Path(__file__).with_name('rust_review_departed_owner.py')),'--binary',str(args.binary.resolve()),'--output',str(target),'--stale-original','--stale-variant',variant]
            if args.sdk_source_root:argv+=['--sdk-source-root',str(args.sdk_source_root.resolve())]
            sdk.f.publish(out/(variant+'.command.json'),dict(argv=argv,global_watchdog=None))
            with (out/(variant+'.stdout')).open('xb') as stdout,(out/(variant+'.stderr')).open('xb') as stderr:r=subprocess.run(argv,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
            if r.returncode:raise ValueError('stale-original-case-failed:'+variant)
            result=json.loads((target/'result.json').read_text());proof=json.loads((target/'owned-review-stale-recovery.json').read_text());manifest=json.loads((target/'fixture.json').read_text());root=Path(manifest['root'])
            required=['original_candidate_preserved','provider_invocation_unchanged','replay_verified','current_report_not_loaded','unchanged_refused','unreadable_refused','missing_report_refused','sql_rollback_verified','public_changed_refused','private_changed_refused','finance_estimate_held']
            if result['status']!='PASS' or result['provider_calls']!=1 or result['recovery_execs']!=0 or not all(proof[k] for k in required) or proof['reason']!=reason or proof['variant']!=variant or proof['state']!='stale' or proof['verdict_applicable'] or proof['task_accepted']:raise ValueError('stale-original-proof-invalid:'+variant)
            if (root/'.swarm/owned-review-claim-fixture').read_text()!=manifest['token']:raise ValueError('stale-fixture-token-changed')
            if variant=='paused':
                code,current=sdk.inputs.native.invoke(args.binary.resolve(),root,target,'public-paused-work',['work','show',manifest['work']]);assert code==0,current
                request=dict(schema_version=1,event_id='owned-review-stale-context-resume',expected_revision=current['work']['revision'],reason='Explicit owned normal context check after stale publication')
                code,resumed=sdk.inputs.native.invoke(args.binary.resolve(),root,target,'public-context-resume',['planning','resume',manifest['work']],request);assert code==0,resumed
            argv=[sys.executable,str(Path(__file__).with_name('rust_review_public_context.py')),'--binary',str(args.binary.resolve()),'--profile',str(args.profile.resolve()),'--fixture',str(target/'fixture.json'),'--output',str(out/(variant+'-context')),'--expected-review-state','stale']
            sdk.f.publish(out/(variant+'-context.command.json'),dict(argv=argv,global_watchdog=None))
            with (out/(variant+'-context.stdout')).open('xb') as stdout,(out/(variant+'-context.stderr')).open('xb') as stderr:r=subprocess.run(argv,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
            if r.returncode:raise ValueError('stale-normal-public-context-failed:'+variant)
            context=json.loads((out/(variant+'-context')/'result.json').read_text())
            if context['expected_review_state']!='stale' or not context['stale_report_explicitly_inapplicable'] or context['model_received'] or context['decision_acknowledged']:raise ValueError('stale-context-proof-invalid:'+variant)
            completed.append(variant);sdk.f.publish(out/'progress.json',dict(completed=completed,expected=4));print('PASS stale original '+variant,flush=True)
        sdk.f.publish(out/'result.json',dict(status='PASS',variants=completed,original_provider_execs=4,recovery_provider_execs=0,normal_public_contexts=4,applicable_verdicts=0,task_accepted=False,financial_estimates_held=True,production_wiring=False,global_watchdog=None));print('PASS stale original recovery 4 cases',flush=True)
    except BaseException:
        sdk.f.publish(out/'failure.json',dict(status='FAIL',completed=completed,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
