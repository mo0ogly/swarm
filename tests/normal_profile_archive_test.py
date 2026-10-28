"""Effective-source archive guards on synthetic bytes, without Cargo or a Root."""
from pathlib import Path
import json,sys,tarfile,tempfile,unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools/verification'))
import verify_native_profile as profile


class EffectiveSourceArchive(unittest.TestCase):
    """A valid archive witness precedes source drift and its explicit recovery."""
    def setUp(self):
        """Create only two synthetic compiled-input stand-ins and private outputs."""
        temp=tempfile.TemporaryDirectory(prefix='native-profile-archive-');self.addCleanup(temp.cleanup)
        self.root=Path(temp.name);(self.root/'one.rs').write_bytes(b'synthetic source one')
        (self.root/'asset.txt').write_bytes(b'synthetic exact asset')
        self.inputs={name:profile.digest(self.root/name) for name in ['one.rs','asset.txt']}
        self.out=self.root/'witness';self.out.mkdir()
        self.proof=profile.archive_effective_sources(self.root,self.out,self.inputs)

    def test_archive_has_exact_original_members(self):
        """Every expected byte must be readable from the archive with its hash."""
        self.assertEqual(self.proof['inputs'],2)
        with tarfile.open(self.out/'effective-sources.tar.gz') as archive:
            self.assertEqual(set(archive.getnames()),set(self.inputs))
            for name,sha in self.inputs.items():
                self.assertEqual(profile.hashlib.sha256(archive.extractfile(name).read()).hexdigest(),sha)

    def test_source_drift_refuses_without_replacing_witness_then_recovers(self):
        """A stale input map refuses; only a new verified precondition permits retry."""
        original=(self.out/'effective-sources.tar.gz').read_bytes();changed=self.root/'one.rs';changed.write_bytes(b'changed synthetic source')
        failed=self.root/'failed';failed.mkdir()
        with self.assertRaisesRegex(ValueError,'profile-archive-source-changed'):
            profile.archive_effective_sources(self.root,failed,self.inputs)
        self.assertEqual((self.out/'effective-sources.tar.gz').read_bytes(),original)
        self.assertFalse((failed/'effective-sources.tar.gz').exists())
        changed.write_bytes(b'synthetic source one')
        resumed=self.root/'resumed';resumed.mkdir()
        self.assertTrue(profile.archive_effective_sources(self.root,resumed,self.inputs)['all_members_verified'])


    def test_checkout_cargo_config_is_retained_as_exact_effective_bytes(self):
        """Configuration must survive independently of the mutable source root."""
        config=self.root/'.cargo/config.toml';config.parent.mkdir()
        raw=b'[env]\nLIBSQLITE3_FLAGS={value="-DSQLITE_ENABLE_FILESTAT",force=true}\n'
        config.write_bytes(raw)
        inputs={**self.inputs,'.cargo/config.toml':profile.digest(config)}
        out=self.root/'config-witness';out.mkdir()
        profile.archive_effective_sources(self.root,out,inputs)
        config.unlink()
        with tarfile.open(out/'effective-sources.tar.gz') as archive:
            self.assertEqual(archive.extractfile('.cargo/config.toml').read(),raw)
            self.assertEqual(set(archive.getnames()),set(inputs))


if __name__=='__main__':unittest.main()
