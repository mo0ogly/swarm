#!/usr/bin/env python3
"""Exercise BF-L0-A declared-input retention in isolated Python-only fixtures.

Original recipes supply declarations, never commands. A byte-identical copy of
the shared runner executes inert sentinels. No Cargo, provider, service, live
Root, automatic retry or global watchdog. Retain all cases, including refusals.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


REPOSITORY = Path(__file__).resolve().parents[2]
RECIPES = ("paea-native-git-reference-transaction", "paea-native-git-intent")
# The planned declaration matrix, not a runtime policy or retry limit.
INPUTS = ("tests/fixture_area.py", "tools/verification/cargo_space_guard.py",
          ".cargo/storage-policy.json")
SENTINEL = '''import json, os, pathlib, sys
phase, target = sys.argv[1:]
root = pathlib.Path.cwd()
control = json.loads((root / 'control.json').read_text())
if phase == 'first' and control['mutate']:
    path = root / target
    path.write_bytes(path.read_bytes() + b'\\n')
with (root / 'events.jsonl').open('a') as stream:
    stream.write(json.dumps({'sentinel': phase, 'tmpdir': os.environ['TMPDIR']}) + '\\n')
    stream.flush(); os.fsync(stream.fileno())
print('A3_' + phase.upper(), flush=True)
'''


def require(condition, cause):
    """Refuse a failed fixture assertion with its safe, named cause; never retry."""
    if not condition:
        raise ValueError(cause)


def digest(raw):
    """Return the complete-byte SHA-256 used to bind copied fixture sources."""
    return hashlib.sha256(raw).hexdigest()


def save(path, value):
    """Create and sync a fresh evidence file; existing paths and IO errors refuse.

    Accept bytes or JSON data. This writes only owned evidence, never replaces
    previous outcomes, and does not authenticate authors or qualify a profile.
    """
    raw = value if isinstance(value, bytes) else (json.dumps(value, indent=2) + "\n").encode()
    with path.open("xb") as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())


def private_area(path):
    """Require an existing owned directory with no group/other-writable ancestor.

    Read metadata only; never chmod shared directories. Failure is a fixture
    prerequisite error, not an implicit fallback or a product access boundary.
    """
    require(path.is_dir() and path.stat().st_uid == os.getuid(), "a3-area-not-owned")
    require(all(not (p.stat().st_mode & 0o022) for p in (path, *path.parents)),
            "a3-area-ancestors-not-private")


def invoke(argv, folder, prefix):
    """Run one finite Python command with EOF, retaining argv and raw streams.

    No shell, retries, timeout or service. Return the actual exit and streams;
    expected failure is judged by the caller, never converted into runner PASS.
    """
    save(folder / (prefix + ".argv.json"), argv)
    with (folder / (prefix + ".stdout")).open("xb") as out, \
            (folder / (prefix + ".stderr")).open("xb") as err:
        result = subprocess.run(argv, stdin=subprocess.DEVNULL, stdout=out, stderr=err)
        out.flush(); os.fsync(out.fileno())
        err.flush(); os.fsync(err.fileno())
    save(folder / (prefix + ".exit.json"), {"exit": result.returncode})
    return result.returncode, (folder / (prefix + ".stdout")).read_bytes()


def events(root):
    """Read append-only sentinel observations; absence means no sentinel recorded."""
    path = root / "events.jsonl"
    return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []


def run_phase(runner, root, manifest, folder, outcome):
    """Observe one control/fault/recovery through the real runner CLI.

    Require exact exits/causes, sentinel deltas, effective TMPDIR and integrity of
    complete evidence. Preflight refusal has no run evidence and is not verified
    as a fabricated FAIL. Writes only owned phase evidence; exceptions stop suite.
    """
    folder.mkdir(mode=0o700)
    prior = events(root)
    save(folder / "events.before.json", prior)
    save(folder / "control.before.json", (root / "control.json").read_bytes())
    output = folder / "run"
    code, stdout = invoke([sys.executable, "-B", str(runner), "run", "--root", str(root),
                           "--recipe", str(manifest), "--output", str(output)], folder, "run")
    observed = events(root)
    save(folder / "events.after.json", observed)
    require(observed[:len(prior)] == prior, "a3-event-history-rewritten")
    delta = observed[len(prior):]
    target = json.loads((root / "control.json").read_text())["target"]
    path = root / target
    save(folder / "target.after.json", {"exists": path.exists(),
         "sha256": digest(path.read_bytes()) if path.exists() else None})
    if path.exists():
        save(folder / "target.after.bin", path.read_bytes())
    if outcome == "missing":
        response = json.loads(stdout)
        require(code == 2 and response.get("status") == "REFUSED" and
                response.get("cause") == "recipe-input-pattern-empty", "a3-missing-cause")
        require(not output.exists() and not delta, "a3-missing-started-work")
        return {"outcome": "REFUSED", "exit": code, "sentinels": 0, "verified": False}
    result = json.loads((output / "result.json").read_text())
    expected = ["first"] if outcome == "modified" else ["first", "second"]
    require([e["sentinel"] for e in delta] == expected, "a3-sentinel-delta")
    expected_tmp = str(root / "fixture-area" / "runner-tmp")
    environment = json.loads((output / "environment.json").read_text())
    require(environment["overrides"]["TMPDIR"] == expected_tmp and
            all(e["tmpdir"] == expected_tmp for e in delta), "a3-unexpected-tmpdir")
    if outcome == "modified":
        require(code == 1 and result["status"] == "FAIL" and len(result["steps"]) == 1 and
                result["failure"]["cause"] == "recipe-declared-inputs-changed",
                "a3-modified-cause")
        before = json.loads((output / "inputs.before.json").read_text())
        after = json.loads((output / "inputs.after.json").read_text())
        require(set(before) == set(after) and
                {p for p in before if before[p] != after[p]} == {target},
                "a3-drift-not-exclusive-to-target")
    else:
        require(code == 0 and result["status"] == "PASS" and len(result["steps"]) == 2 and
                result["failure"] is None, "a3-control-not-valid")
        selected = json.loads((output / "inputs.before.json").read_text())
        require(all(p in selected for p in INPUTS), "a3-control-inputs-missing")
    verify_code, verify_raw = invoke([sys.executable, "-B", str(runner), "verify",
                                      "--evidence", str(output)], folder, "verify")
    verified = json.loads(verify_raw)
    require(verify_code == code and verified.get("integrity") == "VERIFIED" and
            verified.get("recorded_status") == result["status"], "a3-evidence-not-intact")
    return {"outcome": result["status"], "exit": code, "sentinels": len(delta), "verified": True}


def run_group(base, sources, recipe_name, target, fault):
    """Run a fresh three-phase group on copied inputs and one immutable recipe.

    Same sentinel program for all phases. Only the owned control flag or target
    bytes vary. Restore exact originals before recovery; retain fault evidence.
    No native recipe step, wrapper or policy evaluator is invoked.
    """
    group = base / (recipe_name + "-" + target.replace("/", "_") + "-" + fault)
    group.mkdir(mode=0o700)
    root = group / "root"; root.mkdir(mode=0o700)
    for name in INPUTS:
        path = root / name; path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        shutil.copy2(sources / name, path)
    (root / ".swarm").mkdir(mode=0o700)
    # mkdir(parents=True) in the runner need not give intermediate parents its
    # requested leaf mode. Establish this owned setting's valid private control
    # explicitly; never chmod a shared directory or relax the runner's fallback.
    (root / "fixture-area").mkdir(mode=0o700)
    setting = {"values": {"runtime_temporary_directory": str(root / "fixture-area")}}
    save(root / ".swarm/engine-settings.json", setting)
    save(group / "setting.original.json", setting)
    save(root / "step.py", SENTINEL.encode())
    original_recipe = json.loads((sources / "tools/verification/recipes" /
                                 (recipe_name + ".json")).read_text())
    selected = [p for p in original_recipe["inputs"] if p in INPUTS]
    require(set(selected) == set(INPUTS) and len(selected) == len(INPUTS),
            "a3-original-declarations-incomplete")
    steps = [{"id": phase, "command": [sys.executable, "-B", "step.py", phase, target],
              "expected_exit": 0, "required_stdout": ["A3_" + phase.upper()],
              "required_stderr": []} for phase in ("first", "second")]
    manifest = root / "recipe.json"
    save(manifest, {"schema_version": 1, "id": "a3-" + fault, "inputs": selected +
                   ["step.py", "control.json"], "environment": {}, "steps": steps})
    manifest_raw = manifest.read_bytes()
    control = root / "control.json"
    control.write_text(json.dumps({"target": target, "mutate": False}))
    runner = sources / "tools/verification/recipe.py"
    witness = run_phase(runner, root, manifest, group / "witness", "valid")
    original = (root / target).read_bytes()
    if fault == "missing":
        (root / target).unlink()
    else:
        control.write_text(json.dumps({"target": target, "mutate": True}))
    failure = run_phase(runner, root, manifest, group / "fault", fault)
    fault_raw = (group / "fault/run.stdout").read_bytes()
    (root / target).write_bytes(original)
    control.write_text(json.dumps({"target": target, "mutate": False}))
    recovery = run_phase(runner, root, manifest, group / "recovery", "valid")
    require((root / target).read_bytes() == original and manifest.read_bytes() == manifest_raw,
            "a3-recovery-not-original")
    require((group / "fault/run.stdout").read_bytes() == fault_raw, "a3-fault-overwritten")
    record = {"recipe": recipe_name, "target": target, "fault": fault,
              "witness": witness, "failure": failure, "recovery": recovery,
              "manifest_sha256": digest(manifest_raw), "original_sha256": digest(original)}
    save(group / "group.json", record)
    print("A3_GROUP_OK " + recipe_name + " " + target + " " + fault, flush=True)
    return record


def main():
    """Create private fixtures under observed TMPDIR and retain the complete matrix.

    Optional output must be new and private. Source changes or any missing
    assertion stop with nonzero status and retained partial evidence. No cleanup
    or automatic retry; callers own evidence retention and publication authority.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    area = Path(os.environ["TMPDIR"]).resolve(); private_area(area)
    if args.output:
        base = args.output.resolve(); base.mkdir(mode=0o700, parents=False, exist_ok=False)
    else:
        base = Path(tempfile.mkdtemp(prefix="bf-l0-a-a3-", dir=area))
    private_area(base)
    print("A3_EVIDENCE " + str(base), flush=True)
    names = [*INPUTS, "tools/verification/recipe.py", "tools/verification/bf_l0_a_inputs.py",
             "tools/verification/recipes/bf-l0-a-inputs.json"]
    names += ["tools/verification/recipes/" + name + ".json" for name in RECIPES]
    originals = {name: (REPOSITORY / name).read_bytes() for name in names}
    sources = base / "sources"; sources.mkdir(mode=0o700)
    for name, raw in originals.items():
        path = sources / name; path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        save(path, raw)
    save(base / "source-hashes.json", {p: digest(raw) for p, raw in originals.items()})
    try:
        groups = [run_group(base, sources, recipe, target, fault) for recipe in RECIPES
                  for target in INPUTS for fault in ("missing", "modified")]
        require(all((REPOSITORY / p).read_bytes() == raw for p, raw in originals.items()),
                "a3-live-sources-changed")
        require(all(g["witness"]["outcome"] == g["recovery"]["outcome"] == "PASS"
                    for g in groups), "a3-incomplete-controls")
        summary = {"scope": "A-3 miniatures only; not native recipe, compiler/profile or product acceptance",
                   "groups": groups, "groups_count": len(groups), "phases": len(groups) * 3,
                   "controls_recoveries": len(groups) * 2,
                   "missing_refusals": sum(g["fault"] == "missing" for g in groups),
                   "drift_failures": sum(g["fault"] == "modified" for g in groups),
                   "verified_complete_runs": sum(p["verified"] for g in groups for p in
                       (g["witness"], g["failure"], g["recovery"])), "sources_unchanged": True}
        save(base / "summary.json", summary)
        for label, key in [("A3_GROUPS_OK", "groups_count"), ("A3_PHASES_OK", "phases"),
                           ("A3_CONTROLS_AND_RECOVERIES_OK", "controls_recoveries"),
                           ("A3_MISSING_REFUSALS_OK", "missing_refusals"),
                           ("A3_DRIFT_FAILURES_OK", "drift_failures"),
                           ("A3_FULL_RUN_EVIDENCE_VERIFIED", "verified_complete_runs")]:
            print(label + " " + str(summary[key]), flush=True)
        print("A3_SOURCE_UNCHANGED", flush=True)
        return 0
    except BaseException as error:
        save(base / "failure.json", {"type": type(error).__name__, "cause": str(error)})
        raise


if __name__ == "__main__":
    raise SystemExit(main())
