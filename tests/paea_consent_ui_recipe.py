#!/usr/bin/env python3
"""Permanent-recipe driver for the PA-08 consent-page browser qualification.

Builds nothing and mocks nothing: the recipe declares the cargo build/test
steps and passes the freshly built test binary spec here. The driver runs the
real --ignored Rust test ``consent_page_serves_real_bound_approval_to_real_browser``
(real bound approval via prepare_approval + bind_git_call_with_approval, real
native server, real persisted session) and the real served consent page in
real Chrome through the owned two-phase marker protocol. The page itself
performs both refusal submissions (drifted bytes, wrong password); no approval
effect, grant or consumption is produced, no provider and no live mission
database. A failing run preserves both channels and the fixture roots for
diagnosis.

Proof method (qualified linked-output protocol, same as the approval-form
driver): the executed binary is resolved from the cargo JSON artifact stream
(``cargo test --no-run --message-format=json``), never from a glob, a
modification time or a purge; the recipe's own build step guarantees the
fresh build and the bind step retains the exact artifact identity. The digest
captured is the digest of the exact executed file, taken before and after the
run. Both child processes stream their raw output to their own files as it
arrives, both are awaited to their real exits, the streamer threads are
joined with their liveness verified (a live streamer after both children
exited is a hard failure, never a silent partial flush), a Rust test that
dies while the browser harness runs fails the run immediately with its
captured output, and the INVERSE fault is closed symmetrically: a harness
failure while Rust still waits on the phase marker kills live Rust BEFORE
any wait (the harness cause is decided first so the kill never masks it),
so no wait() ever targets a live child on a failure path in either
direction. server.json is the startup handshake only — never a
test-end condition; the marker protocol owns every phase boundary. No global
watchdog exists in this driver: the only waits are the owned publication and
the children's real exits.
"""
import argparse,json,os,subprocess,sys,threading,time,traceback
from datetime import datetime, timezone
from pathlib import Path

