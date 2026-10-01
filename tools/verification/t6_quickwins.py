#!/usr/bin/env python3
"""Execute T6 checks on a temporary binary and an isolated browser fixture."""
import hashlib
import pathlib
import subprocess
import tempfile
import sys
import json

ROOT = pathlib.Path(__file__).resolve().parents[2]
for name in ('runtime_health.go', 'runtime_health_test.go', 'web/runtime-health.js',
             'web/cockpit.js', 'tests/quick_wins_ui.cjs'):
    print('SOURCE', name, hashlib.sha256((ROOT / name).read_bytes()).hexdigest(), flush=True)

def run(command):
    print('COMMAND', command, flush=True)
    subprocess.run(command, cwd=ROOT, check=True, timeout=150)

mode = sys.argv[1] if len(sys.argv) > 1 else 'all'
files = ('runtime_health.go', 'runtime_health_test.go', 'web/runtime-health.js', 'web/cockpit.js', 'tests/quick_wins_ui.cjs')
fingerprints = {name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest() for name in files}
out = ROOT / 'test-results/t6-engine'
if mode in ('entry', 'all'):
    for name in files:
        assert (ROOT / name).is_file(), name
    assert (ROOT / 'node_modules/puppeteer').is_dir()
    print('PASS prerequisites: source, tests and browser dependency present')
if mode in ('version', 'all'):
    run(['go', 'test', '.', '-run', '^TestServerVersionReflectsBuildAndLocalGitWithoutNetwork$', '-count=1', '-v'])
if mode in ('browser', 'all'):
    with tempfile.TemporaryDirectory(prefix='swarm-t6-control-') as directory:
        binary = str(pathlib.Path(directory) / 'swarm')
        run(['go', 'build', '-o', binary, '.'])
        run(['node', 'tests/quick_wins_ui.cjs', binary, str(out)])
    (out / 'sources.json').write_text(json.dumps(fingerprints, sort_keys=True))
if mode in ('validation', 'delivery', 'all'):
    assert json.loads((out / 'sources.json').read_text()) == fingerprints, 'stale browser evidence'
    result = json.loads((out / 'result.json').read_text())
    assert result['status'] == 'PASS' and not result['errors'] and not result['offHost']
    for lang in ('fr', 'en'):
        for theme in ('etat', 'sombre'):
            for kind in ('version', 'deleted-mission'):
                assert (out / f'{kind}-{lang}-{theme}.png').stat().st_size > 1000
    print('PASS current browser artifacts, no JS errors, no external request, eight screenshots')
if mode in ('delivery', 'all'):
    for name in ('docs/T6-quick-wins.md', 'docs/plan-843bb3ce22-T6.md'):
        report = (ROOT / name).read_text()
        assert 'REQ-VER-01' in report and 'REQ-LINK-01' in report
    print('PASS required reports cover both requirement IDs; independent review remains separate')
assert mode in ('entry', 'version', 'browser', 'validation', 'delivery', 'all')
print('PASS T6 control', mode, flush=True)
