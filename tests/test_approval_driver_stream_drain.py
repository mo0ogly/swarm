#!/usr/bin/env python3
"""Regression guard: the approval-form driver drains ALL FOUR recorders before any verdict.

Purpose: prove the general contract of ``run`` in
``tests/paea_approval_form_ui_recipe.py`` after the received causal anomaly
(evidence ``glm-approval-readme-exact-symbols-20261104T051034Z/
supervisor-stream-drain-discriminant/``): the driver published the verdict /
result.json / the nominative stdout marker while one harness recorder had
not reached EOF (``recorders_alive_at_verdict:1``,
``harness_stdout_exists_at_verdict:false``) — the two harness recorders were
started but never retained or joined, and a recorder worker error would
have vanished with its daemon thread.

The tripwire is a strict event ordering, not a source-shape check:

- the single harness-stdout recorder is returned SUSPENDED by the
  ``make_recorder`` seam: its worker waits on an event before running the
  real ``stream``, so its sink can only receive bytes after the release;
- the recorder's ``join()`` records ``suspended.join_entered`` and then
  sets the release event (the causal observation "the controller entered
  join"), and records ``suspended.joined_done`` once the worker finished;
- the ``print_verdict_marker`` seam records ``verdict``;

the guard fails unless ``verdict`` is strictly AFTER ``suspended.joined_done``.
With the received bug (harness recorders never retained or joined) the
worker stays gated forever: no ``joined_done`` is ever recorded, the sink
never receives its bytes and the verdict fires first — an absent marker
makes the ordering comparison raise, a late marker fails the ordering
assertion, so both bug shapes fail this guard. With the fix, the run
completes exit 0 with the complete expected bytes on all four channels,
the real 0/0 exits in ``exits.json`` and the nominative driver marker on
stdout.

Method: REAL local children — two tiny owned python scripts (no cargo, no
node, no Chrome, no provider, no network, no ELF copy, no live database) —
spawned through the driver's own ``spawn_rust_test`` /
``spawn_browser_harness`` seams with real pipes. The fake rust child
publishes ``server.json`` in the control directory exactly like the real
bind, then writes its marker bytes and exits 0; the fake harness child
writes the ``approval-form-ui.json`` proof (four checks, no unexpected
diagnostics) then writes its marker bytes and exits 0. ``resolve_binary``
is substituted with the identity (the fixture "binary" is a tiny owned
file, never executed) so no cargo runs; ``sha256sum`` stays the driver's
real call on the tiny file. No timer, no sleep, no timeout is added by
this guard: the release is a pure event handshake, and the driver's own
polls and unbounded joins are the production behavior under test.

Variants:

- recorder worker error: the harness-stdout sink argument passed through
  the ``make_recorder`` seam is a pre-created DIRECTORY, so the worker's
  ``sink.open('xb')`` raises ``FileExistsError`` (O_EXCL on an existing
  path fails with EEXIST before any directory check — received and
  reproduced 2026-11-04: the guard's original ``IsADirectoryError``
  oracle contradicted the real exclusive-open semantics and was
  corrected, not the product); the driver must refuse with ``output
  recorder worker failed``, keep the cause in ``exits.json``
  ``recorder_errors``, preserve the other three channels, record the
  real observed exits (which may be negative signal exits if the
  containment cancelled an owned child — asserted as observed, never
  invented), and a fresh output directory must then recover to a full
  pass;
- recorder failure under pipe backpressure (received and reproduced
  2026-11-04 in ``supervisor-stream-backpressure/``): a measured pipe
  capacity (measured causally on a probe pipe before any child, never a
  hardcoded 8192) with a producer whose exactly-capacity first write is
  acknowledged over an aux pipe; the producer's OWN rust-stdout recorder
  is ack-gated and killed by a pre-created directory sink once the fill
  has landed, leaving the pipe full and unread — the producer's second
  write can only complete if someone reads. The driver must observe the
  recorder failure in its wait loops, cancel the owned blocked child
  (real negative-signal exit recorded as observed), mark the failed
  channel INCOMPLETE by capture, preserve the healthy channels and
  refuse naming the cause — without waiting forever for the blocked
  producer; a fresh output directory must then recover to a full pass;
- late recorder failure during the final rust wait (received and
  reproduced 2026-11-04 in ``supervisor-late-wait-probe/``): after the
  harness exited and the one-shot check read an EMPTY box, the recorder
  fails DURING the (former bare) rust wait — full unread pipe, producer
  alive, ``FileExistsError`` captured, bare ``wait`` intercepted. The
  driver's corrected loop must keep observing ``error_box`` until the
  child's real exit: the causal release rides the seams (harness
  ``wait`` marks the post-harness boundary, the rust ``poll``
  substitute releases the gated worker on the late loop's first poll),
  the refusal carries the real channel name with INCOMPLETE marking,
  the owned blocked producer is cancelled (real negative-signal exit
  recorded as observed) and a fresh output directory recovers to a full
  pass; a healthy witness with the same suspension completes 0/0;
- spawn failure: a ``spawn_rust_test`` substitute raises before any child
  exists; the original error must propagate unmasked and ``exits.json``
  must record explicit ``null`` exits — never an invented exit.

Inputs: none (self-contained). Outputs: named stdout markers and per-case
assertions; guard fixtures are allocated under the repository fixture area
via ``fixture_area.fixture_root`` (never ``/tmp``). Errors: any violated
invariant is a hard unittest failure; if the driver module itself cannot
be imported the guard SKIPs loudly — and a skip exits non-zero (as does
any failure), so a recipe run can never read a degraded guard as OK.

Limits: this guard exercises the driver's drain/exit/verdict sequencing on
fixture children; it does not run cargo, the real browser harness, the
real Rust test or any product policy, and it does not measure durations —
the with-bug detection rests on the strict event ordering above, not on
latency.
"""
import contextlib
import importlib.util
import io
import json
import os
import subprocess
import sys
import threading
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fixture_area import fixture_root

