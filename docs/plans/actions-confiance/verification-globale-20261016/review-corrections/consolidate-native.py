import json,hashlib,datetime
from pathlib import Path
root=Path.cwd();out=Path(__file__).parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
inputs=json.loads((out/'final-inputs-before.json').read_text());groups=[];names=set();excluded=[]
for name in ['effects-data','identity-process-proof']:
 log=out/(name+'-final.jsonl');events=[json.loads(s) for s in log.read_text().splitlines()];terminal=[e for e in events if not e.get('Test') and e.get('Action') in ['pass','fail']]
 assert terminal and terminal[-1]['Action']=='pass';assert not any(e.get('Action')=='fail' for e in events)
 passed={e['Test'] for e in events if e.get('Action')=='pass' and e.get('Test')}
 skip=passed & {'TestActionsConfianceT4SealedHelper','TestActionsConfianceT6LaboratoryHelper'};excluded+=sorted(skip);names|=passed-skip
 process=json.loads((out/(name+'-final-process.json')).read_text())
 groups.append(dict(name=name,log=str(log),sha256=sha(log),named_passed=len(passed),excluded=sorted(skip),structured_package_result=terminal[-1],exit_code=process.get('exit_code'),exit_code_observed='exit_code' in process))
assert all(sha(root/k)==v for k,v in inputs.items() if k.endswith('.go'))
d=dict(schema_version=1,scope='F1/F2 corrections and T2/T3/T4/T5/T6 regression groups',verified_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),input_sha256=inputs,go_tests=dict(meaningful=len(names),principal=sum('/' not in n for n in names),subcases=sum('/' in n for n in names),named_passed=sorted(names),single_race_execution=False,empty_fixture_excluded=excluded),groups=groups,source_freeze='All frozen inputs matched at each group completion. UI decoder corrected afterwards; Go unchanged.',limitations=['Second Go package emitted PASS; its command exit code was not observed after wrapper SIGTERM. Wrapper exit143 preserved.'],independent_assessment=False,global_compliance=False)
f=out/'native-verification.json';assert not f.exists();f.write_text(json.dumps(d,indent=2)+'\n');print(json.dumps({k:d[k] for k in ['scope','limitations']}));print(d['go_tests']['meaningful'],d['go_tests']['principal'],d['go_tests']['subcases'])
