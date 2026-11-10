"""Retain a standalone historical Go data-only memory guard reference.

The real guard body consumes fixture Rows of already-decoded Works; this proves
neither SQL/envelope parity, accepted RETEX, rights nor actual Go agent launch.
Compiler/tool inputs are retained without a watchdog or automatic retry.
"""
import json, shutil, tarfile
from pathlib import Path
import instruction_oracle as common
import rust_agent_model_route as f

SOURCE='tools/rust-migration/memory-files-reference/main.go'


def build_projection(folder):
    """Build the existing pure SQL probe in a separate fixture-only target.

    This binary opens only its own in-memory database. It never replaces the
    frozen normal-profile product. Retain exact engine inputs, Cargo feature
    declarations, argv, build outputs and the exercised probe's digest.
    """
    folder=Path(folder);folder.mkdir(parents=True,exist_ok=False)
    paths=sorted(Path('rust/crates/swarm-engine').rglob('*.rs'))+[Path('rust/Cargo.lock'),Path('rust/crates/swarm-engine/Cargo.toml')]
    before={str(path):f.digest(path.read_bytes()) for path in paths}
    f.publish(folder/'inputs-before.json',before)
    target=folder/'target'
    args=['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example','identity_plan_probe','--target-dir',str(target),'--message-format=json']
    raw=common.execute(folder,'build',args)
    records=[json.loads(line) for line in raw.splitlines() if line.strip()]
    engines=[r for r in records if r.get('reason')=='compiler-artifact' and r['target']['name']=='swarm_engine']
    assert engines and all('test-fixtures' in r['features'] for r in engines)
    artifacts=[r for r in records if r.get('reason')=='compiler-artifact' and r['target']['name']=='identity_plan_probe' and r.get('executable')]
    assert len(artifacts)==1
    source=Path(artifacts[0]['executable']);binary=folder/'projection-probe';shutil.copyfile(source,binary);binary.chmod(0o700)
    assert f.digest(binary.read_bytes())==f.digest(source.read_bytes())
    assert all(f.digest(Path(path).read_bytes())==digest for path,digest in before.items())
    f.publish(folder/'result.json',dict(status='PASS',fixture_only=True,production_replaced=False,binary=str(binary),sha256=f.digest(binary.read_bytes()),command=args,engine_features=[r['features'] for r in engines],global_watchdog=None))
    return binary


def build(folder):
    """Build a fresh normal Go reference and retain its actual input closure."""
    folder=Path(folder);folder.mkdir(parents=True,exist_ok=False)
    common.execute(folder,'version',['go','version'])
    env=json.loads(common.execute(folder,'environment',['go','env','-json','GOTOOLDIR','GOFLAGS','GOWORK','GOOS','GOARCH']))
    assert not env['GOWORK'] and 'overlay' not in env['GOFLAGS']
    before=common.inventory(folder,'inputs-before',SOURCE)
    tools=[Path(shutil.which('go')).resolve(),Path(env['GOTOOLDIR'])/'compile',Path(env['GOTOOLDIR'])/'link']
    inputs={**before,**{str(path):f.digest(path.read_bytes()) for path in tools}}
    f.publish(folder/'source-inputs.json',inputs)
    with tarfile.open(folder/'source-inputs.tar.gz','x:gz',compresslevel=1) as archive:
        for i,path in enumerate(sorted(inputs)):archive.add(path,arcname=str(i),recursive=False)
    with tarfile.open(folder/'source-inputs.tar.gz','r:gz') as archive:
        for i,path in enumerate(sorted(inputs)):assert f.digest(archive.extractfile(str(i)).read())==inputs[path]
    binary=folder/'reference'
    common.execute(folder,'build',['go','build','-o',str(binary),SOURCE])
    assert common.inventory(folder,'inputs-after',SOURCE)==before
    f.publish(folder/'build.json',dict(status='PASS',binary=str(binary),sha256=f.digest(binary.read_bytes()),inputs=len(inputs),global_watchdog=None,scope='data-only historical Go legacy path guard, no SQL/provider'))
    return binary


def verify(folder):
    """Recheck exact original bytes and compiler membership after the journey."""
    folder=Path(folder);before=json.loads((folder/'source-inputs.json').read_bytes())
    assert all(f.digest(Path(path).read_bytes())==digest for path,digest in before.items())
    after=common.inventory(folder,'inputs-final',SOURCE)
    assert all(before.get(path)==digest for path,digest in after.items())
    f.publish(folder/'verified.json',dict(status='VERIFIED',inputs=len(before)))
    print('PASS memory-Go-reference-input-stability',flush=True)
