#!/usr/bin/env python3
"""Set the first controlled validation policy of an unstarted managed task.

Reuse the native managed planning journey, which plans two `todo` tasks on a
real committed copy without any worker, paid model or global watchdog. Seed
only the selected task's policy, in that isolated fixture, to the state of
the real BIBA tasks: an authorized `human` policy without controls (as the
retained managed-human-controls recipe does for its legacy policy). Then
exercise `task update` with a confirmed policy-only request on that task.

The previous binary must refuse that request (reproducer). The candidate must
refuse a stale contract, an automatic policy without controls and a second
replacement, leaving the Work unchanged each time. It must accept the first
automatic policy with one control, change only that task's policy, and replay
the same event exactly. No control, review, launch or acceptance is started.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
from types import SimpleNamespace

import rust_agent_model_route as fixture
import rust_managed_planning as planning
from rust_memory_files import replace_fixture_source


def contract(task):
    """Return the task review contract digest exactly as the engine encodes it."""
    return hashlib.sha256(json.dumps(
        [task["title"], task["deliverable"], task["criteria"],
         task.get("depends"), task.get("plan_brief_hash", "")],
        ensure_ascii=False, separators=(",", ":"),
    ).encode()).hexdigest()


def causes(response):
    """Return the stable refusal causes of one public CLI response."""
    return response.get("failure", {}).get("causes", [])


def run(binary, previous, profile, projection_probe, output):
    """Run the journey on one owned fixture Root and publish its result.

    `binary` is the candidate, `previous` the binary expected to refuse and
    `projection_probe` the Rust probe used for the fixture-only seed. Every
    call keeps its original input/output under `output`. Raises on the first
    failed assertion; partial evidence stays in place.
    """
    output.mkdir(mode=0o700)
    result, f = planning.run(binary, profile, output / "planning", return_fixture=True)
    f.work = result["planned_work"]

    def show():
        return f.ok(["work", "show", f.work])["work"]

    with sqlite3.connect(f.root / ".swarm/state.db") as db:
        body = db.execute("SELECT body FROM works WHERE id=?", (f.work,)).fetchone()[0]
    seeded = json.loads(body)
    target = next(t for t in seeded["tasks"] if t["id"] == "task")
    target["validation_policy"]["mode"] = "human"
    target["validation_policy"].pop("controls", None)
    assert target["validation_policy"]["actor"] and target["validation_policy"]["authorized_at"]
    replace_fixture_source(
        SimpleNamespace(root=f.root, output=output, projection_probe=projection_probe),
        f.work, json.dumps(seeded, ensure_ascii=False).encode(),
    )
    fixture.publish(output / "fixture-injection.json", {
        "scope": "owned fixture task policy only: human, controls removed, authorization kept",
        "original_sha256": fixture.digest(body.encode() if isinstance(body, str) else body),
    })

    fresh = show()
    mission = f.ok(["mission", "show", f.work])
    if mission["paused"]:
        f.ok(["mission", "resume", f.work], dict(
            schema_version=1, event_id="unstarted-policy-resume",
            expected_revision=fresh["revision"], reason="Verify unstarted task policy"))
        fresh = show()
    if fresh["planning"].get("paused", False):
        f.ok(["planning", "resume", f.work], dict(
            schema_version=1, event_id="unstarted-policy-planning-resume",
            expected_revision=fresh["revision"], reason="Verify unstarted task policy"))
        fresh = show()
    task = next(t for t in fresh["tasks"] if t["id"] == "task")
    other = next(t for t in fresh["tasks"] if t["id"] == "task-2")
    assert task["status"] == "todo" and not task.get("attempts"), task
    assert not (task.get("validation_policy") or {}).get("controls"), task
    # An automatic policy must cover every criterion and declare one control
    # per planned check, under the planned check's own identifier.
    criteria = list(range(1, len(task["criteria"]) + 1))
    plan_checks = sorted(task.get("plan_checks") or {})
    assert plan_checks, task
    controls = [dict(
        id=check, command=["python3", "-c",
            "import sys; from pathlib import Path; assert Path(sys.argv[1]).read_text().strip()",
            task["deliverable"]],
        criteria=criteria, inputs=[task["deliverable"]],
        justification="Read the delivered handoff document",
    ) for check in plan_checks]
    request = dict(schema_version=1, event_id="unstarted-automatic-policy",
                   expected_revision=fresh["revision"], id="task",
                   confirm_contract_revision=True, expected_contract=contract(task),
                   contract_revision_reason="Declare the automatic control before any worker.",
                   validation_policy=dict(mode="automatic", controls=controls))
    operation = ["task", "update", f.work]

    candidate_binary = f.binary
    f.binary = previous
    try:
        code, old = f.call(operation, request)
    finally:
        f.binary = candidate_binary
    assert code != 0 and causes(old), old
    assert show() == fresh
    print("PASS unstarted-policy-previous-binary-refuses", flush=True)

    for label, value, cause in (
        ("stale", {**request, "event_id": "unstarted-stale", "expected_contract": "0" * 64},
         "contract-digest-stale"),
        ("no-controls", {**request, "event_id": "unstarted-empty",
                         "validation_policy": dict(mode="automatic", controls=[])},
         "managed-unstarted-policy-controls-required"),
    ):
        code, refused = f.call(operation, value)
        assert code != 0 and cause in causes(refused), (label, refused)
        assert show() == fresh, label
    print("PASS unstarted-policy-refusals-leave-work-unchanged", flush=True)

    saved = f.ok(operation, request)["work"]
    assert saved["revision"] == fresh["revision"] + 1
    after = next(t for t in saved["tasks"] if t["id"] == "task")
    policy = after["validation_policy"]
    assert policy["mode"] == "automatic" and sorted(c["id"] for c in policy["controls"]) == plan_checks
    assert policy["actor"] and policy["authorized_at"]
    assert {k: v for k, v in after.items() if k != "validation_policy"} == {
        k: v for k, v in task.items() if k != "validation_policy"}
    assert next(t for t in saved["tasks"] if t["id"] == "task-2") == other
    assert after["status"] == "todo" and not after.get("attempts")
    assert not after.get("automatic_validation") and not after.get("independent_review")
    assert f.ok(operation, request)["work"] == saved
    print("PASS unstarted-policy-first-automatic-policy-recorded", flush=True)

    code, duplicate = f.call(operation, {
        **request, "event_id": "unstarted-replacement", "expected_revision": saved["revision"],
        "expected_contract": contract(after)})
    assert code != 0 and "managed-unstarted-policy-first-policy-required" in causes(duplicate), duplicate
    assert show() == saved
    print("PASS unstarted-policy-replacement-refused", flush=True)

    value = dict(status="PASS", work=f.work, candidate_sha256=fixture.digest(Path(binary).read_bytes()),
                 previous_sha256=fixture.digest(Path(previous).read_bytes()),
                 revision_before=fresh["revision"], revision_after=saved["revision"])
    fixture.publish(output / "result.json", value)
    print("MANAGED_UNSTARTED_POLICY_PASS", flush=True)
    return value


def main():
    """Parse explicit paths; no default binary, profile or output is invented."""
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["binary", "previous", "profile", "projection-probe", "output"]:
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    run(args.binary, args.previous, args.profile, args.projection_probe, args.output)


if __name__ == "__main__":
    main()
