#!/usr/bin/env python3
"""Verify receipt-bound screenshot bundle. This is not a visual AI review."""
import hashlib, json, pathlib, struct, zipfile
root = pathlib.Path(__file__).resolve().parents[2]
manifest = json.loads((root/'docs/screenshots/version-history/manifest.json').read_text())
with zipfile.ZipFile(root/'docs/screenshots/version-history/captures.zip') as bundle:
    assert set(bundle.namelist()) == set(manifest['captures']) | {'result.json'}
    result = json.loads(bundle.read('result.json'))
    assert result['status'] == 'PASS' and len(result['checks']) == 10 and not result['errors'] and not result['failed']
    for name, digest in manifest['captures'].items():
        raw = bundle.read(name)
        assert hashlib.sha256(raw).hexdigest() == digest
        assert raw[:8] == b'\x89PNG\r\n\x1a\n'
        width, height = struct.unpack('>II', raw[16:24])
        assert width > 0 and height > 0
assert len(manifest['captures']) == 10
print('PASS: 10 preserved real-journey PNGs, SHA-256, dimensions and native browser result; visual inspection remains supervisor evidence')
