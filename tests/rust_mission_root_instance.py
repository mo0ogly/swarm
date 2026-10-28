#!/usr/bin/env python3
"""Probe conductor/worker Root-instance continuity using owned coherent clones.

Witness and negative use the same post-dispatch SDK hook. Substitute .swarm or
only its SQLite family; readonly backup never copies live WAL/SHM bytes.
No authority SQL, policy changes, paid provider or global watchdog. A provider
entering a substituted instance is an explicit failing counterexample, never a
successful protection result. Non-provider failures need a precise refusal code.
"""
import argparse
import json
import os
from pathlib import Path
import selectors
import signal
import socket
import sqlite3
import stat
import subprocess
import sys
import time
import traceback

import rust_agent_model_route as fixture
import rust_mission_watch_worker as watch
import rust_planning_worker_public_flow as planning

check = watch.check
SQLITE_FAMILY = {'state.db', 'state.db-wal', 'state.db-shm', 'state.db-journal'}


def close_database(db, primary=None):
    """Close a diagnostic SQLite connection without hiding an earlier error."""
    try:
        db.close()
    except BaseException as error:
        if primary is not None:
            raise BaseExceptionGroup('snapshot observation/SQLite close', [primary, error])
        raise


def instance_snapshot(directory):
    """Read coherent SQL plus physical directory identity, changing no SQL.

    The dump is diagnostic private evidence, not a restore or permission source.
    Readonly transaction fixes its snapshot; journal mode is observed, not set.
    Do not print dump contents or treat unchanged labels as live authority.
    """
    local = directory.stat()
    db = sqlite3.connect('file:' + str(directory / 'state.db') + '?mode=ro', uri=True)
    try:
        db.execute('BEGIN')
        mode = db.execute('PRAGMA journal_mode').fetchone()[0]
        integrity = db.execute('PRAGMA integrity_check').fetchall()
        check(integrity == [('ok',)], 'owned snapshot SQLite integrity refused')
        dump = '\n'.join(db.iterdump()).encode()
        main = (directory / 'state.db').stat()
        return dict(identity=[local.st_dev, local.st_ino], database_identity=[main.st_dev, main.st_ino], journal_mode=mode,
                    sql_sha256=fixture.digest(dump), integrity='ok'), dump
    finally:
        close_database(db, sys.exception())


def private_files(directory):
    """Inventory private regular non-hardlinked files, excluding SQLite family.

    Refuse symlinks, foreign/multiple-link/nonprivate files and special nodes;
    no source is deleted or chmodded to make it pass. Record directories too.
    Live database/WAL/SHM/journal files are never copied as arbitrary byte files.
    """
    result = {}
    for path in sorted(directory.rglob('*')):
        relative = path.relative_to(directory)
        # SQLite owns volatile siblings; excluding by exact root name BEFORE
        # metadata prevents a disappearing WAL/SHM from faking a copy failure.
        if len(relative.parts) == 1 and relative.name in SQLITE_FAMILY:
            continue
        info = path.lstat()
        check(info.st_uid == os.getuid() and info.st_mode & 0o077 == 0,
              'owned private snapshot permissions mismatch')
        if stat.S_ISDIR(info.st_mode):
            result[str(relative)] = dict(kind='directory', mode=stat.S_IMODE(info.st_mode))
        else:
            check(stat.S_ISREG(info.st_mode) and info.st_nlink == 1,
                  'owned private snapshot file type/link mismatch')
            data = path.read_bytes()
            result[str(relative)] = dict(kind='file', mode=stat.S_IMODE(info.st_mode),
                sha256=fixture.digest(data), bytes=len(data))
    return result


def backup_database(directory, destination):
    """Backup one fixed readonly SQL snapshot into a fresh owned database file.

    Source main DB must be private/regular/nlink1; SQLite owns its volatile
    siblings. Never copy their bytes. Preserve both close errors and the primary;
    return observed mode and exact SQL dump, without changing any source SQL.
    """
    info = (directory / 'state.db').lstat()
    check(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and info.st_nlink == 1
          and info.st_mode & 0o077 == 0, 'owned source database mismatch')
    descriptor = os.open(destination, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    os.close(descriptor)
    source = sqlite3.connect('file:' + str(directory / 'state.db') + '?mode=ro', uri=True)
    target = None
    try:
        source.execute('BEGIN')
        mode = source.execute('PRAGMA journal_mode').fetchone()[0]
        source.execute('SELECT count(*) FROM sqlite_schema').fetchone()
        original_dump = '\n'.join(source.iterdump()).encode()
        target = sqlite3.connect(destination)
        source.backup(target)
    finally:
        pending = sys.exception()
        errors = []
        for db in (target, source):
            if db is not None:
                try:
                    db.close()
                except BaseException as error:
                    errors.append(error)
        if errors:
            raise BaseExceptionGroup('coherent backup/close', ([pending] if pending else []) + errors)
    return mode, original_dump


def coherent_clone(directory, destination, out):
    """Create a new-inode clone of only one owned fixture's .swarm directory.

    Backup from a readonly, fixed SQLite transaction; copy remaining private
    files with create-new writes, never hardlinks or raw WAL copies. Preserve
    source mode and verify SQL integrity/hash plus private-file stability/equality.
    Destination partial files remain on failure. This is not crash durability,
    same-UID authentication, SQLite-path confinement or a grant of authority.
    """
    started = dict(wall_ns=time.time_ns(), monotonic_ns=time.monotonic_ns())
    source_info = directory.lstat()
    check(stat.S_ISDIR(source_info.st_mode) and source_info.st_uid == os.getuid()
          and source_info.st_mode & 0o077 == 0, 'owned source directory mismatch')
    database_info = (directory / 'state.db').lstat()
    check(stat.S_ISREG(database_info.st_mode) and database_info.st_uid == os.getuid()
          and database_info.st_nlink == 1 and database_info.st_mode & 0o077 == 0,
          'owned source database type ownership or permissions mismatch')
    destination.mkdir(mode=stat.S_IMODE(source_info.st_mode), exist_ok=False)
    before = private_files(directory)
    mode, original_dump = backup_database(directory, destination / 'state.db')
    for relative, entry in before.items():
        path = destination / relative
        if entry['kind'] == 'directory':
            path.mkdir(mode=entry['mode'], exist_ok=False)
        else:
            raw = (directory / relative).read_bytes()
            check(fixture.digest(raw) == entry['sha256'], 'source private file changed during clone')
            with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, entry['mode']), 'wb') as stream:
                stream.write(raw)
                stream.flush()
                os.fsync(stream.fileno())
    check(private_files(directory) == before and private_files(destination) == before,
          'private clone inventory differs or source changed')
    clone, copied_dump = instance_snapshot(destination)
    check(copied_dump == original_dump, 'coherent clone SQL differs from source snapshot')
    check(clone['identity'] != [source_info.st_dev, source_info.st_ino], 'clone reused original directory instance')
    receipt = dict(original_identity=[source_info.st_dev, source_info.st_ino], clone=clone,
        source_journal_mode=mode, private_files=before,
        excluded_sqlite_files=sorted(name for name in SQLITE_FAMILY if (directory / name).exists()),
        source_sql_sha256=fixture.digest(original_dump), hardlinks_used=False,
        started=started, finished=dict(wall_ns=time.time_ns(), monotonic_ns=time.monotonic_ns()))
    fixture.publish(out / 'clone-sql.sql', original_dump)
    fixture.publish(out / 'clone-manifest.json', receipt)
    return receipt


