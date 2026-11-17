import contextlib
"""Focused CD01 wrapper diagnostic-custody regression; no children/compiler."""
import errno
import hashlib
import io
import unittest
from unittest import mock
import cd01_capture_rustc_wrapper as wrapper

class DiagnosticCustodyTests(unittest.TestCase):
    def test_reporting_failure_keeps_original_and_cleanup_causes(self):
        """Healthy safe report, failed report cause custody, then fresh recovery."""
        primary = OSError(errno.EIO, 'PRIMARY_PRIVATE_TEXT')
        secondary = OSError(errno.EBUSY, 'CLEANUP_PRIVATE_TEXT')
        causes = [('owned_close', secondary)]
        healthy = io.StringIO()
        with mock.patch.object(wrapper.sys, 'stderr', healthy):
            wrapper._emit_stderr(primary, 'owned_stage', causes, False)
        self.assertIn('errno=5', healthy.getvalue())
        self.assertIn('errno=16', healthy.getvalue())
        self.assertNotIn('PRIVATE_TEXT', healthy.getvalue())
        class Broken:
            def write(self, text):
                raise OSError(errno.ENOSPC, 'STDERR_PRIVATE_TEXT')
            def flush(self):
                raise AssertionError('unreachable after write failure')
        with mock.patch.object(wrapper.sys, 'stderr', Broken()):
            try:
                wrapper._emit_stderr(primary, 'owned_stage', causes, False)
            except BaseException as error:
                self.assertIs(error.primary, primary)
                self.assertIs(error.causes[0][1], secondary)
                self.assertEqual(error.causes[-1][0], 'stderr_publication')
                self.assertEqual(error.causes[-1][1].errno, errno.ENOSPC)
                self.assertNotIn('PRIVATE_TEXT', str(error))
            else:
                self.fail('reporting error was swallowed instead of retaining original/cleanup/report causes')
        recovery = io.StringIO()
        with mock.patch.object(wrapper.sys, 'stderr', recovery):
            wrapper._emit_stderr(primary, 'owned_stage', causes, False)
        self.assertIn('owned_close', recovery.getvalue())
        self.assertNotIn('PRIVATE_TEXT', recovery.getvalue())


import json
import os
from pathlib import Path
import tempfile

class ProducerContractTests(unittest.TestCase):
    """New producer intent/publication checks; exec always mocked, no children."""
    def invoke(self, root, argv=None):
        """Healthy publication, followed by a known synthetic exec failure."""
        stderr = io.StringIO()
        env = {b'CD01_CAPTURE_DIR': os.fsencode(root), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}
        with mock.patch.dict(os.environb, env), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv', side_effect=OSError(errno.ENOEXEC, 'EXEC_PRIVATE_TEXT')) as execute:
            result = wrapper.main(argv or ['wrapper', '/owned/compiler', 'ordinary'])
        self.assertEqual(result, 1)
        self.assertEqual(execute.call_count, 1)
        self.assertNotIn('PRIVATE_TEXT', stderr.getvalue())
        return execute.call_args

    def test_argument_bytes_and_distinct_private_intents(self):
        """Ordinary control, reversible nonUTF8/empty argv, fresh recovery."""
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            self.invoke(root)
            compiler = b'/owned/compiler-\xff'
            arguments = [b'', b'with space', b'bad-\xff', b'quote"slash\\']
            before = dict(os.environb)
            called = self.invoke(root, ['wrapper', os.fsdecode(compiler), *map(os.fsdecode, arguments)])
            self.assertEqual(dict(os.environb), before)
            self.assertEqual(called.args, (b'/usr/bin/true', [b'/usr/bin/true', compiler, *arguments]))
            found = []
            for directory in root.iterdir():
                record = json.loads((directory/'intent.json').read_bytes())
                self.assertEqual((directory/'intent.json').read_bytes(), (directory/'intent.partial').read_bytes())
                self.assertEqual((directory/'intent.json').stat().st_ino, (directory/'intent.partial').stat().st_ino)
                self.assertEqual(directory.stat().st_mode & 0o777, 0o700)
                self.assertEqual((directory/'intent.json').stat().st_mode & 0o777, 0o600)
                self.assertEqual(record['outcome'], 'UNKNOWN')
                self.assertEqual(record['record_kind'], 'pre_exec_intent')
                if bytes.fromhex(record['compiler_path_hex']) == compiler: found.append(record)
            self.assertEqual(len(found), 1)
            self.assertEqual([bytes.fromhex(x) for x in found[0]['compiler_arguments_hex']], arguments)
            self.assertEqual([bytes.fromhex(x) for x in found[0]['delegation']['argv_hex']], [b'/usr/bin/true', compiler, *arguments])
            self.invoke(root)
            self.assertEqual(len(list(root.iterdir())), 3)

    def test_short_write_control_zero_refusal_and_recovery(self):
        """Partial writes preserve bytes; zero progress refuses rather than loops."""
        payload = b'owned fixture bytes'
        seen = []
        def short(fd, view):
            part = bytes(view[:2]); seen.append(part); return len(part)
        with mock.patch.object(wrapper.os, 'write', short): wrapper._write_all(100, payload)
        self.assertEqual(b''.join(seen), payload)
        with mock.patch.object(wrapper.os, 'write', return_value=0):
            with self.assertRaises(OSError) as error: wrapper._write_all(100, payload)
        self.assertEqual(error.exception.errno, errno.EIO)
        seen.clear()
        with mock.patch.object(wrapper.os, 'write', short): wrapper._write_all(100, payload)
        self.assertEqual(b''.join(seen), payload)

    def test_visible_link_with_sync_failure_never_delegates(self):
        """Control before injection; visible publication is not inferred durable."""
        with tempfile.TemporaryDirectory() as name:
            root = Path(name); self.invoke(root)
            real_sync = os.fsync; injected = []
            def fail_after_link(fd):
                path = Path(os.readlink('/proc/self/fd/' + str(fd)))
                if path.is_dir() and (path/'intent.json').exists() and not injected:
                    injected.append(path); raise OSError(errno.EIO, 'SYNC_PRIVATE_TEXT')
                return real_sync(fd)
            stderr = io.StringIO()
            with mock.patch.dict(os.environb, {b'CD01_CAPTURE_DIR': os.fsencode(root), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}), mock.patch.object(wrapper.os, 'fsync', fail_after_link), mock.patch.object(wrapper.os, 'execv') as execute, mock.patch.object(wrapper.sys, 'stderr', stderr):
                result = wrapper.main(['wrapper', '/owned/compiler'])
            self.assertEqual(result, 1); execute.assert_not_called(); self.assertEqual(len(injected), 1)
            failed = json.loads((injected[0]/'failure.json').read_bytes())
            self.assertTrue(failed['publication']['intent_json_published'])
            self.assertFalse(failed['publication']['invocation_dir_fsynced_after_link'])
            self.assertFalse(failed['exec_attempted']); self.assertEqual(failed['primary_errno'], errno.EIO)
            self.assertNotIn('PRIVATE_TEXT', stderr.getvalue()); self.invoke(root)

    def test_write_and_close_failures_keep_both_causes(self):
        """Control, partial-write primary plus cleanup cause, fresh recovery."""
        with tempfile.TemporaryDirectory() as name:
            root = Path(name); self.invoke(root)
            real_write = wrapper._write_all; real_close = os.close; failed = []
            def path_of(fd): return Path(os.readlink('/proc/self/fd/' + str(fd)))
            def write_fail(fd, payload):
                if path_of(fd).name == 'intent.partial':
                    os.write(fd, payload[:8]); failed.append(path_of(fd).parent)
                    raise OSError(errno.EIO, 'WRITE_PRIVATE_TEXT')
                return real_write(fd, payload)
            def close_fail(fd):
                target = path_of(fd); real_close(fd)
                if target.name == 'intent.partial': raise OSError(errno.EBUSY, 'CLOSE_PRIVATE_TEXT')
            stderr = io.StringIO()
            with mock.patch.dict(os.environb, {b'CD01_CAPTURE_DIR': os.fsencode(root), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}), mock.patch.object(wrapper, '_write_all', write_fail), mock.patch.object(wrapper.os, 'close', close_fail), mock.patch.object(wrapper.os, 'execv') as execute, mock.patch.object(wrapper.sys, 'stderr', stderr):
                result = wrapper.main(['wrapper', '/owned/compiler'])
            self.assertEqual(result, 1); execute.assert_not_called(); self.assertEqual(len(failed), 1)
            self.assertEqual((failed[0]/'intent.partial').stat().st_size, 8)
            self.assertFalse((failed[0]/'intent.json').exists())
            self.assertEqual(json.loads((failed[0]/'failure.json').read_bytes())['primary_errno'], errno.EIO)
            self.assertIn('errno=5', stderr.getvalue()); self.assertIn('errno=16', stderr.getvalue())
            self.assertNotIn('PRIVATE_TEXT', stderr.getvalue()); self.invoke(root)


