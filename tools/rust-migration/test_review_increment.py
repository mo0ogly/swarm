"""Regression guards for review coverage/freshness; isolated local files only."""
import copy
import json
from pathlib import Path
import tempfile
import tarfile
import unittest
from review_increment import AXES, check, digest, publish, resolve_baseline, require_review, ReviewRefused, archive_references, retained_parity, go_test_inventory


class GoInventoryTests(unittest.TestCase):
    """A nonempty named witness precedes false-empty, cached and skipped reports."""
    def test_go_inventory_refuses_zero_cached_skipped_or_mismatched_execution(self):
        witness='--- PASS: TestWitness (0.01s)\nPASS\nok package 0.01s\n'
        listed='TestWitness\nok package 0.01s\n'
        self.assertEqual(go_test_inventory(witness,listed),['TestWitness'])
        for actual,inventory in [('',listed),('PASS\nok package [no tests to run]\n',''),(witness+'(cached)',listed),('--- SKIP: TestWitness (0.01s)',listed),(witness,'TestOther'),(witness+witness,listed)]:
            with self.assertRaises(ValueError):go_test_inventory(actual,inventory)


class ParityReferenceTests(unittest.TestCase):
    """Explicit references accept a witness then refuse tampering and path escapes."""
    def test_explicit_parity_reference_preserves_original_and_refuses_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);folder=root/'chosen-parity';folder.mkdir()
            result=folder/'result.json';original=b'{"status":"PASS"}';result.write_bytes(original)
            reference=dict(schema_version=1,path='chosen-parity',result_sha256=digest(result))
            self.assertEqual(retained_parity(root,reference),folder)
            result.write_bytes(b'{"status":"FAIL"}')
            with self.assertRaises(ValueError):retained_parity(root,reference)
            result.write_bytes(original)
            for path in ['../escape',str(folder),'missing']:
                bad=dict(reference,path=path)
                with self.assertRaises(ValueError):retained_parity(root,bad)
            with self.assertRaises(ValueError):retained_parity(root,dict(reference,command=['ignored']))
            with self.assertRaises(ValueError):retained_parity(root,dict(reference,schema_version=True))
            self.assertEqual(result.read_bytes(),original)
            self.assertEqual(retained_parity(root,reference),folder)