def copy_private_archive(source, destination, manifest):
    """Archive baseline private bytes/modes for attribution, not installation.

    Create new files with no hardlinks; verify every hash. This evidence archive
    is needed if a worker changes a private file in the unchanged .swarm dir.
    It grants no new authority and does not mutate any source file or inode.
    """
    destination.mkdir(mode=0o700, exist_ok=False)
    for relative, entry in manifest.items():
        path = destination / relative
        if entry['kind'] == 'directory':
            path.mkdir(mode=entry['mode'])
        else:
            data = (source / relative).read_bytes()
            check(fixture.digest(data) == entry['sha256'], 'private baseline changed while archiving')
            with os.fdopen(os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, entry['mode']), 'wb') as stream:
                stream.write(data)
                stream.flush()
                os.fsync(stream.fileno())
    check(private_files(source) == manifest and private_files(destination) == manifest,
          'private baseline archive differs')


def prepare_database_variant(local, staging, original, out, state):
    """Prepare coherent DB and untouched-directory private baseline archives.

    No private archive is installed; fault changes only SQLite family pathnames.
    state is caller-owned partial-effect tracking, retained even on IO failure.
    """
    local_info = local.lstat()
    check(stat.S_ISDIR(local_info.st_mode) and local_info.st_uid == os.getuid()
          and local_info.st_mode & 0o077 == 0, 'DB-only source directory ownership mismatch')
    original.mkdir(mode=0o700)
    state.update(original=str(original), local_identity=[local.stat().st_dev, local.stat().st_ino],
                 database_identity=[(local / 'state.db').stat().st_dev, (local / 'state.db').stat().st_ino],
                 original_moved=[], installed=[], absent_original=[], installation_complete=False)
    state['backup_started'] = dict(wall_ns=time.time_ns(), monotonic_ns=time.monotonic_ns())
    manifest = private_files(local)
    state['baseline_private'] = manifest
    copy_private_archive(local, original / 'private-baseline', manifest)
    (original / 'database').mkdir(mode=0o700)
    staging.mkdir(mode=0o700)
    mode, dump = backup_database(local, staging / 'state.db')
    clone, copied = instance_snapshot(staging)
    check(dump == copied, 'DB-only backup differs from fixed source snapshot')
    check(private_files(local) == manifest, 'private files changed before DB-only injection')
    check((staging / 'state.db').stat().st_ino != (local / 'state.db').stat().st_ino,
          'DB-only backup reused main database inode')
    state['backup_finished'] = dict(wall_ns=time.time_ns(), monotonic_ns=time.monotonic_ns())
    receipt = dict(started=state['backup_started'], finished=state['backup_finished'], variant='database', source_journal_mode=mode, source_sql_sha256=fixture.digest(dump),
                   clone=clone, original_identity=state['local_identity'], database_identity=state['database_identity'])
    fixture.publish(out / 'clone-sql.sql', dump)
    fixture.publish(out / 'clone-manifest.json', receipt)
    return receipt


def install_database_variant(local, staging, original, out, state):
    """Preserve old SQLite family by rename, then install fresh backup family.

    Each rename is atomic individually; the family swap is NOT a transaction and
    crash/inter-rename races remain a fixture limitation. Exact volatile siblings
    may disappear before rename; record that absence, never ignore missing main
    DB or unrelated files. Retain partial effects in state before publication.
    """
    order = ['state.db-wal', 'state.db-shm', 'state.db-journal', 'state.db']
    for name in order:
        try:
            os.rename(local / name, original / 'database' / name)
        except FileNotFoundError:
            if name == 'state.db':
                raise
            state['absent_original'].append(name)
        else:
            state['original_moved'].append(name)
        fixture.publish(out / 'database-swap-progress.json', state)
    check(not any((local / name).exists() for name in SQLITE_FAMILY),
          'SQLite sibling reappeared during nontransactional preservation')
    for name in order:
        if (staging / name).exists():
            os.rename(staging / name, local / name)
            state['installed'].append(name)
            fixture.publish(out / 'database-swap-progress.json', state)
    check('state.db' in state['installed'], 'backup main database was not installed')
    check([local.stat().st_dev, local.stat().st_ino] == state['local_identity'],
          'DB-only substitution changed .swarm directory inode')
    check(private_files(local) == state['baseline_private'], 'DB-only injection changed private files')
    state['installed_database_identity'] = [(local / 'state.db').stat().st_dev, (local / 'state.db').stat().st_ino]
    state['installation_complete'] = True
    fixture.publish(out / 'database-swap-progress.json', state)


def restore_database_variant(local, original, retained, state, processes_closed):
    """Preserve exercised DB/private mutations, restore baseline only after reap.

    Retain original SQLite files by rename, never copy live journals. Archive
    every new/modified private path before restoring baseline bytes/modes. Private
    files restored from evidence need not retain inode/timestamps; .swarm and
    original main DB must retain theirs. No crash-atomic family restore claimed.
    Unknown closure refuses without any restoration mutation.
    """
    check(processes_closed, 'cannot restore DB while process closure is unknown')
    retained.mkdir(mode=0o700, exist_ok=False)
    (retained / 'database').mkdir(mode=0o700)
    current = private_files(local)
    baseline = state['baseline_private']
    fixture.publish(retained / 'private-before-restore.json', current)
    mutations = retained / 'private-mutations'
    mutations.mkdir(mode=0o700)
    moved = set()
    for relative, entry in current.items():
        path = Path(relative)
        if any(str(parent) in moved for parent in path.parents):
            continue
        if baseline.get(relative) != entry:
            target = mutations / relative
            target.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            os.rename(local / relative, target)
            moved.add(relative)
    for relative, entry in baseline.items():
        path = local / relative
        if not path.exists():
            if entry['kind'] == 'directory':
                path.mkdir(mode=entry['mode'], parents=True, exist_ok=True)
            else:
                raw = (original / 'private-baseline' / relative).read_bytes()
                check(fixture.digest(raw) == entry['sha256'], 'baseline archive changed before restitution')
                with os.fdopen(os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, entry['mode']), 'wb') as stream:
                    stream.write(raw)
                    stream.flush()
                    os.fsync(stream.fileno())
    for name in sorted(SQLITE_FAMILY):
        current_path = local / name
        if current_path.exists() and (state['installation_complete'] or name in state['installed']):
            os.rename(current_path, retained / 'database' / name)
    for name in state['original_moved']:
        check(not (local / name).exists(), 'unexpected SQLite file blocks original restitution')
        os.rename(original / 'database' / name, local / name)
    check([local.stat().st_dev, local.stat().st_ino] == state['local_identity']
          and [(local / 'state.db').stat().st_dev, (local / 'state.db').stat().st_ino] == state['database_identity'],
          'DB-only restitution did not preserve original directory/main DB identity')
    check(private_files(local) == baseline, 'DB-only private baseline not restored')
    collect_instance(local, retained, 'original-after-restore')
    if (retained / 'database/state.db').exists():
        collect_instance(retained / 'database', retained, 'clone-after-reap')
    fixture.publish(retained / 'restoration.json', dict(original_directory_restored=True,
        original_database_restored=True, private_baseline_restored=True, private_mutations=sorted(moved),
        private_inode_restoration_claimed=False, filesystem_transaction=False))


def restore_instances(root, original, retained_clone, processes_closed):
    """Restore the pathname only after actual owned process closure is known.

    Preserve both instances without deletion. Unknown runtime/cleanup forbids a
    restore; rename errors remain observable partial effects, never rollback.
    """
    check(processes_closed, 'cannot restore Root while process closure is unknown')
    if original.exists():
        check(not retained_clone.exists(), 'retained clone target already exists')
        if (root / '.swarm').exists():
            os.rename(root / '.swarm', retained_clone)
        os.rename(original, root / '.swarm')


