'''Tests for tools/research/a0_property_oracle.py (synthetic fixtures only).

Purpose: exercise the semantic counterexamples and refusal paths named by the
A0 mandate without duplicating the 83 real cases:
  * V06 history: refusal after a same-intent commit -> PV2_intention FAIL,
    PV2_attempt PASS (no new effect);
  * before_commit refusal then recovery: evaluated against the past prefix
    only, never retroactively reclassified;
  * unknown-after_commit: an unknown response plus a persisted effect triggers
    neither PV1 nor PV2;
  * weak terms (approval binds alpha, commit of beta): PT FAIL with PV1 existence preserved, with the earlier fail preserved across later incomplete steps;
  * missing observations: trace holes yield inconc, never fail nor pass;
  * unattributable invalid stdin refusals stay out of PV2 (no invented i);
  * missing required evidence yields error; unattributed historical rows stay inconc;
  * inventory corruption, traversal, symlinks, existing output: refused with
    safe diagnostics, no output written, source tree untouched;
  * withdrawal present: PL inconc (Delta unspecified), never an invented bound.
Limitations: these tests never assert anything about evidence timestamps and
establish no native Swarm, Git, real-provider or global temporal guarantee.
'''

import hashlib
import importlib.util
import json
import os
import sys
import tempfile
import unittest
from unittest import mock
import errno
from pathlib import Path

_TOOL = Path(__file__).resolve().parents[1] / 'tools' / 'research' / 'a0_property_oracle.py'
_SPEC = importlib.util.spec_from_file_location('a0_property_oracle', _TOOL)
oracle = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = oracle
_SPEC.loader.exec_module(oracle)

JOURNEY = 'journeys/e1-test'


def canon(obj):
    return json.dumps(obj, sort_keys=True, separators=(',', ':'), ensure_ascii=False)


def action_digest(text):
    return hashlib.sha256(b'e1-action-v1\n' + text.encode('utf-8')).hexdigest()


CONFIG = {
    'case_run': 'run_test',
    'profile': 'strict',
    'policy_revision': 1,
    'channels': {
        'D': {'role': 'requester', 'actor': 'd', 'run': 'run_test', 'scope': 'scope'},
        'V': {'role': 'approver', 'actor': 'v', 'run': 'run_test', 'scope': 'scope'},
    },
    'accounts': [['ledger', 'source', 10000], ['ledger', 'alpha', 0], ['ledger', 'beta', 0]],
}


def make_action(beneficiary='alpha'):
    return {'schema_version': 1, 'run_id': 'run_test', 'scope_id': 'scope',
            'intent_id': 'invoice', 'requester_id': 'd', 'operation': 'payment.transfer',
            'ledger_id': 'ledger', 'source_account': 'source',
            'beneficiary_account': beneficiary, 'amount_minor': '1200',
            'currency': 'EUR', 'policy_revision': 1}


def observation(config, approvals, effects):
    return {'case_run': config['case_run'], 'configuration': config,
            'approvals': approvals, 'effects': effects, 'accounts': config['accounts']}


def approval_row(state, action_text):
    return ['approval', action_text, action_digest(action_text), action_text,
            hashlib.sha256(action_text.encode('utf-8')).hexdigest(), 'V', state]


def proc(channel, fault=None):
    argv = ['python3', 'e1_receiver.py', '--store', 'receiver.sqlite', '--channel', channel]
    if fault is not None:
        argv = argv + ['--fault', fault]
    return {'argv': argv, 'exit': 0, 'streams_closed': True}


def tree_hashes(root):
    '''SHA-256 of every file under root/location, keyed by relative path.'''
    loc = os.path.join(root, 'location')
    out = {}
    for base, _dirs, names in os.walk(loc):
        for name in names:
            path = os.path.join(base, name)
            rel = os.path.relpath(path, loc)
            with open(path, 'rb') as handle:
                out[rel] = hashlib.sha256(handle.read()).hexdigest()
    return out


class Tree:
    '''Synthetic retained tree plus A0-PREUVES.json builder (test helper).'''

    def __init__(self, rows):
        self.files = {}
        self.rows = rows

    def add(self, rel, data):
        self.files[rel] = data.encode('utf-8') if isinstance(data, str) else data

    def step(self, case, number, stdin, result, channel, before=None, after=None, fault=None):
        base = JOURNEY + '/' + case
        tag = '%03d' % number
        self.add(base + '/' + tag + '.stdin',
                 stdin if isinstance(stdin, bytes) else json.dumps(stdin))
        self.add(base + '/' + tag + '.result.json', json.dumps(result))
        self.add(base + '/' + tag + '.process.json', json.dumps(proc(channel, fault)))
        if before is not None:
            self.add(base + '/before_' + tag + '.observation.json', json.dumps(before))
        if after is not None:
            self.add(base + '/after_' + tag + '.observation.json', json.dumps(after))

    def write(self, root):
        loc = os.path.join(root, 'location')
        self.files['result.json'] = json.dumps({'rows': self.rows}).encode('utf-8')
        for rel, data in sorted(self.files.items()):
            dest = os.path.join(loc, rel)
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            with open(dest, 'wb') as handle:
                handle.write(data)
        entries = [{'path': rel, 'sha256': hashlib.sha256(data).hexdigest()}
                   for rel, data in sorted(self.files.items())]
        payload = json.dumps(entries, sort_keys=True, separators=(',', ':'), ensure_ascii=False)
        index = {'location': loc,
                 'inventory_sha256': hashlib.sha256(payload.encode('utf-8')).hexdigest(),
                 'inventory': entries,
                 'journey_path': JOURNEY,
                 'result_path': 'result.json'}
        index_path = os.path.join(root, 'A0-PREUVES.json')
        with open(index_path, 'w', encoding='utf-8') as handle:
            json.dump(index, handle)
        return index_path


def approval_flow(tree, case, action):
    '''Steps 001-003 (initialize, present, approve); returns reusable snapshots.'''
    text = canon(action)
    empty = observation(CONFIG, [], [])
    approved = observation(CONFIG, [approval_row('approved', text)], [])
    tree.step(case, 1, {'op': 'initialize', 'configuration': CONFIG},
              {'status': 'initialized', 'approval_id': None}, 'C', after=empty)
    tree.step(case, 2, {'op': 'present', 'action': action},
              {'status': 'presented', 'approval_id': None}, 'D', before=empty, after=empty)
    tree.step(case, 3, {'op': 'approve', 'approval_id': 'approval', 'action': action,
                        'presentation': text},
              {'status': 'approved', 'approval_id': 'approval', 'digest': action_digest(text)},
              'V', before=empty, after=approved)
    return {'canon': text, 'empty': empty, 'approved': approved,
            'consumed': observation(CONFIG, [approval_row('consumed', text)], [[1, 'approval', text]])}


def run_case(root, tree, out_name='out'):
    index = tree.write(root)
    out = os.path.join(root, out_name)
    report = oracle.run_oracle(index, out)
    return report, out


