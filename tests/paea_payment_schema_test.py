#!/usr/bin/env python3
"""Payment registry/request/report schema checks; semantic authority is not proven."""
import copy
import json
from pathlib import Path
import sys
import unittest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]


class PaymentSchema(unittest.TestCase):
    """Check maintained closed shapes against frozen valid controls and mutations."""

    @classmethod
    def setUpClass(cls):
        """Load source schemas and fixtures without generating or editing them."""
        cls.schema = json.loads((ROOT / 'schemas/paea/payment-resolution-v1.schema.json').read_bytes())
        cls.registry = json.loads((ROOT / 'tests/fixtures/paea/payment-registry-v1.json').read_bytes())
        cls.request = json.loads((ROOT / 'tests/fixtures/paea/payment-request-v1.json').read_bytes())
        cls.validator = Draft202012Validator(cls.schema)

    def test_closed_registry_and_request_examples(self):
        """Source schemas agree on name/payment representation, without an authority claim."""
        Draft202012Validator.check_schema(self.schema)
        core = json.loads((ROOT / 'schemas/paea/contract-v1.schema.json').read_bytes())
        self.assertEqual(self.schema['$defs']['name'], core['$defs']['name'])
        self.assertEqual(self.schema['$defs']['report']['properties']['proposal']['properties']['action'], core['$defs']['payment'])
        self.validator.validate(self.registry)
        self.validator.validate(self.request)

    def test_invalid_shapes_refuse_after_valid_control(self):
        """Unknown fields, missing aliases and invalid terms cannot become valid shapes."""
        for field, value in [('amount_minor', '01'), ('currency', 'eur'), ('invoice_references', []), ('invoice_references', ['F-12', 'F-12']), ('debt_id', '../escape')]:
            self.validator.validate(self.registry)
            bad = copy.deepcopy(self.registry)
            bad['debts'][0][field] = value
            self.assertFalse(self.validator.is_valid(bad), field)
        for field, value in [('schema_version', 2), ('expected_registry_revision', 0), ('request_id', ''), ('approved', True)]:
            self.validator.validate(self.request)
            bad = copy.deepcopy(self.request)
            bad[field] = value
            self.assertFalse(self.validator.is_valid(bad), field)

    def test_report_requires_explicit_non_authority_and_exact_hash_shapes(self):
        """Report shape excludes authenticated/paid labels; hashes alone prove no source."""
        debt = self.registry['debts'][0]
        action = {k: debt[k] for k in ['invoice_version', 'amount_minor', 'currency', 'beneficiary_account_id']}
        action.update(kind='payment.settle', invoice_id=debt['debt_id'])
        report = dict(schema_version=1, scope='offline-data-only', authority='not-established',
                      registry_authenticity='not-established', effect='not-established',
                      registry_sha256='a'*64, request_sha256='b'*64, registry_revision=1,
                      proposal=dict(schema_version=1, request_id='attempt-1', action=action))
        for field, value in [('authority', 'established'), ('effect', 'COMMITTED'), ('registry_sha256', 'bad'), ('registry_authenticity', 'verified'), ('extra', True)]:
            self.validator.validate(report)
            bad = copy.deepcopy(report)
            bad[field] = value
            self.assertFalse(self.validator.is_valid(bad), field)

    def test_http_envelope_keeps_original_json_as_closed_string_fields(self):
        """Transport preserves JSON integer text; shape validation grants no permission."""
        schema = json.loads((ROOT / 'schemas/paea/http-payment-check-v1.schema.json').read_bytes())
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema)
        valid = dict(schema_version=1, registry='{"revision":9223372036854775807}', request='{}')
        for field, value in [('registry', {}), ('request', None), ('schema_version', 2), ('root', 'private')]:
            validator.validate(valid)
            bad = dict(valid)
            bad[field] = value
            self.assertFalse(validator.is_valid(bad))


if __name__ == '__main__':
    result = unittest.TextTestRunner(stream=sys.stdout, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(PaymentSchema))
    raise SystemExit(not result.wasSuccessful())