_driver_load_error = None
try:
    _spec = importlib.util.spec_from_file_location(
        "paea_approval_form_ui_recipe_under_test",
        str(Path(__file__).resolve().parent / "paea_approval_form_ui_recipe.py"))
    driver = importlib.util.module_from_spec(_spec)
    _spec.loader.exec_module(driver)
except Exception as _exc:  # pragma: no cover - loud skip, never a fake PASS
    _driver_load_error = repr(_exc)
    driver = None


class SuspendedRecorder(threading.Thread):
    """Recorder thread whose worker stays gated until its controller joins it.

    The worker (built from the real ``stream`` target and its arguments)
    first waits on ``release``; the overridden ``join()`` is the only
    releaser during a fixed run, which makes the ordering observation
    causal: ``suspended.join_entered`` is appended exactly when the
    controller calls ``join``, the release unblocks the worker, and
    ``suspended.joined_done`` is appended only once the real ``stream`` has
    run to completion (EOF). Repeated joins (the driver's finally-path
    re-drain) append nothing more. If a regression never joins this
    recorder, the worker stays gated — the sink receives no bytes and no
    ``joined_done`` is ever recorded, which is exactly the counterfactual
    the guard's ordering assertion fails on.
    """

    def __init__(self, target, name, args, release, order):
        super().__init__(target=self._gated_worker, args=(target, args),
                         name=name, daemon=True)
        self._release = release
        self._order = order
        self._join_entered = False
        self._join_done = False

    def _gated_worker(self, target, args):
        self._release.wait()
        target(*args)

    def join(self, timeout=None):
        """Record the causal join observation, release the worker, await it.

        Inputs: the optional inherited join timeout (never used by the
        driver, which joins unbounded). Effects: the first call appends
        ``suspended.join_entered`` to the shared order and sets ``release``;
        the call returns only after the gated worker finished the real
        ``stream``, and that first completion appends
        ``suspended.joined_done``. Errors: none of its own — a worker
        exception is already captured by the driver's error box.
        """
        if not self._join_entered:
            self._join_entered = True
            self._order.append("suspended.join_entered")
            self._release.set()
        super().join(timeout)
        if not self._join_done:
            self._join_done = True
            self._order.append("suspended.joined_done")


@unittest.skipIf(driver is None,
                 "driver under test could not be imported: " + repr(_driver_load_error))