def refusal_codes(document):
    """Read only the schema1 CLI Failure typed safe_causes vector.

    The real CLI serializes causes as Vec<String>, not nested code objects.
    Reject malformed schema/Failure/list types as no qualifying observation;
    never inspect message/error text or arbitrary nested lists/dictionaries.
    Empty/malformed observations cannot make clone_oracle pass.
    """
    if not isinstance(document, dict) or type(document.get('schema_version')) is not int or document['schema_version'] != 1:
        return set()
    if not isinstance(document.get('error'), str):
        return set()
    failure = document.get('failure')
    if not isinstance(failure, dict) or set(failure) != {'code', 'message', 'retryable', 'causes'}:
        return set()
    if not isinstance(failure['code'], str) or not isinstance(failure['message'], str) or failure['retryable'] is not False:
        return set()
    causes = failure['causes']
    if not isinstance(causes, list) or not all(isinstance(value, str) for value in causes):
        return set()
    return set(causes)


def clone_oracle(provider_entered, exit_code, expected_code, stderr, clone_unchanged):
    """Classify observed clone behavior without promoting an unrelated refusal.

    Provider entry always fails the instance-refusal requirement. Only a supplied
    exact source-defined instance guard code (never a wrapper), actual usage exit2
    and unchanged clone can
    qualify refusal; absent/malformed diagnostics remain inconclusive.
    """
    if provider_entered:
        return 'FAIL_PROVIDER_ENTERED_CLONE'
    try:
        codes = refusal_codes(json.loads(stderr))
    except (ValueError, TypeError):
        codes = set()
    # These exact existing guards compare physical instance association; generic
    # wrappers, missing legacy provenance and unsupported capability do not.
    specific_instance_codes = {'mission-agent-instance-mismatch', 'root-database-instance-changed'}
    if expected_code in specific_instance_codes and expected_code in codes and exit_code == 2 and clone_unchanged:
        return 'PASS_INSTANCE_REFUSAL'
    return 'INCONCLUSIVE_OTHER_REFUSAL'


def collect_instance(directory, out, label):
    """Retain full readonly SQL and private-file hashes for one physical instance."""
    receipt, dump = instance_snapshot(directory)
    receipt['private_files'] = private_files(directory)
    fixture.publish(out / (label + '.sql'), dump)
    fixture.publish(out / (label + '.json'), receipt)
    return receipt


def release_provider(root, nonce):
    """Publish only this fixture's exact physical release, never overwrite another."""
    path = root / 'watch-worker/release-worker.json'
    value = dict(nonce=nonce, release=True)
    if path.exists():
        check(json.loads(path.read_bytes()) == value, 'owned provider release collision')
    else:
        fixture.publish(path, value)


def known_child(cli, conductor_pid, binary, root):
    """Require owned actual helper identity before any failure-only direct signal."""
    current = watch.observation(cli['pid'])
    check(current['stamp'] == cli['stamp'] and current['ppid'] in (conductor_pid, os.getpid()),
          'cleanup helper identity or parent changed')
    argv = Path('/proc', str(cli['pid']), 'cmdline').read_bytes().split(b'\0')
    check(argv[:-1] == [os.fsencode(binary), b'_mission_worker_entry', b'watch-worker-agent', b'--root', os.fsencode(root), b'--json']
          and fixture.digest(Path('/proc', str(cli['pid']), 'exe').read_bytes()) == fixture.digest(binary.read_bytes()),
          'cleanup helper executable or exact owned arguments mismatch')


def json_equal(left, right):
    """Compare decoded JSON without Python's boolean/number coercion."""
    return json.dumps(left, sort_keys=True, allow_nan=False) == json.dumps(right, sort_keys=True, allow_nan=False)


def attach_probe_child(initial, conductor, binary, root):
    """Bind a live or SDK-owned zombie child without fabricating executable proof.

    Require actual UID/PID/start stamp/SDK parent around pidfd_open. A zombie
    must already signal departure both before and after reobservation; executable
    and argv then come ONLY from qualified SDK spawn metadata, not /proc/exe.
    A live child gets exact executable/argv observation. Return owned descriptor,
    current kernel identity and explicit proof scope; close errors retain primary.
    """
    before = watch.observation(initial['pid'], allow_terminal=True)
    check(before['stamp'] == initial['stamp'] and before['ppid'] == conductor['pid'], 'probe child stamp or SDK parent changed')
    descriptor = os.pidfd_open(initial['pid'])
    try:
        ready_before = select_ready(descriptor)
        after = watch.observation(initial['pid'], allow_terminal=True)
        check(after['stamp'] == initial['stamp'] and after['ppid'] == conductor['pid'], 'probe child reused or parent changed after pidfd')
        departed = after['state'] in ('Z', 'X')
        if before['state'] in ('Z', 'X') or departed:
            check(ready_before and select_ready(descriptor), 'owned zombie departure not established by pidfd')
            proof = dict(kernel_pid_stamp_parent_uid=True, kernel_departed=True,
                         executable_hash_reobserved=False, argv_reobserved=False,
                         executable_and_argv_source='qualified-sdk-spawn')
        else:
            known_child(after, conductor['pid'], binary, root)
            proof = dict(kernel_pid_stamp_parent_uid=True, kernel_departed=False,
                         executable_hash_reobserved=True, argv_reobserved=True,
                         executable_and_argv_source='kernel-and-qualified-sdk-spawn')
        return descriptor, after, proof
    except BaseException as error:
        watch.close_descriptor(descriptor, error)
        raise


def sdk_reason_nodes(variant):
    """Return exact typed source chains for stop/supervision on substituted Root.

    These node shapes are the inspected SDK/native_error_links protocol, not a
    policy allowlist for arbitrary nonzero exits. An extra/unknown node refuses.
    """
    def node(labels, sqlite=None):
        """Build one safe native classifier row with no raw error text."""
        return dict(labels=labels, sqlite_code=sqlite, io_errno=None)
    if variant == 'directory':
        return [node(['mission-storage-failed']), node(['sqlite-code:1032'], 1032), node([])]
    check(variant == 'database', 'unknown substituted SDK context')
    return [node(['mission-root-unavailable']), node(['root-database-instance-changed'])]


