"""Generate the frozen Bundle validation graph, without reading legacy Go.

The graph validates known unused Bundle fields as Go json.Unmarshal does before
projecting cockpit history. It is data compatibility, never archive import,
authenticated identity or runtime admission. Current Work/Agent codecs can evolve
independently; this historical graph excludes their native-only extensions.
"""
from pathlib import Path
import json,re,os,tempfile,argparse
ROOT=Path(__file__).resolve().parents[2]
CONTRACT=ROOT/'contracts/archive-history-wire.v1.json'
def load_layout():
    """Read the closed version1 schema; malformed/provenance inputs refuse.

    No process, Go source, Root or provider is opened. IO/JSON errors propagate.
    Provenance binds retained compatibility bytes, not source authenticity.
    """
    value=json.loads(CONTRACT.read_bytes())
    if set(value)!={'schema_version','provenance','layout'} or value['schema_version']!=1:raise ValueError('archive-layout-envelope')
    return value['layout']
def snake(name):
    """Translate a checked Go identifier into this private Rust function name."""
    if not re.fullmatch('[A-Za-z][A-Za-z0-9]*',name):raise ValueError('archive-layout-type-name')
    return re.sub(r'(?<!^)(?=[A-Z])','_',name).lower()
def generate(layout):
    """Validate the closed type vocabulary and emit deterministic pure shapes.

    All nested declarations must exist, unsupported/custom/external types refuse.
    Serialization tags retain Go names but omitempty does not weaken input type
    validation. No files or runtime state change; failures return no partial code.
    """
    if layout['roots']!=['Bundle','CockpitHistory'] or layout['external']!=['json.RawMessage'] or layout['custom_methods']:raise ValueError('archive-layout-roots-or-methods')
    types=layout['types'];names=[snake(n) for n in types]
    if len(names)!=len(set(names)):raise ValueError('archive-layout-name-collision')
    def kind(go):
        """Map one closed Go JSON type; integers are the Linux i64 wire range."""
        if go in {'string','bool','float64','int','int64','json.RawMessage'}:return 'Kind::'+{'string':'String','bool':'Bool','float64':'Float64','int':'Int64','int64':'Int64','json.RawMessage':'Raw'}[go]
        for prefix,label in [('*','Pointer'),('[]','Array'),('map[string]','Map')]:
            if go.startswith(prefix):return 'Kind::'+label+'(&'+kind(go[len(prefix):])+')'
        if go not in types:raise ValueError('archive-layout-unknown-type:'+go)
        return 'Kind::Object(shape_'+snake(go)+')'
    code=['//! Frozen legacy Bundle JSON type graph; no import or runtime authority.\n','//! Known unused fields are validated too; unknown fields follow Go decoding.\n','use crate::model_wire::{Fields,Kind};\n']
    for name,t in sorted(types.items()):
        if t.get('unsupported') or t.get('underlying') or not re.fullmatch('[0-9a-f]{64}',t['source_sha256']):raise ValueError('archive-layout-definition')
        if not t['source'].startswith('internal/engine/') or '..' in Path(t['source']).parts:raise ValueError('archive-layout-source')
        fields=[];seen=set()
        for f in t['fields']:
            key,*flags=f['json_tag'].split(',')
            if not re.fullmatch('[a-z_][a-z0-9_]*',key) or flags not in [[],['omitempty']] or key in seen:raise ValueError('archive-layout-field')
            seen.add(key);fields.append('('+json.dumps(key)+','+kind(f['go_type'])+')')
        code.extend(['// '+t['source']+' SHA-256 '+t['source_sha256']+'\n','/// Frozen '+name+' field types; pure metadata, no IO or grant.\n','pub(super) fn shape_'+snake(name)+'()->Fields {&['+','.join(fields)+']}\n'])
    return ''.join(code).encode()
def main():
    """Publish one owned output atomically after complete validation/fsync.

    A failure before replace preserves the old source. A later directory sync
    failure means publication is uncertain, not rolled back. No runtime/provider
    is touched. Output stays inside this repository; both IO/cleanup causes survive.
    """
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);a=parser.parse_args();out=a.output.resolve()
    if not out.is_relative_to(ROOT):raise ValueError('archive-output-outside-repository')
    layout=load_layout();raw=generate(layout);fd,name=tempfile.mkstemp(dir=out.parent,prefix='.archive-shapes-');tmp=Path(name);primary=None;cleanup=None
    try:
        with os.fdopen(fd,'wb') as f:f.write(raw);f.flush();os.fsync(f.fileno())
        if tmp.read_bytes()!=raw:raise OSError('archive-shapes-readback')
        os.chmod(tmp,0o644);os.replace(tmp,out)
        directory=os.open(out.parent,os.O_RDONLY|os.O_DIRECTORY)
        try:os.fsync(directory)
        finally:os.close(directory)
    except BaseException as error:primary=error
    try:
        if tmp.exists():tmp.unlink()
    except BaseException as error:cleanup=error
    if primary is not None and cleanup is not None:raise BaseExceptionGroup('Archive shape publication and cleanup failed',[primary,cleanup])
    if primary is not None:raise primary
    if cleanup is not None:raise cleanup
    print('GENERATED_ARCHIVE_SHAPES',len(layout['types']))
if __name__=='__main__':main()
