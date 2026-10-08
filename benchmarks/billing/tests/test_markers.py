import tempfile
import unittest
from pathlib import Path

from bench import markers


class MarkersTest(unittest.TestCase):
    def test_mark_keeps_timestamp_and_leaves_no_temporary_file(self):
        with tempfile.TemporaryDirectory() as d:
            markers.mark(Path(d) / "f1.json", fault="F1", at="faux")
            found = markers.read_all(d)
            self.assertEqual(list(found), ["f1"])
            self.assertEqual(found["f1"]["fault"], "F1")
            self.assertIsInstance(found["f1"]["at"], float)
            self.assertEqual(sorted(p.name for p in Path(d).iterdir()), ["f1.json"])