def classify_sdk_refusal(failure, cycle, spawned, observed, variant, sdk_code, worker_refused):
    """Recognize only complete typed substituted-context failures after child reap.

    SDK0/CLOSED is validated elsewhere. For nonzero require an exact instance
    worker refusal first, every top-level failure explained by inspected cycle
    wrappers and/or the one stop call, no IO/close/publication/permission errors.
    Known empty wrapper nodes are matched at exact source positions, not ignored.
    Unknown schema/labels/phases stay INCONCLUSIVE; no graceful/lease-stop claim.
    """
    try:
        check(worker_refused is True and type(sdk_code) is int and sdk_code != 0, 'SDK context classification lacks worker refusal')
        check(set(failure) == {'status', 'causes', 'child_ownership', 'worker_not_stopped_by_fixture'}
              and failure['status'] == 'FAIL' and failure['worker_not_stopped_by_fixture'] is True, 'SDK failure shape differs')
        check(observed['worker_exit_observed'] is True and observed['child_reaped_by_sdk'] is True
              and observed['supervisor_pid'] == spawned['supervisor']['pid'], 'SDK child reap observation differs')
        exit_fact = observed['exit_observation']
        check(json_equal(exit_fact, dict(code=2, signal=None, scope='probe-final-try-wait')), 'SDK did not reap refused CLI exit2')
        trace = failure['child_ownership']
        check(set(trace) == {'pid', 'phase', 'supervisor', 'initial_supervisor', 'session_state', 'conductor', 'early_exit',
                            'spawned_publication_attempted', 'spawned_publication_confirmed'}, 'SDK ownership trace shape differs')
        check(type(trace['pid']) is int and trace['pid'] == spawned['supervisor']['pid']
              and trace['phase'] == 'publish-probe-observation' and trace['session_state'] == 'pending'
              and trace['spawned_publication_attempted'] is True and trace['spawned_publication_confirmed'] is True
              and json_equal(trace['supervisor'], spawned['supervisor'])
              and json_equal(trace['initial_supervisor'], spawned['supervisor'])
              and json_equal(trace['conductor'], spawned['conductor']) and json_equal(trace['early_exit'], exit_fact),
              'SDK ownership trace does not match actual spawned/reaped child')
        reason = sdk_reason_nodes(variant)
        phases = []
        empty = dict(labels=[], sqlite_code=None, io_errno=None)
        platform = dict(labels=['runtime-platform'], sqlite_code=None, io_errno=None)
        branches = [reason]  # Exactly one source-level stop call, no checked-close error.
        if cycle is not None:
            check(set(cycle) == {'diagnostics', 'causes'}, 'SDK cycle document shape differs')
            diagnostic = cycle['diagnostics']
            check(set(diagnostic) == {'action_started', 'action_succeeded', 'causes'}
                  and diagnostic['action_started'] is True and diagnostic['action_succeeded'] is True,
                  'SDK probe action did not succeed independently')
            rows = diagnostic['causes']
            check(isinstance(rows, list), 'SDK cycle causes not a typed vector')
            phases = list(dict.fromkeys(row['phase'] for row in rows))
            check(phases in (['journal'], ['supervision', 'journal']), 'SDK unexpected action/cleanup phase')
            expected_rows = []
            for phase in phases:
                for depth, value in enumerate(reason):
                    expected_rows.append(dict(value, phase=phase, path=['root'] + ['source'] * depth, recorded_work=None))
            check(json_equal(rows, expected_rows), 'SDK phase chain has unclassified cause or effect')
            check(json_equal(cycle['causes'], [empty] + reason), 'SDK cycle source chain differs')
            prefix = [empty, dict(child_ownership=trace), platform, platform, empty,
                      dict(fixture_phase='watch-fixture-cycle'), empty]
            branches.insert(0, prefix + reason)
        check(json_equal(failure['causes'], branches), 'SDK top-level failures include unclassified branch')
        return dict(status='EXPECTED_SUBSTITUTED_CONTEXT_REFUSAL', sdk_os_reaped=True,
                    sdk_checked_close_receipt=False, lease_stop_recorded=False,
                    recognized_cycle_phases=phases, recognized_stop_refusal=True)
    except (KeyError, TypeError, ValueError, RuntimeError, AttributeError):
        return dict(status='INCONCLUSIVE_SDK_FAILURE_UNCLASSIFIED', sdk_os_reaped=True,
                    sdk_checked_close_receipt=False, lease_stop_recorded=None)


def business_snapshot(directory, work, agent):
    """Read exact original authority/resource rows, excluding advancing heartbeat.

    Keep Agent/body, canonical start event, controls/consent and reservation/FIFO
    rows; this never modifies SQL or treats exclusions as permission to mutate.
    Full SQL dumps remain retained independently for heartbeat/journal diagnosis.
    """
    db = sqlite3.connect('file:' + str(directory / 'state.db') + '?mode=ro', uri=True)
    try:
        db.execute('BEGIN')
        queries = [('agent', 'SELECT * FROM agents WHERE id=?', (agent,)),
                   ('start', "SELECT * FROM events WHERE work_id=? AND kind IN ('agent.start','mission.start') ORDER BY id", (work,)),
                   ('consent', "SELECT * FROM cockpit_events WHERE work_id=? AND kind='mission-policy' ORDER BY seq", (work,)),
                   ('claims', 'SELECT * FROM native_mission_claim_receipts WHERE work_id=? ORDER BY generation', (work,)),
                   ('receipts', 'SELECT * FROM native_mission_work_receipts WHERE work_id=? ORDER BY event_id', (work,)),
                   ('epochs', 'SELECT * FROM native_mission_lease_epochs WHERE work_id=?', (work,)),
                   ('journal', 'SELECT * FROM mission_coordination_events WHERE work_id=? ORDER BY event_key', (work,)),
                   ('controls', 'SELECT * FROM cockpit_controls WHERE work_id=?', (work,)),
                   ('reservation', 'SELECT * FROM reservations WHERE agent_id=?', (agent,)),
                   ('turn', 'SELECT * FROM workspace_turns WHERE work_id=? AND agent_id=?', (work, agent))]
        return {label: db.execute(sql, values).fetchall() for label, sql, values in queries}
    finally:
        close_database(db, sys.exception())


def supervision_snapshot(directory, work):
    """Read the exact persisted lease row; labels never establish process liveness."""
    db = sqlite3.connect('file:' + str(directory / 'state.db') + '?mode=ro', uri=True)
    db.row_factory = sqlite3.Row
    try:
        rows = db.execute('SELECT * FROM mission_supervision WHERE work_id=?', (work,)).fetchall()
        check(len(rows) == 1, 'original supervision row missing or duplicated')
        return dict(rows[0])
    finally:
        close_database(db, sys.exception())


def recover_by_public_cancellation(owned, out, original_agent, original_business):
    """After restoration verify original queued intent, then stop/reconcile/replay.

    stop proves desired=stop only. Actual unregistered reconcile must interrupt
    and release the exact original reservation/FIFO without new provider. Preserve
    pre-action snapshots; do not acquire a lease, change TTL or claim read alone
    as recovery. New admission under the dead unstopped lease remains unproven.
    """
    actual = owned.ok(['agent', 'show', original_agent['id']])['agent']
    check(json_equal(actual, original_agent) and actual['status'] == 'queued' and actual.get('supervisor_pid', 0) == 0,
          'restored original Agent does not match queued dispatch')
    check(business_snapshot(owned.root / '.swarm', owned.work, actual['id']) == original_business,
          'original authority/resource state changed before recovery')
    providers = watch.completion.provider_files(owned.root)
    before_turn = watch.completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, actual['id'])
    before_money = watch.completion.reservation(owned, actual['id'])
    fixture.publish(out / 'recovery-before.json', dict(agent=actual, turn=before_turn, reservation=before_money))
    stop = owned.ok(['agent', 'stop', actual['id']])
    stopped = owned.ok(['agent', 'show', actual['id']])['agent']
    check(stop['desired'] == 'stop' and stop['termination_confirmed'] is False
          and stopped['desired'] == 'stop' and stopped['status'] == 'queued', 'public stop is not the desired-only stage')
    check(watch.completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, actual['id']) == before_turn
          and watch.completion.reservation(owned, actual['id']) == before_money, 'desired-only stop settled resources unexpectedly')
    fixture.publish(out / 'recovery-stop.json', dict(reply=stop, agent=stopped, turn=before_turn, reservation=before_money))
    reply = owned.ok(['agent', 'reconcile', actual['id']])
    after = owned.ok(['agent', 'show', actual['id']])['agent']
    after_turn = watch.completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, actual['id'])
    check(reply['cancelled_before_start'] is True and reply['accepted_by_cancellation'] is False
          and after['status'] == 'interrupted' and after['desired'] == 'stop'
          and after['native_mission'] == actual['native_mission'], 'public reconciliation did not cancel original intent')
    stable = ('seq', 'work', 'task', 'workspace', 'agent', 'requested_at', 'acquired_at')
    check(after_turn['state'] == 'released' and isinstance(after_turn['released_at'], str) and after_turn['released_at']
          and all(after_turn[key] == before_turn[key] for key in stable)
          and watch.completion.reservation(owned, actual['id']) == ['released', before_money[1]],
          'public cancellation did not release exact original resources')
    replay = watch.diagnostic.snapshot(owned)
    owned.ok(['agent', 'reconcile', actual['id']])
    check(watch.diagnostic.snapshot(owned) == replay and watch.completion.provider_files(owned.root) == providers,
          'public cancellation replay mutated SQL or started provider')
    fixture.publish(out / 'recovery-cancelled.json', dict(status='PASS_RECOVERY_BY_CANCELLATION', agent=after,
        turn=after_turn, reservation=['released', before_money[1]], new_admission_proven=False,
        dead_lease_policy_unchanged=True, provider_started=False))
    return dict(status='PASS_RECOVERY_BY_CANCELLATION', new_admission_proven=False)


