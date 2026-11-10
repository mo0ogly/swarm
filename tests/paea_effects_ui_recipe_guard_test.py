#!/usr/bin/env python3
"""Output-ownership guards for the REAL wrapper tests/paea_effects_ui_recipe.py.

Regression reproducer (glm-pa09, 2026-11-02): with --output pointing at an
existing directory holding a caller failure.txt sentinel, the wrapper's
mkdir refused but its except handler OVERWROTE the preexisting file. Each
guard asserts the corrected contract on the real wrapper: a refused
precondition (existing --output, missing --output-parent) exits nonzero
with stderr only and leaves every preexisting byte and directory entry
unchanged; /tmp or below is refused for --output AND --output-parent
BEFORE any allocation, observed by intercepting Path.mkdir and
tempfile.mkdtemp (never by letting the wrapper write outside the fixture
area); the fresh --output success path creates and owns its wrapper dir
(result.json PASS, no failure.txt) with the driver child doubled — no
browser, no node execution. Fixtures come from tests/fixture_area.py only
(never /tmp, no TMPDIR reliance); subprocess refusals use timeout=None
explicitly (no watchdog), like the wrapper itself.
"""
import hashlib, io, json, os, subprocess, sys, unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests"))
import fixture_area
import paea_effects_ui_recipe as recipe

DUMMY_BINARY = ROOT / ".swarm-fixtures" / "paea-effects-recipe-guard-dummy-binary"


def _run(argv):
    """One real subprocess of the real wrapper; refusal cases must fail
    before allocation, so they never reach node and write nowhere outside
    the caller-owned fixture directory."""
    return subprocess.run([sys.executable, str(ROOT / "tests/paea_effects_ui_recipe.py")] + argv,
                          cwd=ROOT, capture_output=True, timeout=None)


