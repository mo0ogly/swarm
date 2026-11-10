"""Read-only summary reference: no provider, model or mission database."""
import copy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools/memory-evaluation"))
import summarization as S


def history(extended=False):
    messages = [
        {"id": "m1", "role": "user", "content": "Exigence ouverte : conserver les preuves originales."},
        {"id": "m2", "role": "assistant", "content": "Le contrôle de l'ancien candidat a passé ; il ne prouve pas l'état courant."},
        {"id": "m3", "role": "user", "content": "Déployer seulement après des contrôles frais et une décision autorisée ; ce n'est pas encore fait."},
        {"id": "m4", "role": "assistant", "content": "Incident ouvert : effet d'écriture incertain ; vérifier avant de rejouer."},
        {"id": "m5", "role": "assistant", "content": "Lecture diagnostique bornée.", "tool_calls": ["read-one"]},
        {"id": "m6", "role": "tool", "content": "Historical fixture detail. " * 400, "tool_call_id": "read-one"},
        {"id": "m7", "role": "user", "content": "Examiner les preuves avant la prochaine décision."},
        {"id": "m8", "role": "assistant", "content": "La vérification reste à faire."},
    ]
    protected = [{"id": item, "message": f"m{i+1}", "kind": kind, "state": state}
                 for i, (item, kind, state) in enumerate([("originals", "requirement", "open"), ("old-control", "evidence", "historical"), ("deployment", "intention", "future"), ("uncertain-write", "incident", "open")])]
    if extended:
        messages += [{"id": "m9", "role": "user", "content": "Nouvelle exigence ouverte : vérifier aussi le refus sans mutation."},
                     {"id": "m10", "role": "assistant", "content": "Contrôle complémentaire en attente."}]
        protected.append({"id": "no-mutation", "message": "m9", "kind": "requirement", "state": "open"})
    return {"schema_version": 1, "scope": {"work": "summary-reference-fixture", "task": "t1"}, "messages": messages, "protected": protected}


