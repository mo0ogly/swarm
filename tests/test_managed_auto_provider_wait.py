#!/usr/bin/env python3
"""Targeted regressions for the managed-auto provider terminal wait.

Every case drives tests/rust_managed_auto_worker.await_provider_turn against
clearly simulated fixture stand-ins (a fake owned CLI call facade, a real
inotify watch on a temporary owned directory under <repo>/.swarm-fixtures —
never /tmp — and real AF_UNIX sockets): no product binary, no real mission
database and no acceptance, payment or confinement claim is manufactured.
The regressions pin the causal contract only: subscription before the first
public read and descriptor closure even on error, re-read after real activity,
BUSY as refusal with reason never an empty success, terminal-failure priority
with the original public cause kept in the typed refusal's owned payload
(minimal secret-safe rendered message), previous-turn completed agents
ignored, conductor-exit refusal, terminal priority over a simultaneously
ready connection (unconsumed backlog), and conservation of a refusing watch
close without leaking the accepted socket, and the canonical-attach
regressions for the real 2026-11-05 queued false refusal: queued never
terminal, running attachment only after real activity through an EXPLICITLY
simulated kernel seam holding two real owned descriptor stand-ins (no signal
right, no real /proc fact claimed), queued→failed cause preservation, and a
refusing watch close after attachment releasing both taken descriptors as
grouped secondary causes without a lost-error success. Codex executes this file
personally after sources are stable; GLM runs nothing. Real evidence
2026-11-05: `python3 -m unittest tests.test_managed_auto_provider_wait -v`
fails with ModuleNotFoundError (0 cases executed) — the working convention is
the direct run `python3 tests/test_managed_auto_provider_wait.py -v`, which
puts tests/ on sys.path; no global import change.
"""
import errno
import os
from pathlib import Path
import select
import shutil
import socket
import tempfile
import threading
import time
import traceback
import unittest

import rust_managed_auto_worker as scenario


class FakeOwned:
    """Fixture stand-in for the planning owned facade (owned.call + root only).

    Replays scripted (code, value) replies in order and records each argv plus a
    monotonic timestamp, so tests can assert the exact public read sequence and
    whether the inotify subscription preceded the first read. No real CLI, no
    database, no network.
    """

    def __init__(self, root, replies):
        self.root = root
        self.replies = list(replies)
        self.calls = []
        self.call_times = []

    def call(self, argv, body=None):
        self.calls.append(list(argv))
        self.call_times.append(time.monotonic())
        return self.replies.pop(0)


def wait_messages(error):
    """Join the primary and secondary causes of a wait refusal into one string."""
    subs = getattr(error, 'exceptions', None) or [error]
    return ' | '.join(str(sub) for sub in subs)


