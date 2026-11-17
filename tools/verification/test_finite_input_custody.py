"""Unit regressions for the finite-input custody helper.

Standalone unittest module:

    python3 -B tools/verification/test_finite_input_custody.py -v

Every test builds fresh fixtures under the caller-selected TMPDIR
(tempfile follows the runner's environment; the tests never modify it).
Each injected-fault test runs a normal control capture BEFORE the fault
and a fresh recovery capture AFTER it, so no failure is meaningful
without a passing control in the same process.  Producers and receivers
here are in-process test doubles; the owned inert receiver double is
explicitly not a production consumer, and the helper itself claims
nothing about any receiver.  There is no historical replay of past
captures, no watchdog or timeout harness, and no compiler or subprocess.
These tests qualify only the helper's own contract; they do not qualify
any build, wrapper, installation or acceptance.
"""

import errno
import fcntl
import hashlib
import json
import os
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import finite_input_custody

PAYLOAD = (b'finite-input-custody payload '
           + bytes([0x00, 0x01, 0xFF, 0xFE])
           + b' not-utf8 '
           + bytes([0x80])
           + b' tail')
CONTROL_PAYLOAD = b'control payload for finite input custody'
SENTINEL = b'ORIGINAL-SENTINEL-BYTES'


def _inert_receiver_read(fd, chunk_size=4096):
    """Read a descriptor to EOF and return the bytes (test double).

    This is the test's owned inert in-process receiver: it only reads and
    returns bytes.  It is a test double, not a production consumer, and
    the helper makes no claim that any receiver ever runs.
    """
    chunks = []
    while True:
        block = os.read(fd, chunk_size)
        if block == b'':
            return b''.join(chunks)
        chunks.append(block)


