#!/usr/bin/env python3
"""B1: an actual granted worker survives its conductor's graceful/SIGTERM exit.

The SDK dispatches and spawns the qualified normal private session bootstrap,
which reuses the EXISTING agent run supervisor after safe setsid in the child. This
harness never substitutes a worker runner. Two fresh publicly planned Roots use
owned inert providers only. Kernel peer credentials, process stamps, pidfds and
subreaper waitpid distinguish actual entry/survival/reap from labels/receipts.
Queued continuation is verified through public projections before any spawn;
individual reservation SQL rows remain outside that observation proof.
No global watchdog, positive authority SQL, paid model or installed service.
"""
import argparse
import ast
import ctypes
import json
import os
from pathlib import Path
import select
import selectors
import signal
import socket
import sqlite3
import stat
import struct
import subprocess
import sys
import time
import traceback
import types

import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as planning
import rust_native_parent_death as parent_death
import rust_workspace_fifo_completion as completion
import rust_mission_task_order as diagnostic

# Linux inotify ABI: syscall buffer size is a read batch, never a cap on events,
# iterations, lifetime or work. No artificial deadline is added to select().
LIBC = ctypes.CDLL(None, use_errno=True)
LIBC.inotify_init1.argtypes = [ctypes.c_int]
LIBC.inotify_init1.restype = ctypes.c_int
LIBC.inotify_add_watch.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_uint32]
LIBC.inotify_add_watch.restype = ctypes.c_int


def check(value, code):
    """Fail with static safe context, never dump private Agent/SQL/prompt values."""
    if not value:
        raise RuntimeError(code)


def retain_failure(errors, publish):
    """Attempt diagnostic publication without bypassing subsequent cleanup.

    The caller retains its primary exception separately. Append any publication
    failure (including interruption) to its error list, then return to cleanup;
    no logging failure establishes publication or successful recovery.
    """
    try:
        publish()
    except BaseException as error:
        errors.append(error)


def subreaper_scope(operation, report, control):
    """Run owned work and always attempt restoration of the original flag.

    control reads with no argument and sets with a boolean. After its initial
    read succeeds, restoration runs even if enable, work or reporting fails.
    Preserve primary, report and restoration failures independently; return the
    work result only when all required operations succeeded. No child is owned
    or signalled by this flag-management helper.
    """
    old = control()
    errors = []
    result = None
    try:
        control(True)
        result = operation()
    except BaseException as error:
        errors.append(error)
        retain_failure(errors, lambda: report(error))
    finally:
        try:
            control(old)
        except BaseException as error:
            errors.append(error)
    if errors:
        raise BaseExceptionGroup('watch observation/publication/subreaper restoration', errors)
    return result


