#!/usr/bin/env python3
"""Exercise only offline debt resolution on a pinned normal-profile executable.

Requires SWARM_PAEA_TEST_BINARY, SWARM_PAEA_TEST_BINARY_SHA256 and a fresh
SWARM_PAEA_PAYMENT_EVIDENCE. Retains exact fixture inputs and process streams
before assertions; no build, live Root, provider, retry or global watchdog.
"""
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]


class PublicPayment(unittest.TestCase):
    """Validate public output and safe refusals without authenticating snapshots."""

    @classmethod
    def setUpClass(cls):
        """Require fresh owned evidence and the exact caller-selected executable."""
        cls.out = Path(os.environ['SWARM_PAEA_PAYMENT_EVIDENCE']).resolve()
        cls.out.mkdir()
        cls.binary = Path(os.environ['SWARM_PAEA_TEST_BINARY']).resolve(strict=True)
        cls.expected_hash = os.environ['SWARM_PAEA_TEST_BINARY_SHA256']
        cls.sequence = 0
        cls.calls = []
        cls.registry = json.loads((ROOT / 'tests/fixtures/paea/payment-registry-v1.json').read_bytes())
        cls.request = json.loads((ROOT / 'tests/fixtures/paea/payment-request-v1.json').read_bytes())
        cls.validator = Draft202012Validator(json.loads((ROOT / 'schemas/paea/payment-resolution-v1.schema.json').read_bytes()))

    @classmethod
    def tearDownClass(cls):
        """Preserve the complete call index and verify binary stability at closure."""
        (cls.out / 'calls.json').write_text(json.dumps(cls.calls, indent=2) + '\n')
        if hashlib.sha256(cls.binary.read_bytes()).hexdigest() != cls.expected_hash:
            raise ValueError('exercised-binary-drift')
        print('PAEA_PAYMENT_PUBLIC_BINARY_STABLE ' + cls.expected_hash, flush=True)
        print('PAEA_PAYMENT_PUBLIC_CALLS ' + str(len(cls.calls)), flush=True)

    def call(self, registry, request, lang='en'):
        """Save exact named-file inputs/streams before assertions, with no runtime IO.

        Each process runs in a fresh fixture directory. Typed public refusal is
        consumed by tests; no fallback report is produced after nonzero exit.
        """
        self.assertEqual(hashlib.sha256(self.binary.read_bytes()).hexdigest(), self.expected_hash)
        type(self).sequence += 1
        cwd = self.out / ('call-%03d' % self.sequence)
        cwd.mkdir()
        left = json.dumps(registry, ensure_ascii=False).encode()
        right = json.dumps(request, ensure_ascii=False).encode()
        (cwd / 'registry.json').write_bytes(left)
        (cwd / 'request.json').write_bytes(right)
        argv = [str(self.binary), 'protection', 'resolve-payment', 'registry.json', 'request.json', '--json', '--lang', lang]
        result = subprocess.run(argv, cwd=cwd, stdin=subprocess.DEVNULL, capture_output=True)
        (cwd / 'stdout').write_bytes(result.stdout)
        (cwd / 'stderr').write_bytes(result.stderr)
        self.calls.append(dict(argv=argv, directory=cwd.name, exit=result.returncode))
        self.assertEqual(hashlib.sha256(self.binary.read_bytes()).hexdigest(), self.expected_hash)
        self.assertFalse((cwd / '.swarm').exists())
        if result.returncode == 0:
            self.assertEqual(result.stderr, b'')
            value = json.loads(result.stdout)
            self.validator.validate(value)
            self.assertEqual(value['registry_sha256'], hashlib.sha256(left).hexdigest())
            self.assertEqual(value['request_sha256'], hashlib.sha256(right).hexdigest())
            return value
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, b'')
        diagnostic = json.loads(result.stderr)
        self.assertNotIn('CANARY', result.stderr.decode())
        return diagnostic['failure']['code']

    def test_normal_profile_resolves_aliases_in_both_languages(self):
        """Actual binary output matches the schema and stable registry terms, FR/EN."""
        for lang in ('fr', 'en'):
            first = self.call(self.registry, self.request, lang)
            request = copy.deepcopy(self.request)
            request['invoice_reference'] = 'legacy-12'
            alias = self.call(self.registry, request, lang)
            self.assertEqual(alias['proposal'], first['proposal'])
            self.assertNotEqual(alias['request_sha256'], first['request_sha256'])
            self.assertEqual(first['proposal']['action']['invoice_id'], 'debt-17')
            self.assertEqual(first['proposal']['action']['amount_minor'], '420000')
            request['invoice_reference'] = 'F-12'
            request['supplier_id'] = 'supplier-2'
            other = self.call(self.registry, request, lang)
            self.assertEqual(other['proposal']['action']['invoice_id'], 'debt-18')
            self.assertEqual(other['proposal']['action']['beneficiary_account_id'], 'account-18')

    def test_normal_profile_safe_semantic_refusals_then_recovery(self):
        """A valid control brackets each semantic fault, preserving safe error codes."""
        for field, value, code in [
            ('project_id', 'CANARY', 'protection-registry-project-mismatch'),
            ('expected_registry_revision', 2, 'protection-registry-stale'),
            ('expected_invoice_version', 'v8', 'protection-invoice-stale'),
            ('invoice_reference', 'f-12', 'protection-registry-reference-unknown'),
            ('supplier_id', 'CANARY', 'protection-registry-reference-unknown'),
            ('amount_minor', '1', 'protection-document-invalid'),
        ]:
            self.assertIsInstance(self.call(self.registry, self.request), dict)
            bad = copy.deepcopy(self.request)
            bad[field] = value
            self.assertEqual(self.call(self.registry, bad), code)
            self.assertIsInstance(self.call(self.registry, self.request), dict)
        for field, value, code in [
            ('debt_id', 'debt-17', 'protection-registry-ambiguous'),
            ('supplier_id', 'supplier-1', 'protection-registry-ambiguous'),
            ('amount_minor', '01', 'protection-amount-representation'),
        ]:
            self.assertIsInstance(self.call(self.registry, self.request), dict)
            bad = copy.deepcopy(self.registry)
            bad['debts'][1][field] = value
            self.assertEqual(self.call(bad, self.request, 'fr'), code)
            self.assertIsInstance(self.call(self.registry, self.request), dict)


if __name__ == '__main__':
    result = unittest.TextTestRunner(stream=sys.stdout, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(PublicPayment))
    raise SystemExit(not result.wasSuccessful())
