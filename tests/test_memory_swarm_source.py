"""Swarm source bridge and exact M17 quotes, without a model or private database."""
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
import swarm_source as B


def packet():
    snapshot = {"project_sha256": "a" * 64, "work_title": "Source fixture", "work_objective": "Keep evidence", "work_scope": "Owned fixture", "work_criteria": ["Verified sources"], "task": {"id": "t1", "criteria": ["Recheck current evidence"], "status": "todo"}}
    question, answer = "Vérifier avant de déployer <source>.", "Résultat historique, pas preuve courante."
    ref = {"task": "b1", "attempt": "a1", "sha256": B.digest((question + "\n" + answer).encode())}
    # Match memorySummarySourceForWork: Go marshals DialogueRef in this field
    # order before hashing. These ASCII fixture fields need no HTML escaping.
    dialogue_id = "dialogue-" + B.digest(json.dumps(ref, separators=(",", ":")).encode())
    sources = [{"id": "current-task-snapshot", "kind": "task_snapshot", "role": "user", "text": json.dumps(snapshot, ensure_ascii=False)}]
    for suffix, kind, role, text in [("question", "operator_question", "user", question), ("answer", "historical_answer", "assistant", answer)]:
        sources.append({"id": dialogue_id + "-" + suffix, "kind": kind, "role": role, "text": text, "dialogue": ref})
    for source in sources:
        source["sha256"] = B.digest(source["text"].encode())
    payload = {"scope": {"work": "w1", "task": "t1"}, "work_revision": 4, "project_sha256": "a" * 64, "audience": "local_operator", "provider_history_complete": False, "sources": sources}
    raw = json.dumps(payload, ensure_ascii=False)
    value = {"schema_version": 1, "kind": "swarm_summary_source_v1", "payload": raw, "sha256": B.digest(raw.encode())}
    annotations = {"schema_version": 1, "source_sha256": value["sha256"], "protected": [{"id": "task", "message": "current-task-snapshot", "kind": "requirement", "state": "open"}, {"id": "intent", "message": dialogue_id + "-question", "kind": "intention", "state": "future"}]}
    return value, annotations