class V06HistoryTest(unittest.TestCase):
    '''E1-V06 shape: commit at 004, refused approval_consumed at 005, same intent.'''

    def test_pv2_intention_fail_and_pv2_attempt_pass(self):
        with tempfile.TemporaryDirectory() as root:
            tree = Tree(rows=[{'case': 'E1-V06t', 'result': 'PASS'}])
            snapshots = approval_flow(tree, 'E1-V06t', make_action('alpha'))
            apply_request = {'op': 'apply', 'approval_id': 'approval', 'action': make_action('alpha')}
            tree.step('E1-V06t', 4, apply_request,
                      {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                       'digest': action_digest(snapshots['canon'])}, 'D',
                      before=snapshots['approved'], after=snapshots['consumed'])
            tree.step('E1-V06t', 5, apply_request,
                      {'status': 'refused', 'approval_id': 'approval', 'reason': 'approval_consumed'},
                      'D', before=snapshots['consumed'], after=snapshots['consumed'])
            report, out = run_case(root, tree)
            props = report['rows'][0]['properties']
            self.assertEqual(props['PV2_intention']['verdict'], 'fail')
            witness = props['PV2_intention']['witnesses'][0]
            self.assertEqual(witness['past_commit']['step'], 4)
            self.assertEqual(witness['intent'], 'invoice')
            self.assertEqual(props['PV2_attempt']['verdict'], 'pass')
            self.assertEqual(props['PV1_intention']['verdict'], 'pass')
            self.assertEqual(props['PV1_attempt']['verdict'], 'pass')
            self.assertEqual(props['PT']['verdict'], 'pass')
            self.assertEqual(props['PA']['verdict'], 'pass')
            self.assertEqual(props['PR']['verdict'], 'pass')
            self.assertEqual(props['PV2']['verdict'], 'inconc')
            self.assertEqual(props['PL']['verdict'], 'pass')
            self.assertEqual(props['PL']['coverage'], 'vacuous')
            with open(os.path.join(out, 'a0_oracle_table.md'), encoding='utf-8') as handle:
                table = handle.read()
            self.assertIn('E1-V06t', table)
            self.assertIn('FAIL', table)


class BeforeCommitRecoveryTest(unittest.TestCase):
    '''before_commit refusal (no effect) then successful retry: past-prefix only.'''

    def test_no_retroactive_pv2_fail(self):
        with tempfile.TemporaryDirectory() as root:
            tree = Tree(rows=[{'case': 'E1-Qbt', 'result': 'PASS'}])
            snapshots = approval_flow(tree, 'E1-Qbt', make_action('alpha'))
            apply_request = {'op': 'apply', 'approval_id': 'approval', 'action': make_action('alpha')}
            tree.step('E1-Qbt', 4, apply_request,
                      {'status': 'refused', 'approval_id': 'approval', 'reason': 'storage_error'},
                      'D', before=snapshots['approved'], after=snapshots['approved'],
                      fault='before_commit')
            tree.step('E1-Qbt', 5, apply_request,
                      {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                       'digest': action_digest(snapshots['canon'])}, 'D',
                      before=snapshots['approved'], after=snapshots['consumed'])
            report, _out = run_case(root, tree)
            props = report['rows'][0]['properties']
            self.assertEqual(props['PV2_intention']['verdict'], 'pass')
            self.assertEqual(props['PV2_intention']['coverage'], 'triggered')
            self.assertEqual(props['PV2_attempt']['verdict'], 'pass')
            self.assertEqual(props['PV1_intention']['verdict'], 'pass')
            self.assertEqual(props['PV1_attempt']['verdict'], 'pass')
            self.assertEqual(props['PT']['verdict'], 'pass')
            for entry in props.values():
                self.assertEqual(entry['witnesses'], [])


class UnknownAfterCommitTest(unittest.TestCase):
    '''after_commit fault: response unknown, effect persisted at the same step.'''

    def test_unknown_triggers_neither_pv1_nor_pv2(self):
        with tempfile.TemporaryDirectory() as root:
            tree = Tree(rows=[{'case': 'E1-Qat', 'result': 'PASS'}])
            snapshots = approval_flow(tree, 'E1-Qat', make_action('alpha'))
            tree.step('E1-Qat', 4,
                      {'op': 'apply', 'approval_id': 'approval', 'action': make_action('alpha')},
                      {'status': 'unknown', 'approval_id': 'approval', 'reason': 'storage_error'},
                      'D', before=snapshots['approved'], after=snapshots['consumed'],
                      fault='after_commit')
            report, _out = run_case(root, tree)
            row = report['rows'][0]
            props = row['properties']
            for name in ('PV1_intention', 'PV1_attempt', 'PV2_intention', 'PV2_attempt'):
                self.assertEqual(props[name]['verdict'], 'pass')
                self.assertEqual(props[name]['coverage'], 'vacuous')
            self.assertEqual(props['PT']['verdict'], 'pass')
            self.assertEqual(props['PA']['verdict'], 'pass')
            self.assertEqual(props['PR']['verdict'], 'pass')
            self.assertEqual(props['PV1']['verdict'], 'inconc')
            self.assertTrue(any('unknown' in note for note in row['case_notes']))


class WeakTermsTest(unittest.TestCase):
    '''E1-W_sensitivity shape: approval binds alpha, effect commits beta.'''

    def test_pt_fail_with_witness_and_fail_preservation(self):
        with tempfile.TemporaryDirectory() as root:
            alpha = make_action('alpha')
            beta = make_action('beta')
            tree = Tree(rows=[{'case': 'E1-Wt', 'result': 'EXPECTED_COUNTEREXAMPLE'}])
            snapshots = approval_flow(tree, 'E1-Wt', alpha)
            consumed_beta = observation(CONFIG, [approval_row('consumed', canon(alpha))],
                                        [[1, 'approval', canon(beta)]])
            tree.step('E1-Wt', 4, {'op': 'apply', 'approval_id': 'approval', 'action': beta},
                      {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                       'digest': action_digest(canon(beta))}, 'D',
                      before=snapshots['approved'], after=consumed_beta)
            tree.step('E1-Wt', 5, {'op': 'apply', 'approval_id': 'approval', 'action': beta},
                      {'status': 'refused', 'approval_id': 'approval', 'reason': 'approval_consumed'},
                      'D', before=consumed_beta, after=None)
            report, _out = run_case(root, tree)
            props = report['rows'][0]['properties']
            self.assertEqual(props['PT']['verdict'], 'fail')
            witness = props['PT']['witnesses'][0]
            self.assertIn('"beneficiary_account":"beta"', witness['committed_action'])
            self.assertTrue(any('"beneficiary_account":"alpha"' in variant
                                for variant in witness['approved_action_variants']))
            self.assertEqual(props['PV1_attempt']['verdict'], 'pass')
            self.assertEqual(props['PV2_intention']['verdict'], 'fail')
            self.assertEqual(props['PV1_intention']['verdict'], 'pass')
            self.assertEqual(props['PV2_attempt']['verdict'], 'inconc')


