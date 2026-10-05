"""Condition S : logique pure de surveillance et de lecture des remises, sans binaire swarm."""
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from bench import config, provider_s, run_s

SHA = "a" * 64


def work(**planning):
    base = {"failure": "", "decisions": 1, "max_decisions": 30, "activations": 1, "max_activations": 40,
            "reviewer": {"failure": "", "provider": "fixture-planner"},
            "scopes": [{"id": "root", "state": "waiting", "holder": ""}], "inbox": []}
    base.update(planning)
    return {"revision": 3, "tasks": [], "planning": base}


def agent(code, status="completed"):
    return {"id": "auto-1", "status": status, "exit_code": code}


class InfrastructureFaultTest(unittest.TestCase):
    def test_healthy_run_has_no_fault(self):
        self.assertIsNone(run_s.infrastructure_fault(work(), [agent(0)], "none"))

    def test_planning_failure_is_a_bench_error(self):
        self.assertIn("planification", run_s.infrastructure_fault(work(failure="budget"), [], "none"))

    def test_reviewer_failure_is_a_bench_error(self):
        reason = run_s.infrastructure_fault(work(reviewer={"failure": "sortie illisible"}), [], "none")
        self.assertIn("vérificateur", reason)

    def test_exhausted_decision_budget_with_pending_event(self):
        w = work(decisions=30, inbox=[{"id": "e", "scope": "root", "kind": "handoff"}])
        self.assertIn("décisions", run_s.infrastructure_fault(w, [], "none"))

    def test_exhausted_activation_budget_with_pending_event(self):
        w = work(activations=40, inbox=[{"id": "e", "scope": "root", "kind": "handoff"}])
        self.assertIn("activations", run_s.infrastructure_fault(w, [], "none"))

    def test_exhausted_budget_without_pending_event_is_not_a_fault(self):
        w = work(decisions=30, inbox=[{"id": "e", "scope": "root", "kind": "handoff", "decision": "d"}])
        self.assertIsNone(run_s.infrastructure_fault(w, [], "none"))

    def test_unexpected_provider_exit_code(self):
        self.assertIn("code 3", run_s.infrastructure_fault(work(), [agent(3, "failed")], "none"))
        self.assertIn("code 5", run_s.infrastructure_fault(work(), [agent(5, "failed")], "none"))

    def test_exit_code_expected_by_fault(self):
        for fault, code in (("F3", 137), ("F4", 4)):
            self.assertIsNone(run_s.infrastructure_fault(work(), [agent(code, "failed")], fault), fault)

    def test_codes_never_observed_in_exploration_are_errors(self):
        """Exploration du 2026-10-05 (observations.md) : ni 3 sous F1/F5, ni refus métier 2 dans aucune faute."""
        for fault, code in (("F1", 3), ("F5", 3), ("none", 2), ("F4", 2), ("F8", 2)):
            self.assertIsNotNone(run_s.infrastructure_fault(work(), [agent(code, "failed")], fault), (fault, code))

    BUDGET = "Limite d'appels d'outils atteinte ; fin du processus confirmée"

    def guard(self, task="prepare", activity=BUDGET):
        return {"id": "auto-1", "task_id": task, "status": "interrupted", "stop_kind": "garde", "exit_code": -1,
                "activity": activity}

    def test_guard_interruption_expected_only_under_f6(self):
        self.assertIsNone(run_s.infrastructure_fault(work(), [self.guard()], "F6"))
        self.assertIn("code -1", run_s.infrastructure_fault(work(), [self.guard()], "none"))

    def test_f6_exemption_requires_prepare_task(self):
        self.assertIsNotNone(run_s.infrastructure_fault(work(), [self.guard(task="settle")], "F6"))

    def test_f6_exemption_requires_budget_reason(self):
        other = self.guard(activity="Limite de répétitions identiques atteinte ; fin du processus confirmée")
        self.assertIsNotNone(run_s.infrastructure_fault(work(), [other], "F6"))

    def test_running_agent_without_exit_code(self):
        self.assertIsNone(run_s.infrastructure_fault(work(), [agent(None, "running")], "none"))


def settle_agent(code, status="failed"):
    return {"id": "auto-s", "task_id": "settle", "status": status, "exit_code": code}


