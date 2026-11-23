"""Qualification-only receiver, invoked per connection within a private lab."""
import hashlib
import json
import os
import socket
import sqlite3
import struct
import sys


def peer(channel):
    """Read the Linux kernel's connected peer PID and UID."""
    return struct.unpack("3i", channel.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))[:2]


def fields(pairs):
    """Reject duplicate keys before the closed schema check."""
    result = dict(pairs)
    if len(result) != len(pairs):
        raise ValueError("duplicate fields")
    return result


def receive(channel, scope, invoice, db_path, tick, fault):
    """Validate at most 256 fixture bytes and ACK only after SQLite commit.

    Scope and expected invoice arrive in trusted supervisor arguments. A matching
    replay returns the same digest, while a changed payload under the same scope
    is rejected. Controlled crash points exercise rollback and reconciliation.
    """
    with channel.makefile("rb") as reader:
        data = reader.read(257)
    if not data:
        return "no_draft"
    try:
        if len(data) > 256:
            raise ValueError("oversized")
        draft = json.loads(data, object_pairs_hook=fields)
        if (type(draft) is not dict or set(draft) != {"invoice_id", "account_matches"}
                or draft["invoice_id"] != invoice or type(draft["account_matches"]) is not bool):
            raise ValueError("schema")
    except (UnicodeError, ValueError, RecursionError):
        return "invalid_draft"
    digest = hashlib.sha256(data).digest()
    with sqlite3.connect(db_path) as db:
        db.execute("PRAGMA synchronous=FULL")
        db.execute("BEGIN IMMEDIATE")
        row = db.execute("SELECT digest FROM inbox WHERE scope=? AND invoice=?", (scope, invoice)).fetchone()
        if row and row[0] != digest:
            return "conflict"
        if not row:
            db.execute("INSERT INTO inbox VALUES (?, ?, ?, ?, ?)",
                       (scope, invoice, draft["account_matches"], digest, tick))
        if fault == "before":
            os._exit(71)
        db.commit()
        if fault == "after":
            os._exit(72)
    channel.sendall(digest)
    channel.shutdown(socket.SHUT_WR)
    return "replayed" if row else "recorded"


def main():
    """Bind one private socket and serve only the actual supervisor process."""
    path, db_path, scope, invoice, clock, fault, host_pid = sys.argv[1:]
    with socket.socket(socket.AF_UNIX) as listener:
        listener.bind(path)
        os.chmod(path, 0o600)
        listener.listen(1)
        print("ready", flush=True)
        channel, _ = listener.accept()
        with channel:
            channel.settimeout(5)
            outcome = ("wrong_host" if peer(channel) != (int(host_pid), os.getuid())
                       else receive(channel, scope, invoice, db_path, int(clock), fault))
    print(outcome, flush=True)


if __name__ == "__main__":
    main()
