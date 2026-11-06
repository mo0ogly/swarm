"""Targeted guards for the fixture file rendezvous protocol.

Run directly (the tests directory must be importable from this file's
directory):

    python3 tests/test_managed_file_provider.py -v

These guards exercise the real child process with a real ``Popen`` witness
(stdin to EOF, ready, probes, release, exit 0) and the real parent-side
refusals, with causal determinism: no sleep, no temporal polling, no
arbitrary watchdog. Every wait is causal on inotify events, pipe EOF or
pidfd readability. Fixtures come from ``fixture_area.fixture_root`` — a
private durable area under the repository, never /tmp — and no live mission
database is touched. Original fixture files are preserved: no guard deletes
a published file. The optional ``terminal_fds`` rendezvous parameter (added
for the managed-auto integration, where the conductor server descriptor must
be observable) carries its own causal guard 24: a real dead external
terminal ends a wait with a role-named refusal while the provider stays
alive.

Limits stated plainly: these standalone guards prove the file protocol only.
They do NOT prove the real Biba policy; the managed-auto integration run
must establish that. They are neither confinement nor acceptance evidence.
The tests themselves may kill their own fixture children (the module under
test never signals a process).
"""

import hashlib
import json
import os
import select
import subprocess
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import fixture_area  # noqa: E402
import rust_managed_file_provider as protocol  # noqa: E402


def _identity_of(process):
    """Return the already-validated identity dict for a live child process."""
    return dict(pid=process.pid, ppid=os.getpid(), sid=os.getsid(process.pid), pgid=os.getpgid(process.pid))


