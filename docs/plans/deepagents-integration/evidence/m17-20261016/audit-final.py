import ast
import datetime
import difflib
import hashlib
import json
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[5]
out = Path(__file__).resolve().parent
read = lambda name: json.loads((out / name).read_text())
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
base = read("baseline.json")

for name in ["reference-final", "cli-final"]:
    value = read(name + ".json")
    assert value["exit_code"] == 0 and not value["changed_inputs"], name
    assert sha(out / (name + ".log")) == value["log_sha256"], name
    assert not [path for path, h in read(name + "-inputs.json").items() if sha(root / path) != h], name
tests = read("reference-final.json")["named_tests_ran"]
assert len(tests) == len(set(tests)) == 24
assert "Ran 24 tests" in (out / "reference-final.log").read_text()
for name in ["empty-source-red", "candidate-decoder-red"]:
    value = read(name + ".json")
    assert value["exit_code"] == 1
    assert sha(out / (name + ".log")) == value["log_sha256"]
    image = "empty-source" if name == "empty-source-red" else "decoder"
    for path, filename in [("tools/memory-evaluation/summarization.py", "summarization-" + image + "-red.py.txt"), ("tests/test_memory_summarization.py", "tests-" + image + "-red.py.txt")]:
        assert sha(out / "preimages" / filename) == read(name + "-inputs.json")[path]
assert read("candidate-decoder-red-assessment.json")["actual_tests_ran"] == 1
assert read("candidate-decoder-red-assessment.json")["original_log_sha256"] == read("candidate-decoder-red.json")["log_sha256"]

cli = read("cli-final/result.json")
assert cli["status"] == "PASS" and cli["cli_invocations"] == 14 and cli["refused_candidates"] == 10
assert cli["progressive_summaries"] == 2 and cli["model_calls"] == 0
assert cli["model_effectiveness"] == cli["engine_integration"] == "NOT TESTED"
for name, h in cli["source_files_sha256"].items():
    assert sha(out / "cli-final" / name) == h
