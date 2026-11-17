'''Guards for the controlled exec fixture worker; no real exec ever occurs.

Requires Python >= 3.11. Both exec delegates are always mocked, so no actual
exec, child, strace, compiler or other tool can run. A healthy mocked-ENOEXEC
delegate control runs before each mutation and a fresh healthy recovery runs
after. Each test builds its own retained private TMPDIR fixture containing a
fake ELF image, the protocol document and the absent receipt leaf.
'''
import errno
import hashlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import trace_exec_fixture_worker as worker


class TraceExecFixtureGuards(unittest.TestCase):
    '''Guard bytes fidelity, pre-exec refusals, fault preservation and strictness.'''

    def _fixture(self, document=None, raw=None):
        '''Create one retained private fixture and return its facts.

        The fake image begins with ELF magic for the worker format
observation; the executable bit is unnecessary because no real exec occurs.
The document is written 0600 and the receipt leaf stays absent until a
healthy attempt publishes it.
        '''
        root = Path(tempfile.mkdtemp(prefix='trace-exec-fixture-'))

        def write_all(path, payload):
            '''Create one 0600 file and write the payload through partial writes.'''
            handle = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            try:
                view = memoryview(payload)
                while view:
                    count = os.write(handle, view)
                    if count <= 0:
                        raise OSError(errno.EIO, 'fixture write made no progress')
                    view = view[count:]
            finally:
                os.close(handle)

        image = b'\x7fELF-fixture-image-payload'
        executable = root / 'image.elf'
        write_all(executable, image)
        if raw is None:
            if document is None:
                document = {
                    'schema': 'trace-exec-fixture/1',
                    'argv_hex': [b'fixture-image'.hex(), b'--flag'.hex(), b''.hex()],
                    'environment_hex': {b'FIXTURE_ONLY_MARKER'.hex(): b'public-fictitious'.hex(),
                                        b'EMPTY'.hex(): b''.hex()},
                }
            raw = json.dumps(document).encode('utf-8')
        data = root / 'fixture.json'
        write_all(data, raw)
        argv, environment = [], {}
        if document is not None:
            if isinstance(document['argv_hex'], list) and all(
                    type(entry) is str for entry in document['argv_hex']):
                argv = [bytes.fromhex(entry) for entry in document['argv_hex']]
            if isinstance(document['environment_hex'], dict):
                environment = {bytes.fromhex(name): bytes.fromhex(value)
                               for name, value in document['environment_hex'].items()
                               if type(name) is str and type(value) is str}
        return {'root': root, 'executable': executable,
                'executable_sha256': hashlib.sha256(image).hexdigest(),
                'data': data, 'data_sha256': hashlib.sha256(raw).hexdigest(),
                'argv': argv, 'environment': environment,
                'receipt': root / 'receipt.json'}

    def _args(self, fixture, mode, executable_sha=None, data_sha=None, data_path=None):
        '''Build one exact CLI argv list for a fixture and mode.'''
        return ['--mode', mode,
                '--executable', str(fixture['executable']),
                '--executable-sha256', executable_sha or fixture['executable_sha256'],
                '--data', str(data_path or fixture['data']),
                '--data-sha256', data_sha or fixture['data_sha256'],
                '--receipt', str(fixture['receipt'])]

    def _attempt(self, fixture, mode):
        '''Run one healthy worker.main() attempt with both delegates mocked.

        The selected delegate records its exact call inputs and then raises a
synthetic ENOEXEC, so no real exec can occur and the unchanged primary error
must surface carrying its safe metadata.
        '''
        recorded = {'execve': [], 'execveat': []}

        def fake_execve(path, argv, environment):
            '''Record the exact bytes given to the execve delegate, then refuse.'''
            recorded['execve'].append((path, argv, environment))
            raise OSError(errno.ENOEXEC, 'synthetic ENOEXEC')

        def fake_execveat(fd, argv, pairs):
            '''Record the exact bytes given to the execveat delegate, then refuse.'''
            recorded['execveat'].append((fd, argv, pairs))
            raise OSError(errno.ENOEXEC, 'synthetic ENOEXEC')

        with mock.patch.object(worker.os, 'execve', new=fake_execve), \
                mock.patch.object(worker, '_execveat', new=fake_execveat):
            with self.assertRaises(OSError) as raised:
                worker.main(self._args(fixture, mode))
        self.assertEqual(raised.exception.errno, errno.ENOEXEC)
        self.assertEqual(raised.exception.fixture_exec_receipt['outcome'], 'UNKNOWN')
        return recorded

    def _assert_safe_receipt(self, fixture, mode):
        '''Assert the published receipt carries intent-only fields and no values.'''
        raw = fixture['receipt'].read_bytes()
        report = json.loads(raw)
        self.assertEqual(report['schema'], 'trace-exec-fixture/1')
        self.assertEqual(report['mode'], mode)
        self.assertEqual(report['pid'], os.getpid())
        self.assertEqual(report['cwd_hex'], os.fsencode(os.getcwd()).hex())
        self.assertEqual(report['executable_sha256'], fixture['executable_sha256'])
        self.assertEqual(report['argv_hex'],
                         [argument.hex() for argument in fixture['argv']])
        self.assertEqual(report['environment_names_hex'],
                         [name.hex() for name in fixture['environment']])
        self.assertEqual(report['environment_entry_count'], len(fixture['environment']))
        self.assertEqual(report['outcome'], 'UNKNOWN')
        self.assertEqual(report['note'], 'intent only')
        self.assertNotIn('environment_hex', report)
        for value in fixture['environment'].values():
            if value:
                self.assertNotIn(value.hex().encode('utf-8'), raw)
                self.assertNotIn(value, raw)
        self.assertEqual(os.stat(fixture['receipt']).st_mode & 0o777, 0o600)
        self.assertTrue(raw.endswith(b'\n'))

    def _healthy(self, mode):
        '''Run one fresh healthy delegate control with full byte and receipt assertions.

        Exact equality of the recorded argv, environment dict and name=value
pairs with the fixture values proves byte preservation and that nothing
outside the declared fixture bytes was inherited.
        '''
        fixture = self._fixture()
        recorded = self._attempt(fixture, mode)
        self.assertTrue(fixture['receipt'].is_file())
        self._assert_safe_receipt(fixture, mode)
        if mode == 'execve':
            self.assertEqual(recorded['execve'],
                             [(os.fsencode(str(fixture['executable'])),
                               fixture['argv'], fixture['environment'])])
            self.assertEqual(recorded['execveat'], [])
        else:
            self.assertEqual(recorded['execve'], [])
            self.assertEqual(len(recorded['execveat']), 1)
            held_fd, argv, pairs = recorded['execveat'][0]
            self.assertIsInstance(held_fd, int)
            self.assertGreaterEqual(held_fd, 0)
            self.assertEqual(argv, fixture['argv'])
            self.assertEqual(pairs, [name + b'=' + value for name, value
                                     in fixture['environment'].items()])
        return fixture

    def _refused(self, fixture, mode, expected_errno=None, executable_sha=None,
                 data_sha=None, data_path=None, receipt_absent=True):
        '''Run one attempt that must refuse before exec; delegates must never run.

        A delegate invocation raises AssertionError, which this refusal guard
does not catch, so any attempted exec fails the test. The receipt leaf must
stay absent unless the case itself pre-creates it.
        '''
        self._healthy(mode)

        def fake_execve(*args):
            '''Any execve delegation in a refusal case is a test failure.'''
            raise AssertionError('execve must not run in a refusal case')

        def fake_execveat(*args):
            '''Any execveat delegation in a refusal case is a test failure.'''
            raise AssertionError('execveat must not run in a refusal case')

        with mock.patch.object(worker.os, 'execve', new=fake_execve), \
                mock.patch.object(worker, '_execveat', new=fake_execveat):
            with self.assertRaises((ValueError, OSError)) as raised:
                worker.main(self._args(fixture, mode, executable_sha=executable_sha,
                                       data_sha=data_sha, data_path=data_path))
        if expected_errno is not None:
            self.assertEqual(raised.exception.errno, expected_errno)
        if receipt_absent:
            self.assertFalse(fixture['receipt'].exists())
        self._healthy(mode)
        return raised.exception

    def test_healthy_delegate_attempt_preserves_exact_bytes_and_safe_receipt(self):
        '''Both modes must delegate exact fixture bytes and publish a value-free intent receipt.'''
        for mode in ('execve', 'execveat'):
            self._healthy(mode)

    def test_invalid_inputs_and_receipt_collision_refuse_before_exec(self):
        '''Shape, duplicate, digest, NUL, FIFO, symlink, mode and collision refusals precede exec.'''
        self._healthy('execve')
        self._refused(self._fixture(document={
            'schema': 'trace-exec-fixture/1', 'argv_hex': '6161',
            'environment_hex': {}}), 'execve')
        self._refused(self._fixture(raw=(
            b'{"schema":"trace-exec-fixture/1","argv_hex":["6161"],'
            b'"environment_hex":{},"environment_hex":{}}')), 'execve')
        self._refused(self._fixture(raw=(
            b'{"schema":"trace-exec-fixture/1","argv_hex":["6161"],'
            b'"environment_hex":{"61":"62","61":"63"}}')), 'execve')
        self._refused(self._fixture(), 'execve', data_sha='0' * 64)
        self._refused(self._fixture(), 'execve', executable_sha='f' * 64)
        self._refused(self._fixture(document={
            'schema': 'trace-exec-fixture/1', 'argv_hex': [b'a\x00b'.hex()],
            'environment_hex': {}}), 'execve')
        self._refused(self._fixture(document={
            'schema': 'trace-exec-fixture/1', 'argv_hex': ['61'],
            'environment_hex': {b'A=B'.hex(): b''.hex()}}), 'execve')
        fifo = self._fixture()
        os.mkfifo(fifo['root'] / 'pipe')
        self._refused(fifo, 'execve', data_path=fifo['root'] / 'pipe')
        linked = self._fixture()
        os.symlink(linked['data'], linked['root'] / 'link.json')
        self._refused(linked, 'execve', expected_errno=errno.ELOOP,
                      data_path=linked['root'] / 'link.json')
        loose = self._fixture()
        os.chmod(loose['data'], 0o644)
        self._refused(loose, 'execve')
        exposed = self._fixture()
        os.chmod(exposed['root'], 0o755)
        self._refused(exposed, 'execve')
        collision = self._fixture()
        collision['receipt'].write_bytes(b'CALLER-SENTINEL')
        self._refused(collision, 'execve', expected_errno=errno.EEXIST,
                      receipt_absent=False)
        self.assertEqual(collision['receipt'].read_bytes(), b'CALLER-SENTINEL')
        self.assertTrue(any(name.startswith('.trace-exec-fixture-receipt-')
                            for name in os.listdir(collision['root'])),
                        'the refused publication temp must be preserved, never deleted')
        self._healthy('execveat')

    def test_publication_and_close_faults_preserve_original_causes(self):
        '''Publication, close and diagnostic faults keep original causes, raw partials and real closures.'''
        self._healthy('execve')
        fixture = self._fixture()
        with mock.patch.object(worker.os, 'fsync',
                               side_effect=OSError(errno.EIO, 'synthetic fsync')):
            with self.assertRaises(OSError) as raised:
                worker.main(self._args(fixture, 'execve'))
        self.assertEqual(raised.exception.errno, errno.EIO)
        self.assertEqual(raised.exception.fixture_exec_receipt['stage'], 'publish')
        self.assertFalse(fixture['receipt'].exists())
        self.assertTrue(any(name.startswith('.trace-exec-fixture-receipt-')
                            for name in os.listdir(fixture['root'])),
                        'raw partial publication must be preserved, never deleted')
        probe = self._fixture()
        probe_fd = os.open(probe['executable'], os.O_RDONLY)
        with worker._owned_fd(probe_fd):
            pass
        with self.assertRaises(OSError) as released:
            os.fstat(probe_fd)
        self.assertEqual(released.exception.errno, errno.EBADF)
        really_closed = []
        original_close = os.close

        def close_then_fail(fd):
            '''Really close the descriptor exactly once, then raise the synthetic fault.'''
            original_close(fd)
            really_closed.append(fd)
            raise OSError(errno.EIO, 'synthetic close after real close')

        primary = OSError(errno.ENOEXEC, 'synthetic primary')
        with mock.patch.object(worker.os, 'close', new=close_then_fail):
            with self.assertRaises(BaseExceptionGroup) as grouped:
                with worker._owned_fd(os.open(probe['executable'], os.O_RDONLY)):
                    raise primary
        self.assertIs(grouped.exception.exceptions[0], primary)
        self.assertEqual(grouped.exception.exceptions[1].errno, errno.EIO)
        self.assertEqual(len(really_closed), 1)
        with self.assertRaises(OSError) as released:
            os.fstat(really_closed[0])
        self.assertEqual(released.exception.errno, errno.EBADF)
        fixture = self._fixture()

        def fake_execve(path, argv, environment):
            '''Refuse the exec attempt with the known synthetic errno.'''
            raise OSError(errno.ENOEXEC, 'synthetic ENOEXEC')

        def fake_execveat(*args):
            '''Any execveat delegation here is a test failure.'''
            raise AssertionError('NO_EXECVEAT')

        class RefusingStream:
            '''stderr double that refuses every write; nothing is ever emitted.'''
            def write(self, text):
                '''Refuse the diagnostic write with a typed synthetic error.'''
                raise OSError(errno.EIO, 'synthetic stderr write')
            def flush(self):
                '''Refuse any flush symmetrically.'''
                raise OSError(errno.EIO, 'synthetic stderr flush')

        with mock.patch.object(worker.os, 'execve', new=fake_execve), \
                mock.patch.object(worker, '_execveat', new=fake_execveat), \
                mock.patch.object(worker.sys, 'stderr', RefusingStream()):
            with self.assertRaises(BaseExceptionGroup) as grouped:
                worker.cli_main(self._args(fixture, 'execve'))
        self.assertEqual(grouped.exception.exceptions[0].errno, errno.ENOEXEC)
        self.assertEqual(grouped.exception.exceptions[1].errno, errno.EIO)
        self.assertEqual(grouped.exception.fixture_exec_receipt['mode'], 'execve')
        self._healthy('execveat')

    def test_strict_types_and_unexpected_exec_return_raise(self):
        '''Strict JSON typing, CLI shape guards and unexpected delegate returns never report success.'''
        self._healthy('execveat')
        self._refused(self._fixture(document={
            'schema': 'trace-exec-fixture/1', 'argv_hex': ['61'],
            'environment_hex': {'41': True}}), 'execveat')
        self._refused(self._fixture(document={
            'schema': 'trace-exec-fixture/1', 'argv_hex': [7],
            'environment_hex': {}}), 'execveat')
        fixture = self._fixture()
        with mock.patch.object(worker.sys, 'stderr', io.StringIO()):
            with self.assertRaises(SystemExit) as exited:
                worker.main(self._args(fixture, 'execveat', data_sha='NOT-HEX'))
        self.assertEqual(exited.exception.code, 2)
        self.assertFalse(fixture['receipt'].exists())
        fixture = self._fixture()

        def no_execveat(*args):
            '''Any execveat delegation here is a test failure.'''
            raise AssertionError('NO_EXECVEAT')

        with mock.patch.object(worker.os, 'execve', new=lambda *args, **kwargs: None), \
                mock.patch.object(worker, '_execveat', new=no_execveat):
            with self.assertRaises(worker.FixtureExecReturnedError):
                worker.main(self._args(fixture, 'execve'))
        self.assertTrue(fixture['receipt'].is_file())
        self.assertEqual(json.loads(fixture['receipt'].read_bytes())['outcome'], 'UNKNOWN')
        fixture = self._fixture()

        def no_execve(*args, **kwargs):
            '''Any execve delegation here is a test failure.'''
            raise AssertionError('NO_EXECVE')

        with mock.patch.object(worker.os, 'execve', new=no_execve), \
                mock.patch.object(worker, '_execveat', new=lambda *args: 0):
            with self.assertRaises(worker.FixtureExecReturnedError):
                worker.main(self._args(fixture, 'execveat'))
        captured = io.StringIO()
        with mock.patch.object(worker, '_run',
                               side_effect=KeyboardInterrupt('synthetic interruption')), \
                mock.patch.object(worker.sys, 'stderr', captured):
            self.assertEqual(worker.cli_main(self._args(self._fixture(), 'execve')), 130)
        with mock.patch.object(worker, '_run',
                               side_effect=OSError(errno.ENOEXEC, 'synthetic ENOEXEC')), \
                mock.patch.object(worker.sys, 'stderr', captured):
            self.assertEqual(worker.cli_main(self._args(self._fixture(), 'execveat')), 1)
        report = json.loads(captured.getvalue().splitlines()[-1])
        self.assertEqual(report['schema'], 'trace-exec-fixture-error/1')
        self.assertEqual(report['primary']['type'], 'OSError')
        self.assertEqual(report['primary']['errno'], errno.ENOEXEC)
        self._healthy('execve')


    def test_nested_diagnostics_preserve_safe_causes_and_cycles(self):
        """Keep every nested errno without rendering secret messages or looping."""
        self._healthy('execve')
        primary = OSError(errno.ENOEXEC, 'PRIVATE_FIXTURE_ERROR_TEXT')
        cleanup = OSError(errno.EIO, 'PRIVATE_FIXTURE_ERROR_TEXT')
        cause = OSError(errno.EACCES, 'PRIVATE_FIXTURE_ERROR_TEXT')
        context = OSError(errno.ENOSPC, 'PRIVATE_FIXTURE_ERROR_TEXT')
        primary.__cause__ = cause
        cause.__context__ = context
        context.__cause__ = primary
        failure = BaseExceptionGroup('PRIVATE_FIXTURE_ERROR_TEXT', [
            BaseExceptionGroup('nested', [primary, cleanup]), KeyboardInterrupt()])
        raw = worker._safe_report(failure)
        report = json.loads(raw)
        self.assertNotIn('PRIVATE_FIXTURE_ERROR_TEXT', raw)
        nested = report['primary']['children'][0]['children']
        self.assertEqual(nested[0]['errno'], errno.ENOEXEC)
        self.assertEqual(nested[1]['errno'], errno.EIO)
        self.assertEqual(nested[0]['cause']['errno'], errno.EACCES)
        self.assertEqual(nested[0]['cause']['context']['errno'], errno.ENOSPC)
        self.assertTrue(nested[0]['cause']['context']['cause']['repeated_reference'])
        self.assertTrue(worker._is_interruption(failure))
        self._healthy('execve')

    def test_runtime_systemexit_and_linked_interruptions_never_succeed(self):
        """Separate parser exits from runtime interruptions, including cleanup links."""
        for failure in (SystemExit(0), GeneratorExit(),
                        BaseExceptionGroup('nested', [OSError(errno.EIO, 'safe'), SystemExit(0)])):
            self._healthy('execveat')
            captured = io.StringIO()
            with mock.patch.object(worker, '_run', side_effect=failure), \
                    mock.patch.object(worker.sys, 'stderr', captured):
                self.assertEqual(worker.cli_main(self._args(self._fixture(), 'execveat')), 130)
            self.assertTrue(json.loads(captured.getvalue())['primary']['type'])
            self._healthy('execveat')
        ordinary = OSError(errno.EIO, 'safe')
        ordinary.__cause__ = GeneratorExit()
        self.assertTrue(worker._is_interruption(ordinary))
        ordinary.__cause__ = ordinary
        self.assertFalse(worker._is_interruption(ordinary))
        with mock.patch.object(worker, '_run', side_effect=AssertionError('must not run')), \
                mock.patch.object(worker.sys, 'stderr', io.StringIO()), \
                mock.patch.object(worker.sys, 'stdout', io.StringIO()):
            with self.assertRaises(SystemExit) as help_exit:
                worker.cli_main(['--help'])
            self.assertEqual(help_exit.exception.code, 0)
            with self.assertRaises(SystemExit) as invalid_exit:
                worker.cli_main([])
            self.assertEqual(invalid_exit.exception.code, 2)



    def test_terminal_diagnostic_failure_never_renders_original_messages(self):
        """A recovered stderr must not render private primary/cleanup traceback text."""
        import traceback
        self._healthy('execve')
        primary = OSError(errno.ENOEXEC, 'PRIVATE_TERMINAL_PRIMARY')
        publication = OSError(errno.EIO, 'PRIVATE_TERMINAL_PUBLICATION')
        original_cli = worker.cli_main
        observed = []

        class RecoveringStream(io.StringIO):
            """Fail the first write, then accept all subsequent terminal writes."""
            def write(self, value):
                if not observed:
                    observed.append(publication)
                    raise publication
                return super().write(value)

        def retain_cli_failure(argv):
            """Retain original group objects for inspection before terminal mapping."""
            try:
                return original_cli(argv)
            except BaseException as error:
                observed.append(error)
                raise

        for failure, expected in ((primary, 1), (KeyboardInterrupt('PRIVATE_TERMINAL_INTERRUPT'), 130)):
            observed.clear()
            stream = RecoveringStream()
            with mock.patch.object(worker, '_run', side_effect=failure), \
                    mock.patch.object(worker, 'cli_main', side_effect=retain_cli_failure), \
                    mock.patch.object(worker.sys, 'stderr', stream):
                with self.assertRaises(SystemExit) as exited:
                    # Before repair, this is the exact former module entry expression.
                    entry = getattr(worker, 'entry_main', worker.cli_main)
                    raise SystemExit(entry(self._args(self._fixture(), 'execve')))
                rendered = ''.join(traceback.format_exception(exited.exception))
                worker.sys.stderr.write(rendered)
            self.assertEqual(exited.exception.code, expected)
            self.assertIs(observed[1].exceptions[0], failure)
            self.assertIs(observed[1].exceptions[1], publication)
            self.assertNotIn('PRIVATE_TERMINAL', stream.getvalue())
            self._healthy('execve')


if __name__ == '__main__':
    unittest.main(verbosity=2)
