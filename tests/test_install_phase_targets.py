"""Scheduling probe for the Makefile ``test-install`` aggregate (fake installers).

Usage::

    python3 tests/test_install_phase_targets.py --output /path/to/fresh/evidence

Scope, stated plainly: this suite proves only that the public aggregate
target ``test-install`` schedules the named phase targets sequentially
(native installer before Docker installer) under forced parallel make,
refuses to continue to the Docker phase when the native phase fails, and
recovers with a fresh successful run after the synthetic failure input is
restored. It does not qualify the real native installer, the real Docker
installer, real Python, or any GitHub Actions behavior (steps, UI,
retention, reruns, merges), and the workflow split is not validated here
because no GitHub run may be executed.

Mechanics and constraints:

* Only a byte-copy of the repository root ``Makefile`` is exercised,
  inside a fresh ``tempfile.TemporaryDirectory`` workspace per test; that
  single Makefile is the only repository file read (no broad package
  scans), and no actual installer scripts need to exist.
* ``python3`` is routed, via a PATH shim, to an inert ``/bin/sh`` script
  that appends its first argument to a dedicated log, accepts only the
  two installer script paths named by the source Makefile, and can be
  told (via a flag file) to simulate a native failure. It never executes
  real Python or any installer.
* The installed ``make`` (resolved via ``shutil.which``) is invoked with
  ``subprocess.run`` using an argv list -- never ``shell=True`` -- with
  no timeout watchdog and no retry.
* ``-j2`` is forced on every run to probe the aggregate's sequencing; it
  is a test-side probe and introduces no scheduling, timeout, or
  concurrency policy into product files.
* The caller must admit the explicit output parent, including mount identity,
  free space and any held-volume descriptor. This suite grants no storage
  admission, filesystem confinement or permission to run an unknown Makefile.
  Its source-name checks detect drift; they are not a full Make parser.
* Per case the suite retains the actual source Makefile's SHA-256 and a
  byte copy, the make argv, raw stdout/stderr bytes, the return status,
  and the shim's invocation log, under a ``tempfile``-isolated output
  tree created beneath the caller's required fresh ``--output`` directory. Assertions
  are behavioral -- the shim's invocation log plus exit statuses and
  captured channels -- never make's printed output labels alone.
* Any failed assertion yields a nonzero terminal exit (unittest's
  default exit-on-failure). The evidence tree is intentionally left on
  disk for inspection.

This suite does not duplicate the historical 6 + 8 installer test cases;
it adds only the aggregate-scheduling cases described above.
"""

import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

# --- Source-contract constants (must match the current repository Makefile) ---
MAKEFILE_BASENAME = "Makefile"
AGGREGATE_TARGET = "test-install"
PHASE_TARGETS = ("test-install-native", "test-install-docker")
NATIVE_INSTALLER_ARG = "tests/rust_native_install.py"
DOCKER_INSTALLER_ARG = "tests/rust_docker_install.py"
PHONY_LINE_PREFIX = ".PHONY:"
AGGREGATE_NATIVE_RECURSION = "$(MAKE) test-install-native"
AGGREGATE_DOCKER_RECURSION = "$(MAKE) test-install-docker"

# --- Scheduling-probe constants (test-side only; no policy added to product) ---
PARALLEL_MAKE_FLAG = "-j2"  # forces parallel make to probe aggregate sequencing

# --- Synthetic fixture constants (inert /bin/sh shim; never runs real Python) ---
FAKE_PYTHON3_SCRIPT = r"""#!/bin/sh
# Inert synthetic python3 used only by the install-phase scheduling tests.
# Contract: append argv[1] to $SWARM_FAKE_PYTHON_LOG; accept only the two
# installer script paths; optionally fail the native phase when the file
# $SWARM_FAKE_NATIVE_FAIL exists. Never executes real Python or installers.
printf '%s\n' "$1" >> "$SWARM_FAKE_PYTHON_LOG"
case "$1" in
  tests/rust_native_install.py|tests/rust_docker_install.py) ;;
  *)
    printf 'fake-python3: refusing unexpected first argument: %s\n' "$1" >&2
    exit 64
    ;;
esac
if [ -f "$SWARM_FAKE_NATIVE_FAIL" ] && [ "$1" = "tests/rust_native_install.py" ]; then
  printf 'fake-python3: simulated native installer failure\n' >&2
  exit 3
fi
exit 0
"""
FAKE_LOG_ENV = "SWARM_FAKE_PYTHON_LOG"
FAKE_NATIVE_FAIL_ENV = "SWARM_FAKE_NATIVE_FAIL"
FAKE_NATIVE_FAILURE_SENTINEL = "simulated native installer failure"

