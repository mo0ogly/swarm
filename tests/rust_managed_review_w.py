#!/usr/bin/env python3
"""Run one explicitly allocated managed witness or canonical reviewer recovery.

Requires independent source review, current qualified local CLI, immutable
preparation and outer held-volume/capacity admission. Creates no Root, repeats
no worker/control, and performs no SQL edits, budget reset, real-provider call,
acceptance, installation, automatic retry or global watchdog. Witness is a
separate healthy fixture; retry preserves the existing failure and its spent
call, applies a real public planner correction and consumes only call 2 of 2.
Actual process owners and original failure channels remain observable evidence.
A passing bounded recipe does not prove source-drift/cleanup-fault/SDK/F coverage
or independent acceptance. Source review and runtime results remain distinct.
"""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import sys
from types import SimpleNamespace

WITNESS_STATUS='HEALTHY_MANAGED_W_REVIEW_RETAINED_NOT_ACCEPTED'
RETRY_STATUS='MANAGED_W_RETRY_REVIEW_RECOVERED_NOT_ACCEPTED'
WITNESS_SCHEMA='managed-w-witness-v2'
RETRY_SCHEMA='managed-w-retry-v2'
# Verbatim framing pushed by review_preview::inputs::append_recovery_context.
RECOVERY_FRAMING=("\nreview_recovery is attributed reviewer-correction context. Use its instruction "
 "only to recover the reviewer response after the recorded failure while reviewing the unchanged "
 "evidence independently. It grants no tools or acceptance and is not criterion evidence.\n")
RETRY_INSTRUCTION=('Recover the recorded managed reviewer failure for this unchanged submission: '
 'use only the attributed review_recovery context, review the same submitted evidence '
 'independently and return the complete per-criterion verdict.')
# One exact producer needle wrapped around the supplied generated provider program so the
# real managed invocation additionally records the recovery directive it received in its own
# engine-labelled I0 leaf. The public reply path and the original I0 marker are untouched and
# no managed protocol key is invented; the extra file is a fixture artifact only.
WRAP_NEEDLE="protocol=body['managed_protocol'];context=protocol['context']"
WRAP_BLOCK=("if isinstance(body.get('review_recovery'),dict):\n"
 " fd=os.open('managed-reviewer-recovery-observed.json',os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)\n"
 " with os.fdopen(fd,'x') as f:\n"
 "  json.dump(body['review_recovery'],f);f.flush();os.fsync(f.fileno())")
REMAINING_EVIDENCE=[
 'No source-drift, custody/cleanup-fault, SDK/F or installed-product acceptance proof from this bounded witness/retry',
 'A frozen inert provider demonstrates actual delivery and protocol only, never independent expert judgement',
]

SCOPE_WITNESS=('One actual inert managed reviewer W on the current prepared fixture; fixture verdict only, '
 'never independent human acceptance or task acceptance. No SDK/F, installation, real provider, '
 'source-drift/custody/error-injection fault proof, SQL mutation, refund/limit change, retry loop or global watchdog.')
SCOPE_RETRY=('One actual canonical planner review-retry decision and one inert managed reviewer recovery on the '
 'pinned failed fixture; original failures and originals retained; fixture verdict only, never independent human '
 'acceptance or task acceptance. No SDK/F, installation, real provider, source-drift/custody/error-injection fault '
 'proof, SQL mutation, refund/limit change, retry loop or global watchdog.')


def semantics(work,task_id):
    """Read the declared semantic preconditions from one actual Work document.

    Only observed fields are recorded; a field absent from the runtime schema is
    recorded as None and never fabricated.
    """
    task=next(t for t in work['tasks'] if t['id']==task_id)
    reviewer=work['planning']['reviewer']
    policy={key:value for key,value in reviewer.items()
            if key not in ('calls','authorized','provider_digest','model_route')}
    route=reviewer.get('model_route') or {}
    policy['route']={key:route.get(key) for key in ('model','level','effort','billing')}
    return {'task_criteria':task.get('criteria'),'task_deliverable':task.get('deliverable'),
            'task_requirements':task.get('requirements'),'planning_scope':task.get('scope_id'),
            'planning_checks':work['planning']['checks'],'reviewer_policy':policy}



