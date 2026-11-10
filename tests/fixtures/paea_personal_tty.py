"""Actual Linux PTY witness, synthetic command paths only, no secret input.

Requires a canonical PTY with enabled EOF character. Sends that EOF immediately:
without the terminal guard a broken implementation still returns a framing
refusal instead of hanging. No watchdog, provider, Root mutation or human session.
Prints only the child's already-redacted stdout/stderr/exit as test metadata.
"""
import json, os, pty, subprocess, sys, termios
master, slave = pty.openpty()
try:
    attrs = termios.tcgetattr(slave)
    assert attrs[3] & termios.ICANON
    eof = attrs[6][termios.VEOF]
    if isinstance(eof, int):
        eof = bytes([eof])
    assert len(eof) == 1 and eof != b"\0"
    child = subprocess.Popen(sys.argv[1:], stdin=slave, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    os.close(slave)
    slave = None
    os.write(master, eof)
    stdout, stderr = child.communicate()
    print(json.dumps({"exit": child.returncode, "stdout": stdout.decode(), "stderr": stderr.decode(), "terminal": True}))
finally:
    os.close(master)
    if slave is not None:
        os.close(slave)