class ProducerBoundaryTests(unittest.TestCase):
    """Additional changed-producer flush and exclusive-create boundaries."""
    def test_flush_reporting_failure_retains_cause(self):
        """Healthy report, flush failure after write, then a fresh healthy report."""
        primary = OSError(errno.EIO, 'PRIMARY_PRIVATE_TEXT')
        for phase in ['control', 'fault', 'recovery']:
            stream = io.StringIO()
            if phase == 'fault':
                with mock.patch.object(stream, 'flush', side_effect=OSError(errno.ENOSPC, 'FLUSH_PRIVATE_TEXT')), mock.patch.object(wrapper.sys, 'stderr', stream):
                    with self.assertRaises(wrapper.WrapperReportingError) as error:
                        wrapper._emit_stderr(primary, 'owned_stage', [], False)
                self.assertIs(error.exception.primary, primary)
                self.assertEqual(error.exception.causes[-1][1].errno, errno.ENOSPC)
                self.assertNotIn('PRIVATE_TEXT', str(error.exception))
            else:
                with mock.patch.object(wrapper.sys, 'stderr', stream): wrapper._emit_stderr(primary, 'owned_stage', [], False)
            self.assertIn('errno=5', stream.getvalue()); self.assertNotIn('PRIVATE_TEXT', stream.getvalue())

    def test_invocation_collision_refuses_without_overwrite(self):
        """Published control survives an exclusive-name collision; fresh recovery."""
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            with mock.patch.object(wrapper.os, 'urandom', return_value=b'\x00' * 16):
                ProducerContractTests().invoke(root)
                prior = {str(p.relative_to(root)): p.read_bytes() for p in root.rglob('*') if p.is_file()}
                stderr = io.StringIO()
                with mock.patch.dict(os.environb, {b'CD01_CAPTURE_DIR': os.fsencode(root), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}), mock.patch.object(wrapper.os, 'execv') as execute, mock.patch.object(wrapper.sys, 'stderr', stderr):
                    status = wrapper.main(['wrapper', '/owned/compiler'])
                self.assertEqual(status, 1); execute.assert_not_called()
                self.assertIn('errno=' + str(errno.EEXIST), stderr.getvalue())
                self.assertEqual(prior, {str(p.relative_to(root)): p.read_bytes() for p in root.rglob('*') if p.is_file()})
            ProducerContractTests().invoke(root)
            self.assertEqual(len(list(root.iterdir())), 2)


