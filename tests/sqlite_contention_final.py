"""Fail-closed host qualification for the SQLite contention candidate.

The worker may run --measure-only. The host conductor runs the default mode
exactly once; it includes the discovered Go suite exactly once via the preserved
supervision_go_suite.py harness.
"""
import argparse
import hashlib
import json
import os
import shutil
import statistics
import subprocess
import sys
import tarfile
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REQUIRED = {
    "TestSQLiteAuditPlanning",
    "TestSQLiteAuditReview",
    "TestSQLiteRecoveryPlanning",
    "TestSQLiteRecoveryReview",
    "TestSQLiteRecoveryPersistent",
    "TestSQLiteConfigCLI",
    "TestSQLiteConfigHTTP",
    "TestSQLiteQualificationMeasurement",
}


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[max(0, min(len(ordered) - 1, int(len(ordered) * fraction + 0.999999) - 1))]


class Runner:
    def __init__(self, timeout):
        self.deadline = time.monotonic() + timeout
        self.commands = []

    def run(self, command, *, cwd=ROOT, env=None, capture=False):
        remaining = self.deadline - time.monotonic()
        if remaining <= 0:
            raise RuntimeError("global qualification deadline exceeded")
        shown = [str(part) for part in command]
        print("COMMAND " + json.dumps(shown), flush=True)
        started = time.monotonic()
        process = subprocess.run(
            shown, cwd=cwd, env=env, text=True,
            stdout=subprocess.PIPE if capture else None,
            stderr=subprocess.STDOUT if capture else None,
            timeout=remaining,
        )
        elapsed = time.monotonic() - started
        self.commands.append({"command": shown, "exit_code": process.returncode, "duration_seconds": round(elapsed, 3)})
        if capture and process.stdout:
            print(process.stdout, end="" if process.stdout.endswith("\n") else "\n", flush=True)
        print(f"EXIT {process.returncode} duration_seconds={elapsed:.3f}", flush=True)
        if process.returncode:
            raise RuntimeError(f"command failed with exit {process.returncode}: {shown}")
        return process.stdout or ""


def measurement_summary(path):
    payload = json.loads(path.read_text())
    if payload.get("schema_version") != 1 or len(payload.get("samples", [])) != 9:
        raise RuntimeError(f"invalid measurement inventory: {path}")
    samples = payload["samples"]
    durations = [sample["duration_ms"] for sample in samples]
    return {
        "workload": payload["workload"],
        "samples": len(samples),
        "latency_ms": {
            "p50": round(statistics.median(durations), 3),
            "p95": round(percentile(durations, 0.95), 3),
            "max": round(max(durations), 3),
        },
        "initial_storage_errors": sum(bool(sample["initial_storage_busy"]) for sample in samples),
        "durable_business_effects": sum(sample["final_state"] == "passed" for sample in samples),
        "business_errors": sum(sample["final_state"] != "passed" for sample in samples),
        "review_calls": sum(sample["review_calls"] for sample in samples),
        "attempts": sum(sample["attempts"] for sample in samples),
        "final_states": sorted({sample["final_state"] for sample in samples}),
    }


