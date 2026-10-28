"""Regress added-source races without touching repository Go or executing Git/Go.

Creates an owned miniature tree and mocks immutable Git metadata. The first
valid witness establishes archive contents; a deterministic added-file race must
refuse before success publication, then the unchanged expanded tree can resume.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools/rust-migration'))
SPEC = importlib.util.spec_from_file_location('source_capture_test', ROOT / 'tools/rust-migration/capture_source_reference.py')
TOOL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOL)


class SourceReference(unittest.TestCase):
    """Archive a valid source fixture before exercising changed inventory refusal."""

    def test_valid_added_source_refusal_and_same_tree_recovery(self):
        """Adding a name midcapture must not silently shrink the captured scope."""
        with tempfile.TemporaryDirectory(prefix='source-reference-') as temporary:
            root = Path(temporary)
            (root / 'cmd').mkdir()
            (root / 'internal').mkdir()
            raw = b'package main\n'
            (root / 'cmd/main.go').write_bytes(raw)
            (root / 'go.mod').write_bytes(b'module fixture\n')
            (root / 'go.sum').write_bytes(b'')
            blob = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
            commit = '1' * 40
            tree = ('100644 blob ' + blob + '\tcmd/main.go\0').encode()
            cwd = Path.cwd()
            try:
                os.chdir(root)
                def git(argv, **kwargs):
                    """Return deterministic metadata only; unknown Git calls refuse."""
                    if argv[1] == 'rev-parse':
                        return commit + '\n'
                    if argv[1] == 'ls-tree':
                        return tree
                    raise AssertionError('unexpected Git call')
                with mock.patch.object(TOOL.subprocess, 'check_output', side_effect=git):
                    TOOL.collect(commit, root / 'valid')
                    good = json.loads((root / 'valid/result.json').read_bytes())
                    self.assertEqual(good['archive_members'], 3)
                    original = Path.read_bytes
                    def read_then_add(path):
                        """Inject one new source after enumeration, while reading go.sum."""
                        result = original(path)
                        if path.name == 'go.sum':
                            (root / 'internal/added.go').write_bytes(b'package added\n')
                        return result
                    with mock.patch.object(Path, 'read_bytes', new=read_then_add):
                        with self.assertRaisesRegex(ValueError, 'source inventory changed'):
                            TOOL.collect(commit, root / 'race')
                    self.assertFalse((root / 'race/result.json').exists())
                    TOOL.collect(commit, root / 'recovered')
                    recovered = json.loads((root / 'recovered/result.json').read_bytes())
                    self.assertEqual(recovered['counts']['added'], 1)
                    self.assertIn('internal/added.go', recovered['files'])
            finally:
                os.chdir(cwd)


if __name__ == '__main__':
    unittest.main()