class SwarmSummarySource(unittest.TestCase):
    def setUp(self):
        self.packet, self.annotations = packet()

    def mutate_payload(self, mutate):
        payload = json.loads(self.packet["payload"])
        mutate(payload)
        self.packet["payload"] = json.dumps(payload, ensure_ascii=False)
        self.packet["sha256"] = B.digest(self.packet["payload"].encode())
        self.annotations["source_sha256"] = self.packet["sha256"]

    def test_preserves_exact_quotes_scope_and_provenance_for_reference(self):
        history, provenance = B.prepare(self.packet, self.annotations)
        result = S.project(history, S.reference_candidate(history, 2))
        self.assertEqual(result["result"], "PASS")
        self.assertEqual(result["summary_entries"][1]["text"], "Vérifier avant de déployer <source>.")
        self.assertEqual(provenance["history_sha256"], result["source_sha256"])
        self.assertEqual(provenance["work_revision"], 4)
        self.assertFalse(provenance["engine_admission"])

    def test_rejects_payload_and_text_tampering(self):
        self.packet["payload"] += " "
        with self.assertRaises(ValueError): B.prepare(self.packet, self.annotations)
        self.packet, self.annotations = packet()
        self.mutate_payload(lambda p: p["sources"][2].update(text="changed"))
        with self.assertRaises(ValueError): B.prepare(self.packet, self.annotations)

    def test_rejects_changed_dialogue_pair_even_after_text_rehash(self):
        def change(p):
            p["sources"][2]["text"] = "changed"
            p["sources"][2]["sha256"] = B.digest(b"changed")
        self.mutate_payload(change)
        with self.assertRaises(ValueError): B.prepare(self.packet, self.annotations)

    def test_requires_annotations_bound_to_packet_and_current_task(self):
        for mutation in [lambda a: a.update(source_sha256="b" * 64), lambda a: a["protected"].pop(0), lambda a: a.update(protected=[])]:
            value = copy.deepcopy(self.annotations)
            mutation(value)
            with self.assertRaises(ValueError): B.prepare(self.packet, value)

    def test_refuses_wrong_scope_roles_duplicates_and_completeness(self):
        for mutate in [lambda p: p["scope"].update(task="other"), lambda p: p["sources"][2].update(role="user"), lambda p: p["sources"][2].update(id=p["sources"][1]["id"]), lambda p: p.update(provider_history_complete=True), lambda p: p.update(work_revision=True)]:
            self.packet, self.annotations = packet()
            self.mutate_payload(mutate)
            with self.assertRaises(ValueError): B.prepare(self.packet, self.annotations)

    def test_rejects_relabelled_sources_after_all_hashes_and_annotations_change(self):
        for which in [(1,), (2,), (1, 2)]:
            with self.subTest(sources=which):
                self.packet, self.annotations = packet()
                renamed = {}
                def change(payload):
                    for index in which:
                        source = payload["sources"][index]
                        renamed[source["id"]] = source["id"] + "-renamed"
                        source["id"] = renamed[source["id"]]
                self.mutate_payload(change)
                for anchor in self.annotations["protected"]:
                    anchor["message"] = renamed.get(anchor["message"], anchor["message"])
                with self.assertRaisesRegex(ValueError, "source.*identit|identit.*source"):
                    B.prepare(self.packet, self.annotations)

    def test_rejects_duplicate_dialogue_under_distinct_message_ids(self):
        def duplicate(payload):
            pair = copy.deepcopy(payload["sources"][1:3])
            for source in pair:
                source["id"] += "-duplicate"
            payload["sources"].extend(pair)
        self.mutate_payload(duplicate)
        with self.assertRaises(ValueError):
            B.prepare(self.packet, self.annotations)

    def test_dialogue_reference_object_order_is_not_source_identity(self):
        def reverse_keys(payload):
            for source in payload["sources"][1:]:
                source["dialogue"] = dict(reversed(list(source["dialogue"].items())))
        self.mutate_payload(reverse_keys)
        history, _ = B.prepare(self.packet, self.annotations)
        self.assertEqual(len(history["messages"]), 3)

    def test_source_ids_match_retained_go_json_vectors(self):
        vectors = json.loads((ROOT / "tests/memory_summary_source_id_vectors.json").read_text())
        self.assertEqual(len(vectors), 4)
        for vector in vectors:
            with self.subTest(ref=vector["ref"]):
                self.assertEqual(B.dialogue_source_id(vector["ref"]), vector["source_id"])
                self.assertEqual("dialogue-" + B.digest(vector["go_json"].encode()), vector["source_id"])

    def test_legacy_empty_attempt_preserves_content_binding_without_authority(self):
        renamed = {}
        def change(payload):
            for source in payload["sources"][1:]:
                source["dialogue"]["attempt"] = ""
                suffix = "-question" if source["kind"] == "operator_question" else "-answer"
                ref = source["dialogue"]
                prefix = "dialogue-" + B.digest(json.dumps(ref, separators=(",", ":")).encode())
                renamed[source["id"]] = prefix + suffix
                source["id"] = prefix + suffix
        self.mutate_payload(change)
        for anchor in self.annotations["protected"]:
            anchor["message"] = renamed.get(anchor["message"], anchor["message"])
        history, provenance = B.prepare(self.packet, self.annotations)
        self.assertEqual(history["messages"][1]["content"], "Vérifier avant de déployer <source>.")
        self.assertFalse(provenance["engine_admission"])
        self.assertFalse(provenance["provider_history_complete"])

    def test_cli_refuses_relabelled_source_without_creating_output(self):
        self.mutate_payload(lambda p: p["sources"][2].update(id="invented-answer-id"))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, annotations = root / "source.json", root / "annotations.json"
            source.write_text(json.dumps(self.packet)); annotations.write_text(json.dumps(self.annotations))
            before = (source.read_bytes(), annotations.read_bytes())
            result = subprocess.run([sys.executable, str(ROOT / "tools/memory-evaluation/swarm_source.py"),
                                     "--source", str(source), "--annotations", str(annotations),
                                     "--output-dir", str(root / "prepared")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
            self.assertFalse((root / "prepared").exists())
            self.assertEqual(before, (source.read_bytes(), annotations.read_bytes()))
            self.assertNotIn("Traceback", result.stderr)

    def test_new_current_task_snapshot_invalidates_previous_reference(self):
        history, _ = B.prepare(self.packet, self.annotations)
        parent = S.project(history, S.reference_candidate(history, 2))
        def change(p):
            snapshot = json.loads(p["sources"][0]["text"])
            snapshot["task"]["criteria"] = ["Changed current contract"]
            p["sources"][0]["text"] = json.dumps(snapshot)
            p["sources"][0]["sha256"] = B.digest(p["sources"][0]["text"].encode())
        self.mutate_payload(change)
        current, _ = B.prepare(self.packet, self.annotations)
        result = S.project(current, S.reference_candidate(current, 2, parent), parent)
        self.assertEqual(result["result"], "FALLBACK")
        self.assertEqual(result["recent_messages"], current["messages"])

    def test_does_not_mutate_inputs(self):
        before = copy.deepcopy((self.packet, self.annotations))
        history, provenance = B.prepare(self.packet, self.annotations)
        history["protected"][0]["state"] = "changed"
        history["scope"]["work"] = "changed"
        self.assertEqual(provenance["scope"]["work"], "w1")
        self.assertEqual(before, (self.packet, self.annotations))

    def test_cli_writes_new_private_bundle_and_preserves_sources(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            source, annotations = root / "source.json", root / "annotations.json"
            source.write_text(json.dumps(self.packet)); annotations.write_text(json.dumps(self.annotations))
            before = (source.read_bytes(), annotations.read_bytes())
            command = [sys.executable, str(ROOT / "tools/memory-evaluation/swarm_source.py"), "--source", str(source), "--annotations", str(annotations), "--output-dir", str(root / "prepared")]
            result = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            provenance = json.loads((root / "prepared/provenance.json").read_text())
            for filename, sha in provenance["files_sha256"].items():
                path = root / "prepared" / filename
                self.assertEqual(B.digest(path.read_bytes()), sha)
                self.assertEqual(os.stat(path).st_mode & 0o777, 0o600)
            self.assertEqual(os.stat(root / "prepared").st_mode & 0o777, 0o700)
            replay = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(replay.returncode, 2)
            self.assertEqual(before, (source.read_bytes(), annotations.read_bytes()))


if __name__ == "__main__":
    unittest.main(verbosity=2)
