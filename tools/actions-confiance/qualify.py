import argparse
import json
import os
import re
from pathlib import Path
import subprocess
import sys

p=argparse.ArgumentParser()
p.add_argument('task',choices=['T2'])
p.add_argument('mode',choices=['race','portability'])
p.add_argument('--timeout-seconds',type=int,required=True)
a=p.parse_args()
root=Path.cwd().resolve()
local=root/'tools/actions-confiance/qualify.py'
source=Path(__file__).resolve()
if not local.is_file() or local.is_symlink() or local.read_bytes()!=source.read_bytes():
    raise SystemExit('Operator qualification differs from the candidate copy')
if a.timeout_seconds<1:
    raise SystemExit('Positive declared timeout required')
env=os.environ.copy()
if a.mode=='race':
    argv=[sys.executable,str(source.with_name('verify.py')),a.task,'delivery','--timeout-seconds',str(a.timeout_seconds)]
else:
    env['GOARCH']='386'
    env['CGO_ENABLED']='0'
    env.pop('GOFLAGS',None)
    argv=['go','test','./internal/engine','-json','-count=1','-run','^(TestActionsConfianceT2PolicyHasNoArbitraryBusinessCeilings|TestReviewTimeoutConfigurationPreservesBudgetAndHistory|TestWebRequestsInvalidAuthenticationAndConcurrentRevision)$','-timeout',str(a.timeout_seconds)+'s']
result=subprocess.run(argv,cwd=root,env=env,capture_output=True,text=True,timeout=a.timeout_seconds)
sys.stdout.write(result.stdout)
sys.stderr.write(result.stderr)
if result.returncode:
    raise SystemExit(result.returncode)
if a.mode=='race':
    env['CGO_ENABLED']='1'
    argv=['go','test','-race','./internal/...','-v','-count=1','-run','^TestActionsConfiance'+a.task,'-timeout',str(a.timeout_seconds)+'s']
    result=subprocess.run(argv,cwd=root,env=env,capture_output=True,text=True,timeout=a.timeout_seconds)
    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)
    if result.returncode:raise SystemExit(result.returncode)
if a.mode=='portability':
    events=[json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
    runs={e['Test'] for e in events if e.get('Action')=='run' and e.get('Test')}
    passes={e['Test'] for e in events if e.get('Action')=='pass' and e.get('Test')}
else:
    runs=set(re.findall(r'^=== RUN\s+(\S+)\s*$',result.stdout,re.M))
    passes=set(re.findall(r'^\s*--- PASS: (\S+) \(',result.stdout,re.M))
if not runs or not runs<=passes:
    raise SystemExit('Missing or incomplete named Go cases')
if a.mode=='portability':
    expected={'TestActionsConfianceT2PolicyHasNoArbitraryBusinessCeilings','TestReviewTimeoutConfigurationPreservesBudgetAndHistory','TestWebRequestsInvalidAuthenticationAndConcurrentRevision'}
    if runs!=expected or not runs<=passes:
        raise SystemExit('Missing or incomplete named 386 cases')
    print(json.dumps({'task':a.task,'mode':a.mode,'GOARCH':'386','named_go_tests':len(runs),'status':'pass'}))
else:
    print(json.dumps({'task':a.task,'mode':a.mode,'race':True,'named_go_tests':len(runs),'status':'pass'}))
