"""Verify preserved supervisor evidence; this does NOT rerun the full Go suite."""
import hashlib, json, pathlib
root = pathlib.Path(__file__).resolve().parents[3]
receipt = json.loads((root / 'docs/plans/version-history/V2-native-receipt.json').read_text())
assert receipt['full_suite']['exit_code'] == 0
for name, digest in receipt['inputs'].items():
    assert hashlib.sha256((root / name).read_bytes()).hexdigest() == digest, 'candidate drift: ' + name
for check in ['full_suite', 'docker_runtime']:
    item = receipt[check]
    assert item['exit_code'] == 0
    raw = (root / item['output']).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == item['output_sha256'], 'output drift: ' + check
print('PASS: native supervisor evidence preserved and bound to the unchanged candidate; no new full-suite execution claimed')
