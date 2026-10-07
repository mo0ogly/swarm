"""Campagne de vérification des correctifs : grille, variantes, règles de verdict (sans moteur)."""
import unittest

from bench import campaign_verif, run_s, tables_verif


def run(fault="none", key="business", variant=None, status="OK", correct=True, **verif):
    return {"condition": "S", "key_mode": key, "fault": fault, "variant": variant, "seed": 1000, "status": status,
            "correct": correct, "false_success": False, "stopped_by": verif.pop("stopped_by", None),
            "metrics": {"doubles": 0, "wrong": 0, "unpaid": 0 if correct else 12}, "duration_s": 1.0,
            "error": verif.pop("error", None), "verification": verif}


class PlanTest(unittest.TestCase):
    def test_grid_size_and_blocks(self):
        cases = campaign_verif.plan(100, 1000)
        self.assertEqual(len(cases), 1700)
        self.assertIn(("business", "none", "noguard", 1099), cases)
        self.assertIn(("none", "F4e", None, 1000), cases)

    def test_variant_only_without_fault(self):
        with self.assertRaises(ValueError):
            run_s.run("business", "F1", 1000, variant="nodeps")
        with self.assertRaises(ValueError):
            run_s.run("business", "none", 1000, variant="inconnue")


class VerdictTest(unittest.TestCase):
    def test_d5_fails_on_one_busy_exclusion(self):
        runs = [run("F4e", status="ERREUR", error="database is locked (5) (SQLITE_BUSY)")]
        self.assertEqual(tables_verif.verdicts(runs, [])["D5"][0], "NON CORRIGÉ")

    def test_d6_needs_positive_control(self):
        runs = [run(variant="nodeps", settle_started_before_prepare_accepted=False)]
        self.assertEqual(tables_verif.verdicts(runs, [])["D6"][0], "NON CORRIGÉ")
        runs.append(run(variant="noguard", status="ERREUR", correct=False, error="code 5"))
        self.assertEqual(tables_verif.verdicts(runs, [])["D6"][0], "corrigé")

    def test_d3_requires_single_attempt_and_environment_receipt(self):
        two = run("F5", correct=False, prepare_attempts=2, environment_failure=True)
        self.assertEqual(tables_verif.verdicts([two], [])["D3"][0], "NON CORRIGÉ")


if __name__ == "__main__":
    unittest.main()
