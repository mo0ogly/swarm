"""Campagne de vérification des correctifs : grille, variantes, règles de verdict (sans moteur)."""
import unittest
import copy
import json
import tempfile
from pathlib import Path
from contextlib import redirect_stdout, redirect_stderr
from io import StringIO

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
        self.assertEqual(tables_verif.verdicts(runs, [])["D5"][0], "CRITÈRE NON TENU")

    def test_d6_needs_positive_control(self):
        runs = [run(variant="nodeps", settle_started_before_prepare_accepted=False)]
        self.assertEqual(tables_verif.verdicts(runs, [])["D6"][0], "CRITÈRE NON TENU")
        runs.append(run(variant="noguard", status="ERREUR", correct=False, error="code 5"))
        self.assertEqual(tables_verif.verdicts(runs, [])["D6"][0], "critère tenu")

    def test_d3_requires_single_attempt_and_environment_receipt(self):
        two = run("F5", correct=False, prepare_attempts=2, environment_failure=True)
        self.assertEqual(tables_verif.verdicts([two], [])["D3"][0], "CRITÈRE NON TENU")


class ArchivedInputTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        root = Path(__file__).resolve().parents[3] / "docs/benchmarks/billing/resultats"
        cls.datasets = []
        for name in ("verification-20261007", "campagne-20261005"):
            records = [json.loads(line) for line in (root / (name + ".jsonl")).read_text().splitlines() if line.strip()]
            cls.datasets.append([r for r in records if r.get("kind") == "campaign" or r.get("seed") == 1000])

    def check(self, records, historical=False):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "data.jsonl"
            path.write_text("\n".join(json.dumps(r) for r in records))
            return tables_verif.checked_load(path, historical=historical, reps=1)

    def test_complete_archives_pass(self):
        self.assertEqual(len(self.check(self.datasets[0])), 17)
        self.assertEqual(len(self.check(self.datasets[1], True)), 90)

    def test_missing_case_and_entire_cell_fail(self):
        for historical in (False, True):
            with self.assertRaisesRegex(ValueError, "incomplète"):
                self.check(self.datasets[historical][:1] + self.datasets[historical][2:], historical)

    def test_duplicate_seed_and_unexpected_seed_fail(self):
        rows = copy.deepcopy(self.datasets[0])
        with self.assertRaisesRegex(ValueError, "dupliquée"):
            self.check(rows[:-1] + [rows[1], rows[-1]])
        rows[1]["seed"] = 999
        with self.assertRaisesRegex(ValueError, "inattendue"):
            self.check(rows)

    def test_missing_end_fails(self):
        with self.assertRaisesRegex(ValueError, "début et fin"):
            self.check(self.datasets[0][:-1])

    def test_hash_and_row_provenance_mismatch_fail(self):
        for mutate in (lambda r: r[-1].update(swarm_sha256="0" * 64),
                       lambda r: r[1]["provenance"].update(bench_sha256="0" * 64),
                       lambda r: r[-1].update(bench_changed=True)):
            rows = copy.deepcopy(self.datasets[0])
            mutate(rows)
            with self.assertRaises(ValueError):
                self.check(rows)

    def test_missing_order_proof_fails(self):
        rows = copy.deepcopy(self.datasets[0])
        row = next(r for r in rows if r.get("variant") == "nodeps")
        del row["verification"]["settle_started_before_prepare_accepted"]
        with self.assertRaisesRegex(ValueError, "preuves"):
            self.check(rows)

    def test_unknown_status_and_invalid_duration_fail(self):
        for field, value in (("status", "SUCCESS"), ("duration_s", float("nan"))):
            rows = copy.deepcopy(self.datasets[0])
            rows[1][field] = value
            with self.assertRaises(ValueError):
                self.check(rows)

    def test_nested_evidence_must_be_objects(self):
        for field in ("provenance", "metrics", "verification"):
            rows = copy.deepcopy(self.datasets[0])
            rows[1][field] = ["invalid"]
            with self.assertRaisesRegex(ValueError, "structurée"):
                self.check(rows)

    def test_complete_but_failed_criterion_exits_nonzero(self):
        rows = copy.deepcopy(self.datasets[0])
        next(r for r in rows if r.get("fault") == "F5")["verification"]["prepare_attempts"] = 2
        with tempfile.TemporaryDirectory() as directory:
            paths = [Path(directory) / name for name in ("v.jsonl", "h.jsonl")]
            for path, data in zip(paths, (rows, self.datasets[1])):
                path.write_text("\n".join(json.dumps(r) for r in data))
            with redirect_stdout(StringIO()), redirect_stderr(StringIO()):
                self.assertEqual(tables_verif.main([*[str(p) for p in paths], "--reps", "1"]), 1)

    def test_cli_failure_is_nonzero_and_has_no_verdict(self):
        with tempfile.TemporaryDirectory() as directory:
            paths = [Path(directory) / name for name in ("v.jsonl", "h.jsonl")]
            for path, rows in zip(paths, self.datasets):
                path.write_text("\n".join(json.dumps(r) for r in rows))
            stdout, stderr = StringIO(), StringIO()
            with redirect_stdout(stdout), redirect_stderr(stderr):
                self.assertEqual(tables_verif.main([str(p) for p in paths]), 2)
            self.assertEqual(stdout.getvalue(), "")
            self.assertIn("NON VALIDÉES", stderr.getvalue())

    def test_non_regression_does_not_accept_empty_inputs(self):
        self.assertEqual(tables_verif.verdicts([], [])["Non-régression"][0], "ÉCART")


if __name__ == "__main__":
    unittest.main()