def owned_provider_peer(connection, supervisor):
    """Authenticate the kernel peer/parent and hold its pidfd before reading.

    Same UID alone is insufficient: the live peer must descend directly from
    this verified supervisor. attach checks its start stamp around pidfd_open;
    reobserve the parent and stamp afterwards. Return identity plus owned fd;
    on refusal close any opened fd and retain a secondary close error. This
    grants no Agent authority and does not read peer-controlled frame bytes.
    """
    peer, uid, _ = struct.unpack('3i', connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
    check(uid == os.getuid(), 'provider kernel peer UID mismatch')
    identity = observation(peer)
    check(identity['ppid'] == supervisor['pid'], 'provider kernel parent is not supervisor')
    fd = attach(identity)
    try:
        current = observation(peer)
        check(current['stamp'] == identity['stamp'] and current['ppid'] == supervisor['pid'],
              'provider kernel identity changed during attachment')
    except BaseException as error:
        try:
            os.close(fd)
        except BaseException as cleanup:
            raise BaseExceptionGroup('provider ownership/descriptor close', [error, cleanup])
        raise
    return identity, fd


def watch_directory(path, modifications=False):
    """Own an inotify descriptor for file publication; return no timing policy.

    Events wake observation only; callers still inspect actual fresh JSON bytes.
    modifications additionally observes writes to open streams/SQLite files;
    it is a fixture notification subscription, never a product policy.
    Setup/close failures propagate independently rather than becoming absence.
    """
    fd = LIBC.inotify_init1(os.O_CLOEXEC | os.O_NONBLOCK)
    if fd < 0:
        raise OSError(ctypes.get_errno(), 'owned inotify initialization')
    mask = 0x00000008 | 0x00000080 | 0x00000100 | (0x00000002 if modifications else 0)
    if LIBC.inotify_add_watch(fd, os.fsencode(path), mask) < 0:
        error = OSError(ctypes.get_errno(), 'owned inotify watch')
        try:
            os.close(fd)
        except BaseException as cleanup:
            raise BaseExceptionGroup('inotify setup/close', [error, cleanup])
        raise error
    return fd


def wait_running(owned, agent_id, conductor_fd, supervisor_fd):
    """Wait for the parent-recorded running state after actual provider entry.

    The socket can arrive while the canonical Agent is still starting; requiring
    one immediate read races that parent write. Subscribe before the first read,
    then require actual running/permission state; child death or an unexpected
    state refuses. Inotify observes DB modifications, not a temporal watchdog.
    A structured SQLite BUSY during the Root identity Prepare remains refusal;
    actual DB activity is required before a fresh read, never an execution retry.
    No deadline/cap is invented and other identity refusals still fail immediately.
    Finally close our descriptor and preserve primary/cleanup causes together.
    """
    fd = watch_directory(owned.root / '.swarm', modifications=True)
    errors = []
    result = None
    pending = None
    try:
        while result is None:
            code,value=owned.call(['agent','show',agent_id])
            if code:
                failure=value.get('failure',{})
                labels=set(failure.get('causes',[]))
                pending=RuntimeError('public running-state read refused: '+json.dumps(failure,sort_keys=True))
                transient=(failure.get('code')=='root-database-identity' and labels=={'root-database-identity','database-identity-operation:Prepare','sqlite-code:5'})
                if not transient:raise pending
                # No success, no execution retry: observe actual DB activity or
                # owned child/conductor exit before another read-only probe.
                ready,_,_=select.select([fd,conductor_fd,supervisor_fd],[],[])
                check(conductor_fd not in ready and supervisor_fd not in ready,'worker or conductor exited after refused observation')
                if fd in ready:os.read(fd,65536)
                continue
            agent=value['agent']
            if agent['status'] == 'running':
                result = agent
                break
            check(agent['status'] == 'starting', 'worker departed before recorded running')
            ready, _, _ = select.select([fd, conductor_fd, supervisor_fd], [], [])
            check(conductor_fd not in ready and supervisor_fd not in ready,
                  'worker or conductor exited before recorded running')
            if fd in ready:
                os.read(fd, 65536)
    except BaseException as error:
        if pending is not None and pending is not error:errors.append(pending)
        errors.append(error)
    finally:
        try: os.close(fd)
        except BaseException as error: errors.append(error)
    if errors:
        raise BaseExceptionGroup('recorded running observation/close', errors)
    return result


def observation(pid, allow_terminal=False):
    """Read Linux PID/start stamp/parent/group/session/state from one stat record.

    The last closing comm parenthesis precedes Linux fields3..22. ABI offsets are
    representation, not policy. UID and live state are checked; this observation
    alone grants no signal rights and does not authenticate an Agent.
    """
    raw = Path('/proc', str(pid), 'stat').read_text()
    end = raw.rfind(')')
    check(end >= 0, 'process stat comm boundary missing')
    fields = raw[end + 2:].split()
    check(len(fields) > 19 and (allow_terminal or fields[0] not in ('Z', 'X')), 'observed process not live')
    check(Path('/proc', str(pid)).stat().st_uid == os.getuid(), 'process UID mismatch')
    return dict(pid=int(pid), stamp=fields[19], ppid=int(fields[1]),
                pgid=int(fields[2]), sid=int(fields[3]), state=fields[0])


def close_descriptor(fd, primary=None):
    """Close an owned descriptor, retaining a pending operation error on failure.

    Callers in finally pass sys.exception() so descriptor-close failure cannot
    replace the operation being unwound. A successful close leaves that pending
    exception to its caller; this helper grants no ownership or signal rights.
    """
    try:
        os.close(fd)
    except BaseException as cleanup:
        if primary is not None:
            raise BaseExceptionGroup('owned descriptor operation/close', [primary, cleanup])
        raise


def attach(identity):
    """Hold a pidfd only after the supplied stamp matches before AND after open.

    Signal permission still comes from this fixture's owned Popen/descendant
    chain, executable/argv and public Agent correlation, not this dictionary.
    """
    before = observation(identity['pid'])
    check(before['stamp'] == identity['stamp'], 'PID stamp changed before pidfd')
    fd = os.pidfd_open(identity['pid'])
    try:
        check(observation(identity['pid'])['stamp'] == identity['stamp'], 'PID stamp changed after pidfd')
        check(not select.select([fd], [], [], 0)[0], 'pidfd already departed')
    except BaseException as error:
        close_descriptor(fd, error)
        raise
    return fd


def frame(connection):
    """Read the finite owned provider JSON frame; EOF/partial output refuses.

    No global watchdog or hidden frame-size policy; peer/kernel ownership is
    checked before consuming the fixture frame. Text is never execution authority.
    """
    data = bytearray()
    while True:
        byte = connection.recv(1)
        if not byte:
            raise RuntimeError('provider control EOF before complete frame')
        if byte == b'\n':
            return json.loads(data)
        data.extend(byte)


def wait_json(path, process, process_fd, notify_fd):
    """Observe atomic file publication or actual SDK exit, without a timeout.

    No PID/file-existence success alone: caller validates the complete producer
    schema and actual kernel/Agent facts. Retain every original failure file.
    """
    with selectors.DefaultSelector() as selector:
        selector.register(process_fd, selectors.EVENT_READ, 'owner')
        selector.register(notify_fd, selectors.EVENT_READ, 'file')
        while True:
            if path.exists():
                info = path.lstat()
                check(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and info.st_nlink == 1,
                      'publication file ownership/type mismatch')
                return json.loads(path.read_bytes())
            for key, _ in selector.select():
                if key.data == 'owner':
                    if path.exists():
                        continue
                    process.poll()
                    raise RuntimeError('conductor exited before required publication')
                try:
                    os.read(notify_fd, 65536)
                except BlockingIOError:
                    pass  # A drained nonblocking event FD is not publication.


def adopted_supervisors(binary, root):
    """Inventory only this harness's actual kernel children after conductor reap.

    A live supervisor match requires exact normal executable SHA, exact owned
    helper/Root/Agent argv, UID, PID stamp and PPID. Session must be independent
    or still exactly inherited from this harness before setsid. Unknown live
    children are reported, NEVER signalled from labels. Unattributed zombies can
    be waitpid-reaped as real owned children without inventing their business role.
    This fallback does not depend on successful SDK spawned.json publication.
    """
    matched, unknown, reaped = [], [], []
    children = Path('/proc/self/task', str(os.getpid()), 'children').read_text().split()
    expected = [os.fsencode(binary), b'_mission_worker_entry', b'watch-worker-agent', b'--root', os.fsencode(root), b'--json']
    expected_hash = fixture.digest(binary.read_bytes())
    for text in children:
        pid = int(text)
        try:
            identity = observation(pid, allow_terminal=True)
            check(identity['ppid'] == os.getpid(), 'inventoried child parent changed')
            if identity['state'] in ('Z', 'X'):
                waited, status = os.waitpid(pid, os.WNOHANG)
                if waited:
                    reaped.append(dict(pid=waited, raw_wait_status=status, role='unattributed_owned_child'))
                continue
            argv = Path('/proc', str(pid), 'cmdline').read_bytes().split(b'\0')
            if argv and not argv[-1]:
                argv.pop()
            independent = identity['sid'] == pid and identity['pgid'] == pid
            inherited = identity['sid'] == os.getsid(0) and identity['pgid'] == os.getpgrp()
            same = (argv == expected and (independent or inherited)
                    and fixture.digest(Path('/proc', str(pid), 'exe').read_bytes()) == expected_hash)
            if same:
                check(observation(pid)['stamp'] == identity['stamp'], 'inventoried child stamp changed')
                identity['session_phase'] = 'verified-independent' if independent else 'pre-session-inherited'
                matched.append(identity)
            else:
                unknown.append(dict(pid=pid, stamp=identity['stamp'], state=identity['state'], role='unresolved_owned_child'))
        except FileNotFoundError:
            # The child may have exited between inventory and /proc read. Reap
            # only a real child; no exit status or semantic role is guessed.
            try:
                waited, status = os.waitpid(pid, os.WNOHANG)
                if waited:
                    reaped.append(dict(pid=waited, raw_wait_status=status, role='departed_unattributed_owned_child'))
            except ChildProcessError:
                pass
    return dict(matched=matched, unknown=unknown, reaped=reaped)


def send_command(process, command):
    """Send one checked control line; write/flush errors are real fixture errors."""
    process.stdin.write(command.encode())
    process.stdin.flush()


def supervision(owned):
    """Read actual owned lease history without changing policy or ownership.

    Missing/multiple rows refuse. These persisted labels do not prove a process
    alive/departed; only kernel observations/waitstatus establish those facts.
    """
    db = sqlite3.connect('file:' + str(owned.root / '.swarm/state.db') + '?mode=ro', uri=True)
    try:
        rows = db.execute('SELECT conductor_id,stopped_at FROM mission_supervision WHERE work_id=?',
                          (owned.work,)).fetchall()
        check(len(rows) == 1, 'actual conductor lease row missing or duplicated')
        return dict(conductor=rows[0][0], stopped_at=rows[0][1])
    finally:
        db.close()


def spool(path):
    """Create a private durable regular channel, never an inherited reader pipe."""
    return os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), 'wb')


