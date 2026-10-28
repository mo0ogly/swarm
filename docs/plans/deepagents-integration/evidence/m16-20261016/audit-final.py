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
receipts = {}
for name in ["projection-green", "projection-contract", "static", "config", "build-cli", "build-final", "native-cli"]:
    value = read(name + ".json")
    assert value["exit_code"] == 0 and not value["changed_inputs"], name
    assert sha(out / (name + ".log")) == value["log_sha256"], name
    inputs = read(name + "-inputs.json")
    # The five original green tests were subsequently strengthened and extended.
    changed = [p for p, h in inputs.items() if sha(root / p) != h]
    if name == "projection-green":
        assert changed == ["internal/engine/worker_recovery_projection_test.go"]
    else:
        assert not changed, (name, changed)
    receipts[name] = value["named_tests_ran"]

tests = receipts["projection-contract"]
assert len(tests) == len(set(tests)) == 18
assert len(receipts["projection-green"]) == 5
assert set(receipts["projection-green"]) <= set(tests)
log = (out / "projection-contract.log").read_text()
assert len(re.findall(r"^    --- PASS:", log, re.M)) == 10
red = read("projection-red.json")
assert red["exit_code"] == 1 and len(red["named_tests_ran"]) == 5
assert sha(out / "projection-red.log") == red["log_sha256"]
assert sha(out / "preimages/projection-red-test.go.txt") == read("projection-red-inputs.json")["internal/engine/worker_recovery_projection_test.go"]
assert read("build.json")["argv"][-1] == "."
assert read("native.json")["exit_code"] == 1
assert sha(out / "native.log") == read("native.json")["log_sha256"]
assert read("native/failure.json")["checks"] == []
assert read("native/commands.json") == []
assert "PermissionError" in read("native/failure.json")["error"]
assert "current ar archive" in read("build-non-executable.json")["type"]

native = read("native-cli/result.json")
assert native["status"] == "PASS" and len(native["checks"]) == 7
assert native["local_scripted_invocations"] == 2 and native["model_calls"] == 0
binary = Path(read("build-target.json")["binary"])
assert sha(binary) == native["binary_sha256"]
assert sha(root / "tests/worker_recovery_projection.py") == native["script_sha256"]
assert read("build-final.json")["argv"][-1] == "./cmd/swarm"
final_inputs = read("build-final-inputs.json")
for path in ["cmd/swarm/main.go", "resources.go", "go.mod", "go.sum", "tests/worker_recovery_projection.py", str((out / "run-check.py").relative_to(root))]:
    assert final_inputs[path] == sha(root / path), path

cli, api = read("native-cli/cli-agent.json"), read("native-cli/api-agent.json")
assert cli == api and cli["status"] == "completed"
received = (out / "native-cli/provider-input.txt").read_bytes()
assert cli["prompt"].encode() == received
assert cli["context"]["sha256"] == hashlib.sha256(received).hexdigest()
assert cli["context"]["bytes"] == len(received)
prepared = cli["prepared_context"]
assert prepared["sha256"] == hashlib.sha256(received[:prepared["bytes"]]).hexdigest()
assert prepared["bytes"] < len(received)
assert read("native-cli/source-before.json") == read("native-cli/source-after.json")
assert read("native-cli/source-logs-before.json") == read("native-cli/source-logs-after.json")
projection = read("native-cli/projection.json")
p = projection["activity_projection"]
assert projection["bounded_excerpt"] and projection["tool_calls"] == projection["tool_results"] == 3
assert p["retained_activity_rows"] == p["scanned_activity_rows"] == 6
assert p["excluded_tool_result_rows"] == len(projection["observed_operations"]) == len(p["sources"]) == 3
assert p["unscanned_activity_rows"] == p["duplicate_rows"] == p["omitted_budget_rows"] == 0
assert sum(row["truncated"] for row in p["sources"]) == 2
assert projection["task_focus"]["truncated_fields"] == ["criteria.1.expected"]
assert all(row["verification_required"] for row in projection["task_focus"]["criteria"])
assert sha(out / "upstream-summarization.py.txt") == read("upstream.json")["sha256"]
assert read("upstream.json")["commit"] == "29d646fd2d832a44c476964333eddecb0ec1c0cd"

base = read("baseline.json")
for path, h in base["files"].items():
    assert sha(out / "preimages" / path.replace("/", "__")) == h
case_path = "docs/plans/deepagents-integration/cas-evaluation.json"
old = json.loads((out / "preimages" / case_path.replace("/", "__")).read_text())
new = json.loads((root / case_path).read_text())
assert new["evaluations"][:-1] == old["evaluations"]
assert {k: v for k, v in new.items() if k != "evaluations"} == {k: v for k, v in old.items() if k != "evaluations"}
assert new["evaluations"][-1]["id"] == "memory-12-attributed-recovery-projection"
assert [case["result"] for case in new["evaluations"][-1]["cases"]] == ["PARTIAL", "NOT TESTED"]

added = ["internal/engine/worker_recovery_projection.go", "internal/engine/worker_recovery_projection_test.go", "tests/worker_recovery_projection.py", "docs/plans/deepagents-integration/INCREMENT-MEMOIRE-12.md", "docs/plans/deepagents-integration/RESULTATS-MEMOIRE-12.md"]
chunks = []
for path in [*base["files"], *added]:
    before = (out / "preimages" / path.replace("/", "__")).read_text() if path in base["files"] else ""
    chunks.extend(difflib.unified_diff(before.splitlines(keepends=True), (root / path).read_text().splitlines(keepends=True), fromfile="before/" + path, tofile="after/" + path))
(out / "candidate.diff").write_text("".join(chunks))
subprocess.run(["git", "diff", "--check"], cwd=root, check=True, capture_output=True)
for path in added:
    result = subprocess.run(["git", "diff", "--no-index", "--check", "/dev/null", path], cwd=root, capture_output=True)
    assert result.returncode in [0, 1] and not result.stdout and not result.stderr, path
assert not subprocess.check_output(["gofmt", "-l", *[p for p in [*base["files"], *added] if p.endswith(".go")]], cwd=root)
ast.parse((root / "tests/worker_recovery_projection.py").read_text())
for name in ["INCREMENT-MEMOIRE-12.md", "RESULTATS-MEMOIRE-12.md"]:
    path = out.parents[1] / name
    for target in re.findall(r"\]\(([^)#]+)(?:#[^)]*)?\)", path.read_text()):
        if not target.startswith("https:"):
            assert (path.parent / target).exists(), target
runtime = json.loads(subprocess.check_output(["./swarm.sh", "status"], cwd=root, text=True))
assert runtime == base["runtime"]
assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip() == base["head"]
assert subprocess.check_output(["git", "branch", "--show-current"], cwd=root, text=True).strip() == base["branch"]
result = {"status": "PASS", "at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "head": base["head"], "branch": base["branch"], "go_tests": len(tests), "new_named_subtests": 10, "native_checks": len(native["checks"]), "native_scripted_invocations": 2, "model_calls": 0, "current_inputs_verified": True, "entrypoint_and_recipe_tracked": True, "runtime": runtime, "receipts": receipts, "candidate_diff_sha256": sha(out / "candidate.diff"), "review": "Local review in the author's context; no independent acceptance or deployment"}
(out / "audit-final.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({k: result[k] for k in ["status", "go_tests", "native_checks", "model_calls", "current_inputs_verified"]}))
