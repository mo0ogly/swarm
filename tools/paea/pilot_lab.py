#!/usr/bin/env python3
"""Executable PAEA effect fixtures, never a protected service or human approval.

Use only a fresh laboratory directory. SQLite serializes the cooperating fixture
authority and sink; both run with the caller's UID. Git atomically updates a
direct branch and an intent marker. No confinement, real identity, signature,
budget, network service, power-loss guarantee or production authority is supplied.
The public canonicalizer must be an explicitly pinned, already built binary.
"""
import argparse
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys


class Refused(Exception):
    """A laboratory precondition refused before this call's effect publication."""


def git(root, *args, stdin=None, allowed=(0,)):
    """Run local fixture Git without inherited Git routing/configuration or hooks.

    root must be the test-owned directory, never a user repository. Return the
    completed process; unexpected exits preserve stdout/stderr in the exception.
    There is no network, shell, retry, arbitrary deadline or output truncation.
    """
    env = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
    env.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull,
               GIT_AUTHOR_NAME='PAEA fixture', GIT_AUTHOR_EMAIL='fixture@invalid',
               GIT_COMMITTER_NAME='PAEA fixture', GIT_COMMITTER_EMAIL='fixture@invalid',
               GIT_AUTHOR_DATE='2000-01-01T00:00:00Z', GIT_COMMITTER_DATE='2000-01-01T00:00:00Z')
    command = ['git', '-c', 'core.hooksPath=' + os.devnull,
               '--git-dir=' + str(Path(root) / 'repository.git'), *args]
    result = subprocess.run(command, input=stdin, capture_output=True, env=env)
    if result.returncode not in allowed:
        raise RuntimeError({'command': command, 'exit': result.returncode,
                            'stdout': result.stdout.decode(errors='replace'),
                            'stderr': result.stderr.decode(errors='replace')})
    return result


def initialize(root, domain):
    """Exclusively create a payment/Git fixture and its persistent epoch/ledger.

    Refuse an existing path; partial initialization stays available for diagnosis.
    Return real fixture Git OIDs, or an empty object for payment. Seed values are
    test data, not operator policies. SQLite lock contention fails immediately
    (timeout=0); the laboratory has no hidden retry or product timeout setting.
    """
    if domain not in ('payment', 'git'):
        raise Refused('laboratory-domain-invalid')
    root = Path(root)
    root.mkdir()
    with closing(sqlite3.connect(root / 'sink.sqlite', timeout=0)) as db:
        db.executescript('''
            CREATE TABLE metadata(domain TEXT NOT NULL, epoch INTEGER NOT NULL);
            CREATE TABLE approvals(digest TEXT PRIMARY KEY, epoch INTEGER NOT NULL);
            CREATE TABLE effects(intent TEXT PRIMARY KEY, digest TEXT NOT NULL, canonical TEXT NOT NULL);
            CREATE TABLE ordering(seq INTEGER PRIMARY KEY, event TEXT NOT NULL, intent TEXT, epoch INTEGER NOT NULL);
        ''')
        db.execute('INSERT INTO metadata VALUES (?, 1)', (domain,))
        db.commit()
    if domain == 'payment':
        return {}
    git(root, 'init', '--bare', '--object-format=sha1')
    tree = git(root, 'hash-object', '-w', '-t', 'tree', '--stdin', stdin=b'').stdout.strip()
    old = git(root, 'commit-tree', tree.decode(), stdin=b'fixture base\n').stdout.strip().decode()
    candidate = git(root, 'commit-tree', tree.decode(), '-p', old, stdin=b'fixture candidate\n').stdout.strip().decode()
    other = git(root, 'commit-tree', tree.decode(), '-p', old, stdin=b'fixture substitution\n').stdout.strip().decode()
    git(root, 'update-ref', 'refs/heads/pilot', old)
    return {'old': old, 'candidate': candidate, 'other': other}


def canonicalize(raw):
    """Validate exact input bytes using the pinned offline public CLI, not Python JSON.

    Requires SWARM_PAEA_TEST_BINARY and its SHA256; refuses drift before/after the
    subprocess. No compilation, Root or live service is opened. Return the public
    data report. CLI, decoding and output-binding failures propagate, conferring
    neither authority nor permission. The caller supplies only laboratory data.
    """
    binary = Path(os.environ['SWARM_PAEA_TEST_BINARY']).resolve(strict=True)
    expected = os.environ['SWARM_PAEA_TEST_BINARY_SHA256']
    if hashlib.sha256(binary.read_bytes()).hexdigest() != expected:
        raise Refused('laboratory-binary-drift')
    result = subprocess.run([str(binary), 'protection', 'canonicalize', '--input', '-',
                             '--json', '--lang', 'en'], input=raw, capture_output=True)
    if hashlib.sha256(binary.read_bytes()).hexdigest() != expected:
        raise Refused('laboratory-binary-drift')
    if result.returncode != 0:
        raise Refused('canonicalization-refused: ' + result.stderr.decode(errors='replace'))
    value = json.loads(result.stdout)
    if (value['scope'] != 'offline-data-only' or value['authority'] != 'not-established'
            or hashlib.sha256(value['canonical'].encode()).hexdigest() != value['action_digest']):
        raise Refused('laboratory-canonical-report-invalid')
    return value


