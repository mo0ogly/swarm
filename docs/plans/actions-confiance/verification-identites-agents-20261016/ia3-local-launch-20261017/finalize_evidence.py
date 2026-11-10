from pathlib import Path
import datetime, hashlib, json, re, subprocess, tarfile

ROOT = Path.cwd()
B = Path(__file__).resolve().parent
BASE = json.loads((B / 'checkout.json').read_text())

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def command(args):
    r = subprocess.run(args, capture_output=True, text=True)
    assert r.returncode == 0, (args, r.returncode, r.stderr)
    return r.stdout

docs = list(json.loads((B / 'before-docs.json').read_text()))
reference = json.loads((B / 'before-sources.json').read_text())
added = ['internal/engine/agent_identity_config_launch_linux.go',
         'internal/engine/agent_identity_config_launch_linux_test.go']
inputs = {str(p): sha(p) for p in sorted([Path('go.mod'), Path('go.sum'),
          *Path('internal').rglob('*.go'), *Path('cmd').rglob('*.go')])}
for label in ['normal-engine-03', 'race-engine-03', 'vet-03', 'build-03', 'workflow-03']:
    record = json.loads((B / (label + '-run.json')).read_text())
    assert record['exit'] == 0 and not record['changed_inputs'], label
    assert inputs == json.loads((B / (label + '-inputs.json')).read_text()), label
    if 'engine' in label:
        assert record['counts'] == {'pass':117,'fail':0,'skip':0}, label
assert all(sha(p) == digest for p, digest in reference.items()), 'reference changed'
assert not command(['git', 'diff', '--check'])
assert not command(['gofmt', '-l', *added])
assert '5 tests/subtests' in (B / 'workflow-03.jsonl').read_text()
assert not (B / 'workflow-03.stderr').read_bytes()

source_patch = bytearray()
for path in added:
    result = subprocess.run(['git','diff','--no-index','--','/dev/null',path], capture_output=True)
    assert result.returncode == 1 and not result.stderr, path
    source_patch.extend(result.stdout)
(B / 'source.diff').write_bytes(source_patch)
(B / 'documentation.diff').write_text(command(['git','diff',BASE['head'],'--',*docs]))
for name, paths in [('final-sources', sorted(set(reference) | set(added))),
                    ('final-docs', docs)]:
    with tarfile.open(B / (name + '.tar.gz'), 'w:gz') as archive:
        for path in paths:
            archive.add(path, arcname=path)
    with tarfile.open(B / (name + '.tar.gz')) as archive:
        assert all(hashlib.sha256(archive.extractfile(p).read()).hexdigest() == sha(p) for p in paths)
(B / 'source-final.json').write_text(json.dumps(inputs, indent=2)+'\n')
(B / 'documentation-final.json').write_text(json.dumps({p:sha(p) for p in docs},indent=2)+'\n')

# Inspect new/local report links, including the evidence links just introduced.
link_paths = [B / 'RAPPORT-IA3c2f03c.md']
for path in docs:
    if 'AGENT-IDENTITY-CONTRACT' in path:
        section = Path(path).read_text().split('## Remise privée IA3c.2f-03c' if '/architecture/' in path else '## Private handoff IA3c.2f-03c',1)[1]
        for target in re.findall(r'\]\(([^)]+)\)',section):
            assert (Path(path).parent / target.split('#')[0]).exists(), target
for path in link_paths:
    for target in re.findall(r'\]\(([^)]+)\)',path.read_text()):
        if '://' not in target:
            assert (path.parent / target.split('#')[0]).exists(), target
for path in [*map(Path,docs),*map(Path,added),*link_paths]:
    assert all(line.rstrip() == line for line in path.read_text().splitlines()), path

environment = json.loads(command(['go','env','-json','GOOS','GOARCH','GOROOT','GOMOD','GOVERSION','GOFLAGS','GOWORK','CGO_ENABLED','GOTOOLCHAIN']))
assert environment['GOFLAGS'] == '' and environment['GOWORK'] == '', environment
environment.update(kernel=command(['uname','-r']).strip(),go_version=command(['go','version']).strip())
(B / 'environment.json').write_text(json.dumps(environment,indent=2)+'\n')
toolchain = [Path(environment['GOROOT']) / 'src/syscall/exec_linux.go',
             Path(environment['GOROOT']) / 'src/os/pidfd_linux.go']
(B / 'toolchain-contract.json').write_text(json.dumps({str(p):sha(p) for p in toolchain},indent=2)+'\n')
with tarfile.open(B / 'toolchain-contract.tar.gz','w:gz') as archive:
    for path in toolchain: archive.add(path,arcname=path.name)
checks = dict(at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
    initial_head=BASE['head'], current_head=command(['git','rev-parse','HEAD']).strip(),
    input_count=len(inputs), overlay=None, reference_unchanged=len(reference), source_additions=added,
    source_archive_verified=True, docs_archive_verified=True, gofmt=True, diff_check=True,
    new_links=True, whitespace=True, diagrams_visually_inspected=['fr','en'],
    diagram_hashes={lang:sha(B/('diagram-'+lang+'.png')) for lang in ['fr','en']},
    diagram_source_matches={lang:(B/('diagram-'+lang+'.mmd')).read_text() in Path(path).read_text()
        for lang,path in [('fr','docs/architecture/AGENT-IDENTITY-CONTRACT.md'),('en','docs/en/AGENT-IDENTITY-CONTRACT.md')]},
    config_reuse=json.loads((B/'config-reuse.json').read_text()),
    binary=dict(path='/tmp/swarm-identity-config-launch-03c-fixture-03',sha256=sha('/tmp/swarm-identity-config-launch-03c-fixture-03'),deployed=False),
    report_sha256=sha(B/'RAPPORT-IA3c2f03c.md'), independent_review=False, public_activation=False,
    external_unowned_directory='docs/diagrams')
owned = set(docs + added)
prefix = str(B.relative_to(ROOT)) + '/'
status = command(['git','status','--porcelain','-z']).split('\x00')
checks['external_changed_paths'] = [entry[3:] for entry in status if entry and entry[3:] not in owned and not entry[3:].startswith(prefix)]
assert all(checks['diagram_source_matches'].values())
(B/'checks-final.json').write_text(json.dumps(checks,indent=2)+'\n')
print(json.dumps(dict(inputs=len(inputs),reference_unchanged=len(reference),checks='PASS',head=checks['current_head'])))