history = read("cli-final/history-initial.json")
extended = read("cli-final/history-extended.json")
first, second = read("cli-final/first.json"), read("cli-final/second.json")
assert first["cutoff"] == 6 and second["cutoff"] == 9
assert first["recent_messages"] == history["messages"][6:]
assert second["recent_messages"] == extended["messages"][9:]
assert second["summary_entries"][:4] == first["summary_entries"] and len(second["summary_entries"]) == 5
assert [(entry["kind"], entry["state"]) for entry in first["summary_entries"]] == [("requirement", "open"), ("evidence", "historical"), ("intention", "future"), ("incident", "open")]
indexed = {m["id"]: m for m in extended["messages"]}
for value in [first, second]:
    assert value["reference_only"] is True and value["model_effectiveness"] == "NOT TESTED"
    without_sha = {k: v for k, v in value.items() if k != "receipt_sha256"}
    assert value["receipt_sha256"] == hashlib.sha256(json.dumps(without_sha, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()).hexdigest()
    for entry in value["summary_entries"]:
        assert entry["text"] == indexed[entry["message"]]["content"]
assert second["parent_sha256"] == first["receipt_sha256"]
for record in cli["records"]:
    path = out / "cli-final" / (record["name"] + ".json")
    assert sha(path) == record["output_sha256"] and path.stat().st_mode & 0o777 == 0o600
    if record["result"] == "FALLBACK":
        value = json.loads(path.read_text())
        assert record["exit_code"] == 1 and value["cutoff"] == 0
        assert value["summary_entries"] == value["omitted_message_ids"] == []
        assert value["recent_messages"] == history["messages"]
assert cli["first_reference_full_bytes"] == 11655 and cli["first_reference_effective_bytes"] == 1236

upstream = read("upstream-boundary.json")
assert upstream["status"] == "PASS" and upstream["model_calls"] == 0 and len(upstream["observations"]) == 4
assert upstream["upstream_sha256"] == sha(out.parent / "m16-20261016/upstream-summarization.py.txt")
assert upstream["upstream_commit"] == "29d646fd2d832a44c476964333eddecb0ec1c0cd"
assert sha(out / "upstream.log") == read("upstream.json")["log_sha256"]

engine_inputs = json.loads((out.parent / "m16-20261016/build-final-inputs.json").read_text())
changed_engine_inputs = [path for path, h in engine_inputs.items() if sha(root / path) != h]
assert not changed_engine_inputs, changed_engine_inputs
current_engine_go = {str(p.relative_to(root)) for p in (root / "internal/engine").glob("*.go")}
assert current_engine_go <= set(engine_inputs)
(out / "m16-input-reuse.json").write_text(json.dumps({"result": "PASS", "source_manifest": "../m16-20261016/build-final-inputs.json", "checked_inputs": len(engine_inputs), "changed_inputs": [], "new_go_tests_or_build_claimed": False}, indent=2) + "\n")

for path, h in base["files"].items():
    assert sha(out / "preimages" / path.replace("/", "__")) == h
case_path = "docs/plans/deepagents-integration/cas-evaluation.json"
old = json.loads((out / "preimages" / case_path.replace("/", "__")).read_text())
new = json.loads((root / case_path).read_text())
assert new["evaluations"][:-1] == old["evaluations"]
assert {k: v for k, v in new.items() if k != "evaluations"} == {k: v for k, v in old.items() if k != "evaluations"}
assert new["evaluations"][-1]["id"] == "memory-13-extractive-summary-reference"
assert all(case["result"] == "PARTIAL" for case in new["evaluations"][-1]["cases"])
added = ["tools/memory-evaluation/summarization.py", "tests/test_memory_summarization.py", "docs/plans/deepagents-integration/INCREMENT-MEMOIRE-13.md", "docs/plans/deepagents-integration/RESULTATS-MEMOIRE-13.md"]
chunks = []
for path in [*base["files"], *added]:
    before = (out / "preimages" / path.replace("/", "__")).read_text() if path in base["files"] else ""
    chunks.extend(difflib.unified_diff(before.splitlines(keepends=True), (root / path).read_text().splitlines(keepends=True), fromfile="before/" + path, tofile="after/" + path))
    if path.endswith(".py"):
        ast.parse((root / path).read_text())
    if path.endswith(".md"):
        text = (root / path).read_text()
        headings = [line for line in text.splitlines() if line.startswith("## ")]
        assert len(headings) == len(set(headings)), path
        for target in re.findall(r"\]\(([^)#]+)(?:#[^)]*)?\)", text):
            if not target.startswith(("https:", "http:")):
                assert ((root / path).parent / target).exists(), (path, target)
(out / "candidate.diff").write_text("".join(chunks))
subprocess.run(["git", "diff", "--check"], cwd=root, check=True, capture_output=True)
for path in added:
    value = subprocess.run(["git", "diff", "--no-index", "--check", "/dev/null", path], cwd=root, capture_output=True)
    assert value.returncode in [0, 1] and not value.stdout and not value.stderr, path
runtime = json.loads(subprocess.check_output(["./swarm.sh", "status"], cwd=root, text=True))
assert runtime == base["runtime"]
assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip() == base["head"]
assert subprocess.check_output(["git", "branch", "--show-current"], cwd=root, text=True).strip() == base["branch"]
result = {"status": "PASS", "at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "head": base["head"], "branch": base["branch"], "python_tests": 24, "final_cli_invocations": 14, "progressive_references": 2, "rejected_candidates": 10, "upstream_private_method_probes": 4, "model_calls": 0, "engine_integration": "NOT TESTED", "model_effectiveness": "NOT TESTED", "current_execution_inputs_verified": True, "m16_inputs_reused_unchanged": len(engine_inputs), "runtime": runtime, "candidate_diff_sha256": sha(out / "candidate.diff"), "review": "Local self-review, not independent acceptance or runtime deployment"}
(out / "audit-final.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({k: result[k] for k in ["status", "python_tests", "final_cli_invocations", "rejected_candidates", "model_calls", "current_execution_inputs_verified"]}))
