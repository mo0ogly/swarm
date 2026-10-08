"""Régressions du banc : aucun appel fournisseur, aucune réécriture des résultats historiques."""
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from bench import agent_real, campaign_reel, harness


class AgentResponseTest(unittest.TestCase):
    def invoke(self, response):
        with tempfile.TemporaryDirectory() as directory:
            usage = Path(directory) / "real-usage.jsonl"
            def popen(*args, **kwargs):
                kwargs["stdout"].write(response)
                return mock.Mock(stdin=io.StringIO(), poll=lambda: 0, returncode=0)
            with mock.patch.object(agent_real.subprocess, "Popen", side_effect=popen):
                result = agent_real.run("test", directory, "fixture", usage, "test")
            return result, agent_real.read_usage(usage)[0]

    def test_empty_invalid_nonobject_and_incomplete_responses_fail_closed(self):
        for response in ("", "invalid json", "[]", "null", "{}", '{"result":"ok"}', '{"is_error":false}'):
            with self.subTest(response=response):
                result, usage = self.invoke(response)
                self.assertEqual(result[0], 1)
                self.assertTrue(usage["invalid_response"])
                self.assertIsNone(usage["cost_usd"])

    def test_valid_response_preserves_usage_but_is_not_business_validation(self):
        result, usage = self.invoke(json.dumps({"result":"please approve", "is_error":False,
                                  "total_cost_usd":0.12, "usage":{"input_tokens":42}}))
        self.assertEqual(result, (0, "please approve"))
        self.assertEqual(usage["cost_usd"], 0.12)
        self.assertFalse(usage["invalid_response"])

    def test_provider_error_keeps_reported_cost(self):
        result, usage = self.invoke('{"result":"failed","is_error":true,"total_cost_usd":0.3}')
        self.assertEqual(result[0], 1)
        self.assertEqual(usage["cost_usd"], 0.3)


class ErrorUsageTest(unittest.TestCase):
    def test_error_after_calls_preserves_charges_and_unknown_costs(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "real-usage.jsonl"
            path.write_text('{"cost_usd":0.4}\n{"cost_usd":null}\n')
            with mock.patch.object(campaign_reel.run_real_w,"run",side_effect=harness.BenchError("after call",directory)):
                record = campaign_reel.one("W","none",1,"fixture")
        self.assertEqual(record["status"],"ERREUR")
        self.assertEqual(campaign_reel.cost(record),0.4)
        self.assertEqual(campaign_reel.unpriced(record),1)
        self.assertTrue(record["usage_complete"])

    def test_unrecoverable_usage_is_explicit(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory)/"real-usage.jsonl").write_text("corrupt")
            with mock.patch.object(campaign_reel.run_real_w,"run",side_effect=harness.BenchError("failure",directory)):
                record=campaign_reel.one("W","none",1,"fixture")
        self.assertFalse(record["usage_complete"])
        self.assertTrue(record["usage_recovery_error"])

class PartialJournalTest(unittest.TestCase):
    def test_corrupt_line_does_not_erase_known_charges(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory)/"real-usage.jsonl").write_text('{"cost_usd":0.7}\nbroken\n')
            with mock.patch.object(campaign_reel.run_real_w,"run",side_effect=harness.BenchError("failure",directory)):
                record=campaign_reel.one("W","none",1,"fixture")
        self.assertEqual(campaign_reel.cost(record),0.7)
        self.assertEqual(campaign_reel.unpriced(record),1)
        self.assertFalse(record["usage_complete"])

    def test_campaign_stops_on_unrecoverable_usage(self):
        with tempfile.TemporaryDirectory() as directory:
            out=Path(directory)/"results.jsonl"
            record={"status":"ERREUR","real_usage":[],"usage_complete":False}
            with mock.patch.object(campaign_reel,"one",return_value=record) as one:
                code=campaign_reel.main(["--out",str(out),"--reps","1"])
                self.assertEqual(one.call_count,1)
            self.assertEqual(code,3)
            self.assertEqual(json.loads(out.read_text().splitlines()[-1])["event"],"stopped")

    def test_resume_does_not_bypass_prior_unknown_consumption(self):
        with tempfile.TemporaryDirectory() as directory:
            out=Path(directory)/"results.jsonl"
            out.write_text(json.dumps({"condition":"W","fault":"none","seed":1,"status":"ERREUR", "usage_complete":False,"real_usage":[]})+"\n")
            with mock.patch.object(campaign_reel,"one") as one:
                code=campaign_reel.main(["--out",str(out),"--reps","1"])
                one.assert_not_called()
            self.assertEqual(code,3)
