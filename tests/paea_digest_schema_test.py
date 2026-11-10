#!/usr/bin/env python3
"""Exact SHA256 representation across five previously looser PAEA schema fields.
Each known field has valid hex witnesses before malformed variants; this verifies
schema field semantics only, not native producer/consumer/global conformance.
"""
from pathlib import Path
import json,unittest
from jsonschema import Draft202012Validator
ROOT=Path(__file__).resolve().parents[1]
FIELDS=[('operator-package-cli-v1.schema.json',['properties','operator_package_pin']),('operator-package-cli-v1.schema.json',['properties','service_artifact_pin']),('personal-service-response-v1.schema.json',['properties','request_digest']),('service-installation-cli-v1.schema.json',['properties','startup','oneOf',1,'properties','bootstrap_pin']),('provisioning-request-cli-v1.schema.json',['properties','request_digest'])]
class ExactDigestFields(unittest.TestCase):
    """Known digest fields are SHA256 hex64, not operator-configured policy."""
    def test_exact_digest_representations(self):
        """Preserve all five counterexamples; normal values remain accepted."""
        checked=0
        for name,path in FIELDS:
            with self.subTest(schema=name,field=str(path)):
                field=json.loads((ROOT/'schemas/paea'/name).read_text())
                for key in path:field=field[key]
                validator=Draft202012Validator(field)
                for valid in ['00'*32,'af'*32]:self.assertTrue(validator.is_valid(valid))
                for bad in ['ab'*32+'\n','AB'*32,'ab'*31,'ab'*33,'g'*64,None,64]:self.assertFalse(validator.is_valid(bad))
                checked+=1
        self.assertEqual(checked,len(FIELDS))
        print('EXACT_DIGEST_SCHEMA_PASS fields=5',flush=True)
if __name__=='__main__':unittest.main()
