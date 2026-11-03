"""Read-only P0 checks on the configured managed instance; no mission DB fixture.

Observe metadata and hold only our own nonblocking lifecycle lock briefly. Never
stop a process/holder, change policy, authenticate a human or claim UI acceptance.
"""
import fcntl
import hashlib
import json
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
from swarm_local import owned
from swarm_version import session_request


def status():
    """Return public launcher exit/JSON; stderr or malformed output refuses.

    Uses the existing configuration in the checkout, no override or state write.
    The caller asserts semantic content rather than treating exit zero as proof.
    """
    result = subprocess.run(["./swarm.sh", "status"], text=True, capture_output=True)
    if result.stderr:
        raise RuntimeError("installed-status-unexpected-stderr")
    return result.returncode, json.loads(result.stdout)


def main():
    """Verify exact owned image, contention/recovery, notes, commit and UI refusal.

    Preconditions: cwd is the configured source checkout; its owned server is
    ready in Authority mode, the lifecycle lock is free and the latest qualified
    image is installed. A changed identity/state/image, busy lock, invalid probe
    or unexpected protected UI access raises without modifying business data.
    Context managers release only our descriptor/lock even on failure. Output:
    safe JSON observations, never keys or authenticated human session material.
    Limits: no connected Version dialog, general availability or PAEA acceptance.
    """
    root = Path.cwd()
    config = json.loads(Path("deploy/local-web.json").read_text())
    address = config["address"]
    assert Path(config["root"]).resolve() == root
    runtime = root / ".swarm/local-web" / hashlib.sha256(address.encode()).hexdigest()[:16]
    state_path = runtime / "server.json"
    original = state_path.read_bytes()
    state = json.loads(original)
    assert owned(state)
    control = status()
    assert control[0] == 0 and control[1]["ready"] is True
    with (runtime / "lifecycle.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        concurrent = status()
        assert concurrent[0] == 2 and concurrent[1]["status"] == "concurrent-operation"
        assert all(concurrent[1][k] is None for k in ("running", "ready", "pid", "binary", "binary_sha256"))
        assert state_path.read_bytes() == original and owned(state)
    recovered = status()
    assert recovered == control and state_path.read_bytes() == original and owned(state)
    with urllib.request.urlopen(session_request(address, root, "/api/v1/native-version")) as response:
        version = json.load(response)
    prepared = json.loads(Path("deploy/local-build.json").read_text())
    notes = json.loads(Path("version_history.json").read_text())
    sha = hashlib.sha256(Path("/proc", str(state["pid"]), "exe").read_bytes()).hexdigest()
    assert sha == prepared["binary_sha256"] == control[1]["binary_sha256"]
    assert sha == hashlib.sha256(Path("bin/swarm").read_bytes()).hexdigest()
    assert version["binary"]["version"] == prepared["version"]
    assert version["binary"]["commit"] == subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    assert version["binary"]["modified"] is True
    assert version["history"]["local_changes"] == notes["local_changes"]
    refused = []
    for route in ("/", "/runtime-health.js"):
        try:
            with urllib.request.urlopen(session_request(address, root, route)) as response:
                response.read()
        except urllib.error.HTTPError as error:
            assert error.code == 403
            refused.append({"route": route, "status": 403})
        else:
            raise AssertionError("Authority unexpectedly admitted owner-key protected UI")
    assert state_path.read_bytes() == original and owned(state)
    print(json.dumps({"status": "PASS_RUNTIME_IDENTITY_STATUS_AND_NOTES", "control": control,
                      "concurrent": concurrent, "recovered": recovered, "kernel_sha256": sha,
                      "compiled_metadata": version["binary"], "notes_fr_en_equal": True,
                      "prepared_evidence": prepared["evidence"],
                      "state_sha256_before_after": hashlib.sha256(original).hexdigest(),
                      "owner_cookie_ui_refusals": refused, "live_authenticated_dialog_verified": False,
                      "global_watchdog": None, "scope": "Read-only managed observation; no human session or UI acceptance"}))


if __name__ == "__main__":
    main()
