"""Responsable scripté de la condition S : revue, clôture et reprise bornée unique après un code 137."""
import unittest

from bench import planner_fixture

CRASH = "Processus en échec (code 137) ; consulter les journaux ; handoff et validation requis"


def task(tid, status, used=1, blocker="", next_="Régler le lot"):
    return {"id": tid, "status": status, "attempts_used": used, "attempts_max": 2, "blocker": blocker, "next": next_}


def context(*tasks):
    return {"events": [{"id": "e1"}, {"id": "e2"}], "tasks": list(tasks)}


class RespondTest(unittest.TestCase):
    def test_review_passes_non_empty_report(self):
        r = planner_fixture.respond({"report": "lot", "criteria": ["c1", "c2"]})
        self.assertEqual([c["verdict"] for c in r["criteria"]], ["pass", "pass"])

    def test_closes_when_all_accepted(self):
        r = planner_fixture.respond(context(task("prepare", "accepted"), task("settle", "accepted")))
        self.assertEqual(r["operations"], [{"kind": "close"}])
        self.assertEqual(r["input_events"], ["e1", "e2"])

    def test_single_retry_after_crash_137(self):
        r = planner_fixture.respond(context(task("prepare", "accepted"), task("settle", "blocked", 1, CRASH)))
        self.assertEqual(len(r["operations"]), 1)
        op = r["operations"][0]
        self.assertEqual((op["kind"], op["id"]), ("retry", "settle"))
        self.assertTrue(op["next"] and op["next"] != "Régler le lot")

    def test_no_second_retry(self):
        r = planner_fixture.respond(context(task("settle", "blocked", 2, CRASH)))
        self.assertEqual(r["operations"], [])

    def test_no_retry_for_other_failures(self):
        blocker = "Processus en échec (code 4) ; consulter les journaux ; handoff et validation requis"
        r = planner_fixture.respond(context(task("settle", "blocked", 1, blocker)))
        self.assertEqual(r["operations"], [])


if __name__ == "__main__":
    unittest.main()
