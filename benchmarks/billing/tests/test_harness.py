import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from bench import harness, markers

SEED = 7


class ProvenanceTest(unittest.TestCase):
    def setUp(self):
        harness.provenance.cache_clear()
        self.addCleanup(harness.provenance.cache_clear)

    def test_computed_once_per_process(self):
        first = harness.provenance()
        self.assertIs(harness.provenance(), first)
        self.assertRegex(first["bench_sha256"], r"^[0-9a-f]{64}$")
        self.assertIn("swarm_sha256", first)

    def test_bench_digest_follows_the_code(self):
        copy = Path(tempfile.mkdtemp(prefix="banc-digest-")) / "bench"
        self.addCleanup(shutil.rmtree, copy.parent)
        shutil.copytree(harness.BENCH_DIR, copy, ignore=shutil.ignore_patterns("__pycache__"))
        self.assertEqual(harness.bench_digest(copy), harness.bench_digest(harness.BENCH_DIR))
        with (copy / "config.py").open("a") as f:
            f.write("\n# modification\n")
        self.assertNotEqual(harness.bench_digest(copy), harness.bench_digest(harness.BENCH_DIR))

    def test_git_runs_without_optional_locks(self):
        done = subprocess.CompletedProcess([], 0, stdout="abc\n", stderr="")
        with mock.patch.object(harness.subprocess, "run", return_value=done) as run:
            record = harness.provenance()
        self.assertEqual(run.call_count, 2)
        for call in run.call_args_list:
            self.assertEqual(call.kwargs["env"]["GIT_OPTIONAL_LOCKS"], "0")
        self.assertEqual((record["git_commit"], record["git_dirty"]), ("abc", True))

    def test_record_holds_a_copy_of_provenance(self):
        run_dir = harness.new_run_dir("banc-finish-")
        self.addCleanup(shutil.rmtree, run_dir)
        harness.init_ledger(run_dir / "ledger.db", SEED)
        r = harness.finish(run_dir, condition="B0", key_mode="none", fault="none", seed=SEED, started=0.0,
                           declared_success=False, timed_out=False, extra={})
        r["provenance"]["git_commit"] = "altéré"
        self.assertNotEqual(harness.provenance()["git_commit"], "altéré")


class SnapshotFaultReportTest(unittest.TestCase):
    def finish(self, recovered):
        run_dir = harness.new_run_dir("banc-f5-")
        self.addCleanup(shutil.rmtree, run_dir)
        harness.init_ledger(run_dir / "ledger.db", SEED)
        markers.mark(run_dir / "faults" / "snapshot-503.json", fault="snapshot-503", served=4)
        if recovered:
            markers.mark(run_dir / "faults" / "snapshot-recovered.json", fault="snapshot-recovered")
        return harness.finish(run_dir, condition="B0", key_mode="none", fault="F5", seed=SEED, started=0.0,
                              declared_success=False, timed_out=False, extra={})

    def test_reports_503_served_and_not_absorbed(self):
        r = self.finish(recovered=False)
        self.assertEqual((r["status"], r["f5_503_served"], r["f5_absorbed"]), ("OK", 4, False))

    def test_absorbed_fault_is_marked_not_excluded(self):
        r = self.finish(recovered=True)
        self.assertEqual((r["status"], r["f5_absorbed"]), ("OK", True))


class FaultsTest(unittest.TestCase):
    def test_f4e_declared_with_its_marker(self):
        self.assertIn("F4e", harness.FAULTS)
        self.assertEqual(harness.EXPECTED_MARKER["F4e"], "lot-tampered-early")
