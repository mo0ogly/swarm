import hashlib
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from bench import config, harness, run_b

N = config.INVOICE_COUNT


class RunBTest(unittest.TestCase):
    def run_ok(self, condition, key, fault):
        r = run_b.run(condition, key, fault, seed=11)
        self.addCleanup(shutil.rmtree, r["run_dir"], ignore_errors=True)   # run() garde ses preuves, pas les tests
        self.assertEqual(r["status"], "OK", r)
        return r

    def test_negative_control_everything_correct(self):
        for condition in ("B0", "B1"):
            for key in ("none", "attempt", "business"):
                r = self.run_ok(condition, key, "none")
                self.assertTrue(r["correct"] and r["declared_success"], r)
                self.assertEqual(r["final_review"], "conforme")

    def test_canaries_produce_the_expected_defect(self):
        cases = [("F1", "none", "doubles", 1), ("F2", "attempt", "doubles", N), ("F3", "attempt", "doubles", 1),
                 ("F4", "business", "wrong", 1), ("F6", "business", "unpaid", N - N // 2),
                 ("F7", "none", "doubles", N), ("F8", "business", "wrong", 1)]
        for fault, key, metric, expected in cases:
            with self.subTest(fault=fault):
                r = self.run_ok("B0", key, fault)
                self.assertEqual(r["metrics"][metric], expected, r)

    def test_business_key_alone_absorbs_repetition(self):
        for fault in ("F1", "F2", "F3", "F7"):
            with self.subTest(fault=fault):
                self.assertEqual(self.run_ok("B0", "business", fault)["metrics"]["doubles"], 0)

    def test_effect_happens_before_final_review(self):
        r = self.run_ok("B0", "business", "F4")
        self.assertTrue(r["declared_success"] and r["false_success"])
        self.assertEqual(r["final_review"], "anomalie")

    def test_b1_misses_amount_but_not_beneficiary(self):
        self.assertEqual(self.run_ok("B1", "business", "F4")["metrics"]["wrong"], 1)

    def test_environment_fault_reaches_final_review(self):
        r = self.run_ok("B0", "business", "F5")
        self.assertIn("snapshot-503", r["markers"])
        self.assertTrue(r["correct"])
        self.assertEqual(r["final_review"], "indisponible")   # F5 persistante : la relance unique échoue aussi

    def test_f4e_is_injected_like_f4_without_engine(self):
        r = self.run_ok("B0", "business", "F4e")
        self.assertIn("lot-tampered-early", r["markers"])
        self.assertEqual(r["metrics"]["wrong"], 1)
        self.assertTrue(r["declared_success"] and r["false_success"])

    def run_error(self, condition, key, fault):
        """L'exécution doit échouer en BenchError, sans résultat ; la racine conservée est nettoyée ici."""
        with self.assertRaises(harness.BenchError) as caught:
            run_b.run(condition, key, fault, seed=11)
        run_dir = Path(caught.exception.run_dir)
        self.addCleanup(shutil.rmtree, run_dir, ignore_errors=True)
        self.assertTrue(run_dir.name.startswith(f"banc-{condition.lower()}-"), run_dir)
        return caught.exception

    def test_failed_preparation_is_an_error_not_a_result(self):
        with mock.patch.object(config, "B_PREPARE_TIMEOUT_S", 0.01):
            e = self.run_error("B0", "business", "none")
        self.assertIn("préparation", str(e))

    def test_settle_crash_is_an_error_not_a_refusal(self):
        scripts = Path(tempfile.mkdtemp(prefix="banc-script-"))
        self.addCleanup(shutil.rmtree, scripts)
        broken = scripts / "settle.py"
        broken.write_text("raise RuntimeError('règlement en panne')\n")
        with mock.patch.dict(harness.SCRIPTS, {"settle": str(broken)}):
            e = self.run_error("B0", "business", "none")
        self.assertIn("règlement en panne", str(e))

    def test_review_crash_is_an_error_not_an_anomaly(self):
        """Un plantage Python sort en 1, comme « non conforme » : sans mesure publiée, ce n'est pas un verdict."""
        scripts = Path(tempfile.mkdtemp(prefix="banc-script-"))
        self.addCleanup(shutil.rmtree, scripts)
        broken = scripts / "verify_settlement.py"
        broken.write_text("raise RuntimeError('revue en panne')\n")
        with mock.patch.dict(harness.SCRIPTS, {"verify_settlement": str(broken)}):
            e = self.run_error("B0", "business", "none")
        self.assertIn("revue en panne", str(e))

    def test_settle_code_2_without_outcome_is_an_error(self):
        """argparse sort aussi en 2 : sans résultat publié, ce n'est pas un refus."""
        scripts = Path(tempfile.mkdtemp(prefix="banc-script-"))
        self.addCleanup(shutil.rmtree, scripts)
        fake = scripts / "settle.py"
        fake.write_text("import sys\nprint('usage : arguments refusés', file=sys.stderr)\nsys.exit(2)\n")
        with mock.patch.dict(harness.SCRIPTS, {"settle": str(fake)}):
            e = self.run_error("B0", "business", "none")
        self.assertIn("arguments refusés", str(e))

    def test_unexpected_exception_carries_run_dir(self):
        with mock.patch.object(harness, "init_ledger", side_effect=OSError("disque plein")):
            e = self.run_error("B1", "none", "none")
        self.assertIsInstance(e.__cause__, OSError)

    def test_harness_markers_attest_the_lots(self):
        def sha(r, name):
            return hashlib.sha256((Path(r["run_dir"]) / f"lot-{name}.json").read_bytes()).hexdigest()
        r = self.run_ok("B0", "business", "F2")
        self.assertEqual(r["markers"]["dual-launch"]["lot_sha256"], {"a1": sha(r, "a1"), "a2": sha(r, "a2")})
        r = self.run_ok("B0", "business", "F7")
        self.assertEqual(r["markers"]["owner-stalled"]["lot_sha256"], sha(r, "a1"))
        r = self.run_ok("B0", "business", "F8")
        marker = r["markers"]["stale-report"]
        self.assertEqual(marker["lot_sha256"], sha(r, "a1"))
        self.assertEqual(marker["amount_delta_cents"], config.TAMPER_DELTA_CENTS)

    def test_settle_timeout_gives_delay_status(self):
        with mock.patch.object(config, "B_SETTLE_TIMEOUT_S", 0.01):
            r = run_b.run("B0", "business", "none", seed=11)
        self.addCleanup(shutil.rmtree, r["run_dir"], ignore_errors=True)
        self.assertEqual((r["status"], r["declared_success"], r["final_review"]), ("DÉLAI", False, None))

    def test_partial_lot_is_a_false_success(self):
        r = self.run_ok("B0", "business", "F6")
        self.assertTrue(r["declared_success"] and r["false_success"], r)
        self.assertEqual(r["final_review"], "anomalie")

    def test_b1_does_not_stop_repetition(self):
        self.assertEqual(self.run_ok("B1", "none", "F1")["metrics"]["doubles"], 1)
