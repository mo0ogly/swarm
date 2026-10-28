#!/usr/bin/env python3
"""Exercise the offline PAEA CLI using an explicitly supplied existing binary.

Canonicalization, comparison, v1 correspondence and exact-input v2 reports are exercised.
No build, Cargo invocation, server, provider or live project is used. Required
environment: SWARM_PAEA_TEST_BINARY, SWARM_PAEA_TEST_BINARY_SHA256 and a fresh
SWARM_PAEA_TEST_TRANSCRIPT path. Binary bytes are checked before/after the run;
this binds observations to an artifact, not a complete compiler graph or E2/E3.
Every fault has a valid control and recovery in the same isolated root. Original
stdin/stdout/stderr bytes are retained in an atomically published transcript.
There is no global watchdog or automatic retry. Missing prerequisites fail before
collection; skipped/zero tests cannot satisfy the accompanying recipe.
"""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def encode(value):
    """Encode fixture JSON only; expected canonical output comes from frozen vectors."""
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def replace_once(raw, old, new):
    """Mutate exactly one known token; refuse stale/wrong-key fixture substitutions."""
    old, new = old.encode(), new.encode()
    if raw.count(old) != 1:
        raise ValueError('fixture-substitution-is-not-unique')
    return raw.replace(old, new, 1)


