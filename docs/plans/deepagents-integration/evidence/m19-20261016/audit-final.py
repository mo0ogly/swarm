"""Verify M19 evidence, source reuse, actual provider input and retained history."""
import ast
import datetime
import difflib
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

root=Path(__file__).resolve().parents[5]
out=Path(__file__).resolve().parent
read=lambda name:json.loads((out/name).read_text())
sha=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
base,extension=read('baseline.json'),read('scope-extension.json')
recipe='tests/memory_summary_admission.py'
reuse={}
for name in ['contract-final','selection-race','source-reference-regression','build','static','configuration','native-final']:
    record=read(name+'.json');assert record['exit_code']==0 and record['changed_inputs']==[],name
    assert sha(out/(name+'.log'))==record['log_sha256'],name
    inputs=read(name+'-inputs.json')
    differences={p:{'executed_sha256':h,'current_sha256':sha(root/p)} for p,h in inputs.items() if sha(root/p)!=h}
    allowed={recipe} if name!='native-final' else set()
    if name=='source-reference-regression':allowed.add('internal/engine/memory_summary_selection_test.go')
    assert set(differences)<=allowed,(name,differences)
    reuse[name]={'result':'PASS','manifest_inputs':len(inputs),'later_changed_nonconsumed_inputs':differences,'assessment':'Native recipe is not consumed by Go/Python unit checks, configuration or build. Python discovery does not load Go tests. Actual consumed sources are unchanged; final native recipe has no drift.'}
    if name in ['contract-final','selection-race','build','static','native-final']:
        assert {str(p.relative_to(root)) for p in (root/'internal/engine').glob('*.go')}<=set(inputs)
names=read('contract-final.json')['named_tests_ran'];assert len(names)==len(set(names))==21
assert len([n for n in names if n.startswith('TestMemorySummarySelection')])==11
race=read('selection-race.json')['named_tests_ran'];assert len(race)==2 and set(race)<=set(names)
python=read('source-reference-regression.json')['named_tests_ran'];assert len(python)==len(set(python))==49
assert 'Ran 49 tests' in (out/'source-reference-regression.log').read_text()
for name in ['selection-red','selection-contract','confirmation-red','stored-boundary-red','selection-final','native']:
    record=read(name+'.json');assert record['exit_code']==1 and record['changed_inputs']==[],name
    assert sha(out/(name+'.log'))==record['log_sha256'],name
for name,path,pre in [
    ('selection-red','internal/engine/memory_summary_selection_test.go','selection-red-reconstructed-test.go.txt'),
    ('selection-contract','internal/engine/memory_summary_selection_test.go','selection-contract-test.go.txt'),
    ('confirmation-red','internal/engine/memory_summary_selection.go','confirmation-red-memory_summary_selection.go.txt'),
    ('confirmation-red','internal/engine/memory_summary_selection_test.go','confirmation-red-memory_summary_selection_test.go.txt'),
    ('stored-boundary-red','internal/engine/agents_store.go','stored-boundary-red-agents_store.go.txt'),
    ('stored-boundary-red','internal/engine/memory_summary_selection_test.go','stored-boundary-red-memory_summary_selection_test.go.txt'),
    ('selection-final','internal/engine/memory_summary_selection_test.go','concurrency-expectation.go.txt'),
    ('native',recipe,'native-first.py.txt')]:
    assert sha(out/'preimages'/pre)==read(name+'-inputs.json')[path],(name,pre)
assert read('selection-contract.json')['named_tests_ran']==[]
assert read('reconstructed-preimage.json')['sha256']==read('selection-red-inputs.json')['internal/engine/memory_summary_selection_test.go']

native=read('native-final/result.json');assert native['status']=='PASS' and len(native['checks'])==5
assert native['m18_public_checks']==5 and native['source_scripted_invocations']==2 and native['worker_scripted_invocations']==1 and native['model_calls']==0
assert native['script_sha256']==sha(root/recipe)
binary=Path(read('build-target.json')['binary']);assert native['binary_sha256']==sha(binary)
assert read('native-final/source-flow/result.json')['status']=='PASS'
agent=read('native-final/agent-after-reopen.json')
preview=read('native-final/preview-current.json')
assert agent['status']=='completed' and agent['context']['summary']['checked_before_start_at']
assert agent['context']['summary']['checked_source_sha256']==agent['context']['summary']['queued_source_sha256']
assert agent['context']['summary']['operator_snapshot_injected'] is False
assert len(agent['context']['summary']['received_message_ids'])==3 and len(agent['context']['summary']['omitted_message_ids'])==1
assert 'checked_before_start_at' not in agent['prepared_context']['summary']
assert agent['prepared_context']['sha256']==preview['context']['sha256']
assert read('native-final/preview-cli.json')['prompt']==read('native-final/preview-api.json')['prompt']
observed=(out/'native-final/provider-input.txt').read_bytes()
assert observed.decode()==agent['prompt'] and sha(out/'native-final/provider-input.txt')==agent['context']['sha256']
assert agent['context']['bytes']==len(observed)
assert 'work_criteria' not in agent['prompt']
marker='\nEXTRAITS DE CADRAGE — '
fragment=marker+agent['prompt'].split(marker,1)[1].split('\n',1)[0]+'\n'+agent['prompt'].split(marker,1)[1].split('\n',1)[1].split('\n',1)[0]+'\n'
assert hashlib.sha256(fragment.encode()).hexdigest()==agent['context']['summary']['attachment_sha256']
assert len(fragment.encode())==agent['context']['summary']['attachment_bytes']
block=json.loads(fragment.split('\n')[2]);assert block['selection_sha256']==agent['context']['summary']['selection_sha256']
sys.path.insert(0,str(root/'tools/memory-evaluation'))
import admission as A
source,annotations,reference=[read('native-final/current-'+name+'.json') for name in ['source','annotations','reference']]
choice=A.selection(source,annotations,reference,'Operator selected attributed history; inspect current evidence')
assert choice==read('native-final/current-choice.json')==agent['context']['summary']['selection']
expected=[e for e in reference['summary_entries'] if e['message']!='current-task-snapshot']
assert [(e['message'],e['text'],e['kind'],e['state']) for e in block['entries']]==[(e['message'],e['text'],e['kind'],e['state']) for e in expected]
assert [e['text'] for e in block['recent_messages']]==[e['content'] for e in reference['recent_messages']]
work=read('native-final/work-after.json');task=next(t for t in work['tasks'] if t['id']=='t1')
assert task['status']!='accepted' and 'summary_selection' not in task['launch_profile']
for directory in ['native','native-final']:
    fixture=Path(read(directory+'/fixture.json')['root'])
    assert (fixture/'claude.calls').read_text()=='call\ncall\n'
    assert (fixture/'worker/claude.calls').read_text()=='call\n'