class StreamDrain(unittest.TestCase):
    """Drain-before-verdict sequencing of ``run`` on real fixture children."""

    RUST_STDOUT = b"RUST_FIXTURE_STDOUT_MARKER\n"
    RUST_STDERR = b"RUST_FIXTURE_STDERR_MARKER\n"
    HARNESS_STDOUT = b"HARNESS_FIXTURE_STDOUT_MARKER\n"
    HARNESS_STDERR = b"HARNESS_FIXTURE_STDERR_MARKER\n"
    PROOF = {"checks": ["harness-check-1", "harness-check-2",
                        "harness-check-3", "harness-check-4"],
             "unexpected_diagnostics": []}

    def setUp(self):
        # Guard fixtures live under the repository fixture area (never /tmp).
        # The driver reads SWARM_PAEA_SOURCE_FIXTURE unconditionally; the
        # value is only resolved, never consumed by the fixture children.
        self.fixture = Path(fixture_root("approval-driver-stream-drain-"))
        self.source_fixture = self.fixture / "source-fixture"
        self.source_fixture.mkdir()
        patcher = mock.patch.dict(os.environ, {
            "SWARM_PAEA_SOURCE_FIXTURE": str(self.source_fixture)})
        patcher.start()
        self.addCleanup(patcher.stop)
        self.order = []          # causal event order shared with the seams
        self.release = threading.Event()
        self.suspended = None    # the one suspended harness-stdout recorder
        self._write_children()
        self.binary = self.fixture / "fixture-binary"
        self.binary.write_bytes(b"fixture bytes - never executed\n")

    def _write_children(self):
        """Write the two tiny real children spawned through the exec seams.

        The rust child publishes ``server.json`` in the control directory
        (``APPROVAL_FORM_BROWSER_DIR`` from the driver's env) exactly like
        the real bind, then emits its marker bytes and exits 0. The harness
        child receives ``binary output control`` like the real node call,
        writes the proof JSON the driver asserts on, emits its marker bytes
        and exits 0. Both are plain python scripts: no cargo, no node, no
        browser, no network. The byte literals come from the class
        constants via ``%r`` so the assertions and the children cannot
        drift apart.
        """
        self.rust_child = self.fixture / "rust-child.py"
        self.rust_child.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, sys\n"
            "control = os.environ['APPROVAL_FORM_BROWSER_DIR']\n"
            "with open(os.path.join(control, 'server.json'), 'w') as file:\n"
            "    json.dump({'fixture': True}, file)\n"
            "sys.stdout.buffer.write(%r)\n"
            "sys.stdout.buffer.flush()\n"
            "sys.stderr.buffer.write(%r)\n"
            "sys.stderr.buffer.flush()\n"
            "sys.exit(0)\n" % (self.RUST_STDOUT, self.RUST_STDERR))
        self.harness_child = self.fixture / "harness-child.py"
        self.harness_child.write_text(
            "#!/usr/bin/env python3\n"
            "import json, sys\n"
            "binary, output, control = sys.argv[1], sys.argv[2], sys.argv[3]\n"
            "proof = %r\n"
            "with open(output + '/approval-form-ui.json', 'w') as file:\n"
            "    json.dump(proof, file)\n"
            "sys.stdout.buffer.write(%r)\n"
            "sys.stdout.buffer.flush()\n"
            "sys.stderr.buffer.write(%r)\n"
            "sys.stderr.buffer.flush()\n"
            "sys.exit(0)\n" % (self.PROOF, self.HARNESS_STDOUT,
                               self.HARNESS_STDERR))

    def _fake_spawn_rust(self, binary, env):
        """Spawn the real rust fixture child with real pipes (exec seam)."""
        return subprocess.Popen([sys.executable, str(self.rust_child)],
                                stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, env=env)

    def _fake_spawn_harness(self, binary, output, control, env):
        """Spawn the real harness fixture child with real pipes (exec seam)."""
        return subprocess.Popen([sys.executable, str(self.harness_child),
                                 str(binary), str(output), str(control)],
                                stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, env=env)

    def _suspended_make_recorder(self, target, name, *args):
        """Recorder factory: suspend exactly the ``stream:harness.stdout`` one.

        The suspended recorder's worker waits for the release event before
        running the real ``stream``, so only the controller's ``join`` can
        let its bytes reach the sink. All other recorders are the driver's
        ordinary daemon threads.
        """
        if name == "stream:harness.stdout":
            recorder = SuspendedRecorder(target, name, args,
                                         self.release, self.order)
            self.suspended = recorder
            return recorder
        return threading.Thread(target=target, args=args, name=name,
                                daemon=True)

    def _patched(self, make_recorder=None, spawn_rust=None, spawn_harness=None):
        """Enter the seam substitutions for one driver run.

        Only the driver's own module-level seams are substituted — the code
        under test (``run``) is never replaced or re-implemented.
        ``resolve_binary`` becomes the identity (no cargo; the fixture
        binary is never executed), the spawn seams run the two tiny real
        children with real pipes, and the marker seam records the
        ``verdict`` event before printing the real marker. Returns the
        entered ``contextlib.ExitStack``.
        """
        stack = contextlib.ExitStack()
        stack.enter_context(mock.patch.object(
            driver, "resolve_binary", side_effect=lambda spec: spec))
        stack.enter_context(mock.patch.object(
            driver, "spawn_rust_test",
            side_effect=spawn_rust or self._fake_spawn_rust))
        stack.enter_context(mock.patch.object(
            driver, "spawn_browser_harness",
            side_effect=spawn_harness or self._fake_spawn_harness))
        if make_recorder is not None:
            stack.enter_context(mock.patch.object(
                driver, "make_recorder", side_effect=make_recorder))
        real_marker = driver.print_verdict_marker

        def record_verdict(checks):
            """Record the verdict event, then print the real marker."""
            self.order.append("verdict")
            real_marker(checks)

        stack.enter_context(mock.patch.object(
            driver, "print_verdict_marker", side_effect=record_verdict))
        return stack

    def test_docstring_states_the_drain_promise(self):
        """The documented driver contract carries the drain/exit promises."""
        self.assertIn("joined BEFORE any verdict", driver.__doc__)
        self.assertIn("never a false PASS on a thread exception", driver.__doc__)
        self.assertIn("never an invented exit", driver.__doc__)
        self.assertIn("intentionally unbounded", driver.__doc__)

    def test_drain_completes_before_verdict_with_complete_bytes(self):
        """Valid witness: joins finish first, then verdict, bytes complete.

        The suspended harness-stdout recorder can only be released by the
        controller's join, so the run must observe ``suspended.join_entered``
        then ``suspended.joined_done`` strictly before ``verdict``. With the
        received bug the worker stays gated: no ``joined_done`` is recorded,
        the sink never receives its bytes and the verdict fires first —
        this assertion fails on exactly that shape. The passing run also
        proves the complete marker bytes on all four channels, the real
        0/0 exits in ``exits.json``, ``harness_exit`` in ``result.json``
        and the nominative driver marker with 4 checks.
        """
        output = self.fixture / "run-drain"
        with self._patched(make_recorder=self._suspended_make_recorder):
            with contextlib.redirect_stdout(io.StringIO()) as captured:
                try:
                    driver.run(self.binary, output)
                finally:
                    self.release.set()   # bug shape: ungate the worker
        self.assertEqual(self.order,
                         ["suspended.join_entered", "suspended.joined_done",
                          "verdict"])
        self.assertIn("PAEA_APPROVAL_FORM_BROWSER_VERIFIED 4",
                      captured.getvalue())
        self.assertEqual((output / "harness.stdout").read_bytes(),
                         self.HARNESS_STDOUT)
        self.assertEqual((output / "harness.stderr").read_bytes(),
                         self.HARNESS_STDERR)
        self.assertEqual((output / "rust.stdout").read_bytes(),
                         self.RUST_STDOUT)
        self.assertEqual((output / "rust.stderr").read_bytes(),
                         self.RUST_STDERR)
        self.assertEqual(json.loads((output / "exits.json").read_text()),
                         {"rust_exit": 0, "harness_exit": 0})
        result = json.loads((output / "result.json").read_text())
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["rust_exit"], 0)
        self.assertEqual(result["harness_exit"], 0)

    def test_recorder_worker_error_refuses_and_fresh_directory_recovers(self):
        """Worker sink failure: refuse with the cause, then recover fresh.

        The harness-stdout sink argument is replaced by a pre-created
        directory, so the worker's ``sink.open('xb')`` raises
        ``FileExistsError`` (O_EXCL on an existing path fails with EEXIST
        before any directory check — the real exclusive-open semantics;
        received and reproduced 2026-11-04: the original
        ``IsADirectoryError`` oracle contradicted it and was corrected in
        the guard, not in the product). The driver must raise
        ``SystemExit`` carrying ``output recorder worker failed`` and the
        ``FileExistsError`` cause, record the error in ``exits.json``
        ``recorder_errors`` with the real observed exits, preserve the
        other three channels byte-complete, and a fresh output directory
        must then recover to a full pass with complete bytes.
        """
        blocker = self.fixture / "harness.stdout"
        blocker.mkdir()
        output = self.fixture / "run-worker-error"

        def blocked_recorder(target, name, *args):
            if name == "stream:harness.stdout":
                return threading.Thread(target=target,
                                        args=(args[0], blocker, args[2]),
                                        name=name, daemon=True)
            return threading.Thread(target=target, args=args, name=name,
                                    daemon=True)

        with self._patched(make_recorder=blocked_recorder):
            with self.assertRaises(SystemExit) as ctx:
                driver.run(self.binary, output)
        text = str(ctx.exception)
        self.assertIn("output recorder worker failed", text)
        self.assertIn("FileExistsError", text)
        self.assertIn("harness.stdout", text)
        self.assertIn("INCOMPLETE", text)
        exits = json.loads((output / "exits.json").read_text())
        self.assertEqual(len(exits["recorder_errors"]), 1)
        self.assertIn("FileExistsError", exits["recorder_errors"][0])
        self.assertIn("harness.stdout", exits["recorder_errors"][0])
        # Real observed exits only — never invented, never a forced 0/0
        # under normal cancellation: the failed capture makes the owned
        # child cancellable, so each fixture child either self-exited 0 or
        # records its real SIGKILL exit -9 from the owned containment.
        self.assertIn(exits["rust_exit"], (0, -9))
        self.assertIn(exits["harness_exit"], (0, -9))
        # Channels of self-exited children are byte-complete; a cancelled
        # child's channels are preserved only up to the kill and are never
        # asserted complete (the refusal marks the failed channel instead).
        if exits["rust_exit"] == 0:
            self.assertEqual((output / "rust.stdout").read_bytes(),
                             self.RUST_STDOUT)
            self.assertEqual((output / "rust.stderr").read_bytes(),
                             self.RUST_STDERR)
        if exits["harness_exit"] == 0:
            self.assertEqual((output / "harness.stderr").read_bytes(),
                             self.HARNESS_STDERR)
        # Recovery: a fresh output directory runs the same driver to a pass.
        fresh = self.fixture / "run-worker-error-fresh"
        with self._patched():
            with contextlib.redirect_stdout(io.StringIO()) as captured:
                driver.run(self.binary, fresh)
        self.assertIn("PAEA_APPROVAL_FORM_BROWSER_VERIFIED 4",
                      captured.getvalue())
        self.assertEqual((fresh / "harness.stdout").read_bytes(),
                         self.HARNESS_STDOUT)
        self.assertTrue((fresh / "result.json").exists())

    def test_spawn_failure_records_explicit_null_exits(self):
        """Never-spawned children: real error propagates, exits stay null.

        A ``spawn_rust_test`` substitute raises before any child exists.
        The original error must propagate unmasked (never converted into an
        invented exit or a verdict) and ``exits.json`` must record explicit
        ``null`` for both children on that path, with no ``result.json``.
        """
        output = self.fixture / "run-spawn-failure"

        def refused_spawn(binary, env):
            raise FileNotFoundError(f"fixture spawn refused: {binary}")

        with self._patched(spawn_rust=refused_spawn):
            with self.assertRaises(FileNotFoundError):
                driver.run(self.binary, output)
        self.assertEqual(json.loads((output / "exits.json").read_text()),
                         {"rust_exit": None, "harness_exit": None})
        self.assertFalse((output / "result.json").exists())


    # --- received backpressure repro helpers ------------------------------
    # Received and reproduced 2026-11-04 (supervisor-stream-backpressure/
    # before-recovery.json + after-recovery.json): the real ``stream`` with
    # a failing sink returns after capture, its read side of the pipe
    # disappears, a producer writing more than the measured pipe capacity
    # blocks forever on its next write, and a controller that waits for the
    # children before observing ``error_box`` hangs. Only an explicit
    # supervisor drain recovered the child (exit 0, 131072 bytes). The
    # case below reproduces that shape causally (event handshake over an
    # aux pipe, capacity measured on a probe pipe before any child, no
    # sleep, no volume ceiling beyond the measured capacity) and proves
    # the fixed containment.

    @staticmethod
    def _measure_pipe_capacity():
        """Measure the OS pipe capacity causally, without any sleep.

        Writes 1-byte chunks to the write end of a fresh pipe from the
        test thread and counts until the kernel's blocking boundary; a
        reader then drains the measured bytes. Output: the exact capacity
        in bytes (65536 on the received Linux evidence). Errors: raises on
        any unexpected os.write failure. Never a hardcoded 8192 — the
        received evidence explicitly conserved and diagnosed that false
        fixture precondition.
        """
        read_fd, write_fd = os.pipe()
        try:
            os.set_blocking(write_fd, False)
            capacity = 0
            try:
                while True:
                    os.write(write_fd, b"x")
                    capacity += 1
            except BlockingIOError:
                pass
            finally:
                os.set_blocking(write_fd, True)
            # Drain through a real reader: proves the counted bytes are
            # actually buffered in the pipe, not lost.
            os.close(write_fd)
            write_fd = -1
            drained = 0
            while True:
                chunk = os.read(read_fd, 65536)
                if not chunk:
                    break
                drained += len(chunk)
            assert drained == capacity, (drained, capacity)
            return capacity
        finally:
            for fd in (read_fd, write_fd):
                if fd >= 0:
                    os.close(fd)

    def _backpressure_child_text(self, capacity):
        """Return the blocked-producer fixture child source.

        The child writes exactly ``capacity`` bytes of its stdout marker
        (``(marker * (capacity // len(marker) + 1))[:capacity]`` — the
        pipe is filled to its measured boundary, never a short first
        probe), then acknowledges the landed fill over an AUX PIPE passed
        with ``pass_fds`` (causal protocol, no timer), then publishes
        ``server.json`` like the real bind, then attempts a second write
        — which blocks until a reader drains the pipe or the child dies,
        because exactly ``capacity`` unread bytes are already buffered.
        The stderr marker and exit 0 are reachable only if that second
        write completes. The child always acknowledges after the fill:
        the guard's gated worker may therefore await the ack without any
        bound. Inputs: the measured capacity. Output: the python child
        source. Errors: none of its own.
        """
        marker = self.RUST_STDOUT
        return (
            "#!/usr/bin/env python3\n"
            "import json, os, sys\n"
            "aux = int(sys.argv[1])\n"
            "control = os.environ['APPROVAL_FORM_BROWSER_DIR']\n"
            "marker = %r\n"
            "filled = (marker * (%d // len(marker) + 1))[:%d]\n"
            "sys.stdout.buffer.write(filled)\n"
            "sys.stdout.buffer.flush()\n"
            "os.write(aux, b'A')\n"          # causal fill acknowledgment
            "with open(os.path.join(control, 'server.json'), 'w') as file:\n"
            "    json.dump({'fixture': True}, file)\n"
            "sys.stdout.buffer.write(marker)\n"  # blocks: pipe full, no reader
            "sys.stdout.buffer.flush()\n"
            "sys.stderr.buffer.write(%r)\n"
            "sys.stderr.buffer.flush()\n"
            "sys.exit(0)\n"
        ) % (marker, capacity, capacity, self.RUST_STDERR)

    def _backpressure_spawn_rust(self, capacity):
        """Return the spawn seam binding the aux pipe with ``pass_fds``.

        Inputs: the measured capacity (baked into the child source).
        Output: a ``spawn_rust_test(binary, env)`` substitute that creates
        the aux pipe, passes its write end to the child and hands the read
        end to the test via ``self.backpressure_ack`` (closed at test
        cleanup). The parent closes its own copy of the write end as soon
        as the child owns the inherited one — on spawn failure the just
        created pair is closed exactly once before propagating. Errors:
        propagation of Popen's own OSError.
        """
        def spawn(binary, env):
            aux_read, aux_write = os.pipe()
            self.backpressure_ack = os.fdopen(aux_read, "rb")
            self.addCleanup(self.backpressure_ack.close)
            try:
                proc = subprocess.Popen(
                    [sys.executable, "-c", self._backpressure_child_text(capacity),
                     str(aux_write)],
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env,
                    pass_fds=(aux_write,))
            except BaseException:
                os.close(aux_write)
                raise
            os.close(aux_write)   # the child owns the inherited copy now
            return proc
        return spawn

    def test_recorder_failure_under_backpressure_refuses_without_blocked_wait(self):
        """Received repro: failed recorder + full pipe must not hang the run.

        Valid witness first: the same fixture child with a FRESH sink
        writes its full payload through the real ``stream`` and exits 0
        (received control: 65536 bytes, exit 0). Then the fault case: the
        blocked producer's OWN rust-stdout recorder is ack-gated so its
        sink — a pre-created directory — is only opened AFTER the child's
        exactly-capacity first write is acknowledged over the aux pipe
        (causal, no sleep); ``sink.open('xb')`` then fails immediately and
        the recorder's read side of the rust-stdout pipe disappears while
        exactly ``capacity`` unread bytes sit in the pipe, and the child's
        second write can only complete if someone reads. The driver must
        observe the recorder failure inside its wait loops, cancel the
        OWNED blocked child (its real negative-signal exit is recorded as
        observed, never invented), mark the failed channel INCOMPLETE by
        capture and refuse naming the cause — instead of waiting forever
        for the blocked producer. A fresh output directory must then
        recover to a full pass.
        """
        capacity = self._measure_pipe_capacity()
        # The fill payload is exactly ``capacity`` bytes by construction
        # (same formula baked into the child below), so the child's second
        # write always overflows a pipe that nobody drains: the
        # backpressure shape holds for any measured capacity > 0 — no
        # hardcoded 8192, no volume ceiling beyond the measured boundary.
        self.assertGreater(capacity, 0,
                           "measured pipe capacity must be positive for "
                           "the fill/overflow shape to exist")
        marker = self.RUST_STDOUT
        payload = (marker * (capacity // len(marker) + 1))[:capacity]
        self.assertEqual(len(payload), capacity)
        # Valid witness: fresh sink, full payload drained by the real
        # ``stream``, real exit 0 (received control: 65536 bytes, exit 0).
        witness = self.fixture / "run-backpressure-witness"
        spawn_rust = self._backpressure_spawn_rust(capacity)
        with self._patched(spawn_rust=spawn_rust):
            with contextlib.redirect_stdout(io.StringIO()) as captured:
                driver.run(self.binary, witness)
        self.assertIn("PAEA_APPROVAL_FORM_BROWSER_VERIFIED 4",
                      captured.getvalue())
        self.assertTrue(self.backpressure_ack.read(1) == b"A")
        self.assertEqual(json.loads((witness / "exits.json").read_text()),
                         {"rust_exit": 0, "harness_exit": 0})
        self.assertEqual((witness / "rust.stdout").read_bytes(),
                         payload + marker)
        # Fault case: the BLOCKED PRODUCER'S OWN recorder is killed by a
        # pre-created directory sink, ack-gated so the failure is ordered
        # AFTER the landed fill (causal, no sleep): the substitute worker
        # first awaits the child's fill acknowledgment over the aux pipe,
        # then runs the real ``stream`` with the failing sink — its read
        # side of the rust-stdout pipe then disappears while exactly
        # ``capacity`` unread bytes sit in the pipe and the child blocks
        # forever on its second write.
        self.backpressure_ack = None
        self.fill_acked = threading.Event()
        acked = []
        spawn_rust_fault = self._backpressure_spawn_rust(capacity)
        # The blocker carries the REAL sink name: ``error_box`` keys the
        # channel on ``sink.name``, so the refusal must name exactly the
        # channel production would name ("rust.stdout"), not a variant.
        blocker = self.fixture / "rust.stdout"
        blocker.mkdir()
        output = self.fixture / "run-backpressure"

        def gated_failing_recorder(target, name, *args):
            if name == "stream:rust.stdout":
                def gated_worker():
                    acked.append(self.backpressure_ack.read(1))
                    self.fill_acked.set()
                    target(args[0], blocker, args[2])
                return threading.Thread(target=gated_worker, name=name,
                                        daemon=True)
            return threading.Thread(target=target, args=args, name=name,
                                    daemon=True)

        with self._patched(make_recorder=gated_failing_recorder,
                           spawn_rust=spawn_rust_fault):
            with self.assertRaises(SystemExit) as ctx:
                driver.run(self.binary, output)
        text = str(ctx.exception)
        self.assertIn("output recorder worker failed", text)
        self.assertIn("FileExistsError", text)
        self.assertIn("rust.stdout", text)
        self.assertIn("INCOMPLETE", text)
        # Causal fill acknowledgment, consumed by the gated worker itself:
        # the failure could only run after the child's full write landed,
        # so the pipe really was full and unread at the failure — never a
        # timer.
        self.assertTrue(self.fill_acked.is_set())
        self.assertEqual(acked, [b"A"])
        exits = json.loads((output / "exits.json").read_text())
        self.assertEqual(len(exits["recorder_errors"]), 1)
        self.assertIn("FileExistsError", exits["recorder_errors"][0])
        self.assertIn("rust.stdout", exits["recorder_errors"][0])
        # The owned blocked producer must have been cancelled by the
        # containment: it could never self-exit (its second write needs a
        # reader that no longer exists), so its real observed exit is the
        # negative SIGKILL exit — recorded as observed, never an invented
        # 0, never an unbounded wait. The healthy harness child was either
        # never spawned (refusal observed in the first wait loop),
        # self-exited 0, or was cancelled — asserted as observed.
        self.assertEqual(exits["rust_exit"], -9)
        self.assertIn(exits["harness_exit"], (None, 0, -9))
        # INCOMPLETE by capture, proven on disk: the failing recorder never
        # created its sink, so the ``capacity`` bytes the producer wrote
        # into the pipe are lost and the refusal marks the channel — the
        # output is never read as a full record. The stderr marker and the
        # child's exit 0 were never reachable without that reader.
        self.assertFalse((output / "rust.stdout").exists())
        if exits["harness_exit"] == 0:
            self.assertEqual((output / "harness.stderr").read_bytes(),
                             self.HARNESS_STDERR)
        # Recovery: a fresh output directory runs the same driver to a pass.
        fresh = self.fixture / "run-backpressure-fresh"
        self.backpressure_ack = None
        with self._patched():
            with contextlib.redirect_stdout(io.StringIO()) as captured:
                driver.run(self.binary, fresh)
        self.assertIn("PAEA_APPROVAL_FORM_BROWSER_VERIFIED 4",
                      captured.getvalue())
        self.assertTrue((fresh / "result.json").exists())

    def test_late_recorder_failure_during_rust_wait_refuses_without_bare_wait(self):
        """Received late window: the failure lands DURING the final rust wait.

        Received and reproduced 2026-11-04 (supervisor-late-wait-probe/
        probe.py + before-containment.json + after-containment.json):
        after the harness exited, ``recorder_failure()`` was checked ONCE,
        then the bare ``rust.wait()`` blocked — the causal probe kept the
        rust-stdout recorder suspended until ``wait`` was entered, failed
        it on the real exclusive open (pre-created directory sink),
        observed 65536 unread bytes with the producer alive and the
        FileExistsError already captured, then intercepted the blocking
        wait; the containment cancelled the owned child (real exit -9,
        harness 0) with no verdict. Valid witness first: the same fixture
        child with the same suspension released at the wait boundary runs
        to a real 0/0 pass. Fault case: the gated failing recorder is
        released causally by the late-wait loop's FIRST ``poll`` of the
        rust child (the one-shot check between the boundary and the loop
        never calls ``poll``), so the failure lands strictly after that
        check read the EMPTY box, and the refusal must carry the
        FileExistsError with the real channel name. No invented deadline,
        no volume ceiling beyond the measured capacity.
        """
        capacity = self._measure_pipe_capacity()
        self.assertGreater(capacity, 0)
        marker = self.RUST_STDOUT
        payload = (marker * (capacity // len(marker) + 1))[:capacity]
        self.assertEqual(len(payload), capacity)
        # The blocker carries the REAL channel name: ``error_box`` keys on
        # ``sink.name`` and the refusal must name exactly what production
        # names ("rust.stdout"). The pre-created directory makes the real
        # exclusive open raise the received FileExistsError.
        blocker = self.fixture / "rust.stdout"
        blocker.mkdir()
        self.backpressure_ack = None
        self.late_release = threading.Event()
        self.harness_done = threading.Event()

        def late_spawn_rust(binary, env):
            """Spawn the fixture rust child, arm the late-boundary poll seam.

            Same fixture child as the backpressure case (exactly-capacity
            fill, aux-pipe ack, server.json, blocked second write — no
            reader for it in the fault case). The child's ``poll`` is
            wrapped ONCE: its first call after ``harness_done`` is the
            driver's first late-wait-loop poll (strictly after the
            one-shot check, which never calls ``poll``) — it then releases
            the gated recorder worker and delegates to the real ``poll``.
            Errors: propagation of Popen's own OSError (aux fds closed
            exactly once on failure).
            """
            aux_read, aux_write = os.pipe()
            self.backpressure_ack = os.fdopen(aux_read, "rb")
            self.addCleanup(self.backpressure_ack.close)
            try:
                proc = subprocess.Popen(
                    [sys.executable, "-c", self._backpressure_child_text(capacity),
                     str(aux_write)],
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env,
                    pass_fds=(aux_write,))
            except BaseException:
                os.close(aux_write)
                raise
            os.close(aux_write)
            original_poll = proc.poll

            def observed_poll(*args, **kwargs):
                if (self.harness_done.is_set()
                        and not self.late_release.is_set()):
                    self.late_release.set()
                return original_poll(*args, **kwargs)

            proc.poll = observed_poll
            return proc

        def gated_late_recorder(failing: bool):
            """Return the recorder factory for this run.

            The ``stream:rust.stdout`` worker awaits ``late_release`` (set
            by the ``observed_poll`` substitute on the late-wait loop's
            first rust poll), reads the aux ack, then runs the real
            ``stream`` — with the failing pre-created-directory sink in
            the fault case, with the real fresh sink in the witness case
            — so the failure (if any) lands strictly AFTER the driver's
            one-shot check read the empty box, inside the late-wait
            window the loop now observes.
            """
            def factory(target, name, *args):
                if name == "stream:rust.stdout":
                    def gated_worker():
                        self.late_release.wait()
                        self.backpressure_ack.read(1)
                        target(args[0], blocker if failing else args[1], args[2])
                    return threading.Thread(target=gated_worker, name=name,
                                            daemon=True)
                return threading.Thread(target=target, args=args, name=name,
                                        daemon=True)
            return factory

        def late_spawn_harness(binary, output, control, env):
            """Spawn the fixture harness child, mark the late boundary.

            Identical to the ordinary harness spawn except that its
            ``wait`` is wrapped ONCE: the driver calls ``wait`` exactly
            once here (line ~389, right after the harness poll loop and
            before the one-shot recorder check), so the first call sets
            ``harness_done`` — the boundary from which the next rust
            ``poll`` is the late-wait loop's first observation — then
            delegates to the real ``wait``. Errors: propagation of
            Popen's own OSError.
            """
            proc = subprocess.Popen(
                [sys.executable, str(self.harness_child),
                 str(binary), str(output), str(control)],
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
            original_wait = proc.wait

            def marking_wait(*args, **kwargs):
                self.harness_done.set()
                return original_wait(*args, **kwargs)

            proc.wait = marking_wait
            return proc

        # Causal release wiring, seam-level only (no helper thread, no
        # timer): the harness ``wait`` substitute marks the post-harness
        # boundary (``harness_done``); the rust ``poll`` substitute
        # releases the gated worker (``late_release``) on its FIRST poll
        # after that boundary — i.e. the late-wait loop's first
        # observation. The one-shot check between them never calls
        # ``poll`` and never touches ``late_release``, so the worker's
        # failure (if any) can only land strictly AFTER that check read
        # the empty box and DURING the wait the loop now observes. The
        # loop's ``time.sleep(0.2)`` polling is the production observation
        # cadence under test.

        # Valid witness first: same fixture child, same suspension released
        # at the wait boundary, but a FRESH sink — the run must complete
        # 0/0 with the full payload, proving the wait-loop observation does
        # not wedge a healthy run whose recorder is still suspended at the
        # late boundary.
        witness = self.fixture / "run-late-witness"
        with self._patched(make_recorder=gated_late_recorder(False),
                           spawn_rust=late_spawn_rust,
                           spawn_harness=late_spawn_harness):
            with contextlib.redirect_stdout(io.StringIO()) as captured:
                driver.run(self.binary, witness)
        self.assertIn("PAEA_APPROVAL_FORM_BROWSER_VERIFIED 4",
                      captured.getvalue())
        self.assertEqual(json.loads((witness / "exits.json").read_text()),
                         {"rust_exit": 0, "harness_exit": 0})
        self.assertEqual((witness / "rust.stdout").read_bytes(),
                         payload + marker)
        self.assertTrue((witness / "result.json").exists())

        # Fault case: reset the causal state, failing sink this time.
        self.backpressure_ack = None
        self.late_release = threading.Event()
        self.harness_done = threading.Event()
        self.assertTrue(blocker.is_dir())
        output = self.fixture / "run-late-fault"
        with self._patched(make_recorder=gated_late_recorder(True),
                           spawn_rust=late_spawn_rust,
                           spawn_harness=late_spawn_harness):
            with self.assertRaises(SystemExit) as ctx:
                driver.run(self.binary, output)
        text = str(ctx.exception)
        self.assertIn("output recorder worker failed", text)
        self.assertIn("FileExistsError", text)
        self.assertIn("rust.stdout", text)
        self.assertIn("INCOMPLETE", text)
        # Causal ordering: the driver passed the post-harness boundary
        # (``harness_done`` — the one-shot check there read the EMPTY box,
        # otherwise the refusal would have fired earlier) and the first
        # late-wait poll released the worker, whose failure then landed
        # inside the observed loop.
        self.assertTrue(self.harness_done.is_set())
        self.assertTrue(self.late_release.is_set())
        exits = json.loads((output / "exits.json").read_text())
        self.assertEqual(len(exits["recorder_errors"]), 1)
        self.assertIn("FileExistsError", exits["recorder_errors"][0])
        self.assertIn("rust.stdout", exits["recorder_errors"][0])
        # The owned blocked producer could never self-exit (its second
        # write needs a reader that no longer exists): its real observed
        # exit is the negative SIGKILL exit from the owned containment,
        # recorded as observed, never invented. The harness self-exited 0
        # before the late window.
        self.assertEqual(exits["rust_exit"], -9)
        self.assertEqual(exits["harness_exit"], 0)
        # INCOMPLETE by capture, proven on disk: the failing recorder never
        # created its sink, so the capacity bytes the producer wrote into
        # the pipe are lost — the output is never read as a full record.
        self.assertFalse((output / "rust.stdout").exists())
        self.assertEqual((output / "harness.stderr").read_bytes(),
                         self.HARNESS_STDERR)
        self.assertFalse((output / "result.json").exists())
        # Recovery: a fresh output directory runs the same driver to a pass.
        fresh = self.fixture / "run-late-fresh"
        self.backpressure_ack = None
        self.late_release = threading.Event()
        self.harness_done = threading.Event()
        with self._patched(make_recorder=gated_late_recorder(False),
                           spawn_rust=late_spawn_rust,
                           spawn_harness=late_spawn_harness):
            with contextlib.redirect_stdout(io.StringIO()) as captured:
                driver.run(self.binary, fresh)
        self.assertIn("PAEA_APPROVAL_FORM_BROWSER_VERIFIED 4",
                      captured.getvalue())
        self.assertTrue((fresh / "result.json").exists())


if __name__ == "__main__":
    # Explicit stdout runner: both the unittest summary ("OK") and the named
    # marker land on stdout, matching the recipe step's required_stdout
    # (separating channels would let a stderr-only "OK" mask a stdout sink).
    # A skip is a hard failure here: this guard has no legitimate skip once
    # the driver imports, so a skipped guard must never read as recipe "OK".
    suite = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
    result = unittest.TextTestRunner(stream=sys.stdout, verbosity=2).run(suite)
    failures = len(result.failures) + len(result.errors)
    skipped = len(result.skipped)
    print(f"APPROVAL_DRIVER_STREAM_DRAIN_GUARD run={result.testsRun} "
          f"failed={failures} skipped={skipped}", flush=True)
    sys.exit(1 if failures or skipped else 0)
