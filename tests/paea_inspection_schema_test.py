#!/usr/bin/env python3
"""Inspection schema framing only; native tests establish semantics, not authority."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import unittest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]


class InspectionSchema(unittest.TestCase):
    """Reject open/wrong-shaped envelopes while retaining exact inner JSON text."""
    def test_request_is_closed_single_exact_document_string(self):
        """Schema rejects paths/flags as fields, objects and missing or wrong versions."""
        schema = json.loads((ROOT / 'schemas/paea/http-inspection-check-v1.schema.json').read_text())
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema)
        valid = dict(schema_version=1, document='{"revision":9223372036854775807}')
        for key, value in [('schema_version', 2), ('document', {}), ('root', 'private'), ('approved', True)]:
            validator.validate(valid)
            changed = dict(valid); changed[key] = value
            self.assertFalse(validator.is_valid(changed))
        for key in valid:
            changed = dict(valid); del changed[key]
            self.assertFalse(validator.is_valid(changed))

    def test_report_preserves_frozen_canonical_text_and_closed_non_authority(self):
        """Validate frozen vector reports and refuse altered shape/markers/hash types."""
        schema = json.loads((ROOT / 'schemas/paea/inspection-v1.schema.json').read_text())
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema)
        vectors = json.loads((ROOT / 'tests/fixtures/paea/contract-v1.json').read_text())['vectors']
        for vector in vectors:
            raw = json.dumps(dict(proposal=vector['proposal'], context=vector['context'])).encode()
            report = dict(schema_version=1, scope='offline-data-only',
                          document=dict(document_sha256=hashlib.sha256(raw).hexdigest(),
                                        action_digest=vector['action_digest'], intent_id=vector['intent_id']),
                          canonicalization='paea-json-v1', canonical=vector['canonical'],
                          authority='not-established', effect='not-established', approval_validity='not-established')
            validator.validate(report)
            for key, value in [('extra', True), ('approval_validity', 'established'), ('canonical', {}), ('schema_version', 2)]:
                changed = copy.deepcopy(report); changed[key] = value
                self.assertFalse(validator.is_valid(changed))


if __name__ == '__main__':
    result = unittest.TextTestRunner(stream=sys.stdout, verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(InspectionSchema))
    raise SystemExit(not result.wasSuccessful())