# --- Evidence constants (naming only; no retention policy on product files) ---
EVIDENCE_RUN_PREFIX = "swarm-install-phase-evidence-"
WORKSPACE_PREFIX = "install-phase-workspace-"
EVIDENCE_DIRNAME = "evidence"
RECORD_FILENAME = "record.json"
RUN_MANIFEST_FILENAME = "run-manifest.json"
MAKEFILE_COPY_FILENAME = "makefile.source"
STDOUT_FILENAME = "stdout.raw"
STDERR_FILENAME = "stderr.raw"
INSTALLER_LOG_COPY_FILENAME = "installer-args.log"
DECODE_ERRORS = "replace"
STRIPPED_ENV_KEYS = ("MAKEFLAGS", "MFLAGS", "GNUMAKEFLAGS", "MAKEFILES", "MAKE", "MAKEOVERRIDES", "MAKELEVEL")

# Set by the __main__ block when --output names an explicit evidence root.
_RUN_ROOT_OVERRIDE = None


def _split_output_option(argv):
    """Extract one ``--output <dir>`` / ``--output=<dir>`` option from argv.

    Purpose: let the caller name an explicit fresh evidence directory
    without disturbing unittest's own command-line arguments.
    Preconditions: ``argv`` is a sys.argv-shaped list of strings.
    Results: returns ``(output_value_or_None, remaining_argv)`` where
    ``remaining_argv`` preserves argv[0] and all non-option entries in
    order.
    Errors: SystemExit when ``--output`` lacks a non-empty value or is
    supplied more than once.
    Effects: none (pure).
    """
    output_value = None
    remaining = [argv[0]] if argv else []
    index = 1
    while index < len(argv):
        item = argv[index]
        if item == "--output":
            if index + 1 >= len(argv) or not argv[index + 1]:
                raise SystemExit("--output requires a non-empty directory argument")
            if output_value is not None:
                raise SystemExit("--output supplied more than once")
            output_value = argv[index + 1]
            index += 2
            continue
        if item.startswith("--output="):
            if output_value is not None:
                raise SystemExit("--output supplied more than once")
            output_value = item.split("=", 1)[1]
            if not output_value:
                raise SystemExit("--output requires a non-empty directory argument")
            index += 1
            continue
        remaining.append(item)
        index += 1
    return output_value, remaining


def _prepare_fresh_output_root(raw):
    """Materialize the caller-supplied explicit evidence root as fresh.

    Purpose: guarantee the run's evidence starts from nothing so retained
    artifacts cannot mix with older evidence.
    Preconditions: ``raw`` is None (no explicit output) or a filesystem
    path string.
    Results: returns a newly created private ``pathlib.Path``.
    Errors: SystemExit when no explicit output is supplied; FileExistsError
    for an existing path, including a symlink. Missing/unadmitted parents
    are not constructed and their original OSError propagates.
    Effects: creates only the new leaf directory, never a fallback;
    removes nothing.
    """
    if raw is None:
        raise SystemExit("explicit --output in an admitted parent is required")
    root = pathlib.Path(raw).expanduser().absolute()
    root.mkdir(mode=0o700)
    return root


