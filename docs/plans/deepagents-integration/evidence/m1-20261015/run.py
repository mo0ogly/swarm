from pathlib import Path
import datetime, hashlib, json, os, subprocess, sys
base=Path(__file__).parent.resolve()
label=sys.argv[1]
cwd=sys.argv[2]
argv=sys.argv[3:]
run_home=Path('/tmp/swarm-deepagents-memory-audit-home')
run_home.mkdir(exist_ok=True)
if label.startswith('swarm'):
    allowed=('PATH','HOME','GOCACHE','GOMODCACHE','GOPATH','CGO_ENABLED','CC','CXX','TMPDIR')
    env={k:os.environ[k] for k in allowed if k in os.environ}
else:
    env={'PATH':'/tmp/swarm-deepagents-memory-env-944lMS/bin:/usr/bin:/bin','HOME':str(run_home),'LANGSMITH_TRACING':'false','LANGCHAIN_TRACING_V2':'false','PYTEST_DISABLE_PLUGIN_AUTOLOAD':'1','PYTHONPATH':'/tmp/swarm-deepagents-audit-944lMS/libs/deepagents'}
start=datetime.datetime.now(datetime.timezone.utc).isoformat()
with (base/(label+'.log')).open('w') as output:
    proc=subprocess.run(argv,cwd=cwd,env=env,stdout=output,stderr=subprocess.STDOUT)
end=datetime.datetime.now(datetime.timezone.utc).isoformat()
meta={'executor':'Codex direct audit','argv':argv,'cwd':cwd,'environment_keys':sorted(env),'started_at_utc':start,'ended_at_utc':end,'exit_code':proc.returncode,'log_sha256':hashlib.sha256((base/(label+'.log')).read_bytes()).hexdigest()}
(base/(label+'.json')).write_text(json.dumps(meta,indent=2)+'\n')
print(json.dumps({'label':label,'exit_code':proc.returncode,'log':str(base/(label+'.log'))}))
print('\n'.join((base/(label+'.log')).read_text().splitlines()[-9:]))
sys.exit(proc.returncode)