class ReviewIncrementTests(unittest.TestCase):
    """Each fault follows a complete witness under identical preconditions."""

    def setUp(self):
        """Declare two snapshots and a review bound to real source/evidence bytes."""
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / 'source.rs').write_text('/// Return a protocol flag.\npub fn flag() -> bool { false }\n')
        (self.root / 'proof.txt').write_text('Declared semantic review, not independent execution.\n')
        self.before, self.after = self.root / 'before.json', self.root / 'after.json'
        self.before.write_text(json.dumps({'source.rs': '0' * 64}))
        self.after.write_text(json.dumps({'source.rs': digest(self.root / 'source.rs')}))
        self.review = dict(schema_version=1, reviewer='Codex fixture', baseline_sha256=digest(self.before), current_sha256=digest(self.after), files={
            'source.rs': {'sha256': digest(self.root / 'source.rs'), 'axes': {
                axis: {'state': 'reviewed', 'reason': 'Declared fixture only', 'evidence': [{'path': 'proof.txt', 'sha256': digest(self.root / 'proof.txt')}]}
                for axis in AXES}}})

    def witness(self):
        """Require complete declared review before injecting a negative case."""
        result = check(self.root, self.before, self.after, self.review)
        self.assertEqual(result['contract_status'], 'DECLARED_REVIEW_COMPLETE')
        self.assertFalse(result['semantic_guarantee'])
        self.assertTrue(result['lexical_candidates']['source.rs']['functions'])

    def test_reference_originals_survive_document_updates_and_refuse_stale_inputs(self):
        """Archive exact valid evidence before later edits; stale inputs fail closed."""
        self.witness()
        original = (self.root / 'proof.txt').read_bytes()
        output = self.root / 'qualified-originals'
        output.mkdir()
        (output / 'inputs-before.json').write_bytes(self.after.read_bytes())
        (output / 'source-review.json').write_text(json.dumps(self.review))
        require_review(self.root, output, self.before)
        archive = output / 'review-reference-originals.tar.gz'
        self.assertTrue(archive.is_file(), 'review originals were not archived')
        metadata = json.loads((output / 'review-reference-originals.json').read_bytes())
        self.assertEqual(metadata['archive_sha256'], digest(archive))
        self.assertEqual(metadata['files'], {'proof.txt': digest(self.root / 'proof.txt')})
        with tarfile.open(archive) as bundle:
            self.assertEqual(bundle.getnames(), ['proof.txt'])
            self.assertEqual(bundle.extractfile('proof.txt').read(), original)
        (self.root / 'proof.txt').write_text('Later documentation revision')
        with tarfile.open(archive) as bundle:
            self.assertEqual(bundle.extractfile('proof.txt').read(), original)
        refused = self.root / 'qualified-stale'
        refused.mkdir()
        (refused / 'inputs-before.json').write_bytes(self.after.read_bytes())
        (refused / 'source-review.json').write_text(json.dumps(self.review))
        with self.assertRaises(ReviewRefused):
            require_review(self.root, refused, self.before)
        self.assertEqual(json.loads((refused / 'source-review-result.json').read_bytes())['coverage_status'], 'FAIL')
        self.assertFalse((refused / 'review-reference-originals.tar.gz').exists())

    def test_reference_archive_refuses_changed_bytes_and_preserves_collisions(self):
        """Valid control then changed-read/collision faults, preserving originals."""
        self.witness()
        valid = self.root / 'archive-valid'
        valid.mkdir()
        archive_references(self.root, valid, self.review)
        original_archive = (valid / 'review-reference-originals.tar.gz').read_bytes()
        with self.assertRaises(FileExistsError):
            archive_references(self.root, valid, self.review)
        self.assertEqual((valid / 'review-reference-originals.tar.gz').read_bytes(), original_archive)
        self.assertFalse(list(valid.glob('.review-originals.*')))
        changed = self.root / 'archive-changed'
        changed.mkdir()
        (self.root / 'proof.txt').write_text('Changed after the coverage check')
        with self.assertRaisesRegex(ValueError, 'changed-before-archive'):
            archive_references(self.root, changed, self.review)
        self.assertFalse((changed / 'review-reference-originals.tar.gz').exists())
        self.assertFalse((changed / 'review-reference-originals.json').exists())
        self.assertFalse(list(changed.glob('.review-originals.*')))

    def test_unselected_retained_source_requires_reason_and_exact_original_bytes(self):
        """Require explicit unselection with original bytes; ambiguous/stale states refuse."""
        self.witness()
        tool=self.root/'completed-fixture.py';tool.write_text('"""Completed isolated fixture producer."""\n')
        old=json.loads(self.before.read_bytes());old[tool.name]=digest(tool);self.before.write_text(json.dumps(old))
        self.review['baseline_sha256']=digest(self.before)
        entry=copy.deepcopy(self.review['files']['source.rs']);entry['sha256']=None
        entry['selection']={'state':'unselected','sha256':digest(tool),'reason':'Completed fixture source retained as proof, no longer consumed'}
        self.review['files'][tool.name]=entry
        valid=check(self.root,self.before,self.after,self.review)
        self.assertEqual(valid['contract_status'],'DECLARED_REVIEW_COMPLETE')
        self.assertTrue(valid['lexical_candidates'][tool.name]['unselected'])
        original=tool.read_bytes()
        for field,bad in [('selection',None),('reason',''),('sha256','0'*64)]:
            self.assertEqual(check(self.root,self.before,self.after,self.review)['contract_status'],'DECLARED_REVIEW_COMPLETE')
            wrong=copy.deepcopy(self.review)
            if field=='selection':wrong['files'][tool.name].pop('selection')
            else:wrong['files'][tool.name]['selection'][field]=bad
            with self.assertRaises(ValueError):check(self.root,self.before,self.after,wrong)
        absent = copy.deepcopy(self.review)
        tool.unlink()
        with self.assertRaises(ValueError):check(self.root,self.before,self.after,absent)
        tool.write_bytes(original)
        selected = copy.deepcopy(self.review)
        selected['files']['source.rs']['selection'] = copy.deepcopy(entry['selection'])
        with self.assertRaises(ValueError):check(self.root,self.before,self.after,selected)
        tool.write_text('Changed producer after it was removed from the selected graph')
        with self.assertRaises(ValueError):check(self.root,self.before,self.after,self.review)
        tool.write_bytes(original)
        self.assertEqual(check(self.root,self.before,self.after,self.review)['contract_status'],'DECLARED_REVIEW_COMPLETE')

    def test_malformed_declaration_shapes_refuse(self):
        """Malformed JSON shapes are diagnosed as refused declarations."""
        self.witness()
        invalid = [None, [], 'review']
        bad = copy.deepcopy(self.review)
        bad['files']['source.rs'] = []
        invalid.append(bad)
        bad = copy.deepcopy(self.review)
        bad['files']['source.rs']['axes']['errors'] = []
        invalid.append(bad)
        bad = copy.deepcopy(self.review)
        bad['files']['source.rs']['axes']['errors']['evidence'] = [None]
        invalid.append(bad)
        for value in invalid:
            with self.assertRaises(ValueError):
                check(self.root, self.before, self.after, value)

    def test_open_followup_requires_real_strings(self):
        """Null/nontext owners and resumption conditions are not valid follow-up."""
        for field in ('owner', 'resume_when'):
            for invalid in (None, False, [], '', ' '):
                self.witness()
                bad = copy.deepcopy(self.review)
                outcome = bad['files']['source.rs']['axes']['policies']
                outcome.update(state='open', owner='Codex', resume_when='Admin consumer verified')
                outcome[field] = invalid
                with self.assertRaisesRegex(ValueError, 'followup-missing'):
                    check(self.root, self.before, self.after, bad)

    def test_qualification_hook_requires_baseline_and_review(self):
        """The shared hook refuses missing/open/stale review with original reports."""
        self.witness()
        for environ in ({}, {'SWARM_MIGRATION_REVIEW_BASELINE': '../outside'}):
            with self.assertRaises(ValueError):
                resolve_baseline(self.root, environ)
        baseline = resolve_baseline(self.root, {'SWARM_MIGRATION_REVIEW_BASELINE': 'before.json'})
        for mode in ('valid', 'missing', 'open', 'stale'):
            self.witness()
            output = self.root / mode
            output.mkdir()
            (output / 'inputs-before.json').write_bytes(self.after.read_bytes())
            review = copy.deepcopy(self.review)
            if mode == 'open':
                review['files']['source.rs']['axes']['policies'].update(state='open', owner='Codex', resume_when='Verified policy consumer')
            if mode == 'stale':
                review['current_sha256'] = '0' * 64
            if mode != 'missing':
                (output / 'source-review.json').write_text(json.dumps(review))
            if mode == 'valid':
                self.assertEqual(require_review(self.root, output, baseline)['contract_status'], 'DECLARED_REVIEW_COMPLETE')
            else:
                with self.assertRaises(ReviewRefused):
                    require_review(self.root, output, baseline)
                result = json.loads((output / 'source-review-result.json').read_bytes())
                self.assertTrue(result.get('coverage_status') == 'FAIL' or result.get('contract_status') == 'BLOCKED')

    def test_collector_valid_baseline_reaches_actual_inventory_boundary(self):
        """Exercise the real collector preflight before substituting compiler IO.

        A valid baseline must reach the first inventory call; no compiler or live
        operation is run. This catches local-name/import shadowing before hashing.
        """
        from qualification_inputs import collect
        self.witness()
        output = self.root / 'collector-preflight'
        output.mkdir()
        calls = []
        class InventoryReached(RuntimeError):
            """Sentinel stopping this isolated preflight before compiler execution."""
        def stop_at_inventory(label, args):
            """Record the first expected command and terminate without executing it."""
            calls.append((label, args))
            raise InventoryReached()
        with self.assertRaises(InventoryReached):
            collect(self.root, output, {'SWARM_MIGRATION_REVIEW_BASELINE': 'before.json'}, stop_at_inventory, [])
        self.assertEqual(calls[0][0], 'go-env')
        self.assertEqual(json.loads((output / 'review-baseline.json').read_bytes())['sha256'], digest(self.before))

    def test_missing_file_and_axis_refuse(self):
        """A missing review or one omitted axis cannot silently pass."""
        for mutation in ('file', 'axis'):
            self.witness()
            bad = copy.deepcopy(self.review)
            if mutation == 'file':
                bad['files'].clear()
            else:
                bad['files']['source.rs']['axes'].pop('errors')
            with self.assertRaisesRegex(ValueError, 'incomplete'):
                check(self.root, self.before, self.after, bad)

    def test_changed_source_and_proof_refuse(self):
        """Exact source and evidence changes invalidate a declaration."""
        for name in ('source.rs', 'proof.txt'):
            self.witness()
            path = self.root / name
            original = path.read_bytes()
            path.write_bytes(original + b'changed')
            with self.assertRaisesRegex(ValueError, 'stale'):
                check(self.root, self.before, self.after, self.review)
            path.write_bytes(original)

    def test_snapshot_change_and_missing_evidence_refuse(self):
        """A different snapshot and unsupported reviewed claim are rejected."""
        self.witness()
        original = self.after.read_bytes()
        self.after.write_bytes(original + b' ')
        with self.assertRaisesRegex(ValueError, 'snapshot-stale'):
            check(self.root, self.before, self.after, self.review)
        self.after.write_bytes(original)
        self.witness()
        bad = copy.deepcopy(self.review)
        bad['files']['source.rs']['axes']['errors']['evidence'] = []
        with self.assertRaisesRegex(ValueError, 'evidence-missing'):
            check(self.root, self.before, self.after, bad)

    def test_open_finding_blocks_until_documented_resolution(self):
        """Coverage can pass while an unresolved contract remains blocked."""
        self.witness()
        bad = copy.deepcopy(self.review)
        bad['files']['source.rs']['axes']['policies'].update(state='open', owner='Codex', resume_when='Admin consumer verified')
        result = check(self.root, self.before, self.after, bad)
        self.assertEqual(result['coverage_status'], 'PASS')
        self.assertEqual(result['contract_status'], 'BLOCKED')
        self.assertEqual(len(result['open_findings']), 1)
        self.witness()

    def test_escape_refuses_and_existing_report_is_preserved(self):
        """Bad references cannot escape root or overwrite historical evidence."""
        self.witness()
        bad = copy.deepcopy(self.review)
        bad['files']['source.rs']['axes']['errors']['evidence'][0]['path'] = '../private'
        with self.assertRaisesRegex(ValueError, 'path-invalid'):
            check(self.root, self.before, self.after, bad)
        output = self.root / 'result.json'
        publish(output, {'original': True})
        original = output.read_bytes()
        with self.assertRaises(FileExistsError):
            publish(output, {'replacement': True})
        self.assertEqual(output.read_bytes(), original)
        self.assertFalse(list(self.root.glob('.result.json*')))


if __name__ == '__main__':
    unittest.main(verbosity=2)
