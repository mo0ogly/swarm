#!/usr/bin/env python3
"""Regression guard: the approval-form driver keeps raw cargo-resolution evidence.

Purpose: prove the general contract of ``resolve_binary`` in
``tests/paea_approval_form_ui_recipe.py`` — every cargo artifact-resolution
attempt (success OR failure) writes the integral unfiltered stdout, the
integral stderr, the argv and the real child exit into a fresh dedicated
fixture-area directory BEFORE any filtering or failure raise. Reproduces the
two received counterexamples:

- exit 42 with a >2000-byte root-cause stderr and a meaningful stdout (the
  old driver kept only ``stderr[-2000:]`` inside the SystemExit text and no
  raw file at all);
- a non-UTF-8 child (``ORIGINAL\\xff\\r\\n`` on stdout, ``CAUSE\\xfe\\r\\n``
  on stderr, real exit 42): the old driver passed ``text=True`` to
  ``subprocess.run``, so decoding raised UnicodeDecodeError BEFORE any raw
  file or exit record existed — the announced integral retention was false.

This guard fails on any regression to either shape (truncated or missing
stderr, missing stdout, wrong/absent exit, decode-before-save, evidence
reused across attempts).

Method: a REAL child process, no CLI mocking that would hide the boundary —
the driver module is imported as-is (the code under test is the production
definition) and only its ``subprocess.run`` callable is wrapped: the call is
still executed as a real subprocess against a generated executable
``fake-cargo`` script, data-driven by a case file carrying the EXACT BYTES
of each channel (list of stdout byte-lines, raw stderr bytes, exit code)
that writes its bytes without truncation and exits with the case's real
code. The wrapper first asserts the driver still asks for the
recipe-authorized ``cargo test`` invocation (profile/args are the driver's
existing contract, asserted, never changed) and swaps only the ``cargo``
entry for the script path. No real cargo, no compilation, no ELF copy, no
live database, no provider, no network; the spec/binary fixture files are
tiny owned files, never executed. The driver's selection contract is kept
exactly as received (reason ``compiler-artifact`` + ``executable`` basename
prefix): no invented cargo schema extension (no ``successful`` field) and
no stale-artifact fixture.

Inputs: none (self-contained). Outputs: named stdout markers and per-case
assertions; the guard fixture roots are allocated under the repository
fixture area via ``fixture_area.fixture_root`` (never ``/tmp``) and retained
for the caller. Errors: any violated invariant is a hard unittest failure;
if the driver module itself cannot be imported the guard SKIPs loudly — and
a skip exits non-zero (as does any failure), so a recipe run can never read
a degraded guard as OK.

Limits: this guard exercises ``resolve_binary`` in isolation (the module-level
contract and its evidence side effect); it does not run the browser
qualification itself, does not pin cargo's output format beyond the
``compiler-artifact`` JSON shape the driver already depends on, and asserts
nothing about product policy — arguments and fixture names are test inputs,
not product settings.
"""
import importlib.util
import json
import sys
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


@unittest.skipIf(driver is None,
                 "driver under test could not be imported: " + repr(_driver_load_error))
