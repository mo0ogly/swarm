#!/usr/bin/env python3
"""Five proposed inert method guards: no Rust/build/real child/provider.

Future authorized invocation loads pinned controller/common producer sources and
fully replaces subprocess.run. Fake channels/test text are INERT data, never Rust
execution proof. Healthy/fault/fresh recovery use the same caller and real owned
files. This source was not imported/executed during preparation.
"""
import argparse
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


class CustodyGuards(unittest.TestCase):
    """Exercise only invocation/phase/final-renderer custody with inert children."""
    config = None

    def exercise(self, fault='healthy'):
        """Make one fresh owned trial, retain actual typed failure/metadata and
        no fallback subprocess. All fault injections are fixture-only, not kernel
        close failures, real unreadable compiler graphs or Rust test outcomes.
        """
        module, parent = self.config['module'], self.config['parent']
        trial = Path(tempfile.mkdtemp(prefix='inert-cd01-' + fault + '-', dir=parent))
        os.chmod(trial, 0o700)
        root = trial / 'root'; root.mkdir(mode=0o700)
        output = trial / 'out'; output.mkdir(mode=0o700)
        source = trial / 'source.input'; source.write_bytes(b'INERT_STABLE_SOURCE')
        binary = trial / 'never-execute'; binary.write_bytes(b'INERT_IMAGE_NOT_EXECUTABLE')
        record = {'compiler_inputs': {str(source): module.sha(source)}, 'binary': str(binary),
                  'binary_sha256': module.sha(binary), '_invocations': []}
        state = {'calls': 0, 'completed': False, 'post_source_reads': 0, 'post_image_reads': 0,
                 'sync': [], 'stderr_producer_closes': 0, 'receipt_visible_refusal': False}
        stdout = ('running 1 test\ntest ' + module.PAIRED + ' ... ok\n'
                  'test result: ok. 1 passed; 0 failed; 0 ignored\n'
                  'CD01_PAIR_PROBE_COMPLETE fixed presence\n').encode()
        stderr = b'INERT_CHILD_CHANNEL_NOT_A_RUST_EXECUTION\n'
        real_read, real_open, real_sync, real_publish = Path.read_bytes, Path.open, os.fsync, module.write_new

        def fake_run(argv, **kwargs):
            """Produce one completed inert process, never invoke any executable."""
            self.assertEqual(argv, [str(binary), module.PAIRED, '--exact', '--ignored', '--nocapture', '--test-threads=1'])
            state['calls'] += 1
            self.assertEqual(state['calls'], 1, 'no unchanged child retry')
            self.assertEqual(kwargs['stdin'], subprocess.DEVNULL)
            kwargs['stdout'].write(stdout); kwargs['stderr'].write(stderr)
            raw = b'INERT_BAD_JSON' if fault == 'parse-unreadable' else json.dumps({'schema_version': 1, 'case': 'presence', 'variant': 'fixed', 'scope': 'INERT_GUARD_ONLY'}).encode()
            (root / 'probe.json').write_bytes(raw)
            state['completed'] = True
            if fault == 'nonzero-changed': source.write_bytes(b'INERT_CHANGED_SOURCE')
            code = 7 if fault == 'nonzero-changed' else -15 if fault == 'receipt-close' else 0
            state['known_returncode'] = code
            return subprocess.CompletedProcess(argv, code)

        def read(path):
            """Inject a post-completion input read refusal, not preflight failure."""
            if state['completed'] and path == source:
                state['post_source_reads'] += 1
                if fault == 'parse-unreadable': raise PermissionError(13, 'INERT_POST_INPUT_UNREADABLE')
            if state['completed'] and path == binary: state['post_image_reads'] += 1
            return real_read(path)

        class ReleasedClose:
            """Raise only after real release; retain synthetic status honestly."""
            def __init__(self, stream):
                """Own the real inert producer stream until the one close."""
                self.stream = stream
            def __getattr__(self, key):
                """Delegate normal FileIO methods without replacing byte writes."""
                return getattr(self.stream, key)
            def close(self):
                """Release the owned descriptor once, then inject a diagnostic."""
                self.stream.close(); state['stderr_producer_closes'] += 1
                raise OSError(5, 'INERT_RELEASED_STDERR_CLOSE')

        def opened(path, *args, **kwargs):
            """Wrap only the common producer's unbuffered stderr close seam."""
            stream = real_open(path, *args, **kwargs)
            mode = args[0] if args else kwargs.get('mode', 'r')
            return ReleasedClose(stream) if fault == 'receipt-close' and path == output / 'probe.stderr' and mode == 'xb' else stream

        def synced(fd):
            """Fail stdout sync after known completion; stderr is still observed."""
            if state['completed']:
                actual = os.fstat(fd)
                for name in ('probe.stdout', 'probe.stderr'):
                    path = output / name
                    if path.exists() and (actual.st_dev, actual.st_ino) == (path.stat().st_dev, path.stat().st_ino):
                        state['sync'].append(name)
                        if fault == 'sync' and name == 'probe.stdout': raise OSError(5, 'INERT_STDOUT_SYNC')
            return real_sync(fd)

        def published(path, value):
            """Simulate visible/uncertain receipt failure, never pretend rollback."""
            real_publish(path, value)
            if fault == 'receipt-close' and path == output / 'probe-result.json':
                state['receipt_visible_refusal'] = True
                raise PermissionError(13, 'INERT_RECEIPT_VISIBLE_THEN_REFUSED')
            if fault == 'late-publication' and path == output / 'late.json':
                source.write_bytes(b'INERT_LATE_SOURCE_DRIFT')
                raise OSError(5, 'INERT_LATE_PUBLICATION')

        error = None
        with contextlib.ExitStack() as stack:
            stack.enter_context(patch.object(subprocess, 'run', side_effect=fake_run))
            stack.enter_context(patch.object(Path, 'read_bytes', read))
            stack.enter_context(patch.object(Path, 'open', opened))
            stack.enter_context(patch.object(os, 'fsync', side_effect=synced))
            stack.enter_context(patch.object(module, 'write_new', published))
            try:
                if fault == 'late-publication':
                    custody = {'phase': 'inert-controller-publication', 'invocations': record['_invocations']}
                    def work():
                        """Call actual invocation once, then inject a later publication."""
                        module.invoke(record, binary, module.PAIRED, root, 'INERT_TOKEN', output, 'fixed', 'presence')
                        module.write_new(output / 'late.json', {'kind': 'INERT_ONLY'})
                    module.checked_phase(record, work, custody, 'after-inert-late-publication')
                else:
                    module.invoke(record, binary, module.PAIRED, root, 'INERT_TOKEN', output, 'fixed', 'presence')
            except BaseException as caught: error = caught
        # Same final consumer as main, after restoring the fault seams.
        document = module.failure_document(error) if error else None
        if document is not None: module.write_new(output / 'failure.json', document)
        observation = {'scope': 'INERT_METHOD_GUARD_ONLY', 'fault': fault, 'state': state, 'failure': document, 'source_path': str(source),
                       'stdout_hex': real_read(output / 'probe.stdout').hex(), 'stderr_hex': real_read(output / 'probe.stderr').hex(),
                       'real_child_executed': False, 'rust_executed': False, 'product_qualified': False}
        module.write_new(trial / 'guard-observation.json', observation)
        return observation

    def healthy(self):
        """Require actual caller success with inert bytes before/after each fault."""
        observed = self.exercise()
        self.assertIsNone(observed['failure']); self.assertEqual(observed['state']['calls'], 1)
        self.assertEqual(observed['state']['post_source_reads'], 2)
        self.assertEqual(observed['state']['post_image_reads'], 2)

    def fault(self, name):
        """Retain healthy/fault/recovery; require known typed exit, attempted
        postchecks and grouped original causes. Nothing here runs a Rust probe.
        """
        self.healthy(); observed = self.exercise(name); self.healthy()
        self.assertEqual(observed['state']['calls'], 1); self.assertIsNotNone(observed['failure'])
        doc = observed['failure']; encoded = json.dumps(doc)
        self.assertFalse(doc['behavioral_red_established_by_failure'])
        frames = doc['cd01_custody_frames']
        self.assertTrue(frames)
        self.assertGreaterEqual(observed['state']['post_source_reads'], 2)
        self.assertGreaterEqual(observed['state']['post_image_reads'], 2)
        # Inspect real metadata values, never search diagnostic text for an exit.
        def processes(value):
            """Traverse JSON evidence containers for observed subprocess states."""
            if isinstance(value, dict):
                if isinstance(value.get('subprocess'), dict): yield value['subprocess']
                for child in value.values(): yield from processes(child)
            elif isinstance(value, list):
                for child in value: yield from processes(child)
        outcomes_seen = list(processes(frames))
        self.assertTrue(outcomes_seen)
        for item in outcomes_seen:
            self.assertIs(type(item['returncode']), int)
            self.assertEqual(item['returncode'], observed['state']['known_returncode'])
        self.assertEqual(observed['stdout_hex'], ('running 1 test\ntest ' + self.config['module'].PAIRED + ' ... ok\n'
                  'test result: ok. 1 passed; 0 failed; 0 ignored\nCD01_PAIR_PROBE_COMPLETE fixed presence\n').encode().hex())
        self.assertEqual(observed['stderr_hex'], b'INERT_CHILD_CHANNEL_NOT_A_RUST_EXECUTION\n'.hex())
        return observed, encoded

    def has_cause(self, document, kind, message=None):
        """Match actual described exception nodes, never an arbitrary diagnostic
        string/metadata field that happens to mention a type name.
        """
        def nodes(value):
            """Walk only the common producer's real group/cause/context schema."""
            if not isinstance(value, dict): return
            yield value
            for child in value.get('exceptions', []): yield from nodes(child)
            for key in ('cause', 'context'):
                if isinstance(value.get(key), dict): yield from nodes(value[key])
        return any(node.get('type') == kind and (message is None or message in node.get('message', ''))
                   for node in nodes(document['causes']))

    def child_post(self, document):
        """Select the real child validator's final attempted input observation."""
        for frame in document['cd01_custody_frames']:
            custody = frame.get('cd01_custody')
            if isinstance(custody, dict):
                post = custody.get('post_validation_inputs')
                if isinstance(post, dict) and post.get('phase') == 'after-child-and-report-validation': return post
        self.fail('child post-validation observation is missing')

    def test_nonzero_and_changed_input_keep_both_causes(self):
        """Known7 plus post-source drift, one inert child and no PASS."""
        observed, text = self.fault('nonzero-changed')
        self.assertTrue(self.has_cause(observed['failure'], 'ValueError', 'pair-child-nonzero'))
        self.assertTrue(self.has_cause(observed['failure'], 'ValueError', 'pair-compiler-input-changed'))

    def test_parser_and_unreadable_input_keep_unknown_read_state(self):
        """Malformed report plus independent post-input permission error."""
        observed, text = self.fault('parse-unreadable')
        self.assertTrue(self.has_cause(observed['failure'], 'JSONDecodeError'))
        self.assertTrue(self.has_cause(observed['failure'], 'PermissionError', 'INERT_POST_INPUT_UNREADABLE'))
        post = self.child_post(observed['failure'])
        self.assertIs(post['attempted'], True); self.assertIs(post['ok'], False)
        self.assertIsNone(post['facts']['sources'][observed['source_path']]['sha256'])
        self.assertIs(post['facts']['image']['attempted'], True); self.assertIs(post['facts']['image']['ok'], True)

    def test_first_sync_failure_does_not_skip_second_channel(self):
        """First channel sync refusal keeps known0, second sync/hash and postcheck."""
        observed, text = self.fault('sync')
        self.assertTrue(self.has_cause(observed['failure'], 'OSError', 'INERT_STDOUT_SYNC'))
        self.assertIn('probe.stderr', observed['state']['sync'])
        process = observed['failure']['native_outcome']['subprocess']
        self.assertEqual(process['stderr_sha256'], hashlib.sha256(b'INERT_CHILD_CHANNEL_NOT_A_RUST_EXECUTION\n').hexdigest())

    def test_visible_receipt_and_released_close_keep_signal_and_causes(self):
        """Known -15/signal15 survives visible receipt refusal and close error."""
        observed, text = self.fault('receipt-close')
        self.assertTrue(observed['state']['receipt_visible_refusal'])
        self.assertEqual(observed['state']['stderr_producer_closes'], 1)
        self.assertTrue(self.has_cause(observed['failure'], 'PermissionError', 'INERT_RECEIPT_VISIBLE_THEN_REFUSED'))
        self.assertTrue(self.has_cause(observed['failure'], 'OSError', 'INERT_RELEASED_STDERR_CLOSE'))
        self.assertEqual(observed['failure']['native_outcome']['subprocess']['signal'], 15)
        self.assertIs(observed['failure']['native_outcome']['receipt_publication']['ok'], False)

    def test_late_publication_still_attempts_terminal_source_observation(self):
        """A later controller publication refusal retains child0 and new drift."""
        observed, text = self.fault('late-publication')
        self.assertEqual(observed['state']['post_source_reads'], 3)
        self.assertTrue(self.has_cause(observed['failure'], 'OSError', 'INERT_LATE_PUBLICATION'))
        self.assertTrue(self.has_cause(observed['failure'], 'ValueError', 'pair-compiler-input-changed'))


