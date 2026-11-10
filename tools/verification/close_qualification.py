#!/usr/bin/env python3
"""Close an existing Swarm runtime qualification after its exact-byte review.

Separate proof-method phase: does not rerun or relabel native tests. All original
runtime inputs, archives, profiles and named inventories must still match. This
local consistency check grants no independent acceptance, identity or deployment.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
import tarfile

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/rust-migration'))
from qualification_inputs import stream, graph, local_files
from build_environment import capture as build_environment
from recorded_process import capture
from review_increment import digest, publish, require_review, local, retained_parity, go_test_inventory

FIELDS=('GoFiles','CompiledGoFiles','TestGoFiles','XTestGoFiles','CgoFiles','CFiles','CXXFiles','HFiles','SFiles','SysoFiles','EmbedFiles')


def verify_inputs(root, inputs, go_env, metadata, records):
    """Resolve every recorded source from actual compiler paths and check its bytes.

    Unknown/missing/traversing paths and changed hashes refuse; no external input
    is silently skipped. Root, Go toolchain, registry and generated Go inputs use
    the collector's exact namespaces. Reads only; no extraction or code execution.
    """
    external={}
    for package in records:
        directory=Path(package.get('Dir','/nonexistent'))
        for field in FIELDS:
            for name in package.get(field,[]):
                path=directory/name
                key='external-go/'+hashlib.sha256(str(path.parent).encode()).hexdigest()+'/'+path.name
                external[key]=path
    registry={p['name']+'-'+p['version']:Path(p['manifest_path']).parent for p in metadata['packages'] if p.get('source')}
    for name, expected in inputs.items():
        parts=Path(name).parts
        if not parts or Path(name).is_absolute() or '..' in parts:
            raise ValueError('source-key-invalid')
        if name.startswith('go-toolchain/'):
            path=Path(go_env['GOROOT']).joinpath(*parts[1:])
        elif name.startswith('registry/'):
            if len(parts)<3 or parts[1] not in registry:raise ValueError('registry-source-unresolved')
            path=registry[parts[1]].joinpath(*parts[2:])
        elif name.startswith('external-go/'):
            if name not in external:raise ValueError('generated-go-source-unresolved')
            path=external[name]
        else:
            path=root/name
        if not path.is_file() or digest(path)!=expected:
            raise ValueError('source-missing-or-changed: '+name)


def verify_archive(evidence, inputs):
    """Check complete retained source bytes and locked crate archives without extraction.

    Tar links must resolve to bytes inside this archive; missing/duplicate entries,
    invalid paths or modified content refuse. Archives are never modified/repaired.
    """
    expected={n:h for n,h in inputs.items() if not n.startswith('registry/')}
    with tarfile.open(evidence/'sources.tar.gz','r:gz') as archive:
        members=archive.getmembers()
        if len(members)!=len(expected) or {m.name for m in members}!=set(expected):
            raise ValueError('source-archive-membership-invalid')
        for member in members:
            if member.name.startswith('/') or '..' in Path(member.name).parts:
                raise ValueError('source-archive-path-invalid')
            raw=archive.extractfile(member)
            if raw is None or hashlib.sha256(raw.read()).hexdigest()!=expected[member.name]:
                raise ValueError('source-archive-content-invalid')
    for package in json.loads((evidence/'registry.json').read_bytes()):
        path=evidence/'registry-crates'/(package['name']+'-'+package['version']+'.crate')
        if digest(path)!=package['crate_sha256']:raise ValueError('registry-archive-changed')


def inventory_command(label, recorded, host):
    """Validate evidence argv as exact read-only protocol data before execution.

    Recorded files cannot authorize arbitrary commands or extra Go/Cargo flags.
    Host is a compiler triple argument, never an executable or shell fragment.
    """
    import re
    if not isinstance(host,str) or not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.-]*',host):
        raise ValueError('compiler-host-invalid')
    packages=['./cmd/swarm','./internal/engine','./tools/rust-migration/work-layout']
    cargo=['--manifest-path','rust/Cargo.toml','--locked','--offline']
    allowed={
        'go-inputs-normal':['go','list','-compiled','-deps','-test','-json',*packages],
        'go-inputs-race':['go','list','-race','-compiled','-deps','-test','-json',*packages],
        'cargo-metadata':['cargo','metadata',*cargo,'--all-features','--format-version','1','--filter-platform',host],
        'production-features':['cargo','tree',*cargo,'-p','swarm-cli','--format','{p} features=[{f}]','--edges','normal'],
    }
    if label not in allowed or recorded!=allowed[label]:
        raise ValueError('recorded-inventory-command-invalid')
    return allowed[label]


def close(evidence, component):
    """Recheck compiler graphs/inputs/profile before publishing component PASS.

    Requires an explicit semantic source-review.json with exact hashes, truthful
    reviewer identity and same-session method review; no human or independent
    approval is inferred from that declaration. Fresh finalization subdirectory only;
    failure retains original runtime evidence and cannot create a PASS. Original
    runtime test counts remain historical, not newly executed by this method.
    """
    baseline_record=json.loads((evidence/'review-baseline.json').read_bytes())
    baseline=local(ROOT,baseline_record['path'])
    if digest(baseline)!=baseline_record['sha256']:raise ValueError('baseline-changed')
    review=json.loads((evidence/'source-review.json').read_bytes())
    methods=review.get('proof_methods',{})
    expected_methods=(Path(__file__).resolve(),Path(__file__).with_name('test_close_qualification.py'),Path(__file__).with_name('QUALIFICATION-CLOSURE.md'))
    if set(methods)!={str(path.relative_to(ROOT))for path in expected_methods}:raise ValueError('proof-method-set-invalid')
    for path in expected_methods:
        name=str(path.relative_to(ROOT))
        if methods.get(name,{}).get('sha256')!=digest(path):raise ValueError('proof-method-not-reviewed')
        axes=methods[name].get('axes',{})
        if set(axes)!={'policies','functions','errors','documentation'} or any(value.get('state')!='reviewed' or not value.get('reason') for value in axes.values()):raise ValueError('proof-method-review-incomplete')
    output=evidence/'finalization01';output.mkdir(mode=0o700)
    env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'rust/target/r25'),CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',RUSTDOCFLAGS='-D warnings')
    def run(label,args):
        """Record exact read-only inventories without timeout/retry or partial success."""
        result=capture(args,cwd=ROOT,env=env,data=None,stdout_path=output/(label+'.stdout'),stderr_path=output/(label+'.stderr'))
        publish(output/(label+'.command.json'),dict(args=list(map(str,args)),exit=result.returncode))
        print(label,result.returncode,flush=True)
        if result.returncode:raise RuntimeError('closure-inventory-failed: '+label)
        return result.stdout
    try:
        publish(output/'method-inputs.json',{name:digest(ROOT/name) for name in methods})
        for path in methods:
            publish(output/('method-'+hashlib.sha256(path.encode()).hexdigest()+'.json'),dict(path=path,content=(ROOT/path).read_text(),sha256=digest(ROOT/path)))
        before=json.loads((evidence/'inputs-before.json').read_bytes())
        if before!=json.loads((evidence/'inputs-after.json').read_bytes()):raise ValueError('runtime-inputs-unstable')
        if build_environment(env)!=json.loads((evidence/'build-environment.json').read_bytes()):raise ValueError('compiler-environment-changed')
        old_env=json.loads((evidence/'go-env.stdout').read_bytes())
        current_env=json.loads(run('go-env',['go','env','-json','GOROOT','GOOS','GOARCH','GOFLAGS','GOWORK']))
        if old_env!=current_env:raise ValueError('go-environment-changed')
        host=next(line.split(': ',1)[1]for line in (evidence/'rust-version.stdout').read_text().splitlines()if line.startswith('host: '))
        records=[]
        for label,flags in [('normal',[]),('race',['-race'])]:
            original=(evidence/('go-inputs-'+label+'.stdout')).read_bytes()
            package_records=list(stream(original));records.extend(package_records)
            expected_command=json.loads((evidence/('go-inputs-'+label+'.command.json')).read_bytes())['args']
            fresh=run('go-inputs-'+label,inventory_command('go-inputs-'+label,expected_command,host))
            if graph(fresh)!=graph(original):raise ValueError('actual-go-graph-changed')
        metadata=json.loads((evidence/'cargo-metadata.stdout').read_bytes())
        fresh_meta=json.loads(run('cargo-metadata',inventory_command('cargo-metadata',json.loads((evidence/'cargo-metadata.command.json').read_bytes())['args'],host)))
        if fresh_meta!=metadata:raise ValueError('actual-cargo-graph-changed')
        if any(str(path.relative_to(ROOT)) not in before for path in local_files(ROOT)):raise ValueError('new-runtime-source-unqualified')
        verify_inputs(ROOT,before,current_env,metadata,records)
        verify_archive(evidence,before)
        required=['format','build','clippy','native-tests','native-inventory','default-cli-tests','default-cli-inventory','rustdoc','runner-tests','review-guards','go-budgets','go-vet','methods','diff','production-profile-build','go-profile-build','production-features','fixture-profile-build']
        for label in required:
            record=json.loads((evidence/(label+'.command.json')).read_bytes())
            if record['exit']!=0:raise ValueError('runtime-check-incomplete: '+label)
        counts={}
        for prefix in ['native','default-cli']:
            executed=[line[5:-7] for line in (evidence/(prefix+'-tests.stdout')).read_text().splitlines() if line.startswith('test ') and line.endswith(' ... ok')]
            counts[prefix]=len(executed)
            if not executed or sorted(executed)!=sorted(json.loads((evidence/(prefix+'-test-names.json')).read_bytes())):raise ValueError('test-inventory-changed')
        go_names=go_test_inventory((evidence/'go-budgets.stdout').read_text(),(evidence/'go-test-inventory.stdout').read_text())
        if go_names!=json.loads((evidence/'go-test-names.json').read_bytes()):raise ValueError('go-inventory-changed')
        parity_directory=retained_parity(ROOT,json.loads((evidence/'parity-reference.json').read_bytes()))
        parity=json.loads((parity_directory/'result.json').read_bytes())
        if parity['status']!='PASS' or parity['count']!=len(parity['checks']):raise ValueError('parity-incomplete')
        for name,sha in parity['binaries'].items():
            if Path(name).name!=name or digest(parity_directory/name)!=sha:raise ValueError('original-parity-binary-changed')
        if digest(evidence/'go-planning')!=parity['binaries']['go-planning']:raise ValueError('go-profile-changed')
        if digest(ROOT/'rust/target/r25/debug/swarm-rust')!=parity['binaries']['native-production-cli']:raise ValueError('production-profile-changed')
        fixture={'pricing':'pricing_probe','financial':'financial_budget_probe','rejection':'planning_rejection_probe','budgets':'planning_budget_probe'}[component]
        key={'pricing':'native-pricing-fixture','financial':'native-financial-fixture','rejection':'native-rejection-fixture','budgets':'native-budget-fixture'}[component]
        if digest(ROOT/('rust/target/r25/debug/examples/'+fixture))!=parity['binaries'][key]:raise ValueError('fixture-profile-changed')
        tree=run('production-features',inventory_command('production-features',json.loads((evidence/'production-features.command.json').read_bytes())['args'],host))
        if b'test-fixtures' in tree or tree!=(evidence/'production-features.stdout').read_bytes():raise ValueError('production-features-changed')
        require_review(ROOT,evidence,baseline)
        verify_inputs(ROOT,before,current_env,metadata,records)
        pending=json.loads((evidence/'pending-review.json').read_bytes())
        if pending['status']!='REVIEW_REQUIRED' or pending['native_tests']!=counts['native'] or pending['default_cli_tests']!=counts['default-cli'] or pending['inputs']!=len(before) or pending['registry_packages']!=len(json.loads((evidence/'registry.json').read_bytes())):raise ValueError('runtime-summary-incoherent')
        result=dict(component_status='PASS',status='PARTIAL',component=component,pricing_catalogue_cli_ported=component=='pricing',native_tests=pending['native_tests'],default_cli_tests=pending['default_cli_tests'],go_regression_tests=len(go_names),go_native_comparisons=parity['count'],inputs=len(before),registry_packages=pending['registry_packages'],source_stable=True,runtime_tests_reexecuted=False,proof_method_phase='separate hash-bound closure',production_cli_fixture_features=False,financial_reservation_api_verified=component=='financial',budget_configuration_ported=component=='financial',claim_financial_integration=False,budget_views_ported=False,claim_ported=False,identity=False,deployment=False,independent_review=False,global_watchdog=None)
        publish(evidence/'result.json',result);publish(output/'result.json',result);print('PASS component closure; PARTIAL migration',flush=True)
    except BaseException as error:
        publish(output/'failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__));raise


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--evidence',type=Path,required=True);parser.add_argument('--component',choices=['budgets','rejection','financial','pricing'],required=True);args=parser.parse_args();close(args.evidence.resolve(),args.component)
