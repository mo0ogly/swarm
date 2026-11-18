"""Inert prerequisite and recipe sequencing checks; no native child executes.

Synthetic files are plain retained directories, not initialized Swarm Roots or
authority grants. Every fault follows a valid control with the same inputs.
"""
import contextlib
import json
from pathlib import Path
import subprocess
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from fixture_area import fixture_root

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / 'tools/verification'))
import paea_withdrawal_preflight as gate
import cargo_space_guard
import recipe


class PrerequisiteSequencing(unittest.TestCase):
    """Exercise real receipt checks and run_recipe's stop boundary with doubles."""

    def setUp(self):
        """Make a minimal coherent synthetic composition, retaining all evidence."""
        self.root = fixture_root('p4-preflight-inert-')
        def write(path, value):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(value if isinstance(value, str) else json.dumps(value))
            return path
        self.write = write
        compiled = self.root / 'compiled'
        frozen = self.root / 'frozen'
        target = self.root / 'target'
        target.mkdir()
        code = write(frozen / 'code.rs', 'inert source')
        inputs = write(compiled / 'source-inputs.json', {'code.rs': gate.digest(code)})
        write(compiled / 'freeze.json', dict(frozen=str(frozen), fixture_target=str(target),
                                            source_inputs_sha256=gate.digest(inputs)))
        write(compiled / 'result.json', dict(exit=0, source_stable=True, registry_stable=True,
              source_inputs_sha256=gate.digest(inputs),
              build_finished=[dict(reason='build-finished', success=True)]))
        write(compiled / 'actual-source-dependencies.json', {str(code): gate.digest(code)})
        dep = write(self.root / 'code.d', 'inert depfile')
        write(compiled / 'depfile-proof.json', dict(workspace_all_declared=True,
              depfiles={str(dep): gate.digest(dep)}))
        write(compiled / 'depfile-missing.json', [])
        write(compiled / 'depfile-unmatched.json', {})
        elf_path = write(self.root / 'never-executed-elf', 'not an executable')
        elf = dict(path=str(elf_path), sha256=gate.digest(elf_path), features=['test-fixtures'],
                   profile=dict(test=True), body_executed=False,
                   source_inputs_sha256=gate.digest(inputs))
        write(compiled / 'retained-elf.json', elf)
        public_root = self.root / 'original'
        (public_root / '.swarm').mkdir(parents=True)
        original = write(public_root / 'original.txt', 'original fixture bytes')
        marker = write(public_root / 'marker.json', {})
        physical = (public_root / '.swarm').stat()
        identity = dict(device=physical.st_dev, inode=physical.st_ino)
        fresh = self.root / 'fresh'
        proof = fresh / 'source'
        artifact = write(proof / 'artifact.json', {})
        public = dict(status='PASS', task_accepted=True, paid_provider=False,
                      root=str(public_root), physical=identity,
                      artifacts={'artifact.json': gate.digest(artifact)},
                      consumed_originals={str(original): gate.digest(original)},
                      review_marker=dict(path=str(marker), sha256=gate.digest(marker)))
        public_path = write(proof / 'result.json', public)
        process = write(proof / 'process.json', dict(exit=0))
        terminal = write(proof / 'terminal.json', dict(status='QUALIFIED_P4_FRESH_SOURCE',
                         exit=0, process_sha256=gate.digest(process),
                         result_sha256=gate.digest(public_path)))
        helper_root = self.root / 'helper'
        helper = write(helper_root / 'helper.py', 'inert helper')
        write(fresh / 'source-inputs.json', {'helper.py': gate.digest(helper)})
        write(fresh / 'result.json', dict(helper_root=str(helper_root)))
        binary = write(self.root / 'never-executed-default', 'inert default')
        profile = write(self.root / 'profile.json', {})
        qualified = dict(source_evidence=str(proof), root=str(public_root), physical=identity,
                         terminal_sha256=gate.digest(terminal), result_sha256=gate.digest(public_path),
                         binary_sha256=gate.digest(binary), profile_sha256=gate.digest(profile))
        write(fresh / 'qualified-fixture.json', qualified)
        admission = write(self.root / 'synthetic-admission.json',
                          dict(state='PREP_OUTPUTS_INDIVIDUALLY_ADMITTED_AFTER_CORRECTED_AUDIT',
                               A=dict(elf=elf), B=dict(fixture=qualified)))
        policy = cargo_space_guard.load_policy(REPO / '.cargo/storage-policy.json')
        self.minimum = policy['minimum_free_bytes']
        self.space = lambda _: SimpleNamespace(free=self.minimum)
        write(self.root / '.cargo/storage-policy.json', policy)
        self.manifest = dict(schema_version=1, repository=str(self.root),
                             admission=dict(path=str(admission), sha256=gate.digest(admission)),
                             compiled=str(compiled), fresh=str(fresh), default_binary=str(binary),
                             default_profile=str(profile), current_code={str(code): gate.digest(code)})
        self.manifest['receipts'] = {str(p): gate.digest(p) for p in self.root.rglob('*.json')}
        self.fault_file = elf_path

    @contextlib.contextmanager
    def admitted(self, root, target):
        """Inert FD-admission double; checks are counted, never mount or spawn."""
        class Guard:
            def check(inner):
                return {'inert': True}
        yield Guard()

    def run_inert_recipe(self, label):
        """Run the real recipe boundary with only two mocked subprocess calls.

        Step one runs the actual checker in process with inert storage admission;
        step two is a marker, never an ELF. Failure must prevent its invocation.
        """
        manifest_path = self.root / (label + '.recipe.json')
        steps = [dict(id='preflight', command=['python3', 'inert-preflight'], expected_exit=0,
                      required_stdout=['P4_PREREQUISITES_VERIFIED'], required_stderr=[]),
                 dict(id='inert-next-step', command=['python3', 'never-executed'], expected_exit=0,
                      required_stdout=['INERT_NEXT_STEP'], required_stderr=[])]
        self.write(manifest_path, dict(schema_version=1, id='inert-p4-sequencing',
                   inputs=['tools/verification/paea_withdrawal_preflight.py'], environment={}, steps=steps))
        reached = []
        def mocked_run(command, **options):
            if command[1] == 'inert-preflight':
                try:
                    gate.gated(self.manifest, lambda _: None, admission=self.admitted,
                               space=self.space)
                except (ValueError, OSError) as error:
                    options['stderr'].write(type(error).__name__.encode())
                    return subprocess.CompletedProcess(command, 1)
                options['stdout'].write(b'P4_PREREQUISITES_VERIFIED')
            else:
                reached.append('next-step')
                options['stdout'].write(b'INERT_NEXT_STEP')
            return subprocess.CompletedProcess(command, 0)
        with patch.object(subprocess, 'run', side_effect=mocked_run) as run, \
                patch.object(subprocess, 'Popen', side_effect=AssertionError('real child forbidden')) as popen:
            result = recipe.run_recipe(REPO, manifest_path, self.root / (label + '.evidence'))
            popen.assert_not_called()
        return result, reached, run.call_count

    def valid_control(self, label):
        """Require real checking plus the inert second step before any fault."""
        result, reached, count = self.run_inert_recipe(label)
        self.assertEqual((result['status'], reached, count), ('PASS', ['next-step'], 2))

    def test_missing_prerequisite_stops_before_next_step(self):
        """A missing pinned ELF refuses before any later recipe launch."""
        self.valid_control('valid-before-missing')
        self.fault_file.unlink()
        result, reached, count = self.run_inert_recipe('missing')
        self.assertEqual((result['status'], reached, count), ('FAIL', [], 1))
        self.assertEqual(len(result['steps']), 1)

    def test_wrong_hash_stops_before_next_step(self):
        """Existing but changed bytes refuse; presence alone is insufficient."""
        self.valid_control('valid-before-hash')
        self.fault_file.write_text('changed inert bytes')
        result, reached, count = self.run_inert_recipe('hash-mismatch')
        self.assertEqual((result['status'], reached, count), ('FAIL', [], 1))
        self.assertEqual(len(result['steps']), 1)

    def test_step_and_storage_failure_preserve_both_causes(self):
        """A failed child double and post-check failure retain original objects."""
        self.valid_control('valid-before-secondary-error')
        primary = RuntimeError('inert step failure')
        secondary = OSError('inert post-check failure')
        @contextlib.contextmanager
        def failing_admission(root, target):
            class Guard:
                calls = 0
                def check(inner):
                    inner.calls += 1
                    if inner.calls == 2:
                        raise secondary
            yield Guard()
        def fail(context):
            raise primary
        with self.assertRaises(BaseExceptionGroup) as caught:
            gate.gated(self.manifest, fail, admission=failing_admission, space=self.space)
        self.assertIs(caught.exception.exceptions[0], primary)
        self.assertIs(caught.exception.exceptions[1], secondary)

    def test_changed_dependency_inventory_stops_before_next_step(self):
        """Removing inventory entries cannot turn fewer hashes into qualification."""
        self.valid_control('valid-before-inventory-change')
        path = Path(self.manifest['compiled']) / 'actual-source-dependencies.json'
        self.write(path, {})
        result, reached, count = self.run_inert_recipe('changed-inventory')
        self.assertEqual((result['status'], reached, count), ('FAIL', [], 1))
        self.assertEqual(len(result['steps']), 1)

    def test_zero_reserve_refused_by_shared_policy(self):
        """A coherent but invalid policy must fail the existing strict validator."""
        self.valid_control('valid-before-zero-reserve')
        path = self.root / '.cargo/storage-policy.json'
        self.write(path, dict(schema_version=1, minimum_free_bytes=0))
        self.manifest['receipts'][str(path)] = gate.digest(path)
        result, reached, count = self.run_inert_recipe('zero-reserve')
        self.assertEqual((result['status'], reached, count), ('FAIL', [], 1))

    def test_successful_step_cannot_change_immutable_elf(self):
        """A successful step double cannot conceal changed compiled bytes."""
        self.valid_control('valid-before-successful-source-drift')
        def mutate(context):
            self.fault_file.write_text('unexpected immutable change')
            return 0
        with self.assertRaisesRegex(ValueError, 'preflight-elf-hash'):
            gate.gated(self.manifest, mutate, admission=self.admitted, space=self.space)

    def test_step_and_immutable_failure_preserve_both_causes(self):
        """Retain child failure alongside changed immutable bytes after it."""
        self.valid_control('valid-before-failed-source-drift')
        primary = RuntimeError('inert primary step failure')
        def mutate_and_fail(context):
            self.fault_file.write_text('unexpected immutable change')
            raise primary
        with self.assertRaises(BaseExceptionGroup) as caught:
            gate.gated(self.manifest, mutate_and_fail, admission=self.admitted, space=self.space)
        self.assertIs(caught.exception.exceptions[0], primary)
        self.assertIsInstance(caught.exception.exceptions[1], ValueError)
        self.assertEqual(str(caught.exception.exceptions[1]), 'preflight-elf-hash')


if __name__ == '__main__':
    unittest.main(verbosity=2)
