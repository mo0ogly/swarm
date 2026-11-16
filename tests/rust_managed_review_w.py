#!/usr/bin/env python3
"""Correct one observed managed W fixture-contract failure on the same owned Root.

SOURCE ONLY: independent review found managed same-attempt recovery unported.
Do not execute this proposal until source custody and canonical planner retry
acknowledgement are implemented, independently reviewed and freshly admitted.
Principal integration supplement after GLM's frozen stream, not a provider result.
No Root creation, worker/control replay, SDK/F, acceptance, SQL mutation, retry or
whole-test watchdog. Actual Campaign/OwnedRun and original retention are reused.
Run only after independent source review and fresh outer volume/capacity admission.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
from types import SimpleNamespace


def main():
    """Verify immutable preparation and run exactly one public inert review.

    Inputs: actual checkout tests, local qualified CLI/profile, preparation result,
    root verification receipt and its caller-pinned hash, fresh external output.
    Requires sole ownership, unchanged sources, a pinned observed failure/Work and a
    previously reaped worker. Acknowledges the failed original without a new call. Preserves its consumed call,
    allows only the remaining configured call, and copies the old program before replacing only
    this owned fixture's inert provider program, then performs public inspection,
    one review and reconciliation. Actual argv/channels, waited child identities,
    SR1/certificate/invocation originals are retained. A fixture verdict is never
    independent human acceptance. Errors retain typed owners via report_terminal;
    no later review/action follows a failed prerequisite or unresolved owner.
    Mount/space admission and held-volume lifetime belong to the outer launcher;
    these checks are not OS confinement or a guarantee against other same-UID work.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('source-root','normal','normal-profile','prepared','verification','before-work','output'):
        parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--verification-sha256',required=True)
    parser.add_argument('--before-work-sha256',required=True)
    parser.add_argument('--prior-failed-review',required=True)
    parser.add_argument('--event',required=True)
    args=parser.parse_args()
    for name,value in vars(args).copy().items():
        if isinstance(value,Path):setattr(args,name,value.absolute())
    checkout=args.source_root.resolve(strict=True).parent
    if args.source_root.name!='tests':raise ValueError('managed-W-actual-tests-required')
    os.chdir(checkout);sys.path.insert(0,str(args.source_root))
    import rust_unstarted_review_parent as parent
    import rust_native_managed_reviewer as reviewer
    c=None
    try:
        parent.check(__debug__,'managed-W-assertion-oracles-required')
        raw=args.verification.read_bytes()
        parent.check(hashlib.sha256(raw).hexdigest()==args.verification_sha256,'managed-W-verification-pin')
        verification=json.loads(raw);prepared=json.loads(args.prepared.read_bytes())
        parent.check(verification['state']=='READY_MANAGED_CONTROLS_NOT_REVIEWED_VERIFIED'
                     and all(value is True for value in verification['assertions'].values()),'managed-W-preparation-required')
        parent.check(prepared['status']=='READY_MANAGED_CONTROLS_NOT_REVIEWED'
                     and prepared['root']==verification['root'] and prepared['work']==verification['work']
                     and prepared['task']==verification['task'],'managed-W-bound-preparation')
        for name,sha in verification['manifest'].items():
            parent.check(not Path(name).is_absolute() and '..' not in Path(name).parts,'managed-W-manifest-key')
            path=args.prepared.parent/name
            parent.check(path.is_file() and parent.fixture.digest(path.read_bytes())==sha,'managed-W-preparation-artifact-drift')
        root=Path(prepared['root']);work=prepared['work'];task_id=prepared['task']
        parent.check(not root.is_symlink() and root.is_dir() and root.is_relative_to(checkout/'.swarm-fixtures'),'managed-W-owned-fixture-location')
        parent.check(not args.normal.is_symlink() and args.normal.is_file() and args.normal.stat().st_dev==checkout.stat().st_dev,'managed-W-local-regular-CLI-required')
        c=parent.Campaign(SimpleNamespace(normal=args.normal,normal_profile=args.normal_profile,output=args.output))
        c.helper_inputs=parent.helpers.helper_inputs(Path(parent.__file__))
        c.helper_inputs.update(parent.helpers.helper_inputs(Path(reviewer.__file__)))
        c.helper_inputs[str(Path(__file__).absolute())]=parent.fixture.digest(Path(__file__).read_bytes())
        parent.fixture.publish(c.output/'helper-inputs.json',c.helper_inputs)
        profileout=c.output/'normal-profile';profileout.mkdir()
        c.normal_inputs=parent.binding.qualify(args.normal,args.normal_profile,profileout)
        c.fixtures['W']={'spec':{'root':str(root),'work':work}}
        def call(label,arguments,request=None,observe=False):
            """Rebind actual sources before one owned public CLI child; preserve refusal."""
            c.stable();return c.cli('W',label,arguments,request,observe)
        def show(label):
            """Read only the already-bound Work, with the same owned public transport."""
            value,_=call(label,['work','show',work]);return value['work']
        before=show('W-before')
        expected_raw=args.before_work.read_bytes()
        parent.check(parent.fixture.digest(expected_raw)==args.before_work_sha256,'managed-W-current-snapshot-pin')
        expected=json.loads(expected_raw)['work']
        parent.check(parent.equal(before,expected),'managed-W-current-Work-unchanged')
        initial=json.loads((args.prepared.parent/'work-after-validate.json').read_bytes())
        parent.check(before['planning']['reviewer']['calls']==initial['planning']['reviewer']['calls']+1
                     and before['planning']['reviewer']['calls']<before['planning']['reviewer']['max_calls'],
                     'managed-W-one-failed-call-existing-capacity')
        import copy
        projection=copy.deepcopy(before)
        for key in ('revision','updated'):projection[key]=initial[key]
        projection['planning']['reviewer']['calls']=initial['planning']['reviewer']['calls']
        initial_inbox=initial['planning']['inbox'];current_inbox=projection['planning']['inbox']
        parent.check(parent.equal(current_inbox[:len(initial_inbox)],initial_inbox) and len(current_inbox)==len(initial_inbox)+1
                     and current_inbox[-1]['kind']=='independent_review' and current_inbox[-1]['id']==args.prior_failed_review
                     and current_inbox[-1]['task']==task_id,'managed-W-observed-failed-inbox-only')
        projection['planning']['inbox']=initial_inbox
        for item in projection['tasks']:item.pop('independent_review',None)
        parent.check(parent.equal(projection,initial),'managed-W-no-other-preparation-change')
        task=next(t for t in before['tasks'] if t['id']==task_id)
        parent.check(task['criteria'] and task['independent_review']['id']==args.prior_failed_review
                     and task['independent_review']['state']=='error','managed-W-existing-failed-review-contract')
        failed_replay,_=call('W-prior-failed-reconcile',['task','review-reconcile',args.prior_failed_review])
        parent.check(failed_replay['replayed'] is True and failed_replay['verdict_applicable'] is False
                     and failed_replay['task_accepted'] is False,'managed-W-prior-error-not-verdict')
        parent.check(parent.equal(show('W-after-prior-reconcile'),before),'managed-W-prior-reconcile-changed-Work')
        settings,_=call('W-settings',['engine-settings','show']);c.fixtures['W']['settings']=settings['values']
        parent.ledger(root,work,settings['values'],c.output/'ledger-before.json')
        old=root/'codex'
        parent.check(not old.is_symlink() and old.is_file(),'managed-W-owned-regular-program')
        parent.fixture.publish(c.output/'worker-before.py',old.read_bytes())
        parent.check(args.event and all(ch in 'abcdefghijklmnopqrstuvwxyz0123456789-' for ch in args.event),'managed-W-simple-managed-event')
        config={'case':'pass','marker_event':args.event}
        program=reviewer.provider_program(config)
        parent.fixture.publish(c.output/'inert-reviewer.py',program);parent.fixture.publish(c.output/'inert-config.json',config)
        parent.fixture.publish(old,program);old.chmod(0o700)
        request={'schema_version':1,'event_id':args.event,'expected_revision':before['revision'],'task_id':task_id}
        preview,_=call('W-review-inputs',['task','review-inputs',work],request)
        p=preview['preview']
        parent.check(p['native_control_evidence_verified'] is True and p['invocation_context_materialized'] is True
                     and all(p[k] is False for k in ('invocation_authorized','provider_started','task_accepted')),'managed-W-readonly-inputs-required')
        parent.check(parent.equal(show('W-before-review'),before),'managed-W-readonly-preview-mutated-Work')
        result,owner=call('W-review',['task','review',work],request,True)
        after=show('W-after-review')
        parent.check(after['planning']['reviewer']['calls']==before['planning']['reviewer']['calls']+1,'managed-W-exact-one-review-call')
        retained=parent.retain_review(root,after,task_id,result,owner,c.output/'W-originals',managed_protocol=preview['managed_protocol'])
        reconciled,_=call('W-reconcile',['task','review-reconcile',result['review_id']])
        parent.check(reconciled['replayed'] is True and reconciled['review_id']==result['review_id']
                     and reconciled['verdict_applicable'] is True and reconciled['task_accepted'] is False,'managed-W-public-reconciliation')
        final=show('W-final')
        parent.check(final['planning']['reviewer']['calls']==after['planning']['reviewer']['calls'],'managed-W-reconcile-no-new-call')
        parent.ledger(root,work,settings['values'],c.output/'ledger-after.json');c.stable()
        parent.check(all(not r.pending() and r.reaped and r.status==0 for r in c.runs),'managed-W-owned-processes-settled')
        receipt={'status':'HEALTHY_MANAGED_W_REVIEW_RETAINED_NOT_ACCEPTED','root':str(root),'work':work,'task':task_id,
                 'review_id':result['review_id'],'reviewer_calls_before':before['planning']['reviewer']['calls'],'reviewer_calls_after':final['planning']['reviewer']['calls'],'prior_failed_review':args.prior_failed_review,'retention':retained,'task_accepted':False,
                 'global_watchdog':None,'process_waits':[r.summary() for r in c.runs],
                 'scope':'One actual inert reviewer W on current managed fixture; no SDK/F, independent human judgement, task acceptance or installation'}
        parent.fixture.publish(c.output/'W-result.json',receipt);print(json.dumps(receipt,sort_keys=True));return 0
    except BaseException as error:
        observer=c.failure_observations if c is not None and 'settings' in c.fixtures.get('W',{}) else None
        return parent.report_terminal(error,c.output if c is not None else args.output,observer)


if __name__=='__main__':sys.exit(main())
