#!/usr/bin/env python3
"""Five SOURCE-ONLY proposed retained-terminal guards, with no real children.

Healthy owned sealed fixtures precede each independent mutation, followed by
fresh recovery. This file is not imported/run during preparation. Fake admitted
source/image files prove only verifier self-consistency, never a Rust/compiler
profile, execution authority, actual controller completion or independent review.
"""
import argparse
import contextlib
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


class TerminalGuards(unittest.TestCase):
    """Use actual seal/publication/readback functions on owned inert files."""
    config = None

    def fixture(self):
        """Build a new valid sealed fixture with distinct source/image pins.

        This is an inert evidence schema fixture, not a copied Rust source root.
        No child, native probe, mount check or Cargo invocation is performed.
        """
        module = self.config['module']
        trial = Path(tempfile.mkdtemp(prefix='inert-cd01-terminal-', dir=self.config['parent']))
        os.chmod(trial, 0o700)
        folder = trial / 'evidence'; folder.mkdir(mode=0o700)
        files = {}
        for name in ('source', 'image', 'record', 'controller', 'producer'):
            files[name] = trial / name
            files[name].write_bytes(('INERT_DISTINCT_' + name).encode())
        record = {'compiler_inputs': {str(files['source']): module.sha(files['source'])},
                  'binary': str(files['image']), 'binary_sha256': module.sha(files['image'])}
        for key, name in (('_controller_record', 'record'), ('_controller_source', 'controller'), ('_outcome_source', 'producer')):
            record[key] = {'path': str(files[name]), 'sha256': module.sha(files[name])}
        custody = {'phase': 'controller-work-and-publication', 'invocations': []}
        def sealed_work():
            """Create the actual inner evidence before late-receipt publication."""
            module.write_new(folder / 'admitted-record-copy.json', record)
            module.write_new(folder / 'storage-readback.json', {'scope': 'INERT_NOT_MOUNT_ADMISSION'})
            module.write_new(folder / 'result.json', {'status': 'RECORDED_TARGETS_MET', 'scope': 'paired',
                'record_sha256': record['_controller_record']['sha256'], 'binary_sha256': record['binary_sha256'],
                'limits': 'INERT_NO_RUST_NO_CONTROLLER_EXIT_ASSERTION'})
            module.seal_evidence(folder)
        module.checked_phase(record, sealed_work, custody, 'after-controller-and-evidence-publication')
        module.publish_terminal_receipt(record, folder, custody)
        module.verify_retained(folder)
        self.assertFalse((folder / 'failure.json').exists())
        return folder

    def rebind(self, folder):
        """Update only the owned mutation envelope to test semantic validation.

        Explicit fixture forgery is not authenticity. Unbound-change case never
        uses this seam; production publication always uses fresh exclusive writes.
        """
        module = self.config['module']
        path = folder / 'terminal-integrity.json'
        envelope = json.loads(path.read_bytes())
        envelope['bindings']['controller-postcheck.json'] = module.sha(folder / 'controller-postcheck.json')
        path.write_text(json.dumps(envelope))

    def refused(self, mutation, expected, reason=None):
        """Healthy before mutation, expected typed refusal, fresh recovery.

        Patch all subprocess.run calls to fail closed even for a broken verifier;
        assert none was attempted. Never run a probe to recover missing proof.
        """
        module = self.config['module']
        with patch.object(subprocess, 'run', side_effect=AssertionError('NO_CHILD_ALLOWED')) as child:
            folder = self.fixture()
            mutation(folder)
            with self.assertRaises(expected) as caught:
                module.verify_retained(folder)
            if reason is not None: self.assertIn(reason, str(caught.exception))
            self.fixture()
            self.assertEqual(child.call_count, 0)

    def test_empty_terminal_receipt_refuses(self):
        """Even digest-rebound empty receipt must fail actual JSON consumption."""
        def mutate(folder):
            """Replace only the owned receipt with empty bytes, rebind fixture."""
            (folder / 'controller-postcheck.json').write_bytes(b'')
            self.rebind(folder)
        self.refused(mutate, json.JSONDecodeError)

    def test_malformed_terminal_receipt_refuses(self):
        """Malformed JSON refuses despite correctly bound new fixture bytes."""
        def mutate(folder):
            """Retain invalid JSON under its own digest to reach the parser."""
            (folder / 'controller-postcheck.json').write_bytes(b'{INERT_BAD_JSON')
            self.rebind(folder)
        self.refused(mutate, json.JSONDecodeError)

    def test_failed_or_incomplete_postcheck_refuses(self):
        """Independently mutate phase/facts/image/source or envelope schema.

        Rebinding fixture bytes deliberately distinguishes semantic rejection from
        a mere stale-hash rejection. Each variant gets a fresh valid control.
        """
        for variant in ('failed', 'phase', 'missing-source', 'wrong-source', 'wrong-image', 'unknown-image', 'publication-failed', 'schema'):
            with self.subTest(variant=variant):
                def mutate(folder):
                    """Apply one isolated metadata mutation, preserving other facts."""
                    path = folder / 'controller-postcheck.json'
                    receipt = json.loads(path.read_bytes())
                    post = receipt['post_validation_inputs']; facts = post['facts']
                    if variant == 'failed': post['ok'] = False
                    elif variant == 'phase': post['phase'] = 'INERT_WRONG_PHASE'
                    elif variant == 'missing-source': facts['sources'].pop(next(iter(facts['sources'])))
                    elif variant == 'wrong-source': facts['sources'][next(iter(facts['sources']))]['sha256'] = '0' * 64
                    elif variant == 'wrong-image': facts['image']['sha256'] = '0' * 64
                    elif variant == 'unknown-image': facts['image']['sha256'] = None
                    path.write_text(json.dumps(receipt)); self.rebind(folder)
                    ep = folder / 'terminal-integrity.json'; envelope = json.loads(ep.read_bytes())
                    if variant == 'publication-failed': envelope['publication_postcheck']['ok'] = False
                    elif variant == 'schema': envelope['schema_version'] = True
                    ep.write_text(json.dumps(envelope))
                self.refused(mutate, ValueError)

    def test_changed_terminal_bytes_refuse(self):
        """Valid JSON changed after terminal binding must fail exact byte hash."""
        def mutate(folder):
            """Add harmless JSON whitespace without resealing the terminal bytes."""
            path = folder / 'controller-postcheck.json'
            path.write_bytes(path.read_bytes() + b'\n ')
        self.refused(mutate, ValueError, 'pair-terminal-bytes-changed')

    def test_unreadable_terminal_receipt_preserves_cause(self):
        """Synthetic exact-read EACCES after healthy fixture; no privilege guess."""
        module = self.config['module']; real_read = Path.read_bytes
        with patch.object(subprocess, 'run', side_effect=AssertionError('NO_CHILD_ALLOWED')) as child:
            folder = self.fixture(); target = folder / 'controller-postcheck.json'
            error = PermissionError(13, 'INERT_TERMINAL_READ_REFUSED', str(target))
            def unreadable(path):
                """Deny only the owned receipt read, preserving original exception."""
                if path == target: raise error
                return real_read(path)
            with patch.object(Path, 'read_bytes', unreadable):
                with self.assertRaises(PermissionError) as caught: module.verify_retained(folder)
            self.assertIs(caught.exception, error)
            self.fixture(); self.assertEqual(child.call_count, 0)


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
    output = Path(tempfile.mkdtemp(prefix='inert-cd01-terminal-suite-', dir=parent)); os.chmod(output, 0o700)
    producer_spec = importlib.util.spec_from_file_location('native_build_outcome', args.outcome_source)
    producer = importlib.util.module_from_spec(producer_spec); producer_spec.loader.exec_module(producer)
    spec = importlib.util.spec_from_file_location('inert_cd01_pair', args.candidate)
    module = importlib.util.module_from_spec(spec)
    with patch.dict(sys.modules, {'native_build_outcome': producer}): spec.loader.exec_module(module)
    TerminalGuards.config = {'module': module, 'parent': output}
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
            result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(TerminalGuards))
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
        module.write_new(output / 'summary.json', {'scope': 'INERT_TERMINAL_VERIFIER_ONLY', 'tests_run': result.testsRun,
            'failures': len(result.failures), 'errors': len(result.errors), 'skipped': len(result.skipped), 'source_stable': stable,
            'source_postcheck': custody['post_validation_inputs'], 'real_children': 0, 'rust_executed': False, 'status': 'PASS' if ok else 'FAIL'})
    except BaseException as error:
        module.write_new(output / 'suite-failure.json', module.failure_document(error))
        print('INERT_CD01_TERMINAL_EVIDENCE ' + str(output))
        print('INERT_CD01_TERMINAL_FAIL')
        return 2
    print('INERT_CD01_TERMINAL_EVIDENCE ' + str(output))
    print('INERT_CD01_TERMINAL_PASS five-named-methods zero-real-children' if ok else 'INERT_CD01_TERMINAL_FAIL')
    return 0 if ok else 1


if __name__ == '__main__': raise SystemExit(main())
