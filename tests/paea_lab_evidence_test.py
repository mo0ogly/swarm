#!/usr/bin/env python3
"""Real campaign evidence: CLI verification, original-file faults and recovery.

Requires the preceding public campaign and fresh verifier evidence output. The
original campaign is never modified; every fault uses retained isolated copies.
No build, provider, live Root, watchdog or execution of supplied evidence.
"""
import io
from contextlib import redirect_stdout, redirect_stderr
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import unittest
from unittest.mock import patch
import jsonschema

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools.paea.lab_evidence import INVENTORY, REPORT, Refusal, collect, digest
from tools.paea.run_pilot import export_report, PublicationUnknown


class LaboratoryEvidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        """Require a real preceding campaign and fresh evidence, without rerunning it."""
        cls.source = Path(os.environ['SWARM_PAEA_LAB_EVIDENCE']).resolve()
        cls.out = Path(os.environ['SWARM_PAEA_LAB_VERIFIER_EVIDENCE']).resolve()
        cls.out.mkdir()
        cls.pin = digest((cls.source / INVENTORY).read_bytes())
        cls.binary = json.loads((cls.source / REPORT).read_bytes())['binary_sha256']
        cls.sequence = 0
        cls.case_sequence = 0

    def fresh(self):
        """Copy only the canonical bundle into a new retained test-owned directory."""
        type(self).case_sequence += 1
        target = self.out / ('fixture-%03d' % self.case_sequence)
        target.mkdir()
        for name in ('recipe', 'observations'):
            shutil.copytree(self.source / name, target / name)
        for name in (REPORT, INVENTORY):
            shutil.copyfile(self.source / name, target / name)
        return target

    def check(self, target, good=True, pin=None, stdout=None):
        """Invoke the companion CLI and retain argv/output/exit before assertions.

        Refusals must be safe structured diagnostics, with no success stdout.
        Optional stdout explicitly injects a real output writer failure.
        """
        type(self).sequence += 1
        prefix = self.out / ('check-%04d' % self.sequence)
        argv = [sys.executable, str(ROOT / 'tools/paea/verify_pilot.py'), '--campaign', str(target),
                '--inventory-sha256', self.pin if pin is None else pin]
        result = subprocess.run(argv, stdout=stdout if stdout is not None else subprocess.PIPE, stderr=subprocess.PIPE)
        (prefix.with_suffix('.json')).write_text(json.dumps({'argv': argv, 'exit': result.returncode,
                                                          'stdout_fault': stdout is not None}, indent=2)+'\n')
        if result.stdout is not None:
            prefix.with_suffix('.stdout').write_bytes(result.stdout)
        prefix.with_suffix('.stderr').write_bytes(result.stderr)
        self.assertEqual(result.returncode, 0 if good else 2, result.stderr)
        if good:
            value = json.loads(result.stdout)
            self.assertEqual(value['status'], 'laboratory-evidence-consistent')
            self.assertEqual(value['scope'], 'retained-bytes-only')
            self.assertEqual(value['authority'], 'simulated')
            self.assertEqual(value['independent_evaluation'], 'not-established')
            self.assertEqual(value['protected_invariants'], 'not-established')
            self.assertEqual(result.stderr, b'')
        else:
            self.assertIn(result.stdout, (b'', None))
            value = json.loads(result.stderr)
            self.assertEqual(set(value), {'status', 'code'})
            self.assertEqual(value['status'], 'refused')
            self.assertNotIn(b'OUTSIDE_SECRET', result.stderr)
        return result

    def oracle_file(self, target):
        """Locate an actual captured oracle response, requiring real effects/refs data."""
        return next(p for p in sorted((target / 'observations').glob('call-*.stdout'))
                    if b'"available": true' in p.read_bytes())

    def test_actual_campaign_schema_repeat_and_no_original_mutation(self):
        """Actual sealed report satisfies the maintained schema and repeats read-only."""
        schema = json.loads((ROOT / 'schemas/paea/laboratory-evidence-v1.schema.json').read_bytes())
        jsonschema.Draft202012Validator.check_schema(schema)
        inventory = json.loads((self.source / INVENTORY).read_bytes())
        jsonschema.Draft202012Validator(schema).validate(inventory)
        before = {p.relative_to(self.source).as_posix(): digest(p.read_bytes()) for p in self.source.rglob('*') if p.is_file()}
        self.check(self.source)
        self.check(self.source)
        self.assertEqual(before, {p.relative_to(self.source).as_posix(): digest(p.read_bytes()) for p in self.source.rglob('*') if p.is_file()})
        self.assertIn('calls.json', inventory['observation_files'])

    def test_deleted_changed_and_added_observation_refuse_then_recover(self):
        """A valid control precedes each missing/altered/extra raw-oracle file fault."""
        for fault in ('deleted', 'changed', 'added'):
            with self.subTest(fault=fault):
                target = self.fresh()
                self.check(target)
                path = self.oracle_file(target)
                original = path.read_bytes()
                if fault == 'deleted':
                    path.unlink()
                elif fault == 'changed':
                    path.write_bytes(original + b'changed')
                else:
                    path = target / 'observations/unindexed-oracle.json'
                    path.write_text('{"available":true}')
                self.check(target, good=False)
                if fault == 'added':
                    path.unlink()
                else:
                    path.write_bytes(original)
                self.check(target)

    def test_export_refuses_missing_oracle_before_publication(self):
        """Regression: deleted raw oracle blocks new inventory/report, then recovers."""
        target = self.fresh()
        self.check(target)
        path = self.oracle_file(target)
        raw = path.read_bytes()
        for name in (REPORT, INVENTORY):
            (target / name).unlink()
        path.unlink()
        with self.assertRaisesRegex(Refusal, 'laboratory-call-artifact-missing-or-changed'):
            export_report(target / 'recipe', target / 'observations', self.binary)
        self.assertFalse((target / REPORT).exists())
        self.assertFalse((target / INVENTORY).exists())
        path.write_bytes(raw)
        export_report(target / 'recipe', target / 'observations', self.binary)
        self.check(target, pin=digest((target / INVENTORY).read_bytes()))

    def test_pinned_inventory_and_closed_representations_refuse(self):
        """Wrong pin and independently re-pinned malformed inventories cannot pass."""
        for mutation in ('pin', 'duplicate', 'bool-version', 'float', 'traversal', 'extra-field', 'bom'):
            with self.subTest(mutation=mutation):
                target = self.fresh()
                self.check(target)
                path = target / INVENTORY
                original = path.read_bytes()
                if mutation == 'pin':
                    pin = '0' * 64
                else:
                    value = json.loads(original)
                    if mutation == 'duplicate':
                        raw = b'{"schema_version":1,' + original.lstrip()[1:]
                    elif mutation == 'bom':
                        raw = b'\xef\xbb\xbf' + original
                    else:
                        if mutation == 'bool-version':value['schema_version'] = True
                        if mutation == 'float':value['schema_version'] = 1.0
                        if mutation == 'traversal':value['observation_files']['../OUTSIDE_SECRET'] = '0' * 64
                        if mutation == 'extra-field':value['authenticated'] = True
                        raw = json.dumps(value).encode()
                    path.write_bytes(raw)
                    pin = digest(raw)
                self.check(target, good=False, pin=pin)
                path.write_bytes(original)
                self.check(target)

    def test_report_call_index_and_recipe_links_refuse_then_recover(self):
        """Each actual report/index/recipe mutation preserves originals and fails."""
        for name in (REPORT, 'observations/calls.json', 'recipe/step-0001.stdout'):
            with self.subTest(name=name):
                target = self.fresh()
                self.check(target)
                path = target / name
                raw = path.read_bytes()
                path.write_bytes(raw + b'corrupt')
                self.check(target, good=False)
                path.write_bytes(raw)
                self.check(target)

    def test_claimed_authority_in_report_is_rejected_with_updated_pin(self):
        """Self-consistent manifest changes cannot promote the report's authority marker."""
        target = self.fresh()
        self.check(target)
        report = json.loads((target / REPORT).read_bytes())
        report['authority'] = 'authenticated'
        raw = json.dumps(report).encode()
        (target / REPORT).write_bytes(raw)
        inventory = json.loads((target / INVENTORY).read_bytes())
        inventory['report_sha256'] = digest(raw)
        raw = json.dumps(inventory).encode()
        (target / INVENTORY).write_bytes(raw)
        self.check(target, good=False, pin=digest(raw))

    def test_links_fifo_and_directory_substitution_refuse_without_disclosure(self):
        """No-follow descriptor traversal refuses links/special files without waiting."""
        for kind in ('file-link', 'fifo', 'directory-link'):
            with self.subTest(kind=kind):
                target = self.fresh()
                self.check(target)
                path = self.oracle_file(target)
                original = path.read_bytes()
                outside = self.out / ('OUTSIDE_SECRET-' + kind)
                outside.write_text('OUTSIDE_SECRET never disclose this fixture value')
                if kind == 'directory-link':
                    path = target / 'observations'
                    backup = target / 'observations.saved'
                    path.rename(backup)
                    path.symlink_to(self.out, target_is_directory=True)
                else:
                    path.unlink()
                    if kind == 'file-link':path.symlink_to(outside)
                    else:os.mkfifo(path)
                self.check(target, good=False)
                path.unlink()
                if kind == 'directory-link':backup.rename(path)
                else:path.write_bytes(original)
                self.check(target)
                self.assertEqual(outside.read_text(), 'OUTSIDE_SECRET never disclose this fixture value')

    def test_pre_publication_failure_never_creates_a_success_report(self):
        """Injected ENOSPC at either publication phase retains an incomplete bundle."""
        import tools.paea.run_pilot as producer
        original_publish = producer.publish
        for failed in (INVENTORY, REPORT):
            with self.subTest(failed=failed):
                target = self.fresh()
                self.check(target)
                for name in (REPORT, INVENTORY):(target / name).unlink()

                def fail_at(path, raw):
                    """Inject a writer refusal at one explicit phase; otherwise publish normally."""
                    if Path(path).name == failed:raise OSError(28, 'fixture-no-space')
                    return original_publish(path, raw)
                with patch.object(producer, 'publish', side_effect=fail_at):
                    with self.assertRaises(PublicationUnknown) as caught:
                        export_report(target / 'recipe', target / 'observations', self.binary)
                self.assertIsInstance(caught.exception.__cause__, OSError)
                self.assertEqual(caught.exception.__cause__.errno, 28)
                self.assertFalse((target / REPORT).exists())
                self.assertEqual((target / INVENTORY).exists(), failed == REPORT)
                self.check(target, good=False)
                recovered = self.fresh()
                self.check(recovered)

    def test_post_publication_error_is_unknown_and_retained_bundle_is_checkable(self):
        """Replay retained bytes to isolate main's post-publication error consumer.

        The first recipe step really ran eight cases. This local replay copies
        that exact retained recipe/observations, without claiming another run.
        An error after the real exclusive report publication keeps both files;
        main must emit artifact UNKNOWN, never infer absence from the exception.
        """
        import tools.paea.run_pilot as producer
        control = self.fresh()
        self.check(control)
        type(self).case_sequence += 1
        target = self.out / ('replayed-publication-%03d' % self.case_sequence)
        original_publish = producer.publish

        def retained_replay(_root, _recipe, evidence):
            """Copy actual retained PASS bytes for a diagnostic fixture, never execute argv."""
            shutil.copytree(self.source / 'recipe', evidence)
            shutil.copytree(self.source / 'observations', evidence.parent / 'observations')
            return json.loads((evidence / 'result.json').read_bytes())

        def published_then_fail(path, raw):
            """Publish actual bytes, then model a directory-sync failure after the link."""
            original_publish(path, raw)
            if Path(path).name == REPORT:raise OSError(5, 'fixture-sync-after-publication')

        argv = ['run_pilot.py', '--binary', os.environ['SWARM_PAEA_TEST_BINARY'],
                '--sha256', self.binary, '--output', str(target)]
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.object(sys, 'argv', argv), patch.object(producer, 'run_recipe', side_effect=retained_replay), patch.object(producer, 'publish', side_effect=published_then_fail):
            with redirect_stdout(stdout), redirect_stderr(stderr):
                code = producer.main()
        (target / 'publication-unknown.json').write_text(stderr.getvalue())
        self.assertEqual(code, 2)
        self.assertEqual(stdout.getvalue(), '')
        value = json.loads(stderr.getvalue())
        self.assertEqual(value['status'], 'UNKNOWN')
        self.assertEqual(value['scope'], 'laboratory-report-publication')
        self.assertTrue((target / REPORT).is_file())
        self.assertTrue((target / INVENTORY).is_file())
        self.check(target, pin=value['expected_inventory_sha256'])
        print('PAEA_ARTIFACT_PUBLICATION_UNKNOWN_RECONCILED retained-copy replay, no extra campaign run', flush=True)

    def test_output_failure_is_nonzero_then_normal_output_recovers(self):
        """A real /dev/full writer cannot convert a verified bundle into successful output."""
        target = self.fresh()
        self.check(target)
        with open('/dev/full', 'wb', buffering=0) as destination:
            self.check(target, good=False, stdout=destination)
        self.check(target)

    def test_self_consistent_changed_evidence_does_not_establish_authenticity(self):
        """A new pin can cover a forged same-UID observation; authority stays simulated.

        Preserve this counterexample to any authenticity claim. The original pin
        still rejects the changed bundle, which is the promised integrity property.
        """
        target = self.fresh()
        self.check(target)
        path = self.oracle_file(target)
        raw = b'{"available":true,"effects":[],"fixture_forgery":true}\n'
        path.write_bytes(raw)
        calls = json.loads((target / 'observations/calls.json').read_bytes())
        for call in calls:
            if path.name in call['artifacts']:call['artifacts'][path.name] = digest(raw)
        calls_raw = (json.dumps(calls) + '\n').encode()
        (target / 'observations/calls.json').write_bytes(calls_raw)
        report = json.loads((target / REPORT).read_bytes())
        report['calls_sha256'] = digest(calls_raw)
        report_raw = (json.dumps(report) + '\n').encode()
        (target / REPORT).write_bytes(report_raw)
        inventory_raw = (json.dumps(collect(target, report_raw), sort_keys=True, indent=2) + '\n').encode()
        (target / INVENTORY).write_bytes(inventory_raw)
        self.check(target, good=False)
        self.check(target, pin=digest(inventory_raw))
        print('PAEA_EVIDENCE_AUTHENTICITY_NOT_ESTABLISHED counterexample retained', flush=True)


if __name__ == '__main__':
    unittest.main(verbosity=2)
