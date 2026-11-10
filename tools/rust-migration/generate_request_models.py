"""Generate typed request DTOs from the checked Go AST layout, never at runtime."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tempfile
ROOT=Path(__file__).resolve().parents[2]
layout=json.loads(Path(sys.argv[1]).read_text());output=Path(sys.argv[2])
source=Path(layout['source']);source=source if source.is_absolute()else ROOT/source
assert source.resolve()==ROOT/'internal/engine/model.go'
assert hashlib.sha256(source.read_bytes()).hexdigest()==layout['source_sha256'],'actual Go model changed'
custom=layout['request_marshaler'];custom_source=Path(custom['source']);custom_source=custom_source if custom_source.is_absolute()else ROOT/custom_source
assert custom_source.resolve()==ROOT/'internal/engine/task_definition.go'
assert hashlib.sha256(custom_source.read_bytes()).hexdigest()==custom['source_sha256'],'Request.MarshalJSON changed'
assert 'r.Depends != nil'in custom['code']and 'len(r.Depends) == 0'in custom['code'],'custom empty-dependency contract must be reexamined'
types=layout['types'];assert set(types)=={'Request','ValidationPolicy','ValidationControl'}
def const(name):return 'FIELDS_'+re.sub(r'(?<!^)(?=[A-Z])','_',name).upper()
def representation(go):
 if go=='string':return 'String','Kind::String','String::is_empty'
 if go=='int':return 'i64','Kind::Int64','is_zero'
 if go=='bool':return 'bool','Kind::Bool','is_false'
 if go in ['[]string','[]int']:
  element='String'if go=='[]string'else'i64';kind='Kind::String'if element=='String'else'Kind::Int64'
  return 'Option<Vec<'+element+'>>','Kind::Array { element: &'+kind+', nil: true }','empty_vec'
 if go=='map[string][]string':return 'Option<BTreeMap<String, Option<Vec<String>>>>','Kind::Map(&Kind::Array { element: &Kind::String, nil: true })','empty_map'
 if go=='*ValidationPolicy':return 'Option<ValidationPolicy>','Kind::OptionalObject('+const('ValidationPolicy')+')','Option::is_none'
 if go=='[]ValidationControl':return 'Option<Vec<ValidationControl>>','Kind::Array {element: &Kind::Object('+const('ValidationControl')+'), nil: true}','empty_vec'
 raise ValueError('Unsupported Go type requires design and tests: '+go)
parts=['//! Generated from internal/engine/model.go by generate_request_models.py.\n//! Data only: typed origin/policy fields grant no authority.\n// Go model SHA-256: '+layout['source_sha256']+'\n','use serde::{Serialize,Deserialize};\nuse serde::ser::SerializeMap;\nuse std::collections::BTreeMap;\nuse swarm_wire_json::projection::{Fields,Kind};\n','fn is_zero(v:&i64)->bool {*v==0}\nfn is_false(v:&bool)->bool {!*v}\nfn empty_vec<T>(v:&Option<Vec<T>>)->bool {v.as_ref().is_none_or(Vec::is_empty)}\nfn empty_map<K,V>(v:&Option<BTreeMap<K,V>>)->bool {v.as_ref().is_none_or(BTreeMap::is_empty)}\n']
for name in ['ValidationControl','ValidationPolicy','Request']:
 fields=types[name];assert fields
 parts+=['#[derive(Clone,Debug,Default,PartialEq,Eq,'+('Deserialize'if name=='Request'else'Serialize,Deserialize')+')]\n#[serde(default,deny_unknown_fields)]\npub struct '+name+' {\n'];shape=[];names=set();serialized=[]
 for field in fields:
  tokens=field['json_tag'].split(',');key=tokens[0];assert re.fullmatch('[a-z_][a-z0-9_]*',key)and key not in names;names.add(key);assert tokens[1:]in [[],['omitempty']]
  rust,kind,skip=representation(field['go_type']);serialized.append((key,'omitempty'in tokens,skip));parts+=['    // Go '+field['go_name']+', model.go:'+str(field['line'])+'\n']
  if 'omitempty'in tokens:parts+=['    #[serde(skip_serializing_if='+json.dumps(skip)+')]\n']
  parts+=['    pub '+key+': '+rust+',\n'];shape.append('('+json.dumps(key)+', '+kind+')')
 parts+=['}\n','pub(crate) const '+const(name)+':Fields=&[\n'+',\n'.join(shape)+'\n];\n']
parts+=['// Request.MarshalJSON in task_definition.go appends an explicit empty depends\n// after the legacy payload; nonempty depends retains its ordinary field order.\n', 'impl Serialize for Request {\nfn serialize<S:serde::Serializer>(&self,serializer:S)->Result<S::Ok,S::Error> {\nlet mut out=serializer.serialize_map(None)?;\n']
for key,optional,skip in serialized:
 call='out.serialize_entry('+json.dumps(key)+', &self.'+key+')?;'
 parts+=[('if !'+skip+'(&self.'+key+') {'+call+'}'if optional else call)+'\n']
parts+=['if let Some(depends)=self.depends.as_ref().filter(|v|v.is_empty()) {out.serialize_entry("depends",depends)?;}\nout.end()\n}\n}\n']
content=''.join(parts);output.parent.mkdir(parents=True,exist_ok=True);fd,tmp=tempfile.mkstemp(dir=output.parent,prefix='.request-model-')
try:
 with os.fdopen(fd,'w')as f:f.write(content);f.flush();os.fsync(f.fileno())
 os.replace(tmp,output)
finally:
 if Path(tmp).exists():Path(tmp).unlink()
print('generated',sum(len(v)for v in types.values()),'fields from',len(types),'actual Go structs')
