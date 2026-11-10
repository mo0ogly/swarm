#!/usr/bin/env python3
"""Intent shape only; no generation/current authority or rollback guarantee.
Rust additionally enforces exact canonical member order/bytes and source binding.
"""
from pathlib import Path
import json,unittest
from jsonschema import Draft202012Validator
ROOT=Path(__file__).resolve().parents[1]
class IntentSchema(unittest.TestCase):
    """Immutable pre-generation marker, never a key or authenticated approval."""
    def setUp(self):
        """Read maintained schema and explicit synthetic valid witness."""
        schema=json.loads((ROOT/'schemas/paea/initial-provisioning-intent-v1.schema.json').read_text());Draft202012Validator.check_schema(schema);self.validator=Draft202012Validator(schema);self.good={'schema_version':1,'protocol':'swarm-initial-provisioning-intent-v1','request_pin':'ab'*32}
    def test_closed_valid_data(self):
        """Exact declared version/pin shape fits without implying authority."""
        self.assertTrue(self.validator.is_valid(self.good))
    def test_missing_unknown_or_promoted_claim_refuses(self):
        """No omitted record member, extra secret or invented authority accepted."""
        for key in self.good:
            bad=dict(self.good);del bad[key];self.assertFalse(self.validator.is_valid(bad))
        for key,value in [('secret','PRIVATE'),('approved',True),('schema_version',2),('schema_version',True),('protocol','other')]:self.assertFalse(self.validator.is_valid(dict(self.good,**{key:value})))
    def test_exact_pin_representation(self):
        """Length64 is SHA256 representation, not configurable execution policy."""
        for value in ['AB'*32,'ab'*31,'ab'*33,'ab'*32+'\n',None,64]:self.assertFalse(self.validator.is_valid(dict(self.good,request_pin=value)))
if __name__=='__main__':unittest.main()
