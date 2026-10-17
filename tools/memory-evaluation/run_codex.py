#!/usr/bin/env python3
"""Run an explicitly authorized text-only evaluation, without retries."""
from __future__ import annotations

import argparse
import datetime
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import time

import compare
import knowledge
import storage_plan


MODEL = "gpt-5.6-sol"
EFFORT = "medium"
DISABLED = ["shell_tool", "unified_exec", "multi_agent", "apps", "hooks", "browser_use",
            "browser_use_external", "computer_use", "image_generation", "view_image",
            "code_mode", "code_mode_host", "tool_suggest", "plugins", "memories", "goals",
            "sleep_tool", "shell_snapshot", "skill_search", "daemon_auto_start",
            "unbounded_connection_retries"]
SCHEMA = {"type": "object", "additionalProperties": False, "required": ["decision", "actions", "admission", "acceptance", "reason"],
          "properties": {
              "decision": {"type": "string", "enum": ["restore_and_evaluate", "rebuild_evidence", "admit_directly", "accept_task"]},
              "actions": {"type": "array", "items": {"type": "string", "enum": ["verify_archive", "restore_artifact", "evaluate_current", "block_admission", "request_fresh_evidence", "import_acceptance"]}},
              "admission": {"type": "string", "enum": ["after_fresh_pass", "blocked", "now"]},
              "acceptance": {"type": "string", "enum": ["unchanged", "accepted"]}, "reason": {"type": "string"}}}


def write_new(path, value):
    with Path(path).open("x") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def utc():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def evaluation_pack(root):
    header = compare.load_json(root / "manifest.json")
    protocol = header.get("protocol")
    if protocol == storage_plan.PROTOCOL:
        pack, pack_hash = storage_plan.read_pack(root)
        return pack, pack_hash, storage_plan.SCHEMA, protocol
    if protocol == knowledge.PROTOCOL:
        pack, pack_hash = knowledge.read_pack(root)
        return pack, pack_hash, knowledge.SCHEMA, protocol
    if protocol is None:
        pack, pack_hash = compare.read_pack(root)
        return pack, pack_hash, SCHEMA, "import-decision-v1"
    raise ValueError("Unknown evaluation protocol; no provider call allowed")


def analyse_events(events):
    # Codex emits startup diagnostics as item.error even on a completed turn.
    # Preserve them separately; an error label is neither a tool call nor PASS.
    diagnostics = [event["item"] for event in events if event.get("item", {}).get("type") == "error"]
    tools = [event for event in events if event.get("item", {}).get("type") not in (None, "agent_message", "reasoning", "error")]
    completed = [event for event in events if event.get("type") == "turn.completed"]
    failed = [event for event in events if event.get("type") == "turn.failed"]
    return {"tool_activity_count": len(tools), "diagnostics": diagnostics,
            "completed_turns": len(completed), "failed_turns": len(failed),
            "usage": completed[-1].get("usage") if completed else None,
            "thread_ids": [event["thread_id"] for event in events if event.get("type") == "thread.started"]}


def command(binary, cwd, schema, answer, endpoint=None):
    args = [binary, "exec", "--ignore-user-config", "--ignore-rules", "--strict-config", "--ephemeral",
            "--skip-git-repo-check", "--sandbox", "read-only", "--cd", str(cwd),
            "--model", MODEL, "--json", "--color", "never", "--output-schema", str(schema),
            "--output-last-message", str(answer)]
    config = {"approval_policy": "never", "model_reasoning_effort": EFFORT, "web_search": "disabled",
              "project_doc_max_bytes": 0, "skills.include_instructions": False,
              "skills.bundled.enabled": False, "tools.update_plan.enabled": False,
              "tools.experimental_request_user_input.enabled": False,
              "model_provider": "memory_evaluation", "mcp_servers": {},
              "model_providers.memory_evaluation.name": "Authorized memory evaluation",
              "model_providers.memory_evaluation.wire_api": "responses",
              "model_providers.memory_evaluation.requires_openai_auth": endpoint is None,
              "model_providers.memory_evaluation.request_max_retries": 0,
              "model_providers.memory_evaluation.stream_max_retries": 0,
              "model_providers.memory_evaluation.stream_idle_timeout_ms": 30000,
              "model_providers.memory_evaluation.supports_websockets": False}
    if endpoint:
        config["model_providers.memory_evaluation.base_url"] = endpoint
    for key, value in config.items():
        encoded = "{}" if value == {} else json.dumps(value)
        args += ["-c", key + "=" + encoded]
    for flag in DISABLED:
        args += ["--disable", flag]
    return args + ["-"]


def invoke(args, prompt, out, env=None):
    started = utc()
    clock = time.monotonic()
    with (out / "events.jsonl").open("xb") as stdout, (out / "stderr.txt").open("xb") as stderr:
        process = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=stdout, stderr=stderr, env=env)
        try:
            process.communicate(prompt, timeout=180)
            timed_out = False
        except subprocess.TimeoutExpired:
            process.kill()
            process.communicate()
            timed_out = True
    return {"argv": args, "started_at_utc": started, "ended_at_utc": utc(),
            "elapsed_seconds": time.monotonic() - clock, "exit_code": process.returncode,
            "timed_out": timed_out, "prompt_sha256": compare.digest(prompt),
            "events_sha256": compare.digest((out / "events.jsonl").read_bytes()),
            "stderr_sha256": compare.digest((out / "stderr.txt").read_bytes())}