def verify_spawn_ready(document, work, agent, conductor):
    """Validate schema2's real post-check observation before a fixture substitution.

    Exact keys/types and owned conductor identity are required; the process is
    already held by pidfd by the caller. These observations do not grant authority
    or prove child binding, and do not alter schema1's historical ordering.
    """
    keys = {'status', 'schema_version', 'work', 'agent_id', 'original_binding_matches_open_store',
            'mission_cycle_monitor_running', 'original_binding_retained', 'supervisor_spawned',
            'observation_is_authority', 'conductor'}
    check(isinstance(document, dict) and set(document) == keys, 'late probe barrier shape differs')
    check(type(document['schema_version']) is int and document['schema_version'] == 2
          and document['status'] == 'ROOT_INSTANCE_PROBE_SPAWN_READY'
          and document['work'] == work and document['agent_id'] == agent,
          'late probe barrier identity or version differs')
    check(all(document[key] is True for key in ('original_binding_matches_open_store',
          'mission_cycle_monitor_running', 'original_binding_retained'))
          and document['supervisor_spawned'] is False and document['observation_is_authority'] is False,
          'late probe barrier did not verify original context')
    observed = document['conductor']
    check(isinstance(observed, dict) and set(observed) == {'pid', 'stamp', 'ppid', 'pgid', 'sid', 'state'}
          and all(type(observed[key]) is int and observed[key] == conductor[key] for key in ('pid', 'ppid', 'pgid', 'sid'))
          and type(observed['stamp']) is str and observed['stamp'] == conductor['stamp']
          and type(observed['state']) is str and bool(observed['state']),
          'late probe conductor identity changed')


def late_probe_barrier(process, out, sdk_fd, notify_fd, work, agent, conductor):
    """Resume intact schema2 Root, await checked spawn-ready, then return before injection.

    Caller owns cleanup if command/publication/validation fails and must not send
    spawn or inject until this returns. No timeout, new lease or permission grant.
    """
    watch.send_command(process, 'resume\n')
    ready = watch.wait_json(out / 'spawn-ready.json', process, sdk_fd, notify_fd)
    verify_spawn_ready(ready, work, agent, conductor)
    current = watch.observation(conductor['pid'])
    check(current['stamp'] == conductor['stamp'] and current['ppid'] == conductor['ppid'],
          'late probe conductor kernel identity changed')
    fixture.publish(out / 'spawn-ready-observed-time.json', dict(wall_ns=time.time_ns(), monotonic_ns=time.monotonic_ns(),
        root_substitution_not_yet_attempted=True, observation_is_authority=False))


