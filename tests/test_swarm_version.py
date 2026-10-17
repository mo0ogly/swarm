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

    def test_real_root_embed_and_effective_overlay_bytes_change_identity(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "web").mkdir()
            for name, text in [("resources.go", "package resources"), ("main.go", "package main"), ("web/notes.json", "{}")]:
                (root / name).write_text(text)
            packages = [{"Dir": str(root), "GoFiles": ["main.go", "resources.go"], "EmbedFiles": ["web/notes.json"]}]
            baseline = version.package_inputs(root, packages, {})
            self.assertIn("resources.go", baseline)
            self.assertIn("web/notes.json", baseline)
            for name in ["resources.go", "web/notes.json"]:
                original = (root / name).read_text()
                self.assertEqual(version.package_inputs(root, packages, {}), baseline, "valid control before each substitution")
                (root / name).write_text("changed")
                self.assertNotEqual(version.package_inputs(root, packages, {}), baseline)
                (root / name).write_text(original)
            replacement = root / "overlay.go"
            replacement.write_text("package alternate")
            overlay = {str(root / "main.go"): str(replacement)}
            self.assertEqual(version.package_inputs(root, packages, {}), baseline)
            first = version.package_inputs(root, packages, overlay)
            self.assertNotEqual(first, baseline)
            replacement.write_text("package second")
            self.assertNotEqual(version.package_inputs(root, packages, overlay), first)
            self.assertEqual(version.package_inputs(root, packages, {}), baseline)
            with self.assertRaises(ValueError):
                version.package_inputs(root, packages, {str(root / "main.go"): ""})

    def test_go_stream_is_parsed_without_assumed_package_paths(self):
        packages = [{"Dir": "/repo", "GoFiles": ["resources.go"]}, {"Dir": "/repo/cmd", "GoFiles": ["main.go"]}]
        stream = "\n".join(json.dumps(p) for p in packages)
        self.assertEqual(list(version.json_stream(stream)), packages)

    def test_failed_build_preserves_running_artifact_and_last_number(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "deploy").mkdir()
            notes = {"summary": {"fr": "Livraison", "en": "Delivery"}, "items": [{"fr": "Lecture", "en": "Reading"}]}
            (root / "version_history.json").write_text(json.dumps({"releases": [{"version": "v0.1.0"}], "local_changes": notes}))
            binary = root / "swarm"
            binary.write_bytes(b"previous verified binary")
            previous = {"version": "v0.1.1-dev.1", "source_digest": "a", "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
            state = root / "deploy/local-build.json"
            state.write_text(json.dumps(previous))
            with patch.object(version, "source_identity", return_value=("a", {})):
                self.assertEqual(version.prepare(root, binary)[0], binary, "valid reuse control")
            with patch.object(version, "source_identity", return_value=("b", {})), patch.object(version.subprocess, "run", side_effect=subprocess.CalledProcessError(1, ["build"])):
                with self.assertRaises(subprocess.CalledProcessError):
                    version.prepare(root, binary)
            self.assertEqual(binary.read_bytes(), b"previous verified binary")
            self.assertEqual(json.loads(state.read_text()), previous)
            self.assertEqual(list(root.glob("swarm.candidate-*")), [])

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
                manifest.write_text(json.dumps({"releases": [{"version": "v0.1.0"}], "local_changes": valid}))
                with patch.object(version, "source_identity", return_value=("a", {})):
                    self.assertEqual(version.prepare(root, binary)[0], binary)
                manifest.write_text(json.dumps({"releases": [{"version": "v0.1.0"}], "local_changes": invalid}))
                with self.assertRaisesRegex(ValueError, "local_changes"):
                    version.prepare(root, binary)
                self.assertEqual(binary.read_bytes(), b"verified")


if __name__ == "__main__":
    unittest.main()
