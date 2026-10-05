"""Condition S hiérarchique, cas nominal : intégration Swarm réelle, uniquement avec BANC_SWARM=1.

Clé métier, aucune faute. Le binaire mesuré doit exister (`make build` à la racine du dépôt) :
son absence est un échec, pas un saut.
"""
import json
import os
import shutil
import unittest
from pathlib import Path

from bench import harness, run_s


@unittest.skipUnless(os.environ.get("BANC_SWARM") == "1", "intégration Swarm : relancer avec BANC_SWARM=1")
class RunSTest(unittest.TestCase):
    def setUp(self):
        self.assertTrue(run_s.SWARM.is_file(), f"binaire absent : {run_s.SWARM} (make build)")

    def test_nominal_business_key_settles_everything_once(self):
        try:
            r = run_s.run("business", "none", seed=11)
        except harness.BenchError as e:   # le test nettoie aussi la racine d'une ERREUR, puis la signale
            shutil.rmtree(e.run_dir, ignore_errors=True)
            raise
        self.addCleanup(shutil.rmtree, r["run_dir"], ignore_errors=True)   # run() garde ses preuves, pas les tests
        self.assertEqual(r["status"], "OK", r)
        self.assertTrue(r["correct"] and r["declared_success"], r)
        self.assertEqual(r["task_status"], {"prepare": "accepted", "settle": "accepted"}, r)
        self.assertEqual(r["scope_state"], "closed", r)
        handoffs = r["handoffs"]
        self.assertEqual(len(handoffs), 2, handoffs)
        self.assertTrue(all(h["attempt"] and h["artifacts"] for h in handoffs), handoffs)
        self.assertEqual(sorted(h["task"] for h in handoffs), ["prepare", "settle"], handoffs)
        # E3 : le règlement a consommé la remise de la tentative acceptée de prepare, à son empreinte.
        bilan = json.loads((Path(r["run_dir"]) / "swarm" / "docs" / "settle.md").read_text())
        remise = next(h for h in handoffs if h["task"] == "prepare")
        self.assertEqual(bilan["prepare_attempt"], r["accepted_attempts"]["prepare"], bilan)
        self.assertEqual(remise["attempt"], r["accepted_attempts"]["prepare"], remise)
        self.assertEqual(bilan["lot_sha256"], remise["artifacts"][0]["sha256"], (bilan, remise))


if __name__ == "__main__":
    unittest.main()
