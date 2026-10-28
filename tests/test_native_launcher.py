"""Rust-only launcher regression guards; mocks prove probes, not installation."""
import importlib.util,json,sys,tempfile,unittest
from pathlib import Path
from unittest.mock import patch,MagicMock
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
import swarm_local as local

class NativeLauncher(unittest.TestCase):
    """Do not mistake a login page/foreign HTTP 200 for protected native readiness."""
    def test_protected_native_probe_rejects_login_and_foreign_success(self):
        """Valid profile control precedes login HTML and wrong-profile refusals."""
        response=MagicMock();response.__enter__.return_value=response;response.status=200
        with tempfile.TemporaryDirectory() as temporary,patch.object(local.urllib.request,'urlopen',return_value=response) as probe:
            root=Path(temporary);(root/'.swarm').mkdir()
            import hashlib
            key=root/'.swarm'/('web-session-'+hashlib.sha256(b'127.0.0.1:38433').hexdigest()[:16])
            key.write_text('session-'+'a'*24);key.chmod(0o600)
            response.read.return_value=json.dumps({'schema_version':1,'profile':'rust-native-http1-observed'}).encode()
            self.assertTrue(local.ready('127.0.0.1:38433',root))
            self.assertEqual(probe.call_args.args[0].full_url,'http://127.0.0.1:38433/api/v1/native-capabilities')
            for raw in [b'<html>login</html>',b'{}',b'{"profile":"go"}',b'null']:
                response.read.return_value=raw
                self.assertFalse(local.ready('127.0.0.1:38433',root))
    def test_build_entrypoint_contains_no_go_compilation(self):
        """Production build dispatch must select native builder, without Go fallback."""
        text=(ROOT/'build.sh').read_text()
        self.assertNotIn('go build',text)
        self.assertIn('build_native.py',text)

if __name__=='__main__':unittest.main()
