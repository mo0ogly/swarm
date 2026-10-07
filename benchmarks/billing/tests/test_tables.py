"""Tableaux de l'article : Wilson, exclusions, signalement des 10 %, durées, tableaux annexes."""
import json
import tempfile
import unittest
from pathlib import Path

from bench import tables


def run(status="OK", doubles=0, wrong=0, unpaid=0, false_success=False, duration=1.0, fault="F1",
        condition="B0", key="none", **extra):
    record = {"condition": condition, "key_mode": key, "fault": fault, "seed": 1000, "status": status,
              "false_success": false_success, "duration_s": duration,
              "metrics": {"doubles": doubles, "wrong": wrong, "unpaid": unpaid}, **extra}
    if status == "ERREUR":   # une ligne ERREUR de campagne n'a ni mesures ni faux succès
        record = {k: v for k, v in record.items() if k not in ("metrics", "false_success", "duration_s")}
    return record


class WilsonTest(unittest.TestCase):
    def test_bounds(self):
        low, high = tables.wilson(0, 100)
        self.assertEqual(low, 0.0)
        self.assertAlmostEqual(high, 0.0370, places=4)
        low, high = tables.wilson(50, 100)
        self.assertAlmostEqual(low, 0.4038, places=4)
        self.assertAlmostEqual(high, 0.5962, places=4)

    def test_empty_cell_is_uninformative(self):
        self.assertEqual(tables.wilson(0, 0), (0.0, 1.0))


class SummaryTest(unittest.TestCase):
    def cell(self, records):
        return tables.summarize(records)[("B0", "none", "F1")]

    def test_excludes_non_ok_runs_and_counts_them(self):
        row = self.cell([run(doubles=1), run(), run(status="INVALIDE", doubles=5), run(status="DÉLAI"),
                         run(status="ERREUR")])
        self.assertEqual((row["total"], row["ok"], row["invalid"], row["timeout"], row["error"]), (5, 2, 1, 1, 1))
        self.assertEqual(row["double_runs"], 1)   # la ligne INVALIDE à 5 doublons n'entre pas dans le taux

    def test_measures_are_counted_separately_never_added(self):
        row = self.cell([run(wrong=1, unpaid=1), run(doubles=2), run(unpaid=3)])
        self.assertEqual((row["double_runs"], row["wrong_runs"], row["unpaid_runs"]), (1, 1, 2))

    def test_false_success_only_from_ok_runs(self):
        row = self.cell([run(false_success=True), run(status="INVALIDE", false_success=True)])
        self.assertEqual(row["false_success_runs"], 1)

    def test_more_than_ten_percent_excluded_is_flagged(self):
        nine_ok = [run() for _ in range(9)]
        self.assertFalse(self.cell(nine_ok + [run(status="ERREUR")])["flagged"])          # 10 % : non signalé
        self.assertTrue(self.cell(nine_ok[:8] + [run(status="ERREUR")] * 2)["flagged"])  # 20 % : signalé

    def test_duration_median_and_interquartile_range(self):
        row = self.cell([run(duration=d) for d in (1.0, 2.0, 3.0, 4.0, 100.0)])
        self.assertEqual(row["median_duration_s"], 3.0)
        self.assertLess(row["iqr_duration_s"][0], row["iqr_duration_s"][1])
        single = self.cell([run(duration=7.0)])
        self.assertEqual((single["median_duration_s"], single["iqr_duration_s"]), (7.0, (7.0, 7.0)))

    def test_cell_without_valid_run_has_no_duration(self):
        row = self.cell([run(status="ERREUR")])
        self.assertEqual((row["ok"], row["median_duration_s"], row["iqr_duration_s"]), (0, None, None))


class AnnexTest(unittest.TestCase):
    def test_stopped_by_distribution_for_f4_and_f4e(self):
        records = [run(condition="S", fault="F4e", stopped_by="engine"), run(condition="S", fault="F4e", stopped_by="engine"),
                   run(condition="S", fault="F4", stopped_by="settlement"), run(condition="S", fault="F4", stopped_by="none"),
                   run(condition="S", fault="F4", status="INVALIDE", stopped_by="engine")]
        table = tables.stopped_by(records)
        self.assertEqual(table[("S", "none", "F4e")], {"engine": 2, "settlement": 0, "none": 0})
        self.assertEqual(table[("S", "none", "F4")], {"engine": 0, "settlement": 1, "none": 1})

    def test_stopped_by_omits_conditions_without_the_field(self):
        """B0/B1 n'ont pas de moteur : aucune ligne à zéro trompeuse dans le tableau annexe."""
        records = [run(condition="B0", fault="F4"), run(condition="S", fault="F4", stopped_by="settlement")]
        self.assertEqual(list(tables.stopped_by(records)), [("S", "none", "F4")])

    def test_f5_absorbed_count(self):
        records = [run(fault="F5", f5_absorbed=True), run(fault="F5", f5_absorbed=False),
                   run(fault="F5", status="ERREUR")]
        self.assertEqual(tables.f5_absorbed(records)[("B0", "none", "F5")], {"ok": 2, "absorbed": 1})


class LoadTest(unittest.TestCase):
    def test_campaign_lines_are_kept_apart(self):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "c.jsonl"
            lines = [{"kind": "campaign", "event": "start", "bench_sha256": "a"}, run(),
                     {"kind": "campaign", "event": "end", "warning": "empreinte du banc modifiée"}]
            path.write_text("".join(json.dumps(x) + "\n" for x in lines))
            runs, campaign = tables.load(path)
        self.assertEqual((len(runs), len(campaign)), (1, 2))

    def test_duplicate_case_is_refused(self):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "c.jsonl"
            path.write_text(json.dumps(run()) + "\n" + json.dumps(run(doubles=1)) + "\n")
            with self.assertRaisesRegex(ValueError, "en double"):
                tables.load(path)

    def test_markdown_flags_and_warns(self):
        records = [run(status="ERREUR")] + [run()]
        text = tables.markdown(tables.summarize(records), campaign=[{"kind": "campaign", "warning": "attention"}])
        self.assertIn("attention", text)
        self.assertIn("signalée", text)


if __name__ == "__main__":
    unittest.main()
