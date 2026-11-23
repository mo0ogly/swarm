"""Run isolated business receiver qualification against the real Rust sender."""
from contextlib import chdir
import hashlib
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import sys

from receiver import peer

base = Path(__file__).resolve().parent
SCOPE = ("fictional-invoices", "qualification", "local-receiver", "ref")


def connect(work, database, sender_binary, cases, invoice, label, *, fault="none", mode="valid", raw=None, wrong_peer=False, wrong_host=False):
    """Start a fresh peer and check kernel identity before handing off its FD."""
    case = work / ("case-" + str(len(cases)))
    case.mkdir()
    endpoint = "inbox.sock"
    receiver = subprocess.Popen(
        [sys.executable, str(base / "receiver.py"), str(endpoint), str(database),
         json.dumps(SCOPE), invoice, "10", fault, str(os.getpid() + int(wrong_host))],
        cwd=case, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    try:
        assert receiver.stdout.readline() == b"ready\n"
        with socket.socket(socket.AF_UNIX) as channel:
            channel.settimeout(5)
            with chdir(case):
                channel.connect(endpoint)
            expected = (receiver.pid + int(wrong_peer), os.getuid())
            if peer(channel) != expected:
                assert wrong_peer
                observation = {"state": "peer_refused"}
            elif raw is not None:
                channel.sendall(raw)
                channel.shutdown(socket.SHUT_WR)
                observation = {"state": "raw_probe", "reply": channel.recv(33).hex()}
            else:
                # settimeout makes the OS descriptor nonblocking in Python.
                # Rust configures its own blocking recv/send deadlines.
                channel.setblocking(True)
                sender = subprocess.run(
                    [str(sender_binary), invoice, mode], cwd=case, stdin=channel, capture_output=True,
                )
                assert sender.returncode == 0 and sender.stderr == b"", (
                    sender.returncode, sender.stderr.decode(errors="replace")
                )
                observation = json.loads(sender.stdout)
        output, err = receiver.communicate()
        assert err == b"", err
    finally:
        if receiver.poll() is None:
            receiver.kill()
            receiver.wait()
    observation.update(label=label, receiver=output.decode().strip(), receiver_exit=receiver.returncode)
    cases.append(observation)
    return observation


def readback(database, observed_at=None):
    """Independently reopen storage and read records or purge at a fixture tick."""
    with sqlite3.connect(database) as db:
        if observed_at is not None:
            # The demonstration retention is five logical ticks, solely fixture data.
            db.execute("DELETE FROM inbox WHERE stored_at + ? <= ?", (5, observed_at))
        return db.execute("SELECT invoice, matches, hex(digest) FROM inbox ORDER BY invoice").fetchall()


def exercise(work, sender_binary):
    """Observe persistence, exact receipts, denials, crashes and scoped replay."""
    cases = []
    database = work / "records.sqlite"
    with sqlite3.connect(database) as db:
        db.execute("CREATE TABLE inbox(scope TEXT, invoice TEXT, matches INTEGER NOT NULL,"
                   "digest BLOB NOT NULL, stored_at INTEGER NOT NULL, PRIMARY KEY(scope, invoice))")
    result = connect(work, database, sender_binary, cases, "same", "match")
    assert result["state"] == "confirmed" and result["receiver"] == "recorded", result
    assert readback(database) == [("same", 1, result["digest"].upper())]
    for mode in ["wrong-project", "wrong-batch", "wrong-audience", "wrong-reference", "revoked", "expired"]:
        result = connect(work, database, sender_binary, cases, "different", mode, mode=mode)
        assert result["state"] == "refused_before_send" and result["receiver"] == "no_draft"
    result = connect(work, database, sender_binary, cases, "different", "wrong_peer", wrong_peer=True)
    assert result["state"] == "peer_refused" and result["receiver"] == "no_draft"
    result = connect(work, database, sender_binary, cases, "different", "wrong_host", wrong_host=True)
    assert result["state"] == "unconfirmed" and result["receiver"] == "wrong_host"
    assert [row[0] for row in readback(database)] == ["same"]
    result = connect(work, database, sender_binary, cases, "different", "unexpected_field",
                     raw=b'{"invoice_id":"different","account_matches":false,"account_token":"fictional"}')
    assert result["receiver"] == "invalid_draft" and result["reply"] == ""
    result = connect(work, database, sender_binary, cases, "different", "crash_before_commit", fault="before")
    assert result["state"] == "unconfirmed" and result["receiver_exit"] == 71
    assert [row[0] for row in readback(database)] == ["same"]
    result = connect(work, database, sender_binary, cases, "different", "crash_after_commit", fault="after")
    assert result["state"] == "unconfirmed" and result["receiver_exit"] == 72
    expected = b'{"invoice_id":"different","account_matches":false}'
    rows = readback(database)
    assert [(i, m) for i, m, _ in rows] == [("different", 0), ("same", 1)]
    assert rows[0][2] == hashlib.sha256(expected).hexdigest().upper()
    result = connect(work, database, sender_binary, cases, "different", "replay_after_readback")
    assert result["state"] == "confirmed" and result["receiver"] == "replayed"
    assert result["digest"].upper() == rows[0][2] and readback(database) == rows
    result = connect(work, database, sender_binary, cases, "different", "conflicting_replay",
                     raw=b'{"invoice_id":"different","account_matches":true}')
    assert result["receiver"] == "conflict" and result["reply"] == "" and readback(database) == rows
    with sqlite3.connect(database) as db:
        assert [item[1] for item in db.execute("PRAGMA table_info(inbox)")] == [
            "scope", "invoice", "matches", "digest", "stored_at"]
    assert readback(database, 14) == rows
    assert readback(database, 15) == [] and readback(database) == []
    result = {"status": "PASS", "cases": cases, "rows_before_purge": [list(row) for row in rows],
              "rows_after_purge_reopen": 0}
    receipt = json.dumps(result, indent=2)
    assert "fictitious-invoice-account" not in receipt and "account_token" not in receipt
    return result
