#!/usr/bin/env python3
"""Proposed inert collector regressions; not executed in source preparation.

Retention calls invoke() present in both reviewed revisions, with one simulated
CompletedProcess and owned files. No executable, subprocess, product fixture-area
configuration, real kernel FD or Biba API is exercised. Bindings need an ACTUAL
complete reviewed v2 descriptor as healthy control before each isolated metadata
mutation. Synthetic metadata is negative-test input, never a compiler receipt.
"""
import argparse
import base64
import copy
import errno
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import types
import unittest
from unittest import mock

STDOUT = b'OWNED_INERT_STDOUT\n'
STDERR = b'OWNED_INERT_STDERR\n'


def sha(path):
    """Hash only an explicitly bound test input or owned metadata file."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    """Create a new owned test record; never overwrite a historical input."""
    raw = (json.dumps(value, indent=2)+'\n').encode()
    with path.open('xb') as out:
        out.write(raw); out.flush(); os.fsync(out.fileno())


def exception_nodes(error):
    """Walk actual exception-group children and explicit causes, preserving identity.

    Do not interpret text, low_fd_outcome metadata or implicit context as a
    replacement for a retained explicit cause. Return distinct real exceptions.
    """
    pending = [error]; seen = set(); found = []
    while pending:
        node = pending.pop()
        if not isinstance(node, BaseException) or id(node) in seen:
            continue
        seen.add(id(node)); found.append(node)
        if isinstance(node, BaseExceptionGroup):
            pending.extend(node.exceptions)
        if node.__cause__ is not None:
            pending.append(node.__cause__)
    return found


def outcome_nodes(error):
    """Read completion payloads attached to actual explicit exception nodes."""
    return [node.low_fd_outcome for node in exception_nodes(error) if hasattr(node, 'low_fd_outcome')]


def collector(path):
    """Load only during an allocated test run, with fixture allocation disabled.

    A stub prevents importing runtime settings. Tests exercise collector retention
    and descriptor predicates, not fixture_area policy/configuration. No fallback
    or stub is used by real low-FD runs; the approved collector is not modified.
    """
    stub = types.ModuleType('fixture_area')
    def forbidden(_prefix):
        raise AssertionError('inert-test-must-not-allocate-product-fixture')
    stub.fixture_root = forbidden
    spec = importlib.util.spec_from_file_location('owned_low_fd_collector_under_test', path)
    module = importlib.util.module_from_spec(spec)
    with mock.patch.dict(sys.modules, {'fixture_area': stub}):
        spec.loader.exec_module(module)
    return module


class Retention(unittest.TestCase):
    """A valid nonzero-completion retention control precedes every IO fault."""
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix=self._testMethodName+'-', dir=OUTPUT))
        self.module = collector(COLLECTOR)

    def case(self, fault=None, close_fault=False):
        """Collect invoke with simulated completion and original injected exception identities.

        Return paths, attempts, raised error and hook facts without assertions, so
        the caller can retain a fresh recovery before checking a negative result.
        """
        root = Path(tempfile.mkdtemp(prefix='case-', dir=self.root)); out = root/'out'
        driver = root/'driver-never-executed'; driver.write_bytes(b'INERT_NOT_AN_ELF')
        shim = root/'shim-never-loaded'; shim.write_bytes(b'INERT_NOT_A_SHARED_OBJECT')
        attempts = []; close_seen = []; primary = None
        injected = {'publication': [], 'cleanup': []}
        publish = self.module.publish; real_close = os.close
        def fixture(_prefix):
            return Path(tempfile.mkdtemp(prefix='owned-inert-root-', dir=root))
        def publication(path, value):
            attempts.append(path.name)
            refused = path.name == fault or fault == 'all' and path.name in {'process.json', 'stdout', 'stderr', 'retention-failure.json'}
            if refused:
                error = OSError(errno.EACCES, 'owned-publication-refused')
                injected['publication'].append(error)
                raise error
            return publish(path, value)
        def closing(fd):
            target = os.readlink('/proc/self/fd/'+str(fd))
            matched = fd > 2 and target == str(out/'events.tsv')
            result = real_close(fd)
            if matched:
                close_seen.append(fd)
                if close_fault:
                    error = OSError(errno.EIO, 'injected-after-owned-channel-close')
                    injected['cleanup'].append(error)
                    raise error
            return result
        returned = subprocess.CompletedProcess(['INERT_NOT_EXECUTED'], 7, STDOUT, STDERR)
        with mock.patch.object(self.module, 'fixture_root', fixture), mock.patch.object(self.module, 'publish', publication), mock.patch.object(self.module.subprocess, 'run', return_value=returned) as process, mock.patch.object(self.module.os, 'close', closing):
            try:
                self.module.invoke('owned-inert', 'control', driver, shim, out)
            except BaseException as error:
                primary = error
            process_calls = process.call_count
        return out, attempts, primary, dict(process_calls=process_calls, close_seen=close_seen, injected=injected)

    def assert_case(self, observed):
        """Check one already collected case, after any promised recovery is retained."""
        _out, _attempts, error, details = observed
        self.assertEqual(details['process_calls'], 1, 'Never rerun a completed simulated process')
        self.assertIsNotNone(error, 'Nonzero child is an issue, never fixture success')
        self.assertEqual(len(details['close_seen']), 1, 'Own event-channel close must be observed once')

    def control(self):
        """Collect/check a fresh case retaining both streams and known numeric exit7."""
        observed = self.case(); self.assert_case(observed)
        out = observed[0]
        self.assertEqual((out/'stdout').read_bytes(), STDOUT)
        self.assertEqual((out/'stderr').read_bytes(), STDERR)
        self.assertEqual(json.loads((out/'process.json').read_bytes())['returncode'], 7)
        return out

    def assert_failure_causes(self, error, injected, cleanup):
        """Require real process and identical injected IO causes in explicit topology.

        Metadata, diagnostic text and unrelated same-errno instances cannot stand
        in for the original publication or cleanup exception objects. Numeric
        issue7 is checked separately in the completion payload, since the real
        collector's process ValueError carries its static protocol code only.
        """
        nodes = exception_nodes(error)
        self.assertTrue(any(type(node) is ValueError and node.args == ('child-returned-nonzero',) for node in nodes), 'Actual process-failure node must survive')
        self.assertTrue(injected['publication'], 'Publication fault must actually be injected')
        for expected in injected['publication']:
            self.assertIsInstance(expected, OSError)
            self.assertEqual(expected.errno, errno.EACCES)
            self.assertTrue(any(node is expected for node in nodes), 'Original publication EACCES node must survive')
        if cleanup:
            self.assertEqual(len(injected['cleanup']), 1, 'Exactly one owned-close EIO injected')
            expected = injected['cleanup'][0]
            self.assertIsInstance(expected, OSError); self.assertEqual(expected.errno, errno.EIO)
            self.assertTrue(any(node is expected for node in nodes), 'Original cleanup EIO node must survive')
        else:
            self.assertEqual(injected['cleanup'], [])

    def assert_observed(self, fault, cleanup=False):
        """Collect witness, fault and fresh recovery before asserting negative details.

        Recovery uses a new case/root; never retries a refused publication or
        simulated completed process. Finally runs even if fault collection raises.
        """
        witness = self.control()
        try:
            observed = self.case(fault, cleanup)
        finally:
            recovery = self.control()
            write(self.root/'recovery-observation.json', {'witness': str(witness), 'fresh_recovery': str(recovery), 'scope': 'INERT_RETENTION_ONLY_SIMULATED_EXIT7_NOT_A_REAL_CHILD'})
        self.assert_case(observed)
        _, attempted, error, details = observed
        self.assert_failure_causes(error, details['injected'], cleanup)
        for name in ['process.json', 'stdout', 'stderr']:
            self.assertEqual(attempted.count(name), 1)
        nodes = outcome_nodes(error); self.assertTrue(nodes, 'Completed outcome must survive the raised error')
        outcome = nodes[0]; self.assertTrue(outcome['completed']); self.assertEqual(outcome['returncode'], 7)
        self.assertEqual(base64.b64decode(outcome['channels']['stdout']['raw_base64']), STDOUT)
        self.assertEqual(base64.b64decode(outcome['channels']['stderr']['raw_base64']), STDERR)
        self.assertFalse(outcome['paired_file_retention_complete'])
        for name in ['process.json', 'stdout', 'stderr']:
            self.assertEqual(outcome['publications'][name]['ok'], fault != 'all' and fault != name)
        return error, details['injected']

    def assert_cause_dropping_refused(self, omit):
        """Proposed guard discriminator: unchanged payload cannot hide a lost cause.

        After actual collector witness/fault/fresh recovery, construct only a
        private exception double dropping one real cause. Do not modify collector
        source or rerun invoke. Expected AssertionError tests the guard itself.
        """
        error, injected = self.assert_observed('stdout', cleanup=True)
        leaves = [ValueError('child-returned-nonzero')]
        if omit != 'publication':
            leaves.extend(injected['publication'])
        if omit != 'cleanup':
            leaves.extend(injected['cleanup'])
        # Same errno on an unrelated node, text and metadata must not substitute.
        leaves.append(OSError(errno.EIO if omit == 'cleanup' else errno.EACCES, 'UNRELATED_REPLACEMENT'))
        leaves.append(ValueError('metadata-only EIO EACCES child7'))
        dropping = BaseExceptionGroup('INERT_CAUSE_DROPPING_DOUBLE', leaves)
        dropping.private_metadata = {'cleanup_errno': errno.EIO, 'publication_errno': errno.EACCES, 'returncode': 7}
        dropping.low_fd_outcome = outcome_nodes(error)[0]
        with self.assertRaises(AssertionError):
            self.assert_failure_causes(dropping, injected, cleanup=True)

    def test_guard_refuses_dropped_cleanup_despite_intact_completion(self):
        self.assert_cause_dropping_refused('cleanup')

    def test_guard_refuses_dropped_publication_despite_intact_completion(self):
        self.assert_cause_dropping_refused('publication')

    def test_stdout_refusal_keeps_other_channel_and_numeric_issue(self):
        self.assert_observed('stdout')

    def test_stderr_refusal_keeps_other_channel_and_numeric_issue(self):
        self.assert_observed('stderr')

    def test_process_receipt_refusal_keeps_both_channels_and_numeric_issue(self):
        self.assert_observed('process.json')

    def test_primary_and_owned_channel_close_failures_keep_completion(self):
        self.assert_observed('stdout', cleanup=True)

    def test_all_writes_refused_do_not_claim_durable_retention(self):
        self.assert_observed('all')


class Bindings(unittest.TestCase):
    """Admission-only metadata counterexamples after a real qualified v2 control."""
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix=self._testMethodName+'-', dir=OUTPUT))
        self.module = collector(COLLECTOR)

    def mutation(self, edit, expected):
        """Keep originals untouched; collect copied-metadata refusal then fresh admission.

        Record intact recovery before negative assertions; all child calls remain
        forbidden and the original/negative metadata files remain available.
        """
        with mock.patch.object(self.module.subprocess, 'run', side_effect=AssertionError('admission-must-not-spawn')) as process:
            original, _ = self.module.inputs(ARTIFACTS)  # mandatory valid control
            changed = copy.deepcopy(original)
            baseline = json.loads(Path(original['baseline']['receipt']['path']).read_bytes())
            candidate = json.loads(Path(original['candidate']['receipt']['path']).read_bytes())
            edit(baseline, candidate, self.root)
            receipt = self.root/'negative-input-receipt.json'; write(receipt, baseline)
            changed['baseline']['receipt'] = {'path': str(receipt), 'sha256': sha(receipt)}
            document = self.root/'negative-input-descriptor.json'; write(document, changed)
            write(self.root/'scope.json', {'status': 'SYNTHETIC_NEGATIVE_METADATA_NOT_A_PRODUCER_PROOF', 'expected_refusal': expected})
            refusal = None
            try:
                self.module.inputs(document)
            except BaseException as error:
                refusal = error
            finally:
                recovered, _ = self.module.inputs(ARTIFACTS)
                write(self.root/'recovery-observation.json', {'original_descriptor': str(ARTIFACTS), 'original_descriptor_sha256': sha(ARTIFACTS), 'returned_original_document': recovered == original, 'scope': 'INERT_ADMISSION_ONLY_NO_CHILD'})
            self.assertEqual(recovered, original, 'Untouched original descriptor must re-admit')
            self.assertIsInstance(refusal, ValueError)
            self.assertEqual(str(refusal), expected)
            process.assert_not_called()

    def test_mixed_biba_extern_cannot_borrow_normal_record(self):
        def edit(baseline, candidate, _root):
            old = str(Path(baseline['files']['biba_rlib']['path']).resolve())
            baseline['files']['biba_rlib'] = copy.deepcopy(candidate['files']['biba_rlib'])
            new = str(Path(baseline['files']['biba_rlib']['path']).resolve())
            baseline['compiler_argv'] = [a.replace('swarm_biba_linux='+old, 'swarm_biba_linux='+new) for a in baseline['compiler_argv']]
        self.mutation(edit, 'nested-copy-linked-bytes-mismatch')

    def test_unallocated_nonempty_revision_refuses(self):
        def edit(baseline, _candidate, _root):
            baseline['revision'] = 'unallocated-counterexample-revision'
        self.mutation(edit, 'allocated-revision-mismatch')

    def test_biba_filename_cannot_point_to_libc_record(self):
        def edit(baseline, _candidate, _root):
            baseline['cargo_bindings']['biba_rlib']['cargo_filename'] = baseline['cargo_bindings']['libc_rlib']['cargo_filename']
        self.mutation(edit, 'cargo-artifact-filename-not-selected')

    def test_mixed_copy_receipt_refuses(self):
        def edit(baseline, candidate, _root):
            baseline['cargo_bindings']['biba_rlib']['copy_receipt'] = copy.deepcopy(candidate['cargo_bindings']['biba_rlib']['copy_receipt'])
        self.mutation(edit, 'biba-copy-source-mismatch')

    def test_libc_filename_cannot_point_to_biba_record(self):
        def edit(baseline, _candidate, _root):
            baseline['cargo_bindings']['libc_rlib']['cargo_filename'] = baseline['cargo_bindings']['biba_rlib']['cargo_filename']
        self.mutation(edit, 'cargo-artifact-filename-not-selected')

    def test_extra_traits_is_not_the_allocated_libc_profile(self):
        def edit(baseline, _candidate, root):
            source = Path(baseline['files']['cargo_artifacts']['path']); lines = []
            for line in source.read_text().splitlines():
                if line.startswith('{'):
                    row = json.loads(line)
                    if row.get('reason') == 'compiler-artifact' and row['target']['name'] == 'libc' and row['target']['kind'] == ['lib']:
                        row['features'] = ['default', 'extra_traits', 'std']
                    line = json.dumps(row)
                lines.append(line)
            target = root/'negative-cargo-channel.jsonl'
            with target.open('x') as out:
                out.write('\n'.join(lines)+'\n')
            baseline['files']['cargo_artifacts'] = {'path': str(target), 'sha256': sha(target)}
        self.mutation(edit, 'libc-library-artifact-profile')


class Producers(unittest.TestCase):
    """Actual component receipts as controls; synthetic negative metadata only.

    Exercise mapped_library directly without requiring new driver compilation.
    This does not qualify full inputs(), compiler receipts or actual low-FD runs.
    Every fault has a healthy control and an unchanged fresh recovery call.
    """
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix=self._testMethodName+'-', dir=OUTPUT))
        self.module = collector(COLLECTOR)
        self.controls = self.module.load(COMPONENTS)

    def case(self, label, edit=None, expected=None):
        """Check one real producer then one isolated mutation and recovery.

        No original proof is edited. Children are forbidden for the whole case.
        Copy/record mutations are explicitly synthetic inputs, not build evidence.
        """
        item = self.controls[label]
        receipt_path = self.module.bound(item['receipt'])
        cargo = self.module.bound(item['cargo'])
        files = {k: self.module.bound(v) for k, v in item['files'].items()}
        rows = [json.loads(line) for line in cargo.read_text().splitlines() if line.startswith('{')]
        record = next(r for r in rows if r.get('reason') == 'compiler-artifact' and r['target']['name'] == 'swarm_biba_linux')
        libc = next(r for r in rows if r.get('reason') == 'compiler-artifact' and r['target']['name'] == 'libc' and r['target']['kind'] == ['lib'])
        receipt = self.module.load(receipt_path)
        binding = {'cargo_filename': record['filenames'][0], 'cargo_record_sha256': self.module.cargo_record_hash(record), 'copy_receipt': item['receipt']}
        libc_binding = {'cargo_filename': next(p for p in libc['filenames'] if p.endswith('.rlib')), 'cargo_record_sha256': self.module.cargo_record_hash(libc), 'copy_receipt': item['libc_receipt']}
        def call(value):
            self.module.mapped_library('libc_rlib', libc, libc_binding, files, cargo, item['revision'], libc)
            return self.module.mapped_library('biba_rlib', record, value, files, cargo, item['revision'], libc)
        with mock.patch.object(self.module.subprocess, 'run', side_effect=AssertionError('producer-admission-must-not-spawn')) as process:
            self.assertEqual(call(binding), receipt_path)
            if edit is not None:
                changed = copy.deepcopy(receipt); edit(changed)
                target = self.root/(label+'-synthetic-negative.json'); write(target, changed)
                negative = copy.deepcopy(binding); negative['copy_receipt'] = {'path': str(target), 'sha256': sha(target)}
                with self.assertRaises(ValueError) as caught:
                    call(negative)
                self.assertEqual(str(caught.exception), expected)
                self.assertEqual(call(binding), receipt_path)
            process.assert_not_called()

    def test_flat_green_control(self):
        self.case('candidate')

    def test_nested_baseline_control(self):
        self.case('baseline')

    def test_flat_cannot_borrow_nested_shape(self):
        self.case('candidate', lambda r: r.update(frozen_library={}), 'copy-path-fields-invalid')

    def test_nested_cannot_borrow_flat_shape(self):
        self.case('baseline', lambda r: r.update(original_library='synthetic'), 'nested-copy-shape-invalid')

    def test_artifact_test_profile_mismatch_refuses_both_forms(self):
        for label in ['baseline', 'candidate']:
            with self.subTest(label=label):
                self.case(label, lambda r: r['artifact']['profile'].update(test=True), 'biba-copy-record-mismatch')

    def test_artifact_features_mismatch_refuses_both_forms(self):
        for label in ['baseline', 'candidate']:
            with self.subTest(label=label):
                self.case(label, lambda r: r['artifact'].update(features=['test-fixtures']), 'biba-copy-record-mismatch')

    def test_artifact_boolean_number_confusion_refuses_both_forms(self):
        for label in ['baseline', 'candidate']:
            with self.subTest(label=label):
                self.case(label, lambda r: r['artifact']['profile'].update(test=0), 'biba-copy-record-mismatch')

    def test_nested_incomplete_mapping_refuses(self):
        self.case('baseline', lambda r: r['frozen_library'].pop('bytes'), 'nested-copy-fields')

    def test_nested_test_profile_refuses(self):
        self.case('baseline', lambda r: r['frozen_library']['profile'].update(test=True), 'nested-copy-record-mismatch')

    def test_nested_feature_mismatch_refuses(self):
        self.case('baseline', lambda r: r['frozen_library'].update(features=['test-fixtures']), 'nested-copy-record-mismatch')

    def test_nested_freshness_type_refuses(self):
        self.case('baseline', lambda r: r['frozen_library'].update(cargo_fresh=0), 'nested-copy-metadata-types')

    def test_nested_byte_count_boolean_refuses(self):
        self.case('baseline', lambda r: r['frozen_library'].update(bytes=True), 'nested-copy-metadata-types')

    def test_nested_filename_refuses(self):
        self.case('baseline', lambda r: r['frozen_library'].update(original_filename='wrong.rlib'), 'nested-copy-original-mismatch')

    def test_nested_revision_refuses(self):
        self.case('baseline', lambda r: r.update(head='synthetic-unallocated'), 'biba-copy-revision-mismatch')

    def test_nested_libc_profile_refuses(self):
        self.case('baseline', lambda r: r['libc_frozen_mapping'].update(features=['default', 'extra_traits', 'std']), 'nested-copy-record-mismatch')

    def test_nested_libc_other_frozen_bytes_refuse(self):
        def edit(r):
            r['libc_frozen_mapping']['frozen'] = r['frozen_library']['frozen']
            r['libc_frozen_mapping']['sha256'] = r['frozen_library']['sha256']
        self.case('baseline', edit, 'nested-copy-linked-bytes-mismatch')

    def test_source_inventory_mismatch_refuses_both_forms(self):
        for label in ['baseline', 'candidate']:
            with self.subTest(label=label):
                self.case(label, lambda r: r['effective_inputs'].update({'rust/crates/swarm-biba-linux/src/lib.rs': '0'*64}), 'biba-copy-source-mismatch')

    def test_flat_wrong_exact_extern_path_refuses(self):
        self.case('candidate', lambda r: r.update(frozen_library=self.controls['baseline']['files']['biba_rlib']['path']), 'cargo-original-frozen-path-mismatch')

    def test_flat_wrong_bytes_refuse(self):
        self.case('candidate', lambda r: r.update(sha256='0'*64), 'cargo-frozen-bytes-mismatch')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--collector', type=Path, required=True)
    parser.add_argument('--stage', choices=['retention', 'bindings', 'producers'], required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--artifacts', type=Path)
    parser.add_argument('--components', type=Path)
    args = parser.parse_args()
    if args.stage == 'bindings' and args.artifacts is None:
        parser.error('bindings requires a complete qualified descriptor, never the draft')
    if args.stage == 'producers' and args.components is None:
        parser.error('producers requires actual source-bound component controls')
    COMPONENTS = args.components.resolve(strict=True) if args.components else None
    COLLECTOR = args.collector.resolve(strict=True); ARTIFACTS = args.artifacts.resolve(strict=True) if args.artifacts else None
    os.umask(0o077); OUTPUT = args.output.resolve(); OUTPUT.mkdir(mode=0o700, parents=False, exist_ok=False)
    write(OUTPUT/'invocation.json', {'collector': str(COLLECTOR), 'collector_sha256': sha(COLLECTOR), 'stage': args.stage, 'components': str(COMPONENTS) if COMPONENTS else None, 'artifacts': str(ARTIFACTS) if ARTIFACTS else None, 'simulated_completion_only': True, 'actual_children': 0, 'global_watchdog': None})
    suite = unittest.defaultTestLoader.loadTestsFromTestCase({'retention': Retention, 'bindings': Bindings, 'producers': Producers}[args.stage])
    count = suite.countTestCases(); result = unittest.TextTestRunner(verbosity=2).run(suite)
    complete = count > 0 and result.testsRun == count and not result.failures and not result.errors and not result.skipped
    write(OUTPUT/'result.json', {'status': 'INERT_CHECKS_PASS' if complete else 'INERT_CHECKS_FAIL_OR_INCOMPLETE', 'collected': count, 'run': result.testsRun, 'failures': len(result.failures), 'errors': len(result.errors), 'skips': len(result.skipped), 'actual_children': 0, 'scope': 'Collector retention/admission only, no compilation/kernel/provider/product proof'})
    raise SystemExit(0 if complete else 1)
