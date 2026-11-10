"""Authority refresh admission on inert isolated install overlays and mocked CLI.

No sudo, account, service or live Root operations. Original and exact transformed
shell bytes are retained when SWARM_TEST_EVIDENCE_DIR is supplied. No watchdog.
"""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from fixture_area import fixture_area

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
import swarm_local


class AuthorityRefresh(unittest.TestCase):
    def shell(self, initialized, markers=True, arguments=()):
        """Run only --dry-run on exact path-substituted installer bytes.

        Fixtures impersonate no account; run() prints effects. Keep real source
        logic, except owned filesystem destinations and the test binary path.
        """
        area = Path(tempfile.mkdtemp(prefix='authority-refresh-', dir=fixture_area()))
        etc = area / 'etc'
        etc.mkdir()
        if initialized:
            (etc / 'initialized').touch()
        if markers:
            (etc / 'pin').write_text('fixture-pin\n')
            (etc / 'public_key').write_text('fixture-key\n')
        source = (ROOT / 'deploy/authority/install.sh').read_text()
        overlay = source
        for old, new in [('LIB_DIR=/usr/local/lib/swarm-authority', f'LIB_DIR={area}/lib'),
                         ('STATE=/var/lib/swarm-authority', f'STATE={area}/state'),
                         ('ETC=/etc/swarm-authority', f'ETC={etc}'),
                         ('UNIT=/etc/systemd/system/swarm-authority.service', f'UNIT={area}/unit')]:
            assert old in overlay
            overlay = overlay.replace(old, new)
        script = area / 'installer.sh'
        script.write_text(overlay)
        result = subprocess.run(['bash', str(script), '--dry-run', '--binary', '/bin/true', *arguments], capture_output=True, text=True)
        proof = os.environ.get('SWARM_TEST_EVIDENCE_DIR')
        if proof:
            out = Path(proof) / area.name
            out.mkdir(parents=True)
            (out / 'source.sh').write_text(source)
            (out / 'overlay.sh').write_text(overlay)
            (out / 'stdout').write_text(result.stdout)
            (out / 'stderr').write_text(result.stderr)
            (out / 'result.json').write_text(json.dumps(dict(exit=result.returncode, source_sha256=hashlib.sha256(source.encode()).hexdigest(), initialized=initialized, markers=markers))+'\n')
        return result

    def test_existing_install_refresh_never_prepares_or_initializes_accounts(self):
        result = self.shell(True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn(' prepare ', result.stdout)
        self.assertNotIn(' init ', result.stdout)
        self.assertIn('try-restart swarm-authority.service', result.stdout)
        self.assertIn('utilisateur existant conservé', result.stdout)

    def test_missing_markers_refuse_before_install_effects(self):
        result = self.shell(True, False)
        self.assertEqual(result.returncode, 2)
        self.assertNotIn('[dry-run]', result.stdout)

    def test_fresh_install_requires_explicit_admin(self):
        result = self.shell(False, False)
        self.assertEqual(result.returncode, 2)
        control = self.shell(False, False, ('--admin', 'fixture-admin', '--display', 'Fixture'))
        self.assertEqual(control.returncode, 0, control.stderr)
        self.assertIn(' init ', control.stdout)

    def test_cli_existing_refresh_passes_no_invented_identity(self):
        calls = []
        def run(argv, **kwargs):
            calls.append(argv)
            return subprocess.CompletedProcess(argv, 0, json.dumps({'values': {'web_access_mode':'authority'}}), '')
        with patch.object(swarm_local.subprocess, 'run', side_effect=run), patch.object(Path, 'is_file', return_value=True), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(swarm_local.authority(['install']), 0)
        install = next(argv for argv in calls if argv[0]=='sudo')
        self.assertNotIn('--admin', install)
        self.assertNotIn('--display', install)
        self.assertTrue(any('restart' in argv and '--no-open' in argv for argv in calls))

    def test_settings_read_failure_is_not_reported_as_success(self):
        calls = []
        def run(argv, **kwargs):
            calls.append(argv)
            if 'engine-settings' in argv:
                self.assertTrue(kwargs.get('check'))
                raise subprocess.CalledProcessError(7, argv, stderr='fixture-read-refused')
            return subprocess.CompletedProcess(argv, 0, '', '')
        with patch.object(swarm_local.subprocess, 'run', side_effect=run), patch.object(Path, 'is_file', return_value=True), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(RuntimeError, 'Démon installé') as error:
                swarm_local.authority(['install'])
        self.assertIsInstance(error.exception.__cause__,subprocess.CalledProcessError)
        self.assertEqual(error.exception.__cause__.returncode, 7)
        self.assertFalse(any('restart' in argv for argv in calls))

    def test_invalid_settings_preserve_installed_effect_and_refuse_restart(self):
        for output in ['{}', '{broken', '{"values":{"web_access_mode":"unknown"}}']:
            calls = []
            def run(argv, **kwargs):
                calls.append(argv)
                return subprocess.CompletedProcess(argv, 0, output, '')
            with patch.object(swarm_local.subprocess, 'run', side_effect=run), patch.object(Path, 'is_file', return_value=True), contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaisesRegex(RuntimeError, 'Démon installé'):
                    swarm_local.authority(['install'])
            self.assertFalse(any('restart' in argv for argv in calls))

    def test_public_cli_preserves_installed_effect_without_private_output(self):
        # Execute the real public __main__ handler on a relocated exact source,
        # with inert command doubles. Both failures occur after sudo succeeded.
        driver = """import pathlib,runpy,subprocess,sys
stage=sys.argv[3]
sys.path.insert(0,sys.argv[2])
original=pathlib.Path.is_file
def is_file(p):
 return True if str(p)=='/etc/swarm-authority/initialized' else original(p)
pathlib.Path.is_file=is_file
def run(argv,**kwargs):
 if 'engine-settings' in argv:
  if stage=='read':
   raise subprocess.CalledProcessError(7,argv,output='PRIVATE-fixture-output',stderr='PRIVATE-fixture-error')
  return subprocess.CompletedProcess(argv,0,'{"values":{"web_access_mode":"authority"}}','')
 if 'restart' in argv:
  raise subprocess.CalledProcessError(8,argv,stderr='PRIVATE-fixture-error')
 return subprocess.CompletedProcess(argv,0,'','')
subprocess.run=run
sys.argv=[sys.argv[1],'authority','install']
runpy.run_path(sys.argv[0],run_name='__main__')
"""
        for stage, malformed in [('read',False),('read',True),('restart',False)]:
            with self.subTest(stage=stage,malformed=malformed):
                area = Path(tempfile.mkdtemp(prefix='authority-public-', dir=fixture_area()))
                (area/'tools').mkdir()
                source = area/'tools/swarm_local.py'
                source.write_bytes((ROOT/'tools/swarm_local.py').read_bytes())
                if malformed:
                    (area/'deploy').mkdir()
                    (area/'deploy/local-web.json').write_text('{broken')
                result = subprocess.run([sys.executable,'-c',driver,str(source),str(ROOT/'tools'),stage],capture_output=True,text=True)
                self.assertEqual(result.returncode,2)
                self.assertIn('Démon installé',result.stderr)
                self.assertNotIn('PRIVATE-fixture',result.stdout+result.stderr)
                self.assertNotIn('Traceback',result.stderr)
                self.assertNotIn('Démon et moteur à jour.',result.stdout)

    def test_cli_missing_identity_on_fresh_install_refuses_before_build(self):
        with patch.object(swarm_local.subprocess, 'run') as run, patch.object(Path, 'is_file', return_value=False), contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as result:
                swarm_local.authority(['install'])
            self.assertEqual(result.exception.code, 2)
            run.assert_not_called()


if __name__ == '__main__':
    unittest.main()
