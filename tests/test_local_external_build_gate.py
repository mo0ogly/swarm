"""External binaries stay external: `build` refuses nonmanaged destinations.

The actual local.main runs against real isolated fixture files under the owned
fixture area. Process, provider, network, signal, stop and build-producer
boundaries are doubled with inert sentinels; no native binary, provider call
or foreign file write occurs. A managed control (producer sentinel reached)
precedes every external fault and a recovery follows inside each test, so no
check depends on unittest method ordering. The classification and refusal in
main itself are never mocked; refused scenarios must leave an initially
nonexistent Root, runtime and build area absent and the selected external file
byte-, mode- and inode-identical. Results are established only by retained named runs; the eight existing
managed version tests remain separate proofs.
"""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import MagicMock, patch

from fixture_area import fixture_area

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import swarm_local as local


class ReachedBuild(Exception):
    """Producer sentinel: managed build path reached before compilation."""


class ReachedStart(Exception):
    """Start sentinel: owned init boundary reached before any launch."""


class ReachedStop(Exception):
    """Restart sentinel: owned stop gate reached before any stop."""


class ExternalBuildGate(unittest.TestCase):
    """Exercise the actual main caller for the external-build refusal gate."""

    def probe(self, *, action, via='none', running=False, start_probe=False):
        """Run one inert main() call in a fresh owned fixture; return a receipt.

        `via` designates the binary under test: 'argv' passes --binary for a
        real external fixture file, 'env' sets SWARM_BINARY, 'config' persists
        it in the owned settings file, 'absent' passes a --binary path that
        does not exist, 'managed-argv' passes the managed source path
        explicitly, and 'none' keeps the managed default. The build producer
        raises ReachedBuild if ever reached, stop raises ReachedStop, and, when
        `start_probe` is set, the port probe is inert and the init subprocess
        boundary raises ReachedStart; otherwise every process/signal/network
        boundary fails the scenario if touched. Nothing outside the owned
        fixture is touched, no native or provider boundary is used, and caught
        exceptions are returned, never hidden: the classification and refusal
        inside main are never mocked.
        """
        temporary = tempfile.TemporaryDirectory(prefix='external-gate-', dir=fixture_area())
        self.addCleanup(temporary.cleanup)
        area = temporary.name
        source = Path(area).resolve()
        root = source / 'project'
        address = '127.0.0.1:18792'
        runtime = root / '.swarm/local-web' / hashlib.sha256(address.encode()).hexdigest()[:16]
        external = source / 'external' / 'prebuilt-swarm'
        external.parent.mkdir(parents=True)
        external.write_bytes(b'EXTERNAL_PREBUILT_IMAGE')
        os.chmod(external, 0o755)
        before = (external.read_bytes(), external.stat().st_mode,
                  external.stat().st_ino, external.stat().st_dev)
        managed = source / 'bin' / 'swarm-local'
        managed.parent.mkdir(parents=True)
        managed.write_bytes(b'MANAGED_SOURCE_IMAGE')
        absent = source / 'absent' / 'never-built-swarm'
        argv_binary = {'argv': str(external), 'absent': str(absent),
                       'managed-argv': str(managed)}.get(via)
        selected = {'argv': external, 'env': external, 'config': external,
                    'absent': absent}.get(via)
        if via == 'config':
            (source / 'deploy').mkdir()
            (source / 'deploy' / 'local-web.json').write_text(
                json.dumps({'binary': str(external)}))
        if running:
            runtime.mkdir(parents=True)
            (runtime / 'server.json').write_text(
                json.dumps({'pid': 7, 'identity': {'fixture': True}}))
        argv = ['swarm_local.py', action, '--root', str(root),
                '--address', address, '--no-open']
        if argv_binary is not None:
            argv += ['--binary', argv_binary]
        error = None
        with contextlib.ExitStack() as stack:
            stack.enter_context(patch.object(local, 'SOURCE', source))
            stack.enter_context(patch.object(sys, 'argv', argv))
            stack.enter_context(patch.object(local.os, 'umask'))
            environ = stack.enter_context(patch.dict(os.environ))
            environ.pop('SWARM_BINARY', None)
            if via == 'env':
                environ['SWARM_BINARY'] = str(external)
            fresh = stack.enter_context(patch.object(
                local, 'prepare_versioned_build',
                side_effect=ReachedBuild('producer reached')))
            recorded = stack.enter_context(patch.object(
                local, 'prepare_recorded',
                side_effect=ReachedBuild('recorded producer reached')))
            owner = stack.enter_context(patch.object(local, 'owned', return_value=running))
            pair = stack.enter_context(patch.object(local, 'require_matching_assertion_pair'))
            stop = stack.enter_context(patch.object(
                local, 'stop', side_effect=ReachedStop('stop gate reached')))
            if start_probe:
                sock = MagicMock()
                sock.return_value.__enter__.return_value.connect_ex.return_value = 1
                stack.enter_context(patch.object(local.socket, 'socket', sock))
                run = stack.enter_context(patch.object(
                    local.subprocess, 'run',
                    side_effect=ReachedStart('start gate reached')))
            else:
                sock = stack.enter_context(patch.object(
                    local.socket, 'socket',
                    side_effect=AssertionError('forbidden socket')))
                run = stack.enter_context(patch.object(
                    local.subprocess, 'run',
                    side_effect=AssertionError('forbidden subprocess.run')))
            popen = stack.enter_context(patch.object(
                local.subprocess, 'Popen',
                side_effect=AssertionError('forbidden Popen')))
            kill = stack.enter_context(patch.object(
                local.os, 'kill',
                side_effect=AssertionError('forbidden os.kill')))
            urlopen = stack.enter_context(patch.object(
                local.urllib.request, 'urlopen',
                side_effect=AssertionError('forbidden urlopen')))
            stderr = io.StringIO()
            stack.enter_context(contextlib.redirect_stderr(stderr))
            stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
            try:
                local.main()
            except BaseException as caught:
                error = caught
        return {'error': error, 'stderr': stderr.getvalue(), 'via': via,
                'fresh': fresh, 'recorded': recorded, 'stop': stop, 'owner': owner,
                'pair': pair, 'run': run, 'popen': popen, 'kill': kill,
                'urlopen': urlopen, 'sock': sock, 'source': source,
                'root': root, 'runtime': runtime, 'external': external,
                'selected': selected, 'before': before}

    def assert_external_untouched(self, receipt):
        """Assert the real external fixture file kept bytes, mode and inode."""
        before = receipt['before']
        after = receipt['external'].stat()
        self.assertEqual(before[0], receipt['external'].read_bytes())
        self.assertEqual(before[1], after.st_mode)
        self.assertEqual(before[2], after.st_ino)
        self.assertEqual(before[3], after.st_dev)

    def assert_refusal(self, receipt, *, settings_precreated=False):
        """Assert the whole external-build refusal contract for one scenario."""
        error = receipt['error']
        self.assertIsInstance(error, SystemExit, 'refusal must exit cleanly')
        self.assertEqual(error.code, 2)
        stderr = receipt['stderr']
        self.assertIn('Construction refusée', stderr)
        self.assertIn('bin/swarm-local', stderr)
        self.assertNotIn(str(receipt['root']), stderr)
        self.assertNotIn(str(receipt['selected']), stderr)
        if receipt['via'] == 'absent':
            self.assertFalse(receipt['selected'].exists(),
                             'nonmanaged destination must not be created')
        self.assert_external_untouched(receipt)
        for boundary in (receipt['owner'], receipt['fresh'], receipt['recorded'], receipt['stop'],
                         receipt['pair'], receipt['run'], receipt['popen'],
                         receipt['kill'], receipt['urlopen'], receipt['sock']):
            boundary.assert_not_called()
        self.assertFalse(receipt['root'].exists(), 'Root must stay absent')
        self.assertFalse(receipt['runtime'].exists(), 'runtime must stay absent')
        self.assertFalse((receipt['source'] / 'deploy' / '.local-build.lock').exists())
        if not settings_precreated:
            self.assertFalse((receipt['source'] / 'deploy').exists(),
                             'build area must stay absent')

    def assert_control(self, receipt):
        """Assert a managed control reached the producer sentinel exactly once."""
        self.assertIsInstance(receipt['error'], ReachedBuild)
        receipt['fresh'].assert_called_once()
        receipt['recorded'].assert_not_called()

    def test_cli_external_build_control_fault_recovery(self):
        """Managed control, external --binary refusal, external-start recovery."""
        self.assert_control(self.probe(action='build'))
        self.assert_refusal(self.probe(action='build', via='argv'))
        recovery = self.probe(action='start', via='argv', start_probe=True)
        self.assertIsInstance(recovery['error'], ReachedStart)
        recovery['fresh'].assert_not_called()
        recovery['recorded'].assert_not_called()
        recovery['stop'].assert_not_called()
        recovery['pair'].assert_called_once()
        recovery['run'].assert_called_once()
        self.assertEqual(recovery['run'].call_args[0][0][0], str(recovery['external']))
        self.assert_external_untouched(recovery)

    def test_env_external_build_control_fault_recovery(self):
        """Same gate through SWARM_BINARY, recovering with a managed build."""
        self.assert_control(self.probe(action='build'))
        self.assert_refusal(self.probe(action='build', via='env'))
        self.assert_control(self.probe(action='build'))

    def test_persisted_config_external_build_control_fault_recovery(self):
        """Same gate through persisted settings; the settings file is preserved."""
        self.assert_control(self.probe(action='build'))
        fault = self.probe(action='build', via='config')
        self.assert_refusal(fault, settings_precreated=True)
        config = fault['source'] / 'deploy' / 'local-web.json'
        self.assertEqual(json.loads(config.read_text()),
                         {'binary': str(fault['external'])})
        self.assert_control(self.probe(action='build'))

    def test_absent_nonmanaged_destination_refused_and_not_created(self):
        """A nonexistent nonmanaged destination refuses and is never created."""
        self.assert_control(self.probe(action='build'))
        self.assert_refusal(self.probe(action='build', via='absent'))
        self.assert_control(self.probe(action='build'))

    def test_explicit_managed_binary_build_still_reaches_producer(self):
        """Explicitly selecting a managed source path keeps the build path open."""
        self.assert_control(self.probe(action='build'))
        self.assert_control(self.probe(action='build', via='managed-argv'))
        self.assert_control(self.probe(action='build'))

    def test_external_start_bypasses_build_and_reaches_lifecycle_gate(self):
        """External start skips the producer and stops at the init sentinel."""
        self.assert_control(self.probe(action='build'))
        gate = self.probe(action='start', via='argv', start_probe=True)
        self.assertIsInstance(gate['error'], ReachedStart)
        gate['fresh'].assert_not_called()
        gate['recorded'].assert_not_called()
        gate['pair'].assert_called_once()
        gate['stop'].assert_not_called()
        gate['sock'].assert_called_once()
        gate['run'].assert_called_once()
        self.assertEqual(gate['run'].call_args[0][0][0], str(gate['external']))
        for boundary in (gate['popen'], gate['kill'], gate['urlopen']):
            boundary.assert_not_called()
        self.assert_external_untouched(gate)
        self.assert_control(self.probe(action='build'))

    def test_external_restart_bypasses_build_and_reaches_stop_gate(self):
        """External restart skips the producer and stops at the stop sentinel."""
        self.assert_control(self.probe(action='build'))
        gate = self.probe(action='restart', via='argv', running=True)
        self.assertIsInstance(gate['error'], ReachedStop)
        gate['fresh'].assert_not_called()
        gate['recorded'].assert_not_called()
        gate['pair'].assert_called_once()
        gate['stop'].assert_called_once()
        for boundary in (gate['run'], gate['popen'], gate['kill'],
                         gate['urlopen'], gate['sock']):
            boundary.assert_not_called()
        self.assertTrue((gate['runtime'] / 'server.json').exists(),
                        'state must survive until the stop gate actually runs')
        self.assert_external_untouched(gate)
        self.assert_control(self.probe(action='build'))


if __name__ == '__main__':
    unittest.main()
