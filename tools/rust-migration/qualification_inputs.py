"""Exact input collector shared by leaf qualification runners.
Collect actual Go normal/race graphs, generated sources, Rust/fixture membership,
locked registry archives and actual cached source bytes. No mission operations.
"""
if not __debug__:
    raise RuntimeError("qualification requires active assertion checks; disable Python optimization")
import hashlib
import json
import re
from pathlib import Path, PurePosixPath
import shutil
import tarfile
import tomllib
from review_increment import resolve_baseline, require_review, digest as review_digest
from build_environment import capture as capture_build_environment

def archive_sources(path,entries):
    """Same complete tar inputs/metadata, cheaper gzip; no qualification cutoff."""
    with tarfile.open(path,'w:gz',compresslevel=1)as archive:
        for source,name in entries:archive.add(source,arcname=name)
def stream(raw):
    remaining=raw.decode().lstrip();d=json.JSONDecoder()
    while remaining:
        item,n=d.raw_decode(remaining);yield item;remaining=remaining[n:].lstrip()

def graph(raw):
    fields=('GoFiles','CompiledGoFiles','TestGoFiles','XTestGoFiles','CgoFiles','CFiles','CXXFiles','HFiles','SFiles','SysoFiles','EmbedFiles','Imports')
    return {p['ImportPath']:{'Dir':p.get('Dir'),**{f:sorted(p.get(f,[])) for f in fields}} for p in stream(raw)}

def rust_assets(ROOT,sources):
    """Conservative literal include closure; never silently omit dynamic inputs.

    Current local sources use a literal include_str for the v37 SQL. Expressions
    such as OUT_DIR/concat require a compiler-aware extension before qualification.
    """
    found=set();pending=[p for p in sources if p.suffix=='.rs'];seen=set()
    while pending:
        source=pending.pop()
        if source in seen:continue
        seen.add(source);text=source.read_text()
        for match in re.finditer(r'\b(include_str|include_bytes|include)\s*!\s*\(',text):
            tail=text[match.end():].lstrip()
            raw=re.match(r'r(\#*)"(.*?)"\1\s*\)',tail,re.S)
            ordinary=re.match(r'("(?:[^"\\]|\\.)*")\s*\)',tail,re.S)
            if raw:relative=raw.group(2)
            elif ordinary:relative=json.loads(ordinary.group(1))
            else:raise RuntimeError('Rust include expression requires effective-input collection: '+str(source))
            target=(source.parent/relative).resolve()
            if not target.is_relative_to(ROOT.resolve()):raise RuntimeError('Rust include outside qualification root: '+str(target))
            if not target.is_file():raise RuntimeError('Rust include is not a file: '+str(target))
            found.add(target)
            if target.suffix=='.rs':pending.append(target)
    return found

def local_files(ROOT):
    sources={p for folder,endings in ((ROOT/'rust',('.rs','.toml','.lock')),(ROOT/'tools/rust-migration',('.go','.py'))) for p in folder.rglob('*') if p.is_file() and p.suffix in endings and 'target' not in p.relative_to(folder).parts}
    return sources|rust_assets(ROOT,sources)

