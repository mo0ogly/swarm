#!/usr/bin/env python3
"""Verify current delivery and preserved native qualification, without paid agents.

The full-suite receipt is external supervisor evidence, not a replay of Go tests.
This control really rebuilds Make/native installation and executes the Docker CLI.
"""
import hashlib
import json
import os
import pathlib
import re
import subprocess
import tempfile

root = pathlib.Path(__file__).resolve().parents[2]
proof = root / 'docs/plans/version-history/combined-qualification'
receipt = json.loads((proof / 'receipt.json').read_text())
for path, digest in receipt['inputs'].items():
    assert hashlib.sha256((root / path).read_bytes()).hexdigest() == digest, path
for name, digest in receipt['logs'].items():
    assert hashlib.sha256((proof / name).read_bytes()).hexdigest() == digest, name
suite = (proof / 'full-go-suite.log').read_text()
assert 'Full default-suite coverage: 926 ' in suite
assert sorted(re.findall(r'Group (\d+) exit (\d+)', suite)) == [('1', '0'), ('2', '0'), ('3', '0'), ('4', '0')]
assert all(c['exit_code'] == 0 for c in receipt['commands'])

def run(command, env=None):
    return subprocess.check_output(command, cwd=root, env=env, text=True)

current = json.loads(run(['bin/swarm', '--json', 'version']))['binary']
assert run(['bin/swarm', 'version']) == run(['bin/swarm', '--version'])
env = dict(os.environ)
for key, value in current.items():
    if key == 'provenance':
        continue
    env['SWARM_' + key.upper()] = str(value).lower() if isinstance(value, bool) else value or ''
with tempfile.TemporaryDirectory(prefix='swarm-version-delivery-') as temp:
    path = pathlib.Path(temp)
    absent = path / 'must-not-create-storage'
    no_storage = json.loads(run(['bin/swarm', '--root', str(absent), '--json', 'version']))
    assert no_storage['binary'] == current and not absent.exists()
    target = path / 'make' / 'swarm'
    run(['make', 'build', 'SWARM_OUTPUT=' + str(target)], env)
    assert json.loads(run([str(target), '--json', 'version']))['binary'] == current
    run(['bash', './install.sh', '--mode', 'native', '--bin-dir', str(path / 'installed')], env)
    assert json.loads(run([str(path / 'installed/swarm'), '--json', 'version']))['binary'] == current

docker = json.loads(run(['docker', 'run', '--rm', '--network', 'none', '--entrypoint', '/usr/local/bin/swarm', 'swarm-version-combined:local', '--json', 'version']))
assert docker['binary'] == current
for path in ['README.md', 'README.en.md', 'INSTALL.md', 'docs/en/INSTALL.md', 'GUIDE-UTILISATEUR.md', 'docs/en/USER-GUIDE.md']:
    text = (root / path).read_text()
    assert '--json version' in text, path + ': CLI version documentation absent'
    assert 'Version et nouveautés' in text or "Version and what's new" in text, path + ': web entry point absent'
    assert 'devel' in text, path + ': development identity absent'
print('PASS: preserved full native suite 926 targets + race/frontend/vet remain bound; Make/native installer/Docker identities match; version creates no storage; six bilingual documents describe actual entry points. Full suite is not rerun by this control.')
