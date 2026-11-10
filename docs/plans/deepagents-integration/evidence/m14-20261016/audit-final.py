"""Audit local des reçus et de leur attribution, sans exécuter de fournisseur."""
import datetime
import difflib
import hashlib
import json
import pathlib
import re
import subprocess

root = pathlib.Path(__file__).resolve().parents[5]
out = pathlib.Path(__file__).resolve().parent

def read(name):
    return json.loads((out / name).read_text())

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

checks = []
receipts = {}
formatting = read("formatting-equivalence.json")
format_path = formatting["path"]
assert sha(root / format_path) == formatting["after_sha256"]
before_format = out / "preimages/dialogue_restore_terminal_before_gofmt.go.txt"
assert sha(before_format) == formatting["before_sha256"]
assert subprocess.check_output(["gofmt"], input=before_format.read_bytes()) == (root / format_path).read_bytes()
assert formatting["canonical_before_sha256"] == formatting["canonical_after_sha256"]
for name in ["restore-final", "restore-race", "storage-final", "build-release",
             "static", "web-unit", "i18n", "config", "generated",
             "browser-final", "preview-storage-green"]:
    receipt = read(name + ".json")
    assert receipt["exit_code"] == 0 and not receipt["changed_inputs"], name
    assert sha(out / (name + ".log")) == receipt["log_sha256"], name
    inputs = read(name + "-inputs.json")
    drift = sorted(p for p, h in inputs.items() if sha(root / p) != h)
    assert inputs[format_path] == formatting["before_sha256"]
    assert format_path in drift
    semantic_drift = [p for p in drift if p != format_path]
    expected = ["internal/engine/storage_inspection.go", "internal/engine/storage_inspection_test.go"]
    if name in ["restore-final", "restore-race"]:
        assert semantic_drift == expected, (name, drift)
    elif name in ["web-unit", "i18n"]:
        assert semantic_drift == sorted(expected + ["internal/engine/dialogue_recovery.go",
            "internal/engine/dialogue_restore.go", "internal/engine/dialogue_restore_test.go",
            "internal/engine/web_server.go"]), (name, drift)
    else:
        assert not semantic_drift, (name, drift)
    receipts[name] = {"named_tests": receipt["named_tests_ran"], "current_input_differences": drift}
checks.append("Passing receipts, original logs and exact current-input differences verified; one final Go formatting delta canonically identical, frontend dependencies unchanged")

names = set(receipts["restore-final"]["named_tests"])
assert len(names) == 41
assert set(receipts["restore-race"]["named_tests"]) <= names
assert len(receipts["restore-race"]["named_tests"]) == 8
names.update(receipts["storage-final"]["named_tests"])
assert len(names) == 42
checks.append("42 distinct Go test names; eight race cases included, not added twice")

for name, image in [("restore-red", "dialogue_restore_red_test.go.txt"),
                    ("restore-first-green", "dialogue_restore_first_green_test.go.txt")]:
    assert read(name + ".json")["exit_code"] == 1
    assert sha(out / "preimages" / image) == read(name + "-inputs.json")["internal/engine/dialogue_restore_test.go"]
for name, image in [("browser-preliminary", "dialogue_restore_first_browser.cjs.txt"),
                    ("browser-current", "dialogue_restore_browser_current.cjs.txt")]:
    assert read(name + ".json")["exit_code"] == 1
    assert sha(out / "preimages" / image) == read(name + "-metadata.json")["script_sha256"]
    assert (out / name / "failure.json").is_file()
assert read("preview-storage-red.json")["exit_code"] == 1
checks.append("Initial failing test, fixture, browser and migration evidence preserved with exact source images")

meta = read("browser-final-metadata.json")
assert sha(pathlib.Path(meta["binary"])) == meta["binary_sha256"]
assert sha(root / "tests/dialogue_recovery_ui.cjs") == meta["script_sha256"]
assert meta["built_source_manifest"] == "build-release-inputs.json"
browser = read("browser-final/result.json")
assert browser["status"] == "PASS" and not browser["errors"] and not browser["failures"]
assert len(browser["checks"]) == 9 and browser["provider_invocations"] == 4 and browser["model_calls"] == 0
assert len([r for r in browser["expectedFailures"] if r.get("status") == 400]) == 7
assert len(list((out / "browser-final").glob("*.png"))) == 28
for case in read("preview-storage-green-results.json")["cases"]:
    assert case["result"] == "PASS" and case["version_before"] == case["version_after"] == 34
    assert case["exit_code"] == 2 and case["backup_count"] == 0
