"""Generate the closed Work/Task/Event graph from actual compiled Go AST input."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
layout = json.loads(Path(sys.argv[1]).read_text())
output = Path(sys.argv[2])
assert layout['roots'] == ['Work', 'Task', 'Event']
assert layout['external'] == ['json.RawMessage'] and not layout['custom_methods']
types = layout['types']
for t in types.values():
    assert not t.get('unsupported') and not t.get('underlying')
    p = ROOT/t['source']
    assert p.is_relative_to(ROOT/'internal/engine')
    assert hashlib.sha256(p.read_bytes()).hexdigest() == t['source_sha256'], 'Go source changed'


def name(go):
    return go[0].upper()+go[1:]


def snake(go):
    return re.sub(r'(?<!^)(?=[A-Z])', '_', go).lower()


def ident(key):
    assert re.fullmatch('[a-z_][a-z0-9_]*', key), key
    keywords = {'type', 'ref', 'match', 'loop', 'move', 'use', 'mod', 'pub', 'fn', 'in', 'return', 'where', 'const', 'static', 'async', 'await', 'dyn', 'box'}
    return 'r#'+key if key in keywords else key


def representation(go):
    if go.startswith('*'):
        rust, kind, _ = representation(go[1:])
        return 'Option<Box<'+rust+'>>', 'Kind::Pointer(&'+kind+')', 'Option::is_none'
    if go.startswith('[]'):
        rust, kind, _ = representation(go[2:])
        return 'Option<Vec<'+rust+'>>', 'Kind::Array(&'+kind+')', 'empty_vec'
    if go.startswith('map[string]'):
        rust, kind, _ = representation(go[11:])
        return 'Option<BTreeMap<String, '+rust+'>>', 'Kind::Map(&'+kind+')', 'empty_map'
    if go == 'string': return 'String', 'Kind::String', 'String::is_empty'
    if go in ['int', 'int64']: return 'i64', 'Kind::Int64', 'is_zero'
    if go == 'bool': return 'bool', 'Kind::Bool', 'is_false'
    if go == 'float64': return 'GoFloat', 'Kind::Float64', 'GoFloat::is_zero'
    if go == 'json.RawMessage': return 'GoRaw', 'Kind::Raw', 'GoRaw::is_nil'
    assert go in types, 'unsupported type: '+go
    return name(go), 'Kind::Object(shape_'+snake(go)+')', None


parts = ['//! Generated Work/Task/Event data graph; no execution authority.\n',
         'use serde::{Serialize,Deserialize};\nuse std::collections::BTreeMap;\n',
         'use super::{ValidationControl,ValidationPolicy};\nuse super::model_wire::{Fields,Kind,GoFloat,GoRaw};\n',
         'fn is_zero(v:&i64)->bool {*v==0}\nfn is_false(v:&bool)->bool {!*v}\n',
         'fn empty_vec<T>(v:&Option<Vec<T>>)->bool {v.as_ref().is_none_or(Vec::is_empty)}\n',
         'fn empty_map<K,V>(v:&Option<BTreeMap<K,V>>)->bool {v.as_ref().is_none_or(BTreeMap::is_empty)}\n']
for go, t in sorted(types.items()):
    fields = t['fields']; shared = go in ['ValidationControl', 'ValidationPolicy']
    recursive = go == 'ManagedFragmentJournalAnchor'
    if not shared:
        derive = 'Default,Deserialize' if recursive else 'Clone,Debug,Default,PartialEq,Serialize,Deserialize'
        parts += ['// '+t['source']+' SHA-256 '+t['source_sha256']+'\n', '#[derive('+derive+')]\n'+('' if recursive else '#[serde(default)]\n')+'pub struct '+name(go)+' {\n']
    shape = []; named = []
    for f in fields:
        key, *flags = f['json_tag'].split(','); assert flags in [[], ['omitempty']]
        rust, kind, skip = representation(f['go_type']); field = ident(key); named.append(field)
        if not shared:
            parts += ['// '+f['go_name']+' line '+str(f['line'])+'\n', '#[serde(rename='+json.dumps(key)+')]\n']
            if recursive: parts += ['#[serde(default)]\n']
            if flags:
                assert skip, 'Go struct omitempty requires review'
                parts += ['#[serde(skip_serializing_if='+json.dumps(skip)+')]\n']
            parts += ['pub '+field+': '+rust+',\n']
        shape.append('('+json.dumps(key)+','+kind+')')
    if not shared: parts += ['}\n']
    parts += ['pub(crate) fn shape_'+snake(go)+'()->Fields {&['+','.join(shape)+']}\n']
    if recursive:
        assert sum(f['go_type']=='*'+go for f in fields)==1
        assert next(f['json_tag']for f in fields if f['go_type']=='*'+go)=='replanned_from,omitempty'
        other = [f for f in named if f!='replanned_from']
        split = named.index('replanned_from')
        assert any('omitempty' not in f['json_tag'] for f in fields[split+1:]), 'suffix must never be empty'
        for label, subset in [('Prefix',fields[:split]),('Suffix',fields[split+1:])]:
            parts += ['#[derive(Serialize)]\nstruct Anchor'+label+'<\'a>{\n']
            for f in subset:
                key,*flags=f['json_tag'].split(',');rust,_,skip=representation(f['go_type'])
                parts += ['#[serde(rename='+json.dumps(key)+')]\n']
                if flags:parts += ['#[serde(skip_serializing_if='+json.dumps(skip)+')]\n']
                parts += [ident(key)+':&\'a '+rust+',\n']
            parts += ['}\n']
        prefix_fields=[ident(f['json_tag'].split(',')[0]) for f in fields[:split]]
        suffix_fields=[ident(f['json_tag'].split(',')[0]) for f in fields[split+1:]]
        parts += ['impl Serialize for '+go+' {fn serialize<S:serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error>{let mut output=Vec::new();let mut suffixes=Vec::new();let mut node=self;loop{\n',
                  'let prefix=serde_json::to_vec(&AnchorPrefix{'+','.join(f+':&node.'+f for f in prefix_fields)+'}).map_err(serde::ser::Error::custom)?;\n',
                  'let suffix=serde_json::to_vec(&AnchorSuffix{'+','.join(f+':&node.'+f for f in suffix_fields)+'}).map_err(serde::ser::Error::custom)?;\n',
                  'output.push(b\'{\');if prefix.len()>2{output.extend_from_slice(&prefix[1..prefix.len()-1]);output.push(b\',\');}\n',
                  'if let Some(child)=node.replanned_from.as_deref(){output.extend_from_slice(b"\\\"replanned_from\\\":");suffixes.push(suffix);node=child;}else{output.extend_from_slice(&suffix[1..suffix.len()-1]);output.push(b\'}\');break;}\n',
                  '}for suffix in suffixes.into_iter().rev(){output.push(b\',\');output.extend_from_slice(&suffix[1..suffix.len()-1]);output.push(b\'}\');}\n',
                  'let text=String::from_utf8(output).map_err(serde::ser::Error::custom)?;let raw=serde_json::value::RawValue::from_string(text).map_err(serde::ser::Error::custom)?;raw.serialize(s)}}\n']
        parts += ['impl Drop for '+go+' {fn drop(&mut self){let mut next=self.replanned_from.take();while let Some(mut node)=next{next=node.replanned_from.take();}}}\n',
                  'impl Clone for '+go+' {fn clone(&self)->Self {let mut chain=Vec::new();let mut next=Some(self);while let Some(node)=next{chain.push(node);next=node.replanned_from.as_deref();}let mut previous=None;for node in chain.into_iter().rev(){previous=Some(Box::new(Self{replanned_from:previous,'+','.join(f+':node.'+f+'.clone()' for f in other)+'}));}*previous.expect("anchor clone has a root")}}\n',
                  'impl PartialEq for '+go+' {fn eq(&self,other:&Self)->bool{let(mut left,mut right)=(Some(self),Some(other));loop{match(left,right){(None,None)=>return true,(Some(a),Some(b))=>{if '+ '||'.join('a.'+f+'!=b.'+f for f in other)+'{return false;}left=a.replanned_from.as_deref();right=b.replanned_from.as_deref();},_=>return false}}}}\n',
                  'impl std::fmt::Debug for '+go+' {fn fmt(&self,f:&mut std::fmt::Formatter<\'_>)->std::fmt::Result{let mut depth=0usize;let mut node=self.replanned_from.as_deref();while let Some(v)=node{depth+=1;node=v.replanned_from.as_deref();}f.debug_struct("ManagedFragmentJournalAnchor").field("replanned_from_depth",&depth).finish()}}\n']
parts += ['#[cfg(feature="test-fixtures")]\npub fn fixture_model_roundtrip(name:&str,raw:&[u8])->Result<Vec<u8>,super::WorkError>{match name{\n']
for go in sorted(types):
    parts += [json.dumps(go)+'=>super::work::decode::<'+name(go)+'>(raw,shape_'+snake(go)+'()).and_then(|v|super::encode_model(&v)),\n']
parts += ['_=>Err(super::WorkError::Projection(super::model_wire::Error::Invalid("unknown fixture model"))),}}\n']
output.parent.mkdir(parents=True, exist_ok=True)
fd, temporary = tempfile.mkstemp(dir=output.parent, prefix='.work-model-')
try:
    with os.fdopen(fd, 'w') as f:
        f.write(''.join(parts)); f.flush(); os.fsync(f.fileno())
    os.replace(temporary, output)
finally:
    if Path(temporary).exists(): Path(temporary).unlink()
print('Generated', len(types)-2, 'new types;', sum(len(t['fields']) for t in types.values()), 'Go fields including shared validation types')