class ResolutionEvidence(unittest.TestCase):
    """Byte-exact retention contract of ``resolve_binary`` on a real fake cargo."""

    def setUp(self):
        # The driver's evidence allocation is redirected into this guard's
        # fixture root (the original imported helper is restored on cleanup so
        # the module stays untouched for other importers). The fake target
        # tree mirrors the driver's path contract: deps = <target>/debug/deps,
        # CARGO_TARGET_DIR = <target>, workspace = <target>.parent.parent =
        # <fixture>, cargo cwd = <fixture>/rust.
        self._orig_area = driver.fixture_area
        self.fixture = Path(fixture_root("approval-resolution-guard-"))
        self.area = self.fixture / "resolution-evidence"
        self.area.mkdir()
        driver.fixture_area = lambda: str(self.area)
        self.addCleanup(setattr, driver, "fixture_area", self._orig_area)
        # target sits two levels under the guard root, like the real recipe
        # layout (<root>/.cache/<target-dir>): the driver derives
        # workspace = target_dir.parent.parent, so <fixture>/cache/target
        # puts the workspace exactly at <fixture> and the cargo cwd at
        # <fixture>/rust (created below).
        self.cache = self.fixture / "cache"
        self.target = self.cache / "target"
        self.deps = self.target / "debug" / "deps"
        self.deps.mkdir(parents=True)
        (self.fixture / "rust").mkdir()         # cargo cwd (workspace marker)
        self.cargo_bin = self.fixture / "fake-cargo"
        self.binary = self._file("paea_git_call-abc123", b"\x7fELF-fixture",
                                 executable=True)

    def _file(self, name, content, executable=False):
        path = self.deps / name
        path.write_bytes(content)
        if executable:
            path.chmod(0o755)
        return path

    def _spec(self):
        # Contract: spec = deps/<prefix>; the prefix names the test target.
        spec = self.deps / "paea_git_call"
        spec.write_bytes(b"")
        return spec

    def _script(self, case):
        """Generate the fake-cargo executable for one case (exact raw bytes).

        ``stdout_lines`` are raw byte strings (each \\n-terminated by the
        generator loop), ``stderr_bytes`` is written raw, and the process
        exits with the case's real ``exit`` code. The case data lives in a
        JSON sibling of the script (hex-encoded bytes, located via the
        script's own path because cargo's argv passes the subcommand first,
        not the case file) so no shell quoting can alter a single byte of
        the fixture channels.
        """
        case_path = self.fixture / "case.json"
        case_path.write_bytes(json.dumps({
            "stdout_hex": [line.hex() for line in case["stdout_lines"]],
            "stderr_hex": case["stderr_bytes"].hex(),
            "exit": case["exit"],
        }).encode())
        self.cargo_bin.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, sys\n"
            "case_path = os.path.join(os.path.dirname(os.path.abspath(__file__)),\n"
            "                         'case.json')\n"
            "case = json.load(open(case_path, 'rb'))\n"
            "out = sys.stdout.buffer\n"
            "for line in case['stdout_hex']:\n"
            "    out.write(bytes.fromhex(line) + b'\\n')\n"
            "out.flush()\n"
            "sys.stderr.buffer.write(bytes.fromhex(case['stderr_hex']))\n"
            "sys.stderr.buffer.flush()\n"
            "sys.exit(case['exit'])\n")
        self.cargo_bin.chmod(0o755)

    def _resolve(self, spec):
        """Run the real driver resolution with only the binary path swapped.

        The wrapper executes the SAME subprocess.run the driver calls — real
        child, real pipes, real exit — after asserting the driver still asks
        for the recipe-authorized ``cargo test`` invocation. Any drift of the
        resolution command away from ``cargo test`` fails here.
        """
        real_run = driver.subprocess.run

        def run_cargo_script(cmd, **kwargs):
            self.assertEqual(cmd[:2], ["cargo", "test"], cmd)
            cmd = [str(self.cargo_bin)] + list(cmd[1:])
            return real_run(cmd, **kwargs)

        with mock.patch.object(driver.subprocess, "run",
                               side_effect=run_cargo_script):
            return driver.resolve_binary(spec)

    def _evidence_snapshot(self):
        return set(self.area.iterdir())

    def _new_dirs(self, before):
        return sorted(p for p in self.area.iterdir()
                      if p.name.startswith("approval-resolution-evidence-")
                      and p not in before)

    def _artifact_message(self, executable):
        return json.dumps({"reason": "compiler-artifact",
                           "executable": str(executable)})

    def test_docstring_states_the_retention_promise(self):
        """The documented contract mentions the before-failure raw retention."""
        self.assertIn("before any filtering or failure", driver.__doc__)
        self.assertIn("BEFORE any filtering or failure",
                      driver.resolve_binary.__doc__)
        self.assertIn("fixture_area", driver.resolve_binary.__doc__)
        # Byte retention over decoding must be the documented contract.
        self.assertIn("BYTES", driver.resolve_binary.__doc__)

    def test_success_witness_keeps_integral_channels(self):
        """Valid witness: success resolves the emitted artifact, evidence integral.

        The selection stays exactly the received contract (the fresh
        ``compiler-artifact`` message naming the emitted executable); the
        evidence directory holds the integral channels and the real exit 0.
        """
        self._script({
            "stdout_lines": [self._artifact_message(self.binary).encode()],
            "stderr_bytes": b"   Compiling swarm-web v0.1.1\n    Finished test\n",
            "exit": 0,
        })
        before = self._evidence_snapshot()
        resolved = self._resolve(self._spec())
        self.assertEqual(resolved, self.binary)
        dirs = self._new_dirs(before)
        self.assertEqual(len(dirs), 1, dirs)
        evidence = dirs[0]
        argv = json.loads((evidence / "argv.json").read_text())
        self.assertEqual(argv["argv"][:2], ["cargo", "test"])
        self.assertEqual(argv["cwd"], str(self.fixture / "rust"))
        self.assertEqual(argv["exit"], 0)
        stdout = (evidence / "stdout.txt").read_bytes()
        self.assertIn(self.binary.name.encode(), stdout)
        self.assertTrue(stdout.endswith(b"\n"))
        self.assertEqual((evidence / "stderr.txt").read_bytes(),
                         b"   Compiling swarm-web v0.1.1\n    Finished test\n")

    def test_missing_artifact_refusal_keeps_evidence_and_path(self):
        """Named refusal: no matching artifact still writes complete evidence.

        The refusal message must carry the evidence path and the attempt must
        have its own integral stdout — a silent refusal without a raw sink is
        the counterexample family.
        """
        self._script({
            "stdout_lines": [self._artifact_message(self.binary).encode()],
            "stderr_bytes": b"    Finished test\n",
            "exit": 0,
        })
        before = self._evidence_snapshot()
        missing_spec = self.deps / "paea_git_call_moved"
        missing_spec.write_bytes(b"")
        with self.assertRaises(SystemExit) as ctx:
            self._resolve(missing_spec)
        text = str(ctx.exception)
        self.assertIn("no cargo compiler-artifact executable", text)
        self.assertIn("raw evidence", text)
        dirs = self._new_dirs(before)
        self.assertEqual(len(dirs), 1, dirs)
        self.assertIn(self.binary.name.encode(),
                      (dirs[0] / "stdout.txt").read_bytes())

    def test_exit42_keeps_integral_stdout_stderr_and_real_exit(self):
        """The received counterexample: exit 42, big root-cause stderr, stdout.

        Everything must be preserved byte-exact: integral stdout (not dropped),
        integral stderr (longer than any tail cut, starting with its sentinel),
        and the REAL child exit 42 in argv.json. A regression to
        ``stderr[-2000:]``-only or to no-raw-file fails this test.
        """
        stderr_body = b"ROOT_CAUSE_SENTINEL: dependency build failed\n" + b"x" * 6000
        self._script({
            "stdout_lines": [b"ARTIFACT_STDOUT_ORIGINAL",
                             self._artifact_message(self.binary).encode()],
            "stderr_bytes": stderr_body,
            "exit": 42,
        })
        before = self._evidence_snapshot()
        with self.assertRaises(SystemExit) as ctx:
            self._resolve(self._spec())
        text = str(ctx.exception)
        self.assertIn("exit 42", text)
        self.assertIn("raw evidence", text)
        self.assertIn("cargo artifact resolution failed", text)
        dirs = self._new_dirs(before)
        self.assertEqual(len(dirs), 1, dirs)
        evidence = dirs[0]
        argv = json.loads((evidence / "argv.json").read_text())
        self.assertEqual(argv["exit"], 42)
        stdout = (evidence / "stdout.txt").read_bytes()
        self.assertIn(b"ARTIFACT_STDOUT_ORIGINAL", stdout)
        stderr = (evidence / "stderr.txt").read_bytes()
        self.assertEqual(stderr, stderr_body)      # byte-exact, no tail cut
        self.assertGreater(len(stderr), 2000)

    def test_nonutf8_crlf_child_keeps_original_bytes_and_real_exit(self):
        """The non-UTF-8 counterexample: decode-before-save must never return.

        A real owned child writes ``ORIGINAL\\xff\\r\\n`` on stdout and
        ``CAUSE\\xfe\\r\\n`` on stderr and exits 42. The old ``text=True``
        driver died with UnicodeDecodeError leaving ``saved_files: []``; the
        fixed driver must save BOTH channels byte-exact (CRLF kept, 0xff/0xfe
        NOT replaced) with the real exit 42, and its refusal diagnosis must
        decode lossily (replace) without masking the evidence paths.
        """
        stdout_body = b"ORIGINAL\xff\r\n"
        stderr_body = b"CAUSE\xfe\r\n"
        self._script({
            "stdout_lines": [stdout_body[:-1]],   # loop adds the \n; keep \r\n shape
            "stderr_bytes": stderr_body,
            "exit": 42,
        })
        before = self._evidence_snapshot()
        with self.assertRaises(SystemExit) as ctx:
            self._resolve(self._spec())
        text = str(ctx.exception)
        self.assertIn("exit 42", text)
        self.assertIn("raw evidence", text)        # diagnosis still points at proof
        dirs = self._new_dirs(before)
        self.assertEqual(len(dirs), 1, dirs)
        evidence = dirs[0]
        argv = json.loads((evidence / "argv.json").read_text())
        self.assertEqual(argv["exit"], 42)
        self.assertEqual((evidence / "stdout.txt").read_bytes(), stdout_body)
        self.assertEqual((evidence / "stderr.txt").read_bytes(), stderr_body)

    def test_recovery_in_new_directory(self):
        """Recovery: a later valid attempt allocates its own fresh evidence.

        Two successive valid resolutions (simulating the retry an operator
        runs after fixing the cause) must each write their own complete
        evidence directory — same integral stderr both times, distinct dirs,
        no reuse, no overwrite. The first attempt's proof comes ONLY from
        sets frozen at the right moments (``mid - before``, where ``mid`` is
        captured after the first call): re-enumerating the live parent after
        the second call would see both attempts and lie about the first one
        (the first actual guard run failed exactly there, even with ``mid``
        captured, because ``_new_dirs(before)`` re-listed the parent after
        the second call).
        """
        case = {
            "stdout_lines": [self._artifact_message(self.binary).encode()],
            "stderr_bytes": b"    Finished test [unoptimized]\n",
            "exit": 0,
        }
        self._script(case)
        before = self._evidence_snapshot()
        spec = self._spec()
        first = self._resolve(spec)
        self.assertEqual(first, self.binary)
        mid = self._evidence_snapshot()    # frozen after attempt 1, before attempt 2
        first_dirs = sorted(mid - before)
        self.assertEqual(len(first_dirs), 1, first_dirs)
        self.assertTrue(first_dirs[0].name.startswith("approval-resolution-evidence-"))
        second = self._resolve(spec)
        self.assertEqual(second, self.binary)
        after = self._evidence_snapshot()  # frozen after attempt 2
        second_dirs = sorted(after - mid)
        self.assertEqual(len(second_dirs), 1, second_dirs)
        self.assertTrue(second_dirs[0].name.startswith("approval-resolution-evidence-"))
        self.assertNotEqual(first_dirs[0], second_dirs[0])
        stderr1 = (first_dirs[0] / "stderr.txt").read_bytes()
        stderr2 = (second_dirs[0] / "stderr.txt").read_bytes()
        self.assertTrue(stderr1)
        self.assertEqual(stderr1, stderr2)
        # Each attempt's argv.json records its own real child exit.
        self.assertEqual(json.loads((first_dirs[0] / "argv.json").read_text())["exit"], 0)
        self.assertEqual(json.loads((second_dirs[0] / "argv.json").read_text())["exit"], 0)


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
    print(f"APPROVAL_RESOLUTION_RAW_EVIDENCE_GUARD run={result.testsRun} "
          f"failed={failures} skipped={skipped}", flush=True)
    sys.exit(1 if failures or skipped else 0)