def _source_contract_problems(text):
    """Validate one Makefile against the current target/installer names.

    Purpose: fail fast with actionable messages when the repository
    Makefile drifts from the names this suite (and the phase targets)
    rely on, instead of failing obscurely inside make.
    Preconditions: ``text`` is the decoded contents of exactly one
    Makefile -- the repository root one; no other file is read and no
    package or tree scan is performed.
    Results: returns a possibly-empty list of human-readable drift
    descriptions; an empty list means the contract holds.
    Errors: none raised; all problems are returned as data.
    Effects: none (pure).
    """
    problems = []
    lines = [line.strip() for line in text.splitlines()]
    for target in (AGGREGATE_TARGET, *PHASE_TARGETS):
        if not any(line == target + ":" for line in lines):
            problems.append(f"missing target label {target + ':'!r}")
    phony_lines = [line for line in lines if line.startswith(PHONY_LINE_PREFIX)]
    if not any(all(name in line for name in PHASE_TARGETS) for line in phony_lines):
        problems.append(
            "no .PHONY line lists both " + " and ".join(PHASE_TARGETS)
        )
    for argument in (NATIVE_INSTALLER_ARG, DOCKER_INSTALLER_ARG):
        if "python3 " + argument not in text:
            problems.append(f"missing installer invocation 'python3 {argument}'")
    for recursion in (AGGREGATE_NATIVE_RECURSION, AGGREGATE_DOCKER_RECURSION):
        if recursion not in text:
            problems.append(f"aggregate does not delegate via {recursion!r}")
    return problems


