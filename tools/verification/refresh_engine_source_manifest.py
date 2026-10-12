#!/usr/bin/env python3
"""Record current source integrity without renewing historical acceptance."""
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PARENT = "docs/ui-redesign-source-manifest.json"
TARGET = "docs/engine-cursor-source-manifest.json"
PART_BYTES = 48 * 1024  # Artifact packaging, below the existing review piece limit.


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    original = (ROOT / PARENT).read_bytes()
    manifest = json.loads(original)
    manifest["manifest_status"] = "ENGINE_CURSOR_SOURCE_SNAPSHOT_NOT_INDEPENDENT_ACCEPTANCE"
    manifest["historical_manifest"] = PARENT
    manifest["historical_manifest_sha256"] = sha(original)
    manifest["base_commit"] = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    manifest["limits"] = (
        "Current source integrity only. Historical R01-R18 acceptance remains "
        "pending requalification. Existing user edits are observed inputs, not "
        "claimed authorship. This snapshot is neither an executed test receipt "
        "nor an independent verdict. Research and runtime results are separate.")
    inputs = {row["path"]: dict(row) for row in manifest["inputs"]}
    for path in (ROOT / "internal/engine").glob("*.go"):
        name = path.relative_to(ROOT).as_posix()
        inputs[name] = {"path": name, "kind": "test" if name.endswith("_test.go") else "source"}
    # The executable also depends on embedded resources, including files absent
    # from the historical source list. Let Go enumerate its actual build inputs
    # rather than maintaining an incomplete second list of patterns or filenames.
    resources = json.loads(subprocess.check_output(
        ["go", "list", "-json", "."], cwd=ROOT, text=True))
    for name in resources.get("EmbedFiles", []):
        if name not in inputs:
            inputs[name] = {"path": name, "kind": "source"}
    for directory in ["docs/architecture", "docs/en"]:
        for path in (ROOT / directory).glob("*.md"):
            name = path.relative_to(ROOT).as_posix()
            inputs[name] = {"path": name, "kind": "doc"}
    for name, kind in {
        "tools/verification/refresh_engine_source_manifest.py": "harness",
        "tools/agent-workflows/CONTRACT.md": "doc",
        "tools/agent-workflows/check.py": "harness",
        ".claude/skills/apex/SKILL.md": "source",
        ".claude/skills/replan/SKILL.md": "source",
        ".claude/skills/code-reviewer/SKILL.md": "source",
        ".claude/skills/product-planning/SKILL.md": "source",
        "web/journal-console.js": "source",
        "web/log-console.html": "source",
        "web/log-console.js": "source",
        "web/log-console.css": "source",
        "tests/log_console_ui.cjs": "harness",
        "docs/UI-DESIGN.md": "doc",
        "web/instruction-revisions.js": "source",
        "web/post-delivery.js": "source",
        "tests/instruction_retex_ui.cjs": "harness",
        "tests/quotas_ui.cjs": "harness",
        "tests/web_requests_test.cjs": "harness",
        "tests/graph_studio_test.cjs": "harness",
        "tests/graph_performance_ui.cjs": "harness",
        "tests/graph_performance_admin_ui.cjs": "harness",
        "tests/graph_performance_adapter_test.cjs": "harness",
        "docs/GRAPH-PERFORMANCE.md": "doc",
        "tests/graph_studio_ui.cjs": "harness",
        "tests/asset_loading_policy_test.cjs": "harness",
        "tests/preparation_editor_policy_ui.cjs": "harness",
        "tests/web_requests_admin_ui.cjs": "harness",
        "tests/api_wait_admin_ui.cjs": "harness",
        "package.json": "harness",
        "scripts/i18n/build.cjs": "harness",
        "docs/PRODUCT-WORKFLOW.md": "doc",
        "docs/REVIEW-FRAGMENTS.md": "doc",
        "tools/agent-workflows/templates/preparations.json": "source",
    }.items():
        inputs[name] = {"path": name, "kind": kind}
    for name, row in inputs.items():
        row["sha256"] = sha((ROOT / name).read_bytes())
    rows = sorted(inputs.values(), key=lambda row: row["path"])
    lines = sorted(row["sha256"] + "  " + row["path"] + "\n" for row in rows)
    manifest["candidate_digest"] = sha("".join(lines).encode())
    manifest["candidate_id"] = manifest["base_commit"] + "+sha256:" + manifest["candidate_digest"]
    # Keep every row and digest. Separate regular JSON files allow the existing
    # lossless review transport to inspect them without an indivisible giant diff.
    groups, group = [], []
    for row in rows:
        if group and len(json.dumps(group + [row], indent=2).encode()) > PART_BYTES:
            groups.append(group)
            group = []
        group.append(row)
    if group:
        groups.append(group)
    manifest.pop("inputs", None)
    manifest["input_parts"] = []
    for number, group in enumerate(groups, 1):
        name = f"docs/engine-cursor-source-inputs-{number:03}.json"
        raw = (json.dumps(group, indent=2) + "\n").encode()
        (ROOT / name).write_bytes(raw)
        manifest["input_parts"].append({"path": name, "sha256": sha(raw)})
    (ROOT / TARGET).write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Engine source snapshot: {len(inputs)} paths in {len(groups)} verified parts; historical acceptance unchanged.")


if __name__ == "__main__":
    main()
