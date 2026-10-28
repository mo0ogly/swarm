"""Regression checks for the bounded decision experiment, without model calls."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "tools/memory-evaluation/compare.py"
SPEC = importlib.util.spec_from_file_location("memory_comparison", SCRIPT)
COMPARE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COMPARE)


def response(recoverable):
    return {"decision": "restore_and_evaluate" if recoverable else "rebuild_evidence",
            "actions": ["verify_archive", "restore_artifact", "evaluate_current"] if recoverable else ["block_admission", "request_fresh_evidence"],
            "admission": "after_fresh_pass" if recoverable else "blocked",
            "acceptance": "unchanged", "reason": "Décision fondée sur les observations actuelles."}


class MemoryComparison(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="swarm-memory-grader-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        cases = []
        for scenario in ("missing-recoverable", "changed-unrecoverable"):
            recoverable = scenario == "missing-recoverable"
            facts = {"required_sha256": "a" * 64, "target_state": "missing" if recoverable else "different",
                     "target_sha256": None if recoverable else "b" * 64,
                     "archive_available": recoverable, "archive_sha256": "a" * 64 if recoverable else None,
                     "historical_task_status": "accepted"}
            for treatment, revision in (("without", 3), ("with", 4)):
                case_id = scenario + "-" + treatment
                raw = (f"Identity ; révision du travail {revision}.\nCurrent observations: " + json.dumps(facts)
                       + ("\nSelected conditional memory" if treatment == "with" else "")).encode()
                normalized = raw.replace(f" ; révision du travail {revision}.\n".encode(), " ; révision du travail [neutralisée pour comparaison].\n".encode(), 1)
                (self.root / (case_id + ".engine.txt")).write_bytes(raw)
                (self.root / (case_id + ".txt")).write_bytes(normalized)
                cases.append({"id": case_id, "scenario": scenario, "treatment": treatment,
                              "prompt": case_id + ".txt", "raw_prompt": case_id + ".engine.txt",
                              "source_revision": revision, "normalization": "work-revision-only-v1",
                              "prompt_sha256": COMPARE.digest(normalized), "context": {"sha256": COMPARE.digest(raw), "bytes": len(raw)}, "facts": facts})
        self.pack = {"schema_version": 1, "cases": cases}
        self.save_pack()
        self.answers = {"schema_version": 1, "source": {"kind": "fixture"}, "answers": [
            {"case_id": case["id"], "prompt_sha256": case["prompt_sha256"], "response": response(case["scenario"] == "missing-recoverable")}
            for case in cases]}

    def save_pack(self):
        (self.root / "manifest.json").write_text(json.dumps(self.pack))

    def test_correct_controls_are_not_model_quality(self):
        pack, digest = COMPARE.read_pack(self.root)
        self.assertEqual(len(digest), 64)
        result = COMPARE.grade(pack, self.answers)
        self.assertEqual(result["result"], "PASS")
        self.assertEqual(result["model_effectiveness"], "NOT TESTED")
        self.assertTrue(all(c["observed_answer_change"] == "same" for c in result["comparisons"]))
        self.assertTrue(all(r["measurements"]["cost"] is None for r in result["results"]))

    def test_initial_recipe_fails_on_changed_variant(self):
        self.answers["answers"][3]["response"] = response(True)
        result = COMPARE.grade(self.pack, self.answers)
        self.assertEqual(result["result"], "FAIL")
        self.assertEqual(set(result["results"][3]["violations"]), {"decision", "actions", "admission"})
        self.assertEqual(result["comparisons"][0]["observed_answer_change"], "regression")

    def test_historical_acceptance_and_premature_admission_fail(self):
        for key, value in (("acceptance", "accepted"), ("admission", "now")):
            with self.subTest(key=key):
                answers = copy.deepcopy(self.answers)
                answers["answers"][0]["response"][key] = value
                result = COMPARE.grade(self.pack, answers)
                self.assertEqual(result["results"][0]["result"], "FAIL")
                self.assertIn(key, result["results"][0]["violations"])

    def test_action_order_matters(self):
        self.answers["answers"][0]["response"]["actions"].reverse()
        self.assertEqual(COMPARE.grade(self.pack, self.answers)["results"][0]["result"], "FAIL")

    def test_empty_incomplete_and_wrong_type_are_invalid(self):
        for bad in (None, {}, [], {**response(True), "reason": ""}, {**response(True), "actions": "evaluate_current"}, {**response(True), "unexpected": True}):
            with self.subTest(bad=bad):
                answers = copy.deepcopy(self.answers)
                answers["answers"][0]["response"] = bad
                result = COMPARE.grade(self.pack, answers)
                self.assertEqual(result["result"], "FAIL")
                self.assertEqual(result["results"][0]["result"], "INVALID")

    def test_missing_duplicate_or_unknown_answer_refused(self):
        for mode in ("missing", "duplicate", "unknown"):
            with self.subTest(mode=mode):
                answers = copy.deepcopy(self.answers)
                if mode == "missing":
                    answers["answers"].pop()
                elif mode == "duplicate":
                    answers["answers"][3] = answers["answers"][0]
                else:
                    answers["answers"][3]["case_id"] = "unknown"
                with self.assertRaises(COMPARE.InvalidComparison):
                    COMPARE.grade(self.pack, answers)

    def test_answer_hash_and_invalid_measurements_refused(self):
        variants = [("prompt_sha256", "wrong"), ("input_tokens", True), ("output_tokens", 1.5),
                    ("cost", -1), ("elapsed_seconds", float("nan"))]
        for key, value in variants:
            with self.subTest(key=key):
                answers = copy.deepcopy(self.answers)
                answers["answers"][0][key] = value
                with self.assertRaises(COMPARE.InvalidComparison):
                    COMPARE.grade(self.pack, answers)

    def test_undeclared_provenance_or_boolean_version_refused(self):
        for value in ({}, [], {**self.answers, "schema_version": True}, {**self.answers, "source": {"kind": "supplied_outputs"}}):
            with self.subTest(value=value):
                with self.assertRaises(COMPARE.InvalidComparison):
                    COMPARE.grade(self.pack, value)

    def test_corrupt_context_or_undeclared_normalization_refused(self):
        case = self.pack["cases"][0]
        path = self.root / case["prompt"]
        path.write_text(path.read_text() + "\nChanged instruction")
        with self.assertRaises(COMPARE.InvalidComparison):
            COMPARE.read_pack(self.root)
        # Updating just the comparison hash cannot authorize another edit.
        case["prompt_sha256"] = COMPARE.digest(path.read_bytes())
        self.save_pack()
        with self.assertRaises(COMPARE.InvalidComparison):
            COMPARE.read_pack(self.root)

    def test_foreign_prompt_refused(self):
        self.pack["cases"][0]["prompt"] = "../foreign.txt"
        self.save_pack()
        with self.assertRaises(COMPARE.InvalidComparison):
            COMPARE.read_pack(self.root)

    def test_case_shape_and_mismatched_pair_refused(self):
        original = copy.deepcopy(self.pack)
        for mode in ("not-object", "no-scenario", "different-observations"):
            with self.subTest(mode=mode):
                self.pack = copy.deepcopy(original)
                if mode == "not-object":
                    self.pack["cases"][0] = None
                elif mode == "no-scenario":
                    self.pack["cases"][0]["scenario"] = None
                else:
                    self.pack["cases"][0]["facts"] = {**self.pack["cases"][0]["facts"], "historical_task_status": "blocked"}
                self.save_pack()
                with self.assertRaises(COMPARE.InvalidComparison):
                    COMPARE.read_pack(self.root)

    def test_cli_refuses_overwrite_and_preserves_unknown_measurements(self):
        source = self.root / "answers.json"
        source.write_text(json.dumps(self.answers))
        output = self.root / "result.json"
        command = [sys.executable, str(SCRIPT), "--pack", str(self.root), "--answers", str(source), "--output", str(output)]
        first = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
        saved = output.read_bytes()
        second = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(second.returncode, 2)
        self.assertEqual(output.read_bytes(), saved)
        self.assertIsNone(json.loads(saved)["results"][0]["measurements"]["input_tokens"])

    def test_duplicate_json_and_nonfinite_json_refused(self):
        path = self.root / "malformed.json"
        for text in ('{"answer":1,"answer":2}', '{"cost":NaN}'):
            path.write_text(text)
            with self.assertRaises(COMPARE.InvalidComparison):
                COMPARE.load_json(path)


if __name__ == "__main__":
    unittest.main(verbosity=2)