def probe_case(binary, sdk, normal, out, replace, expected_code, variant='directory', expected_binding_version=4, probe_schema_version=2):
    """Run one public SDK barrier with directory or database-only substitution.

    The SDK owns/reaps its child via finish-probe after actual pidfd readiness.
    Multiplex actual provider entry and child exit; never request SDK provider
    verification or infer completion from its original queued Agent. Preserve
    both instances and all failures; restoration requires known runtime closure.
    variant selects a fixture experiment, never a product policy; directory keeps
    the historical whole-directory probe, database preserves the .swarm inode.
    Schema2 resumes the intact Root, validates spawn-ready after actual conductor
    guards, substitutes only then, and sends spawn. Explicit schema1 retains the
    historical injection-before-resume protocol, never detected from a payload.
    expected_binding_version is a declared fixture oracle (default4); explicit3
    permits historical shape checks only and never asserts Root identity.
    """
    out.mkdir(mode=0o700, exist_ok=False)
    ready, owned = planning.flow(binary, out / 'planning', before_worker=True, return_fixture=True)
    root = owned.root
    nonce = fixture.digest(('clone' if replace else 'witness').encode() + os.fsencode(root))
    fixture.publish(root / 'codex', watch.provider_program(nonce).encode())
    (root / 'codex').chmod(0o700)
    check(variant in ('directory', 'database'), 'unknown instance substitution variant')
    check(type(probe_schema_version) is int and probe_schema_version in (1, 2), 'unknown probe protocol schema')
    prefix = '.swarm-root-instance' if variant == 'directory' else '.swarm-db-instance'
    original = root / (prefix + '-original')
    staging = root / (prefix + '-staging')
    retained = root / (prefix + '-exercised-clone')
    database_state = {}
    listener = connection = process = None
    sdk_fd = cli_fd = provider_fd = notify_fd = None
    conductor = cli = provider = None
    channels = []
    primary = None
    cleanup = []
    entered = False
    child_closed = False
    sdk_reaped = False
    clone_manifest = None
    running_agent = None
    result = None
    request = dict(kind='owned-inert-watch-worker-fixture', root=str(root), work=ready['work_id'],
        binary=str(binary), binary_sha256=normal['sha256'], mode='probe',
        result=str(out / 'sdk-result.json'), spawned=str(out / 'spawned.json'),
        ready=str(out / 'unused-provider-ready.json'), supervisor_stdout=str(out / 'supervisor.stdout'),
        supervisor_stderr=str(out / 'supervisor.stderr'), root_instance_probe=dict(schema_version=probe_schema_version,
        queued=str(out / 'queued.json'), observation=str(out / 'probe-observation.json')))
    if probe_schema_version == 2:
        request['root_instance_probe']['spawn_ready'] = str(out / 'spawn-ready.json')
    fixture.publish(root / '.swarm/verification-watch-worker.json', request)
    fixture.publish(out / 'request.json', request)
    fixture.publish(out / 'expectations.json', dict(variant=variant, replacement=replace,
        expected_binding_version=expected_binding_version, binding_root_identity_asserted=expected_binding_version == 4,
        instance_refusal_code=expected_code, probe_schema_version=probe_schema_version))
    try:
        listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        listener.bind(str(root / 'watch-provider.sock'))
        os.chmod(root / 'watch-provider.sock', 0o600)
        listener.listen(1)
        notify_fd = watch.watch_directory(out)
        for name in ('conductor.stdout', 'conductor.stderr'):
            channels.append(watch.spool(out / name))
        process = subprocess.Popen([str(sdk), str(out / 'request.json')], stdin=subprocess.PIPE,
            stdout=channels[0], stderr=channels[1], env=owned.environment)
        conductor = watch.observation(process.pid)
        sdk_fd = watch.attach(conductor)
        queued = watch.wait_json(out / 'queued.json', process, sdk_fd, notify_fd)
        check(queued['status'] == 'ROOT_INSTANCE_PROBE_QUEUED' and queued['agent_id'] == 'watch-worker-agent'
              and queued['agent_status'] == 'queued' and queued['work'] == owned.work
              and queued['original_binding_matches_open_store'] is True
              and queued['original_binding_retained'] is True and queued['supervisor_spawned'] is False
              and queued['directory_observation_is_authority'] is False,
              'probe queued barrier contract mismatch')
        check(queued['conductor']['pid'] == conductor['pid'] and queued['conductor']['stamp'] == conductor['stamp'],
              'probe barrier conductor identity mismatch')
        before_agent = owned.ok(['agent', 'show', 'watch-worker-agent'])['agent']
        check(before_agent['status'] == 'queued' and before_agent['native_mission'] == queued['native_mission']
              and fixture.digest(before_agent['prompt'].encode()) == queued['prompt_sha256'],
              'probe did not pause on exact dispatched intention')
        watch.verify_current_root_binding(before_agent['native_mission'], root, expected_version=expected_binding_version)
        observed_directory = (root / '.swarm').stat()
        check(queued['root_path_directory_observation'] == dict(dev=observed_directory.st_dev, ino=observed_directory.st_ino),
              'probe original pathname observation changed before injection')
        fixture.publish(out / 'barrier-observed-time.json', dict(wall_ns=time.time_ns(), monotonic_ns=time.monotonic_ns()))
        fixture.publish(out / 'original-agent-before.json', before_agent)
        before_business = business_snapshot(root / '.swarm', owned.work, before_agent['id'])
        before_supervision = supervision_snapshot(root / '.swarm', owned.work)
        workspace = root / 'watch-worker'
        workspace_before = workspace.stat()
        # Record raw workspace identity and provider bytes. Copy only .swarm.
        layout = dict(workspace_identity=[workspace_before.st_dev, workspace_before.st_ino],
                      provider_sha256=fixture.digest((root / 'codex').read_bytes()))
        fixture.publish(out / 'unchanged-workspace.json', layout)
        provider_before = watch.completion.provider_files(root)
        if probe_schema_version == 2:
            late_probe_barrier(process, out, sdk_fd, notify_fd, owned.work, before_agent['id'], conductor)
        if replace:
            if variant == 'directory':
                clone_manifest = coherent_clone(root / '.swarm', staging, out)
                os.rename(root / '.swarm', original)
                os.rename(staging, root / '.swarm')
            else:
                clone_manifest = prepare_database_variant(root / '.swarm', staging, original, out, database_state)
                install_database_variant(root / '.swarm', staging, original, out, database_state)
            fixture.publish(out / 'replacement.json', dict(variant=variant, original=str(original), clone=str(root / '.swarm'),
                original_identity=clone_manifest['original_identity'],
                clone_identity=[(root / '.swarm').stat().st_dev, (root / '.swarm').stat().st_ino],
                clone_database_identity=[(root / '.swarm/state.db').stat().st_dev, (root / '.swarm/state.db').stat().st_ino],
                backup_directory_identity=clone_manifest['clone']['identity']))
        check([workspace.stat().st_dev, workspace.stat().st_ino] == layout['workspace_identity']
              and fixture.digest((root / 'codex').read_bytes()) == layout['provider_sha256'],
              'workspace or provider changed during substitution')
        watch.send_command(process, 'spawn\n' if probe_schema_version == 2 else 'resume\n')
        spawned = watch.wait_json(out / 'spawned.json', process, sdk_fd, notify_fd)
        check(spawned['status'] == 'ROOT_INSTANCE_PROBE_SPAWNED'
              and spawned['argv'] == [str(binary), '_mission_worker_entry', 'watch-worker-agent', '--root', str(root), '--json']
              and spawned['binary_sha256'] == normal['sha256'], 'probe helper spawn mismatch')
        cli = spawned['supervisor']
        cli_fd, cli, child_proof = attach_probe_child(cli, conductor, binary, root)
        fixture.publish(out / 'child-kernel-proof.json', child_proof)
        with selectors.DefaultSelector() as selector:
            selector.register(listener, selectors.EVENT_READ, 'provider')
            selector.register(cli_fd, selectors.EVENT_READ, 'child')
            selector.register(sdk_fd, selectors.EVENT_READ, 'conductor')
            while connection is None and not child_closed:
                events = selector.select()
                # Drain actual socket entry first if exit and entry are both ready.
                if any(key.data == 'provider' for key, _ in events):
                    connection, _ = listener.accept()
                    provider, provider_fd = watch.owned_provider_peer(connection, cli)
                    entry = watch.frame(connection)
                    check(entry['pid'] == provider['pid'] and entry['ppid'] == cli['pid'] and entry['nonce'] == nonce,
                          'probe provider entry identity mismatch')
                    entered = True
                    cli = watch.supervisor_session(cli, conductor, True)
                    fixture.publish(out / 'actual-provider-entry.json', dict(provider=provider, supervisor=cli,
                        conductor=conductor, replacement=replace, variant=variant, expected_binding_version=expected_binding_version, entry=entry, nonce=nonce,
                        status='FAIL_PROVIDER_ENTERED_CLONE' if replace else 'OBSERVED_WITNESS_ENTRY'))
                    fixture.publish(out / 'oracle-entry.json', dict(status='FAIL_PROVIDER_ENTERED_CLONE' if replace
                        else 'OBSERVED_WITNESS_ENTRY', actual_provider_entry=True, replacement=replace, variant=variant, expected_binding_version=expected_binding_version,
                        cleanup_not_yet_observed=True, wall_ns=time.time_ns(), monotonic_ns=time.monotonic_ns()))
                    if not replace:
                        running_agent = owned.ok(['agent', 'show', 'watch-worker-agent'])['agent']
                        check(running_agent['status'] == 'running' and running_agent['supervisor_pid'] == cli['pid']
                              and running_agent['child_pid'] == provider['pid'], 'witness actual Agent/kernel mismatch')
                    release_provider(root, nonce)
                if any(key.data == 'child' for key, _ in events):
                    child_closed = True
                if any(key.data == 'conductor' for key, _ in events):
                    raise RuntimeError('probe conductor exited before finish-probe')
        if not child_closed:
            with selectors.DefaultSelector() as selector:
                selector.register(cli_fd, selectors.EVENT_READ, 'child')
                selector.register(sdk_fd, selectors.EVENT_READ, 'conductor')
                for key, _ in selector.select():
                    check(key.data == 'child', 'probe conductor exited while child was active')
                    child_closed = True
        watch.send_command(process, 'finish-probe\n')
        observed = watch.wait_json(out / 'probe-observation.json', process, sdk_fd, notify_fd)
        check(observed['status'] == 'ROOT_INSTANCE_PROBE_OBSERVED' and observed['worker_exit_observed'] is True
              and observed['child_reaped_by_sdk'] is True and observed['supervisor_pid'] == cli['pid'], 'SDK did not observe actual child exit')
        exit_observation = observed['exit_observation']
        check(exit_observation['signal'] is None and isinstance(exit_observation['code'], int),
              'probe child exit not a normal collected status')
        sdk_code = process.wait()
        sdk_reaped = True
        fixture.publish(out / 'sdk-process-exit.json', dict(returncode=sdk_code, actual_popen_wait=True,
            sdk_os_process_closed=True, sdk_close_receipt_not_yet_checked=True))
        for stream in channels:
            stream.flush()
            os.fsync(stream.fileno())
        stdout = (out / 'supervisor.stdout').read_bytes()
        stderr = (out / 'supervisor.stderr').read_bytes()
        after = collect_instance(root / '.swarm', out, 'exercised-instance-after')
        if replace:
            original_directory = original if variant == 'directory' else original / 'database'
            collect_instance(original_directory, out, 'original-instance-after')
            check(business_snapshot(original_directory, owned.work, before_agent['id']) == before_business,
                  'original authority or resources changed under substitution')
            classification = clone_oracle(entered, exit_observation['code'], expected_code, stderr,
                                          after['sql_sha256'] == clone_manifest['source_sql_sha256']
                                          and watch.completion.provider_files(root) == provider_before
                                          and (variant != 'database' or private_files(root / '.swarm') == database_state['baseline_private']))
        else:
            check(entered and exit_observation['code'] == 0, 'same-hook witness did not execute and complete')
            final = json.loads(stdout)
            check(final['agent']['status'] == 'completed' and final['task_accepted'] is False,
                  'same-hook witness completion missing')
            watch.retain_completion(owned, out, final, running_agent, cli, provider)
            classification = 'PASS_SAME_HOOK_WITNESS'
        fixture.publish(out / 'worker-verdict.json', dict(status=classification, variant=variant,
            provider_entered=entered, child_exit=exit_observation, child_reaped_by_sdk=True,
            stdout_sha256=fixture.digest(stdout), stderr_sha256=fixture.digest(stderr),
            clone_snapshot=after, sdk_close_not_yet_classified=True))
        if sdk_code == 0:
            closed = json.loads((out / 'sdk-result.json').read_bytes())
            check(closed['status'] == 'ROOT_INSTANCE_PROBE_CLOSED'
                  and all(closed[key] is True for key in ('lease_released', 'lease_closed', 'root_closed', 'original_root_binding_closed'))
                  and closed['worker_exit_observed'] is True and closed['child_reaped_by_sdk'] is True
                  and json_equal(closed['exit_observation'], exit_observation), 'successful SDK close receipt invalid')
            sdk_outcome = dict(status='SDK_CHECKED_CLOSED', sdk_os_reaped=True,
                               sdk_checked_close_receipt=True, lease_stop_recorded=True)
        elif replace:
            failure = json.loads((out / 'sdk-result.failure.json').read_bytes())
            cycle_path = out / 'sdk-result.cycle-failure.json'
            cycle = json.loads(cycle_path.read_bytes()) if cycle_path.exists() else None
            sdk_outcome = classify_sdk_refusal(failure, cycle, spawned, observed, variant, sdk_code,
                                               classification == 'PASS_INSTANCE_REFUSAL')
        else:
            raise RuntimeError('same-hook witness SDK did not exit0 and close successfully')
        original_directory = (original if variant == 'directory' else original / 'database') if replace else root / '.swarm'
        after_supervision = supervision_snapshot(original_directory, owned.work)
        fixture.publish(out / 'original-supervision-delta.json', dict(before=before_supervision, after=after_supervision,
            changed_fields=[key for key in before_supervision if before_supervision[key] != after_supervision[key]]))
        if sdk_outcome['status'] == 'EXPECTED_SUBSTITUTED_CONTEXT_REFUSAL':
            permitted_pulse = {'heartbeat_at', 'next_check_at'}
            check(all(after_supervision[key] == value for key, value in before_supervision.items() if key not in permitted_pulse)
                  and after_supervision['stopped_at'] == '', 'unexpected original lease/journal mutation under refused context')
        elif sdk_outcome['status'] == 'SDK_CHECKED_CLOSED':
            check(after_supervision['conductor_id'] == before_supervision['conductor_id']
                  and isinstance(after_supervision['stopped_at'], str) and after_supervision['stopped_at'],
                  'successful SDK close did not record original lease stop')
        fixture.publish(out / 'sdk-context-classification.json', sdk_outcome)
        combined = classification
        if classification == 'PASS_INSTANCE_REFUSAL' and sdk_outcome['status'] == 'INCONCLUSIVE_SDK_FAILURE_UNCLASSIFIED':
            combined = 'FAIL_SDK_FAILURE_UNCLASSIFIED'
        result = dict(status=combined, worker_status=classification, sdk=sdk_outcome,
            root=str(root), work=owned.work, replacement=replace, variant=variant,
            probe_schema_version=probe_schema_version, expected_binding_version=expected_binding_version, binding_root_identity_asserted=expected_binding_version == 4,
            provider_entered=entered, child_exit=exit_observation, original_restored=False,
            sdk_os_reaped=True, unchanged_workspace=layout, actual_provider_entry=entered)
    except BaseException as error:
        primary = error
        watch.retain_failure(cleanup, lambda: fixture.publish(out / 'failure.json', dict(status='FAIL', cause=traceback.format_exc())))
    # Cleanup is not evidence of acceptance. Never let a failed sidecar skip it.
    if primary is not None:
        try:
            if provider is not None and provider_fd is not None and not select_ready(provider_fd):
                check(watch.observation(provider['pid'])['stamp'] == provider['stamp'], 'cleanup provider stamp changed')
                release_provider(root, nonce)
            if cli_fd is not None and not select_ready(cli_fd):
                known_child(cli, process.pid, binary, root)
                signal.pidfd_send_signal(cli_fd, signal.SIGTERM)
            if cli_fd is not None:
                with selectors.DefaultSelector() as selector:
                    selector.register(cli_fd, selectors.EVENT_READ)
                    selector.select()
                child_closed = True
        except BaseException as error:
            cleanup.append(error)
        try:
            if process is not None and process.poll() is None:
                # Checked EOF lets the owned SDK unwind its barrier/cycle/Root.
                process.stdin.close()
            if process is not None:
                process.wait()
                sdk_reaped = True
            if cli is not None and child_closed:
                try:
                    waited, status = os.waitpid(cli['pid'], 0)
                    fixture.publish(out / 'cleanup-child-wait.json', dict(pid=waited, raw_wait_status=status))
                except ChildProcessError:
                    # Only acceptable with real SDK receipt of child collection.
                    trace = out / 'probe-observation.json'
                    check(trace.exists() and json.loads(trace.read_bytes())['worker_exit_observed'] is True,
                          'child reap ownership unknown after SDK error')
            if process is not None:
                inventory = watch.adopted_supervisors(binary, root)
                watch.retain_failure(cleanup, lambda: fixture.publish(out / 'cleanup-adopted.json', inventory))
                for identity in inventory['matched']:
                    descriptor = None
                    try:
                        descriptor = watch.attach(identity)
                        known_child(identity, process.pid, binary, root)
                        signal.pidfd_send_signal(descriptor, signal.SIGTERM)
                        waited, status = os.waitpid(identity['pid'], 0)
                        fixture.publish(out / ('cleanup-adopted-' + str(waited) + '.json'), dict(pid=waited, raw_wait_status=status))
                    except BaseException as error:
                        cleanup.append(error)
                    finally:
                        if descriptor is not None:
                            try:
                                os.close(descriptor)
                            except BaseException as error:
                                cleanup.append(error)
                check(not inventory['unknown'], 'unresolved actual adopted child prevents restore')
                if cli is None:
                    child_closed = True  # Actual post-SDK inventory, not missing JSON alone.
        except BaseException as error:
            cleanup.append(error)
    if provider_fd is not None and child_closed and sdk_reaped:
        try:
            if not select_ready(provider_fd):
                check(primary is not None, 'provider still alive after successful child completion')
                check(watch.observation(provider['pid'])['stamp'] == provider['stamp'], 'cleanup adopted provider stamp changed')
                signal.pidfd_send_signal(provider_fd, signal.SIGTERM)
            try:
                waited, status = os.waitpid(provider['pid'], 0)
                fixture.publish(out / 'cleanup-provider-wait.json', dict(pid=waited, raw_wait_status=status))
            except ChildProcessError:
                check(select_ready(provider_fd), 'provider departure remains unknown after child reap')
        except BaseException as error:
            cleanup.append(error)
    if provider_fd is not None and not select_ready(provider_fd):
        cleanup.append(RuntimeError('provider closure unknown; Root restore forbidden'))
    if sdk_reaped:
        try:
            inventory = watch.adopted_supervisors(binary, root)
            watch.retain_failure(cleanup, lambda: fixture.publish(out / 'runtime-post-reap-inventory.json', inventory))
            for identity in inventory['matched']:
                descriptor = watch.attach(identity)
                try:
                    known_child(identity, process.pid, binary, root)
                    signal.pidfd_send_signal(descriptor, signal.SIGTERM)
                    waited, status = os.waitpid(identity['pid'], 0)
                    watch.retain_failure(cleanup, lambda: fixture.publish(out / ('unexpected-owned-child-' + str(waited) + '.json'),
                        dict(pid=waited, raw_wait_status=status)))
                finally:
                    watch.close_descriptor(descriptor, sys.exception())
            check(not inventory['matched'] and not inventory['unknown'],
                  'unexpected adopted process prevents successful restoration proof')
        except BaseException as error:
            cleanup.append(error)
    processes_closed = sdk_reaped and child_closed and not cleanup
    if original.exists():
        try:
            if variant == 'directory':
                restore_instances(root, original, retained, processes_closed)
            else:
                restore_database_variant(root / '.swarm', original, retained, database_state, processes_closed)
            if result is not None:
                result['original_restored'] = True
                result['retained_clone'] = str(retained)
                if result['status'] == 'PASS_INSTANCE_REFUSAL':
                    result['recovery'] = recover_by_public_cancellation(owned, out, before_agent, before_business)
                    result['status'] = 'PASS_FAULT_INJECTION_INSTANCE_REFUSAL'
        except BaseException as error:
            cleanup.append(error)
    for resource in (connection, listener, process.stdin if process is not None else None):
        if resource is not None:
            try:
                resource.close()
            except BaseException as error:
                cleanup.append(error)
    for stream in channels:
        for action in (stream.flush, lambda stream=stream: os.fsync(stream.fileno()), stream.close):
            try:
                action()
            except BaseException as error:
                cleanup.append(error)
    for descriptor in (provider_fd, cli_fd, sdk_fd, notify_fd):
        if descriptor is not None:
            try:
                os.close(descriptor)
            except BaseException as error:
                cleanup.append(error)
    if primary is not None or cleanup:
        raise BaseExceptionGroup('root-instance fixture/cleanup', ([primary] if primary else []) + cleanup)
    fixture.publish(out / 'result.json', result)
    return result


