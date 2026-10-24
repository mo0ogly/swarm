#!/usr/bin/env python3
"""Real root cycle decisions preserve history, consumption and saved policy.

Owned inert model only. No positive SQL seed, live mission, paid call, global
watchdog, autonomous conductor or judge-end authorization. A separate successful
control precedes faults; both keep original frames, prompts and public results.
"""
import argparse
import json
import os
import traceback
from pathlib import Path
import rust_agent_model_route as f
import rust_planning_worker_public_flow as flow
from rust_planning_activation import apply_policy, claim_request
from rust_planning_bootstrap import fresh_work, request
from rust_planning_decision import decision
from rust_native_review_decision import rows
import rust_native_review_acceptance as review


def main():
    """Exercise native continue/renew, then operator refusals and public pause/reclaim recovery.

    Initial eight activations cover four valid model calls, one intentional native
    end refusal and three operator claims around legally paused quota changes in
    the fault variant. These are explicit fixture inputs, never a retry cap/reset. Saved6000byte
    admission accepts5000UTF8bytes; lowering policy to200 leaves history readable.
    Cycle cap3 becomes2 only for a public quota refusal and is restored to3.
    Returned verdicts do not accept the objective or authenticate a human/model.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['binary', 'profile', 'output']:
        parser.add_argument('--'+name, required=True, type=Path)
    parser.add_argument('--faults', action='store_true')
    parser.add_argument('--control', type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    owned = None
    f.publish(out/'method.py', Path(__file__).read_bytes())
    try:
        binary = args.binary.resolve()
        profile = json.loads(args.profile.read_bytes())
        assert profile['no_fixture_feature'] and profile['sha256'] == f.digest(binary.read_bytes())
        control = None
        if args.faults:
            assert args.control is not None
            control = json.loads(args.control.read_bytes())
            assert control['status'] == 'PASS' and not control['faults'] and control['binary_sha256'] == profile['sha256']
        owned = f.Fixture(binary, out/'source', 'global-judge')
        owned.work = fresh_work(owned, 'judge-create')
        apply_policy(owned, 'judge-reason-policy', dict(planning_decision_max_reason_bytes='6000'))
        enabled = owned.ok(['planning', 'enable', owned.work], request(event='judge-enable', max_activations=8, max_decisions=6, max_judge_cycles=3))
        f.publish(owned.root/'codex', flow.feedback.PROGRAM.encode())
        (owned.root/'codex').chmod(0o700)
        environment = {**os.environ, 'CODEX_HOME': str(owned.root/'codex-home')}

        def step(label, work, operations, reason='Observe the current root cycle without accepting the mission', refused=None):
            """Archive the actual call before asserting its outcome and exact received context.

            Native refusals still consume their real claim/call; re-read Work,
            preserve spending and never repeat with a stale revision or reset.
            """
            flow.proposal(owned, work, 'root', operations)
            proposal = json.loads((owned.root/'planning-proposal.json').read_bytes())
            proposal['reason'] = reason
            f.publish(owned.root/'planning-proposal.json', proposal)
            code, value = review.public.native.invoke(binary, owned.root, out, label, ['planning', 'step', owned.work], environment=environment)
            jobs = rows(owned.root, 'SELECT claim_id,phase FROM native_planning_runs WHERE work_id=? AND scope_id=? ORDER BY prepared_revision,claim_id', (owned.work, 'root'))
            claim = jobs[-1][0]
            raw = (owned.root/'.swarm'/('native-planning-'+f.digest(claim.encode())+'.receipt')).read_bytes()
            magic, extent, sha, body = raw.split(b'\n', 3)
            assert magic == b'SR1' and int(extent) == len(body) and f.digest(body) == sha.decode()
            f.publish(out/(label+'-original.frame'), raw)
            f.publish(out/(label+'-observation.json'), dict(root=str(owned.root), work=owned.work, exit=code, failure=value.get('failure'), rows=jobs))
            original = json.loads(body)
            pid = original['run']['child_pid']
            prompt = (owned.root/('fixture-prompt-'+str(pid)+'.bin')).read_bytes()
            witness = json.loads((owned.root/('model-received-'+str(pid)+'.json')).read_bytes())
            context, digest = rows(owned.root, 'SELECT context_json,context_sha256 FROM native_planning_claims WHERE claim_id=? AND work_id=?', (claim, owned.work))[0]
            received = json.loads(context)
            assert f.digest(context.encode()) == digest and context.encode() in prompt
            assert witness['prompt_sha256'] == f.digest(prompt) == original['run']['prompt_sha256']
            assert witness['pid'] == pid and witness['ppid'] == original['run']['parent_pid']
            assert received['judge'] == work['planning']['judge'] and received['judge_end_available'] is False
            assert b'judge (racine seule' in prompt and b'renew (racine seule' in prompt
            f.publish(out/(label+'-prompt.bin'), prompt)
            f.publish(out/(label+'-witness.json'), witness)
            f.publish(out/(label+'-context.json'), received)
            if refused:
                assert code != 0 and refused in value['failure']['causes'], value
                current = owned.ok(['work', 'show', owned.work])['work']
                assert current['planning']['judge'] == work['planning']['judge'] and not current.get('tasks')
                return current
            assert code == 0 and value['run']['decision_applied'], value
            return value['run']['work']

        current = step('healthy-ack', enabled, [])
        assert current['planning']['judge'] == enabled['planning']['judge'] and current['planning']['activations'] == 1
        current = owned.ok(['planning', 'resume', owned.work], dict(schema_version=1, event_id='judge-fresh-input', expected_revision=current['revision'], scope='root', reason='Observe the original objective and current judge'))
        if args.faults:
            current = step('end-refused', current, [dict(kind='judge', judge_cycle=1, judge_verdict='end', expected_candidate='unattested-candidate')], refused='planning-judge-end-managed-evidence-unported')
            current = owned.ok(['planning', 'resume', owned.work], dict(schema_version=1, event_id='judge-after-end-refusal', expected_revision=current['revision'], scope='root', reason='Preserve failed call and resume the still-open objective'))
        long_reason = 'é'*2500
        current = step('continue-first', current, [dict(kind='judge', judge_cycle=1, judge_verdict='continue')], long_reason)
        judge = current['planning']['judge']
        assert judge['cycle'] == 1 and judge['verdict'] == 'continue' and judge['reason'] == long_reason
        first = dict(judge['history'][0])
        current = step('renew-first', current, [dict(kind='renew', judge_cycle=1)])
        judge = current['planning']['judge']
        assert judge['cycle'] == 2 and not judge.get('verdict') and len(judge['history']) == 1
        historical = dict(judge['history'][0])
        assert historical.pop('renewed_at') and historical == first
        current = step('continue-second', current, [dict(kind='judge', judge_cycle=2, judge_verdict='continue')])
        assert current['planning']['activations'] == (5 if args.faults else 4) and current['planning']['decisions'] == 4 and not current.get('tasks')
        claim = rows(owned.root, 'SELECT claim_id FROM native_planning_runs WHERE work_id=? ORDER BY prepared_revision DESC', (owned.work,))[0][0]
        money = review.public.finance(owned.root, owned.work)
        prompts = {str(p): f.digest(p.read_bytes()) for p in owned.root.glob('fixture-prompt-*.bin')}
        replay = owned.ok(['planning', 'reconcile', claim])
        assert replay['replayed'] and replay['decision_applied'] and owned.ok(['work', 'show', owned.work])['work'] == current
        assert review.public.finance(owned.root, owned.work) == money and prompts == {str(p): f.digest(p.read_bytes()) for p in owned.root.glob('fixture-prompt-*.bin')}
        apply_policy(owned, 'judge-reason-lower', dict(planning_decision_max_reason_bytes='200'))
        assert owned.ok(['work', 'show', owned.work])['work'] == current
        signature = dict(max_activations=current['planning']['max_activations'], settings=f.digest(json.dumps(owned.ok(['engine-settings', 'show'])['values'], sort_keys=True).encode()), cycle=current['planning']['judge']['cycle'], max_cycles=3, verdict='continue', history_reasons=[f.digest(h['reason'].encode()) for h in current['planning']['judge']['history']])
        if control:
            assert control['signature'] == signature
        scope = next(s for s in current['planning']['scopes'] if s['id'] == 'root')
        current = owned.ok(['planning', 'claim', owned.work], claim_request(current, 'judge-operator-claim', scope_revision=scope['revision']))
        renew = dict(kind='renew', judge_cycle=2)

        def reject(label, operations, expected, reason='Owned public decision witness'):
            """Require the targeted refusal with exact state, finance and no new process."""
            before = owned.ok(['work', 'show', owned.work])['work']
            before_money = review.public.finance(owned.root, owned.work)
            value = decision(before, label, operations)
            value['reason'] = reason
            code, refused = owned.call(['planning', 'decide', owned.work], value)
            assert code != 0 and expected in refused['failure']['causes'], refused
            assert owned.ok(['work', 'show', owned.work])['work'] == before and review.public.finance(owned.root, owned.work) == before_money
            assert prompts == {str(p): f.digest(p.read_bytes()) for p in owned.root.glob('fixture-prompt-*.bin')}

        if args.faults:
            reject('reason-refused', [renew], 'planning-decision-reason-limit', long_reason)
            reject('cycle-refused', [dict(kind='renew', judge_cycle=1)], 'planning-judge-stale-cycle')
            reject('field-refused', [dict(renew, title='Unrelated plan field')], 'planning-judge-field-not-applicable')
            reject('mixed-refused', [renew, dict(kind='task', id='forbidden-worker', title='No worker', requirements=['req-1'])], 'planning-judge-exclusive-operation')
            quota = owned.ok(['quotas', 'show', owned.work])
            old_limits = dict(quota['limits'])
            limited = dict(old_limits, judge_cycles=2)
            before_owner = owned.ok(['work', 'show', owned.work])['work']
            owner_money = review.public.finance(owned.root, owned.work)
            change = dict(schema_version=1, event_id='judge-limit-active-owner', expected_revision=quota['revision'], limits=limited, reason='Observe active-owner quota protection')
            code, refused = owned.call(['quotas', 'apply', owned.work], change)
            assert code != 0 and 'quota-planner-owner-active' in refused['failure']['causes'], refused
            assert owned.ok(['work', 'show', owned.work])['work'] == before_owner and review.public.finance(owned.root, owned.work) == owner_money
            owned.ok(['planning', 'pause', owned.work], dict(schema_version=1, event_id='judge-before-low-pause', expected_revision=before_owner['revision'], reason='Release only our fixture operator activation before quota change'))
            quota = owned.ok(['quotas', 'show', owned.work])
            owned.ok(['quotas', 'apply', owned.work], dict(schema_version=1, event_id='judge-limit-low', expected_revision=quota['revision'], limits=limited, reason='Exercise saved current cycle ceiling'))
            current = owned.ok(['work', 'show', owned.work])['work']
            current = owned.ok(['planning', 'resume', owned.work], dict(schema_version=1, event_id='judge-after-low-resume', expected_revision=current['revision'], reason='Resume unchanged history under the saved low quota'))
            scope = next(s for s in current['planning']['scopes'] if s['id'] == 'root')
            current = owned.ok(['planning', 'claim', owned.work], claim_request(current, 'judge-low-operator-claim', scope_revision=scope['revision']))
            reject('quota-refused', [renew], 'planning-judge-cycle-budget-exhausted')
            owned.ok(['planning', 'pause', owned.work], dict(schema_version=1, event_id='judge-before-restore-pause', expected_revision=current['revision'], reason='Release only our fixture claim before restoring original quota'))
            quota = owned.ok(['quotas', 'show', owned.work])
            owned.ok(['quotas', 'apply', owned.work], dict(schema_version=1, event_id='judge-limit-restore', expected_revision=quota['revision'], limits=old_limits, reason='Restore original cycle policy, preserve consumed cycles'))
            current = owned.ok(['work', 'show', owned.work])['work']
            current = owned.ok(['planning', 'resume', owned.work], dict(schema_version=1, event_id='judge-after-restore-resume', expected_revision=current['revision'], reason='Continue with original quota and preserved usage'))
            scope = next(s for s in current['planning']['scopes'] if s['id'] == 'root')
            current = owned.ok(['planning', 'claim', owned.work], claim_request(current, 'judge-restored-operator-claim', scope_revision=scope['revision']))
        before = owned.ok(['work', 'show', owned.work])['work']
        before_money = review.public.finance(owned.root, owned.work)
        current = owned.ok(['planning', 'decide', owned.work], decision(before, 'judge-operator-renew', [renew]))
        judge = current['planning']['judge']
        assert judge['cycle'] == 3 and not judge.get('verdict') and judge['max_cycles'] == 3 and len(judge['history']) == 2
        assert judge['history'][0]['reason'] == long_reason and current['planning']['activations'] == before['planning']['activations'] and current['planning']['decisions'] == 5
        assert review.public.finance(owned.root, owned.work) == before_money and prompts == {str(p): f.digest(p.read_bytes()) for p in owned.root.glob('fixture-prompt-*.bin')}
        assert owned.ok(['work', 'show', owned.work])['work'] == current
        f.publish(out/'result.json', dict(status='PASS', root=str(owned.root), work=owned.work, binary_sha256=profile['sha256'], faults=args.faults, signature=signature, actual_native_judge=True, actual_context_received=True, long_reason_bytes=5000, historical_reason_readable_under_lower_cap=True, renewal=True, replayed_without_exec=True, operator_recovery_public_pause=bool(args.faults), public_refusals=7 if args.faults else 0, judge_end=False, mission_accepted=False, authenticated_actor=False, global_watchdog=None))
        print('PASS native global judge cycles, persisted policy and preserved history', flush=True)
    except BaseException:
        f.publish(out/'failure.json', dict(status='FAIL', root=str(owned.root) if owned else None, cause=traceback.format_exc()))
        raise


if __name__ == '__main__':
    main()
