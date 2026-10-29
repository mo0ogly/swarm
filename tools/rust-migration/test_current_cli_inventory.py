"""Static inventory integrity/refusal checks; no real CLI or live Root executes."""
from copy import deepcopy
import errno
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import current_cli_inventory as inventory


class Inventory(unittest.TestCase):
    """Owned text fixtures establish valid bindings before each poisoned case."""
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        for name in inventory.ANCHORS:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('owned static fixture\n')
        self.rows = [dict(surface='fixture-entry', actual_source='owned fixture', classification='wired', missing_contract='not executed', proof_needed='public journey', next_action='verify behavior', source_files=[inventory.ANCHORS[0]])]
        self.document = inventory.capture(self.rows, self.root)
        self.assertEqual(inventory.check(self.document, self.root), 1)

    def test_source_drift_refuses_without_rebinding_manifest(self):
        """A source change cannot silently refresh an earlier reviewed binding."""
        original = deepcopy(self.document)
        (self.root / inventory.ANCHORS[0]).write_text('changed source\n')
        with self.assertRaisesRegex(ValueError, 'stale'):
            inventory.check(self.document, self.root)
        self.assertEqual(original, self.document)

    def test_missing_binding_or_duplicate_row_refuses(self):
        """Missing hashes and duplicate source claims never become bound success."""
        wrong = deepcopy(self.document); wrong['sources'] = {}
        with self.assertRaisesRegex(ValueError, 'missing-or-stale'):
            inventory.check(wrong, self.root)
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            inventory.capture(self.rows + self.rows, self.root)

    def test_external_source_alias_refuses(self):
        """A repo-looking symlink cannot cause an outside source read."""
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'outside.rs'; target.write_text('outside private sentinel')
            path = self.root / inventory.ANCHORS[0]
            path.unlink(); path.symlink_to(target)
            with self.assertRaisesRegex(ValueError, 'external'):
                inventory.check(self.document, self.root)

    def test_malformed_source_list_is_a_controlled_refusal(self):
        """A malformed JSON source entry is rejected before source traversal."""
        wrong = deepcopy(self.rows); wrong[0]['source_files'] = [{}]
        with self.assertRaisesRegex(ValueError, 'source-list-invalid'):
            inventory.capture(wrong, self.root)

    def test_publication_failure_preserves_original_and_recovers(self):
        """No-replace and pre-link ENOSPC keep originals; a fresh retry can publish."""
        path = self.root / 'report.json'
        raw = json.dumps(self.document).encode()
        inventory.publish_new(path, raw)
        with self.assertRaises(FileExistsError):
            inventory.publish_new(path, b'replacement must refuse')
        self.assertEqual(path.read_bytes(), raw)
        other = self.root / 'new-report.json'; before = set(self.root.iterdir())
        with patch.object(inventory.os, 'link', side_effect=OSError(errno.ENOSPC, 'owned fault')):
            with self.assertRaises(OSError) as caught:
                inventory.publish_new(other, raw)
        self.assertEqual(caught.exception.errno, errno.ENOSPC)
        self.assertFalse(other.exists()); self.assertEqual(set(self.root.iterdir()), before)
        self.assertEqual(path.read_bytes(), raw)
        inventory.publish_new(other, raw)
        self.assertEqual(json.loads(other.read_bytes()), self.document)


    def test_postpublication_sync_and_close_failures_keep_both_causes(self):
        """A visible complete output is not rollback after directory-sync failure."""
        raw = json.dumps(self.document).encode()
        control = self.root / 'control.json'; inventory.publish_new(control, raw)
        path = self.root / 'visible-uncertain.json'
        real_close = inventory.os.close
        def failed_close(fd):
            real_close(fd)
            raise OSError(errno.EBADF, 'owned directory close fault')
        with patch.object(inventory.os, 'fsync', side_effect=[None, OSError(errno.EIO, 'owned directory sync fault')]), patch.object(inventory.os, 'close', side_effect=failed_close):
            with self.assertRaises(BaseExceptionGroup) as caught:
                inventory.publish_new(path, raw)
        self.assertEqual([error.errno for error in caught.exception.exceptions], [errno.EIO, errno.EBADF])
        self.assertEqual(path.read_bytes(), raw)
        self.assertEqual(control.read_bytes(), raw)


if __name__ == '__main__':
    unittest.main(verbosity=2)
