#!/usr/bin/env python3
"""Read-only installation evidence checks; browser replay uses a temporary root."""
import hashlib
import json
import pathlib
import re
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]


def run(args):
    print('COMMAND', args, flush=True)
    return subprocess.run(args, cwd=ROOT, check=True, timeout=180,
                          text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout


review = json.loads((ROOT / 'docs/plan-843bb3ce22-T8-visual-review.json').read_text())
images = {item['path']: item for item in review['images']}
referenced = set()
for name in ('INSTALL.md', 'docs/en/INSTALL.md'):
    text = (ROOT / name).read_text()
    before = run(['git', 'show', '7110a0b805caf4be1bb09a57b776176b8dc52a68:' + name])
    # Every original line must remain, in the original order.
    remaining = iter(text.splitlines())
    for line in before.splitlines():
        assert any(current == line for current in remaining), 'original content removed: ' + name
    for target in re.findall(r'!\[[^\]]*\]\(([^)]+)\)', text):
        image = (ROOT / name).parent.joinpath(target).resolve()
        relative = str(image.relative_to(ROOT))
        referenced.add(relative)
        assert relative in images, 'image without visual review: ' + relative
        assert hashlib.sha256(image.read_bytes()).hexdigest() == images[relative]['sha256'], 'image changed after visual review'
        assert images[relative]['secret_visible'] is False and images[relative]['observation']
    print('PASS original content preserved in', name, flush=True)
assert referenced == set(images) and len(images) == 17
assert 'French UI' in (ROOT / 'docs/en/INSTALL.md').read_text()
run(['git', 'diff', '--check'])
print('PASS 17 referenced images match the visual review; this is hash binding, not automatic image analysis', flush=True)
with tempfile.TemporaryDirectory(prefix='swarm-t8-control-') as directory:
    binary = str(pathlib.Path(directory) / 'swarm')
    run(['go', 'build', '-trimpath', '-o', binary, '.'])
    output = run(['node', 'tools/verification/t8_saved_config_shots.cjs', binary, str(pathlib.Path(directory) / 'shots')])
    print(output, flush=True)
    assert 'PASS fr saved revision 1' in output and 'PASS en saved revision 1' in output
print('PASS FR/EN saved state replay and read-back; launch screenshot is historical, captioned as French UI', flush=True)
