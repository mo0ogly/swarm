"""Checker regressions use explicit native-harness doubles, no Cargo or model.

One owned Python signal helper supplies an actual negative wait status.
The separate permanent recipe exercises real Rust contexts. These tests prove
strict evidence parsing and error/CLI routing, never native execution by a double.
"""
import contextlib
import io
import os
import tempfile
from pathlib import Path
import runpy
import subprocess
import sys
import unittest
from unittest.mock import patch

CHECKER = Path(__file__).with_name("check.py")
MODULE = runpy.run_path(str(CHECKER), run_name="checker_test_target")
NAMES = sorted(MODULE['REQUIRED_TESTS'])


def output(names=None, total=None):
    """Build a coherent named-harness positive control before injected faults."""
    names = NAMES if names is None else names
    n = len(names) if total is None else total
    return (f"running {n} tests\n" + ''.join(f"test {name} ... ok\n" for name in names)
            + f"test result: ok. {n} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n")


class WorkflowCheckerEvidenceTests(unittest.TestCase):
    """Require each contract, safe failure propagation and no obsolete Go path."""
    def execute(self, raw, returncode=0):
        """Invoke actual main under a controlled process result and real static pack."""
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch('subprocess.run', return_value=subprocess.CompletedProcess([], returncode, raw, '')) as launch:
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                code = MODULE['main']([])
        argv = launch.call_args.args[0]
        self.assertEqual(argv[0], 'cargo'); self.assertNotIn('go', argv)
        self.assertIn('--offline', argv); self.assertIn('--locked', argv)
        self.assertIn('agent_workflow', argv); self.assertNotIn('timeout', launch.call_args.kwargs)
        return code, stdout.getvalue(), stderr.getvalue()

    def test_accepts_all_named_native_contracts(self):
        code, stdout, stderr = self.execute(output())
        self.assertEqual(code, 0, stderr); self.assertIn('4 native roles, 9 named native tests', stdout)

    def test_empty_collection_never_passes(self):
        self.assertEqual(self.execute(output())[0], 0)
        for raw in ['', output([]), 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0s\n']:
            with self.subTest(raw=raw):
                code, stdout, _ = self.execute(raw); self.assertNotEqual(code, 0); self.assertNotIn('OK:', stdout)

    def test_each_role_is_required_not_just_parent(self):
        self.assertEqual(self.execute(output())[0], 0)
        for role in MODULE['ROLES']:
            with self.subTest(role=role):
                names = [n for n in NAMES if n != 'frozen_role_' + role + '_contract']
                code, stdout, _ = self.execute(output(names)); self.assertNotEqual(code, 0); self.assertNotIn('OK:', stdout)

    def test_incoherent_failed_duplicate_or_ignored_evidence_refuses(self):
        self.assertEqual(self.execute(output())[0], 0)
        invalid = [output(total=10), output(NAMES + [NAMES[0]]),
                   output().replace(' ... ok', ' ... FAILED', 1),
                   output().replace(' ... ok', ' ... ignored', 1),
                   output().replace('0 ignored;', '1 ignored;'),
                   output().replace('0 filtered out;', '1 filtered out;'),
                   output() + output(), output(NAMES[:-1] + ['wrong_contract'])]
        for raw in invalid:
            with self.subTest(raw=raw):
                code, stdout, _ = self.execute(raw); self.assertNotEqual(code, 0); self.assertNotIn('OK:', stdout)

    def test_original_compile_and_interruption_status_survive(self):
        for status in (7, 130):
            with self.subTest(status=status):
                code, stdout, stderr = self.execute('original native compiler failure\n', status)
                self.assertEqual(code, status); self.assertIn('original native compiler failure', stderr); self.assertNotIn('OK:', stdout)

    def test_owned_signal_result_retains_original_and_cli_convention(self):
        """Actual owned Python child supplies SIGTERM status; Cargo is not killed.

        Only its process-result mapping is doubled into the checker. This proves
        signal diagnostic/exit conversion, not a Cargo cancellation or model call.
        """
        child = subprocess.run([sys.executable, '-c', 'import os,signal; os.kill(os.getpid(),signal.SIGTERM)'],
                               stdin=subprocess.DEVNULL, capture_output=True, text=True)
        self.assertEqual(child.returncode, -15)
        code, stdout, stderr = self.execute(child.stdout, child.returncode)
        self.assertEqual(code, 143); self.assertIn('native Cargo exit status -15', stderr); self.assertNotIn('OK:', stdout)

    def test_public_checker_native_shim_rejects_missing_role_without_go(self):
        """Public subprocess routing with a controlled Cargo shim and Go trap.

        A complete named witness precedes one omitted role. This checks wiring,
        not Rust execution; the permanent next step runs the real native tests.
        Only temporary scripts/environment passed to owned children are changed.
        """
        with tempfile.TemporaryDirectory(prefix='swarm-native-role-shim141-') as tmp:
            directory=Path(tmp); cargo=directory/'cargo'; go=directory/'go'
            cargo.write_text('#!'+sys.executable+'\nimport os,sys\nassert sys.argv[1]=="test" and "agent_workflow" in sys.argv\nsys.stdout.write(open(os.environ["SWARM_NATIVE_ROLE_FIXTURE"]).read())\n')
            go.write_text('#!'+sys.executable+'\nimport os\nfrom pathlib import Path\nPath(os.environ["SWARM_GO_TRAP"]).write_text("unexpected Go invocation")\nraise SystemExit(91)\n')
            cargo.chmod(0o700);go.chmod(0o700);marker=directory/'go-was-invoked'
            for names, expected in [(NAMES,0),([n for n in NAMES if n!='frozen_role_reviewer_contract'],1)]:
                frame=directory/'native-output.txt';frame.write_text(output(names))
                environment=dict(os.environ,PATH=str(directory)+os.pathsep+os.environ['PATH'],
                    SWARM_NATIVE_ROLE_FIXTURE=str(frame),SWARM_GO_TRAP=str(marker))
                result=subprocess.run([sys.executable,str(CHECKER)],cwd=CHECKER.resolve().parents[2],
                    env=environment,stdin=subprocess.DEVNULL,capture_output=True,text=True)
                self.assertEqual(result.returncode,expected,result.stdout+result.stderr)
                self.assertFalse(marker.exists(),'public checker attempted Go')
                if expected:self.assertNotIn('OK:',result.stdout)

    def test_go_delta_baseline_refuses_before_execution(self):
        stderr = io.StringIO()
        with patch('subprocess.run') as launch, contextlib.redirect_stderr(stderr):
            with self.assertRaises(SystemExit) as raised:
                MODULE['main'](['--evidence', 'new-native-proof', '--evidence-baseline', 'old-go-proof'])
        self.assertEqual(raised.exception.code, 2); launch.assert_not_called()
        self.assertIn('Go delta baselines cannot qualify native contexts', stderr.getvalue())


if __name__ == '__main__': unittest.main(verbosity=2)