def select_ready(descriptor):
    """Observe pidfd readiness without adding a timeout or liveness guess."""
    import select
    return bool(select.select([descriptor], [], [], 0)[0])


def main():
    """Qualify supplied normal/SDK profiles, witness then clone, retain failures.

    Actual runtime is parent's explicitly launched recipe, never an implicit build.
    Unknown/other refusals fail qualification; no code expectation is invented.
    Own subreaper restoration and primary/reporting/cleanup causes use the verified
    shared scope. This probe is not B1 survival or installed product delivery.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('binary', 'profile', 'sdk-binary', 'sdk-profile', 'output'):
        parser.add_argument('--' + key, type=Path, required=True)
    parser.add_argument('--expected-binding-version', type=int, choices=(3, 4), default=4,
                        help='Explicit historical3/current4 proof expectation, never detected from Agent JSON')
    parser.add_argument('--variant', choices=('directory', 'database'), default='directory',
                        help='Separate retained directory or same-directory SQLite substitution experiment')
    parser.add_argument('--probe-schema-version', type=int, choices=(1, 2), default=2,
                        help='Explicit current2 late barrier or historical1 protocol; never autodetected')
    parser.add_argument('--instance-refusal-code', default='', help='Exact post-fix structured instance refusal code; absent keeps other refusals inconclusive')
    args = parser.parse_args()
    binary, sdk, out = args.binary.resolve(), args.sdk_binary.resolve(), args.output.resolve()
    out.mkdir(mode=0o700, exist_ok=False)
    normal, profile = json.loads(args.profile.read_bytes()), json.loads(args.sdk_profile.read_bytes())
    check(normal['no_fixture_feature'] and normal['sha256'] == fixture.digest(binary.read_bytes()), 'normal profile/hash mismatch')
    check(profile['example'] == 'mission_watch_worker' and profile['engine_features_empty']
          and profile['shared_workspace_features_match_normal'] and not profile['test_profile'], 'SDK default profile mismatch')
    check(profile['normal_profile_sha256'] == normal['sha256']
          and profile['binary_sha256'] == fixture.digest(sdk.read_bytes()), 'SDK normal/profile identity mismatch')
    sources = {str(Path(module.__file__).resolve()): fixture.digest(Path(module.__file__).read_bytes())
        for name, module in tuple(sys.modules.items()) if name.startswith('rust_') and getattr(module, '__file__', None)}
    sources[str(Path(__file__).resolve())] = fixture.digest(Path(__file__).read_bytes())
    fixture.publish(out / 'inputs.json', dict(sources=sources, normal_profile=normal, sdk_profile=profile,
        instance_refusal_code=args.instance_refusal_code, variant=args.variant, probe_schema_version=args.probe_schema_version,
        expected_binding_version=args.expected_binding_version,
        binding_root_identity_asserted=args.expected_binding_version == 4, global_watchdog=None))
    def cases():
        """Preserve same-hook witness before interpreting the fresh clone fault."""
        witness = probe_case(binary, sdk, normal, out / 'witness', False, '', args.variant, args.expected_binding_version, args.probe_schema_version)
        check(witness['status'] == 'PASS_SAME_HOOK_WITNESS', 'same-hook witness failed')
        print('PASS same post-dispatch hook executes original Root and completes', flush=True)
        clone = probe_case(binary, sdk, normal, out / 'clone', True, args.instance_refusal_code, args.variant, args.expected_binding_version, args.probe_schema_version)
        fixture.publish(out / 'result.json', dict(witness=witness, clone=clone, status=clone['status'], variant=args.variant, probe_schema_version=args.probe_schema_version,
            expected_binding_version=args.expected_binding_version,
            binding_root_identity_asserted=args.expected_binding_version == 4))
        check(all(fixture.digest(Path(path).read_bytes()) == sha for path, sha in sources.items()), 'probe source changed during execution')
        check(fixture.digest(binary.read_bytes()) == normal['sha256']
              and fixture.digest(sdk.read_bytes()) == profile['binary_sha256'], 'probe binary changed during execution')
        check(clone['status'] == 'PASS_FAULT_INJECTION_INSTANCE_REFUSAL', 'Root-instance refusal/recovery unverified; inspect retained classifications')
        print('PASS substituted Root instance refuses with exact code; OS reap, restoration and public cancellation verified', flush=True)
    watch.subreaper_scope(cases,
        lambda error: fixture.publish(out / 'failure.json', dict(status='FAIL', cause=''.join(traceback.format_exception(error)))),
        watch.parent_death.subreaper)
    print('REPLAY_EVIDENCE=' + str(out), flush=True)


if __name__ == '__main__':
    main()
