#!/usr/bin/env python3
"""Replay automatic worker provenance on an owned publicly planned fixture.

Consumes qualified normal and SDK binaries; never builds, calls a paid model,
seeds positive SQL authority, installs or accepts work. Requires an actual
heartbeat-maintained cycle around the worker and retained unknown-cost counts.
No missing declaration is presented as free execution. No global watchdog.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import traceback

import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as planning


def main():
    """Bind executables, create two real tasks, and assert private-gate effects.

    Also read worker costs through the normal CLI: the inherited inert program
    declares 25 simulated USD, plus 25 USD of historical-data fixture costs;
    cancelled intentions remain without cost. These rows grant no permission.
    This partial projection is neither free execution nor an authenticated invoice.
    Requires fresh output and matching qualified normal/SDK profiles. Preserves
    channels, fixture and failure context on any error. A PASS covers the owned
    inert process and admission/cancellation scenarios, not a full conductor,
    provider authenticity, installation or independent acceptance.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('binary', 'profile', 'sdk-binary', 'sdk-profile', 'output'):
        parser.add_argument('--' + key, required=True, type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    root = None
    fixture.publish(out / 'method.py', Path(__file__).read_bytes())
    try:
        binary, sdk = args.binary.resolve(), args.sdk_binary.resolve()
        normal = json.loads(args.profile.read_bytes())
        profile = json.loads(args.sdk_profile.read_bytes())
        assert normal['no_fixture_feature'] and normal['sha256'] == fixture.digest(binary.read_bytes())
        assert profile['engine_features_empty'] and profile['shared_workspace_features_match_normal']
        assert profile['test_profile'] is False
        assert profile['binary_sha256'] == fixture.digest(sdk.read_bytes())
        assert profile['normal_profile_sha256'] == normal['sha256']
        fixture.publish(out / 'profiles.json', dict(normal=normal, sdk=profile))
        ready = planning.flow(binary, out / 'source', before_worker=True,
                              extra_worker=True, child_task_budget=2)
        assert ready['status'] == 'READY_FOR_NATIVE_WORKER' and not ready['worker_started']
        root = Path(ready['root'])
        request = dict(root=str(root), work=ready['work_id'])
        fixture.publish(root / '.swarm/verification-mission-worker.json', request)
        fixture.publish(out / 'request.json', request)
        command = [str(sdk), str(out / 'request.json')]
        fixture.publish(out / 'command.json', dict(argv=command, global_watchdog=None,
                        explicit_fixture_cache={'CODEX_HOME': str(root / 'codex-home')}))
        run = subprocess.run(command, env={**os.environ, 'CODEX_HOME': str(root / 'codex-home')},
                             stdin=subprocess.DEVNULL, capture_output=True)
        fixture.publish(out / 'stdout', run.stdout)
        fixture.publish(out / 'stderr', run.stderr)
        fixture.publish(out / 'exit.json', dict(returncode=run.returncode))
        assert run.returncode == 0, run.stderr.decode(errors='replace')
        result = json.loads(run.stdout)
        assert result['status'] == 'PASS' and result['task_accepted'] is False
        for name in ('raw_conductor_fields_refused', 'paused_manual_and_revoked_consent_refused',
                     'saved_capacity_enforced', 'old_intent_not_rearmed', 'stale_lease_refused',
                     'queued_old_intent_cancelled', 'actual_worker_and_private_gate',
                     'automatic_origin_retained', 'handoff_submitted', 'profile_preserved',
                     'heartbeat_cycle_used', 'reported_costs_unknown_retained', 'reported_cost_policy_refusal_recovery', 'reported_cost_pregrant_rechecked'):
            assert result[name] is True, name
        cost_command = [str(binary), 'costs', 'show', ready['work_id'], '--root', str(root), '--json']
        cost_run = subprocess.run(cost_command, stdin=subprocess.DEVNULL, capture_output=True)
        fixture.publish(out / 'costs-command.json', dict(argv=cost_command, global_watchdog=None))
        fixture.publish(out / 'costs.stdout', cost_run.stdout)
        fixture.publish(out / 'costs.stderr', cost_run.stderr)
        assert cost_run.returncode == 0
        costs = json.loads(cost_run.stdout)
        assert costs['scope'] == 'retained-worker-attempts'
        assert costs['reported_usd'] == 50 and costs['attempts_with_cost'] == 2
        assert costs['attempts_without_cost'] > 0
        assert costs['by_task']['task-2']['reported_usd'] == 25
        assert costs['by_task']['task-2']['attempts_with_cost'] == 1
        result['reported_costs_public_cli_partial_retained'] = True
        observed = json.loads((root / 'second/model-received.json').read_bytes())
        report = (root / 'second/handoff.md').read_bytes()
        assert observed['stdin_sha256'].encode() in report
        assert b'not authenticated model or task acceptance' in report
        assert not (root / 'model-received.json').exists()
        result.update(root=str(root), normal_binary_sha256=normal['sha256'],
                      sdk_binary_sha256=profile['binary_sha256'],
                      report_sha256=fixture.digest(report), global_watchdog=None, installed=False)
        fixture.publish(out / 'result.json', result)
        print('PASS automatic worker provenance, revocation, cancellation and actual handoff', flush=True)
        print('PASS reported-cost automatic refusal, equality, saved policy changes and recovery',flush=True)
        print('PASS reported worker costs, partial declarations and normal CLI', flush=True)
    except BaseException:
        fixture.publish(out / 'failure.json', dict(status='FAIL', root=str(root) if root else None,
                                                 cause=traceback.format_exc()))
        raise


if __name__ == '__main__':
    main()
