"""Refresh source integrity only; never renew historical engine acceptance."""
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
HISTORICAL = "docs/training-delivery-source-manifest.json"
TARGET = "docs/ui-redesign-source-manifest.json"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    manifest = json.loads((ROOT / HISTORICAL).read_text())
    manifest["manifest_status"] = "UI_REDESIGN_SOURCE_SNAPSHOT_NOT_INDEPENDENT_ACCEPTANCE"
    manifest["base_commit"] = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    manifest["historical_manifest"] = HISTORICAL
    manifest["limits"] = (
        "Current integrity snapshot of inherited paths and UI redesign files. "
        "Preexisting user edits are observed inputs, not claimed authorship. "
        "Historical R01-R18 evidence remains pending requalification. "
        "UI checks are recorded separately in docs/reports/ui-redesign-20261010.md.")
    inputs = {row["path"]: dict(row) for row in manifest["inputs"]}
    additions = {
        "web/swarm-design.css": "source", "web/swarm-shell.js": "source",
        "frontend/terminal.js": "source", "tests/design_system_ui.cjs": "test",
        "tests/graph_layout_cache_test.cjs": "test",
        "tests/training_design_ui.cjs": "test",
        "tools/verification/training_ui_capture.cjs": "harness",
        "docs/training/casa-pizza/tutoriels/build_graph_overview.py": "harness",
        "docs/UI-DESIGN.md": "doc", "docs/reports/ui-redesign-20261010.md": "doc",
        "tools/refresh_ui_source_manifest.py": "harness",
    }
    for path in (ROOT / "docs/training/casa-pizza/tutoriels/frames").glob("overview-*.png"):
        additions[path.relative_to(ROOT).as_posix()] = "capture"
    for path in (ROOT / "docs/screenshots/ui-redesign-20261010").iterdir():
        if path.is_file():
            additions[path.relative_to(ROOT).as_posix()] = "capture" if path.suffix == ".png" else "doc"
    for name, kind in additions.items():
        inputs[name] = {"path": name, "kind": kind}
    for name, row in inputs.items():
        row["sha256"] = digest((ROOT / name).read_bytes())
    manifest["inputs"] = sorted(inputs.values(), key=lambda row: row["path"])
    lines = sorted(row["sha256"] + "  " + row["path"] + "\n" for row in manifest["inputs"])
    manifest["candidate_digest"] = digest("".join(lines).encode())
    manifest["candidate_id"] = manifest["base_commit"] + "+sha256:" + manifest["candidate_digest"]
    (ROOT / TARGET).write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Source snapshot: {len(inputs)} paths; historical acceptance unchanged.")


if __name__ == "__main__":
    main()
