"""Owned process/error transport for the proposed Gi W/F parent. No import action.

No retry, signal to children, watchdog, model, SQL or authority is provided.
Unknown wait transports the actual owner to the terminal controller, keeping all
handles. Irrecoverable settlement fail-stops with that owner alive, never PASS.
"""
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import rust_agent_model_route as fixture
import rust_mission_watch_worker as watch


class Failure(Exception):
    """Own original typed causes and any unsettled process until safe consumption.

    code is a caller-owned static diagnostic, not private exception text. facts
    contain observations, not permissions. owner is the actual Popen carrier.
    """
    def __init__(self, code, causes=(), owner=None, facts=None):
        """Retain original objects; this constructor performs no publication/IO."""
        super().__init__(code)
        self.code, self.causes, self.owner = code, tuple(causes), owner
        self.facts = facts or {}

    def __str__(self):
        """Render only the safe graph; never stringify an original private cause."""
        return json.dumps(safe(self), sort_keys=True)


def check(value, code):
    """Refuse with a static reason without dumping input or private error text."""
    if not value:
        raise Failure(code)


def safe(error):
    """Project types/errno/static reasons and process observations, not messages.

    Traverses explicit groups and original cause/context with cycle detection.
    No traceback, environment, prompt or private stdout/stderr text is rendered.
    The original objects remain owned; this projection never replaces them.
    """
    pending, seen, nodes = [((), error)], set(), []
    while pending:
        path, value = pending.pop()
        if id(value) in seen:
            continue
        seen.add(id(value))
        node = dict(path=list(path), type=type(value).__name__)
        if isinstance(value, OSError):
            node['errno'] = value.errno
        if isinstance(value, Failure):
            node['code'] = value.code
            node['observations'] = value.facts
            if value.owner is not None:
                node['owned_process'] = value.owner.safe_summary()
            children = value.causes
        elif isinstance(value, BaseExceptionGroup):
            children = value.exceptions
        else:
            children = ()
        nodes.append(node)
        pending.extend(((*path, index), cause) for index, cause in enumerate(children))
        for label, cause in [('cause', value.__cause__), ('context', value.__context__)]:
            if cause is not None:
                pending.append(((*path, label), cause))
    return dict(causes=nodes, no_rollback_inference=True)


def host_tuple():
    """Read the same hostname/boot/PID namespace tuple as SupervisorIdentity."""
    import socket
    parts = (socket.gethostname(), Path('/proc/sys/kernel/random/boot_id').read_text().strip(), os.readlink('/proc/self/ns/pid'))
    check(all(parts), 'parent-host-incomplete')
    return ':'.join(parts)