def provider_program(nonce):
    """Build an inert provider with actual stdin/marker and kernel release gate.

    Socket Q only probes liveness. The physical nonce release file wakes inotify
    and is required before any normal result/report/exit. No model account, sleep,
    arbitrary timeout, retry ceiling or positive runtime-state seed is used.
    """
    return r'''#!/usr/bin/python3
import ctypes, hashlib, json, os, selectors, socket, sys
from pathlib import Path
nonce = ''' + repr(nonce) + r'''
args=sys.argv[1:];model=args[args.index('--model')+1];raw=sys.stdin.buffer.read()
observed=dict(pid=os.getpid(),ppid=os.getppid(),stdin_sha256=hashlib.sha256(raw).hexdigest(),stdin_bytes=len(raw),argv=args)
with open('model-received.json','x',encoding='utf-8') as stream:
    json.dump(observed,stream);stream.flush();os.fsync(stream.fileno())
libc=ctypes.CDLL(None,use_errno=True)
libc.inotify_init1.argtypes=[ctypes.c_int];libc.inotify_init1.restype=ctypes.c_int
libc.inotify_add_watch.argtypes=[ctypes.c_int,ctypes.c_char_p,ctypes.c_uint32];libc.inotify_add_watch.restype=ctypes.c_int
fd=libc.inotify_init1(os.O_CLOEXEC|os.O_NONBLOCK)
if fd<0:raise OSError(ctypes.get_errno(),'provider inotify')
try:
    if libc.inotify_add_watch(fd,b'.',8|128|256)<0:raise OSError(ctypes.get_errno(),'provider release watch')
    with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as control,selectors.DefaultSelector() as selector:
        control.connect('../watch-provider.sock')
        def reply():
            value=dict(pid=os.getpid(),ppid=os.getppid(),sid=os.getsid(0),pgid=os.getpgrp(),nonce=nonce)
            control.sendall((json.dumps(value)+'\n').encode())
        selector.register(control,selectors.EVENT_READ,'probe');selector.register(fd,selectors.EVENT_READ,'release')
        reply()
        while not Path('release-worker.json').exists():
            for key,_ in selector.select():
                if key.data=='probe':
                    if control.recv(1)!=b'Q':raise RuntimeError('provider control closed or invalid')
                    reply()
                else:
                    try:os.read(fd,65536)
                    except BlockingIOError:pass
        if json.loads(Path('release-worker.json').read_bytes())!={'nonce':nonce,'release':True}:raise RuntimeError('provider release mismatch')
finally:os.close(fd)
with open('handoff.md','x',encoding='utf-8') as report:
    report.write('Owned inert worker\nInput SHA256: '+observed['stdin_sha256']+'\n');report.flush();os.fsync(report.fileno())
print(json.dumps({'type':'system','subtype':'init','model':model}),flush=True)
print(json.dumps({'type':'assistant','message':{'model':model,'content':[{'type':'text','text':'owned independent worker'}]}}),flush=True)
print(json.dumps({'type':'result','usage':{'input_tokens':7,'output_tokens':8},'total_cost_usd':1}),flush=True)
'''


def verify_provider_delimiter(out):
    """Exercise ONLY the generated framing function against a byte sink/mock OS.

    No provider, process, socket, runtime grant or paid model runs. The same
    generated reply AST emits real bytes to the sink; require exact LF framing,
    never a backslash+n. Keep the lexical source/snippet result, not an assertion
    inferred from a JSON-escaped tool rendering.
    """
    program = provider_program('delimiter-control')
    tree = ast.parse(program)
    reply = next(node for node in ast.walk(tree) if isinstance(node, ast.FunctionDef) and node.name == 'reply')
    sent = []
    namespace = dict(json=json, nonce='delimiter-control',
        control=types.SimpleNamespace(sendall=sent.append),
        os=types.SimpleNamespace(getpid=lambda: 11, getppid=lambda: 7,
                                 getsid=lambda _pid: 7, getpgrp=lambda: 11))
    exec(compile(ast.Module(body=[reply], type_ignores=[]), 'generated-framing-only', 'exec'), namespace)
    namespace['reply']()
    check(len(sent) == 1 and sent[0][-1:] == b'\n' and not sent[0].endswith(b'\\n'), 'generated provider frame is not LF terminated')
    check(json.loads(sent[0][:-1])['nonce'] == 'delimiter-control', 'generated provider frame payload mismatch')
    fixture.publish(out / 'generated-framing-function.py.txt', ast.unparse(reply).encode())
    fixture.publish(out / 'generated-framing-check.json', dict(status='PASS_FRAME_ONLY_NO_PROVIDER',
        generated_sha256=fixture.digest(program.encode()), final_byte=sent[0][-1],
        frame_bytes=len(sent[0]), literal_backslash_n=False))


