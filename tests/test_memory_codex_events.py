"""Classify actual Codex startup diagnostics without hiding tool/fatal events."""
import importlib.util
from pathlib import Path
import sys
import json
import tempfile
import unittest
from unittest import mock

DIRECTORY = Path(__file__).resolve().parents[1] / "tools/memory-evaluation"
sys.path.insert(0, str(DIRECTORY))
SPEC = importlib.util.spec_from_file_location("memory_codex_runner", DIRECTORY / "run_codex.py")
RUNNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNNER)


class CodexEvents(unittest.TestCase):
    def test_v3_pack_selects_conditional_schema_without_calling_provider(self):
        with mock.patch.object(RUNNER.compare, "load_json", return_value={"protocol": "storage-plan-v3"}), mock.patch.object(RUNNER.storage_plan, "read_pack", return_value=({"cases": []}, "fixture-hash")), mock.patch.object(RUNNER.subprocess, "Popen") as process:
            _, pack_hash, schema, protocol = RUNNER.evaluation_pack(Path("/fixture"))
            self.assertEqual(pack_hash, "fixture-hash")
            self.assertEqual(protocol, "storage-plan-v3")
            self.assertIn("conditional_paths", schema["properties"])
            self.assertIn("immediate_actions", schema["properties"])
            self.assertNotIn("current_actions", schema["properties"])
            process.assert_not_called()

    def test_other_pack_protocol_or_schema_cannot_reach_provider(self):
        schema = {"type": "object"}
        schema_hash = RUNNER.compare.digest((json.dumps(schema, ensure_ascii=False, indent=2) + "\n").encode())
        pack = {"cases": [{"id": "fixture-case"}]}
        for key, bad in [("pack_sha256", "other-pack"), ("protocol", "other-protocol"), ("response_schema_sha256", "other-schema")]:
            with self.subTest(key=key), tempfile.TemporaryDirectory(prefix="swarm-preflight-bind-") as temporary:
                proof = {"result": "PASS", "pack_sha256": "pack-hash", "protocol": "protocol", "response_schema_sha256": schema_hash}
                proof[key] = bad
                with mock.patch.object(RUNNER, "evaluation_pack", return_value=(pack, "pack-hash", schema, "protocol")), mock.patch.object(RUNNER.compare, "load_json", return_value=proof), mock.patch.object(RUNNER.subprocess, "Popen") as process:
                    with self.assertRaises(ValueError):
                        RUNNER.execute_case(Path("/fixture"), "fixture-case", Path(temporary), "codex")
                    process.assert_not_called()
                    self.assertFalse((Path(temporary) / "fixture-case").exists())

    def test_existing_case_is_not_retried(self):
        schema = {"type": "object"}
        proof = {"result": "PASS", "pack_sha256": "pack-hash", "protocol": "protocol", "response_schema_sha256": RUNNER.compare.digest((json.dumps(schema, ensure_ascii=False, indent=2) + "\n").encode())}
        with tempfile.TemporaryDirectory(prefix="swarm-no-repeat-") as temporary:
            root = Path(temporary)
            (root / "fixture-case").mkdir()
            with mock.patch.object(RUNNER, "evaluation_pack", return_value=({"cases": [{"id": "fixture-case"}]}, "pack-hash", schema, "protocol")), mock.patch.object(RUNNER.compare, "load_json", return_value=proof), mock.patch.object(RUNNER.subprocess, "Popen") as process:
                with self.assertRaises(FileExistsError):
                    RUNNER.execute_case(Path("/fixture"), "fixture-case", root, "codex")
                process.assert_not_called()

    def test_unknown_protocol_cannot_reach_provider(self):
        with mock.patch.object(RUNNER.compare, "load_json", return_value={"protocol": "unknown"}), mock.patch.object(RUNNER.subprocess, "Popen") as process:
            with self.assertRaises(ValueError):
                RUNNER.evaluation_pack(Path("/not-an-executable-fixture"))
            process.assert_not_called()

    def test_v2_pack_selects_v2_schema_without_calling_provider(self):
        with mock.patch.object(RUNNER.compare, "load_json", return_value={"protocol": RUNNER.knowledge.PROTOCOL}), mock.patch.object(RUNNER.knowledge, "read_pack", return_value=({"cases": []}, "fixture-hash")), mock.patch.object(RUNNER.subprocess, "Popen") as process:
            _, pack_hash, schema, protocol = RUNNER.evaluation_pack(Path("/fixture"))
            self.assertEqual(pack_hash, "fixture-hash")
            self.assertEqual(protocol, RUNNER.knowledge.PROTOCOL)
            self.assertIn("current_state", schema["properties"])
            self.assertNotIn("admission", schema["properties"])
            process.assert_not_called()

    def test_startup_error_is_preserved_without_inventing_tool_use(self):
        events = [{"type": "thread.started", "thread_id": "fixture"},
                  {"type": "item.completed", "item": {"type": "error", "message": "Code Mode is unavailable because code-mode host is disabled."}},
                  {"type": "turn.started"},
                  {"type": "item.completed", "item": {"type": "agent_message", "text": "{}"}},
                  {"type": "turn.completed", "usage": {"input_tokens": 12, "output_tokens": 3}}]
        result = RUNNER.analyse_events(events)
        self.assertEqual(result["tool_activity_count"], 0)
        self.assertEqual(result["completed_turns"], 1)
        self.assertEqual(result["failed_turns"], 0)
        self.assertEqual(len(result["diagnostics"]), 1)
        self.assertEqual(result["usage"]["input_tokens"], 12)

    def test_actual_tool_and_unknown_item_remain_visible(self):
        for kind in ("command_execution", "mcp_tool_call", "future_tool"):
            with self.subTest(kind=kind):
                result = RUNNER.analyse_events([{"type": "item.completed", "item": {"type": kind}}])
                self.assertEqual(result["tool_activity_count"], 1)
                self.assertEqual(result["completed_turns"], 0)

    def test_failed_or_missing_turn_is_not_success(self):
        for events in ([], [{"type": "turn.failed", "error": {"message": "Unavailable"}}]):
            with self.subTest(events=events):
                result = RUNNER.analyse_events(events)
                self.assertEqual(result["completed_turns"], 0)
                self.assertIsNone(result["usage"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
