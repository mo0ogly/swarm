#!/usr/bin/env python3
"""Kernel channel fixture: normal reader witness then invisible sender PID0.

Explicit SWARM_PAEA_CHANNEL_PROBE/_SHA256/_EVIDENCE. Test-only declared example,
not production daemon or OS separation acceptance. Configure/readiness handshake
precedes each synthetic send. Unprivileged self-mapped namespaces only, no host
accounts/policies/Root/provider operation. No watchdog or automatic retry.
"""
from pathlib import Path
import hashlib
import json
import os
import socket
import subprocess
import tempfile

PROBE = Path(os.environ['SWARM_PAEA_CHANNEL_PROBE']).resolve()
EXPECTED = os.environ['SWARM_PAEA_CHANNEL_PROBE_SHA256']
OUT = Path(os.environ['SWARM_PAEA_CHANNEL_EVIDENCE']).resolve()
OUT.mkdir(parents=True, exist_ok=False)


def keep(name, raw):
    """Publish immutable owned evidence atomically, retaining original on failure."""
    target = OUT / name
    assert not target.exists()
    fd, temporary = tempfile.mkstemp(prefix='.channel-', dir=OUT)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        assert Path(temporary).read_bytes() == raw
        os.rename(temporary, target)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def case(name, argv, expected_label):
    """Release only after actual configured reader readiness; retain both channels.

    Parent sends through the same real socketpair under normal and namespaced
    readers. Peer PID is never supplied as authority. Always close pair/reap child.
    """
    assert hashlib.sha256(PROBE.read_bytes()).hexdigest() == EXPECTED
    a, b = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET)
    process = None
    try:
        process = subprocess.Popen(argv, stdin=b, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        ready = process.stdout.readline()
        if ready == b'CHANNEL_READY\n':
            a.sendall(b'witness')
        rest, stderr = process.communicate()
        stdout = ready + rest
        keep(name + '.stdout', stdout)
        keep(name + '.stderr', stderr)
        keep(name + '.json', json.dumps({'argv': argv, 'exit': process.returncode,
             'binary_sha256': EXPECTED}, indent=2).encode())
        assert ready == b'CHANNEL_READY\n', stderr.decode(errors='replace')
        assert process.returncode == 0, stderr.decode(errors='replace')
        assert expected_label.encode() in stdout and b'CHANNEL_NO_FD_LEAK' in stdout
        assert not stderr
        assert hashlib.sha256(PROBE.read_bytes()).hexdigest() == EXPECTED
    finally:
        a.close()
        b.close()
        if process is not None and process.poll() is None:
            process.communicate()
    print('PASS', name, flush=True)


case('connected-visible-subject', [str(PROBE), 'valid'], 'CHANNEL_VALID')
case('invisible-message-pid-zero', ['unshare', '--user', '--map-root-user', '--pid',
     '--fork', '--mount-proc', str(PROBE), 'invisible'], 'CHANNEL_INVISIBLE_REFUSED')
keep('result.json', json.dumps({'status': 'PASS', 'cases': 2, 'probe_sha256': EXPECTED,
     'scope': 'Test-only FFI boundary, positive same-host subject and PID0 refusal, explicit FD accounting; no listener/admission/human/OS/global acceptance'}, indent=2).encode())
print('IDENTITY_CHANNEL_NATIVE_PASS cases=2', flush=True)