def verify_continuation_observation(value, agent, work):
    """Check typed SDK evidence of actual queued observation, never grant authority.

    Public Agent/Work/receipt/FIFO equality is reported separately from individual
    reservation rows, which this SDK cannot read. No private SQL/prompt is logged.
    Missing, empty, wrong-key, boolean-number and permission claims refuse; this
    validator does not itself call the engine or establish full ledger stability.
    """
    keys = {'status', 'target_agent_id', 'work', 'work_revision', 'settings_revision',
            'generation', 'target_count', 'entries', 'before', 'after',
            'public_projections_unchanged', 'agent_work_receipts_fifo_verified',
            'reservation_rows_verified', 'scope', 'process_started_by_observation',
            'observation_is_permission'}
    check(type(value) is dict and set(value) == keys, 'continuation evidence shape invalid')
    check(value['status'] == 'QUEUED_CONTINUATION_VERIFIED' and value['target_agent_id'] == agent
          and value['work'] == work, 'continuation target mismatch')
    for key in ('work_revision', 'generation', 'target_count'):
        check(type(value[key]) is int and value[key] > 0, 'continuation integer invalid')
    revision = value['settings_revision']
    check(type(revision) is str and revision.isascii() and revision.isdecimal()
          and int(revision) > 0 and str(int(revision)) == revision,
          'continuation settings revision invalid')
    check(value['target_count'] == 1 and type(value['entries']) is list
          and len(value['entries']) == 1, 'continuation target not unique')
    entry = value['entries'][0]
    check(type(entry) is dict and set(entry) == {'agent_id', 'task_id', 'observation'}
          and entry['agent_id'] == agent and type(entry['task_id']) is str and entry['task_id'],
          'continuation entry invalid')
    observation = entry['observation']
    check(type(observation) is dict and set(observation) == {'state', 'generation', 'current_lease_generation'}
          and observation['state'] == 'queued-unregistered-observed'
          and type(observation['generation']) is int and observation['generation'] == value['generation']
          and observation['current_lease_generation'] is True, 'continuation state not queued current generation')
    fact_keys = {'agent_sha256', 'work_sha256', 'receipt_count', 'fifo_turn_count', 'fifo_blocker_count'}
    for key in ('before', 'after'):
        facts = value[key]
        check(type(facts) is dict and set(facts) == fact_keys, 'continuation public facts missing')
        for name in ('agent_sha256', 'work_sha256'):
            digest = facts[name]
            check(type(digest) is str and len(digest) == 64
                  and all(char in '0123456789abcdef' for char in digest), 'continuation digest invalid')
        for name in ('receipt_count', 'fifo_turn_count', 'fifo_blocker_count'):
            check(type(facts[name]) is int and facts[name] >= 0, 'continuation fact counter invalid')
    check(value['before'] == value['after'] and value['public_projections_unchanged'] is True
          and value['agent_work_receipts_fifo_verified'] is True, 'continuation projections changed')
    check(value['reservation_rows_verified'] is False and value['process_started_by_observation'] is False
          and value['observation_is_permission'] is False
          and value['scope'] == 'public-projections-before-spawn', 'continuation scope overclaimed')


def verify_current_root_binding(binding, root, *, expected_version=4):
    """Check the explicitly declared fixture binding, defaulting to current v4.

    Version3 is a deliberate historical proof input: require its canonical parent
    hash/positive turn sequence and ABSENT root_instance, never infer identity.
    Version4 compares strict JSON integer pairs with physical directory/main DB
    metadata. No version is inferred from payload; declaration mismatch refuses.
    These observations validate DTOs, not engine FD authority, grant or same-UID
    authentication. Only isolated historical root probes override default4.
    """
    check(type(expected_version) is int and expected_version in (3, 4),
          'unsupported explicitly expected binding version')
    check(type(binding.get('schema_version')) is int and binding['schema_version'] == expected_version,
          'automatic intent differs from explicitly expected binding version')
    if expected_version == 3:
        parent, sequence = binding.get('parent_profile_sha256'), binding.get('workspace_turn_seq')
        check('root_instance' not in binding, 'historical Binding3 must omit Root instance')
        check(isinstance(parent, str) and len(parent) == 64
              and all(byte in '0123456789abcdef' for byte in parent), 'historical Binding3 parent hash invalid')
        check(isinstance(sequence, str) and sequence.isascii() and sequence.isdigit()
              and int(sequence) > 0 and str(int(sequence)) == sequence,
              'historical Binding3 turn sequence invalid')
        return
    identity = binding.get('root_instance')
    check(isinstance(identity, dict) and set(identity) == {'directory', 'database'},
          'Binding4 Root instance shape missing or unknown')
    for value in identity.values():
        check(isinstance(value, list) and len(value) == 2 and all(type(item) is int for item in value),
              'Binding4 Root instance pair type invalid')
    local, database = (root / '.swarm').stat(), (root / '.swarm/state.db').stat()
    check(identity == dict(directory=[local.st_dev, local.st_ino], database=[database.st_dev, database.st_ino]),
          'Binding4 Root instance differs from fixture observation')


def verify_completion(final, journal, running, work, supervisor, child, reservation, directory):
    """Validate owned receipt/public completion facts and original identities.

    Inputs are decoded actual outputs and readonly fixture observations, never
    authority. Require the four explicit native outcome booleans, exact receipt
    JSON-typed details and bound Agent/Work/process/mission identity, plus the originally
    reserved and now estimated envelope. Refuse missing facts or mismatches with
    static context; no task acceptance or model authenticity follows.
    """
    agent = final['agent']
    check(agent['status'] == 'completed' and final['task_accepted'] is False,
          'normal CLI final result is not observed completion')
    details = agent['native_execution_result']
    recorded = journal['details']
    check(isinstance(details, dict) and isinstance(recorded, dict), 'native completion facts missing')
    for key, expected in (('kernel_reaped', True), ('output_complete', True),
                          ('recovery_required', False), ('workspace_release_confirmed', True)):
        check(details.get(key) is expected and recorded.get(key) is expected,
              'native completion required fact missing or invalid')
    check(journal['schema_version'] == 1 and journal['directory'] == directory,
          'native completion receipt Root/schema mismatch')
    # Serialization preserves JSON boolean/number distinctions that Python's
    # recursive equality erases (True == 1 and False == 0). Both producers use
    # decoded JSON; rejecting nonfinite values also refuses malformed evidence.
    check(json.dumps(recorded, sort_keys=True, allow_nan=False)
          == json.dumps(details, sort_keys=True, allow_nan=False),
          'native receipt differs from public completion facts')
    check(journal['incomplete_output'] is False and journal['exit_code'] == 0
          and journal['failed'] is False and journal['interrupted'] is False
          and journal['may_have_started'] is True, 'native receipt successful reap facts mismatch')
    original = journal['expected']
    for value in (original, agent):
        check(value['id'] == running['id'] and value['work_id'] == work
              and value['supervisor_pid'] == supervisor['pid'] and value['child_pid'] == child['pid'],
              'native receipt Agent Work or process identity mismatch')
        for key in ('task_id', 'attempt_id', 'supervisor_identity', 'child_identity', 'native_mission'):
            check(value.get(key) == running.get(key), 'native receipt original binding mismatch')
    check(journal['reserved'] == [work, 1.0, 'reserved']
          and reservation == [work, 1.0, 'estimated'], 'native completion reservation identity mismatch')