class ProviderWaitRegressions(unittest.TestCase):
    """Shared plumbing: owned inotify directory, real provider listener pair."""

    def setUp(self):
        base = os.environ.get('SWARM_TEST_FIXTURE_AREA')
        if not base:
            base = str(Path(__file__).resolve().parent.parent / '.swarm-fixtures')
        Path(base).mkdir(mode=0o700, exist_ok=True)
        self.area = Path(tempfile.mkdtemp(prefix='managed-auto-wait-', dir=base))
        (self.area / '.swarm').mkdir(mode=0o700)
        self.listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.listener.bind(str(self.area / 'provider.sock'))
        os.chmod(self.area / 'provider.sock', 0o600)
        self.listener.listen(1)
        # Real connected pair standing for the native conductor socket: writes
        # on self.remote make self.conductor (passed as server_fd) readable.
        self.conductor, self.remote = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        self.stoppers = []

    def tearDown(self):
        for stop in self.stoppers:
            stop.set()
        self.listener.close()
        self.conductor.close()
        self.remote.close()
        shutil.rmtree(self.area, ignore_errors=True)

    def touch(self):
        """Produce real database-directory activity for the inotify watch."""
        (self.area / '.swarm' / 'regressions.jsonl').write_bytes(b'activity\n')

    def poke_until(self, condition):
        """Background real activity until the condition holds (test stimulus).

        Writes into the watched directory repeatedly until the observed public
        call sequence advances; this is the test's event production, not a
        temporal policy of the code under test.
        """
        stop = threading.Event()

        def loop():
            while not stop.is_set() and not condition():
                self.touch()
                stop.wait(0.001)

        thread = threading.Thread(target=loop, daemon=True)
        thread.start()
        self.stoppers.append(stop)
        return thread

    def terminated_agent(self, **overrides):
        """One simulated terminal public agent payload (fixture-only data)."""
        agent = dict(id='agent-new', task_id='task', status='failed',
                     exit_code=1,
                     native_execution_result=dict(kernel_reaped=True, output_complete=True),
                     failure=dict(code='provider-spawn', causes=['PermissionError: EPERM socket(AF_UNIX)']))
        agent.update(overrides)
        return agent

    def status_reply(self, agents):
        """Public mission status-native shaped reply from entry stand-ins."""
        entries = [dict(id=a['id'], native_mission=dict(generation=a.get('generation', 0)))
                   for a in agents]
        return 0, dict(agents=entries)

    def show_reply(self, agent):
        return 0, dict(agent=agent)

    def test_terminalfailed_originalcause(self):
        """A newly launched terminal agent raises the structured original cause.

        The very first public read shows the new failed agent; agent show
        returns its full public payload (exit_code 1, kernel_reaped, EPERM on
        socket(AF_UNIX)). The wait stops promptly on that real fact, keeps
        exit code, native result and public failure in the typed refusal's
        owned payload — no bare 'server exited' summary — while the rendered
        message stays minimal, and consumes no provider connection. The peek
        witness reads the CONDUCTOR endpoint, the one that received the byte
        written on self.remote; peeking self.remote would block forever.
        """
        terminal = self.terminated_agent()
        owned = FakeOwned(self.area, [self.status_reply([terminal]), self.show_reply(terminal)])
        self.remote.sendall(b'P')
        with self.assertRaises(BaseExceptionGroup) as caught:
            scenario.await_provider_turn(owned, 'work', self.listener, self.conductor.fileno(), [])
        message = wait_messages(caught.exception)
        self.assertIn('managed-auto-provider-terminal-refusal', message)
        self.assertIn('"task_id": "task"', message)
        self.assertIn('"failure_code": "provider-spawn"', message)
        self.assertIn('"status": "failed"', message)
        self.assertEqual(owned.calls, [['mission', 'status-native', 'work'], ['agent', 'show', 'agent-new']])
        # Peek the endpoint the byte actually arrived on, with no new timeout.
        self.assertEqual(self.conductor.recv(1, socket.MSG_PEEK), b'P')
        self.assertEqual(select.select([self.listener], [], [], 0)[0], [])
        refusal = next(sub for sub in caught.exception.exceptions
                       if isinstance(sub, scenario.ManagedAutoTerminalRefusal))
        # The original public cause is conserved exactly in the owned payload.
        self.assertEqual(refusal.payload['public_agent'], terminal)
        self.assertEqual(refusal.payload['failure']['causes'], ['PermissionError: EPERM socket(AF_UNIX)'])
        self.assertEqual(refusal.payload['native_execution_result'], dict(kernel_reaped=True, output_complete=True))

    def test_listenerready_sain(self):
        """A readable provider listener still yields the accepted connection.

        No agent exists at all (empty status-native); a client connects to the
        real bound listener before the wait starts, and real directory activity
        is produced right after the subscription (inside a recording wrapper on
        the real watch_directory), so the watch event and the pending
        connection turn ready in the same select: the guard re-observes
        publicly before accepting, finds no termination and must NOT consume
        the connection — the wait accepts it and returns it after the
        controlled watch closure. Witness adapted to the reads actually
        required: exactly two status-native reads (up-front plus guard
        re-observation), no second connection pending, and the genuinely
        watched descriptor closed by the helper itself (fstat then fails).
        """
        owned = FakeOwned(self.area, [self.status_reply([]), self.status_reply([])])
        module = scenario
        real_watch = module.worker.watch_directory
        captured = dict(fd=None)

        def activity_watch(path, modifications=False):
            captured['fd'] = real_watch(path, modifications=modifications)
            # Real activity AFTER the subscription, BEFORE the first public
            # read: deterministic simultaneous readiness with the connection.
            self.touch()
            return captured['fd']

        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        accepted = None
        try:
            module.worker.watch_directory = activity_watch
            client.connect(self.listener.getsockname())
            accepted = module.await_provider_turn(owned, 'work', self.listener, self.conductor.fileno(), [])
            self.assertIsInstance(accepted, socket.socket)
            self.assertIsNot(accepted, self.listener)
            self.assertEqual(select.select([self.listener], [], [], 0)[0], [])
            self.assertEqual(owned.calls, [['mission', 'status-native', 'work'], ['mission', 'status-native', 'work']])
            self.assertIsNotNone(captured['fd'])
            with self.assertRaises(OSError):
                os.fstat(captured['fd'])
        finally:
            module.worker.watch_directory = real_watch
            if accepted is not None:
                accepted.close()
            client.close()

    def test_serveurtermine(self):
        """A readable conductor socket refuses immediately, connection or not.

        The conductor side becoming readable means the native server exited
        before the provider connected; the wait refuses with the causal
        message and closes the watch descriptor, without accepting anything.
        """
        owned = FakeOwned(self.area, [self.status_reply([])])
        self.remote.sendall(b'X')
        with self.assertRaises(BaseExceptionGroup) as caught:
            scenario.await_provider_turn(owned, 'work', self.listener, self.conductor.fileno(), [])
        self.assertIn('server exited before provider', wait_messages(caught.exception))
        self.assertEqual(owned.calls, [['mission', 'status-native', 'work']])

    def test_anciensterminaux_ignores_nouveau_terminal_refuse(self):
        """Previous-turn completed agents never refuse; a new terminal one does.

        The first public read returns only the two previous-turn agents
        (generations 0 and 1) — ignored by both id and generation identity —
        and the wait keeps observing. After real DB activity the second read
        exposes the new generation-2 terminal agent, whose original public
        cause is raised. Previous-turn identity alone never fabricates a
        refusal and never fabricates success either.
        """
        previous = [dict(id='agent-old-0', generation=0), dict(id='agent-old-1', generation=1)]
        new_terminal = self.terminated_agent(id='agent-new-2', generation=2)
        owned = FakeOwned(self.area, [
            self.status_reply(previous),
            self.status_reply(previous + [new_terminal]),
            self.show_reply(new_terminal),
        ])
        previous_agents = [dict(id=a['id'], native_mission=dict(generation=a['generation'])) for a in previous]
        thread = self.poke_until(lambda: len(owned.calls) >= 2)
        try:
            with self.assertRaises(BaseExceptionGroup) as caught:
                scenario.await_provider_turn(owned, 'work', self.listener, self.conductor.fileno(), previous_agents)
        finally:
            thread.join()
        message = wait_messages(caught.exception)
        self.assertIn('managed-auto-provider-terminal-refusal', message)
        self.assertIn('agent-new-2', message)
        refusal = next(sub for sub in caught.exception.exceptions
                       if isinstance(sub, scenario.ManagedAutoTerminalRefusal))
        self.assertEqual(refusal.payload['public_agent'], new_terminal)
        self.assertEqual(refusal.payload['failure']['causes'], ['PermissionError: EPERM socket(AF_UNIX)'])
        self.assertEqual([call[1] for call in owned.calls], ['status-native', 'status-native', 'show'])

    def test_terminalprioritaire_connexionsimultanee_nonconsommee(self):
        """Termination keeps priority over a simultaneously ready connection.

        A client connects (and sends one witness byte) BEFORE the wait, and real
        directory activity is produced right after the subscription (inside a
        wrapper on the real watch_directory), so watch activity and the ready
        connection arrive in the same select: the guard re-observes publicly
        BEFORE accepting — that re-observation shows the new terminal agent, the
        wait refuses with the original cause and the pending connection stays
        unconsumed in the backlog: select with a zero timeout still reports the
        listener readable and the byte is peeked intact on the accepted pending
        peer. No fabricated success and no fabricated refusal of the connection
        itself.
        """
        previous = [dict(id='agent-old-0', generation=0)]
        terminal = self.terminated_agent(id='agent-new-1', generation=1)
        owned = FakeOwned(self.area, [
            self.status_reply(previous),
            self.status_reply(previous + [terminal]),
            self.show_reply(terminal),
        ])
        previous_agents = [dict(id=a['id'], native_mission=dict(generation=a['generation'])) for a in previous]
        module = scenario
        real_watch = module.worker.watch_directory

        def activity_watch(path, modifications=False):
            fd = real_watch(path, modifications=modifications)
            # Real activity AFTER the subscription, BEFORE the first public
            # read: deterministic simultaneous readiness with the connection.
            self.touch()
            return fd

        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            module.worker.watch_directory = activity_watch
            client.connect(self.listener.getsockname())
            # One real byte on the connection: the no-leak witness can then
            # prove the pending peer still holds this exact byte unconsumed.
            client.sendall(b'C')
            with self.assertRaises(BaseExceptionGroup) as caught:
                module.await_provider_turn(owned, 'work', self.listener, self.conductor.fileno(), previous_agents)
            message = wait_messages(caught.exception)
            self.assertIn('managed-auto-provider-terminal-refusal', message)
            self.assertIn('agent-new-1', message)
            refusal = next(sub for sub in caught.exception.exceptions
                           if isinstance(sub, scenario.ManagedAutoTerminalRefusal))
            self.assertEqual(refusal.payload['public_agent'], terminal)
            self.assertEqual([call[1] for call in owned.calls], ['status-native', 'status-native', 'show'])
            # The ready connection was never consumed: the backlog still holds it.
            self.assertEqual(select.select([self.listener], [], [], 0)[0], [self.listener])
            pending_peer, _ = self.listener.accept()
            try:
                self.assertEqual(select.select([pending_peer], [], [], 0)[0], [pending_peer])
                self.assertEqual(pending_peer.recv(1, socket.MSG_PEEK), b'C')
            finally:
                pending_peer.close()
        finally:
            module.worker.watch_directory = real_watch
            client.close()

    def test_fermeture_refusee_socketrendue_perdue(self):
        """A refusing watch close is conserved, the accepted socket is not leaked.

        Witness with valid preconditions first: the real watch descriptor is
        captured through a recording wrapper and proven live (fstat succeeds
        while the watch is open, fstat fails EBADF after the healthy flow
        closed it), proving a genuinely watched descriptor existed before the
        guarded close. The injection then replaces _close_watch_fd with an
        OSError(errno.EBADF) refusal for that descriptor only; on the healthy
        close path the helper must still return the accepted connection with
        no group; with the injected refusal the same flow must refuse: the
        group carries the close refusal, no socket is returned (the helper
        closes the accepted one, so no descriptor leak), and the listener
        backlog is empty — the accepted connection was consumed then closed,
        never returned as a lost-error success. This is the regression for the
        return-inside-try path that silently dropped a close error appended in
        finally. The injected refusal leaves the real watch descriptor open;
        this test itself closes it during cleanup after restoring the seam.
        """
        owned = FakeOwned(self.area, [self.status_reply([]), self.status_reply([])])
        module = scenario
        real_watch = module.worker.watch_directory
        real_close = module._close_watch_fd
        captured = dict(fd=None)

        def recording_watch(path, modifications=False):
            captured['fd'] = real_watch(path, modifications=modifications)
            return captured['fd']

        def refusing_close(fd):
            if fd == captured['fd']:
                raise OSError(errno.EBADF, 'injected close refusal (guard)')
            return real_close(fd)

        # Healthy-close witness: connection accepted, returned, watch closed.
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            module.worker.watch_directory = recording_watch
            client.connect(self.listener.getsockname())
            accepted = module.await_provider_turn(owned, 'work', self.listener, self.conductor.fileno(), [])
            self.assertIsInstance(accepted, socket.socket)
            accepted.close()
            self.assertIsNotNone(captured['fd'])
            with self.assertRaises(OSError) as badfd:
                os.fstat(captured['fd'])
            self.assertEqual(badfd.exception.errno, errno.EBADF)
            # Same healthy flow with the injected refusing close: the wait must
            # refuse, never return a lost-error success.
            client2 = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            try:
                module._close_watch_fd = refusing_close
                client2.connect(self.listener.getsockname())
                owned.calls.clear()
                with self.assertRaises(BaseExceptionGroup) as caught:
                    module.await_provider_turn(owned, 'work', self.listener, self.conductor.fileno(), [])
            finally:
                client2.close()
            group_message = wait_messages(caught.exception)
            self.assertIn('injected close refusal (guard)', group_message)
            close_error = next(sub for sub in caught.exception.exceptions if isinstance(sub, OSError))
            self.assertEqual(close_error.errno, errno.EBADF)
            self.assertIn('injected close refusal (guard)', str(close_error))
            # No accepted socket returned by the refusing flow: the second
            # connection was consumed then closed, the backlog is empty.
            self.assertEqual(select.select([self.listener], [], [], 0)[0], [])
        finally:
            module.worker.watch_directory = real_watch
            module._close_watch_fd = real_close
            # The refused close left the genuinely watched descriptor open;
            # only this test's cleanup may release it.
            if captured['fd'] is not None:
                try:
                    os.close(captured['fd'])
                except OSError:
                    pass
            client.close()

    # --- Canonical-attach regressions (real 2026-11-05 queued false refusal).

    def running_agent(self, **overrides):
        """One simulated running public agent payload with usable kernel facts."""
        agent = dict(id='agent-attach', task_id='task', status='running',
                     supervisor_pid=41001, child_pid=41002,
                     supervisor_identity='1001', child_identity='2002',
                     exit_code=None)
        agent.update(overrides)
        return agent

    def queued_agent(self, **overrides):
        """One simulated queued public agent: PIDs 0, empty stamps, null exit.

        Mirrors the exact Queued state the product validates in
        agent_execution_preflight.rs (status queued, both PIDs 0, both
        identities empty, native results None, exit_code null) — a legitimate
        protocol state, never terminal.
        """
        agent = dict(id='agent-queued', task_id='task', status='queued',
                     supervisor_pid=0, child_pid=0,
                     supervisor_identity='', child_identity='',
                     exit_code=None,
                     native_execution_result=None)
        agent.update(overrides)
        return agent

    def test_canonical_queued_jamais_terminal_puis_attache_activite_reelle(self):
        """A queued public agent never refuses; attachment waits for real activity.

        Reproduces the real 2026-11-05 first refusal: the new agent's first
        public payload is exactly the queued state (PIDs 0, empty stamps,
        exit_code null) that the old code classified terminal — this flow must
        keep observing instead. After real watched-directory activity the same
        new agent re-reads as running with usable PIDs and stamps and the
        canonical attach completes. The kernel attachment is EXPLICITLY
        simulated: _attach_agent_kernel is replaced by a fixture seam that
        performs no /proc observation and takes no real pidfd of a fake PID —
        it returns two real owned descriptor stand-ins (an eventfd-style pair
        of real open descriptors created here) that this test closes. No signal
        right, no false enrollment and no real kernel fact is claimed; the
        causality proven is the public lifecycle contract only: queued never
        terminal, attachment only after the agent re-reads running following
        real activity, BUSY pending state cleared by the fresh valid read.
        """
        module = scenario
        queued = self.queued_agent()
        running = self.running_agent(id='agent-queued', generation=0)
        # First canonical round: queued visible (old code refused here). Then a
        # structured BUSY on the re-read, then the running payload: the fresh
        # valid read clears the pending BUSY instead of keeping it.
        busy = (7, dict(failure=dict(code='root-database-identity', causes=[
            'root-database-identity', 'database-identity-operation:Prepare', 'sqlite-code:5'])))
        owned = FakeOwned(self.area, [
            self.status_reply([queued]),
            self.show_reply(queued),
            busy,
            self.status_reply([running]),
            self.show_reply(running),
        ])
        real_kernel = module._attach_agent_kernel
        taken = dict(descriptors=[])
        simulated_calls = []

        def simulated_kernel(agent, binary, root, server_pid):
            """EXPLICIT simulation: no /proc fact, two real owned stand-ins."""
            simulated_calls.append(agent)
            pair = []
            try:
                for _ in range(2):
                    pair.append(os.open('/dev/null', os.O_RDONLY | os.O_CLOEXEC))
            except BaseException:
                for fd in pair:
                    os.close(fd)
                raise
            taken['descriptors'] = pair
            return agent, pair[0], agent['child_pid'], pair[1]

        module._attach_agent_kernel = simulated_kernel
        thread = self.poke_until(lambda: len(owned.calls) >= 4)
        try:
            result = module._attach_canonical_agent(
                owned, 'work', [], 99999, self.conductor.fileno(), 'fixture-binary', str(self.area))
            # Both real owned stand-in descriptors were returned by the helper
            # and are still open — the caller owns them, this test closes them
            # in cleanup below, never a leak on the healthy path.
            self.assertEqual(len(taken['descriptors']), 2)
            for fd in taken['descriptors']:
                os.fstat(fd)
        finally:
            module._attach_agent_kernel = real_kernel
            thread.join()
            for fd in taken['descriptors']:
                try:
                    os.close(fd)
                except OSError:
                    pass
        # The queued payload kept observing (no refusal, no fabricated success)
        # and attachment happened exactly once, on the running re-read only.
        self.assertEqual([call[1] for call in owned.calls],
                         ['status-native', 'show', 'status-native', 'status-native', 'show'])
        self.assertEqual(simulated_calls, [running])
        self.assertEqual(result[0], running)

    def test_canonical_queued_vers_failed_conserve_cause_sans_faux_refus(self):
        """A queued-then-failed agent refuses with the real cause, never before.

        First public payload is the same legitimate queued state — the old code
        refused here before the agent had any real terminal fact. The flow keeps
        observing; after real activity the same new agent re-reads as failed and
        the typed refusal carries the original public cause exactly. No
        attachment seam is needed: the refusal happens on the public payload.
        """
        queued = self.queued_agent()
        failed = self.terminated_agent(id='agent-queued', generation=0)
        owned = FakeOwned(self.area, [
            self.status_reply([queued]),
            self.show_reply(queued),
            self.status_reply([failed]),
            self.show_reply(failed),
        ])
        thread = self.poke_until(lambda: len(owned.calls) >= 3)
        try:
            with self.assertRaises(BaseExceptionGroup) as caught:
                scenario._attach_canonical_agent(
                    owned, 'work', [], 99999, self.conductor.fileno(), 'fixture-binary', str(self.area))
        finally:
            thread.join()
        message = wait_messages(caught.exception)
        self.assertIn('managed-auto-provider-terminal-refusal', message)
        self.assertIn('"status": "failed"', message)
        refusal = next(sub for sub in caught.exception.exceptions
                       if isinstance(sub, scenario.ManagedAutoTerminalRefusal))
        self.assertEqual(refusal.payload['public_agent'], failed)
        self.assertEqual([call[1] for call in owned.calls],
                         ['status-native', 'show', 'status-native', 'show'])
        self.assertEqual(select.select([self.listener], [], [], 0)[0], [])

    def test_canonical_fermeture_watch_refusee_pidfds_rendus_sans_succes(self):
        """A refusing watch close after attachment releases both pidfds, no success.

        Valid witness first: with the production kernel seam (a simulated
        attach that returns two REAL owned descriptors, explicitly no /proc
        fact and no signal right) and a healthy close, the canonical attach
        returns the 5-tuple and the watch descriptor is closed by the helper
        (fstat EBADF). The injection then refuses only the watch close AFTER
        that same successful attachment: the flow must refuse — the group
        carries the close refusal, both taken descriptors were explicitly
        released by the helper (asserted BEFORE any test cleanup: each now
        fails fstat with EBADF — had the release not run they would still be
        open), and no result is returned: a lost-error success is structurally
        impossible. A bounded non-vacuity seam phase then reruns the same
        refusal flow on its own scripted facade: the RERUN kernel itself
        declares the LIVE suppression scope — exactly the two descriptor
        numbers it has just returned, established during this rerun and still
        open — never a stale number an earlier phase released (the rerun's
        watch opens first and could reuse one), and os.close is replaced so
        closes of exactly that scope become no-ops. The ownership assertion
        must then go RED (AssertionError raised inside this guard, an
        expected red phase of the regression, never a false acceptance),
        proving the guard fails when the helper keeps them, while the seam
        never masks the watch close refusal (it stays the group's single
        cause). Cleanup in the finally runs even when an assertion failed,
        restores every seam and closes ONLY descriptors the fixture still
        owns, each verified by its (st_dev, st_ino) identity recorded at open
        time — never a numerically reused descriptor — and never masks the
        primary refusal asserted above.
        """
        module = scenario
        running = self.running_agent()
        owned = FakeOwned(self.area, [
            self.status_reply([running]), self.show_reply(running),
            self.status_reply([running]), self.show_reply(running),
        ])
        real_kernel = module._attach_agent_kernel
        real_close = module._close_watch_fd
        real_watch = module.worker.watch_directory
        real_os_close = os.close
        # Every mutable state used by the finally is initialized BEFORE the
        # protected try: a failure in any early phase must never raise
        # UnboundLocalError from the cleanup, which must always restore every
        # seam and close only still-owned resources.
        taken = dict(descriptors=[], identities=[])
        captured = dict(fd=None)
        watch_taken = []
        suppressed = set()
        rerun = dict(descriptors=[], identities=[])
        seam_installed = dict(value=False)

        def simulated_kernel(agent, binary, root, server_pid):
            """EXPLICIT simulation: no /proc fact, no signal right, no real
            PID claim from these descriptors — two real owned stand-ins,
            recorded with their (st_dev, st_ino) identity at open time."""
            pair = []
            try:
                for _ in range(2):
                    pair.append(os.open('/dev/null', os.O_RDONLY | os.O_CLOEXEC))
            except BaseException:
                for fd in pair:
                    os.close(fd)
                raise
            taken['descriptors'] = pair
            taken['identities'] = [(os.fstat(fd).st_dev, os.fstat(fd).st_ino)
                                   for fd in pair]
            return agent, pair[0], agent['child_pid'], pair[1]

        def recording_watch(path, modifications=False):
            fd = real_watch(path, modifications=modifications)
            info = os.fstat(fd)
            # Every watch this test opens is tracked with its identity: a
            # refused close leaves it open and the next phase opens another
            # watch, so cleanup must release EACH still-open one by identity.
            watch_taken.append((fd, (info.st_dev, info.st_ino)))
            captured['fd'] = fd
            return fd

        def refusing_close(fd):
            if fd == captured['fd']:
                raise OSError(errno.EBADF, 'injected watch close refusal (guard)')
            return real_close(fd)

        def seam_close(fd):
            """No-op the close of EXACTLY the LIVE suppression scope declared
            by rerun_kernel; every other close stays real."""
            if fd in suppressed:
                return None
            return real_os_close(fd)

        def rerun_kernel(agent, binary, root, server_pid):
            """EXPLICIT simulation for the seam phase; declares the LIVE
            suppression scope as EXACTLY the two descriptor numbers it has
            just returned, established during this rerun and verified still
            open — never a stale number an earlier phase released (the
            rerun's watch opens first and could reuse one)."""
            pair = []
            try:
                for _ in range(2):
                    pair.append(os.open('/dev/null', os.O_RDONLY | os.O_CLOEXEC))
            except BaseException:
                for fd in pair:
                    real_os_close(fd)
                    suppressed.discard(fd)
                raise
            for fd in pair:
                os.fstat(fd)
            rerun['descriptors'] = pair
            rerun['identities'] = [(os.fstat(fd).st_dev, os.fstat(fd).st_ino)
                                   for fd in pair]
            suppressed.clear()
            suppressed.update(pair)
            return agent, pair[0], agent['child_pid'], pair[1]

        module._attach_agent_kernel = simulated_kernel
        try:
            # Healthy witness: attachment returns, watch closed, no group.
            module.worker.watch_directory = recording_watch
            result = module._attach_canonical_agent(
                owned, 'work', [], 99999, self.conductor.fileno(), 'fixture-binary', str(self.area))
            module.worker.watch_directory = real_watch
            self.assertEqual(len(result), 5)
            self.assertIsNotNone(captured['fd'])
            with self.assertRaises(OSError) as badfd:
                os.fstat(captured['fd'])
            self.assertEqual(badfd.exception.errno, errno.EBADF)
            for fd in taken['descriptors']:
                os.close(fd)
            taken['descriptors'] = []
            taken['identities'] = []
            owned.calls.clear()
            # Injected refusal after attachment: refuse, release, no success.
            module.worker.watch_directory = recording_watch
            module._close_watch_fd = refusing_close
            with self.assertRaises(BaseExceptionGroup) as caught:
                module._attach_canonical_agent(
                    owned, 'work', [], 99999, self.conductor.fileno(), 'fixture-binary', str(self.area))
            group_message = wait_messages(caught.exception)
            self.assertIn('injected watch close refusal (guard)', group_message)
            close_error = next(sub for sub in caught.exception.exceptions
                               if isinstance(sub, OSError))
            self.assertEqual(close_error.errno, errno.EBADF)
            # Ownership property asserted BEFORE any test cleanup (the guard
            # fixed on 2026-11-05: the former version closed the descriptors
            # in the finally first, making the fstat check below vacuously
            # pass even if the helper had released nothing): every descriptor
            # the helper RETURNED must already be closed BY THE HELPER on its
            # refusal path.
            self.assertEqual(len(taken['descriptors']), 2)
            for fd in taken['descriptors']:
                with self.assertRaises(OSError):
                    os.fstat(fd)
            # Non-vacuity seam phase on its OWN scripted facade: rerun the
            # same refusal flow with os.close replaced so that closes of
            # EXACTLY the LIVE scope declared by rerun_kernel (the two numbers
            # it has just returned, both still open) become no-ops — never a
            # stale number an earlier phase released, which the rerun's watch
            # could reuse. The watch close stays refused by refusing_close:
            # the seam never masks it.
            owned = FakeOwned(self.area, [
                self.status_reply([running]), self.show_reply(running),
            ])
            module.worker.watch_directory = recording_watch
            module._attach_agent_kernel = rerun_kernel
            os.close = seam_close
            seam_installed['value'] = True
            try:
                with self.assertRaises(BaseExceptionGroup) as rerun_caught:
                    module._attach_canonical_agent(
                        owned, 'work', [], 99999, self.conductor.fileno(), 'fixture-binary', str(self.area))
            finally:
                os.close = real_os_close
                seam_installed['value'] = False
            # The seam masks nothing: the rerun group carries ONLY the watch
            # close refusal (suppressed pidfd closes fabricate no error).
            self.assertEqual(len(rerun_caught.exception.exceptions), 1)
            rerun_message = wait_messages(rerun_caught.exception)
            self.assertIn('injected watch close refusal (guard)', rerun_message)
            # A REFUSED close leaves the watch OPEN (the refusal raises before
            # any real close): fstat must still succeed here, and the outer
            # cleanup releases this watch by identity.
            os.fstat(captured['fd'])
            # THE RED PHASE: with the pidfd closes no-opped by the seam, the
            # SAME ownership assertion must now FAIL — the helper kept them
            # open — proving the guard goes red when closure does not happen
            # (never a vacuous pass). Identical assertion shape as above.
            with self.assertRaises(AssertionError):
                for fd in rerun['descriptors']:
                    with self.assertRaises(OSError):
                        os.fstat(fd)
            # Immediate repair of the suppressed closes so no real descriptor
            # leaks, by (st_dev, st_ino) identity — never a reused number.
            for fd, identity in zip(list(rerun['descriptors']),
                                    list(rerun['identities'])):
                try:
                    info = os.fstat(fd)
                except OSError:
                    continue
                if (info.st_dev, info.st_ino) == identity:
                    real_os_close(fd)
                    suppressed.discard(fd)
            rerun['descriptors'] = []
            rerun['identities'] = []
        finally:
            # Restore EVERY seam, even when an assertion failed above.
            if seam_installed['value']:
                os.close = real_os_close
                seam_installed['value'] = False
            module._attach_agent_kernel = real_kernel
            module._close_watch_fd = real_close
            module.worker.watch_directory = real_watch
            # Close ONLY descriptors this fixture still owns: fstat each
            # tracked descriptor and require its (st_dev, st_ino) to still
            # match what was recorded at open time, so a numerically reused
            # descriptor is never closed and another's is never touched.
            # Cleanup never masks the primary refusal asserted above.
            for fd, identity in list(zip(taken['descriptors'],
                                         taken['identities'])):
                try:
                    info = os.fstat(fd)
                except OSError:
                    continue
                if (info.st_dev, info.st_ino) == identity:
                    real_os_close(fd)
            taken['descriptors'] = []
            taken['identities'] = []
            for fd, identity in list(zip(rerun['descriptors'],
                                         rerun['identities'])):
                try:
                    info = os.fstat(fd)
                except OSError:
                    continue
                if (info.st_dev, info.st_ino) == identity:
                    real_os_close(fd)
            rerun['descriptors'] = []
            rerun['identities'] = []
            for fd, identity in list(watch_taken):
                try:
                    info = os.fstat(fd)
                except OSError:
                    continue
                if (info.st_dev, info.st_ino) == identity:
                    real_os_close(fd)
            watch_taken.clear()

    def test_message_surs_secret_nonce(self):
        """A nonce secret in the public agent never reaches the rendered message.

        The simulated public payload carries a distinct nonce in prompt,
        context and native capture. The rendered refusal group message, repr
        and a rendered traceback must not contain any nonce, while the typed
        refusal's owned payload attribute keeps the exact original values and
        the real EPERM cause stays conserved in the payload — nothing
        fabricated, no confinement conclusion drawn, and the minimal message
        still carries the identity/status/exit/failure-code facts.
        """
        nonce_prompt = 'nonce-secret-prompt-9f2a1c'
        nonce_context = 'nonce-secret-context-4d7e08'
        nonce_capture = 'nonce-secret-capture-b31a55'
        terminal = self.terminated_agent(
            prompt=nonce_prompt,
            context=dict(note=nonce_context),
            native_execution_result=dict(kernel_reaped=True, output_complete=True,
                                         capture='provider crashed: '+nonce_capture),
        )
        owned = FakeOwned(self.area, [self.status_reply([terminal]), self.show_reply(terminal)])
        with self.assertRaises(BaseExceptionGroup) as caught:
            scenario.await_provider_turn(owned, 'work', self.listener, self.conductor.fileno(), [])
        message = wait_messages(caught.exception)
        for nonce in (nonce_prompt, nonce_context, nonce_capture):
            self.assertNotIn(nonce, message)
            self.assertNotIn(nonce, repr(caught.exception))
            self.assertNotIn(nonce, ''.join(traceback.format_exception(type(caught.exception), caught.exception, caught.exception.__traceback__)))
        self.assertIn('managed-auto-provider-terminal-refusal', message)
        self.assertIn('"status": "failed"', message)
        self.assertIn('"exit_code": 1', message)
        refusal = next(sub for sub in caught.exception.exceptions
                       if isinstance(sub, scenario.ManagedAutoTerminalRefusal))
        self.assertIn(nonce_prompt, refusal.payload['public_agent']['prompt'])
        self.assertEqual(refusal.payload['public_agent']['context'], dict(note=nonce_context))
        self.assertEqual(refusal.payload['native_execution_result']['capture'], 'provider crashed: '+nonce_capture)
        self.assertEqual(refusal.payload['failure']['causes'], ['PermissionError: EPERM socket(AF_UNIX)'])

    def test_refus_observation_statutinconnu_nonce_rendu_sur(self):
        """An unknown-status ObservationRefusal renders no payload secret.

        The public agent show payload carries a distinct nonce in status,
        prompt, context and native capture, with a lifecycle status outside
        the product contract. The typed refusal raises through the canonical
        attach (conserving the phase cause as a grouped refusal — never an
        empty success, never a fabricated terminal classification): its
        static message, repr and a rendered traceback must not contain ANY
        nonce (the status value itself could carry one, so no payload field
        is rendered at all), while the owned ``payload`` attribute keeps the
        complete original public payload strictly equal for exact
        programmatic access. The refusal is immediate at the FIRST agent
        show, so the fixture scripts exactly one status-native/show pair
        and the guard asserts the full script was consumed — no second
        observation round is scripted and the product refusal timing is
        never altered to feed a call counter.
        """
        nonce_status = 'nonce-secret-status-77aa19'
        nonce_prompt = 'nonce-secret-prompt-2c6e41'
        nonce_context = 'nonce-secret-context-58d3a2'
        nonce_capture = 'nonce-secret-capture-e910bd'
        unknown = self.queued_agent(
            id='agent-unknown', generation=0, status=nonce_status,
            prompt=nonce_prompt, context=dict(note=nonce_context),
            native_execution_result=dict(kernel_reaped=False, output_complete=False,
                                         capture='provider crashed: '+nonce_capture),
        )
        owned = FakeOwned(self.area, [
            self.status_reply([unknown]), self.show_reply(unknown),
        ])
        module = scenario
        real_kernel = module._attach_agent_kernel
        module._attach_agent_kernel = lambda agent, binary, root, server_pid: (_ for _ in ()).throw(
            AssertionError('no attachment may happen from an unknown lifecycle status'))
        try:
            with self.assertRaises(BaseExceptionGroup) as caught:
                module._attach_canonical_agent(
                    owned, 'work', [], 99999, self.conductor.fileno(), 'fixture-binary', str(self.area))
        finally:
            module._attach_agent_kernel = real_kernel
        rendered = wait_messages(caught.exception)
        formatted = ''.join(traceback.format_exception(
            type(caught.exception), caught.exception, caught.exception.__traceback__))
        for nonce in (nonce_status, nonce_prompt, nonce_context, nonce_capture):
            self.assertNotIn(nonce, rendered)
            self.assertNotIn(nonce, repr(caught.exception))
            self.assertNotIn(nonce, formatted)
        self.assertIn('managed-auto-observation-refusal', rendered)
        refusal = next(sub for sub in caught.exception.exceptions
                       if isinstance(sub, scenario.ObservationRefusal))
        self.assertEqual(refusal.reason, 'public agent show payload with unknown lifecycle status')
        self.assertEqual(refusal.payload, unknown)
        self.assertIn(nonce_status, refusal.payload['status'])
        self.assertEqual(refusal.payload['prompt'], nonce_prompt)
        self.assertEqual(refusal.payload['context'], dict(note=nonce_context))
        self.assertEqual(refusal.payload['native_execution_result']['capture'],
                         'provider crashed: '+nonce_capture)
        # The refusal is immediate at the first agent show: exactly one
        # status-native/show round was consumed, and the whole one-pair
        # script was consumed (no further scripted reply exists to be read).
        self.assertEqual([call[1] for call in owned.calls],
                         ['status-native', 'show'])
        self.assertEqual(select.select([self.listener], [], [], 0)[0], [])

    def test_refus_observation_payloadsansstatut_payload_exact(self):
        """A payload without status refuses typed, exact payload, no empty success.

        The agent show reply carries an agent without a status key at all
        (malformed shape). The observation raises the typed
        :class:`ObservationRefusal` with the static safe reason and the FULL
        original reply payload in the owned attribute — the refusal conserves
        the phase cause instead of becoming an empty success, and no
        attachment is ever attempted from it.
        """
        reply_without_status = dict(agent=dict(id='agent-shapeless', task_id='task'))
        owned = FakeOwned(self.area, [
            self.status_reply([dict(id='agent-shapeless', generation=0)]),
            (0, reply_without_status),
        ])
        with self.assertRaises(BaseExceptionGroup) as caught:
            scenario.await_provider_turn(owned, 'work', self.listener, self.conductor.fileno(), [])
        rendered = wait_messages(caught.exception)
        self.assertIn('managed-auto-observation-refusal', rendered)
        self.assertIn('public agent show payload without status', rendered)
        self.assertNotIn('agent-shapeless', rendered)
        refusal = next(sub for sub in caught.exception.exceptions
                       if isinstance(sub, scenario.ObservationRefusal))
        self.assertEqual(refusal.reason, 'public agent show payload without status')
        self.assertEqual(refusal.payload, reply_without_status)
        self.assertEqual([call[1] for call in owned.calls], ['status-native', 'show'])
        self.assertEqual(select.select([self.listener], [], [], 0)[0], [])

    def test_souscriptionavantlecture_fermeture(self):
        """Subscription precedes the first public read and closes on error.

        The real rust_mission_watch_worker.watch_directory is wrapped to record
        its creation time and descriptor; the fake facade records each public
        read time. The watch must exist strictly before the first read, and
        after the wait refuses (terminal first-read case) the recorded
        descriptor must be closed by the helper itself — fstat fails EBADF —
        proving closure even on the error path.
        """
        terminal = self.terminated_agent()
        owned = FakeOwned(self.area, [self.status_reply([terminal]), self.show_reply(terminal)])
        module = scenario.worker
        real_watch = module.watch_directory
        captured = dict(fd=None, created=None)

        def recording_watch(path, modifications=False):
            captured['fd'] = real_watch(path, modifications=modifications)
            captured['created'] = time.monotonic()
            return captured['fd']

        module.watch_directory = recording_watch
        try:
            with self.assertRaises(BaseExceptionGroup) as caught:
                scenario.await_provider_turn(owned, 'work', self.listener, self.conductor.fileno(), [])
        finally:
            module.watch_directory = real_watch
        self.assertIn('managed-auto-provider-terminal-refusal', wait_messages(caught.exception))
        self.assertIsNotNone(captured['fd'])
        self.assertLess(captured['created'], owned.call_times[0])
        with self.assertRaises(OSError):
            os.fstat(captured['fd'])

    def test_busyneprouve_pass(self):
        """Structured BUSY is a preserved refusal, never an empty success.

        The first status-native read returns the structured BUSY refusal
        (root-database-identity Prepare / sqlite-code:5). No success and no
        connection are consumed while BUSY stands: the wait records the
        pending refusal, waits for real database activity, and only a fresh
        public read after that activity can advance. The final refusal group
        keeps both the BUSY reason and the new terminal cause together.
        """
        busy = (7, dict(failure=dict(code='root-database-identity', causes=[
            'root-database-identity', 'database-identity-operation:Prepare', 'sqlite-code:5'])))
        terminal = self.terminated_agent(id='agent-busy-then-failed', generation=0)
        owned = FakeOwned(self.area, [busy, self.status_reply([terminal]), self.show_reply(terminal)])
        thread = self.poke_until(lambda: len(owned.calls) >= 2)
        try:
            with self.assertRaises(BaseExceptionGroup) as caught:
                scenario.await_provider_turn(owned, 'work', self.listener, self.conductor.fileno(), [])
        finally:
            thread.join()
        message = wait_messages(caught.exception)
        self.assertIn('sqlite-code:5', message)
        self.assertIn('managed-auto-provider-terminal-refusal', message)
        self.assertIn('agent-busy-then-failed', message)
        self.assertEqual([call[1] for call in owned.calls], ['status-native', 'status-native', 'show'])
        self.assertEqual(select.select([self.listener], [], [], 0)[0], [])


if __name__ == '__main__':
    unittest.main()