class PublicContract(unittest.TestCase):
    """Public stdin/file/output assertions; supplied context never becomes authority."""

    @classmethod
    def setUpClass(cls):
        """Load frozen fixtures; main has already checked the explicit binary identity."""
        cls.vectors = json.loads((ROOT / 'tests/fixtures/paea/contract-v1.json').read_text())['vectors']
        if len(cls.vectors) != 14:
            raise ValueError('frozen-vector-set-changed')

    def setUp(self):
        """Allocate a new isolated root per test; never use a live mission database."""
        self.temporary = tempfile.TemporaryDirectory(prefix='paea-public-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def document(self, index):
        """Copy one fixture's proposal/context without mutating retained vectors."""
        vector = self.vectors[index]
        return copy.deepcopy({key: vector[key] for key in ('proposal', 'context')})

    def invoke(self, raw, language='en', named=False, stdout=None,
               operation='canonicalize', extra=(), input_override=None):
        """Run only offline canonicalize, record original bytes and assert no project IO.

        Operation/options/input overrides exercise refusals inside this offline namespace.
        Callers supply only isolated paths; no active service may be selected.
        A caller-selected /dev/full writer exercises output failure. Subprocess
        errors propagate; no failure is converted to an empty successful result.
        """
        input_name = '-'
        if named:
            input_name = str(self.root / 'document.json')
            Path(input_name).write_bytes(raw)
        if input_override is not None:
            input_name = input_override
        command = [str(self.binary), 'protection', operation, '--input', input_name,
                   '--json', '--lang', language, *extra]
        result = subprocess.run(command, cwd=self.root, input=b'' if named else raw,
                                stdout=subprocess.PIPE if stdout is None else stdout,
                                stderr=subprocess.PIPE)
        self.transcript.append({
            'test': self.id(), 'argv': command, 'named_input': named,
            'input_hex': raw.hex(), 'exit': result.returncode,
            'stdout_hex': None if result.stdout is None else result.stdout.hex(),
            'stderr_hex': result.stderr.hex(),
            'stdout_destination': 'capture' if stdout is None else '/dev/full',
        })
        self.assertEqual({p.name for p in self.root.iterdir()},
                         {'document.json'} if (self.root / 'document.json').exists() else set())
        return result

    def accepted(self, raw, index, language='en', named=False, extra=()):
        """Compare actual public bytes/digests/intent to the independent frozen oracle."""
        result = self.invoke(raw, language, named, extra=extra)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, b'')
        actual = json.loads(result.stdout)
        vector = self.vectors[index]
        self.assertEqual(actual, {
            'schema_version': 1, 'canonicalization': 'paea-json-v1',
            'canonical': vector['canonical'], 'action_digest': vector['action_digest'],
            'intent_id': vector['intent_id'], 'authority': 'not-established',
            'scope': 'offline-data-only',
        })

    def test_all_frozen_vectors_match_public_bytes_in_both_languages(self):
        """Both domains, Unicode refs, large exact amounts and intent variants agree."""
        for index in range(len(self.vectors)):
            for language in ('fr', 'en'):
                with self.subTest(vector=self.vectors[index]['name'], language=language):
                    self.accepted(encode(self.document(index)), index, language,
                                  named=language == 'fr')

    def test_raw_ambiguities_refuse_between_valid_control_and_recovery(self):
        """Reject actual valid-shape mutations; dummy unknown-key objects cannot mask bugs."""
        payment = encode(self.document(0))
        git = encode(self.document(8))
        document_error = 'protection-document-invalid'
        cases = [
            ('duplicate-root', 0, replace_once(payment, '"proposal":{', '"proposal":{},"proposal":{'), document_error),
            ('duplicate-nested', 0, replace_once(payment, '"kind":"payment.settle"', '"kind":"payment.settle","kind":"payment.settle"'), document_error),
            ('unicode-equivalent-key', 0, replace_once(payment, '"kind":"payment.settle"', '"kind":"payment.settle","ki\\u006ed":"payment.settle"'), document_error),
            ('float', 0, replace_once(payment, '"schema_version":1', '"schema_version":1.0'), document_error),
            ('exponent', 0, replace_once(payment, '"schema_version":1', '"schema_version":1e0'), document_error),
            ('huge-exponent', 0, replace_once(payment, '"schema_version":1', '"schema_version":1e999'), document_error),
            ('revision-overflow', 0, replace_once(payment, '"mandate_revision":1', '"mandate_revision":9223372036854775808'), document_error),
            ('numeric-money', 0, replace_once(payment, '"amount_minor":"420000"', '"amount_minor":420000'), document_error),
            ('leading-zero-money', 0, replace_once(payment, '"amount_minor":"420000"', '"amount_minor":"0420000"'), 'protection-amount-representation'),
            ('unpaired-surrogate', 0, replace_once(payment, 'invoice-17', 'invoice-\\ud800'), document_error),
            ('invalid-utf8', 0, payment + b'\xff', document_error),
            ('second-document', 0, payment + b' {"x":1}', document_error),
            ('forged-authority', 0, replace_once(payment, '"proposal":{', '"PRIVATE-CANARY-NOT-A-PERMISSION":"approved","proposal":{'), document_error),
            ('git-alias', 8, replace_once(git, 'refs/heads/candidate', 'HEAD'), 'protection-repository-target'),
            ('git-mixed-algorithm', 8, replace_once(git, '"candidate_oid":"' + '2' * 40 + '"', '"candidate_oid":"' + '3' * 64 + '"'), 'protection-git-oid'),
            ('git-noop', 8, replace_once(git, '"candidate_oid":"' + '2' * 40 + '"', '"candidate_oid":"' + '1' * 40 + '"'), 'protection-git-no-change'),
        ]
        for name, index, bad, expected_code in cases:
            with self.subTest(case=name):
                valid = encode(self.document(index))
                self.accepted(valid, index)
                refused = self.invoke(bad)
                self.assertEqual(refused.returncode, 2)
                self.assertEqual(refused.stdout, b'')
                self.assertEqual(json.loads(refused.stderr)['failure']['code'], expected_code)
                self.assertNotIn(b'PRIVATE-CANARY-NOT-A-PERMISSION', refused.stderr)
                self.accepted(valid, index)

    def test_equivalent_input_encodings_preserve_exact_content(self):
        """Whitespace, key order, valid Unicode escaping and label order are unambiguous."""
        for index in (0, 8):
            valid = encode(self.document(index))
            self.accepted(valid, index)
            self.accepted(json.dumps(self.document(index), indent=2).encode(), index)
            document = self.document(index)
            document['context']['sources'][0]['confidentiality_labels'].reverse()
            self.accepted(encode(document), index)
            if index == 0:
                self.accepted(replace_once(valid, 'invoice-17', 'invoice-\\u0031\\u0037'), index)
            self.accepted(valid, index)

    def refused(self, result, code):
        """Require an explicit safe non-retryable refusal, empty stdout and no canary leak."""
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, b'')
        failure = json.loads(result.stderr)['failure']
        self.assertEqual(failure['code'], code)
        self.assertIs(failure['retryable'], False)
        self.assertNotIn(b'PRIVATE-CANARY-NOT-A-PERMISSION', result.stderr)

    def test_active_operations_and_runtime_options_refuse_then_recover(self):
        """Unavailable operations/options cannot open a root or write an output artifact.

        Each probe uses both fixture domains and languages with an exact valid
        control/recovery. This is routing evidence, not protected-project mediation.
        Explicit default root/phase remain accepted. No server is started.
        """
        cases = [(op, op, ()) for op in ('commit', 'approve', 'config-apply', 'mandate-create')]
        cases += [(name, 'canonicalize', flags) for name, flags in (
            ('root', ('--root', 'missing-root')),
            ('server', ('--server', 'unused-server')),
            ('output', ('--output', 'must-not-exist.json')),
            ('task', ('--task', 'unused-task')),
            ('phase', ('--phase', 'audit')),
        )]
        for index in (0, 8):
            valid = encode(self.document(index))
            for language in ('fr', 'en'):
                self.accepted(valid, index, language, extra=('--root', '.', '--phase', 'delivery'))
                for name, operation, flags in cases:
                    with self.subTest(domain=index, language=language, case=name):
                        self.accepted(valid, index, language)
                        self.refused(self.invoke(valid, language, operation=operation, extra=flags),
                                     'command_failed')
                        self.accepted(valid, index, language)

    def test_input_failures_refuse_without_mutation_then_recover(self):
        """Missing/non-regular inputs and empty streams/files preserve diagnostic codes.

        Read only isolated paths. Empty named input uses the existing fixture slot;
        stdin empty is EOF, not a stalled stream. No permission-mode assumption or
        transport-size allocation is used. Valid control/recovery share each root.
        """
        for index in (0, 8):
            valid = encode(self.document(index))
            for language in ('fr', 'en'):
                for name, raw, named, override, code in (
                    ('missing', valid, False, str(self.root / 'absent.json'), 'input-document-io'),
                    ('directory', valid, False, str(self.root), 'input-regular-file-required'),
                    ('empty-stdin', b'', False, None, 'protection-document-invalid'),
                    ('empty-file', b'', True, None, 'protection-document-invalid'),
                ):
                    with self.subTest(domain=index, language=language, case=name):
                        self.accepted(valid, index, language)
                        self.refused(self.invoke(raw, language, named, input_override=override), code)
                        self.accepted(valid, index, language)

    def test_output_failure_is_nonzero_after_valid_control_then_recovers(self):
        """Observe a public writer failure, without inferring success from partial output."""
        valid = encode(self.document(8))
        self.accepted(valid, 8)
        with Path('/dev/full').open('wb', buffering=0) as full:
            refused = self.invoke(valid, stdout=full)
        self.assertEqual(refused.returncode, 2)
        self.assertEqual(json.loads(refused.stderr)['failure']['code'], 'storage_unavailable')
        self.accepted(valid, 8)

    def invoke_files(self, operation, left, right, language='en', suffix=(),
                     extra=(), paths=('left.json', 'right.json'), stdout=None):
        """Run only offline two-file routes in an owned root and retain exact IO.

        None removes one owned fixture to exercise missing-file refusal. Paths and
        suffix override framing/state only; operations are a closed test set, never
        an active service. Assert both filename inventory and every remaining byte
        after each call. IO errors propagate; no timeout/retry policy is introduced.
        """
        if operation not in ('compare', 'check-receipt', 'receipt-report'):
            raise ValueError('non-offline-test-operation')
        for name, raw in (('left.json', left), ('right.json', right)):
            path = self.root / name
            if raw is None:
                if path.exists():
                    path.unlink()
            else:
                path.write_bytes(raw)
        originals = {p.name: p.read_bytes() for p in self.root.iterdir()}
        command = [str(self.binary), 'protection', operation, *paths, *suffix,
                   '--json', '--lang', language, *extra]
        result = subprocess.run(command, cwd=self.root, input=b'',
                                stdout=subprocess.PIPE if stdout is None else stdout,
                                stderr=subprocess.PIPE)
        self.transcript.append({
            'test': self.id(), 'argv': command,
            'input_files': {name: raw.hex() for name, raw in originals.items()},
            'exit': result.returncode,
            'stdout_hex': None if result.stdout is None else result.stdout.hex(),
            'stderr_hex': result.stderr.hex(),
            'stdout_destination': 'capture' if stdout is None else '/dev/full',
        })
        self.assertEqual({p.name for p in self.root.iterdir()}, set(originals))
        for name, raw in originals.items():
            self.assertEqual((self.root / name).read_bytes(), raw)
        return result

    def compared(self, before, after, language='en', changed_paths=(), left=None, right=None):
        """Assert the complete comparison against frozen digests and actual file bytes.

        Expected paths are supplied by the scenario, never copied from the result.
        Label/transport equivalence can change file hashes without canonical changes.
        """
        left = encode(self.document(before)) if left is None else left
        right = encode(self.document(after)) if right is None else right
        result = self.invoke_files('compare', left, right, language)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, b'')
        old, new = self.vectors[before], self.vectors[after]
        def identity(raw, vector):
            """Expected raw SHA and independent frozen canonical/intent digests."""
            return {'document_sha256': hashlib.sha256(raw).hexdigest(),
                    'action_digest': vector['action_digest'], 'intent_id': vector['intent_id']}
        self.assertEqual(json.loads(result.stdout), {
            'schema_version': 1, 'scope': 'offline-data-only', 'comparison': {
                'schema_version': 1, 'authority': 'not-established', 'effect': 'not-established',
                'approval_validity': 'not-established', 'before': identity(left, old),
                'after': identity(right, new), 'changed_paths': list(changed_paths),
                'canonical_content_changed': old['action_digest'] != new['action_digest'],
                'intent_changed': old['intent_id'] != new['intent_id'],
                'transport_request_changed': json.loads(left)['proposal']['request_id'] !=
                    json.loads(right)['proposal']['request_id'],
            },
        })
        return result

    def receipt(self, index, outcome='COMMITTED'):
        """Build explicitly unsigned fixture data; no observation is authenticated."""
        vector = self.vectors[index]
        return {'schema_version': 1, 'action_digest': vector['action_digest'],
                'intent_id': vector['intent_id'], 'sink_id': 'sink-1', 'outcome': outcome,
                'observation_digest': 'd' * 64}

    def received(self, index, previous='COMMIT_REQUESTED', outcome='COMMITTED', language='en'):
        """Assert the whole data-only receipt report, never inferring authenticity/effect."""
        result = self.invoke_files('check-receipt', encode(self.document(index)),
                                   encode(self.receipt(index, outcome)), language,
                                   suffix=('sink-1', previous))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, b'')
        self.assertEqual(json.loads(result.stdout), {
            'schema_version': 1, 'scope': 'offline-data-only',
            'action_digest': self.vectors[index]['action_digest'],
            'intent_id': self.vectors[index]['intent_id'], 'previous_state': previous,
            'proposed_state': outcome, 'authority': 'not-established',
            'receipt_authenticity': 'not-established', 'effect': 'not-established',
        })
        return result

    def test_comparison_exact_reports_both_domains_and_transport_equivalence(self):
        """File identities, exact changes, same intent and transport separation agree."""
        for language in ('fr', 'en'):
            for index in range(len(self.vectors)):
                self.compared(index, index, language)
            for before, after, path in ((0, 2, '/action/amount_minor'),
                                        (8, 10, '/action/candidate_oid')):
                self.compared(before, after, language, (path,))
            for index in (0, 8):
                self.compared(index, index + 1, language)
                labels = self.document(index)
                labels['context']['sources'][0]['confidentiality_labels'].reverse()
                self.compared(index, index, language, right=encode(labels))

    def test_comparison_invalid_either_file_refuses_then_recovers(self):
        """Malformed, duplicate, missing and directory inputs retain exact safe errors."""
        for index in (0, 8):
            good = encode(self.document(index))
            duplicate = replace_once(good, '"proposal":{',
                                     '"propo\\u0073al":{},"proposal":{')
            for side in (0, 1):
                for bad, code in ((b'{"PRIVATE-CANARY-NOT-A-PERMISSION":', 'protection-document-invalid'),
                                  (good + b'{}', 'protection-document-invalid'),
                                  (duplicate, 'protection-document-invalid'),
                                  (None, 'input-document-io')):
                    self.compared(index, index)
                    values = [good, good]
                    values[side] = bad
                    self.refused(self.invoke_files('compare', *values), code)
                    self.compared(index, index)
                self.compared(index, index)
                paths = ['left.json', 'right.json']
                paths[side] = '.'
                self.refused(self.invoke_files('compare', good, good, paths=paths),
                             'input-regular-file-required')
                self.compared(index, index)

    def test_comparison_runtime_options_and_stdin_refuse_then_recovers(self):
        """Unsupported runtime/framing refuses without project or output creation."""
        for language in ('fr', 'en'):
            good = encode(self.document(8))
            for extra in (('--root', 'absent-root'), ('--server', 'unused-server'),
                          ('--input', 'extra.json'), ('--output', 'absent-output'),
                          ('--task', 'unused-task'), ('--phase', 'audit'), ('extra-argument',)):
                self.compared(8, 8, language)
                self.refused(self.invoke_files('compare', good, good, language, extra=extra),
                             'command_failed')
                self.compared(8, 8, language)
            for paths in (('-', 'right.json'), ('left.json', '-'), ('left.json',)):
                self.compared(8, 8, language)
                self.refused(self.invoke_files('compare', good, good, language, paths=paths),
                             'command_failed')
                self.compared(8, 8, language)

    def test_comparison_public_writer_failure_recovers(self):
        """A /dev/full write refuses nonzero and the same comparison then recovers."""
        good = encode(self.document(0))
        self.compared(0, 0)
        with Path('/dev/full').open('wb', buffering=0) as full:
            result = self.invoke_files('compare', good, good, stdout=full)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(json.loads(result.stderr)['failure']['code'], 'storage_unavailable')
        self.compared(0, 0)

    def test_receipt_exact_correspondence_both_domains_and_languages(self):
        """All permitted outcomes remain declared-only, including reconciled UNKNOWN."""
        outcomes = (('COMMIT_REQUESTED', 'COMMITTED'), ('COMMIT_REQUESTED', 'UNKNOWN'),
                    ('PROPOSED', 'DENIED'), ('ADMITTED', 'DENIED'),
                    ('APPROVAL_PENDING', 'DENIED'), ('AUTHORIZED', 'DENIED'),
                    ('COMMIT_REQUESTED', 'DENIED'), ('UNKNOWN', 'RECONCILED_COMMITTED'),
                    ('UNKNOWN', 'RECONCILED_NOT_COMMITTED'))
        for index in (0, 8):
            for language in ('fr', 'en'):
                for previous, outcome in outcomes:
                    self.received(index, previous, outcome, language)

    def test_receipt_malformed_and_mismatched_data_refuse_then_recovers(self):
        """Exact content/intent/sink errors are distinguished from malformed data."""
        for index in (0, 8):
            action, receipt = encode(self.document(index)), encode(self.receipt(index))
            cases = []
            for side in (0, 1):
                cases += [(side, b'{"PRIVATE-CANARY-NOT-A-PERMISSION":', 'protection-document-invalid'),
                          (side, None, 'input-document-io')]
            cases.append((1, replace_once(receipt, '"sink_id":"sink-1"',
                          '"sink_id":"sink-1","sink_\\u0069d":"sink-1"'),
                          'protection-document-invalid'))
            for key, value in (('action_digest', 'e' * 64), ('intent_id', 'f' * 64),
                               ('sink_id', 'PRIVATE-CANARY-NOT-A-PERMISSION')):
                bad = self.receipt(index)
                bad[key] = value
                cases.append((1, encode(bad), 'protection-receipt-binding'))
            bad = self.receipt(index, 'AUTHORIZED')
            cases.append((1, encode(bad), 'protection-effect-receipt'))
            cases.append((0, encode(self.document(2 if index == 0 else 10)), 'protection-receipt-binding'))
            for side, invalid, code in cases:
                self.received(index)
                values = [action, receipt]
                values[side] = invalid
                self.refused(self.invoke_files('check-receipt', *values,
                             suffix=('sink-1', 'COMMIT_REQUESTED')), code)
                self.received(index)

    def test_receipt_state_sink_and_runtime_refusals_then_recovery(self):
        """UNKNOWN/terminals cannot relabel or replay; syntax never chooses a default."""
        for language in ('fr', 'en'):
            action, receipt = encode(self.document(8)), encode(self.receipt(8))
            for sink, previous, code in (
                ('sink-1', 'PRIVATE-CANARY-NOT-A-PERMISSION', 'protection-document-invalid'),
                ('', 'COMMIT_REQUESTED', 'protection-receipt-expected-sink'),
                ('SINK-1', 'COMMIT_REQUESTED', 'protection-receipt-binding'),
                *[('sink-1', state, 'protection-transition-invalid') for state in
                  ('UNKNOWN', 'COMMITTED', 'DENIED', 'RECONCILED_COMMITTED', 'RECONCILED_NOT_COMMITTED')],
            ):
                self.received(8, language=language)
                self.refused(self.invoke_files('check-receipt', action, receipt, language,
                             suffix=(sink, previous)), code)
                self.received(8, language=language)
            for extra in (('--root', 'absent-root'), ('--server', 'unused-server'),
                          ('--input', 'extra.json'), ('--output', 'absent-output'),
                          ('--task', 'unused-task'), ('--phase', 'audit'), ('extra-argument',)):
                self.received(8, language=language)
                self.refused(self.invoke_files('check-receipt', action, receipt, language,
                             suffix=('sink-1', 'COMMIT_REQUESTED'), extra=extra), 'command_failed')
                self.received(8, language=language)
            for paths, suffix in ((('-', 'right.json'), ('sink-1', 'COMMIT_REQUESTED')),
                                  (('left.json', '-'), ('sink-1', 'COMMIT_REQUESTED')),
                                  (('left.json', 'right.json'), ('sink-1',))):
                self.received(8, language=language)
                self.refused(self.invoke_files('check-receipt', action, receipt, language,
                             paths=paths, suffix=suffix), 'command_failed')
                self.received(8, language=language)

    def test_receipt_public_writer_failure_recovers(self):
        """Output failure is not an effect or successful command; valid recovery agrees."""
        action, receipt = encode(self.document(0)), encode(self.receipt(0))
        self.received(0)
        with Path('/dev/full').open('wb', buffering=0) as full:
            result = self.invoke_files('check-receipt', action, receipt,
                                       suffix=('sink-1', 'COMMIT_REQUESTED'), stdout=full)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(json.loads(result.stderr)['failure']['code'], 'storage_unavailable')
        self.received(0)

    def reported(self, index, previous='COMMIT_REQUESTED', outcome='COMMITTED',
                 language='en', action=None, receipt=None):
        """Assert the complete v2 result against exact inputs and frozen identities.

        Optional bytes exercise equivalent transport, not changed canonical content.
        A passing unsigned report still establishes no authority, authenticity or
        actual effect. Return the full process result for cross-report comparisons.
        """
        action = encode(self.document(index)) if action is None else action
        receipt = encode(self.receipt(index, outcome)) if receipt is None else receipt
        result = self.invoke_files('receipt-report', action, receipt, language,
                                   suffix=('sink-1', previous))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, b'')
        vector = self.vectors[index]
        self.assertEqual(json.loads(result.stdout), {
            'schema_version': 2, 'scope': 'offline-data-only',
            'document': {'document_sha256': hashlib.sha256(action).hexdigest(),
                         'action_digest': vector['action_digest'], 'intent_id': vector['intent_id']},
            'receipt_sha256': hashlib.sha256(receipt).hexdigest(),
            'previous_state': previous, 'proposed_state': outcome,
            'authority': 'not-established', 'receipt_authenticity': 'not-established',
            'effect': 'not-established',
        })
        return result

    def test_receipt_report_exact_both_domains_languages_and_legacy_shape(self):
        """Both domains and every permitted observation keep v1 and v2 distinct."""
        outcomes = (('COMMIT_REQUESTED', 'COMMITTED'), ('COMMIT_REQUESTED', 'UNKNOWN'),
                    ('PROPOSED', 'DENIED'), ('UNKNOWN', 'RECONCILED_COMMITTED'),
                    ('UNKNOWN', 'RECONCILED_NOT_COMMITTED'))
        for index in (0, 8):
            for language in ('fr', 'en'):
                for previous, outcome in outcomes:
                    self.reported(index, previous, outcome, language)
                    legacy = self.received(index, previous, outcome, language)
                    self.assertEqual(len(json.loads(legacy.stdout)), 9)
                    self.assertNotIn('receipt_sha256', json.loads(legacy.stdout))

    def test_receipt_report_distinguishes_raw_receipt_and_document_bytes(self):
        """OBS-01: observation/whitespace bytes change v2 while v1 can stay identical."""
        for index in (0, 8):
            document = encode(self.document(index))
            original = self.receipt(index)
            altered = {**original, 'observation_digest': 'e' * 64}
            baseline = self.reported(index)
            observation = self.reported(index, receipt=encode(altered))
            document_format = self.reported(index, action=json.dumps(self.document(index), indent=2).encode())
            receipt_format = self.reported(index, receipt=json.dumps(original, indent=2).encode())
            old = self.invoke_files('check-receipt', document, encode(original), suffix=('sink-1', 'COMMIT_REQUESTED'))
            other = self.invoke_files('check-receipt', document, encode(altered), suffix=('sink-1', 'COMMIT_REQUESTED'))
            self.assertEqual(old.returncode, 0, old.stderr)
            self.assertEqual(other.returncode, 0, other.stderr)
            self.assertEqual(old.stdout, other.stdout)
            a, b = json.loads(baseline.stdout), json.loads(observation.stdout)
            self.assertNotEqual(a['receipt_sha256'], b['receipt_sha256'])
            self.assertEqual(a['document'], b['document'])
            self.assertNotEqual(a['document']['document_sha256'], json.loads(document_format.stdout)['document']['document_sha256'])
            self.assertNotEqual(a['receipt_sha256'], json.loads(receipt_format.stdout)['receipt_sha256'])

    def test_receipt_report_refusal_binding_lifecycle_and_framing_then_recovery(self):
        """No report after malformed/missing input, binding/state or routing refusal."""
        for index in (0, 8):
            action, receipt = encode(self.document(index)), encode(self.receipt(index))
            cases = [
                (b'{', receipt, ('sink-1', 'COMMIT_REQUESTED'), (), ('left.json', 'right.json'), 'protection-document-invalid'),
                (action, b'{', ('sink-1', 'COMMIT_REQUESTED'), (), ('left.json', 'right.json'), 'protection-document-invalid'),
                (None, receipt, ('sink-1', 'COMMIT_REQUESTED'), (), ('left.json', 'right.json'), 'input-document-io'),
                (action, None, ('sink-1', 'COMMIT_REQUESTED'), (), ('left.json', 'right.json'), 'input-document-io'),
                (action, replace_once(receipt, '"schema_version":1', '"schema_version":1,"schema_version":1'),
                 ('sink-1', 'COMMIT_REQUESTED'), (), ('left.json', 'right.json'), 'protection-document-invalid'),
            ]
            for key, value in [('action_digest', 'e' * 64), ('intent_id', 'f' * 64), ('sink_id', 'PRIVATE-CANARY-NOT-A-PERMISSION')]:
                cases.append((action, encode({**self.receipt(index), key: value}), ('sink-1', 'COMMIT_REQUESTED'), (), ('left.json', 'right.json'), 'protection-receipt-binding'))
            for previous in ('UNKNOWN', 'COMMITTED', 'DENIED', 'RECONCILED_COMMITTED', 'RECONCILED_NOT_COMMITTED'):
                cases.append((action, receipt, ('sink-1', previous), (), ('left.json', 'right.json'), 'protection-transition-invalid'))
            for suffix, code in [(('', 'COMMIT_REQUESTED'), 'protection-receipt-expected-sink'),
                                 (('sink-1', 'committed'), 'protection-document-invalid'),
                                 (('sink-1',), 'command_failed')]:
                cases.append((action, receipt, suffix, (), ('left.json', 'right.json'), code))
            for flags in [('--root', 'absent-root'), ('--server', 'unused-server'), ('--input', 'extra.json'),
                          ('--output', 'absent-output'), ('--task', 'unused-task'), ('--phase', 'audit'), ('extra',)]:
                cases.append((action, receipt, ('sink-1', 'COMMIT_REQUESTED'), flags, ('left.json', 'right.json'), 'command_failed'))
            for paths in [('-', 'right.json'), ('left.json', '-')]:
                cases.append((action, receipt, ('sink-1', 'COMMIT_REQUESTED'), (), paths, 'command_failed'))
            for language in ('fr', 'en'):
                for left, right, suffix, flags, paths, code in cases:
                    with self.subTest(index=index, language=language, expected=code, suffix=suffix, flags=flags, paths=paths):
                        self.reported(index, language=language)
                        self.refused(self.invoke_files('receipt-report', left, right, language,
                                     suffix=suffix, extra=flags, paths=paths), code)
                        self.reported(index, language=language)

    def test_receipt_report_public_output_failure_then_recovery(self):
        """Failed output remains nonzero without fixture mutation; recovery gives v2."""
        self.reported(0)
        with Path('/dev/full').open('wb', buffering=0) as full:
            result = self.invoke_files('receipt-report', encode(self.document(0)), encode(self.receipt(0)),
                                       suffix=('sink-1', 'COMMIT_REQUESTED'), stdout=full)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(json.loads(result.stderr)['failure']['code'], 'storage_unavailable')
        self.reported(0)


