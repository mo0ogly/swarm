"""SDK dep-info must belong to the actual artifact, never a stale normal alias."""
import os,tempfile,unittest
from fixture_area import fixture_area
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
        with tempfile.TemporaryDirectory(dir=fixture_area()) as folder:
            a,hashed,dep,old=self.fixture(Path(folder));before=old.read_bytes();self.assertEqual(sdk_artifact_depfile(a),dep);self.assertEqual(old.read_bytes(),before)

    def test_ambiguous_same_inode_refuses_instead_of_picking_first(self):
        """One valid witness precedes a second matching hardlink fault."""
        with tempfile.TemporaryDirectory(dir=fixture_area()) as folder:
            a,hashed,dep,_=self.fixture(Path(folder));self.assertEqual(sdk_artifact_depfile(a),dep);os.link(hashed,hashed.parent/'swarm_rust-3c4d')
            with self.assertRaisesRegex(ValueError,'owner-missing-or-ambiguous'):sdk_artifact_depfile(a)

    def test_identical_bytes_without_inode_binding_do_not_supply_authority(self):
        """Same content alone cannot choose an adjacent depfile after unlink."""
        with tempfile.TemporaryDirectory(dir=fixture_area()) as folder:
            a,hashed,dep,_=self.fixture(Path(folder));self.assertEqual(sdk_artifact_depfile(a),dep);data=hashed.read_bytes();hashed.unlink();hashed.write_bytes(data)
            with self.assertRaisesRegex(ValueError,'owner-missing-or-ambiguous'):sdk_artifact_depfile(a)

    def test_test_and_library_paths_are_direct_and_missing_depinfo_refuses(self):
        """Existing hashed test/library behavior is preserved without a fallback."""
        with tempfile.TemporaryDirectory(dir=fixture_area()) as folder:
            root=Path(folder);test=root/'test-1a2b';test.write_bytes(b'test');dep=test.with_suffix('.d');dep.write_text('test: source.rs\n')
            a=dict(executable=str(test),profile=dict(test=True));self.assertEqual(sdk_artifact_depfile(a),dep)
            lib=root/'libengine-3c4d.rlib';lib.write_bytes(b'lib');ld=root/'engine-3c4d.d';ld.write_text('lib: source.rs\n');self.assertEqual(sdk_artifact_depfile(dict(executable=None,filenames=[str(lib)])),ld)
            dep.unlink()
            with self.assertRaisesRegex(ValueError,'depfile-missing'):sdk_artifact_depfile(a)


    def test_sdk_qualification_binds_config_and_refuses_changed_or_removed(self):
        """Drive the actual qualifier with explicitly synthetic Cargo metadata.

        Fresh retained files model one test artifact, dep-info and configuration;
        no compiler, binary, Root, provider or genuine SDK executes. Healthy
        controls verify archived config bytes before each mutation/removal and
        fresh recovery. Reader refusals must preserve the actual faulted input.
        """
        import json,sys,tarfile,hashlib
        repository=Path(__file__).resolve().parents[1]
        sys.path.insert(0,str(repository/'tools/verification'))
        try:
            import verify_native_profile as profile
            from rust_reviewer_claim_candidate import qualify_sdk
            declared=os.environ.get('SWARM_SDK_CONFIG_TEST_AREA')
            area=Path(declared) if declared else fixture_area()
            root=Path(tempfile.mkdtemp(prefix='sdk-config-contract-',dir=area))
            print('SDK_CONFIG_GUARD_ROOT '+str(root),flush=True)
            checkout=root/'checkout';package=checkout/'rust/crates/swarm-engine'
            (package/'src').mkdir(parents=True)
            source=package/'src/lib.rs';source.write_text('// synthetic source, never compiled\n')
            (checkout/'rust/Cargo.toml').write_text('[workspace]\nmembers=[]\n')
            (checkout/'rust/Cargo.lock').write_text('version=4\n')
            (package/'Cargo.toml').write_text('[package]\nname="swarm-engine"\nversion="0.0.0"\n')
            config=checkout/'.cargo/config.toml';config.parent.mkdir()
            config.write_text('[env]\nSYNTHETIC_CONFIG="unit fixture, never applied"\n')
            support=checkout/'tools/verification/cargo_space_guard.py';support.parent.mkdir(parents=True)
            support.write_text('# synthetic admission support; never executed\n')
            inputs={str(p.relative_to(checkout)):p.read_bytes() for p in (config,support)}
            before=profile.candidates(checkout)
            binary=root/'artifacts/swarm_engine-1a2b';binary.parent.mkdir()
            binary.write_bytes(b'SYNTHETIC UNIT ARTIFACT, NOT AN EXECUTABLE')
            binary.with_suffix('.d').write_text(str(binary)+': '+str(source)+'\n')
            event=dict(reason='compiler-artifact',features=[],profile=dict(test=True),
                       target=dict(name='swarm_engine',kind=['lib'],src_path=str(source)),
                       executable=str(binary),filenames=[str(binary)])
            (root/'context.json').write_text(json.dumps(dict(kind='SYNTHETIC_UNIT_ONLY',
                no_compiler_or_artifact_execution=True,before=before)))
            def invoke(label):
                """Retain one fresh qualifier output using fixed synthetic events."""
                out=root/label;out.mkdir()
                (out/'sdk.stdout').write_text(json.dumps(event)+'\n'+json.dumps(dict(reason='build-finished',success=True))+'\n')
                qualify_sdk(checkout,out,before)
                actual=json.loads((out/'sdk.sources.actual.json').read_text())
                for name,raw in inputs.items():
                    self.assertEqual(actual[name],hashlib.sha256(raw).hexdigest())
                with tarfile.open(out/'sdk.sources.tar.gz') as archive:
                    for name,raw in inputs.items():self.assertEqual(archive.extractfile(name).read(),raw)
                return out
            for name,raw in inputs.items():
                path=checkout/name
                for fault in ('changed','removed'):
                    label=str(path.name)+'-'+fault
                    invoke(label+'-control')
                    if fault=='changed':path.write_bytes(raw+b'# faulted configuration\n')
                    else:path.unlink()
                    observed=path.read_bytes() if path.exists() else None
                    with self.assertRaisesRegex(ValueError,'profile-cargo-config-changed-or-removed'):
                        invoke(label+'-refused')
                    self.assertEqual(path.read_bytes() if path.exists() else None,observed)
                    path.write_bytes(raw)
                    invoke(label+'-recovered')
                    print('SDK_CONFIG_CASE '+label+' control/refusal/recovery PASS',flush=True)
        finally:
            sys.path.pop(0)



if __name__=='__main__':unittest.main()
