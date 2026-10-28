#!/usr/bin/env python3
"""Verify normal native planner claim/preview consumes an actual published review.

Only an explicit token-bound existing inert fixture. A claim reserves a planner
call but never runs it; preview is read-only. No model, Go, deployment or global
watchdog. The explicit60s fixture lease is a product input, not a test deadline.
"""
import argparse,json,os,subprocess,sys,traceback
from pathlib import Path
import rust_reviewer_claim_candidate as sdk


def main():
    """Require a qualified normal binary and actual native review before claiming.

    Preserve every public command and original channel. Validate the pending
    review event enters the saved delivery and pure preview has the same hash;
    optional expected state also checks failed-report inapplicability. Do not
    confuse prepared context with model reception or decision. No budget
    reset, private SQL write, fixture provider execution or implicit retry occurs.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',required=True,type=Path)
    parser.add_argument('--profile',required=True,type=Path)
    parser.add_argument('--fixture',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--expected-review-state',choices=('passed','changes_requested','error','stale'))
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    try:
        binary=args.binary.resolve();profile=json.loads(args.profile.read_text());manifest=json.loads(args.fixture.read_text());root=Path(manifest['root']);work=manifest['work']
        if profile['status']!='VERIFIED_DEFAULT_NATIVE_PROFILE' or not profile['all_core_test_false'] or not profile['all_core_features_empty'] or not profile['no_fixture_feature'] or sdk.f.digest(binary.read_bytes())!=profile['sha256']:raise ValueError('normal-default-profile-required')
        if (root/'.swarm/owned-review-claim-fixture').read_text()!=manifest['token'] or not manifest['execute_reviewer']:raise ValueError('owned-review-fixture-required')
        sdk.f.publish(out/'method.py',Path(__file__).read_bytes())
        invocation=(root/'reviewer-invoked.json').read_bytes()
        def call(name,argv,value=None):
            """Run a public CLI operation once with the fixture provider home."""
            command=[str(binary),*argv,'--root',str(root),'--json']
            if value is not None:
                path=out/(name+'.input.json');sdk.f.publish(path,value);command+=['--input',str(path)]
            sdk.f.publish(out/(name+'.command.json'),dict(argv=command,global_watchdog=None))
            result=subprocess.run(command,capture_output=True,stdin=subprocess.DEVNULL,env={**os.environ,'CODEX_HOME':str(root/'codex-home')})
            sdk.f.publish(out/(name+'.stdout'),result.stdout);sdk.f.publish(out/(name+'.stderr'),result.stderr)
            if result.returncode:raise ValueError('public-context-operation-failed:'+name)
            return json.loads(result.stdout)
        before=call('before',['work','show',work])['work']
        pending=[e for e in before['planning'].get('inbox',[]) if e['kind']=='independent_review' and not e.get('decision')]
        if len(pending)!=1:raise ValueError('one-pending-native-review-required')
        event=pending[0]
        if args.expected_review_state:
            task=next(t for t in before['tasks'] if t['id']==event['task'])
            reviews=[task.get('independent_review'),*task.get('previous_reviews',[])]
            original=next((r for r in reviews if r and r['id']==event['id']),None)
            if original is None or original['state']!=args.expected_review_state:raise ValueError('public-review-state-mismatch')
            if args.expected_review_state in ('error','stale') and event.get('artifacts'):raise ValueError('error-misrepresented-as-report')
        scope=next(s for s in before['planning']['scopes'] if s['id']==event['scope'])
        claim='review-context-'+os.urandom(8).hex()
        request=dict(schema_version=1,event_id=claim,expected_revision=before['revision'],scope=scope['id'],scope_revision=scope['revision'],holder='fixture-review-consumer',lease_seconds=60)
        claimed=call('claim',['planning','claim',work],request)
        after_claim=call('after-claim',['work','show',work])['work'];scope=next(s for s in after_claim['planning']['scopes'] if s['id']==event['scope'])
        if event['id'] not in scope['delivery']['events']:raise ValueError('native-review-not-selected-for-delivery')
        unavailable=scope['delivery'].get('unavailable_reports') or {}
        expected_unavailable={'error':'native-review-failed-no-applicable-verdict','stale':'native-review-stale-no-applicable-verdict'}.get(args.expected_review_state)
        if expected_unavailable and unavailable.get(event['id'])!=expected_unavailable:raise ValueError('nonverdict-review-not-explicitly-inapplicable')
        if args.expected_review_state in ('passed','changes_requested') and event['id'] in unavailable:raise ValueError('valid-verdict-misrepresented-as-failed')
        request=dict(schema_version=1,expected_revision=after_claim['revision'],claim_id=claim,scope=scope['id'],scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'])
        preview=call('preview',['planning','call-preview',work],request)
        after_preview=call('after-preview',['work','show',work])['work']
        if after_preview!=after_claim or preview['context_sha256']!=scope['delivery']['context_sha256']:raise ValueError('preview-mutated-or-context-drifted')
        if preview['process_started'] is not False or preview['prepared_inputs_verified'] is not True:raise ValueError('preview-not-prepared-or-process-started')
        if any(e.get('decision') for e in after_preview['planning']['inbox'] if e['id']==event['id']):raise ValueError('review-event-was-acknowledged')
        if (root/'reviewer-invoked.json').read_bytes()!=invocation:raise ValueError('unexpected-provider-reinvocation')
        sdk.f.publish(out/'result.json',dict(status='PASS_NORMAL_NATIVE_PUBLIC_CONTEXT',binary_sha256=profile['sha256'],native_review_event=event['id'],expected_review_state=args.expected_review_state,failed_report_explicitly_inapplicable=args.expected_review_state=='error',stale_report_explicitly_inapplicable=args.expected_review_state=='stale',selected_events=scope['delivery']['events'],context_sha256=preview['context_sha256'],preview_pure=True,new_planner_claim_recorded=True,fixture_lease_seconds=60,model_received=False,decision_acknowledged=False,provider_started=False,global_watchdog=None))
        print('PASS normal native public review context',flush=True)
    except BaseException:
        sdk.f.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise


if __name__=='__main__':main()