class MissingObservationTest(unittest.TestCase):
    '''Committed apply whose after-snapshot is absent: trace holes, not fails.'''

    def test_missing_after_yields_inconc(self):
        with tempfile.TemporaryDirectory() as root:
            tree = Tree(rows=[{'case': 'E1-Mt', 'result': 'PASS'}])
            snapshots = approval_flow(tree, 'E1-Mt', make_action('alpha'))
            tree.step('E1-Mt', 4,
                      {'op': 'apply', 'approval_id': 'approval', 'action': make_action('alpha')},
                      {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                       'digest': action_digest(snapshots['canon'])}, 'D',
                      before=snapshots['approved'], after=None)
            report, _out = run_case(root, tree)
            row = report['rows'][0]
            props = row['properties']
            for name in ('PT', 'PA', 'PR', 'PV1_intention', 'PV1_attempt',
                         'PV2_intention', 'PV2_attempt'):
                self.assertEqual(props[name]['verdict'], 'inconc')
            self.assertEqual(props['PL']['verdict'], 'pass')
            self.assertTrue(any('4:after' in note for note in row['case_notes']))


class InvalidStdinTest(unittest.TestCase):
    '''V08_duplicate shape: unparseable stdin refusal is not attributed to PV2.'''

    def test_refusal_not_attributed(self):
        with tempfile.TemporaryDirectory() as root:
            tree = Tree(rows=[{'case': 'E1-Dt', 'result': 'PASS'}])
            snapshots = approval_flow(tree, 'E1-Dt', make_action('alpha'))
            raw = ('{"op": "apply", "approval_id": "approval", "action": ' + snapshots['canon'] +
                   ', "amount_minor": "1200", "amount_\\u006dinor": "1300"}').encode('utf-8')
            tree.step('E1-Dt', 4, raw,
                      {'status': 'refused', 'reason': 'input_invalid', 'approval_id': None},
                      'D', before=snapshots['approved'], after=snapshots['approved'])
            report, _out = run_case(root, tree)
            row = report['rows'][0]
            props = row['properties']
            self.assertEqual(props['PV2_intention']['verdict'], 'inconc')
            self.assertEqual(props['PV2_intention']['coverage'], 'triggered')
            self.assertEqual(props['PV2_attempt']['verdict'], 'inconc')
            self.assertEqual(props['PT']['verdict'], 'pass')
            self.assertEqual(props['PT']['coverage'], 'vacuous')
            self.assertTrue(any('not attributed' in note for note in row['case_notes']))
            step4 = row['trace']['steps'][3]
            self.assertIsNotNone(step4['request_error'])


class WithdrawalTest(unittest.TestCase):
    '''E1-V05_revoke shape: revoke at 004, refused apply at 005; PL inconc.'''

    def test_pl_inconc_withdrawal_and_pv2_pass(self):
        with tempfile.TemporaryDirectory() as root:
            tree = Tree(rows=[{'case': 'E1-Rt', 'result': 'PASS'}])
            snapshots = approval_flow(tree, 'E1-Rt', make_action('alpha'))
            revoked = observation(CONFIG, [approval_row('revoked', canon(make_action('alpha')))], [])
            tree.step('E1-Rt', 4, {'op': 'revoke', 'approval_id': 'approval'},
                      {'status': 'updated', 'approval_id': 'approval'}, 'V',
                      before=snapshots['approved'], after=revoked)
            tree.step('E1-Rt', 5,
                      {'op': 'apply', 'approval_id': 'approval', 'action': make_action('alpha')},
                      {'status': 'refused', 'approval_id': 'approval', 'reason': 'approval_revoked'},
                      'D', before=revoked, after=revoked)
            report, _out = run_case(root, tree)
            props = report['rows'][0]['properties']
            self.assertEqual(props['PL']['verdict'], 'inconc')
            self.assertEqual(props['PL']['coverage'], 'triggered')
            self.assertEqual(props['PV2_intention']['verdict'], 'pass')
            self.assertEqual(props['PV2_attempt']['verdict'], 'pass')
            self.assertEqual(props['PT']['verdict'], 'pass')
            self.assertEqual(props['PT']['coverage'], 'vacuous')


class ShapeErrorTest(unittest.TestCase):
    '''Missing required evidence yields error, never PASS; absent case dirs too.'''

    def test_missing_result_file_is_error(self):
        with tempfile.TemporaryDirectory() as root:
            tree = Tree(rows=[{'case': 'E1-Et', 'result': 'PASS'}])
            approval_flow(tree, 'E1-Et', make_action('alpha'))
            base = JOURNEY + '/E1-Et'
            tree.add(base + '/004.stdin',
                     json.dumps({'op': 'apply', 'approval_id': 'approval', 'action': make_action('alpha')}))
            tree.add(base + '/004.process.json', json.dumps(proc('D')))
            report, _out = run_case(root, tree)
            props = report['rows'][0]['properties']
            self.assertEqual(props['PT']['verdict'], 'error')
            self.assertEqual(props['PL']['verdict'], 'error')
            self.assertNotEqual(props['PA']['verdict'], 'pass')

    def test_unattributed_row_is_inconclusive(self):
        with tempfile.TemporaryDirectory() as root:
            alpha_text = canon(make_action('alpha'))
            tree = Tree(rows=[{'case': 'E1-OK', 'result': 'PASS'},
                              {'case': 'E1-ABSENT', 'result': 'PASS'}])
            snapshots = approval_flow(tree, 'E1-OK', make_action('alpha'))
            tree.step('E1-OK', 4,
                      {'op': 'apply', 'approval_id': 'approval', 'action': make_action('alpha')},
                      {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                       'digest': action_digest(alpha_text)}, 'D',
                      before=snapshots['approved'], after=snapshots['consumed'])
            report, _out = run_case(root, tree)
            self.assertEqual(len(report['rows']), 2)
            self.assertEqual(report['rows'][0]['properties']['PT']['verdict'], 'pass')
            self.assertEqual(report['rows'][1]['properties']['PT']['verdict'], 'inconc')
            self.assertTrue(any('case_directory_not_found' in note
                                for note in report['rows'][1]['case_notes']))


