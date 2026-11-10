#!/usr/bin/env python3
"""Replay held planner, reviewer and controls with actual SIGINT and SIGTERM.

Every probe owns a fresh Root, validates a healthy run before injecting its signal,
waits for its actual watch/role cleanup and checks the durable failed receipt.
An optional progress witness requires two completed main waits while each role
is held. No live provider, automatic retry or global deadline is used. Preserved evidence
and an exact native profile are required; this is not full asynchronous parity.
"""
import argparse
import json
import subprocess
import sys
from pathlib import Path

import rust_agent_model_route as fixture


def run(binary, profile, output, require_progress=False):
    """Run six fixed protocol cases, retaining argv, output and meaningful results.

    Inputs are qualified immutable binary/profile paths and a fresh evidence root.
    A failing prerequisite stops the matrix, preserves the first failure and all
    earlier cases, and propagates the error; the probe owns process cleanup.
    This driver neither adopts workers nor removes evidence or resets funding.
    """
    output.mkdir(exist_ok=False)
    cases = []
    for role in ('planner', 'reviewer', 'controls'):
        for stop in ('sigint', 'sigterm'):
            name = role + '-' + stop
            target = output / name
            argv = [sys.executable, str(Path(__file__).with_name('rust_mission_watch_signal_probe.py').resolve()),
                    '--binary', str(binary), '--profile', str(profile), '--output', str(target),
                    '--role', role, '--signal', stop]
            if require_progress:
                argv.append('--require-conductor-progress')
            fixture.publish(output / (name + '.argv.json'), dict(argv=argv, global_watchdog=None))
            print('START ' + name, flush=True)
            with (output / (name + '.stdout')).open('xb') as stdout, (output / (name + '.stderr')).open('xb') as stderr:
                completed = subprocess.run(argv, stdout=stdout, stderr=stderr)
            fixture.publish(output / (name + '.exit.json'), dict(exit=completed.returncode))
            completed.check_returncode()
            result = json.loads((target / 'result.json').read_bytes())
            assert result['status'] == 'PASS' and result['role'] == role and result['requested_signal'] == stop
            assert result['expected_deferred'] is False and result['installed'] is False
            assert result['positive']['healthy_role_recorded'] and result['positive']['watch_exit'] == 0
            assert result['signal']['watch_exited_while_provider_held'] and result['signal']['watch_exit'] == 0
            assert result['signal']['failed_call_and_TERM_receipt_verified']
            if require_progress:
                assert result['signal']['two_main_timed_waits_after_granted_entry']
                print('PASS ' + name + ' conductor-progress-while-provider-held', flush=True)
            cases.append(dict(role=role, signal=stop, status='PASS', evidence=str(target)))
            print('PASS ' + name + ' healthy-before-signal and durable-failure', flush=True)
    fixture.publish(output / 'result.json', dict(status='PASS', cases=cases, count=len(cases),
                    installed=False, global_watchdog=None, full_go_async_parity=False))
    print('FOREGROUND_SIGNAL_MATRIX_PASS 6 cases', flush=True)


def main():
    """Require explicit immutable inputs and a previously absent evidence path."""
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ('binary', 'profile', 'output'):
        parser.add_argument('--' + flag, type=Path, required=True)
    parser.add_argument('--require-conductor-progress', action='store_true')
    args = parser.parse_args()
    run(args.binary.resolve(strict=True), args.profile.resolve(strict=True), args.output.resolve(), args.require_conductor_progress)


if __name__ == '__main__':
    main()
