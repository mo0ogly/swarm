#!/usr/bin/env python3
"""Check that the maintained contention assertion detects an early transaction release.

Runs the real named T05/T17 test with an explicit test-helper substitution, in
fresh isolated roots. An expected failing run is retained as a counterexample,
never exported as a passing campaign. No provider, live Root, build or watchdog.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]


class ContentionGuard(unittest.TestCase):
    def test_early_release_is_detected_and_workers_are_reaped(self):
        """Require four specific assertion failures after valid controls, not any error.

        Keep the negative runner's original stdout/stderr, argv, declared override,
        call index and actual child exit states. Check completed peers and aborted
        holders, exposing cleanup failures. Existing paths refuse before launch;
        correct prerequisites and choose a new evidence root for another attempt.
        """
        parent = Path(os.environ['SWARM_PAEA_PILOT_EVIDENCE']).resolve()
        output = parent.with_name(parent.name + '-early-release')
        if output.exists():
            raise ValueError('laboratory-negative-output-must-be-fresh')
        env = os.environ.copy()
        env.update(SWARM_PAEA_PILOT_EVIDENCE=str(output), SWARM_PAEA_CONTENTION_FAULT='early-release')
        command = [sys.executable, str(ROOT / 'tests/paea_pilot_lab_test.py'),
                   'PilotLab.test_t05_t17_ordered_revocation_first_use_and_opposite_order']
        result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True)
        # Setup failure is not the intended falsification; its diagnostics remain visible.
        if not output.is_dir():
            self.fail('negative setup failed: ' + result.stderr.decode(errors='replace'))
        for suffix, raw in [('stdout', result.stdout), ('stderr', result.stderr)]:
            (output / ('negative-run.' + suffix)).write_bytes(raw)
        (output / 'negative-run.json').write_text(json.dumps({'argv': command,
            'overrides': {'SWARM_PAEA_PILOT_EVIDENCE': str(output),
                          'SWARM_PAEA_CONTENTION_FAULT': 'early-release'},
            'exit': result.returncode, 'interpretation': 'expected failed fixture assertion, never a PASS campaign'}, indent=2)+'\n')
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn(b'Ran 1 test', result.stdout)
        self.assertEqual(result.stderr, b'')
        self.assertIn(b'FAILED (failures=4)', result.stdout)
        self.assertEqual(result.stdout.count(b'AssertionError: 0 != 1 : contention peer must fail'), 4)
        self.assertNotIn(b'ERROR:', result.stdout)
        calls = json.loads((output / 'calls.json').read_bytes())
        holders = [call for call in calls if call.get('early_release_fault')]
        peers = [call for call in calls if call.get('fixture_worker') and not call['pause']]
        controls = [call for call in calls if call.get('fixture_worker') and call['pause'] and not call['early_release_fault']]
        self.assertEqual(len(holders), 4)
        self.assertEqual(len(peers), 4)
        self.assertEqual(len(controls), 4)
        self.assertTrue(all(call['exit'] == 0 for call in peers + controls))
        self.assertTrue(all(call['exit'] == 1 for call in holders))
        for call in holders:
            error = json.loads((output / ('call-%04d.stderr' % call['id'])).read_bytes())
            self.assertEqual(error['cause'], 'laboratory-test-gate-aborted')
        self.assertFalse((output / 'paea-lab-report.json').exists())
        print('PAEA_CONTENTION_EARLY_RELEASE_DETECTED four assertions, controls and reaped holders', flush=True)


if __name__ == '__main__':
    unittest.main(testRunner=unittest.TextTestRunner(stream=sys.stdout, verbosity=2))
