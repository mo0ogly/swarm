import json
import unittest
from probe import canonical, decode, encode, measure


class TransportProbeTests(unittest.TestCase):
    def setUp(self):
        self.value = {"diff": '\t+é"\\\n' * 200, "report": "proof\n" * 100,
                      "tasks": [{"report": "proof\n" * 100}], "empty": "", "n": 3}

    def test_exact_utf8_and_duplicate_blocks(self):
        raw = encode(self.value)
        self.assertEqual(decode(raw), self.value)
        self.assertEqual(json.loads(raw.split(b"\n", 1)[0])["block_count"], 2)
        self.assertGreater(measure(self.value)["saved_bytes"], 0)

    def test_marker_inside_content_is_not_framing(self):
        value = {"source": '\n{"bytes":0,"sha256":"pretend"}\n' * 10}
        self.assertEqual(decode(encode(value)), value)

    def test_small_content_can_cost_more(self):
        self.assertLess(measure({"a": "short"})["saved_bytes"], 0)

    def test_corrupt_truncated_and_appended_bytes(self):
        raw = encode(self.value)
        for bad in (raw[:-1], raw + b"extra", raw.replace(b"proof", b"wrong", 1)):
            with self.assertRaises(ValueError):
                decode(bad)

    def test_duplicate_path_and_missing_reference(self):
        raw = encode(self.value)
        header, rest = raw.split(b"\n", 1)
        for mode in ("duplicate", "missing"):
            data = json.loads(header)
            if mode == "duplicate":
                data["replacements"].append(data["replacements"][0])
            else:
                data["replacements"].pop()
            with self.assertRaises(ValueError):
                decode(canonical(data) + b"\n" + rest)

    def test_whole_string_and_non_string_scalars(self):
        for value in ("whole string\n" * 100, [None, False, 4, {"$s": ""}]):
            self.assertEqual(decode(encode(value)), value)


if __name__ == "__main__":
    unittest.main()