def resolve_binary(spec: Path) -> Path:
    """Resolve the EXACT cargo-built test executable from the artifact stream.

    Inputs: ``spec`` is a path whose filename is the test target's crate-name
    prefix (``paea_consent_browser``) under the shared ``CARGO_TARGET_DIR``
    deps directory. The driver asks cargo once more — cheap when fresh, the
    recipe's build step already forced the compile — and selects the
    ``compiler-artifact`` message whose ``executable`` basename starts with
    the spec filename, reading the exact hash-suffixed path cargo itself
    emitted. ``--offline --locked`` keeps the resolution deterministic and
    purely local (same flags as the bind tool's resolution). Errors: cargo
    failure, no matching artifact or a missing/non-executable file is a hard
    failure; modification times and directory globs are never consulted.
    """
    prefix = spec.name
    deps = spec.parent                       # .../debug/deps
    target_dir = deps.parent.parent          # shared CARGO_TARGET_DIR
    workspace = target_dir.parent.parent     # repository root
    proc = subprocess.run(
        ["cargo", "test", "--no-run", "--message-format=json",
         "--manifest-path", str(workspace / "rust" / "Cargo.toml"),
         "--offline", "--locked", "-p", "swarm-web", "--test", "paea_consent_browser"],
        cwd=workspace, env=dict(os.environ, CARGO_TARGET_DIR=str(target_dir)),
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

def stream(pipe, sink: Path, eof: threading.Event) -> None:
    """Stream one child pipe to its raw file as bytes arrive, then signal EOF.

    Inputs: a live subprocess pipe, the destination path opened once in
    exclusive-create mode and a per-pipe EOF event. Output: every byte the
    child emits lands in the file promptly (no PIPE buffer stall, no late
    communicate capture); the event is set ONLY when the pipe has been read
    to EOF and flushed to disk. A missing EOF signal is how the run verdict
    refuses a partial flush — it is never set speculatively and never by a
    timer.
    """
    with sink.open('xb') as file:
        for chunk in iter(lambda: pipe.read(4096), b''):
            file.write(chunk)
            file.flush()
    eof.set()

def run(binary: Path, output: Path) -> None:
    """Run the owned two-phase browser qualification and archive all evidence.

    Inputs: the cargo-artifact binary spec path (deps/paea_consent_browser)
    and a fresh output directory. Outputs: streaming harness/rust
    stdout/stderr, consent-bound-ui.json, fresh/refusal screenshots and
    result.json with the exact executed binary digest (before/after).
    Failure contract (driver-control counterexamples of 2026-11-01): the
    driver NEVER waits on a live child on a failure path — a refused
    prerequisite, a harness error or a Rust failure while the harness runs
    raises immediately, then the finally-cleanup kills only the two OWNED
    Popen children and waits for them (never a foreign PID); every streamer
    of BOTH children is tracked and joined after the children have exited,
    and the verdict requires every streamer's EOF marker — a pipe not read
    to EOF and flushed is a partial flush and refuses the PASS, never a
    silent continuation. Errors: any nonzero exit, missing PASS result,
    unexpected diagnostics, missing raw channel or unfinished streamer is a
    hard failure with the primary cause retained plus both children's real
    exits; control markers and server.json stay in the owned control
    directory under output for replay/diagnosis.
    """
    # Evidence is never overwritten: a leftover directory from an earlier run
    # (usually a prior failure kept for diagnosis) is archived aside with a
    # timestamp and the fresh run starts empty at the declared path. The
    # recipe keeps one stable output path (same convention as the
    # approval-form qualification driver).
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
               CONSENT_BROWSER_DIR=str(control))
    sha256 = lambda p: subprocess.run(["sha256sum", str(p)], capture_output=True,
                                      check=True).stdout.decode().split()[0]
    digest_before = sha256(binary)
    # The Rust test must run first: it binds the real server and publishes
    # server.json, which the browser harness reads at startup. Starting the
    # harness before the test races a guaranteed ENOENT (same observed order
    # constraint as the bound-preview qualification, 2026-10-30). Raw output
    # streams to its file as produced; no PIPE+communicate late capture, no
    # global watchdog — the marker protocol owns every phase boundary and
    # both processes are awaited to their real exits.
    rust_out = output / "rust.stdout"
    rust_err = output / "rust.stderr"
    rust = subprocess.Popen(
        [str(binary),
         "--exact", "consent_page_serves_real_bound_approval_to_real_browser",
         "--ignored", "--nocapture", "--test-threads=1"],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
    # EVERY streamer of BOTH children is tracked: an untracked thread is a
    # pipe nobody joins, i.e. the missing-join false PASS observed in the
    # driver-control counterexample. Each pipe owns its EOF event, set only
    # at read-EOF + flush.
    streamers: list[threading.Thread] = []
    eof_events: dict[str, threading.Event] = {}
    for name, pipe, sink in (("rust.stdout", rust.stdout, rust_out),
                             ("rust.stderr", rust.stderr, rust_err)):
        eof = threading.Event()
        eof_events[name] = eof
        thread = threading.Thread(target=stream, args=(pipe, sink, eof), daemon=True,
                                  name=f"stream-{name}")
        thread.start()
        streamers.append(thread)
    harness = None
    primary: str | None = None
    try:
        # The only wait here is for the owned startup publication: the marker
        # protocol owns every later phase boundary. A rust exit before
        # publication means the bind refused: raise IMMEDIATELY (no wait on
        # any live child on a failure path), its streamed output is the
        # diagnosis. The 0.2 s poll interval only paces a poll of real
        # process/publication state — it bounds nothing.
        while not (control / "server.json").exists():
            if rust.poll() is not None:
                primary = (f"rust test exited {rust.returncode} before "
                           f"publishing server.json")
                raise SystemExit(primary)
            time.sleep(0.2)
        harness = subprocess.Popen(
            ["node", "tests/paea_consent_ui.cjs", str(binary), str(output), str(control)],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
        for name, pipe, sink in (("harness.stdout", harness.stdout, output / "harness.stdout"),
                                 ("harness.stderr", harness.stderr, output / "harness.stderr")):
            eof = threading.Event()
            eof_events[name] = eof
            thread = threading.Thread(target=stream, args=(pipe, sink, eof), daemon=True,
                                      name=f"stream-{name}")
            thread.start()
            streamers.append(thread)
        # Fail fast in BOTH directions while either child still runs (paired
        # failure contract): a Rust failure while the harness runs AND a
        # harness failure while Rust still waits on the marker both raise
        # IMMEDIATELY — the finally-cleanup owns the kill/wait of both
        # children and the channel drain, never a wait on a live child here.
        # A clean Rust exit 0 while the harness still runs is the expected
        # success ordering, not a death: the owned shutdown completes as soon
        # as phase2.done is consumed, while the harness is still closing the
        # real browser (observed 2026-10-31 on the approval-form driver: exit
        # 0 raced browser.close() and must never be turned into a failure).
        while harness.poll() is None:
            if rust.poll() is not None and rust.returncode != 0:
                primary = (f"rust test exited {rust.returncode} while the "
                           f"harness ran; see rust.stdout/rust.stderr")
                raise SystemExit(primary)
            time.sleep(0.2)
        # The poll loop above only proves the HARNESS reached a terminal
        # state — nothing proves Rust did (it may still hold the marker open
        # long after a failing harness has exited). On a KNOWN harness
        # failure, Rust still live at this point is killed BEFORE any wait
        # (it belongs to this run; its outcome cannot be a PASS once its
        # sibling failed), so no wait() ever targets a live child on a
        # failure path. On the success path a live Rust is the expected
        # protocol completion and its real exit is collected by wait, never
        # turned into a failure. No arbitrary timeout replaces the state
        # check. The primary cause is decided BEFORE the kill so a killed
        # sibling (-9) never masks the real cause.
        if harness.returncode != 0:
            primary = f"browser harness failed: exit {harness.returncode}"
            if rust.poll() is None:
                rust.kill()
        harness.wait()
        rust.wait()
        if primary is None and rust.returncode != 0:
            primary = f"rust test failed: exit {rust.returncode}"
    finally:
        # Cleanup owns ONLY the two Popen children this driver created (never
        # a foreign PID): kill what is still live, then wait for both to
        # reach a terminal state so their pipes close and the streamers can
        # reach EOF. The primary failure cause raised from the try survives.
        for process in (harness, rust):
            if process is not None and process.poll() is None:
                process.kill()
        for process in (harness, rust):
            if process is not None:
                process.wait()
        # Drain: both children have now exited, so every pipe MUST close and
        # every streamer MUST reach EOF. A thread still alive after the join
        # means the pipe never closed — a partial flush that would otherwise
        # pass as a complete capture (driver-control counterexample
        # child-output-not-drained). The join polls the streamers' real EOF
        # with no sleep budget of its own: it waits on the threads' exit,
        # which the closed pipes above bound. No EOF marker is the verdict
        # failure below, never a silent continuation.
        for thread in streamers:
            thread.join()
    # Verdict gates, in order: the primary cause (any refused prerequisite,
    # rust failure or harness failure collected above), then a FULL-flush
    # check of every archived channel (raw file present AND its streamer hit
    # EOF) — a run with a missing or partially drained output is refused,
    # never a PASS with absent rust/harness stdout.
    if primary is not None:
        missing = [name for name in eof_events if not eof_events[name].is_set()]
        detail = (f"; partial flush: EOF never reached on {sorted(missing)}"
                  if missing else "; both channels drained")
        raise SystemExit(f"{primary}{detail}")
    undrained = sorted(name for name, eof in eof_events.items() if not eof.is_set())
    if undrained:
        raise SystemExit(
            "child output not fully drained before verdict: EOF never reached "
            f"on {undrained} — partial flush, refusing PASS (raw files kept "
            "for diagnosis)")
    for channel in ("rust.stdout", "rust.stderr", "harness.stdout", "harness.stderr"):
        if not (output / channel).exists():
            raise SystemExit(
                f"raw channel {channel} was never written: no PASS without "
                "the complete captured output")
    proof = json.loads((output / "consent-bound-ui.json").read_bytes())
    assert len(proof["checks"]) == 2, proof["checks"]
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
