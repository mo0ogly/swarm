#!/usr/bin/env python3
"""Public owned-fixture diagnostic: unavailable is an explicit outcome, not R20 PASS."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import signal
import subprocess
import tempfile
import time

p=argparse.ArgumentParser()
p.add_argument('--binary',type=Path,required=True)
p.add_argument('--evidence-dir',type=Path,required=True)
p.add_argument('--timeout-seconds',type=int,default=120,help='recipe watchdog only')
a=p.parse_args();assert a.timeout_seconds>0
binary=a.binary.resolve();out=a.evidence_dir.resolve();out.mkdir(parents=True,exist_ok=True)
root=Path(tempfile.mkdtemp(prefix='swarm-kernel-public-'));checks=[]
tokens={'admin':'kernel-public-admin-fixture-token','alice':'kernel-public-requester-fixture-token','bob':'kernel-public-approver-fixture-token','effect':'kernel-public-effect-fixture-token'}

def cli(action=None,data=None,token='',server=None,session='',expected=0):
    argv=[str(binary),'--root',str(root),'--json']
    if server:argv+=['--server',server]
    argv+=['trust',action] if action else ['init']
    if data is not None:argv+=['--input','-']
    r=subprocess.run(argv,input=json.dumps(data) if data is not None else None,text=True,capture_output=True,
                     env=dict(os.environ,SWARM_TRUST_TOKEN=token,SWARM_WEB_SESSION=session),timeout=a.timeout_seconds)
    if r.returncode!=expected:
        path=out/'kernel-public-failure.json'
        fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
        with os.fdopen(fd,'w') as f:json.dump(dict(operation=action,expected=expected,observed=r.returncode,stdout=r.stdout,stderr=r.stderr),f)
        raise AssertionError('Unexpected public result; private diagnostic saved')
    return json.loads(r.stdout) if r.stdout.strip() else None

cli()
config=dict(schema_version=1,event_id='kernel-public-init',expected_revision=0,
            policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=365,require_separation=True),
            identities=[dict(subject=k,token=v,integrity=3 if k=='admin' else 1 if k=='alice' else 2,
                             roles=[{'admin':'admin','alice':'requester','bob':'approver','effect':'effect'}[k]]) for k,v in tokens.items()],
            reason='isolated public kernel diagnostic recipe')
cli('config-apply',config)
state=root/'.swarm/trust-actions.json';before=state.read_bytes()
cli('kernel-check',token=tokens['admin'],expected=4)
assert state.read_bytes()==before
checks.append('inactive policy refuses without state mutation')
config.update(expected_revision=1,event_id='kernel-public-enable',kernel_check=dict(enabled=True,timeout_seconds=17,ring_bytes=131072))
preview=cli('config-preview',config)
assert preview['kernel_check']==config['kernel_check'] and state.read_bytes()==before
cli('config-apply',config)
assert cli('show')['kernel_check']==config['kernel_check']
checks.append('preview is read-only; selected policy persists and reopens')
before=state.read_bytes()
cli('kernel-check',{'pid':1},tokens['admin'],expected=2)
cli('kernel-check',token=tokens['alice'],expected=4)
assert state.read_bytes()==before
checks.append('caller targets and non-admin principals refuse without mutation')

def unavailable(result):
    assert result['available'] is False and result['source']=='ebpf'
    assert result['scope']=='owned_fixture_only' and result['events']==[]
    assert result['fixture_verified'] is False and result['global_qualification'] is False
    assert result['cleanup_verified'] is True and len(result['owned_pids'])==1
    assert result['effective_policy']==config['kernel_check']
    assert result['reason'] in ('permission_denied','backend_unavailable','architecture_unsupported','main_thread_unavailable')
    assert state.read_bytes()==before

local=cli('kernel-check',token=tokens['admin'],expected=3)
unavailable(local)
(out/'kernel-local-unavailable.json').write_text(json.dumps(local,indent=2)+'\n')
checks.append('local public CLI reports unavailable with exit 3 and no synthetic events')
server=subprocess.Popen([str(binary),'--root',str(root),'web','127.0.0.1:0'],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,start_new_session=True)
try:
    stat=Path(f'/proc/{server.pid}/stat').read_text();start=stat[stat.rfind(')')+2:].split()[19]
    (out/'kernel-public-process.json').write_text(json.dumps(dict(pid=server.pid,start=start,binary=str(binary),root=str(root)),indent=2)+'\n')
    selector=selectors.DefaultSelector();selector.register(server.stdout,selectors.EVENT_READ)
    log=b'';deadline=time.monotonic()+a.timeout_seconds
    while b'/session/' not in log:
        remaining=deadline-time.monotonic();assert remaining>0 and selector.select(remaining),'owned startup timeout'
        chunk=os.read(server.stdout.fileno(),8192);assert chunk,'owned server exited';log+=chunk
    selector.close()
    match=re.search(rb'http://(127\.0\.0\.1:[0-9]+)/session/(session-[a-zA-Z0-9]+)',log);assert match
    origin,session='http://'+match[1].decode(),match[2].decode()
    remote=cli('kernel-check',token=tokens['admin'],server=origin,session=session,expected=3)
    unavailable(remote)
    (out/'kernel-remote-unavailable.json').write_text(json.dumps(remote,indent=2)+'\n')
    checks.append('remote CLI preserves API unavailability and exit 3 without local fallback')
    cli('kernel-check',token=tokens['alice'],server=origin,session=session,expected=4)
    cli('kernel-check',{'command':'anything'},tokens['admin'],origin,session,expected=2)
    assert state.read_bytes()==before
    checks.append('real API refuses wrong authority and arbitrary command without mutation')
finally:
    if server.poll() is None:
        os.killpg(server.pid,signal.SIGTERM)
        try:server.wait(timeout=5)
        except subprocess.TimeoutExpired:os.killpg(server.pid,signal.SIGKILL);server.wait(timeout=5)
    server.stdout.close()
proof=dict(schema_version=1,checks=checks,binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),server_stopped=server.poll() is not None,native_positive_qualified=False,public_unavailable_exit=3,root=str(root))
(out/'kernel-public.json').write_text(json.dumps(proof,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(dict(checks=len(checks),public_unavailable_exit=3,server_stopped=proof['server_stopped'])))