def preflight(out, binary, pack_root):
    """A localhost stub sees the exact tool declarations; no model or auth."""
    _, pack_hash, response_schema, protocol = evaluation_pack(pack_root)
    out.mkdir()
    requests = []

    class Stub(BaseHTTPRequestHandler):
        def do_POST(self):
            raw = self.rfile.read(int(self.headers.get("Content-Length", 0)))
            requests.append(json.loads(raw))
            self.send_response(503)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(b'{"error":{"message":"Intentional local preflight rejection","type":"invalid_request_error"}}')

        def log_message(self, *_):
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Stub)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        with tempfile.TemporaryDirectory(prefix="swarm-memory-preflight-") as temporary:
            cwd = Path(temporary)
            schema = out / "response.schema.json"
            write_new(schema, response_schema)
            args = command(binary, cwd, schema, out / "answer.json", "http://127.0.0.1:%d/v1" % server.server_port)
            env = {k: os.environ[k] for k in ("PATH", "LANG") if k in os.environ}
            env.update(HOME=str(cwd), CODEX_HOME=str(cwd / "codex-home"))
            (cwd / "codex-home").mkdir()
            receipt = invoke(args, b"Local preflight only. Return JSON. No model is contacted.", out, env)
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
    # Request bodies contain no authentication headers or credentials.
    write_new(out / "requests.json", requests)
    receipt.update(kind="local_mock", real_model_calls=0, request_count=len(requests),
                   pack_sha256=pack_hash, protocol=protocol,
                   response_schema_sha256=compare.digest(schema.read_bytes()),
                   intentional_http_status=503,
                   tools=requests[0].get("tools", []) if requests else None,
                   requested_model=requests[0].get("model") if requests else None,
                   requested_reasoning=requests[0].get("reasoning") if requests else None)
    valid = (receipt["exit_code"] != 0 and not receipt["timed_out"] and len(requests) == 1
             and requests[0].get("tools", []) == [] and requests[0].get("model") == MODEL
             and requests[0].get("reasoning", {}).get("effort") == EFFORT
             and requests[0].get("text", {}).get("format", {}).get("schema") == response_schema)
    receipt["result"] = "PASS" if valid else "FAIL"
    write_new(out / "receipt.json", receipt)
    print(json.dumps({k: receipt[k] for k in ("kind", "result", "real_model_calls", "request_count", "tools")}), flush=True)
    if not valid:
        raise ValueError("Local preflight failed; no real model call allowed")


def execute_case(pack_root, case_id, out, binary):
    pack, pack_hash, response_schema, protocol = evaluation_pack(pack_root)
    cases = {case["id"]: case for case in pack["cases"]}
    if case_id not in cases:
        raise ValueError("Case is not in the frozen evaluation pack")
    proof = compare.load_json(out / "preflight" / "receipt.json")
    if proof["result"] != "PASS":
        raise ValueError("A successful tool-free local preflight is required")
    if proof.get("pack_sha256") != pack_hash or proof.get("protocol") != protocol:
        raise ValueError("Preflight does not cover this exact pack and protocol")
    schema_bytes = (json.dumps(response_schema, ensure_ascii=False, indent=2) + "\n").encode()
    if proof.get("response_schema_sha256") != compare.digest(schema_bytes):
        raise ValueError("Preflight does not cover this response schema")
    case = cases[case_id]
    # Existing case directories are never retried or overwritten.
    destination = out / case_id
    destination.mkdir()
    write_new(destination / "reservation.json", {"case_id": case_id, "model": MODEL, "effort": EFFORT,
                                               "pack_sha256": pack_hash, "at": utc(), "automatic_retries": 0})
    schema = destination / "response.schema.json"
    write_new(schema, response_schema)
    with tempfile.TemporaryDirectory(prefix="swarm-memory-model-") as temporary:
        args = command(binary, Path(temporary), schema, destination / "answer.json")
        print(json.dumps({"case_id": case_id, "state": "started", "model": MODEL, "effort": EFFORT}), flush=True)
        receipt = invoke(args, (pack_root / case["prompt"]).read_bytes(), destination)
    receipt.update(case_id=case_id, model_requested=MODEL, effort_requested=EFFORT, pack_sha256=pack_hash)
    events = [json.loads(line) for line in (destination / "events.jsonl").read_text().splitlines() if line.strip().startswith("{")]
    receipt.update(analyse_events(events))
    valid = (receipt["exit_code"] == 0 and not receipt["timed_out"] and receipt["tool_activity_count"] == 0
             and receipt["completed_turns"] == 1 and receipt["failed_turns"] == 0
             and (destination / "answer.json").exists())
    receipt["result"] = "COMPLETE" if valid else "ERROR"
    write_new(destination / "receipt.json", receipt)
    print(json.dumps({k: receipt[k] for k in ("case_id", "result", "elapsed_seconds", "exit_code", "tool_activity_count", "usage")}), flush=True)
    if not valid:
        raise ValueError("Evaluation did not complete; inspect its original receipt without retrying")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pack", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--case")
    parser.add_argument("--preflight", action="store_true")
    args = parser.parse_args()
    binary = shutil.which("codex")
    if not binary:
        raise ValueError("Codex executable is unavailable")
    if args.preflight == bool(args.case):
        raise ValueError("Choose either preflight or one case")
    args.out = args.out.resolve()
    args.out.mkdir(exist_ok=True)
    if args.preflight:
        preflight(args.out / "preflight", binary, args.pack.resolve())
    else:
        execute_case(args.pack.resolve(), args.case, args.out, binary)


if __name__ == "__main__":
    main()
