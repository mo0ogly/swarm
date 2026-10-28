"""Check entry links, tracked privacy and Rust-only delivery commands offline.

Lexical guards cover explicit operational files, not arbitrary shell indirection
or absence of all Go mentions. Historical proofs/compatibility references remain;
failures report candidates for semantic review, never imply runtime acceptance.
No mission database, network, model call or source mutation.
"""
from pathlib import Path
import re
import subprocess
import urllib.parse

root = Path(__file__).resolve().parents[1]
documents = ['README.md', 'README.en.md', 'INSTALL.md', 'docs/en/INSTALL.md',
             'GUIDE-UTILISATEUR.md', 'docs/en/USER-GUIDE.md', 'CONTRIBUTING.md', 'SECURITY.md']
errors = []
for name in documents:
    p = root / name
    for link in re.findall(r'\]\(([^)]+)\)', p.read_text()):
        link = link.strip('<>').split('#')[0]
        if not link or urllib.parse.urlsplit(link).scheme or link.startswith('//'):
            continue
        if not (p.parent / urllib.parse.unquote(link)).exists():
            errors.append(f'{name}: missing link target {link}')
tracked = subprocess.check_output(['git', 'ls-files', '-z'], cwd=root).decode().split('\0')
for name in filter(None, tracked):
    p = Path(name)
    if p.suffix == '.go':
        errors.append(f'Legacy Go source still tracked in Rust distribution: {name}')
    if '.swarm' in p.parts or p.name in ('install.env', '.env') or p.suffix in ('.db', '.sqlite', '.sqlite3'):
        errors.append(f'Private runtime file tracked: {name}')
for name in ['install.sh', 'deploy/entrypoint.sh', 'swarm.sh']:
    if not (root / name).stat().st_mode & 0o111:
        errors.append(f'Not executable: {name}')
for name in ['swarm.sh', 'tools/swarm_local.py', 'tests/test_swarm_local.py']:
    if name not in tracked or not (root / name).is_file():
        errors.append(f'Missing published launcher dependency: {name}')
# Only operational delivery entries: historical Go proofs remain valid records.
for name in ['install.sh', 'Dockerfile', 'Makefile', '.github/workflows/ci.yml', 'build.sh', 'swarm.sh']:
    if re.search(r'golang:|actions/setup-go|\bgo (?:build|mod|vet|test)\b|command -v go\b|supervision_go_suite', (root / name).read_text()):
        errors.append(f'Obsolete Go dependency in active delivery entry: {name}')
if errors:
    raise SystemExit('\n'.join(errors))
print('PASS: entry docs links, executable scripts, no tracked runtime databases or env files')