class RefusalTest(unittest.TestCase):
    '''Validation refusals: safe diagnostics, no output, no source mutation.'''

    def prepare(self, root):
        tree = Tree(rows=[{'case': 'E1-Ct', 'result': 'PASS'}])
        snapshots = approval_flow(tree, 'E1-Ct', make_action('alpha'))
        tree.step('E1-Ct', 4,
                  {'op': 'apply', 'approval_id': 'approval', 'action': make_action('alpha')},
                  {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                   'digest': action_digest(snapshots['canon'])}, 'D',
                  before=snapshots['approved'], after=snapshots['consumed'])
        index = tree.write(root)
        control = oracle.run_oracle(index, os.path.join(root, 'healthy-control'))
        self.assertEqual(control['rows'][0]['properties']['PT']['verdict'], 'pass')
        return index

    def test_digest_mismatch_refuses_without_output(self):
        with tempfile.TemporaryDirectory() as root:
            index = self.prepare(root)
            victim = os.path.join(root, 'location', JOURNEY, 'E1-Ct', '003.stdin')
            data = bytearray(Path(victim).read_bytes())
            data[0] = data[0] ^ 1
            with open(victim, 'wb') as handle:
                handle.write(bytes(data))
            out = os.path.join(root, 'out')
            with self.assertRaises(oracle.OracleError) as context:
                oracle.run_oracle(index, out)
            self.assertEqual(context.exception.reason, 'file_digest_mismatch')
            self.assertFalse(os.path.exists(out))

    def test_traversal_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            index = self.prepare(root)
            with open(index, encoding='utf-8') as handle:
                payload = json.load(handle)
            payload['inventory'].append({'path': '../evil.txt', 'sha256': '0' * 64})
            if 'inventory' in payload:
                payload['inventory_sha256'] = hashlib.sha256(canon(payload['inventory']).encode()).hexdigest()
            with open(index, 'w', encoding='utf-8') as handle:
                json.dump(payload, handle)
            out = os.path.join(root, 'out')
            with self.assertRaises(oracle.OracleError) as context:
                oracle.run_oracle(index, out)
            self.assertEqual(context.exception.reason, 'path_traversal')
            self.assertFalse(os.path.exists(out))

    @unittest.skipUnless(hasattr(os, 'symlink'), 'symlinks unavailable on this platform')
    def test_symlink_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            index = self.prepare(root)
            target = os.path.join(root, 'location', JOURNEY, 'E1-Ct', '003.stdin')
            link = os.path.join(root, 'location', JOURNEY, 'E1-Ct', 'link.stdin')
            os.symlink(target, link)
            with open(index, encoding='utf-8') as handle:
                payload = json.load(handle)
            with open(target, 'rb') as handle:
                digest = hashlib.sha256(handle.read()).hexdigest()
            payload['inventory'].append({'path': JOURNEY + '/E1-Ct/link.stdin', 'sha256': digest})
            if 'inventory' in payload:
                payload['inventory_sha256'] = hashlib.sha256(canon(payload['inventory']).encode()).hexdigest()
            with open(index, 'w', encoding='utf-8') as handle:
                json.dump(payload, handle)
            out = os.path.join(root, 'out')
            with self.assertRaises(oracle.OracleError) as context:
                oracle.run_oracle(index, out)
            self.assertEqual(context.exception.reason, 'symlink_rejected')
            self.assertFalse(os.path.exists(out))

    def test_existing_output_and_no_source_mutation(self):
        with tempfile.TemporaryDirectory() as root:
            index = self.prepare(root)
            out = os.path.join(root, 'out')
            os.makedirs(out)
            before = tree_hashes(root)
            with self.assertRaises(oracle.OracleError) as context:
                oracle.run_oracle(index, out)
            self.assertEqual(context.exception.reason, 'output_exists')
            self.assertEqual(tree_hashes(root), before)

    def test_corrupt_result_json_refused(self):
        with tempfile.TemporaryDirectory() as root:
            index = self.prepare(root)
            result_path = os.path.join(root, 'location', 'result.json')
            with open(result_path, 'wb') as handle:
                handle.write(b'{oops')
            with open(index, encoding='utf-8') as handle:
                payload = json.load(handle)
            for entry in payload['inventory']:
                if entry['path'] == 'result.json':
                    entry['sha256'] = hashlib.sha256(b'{oops').hexdigest()
            payload['inventory_sha256'] = hashlib.sha256(canon(payload['inventory']).encode()).hexdigest()
            with open(index, 'w', encoding='utf-8') as handle:
                json.dump(payload, handle)
            out = os.path.join(root, 'out')
            with self.assertRaises(oracle.OracleError) as context:
                oracle.run_oracle(index, out)
            self.assertEqual(context.exception.reason, 'result_corrupt_json')
            self.assertFalse(os.path.exists(out))

    def test_index_without_manifest_refused(self):
        with tempfile.TemporaryDirectory() as root:
            index = self.prepare(root)
            with open(index, encoding='utf-8') as handle:
                payload = json.load(handle)
            payload.pop('inventory')
            if 'inventory' in payload:
                payload['inventory_sha256'] = hashlib.sha256(canon(payload['inventory']).encode()).hexdigest()
            with open(index, 'w', encoding='utf-8') as handle:
                json.dump(payload, handle)
            out = os.path.join(root, 'out')
            with self.assertRaises(oracle.OracleError) as context:
                oracle.run_oracle(index, out)
            self.assertEqual(context.exception.reason, 'inventory_manifest_missing')
            self.assertFalse(os.path.exists(out))


class ReportShapeTest(unittest.TestCase):
    '''Report/table shape, source read-onlyness, echoed assumptions.'''

    def test_report_and_table_shape_and_readonly_source(self):
        with tempfile.TemporaryDirectory() as root:
            tree = Tree(rows=[{'case': 'E1-S1', 'result': 'PASS'},
                              {'case': 'E1-S2', 'result': 'PASS'}])
            for case in ('E1-S1', 'E1-S2'):
                snapshots = approval_flow(tree, case, make_action('alpha'))
                tree.step(case, 4,
                          {'op': 'apply', 'approval_id': 'approval', 'action': make_action('alpha')},
                          {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                           'digest': action_digest(snapshots['canon'])}, 'D',
                          before=snapshots['approved'], after=snapshots['consumed'])
            index = tree.write(root)
            before = tree_hashes(root)
            out = os.path.join(root, 'out')
            report = oracle.run_oracle(index, out)
            self.assertEqual(len(report['rows']), 2)
            self.assertGreater(report['source']['inventory_entries_validated'], 0)
            self.assertEqual(tree_hashes(root), before)
            with open(os.path.join(out, 'a0_oracle_report.json'), encoding='utf-8') as handle:
                persisted = json.load(handle)
            self.assertEqual(len(persisted['rows']), 2)
            self.assertTrue(persisted['assumptions'])
            with open(os.path.join(out, 'a0_oracle_table.md'), encoding='utf-8') as handle:
                table = handle.read()
            for case in ('E1-S1', 'E1-S2'):
                self.assertIn(case, table)
            self.assertIn('## Counts', table)




