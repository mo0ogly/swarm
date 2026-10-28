"""Prepared source delivery: isolated refusal/recovery and real owned lifecycle.

Synthetic records are unit fixtures only. The public lifecycle test requires an
actual retained qualified build supplied by the caller; no compilation, provider
call, live Root or global watchdog. Preserve production binary originals.
"""
import hashlib, importlib.util, json, os, shutil, socket, subprocess, sys, tarfile, tempfile, unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
SOURCE=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(SOURCE/'tools'));sys.path.insert(0,str(SOURCE/'tests'))
import swarm_version as version
from rust_agent_model_route import publish

def sha(path):
    """Digest complete fixture bytes; no signature or execution authority."""
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def fixture(root):
    """Construct explicitly synthetic profile records and a nonexecutable image."""
    evidence=root/'.cache/local-delivery/fixture';profile=evidence/'build/profile';profile.mkdir(parents=True)
    frozen=root/'.cache/rust-migration-binaries/image';frozen.parent.mkdir(parents=True);frozen.write_bytes(b'INERT_UNIT_FIXTURE')
    source=root/'version_history.json';publish(source,{'local_changes':{'summary':{'fr':'Test','en':'Test'},'items':[{'fr':'Test','en':'Test'}]}})
    after={'version_history.json':sha(source)};publish(profile/'after.json',after)
    with tarfile.open(profile/'effective-sources.tar.gz','w:gz') as archive:archive.add(source,arcname='version_history.json')
    digest=sha(frozen);metadata={'version':'v0.1.1-dev.190'}
    built={'status':'VERIFIED_NATIVE_CANDIDATE','binary':str(frozen),'sha256':digest,'metadata':metadata,'profile':str(profile/'result.json')}
    checked={'status':'VERIFIED_DEFAULT_NATIVE_PROFILE','sha256':digest,**{k:True for k in ['all_core_features_empty','all_core_test_false','no_fixture_feature','actual_dependencies_stable']},'effective_sources_archived':{'sha256':sha(profile/'effective-sources.tar.gz'),'all_members_verified':True}}
    metadata_sha=hashlib.sha256(json.dumps(metadata).encode()).hexdigest();built['metadata_sha256']=metadata_sha;checked['native_build_metadata']={'present':True,'sha256':metadata_sha}
    state={'schema_version':2,'engine':'rust','binary_sha256':digest,'binary':metadata,'version':metadata['version'],'evidence':str(evidence),'effective_inputs':after}
    (root/'deploy').mkdir();publish(root/'deploy/local-build.json',state);publish(evidence/'build/result.json',built);publish(profile/'result.json',checked)
    binary=root/'bin/swarm-local';binary.parent.mkdir();binary.write_bytes(b'PRIOR')
    return SimpleNamespace(root=root,evidence=evidence,profile=profile,frozen=frozen,binary=binary,digest=digest,state=state,built=built,checked=checked)

class PreparedUnit(unittest.TestCase):
    """Check gate failures against a valid synthetic witness, no process exec."""
    def test_valid_copy_preserves_binary_and_pending_sources(self):
        with tempfile.TemporaryDirectory() as d:
            f=fixture(Path(d));publish(f.root/'version_history.json',{'changed':'pending'})
            builder=SimpleNamespace(profile=SimpleNamespace(digest=sha,publish=publish))
            with patch.object(version,'native_builder',return_value=builder),patch.object(version,'verify_candidate') as verify:
                candidate,state=version.prepare_recorded(f.root,f.binary,f.digest)
            self.assertEqual(candidate.read_bytes(),f.frozen.read_bytes());self.assertEqual(f.binary.read_bytes(),b'PRIOR');self.assertEqual(state,f.state);verify.assert_called_once();candidate.unlink()
    def test_rejections_keep_binary_and_do_not_probe(self):
        for fault in ['selection','status','feature','metadata','archive','artifact','inventory']:
            with self.subTest(fault=fault),tempfile.TemporaryDirectory() as d:
                f=fixture(Path(d));expected=f.digest
                if fault=='selection':expected='0'*64
                elif fault=='status':f.built['status']='FAIL';publish(f.evidence/'build/result.json',f.built)
                elif fault=='feature':f.checked['no_fixture_feature']=False;publish(f.profile/'result.json',f.checked)
                elif fault=='metadata':f.built['metadata']={'version':'v0.1.1-dev.191'};publish(f.evidence/'build/result.json',f.built)
                elif fault=='archive':(f.profile/'effective-sources.tar.gz').write_bytes(b'CORRUPT')
                elif fault=='artifact':f.frozen.write_bytes(b'CORRUPT')
                elif fault=='inventory':publish(f.profile/'after.json',{})
                builder=SimpleNamespace(profile=SimpleNamespace(digest=sha,publish=publish))
                with patch.object(version,'native_builder',return_value=builder),patch.object(version,'verify_candidate') as verify:
                    with self.assertRaises((ValueError,KeyError)):version.prepare_recorded(f.root,f.binary,expected)
                    verify.assert_not_called()
                self.assertEqual(f.binary.read_bytes(),b'PRIOR');self.assertEqual(list(f.binary.parent.glob('*.candidate-*')),[])
    def test_artifact_race_refuses_before_probe(self):
        with tempfile.TemporaryDirectory() as d:
            f=fixture(Path(d))
            def raced(path):
                """Change only the owned inert file after its first valid digest."""
                result=sha(path)
                if Path(path)==f.frozen:f.frozen.write_bytes(b'CHANGED_AFTER_DIGEST')
                return result
            with patch.object(version,'native_builder',return_value=SimpleNamespace(profile=SimpleNamespace(digest=raced,publish=publish))),patch.object(version,'verify_candidate') as verify:
                with self.assertRaisesRegex(ValueError,'artifact-changed'):version.prepare_recorded(f.root,f.binary,f.digest)
                verify.assert_not_called()
    def test_version_order_is_not_a_rollback_grant(self):
        version.require_prepared_not_older('v0.1.1-dev.187','v0.1.1-dev.190','a','b')
        version.require_prepared_not_older('devel','unknown','same','same')
        for installed,selected in [('v0.1.1-dev.191','v0.1.1-dev.190'),('devel','v0.1.1-dev.190')]:
            with self.assertRaises(ValueError):version.require_prepared_not_older(installed,selected,'a','b')