def retain_completion(owned, out, final, running, supervisor, child):
    """Read the owned fixture receipt, validate it, retain exact bytes and SHA.

    The filename uses the engine's SHA256-of-Agent-id protocol. Only readonly
    SQLite observation is used; writes target fresh evidence outputs. Preserve
    opaque receipt fields verbatim without printing private prompt/output data.
    Errors prevent PASS and enter the normal case cleanup; this is not receipt
    authentication against another same-UID writer or general recovery proof.
    """
    name = 'native-completion-' + fixture.digest(running['id'].encode()) + '.json'
    path = owned.root / '.swarm' / name
    metadata = path.lstat()
    check(stat.S_ISREG(metadata.st_mode) and metadata.st_uid == os.getuid()
          and metadata.st_nlink == 1, 'native receipt ownership/type mismatch')
    raw = path.read_bytes()
    journal = json.loads(raw)
    db = sqlite3.connect('file:' + str(owned.root / '.swarm/state.db') + '?mode=ro', uri=True)
    try:
        row = db.execute('SELECT work_id,amount,state FROM reservations WHERE agent_id=?', (running['id'],)).fetchone()
        check(row is not None, 'native receipt reservation missing')
        reservation = list(row)
    finally:
        close_error = sys.exception()
        try:
            db.close()
        except BaseException as error:
            if close_error is not None:
                raise BaseExceptionGroup('completion reservation observation/close', [close_error, error])
            raise
    local = (owned.root / '.swarm').stat()
    verify_completion(final, journal, running, owned.work, supervisor, child,
                      reservation, [local.st_dev, local.st_ino])
    fixture.publish(out / name, raw)
    retained = dict(file=name, sha256=fixture.digest(raw), agent_id=running['id'],
                    work_id=owned.work, reservation=reservation,
                    public_details_equal=True, required_native_facts_verified=True)
    fixture.publish(out / 'native-completion-evidence.json', retained)
    return retained


def supervisor_session(initial, conductor, established):
    """Reobserve the same owned child at spawn or after actual provider entry.

    PID/start stamp and direct conductor parent are required at both phases.
    Spawn may still inherit the session; established additionally requires a new
    SID/PGID equal to the child PID and distinct from the conductor session.
    Return the actual observation for subsequent adoption/survival checks.
    This is not Agent permission, a signal grant or a wait-status substitute.
    """
    current = observation(initial['pid'])
    check(current['stamp'] == initial['stamp'] and current['ppid'] == conductor['pid'],
          'bootstrap supervisor PID stamp or parent changed')
    if established:
        check(current['sid'] == current['pid'] and current['pgid'] == current['pid']
              and current['sid'] != conductor['sid'], 'supervisor session is not independent')
    return current


def helper_refusals(binary, out):
    """Run only malformed private-bootstrap argv in isolated subprocesses.

    Before any real provider case, require usage exit2, no success stdout and
    no file mutation/new Root in an owned cwd. Never invoke a valid helper or
    session syscall in this Python process. Retain exact argv/channels; this
    proves these malformed cases only, not valid launch or general confinement.
    """
    out.mkdir(mode=0o700, exist_ok=False)
    cwd = out / 'cwd'
    cwd.mkdir(mode=0o700)
    fixture.publish(cwd / 'sentinel', b'owned unchanged fixture\n')
    root = cwd / 'unopened-root'
    def inventory():
        """Read all owned cwd entries without ignoring newly created folders."""
        return {str(path.relative_to(cwd)): dict(mode=path.lstat().st_mode,
                sha256=fixture.digest(path.read_bytes()) if path.is_file() else None)
                for path in cwd.rglob('*')}
    before = inventory()
    variants = [('missing-agent', ['--root', str(root), '--json']),
                ('help', ['watch-worker-agent', '--root', str(root), '--json', '--help']),
                ('relative-root', ['watch-worker-agent', '--root', 'relative-root', '--json']),
                ('without-json', ['watch-worker-agent', '--root', str(root)])]
    for label, arguments in variants:
        argv = [str(binary), '_mission_worker_entry'] + arguments
        result = subprocess.run(argv, cwd=cwd, capture_output=True)
        fixture.publish(out / (label + '.stdout'), result.stdout)
        fixture.publish(out / (label + '.stderr'), result.stderr)
        fixture.publish(out / (label + '.command.json'), dict(argv=argv, returncode=result.returncode))
        check(result.returncode == 2 and not result.stdout and result.stderr,
              'malformed private bootstrap did not refuse with usage')
        check(inventory() == before and not root.exists() and not (cwd / 'relative-root').exists(),
              'malformed private bootstrap changed owned files or created Root')
    fixture.publish(out / 'result.json', dict(status='PASS', refused=[label for label, _ in variants],
                    file_mutation=False, new_root=False, valid_helper_invoked=False))