class WrapperOwnershipGuards(unittest.TestCase):
    def test_refused_existing_output_preserves_preexisting_bytes(self):
        """The glm-pa09 reproducer: existing --output with a sentinel
        failure.txt must exit nonzero, print on stderr only, and leave the
        sentinel and every directory entry byte-for-byte unchanged."""
        root = Path(fixture_area.fixture_root("paea-effects-recipe-guard-"))
        out = root / "existing-output"
        out.mkdir(mode=0o700)
        sentinel = b"caller-owned failure.txt sentinel - must survive the refusal\n"
        (out / "failure.txt").write_bytes(sentinel)
        run = _run(["--binary", str(DUMMY_BINARY), "--output", str(out)])
        self.assertNotEqual(run.returncode, 0)
        self.assertEqual(run.stdout, b"")
        self.assertIn(b"FileExistsError", run.stderr)
        self.assertEqual((out / "failure.txt").read_bytes(), sentinel)
        self.assertEqual(sorted(p.name for p in out.iterdir()), ["failure.txt"])
        self.assertEqual(sorted(p.name for p in root.iterdir()), ["existing-output"])

    def test_refused_missing_parent_allocates_nothing(self):
        """A missing --output-parent is refused before any allocation: the
        absent parent stays absent, its existing parent keeps exactly its
        prior (empty) listing, exit nonzero with stderr only."""
        root = Path(fixture_area.fixture_root("paea-effects-recipe-guard-"))
        absent = root / "absent-parent"
        run = _run(["--binary", str(DUMMY_BINARY), "--output-parent", str(absent)])
        self.assertNotEqual(run.returncode, 0)
        self.assertEqual(run.stdout, b"")
        self.assertIn(b"NotADirectoryError", run.stderr)
        self.assertFalse(absent.exists())
        self.assertEqual(list(root.iterdir()), [])

    def test_tmp_refused_before_allocation_for_both_options(self):
        """/tmp and below are refused for --output AND --output-parent before
        any allocation: interception fails loudly on any mkdir/mkdtemp
        attempt, nothing is ever written to /tmp, and refuse_tmp itself
        rejects /tmp, /tmp/x and /tmp/a/b for both options while allowing a
        /tmp-prefixed sibling name and the filesystem root."""
        probe = "/tmp/paea-effects-recipe-guard-probe-%d" % os.getpid()
        attempts = []

        def refuse_allocation(kind):
            """Interception stub: record the attempt then raise, so a
            defective wrapper that allocates before refusing stops loudly
            and can never write anything under /tmp through this guard."""
            def boom(*_a, **_k):
                attempts.append(kind)
                raise AssertionError("allocation attempted before /tmp refusal: " + kind)
            return boom

        for flag in ("--output", "--output-parent"):
            with mock.patch.object(sys, "argv",
                                   ["paea_effects_ui_recipe.py", "--binary", str(DUMMY_BINARY), flag, probe]), \
                 mock.patch("pathlib.Path.mkdir", side_effect=refuse_allocation("mkdir")), \
                 mock.patch("tempfile.mkdtemp", side_effect=refuse_allocation("mkdtemp")), \
                 redirect_stdout(io.StringIO()) as captured:
                with self.assertRaises(ValueError) as refused:
                    recipe.main()
            self.assertIn(flag, str(refused.exception))
            self.assertIn("/tmp", str(refused.exception))
            self.assertEqual(captured.getvalue(), "")
        self.assertEqual(attempts, [])  # zero allocation attempts across both options
        self.assertFalse(Path(probe).exists())
        for bad in (Path("/tmp"), Path("/tmp") / "x", Path("/tmp") / "a" / "b"):
            for option in ("--output", "--output-parent"):
                with self.assertRaises(ValueError):
                    recipe.refuse_tmp(bad, option)
        recipe.refuse_tmp(Path("/tmpxyz"), "--output")   # a name prefixed by /tmp is not /tmp
        recipe.refuse_tmp(Path("/"), "--output-parent")  # the root itself is not refused

    def test_success_fresh_output_dir_is_owned(self):
        """Normal fresh --output path through the REAL wrapper with the
        driver child doubled (labeled double, no browser): the wrapper
        creates and owns its directory, prints the marker only after
        verify(), writes result.json PASS, never a failure.txt, and hands
        the driver the unchanged node invocation with a nonexistent browser
        subdir and exactly the 19 named checks."""
        root = Path(fixture_area.fixture_root("paea-effects-recipe-guard-"))
        binary = (root / "fake-swarm").resolve()
        binary.write_bytes(b"fake-managed-binary-bytes")
        out = (root / "fresh-output").resolve()  # must not exist: created by the wrapper
        digest = hashlib.sha256(binary.read_bytes()).hexdigest()
        dynamic = [prefix + "mid" + suffix for prefix, suffix in recipe.DYNAMIC]
        proof = {"kind": "paea-effects-ui-v1", "schema_version": 1,
                 "checks": list(recipe.REQUIRED) + dynamic,
                 "unexpected_diagnostics": [], "expected_denials": 8,
                 "observed_refusals": [{"status": 403}] * 8, "binary_sha256": digest}

        def fake_driver(cmd, capture_output, timeout):
            self.assertEqual(cmd[0], "node")
            self.assertEqual(cmd[1], "tests/paea_effects_ui.cjs")
            self.assertEqual(cmd[2], str(binary))
            self.assertEqual(cmd[3], str(out / "browser"))
            self.assertFalse(Path(cmd[3]).exists())  # the driver requires freshness
            Path(cmd[3]).mkdir(parents=True)
            (Path(cmd[3]) / "paea-effects-ui.json").write_text(json.dumps(proof))
            return subprocess.CompletedProcess(cmd, 0, b"", b"")

        with mock.patch.object(sys, "argv",
                               ["paea_effects_ui_recipe.py", "--binary", str(binary), "--output", str(out)]), \
             mock.patch("subprocess.run", side_effect=fake_driver), \
             redirect_stdout(io.StringIO()) as captured:
            recipe.main()
        self.assertIn(recipe.MARKER, captured.getvalue())
        self.assertEqual(sorted(p.name for p in out.iterdir()),
                         ["browser", "browser.stderr", "browser.stdout", "result.json"])
        self.assertEqual(json.loads((out / "result.json").read_text())["status"], "PASS")
        self.assertFalse((out / "failure.txt").exists())
        self.assertEqual(hashlib.sha256(binary.read_bytes()).hexdigest(), digest)


if __name__ == "__main__":
    unittest.main()
