#!/usr/bin/env python3
"""Permanent-recipe driver for the PAEA approval-form browser qualification.

Mocks nothing: the recipe declares the cargo build/test steps and passes the
freshly built test binary here. The driver runs the real --ignored Rust test
``approval_form_serves_real_browser_session`` (real frozen approval command,
real native server, real persisted session, real config-apply Trust revision
change) and the real served approval form in real Chrome through the owned
two-phase marker protocol. The browser performs the explicit confirmation +
empty-body enrollment consumption once through the page with the labeled
fixture password (auth simulated), then observes the stale refusal. No
provider, no paid call, no live mission database; no Git effect. A failing
run preserves both channels and the fixture roots for diagnosis.

Proof method: the binary is resolved from the cargo JSON artifact stream
(``cargo test --no-run --message-format=json``), never from modification
times. Note that this resolution MAY compile: cargo can build the test binary
(or dependencies) when the shared CARGO_TARGET_DIR is not fresh — the
recipe's separate build step is what guarantees the fresh build, this
resolution only re-runs cargo to read the artifact path from the emitted
JSON. The digest captured is the digest of the exact executed file, taken
before and after the run. Both child processes stream their raw output to
their own files as it arrives, both are awaited to their real exits, the
streamer threads are joined with their liveness verified (a timeout is a
hard failure, never a silent partial flush), and a Rust test that dies while
the browser harness runs fails the run immediately with its captured output.
"""
import argparse,json,os,subprocess,sys,threading,time,traceback
from datetime import datetime, timezone
from pathlib import Path

def resolve_binary(spec: Path) -> Path:
    """Resolve the EXACT cargo-built test executable from the artifact spec.

    Inputs: ``spec`` is a path whose filename is the test target's crate-name
    prefix (for example ``paea_git_call``) under the shared
    ``CARGO_TARGET_DIR`` deps directory. The recipe's build step therefore
    runs ``cargo test --no-run --message-format=json`` and the recipe runner
    saves the raw stream; here the driver itself asks cargo once more — cheap
    when fresh — and selects the ``compiler-artifact`` message whose
    ``executable`` basename starts with the spec filename, reading the exact
    hash-suffixed path cargo itself emitted. Errors: no matching artifact or
    a missing/non-executable file is a hard failure; modification times are
    never consulted.
    """
    prefix = spec.name
    deps = spec.parent                       # .../debug/deps
    target_dir = deps.parent.parent          # shared CARGO_TARGET_DIR
    workspace = target_dir.parent.parent     # repository root
    proc = subprocess.run(
        ["cargo", "test", "--no-run", "--message-format=json",
         "-p", "swarm-web", "--test", "paea_git_call"],
        cwd=workspace / "rust", env=dict(os.environ, CARGO_TARGET_DIR=str(target_dir)),
        capture_output=True, text=True)
    if proc.returncode != 0:
        raise SystemExit(f"cargo artifact resolution failed:\n{proc.stderr[-2000:]}")
    matches = []
    for line in proc.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("reason") != "compiler-artifact":
            continue
        executable = message.get("executable")
        if executable and Path(executable).name.startswith(prefix):
            matches.append(Path(executable))
    if not matches:
        raise SystemExit(f"no cargo compiler-artifact executable for {prefix}")
    candidates = [p for p in matches if p.is_file() and os.access(p, os.X_OK)]
    if len(candidates) != 1:
        raise SystemExit(f"expected exactly one executable artifact for {prefix}, "
                         f"got {sorted(str(p) for p in matches)}")
    return candidates[0]

def stream(pipe, sink: Path) -> None:
    """Stream one child pipe to its raw file as bytes arrive.

    Inputs: a live subprocess pipe and the destination path opened once in
    exclusive-append mode. Output: every byte the child emits lands in the
    file promptly (no PIPE buffer stall, no late communicate capture). The
    thread ends when the pipe closes.
    """
    with sink.open('xb') as file:
        for chunk in iter(lambda: pipe.read(4096), b''):
            file.write(chunk)
            file.flush()

