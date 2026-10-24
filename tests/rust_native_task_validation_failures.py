#!/usr/bin/env python3
"""Exercise real task-control refusals, SQL recovery and authority withdrawal.

All Roots and children are owned fixtures. SQL faults hold a write transaction
without changing a row. No Go, paid model, private seeding or global watchdog.
Saved product deadlines govern controls; actual sockets/pidfds govern progress.
Original failures and source/binary identities are retained, never overwritten.
"""
import argparse, json, os, select, selectors, socket, sqlite3, struct
import subprocess, sys, traceback
from contextlib import contextmanager
from pathlib import Path
import rust_native_task_validation as native
fixture = native.fixture

FAULT_CONTROL = native.CONTROL + "\nif Path('control-fault').exists(): raise SystemExit(7)\n"
HELD_CONTROL = native.CONTROL + """
import socket
with socket.socket(socket.AF_UNIX) as s:
 s.connect('validation-fixture.sock');s.sendall(b'R')
 assert s.recv(1)==b'1'
"""


BATCH_HELD = HELD_CONTROL + """
import time
with Path('validation-first-finished').open('x') as f:
 f.write(str(time.monotonic_ns()));f.flush();os.fsync(f.fileno())
if Path('batch-fault').exists(): raise SystemExit(7)
"""
SECOND_CONTROL = """from pathlib import Path
import os,json,hashlib,time
first=Path('validation-executed.json').read_bytes()
finished=int(Path('validation-first-finished').read_text())
started=time.monotonic_ns();assert started>=finished
with Path('validation-second.json').open('x') as f:
 json.dump(dict(pid=os.getpid(),ppid=os.getppid(),first_sha256=hashlib.sha256(first).hexdigest(),first_completion_ns=finished,started_ns=started),f)
 f.flush();os.fsync(f.fileno())
print('SECOND-CONTROL-SUCCESS',flush=True)
"""


def slot_receipt(root, step_id, expected_hash):
    """Read an owned original SR1 frame, checking extent/hash against terminal SQL.

    SR1/20 decimal length/64 hex hash are the actual native_receipt file format,
    not policy ceilings. This test decoder confers no identity/permission; actual
    Root verification and kernel/pidfd observations remain distinct.
    """
    path = root / '.swarm' / ('native-validation-step-' + fixture.digest(step_id.encode()) + '.receipt')
    assert not path.is_symlink()
    before = path.stat(); raw = path.read_bytes(); after = path.stat()
    assert (before.st_dev,before.st_ino,before.st_size,before.st_mtime_ns)==(after.st_dev,after.st_ino,after.st_size,after.st_mtime_ns)
    magic, digits, digest, body = raw.split(b'\n', 3)
    assert magic == b'SR1' and len(digits)==20 and digits.isdigit() and int(digits)==len(body)
    assert len(digest)==64 and fixture.digest(body).encode()==digest and digest.decode()==expected_hash
    value=json.loads(body);assert value['schema_version']==2
    return value


def call(binary, root, out, name, args, request=None):
    """Retain public argv, bounded inputs, exit and both original channels."""
    return native.invoke(binary, root, out, name, args, request)


def shown(binary, root, out, name, work):
    """Read actual persisted Work through the supported public interface."""
    code, result = call(binary, root, out, name, ['work', 'show', work])
    assert code == 0, result
    return result['work']


def settings(binary, root, out, changes):
    """Persist explicit fixture contention policy with preview/CAS and readback.

    Zero busy wait/retries creates a discriminating SQL publication refusal;
    neither value is a production default or a test-wide time limit.
    """
    code, doc = call(binary, root, out, 'settings-read', ['engine-settings', 'show'])
    assert code == 0
    values = dict(doc['values'], **changes)
    request = dict(schema_version=doc['schema_version'], event_id='validation-fixture-storage',
                   expected_revision=doc['revision'], values=values,
                   reason='Explicit fixture SQL contention witness', preview_token='')
    code, preview = call(binary, root, out, 'settings-preview', ['engine-settings', 'preview'], request)
    assert code == 0, preview
    request['preview_token'] = preview['preview_token']
    code, _ = call(binary, root, out, 'settings-save', ['engine-settings', 'apply'], request)
    assert code == 0
    code, saved = call(binary, root, out, 'settings-reopen', ['engine-settings', 'show'])
    assert code == 0 and saved['values'] == values and saved['history'][:-1] == doc['history']


