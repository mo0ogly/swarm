"""Guard the inert manual fixture against concurrent planner/worker confusion.

These subprocesses have no Swarm database, real provider or role authority.
The public mission recipe separately checks actual grants and recorded effects.
"""
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
import rust_mission_manual_review as manual


class ProviderRoles(unittest.TestCase):
    def invoke(self, directory, prompt):
        """Run frozen fixture bytes; retain child stderr in assertion failures.

        No watchdog is imposed. Each call uses the same executable and cwd, as
        the real planner and manual worker do; only their wire input differs.
        """
        script = Path(directory) / 'codex'
        if not script.exists():
            script.write_text(manual.provider_program())
            script.chmod(0o700)
        return subprocess.run([str(script), '--model', 'fixture-standard'],
                              input=prompt, cwd=directory, capture_output=True)

    def test_current_planner_delivery_and_worker_use_same_frozen_program(self):
        with tempfile.TemporaryDirectory() as directory:
            context = dict(scope=dict(id='child'), events=[dict(id='current', scope='child')])
            prompt = 'Tu es le planificateur du périmètre reçu.\n\n'.encode() + json.dumps(context).encode()
            planner = self.invoke(directory, prompt)
            self.assertEqual(planner.returncode, 0, planner.stderr)
            proposal = json.loads(planner.stdout)['structured_output']
            self.assertEqual(proposal['input_events'], ['current'])
            self.assertEqual(proposal['operations'], [])
            self.assertFalse((Path(directory) / 'handoff.md').exists())
            original = (Path(directory) / 'codex').read_bytes()
            worker = self.invoke(directory, b'Owned manual worker prompt')
            self.assertEqual(worker.returncode, 0, worker.stderr)
            self.assertIn('Input SHA256:', (Path(directory) / 'handoff.md').read_text())
            self.assertEqual((Path(directory) / 'codex').read_bytes(), original)

    def test_malformed_planner_never_falls_back_to_successful_worker(self):
        with tempfile.TemporaryDirectory() as directory:
            result = self.invoke(directory, 'Tu es le planificateur du périmètre reçu.\n\n{}'.encode())
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b'KeyError', result.stderr)
            self.assertFalse((Path(directory) / 'handoff.md').exists())


if __name__ == '__main__':
    unittest.main()
