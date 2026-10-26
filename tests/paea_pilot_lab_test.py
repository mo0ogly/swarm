#!/usr/bin/env python3
"""Falsifiable PAEA laboratory recipes, using real effects and a pinned native CLI.

No build, provider, live Root, authority boundary or independent evaluation.
Every injected fault has a valid control first. Raw subprocess outputs, exact
documents, sink roots and observations remain in a fresh evidence directory.
"""
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools.paea import pilot_lab as lab


class PilotLab(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        """Require fresh evidence and a pinned existing executable; never compile."""
        cls.out = Path(os.environ['SWARM_PAEA_PILOT_EVIDENCE']).resolve()
        cls.out.mkdir()
        cls.binary = Path(os.environ['SWARM_PAEA_TEST_BINARY']).resolve(strict=True)
        cls.binary_hash = os.environ['SWARM_PAEA_TEST_BINARY_SHA256']
        if hashlib.sha256(cls.binary.read_bytes()).hexdigest() != cls.binary_hash:
            raise ValueError('existing-binary-hash-mismatch')
        cls.vector = json.loads((ROOT / 'tests/fixtures/paea/contract-v1.json').read_text())['vectors'][0]
        cls.sequence = 0
        cls.case_sequence = 0
        cls.calls = []

    @classmethod
    def tearDownClass(cls):
        """Retain a complete call index and ensure the exercised binary stayed exact."""
        (cls.out / 'calls.json').write_text(json.dumps(cls.calls, indent=2) + '\n')
        if hashlib.sha256(cls.binary.read_bytes()).hexdigest() != cls.binary_hash:
            raise ValueError('existing-binary-changed-during-run')
        print('PAEA_PILOT_BINARY_STABLE ' + cls.binary_hash, flush=True)

    def run_process(self, command, raw=None, stdout=None, files=()):
        """Capture exact argv/input/stdout/stderr/exit without retry or watchdog.

        Optional stdout is an explicit fault sink such as /dev/full; its absence
        of captured bytes is recorded rather than replaced with fabricated output.
        Every numbered file is new. File inputs are copied before execution and
        checked unchanged afterwards; v2 records hash every retained stream/input.
        Failure evidence survives an assertion error.
        """
        type(self).sequence += 1
        prefix = self.out / ('call-%04d' % self.sequence)
        file_inputs = [(Path(path), Path(path).read_bytes()) for path in files]
        result = subprocess.run([str(v) for v in command], input=raw,
                                stdout=stdout if stdout is not None else subprocess.PIPE,
                                stderr=subprocess.PIPE)
        artifacts = {}
        for suffix, value in (('input', raw), ('stdout', result.stdout), ('stderr', result.stderr)):
            if value is not None:
                target = Path(str(prefix) + '.' + suffix)
                target.write_bytes(value)
                artifacts[target.name] = hashlib.sha256(value).hexdigest()
        input_names = []
        for index, (path, value) in enumerate(file_inputs, 1):
            target = Path(str(prefix) + '.file-%04d.input' % index)
            target.write_bytes(value)
            input_names.append(target.name)
            artifacts[target.name] = hashlib.sha256(value).hexdigest()
        self.calls.append({'recording_version': 2, 'id': self.sequence, 'argv': [str(v) for v in command],
                           'exit': result.returncode, 'stdout_fault': stdout is not None,
                           'input_supplied': raw is not None, 'input_files': input_names, 'artifacts': artifacts})
        for path, value in file_inputs:
            self.assertEqual(path.read_bytes(), value, 'fixture input file changed during subprocess')
        return result

    def prepare(self, domain):
        """Create a fresh sink plus valid domain action, using actual Git commit OIDs."""
        type(self).case_sequence += 1
        root = self.out / ('sink-%03d-%s' % (self.case_sequence, domain))
        oids = lab.initialize(root, domain)
        doc = copy.deepcopy({k: self.vector[k] for k in ('proposal', 'context')})
        if domain == 'git':
            doc['proposal']['action'] = {'kind': 'repository.promote', 'repository_id': 'pilot-repository',
                'target_ref': 'refs/heads/pilot', 'expected_old_oid': oids['old'], 'candidate_oid': oids['candidate']}
        report = self.report(doc)
        lab.authority(root, 'approve', report)
        return root, doc, report, oids

    def report(self, doc):
        """Use the real public CLI and retain its original report/bytes, not a Python hash model."""
        raw = json.dumps(doc, ensure_ascii=False).encode()
        result = self.run_process([self.binary, 'protection', 'canonicalize', '--input', '-', '--json', '--lang', 'en'], raw)
        self.assertEqual(result.returncode, 0, result.stderr)
        value = json.loads(result.stdout)
        self.assertEqual(value['scope'], 'offline-data-only')
        self.assertEqual(value['authority'], 'not-established')
        self.assertEqual(hashlib.sha256(value['canonical'].encode()).hexdigest(), value['action_digest'])
        return value

    def commit(self, root, doc, fault=False, stdout=None, raw=None):
        """Run a new fixture service process so post-effect exit truly drops its reply."""
        name = self.out / ('document-%04d.json' % (self.sequence + 1))
        name.write_bytes(raw if raw is not None else json.dumps(doc).encode())
        command = [sys.executable, ROOT / 'tools/paea/pilot_lab.py', root, name]
        if fault:
            command.append('--fault-after-effect')
        return self.run_process(command, stdout=stdout, files=(name,))

    def oracle(self, root, domain, available=True):
        """Read raw sink state in a separate process; never consult replies/decision logs."""
        result = self.run_process([sys.executable, ROOT / 'tools/paea/pilot_oracle.py', root, domain])
        self.assertEqual(result.returncode, 0 if available else 1, result.stderr)
        value = json.loads(result.stdout if available else result.stderr)
        self.assertIs(value['available'], available)
        if not available:
            self.assertEqual(value['state'], 'UNKNOWN')
        return value

    def assert_effect(self, observation, domain, report, count=1):
        """Assert exact stored canonical effect/marker plus Git branch target, not just exit."""
        if domain == 'payment':
            effects = observation['effects']
            self.assertEqual(len(effects), count)
            self.assertTrue(any(row == {'intent': report['intent_id'], 'digest': report['action_digest'],
                                       'canonical': report['canonical']} for row in effects))
        else:
            actual = {item['ref']: item for item in observation['refs']}
            action = json.loads(report['canonical'])['action']
            self.assertEqual(actual[action['target_ref']]['oid'], action['candidate_oid'])
            markers = [item for item in observation['refs'] if item['ref'].startswith('refs/paea-lab/intents/')]
            self.assertEqual(len(markers), count)
            self.assertEqual(actual['refs/paea-lab/intents/' + report['intent_id']]['canonical'], report['canonical'])

    def witness(self, domain):
        """Establish a valid effect under the same fixture preconditions before each fault."""
        root, doc, report, oids = self.prepare(domain)
        before = self.oracle(root, domain)
        result = self.commit(root, doc)
        self.assertEqual(result.returncode, 0, result.stderr)
        after = self.oracle(root, domain)
        self.assertNotEqual(before, after)
        self.assert_effect(after, domain, report)
        self.assertEqual(lab.reconcile(root, report)['state'], 'RECONCILED_COMMITTED')

    def test_t04_substitution_then_equivalent_json_recovery(self):
        """Both domains refuse changed content, preserve state, then accept equivalent JSON."""
        for domain in ('payment', 'git'):
            with self.subTest(domain=domain):
                self.witness(domain)
                root, doc, report, oids = self.prepare(domain)
                before = self.oracle(root, domain)
                changed = copy.deepcopy(doc)
                action = changed['proposal']['action']
                action['amount_minor' if domain == 'payment' else 'candidate_oid'] = '420001' if domain == 'payment' else oids['other']
                refused = self.commit(root, changed)
                self.assertEqual(refused.returncode, 2, refused.stderr)
                self.assertIn(b'laboratory-approval-missing', refused.stderr)
                self.assertEqual(self.oracle(root, domain), before)
                equivalent = json.dumps(doc, indent=2, sort_keys=True).encode()
                self.assertEqual(self.commit(root, doc, raw=equivalent).returncode, 0)
                self.assert_effect(self.oracle(root, domain), domain, report)

    def test_t06_t21_crash_new_attempt_exact_marker_and_reconciliation(self):
        """Actual post-effect exit leaves one effect; new key or context cannot duplicate it."""
        for domain in ('payment', 'git'):
            with self.subTest(domain=domain):
                self.witness(domain)
                root, doc, report, oids = self.prepare(domain)
                self.oracle(root, domain)
                lost = self.commit(root, doc, fault=True)
                self.assertEqual(lost.returncode, 86, lost.stderr)
                self.assertEqual(lost.stdout, b'')
                # Client has no reply: UNKNOWN precedes explicit observation/reconciliation.
                state = {'before_reconciliation': 'UNKNOWN', 'reason': 'post-effect-process-exit-86'}
                after = self.oracle(root, domain)
                self.assert_effect(after, domain, report)
                if domain == 'git':
                    db = lab.connect(root)
                    try:
                        self.assertEqual(db.execute('SELECT count(*) FROM effects').fetchone()[0], 0)
                    finally:
                        db.close()
                state['after_reconciliation'] = lab.reconcile(root, report)['state']
                self.assertEqual(state['after_reconciliation'], 'RECONCILED_COMMITTED')
                (root / 'client-observation.json').write_text(json.dumps(state) + '\n')
                new_attempt = copy.deepcopy(doc)
                new_attempt['proposal']['request_id'] = 'new-transport-key'
                self.assertEqual(self.report(new_attempt), report)
                self.assertEqual(self.commit(root, new_attempt).returncode, 0)
                self.assertEqual(self.oracle(root, domain), after)
                changed = copy.deepcopy(doc)
                changed['context']['instance_id'] = 'other-instance'
                other = self.report(changed)
                self.assertEqual(other['intent_id'], report['intent_id'])
                self.assertNotEqual(other['action_digest'], report['action_digest'])
                lab.authority(root, 'approve', other)
                conflict = self.commit(root, changed)
                self.assertEqual(conflict.returncode, 2, conflict.stderr)
                self.assertIn(b'laboratory-intent-conflict', conflict.stderr)
                self.assertEqual(lab.reconcile(root, other),
                                 {'state': 'UNKNOWN', 'cause': 'laboratory-intent-conflict'})
                self.assertEqual(self.oracle(root, domain), after)

    def test_t05_t17_ordered_revocation_first_use_and_opposite_order(self):
        """Persisted epoch and real process contention order revocation/commit in both domains.

        Ordered cases remain; the helper adds matched controls, actual busy errors,
        unchanged sink observations, explicit recovery and exact durable ordering.
        Same UID/cooperating writers only; no protected or distributed guarantee."""
        for domain in ('payment', 'git'):
            with self.subTest(domain=domain):
                self.witness(domain)
                root, doc, report, oids = self.prepare(domain)
                before = self.oracle(root, domain)
                lab.authority(root, 'revoke')
                denied = self.commit(root, doc)
                self.assertEqual(denied.returncode, 2, denied.stderr)
                self.assertIn(b'laboratory-epoch-revoked', denied.stderr)
                self.assertEqual(self.oracle(root, domain), before)
                self.assertEqual(lab.reconcile(root, report)['state'], 'RECONCILED_NOT_COMMITTED')
                lab.authority(root, 'approve', report)
                self.assertEqual(self.commit(root, doc).returncode, 0)
                self.assert_effect(self.oracle(root, domain), domain, report)
                db = lab.connect(root)
                try:
                    self.assertEqual(db.execute('SELECT event,epoch FROM ordering ORDER BY seq').fetchall(),
                                     [('approve', 1), ('revoke', 2), ('approve', 2), ('commit', 2)])
                finally:
                    db.close()
                reverse, doc, report, oids = self.prepare(domain)
                self.assertEqual(self.commit(reverse, doc).returncode, 0)
                observed = self.oracle(reverse, domain)
                lab.authority(reverse, 'revoke')
                self.assertEqual(self.commit(reverse, doc).returncode, 2)
                self.assertEqual(self.oracle(reverse, domain), observed)
                self.assertEqual(lab.reconcile(reverse, report)['state'], 'RECONCILED_COMMITTED')

        # Extended T05/T17: real overlapping processes, both SQLite lock owners.
        sys.path.insert(0, str(ROOT / 'tests/helpers'))
        from paea_pilot_contention import exercise
        exercise(self)

    def test_unavailable_oracle_stays_unknown_then_recovers_without_creation(self):
        """Missing sink is never an empty success; restoration recovers the original effect."""
        for domain in ('payment', 'git'):
            with self.subTest(domain=domain):
                self.witness(domain)
                root, doc, report, oids = self.prepare(domain)
                self.assertEqual(self.commit(root, doc).returncode, 0)
                before = self.oracle(root, domain)
                source = root / ('sink.sqlite' if domain == 'payment' else 'repository.git')
                offline = root / (source.name + '.offline')
                source.rename(offline)
                try:
                    self.oracle(root, domain, available=False)
                    self.assertEqual(lab.reconcile(root, report)['state'], 'UNKNOWN')
                    self.assertFalse(source.exists())
                finally:
                    offline.rename(source)
                self.assertEqual(self.oracle(root, domain), before)
                self.assertEqual(lab.reconcile(root, report)['state'], 'RECONCILED_COMMITTED')

    def test_git_stale_and_symbolic_target_refuse_then_recover(self):
        """Direct-ref CAS and symbolic-target guards preserve actual refs on rejection."""
        self.witness('git')
        root, doc, report, oids = self.prepare('git')
        lab.git(root, 'update-ref', 'refs/heads/pilot', oids['other'], oids['old'])
        before = self.oracle(root, 'git')
        result = self.commit(root, doc)
        self.assertEqual(result.returncode, 2)
        self.assertIn(b'laboratory-git-cas-mismatch', result.stderr)
        self.assertEqual(self.oracle(root, 'git'), before)
        lab.git(root, 'update-ref', 'refs/heads/pilot', oids['old'], oids['other'])
        lab.git(root, 'update-ref', 'refs/heads/other', oids['old'])
        lab.git(root, 'symbolic-ref', 'refs/heads/pilot', 'refs/heads/other')
        before = self.oracle(root, 'git')
        result = self.commit(root, doc)
        self.assertEqual(result.returncode, 2)
        self.assertIn(b'laboratory-symbolic-ref-forbidden', result.stderr)
        self.assertEqual(self.oracle(root, 'git'), before)
        lab.git(root, 'symbolic-ref', '--delete', 'refs/heads/pilot')
        lab.git(root, 'update-ref', 'refs/heads/pilot', oids['old'])
        self.assertEqual(self.commit(root, doc).returncode, 0)
        self.assert_effect(self.oracle(root, 'git'), 'git', report)

    def test_output_failure_after_effect_is_unknown_and_reconciles(self):
        """A failed real writer after publication retains effect uncertainty and one effect."""
        for domain in ('payment', 'git'):
            with self.subTest(domain=domain):
                self.witness(domain)
                root, doc, report, oids = self.prepare(domain)
                with open('/dev/full', 'wb', buffering=0) as full:
                    result = self.commit(root, doc, stdout=full)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(b'UNKNOWN', result.stderr)
                self.assert_effect(self.oracle(root, domain), domain, report)
                self.assertEqual(lab.reconcile(root, report)['state'], 'RECONCILED_COMMITTED')

    def test_invoice_alias_is_a_counterexample_without_authoritative_registry(self):
        """Retain the real two-row counterexample: raw IDs alone cannot resolve a debt alias."""
        self.witness('payment')
        root, doc, report, oids = self.prepare('payment')
        self.assertEqual(self.commit(root, doc).returncode, 0)
        alias = copy.deepcopy(doc)
        alias['proposal']['action']['invoice_id'] = 'same-external-debt-other-raw-id'
        aliased = self.report(alias)
        self.assertNotEqual(aliased['intent_id'], report['intent_id'])
        lab.authority(root, 'approve', aliased)
        self.assertEqual(self.commit(root, alias).returncode, 0)
        observed = self.oracle(root, 'payment')
        self.assert_effect(observed, 'payment', report, count=2)
        counterexample = {'finding': 'PAEA-GUIDE-03', 'claim_refuted': 'raw invoice identifiers guarantee global debt uniqueness',
            'scope': 'laboratory without an authoritative invoice registry', 'observed_rows': len(observed['effects']),
            'owner': 'PA-02/03 business registry integration', 'resolution': 'resolve project-unique debt objects before admission; version schema if unavailable'}
        (root / 'counterexample.json').write_text(json.dumps(counterexample, indent=2) + '\n')
        print('PAEA_GUIDE_03_COUNTEREXAMPLE_RETAINED two effects for unresolved raw aliases', flush=True)

    def test_resolved_invoice_aliases_converge_on_one_effect(self):
        """Actual public resolver plus sink/oracle: aliases replay one debt effect.

        This cooperative snapshot is NOT protected. Correction preserves intent
        but cannot be attributed to a prior exact grant or duplicate its effect.
        The original unresolved two-row counterexample remains a separate test.
        """
        self.witness('payment')
        root, doc, _, _ = self.prepare('payment')
        registry = json.loads((ROOT / 'tests/fixtures/paea/payment-registry-v1.json').read_bytes())
        request = json.loads((ROOT / 'tests/fixtures/paea/payment-request-v1.json').read_bytes())
        registry['project_id'] = request['project_id'] = doc['context']['project_id']

        def resolved():
            """Retain both raw inputs and invoke the same pinned public executable.

            Failures are recorded before assertions; no fallback proposal is used.
            A successful report establishes data correspondence only.
            """
            left = root / 'registry.json'
            right = root / 'request.json'
            left.write_text(json.dumps(registry))
            right.write_text(json.dumps(request))
            result = self.run_process([self.binary, 'protection', 'resolve-payment', left, right, '--json'], files=(left, right))
            self.assertEqual(result.returncode, 0, result.stderr)
            value = json.loads(result.stdout)
            self.assertEqual(value['registry_sha256'], hashlib.sha256(left.read_bytes()).hexdigest())
            self.assertEqual(value['request_sha256'], hashlib.sha256(right.read_bytes()).hexdigest())
            self.assertEqual(value['registry_authenticity'], 'not-established')
            return value['proposal']

        doc['proposal'] = resolved()
        first = self.report(doc)
        lab.authority(root, 'approve', first)
        self.assertEqual(self.commit(root, doc).returncode, 0)
        self.assert_effect(self.oracle(root, 'payment'), 'payment', first)
        request['invoice_reference'] = 'legacy-12'
        request['request_id'] = 'attempt-alias'
        doc['proposal'] = resolved()
        alias = self.report(doc)
        self.assertEqual(alias['intent_id'], first['intent_id'])
        self.assertEqual(alias['action_digest'], first['action_digest'])
        self.assertEqual(self.commit(root, doc).returncode, 0)
        self.assert_effect(self.oracle(root, 'payment'), 'payment', first, count=1)
        registry['revision'] = request['expected_registry_revision'] = 2
        registry['debts'][0]['invoice_version'] = request['expected_invoice_version'] = 'v8'
        registry['debts'][0]['amount_minor'] = '420001'
        doc['proposal'] = resolved()
        corrected = self.report(doc)
        self.assertEqual(corrected['intent_id'], first['intent_id'])
        self.assertNotEqual(corrected['action_digest'], first['action_digest'])
        self.assertNotEqual(self.commit(root, doc).returncode, 0)
        self.assert_effect(self.oracle(root, 'payment'), 'payment', first, count=1)
        lab.authority(root, 'approve', corrected)
        self.assertNotEqual(self.commit(root, doc).returncode, 0)
        self.assert_effect(self.oracle(root, 'payment'), 'payment', first, count=1)
        print('PAEA_GUIDE_03_RESOLVED_FIXTURE one effect for two resolved aliases; authority unestablished', flush=True)


if __name__ == '__main__':
    unittest.main(testRunner=unittest.TextTestRunner(stream=sys.stdout, verbosity=2))
