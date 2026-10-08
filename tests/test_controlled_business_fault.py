import json
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import tempfile
import unittest
from controlled_business_fault import GOOD, check_feature, inject_once


class ControlledFaultTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.copies = self.base / 'copies'
        self.workspace = self.copies / 'attempt-1'
        self.workspace.mkdir(parents=True)
        (self.workspace / 'feature.py').write_text(GOOD)
        self.evidence = self.base / 'evidence'

    def inject(self, **changes):
        args = dict(copies_root=self.copies, workspace=self.workspace,
                    evidence_dir=self.evidence, work='w-trial', agent='producer-1', attempt='a-1')
        args.update(changes)
        return inject_once(**args)

    def test_real_business_failure_and_one_shot_correction(self):
        first = self.inject()
        self.assertTrue(first['injected_now'])
        self.assertEqual(first['evidence']['precheck_exit'], 0)
        self.assertNotEqual(first['evidence']['postcheck_exit'], 0)
        self.assertNotEqual(first['evidence']['before_sha256'], first['evidence']['after_sha256'])
        self.assertEqual((self.evidence / 'before-feature.py').read_text(), GOOD)
        self.assertNotEqual(check_feature(self.workspace / 'feature.py').returncode, 0)
        second = self.copies / 'attempt-2'
        second.mkdir()
        (second / 'feature.py').write_text(GOOD)
        self.assertFalse(self.inject(workspace=second, agent='producer-2', attempt='a-2')['injected_now'])
        self.assertEqual(check_feature(second / 'feature.py').returncode, 0)
        self.assertEqual(json.loads((self.evidence / 'injection.json').read_text())['attempt'], 'a-1')

    def test_concurrent_callbacks_only_inject_once(self):
        with ThreadPoolExecutor(max_workers=2) as pool:
            results=list(pool.map(lambda _: self.inject(),range(2)))
        self.assertEqual(sum(r['injected_now'] for r in results),1)
        self.assertEqual(results[0]['evidence'],results[1]['evidence'])

    def test_bad_producer_is_not_misreported_as_injected_fault(self):
        (self.workspace / 'feature.py').write_text('def is_expected(value): return False\n')
        with self.assertRaisesRegex(ValueError, 'already fails'):
            self.inject()
        self.assertFalse((self.evidence / 'injection.json').exists())

    def test_outside_or_symlink_copies_refused(self):
        with self.assertRaisesRegex(ValueError, 'direct disposable'):
            self.inject(workspace=self.base)
        (self.workspace / 'feature.py').unlink()
        outside = self.base / 'outside.py'
        outside.write_text(GOOD)
        (self.workspace / 'feature.py').symlink_to(outside)
        with self.assertRaisesRegex(ValueError, 'regular'):
            self.inject()
        self.assertEqual(outside.read_text(), GOOD)

    def test_pending_marker_and_foreign_work_cannot_reinject(self):
        self.inject()
        marker = self.evidence / 'injection.json'
        with self.assertRaisesRegex(ValueError, 'different work'):
            self.inject(work='w-other')
        r = json.loads(marker.read_text()); r['state'] = 'pending'
        marker.write_text(json.dumps(r))
        with self.assertRaisesRegex(ValueError, 'incomplete injection'):
            self.inject()

    def test_evidence_cannot_be_owned_by_worker(self):
        with self.assertRaisesRegex(ValueError, 'outside agent copies'):
            self.inject(evidence_dir=self.workspace / 'evidence')


if __name__ == '__main__':
    unittest.main()
