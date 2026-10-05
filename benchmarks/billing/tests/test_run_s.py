"""Condition S, cas nominal : intégration Swarm réelle, uniquement avec BANC_SWARM=1."""
import os
import shutil
import unittest

from bench import harness, run_s


@unittest.skipUnless(os.environ.get("BANC_SWARM") == "1", "intégration Swarm : relancer avec BANC_SWARM=1")
class RunSTest(unittest.TestCase):
    def setUp(self):
        self.assertTrue(run_s.SWARM.exists(), f"binaire absent : {run_s.SWARM} (tâche 0)")

    def test_negative_control_settles_everything_once(self):
        try:
            r = run_s.run("business", "none", seed=11)
        except harness.BenchError as e:   # le test nettoie aussi la racine d'une ERREUR, puis la signale
            shutil.rmtree(e.run_dir, ignore_errors=True)
            raise
        self.addCleanup(shutil.rmtree, r["run_dir"], ignore_errors=True)   # run() garde ses preuves, pas les tests
        self.assertEqual(r["status"], "OK", r)
        self.assertTrue(r["correct"] and r["declared_success"], r)
        self.assertEqual(r["task_status"], {"prepare": "accepted", "settle": "accepted"})
