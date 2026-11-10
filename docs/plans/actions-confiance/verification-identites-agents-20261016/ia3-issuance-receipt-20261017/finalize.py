from pathlib import Path
import datetime,hashlib,json,re,subprocess
B=Path(__file__).resolve().parent
ROOT=Path.cwd()
def read(name):return json.loads((B/name).read_text())
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
normal=read('normal-contracts-02-run.json')
race_steps=['race-contracts-02']
groups=[read(name+'-run.json') for name in race_steps]
for record in [normal]+groups:
    assert record['exit']==0 and record['counts']['pass']>0 and not record['counts']['fail'] and not record['counts']['skip']
    assert not record['changed_inputs'] and record['binary_unchanged']
expected=set(read('normal-contracts-01-run.json')['names']['pass'])
assert set(normal['names']['pass'])==expected
assert set().union(*(set(r['names']['pass']) for r in groups))==expected
assert len(expected)==117 and sum(n.startswith('TestAgentIdentityIssuance') for n in groups[0]['names']['pass'])==37
for step in ['race-build-01','normal-build-02','vet-01','app-build-01']:
    r=read(step+'-run.json');assert r['exit']==0 and not r['changed_inputs']
final_inputs=read('normal-contracts-02-inputs.json')
for step in ['race-build-01','vet-01','app-build-01']+race_steps:
    assert read(step+'-inputs.json')==final_inputs,step
for name in ['normal','race','app']:
    r=read(name+'-binary.json');assert sha(r['path'])==r['sha256']
assert read('normal-binary.json')['path'] in normal['command']
assert all(read('race-binary.json')['path'] in r['command'] for r in groups)
red=read('receipt-before-02-run.json')
assert red['exit']==1 and red['counts']=={'pass':0,'fail':1,'skip':0} and not red['changed_inputs']
assert 'lost its issuance receipt' in (B/'receipt-before-02.jsonl').read_text()
for name,h in read('source-final.json').items():assert sha(name)==h,name
source_delta=sorted(p for p in set(final_inputs)|set(read('receipt-before-02-inputs.json')) if final_inputs.get(p)!=read('receipt-before-02-inputs.json').get(p))
assert set(source_delta)==set(read('source-final.json')),source_delta
assert all(d['exit']==0 for d in read('diagrams.json'))
for lang,name in [('fr','docs/architecture/AGENT-IDENTITY-CONTRACT.md'),('en','docs/en/AGENT-IDENTITY-CONTRACT.md')]:
    assert re.findall(r'```mermaid\n(.*?)```',Path(name).read_text(),re.S)[-1]==(B/('receipt-'+lang+'.mmd')).read_text()
check=subprocess.run(['git','diff','--check'],capture_output=True,text=True);assert check.returncode==0,check.stdout+check.stderr
fmt=subprocess.run(['gofmt','-l',*read('source-final.json')],capture_output=True,text=True);assert fmt.returncode==0 and not fmt.stdout,fmt.stdout+fmt.stderr
assert read('run-preservation-guard.json')['original_unchanged']
print(json.dumps({'normal':len(expected),'race':sum(r['counts']['pass'] for r in groups),'new_receipt_names':37,'input_count':len(final_inputs),'owned_source_delta':source_delta}))
