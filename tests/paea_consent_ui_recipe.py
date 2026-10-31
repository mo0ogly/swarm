#!/usr/bin/env python3
"""Permanent-recipe driver for the PA-08 consent-page browser qualification.

Builds nothing and mocks nothing: the recipe declares the cargo build/test
steps and passes the freshly built test binary here. The driver runs the real
--ignored Rust test ``consent_page_serves_real_bound_approval_to_real_browser``
(real bound approval via prepare_approval + bind_git_call_with_approval, real
native server, real persisted session) and the real served consent page in
real Chrome through the owned two-phase marker protocol. The page itself
performs both refusal submissions (drifted bytes, wrong password); no approval
effect, grant or consumption is produced, no provider and no live mission
database. A failing run preserves both channels and the fixture roots for
diagnosis.
"""
import argparse,json,os,subprocess,sys,tempfile,traceback
from datetime import datetime, timezone
from pathlib import Path

def resolve_binary(pattern: Path) -> Path:
    """Resolve the cargo-built test binary from a glob-style pattern.

    `cargo test --no-run` leaves one current executable under debug/deps with
    the test target name (hash-suffixed); the exact executable name is printed
    by cargo but not predictable, so the driver resolves the newest matching
    executable itself. Exactly one match is accepted; anything else fails.
    """
    candidates = sorted(
        p for p in pattern.parent.glob(pattern.name)
        if p.is_file() and os.access(p, os.X_OK) and not p.name.endswith(('d', 'so'))
    )
    if len(candidates) != 1:
        raise SystemExit(f"expected exactly one test binary for {pattern}, got {candidates}")
    return candidates[0]


def run(binary: Path, output: Path) -> None:
    """Run the owned two-phase browser qualification and archive all evidence.

    Inputs: the fresh cargo-built test binary path and an output
    directory. Outputs: harness stdout/stderr, consent-bound-ui.json,
    fresh/refusal screenshots and result.json with the binary digest. Errors:
    any nonzero exit, missing PASS result or unexpected diagnostics is a hard
    failure; control markers and server.json stay in the owned control
    directory under output for replay/diagnosis.
    """
    # Evidence is never overwritten: a leftover directory from an earlier run
    # (usually a prior failure kept for diagnosis) is archived aside with a
    # timestamp and the fresh run starts empty at the declared path.
    if output.exists():
        archive = output.with_name(
            f"{output.name}-superseded-{datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')}"
        )
        output.rename(archive)
        print(f"prior evidence archived aside: {archive}", flush=True)
    output.mkdir(parents=True, exist_ok=False)
    control = output / "control"
    control.mkdir(mode=0o700)
    env = dict(os.environ,
               SWARM_PAEA_SOURCE_FIXTURE=str(Path(os.environ["SWARM_PAEA_SOURCE_FIXTURE"]).resolve()),
               CONSENT_BROWSER_DIR=str(control))
    # The Rust test must run first: it binds the real server and publishes
    # server.json, which the browser harness reads at startup. Starting the
    # harness before the test races a guaranteed ENOENT (same observed order
    # constraint as the bound-preview qualification, 2026-10-30).
    rust = subprocess.Popen(
        [str(binary if binary.is_file() else resolve_binary(binary)),
         "--exact", "consent_page_serves_real_bound_approval_to_real_browser",
         "--ignored", "--nocapture", "--test-threads=1"],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env,
    )
    harness = None
    try:
        # Generous owned start budget (real fixture open + Git); the marker
        # protocol owns every later phase boundary. A rust exit before
        # publication means the bind refused: capture and fail with its output.
        for _ in range(1200):
            if (control / "server.json").exists() or rust.poll() is not None:
                break
            import time; time.sleep(0.5)
        if not (control / "server.json").exists():
            rust.terminate()
            out, err = rust.communicate(timeout=60)
            (output / "rust.stdout").write_bytes(out)
            (output / "rust.stderr").write_bytes(err)
            raise SystemExit("server.json never published by the Rust test")
        harness = subprocess.Popen(
            ["node", "tests/paea_consent_ui.cjs", str(binary), str(output), str(control)],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env,
        )
        out, err = harness.communicate(timeout=1200)
        (output / "harness.stdout").write_bytes(out)
        (output / "harness.stderr").write_bytes(err)
        rust_out, rust_err = rust.communicate(timeout=600)
        (output / "rust.stdout").write_bytes(rust_out)
        (output / "rust.stderr").write_bytes(rust_err)
    except BaseException:
        if harness is not None:
            harness.kill()
            h_out, h_err = harness.communicate()
            (output / "harness.stdout").write_bytes(h_out)
            (output / "harness.stderr").write_bytes(h_err)
        rust.kill()
        r_out, r_err = rust.communicate()
        (output / "rust.stdout").write_bytes(r_out)
        (output / "rust.stderr").write_bytes(r_err)
        raise
    if rust.returncode != 0:
        raise SystemExit(f"rust test failed: exit {rust.returncode}")
    if harness.returncode != 0:
        raise SystemExit(f"browser harness failed: exit {harness.returncode}")
    proof = json.loads((output / "consent-bound-ui.json").read_bytes())
    assert len(proof["checks"]) == 2, proof["checks"]
    assert proof["unexpected_diagnostics"] == [], proof["unexpected_diagnostics"]
    resolved = binary if binary.is_file() else resolve_binary(binary)
    digest = subprocess.run(["sha256sum", str(resolved)], capture_output=True,
                            check=True).stdout.decode().split()[0]
    (output / "result.json").write_text(json.dumps({
        "schema_version": 1,
        "status": "PASS",
        "checks": proof["checks"],
        "rust_exit": rust.returncode,
        "binary_sha256": digest,
        "scope": "real bound approval served to a real browser; page-driven "
                 "visible refusals for wrong bytes and wrong password; no "
                 "approval effect, no consumption, no provider",
    }, indent=2) + "\n")
    print("PAEA_CONSENT_BROWSER_VERIFIED 2", flush=True)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    try:
        run(args.binary.resolve(), args.output.resolve())
    except BaseException:
        if args.output.exists():
            (args.output / "failure.txt").write_text(traceback.format_exc())
        raise
