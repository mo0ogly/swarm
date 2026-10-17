"""Local preparation never treats an M17 receipt as engine authority."""
import copy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from test_memory_swarm_source import packet, ROOT
sys.path.insert(0, str(ROOT / "tools/memory-evaluation"))
import admission as A
import summarization as S
import swarm_source as B


class SummaryAdmission(unittest.TestCase):
    def setUp(self):
        self.source, self.annotations = packet()
        self.history, _ = B.prepare(self.source, self.annotations)
        self.reference = S.project(self.history, S.reference_candidate(self.history, 2))

    def test_prepares_declarative_selection_without_operator_text_or_authority(self):
        result = A.selection(self.source, self.annotations, self.reference, "Operator choice")
        self.assertEqual(result["source_sha256"], self.source["sha256"])
        self.assertEqual(result["cutoff"], 2)
        self.assertEqual(result["dialogues"][0]["attempt"], "a1")
        self.assertNotIn("work_criteria", json.dumps(result))
        self.assertNotIn("text", result)
        result["protected"][0]["state"] = "changed"
        self.assertEqual(self.annotations["protected"][0]["state"], "open")

    def test_rejects_changed_unrelated_or_fallback_receipt(self):
        for mutation in [lambda r:r.update(receipt_sha256="a"*64),
                         lambda r:r.update(source_sha256="b"*64),
                         lambda r:r.update(result="FALLBACK"),
                         lambda r:r["summary_entries"][1].update(state="historical"),
                         lambda r:r.update(parent_sha256="c"*64),
                         lambda r:r.update(extra=True)]:
            reference = copy.deepcopy(self.reference)
            mutation(reference)
            with self.assertRaises(ValueError):
                A.selection(self.source, self.annotations, reference, "Operator choice")

    def test_requires_reason_and_a_protected_dialogue_in_prefix(self):
        with self.assertRaises(ValueError):
            A.selection(self.source, self.annotations, self.reference, " ")
        reference = S.project(self.history, S.reference_candidate(self.history, 1))
        with self.assertRaises(ValueError):
            A.selection(self.source, self.annotations, reference, "Operator choice")

    def test_cli_preserves_inputs_private_output_and_refuses_replacement(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            names = ["source", "annotations", "reference"]
            for name, value in zip(names, [self.source, self.annotations, self.reference]):
                (root / name).write_text(json.dumps(value))
            before = {name:(root/name).read_bytes() for name in names}
            command = [sys.executable, str(ROOT/"tools/memory-evaluation/admission.py")]
            for name in names:command += ["--"+name,str(root/name)]
            command += ["--reason","Operator choice","--output",str(root/"choice")]
            result = subprocess.run(command,capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertEqual(json.loads(result.stdout)["engine_admission"],False)
            choice = (root/"choice").read_bytes()
            self.assertEqual(os.stat(root/"choice").st_mode & 0o777,0o600)
            result = subprocess.run(command,capture_output=True,text=True)
            self.assertEqual(result.returncode,2)
            self.assertEqual((root/"choice").read_bytes(),choice)
            self.assertEqual({name:(root/name).read_bytes() for name in names},before)

    def test_cli_invalid_json_is_an_explicit_error_without_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory);(root/"bad").write_text('{"x":1,"x":2}')
            result = subprocess.run([sys.executable,str(ROOT/"tools/memory-evaluation/admission.py"),"--source",str(root/"bad"),"--annotations",str(root/"missing"),"--reference",str(root/"missing"),"--reason","Choice","--output",str(root/"output")],capture_output=True,text=True)
            self.assertEqual(result.returncode,2)
            self.assertFalse((root/"output").exists())
            self.assertNotIn("Traceback",result.stderr)

    def relabelled_source(self):
        source, annotations, history = copy.deepcopy((self.source, self.annotations, self.history))
        payload = B.parse(source["payload"])
        payload["sources"][2]["id"] = "invented-answer-id"
        source["payload"] = json.dumps(payload, ensure_ascii=False)
        source["sha256"] = B.digest(source["payload"].encode())
        annotations["source_sha256"] = source["sha256"]
        history["messages"][2]["id"] = "invented-answer-id"
        reference = S.project(history, S.reference_candidate(history, 2))
        return source, annotations, reference

    def test_selection_refuses_relabelled_source_even_with_matching_reference(self):
        with self.assertRaises(ValueError):
            A.selection(*self.relabelled_source(), "Operator choice")

    def test_cli_refuses_relabelled_source_without_output_or_input_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            names = ["source", "annotations", "reference"]
            for name, value in zip(names, self.relabelled_source()):
                (root / name).write_text(json.dumps(value))
            before = {name: (root / name).read_bytes() for name in names}
            command = [sys.executable, str(ROOT / "tools/memory-evaluation/admission.py")]
            for name in names:
                command += ["--" + name, str(root / name)]
            command += ["--reason", "Operator choice", "--output", str(root / "choice")]
            result = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
            self.assertFalse((root / "choice").exists())
            self.assertEqual(before, {name: (root / name).read_bytes() for name in names})
            self.assertNotIn("Traceback", result.stderr)


if __name__ == "__main__":unittest.main(verbosity=2)
