#!/usr/bin/env python3
"""IA1 public CLI/API counterexamples. Isolated fixture; scripted providers only.

Exit 0: observations complete; exit 1 with --require-protection: invariants FAIL.
This is an audit recipe, not an authorization or an application fix.
"""
import argparse
import hashlib
import http.cookiejar
import json
import os
from pathlib import Path
import re
import secrets
import select
import subprocess
import sys
import tempfile
import time
import traceback
import urllib.error
import urllib.request
import uuid


PROVIDER = r'''import json, os, pathlib, subprocess, sys
root = pathlib.Path(__file__).resolve().parent
if "--swarm-preflight" in sys.argv:
    print(json.dumps({"schema_version": 1, "capabilities": {"process": "verified", "workspace_read": "verified", "workspace_write": "verified"}}))
    sys.exit(0)
prompt = sys.stdin.read()
mode = sys.argv[1]
with (root / "calls.jsonl").open("a") as f:
    f.write(json.dumps({"mode": mode, "pid": os.getpid()}) + "\n")
if mode == "pivot":
    cfg = json.loads((root / "pivot.json").read_text())
    report = {"uid": os.getuid(), "token_inherited": "SWARM_TRUST_TOKEN" in os.environ}
    report["canary_before"] = (root / ".swarm/ia1-control-canary").read_text()
    (root / ".swarm/ia1-control-canary").write_text("changed by scripted provider\n")
    report["canary_after"] = (root / ".swarm/ia1-control-canary").read_text()
    report["control_access"] = {}
    for name in cfg["control_files"]:
        path = root / name
        with path.open("rb") as f:
            read = bool(f.read(1))
        fd = os.open(path, os.O_RDWR)
        os.close(fd)  # Capability probe only: do not write registry, keys or DB.
        report["control_access"][name] = {"read": read, "read_write_open": True}
    def cli(argv, payload=None):
        cmd = [cfg["binary"], "--root", str(root), "--json", *argv]
        if payload is not None:
            cmd += ["--input", "-"]
        p = subprocess.run(cmd, input=json.dumps(payload) if payload else None,
                           text=True, capture_output=True, timeout=20)
        report.setdefault("commands", []).append({"argv": argv, "input": payload,
            "exit_code": p.returncode, "stdout": p.stdout, "stderr": p.stderr})
        if p.returncode:
            raise RuntimeError("scripted pivot CLI failed: " + str(argv))
        return json.loads(p.stdout)
    w = cli(["work", "show", cfg["work"]])["work"]
    report["child_absent_before"] = all(t["id"] != "unplanned-child" for t in w["tasks"])
    request = {"schema_version": 1, "event_id": "ia1-pivot-add-task", "expected_revision": w["revision"],
               "id": "unplanned-child", "title": "Provider-created child", "deliverable": "audit only",
               "criteria": ["Scripted fixture only"], "next": "Return fixture observation"}
    w = cli(["task", "add", cfg["work"]], request)["work"]
    request = {"schema_version": 1, "event_id": "ia1-pivot-child", "expected_revision": w["revision"],
               "task_id": "unplanned-child", "provider": "leaf", "role": "worker",
               "parent": cfg["parent"], "workspace": "workspaces/pivot-child", "capture_output": True,
               "instruction": "IA1 scripted child only", "timeout_seconds": 30}
    report["child"] = cli(["agent", "start", cfg["work"]], request)["agent"]
    (root / "pivot-result.json").write_text(json.dumps(report, indent=2) + "\n")
print("IA1 scripted provider finished; this output grants no authority")
'''


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--require-protection", action="store_true")
    args = parser.parse_args()
    binary = args.binary.resolve()
    out = args.output.resolve()
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    root = Path(tempfile.mkdtemp(prefix="swarm-ia1-agents-"))
    env = {k: v for k, v in os.environ.items() if not k.startswith("SWARM_")}
    for key, name in [("HOME", "home"), ("CODEX_HOME", "codex-home"), ("XDG_CONFIG_HOME", "config")]:
        (root / name).mkdir()
        env[key] = str(root / name)
    commands, checks, failures, agents = [], [], [], []
    server = None
    admin_token = secrets.token_urlsafe(32)
    requester_token = secrets.token_urlsafe(32)
    invalid_token = secrets.token_urlsafe(32)
    private = [admin_token, requester_token, invalid_token]

    def scrub(value):
        if isinstance(value, dict):
            return {k: "[private fixture credential]" if k in {"token", "csrf", "authorization"} else scrub(v) for k, v in value.items()}
        if isinstance(value, list):
            return [scrub(v) for v in value]
        if isinstance(value, str):
            for token in private:
                value = value.replace(token, "[private fixture credential]")
            return re.sub(r"/session/[^\s/\"']+", "/session/[private]", value)
        return value

    def save(name, value):
        path = out / name
        with path.open("x") as f:
            os.chmod(path, 0o600)
            json.dump(scrub(value), f, ensure_ascii=False, indent=2)
            f.write("\n")

    def cli(argv, payload=None, token=None, want=0):
        child_env = dict(env)
        if token is not None:
            child_env["SWARM_TRUST_TOKEN"] = token
        command = [str(binary), "--root", str(root), "--json", *argv]
        if payload is not None:
            command += ["--input", "-"]
        started = time.time()
        result = subprocess.run(command, input=json.dumps(payload) if payload is not None else None,
                                text=True, capture_output=True, env=child_env, timeout=25)
        commands.append({"argv": command, "input": payload, "credential_supplied": token is not None,
                         "started_unix": started, "ended_unix": time.time(), "exit_code": result.returncode,
                         "stdout": result.stdout, "stderr": result.stderr})
        assert result.returncode == want, (argv, result.returncode, result.stderr)
        return json.loads(result.stdout if want == 0 else result.stderr)

    def req(work, **kw):
        return dict(schema_version=1, event_id=str(uuid.uuid4()), expected_revision=work["revision"], **kw)

    def current():
        return cli(["work", "show", work_id])["work"]

    def wait_agent(agent_id):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            a = cli(["agent", "show", agent_id])["agent"]
            if a["status"] not in {"queued", "starting", "running", "stopping"}:
                assert a["status"] == "completed" and a.get("exit_code") == 0, a
                return a
            time.sleep(.1)
        raise AssertionError("Owned scripted agent did not complete: " + agent_id)

    def calls():
        path = root / "calls.jsonl"
        return path.read_text().splitlines() if path.exists() else []

    def start(task, token=None, parent="", provider="leaf", event=None):
        payload = req(current(), task_id=task, provider=provider, role="worker", parent=parent,
                      workspace="workspaces/" + task, instruction="IA1 isolated scripted observation",
                      capture_output=True, timeout_seconds=30)
        if event:
            payload["event_id"] = event
        result = cli(["agent", "start", work_id], payload, token=token)
        assert result["created"] is True
        agents.append(result["agent"]["id"])
        return wait_agent(result["agent"]["id"]), payload

    def defect(identifier, scenario, observed, expected):
        failures.append({"id": identifier, "scenario": scenario, "security_result": "FAIL",
                         "observed": observed, "expected": expected})
        print(json.dumps({"probe": identifier, "execution": "PASS", "security": "FAIL"}), flush=True)

    def config(revision, disabled):
        return {"schema_version": 1, "event_id": str(uuid.uuid4()), "expected_revision": revision,
                "policy": {"mandate_ttl_seconds": 3600, "unknown_reconcile_seconds": 0,
                           "evidence_retention_days": 365, "require_separation": True},
                "reason": "IA1 isolated identity suspension test",
                "identities": [{"subject": "ia1-admin", "token": admin_token, "integrity": 3, "roles": ["admin"]},
                               {"subject": "ia1-requester", "token": requester_token, "integrity": 1,
                                "roles": ["requester"], "disabled": disabled}]}

    save("fixture.json", {"root": str(root), "binary": str(binary), "binary_sha256": digest(binary.read_bytes()),
                           "script_sha256": digest(Path(__file__).read_bytes()), "model_calls": 0,
                           "uid": os.getuid(), "started_unix": time.time()})
    try:
        cli(["init"])
        provider = root / "provider.py"
        provider.write_text(PROVIDER)
        provider.chmod(0o700)
        for task in ["anonymous", "invalid", "revoked", "api", "api-revoked", "spoof", "pivot", "pivot-child"]:
            (root / "workspaces" / task).mkdir(parents=True)
        ps = {"schema_version": 1, "providers": {name: {"command": sys.executable,
                "args": [str(provider), mode], "preflight_args": [str(provider), "--swarm-preflight"],
                "preflight_kind": "provider-context", "preflight_capabilities": ["process", "workspace_read", "workspace_write"]}
                for name, mode in [("leaf", "leaf"), ("pivot", "pivot")]}}
        (root / ".swarm/providers.json").write_text(json.dumps(ps))
        (root / ".swarm/providers.json").chmod(0o600)
        cli(["trust", "config-apply"], config(0, False))
        work = cli(["work", "create"], req({"revision": 0}, title="IA1 agent authority fixture",
                   objective="Reproduce missing identity boundaries", scope="Isolated local fixture only",
                   criteria=["No model calls or real mission mutation"], next="Audit"))["work"]
        work_id = work["id"]
        for task in ["anonymous", "invalid", "revoked", "api", "api-revoked", "spoof", "pivot"]:
            work = cli(["task", "add", work_id], req(work, id=task, title=task,
                       deliverable="audit evidence", criteria=["Scripted fixture only"], next="Audit"))["work"]
        save("work-before.json", work)

        anonymous, original = start("anonymous")
        defect("IA1-F01", "CLI without an identity token", {"agent": anonymous["id"], "status": anonymous["status"]},
               "Authenticate requester before creating or starting an agent")
        identity_failure = cli(["trust", "identity-context"], {}, token=invalid_token, want=3)
        assert identity_failure["failure"]["code"] == "authorization_required"
        invalid, _ = start("invalid", token=invalid_token)
        defect("IA1-F02", "Invalid token rejected by Trust but accepted for agent start",
               {"identity_failure": identity_failure["failure"]["code"], "agent": invalid["id"]}, "Same identity refusal at agent admission")

        assert cli(["trust", "identity-context"], {}, token=requester_token)["principal_id"] == "ia1-requester"
        launch = req(current(), task_id="revoked", provider="leaf", role="worker", workspace="workspaces/revoked", timeout_seconds=30)
        before_preview, count = current(), len(calls())
        cli(["agent", "context-preview", work_id], launch, token=requester_token)
        assert current() == before_preview and len(calls()) == count
        cli(["trust", "config-apply"], config(1, True), token=admin_token)
        denial = cli(["trust", "identity-context"], {}, token=requester_token, want=3)
        assert denial["failure"]["code"] == "authorization_required"
        created = cli(["agent", "start", work_id], launch, token=requester_token)
        agents.append(created["agent"]["id"])
        revoked = wait_agent(created["agent"]["id"])
        defect("IA1-F03", "Identity suspended after preview still starts a process", {"agent": revoked["id"],
               "preview_without_mutation": True, "trust_denied": True}, "Revalidate current identity at admission and before process start")

        count, before = len(calls()), current()
        replay = cli(["agent", "start", work_id], original, token=invalid_token)
        assert replay["created"] is False and replay["agent"]["id"] == anonymous["id"]
        assert len(calls()) == count and current() == before
        changed = dict(original, instruction="Different content with copied event")
        cli(["agent", "start", work_id], changed, want=3)
        assert current() == before and len(calls()) == count
        checks.append({"id": "IA1-C01", "result": "PASS", "control": "Identical event replay has one effect; changed content refuses without mutation",
                       "limit": "The replay still does not authenticate the caller"})

        spoof, _ = start("spoof", parent=anonymous["id"])
        assert spoof["parent"] == anonymous["id"]
        defect("IA1-F04", "Unrelated local caller declares another agent as parent", {"agent": spoof["id"], "parent": spoof["parent"]},
               "Verify parent principal and delegable creation authority, beyond same-work existence")

        server = subprocess.Popen([str(binary), "--root", str(root), "web", "127.0.0.1:0"],
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
        announced, match = b"", None
        deadline = time.monotonic() + 15
        while match is None and time.monotonic() < deadline:
            assert server.poll() is None, "Owned fixture web server exited"
            ready, _, _ = select.select([server.stdout], [], [], .1)
            if ready:
                announced += os.read(server.stdout.fileno(), 4096)
                match = re.search(r"http://\S+/session/\S+", announced.decode())
        assert match, "Owned fixture session link not announced"
        address = match.group()
        base, session_token = address.split("/session/", 1)
        private.append(session_token)
        opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))

        def api(payload, authorized=True, token=None, want=200):
            headers = {"Origin": base, "X-Swarm-CSRF": session_token, "Content-Type": "application/json"}
            if token:
                headers["Authorization"] = "Bearer " + token
            request = urllib.request.Request(base + "/api/v1/action", data=json.dumps(payload).encode(), headers=headers)
            started = time.time()
            try:
                response = (opener if authorized else urllib.request.build_opener()).open(request, timeout=20)
                status, raw = response.status, response.read().decode()
            except urllib.error.HTTPError as error:
                status, raw = error.code, error.read().decode()
            commands.append({"method": "POST", "path": "/api/v1/action", "input": payload,
                             "cockpit_session": authorized, "bearer_supplied": token is not None,
                             "started_unix": started, "ended_unix": time.time(), "http_status": status, "response": raw})
            assert status == want, (status, raw)
            return json.loads(raw) if status == 200 else raw

        def api_payload(task):
            w = current()
            return {"kind": "start", "work": work_id, "task": task, "event_id": str(uuid.uuid4()),
                    "expected_revision": w["revision"], "provider": "leaf", "role": "worker",
                    "workspace": "workspaces/" + task, "instruction": "IA1 public API fixture", "capture_output": True}

        before, count = current(), len(calls())
        api(api_payload("api"), authorized=False, want=403)
        assert current() == before and len(calls()) == count
        checks.append({"id": "IA1-C02", "result": "PASS", "control": "HTTP without cockpit session refuses without work or process mutation"})
        opener.open(address, timeout=10).read()
        for task, token in [("api", None), ("api-revoked", requester_token)]:
            a = api(api_payload(task), token=token)
            agents.append(a["id"])
            wait_agent(a["id"])
        defect("IA1-F05", "Shared cockpit session starts agents without individual principal, including suspended bearer",
               {"api_no_principal": True, "api_suspended_bearer": True}, "Bind individual verified authority to both API and CLI admission")

        canary = root / ".swarm/ia1-control-canary"
        canary.write_text("private control fixture\n")
        canary.chmod(0o600)
        control_files = [".swarm/trust-actions.json", ".swarm/trust-actions-proof.key"]
        assert all((root / name).is_file() for name in control_files)
        before_controls = {name: digest((root / name).read_bytes()) for name in control_files}
        (root / "pivot.json").write_text(json.dumps({"binary": str(binary), "work": work_id, "parent": "ia1-pivot-parent", "control_files": control_files}))
        pivot, _ = start("pivot", provider="pivot", event="ia1-pivot-parent")
        report = json.loads((root / "pivot-result.json").read_text())
        agents.append(report["child"]["id"])
        child = wait_agent(report["child"]["id"])
        assert report["child_absent_before"] and not report["token_inherited"]
        assert child["parent"] == pivot["id"] and child["task_id"] == "unplanned-child"
        assert {name: digest((root / name).read_bytes()) for name in control_files} == before_controls
        assert report["uid"] == os.getuid() and report["canary_after"] != report["canary_before"]
        save("pivot.json", report)
        defect("IA1-F06", "Running scripted agent creates a new task and launches its own child through public CLI",
               {"child": child["id"], "task_absent_before": True, "token_inherited": False, "parent": child["parent"]},
               "Refuse unplanned creation; retain motivated request pending authenticated human decision")
        defect("IA1-F07", "Scripted provider reads control registry and key, opens them read/write and changes private canary",
               {"same_uid": True, "control_access": report["control_access"], "canary_changed": True,
                "registry_and_key_unchanged": True}, "Protect control registry and key from provider processes at the host boundary")
        save("work-after.json", current())
        assert len(calls()) == len(set(agents)) == 8, (len(calls()), len(set(agents)))
        for a in set(agents):
            assert cli(["agent", "show", a])["agent"]["status"] == "completed"
        save("results.json", {"execution_result": "PASS", "security_result": "FAIL", "security_failures": failures,
                              "existing_controls": checks, "model_calls": 0, "scripted_processes": len(calls()),
                              "root": str(root), "binary_sha256": digest(binary.read_bytes()), "ended_unix": time.time(),
                              "limits": ["No real provider or deployed server tested", "No live mission read or modified",
                                         "No registry/key corruption attempted", "No human OIDC bootstrap tested",
                                         "This fixture is non-hierarchical; existing hierarchical parent guards are not bypassed",
                                         "Prepared launches, import, concurrency, crash/recovery and all role processes need separate qualification"]})
        print(json.dumps({"execution": "PASS", "security": "FAIL", "counterexamples": len(failures),
                          "existing_controls": len(checks), "scripted_processes": len(calls()), "model_calls": 0}), flush=True)
        return 1 if args.require_protection else 0
    except Exception:
        save("failure.json", {"error": traceback.format_exc(), "root": str(root), "checks": checks, "failures": failures})
        raise
    finally:
        if server:
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait(timeout=5)
            stderr = server.stderr.read().decode(errors="replace")
            save("server.json", {"pid": server.pid, "exit_code": server.returncode, "stderr": stderr})
        # Only the owned fixture's processes may be stopped, through public operations.
        cleanup_errors = []
        for agent_id in set(agents):
            try:
                a = cli(["agent", "show", agent_id])["agent"]
                if a["status"] in {"queued", "starting", "running", "stopping"}:
                    cli(["agent", "stop", agent_id])
            except Exception as error:
                cleanup_errors.append({"agent": agent_id, "error": str(error)})
        save("cleanup.json", {"errors": cleanup_errors, "owned_agents": sorted(set(agents))})
        save("commands.json", commands)
        if cleanup_errors and sys.exc_info()[0] is None:
            raise RuntimeError("Owned fixture cleanup failed; see cleanup.json")


if __name__ == "__main__":
    sys.exit(main())