class FiniteInputCustodyTests(unittest.TestCase):
    """Regression suite; see the module docstring for the fixture policy."""

    maxDiff = None

    def setUp(self):
        """Create caller-owned retained fixtures and register strict descriptor cleanup."""
        self.base_dir = tempfile.mkdtemp(prefix='finite-input-custody-tests-')
        # Caller-retained fixtures preserve fault partials and original observations.
        self._input_fds = []
        self.addCleanup(self._close_borrowed_inputs)

    # -- fixture helpers -------------------------------------------------

    @staticmethod
    def _close_quietly(fd):
        """Close one caller-owned descriptor; report cleanup errors."""
        os.close(fd)

    def _close_borrowed_inputs(self):
        """Close every input descriptor the test borrowed as its caller."""
        for fd in self._input_fds:
            self._close_quietly(fd)
        self._input_fds = []

    def _fresh_output_dir(self):
        """Create one exclusive fresh private output directory (mode 0700)."""
        path = tempfile.mkdtemp(prefix='out-', dir=self.base_dir)
        os.chmod(path, 0o700)
        return path

    def _fresh_input_fd(self, payload):
        """Create a fresh regular-file fixture, returned open O_RDONLY.

        The test plays the caller: the descriptor it hands to capture is
        owned by the test and closed again in cleanup, never by the
        helper.
        """
        path = os.path.join(
            self.base_dir, 'input-fixture-%d.bin' % len(self._input_fds))
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        view = memoryview(payload)
        while view:
            view = view[os.write(fd, view):]
        os.close(fd)
        fd = os.open(path, os.O_RDONLY)
        self._input_fds.append(fd)
        return fd

    def _run_control(self, payload, allocation_id='alloc-control'):
        """Run one clean capture and assert the whole success contract.

        Used as the pre-fault control and as the fresh post-fault
        recovery.  The returned descriptor is consumed by the test's
        inert receiver double and closed here; nothing outside
        output_dir is created.  Returns (input_fd, output_dir).
        """
        input_fd = self._fresh_input_fd(payload)
        output_dir = self._fresh_output_dir()
        parent_before = sorted(os.listdir(self.base_dir))
        returned_fd, manifest = finite_input_custody.capture(
            input_fd, output_dir, allocation_id)
        try:
            self.assertEqual(
                manifest['schema'], finite_input_custody.MANIFEST_SCHEMA)
            self.assertEqual(manifest['allocation_id'], allocation_id)
            self.assertTrue(manifest['eof_observed'])
            self.assertTrue(
                manifest['staged_copy_complete_of_observed_stream'])
            self.assertEqual(manifest['byte_count'], len(payload))
            self.assertEqual(
                manifest['sha256'], hashlib.sha256(payload).hexdigest())
            self.assertEqual(
                manifest['not_claimed']['receiver_consumption'], 'unknown')
            self.assertEqual(os.lseek(returned_fd, 0, os.SEEK_CUR), 0)
            flags = fcntl.fcntl(returned_fd, fcntl.F_GETFL)
            self.assertEqual(flags & os.O_ACCMODE, os.O_RDONLY)
            self.assertEqual(_inert_receiver_read(returned_fd), payload)
            raw_path = os.path.join(output_dir, 'input.bin')
            with open(raw_path, 'rb') as handle:
                self.assertEqual(handle.read(), payload)
            manifest_path = os.path.join(output_dir, 'manifest.json')
            with open(manifest_path, 'rb') as handle:
                self.assertEqual(
                    json.loads(handle.read().decode('utf-8')), manifest)
            st_staging = os.stat(os.path.join(output_dir, 'input.partial'))
            st_final = os.stat(raw_path)
            self.assertEqual(
                (st_staging.st_dev, st_staging.st_ino),
                (st_final.st_dev, st_final.st_ino))
            st_manifest_staging = os.stat(
                os.path.join(output_dir, 'manifest.partial'))
            st_manifest_final = os.stat(manifest_path)
            self.assertEqual(
                (st_manifest_staging.st_dev, st_manifest_staging.st_ino),
                (st_manifest_final.st_dev, st_manifest_final.st_ino))
            self.assertEqual(
                sorted(os.listdir(output_dir)),
                ['input.bin', 'input.partial', 'manifest.json',
                 'manifest.partial'])
            os.fstat(input_fd)  # borrowed descriptor still open and usable
            self.assertEqual(sorted(os.listdir(self.base_dir)), parent_before)
        finally:
            os.close(returned_fd)
        return input_fd, output_dir

    # -- success contract ------------------------------------------------

    def test_control_success_full_contract(self):
        """Control: one clean capture satisfies the whole success contract.

        Files, hardlinks, manifest round-trip, read-only offset-zero
        returned descriptor, digest and count over staged bytes only,
        borrowed input descriptor untouched, and nothing created outside
        output_dir.  The receiver here is the test's own inert double.
        """
        input_fd, output_dir = self._run_control(PAYLOAD)
        os.fstat(input_fd)

    def test_empty_stream_stages_zero_bytes_and_eof_manifest(self):
        """An empty but complete stream stages zero bytes and a real manifest.

        An empty complete stream differs from an absent or incomplete one:
        input.bin exists with size zero and manifest.json exists with
        eof_observed true and the SHA-256 of the empty byte string.
        """
        input_fd, output_dir = self._run_control(b'')
        self.assertEqual(
            os.stat(os.path.join(output_dir, 'input.bin')).st_size, 0)
        with open(os.path.join(output_dir, 'manifest.json'), 'rb') as handle:
            published = json.loads(handle.read().decode('utf-8'))
        self.assertTrue(published['eof_observed'])
        self.assertEqual(published['byte_count'], 0)
        self.assertEqual(published['sha256'], hashlib.sha256(b'').hexdigest())

    def test_non_utf8_bytes_staged_verbatim(self):
        """Non-UTF-8 bytes are staged verbatim; JSON stays separate.

        Raw staging never decodes or normalises; the manifest file is its
        own valid UTF-8 JSON document and does not embed the raw bytes.
        """
        payload = (bytes([0x00, 0xFF, 0xFE, 0x80, 0x81])
                   + b'binary-not-utf8-'
                   + bytes([0xFE, 0xFF, 0x00]))
        input_fd, output_dir = self._run_control(payload)
        with open(os.path.join(output_dir, 'input.bin'), 'rb') as handle:
            self.assertEqual(handle.read(), payload)  # verbatim, no decoding
        with open(os.path.join(output_dir, 'manifest.json'), 'rb') as handle:
            raw_manifest = handle.read()
        raw_manifest.decode('utf-8')  # manifest is valid UTF-8 JSON on its own
        self.assertNotIn(payload, raw_manifest)

    # -- benign fragmentation --------------------------------------------

    def test_fragmented_reads_stage_identical_bytes(self):
        """Many tiny reads still stage exactly the observed stream.

        Control first; then the input read is wrapped to return at most
        three bytes per call, exercising the fragmented-read path; a
        fresh recovery closes the test.  Digest and count still describe
        the staged bytes only.
        """
        self._run_control(CONTROL_PAYLOAD)
        input_fd = self._fresh_input_fd(PAYLOAD)
        output_dir = self._fresh_output_dir()
        real_read = os.read
        state = {'calls': 0}

        def tiny_read(fd, size):
            """Inject three-byte fragmentation while preserving source order and EOF."""
            state['calls'] += 1
            return real_read(fd, min(size, 3))

        with mock.patch.object(finite_input_custody, '_os_read', tiny_read):
            returned_fd, manifest = finite_input_custody.capture(
                input_fd, output_dir, 'alloc-fragmented')
        try:
            self.assertGreater(state['calls'], len(PAYLOAD) // 3)
            self.assertEqual(manifest['byte_count'], len(PAYLOAD))
            self.assertEqual(
                manifest['sha256'], hashlib.sha256(PAYLOAD).hexdigest())
            self.assertEqual(_inert_receiver_read(returned_fd), PAYLOAD)
        finally:
            os.close(returned_fd)
        self._run_control(b'fresh recovery after fragmented reads')

    def test_short_writes_loop_to_completion(self):
        """Short writes are looped until the whole chunk is staged.

        Control first; then every staging and manifest write is wrapped
        to write at most five bytes per call; a fresh recovery closes the
        test.  The staged file is byte-identical to the input.
        """
        self._run_control(CONTROL_PAYLOAD)
        input_fd = self._fresh_input_fd(PAYLOAD)
        output_dir = self._fresh_output_dir()
        real_write = os.write
        state = {'calls': 0}

        def five_byte_write(fd, data):
            """Inject successful five-byte writes without dropping the remainder."""
            state['calls'] += 1
            return real_write(fd, data[:5])

        with mock.patch.object(
                finite_input_custody, '_os_write', five_byte_write):
            returned_fd, manifest = finite_input_custody.capture(
                input_fd, output_dir, 'alloc-short-write')
        try:
            self.assertGreaterEqual(state['calls'], len(PAYLOAD) // 5)
            self.assertEqual(
                manifest['sha256'], hashlib.sha256(PAYLOAD).hexdigest())
        finally:
            os.close(returned_fd)
        with open(os.path.join(output_dir, 'input.bin'), 'rb') as handle:
            self.assertEqual(handle.read(), PAYLOAD)
        self._run_control(b'fresh recovery after short writes')

    # -- injected faults --------------------------------------------------

    def test_zero_progress_write_aborts_and_retains_partial(self):
        """A write that stops making progress aborts and retains the partial.

        Control first; then the first staging write stores four bytes and
        every later write returns zero.  The failure is reported at stage
        write-staging with a zero-progress cause and no errno, the four
        staged bytes are retained, nothing is published or deleted, and
        no ready descriptor ever existed.  A fresh recovery closes the
        test.
        """
        self._run_control(CONTROL_PAYLOAD)
        payload = b'ZERO-PROGRESS-PAYLOAD'
        input_fd = self._fresh_input_fd(payload)
        output_dir = self._fresh_output_dir()
        real_write = os.write
        state = {'calls': 0}

        def zero_after_first_write(fd, data):
            """Persist four bytes then inject zero progress; retain the partial."""
            state['calls'] += 1
            if state['calls'] == 1:
                return real_write(fd, data[:4])
            return 0

        with mock.patch.object(
                finite_input_custody, '_os_write', zero_after_first_write):
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, output_dir, 'alloc-zero-progress')
        error = caught.exception
        self.assertEqual(str(error), 'finite input custody failed')
        self.assertEqual(error.stage, 'write-staging')
        self.assertEqual(error.primary['op'], 'write')
        self.assertEqual(error.primary['name'], 'input.partial')
        self.assertEqual(error.primary['reason'], 'zero-progress')
        self.assertIsNone(error.primary['errno'])
        self.assertEqual(error.secondary, [])
        self.assertFalse(error.ready_fd_close_attempted)
        with open(os.path.join(output_dir, 'input.partial'), 'rb') as handle:
            self.assertEqual(handle.read(), payload[:4])
        self.assertFalse(os.path.exists(os.path.join(output_dir, 'input.bin')))
        self.assertFalse(
            os.path.exists(os.path.join(output_dir, 'manifest.json')))
        os.fstat(input_fd)  # borrowed descriptor untouched
        self._run_control(b'fresh recovery after zero-progress write')

    def test_read_fault_retains_partial_and_borrowed_descriptor(self):
        """A read failure mid-stream keeps the primary cause and partial bytes.

        Control first; then the first read returns four bytes and the
        next raises EIO.  The failure is reported at stage read-input
        with the original errno, input.partial retains exactly the staged
        prefix, nothing is published or deleted, and the borrowed input
        descriptor is still open.  A fresh recovery closes the test.
        """
        self._run_control(CONTROL_PAYLOAD)
        payload = b'READ-FAULT-PAYLOAD'
        input_fd = self._fresh_input_fd(payload)
        output_dir = self._fresh_output_dir()
        real_read = os.read
        state = {'calls': 0}

        def read_then_fault(fd, size):
            """Read one prefix then raise the injected input error."""
            state['calls'] += 1
            if state['calls'] == 1:
                return real_read(fd, 4)
            raise OSError(errno.EIO, 'injected read fault')

        with mock.patch.object(
                finite_input_custody, '_os_read', read_then_fault):
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, output_dir, 'alloc-read-fault')
        error = caught.exception
        self.assertEqual(str(error), 'finite input custody failed')
        self.assertEqual(error.stage, 'read-input')
        self.assertEqual(error.primary['op'], 'read')
        self.assertEqual(error.primary['name'], 'input-fd')
        self.assertEqual(error.primary['errno'], errno.EIO)
        self.assertFalse(error.ready_fd_close_attempted)
        with open(os.path.join(output_dir, 'input.partial'), 'rb') as handle:
            self.assertEqual(handle.read(), payload[:4])
        self.assertFalse(os.path.exists(os.path.join(output_dir, 'input.bin')))
        self.assertFalse(
            os.path.exists(os.path.join(output_dir, 'manifest.json')))
        os.fstat(input_fd)
        self._run_control(b'fresh recovery after read fault')

    def test_fsync_fault_retains_staging_without_publication(self):
        """An fsync failure aborts with the original errno and keeps staging.

        Control first; the injected EIO hits the staging fsync, so the
        fully written input.partial is retained, nothing is published,
        nothing is deleted and no ready descriptor existed; a fresh
        recovery closes the test.
        """
        self._run_control(CONTROL_PAYLOAD)
        payload = b'FSYNC-FAULT-PAYLOAD'
        input_fd = self._fresh_input_fd(payload)
        output_dir = self._fresh_output_dir()

        def broken_fsync(fd):
            """Inject an fsync error before publication."""
            raise OSError(errno.EIO, 'injected fsync fault')

        with mock.patch.object(finite_input_custody, '_os_fsync', broken_fsync):
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, output_dir, 'alloc-fsync-fault')
        error = caught.exception
        self.assertEqual(str(error), 'finite input custody failed')
        self.assertEqual(error.stage, 'fsync-staging')
        self.assertEqual(error.primary['op'], 'fsync')
        self.assertEqual(error.primary['name'], 'input.partial')
        self.assertEqual(error.primary['errno'], errno.EIO)
        self.assertEqual(error.secondary, [])
        self.assertFalse(error.ready_fd_close_attempted)
        with open(os.path.join(output_dir, 'input.partial'), 'rb') as handle:
            self.assertEqual(handle.read(), payload)
        self.assertFalse(os.path.exists(os.path.join(output_dir, 'input.bin')))
        self.assertFalse(
            os.path.exists(os.path.join(output_dir, 'manifest.json')))
        self._run_control(b'fresh recovery after fsync fault')

    def test_close_fault_attempted_once_with_primary_cause(self):
        """A failing writer close is the primary cause and is never retried.

        Control first; then the first close the helper performs (the
        staging writer) raises EIO exactly once.  The helper must not
        attempt a second close of that descriptor, so exactly one further
        close (the output directory, on the error path) is observed.  A
        fresh recovery closes the test.
        """
        self._run_control(CONTROL_PAYLOAD)
        payload = b'CLOSE-FAULT-PAYLOAD'
        input_fd = self._fresh_input_fd(payload)
        output_dir = self._fresh_output_dir()
        real_close = os.close
        state = {'calls': 0}

        def close_fault_on_first(fd):
            """Close the writer then report an uncertain close result once."""
            state['calls'] += 1
            if state['calls'] == 1:
                real_close(fd)
                raise OSError(errno.EIO, 'injected close fault')
            return real_close(fd)

        with mock.patch.object(
                finite_input_custody, '_os_close', close_fault_on_first):
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, output_dir, 'alloc-close-fault')
        error = caught.exception
        self.assertEqual(str(error), 'finite input custody failed')
        self.assertEqual(error.stage, 'close-staging')
        self.assertEqual(error.primary['op'], 'close')
        self.assertEqual(error.primary['name'], 'input.partial')
        self.assertEqual(error.primary['errno'], errno.EIO)
        self.assertEqual(error.secondary, [])
        self.assertEqual(state['calls'], 2)  # one failed attempt, one dir close
        self.assertFalse(error.ready_fd_close_attempted)
        with open(os.path.join(output_dir, 'input.partial'), 'rb') as handle:
            self.assertEqual(handle.read(), payload)
        self.assertFalse(os.path.exists(os.path.join(output_dir, 'input.bin')))
        self.assertFalse(
            os.path.exists(os.path.join(output_dir, 'manifest.json')))
        self._run_control(b'fresh recovery after close fault')

    def test_link_fault_eexist_keeps_collision_cause_and_partial(self):
        """A link-time collision is reported with the original errno.

        Control first; then the input.bin hardlink itself fails with
        EEXIST, as a concurrent creator winning after validation would
        cause.  The staged input.partial is retained, nothing is deleted,
        no manifest is published and no ready descriptor existed; a fresh
        recovery closes the test.
        """
        self._run_control(CONTROL_PAYLOAD)
        payload = b'LINK-FAULT-PAYLOAD'
        input_fd = self._fresh_input_fd(payload)
        output_dir = self._fresh_output_dir()

        def colliding_link(src, dst):
            """Inject a no-clobber link collision, retaining the staged original."""
            raise OSError(errno.EEXIST, 'injected collision at link time')

        with mock.patch.object(
                finite_input_custody, '_os_link', colliding_link):
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, output_dir, 'alloc-link-fault')
        error = caught.exception
        self.assertEqual(str(error), 'finite input custody failed')
        self.assertEqual(error.stage, 'link-input')
        self.assertEqual(error.primary['op'], 'link')
        self.assertEqual(error.primary['name'], 'input.bin')
        self.assertEqual(error.primary['errno'], errno.EEXIST)
        self.assertEqual(error.secondary, [])
        self.assertFalse(error.ready_fd_close_attempted)
        with open(os.path.join(output_dir, 'input.partial'), 'rb') as handle:
            self.assertEqual(handle.read(), payload)
        self.assertFalse(os.path.exists(os.path.join(output_dir, 'input.bin')))
        self.assertFalse(
            os.path.exists(os.path.join(output_dir, 'manifest.json')))
        self._run_control(b'fresh recovery after link fault')

    def test_existing_input_bin_collision_rejected_before_consuming(self):
        """A pre-existing input.bin or manifest.json is refused, never touched.

        Control first; each subtest pre-creates one custody name and
        asserts a typed validate-stage refusal with the original sentinel
        bytes unchanged, no staging file created and the borrowed input
        descriptor still open.  The no-clobber hardlink remains the
        atomic backstop behind this pre-check.  A fresh recovery closes
        the test.
        """
        self._run_control(CONTROL_PAYLOAD)
        with self.subTest('existing-input-bin'):
            input_fd = self._fresh_input_fd(b'collision-payload')
            output_dir = self._fresh_output_dir()
            sentinel = os.path.join(output_dir, 'input.bin')
            with open(sentinel, 'wb') as handle:
                handle.write(SENTINEL)
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, output_dir, 'alloc-collision')
            error = caught.exception
            self.assertEqual(error.stage, 'validate')
            self.assertEqual(error.primary['reason'], 'target-exists')
            self.assertEqual(error.primary['name'], 'input.bin')
            with open(sentinel, 'rb') as handle:
                self.assertEqual(handle.read(), SENTINEL)  # original untouched
            self.assertEqual(sorted(os.listdir(output_dir)), ['input.bin'])
            os.fstat(input_fd)
        with self.subTest('existing-manifest-json'):
            input_fd = self._fresh_input_fd(b'collision-payload-2')
            output_dir = self._fresh_output_dir()
            sentinel = os.path.join(output_dir, 'manifest.json')
            with open(sentinel, 'wb') as handle:
                handle.write(b'{"original": true}')
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, output_dir, 'alloc-collision-2')
            error = caught.exception
            self.assertEqual(error.stage, 'validate')
            self.assertEqual(error.primary['reason'], 'target-exists')
            self.assertEqual(error.primary['name'], 'manifest.json')
            with open(sentinel, 'rb') as handle:
                self.assertEqual(handle.read(), b'{"original": true}')
            self.assertEqual(
                sorted(os.listdir(output_dir)), ['manifest.json'])
            os.fstat(input_fd)
        self._run_control(b'fresh recovery after collision refusal')

    def test_manifest_write_fault_publishes_no_manifest_json(self):
        """A manifest write failure leaves no manifest.json behind.

        Control first; then the second write the helper performs (the
        manifest staging write) raises EIO.  input.bin is already fully
        staged and a ready descriptor existed, but the failure closes it,
        publishes nothing and retains the empty manifest.partial; a
        fresh recovery closes the test.
        """
        self._run_control(CONTROL_PAYLOAD)
        payload = b'MANIFEST-WRITE-FAULT-PAYLOAD'
        input_fd = self._fresh_input_fd(payload)
        output_dir = self._fresh_output_dir()
        real_write = os.write
        state = {'calls': 0}

        def write_fault_on_manifest(fd, data):
            """Write raw bytes normally then inject manifest write failure."""
            state['calls'] += 1
            if state['calls'] == 1:
                return real_write(fd, data)
            raise OSError(errno.EIO, 'injected manifest write fault')

        with mock.patch.object(
                finite_input_custody, '_os_write', write_fault_on_manifest):
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, output_dir, 'alloc-manifest-write-fault')
        error = caught.exception
        self.assertEqual(str(error), 'finite input custody failed')
        self.assertEqual(error.stage, 'write-manifest')
        self.assertEqual(error.primary['op'], 'write')
        self.assertEqual(error.primary['name'], 'manifest.partial')
        self.assertEqual(error.primary['errno'], errno.EIO)
        self.assertTrue(error.ready_fd_close_attempted)
        with open(os.path.join(output_dir, 'input.bin'), 'rb') as handle:
            self.assertEqual(handle.read(), payload)
        self.assertEqual(
            os.stat(os.path.join(output_dir, 'manifest.partial')).st_size, 0)
        self.assertFalse(
            os.path.exists(os.path.join(output_dir, 'manifest.json')))
        self._run_control(b'fresh recovery after manifest write fault')

    def test_manifest_link_fault_closes_ready_descriptor(self):
        """A manifest publication failure closes the ready descriptor.

        Control first; then the first hardlink (input.bin) succeeds and
        the second (manifest.json) raises EIO.  manifest.json stays
        absent, the staging names are retained, and the already reopened
        input.bin descriptor is closed exactly once by the failure path;
        a fresh recovery closes the test.
        """
        self._run_control(CONTROL_PAYLOAD)
        payload = b'MANIFEST-LINK-FAULT-PAYLOAD'
        input_fd = self._fresh_input_fd(payload)
        output_dir = self._fresh_output_dir()
        real_link = os.link
        state = {'calls': 0}

        def link_fault_on_manifest(src, dst):
            """Publish raw data normally then fail manifest publication."""
            state['calls'] += 1
            if state['calls'] == 1:
                return real_link(src, dst)
            raise OSError(errno.EIO, 'injected manifest link fault')

        with mock.patch.object(
                finite_input_custody, '_os_link', link_fault_on_manifest):
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, output_dir, 'alloc-manifest-link-fault')
        error = caught.exception
        self.assertEqual(str(error), 'finite input custody failed')
        self.assertEqual(error.stage, 'link-manifest')
        self.assertEqual(error.primary['op'], 'link')
        self.assertEqual(error.primary['name'], 'manifest.json')
        self.assertEqual(error.primary['errno'], errno.EIO)
        self.assertTrue(error.ready_fd_close_attempted)
        self.assertEqual(state['calls'], 2)
        self.assertTrue(os.path.exists(os.path.join(output_dir, 'input.bin')))
        self.assertTrue(
            os.path.exists(os.path.join(output_dir, 'manifest.partial')))
        self.assertFalse(
            os.path.exists(os.path.join(output_dir, 'manifest.json')))
        self._run_control(b'fresh recovery after manifest link fault')

    def test_reopen_fault_reported_with_retained_staged_files(self):
        """A failing reopen of input.bin is typed and stages nothing more.

        Control first; then the third open the helper performs (the
        read-only reopen of input.bin) raises EIO.  The staged files are
        retained, no manifest staging exists yet, and no ready descriptor
        was ever opened; a fresh recovery closes the test.
        """
        self._run_control(CONTROL_PAYLOAD)
        payload = b'REOPEN-FAULT-PAYLOAD'
        input_fd = self._fresh_input_fd(payload)
        output_dir = self._fresh_output_dir()
        real_open = os.open
        state = {'calls': 0}

        def open_fault_on_reopen(path, flags, mode=None):
            """Open directory/writer normally then reject the ready-file reopen."""
            state['calls'] += 1
            if state['calls'] == 3:
                raise OSError(errno.EIO, 'injected reopen fault')
            if mode is None:
                return real_open(path, flags)
            return real_open(path, flags, mode)

        with mock.patch.object(
                finite_input_custody, '_os_open', open_fault_on_reopen):
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, output_dir, 'alloc-reopen-fault')
        error = caught.exception
        self.assertEqual(str(error), 'finite input custody failed')
        self.assertEqual(error.stage, 'reopen-input')
        self.assertEqual(error.primary['op'], 'open')
        self.assertEqual(error.primary['name'], 'input.bin')
        self.assertEqual(error.primary['errno'], errno.EIO)
        self.assertFalse(error.ready_fd_close_attempted)
        with open(os.path.join(output_dir, 'input.bin'), 'rb') as handle:
            self.assertEqual(handle.read(), payload)
        self.assertTrue(
            os.path.exists(os.path.join(output_dir, 'input.partial')))
        self.assertFalse(
            os.path.exists(os.path.join(output_dir, 'manifest.partial')))
        self.assertFalse(
            os.path.exists(os.path.join(output_dir, 'manifest.json')))
        self._run_control(b'fresh recovery after reopen fault')

    def test_close_dir_fault_prevents_success_after_publication(self):
        """A failing directory close prevents the successful return.

        Control first; then the third close (the output directory, after
        manifest publication) raises EIO.  manifest.json is already
        published, but capture must still fail: publication is not
        success, no descriptor is returned, and the ready input.bin
        descriptor is closed exactly once by the failure path.  A fresh
        recovery closes the test.
        """
        self._run_control(CONTROL_PAYLOAD)
        payload = b'CLOSE-DIR-FAULT-PAYLOAD'
        input_fd = self._fresh_input_fd(payload)
        output_dir = self._fresh_output_dir()
        real_close = os.close
        state = {'calls': 0}

        def close_fault_on_dir(fd):
            """Close the directory then report its uncertain close outcome."""
            state['calls'] += 1
            if state['calls'] == 3:
                real_close(fd)
                raise OSError(errno.EIO, 'injected directory close fault')
            return real_close(fd)

        with mock.patch.object(
                finite_input_custody, '_os_close', close_fault_on_dir):
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, output_dir, 'alloc-close-dir-fault')
        error = caught.exception
        self.assertEqual(str(error), 'finite input custody failed')
        self.assertEqual(error.stage, 'close-dir')
        self.assertEqual(error.primary['op'], 'close')
        self.assertEqual(error.primary['name'], 'output-dir')
        self.assertEqual(error.primary['errno'], errno.EIO)
        self.assertTrue(error.ready_fd_close_attempted)
        self.assertEqual(state['calls'], 4)
        with open(os.path.join(output_dir, 'manifest.json'), 'rb') as handle:
            published = json.loads(handle.read().decode('utf-8'))
        self.assertEqual(published['byte_count'], len(payload))
        self.assertTrue(os.path.exists(os.path.join(output_dir, 'input.bin')))
        self._run_control(b'fresh recovery after directory close fault')

    # -- validation and error shape ---------------------------------------

    def test_validation_rejects_before_consuming_input(self):
        """Type, path and privacy validation fails before any input is read.

        Control first and fresh recovery last.  Each subtest asserts a
        typed validate-stage refusal and that the rejected directory
        stays empty (no file created, no byte consumed) while the
        borrowed input descriptor remains open and usable.
        """
        self._run_control(CONTROL_PAYLOAD)
        input_fd = self._fresh_input_fd(b'validation-payload')
        with self.subTest('permissive-directory-mode'):
            target = self._fresh_output_dir()
            os.chmod(target, 0o750)
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, target, 'alloc-validation')
            error = caught.exception
            self.assertEqual(error.stage, 'validate')
            self.assertEqual(error.primary['reason'], 'not-private-mode')
            self.assertEqual(os.listdir(target), [])
        with self.subTest('empty-allocation-id'):
            target = self._fresh_output_dir()
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(input_fd, target, '')
            self.assertEqual(
                caught.exception.primary['reason'], 'not-nonempty-string')
            self.assertEqual(os.listdir(target), [])
        with self.subTest('non-integer-input-fd'):
            target = self._fresh_output_dir()
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    'not-a-fd', target, 'alloc-validation')
            self.assertEqual(caught.exception.primary['reason'], 'not-an-int')
            self.assertEqual(os.listdir(target), [])
        with self.subTest('boolean-input-fd-is-not-an-int'):
            target = self._fresh_output_dir()
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(True, target, 'alloc-validation')
            self.assertEqual(caught.exception.primary['reason'], 'not-an-int')
            self.assertEqual(os.listdir(target), [])
        with self.subTest('missing-output-directory'):
            missing = os.path.join(self.base_dir, 'absent-output-dir')
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, missing, 'alloc-validation')
            error = caught.exception
            self.assertEqual(error.stage, 'validate')
            self.assertEqual(error.primary['op'], 'lstat')
            self.assertEqual(error.primary['errno'], errno.ENOENT)
        os.fstat(input_fd)  # still open and usable after every refusal
        self._run_control(b'fresh recovery after validation refusals')

    def test_error_message_static_and_state_fields_bounded(self):
        """Custody errors keep one static message and bounded state fields.

        Uses the zero-progress injection; asserts the exception text is
        the fixed sentence, that primary keys are exactly the bounded
        set, that every value is a small token or None, and that no raw
        payload bytes leak into the state fields.  A fresh recovery
        closes the test.
        """
        self._run_control(CONTROL_PAYLOAD)
        payload = b'STATE-FIELDS-PAYLOAD'
        input_fd = self._fresh_input_fd(payload)
        output_dir = self._fresh_output_dir()
        real_write = os.write
        state = {'calls': 0}

        def zero_after_first(fd, data):
            """Persist a prefix then stop write progress for diagnostic assertions."""
            state['calls'] += 1
            if state['calls'] == 1:
                return real_write(fd, data[:2])
            return 0

        with mock.patch.object(
                finite_input_custody, '_os_write', zero_after_first):
            with self.assertRaises(
                    finite_input_custody.FiniteInputCustodyError) as caught:
                finite_input_custody.capture(
                    input_fd, output_dir, 'alloc-state-fields')
        error = caught.exception
        self.assertEqual(str(error), 'finite input custody failed')
        self.assertEqual(
            sorted(error.primary), ['detail', 'errno', 'name', 'op', 'reason'])
        self.assertEqual(error.primary['op'], 'write')
        self.assertEqual(error.primary['name'], 'input.partial')
        self.assertIsNone(error.primary['errno'])
        self.assertIsNone(error.primary['detail'])
        self.assertEqual(error.primary['reason'], 'zero-progress')
        self.assertEqual(error.secondary, [])
        self.assertIs(error.ready_fd_close_attempted, False)
        self.assertEqual(error.output_dir, output_dir)
        for value in (error.stage, error.primary['op'],
                      error.primary['name'], error.primary['reason']):
            self.assertIsInstance(value, str)
            self.assertLessEqual(len(value), 40)
        self.assertNotIn(payload, repr(error.primary).encode('utf-8'))
        self._run_control(b'fresh recovery after state-field check')

    # -- real pipe end to end ---------------------------------------------

    def test_real_pipe_cooperative_producer_inert_receiver_digest(self):
        """End to end over a real os.pipe with an in-process producer.

        The cooperative producer writes a small payload and closes its
        end before capture is called (the pre-read/spool ordering of this
        helper); capture then observes EOF on the read end.  The test's
        owned inert receiver double reads the returned descriptor and
        verifies the bytes and digest; this is a test double, not a
        production consumer, and the helper itself claims nothing about
        any receiver.  No compiler and no subprocess are involved.
        """
        payload = b'pipe-payload'  # Below POSIX PIPE_BUF minimum; cooperative prewrite.
        read_fd, write_fd = os.pipe()
        self.addCleanup(self._close_quietly, read_fd)
        view = memoryview(payload)
        while view:
            view = view[os.write(write_fd, view):]
        os.close(write_fd)  # cooperative producer finished and closed
        output_dir = self._fresh_output_dir()
        returned_fd, manifest = finite_input_custody.capture(
            read_fd, output_dir, 'alloc-pipe')
        try:
            self.assertTrue(manifest['eof_observed'])
            self.assertEqual(manifest['byte_count'], len(payload))
            self.assertEqual(
                manifest['sha256'], hashlib.sha256(payload).hexdigest())
            # The manifest was published before any consumption and stays
            # honest about it: receiver consumption is never claimed.
            self.assertEqual(
                manifest['not_claimed']['receiver_consumption'], 'unknown')
            received = _inert_receiver_read(returned_fd)
            self.assertEqual(received, payload)
        finally:
            os.close(returned_fd)
        # The borrowed read end was never closed or replaced by the
        # helper; the caller closes it itself, here at EOF.
        self.assertEqual(os.read(read_fd, 4096), b'')
        # read_fd is closed once by its registered caller cleanup.


    def test_interrupt_preserves_primary_and_cleanup_cause(self):
        """Healthy control, injected interrupt plus close error, fresh recovery."""
        self._run_control(CONTROL_PAYLOAD)
        source = self._fresh_input_fd(b'owned-interrupt-input')
        out = self._fresh_output_dir()
        primary = KeyboardInterrupt('PRIVATE_INTERRUPT')
        secondary = OSError(errno.EIO, 'PRIVATE_CLOSE')
        real_close = os.close
        failed = []
        def close_then_fault(fd):
            """Actually close the first held descriptor, then retain a cleanup cause."""
            real_close(fd)
            if not failed:
                failed.append(fd)
                raise secondary
        caught = None
        try:
            with mock.patch.object(finite_input_custody, '_os_read', side_effect=primary), mock.patch.object(finite_input_custody, '_os_close', close_then_fault):
                try:
                    finite_input_custody.capture(source, out, 'allocated-interrupt')
                except BaseException as error:
                    caught = error
        finally:
            self._run_control(b'fresh-recovery-after-interrupt')
        self.assertIsInstance(caught, finite_input_custody.FiniteInputCustodyError)
        self.assertIs(caught.original_exception, primary)
        self.assertIn(secondary, caught.secondary_exceptions)
        self.assertNotIn('PRIVATE', str(caught))
        self.assertTrue(os.path.isfile(os.path.join(out, 'input.partial')))
        self.assertFalse(os.path.exists(os.path.join(out, 'manifest.json')))
        os.fstat(source)

    def test_lookup_error_refuses_before_consumption(self):
        """Healthy control, denied custody-name lookup, fresh recovery."""
        self._run_control(CONTROL_PAYLOAD)
        source = self._fresh_input_fd(b'owned-lookup-input')
        out = self._fresh_output_dir()
        real_stat = finite_input_custody._os_lstat
        primary = OSError(errno.EACCES, 'PRIVATE_LOOKUP')
        def denied(path):
            """Reject the selected custody-name lookup before input consumption."""
            if os.fspath(path) == os.path.join(out, 'input.partial'):
                raise primary
            return real_stat(path)
        caught = None
        returned = None
        try:
            with mock.patch.object(finite_input_custody, '_os_lstat', denied):
                try:
                    returned = finite_input_custody.capture(source, out, 'allocated-lookup')
                except BaseException as error:
                    caught = error
        finally:
            if returned is not None:
                os.close(returned[0])
            self._run_control(b'fresh-recovery-after-lookup')
        self.assertIsInstance(caught, finite_input_custody.FiniteInputCustodyError)
        self.assertEqual(caught.primary['errno'], errno.EACCES)
        self.assertIs(caught.original_exception, primary)
        self.assertEqual(os.lseek(source, 0, os.SEEK_CUR), 0)
        self.assertEqual(os.listdir(out), [])


if __name__ == '__main__':
    unittest.main()
