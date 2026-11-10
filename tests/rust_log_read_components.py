#!/usr/bin/env python3
"""Named tests of actual complete text/live-reader modules without Root execution.

A qualified profile supplies resolved dependency identities/paths. Actual cached
rlib bytes and current module inputs are hashed before/after; this test binary is
separate from production, and establishes neither Root wrappers nor installation.
"""
import argparse,json,subprocess,tomllib,traceback,shutil
from pathlib import Path
from rust_agent_model_route import publish,digest

def run(profile,output):
    """Retain real argv, compiler, named results and stable source/library hashes.
    No source overlay, model, live datastore, implicit retry or global watchdog.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'method.py',Path(__file__).read_bytes());v=json.loads(profile.read_bytes());assert v['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE';records=[json.loads(l) for l in (profile.parent/'cargo.stdout').read_text().splitlines() if l.startswith('{')];manifest=Path('rust/crates/swarm-engine/Cargo.toml');spec=tomllib.loads(manifest.read_text());libs={};identity={}
    for name in ['swarm_engine','rusqlite','serde','serde_json','sha2']:
        selected=[r for r in records if r.get('reason')=='compiler-artifact' and r['target']['name']==name]
        if name!='swarm_engine':
            dependency=spec['dependencies'][name];version=(dependency if isinstance(dependency,str) else dependency['version']);assert version.startswith('=');selected=[r for r in selected if r['package_id'].endswith('#'+name+'@'+version[1:])]
        assert len(selected)==1,(name,len(selected));libs[name]=Path(next(s for s in selected[0]['filenames'] if s.endswith('.rlib')));identity[name]=selected[0]['package_id']
    sources=['rust/crates/swarm-engine/src/text.rs','rust/crates/swarm-engine/src/agent_live_read.rs'];hashes={n:digest(Path(n).read_bytes()) for n in sources};lib_hashes={n:digest(p.read_bytes()) for n,p in libs.items()};publish(output/'source-text.rs',Path(sources[0]).read_bytes());publish(output/'source-live.rs',Path(sources[1]).read_bytes());wrapper='pub use swarm_engine::AgentStoreError;'+chr(10)+''.join('#[path="'+str(Path(n).resolve())+'"] mod '+Path(n).stem+';'+chr(10) for n in sources);publish(output/'wrapper.rs',wrapper.encode());argv=['rustc','--edition','2021','--test',str(output/'wrapper.rs'),'-L','dependency='+str(libs['serde'].parent),'-o',str(output/'tests')]
    for name,lib in libs.items():argv+=['--extern',name+'='+str(lib)]
    compiler=subprocess.run(['rustc','--version','--verbose'],capture_output=True);publish(output/'compiler.stdout',compiler.stdout);publish(output/'compiler.stderr',compiler.stderr);assert compiler.returncode==0
    publish(output/'inputs.json',dict(argv=argv,source_hashes=hashes,manifest_sha256=digest(manifest.read_bytes()),library_hashes=lib_hashes,dependency_identities=identity,profile_resolution_reference=str(profile),profile_reference_sha256=digest(profile.read_bytes()),scope='Actual cfg(test) free-reader/text modules; current stable cached library bytes, not a new production qualification or Root wrapper proof.',global_watchdog=None))
    r=subprocess.run(argv,capture_output=True);publish(output/'compile.stdout',r.stdout);publish(output/'compile.stderr',r.stderr);assert r.returncode==0,('Rust component compile failed',r.returncode)
    r=subprocess.run([str((output/'tests').resolve()),'--nocapture'],capture_output=True);publish(output/'tests.stdout',r.stdout);publish(output/'tests.stderr',r.stderr);names=[line for line in r.stdout.decode().splitlines() if line.startswith('test ') and line.endswith(' ... ok')];assert r.returncode==0 and len(names)==10 and b'10 passed; 0 failed' in r.stdout
    assert hashes=={n:digest(Path(n).read_bytes()) for n in sources};assert lib_hashes=={n:digest(p.read_bytes()) for n,p in libs.items()};publish(output/'result.json',dict(status='PASS',named_tests=names,count=10,source_hashes=hashes,test_binary_sha256=digest((output/'tests').read_bytes()),root_wrapper=False,production_profile=False));print('LOG_TEXT_LIVE_COMPONENTS_VERIFIED 10',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--profile-result',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.profile_result.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