@contextmanager
def database(root, lock=False):
    """Read owned scalar receipts, or hold BEGIN IMMEDIATE without row writes.

    Explicit rollback and close preserve primary and cleanup failures. This
    never opens a live/user Root; lock release is the recovery precondition.
    """
    path = root / '.swarm/state.db'
    db = sqlite3.connect(str(path) if lock else 'file:' + str(path) + '?mode=ro',
                         uri=not lock, timeout=0)
    errors = []; begun = False
    try:
        if lock: db.execute('BEGIN IMMEDIATE'); begun = True
        yield db
    except BaseException as e: errors.append(e)
    if begun:
        try: db.rollback()
        except BaseException as e: errors.append(e)
    try: db.close()
    except BaseException as e: errors.append(e)
    if len(errors) == 1: raise errors[0]
    if errors: raise BaseExceptionGroup('Owned SQL observation and cleanup failed', errors)


def request(setup, event='validation-held'):
    """Only task/revision/event protocol fields; no asserted execution authority."""
    return dict(schema_version=1, event_id=event, expected_revision=setup['revision'], task_id='task')


def failed_control(binary, out):
    """Run a valid witness first, then the same policy with an explicit exit7 fault.

    Retain both original witnesses. A failed control must publish blocked state,
    remain nonzero on replay/reconcile and never run a replacement command.
    """
    out.mkdir(); setup = native.public.flow(binary, out, control_script=FAULT_CONTROL)
    root = Path(setup['root']); work = setup['work_id']; r = request(setup, 'valid-control')
    code, positive = call(binary, root, out, 'positive', ['task', 'validate', work], r)
    assert code == 0 and positive['controls_passed'] and not positive['task_accepted'], positive
    marker = root / 'validation-executed.json'; marker.rename(root / 'positive-validation-executed.json')
    fixture.publish(root / 'control-fault', b'explicit fixture exit7\n')
    r = dict(r, event_id='failed-control', expected_revision=positive['work']['revision'])
    code, failure = call(binary, root, out, 'exit7', ['task', 'validate', work], r)
    assert code != 0 and 'task-validation-controls-failed' in str(failure), failure
    first = marker.read_bytes(); after = shown(binary, root, out, 'failed-readback', work)
    task = after['tasks'][0]; receipt = task['automatic_validation']
    assert task['status'] == 'blocked' and receipt['state'] == 'blocked'
    assert receipt['controls'][0]['exit_code'] == 7 and not receipt['controls'][0]['passed']
    for label, op in [('replay', 'validate'), ('reconcile', 'validation-reconcile')]:
        code, result = call(binary, root, out, label, ['task', op, work], r)
        assert code != 0 and 'task-validation-controls-failed' in str(result), result
        assert marker.read_bytes() == first and shown(binary, root, out, label + '-readback', work) == after
    return dict(failed_nonzero=True, blocked=True, replay_does_not_execute=True, accepted=False)


