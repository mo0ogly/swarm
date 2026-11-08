"""Evidence ownership guard: an existing run must remain byte-for-byte untouched."""
import subprocess
import sys
import tempfile
import sys as _fa_sys
from pathlib import Path as _fa_Path
_fa_sys.path.insert(0, str(_fa_Path(__file__).resolve().parents[2] / 'tests'))
from fixture_area import fixture_area
import unittest
from pathlib import Path


class Retention(unittest.TestCase):
    def test_existing_output_never_receives_a_new_failure_file(self):
        """Refuse before Go execution without changing an earlier owned proof."""
        with tempfile.TemporaryDirectory(dir=fixture_area()) as directory:
            root = Path(directory)
            output = root / 'old'
            output.mkdir()
            marker = output / 'original.json'
            marker.write_bytes(b'{"retained":true}\n')
            inputs = root / 'input.json'
            inputs.write_text('[]')
            before = {p.name: p.read_bytes() for p in output.iterdir()}
            result = subprocess.run([sys.executable,
                str(Path(__file__).with_name('cockpit_post_delivery_oracle.py')),
                '--inputs', str(inputs), '--output', str(output),
                '--check-fixture', str(root / 'unused.json')], capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b'FileExistsError', result.stderr)
            self.assertEqual({p.name: p.read_bytes() for p in output.iterdir()}, before)

    def test_unit_runner_preserves_existing_output_before_build_or_replay(self):
        """Both unit modes reject an old output before consuming any record."""
        with tempfile.TemporaryDirectory(dir=fixture_area()) as directory:
            root = Path(directory)
            output = root / 'old'
            output.mkdir()
            (output / 'original').write_bytes(b'original')
            for flag in ['--profile-invocation', '--replay-record']:
                result = subprocess.run([sys.executable,
                    str(Path(__file__).with_name('run_cockpit_oracle.py')),
                    flag, str(root / 'unused.json'), '--output', str(output)],
                    capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(b'FileExistsError', result.stderr)
                self.assertEqual({p.name: p.read_bytes() for p in output.iterdir()},
                                 {'original': b'original'})


if __name__ == '__main__':
    unittest.main()
