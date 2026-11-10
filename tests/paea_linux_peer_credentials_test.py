#!/usr/bin/env python3
"""Actual default binary: connected peer witness then invisible PID0 refusal.

Required env SWARM_PAEA_PEER_BINARY/_SHA256/_EVIDENCE. Ephemeral unprivileged
user/PID/mount namespaces only, invalid grant byte and absent private Root. No
provider, host policy/account change, authority/effect or confinement acceptance.
No watchdog/automatic retry. Retain actual argv/stdout/stderr/exit and raw kernel
observation; future tests read current source bytes, not a frozen local script.
"""
from pathlib import Path
import hashlib
import json
import os
import socket
import struct
import subprocess
import sys
import tempfile

BINARY = Path(os.environ['SWARM_PAEA_PEER_BINARY']).resolve()
BINARY_SHA = os.environ['SWARM_PAEA_PEER_BINARY_SHA256']
OUT = Path(os.environ['SWARM_PAEA_PEER_EVIDENCE']).resolve()
OUT.mkdir(parents=True, exist_ok=False)
COUNTER = 0


def keep(path, raw):
    """Atomically preserve owned evidence; partial writes leave original intact."""
    fd, temporary = tempfile.mkstemp(prefix='.peer-evidence-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        assert Path(temporary).read_bytes() == raw
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def run(name, argv, channel=None, pass_fds=()):
    """No timeout or paid/live operation; verify expected normal ELF before child."""
    global COUNTER
    assert hashlib.sha256(BINARY.read_bytes()).hexdigest() == BINARY_SHA
    COUNTER += 1
    result = subprocess.run(argv, stdin=channel, pass_fds=pass_fds, capture_output=True)
    stem = OUT / f'{COUNTER:02d}-{name}'
    keep(stem.with_suffix('.stdout'), result.stdout)
    keep(stem.with_suffix('.stderr'), result.stderr)
    keep(stem.with_suffix('.json'), json.dumps({'argv': argv, 'exit': result.returncode}, indent=2).encode())
    return result


LEADER = '''import subprocess,sys,json,os,socket,struct
fd=int(sys.argv[1])
with socket.socket(fileno=fd)as s:
 raw=s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12)
 values=list(struct.unpack('3i',raw))
 assert values[0]==0 and os.getpid()==1
 p=subprocess.run(sys.argv[2:],stdin=s,capture_output=True)
 print(json.dumps({'child_exit':p.returncode,'stdout':p.stdout.decode(errors='replace'),'stderr':p.stderr.decode(errors='replace'),'leader_pid':os.getpid(),'raw_peer':values}))
'''


def witness_before_zero_peer(gate, refusal):
    """Same actual connected socket/parent witness, then invisible creator child.

    A known invalid grant byte ensures both cases remain before any Root access.
    Explicit safe PID0 refusal must replace errno0, not claim a business success.
    """
    with tempfile.TemporaryDirectory(prefix='paea-peer-credentials-', dir='/tmp') as directory:
        root = Path(directory) / 'root-must-stay-absent'
        argv = [str(BINARY), gate, 'synthetic-peer-credentials', '--root', str(root), '--json', '--lang', 'en']
        a, b = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            raw = b.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12)
            pid, uid, gid = struct.unpack('3i', raw)
            assert pid == os.getpid() and uid == os.geteuid() and gid == os.getegid()
            a.sendall(b'\x00')
            result = run(gate + '-valid-peer', argv, channel=b)
            assert result.returncode and refusal.encode() in result.stderr
            assert not root.exists()
        finally:
            a.close()
            b.close()
        a, b = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            a.sendall(b'\x00')
            launch = ['unshare', '--user', '--map-root-user', '--pid', '--fork', '--mount-proc',
                      sys.executable, '-c', LEADER, str(b.fileno()), *argv]
            result = run(gate + '-invisible-peer', launch, pass_fds=(b.fileno(),))
            assert result.returncode == 0, result.stderr.decode()
            observed = json.loads(result.stdout)
            assert observed['raw_peer'][0] == 0 and observed['child_exit'] != 0
            assert 'linux-peer-credentials-invalid' in observed['stderr']
            assert 'os-errno:0' not in observed['stderr']
            assert not observed['stdout'] and not root.exists()
        finally:
            a.close()
            b.close()


for name, refusal in [('_planning_gate', 'planning-gate-release-invalid'),
                      ('_validation_gate', 'task-validation-gate-grant-invalid'),
                      ('_reviewer_gate', 'review-gate-release-invalid')]:
    witness_before_zero_peer(name, refusal)
    print('PASS connected-witness-then-zero-peer', name, flush=True)
keep(OUT / 'result.json', json.dumps({'status': 'PASS', 'cases': 3, 'commands': COUNTER,
     'binary_sha256': BINARY_SHA, 'scope': 'Raw connected witness and PID0 explicit gate refusal, no Root/grant/effect; no full attributed-channel/OS/human qualification'}, indent=2).encode())
print('LINUX_PEER_NATIVE_PASS cases=3', flush=True)
