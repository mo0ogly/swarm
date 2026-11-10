import datetime, hashlib, json, pathlib, re, subprocess, sys, time
root=pathlib.Path(__file__).resolve().parents[5]
out=pathlib.Path(__file__).resolve().parent
name=sys.argv[1]; argv=sys.argv[2:]
keys=set(json.loads((out.parent/'m16-20261016/build-final-inputs.json').read_text()))
keys.update(str(p.relative_to(root)) for p in (root/'internal/engine').glob('*.go'))
keys.update(['cmd/swarm/main.go','resources.go','go.mod','go.sum','tests/worker_recovery_projection.py',str(pathlib.Path(__file__).resolve().relative_to(root))])
keys.update(['tools/memory-evaluation/swarm_source.py','tests/test_memory_swarm_source.py','tools/memory-evaluation/summarization.py','tests/test_memory_summarization.py','tools/memory-evaluation/compare.py','tests/memory_summary_source.py'])
keys.update(['tools/memory-evaluation/admission.py','tests/test_memory_summary_admission.py','tests/memory_summary_admission.py'])
def inputs(): return {p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in sorted(keys) if (root/p).is_file()}
before=inputs(); (out/(name+'-inputs.json')).write_text(json.dumps(before,indent=2)+'\n')
started=datetime.datetime.now(datetime.timezone.utc).isoformat(); t=time.monotonic()
with (out/(name+'.log')).open('w') as log:
 r=subprocess.run(argv,cwd=root,stdout=log,stderr=subprocess.STDOUT)
after=inputs(); log=(out/(name+'.log')).read_text()
names=[s.split()[2] for s in log.splitlines() if s.startswith('--- PASS:') or s.startswith('--- FAIL:')]
names += re.findall(r'^(test_\w+) \([^\n]+?\) \.\.\.',log,re.M)
record={'argv':argv,'started_at_utc':started,'duration_seconds':round(time.monotonic()-t,3),'exit_code':r.returncode,'named_tests_ran':names,'changed_inputs':[p for p in sorted(set(before)|set(after)) if before.get(p)!=after.get(p)],'log_sha256':hashlib.sha256((out/(name+'.log')).read_bytes()).hexdigest(),'real_model_calls':0}
(out/(name+'.json')).write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(record)); print(log[-2600:])
sys.exit(r.returncode)
