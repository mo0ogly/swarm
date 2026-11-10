#!/usr/bin/env python3
"""Replay lease-owned FIFO through two publicly planned Works in one owned Root.

Consumes qualified normal/SDK binaries, never builds or installs them. Public
planning creates tasks, scoped budgets and decisions; no positive SQL seed or
paid provider is used. The SDK owns its explicit late-journal negative injection
and cleanup. Its inert completion is neither task acceptance nor model identity.
All channels and failures survive, without a global watchdog or automatic retry.
"""
import argparse
import copy
import json
from pathlib import Path
import subprocess
import sys
import time
import traceback

import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as planning


class ReusedFixture(fixture.Fixture):
    """Reuse an actual Fixture context, retaining its publicly initialized Root.

    This adapter never reinitializes or fabricates a Root, profile, budget or
    claim. A private copied command context uses a separate evidence directory
    and prefixes public event IDs so the second Work cannot replay first-Work
    events. Restoring the factory belongs to the caller's finally block.
    """
    original = None

    def __init__(self, binary, output, label):
        """Copy real context and create only evidence; refuse mismatched binary.

        Keep original environment/cache/provider paths, but independent mutable
        dictionaries and command history. Errors propagate before public writes;
        copied fixture metadata grants no permission or authenticated identity.
        """
        if self.original is None or Path(binary).resolve() != self.original.binary.resolve():
            raise ValueError('a matching actual Fixture is required')
        self.__dict__.update(copy.deepcopy(self.original.__dict__))
        self.output = Path(output)
        self.output.mkdir(parents=True, exist_ok=False)
        self.label = 'fifo-second-' + label
        self.calls = []
        fixture.publish(self.output / 'reused-context.json', dict(
            root=str(self.root), original_output=str(self.original.output),
            public_event_prefix='fifo-second-', initialized_again=False,
            positive_sql_seed=False, runtime_permission=False))

    def call(self, args, value=None):
        """Namespace supplied public event IDs, preserving every other input.

        The engine still chooses Work/claim/attempt identities and validates all
        inputs. Copying avoids mutating an upstream request retained as evidence.
        Non-event reads and generated public operations keep their actual argv.
        """
        if value is not None:
            value = copy.deepcopy(value)
            if 'event_id' in value:
                value['event_id'] = 'fifo-second-' + value['event_id']
        return super().call(args, value)


def planned_pair(binary, out):
    """Create two real scoped plans in one datastore through existing public flow.

    First Work explicitly budgets two tasks; the second budgets one. Both return
    before worker admission. The temporary module factory override is process
    local and always restored, including planner failure. No concurrent threads
    use this adapter. Original command/evidence contexts remain separately owned.
    """
    first, owned = planning.flow(binary, out / 'first', before_worker=True,
                                extra_worker=True, child_task_budget=2,
                                return_fixture=True)
    original = fixture.Fixture
    ReusedFixture.original = owned
    fixture.Fixture = ReusedFixture
    try:
        second, reused = planning.flow(binary, out / 'second', before_worker=True,
                                      return_fixture=True)
    finally:
        fixture.Fixture = original
        ReusedFixture.original = None
    assert first['status'] == second['status'] == 'READY_FOR_NATIVE_WORKER'
    assert not first['worker_started'] and not second['worker_started']
    assert first['root'] == second['root'] and first['work_id'] != second['work_id']
    return first, second, owned, reused


