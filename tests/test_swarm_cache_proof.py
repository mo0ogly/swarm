"""Cached local delivery must retain qualification; fake compiler, no live Root."""
import importlib.util,json,tempfile,unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('cache_version',Path(__file__).resolve().parents[1]/'tools/swarm_version.py');version=importlib.util.module_from_spec(spec);spec.loader.exec_module(version)
class CacheProof(unittest.TestCase):
    """Exercise prepare's fast path; rebuilding is a sentinel, never Cargo."""
    def exercise(self,missing=False,invalid=False):
        """Use inert files and mocked proof/compiler outcomes; preserve image on refusal."""
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);(root/'deploy').mkdir();binary=root/'candidate';binary.write_bytes(b'image')
            notes={'summary':{'fr':'Note','en':'Note'},'items':[{'fr':'Note','en':'Note'}]};(root/'version_history.json').write_text(json.dumps({'local_changes':notes,'releases':[]}));state={'schema_version':2,'engine':'rust','candidate_digest':'same','version':'v0.1.1-dev.1','artifact':str(binary),'binary_sha256':'a'*64,'evidence':str(root/'.cache/proof'),'effective_inputs':{}};(root/'deploy/local-build.json').write_text(json.dumps(state))
            builder=SimpleNamespace(profile=SimpleNamespace(digest=lambda p:'a'*64),release=lambda v:True,build=lambda *a:(_ for _ in ()).throw(RuntimeError('fresh-build-selected')))
            error=FileNotFoundError('fixture proof missing') if missing else ValueError('fixture proof invalid') if invalid else None
            with patch.object(version,'native_builder',return_value=builder),patch.object(version,'source_identity',return_value=('same',{})),patch.object(version,'_read_prepared',create=True,side_effect=error,return_value=None) as check:
                if missing:
                    with self.assertRaisesRegex(RuntimeError,'fresh-build-selected'):version.prepare(root,binary)
                elif invalid:
                    with self.assertRaisesRegex(ValueError,'fixture proof invalid'):version.prepare(root,binary)
                else:self.assertEqual(version.prepare(root,binary)[0],binary)
                check.assert_called_once_with(root,'a'*64)
                self.assertEqual(binary.read_bytes(),b'image')
    def test_valid_cache_checks_proof(self):
        """Even an unchanged image needs its retained qualification check."""
        self.exercise()
    def test_missing_proof_selects_fresh_build(self):
        """Missing proof reaches a fresh-build sentinel without changing the old image."""
        self.exercise(missing=True)
    def test_invalid_proof_refuses_without_reuse(self):
        """Incoherence propagates instead of silently reusing or rebuilding."""
        self.exercise(invalid=True)
if __name__=='__main__':unittest.main(verbosity=2)
