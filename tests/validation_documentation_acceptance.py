"""Verify documentary candidate integrity; independent review assesses meaning."""
import hashlib,json,pathlib,subprocess
root=pathlib.Path('.').resolve()
m=json.loads((root/'docs/final-validation-coverage.json').read_text())
assert m['schema_version']==1
assert {r['id'] for r in m['requirements']}=={'req-5','req-6'}
inputs=m['inputs']; paths=[i['path'] for i in inputs]
assert inputs and len(paths)==len(set(paths))
for i in inputs:
 p=(root/i['path']).resolve();assert p.is_relative_to(root)
 assert hashlib.sha256(p.read_bytes()).hexdigest()==i['sha256'], 'stale input: '+i['path']
lines=''.join(sorted(i['sha256']+'  '+i['path']+'\n' for i in inputs))
assert hashlib.sha256(lines.encode()).hexdigest()==m['candidate']['input_digest'], 'candidate digest mismatch'
for p in ('docs/FINAL-VALIDATION-COORDINATION.md','docs/en/FINAL-VALIDATION-COORDINATION.md'):
 s=(root/p).read_text();assert len(s)>1500
 for value in ('300','974','234','unknown'):
  assert value in s,(p,value)
subprocess.run(['python3','tools/agent-workflows/check.py'],check=True)
subprocess.run(['git','diff','--check'],check=True)
print('PASS documentary integrity: '+str(len(inputs))+' inputs, current candidate hashes, req-5/6 coverage and FR/EN historical measurements. Semantic adequacy requires independent review.')