def main():
    """Verify exact profile bindings, SDK assertions and persisted public queue.

    Outputs require a fresh directory. Keep SDK input/marker/result, full channels,
    timestamps and fixture attribution before asserting success. Reopen both Works
    through the qualified normal CLI after SDK-owned lease/Root close and require
    the same durable queue with a waiting first-Work sibling and released worker
    turns. Failure preserves all prior evidence; no test deletes fixtures, resets
    consumption, forces acceptance, retries or establishes installed delivery.
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
        assert normal['no_fixture_feature'] is True
        assert normal['sha256'] == fixture.digest(binary.read_bytes())
        assert profile['engine_features_empty'] is True
        assert profile['shared_workspace_features_match_normal'] is True
        assert profile['test_profile'] is False
        assert profile['binary_sha256'] == fixture.digest(sdk.read_bytes())
        assert profile['normal_profile_sha256'] == normal['sha256']
        fixture.publish(out / 'profiles.json', dict(normal=normal, sdk=profile))
        sources = {str(Path(module.__file__).resolve()): fixture.digest(Path(module.__file__).read_bytes())
                   for name, module in tuple(sys.modules.items())
                   if name.startswith('rust_') and getattr(module, '__file__', None)}
        sources[str(Path(__file__).resolve())] = fixture.digest(Path(__file__).read_bytes())
        fixture.publish(out / 'inputs.json', dict(sources=sources,
                        binary_sha256=normal['sha256'], sdk_binary_sha256=profile['binary_sha256']))
        first, second, owned, reused = planned_pair(binary, out / 'planning')
        root = Path(first['root']).resolve()
        fixture.publish(out / 'ready.json', dict(first=first, second=second))
        for name in ('fifo-shared', 'fifo-disjoint'):
            (root / name).mkdir(exist_ok=False)
        request = dict(kind='owned-inert-fifo-fixture', root=str(root),
                       first=first['work_id'], second=second['work_id'],
                       result=str(out / 'sdk-result.json'))
        fixture.publish(root / '.swarm/verification-fifo.json', request)
        fixture.publish(out / 'request.json', request)
        command = [str(sdk), str(out / 'request.json')]
        fixture.publish(out / 'command.json', dict(argv=command, global_watchdog=None,
                        fixture_cache=str(owned.cache_root)))
        started = time.time()
        run = subprocess.run(command, env=owned.environment, stdin=subprocess.DEVNULL,
                             capture_output=True)
        fixture.publish(out / 'stdout', run.stdout)
        fixture.publish(out / 'stderr', run.stderr)
        fixture.publish(out / 'exit.json', dict(returncode=run.returncode,
                        started=started, finished=time.time()))
        assert run.returncode == 0, 'FIFO SDK failed; retained stdout/stderr'
        lines = run.stdout.decode('utf-8').splitlines()
        names = (
            'PASS actual concurrent Root sync and FIFO denial before any Agent exists',
            'PASS late journal write failure rollback and same-request recovery with Binding v3',
            'PASS public cancellation/replay, requeue history, disjoint worker completion and atomic release',
            'PASS paused mission releases waiting turns and preserves acquired ownership',
            'PASS late cleanup/release faults and overlapping progress',
            'PASS actual lease-owned workspace FIFO lifecycle',
        )
        assert all(lines.count(name) == 1 for name in names), 'missing/duplicate named SDK assertions'
        result = json.loads((out / 'sdk-result.json').read_bytes())
        assert result['status'] == 'PASS' and result['binding_schema'] == 3
        for name in ('two_real_leases', 'two_connection_sync',
                     'safe_cancel_and_completion_release', 'journal_fault_after_root_open',
                     'paused_cleanup_preserves_acquired',
                     'late_cleanup_release_faults_and_overlapping_progress'):
            assert result[name] is True, name
        assert result['task_accepted'] is False and result['provider'] == 'owned inert executable'
        a = owned.ok(['mission', 'queue', first['work_id']])
        b = reused.ok(['mission', 'queue', second['work_id']])
        assert a['work'] == first['work_id'] and b['work'] == second['work_id']
        assert a['turns'] == b['turns'] and not b['blockers']
        turns = {(turn['work'], turn['task']): turn for turn in b['turns']}
        assert turns[first['work_id'], 'task']['state'] == 'released'
        assert turns[first['work_id'], 'task']['agent'] == 'fifo-first-agent'
        assert turns[first['work_id'], 'task-2']['state'] == 'waiting'
        assert turns[second['work_id'], 'task']['state'] == 'released'
        assert turns[second['work_id'], 'task']['agent'] == 'fifo-second-agent'
        assert (root / 'fifo-disjoint/model-received.json').is_file()
        assert not (root / 'fifo-shared/model-received.json').exists()
        assert fixture.digest(binary.read_bytes()) == normal['sha256']
        assert fixture.digest(sdk.read_bytes()) == profile['binary_sha256']
        assert all(fixture.digest(Path(path).read_bytes()) == sha for path, sha in sources.items())
        result.update(root=str(root), first=first['work_id'], second=second['work_id'],
                      normal_binary_sha256=normal['sha256'], sdk_binary_sha256=profile['binary_sha256'],
                      reopened_queue=True, global_watchdog=None, installed=False,
                      paid_provider=False, positive_sql_seed=False, independent_acceptance=False)
        fixture.publish(out / 'result.json', result)
        print('PASS two publicly planned Works share one owned datastore', flush=True)
        print('PASS named SDK FIFO assertions and persisted CLI queue after reopen', flush=True)
        print('REPLAY_EVIDENCE=' + str(out), flush=True)
    except BaseException:
        fixture.publish(out / 'failure.json', dict(status='FAIL', root=str(root) if root else None,
                        cause=traceback.format_exc()))
        raise


if __name__ == '__main__':
    main()
