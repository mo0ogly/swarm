#!/usr/bin/env python3
"""IA2 host-boundary experiment; not an engine integration or a real provider."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import traceback


PROBE = r'''import json, os, pathlib, socket, subprocess, sys
cfg = json.loads(pathlib.Path(sys.argv[1]).read_text())
def access(path, writing=False):
    try:
        fd = os.open(path, os.O_RDWR if writing else os.O_RDONLY)
        if not writing:
            os.read(fd, 1)
        os.close(fd)
        return {"accessible": True}
    except OSError as error:
        return {"accessible": False, "errno": error.errno}
root = pathlib.Path(cfg["project"])
key = root / ".swarm/trust-actions-proof.key"
result = {"pid": os.getpid(), "direct_key": access(key), "write_open_key": access(key, True),
          "direct_registry": access(root / ".swarm/trust-actions.json"),
          "symlink_key": access(pathlib.Path.cwd() / "key-link"),
          "workspace_control": access(pathlib.Path.cwd() / ".swarm/trust-actions.json"),
          "ancestor_root": access("/proc/" + str(cfg["ancestor_pid"]) + "/root" + str(key)),
          "ancestor_visible": pathlib.Path("/proc/" + str(cfg["ancestor_pid"])).exists(),
          "inherited_private_env": "SWARM_IA2_PRIVATE_SENTINEL" in os.environ}
child = subprocess.run([sys.executable, "-c",
    "import json,os,sys\ntry:\n f=open(sys.argv[1],'rb');f.read(1);f.close();ok=True\nexcept OSError:ok=False\nprint(json.dumps({'accessible':ok}))", str(key)],
    capture_output=True, text=True, timeout=5)
assert child.returncode == 0, child.stderr
result["child_key"] = json.loads(child.stdout)
with pathlib.Path("allowed.txt").open("a") as f:
    f.write(cfg["marker"] + "\n")
result["workspace_write"] = cfg["marker"] in pathlib.Path("allowed.txt").read_text().splitlines()
cli = subprocess.run([cfg["binary"], "--root", str(root), "--json", "work", "list"],
                     capture_output=True, text=True, timeout=8)
result["control_cli"] = {"exit_code": cli.returncode, "stdout": cli.stdout, "stderr": cli.stderr}
try:
    connection = socket.create_connection(("127.0.0.1", cfg["listener_port"]), timeout=.5)
    connection.close()
    result["host_loopback"] = True
except OSError:
    result["host_loopback"] = False
print(json.dumps(result))
'''


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    out = args.output.resolve()
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    root = Path(tempfile.mkdtemp(prefix="swarm-ia2-isolation-"))
    project = root / "project"
    project.mkdir()
    records = []
    listener = socket.socket()

    def save(name, value):
        path = out / name
        with path.open("x") as file:
            os.chmod(path, 0o600)
            json.dump(value, file, ensure_ascii=False, indent=2)
            file.write("\n")

    def run(argv, payload=None, env=None, cwd=None):
        started = time.time()
        p = subprocess.run(list(map(str, argv)), input=json.dumps(payload) if payload else None,
                           text=True, capture_output=True, env=env, cwd=cwd, timeout=25)
        # Fixture tokens are intentionally not retained in command inputs.
        records.append({"argv": list(map(str, argv)), "input": "[private fixture identities]" if payload else None,
                        "started_unix": started, "ended_unix": time.time(), "exit_code": p.returncode,
                        "stdout": p.stdout, "stderr": p.stderr})
        assert p.returncode == 0, (argv, p.returncode, p.stderr)
        return json.loads(p.stdout)

    try:
        clean = dict(os.environ)
        clean.pop("SWARM_TRUST_TOKEN", None)
        cli = [binary, "--root", project, "--json"]
        run([*cli, "init"], env=clean)
        config = {"schema_version": 1, "event_id": "ia2-config", "expected_revision": 0,
                  "reason": "Isolated host-boundary experiment, no model", "policy": {
                      "mandate_ttl_seconds": 3600, "unknown_reconcile_seconds": 0,
                      "evidence_retention_days": 365, "require_separation": True}, "identities": [
                      {"subject": role, "token": secrets.token_urlsafe(32),
                       "integrity": integrity, "roles": [role]}
                      for role, integrity in [("admin", 3), ("requester", 1), ("approver", 2), ("effect", 2)]]}
        run([*cli, "trust", "config-apply", "--input", "-"], config, env=clean)
        (project / "allowed.txt").write_text("authorized workspace\n")
        (project / "key-link").symlink_to(project / ".swarm/trust-actions-proof.key")
        probe = root / "probe.py"
        probe.write_text(PROBE)
        listener.bind(("127.0.0.1", 0))
        listener.listen(4)
        cfg = {"project": str(project), "ancestor_pid": os.getpid(), "binary": str(binary),
               "listener_port": listener.getsockname()[1], "marker": "baseline-wrote"}
        config_path = root / "probe.json"
        config_path.write_text(json.dumps(cfg))
        controls = [project / ".swarm/trust-actions.json", project / ".swarm/trust-actions-proof.key", project / ".swarm/state.db"]
        before = {path.name: sha(path) for path in controls}
        baseline_env = dict(clean, SWARM_IA2_PRIVATE_SENTINEL="private fixture value")
        baseline = run([sys.executable, probe, config_path], env=baseline_env, cwd=project)
        save("baseline.json", baseline)
        for name in ["direct_key", "write_open_key", "direct_registry", "symlink_key", "workspace_control", "child_key"]:
            assert baseline[name]["accessible"], (name, baseline[name])
        assert baseline["workspace_write"] and baseline["host_loopback"] and baseline["inherited_private_env"]
        assert baseline["control_cli"]["exit_code"] == 0

        cfg.update(binary="/swarm", marker="sandbox-wrote")
        config_path.write_text(json.dumps(cfg))
        bwrap = shutil.which("bwrap")
        assert bwrap, "bwrap is required for this declared host experiment"
        argv = [bwrap, "--unshare-user", "--unshare-pid", "--unshare-net", "--unshare-ipc", "--unshare-uts",
                "--die-with-parent", "--new-session", "--cap-drop", "ALL", "--ro-bind", "/usr", "/usr"]
        for prefix in ["/bin", "/sbin", "/lib", "/lib64"]:
            if Path(prefix).is_symlink():
                argv += ["--symlink", os.readlink(prefix), prefix]
            elif Path(prefix).exists():
                argv += ["--ro-bind", prefix, prefix]
        if Path("/etc/ld.so.cache").is_file():
            argv += ["--ro-bind", "/etc/ld.so.cache", "/etc/ld.so.cache"]
        argv += ["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp", "--tmpfs", "/run",
                 "--dir", "/home/agent", "--bind", str(project), "/work", "--tmpfs", "/work/.swarm",
                 "--ro-bind", str(probe), "/probe.py", "--ro-bind", str(config_path), "/probe.json",
                 "--ro-bind", str(binary), "/swarm", "--clearenv", "--setenv", "PATH", "/usr/bin:/bin",
                 "--setenv", "HOME", "/home/agent", "--setenv", "LANG", "C.UTF-8", "--chdir", "/work",
                 "--", sys.executable, "/probe.py", "/probe.json"]
        isolated = run(argv, env=baseline_env)
        save("isolated.json", isolated)
        for name in ["direct_key", "write_open_key", "direct_registry", "symlink_key", "workspace_control", "child_key", "ancestor_root"]:
            assert not isolated[name]["accessible"], (name, isolated[name])
        assert isolated["workspace_write"] and not isolated["inherited_private_env"]
        assert not isolated["ancestor_visible"] and not isolated["host_loopback"]
        assert isolated["control_cli"]["exit_code"] != 0
        assert "sandbox-wrote" in (project / "allowed.txt").read_text().splitlines()
        after = {path.name: sha(path) for path in controls}
        assert before == after, (before, after)
        save("results.json", {"result": "PASS", "root": str(root), "model_calls": 0,
                              "binary_sha256": sha(binary), "script_sha256": sha(Path(__file__)),
                              "control_hashes_unchanged": before, "workspace_effect_observed": True,
                              "blocked_variants": ["key read", "key write-open", "registry read", "workspace control", "symlink", "child process", "ancestor proc root", "host loopback", "private inherited environment", "original control CLI"],
                              "limits": ["Standalone host experiment, not integrated into engine launchers", "No real provider or model", "Network disabled: native remote providers need a separately designed authenticated egress path", "No broker/channel authentication, revocation race, approval or resource-control qualification", "Read-only runtime directory mounts are an experiment, not a portable audited allowlist", "Host administrator compromise is outside this experiment"]})
        (out / "recipe.py").write_bytes(Path(__file__).read_bytes())
        print(json.dumps({"result": "PASS", "blocked_variants": 10, "authorized_workspace_write": True,
                          "control_unchanged": True, "engine_integration": False, "model_calls": 0}))
    except Exception:
        save("failure.json", {"error": traceback.format_exc(), "root": str(root)})
        raise
    finally:
        listener.close()
        save("commands.json", records)


if __name__ == "__main__":
    main()
