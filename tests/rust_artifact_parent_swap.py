#!/usr/bin/env python3
"""Compile actual artifact modules with bound cached dependencies; no Root execution.
A qualified profile resolves dependency identities/paths only. Cached library bytes
are independently hashed/stability-checked, not claimed to be that frozen binary.
No source overlay, native installation, acceptance, provider or global watchdog.
"""
import argparse,json,subprocess,tomllib,traceback
from pathlib import Path
from rust_agent_model_route import publish,digest

def run(profile,out,before):
    """Bind complete actual sources, dependencies/compiler and named assertions.
    Before mode expects the reproduced defect, not a passing security contract.
    Controlled private markers belong solely to a fresh inactive filesystem fixture.
    """
    out.mkdir(parents=True,exist_ok=False);publish(out/'method.py',Path(__file__).read_bytes());assert json.loads(profile.read_bytes())['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE';records=[json.loads(l) for l in (profile.parent/'cargo.stdout').read_text().splitlines() if l.startswith('{')];spec=tomllib.loads(Path('rust/crates/swarm-engine/Cargo.toml').read_text());libs={};identities={}
    for name in ['swarm_engine','serde','serde_json','sha2','rustix','nix','swarm_wire_json']:
        rows=[r for r in records if r.get('reason')=='compiler-artifact' and r['target']['name']==name]
        if name!='swarm_engine' and name.replace('_','-') in spec['dependencies']:
            dep=spec['dependencies'][name.replace('_','-')];version=dep if isinstance(dep,str) else dep.get('version');
            if version:rows=[r for r in rows if r['package_id'].endswith('@'+version.lstrip('='))]
        assert len(rows)==1,(name,len(rows));libs[name]=Path(next(v for v in rows[0]['filenames'] if v.endswith('.rlib')));identities[name]=rows[0]['package_id']
    modules=['text','model_wire','gate_json','gate_evaluation'];anchor=Path('rust/crates/swarm-engine/src/anchored_artifact.rs');modules+=['anchored_artifact'] if anchor.exists() else [];sources=['rust/crates/swarm-engine/src/'+n+'.rs' for n in modules]+['tests/rust_artifact_parent_swap.rs'];hashes={n:digest(Path(n).read_bytes()) for n in sources};library_hashes={n:digest(v.read_bytes()) for n,v in libs.items()};wrapper='pub use swarm_engine::*;\n'+''.join('#[path="'+str(Path(n).resolve())+'"] mod '+Path(n).stem+';\n' for n in sources[:-1])+'include!("'+str(Path(sources[-1]).resolve())+'");\n';publish(out/'wrapper.rs',wrapper.encode())
    for i,n in enumerate(sources):publish(out/('source-'+str(i)+'.rs'),Path(n).read_bytes())
    argv=['rustc','--edition','2021',str(out/'wrapper.rs'),'-L','dependency='+str(libs['serde'].parent),'-o',str(out/'reproducer')]
    for name,lib in libs.items():argv+=['--extern',name+'='+str(lib)]
    compiler=subprocess.run(['rustc','--version','--verbose'],capture_output=True);assert compiler.returncode==0;publish(out/'compiler.stdout',compiler.stdout);publish(out/'compiler.stderr',compiler.stderr);publish(out/'inputs.json',dict(argv=argv,source_hashes=hashes,library_hashes=library_hashes,dependency_identities=identities,profile_reference=str(profile),global_watchdog=None,root_wrapper=False,production_qualification=False))
    r=subprocess.run(argv,capture_output=True);publish(out/'compile.stdout',r.stdout);publish(out/'compile.stderr',r.stderr);assert r.returncode==0,('component compilation failed',r.returncode)
    r=subprocess.run([str(out/'reproducer'),str(out/'fixture')],capture_output=True);publish(out/'assertions.stdout',r.stdout);publish(out/'assertions.stderr',r.stderr);assert b'PASS actual-reader-valid-control' in r.stdout and b'PASS actual-reader-recovery' in r.stdout
    if before:assert r.returncode!=0 and b'OBSERVED private-fixture-read-after-parent-swap' in r.stdout
    else:assert r.returncode==0 and b'PASS actual-reader-parent-swap-refusal' in r.stdout
    assert hashes=={n:digest(Path(n).read_bytes()) for n in sources};assert library_hashes=={n:digest(v.read_bytes()) for n,v in libs.items()};publish(out/'result.json',dict(status='REPRODUCED_DEFECT' if before else 'PASS',source_hashes=hashes,assertions=2 if before else 3,exit_code=r.returncode,root_wrapper=False,production_qualification=False));print('ARTIFACT_PARENT_SWAP_REPRODUCED' if before else 'ARTIFACT_PARENT_SWAP_VERIFIED 3',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--profile-result',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--expect-defect',action='store_true');a=p.parse_args()
    try:run(a.profile_result.resolve(),a.output.resolve(),a.expect_defect)
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
