#!/usr/bin/env python3
"""Execute T4 checks on a temporary binary and an isolated browser fixture."""
import hashlib
import pathlib
import subprocess
import tempfile
import sys
import json

ROOT = pathlib.Path(__file__).resolve().parents[2]
FILES = ('run_limits_web.go', 'run_limits_web_test.go', 'web/admin.js',
         'web/cockpit.js', 'web/index.html', 'tests/run_limits_admin_ui.cjs')

def run(command):
    print('COMMAND', command, flush=True)
    subprocess.run(command, cwd=ROOT, check=True, timeout=150)

mode = sys.argv[1] if len(sys.argv) > 1 else 'all'
fingerprints = {name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest() for name in FILES}
out = ROOT / 'test-results/admin'

if mode in ('entry', 'all'):
    for name in FILES:
        assert (ROOT / name).is_file(), name
    assert (ROOT / 'node_modules/puppeteer').is_dir()
    print('PASS prerequisites: source, tests and browser dependency present')

if mode in ('unit', 'all'):
    run(['go', 'build', './...'])
    run(['go', 'test', '-run', 'TestRunLimits', './...', '-count=1', '-v'])
    run(['node', 'tests/i18n_test.cjs'])
    print('PASS go build, targeted run-limits tests and i18n catalogue parity')

if mode in ('browser', 'all'):
    with tempfile.TemporaryDirectory(prefix='swarm-t4-control-') as directory:
        binary = str(pathlib.Path(directory) / 'swarm')
        run(['go', 'build', '-o', binary, '.'])
        run(['node', 'tests/run_limits_admin_ui.cjs', binary, str(out)])
    (out / 'sources.json').write_text(json.dumps(fingerprints, sort_keys=True))

if mode in ('validation', 'delivery', 'all'):
    assert json.loads((out / 'sources.json').read_text()) == fingerprints, 'stale browser evidence'
    result = json.loads((out / 'result.json').read_text())
    assert result['status'] == 'PASS' and not result['errors']
    for prefix in ('admin', 'observation'):
        for lang in ('fr', 'en'):
            for theme in ('etat', 'sombre'):
                assert (out / f'{prefix}-{lang}-{theme}.png').stat().st_size > 1000
    print('PASS current browser artifacts, no JS errors, eight screenshots (admin + observation, FR/EN x 2 themes)')

if mode in ('delivery', 'all'):
    report = (ROOT / 'docs/T4-web-admin.md').read_text()
    for req in ('req-14', 'req-15', 'req-16', 'req-17'):
        assert req in report
    print('PASS report covers req-14..req-17; independent review remains separate')

assert mode in ('entry', 'unit', 'browser', 'validation', 'delivery', 'all')
print('PASS T4 control', mode, flush=True)
