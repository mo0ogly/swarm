"""Disposable child changes only its own dumpability between explicit packets.
Linux PR_GET_DUMPABLE=3/PR_SET_DUMPABLE=4 from installed linux/prctl.h.
No UID mapping/account/host policy change here; unshare is a caller-selected
trusted fixture wrapper. All child state ends with this process.
"""
import ctypes
import socket
import sys
libc=ctypes.CDLL(None,use_errno=True)
libc.prctl.argtypes=[ctypes.c_int,ctypes.c_ulong,ctypes.c_ulong,ctypes.c_ulong,ctypes.c_ulong]
libc.prctl.restype=ctypes.c_int

def dumpable(value):
    """Set/check only this child; preserve errno on actual native refusal."""
    if libc.prctl(4,value,0,0,0)<0:
        raise OSError(ctypes.get_errno(),"fixture-set-dumpable")
    actual=libc.prctl(3,0,0,0,0)
    if actual<0:
        raise OSError(ctypes.get_errno(),"fixture-get-dumpable")
    if actual!=value:
        raise RuntimeError("fixture-dumpable-observation")

def publish(peer,value,stage):
    """One valid child-state packet, then exact parent release before next phase."""
    dumpable(value)
    if peer.send(stage)!=len(stage):
        raise RuntimeError("fixture-short-packet")
    if peer.recv(2)!=b"go":
        raise RuntimeError("fixture-parent-release")

with socket.socket(fileno=0)as peer:
    if sys.argv[1:]==["invalid-dumpable"]:
        publish(peer,1,b"before-fault")
        dumpable(2)
        raise RuntimeError("fixture-invalid-dumpability-was-accepted")
    if sys.argv[1:]:
        raise RuntimeError("fixture-mode")
    for value,stage in [(1,b"before"),(0,b"nondumpable"),(1,b"restored")]:
        publish(peer,value,stage)
