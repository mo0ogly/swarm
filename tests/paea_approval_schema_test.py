#!/usr/bin/env python3
"""Closed approval framing and report consistency, never authenticated authority."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import unittest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]


class ApprovalSchema(unittest.TestCase):
    """Shape checks use positive controls; native code establishes binding semantics."""
    def test_approval_envelope_preserves_two_strings_and_refuses_open_shape(self):
        """Reject unknown/missing/wrong-type/version fields, not inner JSON strings."""
        schema = json.loads((ROOT / 'schemas/paea/http-approval-check-v1.schema.json').read_text())
        Draft202012Validator.check_schema(schema)
        check = Draft202012Validator(schema)
        valid = dict(schema_version=1, document='{"revision":9223372036854775807}', approval='{}')
        for key, value in [('schema_version', 2), ('document', {}), ('approval', {}), ('approved', True), ('root', 'private')]:
            check.validate(valid)
            changed = dict(valid); changed[key] = value
            self.assertFalse(check.is_valid(changed))
        for key in valid:
            check.validate(valid)
            changed = dict(valid); del changed[key]
            self.assertFalse(check.is_valid(changed))

    def test_approval_report_is_closed_and_match_flags_agree_with_field_names(self):
        """All frozen-vector report shapes retain no-auth markers and exact raw hashes.

        Schema checks shape/flag consistency only; field order and semantic equality
        are independently checked by native/browser code, never assumed from JSON.
        """
        schema = json.loads((ROOT / 'schemas/paea/approval-report-v1.schema.json').read_text())
        Draft202012Validator.check_schema(schema)
        check = Draft202012Validator(schema)
        vectors = json.loads((ROOT / 'tests/fixtures/paea/contract-v1.json').read_text())['vectors']
        for vector in vectors:
            document = json.dumps(dict(proposal=vector['proposal'], context=vector['context'])).encode()
            report = dict(schema_version=1, scope='offline-data-only',
                          document=dict(document_sha256=hashlib.sha256(document).hexdigest(),
                                        action_digest=vector['action_digest'], intent_id=vector['intent_id']),
                          approval_sha256='a'*64, binding_matches=True, mismatched_fields=[],
                          authority='not-established', approval_authenticity='not-established',
                          approval_validity='not-established', effect='not-established')
            check.validate(report)
            for key, value in [('extra', True), ('approval_validity', 'established'),
                               ('approval_authenticity', 'established'), ('approval_sha256', 'bad'),
                               ('binding_matches', False), ('mismatched_fields', ['action_digest'])]:
                check.validate(report)
                changed = copy.deepcopy(report); changed[key] = value
                self.assertFalse(check.is_valid(changed))
            report.update(binding_matches=False, mismatched_fields=['action_digest', 'policy_digest'])
            check.validate(report)
            for invalid in [['unknown'], ['action_digest', 'action_digest'], []]:
                check.validate(report)
                changed = copy.deepcopy(report); changed['mismatched_fields'] = invalid
                self.assertFalse(check.is_valid(changed))


if __name__ == '__main__':
    result = unittest.TextTestRunner(stream=sys.stdout, verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(ApprovalSchema))
    raise SystemExit(not result.wasSuccessful())
