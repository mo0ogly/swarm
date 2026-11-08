#!/usr/bin/env python3
"""Bind a cargo-built test binary as a recipe OUTPUT, never a declared input.

Why: `recipe.py` treats every declared input as immutable for the whole run,
so a manifest must never declare a path its own cargo steps create (the
fingerprint metadata of `test-integration-test-paea_git_call` is written by
the run itself — declaring it made every fresh run fail
`recipe-declared-inputs-changed` with no source change, observed 2026-10-31
on `paea-approval-form-rust` run4). The linked-output contract is different:
build/resolve the exact cargo artifact, retain its identity, run, then
re-check that identity — a REAL concurrent edit of a compiled input still
fails `recipe-declared-inputs-changed`, while the run's own artifact
production passes without hiding source changes.

Behavior: re-runs `cargo test --no-run --message-format=json` in the
workspace rust/ directory (cheap when fresh, exactly like the recipe driver's
resolution) to get cargo's own artifact path — never an mtime or a glob guess.
`--verify-stable` re-resolves and compares the retained binding; a changed
path or digest fails with exit 1 and a typed cause, without rewriting the
retained binding. Publication is atomic: the new content is fully written
and fsynced to a temporary file first, then renamed over the retained
record, so a write failure never truncates the prior binding and a re-run
rebinds cleanly (the retained record is a cache describing the binary this
run built, not append-only evidence — per-run history lives in the recipe
evidence directories).
"""
import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

REPOSITORY = Path(__file__).resolve().parents[2]


def digest(path):
    """Hash complete file bytes; a partial read is never a binding."""
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def resolve(deps, target):
    """Ask cargo for the exact emitted test-executable path (never mtimes).

    Inputs: `deps` is the shared CARGO_TARGET_DIR debug/deps directory and
    `target` the integration-test target name. The stream must yield exactly
    one existing executable whose basename starts with the target name —
    cargo itself owns the hash suffix. Errors: no/ambiguous/missing artifact
    refuse with exit 2; the raw cargo stderr is retained in the cause.
    """
    workspace = REPOSITORY / "rust"
    environment = dict(os.environ, CARGO_TARGET_DIR=str(deps.parent.parent))
    proc = subprocess.run(
        ["cargo", "test", "--no-run", "--message-format=json",
         "--manifest-path", str(workspace / "Cargo.toml"),
         "--offline", "--locked", "-p", "swarm-web", "--test", target],
        cwd=REPOSITORY, env=environment, capture_output=True, text=True)
    if proc.returncode != 0:
        print(json.dumps({"status": "REFUSED", "cause": "cargo-artifact-resolution-failed",
                          "cargo_stderr_tail": proc.stderr[-2000:]},
                         separators=(",", ":")))
        return None
    matches = []
    for line in proc.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("reason") != "compiler-artifact":
            continue
        executable = message.get("executable")
        if executable and Path(executable).name.startswith(target):
            matches.append(Path(executable))
    candidates = [p for p in matches if p.is_file() and os.access(p, os.X_OK)]
    if len(candidates) != 1:
        print(json.dumps({"status": "REFUSED", "cause": "cargo-artifact-ambiguous",
                          "matches": sorted(str(p) for p in matches)},
                         separators=(",", ":")))
        return None
    return candidates[0]


def bind(deps, target, output):
    """Resolve once and publish the retained identity atomically.

    Retained: cargo-emitted exact path, sha256, size and target. The resolved
    path plus digest is the linked OUTPUT record consumed by the later
    `--verify-stable` step; it proves which binary the run exercised, not
    authorship or acceptance. Re-running a recipe legitimately rebuilds the
    binary (the build step forces it), so binding atomically REPLACES a prior
    record at the same cache path; a resolution failure still leaves any
    prior binding intact.
    """
    resolved = resolve(deps, target)
    if resolved is None:
        return 2
    record = {"schema_version": 1, "target": target,
              "artifact": str(resolved), "sha256": digest(resolved),
              "bytes": resolved.stat().st_size}
    publish(output, (json.dumps(record, indent=2, sort_keys=True) + "\n").encode())
    # Compact stdout: the recipe's required_stdout asserts the exact compact
    # token ("status":"BOUND"), matching the Node-style JSON conventions used
    # by the other recipe steps' assertions.
    print(json.dumps({"status": "BOUND", "artifact": record["artifact"],
                      "sha256": record["sha256"]},
                     sort_keys=True, separators=(",", ":")))
    return 0


def publish(path, raw):
    """Write, fsync, then atomically rename over the retained record.

    The record is a cache re-bound by every recipe run, so exclusive link
    publication (os.link, refusing an existing path) made the second run die
    with FileExistsError (observed 2026-10-31 on paea-approval-form-rust run2
    step 6). The temporary file carries the complete new content and is
    fsynced before the rename, so any write failure leaves the prior binding
    intact; os.replace never truncates in place and is atomic per POSIX.
    """
    import tempfile
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, temporary = tempfile.mkstemp(dir=path.parent, prefix=".bind-")
    try:
        with os.fdopen(handle, "wb") as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        directory = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        Path(temporary).unlink(missing_ok=True)


def verify_stable(deps, target, output):
    """Re-resolve and compare against the retained binding without rewriting it.

    Inputs: the retained binding file from the earlier `bind` step. The exact
    same artifact (path+digest) must still resolve; anything else — rebuild to
    a new hash suffix, digest change, missing file — fails typed without
    touching the retained record. Exit 0 only on byte-stable identity.
    """
    retained = json.loads(Path(output).read_bytes())
    resolved = resolve(deps, target)
    if resolved is None:
        return 2
    actual = {"target": target, "artifact": str(resolved),
              "sha256": digest(resolved), "bytes": resolved.stat().st_size}
    if {k: retained.get(k) for k in actual} != actual:
        print(json.dumps({"status": "REFUSED", "cause": "bound-binary-unstable",
                          "retained": {k: retained.get(k) for k in actual},
                          "observed": actual},
                         sort_keys=True, separators=(",", ":")))
        return 1
    print(json.dumps({"status": "STABLE", "artifact": actual["artifact"],
                      "sha256": actual["sha256"]},
                     sort_keys=True, separators=(",", ":")))
    return 0


def main():
    """Expose bind/verify-stable; exit 0 only for BOUND/STABLE, 1 refusal, 2 prerequisite."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--deps", type=Path, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify-stable", action="store_true")
    args = parser.parse_args()
    if args.verify_stable:
        return verify_stable(args.deps, args.target, args.output)
    return bind(args.deps, args.target, args.output)


if __name__ == "__main__":
    sys.exit(main())
