"""Bind newly executed shared-effect regressions; preserve historical receipts."""
import hashlib,json,re
from pathlib import Path
root=Path(__file__).resolve().parents[4];out=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
m=json.loads((root/'web/trust-controls.json').read_text());effect=next(v for v in m['evidence'] if v['id']=='effects')
parts=[]
for name in ['observe-effects.out']:
 p=out/name;s=p.read_text();assert '\nPASS\nok\t' in s or re.search(r'\nPASS\nok\s',s)
 assert not re.search(r'^\s*--- FAIL:|^FAIL',s,re.M)
 names=re.findall(r'^\s*--- PASS: (\S+)',s,re.M);assert names
 parts.append(dict(path=str(p.relative_to(root)),sha256=sha(p),passed_names=names,exit_code=0))
receipt=dict(schema_version=1,scope='shared simulated effects after optional kernel_observation policy integration',input_sha256={b['path']:sha(root/b['path']) for b in effect['source_bindings']},executed=parts,native_observation_qualified=False,independent_assessment=False,global_compliance=False,source_freeze='Go inputs unchanged from these executions; subsequent changes only contextual mapping, architecture links and documentation')
p=out/'observe-effects-verification.json';p.write_text(json.dumps(receipt,indent=2)+'\n')
effect['receipt']=dict(path=str(p.relative_to(root)),sha256=sha(p))
for b in effect['source_bindings']:b['sha256']=receipt['input_sha256'][b['path']]
m['matrix_version']+=1
(root/'web/trust-controls.json').write_text(json.dumps(m,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(dict(matrix_version=m['matrix_version'],effect_receipt=effect['receipt']['path'],other_evidence_preserved=True)))
