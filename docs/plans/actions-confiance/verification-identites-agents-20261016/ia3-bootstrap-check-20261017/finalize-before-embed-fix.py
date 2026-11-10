from pathlib import Path
import datetime, difflib, hashlib, importlib.util, json, re, subprocess, tarfile

ROOT=Path.cwd()
B=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('checks',B/'run_checks.py')
checks_module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(checks_module)
inputs=checks_module.inputs()
before=json.loads((B/'before.json').read_text())
docs=[p for p in before if p.endswith('.md')]
owned=['internal/engine/main.go','internal/enrollmentconfig/store_linux.go',
       'internal/enrollmentconfig/path_linux.go','internal/enrollmentbootstrap/bootstrap_linux.go',
       'internal/enrollmentbootstrap/bootstrap_linux_test.go','internal/engine/agent_identity_bootstrap_linux.go',
       'internal/engine/agent_identity_bootstrap_linux_test.go']

def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def run(args):
    r=subprocess.run(args,capture_output=True,text=True)
    assert r.returncode==0,(args,r.returncode,r.stderr)
    return r.stdout

for label,count in [('normal-packages-04',141),('race-packages-04',141),('normal-engine-04',118),('race-engine-04',118),
                    ('vet-04',None),('build-04',None),('public-cli-01',None),('workflow-04',None)]:
    record=json.loads((B/(label+'-run.json')).read_text())
    assert record['exit']==0 and not record['changed_inputs'],label
    assert inputs==json.loads((B/(label+'-inputs.json')).read_text()),label
    if count:assert record['counts']=={'pass':count,'fail':0,'skip':0},label
assert '5 tests/subtests' in (B/'workflow-04.jsonl').read_text()
assert '"bind_alias_rejected": true' in (B/'public-cli-01.jsonl').read_text()
assert not run(['git','diff','--check'])
assert not run(['gofmt','-l',*owned])
with tarfile.open(B/'before.tar.gz') as archive:
    initial={p:archive.extractfile(p).read().decode() for p in [*docs,*[p for p in owned if p in before]]}
    old=initial['internal/enrollmentconfig/store_linux.go']
    old_traversal=old[old.index('\tif uint32(os.Geteuid())'):old.index('\ts := &Store{dir: os.NewFile')]
    helper=Path('internal/enrollmentconfig/path_linux.go').read_text()
    new_traversal=helper[helper.index('\tif uint32(os.Geteuid())'):helper.index('\treturn os.NewFile')]
    assert re.sub(r'\s+','',old_traversal)==re.sub(r'\s+','',new_traversal),'changed directory boundary'
for name,paths in [('source',owned),('documentation',docs)]:
    diff=''.join(''.join(difflib.unified_diff(initial.get(p,'').splitlines(True),Path(p).read_text().splitlines(True),fromfile='before/'+p,tofile='after/'+p)) for p in paths)
    (B/(name+'.diff')).write_text(diff)
    with tarfile.open(B/('final-'+name+'.tar.gz'),'w:gz') as archive:
        for path in paths:archive.add(path,arcname=path)
    with tarfile.open(B/('final-'+name+'.tar.gz')) as archive:
        assert all(hashlib.sha256(archive.extractfile(p).read()).hexdigest()==sha(p) for p in paths)

# Reconstruct all final Go/module inputs from the original archive plus the
# seven final owned sources. The only 02/04 changes are the two test fixtures.
with tarfile.open(B/'candidate-01-go-sources.tar.gz') as archive:
    reconstructed={p:hashlib.sha256(archive.extractfile(p).read()).hexdigest() for p in inputs}
reconstructed.update({p:sha(p) for p in owned})
assert reconstructed==inputs,'candidate reconstruction differs'
old_inputs=json.loads((B/'normal-packages-02-inputs.json').read_text())
delta=sorted(p for p in set(inputs)|set(old_inputs) if inputs.get(p)!=old_inputs.get(p))
assert delta==sorted(['internal/enrollmentbootstrap/bootstrap_linux_test.go','internal/engine/agent_identity_bootstrap_linux_test.go'])

report=B/'RAPPORT-IA3c2f03d.md'
for path in [*map(Path,owned),*map(Path,docs),report]:
    assert all(line.rstrip()==line for line in path.read_text().splitlines()),path
for target in re.findall(r'\]\(([^)]+)\)',report.read_text()):
    if '://' not in target:assert (report.parent/target.split('#')[0]).exists(),target
for language,path in [('fr','docs/architecture/AGENT-IDENTITY-CONTRACT.md'),('en','docs/en/AGENT-IDENTITY-CONTRACT.md')]:
    assert (B/('diagram-'+language+'.mmd')).read_text() in Path(path).read_text()
    assert (B/('diagram-'+language+'.png')).is_file()

environment=json.loads(run(['go','env','-json','GOOS','GOARCH','GOROOT','GOMOD','GOVERSION','GOFLAGS','GOWORK','CGO_ENABLED']))
assert environment['GOFLAGS']=='' and environment['GOWORK']==''
environment['kernel']=run(['uname','-r']).strip()
environment['bubblewrap']=run(['bwrap','--version']).strip()
(B/'environment.json').write_text(json.dumps(environment,indent=2)+'\n')
(B/'source-final.json').write_text(json.dumps(inputs,indent=2)+'\n')
(B/'owned-source-final.json').write_text(json.dumps({p:sha(p) for p in owned},indent=2)+'\n')
(B/'documentation-final.json').write_text(json.dumps({p:sha(p) for p in docs},indent=2)+'\n')
binary='/tmp/swarm-identity-bootstrap-03d-fixture-04'
digest=sha(binary)
for prefix in ['cli-valid','cli-wrong-pin','namespace-valid','namespace-alias']:
    assert json.loads((B/(prefix+'-01.json')).read_text())['binary_sha256']==digest
scope=set(owned+docs)
proof_prefix=str(B.relative_to(ROOT))+'/'
status=run(['git','status','--porcelain','-z']).split('\x00')
record=dict(at=datetime.datetime.now(datetime.timezone.utc).isoformat(),head=run(['git','rev-parse','HEAD']).strip(),
            input_count=len(inputs),overlay=None,owned_sources=owned,normal_counts={'packages':141,'engine':118},
            race_counts={'packages':141,'engine':118},zero_fail_skip=True,source_archives_verified=True,
            complete_input_reconstruction=True,original_path_boundary_unchanged=True,fixture_only_delta_02_04=delta,
            gofmt=True,diff_check=True,whitespace=True,report_links=True,diagrams_rendered_and_inspected=['fr','en'],
            binary={'path':binary,'sha256':digest,'deployed':False},report_sha256=sha(report),
            human_authenticated=False,service_activated=False,host_account_created=False,independent_review=False,
            out_of_scope_paths=[entry[3:] for entry in status if entry and entry[3:] not in scope and not entry[3:].startswith(proof_prefix)])
(B/'checks-final.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(dict(checks='PASS',inputs=len(inputs),source_boundary='unchanged',binary_sha256=digest)))
