"""Generate the actual Agent/Launch/Providers closure, reusing checked Work types.

Developer tool only. Historical hashes bind the versioned wire contract; shared Rust fields must
match current native declarations. No Go source/tool, provider, process or
database opens. Historical provenance is not authenticity.
"""
import json
import os
from pathlib import Path
import re
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / 'contracts/agent-wire-layout.v1.json'


def load_layout():
    """Read versioned compatibility input without a legacy source fallback.

    Missing/malformed contracts propagate original IO/JSON errors. Provenance
    identifies historical evidence, not current authority. No Go/process IO.
    """
    value = json.loads(CONTRACT.read_bytes())
    if (set(value) != {'schema_version', 'provenance', 'layout'}
            or value['schema_version'] != 1
            or not isinstance(value['layout'], dict)):
        raise ValueError('unsupported-agent-wire-contract')
    return value['layout']


def snake(name):
    """Map inventoried Go type names to the existing native shape vocabulary."""
    return re.sub(r'(?<!^)(?=[A-Z])', '_', name).lower()


def generate(layout):
    """Emit DTOs after checking frozen provenance and current shared Rust fields.

    Unknown/custom/external vocabulary, provenance mismatch or shared layout drift
    refuse before publication. Public JSON carries no identity or launch rights.
    Agent's non-JSON unregistered-claim observation is reconstructed by row reads.
    Its optional native guard snapshot stays opaque/required instead of using legacy
    zero/default projection. Current provider declarations stay opaque, additive
    data, with no invoice/model/actor authentication. A future contract wire-name
    collision with any native extension refuses generation.
    """
    if (layout['roots'] != ['Agent', 'Launch', 'Providers']
            or layout['external'] or layout['custom_methods']):
        raise ValueError('unsupported-agent-layout')
    types = layout['types']
    historical = load_layout()['types']
    source = (ROOT / 'rust/crates/swarm-engine/src/models.rs').read_text()
    shared = set(re.findall(r'pub struct (\w+)', source)) & types.keys()

    def representation(go):
        """Translate this closed Go data vocabulary; unknown types fail closed."""
        primitive = {
            'string': ('String', 'Kind::String', 'String::is_empty'),
            'int': ('i64', 'Kind::Int64', 'is_zero'),
            'int64': ('i64', 'Kind::Int64', 'is_zero'),
            'bool': ('bool', 'Kind::Bool', 'is_false'),
            'float64': ('crate::GoFloat', 'Kind::Float64', 'crate::GoFloat::is_zero'),
        }
        if go in primitive:
            return primitive[go]
        if go.startswith('*'):
            rust, kind, _ = representation(go[1:])
            return f'Option<Box<{rust}>>', f'Kind::Pointer(&{kind})', 'Option::is_none'
        if go.startswith('[]'):
            rust, kind, _ = representation(go[2:])
            return f'Option<Vec<{rust}>>', f'Kind::Array(&{kind})', 'empty_vec'
        if go.startswith('map[string]'):
            rust, kind, _ = representation(go[11:])
            return f'Option<std::collections::BTreeMap<String,{rust}>>', f'Kind::Map(&{kind})', 'empty_map'
        if go not in types:
            raise ValueError('unsupported-agent-type: ' + go)
        prefix = 'crate::models::' if go in shared else ''
        return prefix + go, f'Kind::Object({prefix}shape_{snake(go)})', None

    def normalize(rust):
        """Compare equivalent existing qualified names without changing fields."""
        return re.sub(r'\s+|crate::models::|crate::|std::collections::', '', rust)

    for name, definition in types.items():
        if (definition.get('unsupported') or definition.get('underlying')
                or name not in historical
                or definition['source'] != historical[name]['source']
                or definition['source_sha256'] != historical[name]['source_sha256']):
            raise ValueError('stale-or-unsupported-agent-producer: ' + name)
        if name in shared:
            block = re.search(r'pub struct ' + re.escape(name) + r' \{(.*?)^\}', source, re.S | re.M)
            if block is None:
                raise ValueError('missing-shared-native-type: ' + name)
            actual, pending = [], ''
            for line in block[1].splitlines():
                field = re.fullmatch(r'\s*pub (\S+): (.+),', line)
                if field is None:
                    pending += line + '\n'
                    continue
                omitted = re.findall(r'skip_serializing_if\s*=\s*"([^"]+)"', pending)
                if len(omitted) > 1:
                    raise ValueError('unsupported-shared-serialization: ' + name)
                actual.append((field[1].removeprefix('r#'), normalize(field[2]),
                               normalize(omitted[0]) if omitted else None))
                pending = ''
            expected = []
            for field in definition['fields']:
                key, *flags = field['json_tag'].split(',')
                rust, _, skip = representation(field['go_type'])
                expected.append((key, normalize(rust), normalize(skip) if flags and skip else None))
            if actual != expected:
                raise ValueError('shared-agent-fields-changed: ' + name)
    parts = ['//! Generated native agent data. Decoding grants no identity or execution authority.\n',
             'use serde::{Serialize,Deserialize};\nuse crate::model_wire::{Fields,Kind};\n',
             'fn is_zero(v:&i64)->bool{*v==0}\nfn is_false(v:&bool)->bool{!*v}\n',
             'fn empty_vec<T>(v:&Option<Vec<T>>)->bool{v.as_ref().is_none_or(Vec::is_empty)}\n',
             'fn empty_map<K,V>(v:&Option<std::collections::BTreeMap<K,V>>)->bool{v.as_ref().is_none_or(std::collections::BTreeMap::is_empty)}\n']
    for name in sorted(types.keys() - shared):
        definition = types[name]
        parts += [f'// {definition["source"]} SHA-256 {definition["source_sha256"]}\n',
                  '/// Go-compatible supplied/retained data, not authenticated identity or permission.\n',
                  '#[derive(Clone,Default,PartialEq,Serialize,Deserialize)]\n#[serde(default)]\n',
                  f'pub struct {name}{{\n']
        fields = []
        for field in definition['fields']:
            key, *flags = field['json_tag'].split(',')
            if not re.fullmatch('[a-z_][a-z0-9_]*', key) or flags not in [[], ['omitempty']]:
                raise ValueError('unsupported-agent-json-field: ' + key)
            rust, kind, skip = representation(field['go_type'])
            if flags:
                if skip is None:
                    # encoding/json never considers a value struct empty;
                    # omitempty therefore retains it, including its zero state.
                    parts.append('// Go value-struct omitempty still serializes this field.\n')
                else:
                    parts.append('#[serde(skip_serializing_if=' + json.dumps(skip) + ')]\n')
            parts.append(f'pub {key}:{rust},\n')
            fields.append('(' + json.dumps(key) + ',' + kind + ')')
        if name == 'Agent':
            if any(f['json_tag'].split(',')[0] in ('native_tool_guard_policy','native_execution','native_execution_result','native_provider_facts') for f in definition['fields']):
                raise ValueError('native-agent-extension-wire-collision')
            parts.append('/// Native v1 opaque/required snapshot; absence stays historical, never a default.\n#[serde(skip_serializing_if="Option::is_none")]\npub native_tool_guard_policy:Option<Box<crate::AgentGuardSnapshot>>,\n')
            fields.append('("native_tool_guard_policy",Kind::Pointer(&Kind::Raw))')
            parts.append('/// Opaque native execution constraints; absence is historical, never a permit.\n#[serde(skip_serializing_if="Option::is_none")]\npub native_execution:Option<crate::GoRaw>,\n')
            fields.append('("native_execution",Kind::Pointer(&Kind::Raw))')
            parts.append('/// Opaque native end observations; never task acceptance or action authority.\n#[serde(skip_serializing_if="Option::is_none")]\npub native_execution_result:Option<crate::GoRaw>,\n')
            fields.append('("native_execution_result",Kind::Pointer(&Kind::Raw))')
            parts.append('/// Opaque current provider declarations and diagnostic quality; no model, invoice or actor authentication.\n#[serde(skip_serializing_if="Option::is_none")]\npub native_provider_facts:Option<crate::GoRaw>,\n')
            fields.append('("native_provider_facts",Kind::Pointer(&Kind::Raw))')
        if name == 'Agent':
            parts.append('/// Internal SQL/body claim observation; never serialized or supplied.\n#[serde(skip)]\npub(crate) unregistered_claim:bool,\n')
        parts += ['}\n', f'impl std::fmt::Debug for {name}{{fn fmt(&self,f:&mut std::fmt::Formatter<\'_>)->std::fmt::Result{{f.write_str("{name} {{ data: redacted }}")}}}}\n',
                  '/// Static Go field order and types; no validation, IO or authorization.\n',
                  f'pub(crate) fn shape_{snake(name)}()->Fields{{&[' + ','.join(fields) + ']}\n']
    text = ''.join(parts)
    # A checked closure need not contain every container kind. Emit only helpers
    # consumed by its serde attributes; do not silence dead-code diagnostics.
    for helper in ('is_zero', 'is_false', 'empty_vec', 'empty_map'):
        if 'skip_serializing_if="' + helper + '"' not in text:
            text = re.sub(r'^fn ' + helper + r'[^\n]*\n', '', text, flags=re.M)
    return text.encode()


def main():
    """Atomically replace one repository output after validating the full input.

    Reads a layout path and output path; IO/contract failures propagate.
    Historical provenance binds the versioned contract, never live Go sources.
    A failed write preserves the original; no provider or runtime mutation occurs.
    """
    if len(sys.argv) != 3:
        raise ValueError('layout-and-output-required')
    output = Path(sys.argv[2]).resolve()
    if not output.is_relative_to(ROOT):
        raise ValueError('output-outside-repository')
    content = generate(json.loads(Path(sys.argv[1]).read_bytes()))
    fd, name = tempfile.mkstemp(dir=output.parent, prefix='.agent-models-')
    temporary = Path(name)
    try:
        with os.fdopen(fd, 'wb') as stream:
            os.fchmod(stream.fileno(), output.stat().st_mode & 0o777 if output.exists() else 0o644)
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        if temporary.read_bytes() != content:
            raise OSError('agent-model-write-mismatch')
        os.replace(temporary, output)
    finally:
        temporary.unlink(missing_ok=True)
    print('Generated checked agent data; runtime remains separate')


if __name__ == '__main__':
    main()
