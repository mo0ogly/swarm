import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from token_probe import measure


class TokenProbeTests(unittest.TestCase):
    def test_ascii_tokens_are_not_bytes(self):
        result = measure(b"hello world", "gpt-5.6-sol")
        self.assertEqual(result["text_tokens"], 2)
        self.assertEqual(result["utf8_bytes"], 11)
        self.assertEqual(result["encoding"], "o200k_base")
        self.assertFalse(result["provider_admission_authorized"])

    def test_unicode_and_literal_special_tokens_remain_data(self):
        raw = "é 中文 👩‍💻 <|endoftext|>\n".encode()
        result = measure(raw, "gpt-5.6-sol")
        self.assertTrue(result["roundtrip_exact"])
        self.assertEqual(result["sha256"], hashlib.sha256(raw).hexdigest())
        self.assertEqual(result["utf8_bytes"], len(raw))

    def test_no_guess_for_unknown_model(self):
        with self.assertRaises(KeyError):
            measure(b"proof", "unknown-provider-model")

    def test_invalid_utf8_is_not_replaced(self):
        with self.assertRaises(UnicodeError):
            measure(b"\xff", "gpt-5.6-sol")

    def test_cli_does_not_emit_evidence(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "input.txt"
            path.write_text("private-evidence-marker\n")
            result = subprocess.run(
                [sys.executable, str(Path(__file__).with_name("token_probe.py")),
                 str(path), "--model", "gpt-5.6-sol"],
                capture_output=True, text=True, check=True)
            self.assertNotIn("private-evidence-marker", result.stdout + result.stderr)
            self.assertFalse(json.loads(result.stdout)["provider_admission_authorized"])

    def test_cli_refuses_invalid_text_without_echoing_it(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "input.txt"
            path.write_bytes(b"private-evidence-marker\xff")
            result = subprocess.run(
                [sys.executable, str(Path(__file__).with_name("token_probe.py")),
                 str(path), "--model", "gpt-5.6-sol"],
                capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(result.stdout, "")
            self.assertNotIn("private-evidence-marker", result.stderr)


if __name__ == "__main__":
    unittest.main()
