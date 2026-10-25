"""Source identity and local delivery numbering; isolated files, no provider."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import subprocess
import hashlib

SPEC = importlib.util.spec_from_file_location("swarm_version", Path(__file__).resolve().parents[1] / "tools/swarm_version.py")
version = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(version)


class LocalVersion(unittest.TestCase):
    def test_number_advances_only_for_changed_inputs(self):
        self.assertEqual(version.select_version("v0.1.0", "a", None), "v0.1.1-dev.1")
        prior = {"version": "v0.1.1-dev.1", "source_digest": "a"}
        self.assertEqual(version.select_version("v0.1.0", "a", prior), prior["version"])
        self.assertEqual(version.select_version("v0.1.0", "b", prior), "v0.1.1-dev.2")
        self.assertEqual(version.select_version("v0.2.0", "b", prior), "v0.2.1-dev.1")
        with self.assertRaises(ValueError):
            version.select_version("not-a-version", "b", prior)

    def test_failure_preserves_running_artifact(self):
        """Missing native build prerequisites cannot overwrite the installed binary."""
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);binary=root/'swarm';binary.write_bytes(b'previous verified binary')
            (root/'version_history.json').write_text(json.dumps({'releases':[], 'local_changes':{'summary':{'fr':'Oui','en':'Yes'},'items':[{'fr':'Note','en':'Note'}]}}))
            with self.assertRaises(FileNotFoundError):version.prepare(root,binary)
            self.assertEqual(binary.read_bytes(),b'previous verified binary')
            self.assertFalse((root/'deploy/local-build.json').exists())

    def test_malformed_local_notes_are_rejected_before_building(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "deploy").mkdir()
            binary = root / "swarm"
            binary.write_bytes(b"verified")
            state = {"version": "v0.1.1-dev.1", "source_digest": "a", "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
            (root / "deploy/local-build.json").write_text(json.dumps(state))
            valid = {"summary": {"fr": "Livraison", "en": "Delivery"}, "items": [{"fr": "Lecture", "en": "Reading"}]}
            manifest = root / "version_history.json"
            for invalid in ["wrong", [], {"items": "wrong"}, {"summary": {}, "items": [None]}, {"summary": {"fr": 3, "en": "Text"}, "items": valid["items"]}]:
                manifest.write_text(json.dumps({"releases": [{"version": "v0.1.0"}], "local_changes": invalid}))
                with self.assertRaisesRegex(ValueError,"local_changes"):version.prepare(root,binary)
                self.assertEqual(binary.read_bytes(),b"verified")



if __name__ == "__main__":
    unittest.main()
