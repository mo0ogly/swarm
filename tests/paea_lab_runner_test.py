#!/usr/bin/env python3
"""Exercise the public laboratory command once, then refusal without mutation.

An existing pinned native binary and fresh SWARM_PAEA_LAB_EVIDENCE are required.
Real effects/oracles remain isolated; no build, provider, live Root or watchdog.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import shutil
import jsonschema
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools.paea.run_pilot import CASES, export_report


class LabCommand(unittest.TestCase):
    def test_actual_campaign_export_then_hash_and_existing_output_refusals(self):
        """A real valid campaign precedes wrong-hash and overwrite faults; sources retained."""
        output = Path(os.environ['SWARM_PAEA_LAB_EVIDENCE']).resolve()
        binary = Path(os.environ['SWARM_PAEA_TEST_BINARY']).resolve(strict=True)
        digest = os.environ['SWARM_PAEA_TEST_BINARY_SHA256']
        self.assertEqual(hashlib.sha256(binary.read_bytes()).hexdigest(), digest)
        argv = [sys.executable, str(ROOT / 'tools/paea/run_pilot.py'), '--binary', str(binary),
                '--sha256', digest, '--output', str(output)]
        result = subprocess.run(argv, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        print(result.stdout.decode(), end='')
        raw = (output / 'paea-lab-report.json').read_bytes()
        value = json.loads(raw)
        jsonschema.Draft202012Validator(json.loads((ROOT / 'schemas/paea/laboratory-report-v1.schema.json').read_bytes())).validate(value)
        self.assertEqual([v['id'] for v in value['tests']], list(CASES))
        self.assertTrue(all(v['status'] == 'PASS' for v in value['tests']))
        self.assertEqual(value['binary_sha256'], digest)
        self.assertEqual(value['authority'], 'simulated')
        self.assertEqual(value['protected_invariants'], 'not-established')
        self.assertEqual(value['refuted_claim'], 'raw-invoice-identifiers-guarantee-global-debt-uniqueness')
        for key, name in [('recipe_result_sha256', 'recipe/result.json'),
                          ('source_inventory_sha256', 'recipe/inputs.before.json'), ('calls_sha256', 'observations/calls.json')]:
            self.assertEqual(value[key], hashlib.sha256((output / name).read_bytes()).hexdigest())
        for name, command, cause in [('wrong-hash', argv[:], b'laboratory-binary-hash-mismatch'),
                                    ('existing-output', argv[:], b'laboratory-output-must-be-new')]:
            if name == 'wrong-hash':
                command[command.index('--sha256') + 1] = '0' * 64
            refused = subprocess.run(command, capture_output=True)
            (output / (name + '.stderr')).write_bytes(refused.stderr)
            self.assertEqual(refused.returncode, 2)
            self.assertIn(cause, refused.stderr)
            self.assertEqual((output / 'paea-lab-report.json').read_bytes(), raw)
        # Fresh refusal has no effect and does not create an empty success directory.
        command = argv[:]; fresh = output.parent / (output.name + '-bad-hash')
        command[command.index('--sha256') + 1] = '0' * 64
        command[command.index('--output') + 1] = str(fresh)
        self.assertEqual(subprocess.run(command, capture_output=True).returncode, 2)
        self.assertFalse(fresh.exists())
        altered = output / 'altered'
        altered.mkdir()
        shutil.copytree(output / 'recipe', altered / 'recipe')
        with (altered / 'recipe/step-0001.stdout').open('ab') as stream:
            stream.write(b'corrupt-proof')
        with self.assertRaises(ValueError):
            export_report(altered / 'recipe', output / 'observations', digest)
        self.assertFalse((altered / 'paea-lab-report.json').exists())
        print('PAEA_LAB_COMMAND_EXPORT_AND_REFUSALS_PASS', flush=True)


if __name__ == '__main__':
    unittest.main(verbosity=2)
