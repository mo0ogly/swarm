"""Generate the actual PlanningRequest wire contract, never runtime orchestration.

Input is work-layout's explicit PlanningRequest closure and producer hashes.
Only JSON-visible fields are generated; custom dependency marshaling is checked
against its actual Go source. Output is atomically replaced inside this repo.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
SHARED = {
    'ExchangeArtifact': [('path', 'string'), ('sha256', 'string')],
    'ValidationControl': [('environment_exit_codes,omitempty', '[]int'),
                          ('review_output,omitempty', 'bool'), ('inputs,omitempty', '[]string'),
                          ('id', 'string'), ('command', '[]string'), ('criteria', '[]int'),
                          ('justification', 'string'), ('dir,omitempty', 'string'),
                          ('timeout_seconds,omitempty', 'int')],
}


def snake(name):
    """Map checked exported Go identifiers to generated private shape symbols."""
    return re.sub(r'(?<!^)(?=[A-Z])', '_', name).lower()


def generate(layout):
    """Validate actual sources and emit DTO/shape/custom-serializer Rust bytes.

    Refuses unsupported roots, stale producers, changed shared shapes or custom
    marshaling. Data conversion only: no Work, identity, policy or action applies.
    """
    assert layout['roots'] == ['PlanningRequest'] and layout['external'] == []
    types = layout['types']
    assert set(types) == {'PlanningRequest', 'PlanningOperation', 'ManagedRepositoryRequest', *SHARED}
    for name, description in types.items():
        assert not description.get('unsupported') and not description.get('underlying')
        source = (ROOT / description['source']).resolve()
        assert source.is_relative_to(ROOT / 'internal/engine')
        assert hashlib.sha256(source.read_bytes()).hexdigest() == description['source_sha256']
        if name in SHARED:
            assert [(f['json_tag'], f['go_type']) for f in description['fields']] == SHARED[name], 'shared model changed: ' + name
    assert set(layout['custom_methods']) == {'PlanningOperation'}
    custom, = layout['custom_methods']['PlanningOperation']
    assert custom['method'] == 'MarshalJSON'
    assert custom['source'] == 'internal/engine/planning_dependency_revision.go'
    assert hashlib.sha256((ROOT / custom['source']).read_bytes()).hexdigest() == custom['source_sha256']
    assert 'op.Kind == "revise-dependencies" && op.Depends != nil && len(op.Depends) == 0' in custom['code']
    assert 'raw = append(raw[:len(raw)-1], []byte(`,"depends":[]}`)...)' in custom['code']

    def representation(go):
        """Map this closed Go DTO vocabulary; unknown types refuse generation."""
        if go == 'string': return 'String', 'Kind::String', 'String::is_empty'
        if go == 'int': return 'i64', 'Kind::Int64', 'is_zero'
        if go == 'bool': return 'bool', 'Kind::Bool', 'is_false'
        if go.startswith('[]'):
            rust, kind, _ = representation(go[2:])
            return 'Option<Vec<' + rust + '>>', 'Kind::Array(&' + kind + ')', 'empty_vec'
        if go.startswith('map[string]'):
            rust, kind, _ = representation(go[11:])
            return 'Option<BTreeMap<String,' + rust + '>>', 'Kind::Map(&' + kind + ')', 'empty_map'
        if go.startswith('*'):
            rust, kind, _ = representation(go[1:])
            return 'Option<Box<' + rust + '>>', 'Kind::Pointer(&' + kind + ')', 'Option::is_none'
        assert go in types, 'unsupported DTO type: ' + go
        function = ('crate::models::' if go in SHARED else '') + 'shape_' + snake(go)
        return go, 'Kind::Object(' + function + ')', None

    parts = [
        '//! Generated PlanningRequest data; no action, identity or authority is established.\n',
        'use serde::{Serialize, Deserialize};\nuse serde::ser::SerializeMap;\n',
        'use std::collections::BTreeMap;\nuse crate::{ExchangeArtifact, ValidationControl};\n',
        'use crate::model_wire::{Fields, Kind};\n',
        'fn is_zero(v:&i64)->bool{*v==0}\nfn is_false(v:&bool)->bool{!*v}\n',
        'fn empty_vec<T>(v:&Option<Vec<T>>)->bool{v.as_ref().is_none_or(Vec::is_empty)}\n',
        'fn empty_map<K,V>(v:&Option<BTreeMap<K,V>>)->bool{v.as_ref().is_none_or(BTreeMap::is_empty)}\n',
    ]
    for name in ['ManagedRepositoryRequest', 'PlanningOperation', 'PlanningRequest']:
        description = types[name]
        parts += ['// ' + description['source'] + ' SHA-256 ' + description['source_sha256'] + '\n',
                  '/// Supplied Go-compatible request data; decoding grants no authority or action.\n',
                  '#[derive(Clone, Default, PartialEq, Deserialize' + (', Serialize' if name != 'PlanningOperation' else '') + ')]\n',
                  '#[serde(default, deny_unknown_fields)]\npub struct ' + name + ' {\n']
        shapes = []
        names = set()
        for field in description['fields']:
            key, *flags = field['json_tag'].split(',')
            assert re.fullmatch(r'[a-z_][a-z0-9_]*', key) and key not in names and flags in [[], ['omitempty']]
            names.add(key)
            rust, kind, skip = representation(field['go_type'])
            parts += ['// Go ' + field['go_name'] + ', line ' + str(field['line']) + '\n']
            if flags:
                assert skip
                parts += ['#[serde(skip_serializing_if=' + json.dumps(skip) + ')]\n']
            parts += ['pub ' + key + ': ' + rust + ',\n']
            shapes.append('(' + json.dumps(key) + ',' + kind + ')')
        parts += ['}\n', 'pub(crate) fn shape_' + snake(name) + '()->Fields{&[' + ','.join(shapes) + ']}\n']
    parts += ['// Actual PlanningOperation.MarshalJSON producer SHA-256 ' + custom['source_sha256'] + '\n',
              'impl Serialize for PlanningOperation {\n',
              '/// Encode the actual Go dependency-revision wire contract: an explicit empty\n/// depends list is retained only for revise-dependencies. Serializer failures\n/// propagate; no IO, Work edits, operation or authority is established.\nfn serialize<S:serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error>{\nlet mut out=s.serialize_map(None)?;\n']
    for field in types['PlanningOperation']['fields']:
        key, *flags = field['json_tag'].split(',')
        _, _, skip = representation(field['go_type'])
        call = 'out.serialize_entry(' + json.dumps(key) + ',&self.' + key + ')?;'
        parts += [('if !' + skip + '(&self.' + key + '){' + call + '}' if flags else call) + '\n']
    parts += ['if self.kind=="revise-dependencies"&&self.depends.as_ref().is_some_and(Vec::is_empty){out.serialize_entry("depends",&self.depends)?;}\n',
              'out.end()\n}\n}\n']
    return ''.join(parts).encode()


def main():
    """Read layout/output args, atomically publish verified source or retain original.

    IO/JSON/contract failures propagate; output must stay inside this repository.
    No database, provider, native process or operation is started.
    """
    if len(sys.argv) != 3:
        raise ValueError('actual PlanningRequest layout and repository output required')
    output = Path(sys.argv[2]).resolve()
    assert output.is_relative_to(ROOT)
    content = generate(json.loads(Path(sys.argv[1]).read_bytes()))
    output.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(dir=output.parent, prefix='.planning-model-')
    temporary = Path(name)
    try:
        with os.fdopen(fd, 'wb') as stream:
            os.fchmod(stream.fileno(), output.stat().st_mode & 0o777 if output.exists() else 0o644)
            stream.write(content); stream.flush(); os.fsync(stream.fileno())
        assert temporary.read_bytes() == content
        os.replace(temporary, output)
    finally:
        temporary.unlink(missing_ok=True)
    print('Generated PlanningRequest/PlanningOperation/ManagedRepositoryRequest, shared models checked')


if __name__ == '__main__':
    main()
