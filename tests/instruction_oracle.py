"""Build and retain the extracted Go oracle's real source and embed closure.

Only owned fixture output is written. No providers, runtime SQL, overlays, retry
or watchdog. Evidence binds local compilation inputs, not caller identity or full
Go product parity. Partial evidence remains unqualified after any error.
"""
import hashlib,json,os,shutil,subprocess,tarfile
from pathlib import Path
import rust_agent_model_route as f

FIELDS=('GoFiles','CompiledGoFiles','CgoFiles','CFiles','CXXFiles','HFiles','SFiles','SysoFiles','EmbedFiles')


def execute(folder,label,args):
    """Run explicit argv and retain both channels; failure stops dependants."""
    result=subprocess.run(args,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    f.publish(folder/(label+'.stdout'),result.stdout);f.publish(folder/(label+'.stderr'),result.stderr)
    assert result.returncode==0,(label,result.stderr)
    return result.stdout


def inventory(folder,label,package):
    """Hash all actual normal compiler sources and embeds, including root resources.

    Generated/cached Go sources are included by -compiled. Missing source is an
    error. Paths identify local proof inputs only; no pathname grants authority.
    """
    raw=execute(folder,label,['go','list','-deps','-compiled','-json',package]);remaining=raw.decode().lstrip();decoder=json.JSONDecoder();paths=set()
    while remaining:
        value,end=decoder.raw_decode(remaining);remaining=remaining[end:].lstrip()
        paths.update((Path(value['Dir'])/name).resolve() for field in FIELDS for name in value.get(field,[]))
    paths.update([Path('go.mod').resolve(),Path('go.sum').resolve()]);assert paths
    return {str(path):f.digest(path.read_bytes()) for path in sorted(paths)}


def build(folder):
    """Freeze extractor then generated reference inputs before each compilation.

    Refuse overlays/workspaces requiring a dedicated collector. The caller must
    run verify() after the actual differential journey before claiming stability.
    Fresh archive contents are read back; earlier failures are never overwritten.
    """
    folder=Path(folder);folder.mkdir();(folder/'oracle').mkdir()
    keys=['GOROOT','GOTOOLDIR','GOOS','GOARCH','GOFLAGS','GOWORK','CGO_ENABLED','CC','CXX','CGO_CFLAGS','CGO_CPPFLAGS','CGO_CXXFLAGS','CGO_LDFLAGS','GOEXPERIMENT']
    env=json.loads(execute(folder,'go-environment',['go','env','-json',*keys]));assert not env['GOWORK'] and 'overlay' not in env['GOFLAGS'],'overlay/workspace requires explicit capture'
    execute(folder,'go-version',['go','version'])
    tool_paths=[Path(shutil.which('go')).resolve(),*(Path(env['GOTOOLDIR'])/name for name in ['compile','link','cgo'])]
    tools={str(path):f.digest(path.read_bytes()) for path in tool_paths}
    before=inventory(folder,'extractor-before','./tools/rust-migration/instruction-learning-reference')
    execute(folder,'extractor-build',['go','build','-o',str(folder/'extractor'),'./tools/rust-migration/instruction-learning-reference'])
    assert inventory(folder,'extractor-after','./tools/rust-migration/instruction-learning-reference')==before
    execute(folder,'extract-source',[str(folder/'extractor'),str(folder/'oracle/main.go')])
    reference=inventory(folder,'reference-before',str(folder/'oracle/main.go'))
    for path,digest in before.items():assert path not in reference or reference[path]==digest
    inputs={**before,**reference,**tools};f.publish(folder/'source-inputs.json',inputs)
    archive=folder/'source-inputs.tar.gz'
    with tarfile.open(archive,'x:gz',compresslevel=1) as tar:
        for index,(path,digest) in enumerate(sorted(inputs.items())):tar.add(path,arcname=str(index),recursive=False)
    with tarfile.open(archive,'r:gz') as tar:
        for index,(path,digest) in enumerate(sorted(inputs.items())):
            member=tar.extractfile(str(index));assert member is not None and f.digest(member.read())==digest
    execute(folder,'reference-build',['go','build','-o',str(folder/'go-reference'),str(folder/'oracle/main.go')])
    f.publish(folder/'build.json',dict(binary=str(folder/'go-reference'),sha256=f.digest((folder/'go-reference').read_bytes()),archive_sha256=f.digest(archive.read_bytes()),inputs=len(inputs),root_embeds_included='resources.go' in ' '.join(inputs)))
    return folder/'go-reference'


def verify(folder):
    """Require the original compiler/embedded bytes after the differential journey."""
    folder=Path(folder);before=json.loads((folder/'source-inputs.json').read_bytes())
    assert all(f.digest(Path(path).read_bytes())==digest for path,digest in before.items())
    # New membership cannot be hidden by rechecking only old files.
    after=inventory(folder,'reference-after',str(folder/'oracle/main.go'))
    assert all(before.get(path)==digest for path,digest in after.items())
    f.publish(folder/'source-stability.json',dict(status='PASS',inputs=len(before),archive_sha256=f.digest((folder/'source-inputs.tar.gz').read_bytes()),global_watchdog=None))
    print('PASS Go-oracle-actual-source-and-embed-stability',flush=True)