def connect(root, write=False):
    """Open an existing fixture DB in explicit ro/rw mode; never create on read.

    The caller owns closure and transactions. Missing files/locks/invalid schemas
    remain errors; there is no fallback to an empty sink or automatic retry.
    """
    uri = (Path(root) / 'sink.sqlite').resolve().as_uri() + ('?mode=rw' if write else '?mode=ro')
    return sqlite3.connect(uri, uri=True, timeout=0)


def refs(root):
    """Read actual local refs, preserving symbolic targets for refusal/reconciliation.

    No receipt/control journal is consulted. Git read failure propagates, never
    becoming an empty ref set. Returned tuples are object ID and symbolic target.
    """
    lines = git(root, 'for-each-ref', '--format=%(refname) %(objectname) %(symref)').stdout.decode().splitlines()
    return {parts[0]: (parts[1], parts[2] if len(parts) == 3 else '')
            for parts in (line.split(maxsplit=2) for line in lines)}


def authority(root, operation, report=None):
    """Fixture-only grant/revoke, serialized with cooperating commit operations.

    approve records a data digest at the current epoch; revoke advances the epoch.
    This is a deterministic test double, with no authenticated human or real
    mandate. Existing effects are preserved. SQL errors roll back and propagate.
    """
    db = connect(root, write=True)
    try:
        db.execute('BEGIN IMMEDIATE')
        epoch = db.execute('SELECT epoch FROM metadata').fetchone()[0]
        if operation == 'approve' and report is not None:
            db.execute('INSERT OR REPLACE INTO approvals VALUES (?, ?)', (report['action_digest'], epoch))
        elif operation == 'revoke':
            epoch += 1
            db.execute('UPDATE metadata SET epoch=?', (epoch,))
        else:
            raise Refused('laboratory-authority-operation-invalid')
        db.execute('INSERT INTO ordering(event,epoch) VALUES (?,?)', (operation, epoch))
        db.commit()
        return epoch
    finally:
        db.close()


def commit(root, report, fault_after_effect=False):
    """Consume a lab grant at current epoch and publish one stable-intent effect.

    report must come from canonicalize, never an agent-supplied digest. SQLite's
    write transaction orders lab revocation and first use. Payment inserts the
    exact canonical action under a unique intent. Git CAS updates branch+intent
    marker in one Git ref transaction; its SQL journal is not the effect oracle.
    An explicit fixture fault exits 86 after effect publication, before reply
    (for Git also before journal commit). Caller must retain UNKNOWN and reconcile.
    Refused is raised only before a new effect. Other errors may follow a Git
    effect and must remain UNKNOWN. No protected authority/isolation is claimed.
    """
    doc = json.loads(report['canonical'])
    action = doc['action']
    intent, digest = report['intent_id'], report['action_digest']
    db = connect(root, write=True)
    try:
        db.execute('BEGIN IMMEDIATE')
        domain, epoch = db.execute('SELECT domain,epoch FROM metadata').fetchone()
        if action['kind'] != {'payment': 'payment.settle', 'git': 'repository.promote'}[domain]:
            raise Refused('laboratory-domain-mismatch')
        grant = db.execute('SELECT epoch FROM approvals WHERE digest=?', (digest,)).fetchone()
        if grant is None:
            raise Refused('laboratory-approval-missing')
        if grant[0] != epoch:
            raise Refused('laboratory-epoch-revoked')
        previous = db.execute('SELECT digest FROM effects WHERE intent=?', (intent,)).fetchone()
        if previous:
            if previous[0] != digest:
                raise Refused('laboratory-intent-conflict')
            return {'state': 'COMMITTED', 'publication': 'already-recorded'}
        if domain == 'git':
            actual = refs(root)
            marker = 'refs/paea-lab/intents/' + intent
            target = action['target_ref']
            if actual.get(target, ('', ''))[1] or actual.get(marker, ('', ''))[1]:
                raise Refused('laboratory-symbolic-ref-forbidden')
            if marker in actual:
                if git(root, 'cat-file', 'blob', actual[marker][0]).stdout != report['canonical'].encode():
                    raise Refused('laboratory-intent-conflict')
                return {'state': 'COMMITTED', 'publication': 'git-marker-recorded'}
            if actual.get(target, ('', ''))[0] != action['expected_old_oid']:
                raise Refused('laboratory-git-cas-mismatch')
            for oid in (action['expected_old_oid'], action['candidate_oid']):
                if git(root, 'cat-file', '-t', oid).stdout.strip() != b'commit':
                    raise Refused('laboratory-git-object-not-commit')
            # The marker binds ALL canonical content, not merely candidate_oid:
            # after a crash the SQL journal may not exist, even for the same candidate.
            marker_oid = git(root, 'hash-object', '-w', '--stdin', stdin=report['canonical'].encode()).stdout.strip().decode()
            transaction = ('start\noption no-deref\nupdate ' + target + ' ' + action['candidate_oid']
                           + ' ' + action['expected_old_oid'] + '\ncreate ' + marker + ' '
                           + marker_oid + '\nprepare\ncommit\n')
            git(root, 'update-ref', '--stdin', stdin=transaction.encode())
            if fault_after_effect:
                os._exit(86)
        db.execute('INSERT INTO effects VALUES (?,?,?)', (intent, digest, report['canonical']))
        db.execute('INSERT INTO ordering(event,intent,epoch) VALUES (?,?,?)', ('commit', intent, epoch))
        db.commit()
        if fault_after_effect:
            os._exit(86)
        return {'state': 'COMMITTED', 'publication': 'new-effect'}
    finally:
        db.close()


