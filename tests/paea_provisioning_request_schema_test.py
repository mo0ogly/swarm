#!/usr/bin/env python3
"""Closed preflight result witnesses; schemas never authenticate actual reports."""
from pathlib import Path
import copy
import json
import unittest
from jsonschema import Draft202012Validator, ValidationError
ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads((ROOT / 'schemas/paea/provisioning-request-cli-v1.schema.json').read_text())
Draft202012Validator.check_schema(SCHEMA)
VALIDATE = Draft202012Validator(SCHEMA)


def witness():
    """Synthetic complete report only; no current authority, entropy or service."""
    return {'schema_version': 1, 'protocol': 'swarm-provisioning-request-cli-v1',
            'operation_result': 'completed', 'request_digest': 'ab' * 32,
            'service_uid': 0, 'controller_uid': (1 << 32) - 2,
            'request_validated': True, 'input_durability': 'directory_acknowledged',
            **{f: False for f in ['provisioning_started', 'human_authentication_verified',
              'current_authority_verified', 'service_started', 'confinement_verified']}}


class Report(unittest.TestCase):
    """Valid controls precede invalid mutations; schema is data validation only."""
    def test_valid_uid_representation_is_not_authority(self):
        """Root/largest valid numeric data may appear without privilege claims."""
        VALIDATE.validate(witness())
    def test_boolean_claims_may_not_be_promoted(self):
        """Every unproved execution/identity claim must remain false."""
        for field in ['provisioning_started', 'human_authentication_verified', 'current_authority_verified', 'service_started', 'confinement_verified']:
            good = witness()
            VALIDATE.validate(good)
            bad = copy.deepcopy(good)
            bad[field] = True
            with self.assertRaises(ValidationError):
                VALIDATE.validate(bad)
            VALIDATE.validate(good)
    def test_missing_unknown_or_reserved_uid_refuse(self):
        """No partial/empty success, unknown field or invalid Linux marker."""
        for field in ['request_digest', 'controller_uid', 'unknown', 'service_uid']:
            good = witness()
            VALIDATE.validate(good)
            bad = copy.deepcopy(good)
            if field == 'request_digest':
                del bad[field]
            elif field == 'unknown':
                bad[field] = 'unexpected'
            else:
                bad[field] = (1 << 32) - 1
            with self.assertRaises(ValidationError):
                VALIDATE.validate(bad)
            VALIDATE.validate(good)


if __name__ == '__main__':
    unittest.main(verbosity=2)
