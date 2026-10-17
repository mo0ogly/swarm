#!/usr/bin/env python3
"""Read-only validation of a contextual mapping, never a compliance verdict."""
import argparse,datetime,hashlib,json,os,re,stat,sys
from pathlib import Path,PurePosixPath
INPUT_MAX_BYTES=16*1024*1024
AREAS={'access','identity','audit','cryptography','lifecycle','providers','incidents'}
def exact(value,keys):
 if not isinstance(value,dict)or set(value)!=set(keys):raise ValueError('exact object fields required')
def text(value):
 if not isinstance(value,str)or not value.strip():raise ValueError('non-empty text required')
def localized(value):
 exact(value,['fr','en'])
 for v in value.values():text(v)
def digest(value):
 if not isinstance(value,str)or not re.fullmatch('[a-f0-9]{64}',value):raise ValueError('SHA-256 required')
def integer(value):
 if type(value)is not int or value<1:raise ValueError('positive integer required')
def claims(value):
 exact(value,['certification','global_compliance','independent_assessment'])
 if any(v is not False for v in value.values()):raise ValueError('unsupported assurance claim')
def safe_read(root,name):
 text(name);rel=PurePosixPath(name)
 if rel.is_absolute()or '..'in rel.parts or str(rel)!=name or not rel.parts:raise ValueError('relative contained path required')
 fd=os.open(root,os.O_RDONLY|os.O_DIRECTORY)
 try:
  for part in rel.parts[:-1]:
   child=os.open(part,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW,dir_fd=fd);os.close(fd);fd=child
  datafd=os.open(rel.parts[-1],os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK,dir_fd=fd)
  with os.fdopen(datafd,'rb')as stream:
   meta=os.fstat(stream.fileno())
   if not stat.S_ISREG(meta.st_mode)or meta.st_size>INPUT_MAX_BYTES:raise ValueError('regular bounded input required')
   raw=stream.read(INPUT_MAX_BYTES+1)
   if len(raw)>INPUT_MAX_BYTES:raise ValueError('input exceeds technical byte bound')
   return raw
 except OSError as e:raise ValueError('unreadable contained regular file: '+name)from e
 finally:os.close(fd)
def pairs(items):
 value={}
 for k,v in items:
  if k in value:raise ValueError('duplicate JSON property')
  value[k]=v
 return value
def decode(raw):return json.loads(raw,object_pairs_hook=pairs)
def sha(raw):return hashlib.sha256(raw).hexdigest()
def bound_file(root,row):
 exact(row,['path','sha256']);digest(row['sha256']);raw=safe_read(root,row['path'])
 if sha(raw)!=row['sha256']:raise ValueError('file digest mismatch: '+row['path'])
 return raw

