#!/usr/bin/env python3
"""Pure wrapper guards: owned temporary files and mocks, no CLI/provider/runtime.

Every negative variant starts from a valid witness and is followed by recovery.
These guards do not execute the ignored Rust ledger test or authenticate profiles.
"""
import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rust_mission_worker_continuation as subject


class ContinuationGuards(unittest.TestCase):
    """Refusal/recovery coverage for executable identity, names and fresh evidence."""

    def test_profile_hash_default_and_test_identity(self):
        """Valid two-artifact witness precedes invalid profile/hash variants."""
        with tempfile.TemporaryDirectory() as directory:
            normal, test = Path(directory) / 'normal', Path(directory) / 'test'
            normal.write_bytes(b'normal fixture bytes')
            test.write_bytes(b'test fixture bytes')
            n = dict(no_fixture_feature=True, sha256=subject.fixture.digest(normal.read_bytes()))
            t = dict(status='COMPILED_DEFAULT_TEST_ARTIFACT',
                     binary_sha256=subject.fixture.digest(test.read_bytes()),
                     artifact=dict(features=[], profile=dict(test=True),
                                   target=dict(name='swarm_engine', kind=['lib'])))
            subject.qualify(n, t, normal, test)
            for field, value in [('features', ['fixture']), ('profile', {'test': False}),
                                 ('target', {'name': 'other', 'kind': ['lib']})]:
                bad = copy.deepcopy(t)
                bad['artifact'][field] = value
                with self.assertRaises(RuntimeError):
                    subject.qualify(n, bad, normal, test)
                subject.qualify(n, t, normal, test)
            for bad in [dict(n, no_fixture_feature=1), dict(n, sha256='0' * 64)]:
                with self.assertRaises(RuntimeError):
                    subject.qualify(bad, t, normal, test)
                subject.qualify(n, t, normal, test)
            test.write_bytes(b'changed')
            with self.assertRaises(RuntimeError):
                subject.qualify(n, t, normal, test)
            test.write_bytes(b'test fixture bytes')
            subject.qualify(n, t, normal, test)

    def test_exact_ignored_listing_refuses_absent_extra_and_failed(self):
        """A named ignored witness is required; empty and unrelated lists refuse."""
        valid = (subject.TEST + ': test\n\n1 test, 0 benchmarks\n').encode()
        subject.listed(0, valid)
        for code, output in [(0, b'0 tests, 0 benchmarks\n'), (1, valid),
                             (0, valid + b'other: test\n'),
                             (0, valid.replace(subject.TEST.encode(), b'wrong'))]:
            with self.assertRaises(RuntimeError):
                subject.listed(code, output)
            subject.listed(0, valid)

    def test_named_execution_refuses_zero_failed_ignored_or_wrong_name(self):
        """Process exit alone never substitutes for the exact named test count."""
        valid = ('test ' + subject.TEST + ' ... ok\n\ntest result: ok. 1 passed; '
                 '0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n').encode()
        subject.ran(0, valid)
        for code, output in [(101, valid), (0, valid.replace(b'1 passed', b'0 passed')),
                             (0, valid.replace(b'0 ignored', b'1 ignored')),
                             (0, valid.replace(subject.TEST.encode(), b'wrong')),
                             (0, b''),
                             (0, valid + b'test result: ok. 0 passed; 0 failed; 0 ignored;\n')]:
            with self.assertRaises(RuntimeError):
                subject.ran(code, output)
            subject.ran(0, valid)

    def test_result_requires_exact_positive_typed_rows_twice_and_cleanup(self):
        """Valid witness precedes each altered ledger, empty or false proof."""
        root = Path('/owned-fixture')
        row = [{'Text': list(b'agent')}, {'Text': list(b'work')},
               {'Real': 4607182418800017408}, {'Text': list(b'reserved')}]
        observation = dict(ordinal=1, ledger_before=[copy.deepcopy(row)],
                           ledger_after=[copy.deepcopy(row)], all_rows_equal=True,
                           agent_work_receipts_fifo_equal=True, target_unique=True,
                           current_generation=True, total_changes_unchanged=True,
                           target_agent_id='agent', target_work_id='work',
                           target_amount_ieee_bits=4607182418800017408,
                           positive_target_reservation=True)
        proof = dict(status='PASS', observations=[observation, dict(copy.deepcopy(observation), ordinal=2)],
                     reservation_rows_verified=True, observations_are_permission=False,
                     provider_spawn_called=False,
                     scope='owned-queued-agent-reservations-same-Root-connection',
                     lease_released=True, lease_closed=True, root_closed=True)
        subject.validate_result(proof, root, 'work')
        variants = []
        for key, value in [('observations', []), ('observations', [observation]),
                           ('root_closed', False), ('lease_released', 1),
                           ('provider_spawn_called', True), ('status', 'FAIL')]:
            variants.append(dict(copy.deepcopy(proof), **{key: value}))
        for key, value in [('ledger_before', []), ('target_work_id', 'other'),
                           ('target_agent_id', ''), ('ordinal', True),
                           ('target_amount_ieee_bits', 0), ('all_rows_equal', 1)]:
            bad = copy.deepcopy(proof)
            bad['observations'][0][key] = value
            variants.append(bad)
        for column, value in [(0, {'Text': list(b'other')}),
                              (2, {'Real': 4611686018427387904}),
                              (2, {'Integer': 1}), (3, {'Text': list(b'released')})]:
            bad = copy.deepcopy(proof)
            bad['observations'][0]['ledger_after'][0][column] = value
            variants.append(bad)
        for value in [{'Real': True}, {'Real': 9218868437227405312},
                      {'Blob': [1]}, 'Null', {'Integer': 1 << 64}]:
            bad = copy.deepcopy(proof)
            for side in ('ledger_before', 'ledger_after'):
                bad['observations'][0][side][0][2] = value
            variants.append(bad)
        bad = copy.deepcopy(proof)
        for side in ('ledger_before', 'ledger_after'):
            bad['observations'][0][side].append(copy.deepcopy(row))
        variants.append(bad)
        # Equal row count/amount sum cannot hide a swapped owner/amount.
        bad = copy.deepcopy(proof)
        second = copy.deepcopy(row)
        second[0] = {'Text': list(b'other')}
        second[2] = {'Real': 4611686018427387904}
        for item in bad['observations']:
            for side in ('ledger_before', 'ledger_after'):
                item[side].append(copy.deepcopy(second))
        subject.validate_result(bad, root, 'work')
        bad['observations'][0]['ledger_after'][0][2], bad['observations'][0]['ledger_after'][1][2] = (
            bad['observations'][0]['ledger_after'][1][2], bad['observations'][0]['ledger_after'][0][2])
        variants.append(bad)
        for bad in variants:
            with self.assertRaises((RuntimeError, UnicodeError)):
                subject.validate_result(bad, root, 'work')
            subject.validate_result(proof, root, 'work')

    def test_fresh_output_refuses_existing_and_dangling_symlink(self):
        """Fresh witness, existing-file refusal and dangling-link refusal recover."""
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory).resolve() / 'result.json'
            self.assertEqual(subject.fresh(output), output)
            output.write_bytes(b'original proof')
            with self.assertRaises(RuntimeError):
                subject.fresh(output)
            self.assertEqual(output.read_bytes(), b'original proof')
            output.unlink()  # Only this guard's owned temporary file.
            self.assertEqual(subject.fresh(output), output)
            output.symlink_to(output.parent / 'missing')
            with self.assertRaises(RuntimeError):
                subject.fresh(output)
            output.unlink()  # Only this guard's owned temporary link.
            self.assertEqual(subject.fresh(output), output)

    def test_invoke_forwards_fixture_environment_without_recording_values(self):
        """Mocked launch preserves CODEX_HOME and fixture descriptor, no env dump."""
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory).resolve()
            environment = dict(CODEX_HOME='private-fixture-home',
                               SWARM_CONTINUATION_LEDGER_FIXTURE='owned-marker',
                               PRIVATE_TOKEN='must-not-be-recorded')
            def run(argv, **kwargs):
                """Record only supplied mock streams; never start a process."""
                self.assertEqual(kwargs['env'], environment)
                self.assertNotIn('timeout', kwargs)
                kwargs['stdout'].write(b'original stdout')
                kwargs['stderr'].write(b'original stderr')
                return type('Result', (), {'returncode': 7})()
            with patch.object(subject.subprocess, 'run', side_effect=run):
                self.assertEqual(subject.invoke(['owned-program'], environment, output, 'case'),
                                 (7, b'original stdout', b'original stderr'))
            command = (output / 'case.command.json').read_text()
            self.assertNotIn('private-fixture-home', command)
            self.assertNotIn('must-not-be-recorded', command)
            self.assertEqual(json.loads(command)['global_watchdog'], None)
            self.assertEqual(json.loads((output / 'case.exit.json').read_bytes())['exit'], 7)

    def test_failure_publication_preserves_primary_and_secondary_causes(self):
        """Witness metadata survives; publication refusal retains both objects."""
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory).resolve()
            primary, secondary = RuntimeError('private-primary'), OSError('private-secondary')
            subject.publish_failure(output, primary)
            document = json.loads((output / 'failure.json').read_bytes())
            self.assertEqual(document['status'], 'FAIL')
            self.assertNotIn('private-primary', (output / 'failure.json').read_text())
            other = output / 'other'
            other.mkdir()
            with patch.object(subject.fixture, 'publish', side_effect=secondary):
                with self.assertRaises(BaseExceptionGroup) as caught:
                    subject.publish_failure(other, primary)
            self.assertIs(caught.exception.exceptions[0], primary)
            self.assertIs(caught.exception.exceptions[1], secondary)
            subject.publish_failure(other, primary)
            self.assertEqual(json.loads((other / 'failure.json').read_bytes())['status'], 'FAIL')


if __name__ == '__main__':
    unittest.main()
