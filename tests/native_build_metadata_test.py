"""Native candidate invocation metadata guards; fixtures perform no Cargo build."""
import importlib.util,sys,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools/verification'))
import verify_native_profile as profile
spec=importlib.util.spec_from_file_location('native_build',Path(__file__).resolve().parents[1]/'tools/rust-migration/build_native.py')
build=importlib.util.module_from_spec(spec);spec.loader.exec_module(build)

class MetadataBinding(unittest.TestCase):
    """Hash exact declarations only, distinguish absence and reject false releases."""
    def test_metadata_presence_hash_and_private_values(self):
        """Valid declared input precedes empty/private/absent variants; no raw value."""
        import hashlib,json
        raw='{"schema_version":1,"version":"devel"}'
        self.assertEqual(profile.build_metadata_binding({'SWARM_NATIVE_BUILD_METADATA':raw}),dict(present=True,sha256=hashlib.sha256(raw.encode()).hexdigest()))
        self.assertNotEqual(profile.build_metadata_binding({}),profile.build_metadata_binding({'SWARM_NATIVE_BUILD_METADATA':''}))
        self.assertNotIn('PRIVATE',json.dumps(profile.build_metadata_binding({'SWARM_NATIVE_BUILD_METADATA':'PRIVATE'})))
        self.assertEqual(profile.build_metadata_binding({'OTHER':'PRIVATE'}),dict(present=False,sha256=None))
    def test_collector_signal_is_retained_before_refusal(self):
        """A simulated killed collector refuses; its signal and channels survive.

        Owned temporary paths and mocked subprocess/observations perform no
        Cargo build, service change or provider call.
        """
        import json,tempfile,subprocess
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);out=root/'proof'
            with patch.object(build,'observe',return_value=dict(commit='a'*40,modified=True)),patch.object(build.subprocess,'run',return_value=subprocess.CompletedProcess([], -9)):
                with self.assertRaisesRegex(ValueError,'native-build-profile-failed'):
                    build.build(root,out,root/'target','devel')
            result=json.loads((out/'collector-result.json').read_text())
            self.assertEqual(result['returncode'],-9)
            self.assertEqual(result['signal'],9)
            self.assertEqual(result['stdout_sha256'],result['stderr_sha256'])
            self.assertTrue(json.loads((out/'failure.json').read_text())['candidate_unqualified'])
    def test_release_label_matches_legacy_prerelease_rules(self):
        """Labels validate syntax, never imply a real tag/release or repository."""
        for value in ['devel','v0.0.0','v1.2.3-dev.0','v1.2.3+build.01']:self.assertTrue(build.release(value),value)
        for value in ['v01.2.3','1.2.3','v1.2.3-01','v1.2.3+','PRIVATE']:self.assertFalse(build.release(value),value)

if __name__=='__main__':unittest.main()