def main():
    """Require explicit artifact identity, run named checks and publish fresh evidence.

    Binary changes invalidate the run. Transcript includes failed observations and
    original bytes; publication errors propagate instead of replacing old evidence.
    Return 2 for missing prerequisites or failed checks, 0 only for an executed pass.
    """
    required = ('SWARM_PAEA_TEST_BINARY', 'SWARM_PAEA_TEST_BINARY_SHA256',
                'SWARM_PAEA_TEST_TRANSCRIPT')
    if any(not os.environ.get(key) for key in required):
        print('PRECONDITION_REFUSED: supply binary, SHA256 and fresh transcript path')
        return 2
    expected = os.environ[required[1]]
    destination = Path(os.environ[required[2]])
    try:
        binary = Path(os.environ[required[0]]).resolve(strict=True)
        actual = hashlib.sha256(binary.read_bytes()).hexdigest()
    except OSError as error:
        print(f'PRECONDITION_REFUSED: binary IO errno={error.errno}')
        return 2
    if (len(expected) != 64 or any(c not in '0123456789abcdef' for c in expected)
            or actual != expected or not os.access(binary, os.X_OK)
            or destination.exists() or not destination.parent.is_dir()):
        print('PRECONDITION_REFUSED: artifact identity or fresh evidence destination')
        return 2
    PublicContract.binary = binary
    PublicContract.transcript = []
    result = unittest.TextTestRunner(stream=sys.stdout, verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(PublicContract))
    stable = hashlib.sha256(binary.read_bytes()).hexdigest() == expected
    spec = importlib.util.spec_from_file_location('paea_public_publisher', ROOT / 'tools/verification/recipe.py')
    publisher = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(publisher)
    publisher.publish(destination, {
        'schema_version': 1, 'scope': 'existing binary, offline fixture observations only',
        'binary': str(binary), 'binary_sha256': expected, 'binary_stable': stable,
        'tests_run': result.testsRun, 'successful': result.wasSuccessful(),
        'invocations': PublicContract.transcript,
    })
    print('PAEA_PUBLIC_BINARY_STABLE' if stable else 'PAEA_PUBLIC_BINARY_CHANGED')
    return 0 if stable and result.wasSuccessful() and result.testsRun == 18 else 2


if __name__ == '__main__':
    raise SystemExit(main())
