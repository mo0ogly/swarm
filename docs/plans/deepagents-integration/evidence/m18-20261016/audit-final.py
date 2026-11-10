"""Bind M18 checks to current consumed sources and retained original evidence."""
import ast
import datetime
import difflib
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

root = Path(__file__).resolve().parents[5]
out = Path(__file__).resolve().parent
read = lambda name: json.loads((out / name).read_text())
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
base, extension = read("baseline.json"), read("scope-extension.json")
recipe = "tests/memory_summary_source.py"
py_checks = ["bridge-final", "reference-regression"]
final_checks = ["source-final", *py_checks, "build-final", "static-final", "config-final", "native-flow"]
reuse = {}
for name in final_checks:
    record = read(name + ".json")
    assert record["exit_code"] == 0 and record["changed_inputs"] == [], name
    assert sha(out / (name + ".log")) == record["log_sha256"], name
    manifest = read(name + "-inputs.json")
    differences = {path: {"executed_sha256": h, "current_sha256": sha(root / path)}
                   for path, h in manifest.items() if sha(root / path) != h}
    allowed = {recipe} if name != "native-flow" else set()
    if name in py_checks:
        allowed |= {"internal/engine/memory_summary_source_test.go", "internal/engine/storage_inspection.go"}
    assert set(differences) <= allowed, (name, differences)
    reason = ("No source drift." if not differences else
              "Changed native recipe was not consumed by this check; Python-only checks also did not load the changed Go inspection classifier/test. All actual program, test and embedded/build sources remain identical.")
    reuse[name] = {"status": "PASS", "manifest_inputs": len(manifest), "later_changed_nonconsumed_inputs": differences, "assessment": reason}
    if name in ["source-final", "build-final", "static-final", "native-flow"]:
        assert {str(p.relative_to(root)) for p in (root / "internal/engine").glob("*.go")} <= set(manifest), name
tests = read("source-final.json")["named_tests_ran"]
assert len(tests) == len(set(tests)) == 10
assert len([name for name in tests if name.startswith("TestMemorySummarySource")]) == 6
for name, count in [("bridge-final", 8), ("reference-regression", 24)]:
    names = read(name + ".json")["named_tests_ran"]
    assert len(names) == len(set(names)) == count
    assert "Ran " + str(count) + " tests" in (out / (name + ".log")).read_text()
for name in ["source-red", "method-red", "inspection-red", "native", "native-final"]:
    record = read(name + ".json")
    assert record["exit_code"] == 1 and record["real_model_calls"] == 0
    assert sha(out / (name + ".log")) == record["log_sha256"]
for name, path, pre in [
    ("source-red", "internal/engine/memory_summary_source_test.go", "source-red-test.go.txt"),
    ("method-red", "internal/engine/project_memory_public.go", "method-red-public.go.txt"),
    ("method-red", "internal/engine/memory_summary_source_test.go", "method-red-test.go.txt"),
    ("inspection-red", "internal/engine/storage_inspection.go", "internal__engine__storage_inspection.go"),
    ("inspection-red", "internal/engine/memory_summary_source_test.go", "inspection-red-test.go.txt"),
    ("native", recipe, "native-first.py.txt"),
    ("native-final", recipe, "native-second.py.txt"),
]:
    assert sha(out / "preimages" / pre) == read(name + "-inputs.json")[path], (name, pre)

flow = out / "native-flow"
native = read("native-flow/result.json")
assert native["status"] == "PASS" and len(native["checks"]) == 5
assert native["model_calls"] == 0 and native["scripted_provider_invocations"] == 2
assert native["script_sha256"] == sha(root / recipe)
binary = Path(read("build-final.json")["argv"][3])
assert native["binary_sha256"] == sha(binary)
assert read("native-flow/source-api.json") == read("native-flow/source-cli.json")
assert read("native-flow/work-before.json") == read("native-flow/work-after-reads.json")
for directory in ["prepared", "current"]:
    bundle = flow / directory
    provenance = json.loads((bundle / "provenance.json").read_text())
    for path, h in provenance["files_sha256"].items():
        assert sha(bundle / path) == h
        assert (bundle / path).stat().st_mode & 0o777 == 0o600
    assert bundle.stat().st_mode & 0o777 == 0o700
    assert provenance["engine_admission"] is False and provenance["provider_history_complete"] is False
    assert provenance["audience"] == "local_operator" and provenance["model_effectiveness"] == "NOT TESTED"
history = read("native-flow/prepared/history.json")
reference = read("native-flow/reference.json")
assert reference["result"] == "PASS" and reference["reference_only"] is True
assert reference["source_sha256"] == read("native-flow/prepared/provenance.json")["history_sha256"]
indexed = {message["id"]: message for message in history["messages"]}
for entry in reference["summary_entries"]:
    assert entry["text"] == indexed[entry["message"]]["content"]
