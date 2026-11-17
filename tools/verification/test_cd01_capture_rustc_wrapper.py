"""Focused CD01 wrapper diagnostic-custody regression; no children/compiler."""
import errno
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

if __name__ == '__main__':
    unittest.main()