class AdditionalSemanticGuards(unittest.TestCase):
    """Discriminating guards for the retained layout and temporal formulas."""

    def committed(self, root, *, extra_effect=False):
        """Build and evaluate an isolated one-commit control or consumption fault."""
        tree = Tree(rows=[{'case':'control', 'result':'PASS'}])
        action = make_action('alpha')
        states = approval_flow(tree, 'control', action)
        after = states['consumed']
        if extra_effect:
            after = observation(CONFIG, [approval_row('consumed', canon(action))],
                                [[1,'approval',canon(action)], [2,'approval',canon(action)]])
        tree.step('control', 4, {'op':'apply','approval_id':'approval','action':action},
                  {'status':'committed','approval_id':'approval','effect_id':1,
                   'digest':action_digest(canon(action))}, 'D', before=states['approved'], after=after)
        return run_case(root, tree)[0]

    def test_two_same_grant_commitments_cannot_pass_consumption(self):
        with tempfile.TemporaryDirectory() as root:
            control = self.committed(os.path.join(root,'control'))
            self.assertEqual(control['rows'][0]['properties']['PA']['verdict'], 'pass')
            fault = self.committed(os.path.join(root,'fault'), extra_effect=True)
            self.assertEqual(fault['rows'][0]['properties']['PA']['verdict'], 'fail')
            self.assertTrue(fault['rows'][0]['properties']['PA']['witnesses'])

    def test_effect_receipt_terms_mismatch_does_not_falsify_pv1(self):
        with tempfile.TemporaryDirectory() as root:
            self.assertEqual(self.committed(os.path.join(root,'control'))['rows'][0]['properties']['PV1_attempt']['verdict'], 'pass')
            tree = Tree(rows=[{'case':'weak','result':'EXPECTED_COUNTEREXAMPLE'}])
            alpha, beta = make_action('alpha'), make_action('beta')
            states = approval_flow(tree,'weak',alpha)
            after = observation(CONFIG,[approval_row('consumed',canon(alpha))],[[1,'approval',canon(beta)]])
            tree.step('weak',4,{'op':'apply','approval_id':'approval','action':beta},
                      {'status':'committed','approval_id':'approval','effect_id':1,'digest':action_digest(canon(beta))},
                      'D',before=states['approved'],after=after)
            report,_ = run_case(os.path.join(root,'fault'),tree)
            props = report['rows'][0]['properties']
            self.assertEqual(props['PT']['verdict'],'fail')
            self.assertEqual(props['PV1_attempt']['verdict'],'pass')
            self.assertEqual(props['PV1_intention']['verdict'],'pass')

    def test_revocation_history_survives_later_row_reset(self):
        with tempfile.TemporaryDirectory() as root:
            self.assertEqual(self.committed(os.path.join(root,'control'))['rows'][0]['properties']['PA']['verdict'],'pass')
            tree = Tree(rows=[{'case':'restored','result':'DIAGNOSTIC_FIXTURE'}])
            action = make_action('alpha');states=approval_flow(tree,'restored',action)
            revoked=observation(CONFIG,[approval_row('revoked',canon(action))],[])
            tree.step('restored',4,{'op':'revoke','approval_id':'approval'},
                      {'status':'updated','approval_id':'approval'},'V',before=states['approved'],after=revoked)
            # Deliberately contradictory restored state is synthetic evidence,
            # never a mutation of a real receiver/database or proof of A0 restore.
            tree.step('restored',5,{'op':'apply','approval_id':'approval','action':action},
                      {'status':'committed','approval_id':'approval','effect_id':1,'digest':action_digest(canon(action))},
                      'D',before=states['approved'],after=states['consumed'])
            report,_=run_case(os.path.join(root,'fault'),tree);props=report['rows'][0]['properties']
            self.assertEqual(props['PR']['verdict'],'fail')
            self.assertEqual(props['PA']['verdict'],'fail')

    def test_sparse_readonly_steps_do_not_erase_positive_witness(self):
        with tempfile.TemporaryDirectory() as root:
            tree=Tree(rows=[{'case':'sparse','result':'PASS'}]);action=make_action('alpha')
            states=approval_flow(tree,'sparse',action)
            for rel in list(tree.files):
                if any(rel.endswith('/'+side+'_'+number+'.observation.json')
                       for side in ('before','after') for number in ('001','002','003')):
                    del tree.files[rel]
            tree.add(JOURNEY+'/sparse/initial.observation.json',json.dumps(observation(CONFIG,[],[])))
            tree.step('sparse',4,{'op':'apply','approval_id':'approval','action':action},
                      {'status':'committed','approval_id':'approval','effect_id':1,'digest':action_digest(canon(action))},
                      'D',before=states['approved'],after=states['consumed'])
            report,_=run_case(root,tree);props=report['rows'][0]['properties']
            for name in ('PT','PA','PR','PV1_intention','PV1_attempt'):
                self.assertEqual(props[name]['verdict'],'pass',name)

    def test_actual_index_dictionary_manifest_and_absolute_journey(self):
        with tempfile.TemporaryDirectory() as root:
            tree=Tree(rows=[{'case':'layout','result':'PASS'}]);action=make_action('alpha')
            states=approval_flow(tree,'layout',action)
            tree.step('layout',4,{'op':'apply','approval_id':'approval','action':action},
                      {'status':'committed','approval_id':'approval','effect_id':1,'digest':action_digest(canon(action))},
                      'D',before=states['approved'],after=states['consumed'])
            index=tree.write(root);location=Path(root)/'location'
            (location/JOURNEY/'result.json').write_bytes((location/'result.json').read_bytes())
            (location/'result.json').unlink()
            manifest={str(path.relative_to(location)):hashlib.sha256(path.read_bytes()).hexdigest()
                      for path in location.rglob('*') if path.is_file()}
            raw=json.dumps(manifest,indent=2,sort_keys=True).encode()
            (location/'A0-INVENTORY.json').write_bytes(raw)
            Path(index).write_text(json.dumps({'location':str(location),'journey':str(location/JOURNEY),
                                               'inventory_sha256':hashlib.sha256(raw).hexdigest()}))
            report=oracle.run_oracle(index,str(Path(root)/'out'))
            self.assertEqual(report['rows'][0]['properties']['PT']['verdict'],'pass')
            self.assertTrue((Path(root)/'out/COMPLETE.json').is_file())