assert len(read("preview-storage-green-results.json")["cases"]) == 4
checks.append("Native CLI/browser receipts bound to final binary and script: nine checks, four scripted calls, seven expected refusals, 28 captures, four read-only version refusals")

base = read("baseline.json")
for path, expected in base["files"].items():
    assert sha(out / "preimages" / path.replace("/", "__")) == expected, path
case_path = "docs/plans/deepagents-integration/cas-evaluation.json"
old = json.loads((out / "preimages" / case_path.replace("/", "__")).read_text())
new = json.loads((root / case_path).read_text())
assert new["evaluations"][:-1] == old["evaluations"]
assert {k: v for k, v in new.items() if k != "evaluations"} == {k: v for k, v in old.items() if k != "evaluations"}
assert new["evaluations"][-1]["id"] == "memory-10-explicit-dialogue-restoration"
assert [c["result"] for c in new["evaluations"][-1]["cases"]] == ["PARTIAL", "PARTIAL"]
checks.append("Baseline preimages and prior evaluation results unchanged; only M14 evaluation appended")

owned_new = ["internal/engine/dialogue_restore.go", "internal/engine/dialogue_restore_usage.go",
             "internal/engine/dialogue_restore_test.go", "docs/plans/deepagents-integration/INCREMENT-MEMOIRE-10.md",
             "docs/plans/deepagents-integration/RESULTATS-MEMOIRE-10.md"]
chunks = []
for name in [*base["files"], *owned_new]:
    before = (out / "preimages" / name.replace("/", "__")).read_text() if name in base["files"] else ""
    current = (root / name).read_text()
    chunks.extend(difflib.unified_diff(before.splitlines(keepends=True), current.splitlines(keepends=True),
                  fromfile="before/" + name, tofile="after/" + name))
(out / "candidate.diff").write_text("".join(chunks))
subprocess.run(["git", "diff", "--check"], cwd=root, check=True, capture_output=True)
for path in owned_new:
    diff = subprocess.run(["git", "diff", "--no-index", "--check", "/dev/null", path], cwd=root, capture_output=True)
    assert diff.returncode in [0, 1] and not diff.stdout and not diff.stderr, path
go_files = [p for p in [*base["files"], *owned_new] if p.endswith(".go")]
assert not subprocess.check_output(["gofmt", "-l", *go_files], cwd=root)
for name in ["web/mission-insights.js", "tests/dialogue_recovery_ui.cjs"]:
    subprocess.run(["node", "--check", name], cwd=root, check=True, capture_output=True)
for name in ["INCREMENT-MEMOIRE-10.md", "RESULTATS-MEMOIRE-10.md"]:
    path = out.parents[1] / name
    text = path.read_text()
    assert "Vérification finale en cours" not in text
    for target in re.findall(r"\]\(([^)#]+)(?:#[^)]*)?\)", text):
        if not target.startswith("https:"):
            assert (path.parent / target).exists(), target
assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip() == base["head"]
assert subprocess.check_output(["git", "branch", "--show-current"], cwd=root, text=True).strip() == base["branch"]
runtime = read("runtime-final.json")["runtime"]
assert runtime == base["runtime"]
checks.append("Owned diff, whitespace, Go formatting, JS syntax, report links, branch and unchanged ready user server verified")

result = {"status": "PASS", "at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "head": base["head"], "branch": base["branch"], "checks": checks, "receipts": receipts,
    "distinct_go_tests": sorted(names), "race_tests": 8, "model_calls": 0,
    "review": "Same-context local review; no independent acceptance or deployment",
    "runtime": runtime, "candidate_diff_sha256": sha(out / "candidate.diff")}
(out / "audit-final.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({"status": "PASS", "checks": len(checks), "distinct_go_tests": len(names), "race_tests": 8, "model_calls": 0}))
