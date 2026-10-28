"""Exercise launcher's input boundary with an inert, explicitly fake compiler.

The real source_identity implementation hashes temporary files/metadata. No Cargo,
provider, server restart or live Root. A fake probe is never runtime qualification.
"""
import hashlib,importlib.util,json,os,tempfile,unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
from rust_agent_model_route import publish

SPEC=importlib.util.spec_from_file_location('candidate_version',Path(__file__).resolve().parents[1]/'tools/swarm_version.py')
version=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(version)


def sha(path):
    """Hash complete fixture bytes; no file/path is treated as authority."""
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class CandidateBoundary(unittest.TestCase):
    """Mock only compilation/system observations; assert real prepare decisions."""
    def exercise(self,change=None):
        """Prepare an owned temporary artifact and return build/cache observations.

        Actual test files never execute. Mutations model a concurrent writer after
        a qualified profile snapshot; refusals must leave old artifact/state intact.
        """
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);binary=root/'swarm';publish(binary,b'old artifact')
            files=['build.sh','tools/swarm_version.py','tools/swarm_local.py','tools/rust-migration/build_native.py','tools/verification/verify_native_profile.py','actual.rs','unused.rs']
            for name in files:
                f=root/name;f.parent.mkdir(parents=True,exist_ok=True);publish(f,b'original')
            notes={'summary':{'fr':'Note','en':'Note'},'items':[{'fr':'Note','en':'Note'}]}
            publish(root/'version_history.json',{'releases':[{'version':'v0.1.0'}],'local_changes':notes})
            facts={'commit':'a'*40,'cargo':'fixture-cargo','rustc':'fixture-rustc','dirty':'M fixture'};builds=[]
            def output(argv,**kwargs):
                """Supply fixed system facts; a changed fact must fail delivery."""
                key='commit' if argv[:2]==['git','rev-parse'] else 'dirty' if argv[:2]==['git','status'] else argv[0]
                return facts[key]+'\n'
            def build(source,out,target,label):
                """Retain an inert actual-input snapshot, then inject one change."""
                builds.append(label);out.mkdir(parents=True);(out/'profile').mkdir()
                publish(out/'profile/after.json',{'actual.rs':sha(root/'actual.rs')})
                frozen=root/'frozen';publish(frozen,b'new inert artifact')
                if change=='unused':publish(root/'unused.rs',b'changed unrelated test')
                elif change=='actual':publish(root/'actual.rs',b'changed compiled input')
                elif change=='commit':facts['commit']='b'*40
                elif change=='toolchain':facts['rustc']='changed-rustc'
                elif change=='notes':publish(root/'version_history.json',{'releases':[],'local_changes':notes})
                elif change=='launcher':publish(root/'tools/swarm_local.py',b'changed launcher')
                return {'binary':str(frozen),'metadata':{'version':label,'commit':'a'*40,'modified':True}}
            def raw_publish(path,value):
                """Match the real profile publisher: strings are raw UTF8, not JSON strings."""
                publish(path,value.encode() if isinstance(value,str) else value)
            profile=SimpleNamespace(digest=sha,publish=raw_publish,candidates=lambda source:{name:sha(root/name) for name in [*files,'version_history.json']})
            builder=SimpleNamespace(profile=profile,build=build,release=lambda value:True)
            with patch.object(version,'_read_prepared',return_value=None),patch.object(version,'native_builder',return_value=builder),patch.object(version.subprocess,'check_output',side_effect=output),patch.object(version,'verify_candidate') as probe,patch.dict(os.environ,{'SWARM_VERSION':'v0.1.1-dev.1'}):
                if change in ['actual','commit','toolchain','notes','launcher']:
                    with self.assertRaisesRegex(RuntimeError,'Native .*inputs changed'):
                        version.prepare(root,binary)
                    self.assertEqual(binary.read_bytes(),b'old artifact');self.assertFalse((root/'deploy/local-build.json').exists());probe.assert_not_called()
                    return
                artifact,state=version.prepare(root,binary)
                self.assertEqual(artifact.read_bytes(),b'new inert artifact');self.assertEqual(state['effective_inputs'],{'actual.rs':sha(root/'actual.rs')})
                self.assertEqual(state['candidate_digest'],version.source_identity(root)[0]);self.assertEqual(len(builds),1)
                again,stored=version.prepare(root,binary);self.assertEqual(again,artifact);self.assertEqual(stored['binary_sha256'],sha(artifact));self.assertEqual(len(builds),1);probe.assert_called_once()

    def test_unchanged_control_reuses_candidate(self):
        """A stable qualified fixture prepares once and then reuses exact bytes."""
        self.exercise()

    def test_unused_candidate_change_does_not_refuse_qualified_inputs(self):
        """An unused test edit cannot invalidate a stable actual-input profile."""
        self.exercise('unused')

    def test_compiled_source_changed_after_profile_still_refuses(self):
        """A post-profile used-source edit never becomes a new accepted digest."""
        self.exercise('actual')

    def test_metadata_change_still_refuses(self):
        """Changed Git metadata stays distinguishable from unused file changes."""
        self.exercise('commit')

    def test_toolchain_change_still_refuses(self):
        """Toolchain drift is never excused as an unused candidate edit."""
        self.exercise('toolchain')

    def test_notes_change_still_refuses(self):
        """Version notes must match the source observation before the build."""
        self.exercise('notes')

    def test_launcher_change_still_refuses(self):
        """Delivery tooling changes keep the old artifact intact."""
        self.exercise('launcher')

if __name__=='__main__':unittest.main()
