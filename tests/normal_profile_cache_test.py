"""A previous Cargo depfile cannot authorize an undeclared old snapshot input.

Synthetic compiler-input fixtures only: no Cargo run, native permission, SQL or
positive process receipt. The real moved-cache counterexample is retained in170.
"""
from pathlib import Path
import tempfile,sys,unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools/verification'))
import verify_native_profile as profile


class ProfileCandidateAuthority(unittest.TestCase):
    """Valid current include precedes an old nested snapshot dependency fault."""
    def test_prior_depfile_does_not_promote_stale_snapshot_input(self):
        """Only declared current producers prebind bytes; prior output is no input authority.

        A legitimate declared include remains accepted with/without a prior file.
        Rejected stale paths stay preserved for diagnosis, never silently omitted
        from actual dependencies or admitted merely because an old build used them.
        """
        with tempfile.TemporaryDirectory(prefix='profile-prior-authority-') as tmp:
            root=Path(tmp);source=root/'rust/crates/example/src';source.mkdir(parents=True)
            for name,body in [('rust/Cargo.toml','[workspace]\n'),('rust/Cargo.lock','# synthetic\n'),('rust/crates/example/Cargo.toml','[package]\nname="example"\nversion="0.1.0"\n'),('rust/crates/example/src/lib.rs','const CONTRACT: &str = include_str!("contract.txt");\n'),('rust/crates/example/src/contract.txt','current declared producer\n')]:
                (root/name).write_text(body)
            current='rust/crates/example/src/contract.txt';actual={current:profile.digest(root/current)}
            healthy=profile.candidates(root);profile.effective_unchanged(root,healthy,actual)
            dep=root/'prior.d';dep.write_text('synthetic: '+str(root/current)+'\n')
            with_prior=profile.candidates(root,dep);profile.effective_unchanged(root,with_prior,actual)
            old=root/'.cache/old-snapshot/contract.txt';old.parent.mkdir(parents=True);old.write_text('old snapshot bytes\n')
            dep.write_text('synthetic: '+str(old)+'\n');raw=dep.read_bytes();bound=profile.candidates(root,dep)
            with self.assertRaisesRegex(ValueError,'profile-effective-input-not-prebound'):
                profile.effective_unchanged(root,bound,{str(old.relative_to(root)):profile.digest(old)})
            self.assertEqual(dep.read_bytes(),raw);self.assertEqual(old.read_text(),'old snapshot bytes\n')


class ProfileCargoConfiguration(unittest.TestCase):
    """Local Cargo config is a real input even when rustc depfiles omit it."""
    def setUp(self):
        """Construct a minimal synthetic workspace; never launch Cargo."""
        temporary=tempfile.TemporaryDirectory(prefix='profile-config-inputs-')
        self.addCleanup(temporary.cleanup);self.root=Path(temporary.name)
        (self.root/'rust/crates/example/src').mkdir(parents=True)
        for name,body in [('rust/Cargo.toml','[workspace]\n'),('rust/Cargo.lock','# fixture\n'),
                          ('rust/crates/example/Cargo.toml','[package]\nname="example"\nversion="0.1.0"\n'),
                          ('rust/crates/example/src/lib.rs','// synthetic\n')]:
            (self.root/name).write_text(body)
        self.depfile=self.root/'synthetic.d'
        self.depfile.write_text('synthetic: '+str(self.root/'rust/crates/example/src/lib.rs')+'\n')
        self.config=self.root/'.cargo/config.toml';self.config.parent.mkdir()
        self.config_bytes=b'[env]\nLIBSQLITE3_FLAGS={value="-DSQLITE_ENABLE_FILESTAT",force=true}\n'

    def test_config_prebound_effective_drift_removal_and_recovery(self):
        """A valid control precedes config changes, removal and restoration."""
        self.config.write_bytes(self.config_bytes)
        before=profile.candidates(self.root)
        self.assertIn('.cargo/config.toml',before)
        actual=profile.dependencies(self.root,self.depfile)
        self.assertEqual(actual['.cargo/config.toml'],profile.digest(self.config))
        profile.effective_unchanged(self.root,before,actual)
        self.config.write_bytes(b'[env]\nLIBSQLITE3_FLAGS="different"\n')
        with self.assertRaises(ValueError):
            profile.effective_unchanged(self.root,before,profile.dependencies(self.root,self.depfile))
        self.config.unlink()
        with self.assertRaises(ValueError):
            profile.effective_unchanged(self.root,before,profile.dependencies(self.root,self.depfile))
        self.config.write_bytes(self.config_bytes)
        profile.effective_unchanged(self.root,before,profile.dependencies(self.root,self.depfile))

    def test_absent_config_legacy_compatible_but_late_appearance_refused(self):
        """Historical no-config builds work; adding config after binding refuses."""
        before=profile.candidates(self.root)
        profile.effective_unchanged(self.root,before,profile.dependencies(self.root,self.depfile))
        self.assertNotIn('.cargo/config.toml',before)
        self.config.write_bytes(self.config_bytes)
        with self.assertRaises(ValueError):
            profile.effective_unchanged(self.root,before,profile.dependencies(self.root,self.depfile))

    def test_config_symlink_refuses_without_promoting_an_aliased_manifest_key(self):
        """An internal config alias is not silently accepted as a different key."""
        target=self.config.parent/'retained-config';target.write_bytes(self.config_bytes)
        self.config.symlink_to(target.name)
        with self.assertRaisesRegex(ValueError,'config-symlink-unsupported'):
            profile.candidates(self.root)
        self.config.unlink()
        self.config.write_bytes(self.config_bytes)
        self.assertIn('.cargo/config.toml',profile.candidates(self.root))

    def test_invocation_requires_exact_config_and_declared_environment_when_present(self):
        """Legacy record stays valid; present config cannot omit its provenance."""
        self.assertEqual(profile.verify_config_invocation(self.root,{}),{})
        self.config.write_bytes(self.config_bytes)
        with self.assertRaisesRegex(ValueError,'invocation-config-mismatch'):
            profile.verify_config_invocation(self.root,{})
        declared={'.cargo/config.toml':profile.digest(self.config)}
        call={'cargo_config_inputs':declared,'cargo_environment_before_config':{'LIBSQLITE3_FLAGS':None}}
        self.assertEqual(profile.verify_config_invocation(self.root,call),declared)
        with self.assertRaisesRegex(ValueError,'environment-missing-or-invalid'):
            profile.verify_config_invocation(self.root,{'cargo_config_inputs':declared})
        self.config.write_bytes(b'# replaced config\n')
        with self.assertRaisesRegex(ValueError,'invocation-config-mismatch'):
            profile.verify_config_invocation(self.root,call)


if __name__=='__main__':unittest.main()
