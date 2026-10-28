#!/usr/bin/env python3
"""Structural private-anchor schema only; Rust additionally enforces exact bytes.
No schema verdict grants protected-locator provenance/current authority or creates
material. Digests here are synthetic values, not actual issued package pins.
"""
from pathlib import Path
import json
import unittest
from jsonschema import Draft202012Validator
ROOT=Path(__file__).resolve().parents[1]
class AnchorSchema(unittest.TestCase):
    """Closed shape and exact SHA256 representation, without canonical-byte claims."""
    def setUp(self):
        """Load current schema and explicit synthetic witness."""
        schema=json.loads((ROOT/'schemas/paea/operator-anchor-v1.schema.json').read_text())
        Draft202012Validator.check_schema(schema)
        self.validate=Draft202012Validator(schema)
        self.good={'schema_version':1,'protocol':'swarm-operator-anchor-v1','request_pin':'ab'*32,'package_pin':'cd'*32}
    def test_valid_closed_witness(self):
        """Both independent data bindings fit, without authenticating either."""
        self.assertTrue(self.validate.is_valid(self.good))
    def test_shape_and_protocol_refuse(self):
        """Unknown/missing/boolean protocol fields refuse, no permissive fallback."""
        for change in [{'extra':True},{'schema_version':True},{'schema_version':2},{'protocol':'other'}]:
            self.assertFalse(self.validate.is_valid(dict(self.good,**change)))
        for key in self.good:
            bad=dict(self.good);del bad[key];self.assertFalse(self.validate.is_valid(bad))
    def test_exact_pin_representation_refuses_newline(self):
        """Regex dollar accepts final newline alone; exact length is also required."""
        for key in ['request_pin','package_pin']:
            for value in ['ab'*31,'ab'*33,'AB'*32,'ab'*32+'\n','x'*64,4,None]:
                self.assertFalse(self.validate.is_valid(dict(self.good,**{key:value})))
if __name__=='__main__':
    unittest.main()
