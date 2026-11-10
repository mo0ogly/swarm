#!/usr/bin/env python3
"""Permanent-recipe driver for the PA-08 bound-preview browser qualification.

Builds nothing and mocks nothing: the recipe declares the cargo build/test
steps and passes the freshly built test binary here. The driver runs the real
--ignored Rust test ``preview_bound_serves_real_browser_session`` (real bound
prepared call, real native server, real persisted session, real public Trust
revision change) and the real served page in real Chrome through the owned
two-phase marker protocol. No provider, no paid call, no live mission
database; the browser only reads the guarded GET endpoints through the page
and never performs a guarded call. A failing run preserves both channels and
the fixture roots for diagnosis.
"""
import argparse,json,os,subprocess,sys,tempfile,traceback
from pathlib import Path

def run(binary: Path, output: Path) -> None:
    """Run the owned two-phase browser qualification and archive all evidence.

    Inputs: the fresh cargo-built test binary path and a fresh output
    directory. Outputs: harness stdout/stderr, preview-bound-ui.json,
    fresh/stale screenshots and result.json with the binary digest. Errors:
    any nonzero exit, missing PASS result or unexpected diagnostics is a hard
    failure; control markers and server.json stay in the owned control
    directory under output for replay/diagnosis.
    """
    output.mkdir(parents=True, exist_ok=False)
    control = output / "control"
    control.mkdir(mode=0o700)
    env = dict(os.environ,
               SWARM_PAEA_SOURCE_FIXTURE=str(Path(os.environ["SWARM_PAEA_SOURCE_FIXTURE"]).resolve()),
               PREVIEW_BOUND_BROWSER_DIR=str(control))
    # The Rust test must run first: it binds the real server and publishes
    # server.json, which the browser harness reads at startup. Starting the
    # harness before the test races a guaranteed ENOENT (observed 2026-10-30:
    # harness died at line 17 before the test ever bound a socket).
    rust = subprocess.Popen(
        [str(binary), "--exact", "preview_bound_serves_real_browser_session",
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
            ["node", "tests/paea_preview_bound_ui.cjs", str(binary), str(output), str(control)],
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
            out, err = harness.communicate()
            (output / "harness.stdout").write_bytes(out)
            (output / "harness.stderr").write_bytes(err)
        rust.kill()
        out, err = rust.communicate()
        (output / "rust.stdout").write_bytes(out)
        (output / "rust.stderr").write_bytes(err)
        raise
    if rust.returncode != 0:
        raise SystemExit(f"rust test failed: exit {rust.returncode}")
    if harness.returncode != 0:
        raise SystemExit(f"browser harness failed: exit {harness.returncode}")
    proof = json.loads((output / "preview-bound-ui.json").read_bytes())
    assert len(proof["checks"]) == 2, proof["checks"]
    assert proof["unexpected_diagnostics"] == [], proof["unexpected_diagnostics"]
    digest = subprocess.run(["sha256sum", str(binary)], capture_output=True,
                            check=True).stdout.decode().split()[0]
    (output / "result.json").write_text(json.dumps({
        "schema_version": 1,
        "status": "PASS",
        "checks": proof["checks"],
        "rust_exit": rust.returncode,
        "binary_sha256": digest,
        "scope": "real bound preparation served to a real browser, fresh then stale; "
                 "no consent, no consumption, no provider",
    }, indent=2) + "\n")
    print("PAEA_PREVIEW_BOUND_BROWSER_VERIFIED 2", flush=True)

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