assert reference["summary_entries"][1]["state"] == "future"
assert reference["summary_entries"][3]["state"] == "open"
fallback = read("native-flow/fallback.json")
assert fallback["result"] == "FALLBACK" and fallback["reasons"] == ["parent_or_sources_changed"]
assert fallback["summary_entries"] == []
assert fallback["recent_messages"] == read("native-flow/current/history.json")["messages"]
assert len(fallback["recent_messages"]) == 5 and not (flow / "stale-annotations").exists()
assert not (flow / "candidate-current.json").exists()
assert (flow / "prepared/source-packet.json").read_bytes() == (flow / "source-cli.json").read_bytes()
assert (flow / "prepared/annotations.json").read_bytes() == (flow / "annotations.json").read_bytes()
invocations = {}
for name, fixture in [("native", "fixture-identified.json"), ("native-final", "fixture.json"), ("native-flow", "fixture.json")]:
    fixture_root = Path(read(name + "/" + fixture)["root"])
    invocations[name] = len((fixture_root / "claude.calls").read_text().splitlines())
assert invocations == {"native": 1, "native-final": 2, "native-flow": 2}

before_files = {**base["files"], **extension["files"]}
for path, h in before_files.items():
    assert sha(out / "preimages" / path.replace("/", "__")) == h
case_path = "docs/plans/deepagents-integration/cas-evaluation.json"
old = json.loads((out / "preimages" / case_path.replace("/", "__")).read_text())
new = json.loads((root / case_path).read_text())
assert new["evaluations"][:-1] == old["evaluations"]
assert {k: v for k, v in new.items() if k != "evaluations"} == {k: v for k, v in old.items() if k != "evaluations"}
assert new["evaluations"][-1]["id"] == "memory-14-swarm-summary-sources"
assert all(case["result"] == "PARTIAL" for case in new["evaluations"][-1]["cases"])
added = ["internal/engine/memory_summary_source.go", "internal/engine/memory_summary_source_test.go",
         "tools/memory-evaluation/swarm_source.py", "tests/test_memory_swarm_source.py", recipe,
         "docs/plans/deepagents-integration/INCREMENT-MEMOIRE-14.md",
         "docs/plans/deepagents-integration/RESULTATS-MEMOIRE-14.md"]
chunks = []
for path in [*before_files, *added]:
    before = (out / "preimages" / path.replace("/", "__")).read_text() if path in before_files else ""
    current = (root / path).read_text()
    chunks.extend(difflib.unified_diff(before.splitlines(keepends=True), current.splitlines(keepends=True), fromfile="before/" + path, tofile="after/" + path))
    if path.endswith(".py"):
        ast.parse(current)
    if path.endswith(".md"):
        for target in re.findall(r"\]\(([^)#]+)(?:#[^)]*)?\)", current):
            if not target.startswith(("https:", "http:")):
                assert ((root / path).parent / target).exists(), (path, target)
(out / "candidate.diff").write_text("".join(chunks))
gofiles = [path for path in [*before_files, *added] if path.endswith(".go")]
assert not subprocess.check_output(["gofmt", "-l", *gofiles], cwd=root)
subprocess.run(["git", "diff", "--check"], cwd=root, check=True, capture_output=True)
# All owned files, including files untracked before M18, get a whitespace check.
for path in [*before_files, *added, str(Path(__file__).relative_to(root))]:
    result = subprocess.run(["git", "diff", "--no-index", "--check", "/dev/null", path], cwd=root, capture_output=True)
    assert result.returncode in [0, 1] and not result.stdout and not result.stderr, path
runtime = json.loads(subprocess.check_output(["./swarm.sh", "status"], cwd=root, text=True))
assert runtime == base["runtime"], runtime
assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip() == base["head"]
assert subprocess.check_output(["git", "branch", "--show-current"], cwd=root, text=True).strip() == base["branch"]
(out / "input-reuse-assessment.json").write_text(json.dumps(reuse, indent=2) + "\n")
(out / "build-final-target.json").write_text(json.dumps({"binary": str(binary), "sha256": sha(binary), "build_record": "build-final.json", "source_manifest": "build-final-inputs.json", "native_record": "native-flow/result.json"}, indent=2) + "\n")
result = {"status": "PASS", "at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
          "head": base["head"], "branch": base["branch"], "go_tests": 10, "bridge_python_tests": 8,
          "reference_regression_python_tests": 24, "native_checks": 5,
          "final_scripted_invocations": 2, "all_retained_fixture_scripted_invocations": invocations,
          "model_calls": 0, "schema_migration_added": False, "engine_summary_admission": "NOT IMPLEMENTED",
          "model_effectiveness": "NOT TESTED", "independent_review": "NOT TESTED", "deployed": False,
          "input_reuse": "input-reuse-assessment.json", "runtime": runtime,
          "owned_files_sha256": {path: sha(root / path) for path in [*before_files, *added]},
          "candidate_diff_sha256": sha(out / "candidate.diff"),
          "python": sys.version.split()[0], "go": subprocess.check_output(["go", "version"], text=True).strip()}
(out / "audit-final.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
evidence = {str(path.relative_to(out)): sha(path) for path in sorted(out.rglob("*")) if path.is_file() and path.name != "evidence-index.json"}
(out / "evidence-index.json").write_text(json.dumps(evidence, indent=2) + "\n")
print(json.dumps({key: result[key] for key in ["status", "go_tests", "bridge_python_tests", "reference_regression_python_tests", "native_checks", "model_calls", "deployed"]}))
