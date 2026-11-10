#!/usr/bin/env python3
"""Exercise the public read-only mapping validator against real source evidence."""
import argparse,copy,hashlib,json,os,subprocess,sys
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);p.add_argument('--evidence-dir',type=Path,required=True);p.add_argument('--timeout-seconds',type=int,default=120);a=p.parse_args();assert a.timeout_seconds>0;root=a.root.resolve();out=a.evidence_dir.resolve();out.mkdir(parents=True,exist_ok=True);assert out.is_relative_to(root)
tool=root/'tools/actions-confiance/controls.py';matrix=root/'web/trust-controls.json';original=matrix.read_bytes();data=json.loads(original);checks=[]
def run(path,expected):
 r=subprocess.run([sys.executable,str(tool),'--root',str(root),'--matrix',str(path.relative_to(root))],capture_output=True,text=True,timeout=a.timeout_seconds)
 assert r.returncode==expected,(r.returncode,r.stderr)
 value=json.loads(r.stdout if not expected else r.stderr)
 assert value['global_compliance']is False
 return value
v=run(matrix,0);assert v['mapped_controls']==12 and v['status_counts']=={'partial':9,'not_assessed':2,'out_of_scope':1};assert v['file_integrity_checked']and not v['tests_rerun']and not v['runtime_qualified'];checks.append('actual repository matrix verifies pinned primary catalog, historical receipts and relevant current source bindings without qualification claim')
variants=[]
for claim in ['certification','global_compliance','independent_assessment']:variants.append((claim,lambda m,k=claim:m['claims'].update({k:True})))
variants.extend([('unknown-control',lambda m:m['mappings'][0].update(control_id='ZZ-999')),('unknown-status',lambda m:m['mappings'][0].update(status='compliant')),('missing-evidence',lambda m:m['evidence'][0]['receipt'].update(path='missing-proof.json')),('altered-receipt',lambda m:m['evidence'][0]['receipt'].update(sha256='0'*64)),('stale-source',lambda m:m['evidence'][0]['source_bindings'][0].update(sha256='0'*64)),('escaped-proof',lambda m:m['evidence'][0]['receipt'].update(path='../outside.json')),('no-gap',lambda m:m['mappings'][0].update(gaps=[])),('false-runtime-context',lambda m:m['context'].update(runtime_assessment=True)),('unknown-field',lambda m:m.update(certified=True))])
for name,change in variants:
 m=copy.deepcopy(data);change(m);path=out/(name+'.json');path.write_text(json.dumps(m));value=run(path,2);assert value['status']=='mapping_artifact_rejected';checks.append(name+' rejected')
assert matrix.read_bytes()==original
(out/'mapping-public.json').write_text(json.dumps({'checks':checks,'matrix_sha256':hashlib.sha256(original).hexdigest(),'source_mutation':False,'qualification':False},indent=2)+'\n');print(json.dumps({'checks':len(checks),'source_mutation':False}))