def validate(root,matrix_path):
 root=Path(root).resolve();raw=safe_read(root,matrix_path);m=decode(raw)
 exact(m,['schema_version','matrix_id','matrix_version','assessed_at','context','claims','framework','evidence','mappings'])
 if type(m['schema_version'])is not int or m['schema_version']!=1:raise ValueError('mapping schema 1 required')
 text(m['matrix_id']);integer(m['matrix_version']);datetime.date.fromisoformat(m['assessed_at']);claims(m['claims'])
 c=m['context'];exact(c,['id','scope','classifications','effects','baseline_selected','areas','runtime_assessment'])
 if c['id']!='local-simulation-v1'or c['effects']!='simulated'or c['baseline_selected']is not False or c['runtime_assessment']is not False:raise ValueError('unsupported target context or baseline claim')
 localized(c['scope'])
 if c['classifications']!=['public','internal']or not isinstance(c['areas'],list)or set(c['areas'])!=AREAS or len(c['areas'])!=len(AREAS):raise ValueError('explicit target scope required')
 f=m['framework'];exact(f,['id','revision','catalog_version','source_url','source_sha256','reference_path','reference_sha256'])
 reference=decode(bound_file(root,{'path':f['reference_path'],'sha256':f['reference_sha256']}))
 exact(reference,['schema_version','framework_id','revision','catalog_version','source_url','source_sha256','source_path','retrieved_at','controls'])
 if type(reference['schema_version'])is not int or reference['schema_version']!=1:raise ValueError('reference schema required')
 for key,refkey in [('id','framework_id'),('revision','revision'),('catalog_version','catalog_version'),('source_url','source_url'),('source_sha256','source_sha256')]:
  if f[key]!=reference[refkey]:raise ValueError('framework/reference mismatch')
 if not re.fullmatch(r'https://raw\.githubusercontent\.com/usnistgov/oscal-content/[a-f0-9]{40}/nist.gov/SP800-53/rev5/json/NIST_SP-800-53_rev5_catalog.json',f['source_url']):raise ValueError('pinned primary catalog URL required')
 primary=decode(bound_file(root,{'path':reference['source_path'],'sha256':f['source_sha256']}))['catalog']
 if primary['metadata']['version']!=f['catalog_version']:raise ValueError('catalog version mismatch')
 known={}
 def visit(node):
  for control in node.get('controls',[]):known[control['id'].upper()]=control['title'];visit(control)
  for group in node.get('groups',[]):visit(group)
 visit(primary);selected=set()
 for control in reference['controls']:
  exact(control,['id','title'])
  if control['id']in selected or known.get(control['id'])!=control['title']:raise ValueError('unknown or duplicate primary control')
  selected.add(control['id'])
 if not selected:raise ValueError('empty reference')
 evidence={}
 if not isinstance(m['evidence'],list):raise ValueError('evidence list required')
 for e in m['evidence']:
  exact(e,['id','kind','receipt','source_bindings','limitations']);text(e['id']);localized(e['limitations'])
  if e['id']in evidence or e['kind']!='historical_local_verification':raise ValueError('duplicate or unsupported evidence kind')
  receipt=decode(bound_file(root,e['receipt']))
  if not isinstance(e['source_bindings'],list)or not e['source_bindings']:raise ValueError('relevant source bindings required')
  paths=set()
  for binding in e['source_bindings']:
   bound_file(root,binding)
   if binding['path']in paths or receipt.get('input_sha256',{}).get(binding['path'])!=binding['sha256']:raise ValueError('source not bound by original receipt')
   paths.add(binding['path'])
  evidence[e['id']]=e
 if not isinstance(m['mappings'],list)or not m['mappings']:raise ValueError('non-empty mappings required')
 seen=set();areas=set();used=set();statuses={}
 for row in m['mappings']:
  exact(row,['control_id','title','area','scope','rationale','owner','status','evidence_ids','gaps','external_assessment'])
  key=row['control_id']
  if key not in selected or key in seen:raise ValueError('unknown or duplicate mapped control')
  if row['area']not in AREAS:raise ValueError('unknown control area')
  for field in ['title','scope','rationale']:localized(row[field])
  text(row['owner'])
  if row['status']not in ['partial','not_assessed','out_of_scope']:raise ValueError('unsupported control qualification')
  if not isinstance(row['evidence_ids'],list)or len(set(row['evidence_ids']))!=len(row['evidence_ids'])or any(k not in evidence for k in row['evidence_ids']):raise ValueError('unknown or duplicate evidence reference')
  if row['status']=='partial'and not row['evidence_ids']:raise ValueError('partial mapping requires evidence')
  if not isinstance(row['gaps'],list)or not row['gaps']:raise ValueError('explicit control gaps required')
  for gap in row['gaps']:localized(gap)
  external=row['external_assessment'];exact(external,['status','required_for_target_use'])
  if external['status']!='not_performed':raise ValueError('external assessment is not established')
  localized(external['required_for_target_use']);seen.add(key);areas.add(row['area']);used.update(row['evidence_ids']);statuses[row['status']]=statuses.get(row['status'],0)+1
 if seen!=selected or areas!=AREAS or used!=set(evidence):raise ValueError('declared controls, areas and evidence must be fully attributed')
 return {'schema_version':1,'status':'mapping_artifact_valid','matrix_sha256':sha(raw),'matrix_id':m['matrix_id'],'matrix_version':m['matrix_version'],'catalog_version':f['catalog_version'],'mapped_controls':len(seen),'status_counts':statuses,'evidence_files':len(evidence),'source_bindings':sum(len(e['source_bindings'])for e in evidence.values()),'file_integrity_checked':True,'tests_rerun':False,'runtime_qualified':False,'certification':False,'global_compliance':False,'independent_assessment':False,'technical_bounds':{'input_max_bytes':INPUT_MAX_BYTES}}

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[2]);p.add_argument('--matrix',default='web/trust-controls.json');a=p.parse_args()
 try:result=validate(a.root,a.matrix)
 except (ValueError,KeyError,TypeError,UnicodeError,OSError)as e:
  print(json.dumps({'error':str(e),'status':'mapping_artifact_rejected','global_compliance':False}),file=sys.stderr);return 2
 print(json.dumps(result));return 0
if __name__=='__main__':sys.exit(main())