class InstallPhaseTargetsTest(unittest.TestCase):
    """Scheduling and failure-propagation probes for ``make test-install``.

    Purpose: verify the aggregate's sequential native-then-Docker order
    under forced parallel make, its refusal to run Docker after a native
    failure, and its recovery after the synthetic failure input is
    removed -- using only a copied Makefile and an inert ``python3`` shim.
    Preconditions: the repository root Makefile exists beside this test's
    parent directory and satisfies ``_source_contract_problems``; an
    installed ``make`` is on PATH (absence is a prerequisite error).
    Results: each test records per-case evidence (source hash and byte
    copy, make argv, raw stdout/stderr bytes, return status, shim
    invocation log) under the evidence tree and asserts the scheduling
    contract; any failure yields a nonzero terminal exit.
    Errors: AssertionError on contract drift or scheduling violations;
    AssertionError when no installed make exists; zero tests is not success.
    Effects: creates temporary workspaces and evidence files; leaves the
    evidence tree on disk intentionally; launches no real installers and
    introduces no product policies, timeouts, or concurrency settings.
    """

    @classmethod
    def setUpClass(cls):
        """Snapshot the source Makefile, validate the contract, open evidence.

        Purpose: bind every case to the exact repository Makefile bytes
        and reject drift before any make run.
        Preconditions: repository root is one level above this test file;
        ``--output`` may have supplied an explicit fresh evidence root via
        the ``__main__`` block.
        Results: sets class attributes ``source_path``, ``source_bytes``,
        ``source_sha256``, ``run_root``, ``evidence_dir``, and
        ``make_path``.
        Errors: AssertionError when the Makefile is missing, drifts from
        the source contract, explicit output is absent or make is unavailable.
        Effects: creates the evidence directory -- a fresh ``tempfile``
        tree beneath the required explicit admitted ``--output`` root,
        and leaves it on disk for inspection. No implicit local fallback.
        """
        repo_root = pathlib.Path(__file__).resolve().parent.parent
        cls.source_path = repo_root / MAKEFILE_BASENAME
        if not cls.source_path.is_file():
            raise AssertionError(f"repository Makefile not found at {cls.source_path}")
        cls.source_bytes = cls.source_path.read_bytes()
        cls.source_sha256 = hashlib.sha256(cls.source_bytes).hexdigest()
        problems = _source_contract_problems(
            cls.source_bytes.decode("utf-8", DECODE_ERRORS)
        )
        if problems:
            raise AssertionError(
                "Makefile source-contract drift (update these tests with the "
                "current names): " + "; ".join(problems)
            )
        if _RUN_ROOT_OVERRIDE is None:
            raise AssertionError("explicit admitted --output is required")
        cls.run_root = _RUN_ROOT_OVERRIDE
        print(f"install-phase evidence tree: {cls.run_root}")
        cls.evidence_dir = cls.run_root / EVIDENCE_DIRNAME
        cls.evidence_dir.mkdir(parents=True, exist_ok=True)
        cls.make_path = shutil.which("make")
        if cls.make_path is None:
            raise AssertionError(
                "make-required: no installed make on PATH; scheduling not probed"
            )

    @classmethod
    def tearDownClass(cls):
        """Write a manifest correlating evidence cases with the source hash.

        Purpose: make the retained evidence self-describing for later
        inspection.
        Preconditions: setUpClass completed (tearDownClass is not called
        when setUpClass itself failed).
        Results: writes ``run-manifest.json`` with the source path/hash
        and the recorded case directories.
        Errors: none expected; OSError propagates if the manifest cannot
        be written.
        Effects: one file inside the evidence tree.
        """
        if getattr(cls, "evidence_dir", None) is None:
            return
        cases = sorted(p.name for p in cls.evidence_dir.iterdir() if p.is_dir())
        manifest = {
            "makefile_source_path": str(cls.source_path),
            "makefile_source_sha256": cls.source_sha256,
            "evidence_tree": str(cls.run_root),
            "cases": cases,
        }
        (cls.evidence_dir / RUN_MANIFEST_FILENAME).write_text(
            json.dumps(manifest, indent=2, sort_keys=True), encoding="utf-8"
        )

    def setUp(self):
        """Build a fresh workspace: copied Makefile, shim, log, failure flag.

        Purpose: give each test an isolated, disposable fixture tree so
        cases cannot contaminate each other's shim log or flags.
        Preconditions: setUpClass attributes are populated.
        Results: sets ``workspace_path``, ``makefile_copy``, ``bin_dir``,
        ``fake_python3``, ``installer_log``, ``native_fail_flag`` and
        registers workspace cleanup.
        Errors: none expected; OSError propagates if the filesystem
        refuses fixture creation.
        Effects: creates files only inside a fresh TemporaryDirectory
        placed beneath the run root.
        """
        workspace = tempfile.TemporaryDirectory(
            prefix=WORKSPACE_PREFIX, dir=str(self.run_root)
        )
        self.addCleanup(workspace.cleanup)
        self.workspace_path = pathlib.Path(workspace.name)
        self.makefile_copy = self.workspace_path / MAKEFILE_BASENAME
        self.makefile_copy.write_bytes(self.source_bytes)
        self.bin_dir = self.workspace_path / "bin"
        self.bin_dir.mkdir()
        self.fake_python3 = self.bin_dir / "python3"
        self.fake_python3.write_text(FAKE_PYTHON3_SCRIPT, encoding="utf-8")
        self.fake_python3.chmod(0o755)
        self.installer_log = self.workspace_path / "installer-args.log"
        self.native_fail_flag = self.workspace_path / "native-failure.requested"

    def _shim_env(self):
        """Build the child environment that routes python3 to the shim.

        Purpose: make ``python3`` resolve to the inert shim, point the
        shim at this case's dedicated log and failure flag, and keep the
        probe's own ``-j2`` the effective scheduling.
        Preconditions: setUp created ``bin_dir``, ``installer_log`` and
        ``native_fail_flag``.
        Results: returns a minimal child environment containing only the
        shim path, standard executable search path and two fixture variables.
        Ambient Make flags, MAKEFILES, command overrides and credentials
        are not passed to this fake-installer child.
        Errors: none.
        Effects: none on the parent process environment.
        """
        env = {"PATH": str(self.bin_dir) + os.pathsep + os.defpath}
        env[FAKE_LOG_ENV] = str(self.installer_log)
        env[FAKE_NATIVE_FAIL_ENV] = str(self.native_fail_flag)
        return env

    def _run_aggregate(self, case_name):
        """Run one parallel-make probe of the aggregate and retain evidence.

        Purpose: execute one case against the copied Makefile with forced
        parallelism and persist raw observations before any assertion, so
        failed assertions still leave evidence.
        Preconditions: setUp fixture ready; ``case_name`` unique per run.
        Results: returns the completed ``subprocess.CompletedProcess``
        for the installed make invocation
        ``[make, '-j2', 'test-install']`` with cwd at the workspace.
        Errors: OSError propagates if make cannot be executed; per the
        test constraints no timeout watchdog and no retry are installed.
        Effects: writes ``record.json``, ``makefile.source``,
        ``stdout.raw``, ``stderr.raw`` and ``installer-args.log`` under
        the evidence case directory; never uses shell=True.
        """
        argv = [self.make_path, PARALLEL_MAKE_FLAG, AGGREGATE_TARGET]
        proc = subprocess.run(
            argv,
            cwd=str(self.workspace_path),
            env=self._shim_env(),
            capture_output=True,
        )
        self._write_case_evidence(case_name, argv, proc)
        return proc

    def _logged_args(self):
        """Read the shim's dedicated invocation log in append order.

        Purpose: provide the behavioral record of which installer
        arguments the aggregate actually invoked.
        Preconditions: setUp created ``installer_log`` (which may not
        exist yet if the shim never ran).
        Results: returns the list of logged first-argument strings;
        ``[]`` when the shim has not been invoked.
        Errors: none expected; undecodable bytes are replaced.
        Effects: none (read-only).
        """
        if not self.installer_log.exists():
            return []
        return self.installer_log.read_bytes().decode("utf-8", DECODE_ERRORS).splitlines()

    def _reset_installer_log(self):
        """Start a fresh shim log for a fresh run.

        Purpose: ensure the next case's ``exactly`` assertions speak only
        about that run's invocations.
        Preconditions: setUp created ``installer_log``.
        Results: the log file no longer exists afterwards; the shim
        recreates it on its next invocation.
        Errors: none expected.
        Effects: deletes only this case's log file.
        """
        if self.installer_log.exists():
            self.installer_log.unlink()

    def _write_case_evidence(self, case_name, argv, proc):
        """Persist one case's raw observations and the exact source used.

        Purpose: retain the actual source hash/copy, argv, raw channels,
        status, and shim log for the case, before assertions run.
        Preconditions: ``proc`` is a completed CompletedProcess returned
        by ``_run_aggregate``; ``evidence_dir`` exists.
        Results: creates the case evidence directory containing
        ``record.json``, ``makefile.source``, ``stdout.raw``,
        ``stderr.raw`` and ``installer-args.log`` (byte copies; an empty
        log copy when the shim was never invoked).
        Errors: OSError propagates if evidence cannot be written.
        Effects: writes files only inside the evidence tree.
        """
        case_dir = self.evidence_dir / case_name
        case_dir.mkdir()
        (case_dir / MAKEFILE_COPY_FILENAME).write_bytes(self.source_bytes)
        (case_dir / STDOUT_FILENAME).write_bytes(proc.stdout or b"")
        (case_dir / STDERR_FILENAME).write_bytes(proc.stderr or b"")
        log_bytes = (
            self.installer_log.read_bytes() if self.installer_log.exists() else b""
        )
        (case_dir / INSTALLER_LOG_COPY_FILENAME).write_bytes(log_bytes)
        record = {
            "case": case_name,
            "makefile_source_path": str(self.source_path),
            "makefile_source_sha256": self.source_sha256,
            "make_executable": str(self.make_path),
            "make_argv": list(argv),
            "make_cwd": str(self.workspace_path),
            "make_returncode": proc.returncode,
            "stdout_text": (proc.stdout or b"").decode("utf-8", DECODE_ERRORS),
            "stderr_text": (proc.stderr or b"").decode("utf-8", DECODE_ERRORS),
            "installer_args_logged": log_bytes.decode(
                "utf-8", DECODE_ERRORS
            ).splitlines(),
            "path_shim_dir": str(self.bin_dir),
            "stripped_env_keys": list(STRIPPED_ENV_KEYS),
        }
        (case_dir / RECORD_FILENAME).write_text(
            json.dumps(record, indent=2, sort_keys=True), encoding="utf-8"
        )

    def test_parallel_aggregate_installs_native_then_docker(self):
        """Sequential native-before-Docker success under forced parallel make.

        Purpose: verify exactly two distinct installer invocations in the
        contract order when nothing fails, using the shim's behavioral
        log rather than make's printed output labels.
        Preconditions: fresh fixture from setUp; contract-validated
        Makefile copy; no native-failure flag present.
        Results: make exits 0 and the shim log is exactly
        ``[tests/rust_native_install.py, tests/rust_docker_install.py]``.
        Errors: AssertionError on nonzero status or wrong count/order.
        Effects: one make run and one evidence case directory.
        """
        proc = self._run_aggregate("success-native-then-docker")
        self.assertEqual(
            proc.returncode,
            0,
            "aggregate must succeed with inert installers",
        )
        self.assertEqual(
            self._logged_args(),
            [NATIVE_INSTALLER_ARG, DOCKER_INSTALLER_ARG],
            "aggregate must invoke exactly the native then the Docker installer, in order",
        )

    def test_parallel_aggregate_refuses_docker_after_native_failure(self):
        """Refusal: native failure stops the aggregate before Docker.

        Purpose: with the shim told to fail the native phase, the
        aggregate must exit nonzero after exactly one (native)
        invocation -- evidenced behaviorally by the shim log, the nonzero
        status, and the failure sentinel in the captured channels, not by
        output labels.
        Preconditions: fresh fixture from setUp; a valid run succeeds on
        that fixture before the native-failure flag is written.
        Results: make exits nonzero; the shim log is exactly the native
        installer argument (no Docker entry); the sentinel appears in the
        captured stdout+stderr bytes.
        Errors: AssertionError on zero status, extra invocations, or a
        missing sentinel.
        Effects: a valid witness and one refusal run with separate retained cases.
        """
        witness = self._run_aggregate("failure-valid-before-injection")
        self.assertEqual(witness.returncode, 0)
        self.assertEqual(self._logged_args(), [NATIVE_INSTALLER_ARG, DOCKER_INSTALLER_ARG])
        self._reset_installer_log()
        self.native_fail_flag.write_text("1", encoding="utf-8")
        proc = self._run_aggregate("failure-native-refusal")
        self.assertNotEqual(
            proc.returncode, 0, "native failure must fail the aggregate"
        )
        self.assertEqual(
            self._logged_args(),
            [NATIVE_INSTALLER_ARG],
            "refusal must stop after the native invocation, with no Docker invocation",
        )
        combined = (proc.stdout or b"") + (proc.stderr or b"")
        self.assertIn(
            FAKE_NATIVE_FAILURE_SENTINEL.encode("utf-8"),
            combined,
            "captured channels must show the native shim's failure sentinel",
        )

    def test_parallel_aggregate_recovers_after_restoring_synthetic_input(self):
        """Recovery: fresh success after the synthetic failure input is restored.

        Purpose: show refusal is not sticky -- after a failing run,
        removing the synthetic failure flag and starting a fresh shim log
        yields a clean sequential native-then-Docker success.
        Preconditions: fresh fixture from setUp and valid witness first;
        failure flag then set and removed ("restored synthetic input");
        a fresh log is used for each run.
        Results: the injected run exits nonzero with exactly one native
        invocation; the restored run exits 0 with exactly
        ``[native, Docker]`` in the fresh log.
        Errors: AssertionError on any violated expectation.
        Effects: valid witness, refusal and recovery with three retained cases.
        """
        witness = self._run_aggregate("recovery-valid-before-injection")
        self.assertEqual(witness.returncode, 0)
        self.assertEqual(self._logged_args(), [NATIVE_INSTALLER_ARG, DOCKER_INSTALLER_ARG])
        self._reset_installer_log()
        self.native_fail_flag.write_text("1", encoding="utf-8")
        failing = self._run_aggregate("recovery-before-restore")
        self.assertNotEqual(
            failing.returncode, 0, "native failure must fail the first run"
        )
        self.assertEqual(
            self._logged_args(),
            [NATIVE_INSTALLER_ARG],
            "first run must invoke only the native installer",
        )
        self.native_fail_flag.unlink()  # restore the synthetic input
        self._reset_installer_log()     # fresh log for the fresh run
        recovered = self._run_aggregate("recovery-after-restore")
        self.assertEqual(
            recovered.returncode,
            0,
            "aggregate must recover once the synthetic failure input is restored",
        )
        self.assertEqual(
            self._logged_args(),
            [NATIVE_INSTALLER_ARG, DOCKER_INSTALLER_ARG],
            "recovered run must invoke exactly native then Docker, in order",
        )


    def test_ambient_makefiles_cannot_change_the_fixture(self):
        """Keep an ambient Make include out of the copied-Makefile probe.

        A valid witness precedes an inherited MAKEFILES input containing only
        a write to an owned temporary marker. The controlled child must still
        invoke exactly the fake native/Docker pair without loading that input.
        This tests ambient-configuration isolation, not hostile-Make confinement.
        """
        witness = self._run_aggregate("ambient-valid-before-injection")
        self.assertEqual(witness.returncode, 0)
        self.assertEqual(self._logged_args(), [NATIVE_INSTALLER_ARG, DOCKER_INSTALLER_ARG])
        self._reset_installer_log()
        marker = self.workspace_path / "ambient-loaded.marker"
        include = self.workspace_path / "ambient.mk"
        text = "$(file >" + str(marker) + ",AMBIENT_WAS_LOADED)\n"
        include.write_text(text, encoding="utf-8")
        (self.evidence_dir / "ambient-input.mk").write_text(text, encoding="utf-8")
        with mock.patch.dict(os.environ, {"MAKEFILES": str(include)}):
            result = self._run_aggregate("ambient-configuration-excluded")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(self._logged_args(), [NATIVE_INSTALLER_ARG, DOCKER_INSTALLER_ARG])
        self.assertFalse(marker.exists(), "ambient Make configuration was executed")