def main():
    """Future allocated guard-only execution; source pins, fresh evidence, five
    named methods, no real subprocess fallback. No command runs in preparation.
    """
    if sys.flags.optimize: raise RuntimeError('inert guard normal Python required')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--candidate-sha256', required=True)
    parser.add_argument('--outcome-source', type=Path, required=True)
    parser.add_argument('--outcome-sha256', required=True)
    parser.add_argument('--evidence-parent', type=Path, required=True)
    args = parser.parse_args()
    for path, pin in ((args.candidate, args.candidate_sha256), (args.outcome_source, args.outcome_sha256)):
        if hashlib.sha256(path.read_bytes()).hexdigest() != pin: raise ValueError('guard source pin mismatch')
    parent = args.evidence_parent.resolve(strict=True)
    output = Path(tempfile.mkdtemp(prefix='inert-cd01-custody-', dir=parent)); os.chmod(output, 0o700)
    producer_spec = importlib.util.spec_from_file_location('native_build_outcome', args.outcome_source)
    producer = importlib.util.module_from_spec(producer_spec); producer_spec.loader.exec_module(producer)
    spec = importlib.util.spec_from_file_location('inert_cd01_pair', args.candidate)
    module = importlib.util.module_from_spec(spec)
    with patch.dict(sys.modules, {'native_build_outcome': producer}): spec.loader.exec_module(module)
    CustodyGuards.config = {'module': module, 'parent': output}
    source_map = {str(path.resolve()): pin for path, pin in ((args.candidate, args.candidate_sha256), (args.outcome_source, args.outcome_sha256))}
    source_map[str(Path(__file__).resolve())] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    interpreter = Path(sys.executable).resolve(strict=True)
    envelope = {'compiler_inputs': source_map, 'binary': str(interpreter), 'binary_sha256': module.sha(interpreter)}
    custody = {'scope': 'INERT_SUITE_METHOD_INPUTS_NOT_COMPILER_GRAPH', 'result_known': False}

    def suite_work():
        """Capture returned unittest counts before fallible stream sync/close;
        retain every IO cause and let the shared phase perform independent pins.
        """
        errors, result, stream = [], None, None
        try:
            stream = (output / 'unittest.stderr').open('x')
            result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(CustodyGuards))
            custody.update(result_known=True, tests_run=result.testsRun, failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped))
        except BaseException as error: errors.append(error)
        finally:
            if stream is not None:
                for action in (stream.flush, lambda: os.fsync(stream.fileno()), stream.close):
                    try: action()
                    except BaseException as error: errors.append(error)
        if errors: raise module.joined_failure('Inert suite and channel custody refused', errors, custody)
        return result

    try:
        result = module.checked_phase(envelope, suite_work, custody, 'after-inert-suite-and-output-custody')
        stable = custody['post_validation_inputs']['ok'] is True
        ok = result.wasSuccessful() and result.testsRun == 5 and not result.skipped and stable
        module.write_new(output / 'summary.json', {'scope': 'INERT_METHOD_GUARD_ONLY', 'tests_run': result.testsRun,
            'failures': len(result.failures), 'errors': len(result.errors), 'skipped': len(result.skipped), 'source_stable': stable,
            'source_postcheck': custody['post_validation_inputs'], 'real_children': 0, 'rust_executed': False, 'status': 'PASS' if ok else 'FAIL'})
    except BaseException as error:
        module.write_new(output / 'suite-failure.json', module.failure_document(error))
        print('INERT_CD01_CUSTODY_EVIDENCE ' + str(output))
        print('INERT_CD01_CUSTODY_FAIL')
        return 2
    print('INERT_CD01_CUSTODY_EVIDENCE ' + str(output))
    print('INERT_CD01_CUSTODY_PASS five-named-methods zero-real-children' if ok else 'INERT_CD01_CUSTODY_FAIL')
    return 0 if ok else 1


if __name__ == '__main__': raise SystemExit(main())