before_files={**base['files'],**extension['files']}
for path,h in before_files.items():assert sha(out/'preimages'/path.replace('/','__'))==h
case_path='docs/plans/deepagents-integration/cas-evaluation.json'
old=json.loads((out/'preimages'/case_path.replace('/','__')).read_text());new=json.loads((root/case_path).read_text())
assert new['evaluations'][:-1]==old['evaluations']
assert {k:v for k,v in new.items() if k!='evaluations'}=={k:v for k,v in old.items() if k!='evaluations'}
assert new['evaluations'][-1]['id']=='memory-15-explicit-extractive-admission'
assert all(c['result']=='PARTIAL' for c in new['evaluations'][-1]['cases'])
added=['internal/engine/memory_summary_selection.go','internal/engine/memory_summary_selection_test.go','tools/memory-evaluation/admission.py','tests/test_memory_summary_admission.py',recipe,'docs/plans/deepagents-integration/INCREMENT-MEMOIRE-15.md','docs/plans/deepagents-integration/RESULTATS-MEMOIRE-15.md']
owned=[*before_files,*added];chunks=[]
for path in owned:
    before=(out/'preimages'/path.replace('/','__')).read_text() if path in before_files else ''
    current=(root/path).read_text()
    chunks.extend(difflib.unified_diff(before.splitlines(True),current.splitlines(True),fromfile='before/'+path,tofile='after/'+path))
    if path.endswith('.py'):ast.parse(current)
    if path.endswith('.md'):
        for target in re.findall(r'\]\(([^)#]+)(?:#[^)]*)?\)',current):
            if not target.startswith(('https:','http:')):assert ((root/path).parent/target).exists(),(path,target)
(out/'candidate.diff').write_text(''.join(chunks))
assert not subprocess.check_output(['gofmt','-l',*[p for p in owned if p.endswith('.go')]],cwd=root)
subprocess.run(['git','diff','--check'],cwd=root,check=True,capture_output=True)
for path in [*owned,str(Path(__file__).relative_to(root))]:
    check=subprocess.run(['git','diff','--no-index','--check','/dev/null',path],cwd=root,capture_output=True)
    assert check.returncode in [0,1] and not check.stdout and not check.stderr,path
runtime=json.loads(subprocess.check_output(['./swarm.sh','status'],cwd=root,text=True));assert runtime==base['runtime']
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()==base['head']
assert subprocess.check_output(['git','branch','--show-current'],cwd=root,text=True).strip()==base['branch']
(out/'input-reuse-assessment.json').write_text(json.dumps(reuse,indent=2)+'\n')
result={'status':'PASS','at_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'head':base['head'],'branch':base['branch'],'go_tests':21,'race_tests_subset':2,'python_tests':49,'m19_native_checks':5,'m18_native_checks':5,'final_native_scripted_invocations':3,'all_retained_native_scripted_invocations':6,'model_calls':0,'binary_sha256':sha(binary),'runtime':runtime,'deployed':False,'model_effectiveness':'NOT TESTED','independent_review':'NOT TESTED','operational_progressive_summary':'NOT IMPLEMENTED','owned_files_sha256':{p:sha(root/p) for p in owned},'candidate_diff_sha256':sha(out/'candidate.diff')}
(out/'audit-final.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
index={str(p.relative_to(out)):sha(p) for p in sorted(out.rglob('*')) if p.is_file() and p.name!='evidence-index.json'}
(out/'evidence-index.json').write_text(json.dumps(index,indent=2)+'\n')
print(json.dumps({k:result[k] for k in ['status','go_tests','race_tests_subset','python_tests','m19_native_checks','m18_native_checks','model_calls','deployed']}))