class StdinLeaseTransferTests(unittest.TestCase):
    """Targeted packet20 stdin-lease guards; exec mocked, descriptor 0 self-restored.

    Every selected invocation runs inside this unittest process: os.execv is
    always mocked, so no compiler, guard executable, service or provider ever
    runs and no historical campaign is replayed. Each test that can reach the
    transfer flow saves descriptor 0 first and restores it in a finally block,
    so even a failed assertion cannot leak a replaced descriptor. All fixtures
    live under the caller's TMPDIR via tempfile; there is no arbitrary timeout,
    size cap or retry anywhere, and every injection is preceded by a
    same-precondition healthy control and followed by a fresh recovery.
    """

    def setUp(self):
        """Retain fixtures and restore the original stdin inheritable flag."""
        flag = os.get_inheritable(0)
        self.addCleanup(os.set_inheritable, 0, flag)

    @contextlib.contextmanager
    def _retained_dir(self):
        """Yield a caller-owned fixture root without deleting failure evidence."""
        yield tempfile.mkdtemp(prefix='stdin-transfer-tests-')

    def _invocation(self, capture_dir):
        """Resolve the sole actual child invocation, distinct from its parent."""
        found = list(capture_dir.glob('cd01-intent-*'))
        self.assertEqual(len(found), 1)
        return found[0]

    def _lease(self, root, payload, allocation_id='alloc-packet20', compiler='/owned/compiler', arguments=('ordinary',)):
        """Create one fresh private capture directory, input fixture and valid lease."""
        capture_dir = root / ('lease-' + os.urandom(8).hex())
        capture_dir.mkdir(mode=0o700)
        input_path = root / ('input-' + os.urandom(8).hex() + '.bin')
        input_path.write_bytes(payload)
        input_fd = os.open(str(input_path), os.O_RDONLY)
        self.addCleanup(os.close, input_fd)
        lease = {
            'allocation_id': allocation_id,
            'input_fd': input_fd,
            'compiler_path_hex': os.fsencode(compiler).hex(),
            'compiler_arguments_hex': [os.fsencode(argument).hex() for argument in arguments],
            'cwd_hex': os.getcwdb().hex(),
        }
        return capture_dir, input_fd, lease

    def _run_lease(self, capture_dir, lease, reads):
        """Run one selected invocation with mocked exec that double-reads fd 0."""
        def fake_exec(executable, delegation_argv):
            """Read the transferred descriptor in-process, then simulate exec failure."""
            reads.append(os.read(0, 65536))
            reads.append(os.read(0, 65536))
            raise OSError(errno.ENOEXEC, 'EXEC_PRIVATE_TEXT')
        stderr = io.StringIO()
        env = {b'CD01_CAPTURE_DIR': os.fsencode(str(capture_dir)), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}
        with mock.patch.dict(os.environb, env), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv', side_effect=fake_exec) as execute:
            result = wrapper.main(['wrapper', '/owned/compiler', 'ordinary'], stdin_lease=lease)
        self.assertNotIn('PRIVATE_TEXT', stderr.getvalue())
        return result, stderr, execute

    def _healthy(self, root, payload):
        """One same-precondition healthy selected invocation; returns its directory."""
        capture_dir, _, lease = self._lease(root, payload)
        reads = []
        result, _, execute = self._run_lease(capture_dir, lease, reads)
        self.assertEqual(reads, [payload, b''])
        self.assertEqual(result, 1)
        self.assertEqual(execute.call_count, 1)
        return self._invocation(capture_dir)

    def _recovered(self, root, payload):
        """One fresh-recovery healthy invocation; asserts the fd 0 double read."""
        capture_dir, _, lease = self._lease(root, payload)
        reads = []
        result, _, execute = self._run_lease(capture_dir, lease, reads)
        self.assertEqual(result, 1)
        self.assertEqual(execute.call_count, 1)
        self.assertEqual(reads, [payload, b''])
        return self._invocation(capture_dir)

    def test_unselected_lease_keeps_exact_v1_protocol(self):
        """No lease: byte-exact current v1 intent/failure records, no custody, no fd 0 use."""
        with self._retained_dir() as name:
            root = Path(name)
            stderr = io.StringIO()
            env = {b'CD01_CAPTURE_DIR': os.fsencode(str(root)), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}
            with mock.patch.dict(os.environb, env), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv', side_effect=OSError(errno.ENOEXEC, 'EXEC_PRIVATE_TEXT')) as execute:
                result = wrapper.main(['wrapper', '/owned/compiler', 'or-\xff'])
            self.assertEqual(result, 1)
            self.assertEqual(execute.call_count, 1)
            self.assertNotIn('PRIVATE_TEXT', stderr.getvalue())
            directories = list(root.iterdir())
            self.assertEqual(len(directories), 1)
            directory = directories[0]
            self.assertEqual(sorted(p.name for p in directory.iterdir()), ['failure.json', 'intent.json', 'intent.partial'])
            expected = wrapper._render_intent_record(os.fsencode('/owned/compiler'), [os.fsencode('or-\xff')], b'/usr/bin/true')
            self.assertEqual((directory / 'intent.partial').read_bytes(), expected)
            self.assertEqual((directory / 'intent.json').read_bytes(), expected)
            record = json.loads(expected)
            self.assertEqual(record['schema'], 'cd01.rustc_intent.v1')
            self.assertEqual(record['outcome'], 'UNKNOWN')
            self.assertNotIn('stdin_transfer', record)
            failure = json.loads((directory / 'failure.json').read_bytes())
            self.assertNotIn('stdin_transfer', failure)
            self.assertTrue(failure['exec_attempted'])

    def test_healthy_lease_transfers_bytes_to_fd0(self):
        """Selected lease: raw/nonUTF8 bytes reach fd 0 by real dup2; mocked exec double-reads them."""
        payload = b'\x00\xff\xfe raw \x80 nonUTF8 \xc3(\x7f bytes'
        saved0 = os.dup(0)
        try:
            with self._retained_dir() as name:
                root = Path(name)
                capture_dir, input_fd, lease = self._lease(root, payload)
                reads = []
                result, stderr, execute = self._run_lease(capture_dir, lease, reads)
                capture_dir = self._invocation(capture_dir)
                self.assertEqual(result, 1)
                self.assertEqual(execute.call_count, 1)
                self.assertTrue(os.get_inheritable(0))
                self.assertEqual(reads, [payload, b''])
                os.fstat(input_fd)
                self.assertEqual(sorted(p.name for p in capture_dir.iterdir()), ['failure.json', 'intent.json', 'intent.partial', 'stdin-custody', 'stdin-transfer.json'])
                custody = capture_dir / 'stdin-custody'
                self.assertEqual(sorted(p.name for p in custody.iterdir()), ['input.bin', 'input.partial', 'manifest.json', 'manifest.partial'])
                self.assertEqual((custody / 'input.bin').read_bytes(), payload)
                manifest = json.loads((custody / 'manifest.json').read_bytes())
                self.assertTrue(manifest['eof_observed'])
                self.assertEqual(manifest['byte_count'], len(payload))
                self.assertEqual(manifest['sha256'], hashlib.sha256(payload).hexdigest())
                expected_intent = wrapper._render_intent_record(os.fsencode('/owned/compiler'), [os.fsencode('ordinary')], b'/usr/bin/true', {'allocation_id': 'alloc-packet20', 'custody_relative_directory': 'stdin-custody'})
                self.assertEqual((capture_dir / 'intent.json').read_bytes(), expected_intent)
                self.assertEqual((capture_dir / 'intent.partial').read_bytes(), expected_intent)
                intent = json.loads(expected_intent)
                self.assertEqual(intent['schema'], 'cd01.rustc_intent.v2')
                self.assertEqual(intent['stdin_transfer']['status'], 'PENDING')
                self.assertEqual(intent['stdin_transfer']['allocation_id'], 'alloc-packet20')
                self.assertEqual(intent['stdin_transfer']['custody_relative_directory'], 'stdin-custody')
                receipt = json.loads((capture_dir / 'stdin-transfer.json').read_bytes())
                self.assertEqual(receipt['schema'], 'cd01.stdin_transfer.v1')
                self.assertEqual(receipt['outcome'], 'UNKNOWN')
                self.assertEqual(receipt['allocation_id'], 'alloc-packet20')
                self.assertEqual(receipt['observed_custody_manifest']['byte_count'], len(payload))
                self.assertEqual(receipt['observed_custody_manifest']['sha256'], manifest['sha256'])
                self.assertTrue(receipt['descriptor_replacement']['dup2_completed'])
                self.assertTrue(receipt['descriptor_replacement']['spare_close_completed'])
                self.assertEqual(receipt['delegation']['outcome'], 'UNKNOWN')
                failure = json.loads((capture_dir / 'failure.json').read_bytes())
                self.assertTrue(failure['exec_attempted'])
                transfer = failure['stdin_transfer']
                self.assertTrue(transfer['capture_completed'])
                self.assertTrue(transfer['fd_dup2_completed'])
                self.assertTrue(transfer['spare_close_completed'])
                self.assertTrue(transfer['receipt_published'])
        finally:
            os.dup2(saved0, 0)
            os.close(saved0)

    def test_lease_mismatch_refuses_before_mutation_or_read(self):
        """Control first; a mismatching lease refuses 78 with no directory, read or exec."""
        saved0 = os.dup(0)
        try:
            with self._retained_dir() as name:
                root = Path(name)
                self._healthy(root, b'control bytes')
                mismatch_dir, mismatch_fd, mismatch_lease = self._lease(root, b'unread bytes')
                mismatch_lease['compiler_path_hex'] = os.fsencode('/other/compiler').hex()
                stderr = io.StringIO()
                env = {b'CD01_CAPTURE_DIR': os.fsencode(str(mismatch_dir)), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}
                with mock.patch.dict(os.environb, env), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv') as execute:
                    result = wrapper.main(['wrapper', '/owned/compiler', 'ordinary'], stdin_lease=mismatch_lease)
                self.assertEqual(result, 78)
                execute.assert_not_called()
                self.assertIn('does not match argv[1]', stderr.getvalue())
                self.assertEqual(list(mismatch_dir.iterdir()), [])
                self.assertEqual(os.lseek(mismatch_fd, 0, os.SEEK_CUR), 0)
                self._recovered(root, b'recovery bytes')
        finally:
            os.dup2(saved0, 0)
            os.close(saved0)

    def test_capture_fault_keeps_capsule_and_never_delegates(self):
        """Control, helper read fault with bounded original-cause capsule, fresh recovery."""
        import finite_input_custody
        saved0 = os.dup(0)
        try:
            with self._retained_dir() as name:
                root = Path(name)
                self._healthy(root, b'control bytes')
                fault_dir, _, fault_lease = self._lease(root, b'fault bytes')
                stderr = io.StringIO()
                env = {b'CD01_CAPTURE_DIR': os.fsencode(str(fault_dir)), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}
                with mock.patch.object(wrapper, '_load_custody_module', return_value=finite_input_custody), mock.patch.dict(os.environb, env), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv') as execute, mock.patch.object(finite_input_custody, '_os_read', side_effect=OSError(errno.EIO, 'READ_PRIVATE_TEXT')):
                    result = wrapper.main(['wrapper', '/owned/compiler', 'ordinary'], stdin_lease=fault_lease)
                fault_dir = self._invocation(fault_dir)
                self.assertEqual(result, 1)
                execute.assert_not_called()
                self.assertNotIn('PRIVATE_TEXT', stderr.getvalue())
                self.assertIn('FiniteInputCustodyError', stderr.getvalue())
                self.assertEqual(sorted(p.name for p in fault_dir.iterdir()), ['failure.json', 'intent.json', 'intent.partial', 'stdin-custody'])
                self.assertEqual(sorted(x.name for x in (fault_dir / 'stdin-custody').iterdir()), ['input.partial'])
                self.assertEqual((fault_dir / 'intent.json').read_bytes(), (fault_dir / 'intent.partial').read_bytes())
                failure = json.loads((fault_dir / 'failure.json').read_bytes())
                self.assertEqual(failure['primary_error_type'], 'FiniteInputCustodyError')
                self.assertFalse(failure['exec_attempted'])
                transfer = failure['stdin_transfer']
                self.assertTrue(transfer['capture_attempted'])
                self.assertFalse(transfer['capture_completed'])
                self.assertFalse(transfer['fd_dup2_attempted'])
                capsule = transfer['custody_error']
                self.assertEqual(capsule['stage'], 'read-input')
                self.assertEqual(capsule['primary']['op'], 'read')
                self.assertEqual(capsule['primary']['name'], 'input-fd')
                self.assertEqual(capsule['primary']['errno'], errno.EIO)
                self.assertIsNone(capsule['primary']['detail'])
                self.assertNotIn('stdin-transfer.json', [p.name for p in fault_dir.iterdir()])
                self._recovered(root, b'recovery bytes')
        finally:
            os.dup2(saved0, 0)
            os.close(saved0)

    def test_dup2_fault_reports_attempt_and_never_delegates(self):
        """Control, dup2 fault after successful capture, then fresh recovery."""
        saved0 = os.dup(0)
        try:
            with self._retained_dir() as name:
                root = Path(name)
                self._healthy(root, b'control bytes')
                os.dup2(saved0, 0)
                fault_dir, _, fault_lease = self._lease(root, b'fault bytes')
                stderr = io.StringIO()
                env = {b'CD01_CAPTURE_DIR': os.fsencode(str(fault_dir)), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}
                with mock.patch.dict(os.environb, env), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv') as execute, mock.patch.object(wrapper.os, 'dup2', side_effect=OSError(errno.EBADF, 'DUP2_PRIVATE_TEXT')):
                    result = wrapper.main(['wrapper', '/owned/compiler', 'ordinary'], stdin_lease=fault_lease)
                fault_dir = self._invocation(fault_dir)
                self.assertEqual(result, 1)
                execute.assert_not_called()
                self.assertNotIn('PRIVATE_TEXT', stderr.getvalue())
                failure = json.loads((fault_dir / 'failure.json').read_bytes())
                transfer = failure['stdin_transfer']
                self.assertTrue(transfer['capture_attempted'])
                self.assertTrue(transfer['capture_completed'])
                self.assertTrue(transfer['fd_dup2_attempted'])
                self.assertFalse(transfer['fd_dup2_completed'])
                self.assertFalse(transfer['spare_close_attempted'])
                self.assertFalse(transfer['receipt_published'])
                self.assertFalse(failure['exec_attempted'])
                self.assertNotIn('stdin-transfer.json', [p.name for p in fault_dir.iterdir()])
                self.assertEqual(os.fstat(0).st_ino, os.fstat(saved0).st_ino)
                self._recovered(root, b'recovery bytes')
        finally:
            os.dup2(saved0, 0)
            os.close(saved0)

    def test_close_or_receipt_failure_keeps_uncertain_state(self):
        """Spare-close and receipt-fsync faults: no exec, uncertain fd 0 state retained."""
        import finite_input_custody
        real_close = os.close
        real_fsync = os.fsync
        saved0 = os.dup(0)
        try:
            with self._retained_dir() as name:
                root = Path(name)
                # Phase A: healthy control, then the spare close fails after a real dup2.
                self._healthy(root, b'control-a bytes')
                os.dup2(saved0, 0)
                fault_dir, _, lease = self._lease(root, b'fault-a bytes')
                closed_spares = []
                def close_fail(fd):
                    """Close the selected spare then report its uncertain outcome once."""
                    target = Path(os.readlink('/proc/self/fd/' + str(fd)))
                    real_close(fd)
                    if target.name == 'input.bin' and not closed_spares:
                        closed_spares.append(target)
                        raise OSError(errno.EBUSY, 'CLOSE_PRIVATE_TEXT')
                stderr = io.StringIO()
                env = {b'CD01_CAPTURE_DIR': os.fsencode(str(fault_dir)), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}
                with mock.patch.object(wrapper, '_load_custody_module', return_value=finite_input_custody), mock.patch.dict(os.environb, env), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv') as execute, mock.patch.object(wrapper.os, 'close', close_fail):
                    result = wrapper.main(['wrapper', '/owned/compiler', 'ordinary'], stdin_lease=lease)
                fault_dir = self._invocation(fault_dir)
                self.assertEqual(result, 1)
                execute.assert_not_called()
                self.assertNotIn('PRIVATE_TEXT', stderr.getvalue())
                self.assertEqual(len(closed_spares), 1)
                failure = json.loads((fault_dir / 'failure.json').read_bytes())
                transfer = failure['stdin_transfer']
                self.assertTrue(transfer['capture_completed'])
                self.assertTrue(transfer['fd_dup2_completed'])
                self.assertTrue(transfer['spare_close_attempted'])
                self.assertFalse(transfer['spare_close_completed'])
                self.assertFalse(transfer['receipt_published'])
                self.assertFalse(failure['exec_attempted'])
                self.assertNotIn('stdin-transfer.json', [p.name for p in fault_dir.iterdir()])
                self.assertEqual(os.readlink('/proc/self/fd/0').rsplit('/', 1)[-1], 'input.bin')
                os.dup2(saved0, 0)
                self._recovered(root, b'recovery-a bytes')
                os.dup2(saved0, 0)
                # Phase B: healthy control, then the receipt directory fsync fails.
                self._healthy(root, b'control-b bytes')
                os.dup2(saved0, 0)
                fault_dir, _, lease = self._lease(root, b'fault-b bytes')
                synced = []
                def fsync_fail(fd):
                    """Fail the receipt directory sync after a visible publication."""
                    path = Path(os.readlink('/proc/self/fd/' + str(fd)))
                    if path.is_dir() and (path / 'stdin-transfer.json').exists() and not synced:
                        synced.append(path)
                        raise OSError(errno.EIO, 'SYNC_PRIVATE_TEXT')
                    return real_fsync(fd)
                stderr = io.StringIO()
                env = {b'CD01_CAPTURE_DIR': os.fsencode(str(fault_dir)), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}
                with mock.patch.object(wrapper, '_load_custody_module', return_value=finite_input_custody), mock.patch.dict(os.environb, env), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv') as execute, mock.patch.object(wrapper.os, 'fsync', fsync_fail):
                    result = wrapper.main(['wrapper', '/owned/compiler', 'ordinary'], stdin_lease=lease)
                fault_dir = self._invocation(fault_dir)
                self.assertEqual(result, 1)
                execute.assert_not_called()
                self.assertNotIn('PRIVATE_TEXT', stderr.getvalue())
                self.assertEqual(len(synced), 1)
                failure = json.loads((fault_dir / 'failure.json').read_bytes())
                transfer = failure['stdin_transfer']
                self.assertTrue(transfer['fd_dup2_completed'])
                self.assertTrue(transfer['spare_close_completed'])
                self.assertTrue(transfer['receipt_published'])
                self.assertFalse(transfer['receipt_dir_fsynced'])
                self.assertFalse(failure['exec_attempted'])
                self.assertEqual(os.readlink('/proc/self/fd/0').rsplit('/', 1)[-1], 'input.bin')
                self._recovered(root, b'recovery-b bytes')
        finally:
            os.dup2(saved0, 0)
            os.close(saved0)

    def test_missing_sibling_refuses_cached_module(self):
        """Healthy transfer, missing bound sibling despite cache, fresh recovery."""
        import finite_input_custody
        saved0 = os.dup(0)
        try:
            with self._retained_dir() as name:
                root = Path(name)
                self._healthy(root, b'loader-control')
                capture_dir, input_fd, lease = self._lease(root, b'must-stay-unread')
                absent = root / 'no-sibling' / 'wrapper.py'
                env = {b'CD01_CAPTURE_DIR': os.fsencode(capture_dir), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}
                stderr = io.StringIO()
                with mock.patch.object(wrapper, '__file__', str(absent)), mock.patch.dict(os.environb, env), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv', side_effect=OSError(errno.ENOEXEC, 'OWNED_EXEC')) as execute:
                    result = wrapper.main(['wrapper', '/owned/compiler', 'ordinary'], stdin_lease=lease)
                self._recovered(root, b'loader-recovery')
                self.assertNotEqual(result, 0)
                execute.assert_not_called()
                self.assertEqual(os.lseek(input_fd, 0, os.SEEK_CUR), 0)
        finally:
            os.dup2(saved0, 0)
            os.close(saved0)

    def test_nonexact_input_fd_refuses_before_mutation(self):
        """Healthy transfer, nonexact integer descriptor refused, fresh recovery."""
        class Descriptor(int):
            """Fixture integer subclass, excluded by exact-int lease schema."""
        saved0 = os.dup(0)
        try:
            with self._retained_dir() as name:
                root = Path(name)
                self._healthy(root, b'type-control')
                capture_dir, input_fd, lease = self._lease(root, b'unread-type-input')
                lease['input_fd'] = Descriptor(input_fd)
                env = {b'CD01_CAPTURE_DIR': os.fsencode(capture_dir), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}
                stderr = io.StringIO()
                with mock.patch.dict(os.environb, env), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv', side_effect=OSError(errno.ENOEXEC, 'OWNED_EXEC')) as execute:
                    result = wrapper.main(['wrapper', '/owned/compiler', 'ordinary'], stdin_lease=lease)
                self._recovered(root, b'type-recovery')
                self.assertEqual(result, 78)
                execute.assert_not_called()
                self.assertEqual(list(capture_dir.iterdir()), [])
                self.assertEqual(os.lseek(input_fd, 0, os.SEEK_CUR), 0)
        finally:
            os.dup2(saved0, 0)
            os.close(saved0)


    def test_normalized_capture_interrupt_keeps_category(self):
        """Control, normalized primary/cleanup interruptions, fresh recovery per case."""
        import finite_input_custody
        saved0 = os.dup(0)
        try:
            with self._retained_dir() as name:
                root = Path(name)
                for mode in ['primary', 'cleanup']:
                    with self.subTest(mode=mode):
                        self._healthy(root, b'interrupt-control')
                        capture_dir, _, lease = self._lease(root, b'interrupt-input')
                        interrupt = KeyboardInterrupt('PRIVATE_INTERRUPT')
                        primary = interrupt if mode == 'primary' else OSError(errno.EIO, 'PRIVATE_INPUT')
                        observed = []
                        closed = []
                        real_handle = wrapper._handle_failure
                        real_close = finite_input_custody._os_close
                        def observe(error, *args):
                            """Retain normalized original causes before safe wrapper reporting."""
                            observed.append(error)
                            return real_handle(error, *args)
                        def close_then_interrupt(fd):
                            """Close an owned fixture fd, then inject one cleanup interruption."""
                            real_close(fd)
                            if mode == 'cleanup' and not closed:
                                closed.append(fd)
                                raise interrupt
                        env = {b'CD01_CAPTURE_DIR': os.fsencode(capture_dir), b'CD01_SPACEGUARD_PATH': b'/usr/bin/true'}
                        stderr = io.StringIO()
                        with mock.patch.object(wrapper, '_load_custody_module', return_value=finite_input_custody), mock.patch.dict(os.environb, env), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv') as execute, mock.patch.object(finite_input_custody, '_os_read', side_effect=primary), mock.patch.object(finite_input_custody, '_os_close', side_effect=close_then_interrupt), mock.patch.object(wrapper, '_handle_failure', side_effect=observe):
                            result = wrapper.main(['wrapper', '/owned/compiler', 'ordinary'], stdin_lease=lease)
                        self._recovered(root, b'interrupt-recovery')
                        self.assertEqual(result, 130)
                        execute.assert_not_called()
                        self.assertIs(observed[0].original_exception, primary)
                        self.assertIn(interrupt, (observed[0].original_exception, *observed[0].secondary_exceptions))
                        self.assertIn('interrupted', stderr.getvalue())
                        self.assertNotIn('PRIVATE', stderr.getvalue())
                        directory = self._invocation(capture_dir)
                        failure = json.loads((directory / 'failure.json').read_bytes())
                        self.assertTrue(failure['interrupted'])
                        self.assertTrue(failure['stdin_transfer']['custody_error']['interrupted'])
                        self.assertFalse(failure['exec_attempted'])
        finally:
            os.dup2(saved0, 0)
            os.close(saved0)