def held(binary, out, mode, batch=False):
    """Own one real socket/pidfd control through success, SQL refusal or pause.

    A healthy run under identical explicit storage policy precedes faults in the
    main recipe. SQL recovery requires original owner departure and lock release.
    Pause must stop the owned child; explicit retirement must never apply stale results.
    Optional batch adds a second ordered control and checks original per-slot
    attempt/wait facts. SQL mode here targets final publication for a single
    held control; exit7 mode intentionally fails the first of two controls.
    """
    assert mode in ('healthy','sql','pause','exit7') and (mode!='exit7' or batch) and not (batch and mode=='sql')
    extra=[dict(id='second-ordered',command=['python3','-c',SECOND_CONTROL],criteria=[1],inputs=['handoff.md'],review_output=True,justification='Require the actual first control completion before second observation')] if batch else []
    out.mkdir(); setup = native.public.flow(binary, out, control_script=BATCH_HELD if batch else HELD_CONTROL,extra_controls=extra)
    root = Path(setup['root']); work = setup['work_id']; r = request(setup)
    if mode=='exit7': fixture.publish(root/'batch-fault',b'Explicit first-control exit7 fixture\n')
    settings(binary, root, out, dict(storage_busy_retries='0', storage_busy_timeout_ms='0'))
    code, preview = call(binary, root, out, 'preview', ['task', 'validation-preview', work], r)
    assert code == 0 and preview['execution_authorized'] is False
    listener = socket.socket(socket.AF_UNIX); connection = None; selector = selectors.DefaultSelector()
    process = None; pidfd = childfd = None; primary = None; result = None
    stdout = (out / 'run.stdout').open('xb'); stderr = (out / 'run.stderr').open('xb')
    try:
        listener.bind(str(root / 'validation-fixture.sock')); os.chmod(root / 'validation-fixture.sock', 0o600)
        listener.listen(1); listener.setblocking(False); selector.register(listener, selectors.EVENT_READ, 'ready')
        inp = out / 'run.input.json'; fixture.publish(inp, r)
        argv = [str(binary), 'task', 'validate', work, '--root', str(root), '--input', str(inp), '--json']
        process = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
        fixture.publish(out / 'owner.json', dict(argv=argv, pid=process.pid, global_watchdog=None))
        pidfd = os.pidfd_open(process.pid); selector.register(pidfd, selectors.EVENT_READ, 'exit')
        while connection is None:
            events = selector.select()
            assert not any(key.data == 'exit' for key, _ in events), 'Owner exited before control handshake'
            connection, _ = listener.accept()
        selector.unregister(listener)
        peer, uid, _ = struct.unpack('3i', connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        assert uid == os.getuid() and connection.recv(1) == b'R'
        childfd = os.pidfd_open(peer)
        witness = json.loads((root / 'validation-executed.json').read_bytes())
        assert witness['pid'] == peer and witness['ppid'] == process.pid
        with database(root) as db:
            row = db.execute('SELECT j.owner_pid,s.child_pid,s.parent_pid,s.permission,s.phase FROM native_validation_jobs j JOIN native_validation_steps s ON s.job_id=j.id WHERE s.ordinal=0').fetchone()
            assert row == (process.pid, peer, process.pid, 1, 'permitted'), row
            if batch:
                assert db.execute('SELECT phase,permission,child_pid FROM native_validation_steps WHERE ordinal=1').fetchone()==('prepared',0,None)
                assert not (root/'validation-second.json').exists()

        code, live = call(binary, root, out, 'live-replay', ['task', 'validate', work], r)
        assert code != 0 and 'task-validation-reconciliation-required' in str(live), live
        code, live_retire = call(binary, root, out, 'live-retire', ['task', 'validation-retire', work], r)
        assert code != 0, live_retire
        assert process.poll() is None and not select.select([childfd], [], [], 0)[0]
        if mode == 'sql':
            with database(root, True):
                connection.sendall(b'1'); connection.close(); connection = None
                code = process.wait(); assert code != 0
        elif mode == 'pause':
            current = shown(binary, root, out, 'before-pause', work)
            code, paused = call(binary, root, out, 'pause', ['planning', 'pause', work],
                                dict(schema_version=1, event_id='pause-control', expected_revision=current['revision']))
            assert code == 0 and paused['planning']['paused'], paused
            code = process.wait(); assert code != 0
        else:
            connection.sendall(b'1'); connection.close(); connection = None
            code = process.wait(); assert (code != 0 if mode=='exit7' else code == 0)
        assert select.select([childfd], [], [], 0)[0] == [childfd], 'Owned control was not reaped'
        stdout.flush(); stderr.flush(); os.fsync(stdout.fileno()); os.fsync(stderr.fileno())
        raw = json.loads((out / ('run.stdout' if code == 0 else 'run.stderr')).read_bytes())
        if mode == 'sql':
            assert raw['failure']['code'] == 'work-state-preserved' and 'agent-launch-storage-busy' in raw['failure']['causes'] and 'sqlite-code:5' in raw['failure']['causes'], raw
        if mode == 'pause': assert 'task-validation-planning-paused' in str(raw), raw
        first = (root / 'validation-executed.json').read_bytes()
        code, recovered = call(binary, root, out, 'reconcile', ['task', 'validation-reconcile', work], r)
        if mode == 'pause':
            assert code != 0 and 'task-validation-planning-paused' in recovered['failure']['causes'], recovered
            code, recovered = call(binary, root, out, 'retire', ['task', 'validation-retire', work], r)
            assert code == 0 and recovered['retired'] and not recovered['controls_passed'] and not recovered['task_accepted'], recovered
            current = recovered['work']
            assert current['tasks'][0]['status'] == 'submitted' and current['tasks'][0]['evidence_stale_reason'] == 'validation-observations-retired'
            code, replayed = call(binary, root, out, 'retire-replay', ['task', 'validation-retire', work], r)
            assert code == 0 and replayed['replayed'] and replayed['retired'] and replayed['work'] == current
            code, refused = call(binary, root, out, 'retired-execute', ['task', 'validate', work], r)
            assert code != 0 and 'task-validation-original-refused' in refused['failure']['causes'], refused
            fresh = dict(r, event_id='new-paused-validation', expected_revision=current['revision'])
            code, refused = call(binary, root, out, 'fresh-paused', ['task', 'validate', work], fresh)
            assert code != 0 and 'task-validation-planning-paused' in refused['failure']['causes'], refused
            with database(root) as db:
                assert db.execute('SELECT phase FROM native_validation_jobs').fetchall() == [('refused',)]
                assert db.execute('SELECT phase FROM native_validation_steps').fetchall() == [('refused',)] * (2 if batch else 1)
        elif mode=='exit7':
            assert code != 0 and 'task-validation-controls-failed' in recovered['failure']['causes'],recovered
        else:
            assert code == 0 and recovered['controls_passed'] and recovered['replayed'] and not recovered['task_accepted'], recovered
        assert (root / 'validation-executed.json').read_bytes() == first
        if batch:
            with database(root) as db:
                rows=db.execute('SELECT id,ordinal,phase,permission,child_pid,receipt_sha256 FROM native_validation_steps ORDER BY ordinal').fetchall()
            assert len(rows)==2 and [r[1] for r in rows]==[0,1]
            originals=[slot_receipt(root,row[0],row[5]) for row in rows]
            assert originals[0]['process_attempted'] and originals[0]['waited'] and rows[0][4]==peer
            if mode=='pause':
                labels=[label for c in originals[0]['causes'] for label in c['labels']]
                assert 'task-validation-planning-paused' in labels, labels
                assert not originals[1]['process_attempted'] and not originals[1]['waited'] and originals[1]['result'] is None
                assert rows[1][3:5]==(0,None) and not (root/'validation-second.json').exists()
            else:
                second=json.loads((root/'validation-second.json').read_bytes())
                assert originals[1]['process_attempted'] and originals[1]['waited'] and originals[1]['exit_code']==0
                assert second['pid']==rows[1][4] and second['ppid']==process.pid and second['first_sha256']==fixture.digest(first)
                assert second['started_ns']>=second['first_completion_ns']
                after=shown(binary,root,out,'batch-after',work); controls=after['tasks'][0]['automatic_validation']['controls']
                assert len(controls)==2 and controls[1]['review_output']=='SECOND-CONTROL-SUCCESS\n' and controls[1]['passed']
                assert controls[0]['exit_code']==(7 if mode=='exit7' else 0)
                assert after['tasks'][0]['status']==('blocked' if mode=='exit7' else 'submitted')
        result = dict(mode=mode, actual_child=True, owner_departed=True, child_reaped=True,
                      recovered_without_execution=True, controls_passed=False if mode=='exit7' else recovered['controls_passed'], accepted=False,
                      batch=batch, second_unattempted_after_pause=batch and mode=='pause', second_executed=batch and mode!='pause')
    except BaseException as e: primary = e
    cleanup = []
    if process is not None and process.poll() is None:
        try: process.terminate(); process.wait()
        except BaseException as e: cleanup.append(e)
    for fn in [lambda: connection.close() if connection else None, listener.close, selector.close,
               lambda: os.close(pidfd) if pidfd is not None else None,
               lambda: os.close(childfd) if childfd is not None else None, stdout.close, stderr.close]:
        try: fn()
        except BaseException as e: cleanup.append(e)
    if primary or cleanup:
        errors = ([primary] if primary else []) + cleanup
        if len(errors) == 1: raise errors[0]
        raise BaseExceptionGroup('Owned validation and cleanup failed', errors)
    return result


def main():
    """Bind outcomes to exact immutable candidate binary and imported recipe sources."""
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary', required=True, type=Path); p.add_argument('--output', required=True, type=Path)
    a = p.parse_args(); binary = a.binary.resolve(); out = a.output.resolve(); out.mkdir(exist_ok=False)
    sources = {str(Path(m.__file__)): fixture.digest(Path(m.__file__).read_bytes())
               for m in list(sys.modules.values()) if getattr(m, '__file__', None)
               and str(Path(m.__file__).resolve()).startswith(str(Path('tests').resolve()))}
    sources[str(Path(__file__))] = fixture.digest(Path(__file__).read_bytes())
    inputs = dict(binary=str(binary), binary_sha256=fixture.digest(binary.read_bytes()), sources=sources)
    fixture.publish(out / 'inputs.json', inputs)
    try:
        healthy = held(binary, out / 'healthy', 'healthy')
        failed = failed_control(binary, out / 'failed')
        sql = held(binary, out / 'sql', 'sql')
        pause = held(binary, out / 'pause', 'pause')
        assert fixture.digest(binary.read_bytes()) == inputs['binary_sha256']
        assert all(fixture.digest(Path(n).read_bytes()) == h for n, h in sources.items())
        result = dict(status='PASS', healthy=healthy, failed=failed, sql=sql, pause=pause,
                      real_provider=False, installed=False, scope='Owned native task control only; no independent/global acceptance')
        fixture.publish(out / 'result.json', result); print(json.dumps(result))
    except BaseException:
        fixture.publish(out / 'failure.json', dict(status='FAIL', cause=traceback.format_exc())); raise


if __name__ == '__main__': main()
