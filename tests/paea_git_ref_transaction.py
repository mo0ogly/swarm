#!/usr/bin/env python3
"""Qualify local Git reference primitives in disposable bare repositories.

No project checkout, mission, provider or PAEA authority is mutated. This is a
protocol experiment, not a production effect service. Real Git locks are held
between prepare and commit/abort; no global watchdog or assumed Git deadline.
Fixture commits are synthetic object bytes, never project commits/attribution.
Exceptions fail the check; owned children/pipes are explicitly reaped. Production
integration still owes owned cancellation, durable intent and uncertain recovery.
"""
import os
import hashlib
import json
import shutil
from pathlib import Path
import subprocess
import tempfile
import unittest


GIT = shutil.which('git')
if GIT is None:
    raise FileNotFoundError('git executable required')
GIT = str(Path(GIT).resolve())


class GitFixture(unittest.TestCase):
    """Own one isolated bare repository and all synchronous fixed Git commands."""
    def setUp(self):
        """Create an owned bare fixture, synthetic commits and the initial direct ref."""
        self.tmp = tempfile.TemporaryDirectory(prefix='paea-git-transaction-')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.env = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
        self.env.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null',
                        GIT_TERMINAL_PROMPT='0')
        self.command('init', '--bare', '.')
        tree = self.command('hash-object', '-t', 'tree', '-w', '--stdin', raw=b'').stdout.strip().decode()
        self.old = self.object(tree, 'old')
        self.new = self.object(tree, 'new', self.old)
        self.other = self.object(tree, 'other', self.old)
        self.ref = 'refs/heads/target'
        self.command('update-ref', self.ref, self.old)

    def argv(self, *args):
        """Fixed fixture runner suppresses inherited Git config and hooks."""
        return [GIT, '-c', 'core.hooksPath=/dev/null', '-c', 'core.fsync=all', *args]

    def command(self, *args, raw=None, success=True):
        """Run/reap a fixed operation; unexpected failure retains stderr in assert."""
        result = subprocess.run(self.argv(*args), input=raw, cwd=self.root,
                                env=self.env, capture_output=True)
        if success:
            self.assertEqual(result.returncode, 0, result.stderr)
        return result

    def object(self, tree, message, parent=None):
        """Synthetic valid commit object, with explicit fictional fixture metadata."""
        body = f'tree {tree}\n' + (f'parent {parent}\n' if parent else '')
        body += ('author Fixture <fixture@invalid.example> 1 +0000\n'
                 'committer Fixture <fixture@invalid.example> 1 +0000\n\n')
        return self.command('hash-object', '-t', 'commit', '-w', '--stdin',
                            raw=(body + message + '\n').encode()).stdout.strip().decode()

    def value(self, ref=None):
        """Observe current resolved OID; an absent ref is never empty success."""
        return self.command('rev-parse', '--verify', ref or self.ref).stdout.strip().decode()

    def transaction(self, expected, decide, new=None):
        """Hold Git's prepared no-deref lock while trusted callback decides.

        Prepare refusal yields False. The callback reads original reference type;
        True commits, False aborts. Acknowledgments, child exit and stderr must
        match. Finally closes stdin and reaps even when an assertion fails; no
        production rollback or no-effect claim follows from an exception here.
        """
        child = subprocess.Popen(self.argv('update-ref', '--no-deref', '--stdin'),
                                 cwd=self.root, env=self.env, stdin=subprocess.PIPE,
                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            child.stdin.write((f'start\nupdate {self.ref} {new or self.new} {expected}\nprepare\n').encode())
            child.stdin.flush()
            self.assertEqual(child.stdout.readline(), b'start: ok\n')
            acknowledgment = child.stdout.readline()
            if acknowledgment != b'prepare: ok\n':
                self.assertEqual(acknowledgment, b'')
                child.stdin.close()
                self.assertNotEqual(child.wait(), 0)
                self.assertTrue(child.stderr.read())
                return False
            publish = decide()
            action = b'commit' if publish else b'abort'
            child.stdin.write(action + b'\n')
            child.stdin.flush()
            self.assertEqual(child.stdout.readline(), action + b': ok\n')
            child.stdin.close()
            self.assertEqual(child.wait(), 0)
            self.assertEqual(child.stdout.read(), b'')
            self.assertEqual(child.stderr.read(), b'')
            return publish
        finally:
            if not child.stdin.closed:
                child.stdin.close()
            child.wait()
            child.stdout.close()
            child.stderr.close()

    def direct(self):
        """Classify original ref under prepared lock, refusing every error but1.

        Git symbolic-ref --quiet documents1 for a non-symbolic ref. This fixture
        also checks the exact old resolved OID independently; future production
        code must retain and validate its actual repository descriptor.
        """
        result = self.command('symbolic-ref', '--quiet', '--no-recurse', self.ref, success=False)
        self.assertIn(result.returncode, [0, 1], result.stderr)
        self.assertEqual(self.value(), self.old)
        return result.returncode == 1

    def control(self):
        """Valid prepared publication then conditional restoration before faults."""
        self.assertTrue(self.transaction(self.old, self.direct))
        self.assertEqual(self.value(), self.new)
        self.command('update-ref', self.ref, self.old, self.new)

    def test_direct_prepared_publication_and_explicit_abort(self):
        """Observe real publication, explicit abort without change, then recovery."""
        self.control()
        self.assertFalse(self.transaction(self.old, lambda: False))
        self.assertEqual(self.value(), self.old)
        self.assertTrue(self.transaction(self.old, self.direct))
        self.assertEqual(self.value(), self.new)

    def test_unsafe_no_deref_alone_overwrites_symbolic_ref(self):
        """Retain the unsafe primitive counterexample; this is never a service PASS."""
        self.control()
        self.command('update-ref', 'refs/heads/indirect', self.old)
        self.command('symbolic-ref', self.ref, 'refs/heads/indirect')
        self.command('update-ref', '--no-deref', self.ref, self.new, self.old)
        self.assertEqual(self.command('symbolic-ref', '-q', self.ref, success=False).returncode, 1)
        self.assertEqual(self.value(), self.new)
        self.assertEqual(self.value('refs/heads/indirect'), self.old)

    def test_symbolic_ref_refused_under_prepared_lock_then_recovers(self):
        """Refuse a symbolic ref while locked, preserve both refs, then recover."""
        self.control()
        self.command('update-ref', 'refs/heads/indirect', self.old)
        self.command('symbolic-ref', self.ref, 'refs/heads/indirect')
        self.assertFalse(self.transaction(self.old, self.direct))
        self.assertEqual(self.command('symbolic-ref', '-q', self.ref).stdout, b'refs/heads/indirect\n')
        self.assertEqual(self.value('refs/heads/indirect'), self.old)
        self.command('update-ref', '--no-deref', self.ref, self.old, self.old)
        self.assertTrue(self.transaction(self.old, self.direct))

    def test_decision_failure_closes_transaction_without_publication(self):
        """Raise after prepare, verify EOF abort/no lock, then publish successfully."""
        self.control()
        def refused():
            """Local diagnostic failure occurs after prepare, before commit input."""
            raise ValueError('fixture-decision-refusal')
        with self.assertRaisesRegex(ValueError, 'fixture-decision-refusal'):
            self.transaction(self.old, refused)
        self.assertEqual(self.value(), self.old)
        self.assertFalse((self.root / (self.ref + '.lock')).exists())
        self.assertTrue(self.transaction(self.old, self.direct))

    def test_wrong_old_oid_refuses_then_recovers(self):
        """Reject stale expected content before decision, preserve ref and recover."""
        self.control()
        self.assertFalse(self.transaction(self.other, lambda: self.fail('must not reach decision')))
        self.assertEqual(self.value(), self.old)
        self.assertTrue(self.transaction(self.old, self.direct))

    def test_prepared_lock_prevents_competing_symbolic_and_direct_updates(self):
        """Observe the real lock rejecting both competing writer kinds before commit."""
        self.control()
        def decide():
            """Faults occur only while actual prepared lock is observed."""
            self.assertTrue(self.direct())
            self.assertTrue((self.root / (self.ref + '.lock')).is_file())
            self.assertNotEqual(self.command('update-ref', self.ref, self.other, self.old, success=False).returncode, 0)
            self.assertNotEqual(self.command('symbolic-ref', self.ref, 'refs/heads/elsewhere', success=False).returncode, 0)
            self.assertTrue(self.direct())
            return True
        self.assertTrue(self.transaction(self.old, decide))
        self.assertEqual(self.value(), self.new)
        self.assertFalse((self.root / (self.ref + '.lock')).exists())


if __name__ == '__main__':
    tool_hash = hashlib.sha256(Path(GIT).read_bytes()).hexdigest()
    print(json.dumps({'git_executable': GIT, 'sha256': tool_hash,
                      'version': subprocess.check_output([GIT, '--version']).decode().strip()}), flush=True)
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(GitFixture)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    assert hashlib.sha256(Path(GIT).read_bytes()).hexdigest() == tool_hash, 'Git executable changed during probe'
    if result.wasSuccessful() and result.testsRun == 6:
        print('PAEA_GIT_REF_TRANSACTION_VERIFIED 6')
    raise SystemExit(0 if result.wasSuccessful() and result.testsRun == 6 else 1)