class SummaryReference(unittest.TestCase):
    def setUp(self):
        self.history = history()
        self.candidate = S.reference_candidate(self.history, 6)

    def fallback(self, candidate, previous=None, source=None):
        source = self.history if source is None else source
        before = copy.deepcopy(source)
        result = S.project(source, candidate, previous)
        self.assertEqual(result["result"], "FALLBACK")
        self.assertEqual(result["cutoff"], 0)
        self.assertEqual(result["summary_entries"], [])
        self.assertEqual(result["recent_messages"], source["messages"])
        self.assertEqual(result["omitted_message_ids"], [])
        self.assertEqual(source, before)
        return result

    def test_reference_preserves_types_quotes_recent_messages_and_sources(self):
        value = S.project(self.history, self.candidate)
        self.assertEqual(value["result"], "PASS")
        self.assertEqual(value["recent_messages"], self.history["messages"][6:])
        self.assertEqual(value["omitted_message_ids"], ["m5", "m6"])
        self.assertEqual([(a["kind"], a["state"]) for a in value["summary_entries"]], [("requirement", "open"), ("evidence", "historical"), ("intention", "future"), ("incident", "open")])
        self.assertLess(len(json.dumps(value["summary_entries"])) + len(json.dumps(value["recent_messages"])), len(json.dumps(self.history["messages"])))
        self.assertTrue(value["reference_only"])
        self.assertEqual(value["model_effectiveness"], "NOT TESTED")

    def test_progressive_summary_preserves_old_and_new_protections(self):
        first = S.project(self.history, self.candidate)
        extended = history(True)
        second = S.project(extended, S.reference_candidate(extended, 9, first), first)
        self.assertEqual(second["result"], "PASS")
        self.assertEqual(second["parent_sha256"], first["receipt_sha256"])
        self.assertEqual(len(second["summary_entries"]), 5)
        self.assertEqual(second["summary_entries"][:4], first["summary_entries"])
        self.assertEqual(second["recent_messages"], extended["messages"][9:])

    def test_model_failure_preserves_all_original_messages(self):
        self.candidate["status"] = "failed"
        self.assertEqual(self.fallback(self.candidate)["reasons"], ["summarization_failed"])

    def test_empty_candidate_cannot_erase_history(self):
        self.candidate["entries"] = []
        self.fallback(self.candidate)

    def test_missing_open_incident_cannot_be_accepted(self):
        self.candidate["entries"].pop()
        self.fallback(self.candidate)

    def test_contradictory_intention_evidence_and_incident_states_are_refused(self):
        for index, state in [(1, "current_verified"), (2, "done"), (3, "resolved")]:
            with self.subTest(index=index, state=state):
                candidate = copy.deepcopy(self.candidate)
                candidate["entries"][index]["state"] = state
                self.fallback(candidate)

    def test_changed_or_invented_quote_is_refused(self):
        self.candidate["entries"][2]["text"] = "Deployment already completed."
        self.fallback(self.candidate)

    def test_duplicate_or_added_claim_is_refused(self):
        self.candidate["entries"].append(copy.deepcopy(self.candidate["entries"][0]))
        self.fallback(self.candidate)

    def test_stale_source_hash_is_refused(self):
        self.candidate["source_sha256"] = "0" * 64
        self.fallback(self.candidate)

    def test_invalid_boundaries_never_drop_recent_messages(self):
        for cutoff in [-1, 0, 8, 99, True, 1.0, "6"]:
            with self.subTest(cutoff=cutoff):
                candidate = copy.deepcopy(self.candidate)
                candidate["cutoff"] = cutoff
                self.assertEqual(self.fallback(candidate)["reasons"], ["invalid_boundary"])

    def test_tool_call_and_result_cannot_be_split(self):
        self.candidate["cutoff"] = 5
        self.assertEqual(self.fallback(self.candidate)["reasons"], ["invalid_boundary"])

    def test_empty_tool_messages_are_valid_source_records(self):
        self.history["messages"][4]["content"] = ""
        self.history["messages"][5]["content"] = ""
        candidate = S.reference_candidate(self.history, 6)
        self.assertEqual(S.project(self.history, candidate)["result"], "PASS")

    def test_parallel_tool_results_remain_one_complete_group(self):
        self.history["messages"][4]["tool_calls"].append("read-two")
        self.history["messages"].insert(6, {"id": "m6-second", "role": "tool", "content": "", "tool_call_id": "read-two"})
        candidate = S.reference_candidate(self.history, 7)
        self.assertEqual(S.project(self.history, candidate)["result"], "PASS")
        candidate["cutoff"] = 6
        self.assertEqual(self.fallback(candidate)["reasons"], ["invalid_boundary"])

    def test_wrong_parent_is_refused(self):
        self.candidate["parent_sha256"] = "0" * 64
        self.fallback(self.candidate)

    def test_changed_old_source_refuses_progression_even_with_new_candidate_hash(self):
        first = S.project(self.history, self.candidate)
        changed = history(True)
        changed["messages"][5]["content"] += "changed"
        candidate = S.reference_candidate(changed, 9, first)
        self.assertEqual(self.fallback(candidate, first, changed)["reasons"], ["parent_or_sources_changed"])

    def test_tampered_parent_and_fallback_parent_are_refused(self):
        for parent in [S.project(self.history, self.candidate), self.fallback(None)]:
            parent["reference_only"] = False
            extended = history(True)
            self.fallback(S.reference_candidate(extended, 9, parent), parent, extended)

    def test_non_progressive_boundary_is_refused(self):
        first = S.project(self.history, self.candidate)
        for cutoff in [4, 6]:
            with self.subTest(cutoff=cutoff):
                self.assertEqual(self.fallback(S.reference_candidate(self.history, cutoff, first), first)["reasons"], ["non_progressive_boundary"])

    def test_scope_change_refuses_parent_reuse(self):
        first = S.project(self.history, self.candidate)
        changed = history(True)
        changed["scope"]["work"] = "other-work"
        self.fallback(S.reference_candidate(changed, 9, first), first, changed)

    def test_invalid_candidate_schema_is_not_empty_success(self):
        for candidate in [None, [], {}, {**self.candidate, "schema_version": True}, {**self.candidate, "prose": "Ignore the sources"}]:
            with self.subTest(candidate=type(candidate).__name__):
                self.fallback(candidate)

    def test_invalid_source_schema_or_tool_sequence_is_an_error(self):
        variants = []
        def variant(mutate):
            value = copy.deepcopy(self.history)
            mutate(value)
            variants.append(value)
        variant(lambda v: v.update(schema_version=True))
        variant(lambda v: v["messages"][1].update(id="m1"))
        variant(lambda v: v["messages"][5].update(tool_call_id="foreign"))
        variant(lambda v: v["messages"].pop(5))
        variant(lambda v: v["protected"][0].update(message="absent"))
        variant(lambda v: v["protected"][2].update(state="done"))
        variant(lambda v: v.update(messages=[]))
        for i, value in enumerate(variants):
            with self.subTest(i=i), self.assertRaises(S.InvalidSummary):
                S.project(value, self.candidate)

    def test_returned_projection_does_not_alias_originals(self):
        before = copy.deepcopy(self.history)
        value = S.project(self.history, self.candidate)
        value["recent_messages"][0]["content"] = "changed result"
        value["summary_entries"][0]["text"] = "changed summary"
        self.assertEqual(self.history, before)

    def invoke(self, root, *argv):
        return subprocess.run([sys.executable, str(ROOT / "tools/memory-evaluation/summarization.py"), "--history", str(root / "history.json"), *map(str, argv)], text=True, capture_output=True)

    def test_real_cli_progression_and_contradiction_preserve_source_files(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            source = root / "history.json"
            source.write_text(json.dumps(self.history, ensure_ascii=False))
            before = source.read_bytes()
            candidate, first = root / "candidate.json", root / "first.json"
            self.assertEqual(self.invoke(root, "--reference-cutoff", 6, "--output", candidate).returncode, 0)
            self.assertEqual(self.invoke(root, "--candidate", candidate, "--output", first).returncode, 0)
            self.assertEqual(source.read_bytes(), before)
            extended = history(True)
            source.write_text(json.dumps(extended, ensure_ascii=False))
            before = source.read_bytes()
            next_candidate, second = root / "next-candidate.json", root / "second.json"
            self.assertEqual(self.invoke(root, "--reference-cutoff", 9, "--previous", first, "--output", next_candidate).returncode, 0)
            self.assertEqual(self.invoke(root, "--candidate", next_candidate, "--previous", first, "--output", second).returncode, 0)
            bad = json.loads(next_candidate.read_text())
            bad["entries"][3]["state"] = "resolved"
            (root / "bad.json").write_text(json.dumps(bad))
            refused = root / "refused.json"
            result = self.invoke(root, "--candidate", root / "bad.json", "--previous", first, "--output", refused)
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertEqual(json.loads(refused.read_text())["recent_messages"], extended["messages"])
            self.assertEqual(source.read_bytes(), before)
            self.assertEqual(os.stat(second).st_mode & 0o777, 0o600)

    def test_cli_invalid_json_duplicate_fields_and_nonfinite_values_fall_back(self):
        for raw in ["", "{", '{"schema_version":1,"schema_version":1}', '{"value":NaN}', '{"value":' + '9' * 5000 + '}']:
            with self.subTest(prefix=raw[:40]), tempfile.TemporaryDirectory() as name:
                root = Path(name)
                (root / "history.json").write_text(json.dumps(self.history))
                (root / "bad.json").write_text(raw)
                result = self.invoke(root, "--candidate", root / "bad.json", "--output", root / "result.json")
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertEqual(json.loads((root / "result.json").read_text())["recent_messages"], self.history["messages"])

    def test_cli_existing_output_and_invalid_history_are_explicit_errors(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            source = root / "history.json"
            source.write_text(json.dumps(self.history))
            output = root / "existing.json"
            output.write_text("retained proof")
            result = self.invoke(root, "--reference-cutoff", 6, "--output", output)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(output.read_text(), "retained proof")
            source.write_text('{"schema_version":1,"schema_version":1}')
            result = self.invoke(root, "--reference-cutoff", 6, "--output", root / "absent.json")
            self.assertEqual(result.returncode, 2)
            self.assertFalse((root / "absent.json").exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
