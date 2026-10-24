"""Historical tiny-Go archive fixtures only; native role guards live in test_check.py."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
import go_source_inputs as source



class GoSourceInputsTests(unittest.TestCase):
    """Preserve a failed private fixture; successful fixtures are regenerable."""
    def setUp(self):
        """Create a real local module with a selected embedded byte payload."""
        self.root = Path(tempfile.mkdtemp(prefix="swarm-go-input-test107-"))
        subprocess.run(["git", "init", "--quiet", str(self.root)], check=True)
        language = re.search(r"(?m)^go\s+([0-9.]+)", (ROOT / "go.mod").read_text()).group(1)
        (self.root / "go.mod").write_text("module fixture.local/source\n\ngo " + language + "\n")
        (self.root / "source.go").write_text('package source\nimport "embed"\n//go:embed assets/*\nvar Data embed.FS\n')
        (self.root / "source_test.go").write_text('package source\nimport "testing"\nfunc TestSource(t *testing.T) {}\n')
        (self.root / "assets").mkdir()
        (self.root / "assets/value.bin").write_bytes(b"real embedded fixture bytes")
        print("owned fixture:", self.root, flush=True)

    def tearDown(self):
        """Remove only this successful fixture; failures retain their location."""
        if self._outcome.result.wasSuccessful():
            shutil.rmtree(self.root)

    def complete(self, context):
        """Validate source stability only; this is not actual role-test evidence."""
        return source.finish(context, {"argv": ["fixture source validation"],
                                     "exit": 0, "scope": "fixture, no role test"}, True)

    def test_actual_embed_retained_and_verified_delta_replay(self):
        """Real Go discovery retains root source/embed; a delta inherits exact bytes."""
        first = source.prepare(self.root, self.root / "first", package=".")
        self.assertIn("source.go", first["before"]["project"])
        self.assertTrue(first["before"]["generated"])
        self.assertTrue(all(Path(v["source"]).is_file() for v in first["before"]["generated"].values()))
        self.assertIn("assets/value.bin", first["before"]["project"])
        self.assertEqual(self.complete(first)["status"], "PASS")
        second = source.prepare(self.root, self.root / "second", baseline=first["output"], package=".")
        self.assertEqual(json.loads((second["output"] / "source.json").read_text())["delta"], [])
        self.assertEqual(self.complete(second)["status"], "PASS")
        (self.root / "assets/new.bin").write_bytes(b"new actual Go embed")
        third = source.prepare(self.root, self.root / "third", baseline=first["output"], package=".")
        self.assertEqual(json.loads((third["output"] / "source.json").read_text())["delta"], ["assets/new.bin"])
        self.assertEqual(self.complete(third)["status"], "PASS")
        self.assertEqual(source.sources(self.root, third["output"]), source.retained_inputs(third["before"]))
        with self.assertRaises(FileExistsError):
            source.prepare(self.root, first["output"], package=".")
        self.assertEqual(json.loads((first["output"] / "result.json").read_text())["status"], "PASS")

    def test_changed_input_is_incomplete_and_restored_source_recovers(self):
        """Mutation after witness causes FAIL, preserving originals and fresh recovery."""
        witness = source.prepare(self.root, self.root / "witness", package=".")
        self.complete(witness)
        check = source.prepare(self.root, self.root / "changing", baseline=witness["output"], package=".")
        asset = self.root / "assets/value.bin"
        original = asset.read_bytes()
        asset.write_bytes(b"changed during check")
        with self.assertRaisesRegex(ValueError, "stability"):
            self.complete(check)
        self.assertEqual(json.loads((check["output"] / "result.json").read_text())["status"], "FAIL")
        self.assertEqual(asset.read_bytes(), b"changed during check")
        asset.write_bytes(original)
        recovered = source.prepare(self.root, self.root / "recovered", baseline=witness["output"], package=".")
        self.assertEqual(self.complete(recovered)["status"], "PASS")
        self.assertEqual(json.loads((check["output"] / "result.json").read_text())["status"], "FAIL")

    def test_bad_baseline_refuses_without_replacing_valid_source(self):
        """Tampered retained archive refuses; restore the exact fixture then recover."""
        witness = source.prepare(self.root, self.root / "witness", package=".")
        self.complete(witness)
        archive = witness["output"] / "source.tar.gz"
        original = archive.read_bytes()
        archive.write_bytes(b"fixture tampering")
        with self.assertRaisesRegex(ValueError, "archive-changed"):
            source.prepare(self.root, self.root / "refused", baseline=witness["output"], package=".")
        self.assertFalse((self.root / "refused/result.json").exists())
        self.assertEqual(archive.read_bytes(), b"fixture tampering")
        archive.write_bytes(original)
        recovered = source.prepare(self.root, self.root / "recovered", baseline=witness["output"], package=".")
        self.assertEqual(self.complete(recovered)["status"], "PASS")
        outside = self.root.parent / (self.root.name + "-outside")
        with self.assertRaisesRegex(ValueError, "outside-root"):
            source.prepare(self.root, outside, package=".")
        self.assertFalse(outside.exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