class SettleOutcomeTest(unittest.TestCase):
    def test_engine_stopped_settlement_never_launched(self):
        prep = {"id": "auto-p", "task_id": "prepare", "status": "completed", "exit_code": 0}
        self.assertEqual(run_s.settle_outcome([prep], "F4"),
                         {"settle_launched": False, "settle_exit_codes": [], "stopped_by": "engine"})

    def test_settlement_refused_modified_lot(self):
        self.assertEqual(run_s.settle_outcome([settle_agent(4)], "F4"),
                         {"settle_launched": True, "settle_exit_codes": [4], "stopped_by": "settlement"})

    def test_nothing_stopped_payment(self):
        self.assertEqual(run_s.settle_outcome([settle_agent(0, "completed")], "F4")["stopped_by"], "none")

    def test_stopped_by_only_meaningful_under_f4(self):
        self.assertIsNone(run_s.settle_outcome([settle_agent(0, "completed")], "none")["stopped_by"])

    def test_stopped_by_also_under_f4e(self):
        self.assertEqual(run_s.settle_outcome([], "F4e")["stopped_by"], "engine")

    def test_exit_codes_in_launch_order(self):
        # agent list renvoie le plus récent d'abord (agents_store.go : ORDER BY ... DESC)
        agents = [settle_agent(0, "completed"), settle_agent(137)]
        self.assertEqual(run_s.settle_outcome(agents, "F3")["settle_exit_codes"], [137, 0])


class OrderProofTest(unittest.TestCase):
    """Preuves d'ordre exigées pour F4e et F7 ; sans elles, l'exécution est INVALIDE."""
    ACCEPTED = "2026-10-05T14:53:37.758000000Z"   # 1791212017.758

    def test_f4e_settle_never_launched(self):
        proof = run_s.f4e_order(self.ACCEPTED, 1791212017.9, None)
        self.assertTrue(proof["proven"], proof)

    def test_f4e_settle_launched_after_tamper(self):
        self.assertTrue(run_s.f4e_order(self.ACCEPTED, 1791212017.9, "2026-10-05T14:53:38.000Z")["proven"])

    def test_f4e_settle_launched_before_tamper(self):
        self.assertFalse(run_s.f4e_order(self.ACCEPTED, 1791212017.9, "2026-10-05T14:53:37.774Z")["proven"])

    def test_f4e_tamper_before_acceptance(self):
        self.assertFalse(run_s.f4e_order(self.ACCEPTED, 1791212017.0, None)["proven"])

    def test_f4e_missing_acceptance(self):
        self.assertFalse(run_s.f4e_order(None, 1791212017.9, None)["proven"])

    def f7(self, started="2026-10-05T14:57:20.000Z", launcher="conductor-b", old="conductor-a", new="conductor-b",
           source="serveur web"):
        return run_s.f7_order(1791212235.6, started, launcher, old, new, source)["proven"]

    def test_f7_ephemeral_release_holder_is_not_the_second_conductor(self):
        """dispatcher.go : dispatchAfterSettle prend un bail éphémère « release-conductor-… »."""
        self.assertFalse(self.f7(launcher="release-conductor-x", new="release-conductor-x",
                                 source="libération de ressource"))
        self.assertFalse(self.f7(source="libération de ressource"))

    def test_f7_settle_after_takeover_by_new_holder(self):
        self.assertTrue(self.f7())

    def test_f7_settle_before_takeover(self):
        self.assertFalse(self.f7(started="2026-10-05T14:56:40.148Z"))

    def test_f7_settle_never_launched(self):
        self.assertFalse(self.f7(started=None))

    def test_f7_settle_launched_by_old_conductor(self):
        self.assertFalse(self.f7(launcher="conductor-a"))

    def test_f7_no_takeover_observed(self):
        self.assertFalse(self.f7(new="conductor-a"))
        self.assertFalse(self.f7(new=None))

    def test_f4_tamper_after_settle_started(self):
        self.assertTrue(run_s.f4_order("2026-10-05T14:53:37.774Z", 1791212017.9)["proven"])

    def test_f4_tamper_before_settle_started(self):
        self.assertFalse(run_s.f4_order("2026-10-05T14:53:38.000Z", 1791212017.9)["proven"])

    def test_f4_settle_never_started(self):
        self.assertFalse(run_s.f4_order(None, 1791212017.9)["proven"])

    def test_engine_stop_requires_stale_prepare(self):
        self.assertTrue(run_s.engine_stop_consistent("engine", "stale"))
        self.assertFalse(run_s.engine_stop_consistent("engine", "fresh"))
        self.assertFalse(run_s.engine_stop_consistent("engine", None))
        self.assertTrue(run_s.engine_stop_consistent("settlement", "fresh"))

    def test_f4e_settlement_digest_code_is_a_measure(self):
        self.assertIsNone(run_s.infrastructure_fault(work(), [agent(4, "failed")], "F4e"))


class CliRobustnessTest(unittest.TestCase):
    """Une sortie non JSON de la CLI est une erreur lisible, et le nettoyage arrête quand même les conducteurs."""

    def fake_swarm(self, output):
        d = Path(tempfile.mkdtemp(prefix="banc-cli-"))
        self.addCleanup(shutil.rmtree, d, ignore_errors=True)
        binary = d / "swarm"
        binary.write_text(f"#!/bin/sh\necho '{output}'\n")
        binary.chmod(0o700)
        return run_s.Swarm(binary, d)

    def test_non_json_output_is_a_readable_runtime_error(self):
        with self.assertRaises(RuntimeError) as caught:
            self.fake_swarm("pas du json").cli(["work", "list"])
        self.assertIn("pas du json", str(caught.exception))

    def test_stop_still_stops_conductors_when_cli_is_garbled(self):
        proc = subprocess.Popen(["sleep", "30"])
        errors = run_s.stop(self.fake_swarm("pas du json"), "w-1", [proc])
        self.assertIsNotNone(proc.poll())
        self.assertTrue(any("mission stop" in e for e in errors), errors)


