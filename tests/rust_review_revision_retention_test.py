"""Pure revision-replay retention guards on synthetic files, never engine receipts."""
import json,tempfile,unittest
from pathlib import Path
import rust_review_revision_continuity as replay


class RevisionRetention(unittest.TestCase):
    """Healthy copies precede marker, symlink and missing-original failures."""
    def setUp(self):
        """Own a synthetic Root marker and one ordinary source, without a database."""
        temp=tempfile.TemporaryDirectory(prefix='review-revision-retention-');self.addCleanup(temp.cleanup)
        self.base=Path(temp.name);self.root=self.base/'root';self.root.mkdir();(self.root/'.swarm').mkdir();self.out=self.base/'proof';self.out.mkdir()
        self.marker=self.root/'.swarm/owned-review-claim-fixture';self.marker.write_text('synthetic-token')
        self.name='owned-review-revision-observation.json';self.source=self.root/self.name;self.source.write_bytes(b'{"synthetic":true}')
        self.witness=replay.retain(self.root,self.out,'synthetic-token',required=False)

    def test_regular_copy_and_hash(self):
        """Copied original bytes/hash match; retention alone creates no success marker."""
        self.assertEqual((self.out/self.name).read_bytes(),self.source.read_bytes())
        self.assertEqual(self.witness[self.name],replay.sdk.f.digest(self.source.read_bytes()))
        self.assertFalse((self.out/'result.json').exists())

    def test_changed_marker_refuses_without_replacing_original(self):
        """A foreign marker refuses before collection and preserves prior proof."""
        original=(self.out/self.name).read_bytes();self.marker.write_text('different-synthetic-token')
        with self.assertRaisesRegex(ValueError,'owned-revision-marker-changed'):
            replay.retain(self.root,self.out,'synthetic-token',required=False)
        self.assertEqual((self.out/self.name).read_bytes(),original)

    def test_symlink_original_refuses_after_healthy_witness(self):
        """A fixed source name may not resolve outside the fixture through a symlink."""
        original=(self.out/self.name).read_bytes();other=self.base/'outside';other.write_bytes(b'never collected')
        self.source.unlink();self.source.symlink_to(other)
        with self.assertRaisesRegex(ValueError,'revision-original-not-contained-regular-file'):
            replay.retain(self.root,self.out,'synthetic-token',required=False)
        self.assertEqual((self.out/self.name).read_bytes(),original)

    def test_required_missing_refuses_without_success_marker(self):
        """A partial copy cannot satisfy successful-SDK evidence requirements."""
        with self.assertRaisesRegex(ValueError,'required-revision-original-missing'):
            replay.retain(self.root,self.out,'synthetic-token',required=True)
        self.assertFalse((self.out/'result.json').exists())


if __name__=='__main__':unittest.main()
