#!/usr/bin/env python3
"""Validate fictional downloads and their page/translation inventory, not authority."""
import json
from html.parser import HTMLParser
from pathlib import Path
import sys
import unittest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]


class ExampleLinks(HTMLParser):
    """Collect only the explicit example disclosure's links and translatable text."""
    def __init__(self):
        """Initialize empty collection state; no file is read until HTML is supplied."""
        super().__init__()
        self.inside = False
        self.links = []
        self.text = []

    def handle_starttag(self, tag, attrs):
        """Enter the known non-nested disclosure and retain its download attributes."""
        attrs = dict(attrs)
        if tag == 'details' and attrs.get('id') == 'paea-example-help':
            self.inside = True
        if self.inside and tag == 'a':
            self.links.append(attrs)

    def handle_endtag(self, tag):
        """End the known disclosure; other page text cannot satisfy this check."""
        if tag == 'details':
            self.inside = False

    def handle_data(self, data):
        """Retain visible nonempty disclosure text for translation coverage."""
        if self.inside and data.strip():
            self.text.append(data.strip())


class Examples(unittest.TestCase):
    """Check shapes and usable links; native journeys establish semantic consistency."""
    def test_all_eight_fictional_files_have_the_declared_data_shape(self):
        """Require closed v1 documents/receipts/registry/request from the actual schemas."""
        contract = json.loads((ROOT / 'schemas/paea/contract-v1.schema.json').read_text())
        payment = json.loads((ROOT / 'schemas/paea/payment-resolution-v1.schema.json').read_text())
        files = sorted((ROOT / 'web/paea-examples/v1').glob('*.json'))
        self.assertEqual(len(files), 8)
        for file in files:
            value = json.loads(file.read_text())
            if file.name.endswith(('-document.json', '-after.json')):
                schema = contract
            else:
                kind = file.stem.split('-')[-1]
                parent = contract if kind == 'receipt' else payment
                schema = dict(parent, **{'$ref': '#/$defs/' + kind})
                # The definitions are the selected root, not the report/document root.
                schema = {'$schema': parent['$schema'], '$defs': parent['$defs'], '$ref': schema['$ref']}
            Draft202012Validator(schema).validate(value)

    def test_all_three_disclosures_link_only_exact_versioned_downloads_with_english(self):
        """No live data URL, path selection or missing labels in example download help."""
        seen = set()
        translations = json.loads((ROOT / 'locales/en.json').read_text())
        for page, count in [('paea.html', 4), ('paea-payment.html', 2), ('paea-compare.html', 4)]:
            parser = ExampleLinks()
            parser.feed((ROOT / 'web' / page).read_text())
            self.assertEqual(len(parser.links), count)
            for link in parser.links:
                name = link['download']
                self.assertEqual(link['href'], '/paea-examples/v1/' + name)
                self.assertEqual(Path(name).name, name)
                self.assertTrue((ROOT / 'web/paea-examples/v1' / name).is_file())
                seen.add(name)
            for text in parser.text:
                self.assertTrue(translations.get(text), text)
        self.assertEqual(seen, {p.name for p in (ROOT / 'web/paea-examples/v1').glob('*.json')})


if __name__ == '__main__':
    result = unittest.TextTestRunner(stream=sys.stdout, verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(Examples))
    raise SystemExit(not result.wasSuccessful())
