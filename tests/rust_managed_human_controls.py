#!/usr/bin/env python3
"""Check a managed submission's first human validation policy on Linux.

Generate the worker, report and submission through native public commands.
Seed only a legacy human/no-controls policy in that isolated fixture, after
the owned processes exit. The managed Git/result/submission proofs stay real.
The fixture refuses to use a live Root. No paid model or global watchdog.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import tempfile
from types import SimpleNamespace

import rust_agent_model_route as fixture
import rust_managed_auto_worker as managed
from rust_native_managed_candidate import invoke
from rust_memory_files import replace_fixture_source


def run(binary, producer, profile, output, projection_probe, workers_evidence=None, candidate_only=False):
    """Test both binaries against the same owned submission and saved report.

    Save the binary and probe digests in the isolated evidence directory.
    A successful human-mode control must still refuse acceptance before review.
    The candidate-only fixture qualifies the current binary across both worker
    cycles and the amendment. Use a distinct producer to compare legacy refusal.
    """
    output.mkdir(mode=0o700)
    executable_digests = {}
    for name, executable in (("candidate", binary), ("projection_probe", projection_probe)):
        with executable.open("rb") as stream:
            executable_digests[name] = hashlib.file_digest(stream, "sha256").hexdigest()
    fixture.publish(output / "executable-digests.json", executable_digests)
    producer_digest = fixture.digest(producer.read_bytes())
    if candidate_only:
        assert producer_digest == executable_digests["candidate"]
    else:
        assert producer_digest != executable_digests["candidate"]
    fixture.fixture_root = lambda prefix: Path(tempfile.mkdtemp(prefix=prefix, dir=output))
    workers = workers_evidence or (output / "workers")
    control = dict(
        id="existing-document", command=["python3", "-c",
            "from pathlib import Path; assert Path('tracked.txt').read_text().strip()"],
        criteria=[1], inputs=["tracked.txt"], justification="Read the original committed artifact",
    )
    if workers_evidence is None:
        managed.worker.subreaper_scope(
            lambda: managed.run(producer, profile, workers, controls=[control]),
            lambda error: None, managed.parent_death.subreaper,
        )
    retained = json.loads((workers / "result.json").read_bytes())
    assert retained["status"] == "PASS"
    assert retained["binary_sha256"] == fixture.digest(producer.read_bytes())
    birth = json.loads((workers / "fixture.json").read_bytes())
    root = Path(birth["root"]).resolve()
    work = birth["work"]
    assert root.parent == workers.parent and root.name.startswith("swarm-model83-")
    env = dict(os.environ, CODEX_HOME=str(root / "codex-home"))

    def call(label, operation, request=None, executable=binary):
        """Use a fresh original record for every actual public CLI operation."""
        return invoke(executable, root, output, label, operation, request, environment=env)

    def require(label, operation, request=None):
        """Return the public result only after command success."""
        code, value = call(label, operation, request)
        assert code == 0, (label, value)
        return value

    def show(label):
        """Read the actual Work from the selected executable."""
        return require(label, ["work", "show", work])["work"]

    if candidate_only:
        first_id = retained["agent_ids"][0]
        first = require("first-terminal-agent", ["agent", "show", first_id])["agent"]
        journal = root / ".swarm" / ("native-completion-" + fixture.digest(first_id.encode()) + ".json")
        recorded = json.loads(journal.read_bytes())["expected"]
        assert recorded["id"] == first["id"] == first_id
        assert recorded.get("native_task_actions") and not first.get("native_task_actions")
        assert first["status"] == "completed"
        assert "managed-auto-cycle-2-settles-once-with-durable-original" in retained["assertions"]
        print("PASS managed-human-controls-reopen-after-live-actions", flush=True)

    original = show("original")
    selected = next(t for t in original["tasks"] if t["id"] == "task")
    assert selected["status"] == "submitted" and selected["attempts"][-1]["status"] == "completed"
    with sqlite3.connect(root / ".swarm/state.db") as db:
        body = db.execute("SELECT body FROM works WHERE id=?", (work,)).fetchone()[0]
        seeded = json.loads(body)
        task = next(t for t in seeded["tasks"] if t["id"] == "task")
        task["validation_policy"]["mode"] = "human"
        task["validation_policy"].pop("controls", None)
    replace_fixture_source(
        SimpleNamespace(root=root, output=output, projection_probe=projection_probe),
        work, json.dumps(seeded, ensure_ascii=False).encode(),
    )
    fixture.publish(output / "fixture-injection.json", {
        "scope": "owned legacy policy only", "original_sha256": fixture.digest(body.encode() if isinstance(body, str) else body),
        "submissions_and_native_receipts_changed": False,
    })
    fresh = show("seeded")
    mission = require("read-mission", ["mission", "show", work])
    if mission["paused"]:
        require("resume-mission", ["mission", "resume", work],
                dict(schema_version=1, event_id="human-controls-fixture-resume",
                     expected_revision=fresh["revision"], reason="Verify owned candidate controls"))
        fresh = show("after-resume-mission")
    if fresh["planning"].get("paused", False):
        require("resume-planning", ["planning", "resume", work],
                dict(schema_version=1, event_id="human-controls-planning-resume",
                     expected_revision=fresh["revision"], reason="Verify owned candidate controls"))
        fresh = show("after-resume-planning")
    task = next(t for t in fresh["tasks"] if t["id"] == "task")
    contract = hashlib.sha256(json.dumps(
        [task["title"], task["deliverable"], task["criteria"],
         task.get("depends"), task.get("plan_brief_hash", "")],
        ensure_ascii=False, separators=(",", ":"),
    ).encode()).hexdigest()
    assert not task.get("memory_selections")
    request = dict(schema_version=1, event_id="managed-human-controls",
                   expected_revision=fresh["revision"], id="task",
                   confirm_contract_revision=True, expected_contract=contract,
                   contract_revision_reason="Attach the existing bounded fixture control.",
                   validation_policy=dict(mode="human", controls=[control]))
    operation = ["task", "update", work]
    if not candidate_only:
        refusal, old = call("previous-binary-refusal", operation, request, executable=producer)
        assert refusal != 0 and any("contract-" in c for c in old["failure"]["causes"]), old
    refusal, bad = call("wrong-contract", operation, {**request, "expected_contract": "0" * 64})
    assert refusal != 0 and "contract-digest-stale" in bad["failure"]["causes"], bad
    assert show("same-after-refusals") == fresh

    saved = require("attach", operation, request)["work"]
    assert saved["revision"] == fresh["revision"] + 1
    after = next(t for t in saved["tasks"] if t["id"] == "task")
    assert after["validation_policy"]["mode"] == "human"
    assert after["validation_policy"]["controls"][0]["id"] == control["id"]
    assert {k: v for k, v in after.items() if k != "validation_policy"} == {
        k: v for k, v in task.items() if k != "validation_policy"}
    assert require("exact-replay", operation, request)["work"] == saved
    refusal, duplicate = call("replacement", operation, {
        **request, "event_id": "replace-human-controls", "expected_revision": saved["revision"],
    })
    assert refusal != 0 and "managed-submission-controls-first-human-policy-required" in duplicate["failure"]["causes"]
    assert show("same-after-replacement") == saved
    print("PASS managed-human-controls-original-preserved", flush=True)

    fresh = show("before-candidate")
    candidate = dict(schema_version=1, event_id="human-control-candidate",
                     expected_revision=fresh["revision"], task_id="task")
    prepared = require("prepare-candidate", ["task", "validation-candidate", work], candidate)
    assert not prepared["task_accepted"] and not prepared["controls_started"]
    checked = require("run-control", ["task", "validate", work], candidate)
    assert not checked["task_accepted"]
    after_control = show("after-control")
    after = next(t for t in after_control["tasks"] if t["id"] == "task")
    assert after["status"] == "submitted"
    assert after["automatic_validation"]["state"] == "pending_human"
    assert after["attempts"] == task["attempts"]
    assert not after.get("independent_review")
    print("PASS managed-human-controls-stay-pending-human", flush=True)

    refusal, rejected = call(
        "accept-before-independent-review", ["task", "update", work],
        dict(schema_version=1, event_id="premature-human-acceptance",
             expected_revision=after_control["revision"], id="task", status="accepted"),
    )
    assert refusal != 0, rejected
    assert "native-scoped-submitted-review-required" in rejected["failure"]["causes"], rejected
    assert show("same-after-premature-acceptance") == after_control
    print("PASS managed-human-controls-require-independent-review", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for argument in ("binary", "producer", "profile", "output", "projection_probe"):
        parser.add_argument("--" + argument.replace("_", "-"), type=Path, required=True)
    parser.add_argument("--workers-evidence", type=Path)
    parser.add_argument("--candidate-only", action="store_true")
    args = parser.parse_args()
    run(*(getattr(args, name).resolve() for name in
          ("binary", "producer", "profile", "output", "projection_probe")),
        workers_evidence=args.workers_evidence.resolve() if args.workers_evidence else None,
        candidate_only=args.candidate_only)
