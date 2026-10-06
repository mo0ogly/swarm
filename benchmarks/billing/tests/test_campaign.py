"""Campagne : grille, reprise, lignes ERREUR, parallélisme par condition, empreinte du banc, interruption."""
import json
import tempfile
import threading
import time
import unittest
from pathlib import Path
from unittest import mock

from bench import campaign, harness


def record(condition, key, fault, seed, status="OK"):
    return {"condition": condition, "key_mode": key, "fault": fault, "seed": seed, "status": status}


class CampaignTest(unittest.TestCase):
    def setUp(self):
        self.dir = Path(tempfile.mkdtemp(prefix="banc-campagne-"))
        self.addCleanup(lambda: __import__("shutil").rmtree(self.dir, ignore_errors=True))
        self.out = self.dir / "c.jsonl"

    def lines(self):
        return [json.loads(line) for line in self.out.read_text().splitlines()]

    def runs(self):
        return [r for r in self.lines() if r.get("kind") != "campaign"]

    def main(self, *args, one=None):
        with mock.patch.object(campaign, "one", side_effect=one or (lambda c, k, f, s: record(c, k, f, s))):
            return campaign.main(["--out", str(self.out), *args])

    def test_grid_includes_f4e(self):
        grid = campaign.plan(["B0", "B1", "S"], list(harness.KEY_MODES), list(harness.FAULTS), reps=2, seed_base=1000)
        self.assertEqual(len(grid), 3 * 3 * 10 * 2)
        self.assertIn(("S", "business", "F4e", 1001), grid)

    def test_resume_skips_cells_already_present_with_same_seed(self):
        self.out.write_text(json.dumps(record("B0", "none", "F1", 1000)) + "\n")
        self.main("--reps", "1", "--conditions", "B0", "--keys", "none", "--faults", "F1,F2")
        runs = self.runs()
        self.assertEqual(sorted((r["fault"], r["seed"]) for r in runs), [("F1", 1000), ("F2", 1000)])

    def test_error_lines_are_kept_by_resume(self):
        """Une ERREUR est un résultat publié : la reprise ne la rejoue pas (6.7)."""
        self.out.write_text(json.dumps(record("B0", "none", "F1", 1000, "ERREUR")) + "\n")
        self.main("--reps", "1", "--conditions", "B0", "--keys", "none", "--faults", "F1")
        self.assertEqual(len(self.runs()), 1)

    def test_bench_error_becomes_error_line_with_run_dir(self):
        with mock.patch.object(campaign.run_b, "run", side_effect=harness.BenchError("API morte", "/tmp/banc-x")):
            r = campaign.one("B0", "none", "F1", 1000)
        self.assertEqual((r["status"], r["run_dir"]), ("ERREUR", "/tmp/banc-x"))
        self.assertIn("API morte", r["error"])

    def test_unexpected_exception_becomes_error_line_without_run_dir(self):
        with mock.patch.object(campaign.run_s, "run", side_effect=ValueError("inattendu")):
            r = campaign.one("S", "none", "F1", 1000)
        self.assertEqual((r["status"], r["run_dir"]), ("ERREUR", None))

    def test_conditions_run_in_sequence_with_their_own_parallelism(self):
        lock, active, peak, order = threading.Lock(), {"B": 0, "S": 0}, {"B": 0, "S": 0}, []

        def one(c, k, f, s):
            group = "S" if c == "S" else "B"
            with lock:
                active[group] += 1
                peak[group] = max(peak[group], active[group])
                order.append(group)
                self.assertEqual(active["B"] * active["S"], 0, "B et S ne doivent pas tourner ensemble")
            time.sleep(0.02)
            with lock:
                active[group] -= 1
            return record(c, k, f, s)

        self.main("--reps", "1", "--keys", "none,business", "--faults", "F1,F2,F3", "--jobs-b", "4",
                  "--jobs-s", "1", one=one)
        self.assertEqual(peak["S"], 1)
        self.assertGreater(peak["B"], 1)
        self.assertEqual(order, sorted(order))   # tout B avant S

    def test_bench_fingerprint_recorded_and_change_warned(self):
        digests = iter(["a" * 64, "b" * 64])
        harness.provenance.cache_clear()   # empreinte de début : provenance(), mise en cache au premier appel
        self.addCleanup(harness.provenance.cache_clear)
        with mock.patch.object(campaign.harness, "bench_digest", side_effect=lambda d: next(digests)):
            self.main("--reps", "1", "--conditions", "B0", "--keys", "none", "--faults", "F1")
        start, end = [r for r in self.lines() if r.get("kind") == "campaign"]
        self.assertEqual((start["event"], start["bench_sha256"]), ("start", "a" * 64))
        self.assertEqual((end["event"], end["bench_sha256"], end["bench_changed"]), ("end", "b" * 64, True))
        self.assertIn("warning", end)

    def test_unchanged_fingerprint_has_no_warning(self):
        self.main("--reps", "1", "--conditions", "B0", "--keys", "none", "--faults", "F1")
        end = [r for r in self.lines() if r.get("kind") == "campaign"][-1]
        self.assertFalse(end["bench_changed"])
        self.assertNotIn("warning", end)

    def purge_case(self, status, *options):
        root = self.dir / f"banc-{status.lower()}"
        (root / "faults").mkdir(parents=True)
        one = lambda c, k, f, s: {**record(c, k, f, s, status), "run_dir": str(root)}
        self.main("--reps", "1", "--conditions", "B0", "--keys", "none", "--faults", "F1", *options, one=one)
        return root, self.runs()[0]

    def test_purge_ok_removes_ok_root_and_says_so(self):
        root, line = self.purge_case("OK", "--purge-ok")
        self.assertFalse(root.exists())
        self.assertEqual((line["run_dir"], line["run_dir_purged"]), (str(root), True))   # run_dir reste exact

    def test_purge_ok_keeps_non_ok_roots_for_diagnosis(self):
        for status in ("ERREUR", "INVALIDE", "DÉLAI"):
            with self.subTest(status=status):
                self.out.unlink(missing_ok=True)
                root, line = self.purge_case(status, "--purge-ok")
                self.assertTrue(root.exists())
                self.assertNotIn("run_dir_purged", line)

    def test_without_option_nothing_is_removed(self):
        root, line = self.purge_case("OK")
        self.assertTrue(root.exists())
        self.assertNotIn("run_dir_purged", line)

    def test_interruption_leaves_complete_lines(self):
        calls = []

        def one(c, k, f, s):
            calls.append(f)
            if len(calls) == 2:
                raise KeyboardInterrupt
            return record(c, k, f, s)

        code = self.main("--reps", "1", "--conditions", "S", "--keys", "none", "--faults", "F1,F2,F3,F4",
                         "--jobs-s", "1", one=one)
        self.assertEqual(code, 130)
        lines = self.lines()   # chaque ligne se relit : aucune ligne partielle
        self.assertEqual(len(self.runs()), 1)
        self.assertEqual(lines[-1]["event"], "interrupted")
        self.assertEqual(len(calls), 2)   # aucune nouvelle case après l'interruption


    def test_error_during_interruption_is_discarded_and_left_to_rerun(self):
        # Ctrl-C atteint aussi les sous-processus : une ERREUR née de l'interruption n'est pas un résultat
        gate = threading.Event()

        def one(c, k, f, s):
            if f == "F1":
                gate.wait(5)
                time.sleep(0.3)   # finir après l'interruption de F2, jamais avant : sinon publication normale
                return record(c, k, f, s, "ERREUR")
            gate.set()
            raise KeyboardInterrupt

        code = self.main("--reps", "1", "--conditions", "S", "--keys", "none", "--faults", "F1,F2",
                         "--jobs-s", "2", one=one)
        self.assertEqual(code, 130)
        self.assertEqual(self.runs(), [])
        events = [l.get("event") for l in self.lines() if l.get("kind") == "campaign"]
        self.assertIn("discarded-after-interrupt", events)
        self.assertNotIn(("S", "none", "F1", 1000), campaign.already_done(self.out))

if __name__ == "__main__":
    unittest.main()
