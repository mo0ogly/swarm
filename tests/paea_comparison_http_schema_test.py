#!/usr/bin/env python3
"""Closed HTTP comparison framing; inner validation and authority need actual Rust."""
import copy
import json
from pathlib import Path
import sys
import unittest
from jsonschema import Draft202012Validator
ROOT = Path(__file__).resolve().parents[1]

class ComparisonEnvelope(unittest.TestCase):
    """Require exact string fields, without treating a string as a valid action."""
    def test_closed_string_envelope_and_refusal_shapes(self):
        """Schema permits raw integer text and refuses paths/flags/objects/missing fields."""
        schema = json.loads((ROOT / 'schemas/paea/http-comparison-check-v1.schema.json').read_text())
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema)
        valid = dict(schema_version=1, before='{"revision":9223372036854775807}', after='{}')
        for key, value in [('before', {}), ('after', None), ('schema_version', 2), ('approved', True), ('root', 'private')]:
            validator.validate(valid)
            invalid = copy.deepcopy(valid); invalid[key] = value
            self.assertFalse(validator.is_valid(invalid), key)
        for missing in ['before', 'after', 'schema_version']:
            validator.validate(valid)
            invalid = dict(valid); del invalid[missing]
            self.assertFalse(validator.is_valid(invalid), missing)
        self.assertEqual(json.loads(json.dumps(valid)), valid)

    def test_browser_path_catalogue_matches_shared_report_schema(self):
        """Each fixed producer path has a translated client label, with no hidden extension."""
        import re
        schema = json.loads((ROOT / 'schemas/paea/comparison-v1.schema.json').read_text())
        source = (ROOT / 'web/paea.js').read_text()
        match = re.search(r'const comparisonFields = (\{.*?\});', source, re.S)
        self.assertIsNotNone(match)
        labels = json.loads(match.group(1))
        self.assertEqual(set(labels), set(schema['$defs']['comparison']['properties']['changed_paths']['items']['enum']))
        translations = json.loads((ROOT / 'locales/en.json').read_text())
        for label in labels.values():
            self.assertIsInstance(label, str)
            self.assertTrue(translations.get(label), label)

if __name__ == '__main__':
    result = unittest.TextTestRunner(stream=sys.stdout, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(ComparisonEnvelope))
    raise SystemExit(not result.wasSuccessful())