def _start_provider(root, nonce, turn_name, stdin_bytes=b'fixture input\n', hold_stdin=False):
    """Start one real provider child and return ``(process, rendezvous, pidfd, stdin_bytes)``.

    The child runs the module program in ``root`` with the model on argv.
    Unless ``hold_stdin`` is true the stdin bytes are written and the pipe is
    closed immediately, so the child reaches EOF and publishes its ready
    file; with ``hold_stdin`` the child stays causally blocked on its stdin
    read and cannot publish ready, which the death-before-ready guard relies
    on. The pidfd is opened by the test (caller-owned, as in the future
    integration) and passed to the rendezvous together with the identity the
    test itself validated from the real process.
    """
    program_path = root / ('provider-%s.py' % turn_name)
    program_path.write_text(protocol.program(nonce, turn_name), encoding='utf-8')
    process = subprocess.Popen(
        [sys.executable, program_path.name, '--model', 'fixture-model'],
        cwd=root,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if not hold_stdin:
        process.stdin.write(stdin_bytes)
        process.stdin.flush()
        process.stdin.close()
    pidfd = os.pidfd_open(process.pid)
    rendezvous = protocol.FileRendezvous(root, nonce, turn_name, _identity_of(process), pidfd)
    return process, rendezvous, pidfd, stdin_bytes


def _finish(process, pidfd):
    """Reap a child that got its stdin EOF: return ``(code, stdout, stderr)``."""
    out = process.stdout.read()
    err = process.stderr.read()
    process.stdout.close()
    process.stderr.close()
    code = process.wait()
    os.close(pidfd)
    return code, out, err


def _kill_reap(process, pidfd):
    """Kill this test's own fixture child, reap it, return ``(code, stderr)``."""
    try:
        process.stdin.close()
    except (BrokenPipeError, OSError, ValueError):
        pass
    process.kill()
    code = process.wait()
    err = process.stderr.read().decode()
    process.stdout.close()
    process.stderr.close()
    os.close(pidfd)
    return code, err


def _publish_bytes(root, name, data):
    """Hand-publish one workspace file the way the protocol publishes: 0600 temp, fsync, rename."""
    temp = root / ('.' + name + '.tmp')
    fd = os.open(temp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temp, root / name)
    except BaseException:
        try:
            os.unlink(temp)
        except OSError:
            pass
        raise


def _publish_json(root, name, payload):
    """Publish one JSON payload file (hand publication helper)."""
    _publish_bytes(root, name, (json.dumps(payload) + '\n').encode())


class ProgramContract(unittest.TestCase):
    """Guards on the generated provider source and its generation checks."""

    def setUp(self):
        self.root = fixture_area.fixture_root('managed-file-program-')

    def test_program_rejects_traversal_turn_name(self):
        """A traversal, separator or empty turn name is refused at generation."""
        for bad in ('../escape', 'a/b', '.', '..', '', 'with\x00nul'):
            with self.assertRaises(ValueError):
                protocol.program('nonce-a', bad)

    def test_program_rejects_empty_nonce(self):
        """An empty or non-string nonce is refused at generation."""
        for bad in ('', None, 5):
            with self.assertRaises(ValueError):
                protocol.program(bad, 'alpha')

    def test_program_source_carries_no_socket(self):
        """The generated program contains no socket import, call or family."""
        source = protocol.program('nonce-a', 'alpha')
        self.assertNotIn('socket', source)
        self.assertNotIn('AF_UNIX', source)
        self.assertNotIn('SOCK_', source)

    def test_program_source_keeps_fixture_usage(self):
        """The generated program keeps the inherited fixture usage values."""
        source = protocol.program('nonce-a', 'alpha')
        self.assertIn("'input_tokens': 7", source)
        self.assertIn("'output_tokens': 8", source)
        self.assertIn("'total_cost_usd': 1", source)
        self.assertIn('Owned inert worker', source)
        self.assertIn("model-received.json", source)


class RealChildWitness(unittest.TestCase):
    """Real Popen witness of the full child lifecycle over workspace files."""

    def setUp(self):
        self.root = fixture_area.fixture_root('managed-file-witness-')
        self.nonce = 'nonce-witness'
        self.turn = 'alpha'
        self.process = None
        self.rendezvous = None
        self.pidfd = None

    def tearDown(self):
        if self.rendezvous is not None:
            self.rendezvous.close()
        if self.process is not None and self.process.poll() is None:
            process, pidfd = self.process, self.pidfd
            self._quiet_kill(process, pidfd)
        elif self.pidfd is not None:
            os.close(self.pidfd)

    @staticmethod
    def _quiet_kill(process, pidfd):
        """Last-resort cleanup of a still-live fixture child (no assertions)."""
        try:
            process.stdin.close()
        except (BrokenPipeError, OSError, ValueError):
            pass
        process.kill()
        process.wait()
        process.stdout.close()
        process.stderr.close()
        os.close(pidfd)

    def test_full_lifecycle_ready_probe_release_exit0(self):
        """stdin EOF→ready→two probes→release→handoff→events→real exit 0."""
        self.process, self.rendezvous, self.pidfd, stdin_bytes = _start_provider(
            self.root, self.nonce, self.turn)
        ready = self.rendezvous.ready()
        self.assertEqual(ready['nonce'], self.nonce)
        self.assertEqual(ready['pid'], self.process.pid)
        self.assertEqual(ready['ppid'], os.getpid())
        self.assertEqual(ready['sid'], os.getsid(self.process.pid))
        self.assertEqual(ready['pgid'], os.getpgid(self.process.pid))

        first = self.rendezvous.probe()
        second = self.rendezvous.probe()
        self.assertEqual(first['pid'], self.process.pid)
        self.assertEqual(second['pid'], self.process.pid)
        self.assertEqual(first['nonce'], second['nonce'])

        self.rendezvous.release()
        code, out, err = _finish(self.process, self.pidfd)
        self.pidfd = None
        self.assertEqual(code, 0, 'provider exited %s stderr=%r' % (code, err))

        received = json.loads((self.root / 'model-received.json').read_bytes())
        self.assertEqual(received['stdin_sha256'], hashlib.sha256(stdin_bytes).hexdigest())
        self.assertEqual(received['stdin_bytes'], len(stdin_bytes))
        self.assertEqual(received['pid'], self.process.pid)
        self.assertEqual(received['ppid'], os.getpid())
        self.assertEqual(received['argv'], ['--model', 'fixture-model'])

        handoff = (self.root / 'handoff.md').read_bytes().decode()
        self.assertIn('Owned inert worker', handoff)
        self.assertIn(received['stdin_sha256'], handoff)

        events = [json.loads(line) for line in out.decode().splitlines() if line]
        self.assertEqual([event['type'] for event in events],
                         ['system', 'assistant', 'result'])
        self.assertEqual(events[0], {'type': 'system', 'subtype': 'init', 'model': 'fixture-model'})
        self.assertEqual(events[2]['usage'], {'input_tokens': 7, 'output_tokens': 8})
        self.assertEqual(events[2]['total_cost_usd'], 1)

        # Atomic exclusive publication: every rendezvous publication is 0600.
        for name in ('ready-%s.json' % self.turn, 'handoff.md'):
            info = os.lstat(self.root / name)
            self.assertEqual(info.st_mode & 0o777, 0o600, name)
        # Two identified probe requests, each answered exactly once, never reused.
        requests = sorted(p.name for p in self.root.iterdir()
                          if p.name.startswith('probe-') and p.name.endswith('.request.json'))
        replies = sorted(p.name for p in self.root.iterdir()
                         if p.name.endswith('.reply.json'))
        self.assertEqual(len(requests), 2)
        self.assertEqual(len(replies), 2)
        self.assertTrue(requests[0] < requests[1])

    def test_release_file_content_is_exact(self):
        """The parent-published release file holds exactly nonce+release True."""
        self.process, self.rendezvous, self.pidfd, _ = _start_provider(
            self.root, self.nonce, self.turn)
        self.rendezvous.ready()
        self.rendezvous.release()
        payload = json.loads((self.root / ('release-%s.json' % self.turn)).read_bytes())
        self.assertEqual(payload, {'nonce': self.nonce, 'release': True})
        code, _, err = _finish(self.process, self.pidfd)
        self.pidfd = None
        self.assertEqual(code, 0, err)

    def test_second_release_refused_original_preserved(self):
        """A second release() is refused; the original release file is untouched."""
        self.process, self.rendezvous, self.pidfd, _ = _start_provider(
            self.root, self.nonce, self.turn)
        self.rendezvous.ready()
        self.rendezvous.release()
        release_path = self.root / ('release-%s.json' % self.turn)
        before = release_path.read_bytes()
        with self.assertRaises(RuntimeError):
            self.rendezvous.release()
        self.assertEqual(release_path.read_bytes(), before)
        code, _, err = _finish(self.process, self.pidfd)
        self.pidfd = None
        self.assertEqual(code, 0, err)

    def test_probe_and_release_before_ready_refused(self):
        """probe() and release() before ready() are refused without publication."""
        # hold_stdin keeps the child causally blocked before its ready file.
        self.process, self.rendezvous, self.pidfd, _ = _start_provider(
            self.root, self.nonce, self.turn, hold_stdin=True)
        with self.assertRaises(RuntimeError):
            self.rendezvous.probe()
        with self.assertRaises(RuntimeError):
            self.rendezvous.release()
        published = sorted(p.name for p in self.root.iterdir()
                           if p.name.startswith(('probe-', 'release-')))
        self.assertEqual(published, [])


class ParentRefusals(unittest.TestCase):
    """Refusal guards, each preserving the original evidence files."""

    def setUp(self):
        self.root = fixture_area.fixture_root('managed-file-refusal-')
        self.nonce = 'nonce-refusal'
        self.turn = 'alpha'
        self.process = None
        self.rendezvous = None
        self.pidfd = None

    def tearDown(self):
        if self.rendezvous is not None:
            self.rendezvous.close()
        if self.process is not None and self.process.poll() is None:
            RealChildWitness._quiet_kill(self.process, self.pidfd)
        elif self.pidfd is not None:
            os.close(self.pidfd)

    def _start_ready(self):
        self.process, self.rendezvous, self.pidfd, self.stdin_bytes = _start_provider(
            self.root, self.nonce, self.turn)
        self.rendezvous.ready()

    def test_wrong_nonce_ready_refused_then_child_refuses_bogus_release(self):
        """Foreign-nonce ready is refused parent-side; the child itself refuses a bad release nonce and exits non zero."""
        foreign_nonce = 'nonce-foreign'
        program_path = self.root / 'provider-alpha.py'
        program_path.write_text(protocol.program(foreign_nonce, self.turn), encoding='utf-8')
        self.process = subprocess.Popen(
            [sys.executable, program_path.name, '--model', 'fixture-model'],
            cwd=self.root, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE)
        self.process.stdin.write(b'foreign input\n')
        self.process.stdin.flush()
        self.process.stdin.close()
        self.pidfd = os.pidfd_open(self.process.pid)
        self.rendezvous = protocol.FileRendezvous(
            self.root, self.nonce, self.turn, _identity_of(self.process), self.pidfd)
        with self.assertRaises(RuntimeError) as caught:
            self.rendezvous.ready()
        self.assertIn('nonce mismatch', str(caught.exception))
        # The original foreign ready file is preserved as evidence.
        ready_path = self.root / ('ready-%s.json' % self.turn)
        foreign_ready = json.loads(ready_path.read_bytes())
        self.assertEqual(foreign_ready['nonce'], foreign_nonce)

        # The child validates its release nonce: publish a bogus one by hand;
        # the child wakes on the inotify event and exits non zero on its own.
        _publish_json(self.root, 'release-%s.json' % self.turn,
                      {'nonce': 'bogus', 'release': True})
        code, _, err = _finish(self.process, self.pidfd)
        self.pidfd = None
        self.assertNotEqual(code, 0, err)
        # err is the real stderr bytes; the child's release refusal names it.
        self.assertIn(b'release', err)
        self.rendezvous.close()
        self.rendezvous = None
        self.assertTrue(ready_path.exists())

    def test_forged_declared_pid_refused_original_preserved(self):
        """A ready file declaring a PID different from the validated identity is refused."""
        identity = dict(pid=os.getpid(), ppid=os.getpid(), sid=os.getsid(0), pgid=os.getpgrp())
        self.pidfd = os.pidfd_open(os.getpid())  # this very test process: alive, never fires
        forged = protocol.FileRendezvous(self.root, self.nonce, self.turn, identity, self.pidfd)
        self.pidfd = None  # forged owns the reference now
        self.rendezvous = forged
        _publish_json(self.root, 'ready-%s.json' % self.turn,
                      dict(pid=1, ppid=identity['ppid'], sid=identity['sid'],
                           pgid=identity['pgid'], nonce=self.nonce))
        with self.assertRaises(RuntimeError) as caught:
            forged.ready()
        self.assertIn('pid mismatch', str(caught.exception))
        # Original forged publication preserved: a report never authorizes a PID.
        self.assertTrue((self.root / ('ready-%s.json' % self.turn)).exists())
        forged.close()
        self.rendezvous = None

    def test_symlink_publication_refused_target_preserved(self):
        """A symlinked publication is refused by lstat; the real target is preserved."""
        self._start_ready()
        ready_name = 'ready-%s.json' % self.turn
        original = self.root / ready_name
        data = original.read_bytes()
        os.replace(original, self.root / ('.' + ready_name + '.real'))
        os.symlink('.' + ready_name + '.real', original)
        with self.assertRaises(RuntimeError) as caught:
            self.rendezvous._read_publication(ready_name)
        self.assertIn('not a regular file', str(caught.exception))
        self.assertEqual((self.root / ('.' + ready_name + '.real')).read_bytes(), data)
        self.rendezvous.close()
        self.rendezvous = None

    def test_hard_link_publication_refused(self):
        """A hard-linked publication (st_nlink > 1) is refused by lstat."""
        self._start_ready()
        ready_name = 'ready-%s.json' % self.turn
        os.link(self.root / ready_name, self.root / 'second-link.json')
        with self.assertRaises(RuntimeError) as caught:
            self.rendezvous._read_publication('second-link.json')
        self.assertIn('has been linked', str(caught.exception))
        self.rendezvous.close()
        self.rendezvous = None

    def test_traversal_name_refused_without_tree_writes(self):
        """A traversal publication name is refused as a name-shape violation."""
        self.pidfd = os.pidfd_open(os.getpid())
        forged = protocol.FileRendezvous(
            self.root, self.nonce, self.turn,
            dict(pid=os.getpid(), ppid=os.getpid(), sid=os.getsid(0), pgid=os.getpgrp()),
            self.pidfd)
        with self.assertRaises(ValueError) as caught:
            forged._await_publication('../outside.json')
        self.assertIn('plain workspace component', str(caught.exception))
        self.assertFalse((self.root.parent / 'outside.json').exists())
        forged.close()
        os.close(self.pidfd)
        self.pidfd = None

    def test_outside_workspace_component_refused(self):
        """Absolute or nested components are refused as outside the workspace."""
        self.pidfd = os.pidfd_open(os.getpid())
        forged = protocol.FileRendezvous(
            self.root, self.nonce, self.turn,
            dict(pid=os.getpid(), ppid=os.getpid(), sid=os.getsid(0), pgid=os.getpgrp()),
            self.pidfd)
        for bad in ('/etc/passwd', 'sub/dir.json'):
            with self.assertRaises(ValueError):
                forged._await_publication(bad)
        forged.close()
        os.close(self.pidfd)
        self.pidfd = None


class ChildDeathTermination(unittest.TestCase):
    """A real child death before ready/probe terminates the wait via pidfd."""

    def setUp(self):
        self.root = fixture_area.fixture_root('managed-file-death-')
        self.nonce = 'nonce-death'
        self.turn = 'alpha'
        self.process = None
        self.rendezvous = None
        self.pidfd = None

    def tearDown(self):
        if self.rendezvous is not None:
            self.rendezvous.close()
        if self.process is not None and self.process.poll() is None:
            RealChildWitness._quiet_kill(self.process, self.pidfd)
        elif self.pidfd is not None:
            os.close(self.pidfd)

    def _spawn(self, hold_stdin):
        program_path = self.root / 'provider-alpha.py'
        program_path.write_text(protocol.program(self.nonce, self.turn), encoding='utf-8')
        self.process = subprocess.Popen(
            [sys.executable, program_path.name, '--model', 'fixture-model'],
            cwd=self.root, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE)
        if not hold_stdin:
            self.process.stdin.write(b'death input\n')
            self.process.stdin.flush()
            self.process.stdin.close()
        self.pidfd = os.pidfd_open(self.process.pid)
        self.rendezvous = protocol.FileRendezvous(
            self.root, self.nonce, self.turn, _identity_of(self.process), self.pidfd)

    def test_death_before_ready_terminates(self):
        """A child killed before ready ends ready() with a real refusal, no loop.

        The test owns the stdin pipe (hold_stdin keeps it open so the child
        cannot reach EOF and publish ready); the kill happens BEFORE any EOF,
        and the owned stdin BufferedWriter is explicitly closed in the
        cleanup below — before the handle is lost — with the close refusals
        tolerated without hiding the primary assertion. No warning is
        suppressed and no garbage collection is forced: the leak guard is
        the real close, verifiable as ``self.process.stdin.closed``.
        """
        # hold_stdin: the child is blocked on its stdin read and cannot publish ready.
        self._spawn(hold_stdin=True)
        self.process.kill()
        with self.assertRaises(RuntimeError) as caught:
            self.rendezvous.ready()
        self.assertIn('died before publishing ready-%s.json' % self.turn, str(caught.exception))
        code = self.process.wait()
        err = self.process.stderr.read().decode()
        self.process.stdout.close()
        self.process.stderr.close()
        # Explicitly release the stdin the test still owns (the child died
        # without EOF): a refused close keeps its cause and never masks the
        # witness above.
        try:
            self.process.stdin.close()
        except (BrokenPipeError, OSError, ValueError) as cleanup:
            raise AssertionError('owned stdin close refused: %r' % (cleanup,)) from cleanup
        self.assertTrue(self.process.stdin.closed)
        os.close(self.pidfd)
        self.pidfd = None
        self.assertEqual(code, -9)
        self.assertEqual(err, '')

    def test_death_before_probe_terminates(self):
        """A child dying after ready but before probe ends probe() with a refusal."""
        self._spawn(hold_stdin=False)
        self.rendezvous.ready()
        self.process.kill()
        with self.assertRaises(RuntimeError) as caught:
            self.rendezvous.probe()
        self.assertIn('died before publishing', str(caught.exception))
        code = self.process.wait()
        err = self.process.stderr.read().decode()
        self.process.stdout.close()
        self.process.stderr.close()
        os.close(self.pidfd)
        self.pidfd = None
        self.assertEqual(code, -9)
        self.assertEqual(err, '')


class ExternalTerminal(unittest.TestCase):
    """A dead declared external terminal terminates ready/probe causally."""

    def setUp(self):
        self.root = fixture_area.fixture_root('managed-file-terminal-')
        self.nonce = 'nonce-terminal'
        self.turn = 'alpha'
        self.process = None
        self.rendezvous = None
        self.pidfd = None
        self.terminal_pidfd = None

    def tearDown(self):
        if self.rendezvous is not None:
            self.rendezvous.close()
        if self.process is not None and self.process.poll() is None:
            RealChildWitness._quiet_kill(self.process, self.pidfd)
        elif self.pidfd is not None:
            os.close(self.pidfd)
        if self.terminal_pidfd is not None:
            os.close(self.terminal_pidfd)

    def test_dead_external_terminal_ends_ready_with_role_named_refusal(self):
        """A fired (dead) external terminal raises a role-named refusal, provider alive.

        The conductor stand-in is this very test process observed through a real
        pidfd; the terminal is fired for real by killing a real child process
        whose pidfd stands in for the external terminal, so the selector fires
        on genuine kernel readability — no fake pass, no injected event and no
        watchdog. The provider child stays alive the whole time (hold_stdin
        keeps it blocked before ready), proving ready() exited on the terminal,
        not on the provider; the caller-owned terminal descriptor is never
        closed by the rendezvous (still fstat-able afterwards) and the provider
        is then reaped with its real exit status conserved.
        """
        # Real external-terminal witness: a real child process of this test,
        # observed through a real pidfd. Killing it makes that pidfd readable,
        # exactly as a dead conductor's descriptor would fire in integration.
        terminal = subprocess.Popen(
            [sys.executable, '-c', 'import sys; sys.stdin.read()'],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            self.terminal_pidfd = os.pidfd_open(terminal.pid)
            # Valid witness first: the terminal pidfd does not fire while the
            # stand-in process is alive (zero-time select, no event).
            self.assertFalse(select.select([self.terminal_pidfd], [], [], 0)[0])
            terminal.kill()
            terminal.wait()
            self.assertTrue(select.select([self.terminal_pidfd], [], [], 0)[0])
            # Provider alive and blocked before ready; the rendezvous declares
            # the dead terminal, so ready() must refuse naming the role.
            self.process, self.rendezvous, self.pidfd, _ = _start_provider(
                self.root, self.nonce, self.turn, hold_stdin=True)
            rendezvous = protocol.FileRendezvous(
                self.root, self.nonce, self.turn, _identity_of(self.process),
                self.pidfd, terminal_fds=(('conductor', self.terminal_pidfd),))
            self.rendezvous.close()
            self.rendezvous = rendezvous
            with self.assertRaises(RuntimeError) as caught:
                rendezvous.ready()
            self.assertIn('external terminal fired: conductor', str(caught.exception))
            # The provider child is still alive: the terminal, not the provider,
            # ended the wait. Its real exit status is conserved afterwards.
            self.assertIsNone(self.process.poll())
            # The caller-owned terminal descriptor was not closed by the module.
            os.fstat(self.terminal_pidfd)
        finally:
            terminal.stdin.close()
            terminal.stdout.close()
            terminal.stderr.close()
        # Reap the still-alive provider with its real kill exit status.
        code, err = _kill_reap(self.process, self.pidfd)
        self.pidfd = None
        self.assertEqual(code, -9)
        self.assertEqual(err, '')


class CloseSeamInjection(unittest.TestCase):
    """An injected close refusal conserves primary+secondary causes and leaves no leak."""

    def setUp(self):
        self.root = fixture_area.fixture_root('managed-file-close-')
        self.nonce = 'nonce-close'
        self.turn = 'alpha'
        self.original_close = protocol.close_descriptor

    def tearDown(self):
        protocol.close_descriptor = self.original_close

    def test_injected_close_refusal_on_close_is_conserved_without_leak(self):
        """Valid witness first, then close() refusal propagates once, slot cleared."""
        program_path = self.root / 'provider-alpha.py'
        program_path.write_text(protocol.program(self.nonce, self.turn), encoding='utf-8')
        process = subprocess.Popen(
            [sys.executable, program_path.name, '--model', 'fixture-model'],
            cwd=self.root, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE)
        process.stdin.write(b'close input\n')
        process.stdin.flush()
        process.stdin.close()
        pidfd = os.pidfd_open(process.pid)
        rendezvous = protocol.FileRendezvous(
            self.root, self.nonce, self.turn,
            dict(pid=process.pid, ppid=os.getpid(), sid=os.getsid(process.pid),
                 pgid=os.getpgid(process.pid)), pidfd)
        try:
            # Valid witness: the full lifecycle works before any injection.
            rendezvous.ready()
            rendezvous.probe()
            rendezvous.release()
            out = process.stdout.read()
            err = process.stderr.read()
            process.stdout.close()
            process.stderr.close()
            code = process.wait()
            self.assertEqual(code, 0, err)
            self.assertIn('"result"', out.decode())

            watch_fd = rendezvous._watch_fd
            refusals = []

            def refusing_close(fd):
                refusals.append(fd)
                raise OSError(9, 'injected close refusal')

            protocol.close_descriptor = refusing_close
            with self.assertRaises(OSError) as caught:
                rendezvous.close()
            self.assertEqual(caught.exception.errno, 9)
            protocol.close_descriptor = self.original_close
            self.assertEqual(refusals, [watch_fd])
            self.assertIsNone(rendezvous._watch_fd)
            # Second close is a no-op: the same descriptor is never re-closed.
            rendezvous.close()
            self.assertEqual(refusals, [watch_fd])
            # The test really closes the still-open descriptor: no leak.
            os.close(watch_fd)
        finally:
            protocol.close_descriptor = self.original_close
            os.close(pidfd)

    def test_primary_and_secondary_close_causes_grouped(self):
        """A read refusal plus an injected close refusal surface as one conserved group."""
        _publish_bytes(self.root, 'good.json', (json.dumps({'nonce': self.nonce}) + '\n').encode())
        _publish_bytes(self.root, 'bad.json', b'this is not json\n')
        self.pidfd = os.pidfd_open(os.getpid())
        rendezvous = protocol.FileRendezvous(
            self.root, self.nonce, self.turn,
            dict(pid=os.getpid(), ppid=os.getpid(), sid=os.getsid(0), pgid=os.getpgrp()),
            self.pidfd)
        try:
            # Valid witness: a good publication reads cleanly before injection.
            payload = rendezvous._read_publication('good.json')
            self.assertEqual(payload, {'nonce': self.nonce})
            # Without injection the same bad file yields its primary cause alone.
            with self.assertRaises(RuntimeError) as plain:
                rendezvous._read_publication('bad.json')
            self.assertIn('not valid JSON', str(plain.exception))

            def refusing_close(fd):
                raise OSError(9, 'injected close refusal')

            protocol.close_descriptor = refusing_close
            try:
                with self.assertRaises(BaseExceptionGroup) as caught:
                    rendezvous._read_publication('bad.json')
            finally:
                protocol.close_descriptor = self.original_close
            causes = caught.exception.exceptions
            self.assertEqual(len(causes), 2)
            self.assertIsInstance(causes[0], RuntimeError)
            self.assertIn('not valid JSON', str(causes[0]))
            self.assertIsInstance(causes[1], OSError)
            self.assertEqual(causes[1].errno, 9)
        finally:
            protocol.close_descriptor = self.original_close
            os.close(self.pidfd)
            self.pidfd = None


class IdentityShapes(unittest.TestCase):
    """Constructor shape refusals; no process is started and no file is written."""

    def setUp(self):
        self.root = fixture_area.fixture_root('managed-file-shapes-')
        self.pidfd = os.pidfd_open(os.getpid())

    def tearDown(self):
        os.close(self.pidfd)

    def _identity(self, **overrides):
        base = dict(pid=10, ppid=1, sid=10, pgid=10)
        base.update(overrides)
        return base

    def test_constructor_refuses_missing_identity_fields(self):
        """An identity missing an already-validated field is refused."""
        with self.assertRaises(ValueError):
            protocol.FileRendezvous(self.root, 'n', 'alpha', dict(pid=10), self.pidfd)

    def test_constructor_refuses_non_integer_identity(self):
        """A non-integer identity field is refused."""
        with self.assertRaises(ValueError):
            protocol.FileRendezvous(self.root, 'n', 'alpha', self._identity(pid='10'), self.pidfd)

    def test_constructor_refuses_bad_pidfd(self):
        """A negative or non-descriptor pidfd is refused."""
        for bad in (-1, 'fd', None):
            with self.assertRaises(ValueError):
                protocol.FileRendezvous(self.root, 'n', 'alpha', self._identity(), bad)

    def test_constructor_refuses_bad_nonce(self):
        """An empty or non-string nonce is refused."""
        for bad in ('', None, 5):
            with self.assertRaises(ValueError):
                protocol.FileRendezvous(self.root, bad, 'alpha', self._identity(), self.pidfd)

    def test_constructor_refuses_traversal_turn(self):
        """A traversal turn name is refused at construction."""
        with self.assertRaises(ValueError):
            protocol.FileRendezvous(self.root, 'n', '../x', self._identity(), self.pidfd)

    def test_constructor_refuses_bad_terminal_fds(self):
        """A terminal pair with an empty role or a bad fd is refused; shapes only."""
        for bad in (('', 5), (None, 5), ('conductor', -1), ('conductor', 'fd')):
            with self.assertRaises(ValueError):
                protocol.FileRendezvous(self.root, 'n', 'alpha', self._identity(),
                                        self.pidfd, terminal_fds=(bad,))


if __name__ == '__main__':
    unittest.main(verbosity=2)