def reconcile(root, report):
    """Read fixture sink state without changing effects, authority or reservations.

    Return a declared laboratory reconciliation, not a signed PAEA receipt.
    Missing/unreadable sink, attribution conflict or unexpected Git state gives
    UNKNOWN with its diagnostic. A missing marker plus unchanged old branch is
    the bounded Git negative witness; arbitrary external writers invalidate it.
    """
    db = None
    try:
        db = connect(root)
        domain = db.execute('SELECT domain FROM metadata').fetchone()[0]
        if domain == 'payment':
            row = db.execute('SELECT digest FROM effects WHERE intent=?', (report['intent_id'],)).fetchone()
            if row is None:
                return {'state': 'RECONCILED_NOT_COMMITTED'}
            if row[0] == report['action_digest']:
                return {'state': 'RECONCILED_COMMITTED'}
            return {'state': 'UNKNOWN', 'cause': 'laboratory-intent-conflict'}
        action = json.loads(report['canonical'])['action']
        actual = refs(root)
        marker = actual.get('refs/paea-lab/intents/' + report['intent_id'])
        target = actual.get(action['target_ref'])
        if marker is not None and marker[1] == '':
            exact = git(root, 'cat-file', 'blob', marker[0]).stdout == report['canonical'].encode()
            if exact:
                return {'state': 'RECONCILED_COMMITTED'}
            return {'state': 'UNKNOWN', 'cause': 'laboratory-intent-conflict'}
        if marker is None and target == (action['expected_old_oid'], ''):
            return {'state': 'RECONCILED_NOT_COMMITTED'}
        return {'state': 'UNKNOWN', 'cause': 'laboratory-sink-attribution-unavailable'}
    except (OSError, sqlite3.Error, RuntimeError, KeyError, ValueError, TypeError) as error:
        return {'state': 'UNKNOWN', 'cause': str(error)}
    finally:
        if db is not None:
            db.close()


def main():
    """Expose fixture commit only; failures retain conservative effect uncertainty.

    Explicit paths and pinned CLI are required. Exit 2 denotes a pre-effect
    refusal, 1 an error with possibly UNKNOWN effect, 86 deliberate post-effect
    crash. Output/flush failures also exit nonzero; no retry is authorized.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('document', type=Path)
    parser.add_argument('--fault-after-effect', action='store_true')
    args = parser.parse_args()
    try:
        report = canonicalize(args.document.read_bytes())
        result = commit(args.root, report, args.fault_after_effect)
        print(json.dumps({'scope': 'isolated-laboratory-only', **result}), flush=True)
        return 0
    except Refused as error:
        print(json.dumps({'state': 'DENIED', 'cause': str(error)}), file=sys.stderr, flush=True)
        return 2
    except (OSError, sqlite3.Error, RuntimeError, KeyError, ValueError, TypeError) as error:
        print(json.dumps({'state': 'UNKNOWN', 'cause': str(error)}), file=sys.stderr, flush=True)
        return 1


if __name__ == '__main__':
    sys.exit(main())
