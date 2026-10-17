"""Regression checks for the actual checker main, without launching a Go build."""

import contextlib
import io
import json
from pathlib import Path
import runpy
import subprocess
import unittest
from unittest.mock import patch


CHECKER = Path(__file__).with_name("check.py")
PACKAGE = "swarm.local/companion/internal/engine"
TEST = "TestAgentWorkflowRolesAreBounded"


def events(*items):
    return "".join(json.dumps({"Package": PACKAGE, **item}) + "\n" for item in items)


class WorkflowCheckerEvidenceTests(unittest.TestCase):
    def execute(self, output, returncode=0):
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch("subprocess.run", return_value=subprocess.CompletedProcess([], returncode, output, "")) as launch:
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                try:
                    runpy.run_path(str(CHECKER), run_name="__main__")
                    code = 0
                except SystemExit as exc:
                    code = exc.code
        self.assertIn("0", launch.call_args.args[0])
        self.assertNotIn("30s", launch.call_args.args[0])
        return code, stdout.getvalue(), stderr.getvalue()

    def test_rejects_empty_success(self):
        code, stdout, stderr = self.execute(events({"Action": "output", "Output": "[no tests to run]\n"}, {"Action": "pass"}))
        self.assertNotEqual(code, 0, (stdout, stderr))
        self.assertNotIn("OK:", stdout)

    def test_rejects_incomplete_or_wrong_evidence(self):
        for label, output in {
            "wrong-name": events({"Action": "pass", "Test": TEST + "Other"}, {"Action": "pass"}),
            "skip": events({"Action": "skip", "Test": TEST}, {"Action": "pass"}),
            "missing-pass": events({"Action": "run", "Test": TEST}, {"Action": "pass"}),
            "missing-package": events({"Action": "run", "Test": TEST}, {"Action": "pass", "Test": TEST}),
            "failed-child": events({"Action": "run", "Test": TEST}, {"Action": "fail", "Test": TEST + "/child"}, {"Action": "pass", "Test": TEST}, {"Action": "pass"}),
            "malformed-json": "not json\n",
            "wrong-type": "[]\n",
        }.items():
            with self.subTest(label=label):
                code, stdout, stderr = self.execute(output)
                self.assertNotEqual(code, 0, (stdout, stderr))
                self.assertNotIn("OK:", stdout)

    def test_accepts_named_completed_evidence(self):
        output = events({"Action": "run", "Test": TEST}, {"Action": "pass", "Test": TEST + "/worker"}, {"Action": "pass", "Test": TEST}, {"Action": "pass"})
        code, stdout, stderr = self.execute(output)
        self.assertEqual(code, 0, stderr)
        self.assertIn("OK:", stdout)
        self.assertIn("2 tests/subtests", stdout)

    def test_preserves_go_failure(self):
        code, stdout, stderr = self.execute("original compiler failure\n", 7)
        self.assertEqual(code, 7)
        self.assertIn("original compiler failure", stderr)
        self.assertNotIn("OK:", stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
