"""Condition S : logique pure de surveillance et de lecture des remises, sans binaire swarm."""
import unittest
from pathlib import Path

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
        for fault, code in (("F1", 3), ("F5", 3), ("F3", 137), ("F4", 4)):
            self.assertIsNone(run_s.infrastructure_fault(work(), [agent(code, "failed")], fault), fault)

    def test_running_agent_without_exit_code(self):
        self.assertIsNone(run_s.infrastructure_fault(work(), [agent(None, "running")], "none"))


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

    def test_no_handoff_for_accepted_attempt(self):
        w = prepared()
        w["planning"]["inbox"].pop()
        with self.assertRaises(provider_s.Unexpected):
            provider_s.accepted_handoff(w, "prepare")


if __name__ == "__main__":
    unittest.main()
