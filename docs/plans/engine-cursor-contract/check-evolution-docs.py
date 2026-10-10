"""Mechanical architecture checks; semantic acceptance requires independent review."""
import json
import pathlib
import re
import sys

root = pathlib.Path(__file__).resolve().parents[3]
task_id = sys.argv[1]
plan = json.loads((root / "docs/plans/engine-cursor-contract/evolution-20261010.json").read_text())
task = next(t for t in plan["tasks"] if t["id"] == task_id)
report = root / task["deliverable"]
architecture = root / "docs/architecture/CURSOR-ENGINE-EVOLUTION-20261010.md"
assert report.is_file(), f"Missing task evidence: {report}"
assert architecture.is_file(), "Missing architecture document"
text = report.read_text()
assert len(text.strip()) >= 500, "Evidence report is incomplete"
assert re.search(r"\b[0-9a-f]{40}\b", text), "Report must identify the candidate SHA"
assert re.search(r"\b(PASS|FAIL|PARTIAL|NOT TESTED)\b", text), "Explicit evidence statuses required"
if task_id == "T1":
    for prefix, count in (("JAN", 7), ("FEB", 4)):
        for number in range(1, count + 1):
            assert f"{prefix}-{number}" in text, f"Missing {prefix}-{number} audit row"
for doc in (report, architecture):
    for target in re.findall(r"\]\(([^)]+)\)", doc.read_text()):
        if "://" in target or target.startswith("#"):
            continue
        target = target.split("#", 1)[0]
        if target:
            assert (doc.parent / target).exists(), f"Broken local link: {target} in {doc}"
print(f"{task_id}: architecture/evidence structure verified; semantic review still required")