class InstallPhasePrerequisiteTest(unittest.TestCase):
    """Verify the public test CLI refuses a missing tool instead of empty success."""

    def test_missing_make_is_terminal_failure(self):
        """Run a valid selected fake-only case, then remove make from child PATH.

        Each child gets a fresh explicit evidence root and the same source.
        The negative child must exit nonzero with the prerequisite diagnostic,
        before any test body or installer. Both channels/argv/status are retained.
        No test deadline, actual installer or Docker process is introduced.
        """
        self.assertIsNotNone(_RUN_ROOT_OVERRIDE, "explicit admitted --output required")
        directory = _RUN_ROOT_OVERRIDE / "missing-make-cli"
        directory.mkdir()
        empty = directory / "empty-path"
        empty.mkdir()
        method = "InstallPhaseTargetsTest.test_parallel_aggregate_installs_native_then_docker"
        records = []
        for name, path in (("valid-tool-witness", os.defpath), ("missing-tool", str(empty))):
            argv = [sys.executable, str(pathlib.Path(__file__).resolve()),
                    "--output", str(directory / (name + "-evidence")), method]
            result = subprocess.run(argv, env={"PATH": path}, capture_output=True)
            (directory / (name + ".stdout.raw")).write_bytes(result.stdout)
            (directory / (name + ".stderr.raw")).write_bytes(result.stderr)
            records.append({"case": name, "argv": argv, "returncode": result.returncode})
            (directory / "record.json").write_text(json.dumps(records, indent=2), encoding="utf-8")
            if name == "valid-tool-witness":
                self.assertEqual(result.returncode, 0)
                self.assertIn(b"Ran 1 test", result.stderr)
                self.assertNotIn(b"skipped", result.stderr)
            else:
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(b"make-required", result.stderr)
                self.assertIn(b"Ran 0 tests", result.stderr)


if __name__ == "__main__":
    _OUTPUT_VALUE, _REMAINING_ARGV = _split_output_option(sys.argv)
    _OUTPUT_ROOT = _prepare_fresh_output_root(_OUTPUT_VALUE)
    if _OUTPUT_ROOT is not None:
        _RUN_ROOT_OVERRIDE = pathlib.Path(
            tempfile.mkdtemp(prefix=EVIDENCE_RUN_PREFIX, dir=str(_OUTPUT_ROOT))
        )
    sys.argv = _REMAINING_ARGV
    unittest.main(verbosity=2)