class PlanningBusyTest(unittest.TestCase):
    def test_idle_scope(self):
        self.assertFalse(run_s.planning_busy(work()))

    def test_holder_present(self):
        self.assertTrue(run_s.planning_busy(work(scopes=[{"id": "root", "state": "ready", "holder": "planner-1"}])))

    def test_pending_event_on_open_scope(self):
        w = work(scopes=[{"id": "root", "state": "ready", "holder": ""}],
                 inbox=[{"id": "e", "scope": "root", "kind": "validation_changed"}])
        self.assertTrue(run_s.planning_busy(w))

    def test_closed_scope_is_idle(self):
        w = work(scopes=[{"id": "root", "state": "closed", "holder": ""}],
                 inbox=[{"id": "e", "scope": "root", "kind": "brief"}])
        self.assertFalse(run_s.planning_busy(w))


def prepared(handoff_sha=SHA, validated_sha=SHA, status="accepted"):
    return {"tasks": [{"id": "prepare", "status": status, "automatic_validation": {
                "state": "accepted", "attempt_id": "a-2",
                "artifacts": {"docs/prepare.md": validated_sha, ".swarm/validation/r.json": "b" * 64}}}],
            "planning": {"inbox": [
                {"kind": "handoff", "task": "prepare", "attempt": "a-1",
                 "artifacts": [{"path": "docs/prepare.md", "sha256": "c" * 64}]},
                {"kind": "handoff", "task": "prepare", "attempt": "a-2",
                 "artifacts": [{"path": "docs/prepare.md", "sha256": handoff_sha}]}]}}


class AcceptedHandoffTest(unittest.TestCase):
    def test_returns_handoff_of_accepted_attempt(self):
        attempt, artifact = provider_s.accepted_handoff(prepared(), "prepare")
        self.assertEqual((attempt, artifact["sha256"]), ("a-2", SHA))

    def test_handoff_digest_differs_from_validated_artifact(self):
        with self.assertRaises(provider_s.ValidationMismatch):
            provider_s.accepted_handoff(prepared(validated_sha="d" * 64), "prepare")

    def test_task_not_accepted(self):
        with self.assertRaises(provider_s.Unexpected):
            provider_s.accepted_handoff(prepared(status="blocked"), "prepare")

    def test_settle_exits_internal_on_validation_mismatch(self):
        """Incohérence du contrat du moteur : 5, jamais 4, réservé sous F4 au lot modifié."""
        w = prepared(validated_sha="d" * 64)
        w["tasks"].append({"id": "settle", "status": "running", "attempts": [{"id": "a-9", "status": "recorded"}]})

        class FakeSwarm:
            def work(self):
                return w
        code = provider_s.run_settle(FakeSwarm(), Path("/nonexistent"), Path("/nonexistent"), "http://127.0.0.1:1",
                                     "business", "none", "/nonexistent/token", Path("/nonexistent"))
        self.assertEqual(code, config.EXIT_INTERNAL)

    def test_prepare_exits_internal_when_f7_freeze_never_comes(self):
        class FakeSwarm:
            def work(self):
                return {"tasks": [{"id": "prepare", "status": "running", "attempts": [{"id": "a-1"}]}]}
        with tempfile.TemporaryDirectory() as d, mock.patch.object(provider_s.config, "S_GATE_TIMEOUT_S", 0.2):
            code = provider_s.run_prepare(FakeSwarm(), Path(d), "http://127.0.0.1:1", "F7", Path(d))
        self.assertEqual(code, config.EXIT_INTERNAL)

    def test_settle_exits_internal_when_f4_tamper_never_comes(self):
        """F4 : sans modification dans le délai, le règlement ne paie pas le lot intact ; erreur du banc."""
        class NoSwarm:
            def work(self):
                raise AssertionError("le règlement ne doit pas continuer")
        with tempfile.TemporaryDirectory() as d, mock.patch.object(provider_s.config, "S_GATE_TIMEOUT_S", 0.2):
            code = provider_s.run_settle(NoSwarm(), Path(d), Path(d), "http://127.0.0.1:1", "business", "F4",
                                         str(Path(d) / "token"), Path(d))
        self.assertEqual(code, config.EXIT_INTERNAL)

    def test_no_handoff_for_accepted_attempt(self):
        w = prepared()
        w["planning"]["inbox"].pop()
        with self.assertRaises(provider_s.Unexpected):
            provider_s.accepted_handoff(w, "prepare")


if __name__ == "__main__":
    unittest.main()
