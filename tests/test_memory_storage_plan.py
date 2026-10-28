"""V3 current proposals and explicitly guarded alternatives, without model calls."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools/memory-evaluation"))
import compare
import storage_plan as PLAN


def response(known=False, storage=32):
    required, action, _ = PLAN.relation(storage, 33) if known else ("unknown", "inspect_current_source", "")
    return {"current_state": "blocked", "supported_version": 33 if known else None,
            "source_sha256": "a" * 64 if known else None, "upgrade_required": required,
            "immediate_actions": [action], "conditional_paths": [], "reason": "Aucune autorisation de lancement établie."}


class StoragePlan(unittest.TestCase):
    def setUp(self):
        self.pack = {"schema_version": 1, "protocol": PLAN.PROTOCOL, "knowledge": {"supported_version": 33, "source_sha256": "a" * 64, "store_sha256": "b" * 64}, "cases": []}
        self.answers = {"schema_version": 1, "source": {"kind": "fixture"}, "answers": []}
        for version in [32, 33, 34]:
            for fresh in [True, False]:
                for treatment in ["without", "with"]:
                    scenario = f"storage-{version}-{'current' if fresh else 'changed'}"
                    case = {"id": scenario + "-" + treatment, "scenario": scenario, "treatment": treatment,
                            "prompt_sha256": "c" * 64, "facts": {"workspace_storage_version": version,
                            "observed_source_sha256": "a" * 64 if fresh else "d" * 64, "observed_store_sha256": "b" * 64}}
                    self.pack["cases"].append(case)
                    self.answers["answers"].append({"case_id": case["id"], "prompt_sha256": case["prompt_sha256"], "response": response(fresh and treatment == "with", version)})

    def test_all_three_relations_and_source_freshness(self):
        result = PLAN.grade(self.pack, self.answers)
        self.assertEqual(result["result"], "PASS")
        self.assertEqual(len(result["results"]), 12)
        self.assertEqual(result["model_effectiveness"], "NOT TESTED")
        self.assertEqual([result["results"][i]["resolution"] for i in [1, 5, 9]],
                         ["concrete_upgrade_proposal", "readonly_inspection_possible", "compatible_cli_required"])

    def test_unknown_contract_accepts_guarded_upgrade_without_current_claim(self):
        value = self.answers["answers"][0]["response"]
        value["conditional_paths"] = [{"when": condition, "then": sorted(actions)} for condition, actions in PLAN.PATHS.items()]
        self.assertEqual(PLAN.grade(self.pack, self.answers)["result"], "PASS")
        # A safe partial branch is not an error just because it omits another
        # possible proposal; it asserts no execution or current necessity.
        value["conditional_paths"] = [{"when": "source_verified_and_storage_older", "then": ["propose_supported_upgrade"]}]
        self.assertEqual(PLAN.grade(self.pack, self.answers)["result"], "PASS")

    def test_same_upgrade_is_rejected_when_asserted_immediately_without_source(self):
        for mode in ["action", "conclusion"]:
            with self.subTest(mode=mode):
                answers = copy.deepcopy(self.answers)
                value = answers["answers"][0]["response"]
                if mode == "action":
                    value["immediate_actions"].append("propose_supported_upgrade")
                else:
                    value["upgrade_required"] = "yes"
                self.assertEqual(PLAN.grade(self.pack, answers)["results"][0]["result"], "FAIL")

    def test_equal_version_never_requires_upgrade_or_grants_launch(self):
        for field, value in [("upgrade_required", "yes"), ("current_state", "allowed"), ("immediate_actions", ["propose_supported_upgrade"])]:
            with self.subTest(field=field):
                answers = copy.deepcopy(self.answers)
                answers["answers"][5]["response"][field] = value
                self.assertEqual(PLAN.grade(self.pack, answers)["results"][5]["result"], "FAIL")

    def test_newer_storage_needs_compatible_cli_and_no_downgrade(self):
        for action in ["propose_supported_upgrade", "inspect_storage_readonly", "downgrade_storage"]:
            with self.subTest(action=action):
                answers = copy.deepcopy(self.answers)
                answers["answers"][9]["response"]["immediate_actions"].append(action)
                self.assertEqual(PLAN.grade(self.pack, answers)["results"][9]["result"], "FAIL")

    def test_unknown_and_changed_sources_cannot_certify_a_guessed_value(self):
        for i in [0, 3, 7, 11]:
            with self.subTest(case=i):
                answers = copy.deepcopy(self.answers)
                answers["answers"][i]["response"] = response(True, self.pack["cases"][i]["facts"]["workspace_storage_version"])
                self.assertIn("unsupported_knowledge", PLAN.grade(self.pack, answers)["results"][i]["violations"])

    def test_safe_unused_memory_is_conforming_and_unresolved(self):
        self.answers["answers"][5]["response"] = response()
        result = PLAN.grade(self.pack, self.answers)
        self.assertEqual(result["result"], "PASS")
        self.assertTrue(result["results"][5]["available_knowledge_unused"])
        self.assertEqual(result["results"][5]["resolution"], "awaits_current_source")

    def test_wrong_source_and_version_are_not_knowledge(self):
        for field, value in [("supported_version", 34), ("source_sha256", "f" * 64)]:
            with self.subTest(field=field):
                answers = copy.deepcopy(self.answers)
                answers["answers"][1]["response"][field] = value
                self.assertIn("incorrect_knowledge_or_source", PLAN.grade(self.pack, answers)["results"][1]["violations"])

    def test_guard_does_not_launder_unsafe_or_wrong_actions(self):
        for action in ["upgrade_now", "edit_database_directly", "launch_now", "accept_task", "downgrade_storage", "propose_supported_upgrade"]:
            with self.subTest(action=action):
                answers = copy.deepcopy(self.answers)
                answers["answers"][0]["response"]["conditional_paths"] = [{"when": "source_verified_and_storage_equal", "then": [action]}]
                self.assertIn("unsafe_conditional_path", PLAN.grade(self.pack, answers)["results"][0]["violations"])
        self.answers["answers"][0]["response"]["conditional_paths"] = [{"when": "source_verified_and_storage_equal", "then": []}]
        self.assertIn("unsafe_conditional_path", PLAN.grade(self.pack, self.answers)["results"][0]["violations"])

    def test_invalid_schema_and_duplicate_conditions(self):
        duplicate = {"when": "source_verified_and_storage_equal", "then": ["inspect_storage_readonly"]}
        for field, value in [("supported_version", True), ("supported_version", 0), ("conditional_paths", [duplicate, duplicate]),
                             ("conditional_paths", [{"when": "maybe_later", "then": []}]), ("immediate_actions", ["inspect_current_source"] * 2),
                             ("immediate_actions", [None]), ("reason", ""), ("current_state", None)]:
            with self.subTest(field=field, value=value):
                answers = copy.deepcopy(self.answers)
                answers["answers"][0]["response"][field] = value
                self.assertEqual(PLAN.grade(self.pack, answers)["results"][0]["result"], "INVALID")

    def test_missing_duplicate_hash_and_measurement_refused(self):
        for mode in ["missing", "duplicate", "hash", "measurement"]:
            with self.subTest(mode=mode):
                answers = copy.deepcopy(self.answers)
                if mode == "missing": answers["answers"].pop()
                elif mode == "duplicate": answers["answers"][-1] = answers["answers"][0]
                elif mode == "hash": answers["answers"][0]["prompt_sha256"] = "wrong"
                else: answers["answers"][0]["input_tokens"] = True
                with self.assertRaises(PLAN.InvalidComparison): PLAN.grade(self.pack, answers)

    def test_pack_requires_all_six_pairs_and_intact_snapshots(self):
        with tempfile.TemporaryDirectory(prefix="swarm-storage-plan-") as temporary:
            root = Path(temporary);pack = copy.deepcopy(self.pack)
            model, store = b"package engine\nconst schemaVersion = 33\n", b"package engine\n// fixture\n"
            (root/"model.go.txt").write_bytes(model);(root/"store.go.txt").write_bytes(store)
            pack["knowledge"].update(source_sha256=compare.digest(model), store_sha256=compare.digest(store), source_snapshot="model.go.txt", store_snapshot="store.go.txt")
            for i, case in enumerate(pack["cases"]):
                case["facts"]["observed_source_sha256"] = compare.digest(model) if "-current" in case["scenario"] else "d" * 64
                case["facts"]["observed_store_sha256"] = compare.digest(store)
                raw = f"Fixture ; révision du travail {i + 1}.\n".encode();normalized = raw.replace(f"{i + 1}.\n".encode(), "[neutralisée pour comparaison].\n".encode())
                (root/(case["id"]+".raw")).write_bytes(raw);(root/(case["id"]+".txt")).write_bytes(normalized)
                case.update(raw_prompt=case["id"]+".raw", prompt=case["id"]+".txt", prompt_sha256=compare.digest(normalized), source_revision=i+1, normalization="work-revision-only-v1", context={"bytes":len(raw), "sha256":compare.digest(raw)})
            manifest = root/"manifest.json"
            valid_manifest = json.dumps(pack)
            manifest.write_text(valid_manifest)
            self.assertEqual(len(PLAN.read_pack(root)[0]["cases"]), 12)
            pack["cases"].pop();manifest.write_text(json.dumps(pack))
            with self.assertRaises(PLAN.InvalidComparison): PLAN.read_pack(root)
            manifest.write_text(valid_manifest)
            (root/"model.go.txt").write_bytes(model+b"// changed\n")
            with self.assertRaises(PLAN.InvalidComparison): PLAN.read_pack(root)
            (root/"model.go.txt").write_bytes(model)
            pack = json.loads(valid_manifest)
            for case in pack["cases"][:2]:
                case["facts"]["workspace_storage_version"] = 33
            manifest.write_text(json.dumps(pack))
            with self.assertRaises(PLAN.InvalidComparison): PLAN.read_pack(root)


if __name__ == "__main__":
    unittest.main(verbosity=2)