def measure(runner, baseline_ref):
    with tempfile.TemporaryDirectory(prefix="sqlite-contention-qualification-") as folder:
        temp = Path(folder)
        baseline = temp / "baseline"
        baseline.mkdir()
        archive = temp / "baseline.tar"
        runner.run(["git", "archive", "--format=tar", baseline_ref, "-o", str(archive)])
        with tarfile.open(archive) as bundle:
            bundle.extractall(baseline, filter="data")
        for name in ("sqlite_contention_audit_test.go", "sqlite_contention_qualification_test.go"):
            shutil.copy2(ROOT / "internal" / "engine" / name, baseline / "internal" / "engine" / name)
        archive_input = Path("docs/benchmarks/billing/analyse-32-exclusions-20261008.json")
        (baseline / archive_input).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / archive_input, baseline / archive_input)
        outputs = {}
        for label, cwd in (("before_git_head", baseline), ("after_candidate", ROOT)):
            output = temp / f"{label}.json"
            env = dict(os.environ, SQLITE_QUALIFICATION_OUTPUT=str(output))
            runner.run(["go", "test", "./internal/engine", "-run", "^TestSQLiteQualificationMeasurement$", "-count=1", "-timeout", "240s"], cwd=cwd, env=env)
            outputs[label] = measurement_summary(output)
        before, after = outputs["before_git_head"], outputs["after_candidate"]
        if before["workload"] != after["workload"] or before["samples"] != after["samples"]:
            raise RuntimeError("before/after workload mismatch")
        if before["initial_storage_errors"] != 9 or after["initial_storage_errors"] != 9:
            raise RuntimeError("contention trigger did not affect every sample")
        if before["durable_business_effects"] != 0 or before["business_errors"] != 9:
            raise RuntimeError("Git baseline no longer demonstrates the pre-candidate failure")
        if after["durable_business_effects"] != 9 or after["business_errors"] != 0:
            raise RuntimeError("candidate did not recover every business effect")
        if before["review_calls"] != after["review_calls"] or before["attempts"] != after["attempts"]:
            raise RuntimeError("candidate changed review-call or attempt consumption")
        result = {
            "schema_version": 1,
            "measurement": "same scripted workload; isolated temporary roots; local provider double",
            "before": before,
            "after": after,
            "provider_autonomy": "not_demonstrated",
        }
        encoded = json.dumps(result, ensure_ascii=False, sort_keys=True).encode()
        result["result_sha256"] = hashlib.sha256(encoded).hexdigest()
        print("MEASUREMENT " + json.dumps(result, ensure_ascii=False, sort_keys=True), flush=True)
        return result


def targeted_race(runner):
    pattern = "^(" + "|".join(sorted(REQUIRED)) + ")$"
    output = runner.run(["go", "test", "-race", "./internal/engine", "-json", "-count=1", "-run", pattern, "-timeout", "300s"], capture=True)
    events = [json.loads(line) for line in output.splitlines() if line.startswith("{")]
    passed = {event.get("Test") for event in events if event.get("Action") == "pass"}
    skipped = {event.get("Test") for event in events if event.get("Action") == "skip" and event.get("Test")}
    missing = REQUIRED - passed
    if missing or skipped:
        raise RuntimeError(f"targeted race missing={sorted(missing)} skipped={sorted(skipped)}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--timeout", type=int, required=True)
    parser.add_argument("--measure-only", action="store_true")
    parser.add_argument("--baseline-ref", required=True, help="Explicit pre-correction Git revision for the before/after measurement")
    args = parser.parse_args()
    if args.timeout < 1:
        parser.error("positive global timeout required")
    os.chdir(ROOT)
    runner = Runner(args.timeout)
    measurement = measure(runner, args.baseline_ref)
    if args.measure_only:
        print(json.dumps({"status": "PASS", "scope": "measurement_only", "measurement": measurement, "commands": runner.commands}, ensure_ascii=False))
        return
    targeted_race(runner)
    runner.run(["go", "vet", "./..."])
    runner.run(["npm", "test"])
    runner.run(["python3", "tools/agent-workflows/check.py"])
    runner.run(["git", "diff", "--check"])
    # This is the sole full-suite invocation in the recipe. The preserved harness
    # compiles once, inventories once, partitions disjointly and fails on missing
    # or incomplete tests while reporting every optional skip.
    runner.run(["python3", "tests/supervision_go_suite.py"])
    print(json.dumps({
        "status": "PASS",
        "measurement": measurement,
        "targeted_race": {"required": sorted(REQUIRED), "skips": []},
        "full_go_suite_invocations": 1,
        "provider_autonomy": "not_demonstrated",
        "commands": runner.commands,
    }, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.TimeoutExpired, json.JSONDecodeError, OSError) as error:
        print(f"QUALIFICATION FAILED: {error}", file=sys.stderr, flush=True)
        raise SystemExit(1)
