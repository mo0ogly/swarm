"""Artifact-integrity tests only: synthetic bytes are not engine/SDK receipts.

No Root, provider, Go, process, deadline or acceptance. Temporary file fixtures
exercise archival read guards independently of the lifetime of an actual Root.
"""
import json
from pathlib import Path
import tempfile
import unittest
import rust_review_required_receipt_guard as guard


class RetainedEvidence(unittest.TestCase):
    """Healthy byte manifests before each corruption, path or missing-input fault."""
    def setUp(self):
        """Create synthetic archival witnesses in a private temporary directory."""
        self.temp=tempfile.TemporaryDirectory(prefix='review-archive-integrity-')
        self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)
        names=['owned-review-process-observation.json','owned-review-exec.stdout','owned-review-exec.stderr','reviewer-invoked.json']
        self.files={name:guard.sdk.f.digest(name.encode()) for name in names}
        for name in names:(self.root/name).write_bytes(name.encode())
        self.manifest(self.files)
        self.assertEqual(guard.retained_evidence(self.root)['files'],self.files)

    def manifest(self,files):
        """Write only a synthetic integrity fixture, never an execution attestation."""
        (self.root/'process-evidence-retention.json').write_text(json.dumps(dict(status='PASS',files=files)))

    def test_archived_bytes_need_no_live_root(self):
        """Only the copied bytes and manifest are needed; no Root path exists."""
        self.assertEqual(guard.retained_evidence(self.root)['status'],'PASS')

    def test_changed_original_refuses(self):
        """A single changed witness must fail instead of inheriting the PASS label."""
        (self.root/'owned-review-exec.stdout').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'retained-process-bytes-changed'):
            guard.retained_evidence(self.root)

    def test_missing_required_witness_refuses(self):
        """Missing named observations cannot be replaced by other valid files."""
        files=dict(self.files);files.pop('reviewer-invoked.json');self.manifest(files)
        with self.assertRaisesRegex(ValueError,'retained-process-witness-missing'):
            guard.retained_evidence(self.root)

    def test_escape_and_symlink_refuse(self):
        """Reject path escape before reading and links even within this fixture."""
        files={**self.files,'../outside':'0'*64};self.manifest(files)
        with self.assertRaisesRegex(ValueError,'retained-process-path-invalid'):
            guard.retained_evidence(self.root)
        self.manifest(self.files);guard.retained_evidence(self.root)
        target='owned-review-exec.stdout';(self.root/'link').symlink_to(target)
        self.manifest({**self.files,'link':self.files[target]})
        with self.assertRaisesRegex(ValueError,'retained-process-path-invalid'):
            guard.retained_evidence(self.root)


if __name__=='__main__':unittest.main()
