#!/usr/bin/env python3
"""Source qualification guards on owned files; no Cargo/provider/live Root."""
import errno,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import verify_native_profile as profile


class NativeProfileInputs(unittest.TestCase):
    """New effective inputs and storage failures must not become a false PASS."""
    def setUp(self):
        """Create owned candidate inputs; no actual repository/Cargo mutation."""
        self.temporary=tempfile.TemporaryDirectory(prefix='native-profile-contract-');self.root=Path(self.temporary.name).resolve()
        for name,data in {'rust/Cargo.toml':'[workspace]\n','rust/Cargo.lock':'# fixture\n','rust/crates/engine/Cargo.toml':'[package]\n','rust/crates/engine/src/lib.rs':'const TEXT:&str=include_str!("../../../../docs/protocol.txt");','docs/protocol.txt':'complete fixture protocol','web/new.js':'// new effective embed','web/unused.js':'// not compiled'}.items():
            path=self.root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(data)
    def tearDown(self):
        """Remove only this test temporary tree; cleanup errors fail the case."""
        self.temporary.cleanup()
    def test_new_literal_and_web_input_prebound_without_prior_depfile(self):
        """New compiled embeds must be prebound; a missing candidate refuses."""
        before=profile.candidates(self.root)
        actual=profile.manifest(self.root,[self.root/'rust/crates/engine/src/lib.rs',self.root/'docs/protocol.txt',self.root/'web/new.js'])
        profile.effective_unchanged(self.root,before,actual)
        missing=dict(before);missing.pop('web/new.js')
        with self.assertRaisesRegex(ValueError,'effective-input-not-prebound'):profile.effective_unchanged(self.root,missing,actual)
    def test_used_change_refuses_but_unused_candidate_is_not_promoted(self):
        """Only effective compiled changes invalidate the recorded source proof."""
        before=profile.candidates(self.root);used=self.root/'web/new.js';unused=self.root/'web/unused.js';actual=profile.manifest(self.root,[used]);unused.write_text('// unrelated candidate changed')
        profile.effective_unchanged(self.root,before,actual)
        used.write_text('// used input changed')
        with self.assertRaisesRegex(ValueError,'effective-input-changed'):profile.effective_unchanged(self.root,before,profile.manifest(self.root,[used]))
    def test_effective_external_input_refuses(self):
        """An external effective path must refuse instead of disappearing."""
        with tempfile.TemporaryDirectory(prefix='external-profile-input-') as directory:
            external=Path(directory)/'outside.rs';external.write_text('// external source')
            with self.assertRaisesRegex(ValueError,'absent-or-external'):profile.manifest(self.root,[external])
    def test_enospc_before_rename_preserves_original(self):
        """Injected pre-rename disk failure preserves original fixture bytes."""
        path=self.root/'original';path.write_bytes(b'original evidence')
        with patch.object(profile.os,'fsync',side_effect=OSError(errno.ENOSPC,'fixture-full')):
            with self.assertRaises(OSError) as error:profile.publish(path,b'candidate')
        self.assertEqual(error.exception.errno,errno.ENOSPC);self.assertEqual(path.read_bytes(),b'original evidence')
    def test_directory_sync_failure_is_not_reported_as_rollback(self):
        """A failed post-rename sync cannot imply restoration of old bytes."""
        path=self.root/'original';path.write_bytes(b'original evidence')
        with patch.object(profile.os,'fsync',side_effect=[None,OSError(errno.EIO,'fixture-directory-sync')]):
            with self.assertRaises(OSError):profile.publish(path,b'candidate')
        self.assertEqual(path.read_bytes(),b'candidate')


if __name__=='__main__':unittest.main(verbosity=2)