class OutputFailureGuards(unittest.TestCase):
    """Verify exclusive publication, retained IO cause, and incomplete-marker semantics."""
    def test_publication_failure_preserves_existing_and_partial_output(self):
        """Healthy publication precedes injected ENOSPC; retry uses a fresh path."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            oracle.write_output(str(root / 'healthy'), {'healthy': True}, 'healthy')
            original = (root / 'healthy' / oracle.REPORT_NAME).read_bytes()
            with self.assertRaises(oracle.OracleError):
                oracle.write_output(str(root / 'healthy'), {}, 'replacement')
            self.assertEqual((root / 'healthy' / oracle.REPORT_NAME).read_bytes(), original)
            failure = OSError(errno.ENOSPC, 'synthetic no space')
            with mock.patch.object(oracle.os, 'link', side_effect=failure):
                with self.assertRaises(oracle.OracleError) as caught:
                    oracle.write_output(str(root / 'partial'), {}, 'partial')
            self.assertIs(caught.exception.__cause__, failure)
            self.assertFalse((root / 'partial' / 'COMPLETE.json').exists())
            self.assertTrue(list((root / 'partial').iterdir()))
            oracle.write_output(str(root / 'recovered'), {}, 'recovered')
            self.assertTrue((root / 'recovered' / 'COMPLETE.json').exists())


class EffectPersistenceConflictTest(unittest.TestCase):
    '''Defect 1: an effect row that disappears, or is rewritten under the same
    identifier, between two snapshots is a persistence inconsistency. It must
    yield error/inconc, never a merely annotated pass, and no new commit may be
    invented from a same-id rewrite. Healthy control first; fresh outputs.'''

    def _committed(self, root, name):
        tree = Tree(rows=[{'case': name, 'result': 'PASS'}])
        action = make_action('alpha')
        states = approval_flow(tree, name, action)
        tree.step(name, 4, {'op': 'apply', 'approval_id': 'approval', 'action': action},
                  {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                   'digest': action_digest(canon(action))}, 'D',
                  before=states['approved'], after=states['consumed'])
        return run_case(root, tree)[0]

    def test_disappearing_effect_row_is_error_not_pass(self):
        with tempfile.TemporaryDirectory() as root:
            control = self._committed(os.path.join(root, 'control'), 'persist-control')
            self.assertEqual(control['rows'][0]['properties']['PT']['verdict'], 'pass')
            self.assertEqual(control['rows'][0]['properties']['PA']['verdict'], 'pass')
            tree = Tree(rows=[{'case': 'persist-fault', 'result': 'PASS'}])
            action = make_action('alpha')
            states = approval_flow(tree, 'persist-fault', action)
            tree.step('persist-fault', 4, {'op': 'apply', 'approval_id': 'approval', 'action': action},
                      {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                       'digest': action_digest(canon(action))}, 'D',
                      before=states['approved'], after=states['consumed'])
            vanished = observation(CONFIG, [approval_row('consumed', canon(action))], [])
            tree.step('persist-fault', 5, {'op': 'present', 'action': action},
                      {'status': 'presented', 'approval_id': None}, 'D',
                      before=states['consumed'], after=vanished)
            report, _out = run_case(os.path.join(root, 'fault'), tree)
            row = report['rows'][0]
            props = row['properties']
            self.assertEqual(props['PT']['verdict'], 'error')
            self.assertEqual(props['PA']['verdict'], 'error')
            self.assertTrue(row['trace']['persistence_conflicts'])
            self.assertEqual(len(row['trace']['commits']), 1)
            self.assertTrue(any('persistence' in note for note in row['case_notes']))

    def test_same_id_effect_rewrite_adds_no_commit_and_flags_inconsistency(self):
        with tempfile.TemporaryDirectory() as root:
            control = self._committed(os.path.join(root, 'control'), 'rewrite-control')
            self.assertEqual(control['rows'][0]['properties']['PT']['verdict'], 'pass')
            tree = Tree(rows=[{'case': 'rewrite-fault', 'result': 'PASS'}])
            action = make_action('alpha')
            states = approval_flow(tree, 'rewrite-fault', action)
            tree.step('rewrite-fault', 4, {'op': 'apply', 'approval_id': 'approval', 'action': action},
                      {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                       'digest': action_digest(canon(action))}, 'D',
                      before=states['approved'], after=states['consumed'])
            rewritten = observation(CONFIG, [approval_row('consumed', canon(action))],
                                    [[1, 'approval', canon(make_action('beta'))]])
            tree.step('rewrite-fault', 5, {'op': 'present', 'action': action},
                      {'status': 'presented', 'approval_id': None}, 'D',
                      before=states['consumed'], after=rewritten)
            report, _out = run_case(os.path.join(root, 'fault'), tree)
            row = report['rows'][0]
            props = row['properties']
            self.assertEqual(len(row['trace']['commits']), 1)
            self.assertEqual(props['PT']['verdict'], 'error')
            self.assertEqual(props['PV1_attempt']['verdict'], 'error')
            self.assertTrue(row['trace']['persistence_conflicts'])

class CaseRunBindingTest(unittest.TestCase):
    '''Defect 2: every snapshot of a case must bind one case-global run; a
    mismatch between snapshots is an evidence error and no effect diff (hence
    no commit) is derived across runs. Healthy control first; fresh outputs.'''

    def test_single_run_control_passes_and_mixed_run_case_is_error(self):
        with tempfile.TemporaryDirectory() as root:
            action = make_action('alpha')
            tree = Tree(rows=[{'case': 'runctl', 'result': 'PASS'}])
            states = approval_flow(tree, 'runctl', action)
            tree.step('runctl', 4, {'op': 'apply', 'approval_id': 'approval', 'action': action},
                      {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                       'digest': action_digest(canon(action))}, 'D',
                      before=states['approved'], after=states['consumed'])
            control, _out = run_case(os.path.join(root, 'control'), tree)
            self.assertEqual(control['rows'][0]['properties']['PT']['verdict'], 'pass')
            self.assertEqual(len(control['rows'][0]['trace']['commits']), 1)
            other_config = dict(CONFIG)
            other_config['case_run'] = 'run_test_2'
            tree2 = Tree(rows=[{'case': 'runfault', 'result': 'PASS'}])
            states2 = approval_flow(tree2, 'runfault', action)
            drifted = observation(other_config, [approval_row('consumed', canon(action))],
                                  [[1, 'approval', canon(action)]])
            tree2.step('runfault', 4, {'op': 'apply', 'approval_id': 'approval', 'action': action},
                       {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                        'digest': action_digest(canon(action))}, 'D',
                       before=states2['approved'], after=drifted)
            report2, _out2 = run_case(os.path.join(root, 'fault'), tree2)
            row = report2['rows'][0]
            props = row['properties']
            self.assertEqual(props['PT']['verdict'], 'error')
            self.assertTrue(any('case_run_mismatch' in note for note in row['case_notes']))
            self.assertEqual(row['trace'].get('commits', []), [])

class PTPrefixEvidenceTest(unittest.TestCase):
    '''Defect 3: PT may conclude failure (absence of an approval) only when the
    prefix before the commit is completely observed; an incomplete prefix yields
    inconc, a full prefix keeps the finite fail, and a found exact approval
    witness is retained even with an unrelated later hole. Healthy control first.'''

    def test_prefix_completeness_governs_pt_fail_vs_inconc(self):
        with tempfile.TemporaryDirectory() as root:
            action = make_action('alpha')
            empty = observation(CONFIG, [], [])
            effect = observation(CONFIG, [], [[1, 'approval', canon(action)]])
            tree = Tree(rows=[{'case': 'ptctl', 'result': 'PASS'}])
            states = approval_flow(tree, 'ptctl', action)
            tree.step('ptctl', 4, {'op': 'apply', 'approval_id': 'approval', 'action': action},
                      {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                       'digest': action_digest(canon(action))}, 'D',
                      before=states['approved'], after=states['consumed'])
            control, _out = run_case(os.path.join(root, 'control'), tree)
            self.assertEqual(control['rows'][0]['properties']['PT']['verdict'], 'pass')
            tree2 = Tree(rows=[{'case': 'ptprefix', 'result': 'PASS'}])
            tree2.step('ptprefix', 1, {'op': 'initialize', 'configuration': CONFIG},
                       {'status': 'initialized', 'approval_id': None}, 'C')
            tree2.step('ptprefix', 2, {'op': 'present', 'action': action},
                       {'status': 'presented', 'approval_id': None}, 'D')
            tree2.step('ptprefix', 3, {'op': 'apply', 'approval_id': 'approval', 'action': action},
                       {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                        'digest': action_digest(canon(action))}, 'D',
                       before=empty, after=effect)
            fault, _out2 = run_case(os.path.join(root, 'fault'), tree2)
            props = fault['rows'][0]['properties']
            self.assertEqual(props['PT']['verdict'], 'inconc')
            self.assertEqual(props['PT']['witnesses'], [])
            tree3 = Tree(rows=[{'case': 'ptfail', 'result': 'EXPECTED_COUNTEREXAMPLE'}])
            tree3.step('ptfail', 1, {'op': 'initialize', 'configuration': CONFIG},
                       {'status': 'initialized', 'approval_id': None}, 'C', after=empty)
            tree3.step('ptfail', 2, {'op': 'present', 'action': action},
                       {'status': 'presented', 'approval_id': None}, 'D', before=empty, after=empty)
            tree3.step('ptfail', 3, {'op': 'apply', 'approval_id': 'approval', 'action': action},
                       {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                        'digest': action_digest(canon(action))}, 'D',
                       before=empty, after=effect)
            full, _out3 = run_case(os.path.join(root, 'full'), tree3)
            props = full['rows'][0]['properties']
            self.assertEqual(props['PT']['verdict'], 'fail')
            self.assertTrue(props['PT']['witnesses'])
            tree4 = Tree(rows=[{'case': 'ptsurvive', 'result': 'PASS'}])
            states4 = approval_flow(tree4, 'ptsurvive', action)
            tree4.step('ptsurvive', 4, {'op': 'apply', 'approval_id': 'approval', 'action': action},
                       {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                        'digest': action_digest(canon(action))}, 'D',
                       before=states4['approved'], after=states4['consumed'])
            tree4.step('ptsurvive', 5, {'op': 'policy', 'revision': 1},
                       {'status': 'updated', 'approval_id': None}, 'V',
                       before=states4['consumed'])
            survive, _out4 = run_case(os.path.join(root, 'survive'), tree4)
            props = survive['rows'][0]['properties']
            self.assertTrue(any(instance['verdict'] == 'pass'
                                for instance in props['PT']['instances']))

class FinalDirFsyncUncertaintyTest(unittest.TestCase):
    '''Defect 4: a final directory fsync failure after the marker link leaves
    the marker in place; publication is reported uncertain with the original
    OSError, the marker and partial artifacts are retained (no rollback), and
    marker presence alone never proves a successful run. Healthy control first,
    fresh recovery after.'''

    def test_control_succeeds_then_injected_dir_fsync_failure_is_uncertain(self):
        import stat as stat_module
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            oracle.write_output(str(root / 'control'), {'k': 0}, 'table')
            marker = json.loads((root / 'control' / 'COMPLETE.json').read_text(encoding='utf-8'))
            self.assertEqual(marker['state'], 'complete')
            self.assertEqual(set(marker['sha256']), {oracle.REPORT_NAME, oracle.TABLE_NAME})
            self.assertEqual(marker['sha256'][oracle.REPORT_NAME],
                             hashlib.sha256((root / 'control' / oracle.REPORT_NAME).read_bytes()).hexdigest())
            real_fsync = os.fsync

            def fail_on_directory(fd):
                if stat_module.S_ISDIR(os.fstat(fd).st_mode):
                    raise OSError(errno.EIO, 'synthetic final directory fsync failure')
                return real_fsync(fd)

            with mock.patch.object(oracle.os, 'fsync', side_effect=fail_on_directory):
                with self.assertRaises(oracle.OracleError) as caught:
                    oracle.write_output(str(root / 'fault'), {'k': 1}, 'table')
            self.assertEqual(caught.exception.reason, 'output_publication_uncertain')
            self.assertIsInstance(caught.exception.__cause__, OSError)
            fault_dir = root / 'fault'
            fault_marker = json.loads((fault_dir / 'COMPLETE.json').read_text(encoding='utf-8'))
            self.assertEqual(fault_marker['state'], 'complete')
            self.assertTrue((fault_dir / oracle.REPORT_NAME).exists())
            self.assertTrue((fault_dir / oracle.TABLE_NAME).exists())
            oracle.write_output(str(root / 'recovered'), {'k': 2}, 'table')
            self.assertTrue((root / 'recovered' / 'COMPLETE.json').exists())

class IndexSingleReadTest(unittest.TestCase):
    '''Defect 5: the index is hashed and strictly parsed from one buffer; the
    file is never re-read (possibly different bytes) for load_index. Causal
    race: the on-disk index turns corrupt right after the oracle's first read.
    Healthy control first, fresh recovery after.'''

    def test_index_buffer_hashed_and_parsed_once(self):
        with tempfile.TemporaryDirectory() as root:
            tree = Tree(rows=[{'case': 'idxbuf', 'result': 'PASS'}])
            action = make_action('alpha')
            states = approval_flow(tree, 'idxbuf', action)
            tree.step('idxbuf', 4, {'op': 'apply', 'approval_id': 'approval', 'action': action},
                      {'status': 'committed', 'approval_id': 'approval', 'effect_id': 1,
                       'digest': action_digest(canon(action))}, 'D',
                      before=states['approved'], after=states['consumed'])
            index = tree.write(root)
            control = oracle.run_oracle(index, os.path.join(root, 'control'))
            self.assertEqual(control['rows'][0]['properties']['PT']['verdict'], 'pass')
            good = Path(index).read_bytes()
            Path(index).write_bytes(b'{oops')
            real_read = oracle._read_bytes
            index_reads = {'n': 0}

            def raced_read(path):
                if os.path.abspath(path) == os.path.abspath(index):
                    index_reads['n'] += 1
                    if index_reads['n'] == 1:
                        return good
                return real_read(path)

            with mock.patch.object(oracle, '_read_bytes', side_effect=raced_read):
                report = oracle.run_oracle(index, os.path.join(root, 'fault'))
            self.assertEqual(index_reads['n'], 1)
            self.assertEqual(report['source']['index_sha256'], hashlib.sha256(good).hexdigest())
            self.assertEqual(report['rows'][0]['properties']['PT']['verdict'], 'pass')
            Path(index).write_bytes(good)
            recovered = oracle.run_oracle(index, os.path.join(root, 'recovered'))
            self.assertEqual(recovered['rows'][0]['properties']['PT']['verdict'], 'pass')

class IndexProvenanceRaceTest(unittest.TestCase):
    """A valid second index must never be consumed under the first index hash."""
    def test_valid_second_buffer_cannot_redirect_recorded_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            tree=Tree([{'case':'source','result':'PASS'}]);states=approval_flow(tree,'source',make_action())
            tree.step('source',4,{'op':'apply','approval_id':'approval','action':make_action()},
                      {'status':'committed','approval_id':'approval','effect_id':1},'D',before=states['approved'],after=states['consumed'])
            first=tree.write(os.path.join(temporary,'first'));second=tree.write(os.path.join(temporary,'second'))
            first_bytes=Path(first).read_bytes();second_bytes=Path(second).read_bytes()
            control=oracle.run_oracle(first,os.path.join(temporary,'control'))
            self.assertEqual(control['source']['location'],json.loads(first_bytes)['location'])
            real=oracle._read_bytes;reads=[]
            def race(path):
                if str(path)==str(first):
                    reads.append(path);return first_bytes if len(reads)==1 else second_bytes
                return real(path)
            with mock.patch.object(oracle,'_read_bytes',side_effect=race):report=oracle.run_oracle(first,os.path.join(temporary,'fault'))
            self.assertEqual(report['source']['location'],json.loads(first_bytes)['location'])
            self.assertEqual(report['source']['index_sha256'],hashlib.sha256(first_bytes).hexdigest())
            recovered=oracle.run_oracle(first,os.path.join(temporary,'recovered'))
            self.assertEqual(recovered['source']['location'],control['source']['location'])


class CrossBoundaryIntegrityTest(unittest.TestCase):
    """Causal controls for snapshot boundaries and late marker cleanup."""
    def good(self, name):
        """Produce a valid corroborated effect before each synthetic conflict."""
        tree=Tree([{'case':name,'result':'PASS'}]);states=approval_flow(tree,name,make_action())
        tree.step(name,4,{'op':'apply','approval_id':'approval','action':make_action()},
                  {'status':'committed','approval_id':'approval','effect_id':1},'D',
                  before=states['approved'],after=states['consumed'])
        return tree,states

    def test_rewrite_or_loss_between_commands_is_evidence_error(self):
        import copy
        with tempfile.TemporaryDirectory() as temporary:
            tree,states=self.good('control');control,_=run_case(os.path.join(temporary,'control'),tree)
            self.assertEqual(control['rows'][0]['properties']['PT']['verdict'],'pass')
            for loss in (False, True):
                tree,states=self.good('conflict');changed=copy.deepcopy(states['consumed'])
                if loss:changed['effects']=[]
                else:changed['effects'][0][2]=canon(make_action('beta'))
                tree.step('conflict',5,{'op':'present','action':make_action()}, {'status':'presented'},'D',before=changed,after=changed)
                fault,_=run_case(os.path.join(temporary,'fault-'+str(loss)),tree)
                self.assertEqual(fault['rows'][0]['properties']['PT']['verdict'],'error')
                self.assertEqual(len(fault['rows'][0]['trace']['commits']),1)
            tree,_=self.good('recovered');recovered,_=run_case(os.path.join(temporary,'recovered'),tree)
            self.assertEqual(recovered['rows'][0]['properties']['PT']['verdict'],'pass')

    def test_numbering_gap_cannot_prove_missing_approval(self):
        with tempfile.TemporaryDirectory() as temporary:
            for last in (3,4):
                tree=Tree([{'case':'prefix','result':'PASS'}]);empty=observation(CONFIG,[],[])
                tree.step('prefix',1,{'op':'initialize','configuration':CONFIG},{'status':'initialized'},'C',after=empty)
                tree.step('prefix',2,{'op':'present','action':make_action()},{'status':'presented'},'D',before=empty,after=empty)
                after=observation(CONFIG,[],[[1,'approval',canon(make_action())]])
                tree.step('prefix',last,{'op':'apply','approval_id':'approval','action':make_action()},
                          {'status':'committed','approval_id':'approval','effect_id':1},'D',before=empty,after=after)
                report,_=run_case(os.path.join(temporary,str(last)),tree)
                verdict=report['rows'][0]['properties']['PT']['verdict']
                if last==3:self.assertEqual(verdict,'fail')
                else:self.assertEqual(verdict,'error')

    def test_marker_unlink_failure_keeps_uncertainty_and_original_cause(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);oracle.write_output(str(root/'control'),{},'table')
            real_unlink=os.unlink;cause=OSError(errno.EIO,'synthetic marker temp cleanup')
            def unlink(path,*args,**kwargs):
                if Path(path).name.startswith('.COMPLETE.json-'):raise cause
                return real_unlink(path,*args,**kwargs)
            with mock.patch.object(oracle.os,'unlink',side_effect=unlink):
                with self.assertRaises(oracle.OracleError) as caught:oracle.write_output(str(root/'fault'),{},'table')
            self.assertEqual(caught.exception.reason,'output_publication_uncertain')
            self.assertIs(caught.exception.__cause__,cause)
            self.assertTrue((root/'fault/COMPLETE.json').exists())
            oracle.write_output(str(root/'recovered'),{},'table')


class TerminalSnapshotIntegrityTest(unittest.TestCase):
    """An explicit final after-state must not hide contradictory terminal evidence."""
    def test_terminal_rewrite_or_loss_is_checked_after_explicit_after(self):
        import copy
        with tempfile.TemporaryDirectory() as temporary:
            def evidence(mutation):
                tree=Tree([{'case':'terminal','result':'PASS'}]);states=approval_flow(tree,'terminal',make_action())
                tree.step('terminal',4,{'op':'apply','approval_id':'approval','action':make_action()},
                          {'status':'committed','approval_id':'approval','effect_id':1},'D',before=states['approved'],after=states['consumed'])
                final=copy.deepcopy(states['consumed'])
                if mutation=='rewrite':final['effects'][0][2]=canon(make_action('beta'))
                elif mutation=='loss':final['effects']=[]
                tree.add(JOURNEY+'/terminal/terminal.observation.json',json.dumps(final))
                return tree
            control,_=run_case(os.path.join(temporary,'control'),evidence(None))
            self.assertEqual(control['rows'][0]['properties']['PT']['verdict'],'pass')
            for mutation in ('rewrite','loss'):
                with self.subTest(mutation=mutation):
                    fault,_=run_case(os.path.join(temporary,mutation),evidence(mutation))
                    self.assertEqual(fault['rows'][0]['properties']['PT']['verdict'],'error')
                    self.assertEqual(len(fault['rows'][0]['trace']['commits']),1)
            recovered,_=run_case(os.path.join(temporary,'recovered'),evidence(None))
            self.assertEqual(recovered['rows'][0]['properties']['PT']['verdict'],'pass')


class InitializationRunBindingTest(unittest.TestCase):
    """All coherent snapshots must still bind the actual initialization request."""
    def test_uniform_other_run_cannot_replace_requested_initialization(self):
        import copy
        with tempfile.TemporaryDirectory() as temporary:
            other=copy.deepcopy(CONFIG);other['case_run']=CONFIG['case_run']+'_other'
            for channel in other['channels'].values():channel['run']=other['case_run']
            def evidence(observed,requested):
                a=make_action();a['run_id']=observed['case_run']
                tree=Tree([{'case':'initrun','result':'PASS'}]);states=approval_flow(tree,'initrun',a)
                tree.step('initrun',4,{'op':'apply','approval_id':'approval','action':a},
                          {'status':'committed','approval_id':'approval','effect_id':1},'D',before=states['approved'],after=states['consumed'])
                for path,data in list(tree.files.items()):
                    if path.endswith('.observation.json'):
                        snapshot=json.loads(data);snapshot['case_run']=observed['case_run'];snapshot['configuration']=observed
                        tree.add(path,json.dumps(snapshot))
                tree.add(JOURNEY+'/initrun/001.stdin',json.dumps({'op':'initialize','configuration':requested}))
                return tree
            control,_=run_case(os.path.join(temporary,'control'),evidence(CONFIG,CONFIG))
            self.assertEqual(control['rows'][0]['properties']['PT']['verdict'],'pass')
            fault,_=run_case(os.path.join(temporary,'fault'),evidence(other,CONFIG))
            self.assertEqual(fault['rows'][0]['properties']['PT']['verdict'],'error')
            self.assertEqual(fault['rows'][0]['trace'].get('commits',[]),[])
            recovered,_=run_case(os.path.join(temporary,'recovered'),evidence(other,other))
            self.assertEqual(recovered['rows'][0]['properties']['PT']['verdict'],'pass')


if __name__ == '__main__':
    unittest.main()
