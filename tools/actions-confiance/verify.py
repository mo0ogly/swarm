import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

parser=argparse.ArgumentParser()
parser.add_argument('task',choices=[f'T{i}' for i in range(1,9)])
parser.add_argument('mode',choices=['entry','validation','delivery','c1','c2','c3','c4'])
parser.add_argument('--timeout-seconds',type=int,required=True)
args=parser.parse_args()
root=Path.cwd().resolve()
source=Path(__file__).resolve()
local=root/'tools/actions-confiance/verify.py'
if not local.is_file() or local.read_bytes()!=source.read_bytes():
    raise SystemExit('Operator verifier differs from the candidate copy')
if args.timeout_seconds<1:
    raise SystemExit('Positive declared timeout required')

def contained(name):
    rel=Path(name)
    if rel.is_absolute() or '..' in rel.parts:
        raise ValueError('Evidence outside candidate')
    path=root/rel
    if path.is_symlink() or not path.is_file() or root not in path.resolve().parents:
        raise ValueError('Regular candidate evidence required: '+name)
    return path

def run(argv):
    p=subprocess.run(argv,cwd=root,text=True,capture_output=True,timeout=args.timeout_seconds)
    sys.stdout.write(p.stdout)
    if p.stderr:sys.stderr.write(p.stderr)
    if p.returncode:
        raise SystemExit(p.returncode)
    return p.stdout

if args.mode=='entry':
    for name in ['go.mod','AGENTS.md','tools/agent-workflows/CONTRACT.md']:
        contained(name)
    if args.task!='T1':contained('docs/architecture/actions-confiance-contract.json')
    print(json.dumps({'task':args.task,'mode':'entry','candidate':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'status':'pass'}))
    raise SystemExit(0)

if args.task=='T1':
    raw=contained('docs/architecture/actions-confiance-contract.json').read_text()
    contract=json.loads(raw)
    if contract.get('version')!=1:raise ValueError('Contract version required')
    requirements=contract.get('requirements',[])
    if {r.get('id') for r in requirements}!={f'R{i:02}' for i in range(1,21)} or len(requirements)!=20:
        raise ValueError('Exact R01-R20 coverage required')
    for r in requirements:
        for key in ['owner','observable','verification','evidence_paths']:
            if not r.get(key):raise ValueError('Requirement contract incomplete: '+key)
        for name in r['evidence_paths']:contained(name)
    stories=contract.get('story_ids',[])
    expected={'s01-mandat','s02-reglement','s03-reprise','s04-preuve','s05-pilotage','s06-recette','s07-identites','s08-donnees','s09-processus','s10-conservation','s11-simulation','s12-connecteurs','s13-referentiels','s14-ebpf'}
    if set(stories)!=expected or len(stories)!=14:raise ValueError('Exact story coverage required')
    for name in ['docs/architecture/ACTIONS-CONFIANCE.md','docs/en/ACTIONS-CONFIANCE.md']:
        text=contained(name).read_text()
        if not all(f'R{i:02}' in text for i in range(1,21)) or '```mermaid' not in text:
            raise ValueError('Architecture must map every requirement and include its flow')
    for key in ['boundaries','interfaces','alternatives','recovery','configuration','ui_states']:
        if not contract.get(key):raise ValueError('Missing architecture contract: '+key)
    for boundary in contract['boundaries']:
        if boundary.get('state') not in ['existing','target','unverified']:
            raise ValueError('Boundary implementation state required')
        if not boundary.get('owner') or not boundary.get('resource') or not boundary.get('control'):
            raise ValueError('Boundary responsibility and control required')
    if len(contract['alternatives'])<2:raise ValueError('Architecture alternatives required')
    print(json.dumps({'task':'T1','mode':args.mode,'requirements':len(requirements),'stories':len(stories),'contract_sha256':hashlib.sha256(raw.encode()).hexdigest(),'status':'pass'}))
    raise SystemExit(0)

recipe=contained('tests/actions_confiance_t'+args.task[1:]+'.py')
mode=args.mode
criterion=0 if mode in ['validation','delivery'] else int(mode[1:])
output=run([sys.executable,str(recipe),'--criterion',str(criterion),'--root',str(root),'--timeout-seconds',str(args.timeout_seconds)])
result=json.loads(output)
if result.get('version')!=1 or result.get('task')!=args.task or result.get('criterion')!=criterion:
    raise ValueError('Recipe result attributed to another task or criterion')
cases=result.get('cases')
if not isinstance(cases,list) or not cases:
    raise ValueError('Zero observed cases is not a successful recipe')
ids=set()
for case in cases:
    if not isinstance(case,dict) or not case.get('id') or case['id'] in ids:
        raise ValueError('Missing or duplicate case identity')
    ids.add(case['id'])
    if case.get('status')!='pass' or type(case.get('assertions')) is not int or case['assertions']<1 or not isinstance(case.get('observed'),dict) or not case['observed']:
        raise ValueError('Case lacks passing assertions and observed state')
if args.task=='T7' and mode in ['validation','delivery','c4']:
    observation=result.get('kernel_observation',{})
    if observation.get('available') is False:
        print('Real eBPF collection unavailable; permission or kernel prerequisite required',file=sys.stderr)
        raise SystemExit(3)
    if observation.get('source')!='ebpf' or not observation.get('events') or not observation.get('owned_pids'):
        raise ValueError('Real project-scoped kernel observation required')
if mode=='validation':
    out=run(['go','test','./internal/...','-json','-count=1','-run','^TestActionsConfiance'+args.task+'','-timeout',str(args.timeout_seconds)+'s'])
    events=[json.loads(line) for line in out.splitlines() if line.startswith('{')]
    runs={e['Package']+'/'+e['Test'] for e in events if e.get('Action')=='run' and e.get('Test')}
    passes={e['Package']+'/'+e['Test'] for e in events if e.get('Action')=='pass' and e.get('Test')}
    if not runs or not runs<=passes:raise ValueError('Named Go tests missing or incomplete')
    print(json.dumps({'named_go_tests':len(runs),'status':'pass'}))
print(json.dumps({'task':args.task,'mode':mode,'cases':len(cases),'status':'pass'}))
