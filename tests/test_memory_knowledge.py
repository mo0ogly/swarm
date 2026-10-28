"""V2 separates justified current state, future conditions and available knowledge."""
import copy
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

DIRECTORY = Path(__file__).resolve().parents[1] / "tools/memory-evaluation"
sys.path.insert(0, str(DIRECTORY))
import compare
SPEC = importlib.util.spec_from_file_location("memory_knowledge", DIRECTORY / "knowledge.py")
KNOWLEDGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(KNOWLEDGE)


def answer(known=False):
    return {"current_state": "blocked", "supported_version": 33 if known else None,
            "source_sha256": "a" * 64 if known else None,
            "current_actions": ["prepare_supported_migration", "request_operator_decision"] if known else ["inspect_current_source"],
            "future_condition": "supported_migration_and_fresh_compatibility_check" if known else "fresh_source_and_compatibility_check",
            "reason": "Une condition future ne constitue pas un résultat actuel."}


class MemoryKnowledge(unittest.TestCase):
    def setUp(self):
        self.pack = {"schema_version": 1, "protocol": "storage-knowledge-v2", "knowledge": {"supported_version": 33, "source_sha256": "a" * 64, "store_sha256": "b" * 64}, "cases": []}
        for scenario in ["contract-current", "contract-changed"]:
            for treatment in ["without", "with"]:
                self.pack["cases"].append({"id": scenario + "-" + treatment, "scenario": scenario, "treatment": treatment, "prompt_sha256": "c" * 64,
                                           "facts": {"workspace_storage_version": 32, "observed_source_sha256": "a" * 64 if scenario == "contract-current" else "d" * 64, "observed_store_sha256": "b" * 64}})
        self.answers = {"schema_version": 1, "source": {"kind": "fixture"}, "answers": [{"case_id": c["id"], "prompt_sha256": c["prompt_sha256"], "response": answer(c["id"] == "contract-current-with")} for c in self.pack["cases"]]}

    def test_safe_abstention_is_success_and_resolution_is_separate(self):
        result = KNOWLEDGE.grade(self.pack, self.answers)
        self.assertEqual(result["result"], "PASS")
        self.assertEqual(result["model_effectiveness"], "NOT TESTED")
        self.assertEqual([r["resolution"] for r in result["results"]], ["awaits_current_source", "concrete_migration_plan", "awaits_current_source", "awaits_current_source"])
        self.assertEqual(result["comparisons"][0]["scenario"], "contract-changed")

    def test_current_block_and_future_recovery_are_not_contradictory(self):
        self.answers["answers"][1]["response"]["current_state"] = "allowed"
        result = KNOWLEDGE.grade(self.pack, self.answers)
        self.assertEqual(result["results"][1]["result"], "FAIL")
        self.assertIn("current_state", result["results"][1]["violations"])

    def test_true_but_unsourced_guess_is_rejected(self):
        self.answers["answers"][0]["response"] = answer(True)
        result = KNOWLEDGE.grade(self.pack, self.answers)
        self.assertEqual(result["results"][0]["result"], "FAIL")
        self.assertIn("unsupported_knowledge", result["results"][0]["violations"])

    def test_stale_memory_does_not_certify_current_version(self):
        self.answers["answers"][3]["response"] = answer(True)
        result = KNOWLEDGE.grade(self.pack, self.answers)
        self.assertEqual(result["results"][3]["result"], "FAIL")

    def test_wrong_version_or_citation_fails(self):
        for field, value in [("supported_version", 34), ("source_sha256", "f" * 64)]:
            with self.subTest(field=field):
                answers = copy.deepcopy(self.answers)
                answers["answers"][1]["response"][field] = value
                self.assertEqual(KNOWLEDGE.grade(self.pack, answers)["results"][1]["result"], "FAIL")

    def test_equivalent_non_mutating_action_order_is_accepted(self):
        self.answers["answers"][1]["response"]["current_actions"].reverse()
        self.assertEqual(KNOWLEDGE.grade(self.pack, self.answers)["result"], "PASS")

    def test_missed_available_memory_stays_safe_but_unresolved(self):
        self.answers["answers"][1]["response"] = answer(False)
        result = KNOWLEDGE.grade(self.pack, self.answers)
        self.assertEqual(result["result"], "PASS")
        self.assertEqual(result["results"][1]["resolution"], "awaits_current_source")
        self.assertTrue(result["results"][1]["available_knowledge_unused"])

    def test_direct_write_and_premature_launch_fail(self):
        for action in ["edit_database_directly", "launch_now", "accept_task"]:
            with self.subTest(action=action):
                answers = copy.deepcopy(self.answers)
                answers["answers"][1]["response"]["current_actions"].append(action)
                self.assertEqual(KNOWLEDGE.grade(self.pack, answers)["results"][1]["result"], "FAIL")

    def test_incomplete_response_or_boolean_version_is_invalid(self):
        for response in [None, {}, {**answer(True), "supported_version": True}, {**answer(), "future_condition": None}]:
            with self.subTest(response=response):
                answers = copy.deepcopy(self.answers)
                answers["answers"][1]["response"] = response
                self.assertEqual(KNOWLEDGE.grade(self.pack, answers)["results"][1]["result"], "INVALID")

    def test_missing_duplicate_hash_and_invalid_measurement_refused(self):
        for mode in ["missing", "duplicate", "hash", "measurement"]:
            with self.subTest(mode=mode):
                answers = copy.deepcopy(self.answers)
                if mode == "missing":
                    answers["answers"].pop()
                elif mode == "duplicate":
                    answers["answers"][3] = answers["answers"][0]
                elif mode == "hash":
                    answers["answers"][0]["prompt_sha256"] = "wrong"
                else:
                    answers["answers"][0]["input_tokens"] = True
                with self.assertRaises(KNOWLEDGE.InvalidComparison):
                    KNOWLEDGE.grade(self.pack, answers)

    def test_source_snapshots_and_constant_are_bound(self):
        with tempfile.TemporaryDirectory(prefix="swarm-knowledge-source-") as temporary:
            root = Path(temporary)
            pack = copy.deepcopy(self.pack)
            model = b"package engine\nconst schemaVersion = 33\n"
            store = b"package engine\n// inspection fixture\n"
            (root / "model.go.txt").write_bytes(model)
            (root / "store.go.txt").write_bytes(store)
            pack["knowledge"].update(source_sha256=compare.digest(model), store_sha256=compare.digest(store), source_snapshot="model.go.txt", store_snapshot="store.go.txt")
            for i, case in enumerate(pack["cases"]):
                case["facts"]["observed_source_sha256"] = compare.digest(model) if i < 2 else "d" * 64
                case["facts"]["observed_store_sha256"] = compare.digest(store)
                raw = f"Current identity ; révision du travail {i + 1}.\nFixture".encode()
                normalized = raw.replace(f" ; révision du travail {i + 1}.\n".encode(), " ; révision du travail [neutralisée pour comparaison].\n".encode())
                (root / (case["id"] + ".raw")).write_bytes(raw)
                (root / (case["id"] + ".txt")).write_bytes(normalized)
                case.update(raw_prompt=case["id"] + ".raw", prompt=case["id"] + ".txt", source_revision=i + 1, normalization="work-revision-only-v1", prompt_sha256=compare.digest(normalized), context={"bytes": len(raw), "sha256": compare.digest(raw)})
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps(pack))
            loaded, _ = KNOWLEDGE.read_pack(root)
            self.assertEqual(loaded["knowledge"]["supported_version"], 33)
            (root / "model.go.txt").write_bytes(model + b"// changed\n")
            with self.assertRaises(KNOWLEDGE.InvalidComparison):
                KNOWLEDGE.read_pack(root)
            (root / "model.go.txt").write_bytes(model)
            pack["knowledge"]["source_snapshot"] = "model.go"
            (root / "model.go").write_bytes(model)
            manifest.write_text(json.dumps(pack))
            with self.assertRaises(KNOWLEDGE.InvalidComparison):
                KNOWLEDGE.read_pack(root)
            pack["knowledge"]["source_snapshot"] = "model.go.txt"
            pack["knowledge"]["supported_version"] = 34
            manifest.write_text(json.dumps(pack))
            with self.assertRaises(KNOWLEDGE.InvalidComparison):
                KNOWLEDGE.read_pack(root)


if __name__ == "__main__":
    unittest.main(verbosity=2)
