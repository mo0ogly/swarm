"""Offline corruption guards; no CLI, database, provider or historical run replay."""
import json
import tempfile
import unittest
from pathlib import Path

import rust_managed_resume_evidence as evidence
import rust_managed_review_w as base


class WitnessEvidenceTests(unittest.TestCase):
    """Require a valid synthetic witness before altering one evidence boundary."""

    def setUp(self):
        """Create closed fake proof bytes with the actual receipt/owner schemas."""
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.checkout = self.root / 'checkout'
        self.checkout.mkdir()
        helper = self.checkout / 'helper.py'
        helper.write_bytes(b'# inert source fixture\n')
        self.directory = self.root / 'proof'
        self.directory.mkdir()
        final = {'id': 'w-fixture', 'tasks': [
            {'id': 'task', 'status': 'submitted', 'criteria': ['fixture criterion'],
             'independent_review': {'id': 'review-fixture', 'state': 'passed'}}],
            'planning': {'reviewer': {'calls': 1, 'max_calls': 2}, 'checks': {}}}
        helpers = {str(helper): evidence.digest(helper.read_bytes())}
        proofs = {}
        for name in ('work-final.json', 'helper-inputs.json', 'engine-settings.json',
                     'original.frame', 'certificate.json', 'invocation.json',
                     'inert-provider.py', 'retention.json', 'managed-context.json'):
            path = self.directory / name
            value = final if name == 'work-final.json' else helpers if name == 'helper-inputs.json' else {}
            raw = json.dumps(value).encode()
            path.write_bytes(raw)
            proofs[name] = {'path': str(path), 'sha256': evidence.digest(raw), 'bytes': len(raw)}
        self.receipt = {'status': base.WITNESS_STATUS, 'receipt_schema': base.WITNESS_SCHEMA,
                        'reviewer_calls_before': 0, 'reviewer_calls_after': 1,
                        'task_accepted': False, 'work': final['id'], 'task': 'task',
                        'review_id': 'review-fixture', 'work_final': proofs['work-final.json'],
                        'helper_inputs': helpers, 'output_proofs': proofs,
                        'semantic_preconditions': base.semantics(final, 'task'),
                        'process_waits': [{'pending': [], 'returncode': 0,
                                           'reap_confirmed': True, 'wait_validated': True,
                                           'capture_closed_observed': True,
                                           'pidfd_held': False, 'streams_held': False}]}

    def validate(self):
        """Repin the synthetic caller receipt, without blessing its nested proofs."""
        path = self.directory / 'receipt.json'
        raw = json.dumps(self.receipt).encode()
        path.write_bytes(raw)
        return evidence.healthy_witness(path, evidence.digest(raw), self.checkout)

    def test_valid_closed_witness(self):
        self.assertEqual(self.validate()['review_id'], 'review-fixture')

    def test_missing_originals_cannot_become_empty_success(self):
        self.validate()
        self.receipt['output_proofs'] = {}
        with self.assertRaisesRegex(evidence.EvidenceError, 'originals-required'):
            self.validate()

    def test_unknown_capture_closure_refuses(self):
        self.validate()
        self.receipt['process_waits'][0]['streams_held'] = None
        with self.assertRaisesRegex(evidence.EvidenceError, 'child-not-settled'):
            self.validate()

    def test_unpinned_final_work_cannot_replace_named_original(self):
        self.validate()
        other = self.directory / 'other.json'
        other.write_bytes((self.directory / 'work-final.json').read_bytes())
        self.receipt['output_proofs']['work-final.json']['path'] = str(other)
        with self.assertRaisesRegex(evidence.EvidenceError, 'named-original-binding'):
            self.validate()

    def test_helper_source_drift_refuses(self):
        self.validate()
        (self.checkout / 'helper.py').write_bytes(b'# changed source\n')
        with self.assertRaisesRegex(evidence.EvidenceError, 'helper-drift'):
            self.validate()

    def test_inline_work_cannot_replace_the_producer_file_pin(self):
        self.validate()
        self.receipt['work_final'] = json.loads((self.directory / 'work-final.json').read_bytes())
        with self.assertRaisesRegex(evidence.EvidenceError, 'final-work-binding'):
            self.validate()

    def test_final_work_call_count_must_match_witness_receipt(self):
        self.validate()
        path = self.directory / 'work-final.json'
        final = json.loads(path.read_bytes())
        final['planning']['reviewer']['calls'] = 2
        raw = json.dumps(final).encode()
        path.write_bytes(raw)
        pin = {'path': str(path), 'sha256': evidence.digest(raw), 'bytes': len(raw)}
        self.receipt['output_proofs']['work-final.json'] = pin
        self.receipt['work_final'] = pin
        with self.assertRaisesRegex(evidence.EvidenceError, 'witness-final-call-count'):
            self.validate()

    def test_proof_byte_change_refuses_and_restore_recovers(self):
        self.validate()
        path = self.directory / 'original.frame'
        original = path.read_bytes()
        path.write_bytes(original + b'changed')
        with self.assertRaisesRegex(evidence.EvidenceError, 'proof-drift'):
            self.validate()
        path.write_bytes(original)
        self.validate()


if __name__ == '__main__':
    unittest.main()
