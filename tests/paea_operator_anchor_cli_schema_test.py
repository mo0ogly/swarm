#!/usr/bin/env python3
"""Closed actual-read report only; no trust/current-authority/package proof."""
from pathlib import Path
import unittest,json
from jsonschema import Draft202012Validator
ROOT=Path(__file__).resolve().parents[1]
class AnchorReport(unittest.TestCase):
    """Successful closed metadata and refusal of silent authority promotion."""
    def setUp(self):
        """Build explicit synthetic witness from fixed protocol constants."""
        schema=json.loads((ROOT/'schemas/paea/operator-anchor-cli-v1.schema.json').read_text())
        Draft202012Validator.check_schema(schema);self.validator=Draft202012Validator(schema)
        self.good={k:v['const'] for k,v in schema['properties'].items() if 'const' in v}
        self.good.update(request_pin='ab'*32,package_pin='cd'*32)
    def test_good_closed_report(self):
        """Exact read/ack metadata fit the report without granting authority."""
        self.assertTrue(self.validator.is_valid(self.good))
    def test_missing_extra_or_pin_promotion_refuses(self):
        """Omitted fields, extra secret and noncanonical digests must refuse."""
        for key in self.good:
            bad=dict(self.good);del bad[key];self.assertFalse(self.validator.is_valid(bad))
        for key,value in [('secret','PRIVATE'),('request_pin','ab'*32+'\n'),('package_pin','AB'*32),('read_validation','pin_matched')]:
            self.assertFalse(self.validator.is_valid(dict(self.good,**{key:value})))
    def test_unproven_claim_promotion_refuses(self):
        """Caller locator trust is a precondition; UID/current/human are not proven."""
        for key in ['locator_trust_verified','current_authority_verified','human_authentication_verified','separate_os_uid_verified','operator_package_read','material_generated','service_installed']:
            self.assertFalse(self.validator.is_valid(dict(self.good,**{key:True})))
if __name__=='__main__':
    unittest.main()
