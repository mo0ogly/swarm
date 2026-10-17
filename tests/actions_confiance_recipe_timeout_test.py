import ast,datetime,errno,hashlib,json,os,signal,subprocess,sys,tempfile
from pathlib import Path
repo=Path(__file__).resolve().parents[1];p=Path(sys.argv[1]);p.mkdir(parents=True,exist_ok=True);source=(repo/'tests/actions_confiance_t2.py').read_text();fn=next(n for n in ast.parse(source).body if isinstance(n,ast.FunctionDef) and n.name=='bounded_browser');namespace={'os':os,'signal':signal,'subprocess':subprocess,'repo':repo};exec(compile(ast.Module(body=[fn],type_ignores=[]),'bounded_browser','exec'),namespace)
run=namespace['bounded_browser'];assert run([sys.executable,'-c','print("completed")'],2).stdout.strip()=='completed'
root=Path(tempfile.mkdtemp(prefix='swarm-public-timeout-guard-'));pidfile=root/'child.pid';code='import subprocess,time,pathlib; c=subprocess.Popen(['+repr(sys.executable)+',"-c","import time;time.sleep(60)"]);pathlib.Path('+repr(str(pidfile))+').write_text(str(c.pid));time.sleep(60)'
try:run([sys.executable,'-c',code],1)
except AssertionError as e:assert 'owned process group terminated' in str(e)
else:raise AssertionError('timeout was not enforced')
pid=int(pidfile.read_text())
def process_state(read):
 try:return read().split(') ',1)[1].split()[0]
 except OSError as e:
  if e.errno in [errno.ENOENT,errno.ESRCH]:return 'gone'
  raise
for missing in [errno.ENOENT,errno.ESRCH]:
 def absent():raise OSError(missing,'process disappeared')
 assert process_state(absent)=='gone'
def inaccessible():raise OSError(errno.EACCES,'permission denied')
try:process_state(inaccessible)
except PermissionError:pass
else:raise AssertionError('permission error hidden')
assert process_state(lambda:'1 (fixture) Z 0')=='Z'
assert process_state(lambda:'1 (fixture) S 0')=='S'
state=process_state(lambda:(Path('/proc')/str(pid)/'stat').read_text())
assert state in ['gone','Z'],state
v={'missing_errno_guard_cases':[errno.ENOENT,errno.ESRCH],'permission_error_propagated':True,'normal_completion':True,'timeout_rejected':True,'child_not_running':True,'observed_child_state':state,'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'at':datetime.datetime.now(datetime.timezone.utc).isoformat()};(p/'public-timeout-cleanup-guard.json').write_text(json.dumps(v,indent=2)+'\n');print(v)