@unittest.skipUnless(os.environ.get('SWARM_TEST_BUILD_STATE'),'retained qualified build required')
class PreparedLifecycle(unittest.TestCase):
    """Exercise actual start/refusal/recovery/stop through the public launcher."""
    def test_public_prepared_restart_and_tamper_refusal(self):
        with tempfile.TemporaryDirectory(prefix='swarm-prepared-lifecycle-') as d:
            root=Path(d);source=root/'source';project=root/'project';source.mkdir();project.mkdir()
            for name in ['tools/swarm_local.py','tools/swarm_version.py','tools/rust-migration/build_native.py','tools/verification/verify_native_profile.py']:
                dst=source/name;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(SOURCE/name,dst)
            state=json.loads(Path(os.environ['SWARM_TEST_BUILD_STATE']).read_bytes());old_evidence=Path(state['evidence']);built=json.loads((old_evidence/'build/result.json').read_bytes());old_profile=Path(built['profile']).parent
            evidence=source/'.cache/local-delivery/retained';profile=evidence/'build/profile';profile.mkdir(parents=True)
            for name in ['result.json','after.json','effective-sources.tar.gz']:shutil.copy2(old_profile/name,profile/name)
            frozen=source/'.cache/rust-migration-binaries/image';frozen.parent.mkdir(parents=True);shutil.copy2(built['binary'],frozen)
            built.update(binary=str(frozen),profile=str(profile/'result.json'));state['evidence']=str(evidence);publish(evidence/'build/result.json',built);(source/'deploy').mkdir();publish(source/'deploy/local-build.json',state)
            # Deliberately incomplete/hot source tree: selected artifact stays exact.
            (source/'version_history.json').write_text('{"pending":true}')
            with socket.socket() as sock:sock.bind(('127.0.0.1',0));address='127.0.0.1:'+str(sock.getsockname()[1])
            common=[sys.executable,str(source/'tools/swarm_local.py'),'--root',str(project),'--address',address,'--no-open']
            def call(action,expected=0,extra=()):
                """Retain bounded safe launcher result; never disclose session tokens."""
                run=subprocess.run([*common,action,*extra],capture_output=True,text=True)
                self.assertEqual(run.returncode,expected,run.stdout+run.stderr);return run
            def observed():
                """Read actual process SHA/metadata via the protected public status."""
                return json.loads(call('status').stdout)
            try:
                call('start',extra=['--prepared-build',state['binary_sha256']]);before=observed();self.assertTrue(before['running']);self.assertEqual(before['binary_sha256'],state['binary_sha256'])
                call('start',extra=['--prepared-build',state['binary_sha256']]);self.assertEqual(observed()['pid'],before['pid'])
                call('start',expected=2,extra=['--prepared-build','0'*64]);self.assertEqual(observed()['pid'],before['pid'])
                call('restart',expected=2,extra=['--prepared-build','0'*64]);self.assertEqual(observed()['pid'],before['pid'])
                with frozen.open('ab') as stream:stream.write(b'TAMPER')
                call('restart',expected=2,extra=['--prepared-build',state['binary_sha256']]);self.assertEqual(observed()['pid'],before['pid'])
                shutil.copy2(json.loads((old_evidence/'build/result.json').read_bytes())['binary'],frozen)
                call('restart',extra=['--prepared-build',state['binary_sha256']]);after=observed();self.assertNotEqual(after['pid'],before['pid']);self.assertEqual(after['binary_sha256'],state['binary_sha256']);self.assertTrue((project/'.swarm/state.db').is_file())
            finally:call('stop')
            print('PREPARED_LIFECYCLE_VERIFIED',flush=True)
if __name__=='__main__':unittest.main(verbosity=2)