def run(binary: Path, output: Path) -> None:
    """Run the owned two-phase browser qualification and archive all evidence.

    Inputs: the cargo-artifact binary spec path (deps/paea_git_call) and a
    fresh output directory. Outputs: streaming harness/rust stdout/stderr,
    approval-form-ui.json, fresh/stale screenshots and result.json with the
    exact executed binary digest (before/after). Errors: any nonzero exit,
    missing PASS result or unexpected diagnostics is a hard failure; a Rust
    test that exits while the harness still runs fails immediately with its
    captured output; control markers and server.json stay in the owned control
    directory under output for replay/diagnosis.
    """
    # Evidence is never overwritten: a leftover directory from an earlier run
    # (usually a prior failure kept for diagnosis) is archived aside with a
    # timestamp and the fresh run starts empty at the declared path. The
    # recipe therefore keeps one stable output path (same convention as the
    # consent qualification driver) instead of bumping a suffix each run.
    if output.exists():
        archive = output.with_name(
            f"{output.name}-superseded-{datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')}"
        )
        output.rename(archive)
        print(f"prior evidence archived aside: {archive}", flush=True)
    binary = resolve_binary(binary)
    output.mkdir(parents=True, exist_ok=False)
    control = output / "control"
    control.mkdir(mode=0o700)
    env = dict(os.environ,
               SWARM_PAEA_SOURCE_FIXTURE=str(Path(os.environ["SWARM_PAEA_SOURCE_FIXTURE"]).resolve()),
               APPROVAL_FORM_BROWSER_DIR=str(control))
    sha256 = lambda p: subprocess.run(["sha256sum", str(p)], capture_output=True,
                                      check=True).stdout.decode().split()[0]
    digest_before = sha256(binary)
    # The Rust test must run first: it binds the real server and publishes
    # server.json, which the browser harness reads at startup. Starting the
    # harness before the test races a guaranteed ENOENT (observed 2026-10-30
    # on the preview driver). Raw output streams to its file as produced; no
    # PIPE+communicate late capture, no global watchdog — the marker protocol
    # owns every phase boundary and both processes are awaited to their real
    # exits.
    rust_out = output / "rust.stdout"
    rust_err = output / "rust.stderr"
    rust = subprocess.Popen(
        [str(binary),
         "--exact", "approval_form_serves_real_browser_session",
         "--ignored", "--nocapture", "--test-threads=1"],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
    streamers = [threading.Thread(target=stream, args=(pipe, sink), daemon=True)
                 for pipe, sink in ((rust.stdout, rust_out), (rust.stderr, rust_err))]
    for thread in streamers:
        thread.start()
    harness = None
    try:
        # The only wait here is for the owned startup publication: the marker
        # protocol owns every later phase boundary. A rust exit before
        # publication means the bind refused: fail immediately with its output.
        while not (control / "server.json").exists():
            if rust.poll() is not None:
                raise SystemExit(
                    f"rust test exited {rust.returncode} before publishing server.json")
            time.sleep(0.2)
        harness = subprocess.Popen(
            ["node", "tests/paea-approval-form-ui.cjs", str(binary), str(output), str(control)],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
        for pipe, sink in ((harness.stdout, output / "harness.stdout"),
                           (harness.stderr, output / "harness.stderr")):
            threading.Thread(target=stream, args=(pipe, sink), daemon=True).start()
        # Fail fast if the rust test FAILS while the harness still runs: its
        # captured raw output is the diagnosis. A clean exit 0 while the
        # harness still runs is the expected success ordering, not a death:
        # the owned shutdown completes as soon as phase2.done is consumed,
        # while the harness is still closing the real browser (observed
        # 2026-10-31: exit 0 raced browser.close() and must never be turned
        # into a failure).
        while harness.poll() is None:
            if rust.poll() is not None and rust.returncode != 0:
                harness.wait()
                raise SystemExit(
                    f"rust test exited {rust.returncode} while the harness ran; "
                    f"see rust.stdout/rust.stderr")
            time.sleep(0.2)
        harness.wait()
        rust.wait()
        # Drain both channels fully before any verdict: the stream threads
        # finish only when the pipes close, so a completed join proves every
        # captured byte is on disk. A thread still alive after the join bound
        # is a hard failure (partial flush), never a silent continuation: the
        # bound is a diagnosis aid for a stalled pipe — both children have
        # already exited at this point, so a live streamer means the pipe
        # never closed, which is itself the defect to surface.
        for thread in streamers:
            thread.join(timeout=30)
            if thread.is_alive():
                raise SystemExit(
                    "output streamer still alive after both child processes "
                    "exited: captured output may be incomplete (partial flush)")
    finally:
        for process in (harness, rust):
            if process is not None and process.poll() is None:
                process.kill()
        for process in (harness, rust):
            if process is not None:
                process.wait()
    if rust.returncode != 0:
        raise SystemExit(f"rust test failed: exit {rust.returncode}")
    if harness.returncode != 0:
        raise SystemExit(f"browser harness failed: exit {harness.returncode}")
    proof = json.loads((output / "approval-form-ui.json").read_bytes())
    assert len(proof["checks"]) == 4, proof["checks"]

    assert proof["unexpected_diagnostics"] == [], proof["unexpected_diagnostics"]
    digest_after = sha256(binary)
    assert digest_before == digest_after, "executed binary changed during the run"
    (output / "result.json").write_text(json.dumps({
        "schema_version": 1,
        "status": "PASS",
        "checks": proof["checks"],
        "rust_exit": rust.returncode,
        "binary_path": str(binary),
        "binary_sha256": digest_before,
        "scope": "real frozen approval command served to a real browser; one explicit "
                 "labeled-fixture empty-body consumption through the form, then the real "
                 "stale refusal; no Git effect, no provider, no second consumption",
    }, indent=2) + "\n")
    print("PAEA_APPROVAL_FORM_BROWSER_VERIFIED 4", flush=True)

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