class OwnedRun:
    """Actual sole parent of one Popen child, raw files and optional pidfd.

    wait errors preserve this object including all handles. close is allowed only
    after observed reap or before spawn. No PID from JSON authorizes a signal.
    """
    def __init__(self, output, argv, environment, observe=False):
        """Allocate an in-memory owner only; run creates fresh evidence/spawns."""
        self.output, self.argv = Path(output), list(map(str, argv))
        self.environment, self.observe = environment, observe
        self.process = self.pidfd = self.identity = self.status = None
        self.streams, self.errors = [], []
        self.reaped = self.finalized = False
        self.exit_observation = None
        self.consumed_wait = None
        self.wait_validated = False
        self.capture_attempted = False
        self.stream_closures = []
        self.pidfd_close_state = 'not-attempted'
        self.captured = {}
        self.artifacts = {}
        self.publications_attempted = []

    def pending(self):
        """Return unresolved owned effects that prohibit terminal release.

        A received but unvalidated wait is distinct from missing reap. Capture
        or pidfd uncertainty also keeps this owner alive after known reap.
        These reasons grant no permission to retry, close, validate or transfer.
        Only observed settlement or an explicitly authorized receiver can change
        their underlying facts; there is no automatic arbitration here.
        """
        reasons = []
        if self.process is not None and not self.reaped:
            reasons.append('owned-reap-unknown')
        if (self.consumed_wait is not None or self.process is not None) and not self.wait_validated:
            reasons.append('wait-validation-pending')
        if self.capture_attempted and not self.finalized:
            reasons.append('capture-closure-pending')
        if not self.capture_attempted and (self.streams or self.pidfd is not None):
            reasons.append('capture-not-attempted')
        if self.pidfd_close_state == 'unknown' or (self.finalized and self.pidfd is not None):
            reasons.append('pidfd-closure-pending')
        return reasons

    def summary(self):
        """Return lifecycle facts; None held-state means closure is unknown, not open.

        Consumed wait, owned reap, status and validation are distinct observations.
        Pending reasons are shared with observers and the terminal. Capture-state
        dictionaries are copied so later decisions cannot rewrite this snapshot.
        No private exception text or inferred success is included.
        """
        return dict(pending=self.pending(),pid=self.process.pid if self.process is not None else None,
                    identity=self.identity, returncode=self.status, reap_confirmed=self.reaped,
                    pidfd_held=(self.pidfd is not None) if self.pidfd_close_state != 'unknown' else None,
                    streams_held=(None if any(state['state'] == 'unknown' for state in self.stream_closures) else any(state['state'] == 'open' for state in self.stream_closures)) if self.capture_attempted else bool(self.streams),
                    capture_attempted=self.capture_attempted,capture_closed_observed=self.finalized,
                    stream_closures=[dict(state) for state in self.stream_closures],pidfd_close_state=self.pidfd_close_state,
                    consumed_wait=None if self.consumed_wait is None else dict(
                        pid=self.consumed_wait.si_pid,code=self.consumed_wait.si_code,status=self.consumed_wait.si_status),
                    wait_validated=self.wait_validated,kernel_exit_observation=self.exit_observation,
                    raw_hashes={n:fixture.digest(b) for n,b in self.captured.items()},
                    artifacts=self.artifacts,publications_attempted=self.publications_attempted,
                    output=str(self.output), launch='started' if self.process is not None else 'unobserved')

    def publish(self,name,value):
        """Retain publication intent separately from confirmed artifact hashes.

        A late fsync refusal can leave visible bytes, so an unconfirmed attempt
        never becomes an absent file or successful publication by inference.
        """
        self.publications_attempted.append(name)
        fixture.publish(self.output/name,value)
        self.artifacts[name]=fixture.digest((self.output/name).read_bytes())

    def safe_summary(self):
        """Project lifecycle and identity hash, keeping raw host/paths private."""
        value=self.summary()
        value.pop('output',None)
        identity=value.pop('identity',None)
        value['identity_sha256']=fixture.digest(json.dumps(identity,sort_keys=True).encode()) if identity is not None else None
        return value

    def finish_capture(self):
        """Attempt capture once after known reap; distinguish close observations.

        Attempted is not closed. A failed/interrupted close retains its stream
        and typed cause with explicit unknown state; it is never retried here.
        The terminal receives pending closures even for an already reaped owner.
        """
        check(self.process is None or self.reaped, 'parent-close-before-settlement')
        if self.capture_attempted:
            return
        self.stream_closures = [dict(close_attempted=False,close_returned=False,closed_observed=None,state='open') for _ in self.streams]
        self.capture_attempted = True
        for stream,state in zip(self.streams,self.stream_closures):
            for action in (stream.flush, lambda stream=stream: os.fsync(stream.fileno())):
                try:
                    action()
                except BaseException as error:
                    self.errors.append(error)
            state['close_attempted'] = True
            state['state'] = 'unknown'
            try:
                stream.close()
                state['close_returned'] = True
            except BaseException as error:
                self.errors.append(error)
            try:
                state['closed_observed'] = bool(stream.closed)
                if state['closed_observed']:
                    state['state'] = 'closed'
            except BaseException as error:
                self.errors.append(error)
        if self.pidfd is not None:
            self.pidfd_close_state = 'unknown'
            try:
                os.close(self.pidfd)
                self.pidfd = None
                self.pidfd_close_state = 'closed'
            except BaseException as error:
                self.errors.append(error)
        self.finalized = all(state['state'] == 'closed' for state in self.stream_closures) and self.pidfd_close_state != 'unknown'
        for name in ('stdout.raw','stderr.raw'):
            try:
                self.captured[name]=(self.output/name).read_bytes()
            except BaseException as error:
                self.errors.append(error)

    def wait_kernel(self):
        """Single kernel status authority: observe exit without reap, then consume.

        Resume only this owner state on interruption; never call Popen.wait or
        guess success from ECHILD. Exact spawned child/PIDFD remains owned. A
        retained exit observation proves status, but only consumed wait proves
        reap; errors preserve both distinctions and every still-owned handle.
        Received consumption is persisted before validation, which is separate
        from owned-PID reap and a known status. Refusal never discards the receipt.
        """
        kind, identifier = (os.P_PIDFD, self.pidfd) if self.pidfd is not None else (os.P_PID, self.process.pid)
        if self.exit_observation is None:
            value = os.waitid(kind, identifier, os.WEXITED | os.WNOWAIT)
            check(value.si_pid == self.process.pid, 'parent-wait-child-mismatch')
            self.exit_observation = dict(pid=value.si_pid, code=value.si_code, status=value.si_status)
        if self.consumed_wait is None:
            # Store the native consumption receipt directly on the owner BEFORE
            # validation that can refuse. Never consume again just because that
            # later validation failed or was interrupted.
            self.consumed_wait = os.waitid(kind, identifier, os.WEXITED)
        value = self.consumed_wait
        self.reaped = value.si_pid == self.process.pid
        check(self.reaped, 'parent-consumed-child-mismatch')
        if value.si_code == os.CLD_EXITED:
            check(type(value.si_status) is int and 0 <= value.si_status <= 255,'parent-consumed-exit-status')
            self.status = value.si_status
        else:
            check(value.si_code in (os.CLD_KILLED, os.CLD_DUMPED)
                  and type(value.si_status) is int and value.si_status > 0, 'parent-wait-not-terminal')
            self.status = -value.si_status
        self.process.returncode = self.status
        consumed = dict(pid=value.si_pid, code=value.si_code, status=value.si_status)
        check(consumed == self.exit_observation, 'parent-reap-observation-changed')
        self.wait_validated = True

    def settle(self):
        """Receive this explicit owner and complete its pending kernel wait.

        No new process, command, kill, timer or guessed status. Original failures
        stay failures even if settlement later succeeds. Irrecoverable wait keeps
        the capsule alive for the terminal fail-stop consumer. A consumed but
        unvalidated receipt refuses unchanged settlement without another wait.
        """
        if self.consumed_wait is not None and not self.wait_validated:
            # Known consumption plus rejected validation is for arbitration,
            # not another consuming wait. Preserve the explicit receipt/owner.
            raise Failure('parent-consumed-wait-unvalidated',owner=self)
        if not self.reaped and self.process is not None:
            self.wait_kernel()
        self.finish_capture()
        self.publish('settlement.json', self.summary())

    def run(self):
        """Spawn once, observe when required, wait whole process and retain raw.

        Errors before/after spawn are owned. Unknown wait raises Failure(owner=self)
        without closing handles. A controller MUST consume that owner before exit.
        Rejected wait validation or unconfirmed capture closure also refuses
        success even when an owned terminal status is known.
        """
        self.output.mkdir(mode=0o700, exist_ok=False)
        self.publish('command.json', dict(argv=self.argv, global_watchdog=None, scope='intent only'))
        try:
            for name in ('stdout.raw', 'stderr.raw'):
                self.streams.append((self.output/name).open('xb'))
            self.process = subprocess.Popen(self.argv, stdin=subprocess.DEVNULL, stdout=self.streams[0], stderr=self.streams[1], env=self.environment)
            self.publish('launch.json', dict(pid=self.process.pid))
            self.pidfd = os.pidfd_open(self.process.pid)
            if self.observe:
                before = watch.observation(self.process.pid, allow_terminal=True)
                check(before['ppid'] == os.getpid(), 'parent-child-relationship')
                after = watch.observation(self.process.pid, allow_terminal=True)
                check(after['stamp'] == before['stamp'], 'parent-child-stamp-changed')
                self.identity = dict(after, host=host_tuple(), executable_from='qualified-spawn')
                if after['state'] not in ('Z', 'X'):
                    check(fixture.digest(Path('/proc',str(self.process.pid),'exe').read_bytes()) == fixture.digest(Path(self.argv[0]).read_bytes()), 'parent-running-image-mismatch')
                    self.identity['executable_from'] = 'kernel-and-qualified-spawn'
                self.publish('identity.json', self.identity)
        except BaseException as error:
            self.errors.append(error)
        if self.process is not None:
            try:
                self.wait_kernel()
            except BaseException as error:
                self.errors.append(error)
        if self.process is None or self.reaped:
            self.finish_capture()
        try:
            self.publish('wait.json' if self.reaped else 'wait-pending.json', self.summary())
        except BaseException as error:
            self.errors.append(error)
        if self.errors or not self.reaped or not self.wait_validated or not self.finalized or self.status != 0:
            raise Failure('parent-owned-command-failed', self.errors, self, self.safe_summary())
        return self

    def raw(self, name):
        """Read retained bytes; missing/truncated/undecodable evidence is not success."""
        return self.captured[name] if name in self.captured else (self.output/name).read_bytes()


