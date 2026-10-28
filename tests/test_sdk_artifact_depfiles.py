"""SDK dep-info must belong to the actual artifact, never a stale normal alias."""
import os,tempfile,unittest
from pathlib import Path
from rust_reviewer_claim_candidate import sdk_artifact_depfile


class ArtifactDepfiles(unittest.TestCase):
    """Only owned synthetic artifact metadata; no Cargo or real cache mutation."""
    def fixture(self,root):
        """Create one current linked bin and a deliberately stale top-level .d."""
        debug=root/'debug';deps=debug/'deps';deps.mkdir(parents=True)
        hashed=deps/'swarm_rust-1a2b';hashed.write_bytes(b'current owned binary');binary=debug/'swarm-rust';os.link(hashed,binary)
        old=debug/'swarm-rust.d';old.write_text('old: /outside/old.rs\n')
        dep=hashed.with_suffix('.d');dep.write_text('current: current/main.rs\n')
        a=dict(executable=str(binary),profile=dict(test=False),target=dict(name='swarm-rust',kind=['bin']))
        return a,hashed,dep,old

    def test_current_inode_chooses_hashed_depfile_and_keeps_stale_original(self):
        """A valid current companion disproves reuse of the stale alias."""
        with tempfile.TemporaryDirectory() as folder:
            a,hashed,dep,old=self.fixture(Path(folder));before=old.read_bytes();self.assertEqual(sdk_artifact_depfile(a),dep);self.assertEqual(old.read_bytes(),before)

    def test_ambiguous_same_inode_refuses_instead_of_picking_first(self):
        """One valid witness precedes a second matching hardlink fault."""
        with tempfile.TemporaryDirectory() as folder:
            a,hashed,dep,_=self.fixture(Path(folder));self.assertEqual(sdk_artifact_depfile(a),dep);os.link(hashed,hashed.parent/'swarm_rust-3c4d')
            with self.assertRaisesRegex(ValueError,'owner-missing-or-ambiguous'):sdk_artifact_depfile(a)

    def test_identical_bytes_without_inode_binding_do_not_supply_authority(self):
        """Same content alone cannot choose an adjacent depfile after unlink."""
        with tempfile.TemporaryDirectory() as folder:
            a,hashed,dep,_=self.fixture(Path(folder));self.assertEqual(sdk_artifact_depfile(a),dep);data=hashed.read_bytes();hashed.unlink();hashed.write_bytes(data)
            with self.assertRaisesRegex(ValueError,'owner-missing-or-ambiguous'):sdk_artifact_depfile(a)

    def test_test_and_library_paths_are_direct_and_missing_depinfo_refuses(self):
        """Existing hashed test/library behavior is preserved without a fallback."""
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);test=root/'test-1a2b';test.write_bytes(b'test');dep=test.with_suffix('.d');dep.write_text('test: source.rs\n')
            a=dict(executable=str(test),profile=dict(test=True));self.assertEqual(sdk_artifact_depfile(a),dep)
            lib=root/'libengine-3c4d.rlib';lib.write_bytes(b'lib');ld=root/'engine-3c4d.d';ld.write_text('lib: source.rs\n');self.assertEqual(sdk_artifact_depfile(dict(executable=None,filenames=[str(lib)])),ld)
            dep.unlink()
            with self.assertRaisesRegex(ValueError,'depfile-missing'):sdk_artifact_depfile(a)


if __name__=='__main__':unittest.main()
