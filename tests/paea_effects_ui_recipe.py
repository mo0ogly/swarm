#!/usr/bin/env python3
"""Permanent-recipe driver for the PA-08 effect-state browser qualification.

Builds nothing itself: the recipe declares the cargo build and cargo test steps
and passes the freshly built swarm-rust binary here. The driver runs the real
served page against an isolated 0700 fixture root (real init, real web server,
real session credential) and archives the full browser evidence. No provider,
no paid call, no live mission database; the driver only reads the two guarded
GET endpoints through the page and never performs a guarded call.
"""
import argparse,json,os,subprocess,sys,traceback
from pathlib import Path

def run(binary: Path, output: Path) -> None:
    """Archive one complete browser run under ``output`` and fail on any error.

    Inputs: an existing swarm-rust binary path and a fresh (nonexistent)
    output directory. Outputs: browser stdout/stderr, paea-effects-ui.json,
    per-language screenshots and result.json with the binary digest. Errors:
    any nonzero child exit, missing PASS result or leftover fixture is a hard
    failure; the captured stdout/stderr is preserved for diagnosis.
    """
    output.mkdir(parents=True, exist_ok=False)
    result = subprocess.run(
        ["node", "tests/paea_effects_ui.cjs", str(binary), str(output)],
        capture_output=True, timeout=None,
    )
    (output / "browser.stdout").write_bytes(result.stdout)
    (output / "browser.stderr").write_bytes(result.stderr)
    if result.returncode != 0:
        print(result.stderr.decode(errors="replace"), file=sys.stderr)
        raise SystemExit(f"browser run failed: exit {result.returncode}")
    proof = json.loads((output / "paea-effects-ui.json").read_bytes())
    assert len(proof["checks"]) == 5, proof["checks"]
    assert proof["unexpected_diagnostics"] == [], proof["unexpected_diagnostics"]
    assert proof["expected_denials"] >= 1, "expected fixture refusals must be recorded"
    digest = subprocess.run(
        ["sha256sum", str(binary)], capture_output=True, check=True,
    ).stdout.decode().split()[0]
    (output / "result.json").write_text(json.dumps({
        "schema_version": 1,
        "status": "PASS",
        "checks": len(proof["checks"]),
        "expected_denials": proof["expected_denials"],
        "binary_sha256": digest,
        "scope": "real served browser page, isolated 0700 fixture; no provider, no guarded call",
    }, indent=2) + "\n")
    print("PAEA_EFFECTS_UI_BROWSER_VERIFIED 5", flush=True)

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
