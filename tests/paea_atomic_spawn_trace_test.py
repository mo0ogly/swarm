#!/usr/bin/env python3
"""Trace one actual default-feature library integration test, not the daemon.
Retain compiler JSON, frozen test executable identity, native clone flags and
pidfd waits. No source mutation, global watchdog, provider or live database.
Caller must supply a fresh SWARM_PAEA_ATOMIC_TRACE_EVIDENCE directory.
"""
from pathlib import Path
import subprocess,os,json,re,hashlib
ROOT=Path(__file__).resolve().parents[1]
NAME="atomic_spawn_actual_subject_explicit_environment_directory_signals_and_wait"
def sha(p):
 """Hash exact retained/exercised bytes, never infer identity from a filename."""
 with Path(p).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(p,data):
 """Write only a fresh owned evidence file, rejecting proof replacement."""
 with p.open('xb')as f:f.write(data)
def complete_calls(raw):
 """Join strace unfinished/resumed records per traced TID/syscall, not by time."""
 pending={};out=[]
 for line in raw.splitlines():
  m=re.match(r"^(\d+)\s+(.*)$",line)
  if not m:continue
  tid,body=m.groups()
  if '<unfinished ...>'in body:
   name=body.split('(',1)[0];pending[(tid,name)]=body.replace('<unfinished ...>','');continue
  resumed=re.match(r"<\.\.\. (\w+) resumed>(.*)",body)
  if resumed:
   name,suffix=resumed.groups();body=pending.pop((tid,name),'')+suffix
  out.append((tid,body))
 return out

def main():
 """Require one actual passing test, atomic native creation and no child PID open."""
 target=Path(os.environ['SWARM_PAEA_ATOMIC_TRACE_EVIDENCE']);target.mkdir(parents=True,exist_ok=False)
 argv=['cargo','test','--manifest-path','rust/Cargo.toml','--offline','--locked','-p','swarm-linux-identity-channel','--test','atomic_spawn','--no-run','--message-format=json']
 run=subprocess.run(argv,cwd=ROOT,capture_output=True);save(target/'cargo.stdout',run.stdout);save(target/'cargo.stderr',run.stderr);assert run.returncode==0,'trace-cargo-prerequisite'
 rows=[json.loads(l)for l in run.stdout.decode().splitlines()if l.startswith('{')]
 artifacts=[r for r in rows if r.get('reason')=='compiler-artifact'];tests=[r for r in artifacts if r['target']['name']=='atomic_spawn'and r['profile']['test']and r.get('executable')];assert len(tests)==1
 core=[r for r in artifacts if r['target']['name']=='swarm_linux_identity_channel'and not r['profile']['test']];assert len(core)==1 and core[0]['features']==[],'trace-normal-library-profile'
 binary=Path(tests[0]['executable']);digest=sha(binary);trace=target/'syscalls.trace'
 command=['strace','-f','-qq','-e','trace=clone,clone3,pidfd_open,waitid,execve,getpid','-o',str(trace),str(binary),'--exact',NAME,'--nocapture']
 run=subprocess.run(command,cwd=ROOT,capture_output=True);save(target/'test.stdout',run.stdout);save(target/'test.stderr',run.stderr)
 assert run.returncode==0 and NAME+' ... ok'in run.stdout.decode()and'1 passed; 0 failed'in run.stdout.decode(),'trace-named-test-did-not-pass'
 calls=complete_calls(trace.read_text());creation=[]
 for tid,body in calls:
  if body.startswith(('clone(','clone3('))and'CLONE_PIDFD'in body:
   done=re.search(r'= (\d+)\s*$',body)
   if done:creation.append({'tid':tid,'child_pid':int(done.group(1)),'call':body})
 assert creation,'trace-missing-successful-atomic-CLONE_PIDFD'
 child_pids={r['child_pid']for r in creation};opens=[]
 for tid,body in calls:
  m=re.match(r'pidfd_open\((\d+),',body)
  if m:opens.append(int(m.group(1)))
 assert not child_pids.intersection(opens),'trace-forbidden-child-pid-capture'
 assert any(body.startswith('waitid(P_PIDFD')and'CLD_EXITED'in body for tid,body in calls),'trace-no-actual-pidfd-exit-wait'
 assert sha(binary)==digest,'trace-executable-changed'
 result={'status':'PASS','scope':'Traced library integration test, not installed daemon, human approval, distinct UID or global acceptance','compiler_argv':argv,'trace_argv':command,'test_binary_sha256':digest,'normal_library_artifact':core[0],'creation':creation,'pidfd_open_pids':opens,'trace_sha256':sha(trace),'global_watchdog':None}
 save(target/'result.json',(json.dumps(result,indent=2)+'\n').encode());print('ATOMIC_NATIVE_TRACE_PASS named_tests=1 native_atomic_creation='+str(len(creation)),flush=True)
if __name__=='__main__':main()