def collect(ROOT, OUTPUT, ENV, run, packages, allow_root=False, extra_files=()):
    """Freeze effective source/toolchain inputs and require a scoped review.

    Caller supplies a fresh owned evidence directory, exact fixture environment,
    streaming runner and actual Go packages. An explicit repository baseline in
    SWARM_MIGRATION_REVIEW_BASELINE is required before any expensive checks. The
    returned close_graphs callback rechecks Go graphs and requires a complete,
    fresh four-axis source-review.json in OUTPUT; call it before claiming PASS.
    Open/invalid review and changed inputs raise, retaining evidence. This saves
    archives/reports and runs compiler inventory commands, never live missions,
    provider calls, independent review or runtime acceptance. The lexical/human
    review boundary is documented by review_increment; custom runners that bypass
    this collector still need equivalent checks and cannot inherit its evidence.
    """
    # Explicit prior evidence and a fresh declared review are required for every
    # shared qualification. This is a developer-method guard, never engine policy.
    review_baseline=resolve_baseline(ROOT,ENV)
    review_baseline_hash=review_digest(review_baseline)
    (OUTPUT/'review-baseline.json').write_text(json.dumps(dict(path=str(review_baseline.relative_to(ROOT)),sha256=review_baseline_hash),indent=2)+'\n')
    goenv=json.loads(run('go-env',['go','env','-json','GOROOT','GOOS','GOARCH','GOFLAGS','GOWORK']))
    if 'overlay' in goenv['GOFLAGS'] or goenv['GOWORK']:raise RuntimeError('effective overlay/workspace collector required')
    version=run('rust-version',['rustc','-vV']).decode();host=next(s.split(': ',1)[1]for s in version.splitlines()if s.startswith('host: '))
    run('go-version',['go','version']);run('cc-version',['cc','--version'])
    build_env=capture_build_environment(ENV)
    (OUTPUT/'build-environment.json').write_text(json.dumps(build_env,indent=2)+'\n')
    inputs={ROOT/'go.mod',ROOT/'go.sum'}
    for extra in extra_files:
        path=Path(extra).resolve()
        if not path.is_relative_to(ROOT.resolve())or not path.is_file():raise RuntimeError('extra verification input must be an existing repository file')
        inputs.add(path)
    inputs.update(local_files(ROOT))
    graphs={};root_inputs={}
    for label,flags in [('normal',[]),('race',['-race'])]:
        raw=run('go-inputs-'+label,['go','list',*flags,'-compiled','-deps','-test','-json',*packages])
        graphs[label]=(flags,graph(raw))
        for package in stream(raw):
            if package['ImportPath']=='swarm.local/companion':
                if not allow_root:raise RuntimeError('unexpected root/embed dependency')
                assert Path(package['Dir'])==ROOT and package.get('EmbedFiles'), 'actual root/embed producer missing'
                root_inputs[label]={field:package.get(field,[])for field in ('GoFiles','CompiledGoFiles','EmbedPatterns','EmbedFiles')}

            directory=Path(package.get('Dir','/nonexistent'))
            for field in ('GoFiles','CompiledGoFiles','TestGoFiles','XTestGoFiles','CgoFiles','CFiles','CXXFiles','HFiles','SFiles','SysoFiles','EmbedFiles'):
                inputs.update(directory/name for name in package.get(field,[])if(directory/name).is_file())
    if allow_root:
        assert set(root_inputs)=={'normal','race'}, 'root/embed missing in actual Go graphs'
        (OUTPUT/'root-embedded-inputs.json').write_text(json.dumps(root_inputs,indent=2)+'\n')
    metadata=json.loads(run('cargo-metadata',['cargo','metadata','--manifest-path','rust/Cargo.toml','--locked','--offline','--all-features','--format-version','1','--filter-platform',host]))
    lock=tomllib.loads((ROOT/'rust/Cargo.lock').read_text());checksums={(p['name'],p['version']):p.get('checksum')for p in lock['package']}
    registry=[];roots={};archives=OUTPUT/'registry-crates';archives.mkdir()
    for package in metadata['packages']:
        if not package['source']:continue
        directory=Path(package['manifest_path']).parent;name=package['name']+'-'+package['version'];cache=directory.parents[2]/'cache'/directory.parent.name/(name+'.crate')
        digest=hashlib.sha256(cache.read_bytes()).hexdigest();assert digest==checksums[(package['name'],package['version'])]
        shutil.copyfile(cache,archives/cache.name);roots[directory]='registry/'+name;count=0
        with tarfile.open(cache)as archive:
            for member in archive.getmembers():
                if not member.isfile():continue
                parts=PurePosixPath(member.name).parts;assert parts[0]==name and '..'not in parts
                p=directory.joinpath(*parts[1:]);assert hashlib.sha256(p.read_bytes()).digest()==hashlib.sha256(archive.extractfile(member).read()).digest(),(name,member.name)
                inputs.add(p);count+=1
        registry.append(dict(name=package['name'],version=package['version'],crate_sha256=digest,actual_files=count))
    goroot=Path(goenv['GOROOT'])
    def key(p):
        if p.is_relative_to(ROOT):return str(p.relative_to(ROOT))
        if p.is_relative_to(goroot):return 'go-toolchain/'+str(p.relative_to(goroot))
        for directory,prefix in roots.items():
            if p.is_relative_to(directory):return prefix+'/'+str(p.relative_to(directory))
        # Go dependencies are outside the local repository, never silently omitted.
        return 'external-go/'+hashlib.sha256(str(p.parent).encode()).hexdigest()+'/'+p.name
    local_before=local_files(ROOT)
    def close_graphs():
        """Recheck original Go graphs/baseline and publish the required review.

        Runs read-only compiler inventories, then validates exact source/proof
        hashes and all review axes. Missing/stale/open declarations raise with
        retained diagnostics; a successful call grants no runtime acceptance.
        """
        for label,(flags,expected_graph) in graphs.items():
            raw=run('go-inputs-'+label+'-after',['go','list',*flags,'-compiled','-deps','-test','-json',*packages])
            assert graph(raw)==expected_graph, 'actual Go input graph changed'
        if review_digest(review_baseline)!=review_baseline_hash:raise RuntimeError('review-baseline-changed-during-qualification')
        require_review(ROOT,OUTPUT,review_baseline)
    def snapshot():
        assert local_files(ROOT)==local_before, 'local input added or removed during qualification'
        out={key(p):hashlib.sha256(p.read_bytes()).hexdigest()for p in sorted(inputs)};assert len(out)==len(inputs);return out
    before=snapshot();(OUTPUT/'inputs-before.json').write_text(json.dumps(before,indent=2)+'\n');(OUTPUT/'registry.json').write_text(json.dumps(registry,indent=2)+'\n')
    archive_sources(OUTPUT/'sources.tar.gz',((p,key(p))for p in sorted(inputs)if not any(p.is_relative_to(d)for d in roots)))
    return before,snapshot,registry,close_graphs
