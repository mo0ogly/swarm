import datetime
import difflib
import hashlib
import json
import pathlib
import re
import subprocess

root = pathlib.Path(__file__).resolve().parents[5]
out = pathlib.Path(__file__).resolve().parent
read = lambda name: json.loads((out / name).read_text())
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
receipts = {}
for name in ["memory-lineage-green", "memory-final", "memory-race", "build", "static", "web-unit", "i18n", "generated", "config", "browser-flow"]:
    value = read(name + ".json")
    assert value["exit_code"] == 0 and not value["changed_inputs"], name
    assert sha(out / (name + ".log")) == value["log_sha256"], name
    inputs = read(name + "-inputs.json")
    assert not [p for p, h in inputs.items() if sha(root / p) != h], name
    receipts[name] = value["named_tests_ran"]
assert len(receipts["memory-final"]) == 33
assert len(receipts["memory-race"]) == 4
assert set(receipts["memory-race"]) <= set(receipts["memory-final"])
assert read("memory-lineage-red.json")["exit_code"] == 1
assert sha(out / "preimages/memory_lineage_red_test.go.txt") == read("memory-lineage-red-inputs.json")["internal/engine/project_memory_dialogue_test.go"]
for name, image in [("browser", "memory_lineage_browser_first.cjs.txt"), ("browser-final", "memory_lineage_browser_second.cjs.txt")]:
    assert read(name + ".json")["exit_code"] == 1
    assert sha(out / "preimages" / image) == read(name + "-metadata.json")["script_sha256"]
for name in ["memory-lineage-red", "browser", "browser-final"]:
    assert sha(out / (name + ".log")) == read(name + ".json")["log_sha256"]
meta = read("browser-flow-metadata.json")
assert sha(pathlib.Path(meta["binary"])) == meta["binary_sha256"]
assert sha(pathlib.Path(meta["bundle"])) == meta["bundle_sha256"]
assert sha(root / "tests/dialogue_recovery_ui.cjs") == meta["script_sha256"]
assert meta["built_source_manifest"] == "build-inputs.json"
browser = read("browser-flow/result.json")
assert browser["status"] == "PASS" and not browser["errors"] and not browser["failures"]
assert browser["provider_invocations"] == 2 and browser["model_calls"] == 0
assert len(browser["checks"]) == 9
assert len([r for r in browser["expectedFailures"] if r.get("status") == 409]) == 2
assert len(list((out / "browser-flow").glob("*.png"))) == 24
for name in ["deselected", "withdrawn"]:
    assert read("browser-flow/" + name + "-cli-refusal.json")["failure"]["code"] == "preview_stale"
base = read("baseline.json")
for path, h in base["files"].items():
    assert sha(out / "preimages" / path.replace("/", "__")) == h
case_path = "docs/plans/deepagents-integration/cas-evaluation.json"
old = json.loads((out / "preimages" / case_path.replace("/", "__")).read_text())
new = json.loads((root / case_path).read_text())
assert new["evaluations"][:-1] == old["evaluations"]
assert {k: v for k, v in new.items() if k != "evaluations"} == {k: v for k, v in old.items() if k != "evaluations"}
assert new["evaluations"][-1]["id"] == "memory-11-inherited-memory-withdrawal"
assert new["evaluations"][-1]["cases"][0]["result"] == "PARTIAL"
added = ["internal/engine/project_memory_dialogue.go", "internal/engine/project_memory_dialogue_test.go", "docs/plans/deepagents-integration/INCREMENT-MEMOIRE-11.md", "docs/plans/deepagents-integration/RESULTATS-MEMOIRE-11.md"]
chunks = []
for path in [*base["files"], *added]:
    before = (out / "preimages" / path.replace("/", "__")).read_text() if path in base["files"] else ""
    chunks.extend(difflib.unified_diff(before.splitlines(keepends=True), (root / path).read_text().splitlines(keepends=True), fromfile="before/" + path, tofile="after/" + path))
(out / "candidate.diff").write_text("".join(chunks))
subprocess.run(["git", "diff", "--check"], cwd=root, check=True, capture_output=True)
for path in added:
    r = subprocess.run(["git", "diff", "--no-index", "--check", "/dev/null", path], cwd=root, capture_output=True)
    assert r.returncode in [0, 1] and not r.stdout and not r.stderr, path
assert not subprocess.check_output(["gofmt", "-l", *[p for p in [*base["files"], *added] if p.endswith(".go")]], cwd=root)
subprocess.run(["node", "--check", "tests/dialogue_recovery_ui.cjs"], cwd=root, check=True, capture_output=True)
for name in ["INCREMENT-MEMOIRE-11.md", "RESULTATS-MEMOIRE-11.md"]:
    path = out.parents[1] / name
    text = path.read_text()
    assert "Vérification en cours" not in text
    for target in re.findall(r"\]\(([^)#]+)(?:#[^)]*)?\)", text):
        if not target.startswith("https:"):
            assert (path.parent / target).exists(), target
runtime = read("runtime-final.json")["runtime"]
assert runtime == base["runtime"]
assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip() == base["head"]
assert subprocess.check_output(["git", "branch", "--show-current"], cwd=root, text=True).strip() == base["branch"]
result = {"status": "PASS", "at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "head": base["head"], "branch": base["branch"], "receipts": receipts, "go_tests": 33, "race_tests_included": 4, "browser_checks": len(browser["checks"]), "screenshots": 24, "local_scripted_invocations": 2, "model_calls": 0, "current_inputs_verified": True, "runtime": runtime, "candidate_diff_sha256": sha(out / "candidate.diff"), "review": "Local review in the author's context; no independent acceptance or deployment"}
(out / "audit-final.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({"status": "PASS", "go_tests": 33, "race_tests_included": 4, "browser_checks": len(browser["checks"]), "screenshots": 24, "model_calls": 0}))