def run_case(binary, sdk, normal, profile, out, mode):
    """Run one actual conductor exit with known owned cleanup and genuine reap.

    Graceful witness runs before SIGTERM on a fresh same-contract Root. Subreaper
    adoption plus exact waitpid yields normal CLI status; spool/receipt is NEVER
    a substitute. Signal cleanup only targets owned, stamp-bound fixture actors.
    Unknown effects remain failure, preserving all files and independent causes.
    """
    out.mkdir(parents=True, exist_ok=False)
    ready, owned = planning.flow(binary, out / 'planning', before_worker=True, return_fixture=True)
    root = owned.root
    nonce = fixture.digest((mode + str(root)).encode())
    fixture.publish(root / 'codex', provider_program(nonce).encode())
    (root / 'codex').chmod(0o700)
    listener = None
    process = connection = None
    sdk_fd = cli_fd = provider_fd = notify_fd = None
    cli = provider = None
    cli_reaped = False
    channels = []
    primary = None
    cleanup = []
    result = None
    request = dict(kind='owned-inert-watch-worker-fixture', root=str(root), work=ready['work_id'],
        binary=str(binary), binary_sha256=normal['sha256'], mode=mode,
        result=str(out / 'sdk-result.json'), spawned=str(out / 'spawned.json'),
        ready=str(out / 'provider-ready.json'), supervisor_stdout=str(out / 'supervisor.stdout'),
        supervisor_stderr=str(out / 'supervisor.stderr'))
    fixture.publish(root / '.swarm/verification-watch-worker.json', request)
    fixture.publish(out / 'request.json', request)
    try:
        listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        listener.bind(str(root / 'watch-provider.sock'))
        os.chmod(root / 'watch-provider.sock', 0o600)
        listener.listen(1)
        notify_fd = watch_directory(out)
        stdout = spool(out / 'conductor.stdout')
        channels.append(stdout)
        stderr = spool(out / 'conductor.stderr')
        channels.append(stderr)
        argv = [str(sdk), str(out / 'request.json')]
        fixture.publish(out / 'command.json', dict(argv=argv, mode=mode, global_watchdog=None))
        process = subprocess.Popen(argv, env=owned.environment, stdin=subprocess.PIPE, stdout=stdout, stderr=stderr)
        conductor = observation(process.pid)
        sdk_fd = attach(conductor)
        spawned = wait_json(Path(request['spawned']), process, sdk_fd, notify_fd)
        check(spawned['status'] == 'SUPERVISOR_SPAWNED' and spawned['agent_id'] == 'watch-worker-agent', 'wrong spawn producer document')
        verify_continuation_observation(spawned.get('continuation_observation'), spawned['agent_id'], request['work'])
        print('PASS actual queued continuation public projections verified before spawn; reservation rows unverified', flush=True)
        check(spawned['conductor']['pid'] == process.pid and spawned['conductor']['stamp'] == conductor['stamp'], 'conductor correlation mismatch')
        check(spawned['binary_sha256'] == normal['sha256'], 'spawned normal binary hash mismatch')
        check(spawned['argv'] == [str(binary), '_mission_worker_entry', 'watch-worker-agent', '--root', str(root), '--json'], 'spawn argv differs from qualified existing CLI')
        cli = spawned['supervisor']
        cli_fd = attach(cli)
        check(spawned['session_state'] == 'pending', 'spawn must not claim verified session')
        cli = supervisor_session(cli, conductor, False)
        with selectors.DefaultSelector() as selector:
            selector.register(listener, selectors.EVENT_READ, 'provider')
            selector.register(sdk_fd, selectors.EVENT_READ, 'conductor')
            selector.register(cli_fd, selectors.EVENT_READ, 'supervisor')
            while connection is None:
                for key, _ in selector.select():
                    if key.data != 'provider':
                        raise RuntimeError('owned process exited before actual provider entry')
                    connection, _ = listener.accept()
        provider, provider_fd = owned_provider_peer(connection, cli)
        peer = provider['pid']
        entry = frame(connection)
        check(entry['pid'] == peer and entry['ppid'] == cli['pid'] and entry['nonce'] == nonce, 'provider entry/kernel peer mismatch')
        send_command(process, 'verify-provider\n')
        verified = wait_json(Path(request['ready']), process, sdk_fd, notify_fd)
        check(verified['status'] == 'PROVIDER_VERIFIED_UNDER_LIVE_CONDUCTOR', 'provider not verified by actual conductor')
        check(verified['provider']['pid'] == peer and verified['provider']['stamp'] == provider['stamp'], 'provider start stamp mismatch')
        check(verified['supervisor']['pid'] == cli['pid'] and verified['supervisor']['stamp'] == cli['stamp'], 'verified supervisor differs from spawn')
        check(verified['session_state'] == 'verified-independent', 'provider verification lacks established session')
        cli = supervisor_session(cli, conductor, True)
        check(all(verified['supervisor'][key] == cli[key] for key in ('pid', 'stamp', 'ppid', 'sid', 'pgid')),
              'verified session differs from kernel observation')
        agent = owned.ok(['agent', 'show', 'watch-worker-agent'])['agent']
        check(agent['status'] == 'running' and agent['supervisor_pid'] == cli['pid'] and agent['child_pid'] == peer, 'actual registered worker does not match kernel actors')
        check(agent['native_execution']['phase'] == 'permission_recorded', 'actual native grant absent')
        verify_current_root_binding(agent['native_mission'], root)
        before_turn = completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, agent['id'])
        before_money = completion.reservation(owned, agent['id'])
        check(before_turn['state'] == 'acquired' and before_money == ['reserved', 1.0], 'owned running resources not held')
        before_lease = supervision(owned)
        check(before_lease['conductor'] == agent['native_mission']['conductor'] and not before_lease['stopped_at'],
              'actual running lease provenance mismatch')
        release = root / 'watch-worker/release-worker.json'
        check(not release.exists(), 'provider release must not exist before conductor exit')
        if mode == 'graceful':
            send_command(process, 'stop-conductor\n')
        else:
            check(observation(process.pid)['stamp'] == conductor['stamp'], 'conductor identity changed before owned SIGTERM')
            signal.pidfd_send_signal(sdk_fd, signal.SIGTERM)
        returncode = process.wait()
        check(returncode == (0 if mode == 'graceful' else -signal.SIGTERM), 'unexpected actual conductor wait status')
        fixture.publish(out / 'conductor-exit.json', dict(returncode=returncode, kernel_wait=True, mode=mode))
        if mode == 'graceful':
            stopped = json.loads(Path(request['result']).read_bytes())
            check(stopped['status'] == 'GRACEFUL_CONDUCTOR_STOPPED' and stopped['lease_released'] and stopped['root_closed'] and not stopped['worker_exit_observed'], 'graceful close receipt mismatch')
        else:
            check(not Path(request['result']).exists(), 'signal case must not invent graceful result')
        after_lease = supervision(owned)
        check(after_lease['conductor'] == before_lease['conductor'], 'conductor identity changed during exit')
        check(bool(after_lease['stopped_at']) == (mode == 'graceful'), 'lease history incorrectly claims signal cleanup')
        fixture.publish(out / 'lease-after-exit.json', dict(before=before_lease, after=after_lease, mode=mode))
        after_cli = observation(cli['pid'])
        check(after_cli['stamp'] == cli['stamp'] and after_cli['ppid'] == os.getpid() and after_cli['sid'] == cli['sid'], 'normal CLI not live/adopted after conductor exit')
        check(observation(peer)['stamp'] == provider['stamp'], 'actual provider departed with conductor')
        connection.sendall(b'Q')
        survived = frame(connection)
        check(survived['pid'] == peer and survived['ppid'] == cli['pid'] and survived['nonce'] == nonce, 'provider did not execute liveness reply after conductor reaped')
        check(completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, agent['id']) == before_turn, 'conductor exit released a running FIFO turn')
        check(completion.reservation(owned, agent['id']) == before_money, 'conductor exit released worker reservation')
        fixture.publish(out / 'post-conductor-survival.json', dict(cli=after_cli, provider=survived, provider_identity=provider, turn=before_turn, reservation=before_money))
        fixture.publish(release, dict(nonce=nonce, release=True))
        with selectors.DefaultSelector() as selector:
            selector.register(cli_fd, selectors.EVENT_READ)
            selector.select()
        waited, status = os.waitpid(cli['pid'], 0)
        cli_reaped = True
        check(waited == cli['pid'] and os.WIFEXITED(status) and os.WEXITSTATUS(status) == 0, 'independent normal CLI did not actually exit0')
        fixture.publish(out / 'supervisor-exit.json', dict(pid=waited, raw_wait_status=status, exited=True, exit_code=os.WEXITSTATUS(status), subreaper_waitpid=True))
        for name in ('supervisor_stdout', 'supervisor_stderr'):
            with open(request[name], 'rb') as stream:
                os.fsync(stream.fileno())
        final = json.loads(Path(request['supervisor_stdout']).read_bytes())
        receipt_evidence = retain_completion(owned, out, final, agent, cli, provider)
        after_turn = completion.one_turn(owned.ok(['mission', 'queue', owned.work]), owned.work, agent['id'])
        check(after_turn['state'] == 'released' and after_turn['seq'] == before_turn['seq'] and after_turn['agent'] == before_turn['agent'], 'completion did not release the exact acquired turn')
        check(completion.reservation(owned, agent['id']) == ['estimated', 1.0], 'completion did not retain estimated consumption')
        work = owned.ok(['work', 'show', owned.work])['work']
        check(next(value for value in work['tasks'] if value['id'] == 'task')['status'] == 'blocked', 'provider completion accepted task')
        before_replay = diagnostic.snapshot(owned)
        providers = completion.provider_files(root)
        owned.ok(['agent', 'reconcile', agent['id']])
        check(diagnostic.snapshot(owned) == before_replay and completion.provider_files(root) == providers, 'completion replay changed SQL or reran provider')
        result = dict(status='PASS', mode=mode, root=str(root), work=owned.work, independent_normal_cli=True,
            conductor_exit=returncode, provider_survived=True, supervisor_waitpid_exit=0,
            actual_provider_entry=True, released_turn=after_turn, task_accepted=False,
            completion_receipt=receipt_evidence,
            paid_provider=False, installed=False, global_watchdog=None, crash_cleanup_proven=False)
    except BaseException as error:
        primary = error
        retain_failure(cleanup, lambda: fixture.publish(out / 'failure.json',
            dict(status='FAIL', root=str(root), mode=mode, cause=traceback.format_exc())))
    if primary is not None:
        # Failure-only cleanup is not acceptance evidence. Use the public owned
        # stop intention, not arbitrary PID labels or private SQL authority.
        try:
            if provider is not None and provider_fd is not None and not select.select([provider_fd], [], [], 0)[0]:
                check(observation(provider['pid'])['stamp'] == provider['stamp'], 'cleanup provider stamp changed')
                cleanup_release = root / 'watch-worker/release-worker.json'
                if cleanup_release.exists():
                    check(json.loads(cleanup_release.read_bytes()) == dict(nonce=nonce, release=True),
                          'cleanup will not overwrite an unrelated release file')
                else:
                    fixture.publish(cleanup_release, dict(nonce=nonce, release=True))
        except BaseException as error:
            cleanup.append(error)
        try:
            if process is not None and (cli is None or (not cli_reaped and cli_fd is not None and not select.select([cli_fd], [], [], 0)[0])):
                # A publication/read failure can precede our CLI metadata. Public
                # stop by the fixed owned event remains authoritative; absence or
                # an unsupported state is an explicit secondary cleanup failure.
                owned.ok(['agent', 'stop', 'watch-worker-agent'])
        except BaseException as error:
            cleanup.append(error)
        try:
            if process is not None and process.poll() is None:
                check(observation(process.pid)['stamp'] == conductor['stamp'], 'cleanup conductor stamp mismatch')
                signal.pidfd_send_signal(sdk_fd, signal.SIGTERM)
                process.wait()
        except BaseException as error:
            cleanup.append(error)
        try:
            if cli is None and Path(request['spawned']).exists():
                partial = json.loads(Path(request['spawned']).read_bytes())
                check(partial['binary_sha256'] == normal['sha256'] and partial['agent_id'] == 'watch-worker-agent',
                      'partial cleanup publication identity mismatch')
                check(partial['conductor']['pid'] == process.pid, 'partial cleanup conductor mismatch')
                cli = partial['supervisor']
            if cli is not None and not cli_reaped and cli_fd is not None:
                # SDK is reaped first. A still-live known supervisor interrupt
                # additionally requires its held pidfd, exact stamp and actual
                # binary/Root/Agent argv. No unknown PID or group is signalled.
                if cli_fd is not None and not select.select([cli_fd], [], [], 0)[0]:
                    current = observation(cli['pid'])
                    check(current['stamp'] == cli['stamp'] and current['ppid'] == os.getpid(), 'cleanup CLI identity changed')
                    actual_argv = Path('/proc', str(cli['pid']), 'cmdline').read_bytes().split(b'\0')
                    check(actual_argv[:-1] == [os.fsencode(binary), b'_mission_worker_entry', b'watch-worker-agent', b'--root', os.fsencode(root), b'--json']
                          and fixture.digest(Path('/proc', str(cli['pid']), 'exe').read_bytes()) == normal['sha256'],
                          'cleanup CLI executable or owned arguments mismatch')
                    signal.pidfd_send_signal(cli_fd, signal.SIGTERM)
                waited, status = os.waitpid(cli['pid'], 0)
                cli_reaped = waited == cli['pid']
                fixture.publish(out / 'cleanup-supervisor-wait.json', dict(pid=waited, raw_wait_status=status))
        except BaseException as error:
            cleanup.append(error)
    if provider is not None and cli_reaped:
        try:
            waited, status = os.waitpid(provider['pid'], os.WNOHANG)
            if waited == 0:
                check(provider_fd is not None and observation(provider['pid'])['stamp'] == provider['stamp'],
                      'adopted cleanup provider identity changed')
                signal.pidfd_send_signal(provider_fd, signal.SIGTERM)
                waited, status = os.waitpid(provider['pid'], 0)
            fixture.publish(out / 'cleanup-provider-wait.json', dict(pid=waited, raw_wait_status=status, cleanup_only=True))
        except ChildProcessError:
            pass  # The actual CLI already reaped its child: no status invented.
        except BaseException as error:
            cleanup.append(error)
    if primary is not None and process is not None:
        try:
            if process.poll() is None:
                # A failed observation may prevent a safe signal. Closing this
                # owned control pipe gives the SDK a checked EOF, then reap it.
                process.stdin.close()
                process.wait()
            inventory = adopted_supervisors(binary, root)
            fixture.publish(out / 'cleanup-adopted-inventory.json', inventory)
            for identity in inventory['matched']:
                descriptor = None
                try:
                    descriptor = attach(identity)
                    try:
                        owned.ok(['agent', 'stop', 'watch-worker-agent'])
                    except BaseException as error:
                        cleanup.append(error)
                    # Public stop is authoritative; if that runtime still needs
                    # an owned interrupt, bind it to the same verified pidfd.
                    if not select.select([descriptor], [], [], 0)[0]:
                        check(observation(identity['pid'])['stamp'] == identity['stamp'], 'cleanup supervisor identity changed')
                        signal.pidfd_send_signal(descriptor, signal.SIGTERM)
                    waited, status = os.waitpid(identity['pid'], 0)
                    fixture.publish(out / ('cleanup-adopted-' + str(waited) + '.json'), dict(pid=waited, raw_wait_status=status, cleanup_only=True))
                finally:
                    if descriptor is not None:
                        close_descriptor(descriptor, sys.exception())
            check(not inventory['unknown'], 'unresolved adopted child remains; no unknown PID was signalled')
        except BaseException as error:
            cleanup.append(error)
    for resource in [connection, listener, process.stdin if process is not None else None]:
        if resource is not None:
            try:
                resource.close()
            except BaseException as error:
                cleanup.append(error)
    for stream in channels:
        for operation in (stream.flush, lambda stream=stream: os.fsync(stream.fileno()), stream.close):
            try:
                operation()
            except BaseException as error:
                cleanup.append(error)
    for fd in [provider_fd, cli_fd, sdk_fd, notify_fd]:
        if fd is not None:
            try:
                os.close(fd)
            except BaseException as error:
                cleanup.append(error)
    if primary is not None or cleanup:
        raise BaseExceptionGroup('owned watch fixture observation/cleanup failed', ([primary] if primary else []) + cleanup)
    fixture.publish(out / 'result.json', result)
    return result