def owners(error):
    """Find all explicitly carried owners, not incidental traceback references."""
    found, pending, seen = [], [error], set()
    while pending:
        e = pending.pop()
        if id(e) in seen:
            continue
        seen.add(id(e))
        if isinstance(e, Failure):
            if e.owner is not None and e.owner not in found:
                found.append(e.owner)
            pending.extend(e.causes)
        elif isinstance(e, BaseExceptionGroup):
            pending.extend(e.exceptions)
        for cause in (e.__cause__,e.__context__):
            if cause is not None:pending.append(cause)
    return found


def report_terminal(error, output, observe=None):
    """Consume owner handback, then emit safe failure without losing originals.

    Settlement is explicit, not a new product request. If any owned effect
    remains unresolved (wait validation, reap or closure), fail-stop with actual carrier/handles retained and no automatic
    retry/kill; do not return an exit status and destroy ownership. Only an
    external operator can arbitrate that irrecoverable state. No supported
    in-process arbitration channel is supplied here; absent changed facts the
    terminal stays held. Returning1 requires all held owners resolved. Hardkill/parent
    death and lost terminal IO cannot be repaired by this protocol.
    """
    combined = Failure('parent-campaign-failed',[error])
    received=set()
    def receive(current):
        """Receive owners once, preserving consumed receipts and pending closures.

        Neither an ambiguous close nor rejected consumption is retried here;
        their original typed errors remain attached for external arbitration.
        """
        extras=[]
        for owner in owners(current):
            if id(owner) in received:
                continue
            received.add(id(owner))
            count=len(owner.errors)
            try:
                if owner.consumed_wait is not None and not owner.wait_validated:
                    extras.append(Failure('parent-consumed-wait-needs-arbitration',owner=owner))
                elif owner.process is not None and not owner.reaped:
                    owner.settle()
                if (owner.process is None or owner.reaped) and not owner.capture_attempted:
                    owner.finish_capture()
                if owner.capture_attempted and not owner.finalized:
                    extras.append(Failure('parent-capture-closure-pending',owner=owner))
            except BaseException as cause:
                extras.append(Failure('parent-settlement-still-unknown',[cause],owner))
            finally:
                extras.extend(owner.errors[count:])
        return extras
    combined=Failure('parent-received-ownership',[combined,*receive(combined)])
    if observe is not None and all(not o.pending() for o in owners(combined)):
        try:combined=Failure('parent-failure-observations',[combined,observe()])
        except BaseException as cause:combined=Failure('parent-failure-observation-failed',[combined,cause])
        combined=Failure('parent-received-observer-ownership',[combined,*receive(combined)])
    held=owners(combined)
    data = safe(combined)
    try:
        if output is not None:
            fixture.publish(Path(output)/'failure.json', data)
    except BaseException as cause:
        combined = Failure('parent-failure-publication-failed', [combined, cause])
        data = safe(combined)
    try:
        sys.stderr.write(json.dumps(data, sort_keys=True)+'\n')
        sys.stderr.flush()
    except BaseException as cause:
        combined = Failure('parent-terminal-output-failed', [combined, cause])
    # This live terminal frame is the explicit recipient. A failure.json or
    # exit1 is not a resource handoff. Keep both graph and actual owners alive
    # for every unresolved effect, including known reap plus unvalidated wait
    # or ambiguous closure. Rechecking facts never retries a kernel operation.
    while any(o.pending() for o in held):
        try:
            signal.pause()
        except BaseException as cause:
            combined = Failure('parent-held-ownership-interrupted', [combined, cause])
    return 1
