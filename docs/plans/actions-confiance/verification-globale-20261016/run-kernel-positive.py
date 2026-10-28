#!/usr/bin/env python3
"""Execute one prepared owned-fixture diagnostic only after changed environment."""
import argparse,hashlib,json,os,signal,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('--prepared',type=Path,required=True)
p.add_argument('--environment-change',required=True,help='operator description; not an automatic authorization')
p.add_argument('--execute',action='store_true',help='perform exactly one native diagnostic; otherwise validate only')
p.add_argument('--watchdog-seconds',type=int,help='recipe watchdog only; required with --execute')
a=p.parse_args();base=a.prepared.resolve();d=json.loads((base/'prepared.json').read_text())
sha=lambda path:hashlib.sha256(Path(path).read_bytes()).hexdigest()
assert a.environment_change.strip(),'changed precondition must be described'
assert sha(d['binary'])==d['binary_sha256'],'candidate changed'
state=Path(d['root'])/'.swarm/trust-actions.json'
assert sha(state)==d['state_sha256'],'prepared policy/state changed'
status=dict(line.split(':',1) for line in Path('/proc/self/status').read_text().splitlines() if ':' in line)
environment=dict(uid=os.getuid(),kernel=os.uname().release,machine=os.uname().machine,cap_eff=status['CapEff'].strip(),unprivileged_bpf_disabled=Path('/proc/sys/kernel/unprivileged_bpf_disabled').read_text().strip(),perf_event_paranoid=Path('/proc/sys/kernel/perf_event_paranoid').read_text().strip())
if environment==d['environment']:
 print(json.dumps(dict(executed=False,available=False,reason='environment_unchanged',bpf_attempted=False)));raise SystemExit(3)
if not a.execute:
 print(json.dumps(dict(executed=False,environment_changed=True,bpf_attempted=False)));raise SystemExit(0)
assert a.watchdog_seconds and a.watchdog_seconds>0,'explicit recipe watchdog required'
dest=base/'native-positive-once';dest.mkdir(exist_ok=False)
argv=[d['binary'],'--root',d['root'],'--json','trust','kernel-check']
token_path=Path(d['root'])/'positive-admin-token'
assert not token_path.is_symlink() and token_path.stat().st_mode & 0o077==0,'private fixture credential required'
env=dict(os.environ,SWARM_TRUST_TOKEN=token_path.read_text())
fd=os.open(dest/'stdout',os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
err=os.open(dest/'stderr',os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
child=subprocess.Popen(argv,stdout=fd,stderr=err,env=env,start_new_session=True);os.close(fd);os.close(err)
stat=Path(f'/proc/{child.pid}/stat').read_text()
r=dict(argv=argv,pid=child.pid,start_ticks=stat[stat.rfind(')')+2:].split()[19],started_unix=time.time(),environment=environment,environment_change=a.environment_change,binary_sha256=d['binary_sha256'],recipe_watchdog_seconds=a.watchdog_seconds,status='running')
receipt=dest/'process.json';receipt.write_text(json.dumps(r,indent=2)+'\n')
try:code=child.wait(timeout=a.watchdog_seconds)
except subprocess.TimeoutExpired:
 os.killpg(child.pid,signal.SIGKILL);child.wait();r.update(status='recipe_watchdog_expired',exit_code=child.returncode);receipt.write_text(json.dumps(r,indent=2)+'\n');raise
r.update(exit_code=code,ended_unix=time.time(),status='completed');receipt.write_text(json.dumps(r,indent=2)+'\n')
assert sha(state)==d['state_sha256'],'business state changed'
assert code==0,'native positive did not pass; private originals retained, no retry'
v=json.loads((dest/'stdout').read_text())
assert v['available'] is True and v['fixture_verified'] is True and v['cleanup_verified'] is True
assert v['source']=='ebpf' and v['scope']=='owned_fixture_only' and v['global_qualification'] is False and v['lost']==0
assert v['effective_policy']==d['policy'] and len(v['owned_pids'])==1
events=v['events'];assert {'process','file','network'}<={e['event_type'] for e in events}
assert all(e['pid']==v['owned_pids'][0] and e['mission_id']=='diagnostic' for e in events)
pid=v['owned_pids'][0];proc=Path(f'/proc/{pid}/stat')
try:
 stat=proc.read_text();parts=stat[stat.rfind(')')+2:].split()
 assert parts[19]!=events[0]['pid_start_time'] or parts[0]=='Z','helper remained active'
except (FileNotFoundError,ProcessLookupError):pass
proof=dict(native_diagnostic_positive=True,executed=True,owned_fixture_only=True,business_state_unchanged=True,process_file_network_observed=True,helper_stopped=True,binary_sha256=d['binary_sha256'],event_count=len(events),stdout_sha256=sha(dest/'stdout'),global_qualification=False,remaining=['business attribution','foreign fixture exclusion','loss/restart','paired overhead'])
(dest/'verification.json').write_text(json.dumps(proof,indent=2)+'\n')
print(json.dumps(proof))

