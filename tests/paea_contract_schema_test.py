#!/usr/bin/env python3
"""PA-01 JSON Schema and Python vector-oracle checks; no effects or E2/E3 claim.

Run named unittest cases with no global watchdog. Schemas describe shape; Rust
additionally checks raw duplicate/number syntax and documented semantic invariants.
"""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import unittest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]


class ContractSchema(unittest.TestCase):
    """Validate schema structure, both domains and non-authoritative input shape."""

    @classmethod
    def setUpClass(cls):
        """Read retained schema/vectors without generating or changing them."""
        cls.schema = json.loads((ROOT / 'schemas/paea/contract-v1.schema.json').read_text())
        cls.vectors = json.loads((ROOT / 'tests/fixtures/paea/contract-v1.json').read_text())['vectors']
        cls.validator = Draft202012Validator(cls.schema)
        spec = importlib.util.spec_from_file_location('paea_vector_oracle', ROOT / 'tools/paea/make_contract_vectors.py')
        cls.oracle = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.oracle)

    def test_schema_and_fourteen_both_domain_vectors(self):
        """Every frozen document must satisfy the declared shape."""
        Draft202012Validator.check_schema(self.schema)
        self.assertEqual(len(self.vectors), 14)
        for vector in self.vectors:
            with self.subTest(vector=vector['name']):
                self.validator.validate({k: vector[k] for k in ('proposal', 'context')})

    def test_retained_canonical_bytes_and_digest_are_reproducible(self):
        """Recompute Python spec encoding independently of Rust outputs."""
        for v in self.vectors:
            with self.subTest(vector=v['name']):
                regenerated = self.oracle.vector(v['name'], v['proposal']['action'], v['proposal']['request_id'], **v['context'])
                self.assertEqual(regenerated, v)
                self.assertEqual(hashlib.sha256(v['canonical'].encode()).hexdigest(), v['action_digest'])

    def test_unknown_authority_claims_and_malformed_shapes_refuse(self):
        """Schema itself never admits a supplied actor or approval flag."""
        valid = {k: self.vectors[0][k] for k in ('proposal', 'context')}
        for field in ('actor', 'approved', 'integrity', 'capability'):
            bad = copy.deepcopy(valid)
            bad['proposal'][field] = 'forged'
            self.assertFalse(self.validator.is_valid(bad))
        for amount in (420000, '01', '1e3', '1\n'):
            bad = copy.deepcopy(valid)
            bad['proposal']['action']['amount_minor'] = amount
            self.assertFalse(self.validator.is_valid(bad))

    def test_label_duplicates_and_identifier_controls_refuse(self):
        """Existing identifier grammar and set labels are explicit representations."""
        valid = {k: self.vectors[0][k] for k in ('proposal', 'context')}
        bad = copy.deepcopy(valid)
        bad['context']['sources'][0]['confidentiality_labels'] = ['internal', 'internal']
        self.assertFalse(self.validator.is_valid(bad))
        for value in ('../escape', 'project\n', '', 'a' * 101):
            bad = copy.deepcopy(valid)
            bad['context']['project_id'] = value
            self.assertFalse(self.validator.is_valid(bad))

    def test_approval_and_receipt_definitions_keep_observations_distinct(self):
        """AUTHORIZED cannot masquerade as an effect-service outcome."""
        definitions = self.schema['$defs']
        receipt = Draft202012Validator({'$ref': '#/$defs/receipt', '$defs': definitions})
        value = dict(schema_version=1, action_digest='a'*64, intent_id='b'*64, sink_id='sink-1', outcome='UNKNOWN', observation_digest='c'*64)
        self.assertTrue(receipt.is_valid(value))
        value['outcome'] = 'AUTHORIZED'
        self.assertFalse(receipt.is_valid(value))
        approval = Draft202012Validator({'$ref': '#/$defs/approval', '$defs': definitions})
        self.assertTrue(approval.is_valid(dict(schema_version=1, action_digest='a'*64, policy_digest='b'*64, approver_id='human-1')))


if __name__ == '__main__':
    result = unittest.TextTestRunner(stream=sys.stdout, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(ContractSchema))
    raise SystemExit(not result.wasSuccessful())