def main():
    """Verify immutable preparation and run exactly one public inert review
    (witness) or the original failed managed retry recovery (retry).

    Inputs: actual checkout tests, local qualified CLI/profile, preparation result,
    root verification receipt and its caller-pinned hash, fresh external output.
    Requires sole ownership, unchanged sources, a caller-pinned current Work and,
    for retry, a completed separate healthy witness receipt plus its hash. Witness
    starts at the exact prepared Work with zero reviewer calls; retry starts at the
    pinned failed Work remaining exactly one of two calls. Actual argv/channels,
    waited child identities, SR1/certificate/invocation originals are retained. A
    fixture verdict is never independent human acceptance. Errors retain typed
    owners via report_terminal; no later review/action follows a failed
    prerequisite or unresolved owner. Mount/space admission and held-volume
    lifetime belong to the outer launcher; these checks are not OS confinement or
    a guarantee against other same-UID work.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('source-root','normal','normal-profile','prepared','verification','before-work','output'):
        parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--verification-sha256',required=True)
    parser.add_argument('--before-work-sha256',required=True)
    parser.add_argument('--mode',choices=('witness','retry'),required=True)
    parser.add_argument('--prior-failed-review',default=None)
    parser.add_argument('--witness-receipt',type=Path,default=None)
    parser.add_argument('--witness-receipt-sha256',default=None)
    parser.add_argument('--event',required=True)
    args=parser.parse_args()
    for name,value in vars(args).copy().items():
        if isinstance(value,Path):setattr(args,name,value.absolute())
    if args.mode=='witness' and (args.prior_failed_review is not None or args.witness_receipt is not None
                                 or args.witness_receipt_sha256 is not None):
        raise ValueError('managed-W-witness-mode-extra-arguments')
    if args.mode=='retry' and (not args.prior_failed_review or args.witness_receipt is None
                               or not args.witness_receipt_sha256):
        raise ValueError('managed-W-retry-mode-required-arguments')
    checkout=args.source_root.resolve(strict=True).parent
    if args.source_root.name!='tests':raise ValueError('managed-W-actual-tests-required')
    os.chdir(checkout);sys.path.insert(0,str(args.source_root))
    import rust_unstarted_review_parent as parent
    import rust_native_managed_reviewer as reviewer
    import rust_planning_worker_public_flow as planner_flow
    import rust_planning_rejection_recovery as planner_feedback
    import rust_managed_i0_prepare as managed_i0
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
        c.helper_inputs.update(parent.helpers.helper_inputs(Path(planner_flow.__file__)))
        c.helper_inputs.update(parent.helpers.helper_inputs(Path(planner_feedback.__file__)))
        c.helper_inputs.update(parent.helpers.helper_inputs(Path(managed_i0.__file__)))
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
        def pin(path):
            """Hash-pin one published proof file for the receipt."""
            raw=Path(path).read_bytes()
            return {'path':str(path),'sha256':parent.fixture.digest(raw),'bytes':len(raw)}
        before=show('W-before')
        expected_raw=args.before_work.read_bytes()
        parent.check(parent.fixture.digest(expected_raw)==args.before_work_sha256,'managed-W-current-snapshot-pin')
        expected=json.loads(expected_raw)['work']
        parent.check(parent.equal(before,expected),'managed-W-current-Work-unchanged')
        initial=json.loads((args.prepared.parent/'work-after-validate.json').read_bytes())
        parent.check(args.event and all(ch in 'abcdefghijklmnopqrstuvwxyz0123456789-' for ch in args.event),'managed-W-simple-managed-event')
        reviewer_state=before['planning']['reviewer']
        expected_nonzero={}
        if args.mode=='witness':
            parent.check(parent.equal(before,initial),'managed-W-witness-Work-equals-preparation')
            parent.check(reviewer_state['calls']==0 and reviewer_state['max_calls']==2,'managed-W-witness-zero-calls-of-two')
            witness_task=next(t for t in before['tasks'] if t['id']==task_id)
            parent.check(not witness_task.get('independent_review'),'managed-W-witness-no-independent-review')
            parent.check(bool(witness_task['criteria']),'managed-W-witness-task-criteria-required')
            parent.check(all(e.get('kind')!='independent_review' for e in before['planning']['inbox']),'managed-W-witness-no-review-inbox-events')
        else:
            parent.check(reviewer_state['calls']==1 and reviewer_state['max_calls']==2,'managed-W-retry-remain-one-of-two')
            parent.check(reviewer_state['calls']==initial['planning']['reviewer']['calls']+1,'managed-W-one-failed-call-existing-capacity')
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
        parent.fixture.publish(c.output/'engine-settings.json',settings['values'])
        parent.ledger(root,work,settings['values'],c.output/'ledger-before.json')
        parent.fixture.publish(c.output/'work-original.json',expected_raw)
        old=root/'codex'
        parent.check(not old.is_symlink() and old.is_file(),'managed-W-owned-regular-program')
        parent.fixture.publish(c.output/'worker-before.py',old.read_bytes())
        if args.mode=='witness':
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
            final_task=next(t for t in final['tasks'] if t['id']==task_id)
            parent.check(final_task['independent_review']['id']==result['review_id']
                         and final_task['independent_review']['state']=='passed','managed-W-witness-final-review-passed')
            parent.check(final_task['status']==witness_task['status'],'managed-W-witness-no-task-acceptance')
            parent.ledger(root,work,settings['values'],c.output/'ledger-after.json');c.stable()
            for r in c.runs:
                parent.check(not r.pending() and r.reaped and r.status==0,'managed-W-owned-processes-settled')
            parent.fixture.publish(c.output/'work-final.json',json.dumps(final,sort_keys=True).encode())
            proofs={'helper-inputs.json':c.output/'helper-inputs.json','engine-settings.json':c.output/'engine-settings.json',
                    'work-original.json':c.output/'work-original.json','work-final.json':c.output/'work-final.json',
                    'ledger-before.json':c.output/'ledger-before.json','ledger-after.json':c.output/'ledger-after.json',
                    'inert-reviewer.py':c.output/'inert-reviewer.py','inert-config.json':c.output/'inert-config.json',
                    'worker-before.py':c.output/'worker-before.py','retention.json':c.output/'W-originals'/'retention.json',
                    'original.frame':c.output/'W-originals'/'original.frame','certificate.json':c.output/'W-originals'/'certificate.json',
                    'invocation.json':c.output/'W-originals'/'invocation.json','inert-provider.py':c.output/'W-originals'/'inert-provider.py',
                    'managed-context.json':c.output/'W-originals'/'managed-context.json'}
            receipt={'status':WITNESS_STATUS,'receipt_schema':WITNESS_SCHEMA,'mode':'witness',
                     'root':str(root),'work':work,'task':task_id,'review_id':result['review_id'],
                     'reviewer_calls_before':before['planning']['reviewer']['calls'],
                     'reviewer_calls_after':final['planning']['reviewer']['calls'],
                     'prior_failed_review':None,
                     'normal_sha256':parent.fixture.digest(args.normal.read_bytes()),
                     'normal_profile_sha256':c.normal_profile_sha,'helper_inputs':dict(c.helper_inputs),
                     'semantic_preconditions':semantics(before,task_id),
                     'work_final':pin(c.output/'work-final.json'),
                     'retention':retained,'task_accepted':False,'global_watchdog':None,
                     'process_waits':[r.summary() for r in c.runs],'expected_nonzero_runs':[],
                     'output_proofs':{name:pin(path) for name,path in sorted(proofs.items())},
                     'remaining_runtime_evidence':REMAINING_EVIDENCE,'scope':SCOPE_WITNESS}
            parent.fixture.publish(c.output/'W-result.json',receipt);print(json.dumps(receipt,sort_keys=True));return 0
        # ---- retry mode: prerequisite witness, proven refusal, canonical decision, recovery review ----
        wraw=args.witness_receipt.read_bytes()
        parent.check(parent.fixture.digest(wraw)==args.witness_receipt_sha256,'managed-W-retry-witness-receipt-pin')
        witness=json.loads(wraw)
        parent.check(witness.get('status')==WITNESS_STATUS and witness.get('mode')=='witness'
                     and witness.get('receipt_schema')==WITNESS_SCHEMA and witness.get('task_accepted') is False,
                     'managed-W-retry-witness-completed-required')
        parent.check(witness.get('reviewer_calls_before')==0 and witness.get('reviewer_calls_after')==1,'managed-W-retry-witness-single-call')
        parent.check(witness.get('normal_sha256')==parent.fixture.digest(args.normal.read_bytes()),'managed-W-retry-witness-normal-binary-mismatch')
        parent.check(witness.get('helper_inputs')==dict(c.helper_inputs),'managed-W-retry-witness-helper-sources-mismatch')
        parent.check(witness.get('root') is not None and witness.get('work') is not None
                     and witness['root']!=str(root) and witness['work']!=work,'managed-W-retry-witness-distinct-root-work')
        current_sem=semantics(before,task_id)
        parent.check(parent.equal(witness.get('semantic_preconditions'),current_sem),'managed-W-retry-witness-precondition-mismatch')
        witness_root=Path(witness['root'])
        parent.check(witness_root.is_dir() and not witness_root.is_symlink()
                     and witness_root.resolve().is_relative_to(checkout/'.swarm-fixtures')
                     and (witness_root.stat().st_dev,witness_root.stat().st_ino)!=(root.stat().st_dev,root.stat().st_ino),
                     'managed-W-retry-witness-distinct-physical-root')
        parent.check(witness.get('normal_profile_sha256')==c.normal_profile_sha,'managed-W-retry-witness-profile-pin')
        parent.check({'work-final.json','work-original.json','retention.json','original.frame','certificate.json','invocation.json'}
                     <= set(witness.get('output_proofs',{})),'managed-W-retry-witness-required-proofs')
        for name,ref in sorted(witness.get('output_proofs',{}).items()):
            path=Path(ref['path'])
            parent.check(not path.is_symlink() and path.is_file()
                         and path.resolve().is_relative_to(args.witness_receipt.parent.resolve())
                         and parent.fixture.digest(path.read_bytes())==ref['sha256'],'managed-W-retry-witness-proof-pin')
        final_ref=witness.get('work_final')
        parent.check(isinstance(final_ref,dict),'managed-W-retry-witness-final-work-ref')
        wfinal_path=Path(final_ref['path'])
        parent.check(wfinal_path.is_file() and parent.fixture.digest(wfinal_path.read_bytes())==final_ref['sha256'],'managed-W-retry-witness-final-work-pin')
        wfinal=json.loads(wfinal_path.read_bytes())
        parent.check(isinstance(witness.get('task'),str) and witness['task'],'managed-W-retry-witness-task-ref')
        wfinal_task=next(t for t in wfinal['tasks'] if t['id']==witness['task'])
        parent.check(wfinal['planning']['reviewer']['calls']==1
                     and wfinal_task['independent_review']['state']=='passed','managed-W-retry-witness-final-work-state')
        refusal_request={'schema_version':1,'event_id':args.event,'expected_revision':before['revision'],'task_id':task_id}
        refusal=None
        try:
            call('W-retry-refusal-inputs',['task','review-inputs',work],refusal_request)
        except parent.Failure as error:
            owner=getattr(error,'owner',None)
            if (error.code=='parent-owned-command-failed' and owner is not None and c.runs and c.runs[-1] is owner
                    and owner.argv[1:4]==['task','review-inputs',work] and len(owner.errors)==0
                    and owner.reaped and owner.wait_validated and owner.finalized
                    and owner.status is not None and owner.status>0 and owner.pending()==[]):
                refusal=error
            else:
                raise
        parent.check(refusal is not None,'managed-W-retry-inputs-refusal-required')
        refusal_owner=refusal.owner
        refusal_index=len(c.runs)-1
        expected_nonzero[refusal_index]=refusal_owner
        refusal_stdout=refusal_owner.raw('stdout.raw')
        refusal_stderr=refusal_owner.raw('stderr.raw')
        # Actual CLI lib.rs::report writes failure JSON to stderr before any result.
        refusal_parsed=json.loads(refusal_stderr)
        failure=refusal_parsed['failure']
        parent.check(refusal_stdout==b'' and isinstance(failure,dict)
                     and (failure['code']=='native-retry-decision-required'
                          or 'native-retry-decision-required' in failure['causes']),
                     'managed-W-retry-refusal-missing-canonical-decision')
        parent.fixture.publish(c.output/'retry-refusal-stdout.raw',refusal_stdout)
        parent.fixture.publish(c.output/'retry-refusal-stderr.raw',refusal_stderr)
        parent.check(parent.equal(show('W-after-refusal'),before),'managed-W-retry-refusal-changed-Work')
        parent.ledger(root,work,settings['values'],c.output/'ledger-refusal.json')
        parent.check(parent.equal(json.loads((c.output/'ledger-refusal.json').read_bytes()),
                                  json.loads((c.output/'ledger-before.json').read_bytes())),'managed-W-retry-refusal-ledger-unchanged')
        leaf=root/'.swarm'/'biba-i0'/'reviewer'/args.event
        parent.check(not leaf.is_symlink(),'managed-W-retry-refusal-leaf-symlink')
        if leaf.is_dir():
            parent.check(not (leaf/'managed-reviewer-invoked.json').exists()
                         and not (leaf/'reviewer-invoked.json').exists(),'managed-W-retry-refusal-no-provider-marker')
        pre_step=show('W-before-retry-step')
        parent.check(parent.equal(pre_step,before),'managed-W-retry-pre-step-Work-unchanged')
        reviewer_now=pre_step['planning']['reviewer']
        parent.check(reviewer_now['calls']==1 and reviewer_now['max_calls']==2,'managed-W-retry-step-capacity')
        pending_failed=[e for e in pre_step['planning']['inbox']
                        if e['id']==args.prior_failed_review and e['task']==task_id and not e.get('decision')]
        parent.check(len(pending_failed)==1,'managed-W-retry-pending-failed-review-required')
        failed_event=pending_failed[0]
        scope=failed_event['scope']
        parent.check(isinstance(scope,str) and bool(scope),'managed-W-retry-failed-event-scope')
        scopes={s['id']:s for s in pre_step['planning']['scopes']}
        parent.check(scope in scopes and scopes[scope]['state']=='ready' and not scopes[scope].get('holder'),
                     'managed-W-retry-scope-ready-without-lease')
        planning=pre_step['planning']
        parent.check(planning['paused'] is False,'managed-W-retry-planning-paused')
        for container,used,cap in ((planning,'activations','max_activations'),
                                   (planning,'decisions','max_decisions'),
                                   (scopes[scope],'activations','max_activations')):
            parent.check(type(container[used]) is int and type(container[cap]) is int
                         and 0<=container[used]<container[cap],'managed-W-retry-planner-capacity')
        operations=[{'kind':'review-retry','id':task_id,'next':RETRY_INSTRUCTION}]
        planner_flow.proposal(SimpleNamespace(root=root),pre_step,scope,operations)
        proposal_raw=(root/'planning-proposal.json').read_bytes()
        proposal=json.loads(proposal_raw)
        parent.check(proposal.get('operations')==operations,'managed-W-retry-proposal-operations')
        selected=[e['id'] for e in pre_step['planning']['inbox'] if e['scope']==scope and not e.get('decision')]
        parent.check(proposal.get('input_events')==selected and args.prior_failed_review in selected
                     and len(selected)==len(set(selected)),'managed-W-retry-proposal-input-events')
        planner_program,retry_literal=managed_i0.frozen_program(planner_feedback.PROGRAM,proposal)
        parent.fixture.publish(c.output/'retry-proposal.json',proposal_raw)
        parent.fixture.publish(c.output/'retry-literal.json',retry_literal)
        parent.fixture.publish(c.output/'retry-program.py',planner_program.encode('utf-8'))
        parent.fixture.publish(old,planner_program.encode('utf-8'));old.chmod(0o700)
        step,step_owner=call('W-retry-planning-step',['planning','step',work],None,True)
        parent.check(isinstance(step,dict) and isinstance(step.get('run'),dict) and step['run']['decision_applied'] is True,'managed-W-retry-planning-decision-applied')
        parent.fixture.publish(c.output/'retry-step-result.json',json.dumps(step,sort_keys=True).encode())
        after_step=show('W-after-retry-step')
        before_task=next(t for t in before['tasks'] if t['id']==task_id)
        step_task=next(t for t in after_step['tasks'] if t['id']==task_id)
        parent.check(step_task['next']==RETRY_INSTRUCTION and step_task['next']!=before_task['next'],'managed-W-retry-next-corrective-instruction')
        expected_task=copy.deepcopy(before_task);expected_task['next']=RETRY_INSTRUCTION
        parent.check(parent.equal(step_task,expected_task),'managed-W-retry-only-task-instruction-changed')
        parent.check(parent.equal(step_task['independent_review'],before_task['independent_review']),'managed-W-retry-original-review-still-error')
        parent.check(parent.equal(after_step['planning']['reviewer'],before['planning']['reviewer']),'managed-W-retry-reviewer-controls-unchanged')
        after_inbox=after_step['planning']['inbox'];before_inbox=before['planning']['inbox']
        parent.check(len(after_inbox)==len(before_inbox),'managed-W-retry-inbox-length-unchanged')
        decision_ids=set()
        for b,a in zip(before_inbox,after_inbox):
            parent.check(a['id']==b['id'],'managed-W-retry-inbox-order-unchanged')
            if b['id'] in selected:
                parent.check(not b.get('decision') and bool(a.get('decision')),'managed-W-retry-canonical-decision-required')
                decision_ids.add(a['decision'])
                reverted=copy.deepcopy(a)
                if 'decision' in b:reverted['decision']=b['decision']
                else:reverted.pop('decision',None)
                parent.check(parent.equal(reverted,b),'managed-W-retry-decision-only-change')
            else:parent.check(parent.equal(a,b),'managed-W-retry-other-inbox-unchanged')
        parent.check(len(decision_ids)==1,'managed-W-retry-one-canonical-decision')
        decision_id=next(iter(decision_ids))
        counters={}
        for key in ('decisions','activations'):
            if key in before['planning'] and key in after_step['planning']:
                counters[key+'_before']=before['planning'][key];counters[key+'_after']=after_step['planning'][key]
        parent.fixture.publish(c.output/'work-after-retry-step.json',json.dumps(after_step,sort_keys=True).encode())
        parent.ledger(root,work,settings['values'],c.output/'ledger-after-step.json')
        config={'case':'pass','marker_event':args.event}
        text=reviewer.provider_program(config).decode('utf-8')
        parent.check(text.count(WRAP_NEEDLE)==1,'managed-W-retry-wrap-needle-count')
        wrapped=text.replace(WRAP_NEEDLE,WRAP_NEEDLE+'\n'+WRAP_BLOCK.rstrip('\n'))
        parent.check(wrapped.count(WRAP_NEEDLE)==1,'managed-W-retry-wrap-needle-residue')
        reviewer_program=wrapped.encode('utf-8')
        compile(wrapped,'managed-inert-reviewer','exec')
        parent.fixture.publish(c.output/'inert-reviewer.py',reviewer_program)
        parent.fixture.publish(c.output/'inert-config.json',config)
        parent.fixture.publish(old,reviewer_program);old.chmod(0o700)
        retry_request={'schema_version':1,'event_id':args.event,'expected_revision':after_step['revision'],'task_id':task_id}
        preview,_=call('W-retry-preview',['task','review-inputs',work],retry_request)
        p=preview['preview']
        parent.check(p['native_control_evidence_verified'] is True and p['invocation_context_materialized'] is True
                     and all(p[k] is False for k in ('invocation_authorized','provider_started','task_accepted')),'managed-W-readonly-inputs-required')
        parent.check(parent.equal(show('W-before-retry-review'),after_step),'managed-W-readonly-preview-mutated-Work')
        prompt=preview['prompt']
        parent.check(RECOVERY_FRAMING in prompt,'managed-W-retry-recovery-framing-present')
        body=json.loads(prompt.rsplit('Do not use tools.\n',1)[1])
        directive=body['review_recovery']
        parent.check(directive['instruction']==RETRY_INSTRUCTION and directive['review']==args.prior_failed_review
                     and directive['decision']==decision_id and directive['task']==task_id
                     and directive['attempt']==before_task['independent_review']['attempt']
                     and directive['producer']==before_task['independent_review']['producer']
                     and directive['report_sha256']==before_task['independent_review']['sha256'],
                     'managed-W-retry-recovery-original-and-decision-bound')
        parent.check(body['managed_protocol']==preview['managed_protocol'],'managed-W-retry-exact-managed-protocol')
        catalog_serialized=json.dumps(body['managed_protocol'],sort_keys=True)
        parent.check(RETRY_INSTRUCTION not in catalog_serialized and args.prior_failed_review not in catalog_serialized,
                     'managed-W-retry-recovery-outside-managed-evidence')
        catalog_check='performed'
        prior_in_directive=True
        parent.fixture.publish(c.output/'retry-recovery-directive.json',json.dumps(directive,sort_keys=True).encode())
        result,owner=call('W-retry-review',['task','review',work],retry_request,True)
        after=show('W-after-retry-review')
        parent.check(after['planning']['reviewer']['calls']==before['planning']['reviewer']['calls']+1,'managed-W-retry-exact-one-remaining-call-consumed')
        parent.check(after['planning']['reviewer']['calls']==after['planning']['reviewer']['max_calls'],'managed-W-retry-capacity-exhausted-exactly')
        after_task=next(t for t in after['tasks'] if t['id']==task_id)
        parent.check(after_task['independent_review']['id']==result['review_id']
                     and after_task['independent_review']['id']!=args.prior_failed_review
                     and after_task['independent_review']['state']=='passed','managed-W-retry-current-passed-opinion')
        parent.check(after_task['next']==RETRY_INSTRUCTION,'managed-W-retry-next-unchanged-by-review')
        retained=parent.retain_review(root,after,task_id,result,owner,c.output/'W-retry-originals',managed_protocol=preview['managed_protocol'])
        observed_path=root/'.swarm'/'biba-i0'/'reviewer'/args.event/'managed-reviewer-recovery-observed.json'
        parent.check(observed_path.is_file() and not observed_path.is_symlink(),'managed-W-retry-recovery-observed-marker-required')
        observed=json.loads(observed_path.read_bytes())
        parent.check(parent.equal(observed,directive),'managed-W-retry-recovery-observed-exact-directive')
        parent.fixture.publish(c.output/'retry-recovery-observed.json',observed_path.read_bytes())
        reconciled,_=call('W-retry-reconcile',['task','review-reconcile',result['review_id']])
        parent.check(reconciled['replayed'] is True and reconciled['review_id']==result['review_id']
                     and reconciled['verdict_applicable'] is True and reconciled['task_accepted'] is False,'managed-W-retry-public-reconciliation')
        final=show('W-final')
        parent.check(final['planning']['reviewer']['calls']==after['planning']['reviewer']['calls'],'managed-W-retry-reconcile-no-new-call')
        final_task=next(t for t in final['tasks'] if t['id']==task_id)
        parent.check(final_task['independent_review']['id']==result['review_id']
                     and final_task['independent_review']['state']=='passed','managed-W-retry-final-review-passed')
        parent.check(final_task['next']==RETRY_INSTRUCTION,'managed-W-retry-final-next-unchanged')
        parent.check(final_task['status']==before_task['status'],'managed-W-retry-no-task-acceptance')
        old_review=before_task['independent_review']
        history=final_task['previous_reviews']
        parent.check(bool(history) and parent.equal(history[-1],old_review),'managed-W-retry-immediate-previous-review-unchanged')
        parent.check(parent.equal(final_task['automatic_validation'],before_task['automatic_validation'])
                     and parent.equal(final_task['attempts'],before_task['attempts']),
                     'managed-W-retry-no-control-or-worker-replay')
        records=[('tasks/'+task_id+'/previous_reviews/-1',history[-1])]
        parent.fixture.publish(c.output/'retry-previous-review-locations.json',json.dumps([path for path,_ in records]).encode())
        parent.fixture.publish(c.output/'retry-original-failed-review.json',json.dumps(old_review,sort_keys=True).encode())
        parent.ledger(root,work,settings['values'],c.output/'ledger-after.json');c.stable()
        parent.check(len(expected_nonzero)==1,'managed-W-retry-one-expected-refusal')
        for index,r in enumerate(c.runs):
            if index in expected_nonzero:
                parent.check(c.runs[index] is expected_nonzero[index],'managed-W-retry-expected-run-identity')
                parent.check(not r.pending() and r.reaped and r.wait_validated and r.finalized
                             and r.status is not None and r.status>0,'managed-W-retry-expected-nonzero-run-settled')
            else:
                parent.check(not r.pending() and r.reaped and r.status==0,'managed-W-owned-processes-settled')
        parent.fixture.publish(c.output/'work-final.json',json.dumps(final,sort_keys=True).encode())
        originals=c.output/'W-retry-originals'
        proofs={'helper-inputs.json':c.output/'helper-inputs.json','engine-settings.json':c.output/'engine-settings.json',
                'work-original.json':c.output/'work-original.json','work-final.json':c.output/'work-final.json',
                'work-after-retry-step.json':c.output/'work-after-retry-step.json',
                'ledger-before.json':c.output/'ledger-before.json','ledger-refusal.json':c.output/'ledger-refusal.json',
                'ledger-after-step.json':c.output/'ledger-after-step.json','ledger-after.json':c.output/'ledger-after.json',
                'inert-reviewer.py':c.output/'inert-reviewer.py','inert-config.json':c.output/'inert-config.json',
                'worker-before.py':c.output/'worker-before.py','retry-proposal.json':c.output/'retry-proposal.json',
                'retry-literal.json':c.output/'retry-literal.json','retry-program.py':c.output/'retry-program.py',
                'retry-step-result.json':c.output/'retry-step-result.json','retry-refusal-stdout.raw':c.output/'retry-refusal-stdout.raw',
                'retry-refusal-stderr.raw':c.output/'retry-refusal-stderr.raw',
                'retry-recovery-directive.json':c.output/'retry-recovery-directive.json',
                'retry-recovery-observed.json':c.output/'retry-recovery-observed.json',
                'retry-previous-review-locations.json':c.output/'retry-previous-review-locations.json',
                'retry-original-failed-review.json':c.output/'retry-original-failed-review.json',
                'retention.json':originals/'retention.json','original.frame':originals/'original.frame',
                'certificate.json':originals/'certificate.json','invocation.json':originals/'invocation.json',
                'inert-provider.py':originals/'inert-provider.py','managed-context.json':originals/'managed-context.json'}
        receipt={'status':RETRY_STATUS,'receipt_schema':RETRY_SCHEMA,'mode':'retry',
                 'root':str(root),'work':work,'task':task_id,'prior_failed_review':args.prior_failed_review,
                 'witness':{'path':str(args.witness_receipt),'sha256':args.witness_receipt_sha256,'status':witness['status']},
                 'corrective_next':RETRY_INSTRUCTION,'review_id':result['review_id'],
                 'current_review':after_task['independent_review'],'previous_review':old_review,
                 'previous_review_locations':[path for path,_ in records],
                 'reviewer_calls_before':before['planning']['reviewer']['calls'],
                 'reviewer_calls_after':final['planning']['reviewer']['calls'],
                 'replaced_provider_program_sha256':parent.fixture.digest((c.output/'worker-before.py').read_bytes()),
                 'refusal':{'label':'W-retry-refusal-inputs','returncode':refusal_owner.status,
                            'stdout_sha256':parent.fixture.digest(refusal_stdout),
                            'stderr_sha256':parent.fixture.digest(refusal_stderr),
                            'stderr_object_keys':sorted(refusal_parsed.keys()),
                            'reason_check':'native-retry-decision-required from actual Root canonical observer'},
                 'planning_counters':counters,
                 'recovery_directive_references_prior_review':prior_in_directive,
                 'catalogue_exclusion':catalog_check,'provider_program_wrapped':True,
                 'normal_sha256':parent.fixture.digest(args.normal.read_bytes()),
                 'normal_profile_sha256':c.normal_profile_sha,'helper_inputs':dict(c.helper_inputs),
                 'semantic_preconditions':current_sem,
                 'retention':retained,'task_accepted':False,'global_watchdog':None,
                 'process_waits':[r.summary() for r in c.runs],
                 'expected_nonzero_runs':[{'index':refusal_index,'label':'W-retry-refusal-inputs',
                                           'returncode':refusal_owner.status,'summary':refusal_owner.summary()}],
                 'output_proofs':{name:pin(path) for name,path in sorted(proofs.items())},
                 'remaining_runtime_evidence':REMAINING_EVIDENCE,'scope':SCOPE_RETRY}
        parent.fixture.publish(c.output/'W-result.json',receipt);print(json.dumps(receipt,sort_keys=True));return 0
    except BaseException as error:
        observer=getattr(c,'failure_observations',None) if c is not None and 'settings' in c.fixtures.get('W',{}) else None
        return parent.report_terminal(error,c.output if c is not None else args.output,observer)


if __name__=='__main__':sys.exit(main())