class StdinManifestSelectionTests(unittest.TestCase):
    """Targeted packet23 operator-manifest selection guards; exec mocked, fd 0 self-restored.

    Every invocation runs inside this unittest process: os.execv is always
    mocked with a deliberate ENOSYS, so a reached delegation reports rc 1
    and never success; no compiler, guard executable, provider or collector
    runs and no historical campaign is replayed. The paired operator
    manifest environment is the only new selection input. Each test that
    can reach selection or transfer saves descriptor 0 first, installs its
    own fake descriptor-0 fixture and restores the original in a finally
    block, so even a failed assertion cannot leak a replaced descriptor;
    the borrowed input behind descriptor 0 is never closed during a run.
    Fixtures stay under the caller's TMPDIR and are retained; every
    injection is preceded by a same-precondition healthy control and
    followed by a fresh recovery.
    """

    def setUp(self):
        """Retain fixtures and restore the original stdin inheritable flag."""
        flag = os.get_inheritable(0)
        self.addCleanup(os.set_inheritable, 0, flag)

    @contextlib.contextmanager
    def _retained_dir(self):
        """Yield a caller-owned fixture root without deleting failure evidence."""
        yield tempfile.mkdtemp(prefix='stdin-manifest-tests-')

    def _capture_dir(self, root):
        """Create one fresh private capture directory under the fixture root."""
        capture_dir = root / ('manifest-' + os.urandom(8).hex())
        capture_dir.mkdir(mode=0o700)
        return capture_dir

    def _document(self, *entries):
        """Build one exact-protocol manifest document from lease entries."""
        return {
            'schema': 'cd01.stdin_lease_manifest/1',
            'record_kind': 'operator_stdin_lease_manifest',
            'leases': list(entries),
        }

    def _entry(self, allocation_id='alloc-packet23', compiler='/owned/compiler',
               arguments=('ordinary',), input_fd=0, repeat_policy='repeatable',
               declarations=None, selector=None):
        """Build one valid lease entry; keyword overrides drive refusal shapes."""
        entry = {
            'allocation_id': allocation_id,
            'input_fd': input_fd,
            'selector': {
                'compiler_path_hex': os.fsencode(compiler).hex(),
                'compiler_arguments_hex': [os.fsencode(argument).hex() for argument in arguments],
                'cwd_hex': os.getcwdb().hex(),
            },
            'declarations': {
                'finite_cooperative_single_writer_input': True,
                'producer_will_close_input_fd': True,
                'persistence_authorized': True,
            },
            'repeat_policy': repeat_policy,
        }
        if selector is not None:
            entry['selector'] = selector
        if declarations is not None:
            entry['declarations'] = declarations
        return entry

    def _manifest(self, root, document, raw=None, mode=0o600):
        """Write one manifest fixture; return (path bytes, sha256 bytes)."""
        if raw is None:
            raw = (json.dumps(document, ensure_ascii=True, sort_keys=True, indent=2) + '\n').encode('ascii')
        path = root / ('manifest-' + os.urandom(8).hex() + '.json')
        path.write_bytes(raw)
        os.chmod(str(path), mode)
        return os.fsencode(str(path)), hashlib.sha256(raw).hexdigest().encode('ascii')

    def _fake_fd0(self, root, payload):
        """Install one private descriptor-0 fixture; return (saved fd, fixture fd).

        The fixture descriptor stays open for the whole test: it is the
        borrowed input behind descriptor 0 and is closed by cleanup only
        after the caller restored the saved original in its finally block.
        """
        saved0 = os.dup(0)
        fixture_path = root / ('fd0-' + os.urandom(8).hex() + '.bin')
        fixture_path.write_bytes(payload)
        fixture_fd = os.open(str(fixture_path), os.O_RDONLY)
        self.addCleanup(os.close, fixture_fd)
        os.dup2(fixture_fd, 0)
        return saved0, fixture_fd

    def _library_lease(self, root, payload):
        """Create one fresh capture directory, borrowed input fd and valid lease."""
        capture_dir = self._capture_dir(root)
        input_path = root / ('input-' + os.urandom(8).hex() + '.bin')
        input_path.write_bytes(payload)
        input_fd = os.open(str(input_path), os.O_RDONLY)
        self.addCleanup(os.close, input_fd)
        lease = {
            'allocation_id': 'alloc-library',
            'input_fd': input_fd,
            'compiler_path_hex': os.fsencode('/owned/compiler').hex(),
            'compiler_arguments_hex': [os.fsencode('ordinary').hex()],
            'cwd_hex': os.getcwdb().hex(),
        }
        return capture_dir, input_fd, lease

    def _run(self, capture_dir, argv=None, env_extra=None, stdin_lease=None):
        """Run one in-process invocation; exec is mocked ENOSYS (rc 1, never success)."""
        stderr = io.StringIO()
        env = {
            b'CD01_CAPTURE_DIR': os.fsencode(str(capture_dir)),
            b'CD01_SPACEGUARD_PATH': b'/usr/bin/true',
        }
        if env_extra:
            env.update(env_extra)
        with mock.patch.dict(os.environb, env, clear=True), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv', side_effect=OSError(errno.ENOSYS, 'EXEC_PRIVATE_TEXT')) as execute:
            result = wrapper.main(argv or ['wrapper', '/owned/compiler', 'ordinary'], stdin_lease=stdin_lease)
        self.assertNotIn('PRIVATE_TEXT', stderr.getvalue())
        return result, stderr, execute

    def _invocation(self, capture_dir):
        """Resolve the sole actual child invocation, distinct from its parent."""
        found = list(Path(capture_dir).glob('cd01-intent-*'))
        self.assertEqual(len(found), 1)
        return found[0]

    def test_absent_pair_keeps_exact_v1_without_descriptor_read(self):
        """Both manifest variables absent: byte-exact v1 flow, descriptor 0 untouched."""
        with self._retained_dir() as name:
            root = Path(name)
            capture_dir = self._capture_dir(root)
            saved0, _ = self._fake_fd0(root, b'absent-pair bytes')
            try:
                result, stderr, execute = self._run(capture_dir)
                self.assertEqual(result, 1)
                self.assertEqual(execute.call_count, 1)
                self.assertEqual(os.lseek(0, 0, os.SEEK_CUR), 0)
                directory = self._invocation(capture_dir)
                expected = wrapper._render_intent_record(
                    os.fsencode('/owned/compiler'), [os.fsencode('ordinary')], b'/usr/bin/true')
                self.assertEqual((directory / 'intent.json').read_bytes(), expected)
                record = json.loads(expected)
                self.assertEqual(record['schema'], 'cd01.rustc_intent.v1')
                self.assertNotIn('stdin_transfer', record)
                self.assertNotIn('stdin-custody', [p.name for p in directory.iterdir()])
            finally:
                os.dup2(saved0, 0)
                os.close(saved0)
            # Fresh recovery: the healthy complete pair still selects and transfers.
            manifest_path, manifest_sha = self._manifest(root, self._document(self._entry()))
            recovery_dir = self._capture_dir(root)
            saved1, _ = self._fake_fd0(root, b'recovery bytes')
            try:
                result, stderr, execute = self._run(recovery_dir, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': manifest_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': manifest_sha})
                self.assertEqual(result, 1)
                self.assertEqual(execute.call_count, 1)
                recovery = self._invocation(recovery_dir)
                receipt = json.loads((recovery / 'stdin-transfer.json').read_bytes())
                self.assertEqual(receipt['schema'], 'cd01.stdin_transfer.v1')
                self.assertEqual(receipt['allocation_id'], 'alloc-packet23')
            finally:
                os.dup2(saved1, 0)
                os.close(saved1)

    def test_partial_pair_and_library_conflict_refuse_before_effects(self):
        """Control, partial-pair and library+manifest refusals before any effect, recovery."""
        with self._retained_dir() as name:
            root = Path(name)
            control_path, control_sha = self._manifest(root, self._document(self._entry(allocation_id='alloc-control')))
            control_dir = self._capture_dir(root)
            saved0, _ = self._fake_fd0(root, b'control bytes')
            try:
                result, stderr, execute = self._run(control_dir, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': control_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': control_sha})
                self.assertEqual(result, 1)
                self.assertEqual(execute.call_count, 1)
                self.assertIn('stdin-transfer.json', [p.name for p in self._invocation(control_dir).iterdir()])
            finally:
                os.dup2(saved0, 0)
                os.close(saved0)
            manifest_path, manifest_sha = self._manifest(root, self._document(self._entry()))
            _, lease_fd, lease = self._library_lease(root, b'library bytes')
            for env_extra, label in [
                    ({b'CD01_STDIN_LEASE_MANIFEST': manifest_path}, 'path-only'),
                    ({b'CD01_STDIN_LEASE_MANIFEST_SHA256': manifest_sha}, 'sha-only')]:
                with self.subTest(label=label):
                    directory = self._capture_dir(root)
                    saved1, _ = self._fake_fd0(root, b'partial bytes')
                    try:
                        result, stderr, execute = self._run(directory, env_extra=env_extra)
                        self.assertEqual(result, 78)
                        execute.assert_not_called()
                        self.assertIn('must be configured together', stderr.getvalue())
                        self.assertEqual(list(directory.iterdir()), [])
                        self.assertEqual(os.lseek(0, 0, os.SEEK_CUR), 0)
                    finally:
                        os.dup2(saved1, 0)
                        os.close(saved1)
            directory = self._capture_dir(root)
            saved2, _ = self._fake_fd0(root, b'conflict bytes')
            try:
                result, stderr, execute = self._run(directory, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': manifest_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': manifest_sha}, stdin_lease=lease)
                self.assertEqual(result, 78)
                execute.assert_not_called()
                self.assertIn('cannot be combined', stderr.getvalue())
                self.assertEqual(list(directory.iterdir()), [])
                self.assertEqual(os.lseek(lease_fd, 0, os.SEEK_CUR), 0)
                self.assertEqual(os.lseek(0, 0, os.SEEK_CUR), 0)
            finally:
                os.dup2(saved2, 0)
                os.close(saved2)
            recovery_dir = self._capture_dir(root)
            saved3, _ = self._fake_fd0(root, b'recovery bytes')
            try:
                result, stderr, execute = self._run(recovery_dir, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': manifest_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': manifest_sha})
                self.assertEqual(result, 1)
                self.assertEqual(execute.call_count, 1)
                self.assertIn('stdin-transfer.json', [p.name for p in self._invocation(recovery_dir).iterdir()])
            finally:
                os.dup2(saved3, 0)
                os.close(saved3)

    def test_manifest_hash_and_read_faults_keep_original_causes(self):
        """Healthy control, malformed/mismatched digest refusals, read IO/interrupt, recovery."""
        with self._retained_dir() as name:
            root = Path(name)
            manifest_path, manifest_sha = self._manifest(root, self._document(self._entry()))
            # Healthy same-precondition control: the exact pair selects normally.
            control_dir = self._capture_dir(root)
            saved0, _ = self._fake_fd0(root, b'control bytes')
            try:
                result, stderr, execute = self._run(control_dir, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': manifest_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': manifest_sha})
                self.assertEqual(result, 1)
                self.assertEqual(execute.call_count, 1)
                self.assertIn('stdin-transfer.json', [p.name for p in self._invocation(control_dir).iterdir()])
            finally:
                os.dup2(saved0, 0)
                os.close(saved0)
            for env_extra, label in [
                    ({b'CD01_STDIN_LEASE_MANIFEST': manifest_path,
                      b'CD01_STDIN_LEASE_MANIFEST_SHA256': b'0123456789ABCDEF' * 4}, 'uppercase-sha'),
                    ({b'CD01_STDIN_LEASE_MANIFEST': manifest_path,
                      b'CD01_STDIN_LEASE_MANIFEST_SHA256': b'abcd'}, 'short-sha'),
                    ({b'CD01_STDIN_LEASE_MANIFEST': manifest_path,
                      b'CD01_STDIN_LEASE_MANIFEST_SHA256': hashlib.sha256(b'unrelated').hexdigest().encode('ascii')}, 'digest-mismatch')]:
                with self.subTest(label=label):
                    directory = self._capture_dir(root)
                    result, stderr, execute = self._run(directory, env_extra=env_extra)
                    self.assertEqual(result, 78)
                    execute.assert_not_called()
                    self.assertEqual(list(directory.iterdir()), [])
            for injection, expected, label in [
                    (OSError(errno.EIO, 'READ_PRIVATE_TEXT'), 1, 'read-io'),
                    (KeyboardInterrupt('PRIVATE_INTERRUPT'), 130, 'read-interrupt')]:
                with self.subTest(label=label):
                    directory = self._capture_dir(root)
                    saved1, _ = self._fake_fd0(root, b'unread fault bytes')
                    try:
                        with mock.patch.object(wrapper.os, 'read', side_effect=injection):
                            result, stderr, execute = self._run(directory, env_extra={
                                b'CD01_STDIN_LEASE_MANIFEST': manifest_path,
                                b'CD01_STDIN_LEASE_MANIFEST_SHA256': manifest_sha})
                        self.assertEqual(result, expected)
                        execute.assert_not_called()
                        self.assertEqual(list(directory.iterdir()), [])
                        self.assertEqual(os.lseek(0, 0, os.SEEK_CUR), 0)
                        self.assertNotIn('PRIVATE', stderr.getvalue())
                        if expected == 1:
                            self.assertIn('errno=' + str(errno.EIO), stderr.getvalue())
                        else:
                            self.assertIn('interrupted', stderr.getvalue())
                    finally:
                        os.dup2(saved1, 0)
                        os.close(saved1)
            # Fresh recovery: the same pair without injection selects again.
            recovery_dir = self._capture_dir(root)
            saved2, _ = self._fake_fd0(root, b'recovery bytes')
            try:
                result, stderr, execute = self._run(recovery_dir, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': manifest_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': manifest_sha})
                self.assertEqual(result, 1)
                self.assertEqual(execute.call_count, 1)
                self.assertIn('stdin-transfer.json', [p.name for p in self._invocation(recovery_dir).iterdir()])
            finally:
                os.dup2(saved2, 0)
                os.close(saved2)

    def test_malformed_manifest_and_privacy_refusals(self):
        """Healthy control, malformed/duplicate/schema/private/symlink refusals, recovery."""
        with self._retained_dir() as name:
            root = Path(name)
            good_path, good_sha = self._manifest(root, self._document(self._entry()))
            # Healthy same-precondition control: the exact pair selects normally.
            control_dir = self._capture_dir(root)
            saved0, _ = self._fake_fd0(root, b'control bytes')
            try:
                result, stderr, execute = self._run(control_dir, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': good_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': good_sha})
                self.assertEqual(result, 1)
                self.assertEqual(execute.call_count, 1)
                self.assertIn('stdin-transfer.json', [p.name for p in self._invocation(control_dir).iterdir()])
            finally:
                os.dup2(saved0, 0)
                os.close(saved0)
            wrong_schema = self._document(self._entry())
            wrong_schema['schema'] = 'cd01.stdin_lease_manifest/2'
            wrong_kind = self._document(self._entry())
            wrong_kind['record_kind'] = 'not_the_operator_kind'
            top_extra = self._document(self._entry())
            top_extra['operator'] = 'unused'
            entry_extra = self._document(self._entry())
            entry_extra['leases'][0]['producer'] = 'unused'
            duplicate_raw = (b'{"schema": "cd01.stdin_lease_manifest/1", '
                             b'"schema": "cd01.stdin_lease_manifest/1", '
                             b'"record_kind": "operator_stdin_lease_manifest", "leases": []}')
            cases = [
                (None, b'{not json', 78, 'malformed-json'),
                (None, duplicate_raw, 78, 'duplicate-members'),
                (wrong_schema, None, 78, 'wrong-schema'),
                (wrong_kind, None, 78, 'wrong-kind'),
                (top_extra, None, 78, 'top-extra-member'),
                (entry_extra, None, 78, 'entry-extra-member'),
            ]
            for document, raw, expected, label in cases:
                with self.subTest(label=label):
                    manifest_path, manifest_sha = self._manifest(root, document, raw=raw)
                    directory = self._capture_dir(root)
                    result, stderr, execute = self._run(directory, env_extra={
                        b'CD01_STDIN_LEASE_MANIFEST': manifest_path,
                        b'CD01_STDIN_LEASE_MANIFEST_SHA256': manifest_sha})
                    self.assertEqual(result, expected)
                    execute.assert_not_called()
                    self.assertEqual(list(directory.iterdir()), [])
            private_root = root / ('private-' + os.urandom(8).hex())
            private_root.mkdir(mode=0o700)
            private_path, private_sha = self._manifest(private_root, self._document(self._entry()), mode=0o644)
            with self.subTest(label='private-mode'):
                directory = self._capture_dir(root)
                result, stderr, execute = self._run(directory, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': private_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': private_sha})
                self.assertEqual(result, 78)
                execute.assert_not_called()
                self.assertEqual(list(directory.iterdir()), [])
            link = private_root / 'link.json'
            os.symlink(os.fsdecode(good_path), str(link))
            with self.subTest(label='nofollow-symlink'):
                directory = self._capture_dir(root)
                result, stderr, execute = self._run(directory, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': os.fsencode(str(link)),
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': good_sha})
                self.assertEqual(result, 1)
                execute.assert_not_called()
                self.assertIn('errno=' + str(errno.ELOOP), stderr.getvalue())
                self.assertEqual(list(directory.iterdir()), [])
            # Fresh recovery with the good manifest on a fresh directory.
            recovery_dir = self._capture_dir(root)
            saved1, _ = self._fake_fd0(root, b'recovery bytes')
            try:
                result, stderr, execute = self._run(recovery_dir, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': good_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': good_sha})
                self.assertEqual(result, 1)
                self.assertEqual(execute.call_count, 1)
                self.assertIn('stdin-transfer.json', [p.name for p in self._invocation(recovery_dir).iterdir()])
            finally:
                os.dup2(saved1, 0)
                os.close(saved1)

    def test_entry_types_duplicates_and_selector_mismatch(self):
        """Control, strict-type/duplicate refusals, then no-match and empty-list v1."""
        with self._retained_dir() as name:
            root = Path(name)
            good_path, good_sha = self._manifest(root, self._document(self._entry()))
            # Healthy same-precondition control: the exact pair selects normally.
            control_dir = self._capture_dir(root)
            saved0, _ = self._fake_fd0(root, b'control bytes')
            try:
                result, stderr, execute = self._run(control_dir, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': good_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': good_sha})
                self.assertEqual(result, 1)
                self.assertEqual(execute.call_count, 1)
                self.assertIn('stdin-transfer.json', [p.name for p in self._invocation(control_dir).iterdir()])
            finally:
                os.dup2(saved0, 0)
                os.close(saved0)
            uppercase_selector = {
                'compiler_path_hex': os.fsencode('/owned/compiler').hex().upper(),
                'compiler_arguments_hex': [os.fsencode('ordinary').hex()],
                'cwd_hex': os.getcwdb().hex(),
            }
            cases = [
                ('bool-input-fd', self._document(self._entry(input_fd=True))),
                ('one-input-fd', self._document(self._entry(input_fd=1))),
                ('integer-declaration', self._document(self._entry(declarations={
                    'finite_cooperative_single_writer_input': 1,
                    'producer_will_close_input_fd': True,
                    'persistence_authorized': True}))),
                ('wrong-repeat-policy', self._document(self._entry(repeat_policy='once'))),
                ('duplicate-allocation-id', self._document(self._entry(), self._entry())),
                ('duplicate-selector', self._document(self._entry(), self._entry(allocation_id='alloc-twin'))),
                ('uppercase-selector-hex', self._document(self._entry(selector=uppercase_selector))),
            ]
            for label, document in cases:
                with self.subTest(label=label):
                    manifest_path, manifest_sha = self._manifest(root, document)
                    directory = self._capture_dir(root)
                    result, stderr, execute = self._run(directory, env_extra={
                        b'CD01_STDIN_LEASE_MANIFEST': manifest_path,
                        b'CD01_STDIN_LEASE_MANIFEST_SHA256': manifest_sha})
                    self.assertEqual(result, 78)
                    execute.assert_not_called()
                    self.assertEqual(list(directory.iterdir()), [])
            # A valid manifest whose selector never matches keeps exact v1, no fd 0 read.
            saved1, _ = self._fake_fd0(root, b'must-stay-unread')
            try:
                mismatch_path, mismatch_sha = self._manifest(root, self._document(self._entry(compiler='/other/compiler')))
                directory = self._capture_dir(root)
                result, stderr, execute = self._run(directory, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': mismatch_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': mismatch_sha})
                self.assertEqual(result, 1)
                self.assertEqual(execute.call_count, 1)
                self.assertEqual(os.lseek(0, 0, os.SEEK_CUR), 0)
                directory = self._invocation(directory)
                expected = wrapper._render_intent_record(
                    os.fsencode('/owned/compiler'), [os.fsencode('ordinary')], b'/usr/bin/true')
                self.assertEqual((directory / 'intent.json').read_bytes(), expected)
                self.assertEqual(json.loads(expected)['schema'], 'cd01.rustc_intent.v1')
                self.assertNotIn('stdin-custody', [p.name for p in directory.iterdir()])
                # An empty leases list is valid and also selects nothing.
                empty_path, empty_sha = self._manifest(root, self._document())
                empty_dir = self._capture_dir(root)
                result, stderr, execute = self._run(empty_dir, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': empty_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': empty_sha})
                self.assertEqual(result, 1)
                self.assertEqual(os.lseek(0, 0, os.SEEK_CUR), 0)
                self.assertEqual(json.loads((self._invocation(empty_dir) / 'intent.json').read_bytes())['schema'], 'cd01.rustc_intent.v1')
            finally:
                os.dup2(saved1, 0)
                os.close(saved1)
            # Fresh recovery with the good manifest on a fresh directory.
            recovery_dir = self._capture_dir(root)
            saved2, _ = self._fake_fd0(root, b'recovery bytes')
            try:
                result, stderr, execute = self._run(recovery_dir, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': good_path,
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': good_sha})
                self.assertEqual(result, 1)
                self.assertEqual(execute.call_count, 1)
                self.assertIn('stdin-transfer.json', [p.name for p in self._invocation(recovery_dir).iterdir()])
            finally:
                os.dup2(saved2, 0)
                os.close(saved2)

    def test_selected_repeatable_flow_real_helper_distinct_receipts(self):
        """Manifest-selected transfer via the real helper, repeated with exclusive receipts."""
        with self._retained_dir() as name:
            root = Path(name)
            manifest_path, manifest_sha = self._manifest(root, self._document(self._entry()))
            seen = []
            for payload in [b'first \x00\xff raw bytes', b'second \x80 nonUTF8 payload']:
                saved0 = os.dup(0)
                fixture_path = root / ('input-' + os.urandom(8).hex() + '.bin')
                fixture_path.write_bytes(payload)
                fixture_fd = os.open(str(fixture_path), os.O_RDONLY)
                self.addCleanup(os.close, fixture_fd)
                try:
                    os.dup2(fixture_fd, 0)
                    capture_dir = self._capture_dir(root)
                    reads = []
                    stderr = io.StringIO()
                    env = {
                        b'CD01_CAPTURE_DIR': os.fsencode(str(capture_dir)),
                        b'CD01_SPACEGUARD_PATH': b'/usr/bin/true',
                        b'CD01_STDIN_LEASE_MANIFEST': manifest_path,
                        b'CD01_STDIN_LEASE_MANIFEST_SHA256': manifest_sha,
                    }
                    def fake_exec(executable, delegation_argv):
                        """Read the transferred descriptor in-process, then fail ENOSYS."""
                        reads.append(os.read(0, 65536))
                        reads.append(os.read(0, 65536))
                        raise OSError(errno.ENOSYS, 'EXEC_PRIVATE_TEXT')
                    with mock.patch.dict(os.environb, env, clear=True), mock.patch.object(wrapper.sys, 'stderr', stderr), mock.patch.object(wrapper.os, 'execv', side_effect=fake_exec) as execute:
                        result = wrapper.main(['wrapper', '/owned/compiler', 'ordinary'])
                    self.assertNotIn('PRIVATE_TEXT', stderr.getvalue())
                    self.assertEqual(result, 1)
                    self.assertEqual(execute.call_count, 1)
                    self.assertEqual(reads, [payload, b''])
                    directory = self._invocation(capture_dir)
                    custody = directory / 'stdin-custody'
                    self.assertEqual((custody / 'input.bin').read_bytes(), payload)
                    manifest = json.loads((custody / 'manifest.json').read_bytes())
                    self.assertTrue(manifest['eof_observed'])
                    self.assertEqual(manifest['byte_count'], len(payload))
                    self.assertEqual(manifest['sha256'], hashlib.sha256(payload).hexdigest())
                    receipt = json.loads((directory / 'stdin-transfer.json').read_bytes())
                    self.assertEqual(receipt['allocation_id'], 'alloc-packet23')
                    self.assertEqual(receipt['observed_custody_manifest']['sha256'], manifest['sha256'])
                    self.assertEqual(receipt['delegation']['outcome'], 'UNKNOWN')
                    intent = json.loads((directory / 'intent.json').read_bytes())
                    self.assertEqual(intent['schema'], 'cd01.rustc_intent.v2')
                    self.assertEqual(intent['stdin_transfer']['status'], 'PENDING')
                    seen.append(str(directory))
                finally:
                    os.dup2(saved0, 0)
                    os.close(saved0)
            # Repeated exact invocations selected independently into distinct directories.
            self.assertEqual(len(seen), 2)
            self.assertNotEqual(seen[0], seen[1])


    def test_manifest_encoding_is_explicit_utf8(self):
        """UTF8 control, foreign byte encodings refused, then fresh UTF8 recovery."""
        document = self._document(self._entry())
        text = json.dumps(document)
        self.assertEqual(wrapper._parse_manifest_document(text.encode('utf8')), document)
        for encoding in ('utf16', 'utf32'):
            with self.subTest(encoding=encoding), self.assertRaises(wrapper.Refusal):
                wrapper._parse_manifest_document(text.encode(encoding))
        self.assertEqual(wrapper._parse_manifest_document(text.encode('utf8')), document)

    def test_manifest_cleanup_interrupt_keeps_primary_and_terminal_category(self):
        """Read IO plus cleanup interruption retain both causes and terminal130."""
        with self._retained_dir() as name:
            root = Path(name)
            path, sha = self._manifest(root, self._document())
            env = {b'CD01_STDIN_LEASE_MANIFEST': path,
                   b'CD01_STDIN_LEASE_MANIFEST_SHA256': sha}
            # Same no-match manifest is readable and preserves v1 before injection.
            result, _, execute = self._run(self._capture_dir(root), env_extra=env)
            self.assertEqual(result, 1)
            self.assertEqual(execute.call_count, 1)
            directory = self._capture_dir(root)
            close = os.close
            def close_then_interrupt(descriptor):
                """Close the real owned manifest descriptor, then inject interruption."""
                close(descriptor)
                raise KeyboardInterrupt('CLEANUP_PRIVATE_TEXT')
            primary = OSError(errno.EIO, 'PRIMARY_PRIVATE_TEXT')
            with mock.patch.object(wrapper.os, 'read', side_effect=primary), \
                 mock.patch.object(wrapper.os, 'close', side_effect=close_then_interrupt):
                result, stderr, execute = self._run(directory, env_extra=env)
            self.assertEqual(result, 130)
            execute.assert_not_called()
            self.assertEqual(list(directory.iterdir()), [])
            self.assertIn('errno=' + str(errno.EIO), stderr.getvalue())
            self.assertIn('KeyboardInterrupt', stderr.getvalue())
            self.assertIn('interrupted', stderr.getvalue())
            self.assertNotIn('PRIVATE', stderr.getvalue())
            recovery, _, execute = self._run(self._capture_dir(root), env_extra=env)
            self.assertEqual(recovery, 1)
            self.assertEqual(execute.call_count, 1)


    def test_manifest_fifo_refuses_without_blocking_open(self):
        """Regular control, checked nonblocking FIFO open/type refusal, recovery."""
        with self._retained_dir() as name:
            root = Path(name)
            path, sha = self._manifest(root, self._document())
            env = {b'CD01_STDIN_LEASE_MANIFEST': path,
                   b'CD01_STDIN_LEASE_MANIFEST_SHA256': sha}
            control, _, execute = self._run(self._capture_dir(root), env_extra=env)
            self.assertEqual(control, 1)
            self.assertEqual(execute.call_count, 1)
            fifo = root / 'manifest.fifo'
            os.mkfifo(fifo, 0o600)
            opened = os.open
            attempts = []
            def require_nonblocking_then_open(file, flags, *args, **kwargs):
                """Assert the transport flag before opening FIFO; never risk a blocking read."""
                if os.fsencode(file) == os.fsencode(fifo):
                    attempts.append(flags)
                    self.assertTrue(flags & os.O_NONBLOCK)
                return opened(file, flags, *args, **kwargs)
            directory = self._capture_dir(root)
            with mock.patch.object(wrapper.os, 'open', side_effect=require_nonblocking_then_open):
                result, stderr, execute = self._run(directory, env_extra={
                    b'CD01_STDIN_LEASE_MANIFEST': os.fsencode(fifo),
                    b'CD01_STDIN_LEASE_MANIFEST_SHA256': sha})
            self.assertEqual(len(attempts), 1)
            self.assertEqual(result, 78)
            self.assertIn('regular file', stderr.getvalue())
            execute.assert_not_called()
            self.assertEqual(list(directory.iterdir()), [])
            recovery, _, execute = self._run(self._capture_dir(root), env_extra=env)
            self.assertEqual(recovery, 1)
            self.assertEqual(execute.call_count, 1)

if __name__ == '__main__':
    unittest.main()
