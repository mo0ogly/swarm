"""Generate identity projection rules and exact SQL from hash-checked Go AST."""
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile
ROOT=Path(__file__).resolve().parents[2]
v=json.loads(Path(sys.argv[1]).read_text());out=Path(sys.argv[2]);out.mkdir(parents=True,exist_ok=True)
assert v['version']==1 and set(v['omits'])=={'object','task','planning','scope'}
assert v['udf']=='swarm_identity_plan_digest_v1' and 'PRAGMA user_version=37;'in v['schema_sql']
for name,digest in v['sources'].items():assert hashlib.sha256((ROOT/name).read_bytes()).hexdigest()==digest
text='//! Generated from actual Go projection sources. No adoption or creation authority.\n'
for name,digest in sorted(v['sources'].items()):text+='// '+name+' SHA-256 '+digest+'\n'
text+='pub const PROJECTION_VERSION:i64=1;\npub const UDF_NAME:&str='+json.dumps(v['udf'])+';\n'
text+='pub const IDENTITY_PLAN_HEADS_SQL:&str=include_str!("identity_plan_heads.sql");\n'
text+='pub(crate) const OMITS:&[(&str,&[&str])]=&['+','.join('('+json.dumps(k)+',&'+json.dumps(val)+')'for k,val in sorted(v['omits'].items()))+'];\n'
text+='pub(crate) const NESTED:&[(&str,&str,&[&str])]=&['+','.join('('+json.dumps(n['owner'])+','+json.dumps(n['key'])+',&'+json.dumps(n['omit'])+')'for n in v['nested'])+'];\n'
for filename,data in [('identity_plan_rules.rs',text),('identity_plan_heads.sql',v['schema_sql'])]:
 p=out/filename;fd,tmp=tempfile.mkstemp(dir=out,prefix='.'+filename+'.')
 try:
  with os.fdopen(fd,'w')as f:f.write(data);f.flush();os.fsync(f.fileno())
  os.replace(tmp,p)
 finally:
  if Path(tmp).exists():Path(tmp).unlink()
print('Generated four omission groups, four nested rules and exact v37 SQL')