def main():
    """Qualify exact normal/SDK identities, run witness then SIGTERM and restore.

    All original cases/errors survive. Set/restore only this harness's subreaper
    flag and require real adopted-child waitstatus. No build/installation occurs.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'sdk-binary', 'sdk-profile', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    binary, sdk, out = args.binary.resolve(), args.sdk_binary.resolve(), args.output.resolve()
    out.mkdir(parents=True, mode=0o700, exist_ok=False)
    fixture.publish(out / 'method.py', Path(__file__).read_bytes())
    verify_provider_delimiter(out)
    normal, profile = json.loads(args.profile.read_bytes()), json.loads(args.sdk_profile.read_bytes())
    check(normal['no_fixture_feature'] and normal['sha256'] == fixture.digest(binary.read_bytes()), 'normal profile/hash mismatch')
    check(profile['example'] == 'mission_watch_worker' and profile['engine_features_empty'] and profile['shared_workspace_features_match_normal'] and not profile['test_profile'], 'SDK effective default profile mismatch')
    check(profile['normal_profile_sha256'] == normal['sha256'] and profile['binary_sha256'] == fixture.digest(sdk.read_bytes()), 'SDK profile/hash mismatch')
    fixture.publish(out / 'profiles.json', dict(normal=normal, sdk=profile))
    sources = {str(Path(module.__file__).resolve()): fixture.digest(Path(module.__file__).read_bytes())
        for name, module in tuple(sys.modules.items()) if name.startswith('rust_') and getattr(module, '__file__', None)}
    sources[str(Path(__file__).resolve())] = fixture.digest(Path(__file__).read_bytes())
    fixture.publish(out / 'inputs.json', dict(sources=sources, binary_sha256=normal['sha256'], sdk_sha256=profile['binary_sha256']))
    def cases():
        """Run both qualified fixtures and verify their actual source stability."""
        helper_refusals(binary, out / 'helper-refusals')
        print('PASS malformed private bootstrap refuses without Root or file mutation', flush=True)
        graceful = run_case(binary, sdk, normal, profile, out / 'graceful', 'graceful')
        print('PASS actual granted worker survives graceful conductor exit and completes independently', flush=True)
        signalled = run_case(binary, sdk, normal, profile, out / 'signal', 'signal')
        print('PASS actual granted worker survives conductor SIGTERM and completes independently', flush=True)
        check(all(fixture.digest(Path(path).read_bytes()) == sha for path, sha in sources.items()), 'verification source changed during execution')
        check(fixture.digest(binary.read_bytes()) == normal['sha256'] and fixture.digest(sdk.read_bytes()) == profile['binary_sha256'], 'exercised binary changed')
        return dict(status='PASS', cases=[graceful, signalled], actual_kernel_waitpid=True,
            paid_provider=False, task_accepted=False, installed=False, source_stable=True, global_watchdog=None)
    results = subreaper_scope(cases,
        lambda error: fixture.publish(out / 'failure.json',
            dict(status='FAIL', cause=''.join(traceback.format_exception(error)))),
        parent_death.subreaper)
    fixture.publish(out / 'result.json', results)
    print('REPLAY_EVIDENCE=' + str(out), flush=True)


if __name__ == '__main__':
    main()
