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
JSON — and archives the integral raw channels as ORIGINAL BYTES (a child
emitting non-UTF-8 diagnostics must not lose stdout, stderr or the exit to
a decode error), argv and real exit of that cargo call in a fresh
fixture-area evidence directory on every attempt,
before any filtering or failure (a failed resolution keeps its complete
diagnosis even though ``run`` has not created its output directory yet). The
digest captured is the digest of the exact executed file, taken before and
after the run. All four recorder threads (both channels of both children)
stream raw output to their own files as it arrives; every recorder is
retained, awaited to its real child-exit/EOF and joined BEFORE any verdict
or result marker — never a silent partial flush,
never a false PASS on a thread exception. A recorder worker error (a
sink open or write failure included) is captured WITH its channel name
and observed by the
controller for as long as any owned child is awaited: inside the poll
loops and throughout every blocking wait, including the late window
after the last one-shot check — a failure may land DURING a wait
(received and reproduced 2026-11-04), so
the run refuses WITHOUT waiting for the
failed channel's producer, because that producer may be blocked forever
writing into the pipe whose only reader just died (received and reproduced
2026-11-04: with a 65536-byte pipe measured before the child, the failed
recorder left the pipe full and unread and the producer's second write
could never complete). On observation the containment cancels the two
OWNED children if still alive (never any external PID), drains every
remaining recorder and records the real observed exits — a cancelled
child's negative signal exit is recorded as-is, never an invented exit —
while the refusal names the cause and marks the failed recorder's channel
incomplete by capture (the healthy channels keep their complete captured
bytes; "all channels drained" is never claimed for a failed capture). The
real exit of each child is recorded on every path (``exits.json``;
explicit ``None`` for a child that never spawned, never an invented exit).
The joins
are intentionally unbounded: both children have already been waited to
their real exits, so EOF is the only remaining end condition and a pipe
held open past its child's death is the defect to surface, diagnosable
without any invented time bound. A Rust test that dies while the browser
harness runs fails the run immediately with its captured output.
"""
import argparse,json,os,subprocess,sys,threading,time,traceback,tempfile
from datetime import datetime, timezone
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from fixture_area import fixture_area

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
    never consulted. Side effects: every invocation writes the full raw
    evidence of the resolution into a fresh
    ``.swarm-fixtures/approval-resolution-evidence-<random>/`` directory
    (``fixture_area``, never ``/tmp``) — the cargo ``argv``, the integral
    unfiltered ``stdout`` and ``stderr`` as ORIGINAL BYTES (the child may
    emit non-UTF-8 diagnostics: the raw files keep the exact bytes the
    child wrote, and the human-readable diagnosis decodes with
    ``errors="replace"`` afterwards) and the real child exit — written
    BEFORE any filtering or failure raise, so even a failed resolution keeps
    its complete diagnosis. The driver's output directory is not used for
    this: ``run`` calls the resolution before ``output.mkdir``, and the
    retention promise must hold at exactly that boundary. Limits: cargo is
    invoked with the recipe's existing profile/args unchanged (no
    compile-forcing flag, no retry, no timeout); an OSError from the evidence
    write is never masked by the subsequent failure raise.
    """
    prefix = spec.name
    deps = spec.parent                       # .../debug/deps
    target_dir = deps.parent.parent          # shared CARGO_TARGET_DIR
    workspace = target_dir.parent.parent     # repository root
    argv = ["cargo", "test", "--no-run", "--message-format=json",
            "-p", "swarm-web", "--test", "paea_git_call"]
    env = dict(os.environ, CARGO_TARGET_DIR=str(target_dir))
    # Fresh dedicated evidence directory per attempt; existing archives are
    # never overwritten (mkdtemp allocation, sibling of the
    # superseded-evidence convention below).
    evidence = Path(tempfile.mkdtemp(prefix="approval-resolution-evidence-",
                                     dir=str(fixture_area())))
    proc = subprocess.run(argv, cwd=workspace / "rust", env=env,
                          capture_output=True)   # bytes, never text=True
    # Raw evidence first, before any filtering or failure: integral channels
    # (original bytes — a non-UTF-8 diagnostic byte must never lose stdout,
    # stderr or the exit to a UnicodeDecodeError) and the real exit code, so
    # a failed resolution is fully diagnosable.
    (evidence / "argv.json").write_text(json.dumps({"argv": argv,
                                                    "cwd": str(workspace / "rust"),
                                                    "exit": proc.returncode},
                                                   indent=2) + "\n")
    (evidence / "stdout.txt").write_bytes(proc.stdout)
    (evidence / "stderr.txt").write_bytes(proc.stderr)
    if proc.returncode != 0:
        raise SystemExit(f"cargo artifact resolution failed (exit {proc.returncode}, "
                         f"raw evidence: {evidence}):\n"
                         + proc.stderr.decode("utf-8", errors="replace")[-2000:])
    matches = []
    for line in proc.stdout.splitlines():
        try:
            message = json.loads(line)
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
        if message.get("reason") != "compiler-artifact":
            continue
        executable = message.get("executable")
        if executable and Path(executable).name.startswith(prefix):
            matches.append(Path(executable))
    if not matches:
        raise SystemExit(f"no cargo compiler-artifact executable for {prefix} "
                         f"(raw evidence: {evidence})")
    candidates = [p for p in matches if p.is_file() and os.access(p, os.X_OK)]
    if len(candidates) != 1:
        raise SystemExit(f"expected exactly one executable artifact for {prefix}, "
                         f"got {sorted(str(p) for p in matches)} "
                         f"(raw evidence: {evidence})")
    return candidates[0]

def stream(pipe, sink: Path, error_box: list) -> None:
    """Stream one child pipe to its raw file as bytes arrive.

    Inputs: a live subprocess pipe, the destination path opened once in
    exclusive mode, and the shared ``error_box`` list. Output: every byte
    the child emits lands in the file promptly (no PIPE buffer stall, no
    late communicate capture). The thread ends when the pipe closes (EOF
    after its child exits) or on the first sink error. Errors: an open or
    write failure on the sink is stored in ``error_box`` as a
    ``(channel, exception)`` pair — never swallowed by the daemon thread.
    The thread then RETURNS: its read side of the pipe disappears, so a
    producer writing more than the pipe capacity into it blocks forever —
    which is exactly why the controller must observe ``error_box`` in its
    own poll loops and cancel the owned children instead of waiting for
    that producer (the containment contract documented on ``run``).
    """
    try:
        with sink.open('xb') as file:
            for chunk in iter(lambda: pipe.read(4096), b''):
                file.write(chunk)
                file.flush()
    except BaseException as exc:  # noqa: BLE001 - forwarded to the controller
        error_box.append((sink.name, exc))

def spawn_rust_test(binary: Path, env: dict):
    """Spawn the owned Rust test child and return its live Popen.

    Inputs: the resolved test binary path and the prepared environment.
    Output: a ``subprocess.Popen`` for the real ``--ignored`` test with both
    pipes captured. Errors: propagation of Popen's own OSError (recorded as
    exit ``None`` in exits.json by ``run``). This function is the exec
    boundary the stream-drain guard substitutes with a tiny local fixture
    child; production argv is unchanged.
    """
    return subprocess.Popen(
        [str(binary),
         "--exact", "approval_form_serves_real_browser_session",
         "--ignored", "--nocapture", "--test-threads=1"],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)

def spawn_browser_harness(binary: Path, output: Path, control: Path, env: dict):
    """Spawn the owned browser-harness child and return its live Popen.

    Inputs: the resolved test binary path (forwarded to the harness), the
    output directory (screenshot/proof target), the control directory
    (server.json protocol) and the prepared environment. Output: a
    ``subprocess.Popen`` for the real node harness with both pipes captured.
    Errors: propagation of Popen's own OSError. This function is the exec
    boundary the stream-drain guard substitutes with a tiny local fixture
    child; production argv is unchanged.
    """
    return subprocess.Popen(
        ["node", "tests/paea-approval-form-ui.cjs", str(binary), str(output), str(control)],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)

def make_recorder(target, name: str, *args):
    """Create one retained recorder thread (factory seam for the guard).

    Inputs: the worker callable, the nominative thread name, and the worker
    arguments. Output: a started-ready daemon ``threading.Thread`` named
    ``name``; the caller starts it and retains it until a completed join.
    The stream-drain guard overrides this factory to instrument recorder
    scheduling causally (event handshakes, never timers).
    """
    return threading.Thread(target=target, args=args, name=name, daemon=True)

def record_exits(path: Path, rust, harness, error_box: list) -> None:
    """Write exits.json: the real exit of each child, on every path.

    Inputs: the output path, the two child Popen objects (either may be
    None if the run failed before spawning it) and the recorder error box
    (``(channel, exception)`` pairs from ``stream``). Output: a small JSON
    file with ``rust_exit``/``harness_exit`` as real integers once the
    children were waited, explicit ``None`` when a child never spawned, a
    real negative signal exit as-is for a child cancelled by the
    containment (never an invented exit), plus the recorder worker errors
    as ``channel: repr`` strings if any. Errors: a write failure
    propagates — exit recording is not silently dropped. Called from
    ``run``'s finally before any verdict, so the exits exist even on
    fail-fast, spawn-failure, containment or recorder-error paths.
    """
    payload = {
        "rust_exit": rust.returncode if rust is not None else None,
        "harness_exit": harness.returncode if harness is not None else None,
    }
    if error_box:
        payload["recorder_errors"] = [f"{name}: {exc!r}"
                                      for name, exc in error_box]
    path.write_text(json.dumps(payload, indent=2) + "\n")

def run(binary: Path, output: Path) -> None:
    """Run the owned two-phase browser qualification and archive all evidence.

    Inputs: the cargo-artifact binary spec path (deps/paea_git_call) and a
    fresh output directory. Outputs: streaming harness/rust stdout/stderr,
    approval-form-ui.json, fresh/stale screenshots, result.json with the
    exact executed binary digest (before/after), and exits.json recording
    the real exit of each child on every path (explicit None for a child
    that never spawned, plus any recorder worker error — never an invented
    exit). Errors: any nonzero exit, missing PASS result or unexpected
    diagnostics is a hard failure; a Rust test that exits while the harness
    still runs fails immediately with its captured output (the harness is
    then drained to its real exit WITH ``error_box`` observed — never a
    bare blocking ``harness.wait()``); a recorder worker failure (sink
    open/write) is OBSERVED for as long as any owned child is awaited —
    inside the poll loops AND throughout every blocking wait including the
    late window after the last one-shot check (a failure may land during
    the wait itself: received and reproduced 2026-11-04, causal probe
    intercepted the blocking wait with the full unread pipe, the producer
    alive and the error already captured) — so the run refuses without
    waiting for that channel's producer, which may
    be blocked forever writing into the pipe whose only reader just died
    (received and reproduced 2026-11-04 with a measured 65536-byte pipe);
    the containment then cancels the two OWNED children if still alive
    (never an external PID, no watchdog, no retry), drains every remaining
    recorder, records the real observed exits and raises naming the cause
    and the incomplete channel — never a false PASS on a thread exception;
    control markers and server.json stay in the owned control directory
    under output for replay/diagnosis. The recorder joins are unbounded:
    both children are waited to their real exits first, so EOF is the only
    remaining end condition and no invented time bound replaces the earlier
    comment-declared join(timeout=30) diagnosis aid. Limits: exits.json
    exists only once the output directory exists; a failure before that
    (cargo resolution) keeps its complete diagnosis in ``resolve_binary``'s
    own evidence directory. The child-spawn boundary
    (``spawn_rust_test``/``spawn_browser_harness``), the cargo resolution,
    the recorder factory and the verdict marker are module-level seams the
    stream-drain guard substitutes with local fixture children — production
    behavior is unchanged.
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
    # exits. ALL FOUR recorders (both channels of both children) are retained
    # and joined before any verdict; a recorder worker error is observed
    # inside the wait loops and contained (never a false PASS on a thread
    # exception, never an indefinite wait on a producer whose reader died).

    def recorder_failure() -> str:
        """Return the refusal text for a recorder failure, or "" while none.

        Inputs: the shared ``error_box`` of ``(channel, exception)`` pairs.
        Output: "" before any failure; otherwise the complete refusal text:
        each failure with its channel and exception, the exits.json
        pointer and the explicit INCOMPLETE marking for the failed
        channel(s) — never a claim that all channels were drained. Errors:
        none of its own. Used by every wait loop (and by the post-join
        backstop below) so a recorder that just died is observed WITHOUT
        waiting for its producer, which may be blocked forever writing
        into the pipe whose only reader disappeared (received and
        reproduced 2026-11-04 with a measured 65536-byte pipe).
        """
        if not error_box:
            return ""
        causes = "; ".join(f"output recorder worker failed: {name}: {exc!r}"
                           for name, exc in error_box)
        names = ", ".join(name for name, _ in error_box)
        return (f"{causes} (real observed exits recorded in exits.json; "
                f"channel(s) {names} INCOMPLETE by capture — only the "
                f"healthy channels keep their complete bytes; never read "
                f"this output as a full record)")

    rust_out = output / "rust.stdout"
    rust_err = output / "rust.stderr"
    rust = None
    harness = None
    error_box = []   # (channel, exception) pairs, observed by the wait loops
    recorders = []   # ALL FOUR recorders, retained and joined before any verdict
    try:
        rust = spawn_rust_test(binary, env)
        for pipe, sink in ((rust.stdout, rust_out), (rust.stderr, rust_err)):
            thread = make_recorder(stream, f"stream:{sink.name}", pipe, sink, error_box)
            recorders.append(thread)
            thread.start()
        # The only wait here is for the owned startup publication: the marker
        # protocol owns every later phase boundary. A rust exit before
        # publication means the bind refused: fail immediately with its output.
        while not (control / "server.json").exists():
            failure = recorder_failure()
            if failure:
                raise SystemExit(failure)
            if rust.poll() is not None:
                raise SystemExit(
                    f"rust test exited {rust.returncode} before publishing server.json")
            time.sleep(0.2)
        harness = spawn_browser_harness(binary, output, control, env)
        for pipe, sink in ((harness.stdout, output / "harness.stdout"),
                           (harness.stderr, output / "harness.stderr")):
            thread = make_recorder(stream, f"stream:{sink.name}", pipe, sink, error_box)
            recorders.append(thread)
            thread.start()
        # Fail fast if the rust test FAILS while the harness still runs: its
        # captured raw output is the diagnosis. A clean exit 0 while the
        # harness still runs is the expected success ordering, not a death:
        # the owned shutdown completes as soon as phase2.done is consumed,
        # while the harness is still closing the real browser (observed
        # 2026-10-31: exit 0 raced browser.close() and must never be turned
        # into a failure).
        while harness.poll() is None:
            failure = recorder_failure()
            if failure:
                raise SystemExit(failure)
            if rust.poll() is not None and rust.returncode != 0:
                # The rust test failed while the harness still runs: drain
                # the harness WITHOUT an unobserved blocking wait. A recorder
                # may fail during this drain (the same late window: received
                # and reproduced 2026-11-04), and its producer may then be
                # blocked forever writing into a pipe whose only reader just
                # died — observe the recorder failure until the harness's
                # real exit instead of a bare ``harness.wait()``.
                while harness.poll() is None:
                    failure = recorder_failure()
                    if failure:
                        raise SystemExit(failure)
                    time.sleep(0.2)
                failure = recorder_failure()
                if failure:
                    raise SystemExit(failure)
                raise SystemExit(
                    f"rust test exited {rust.returncode} while the harness ran; "
                    f"see rust.stdout/rust.stderr")
            time.sleep(0.2)
        harness.wait()
        # Same observation before the last wait: the harness exited but the
        # rust producer may still be blocked writing into a pipe whose only
        # reader just died — observe the recorder failure instead of waiting
        # for that producer (containment contract below in ``finally``).
        failure = recorder_failure()
        if failure:
            raise SystemExit(failure)
        # rust.wait() may still block on a producer whose reader died (the
        # child can be alive here: writing into the full unread pipe). A
        # recorder failing DURING this wait is the received late window
        # (2026-11-04): the failure arrives after the one-shot check above,
        # so observe error_box UNTIL the child's real exit instead of a bare
        # blocking wait — the loop ends on the genuine exit and the single
        # final check preserves any error captured with it.
        while rust.poll() is None:
            failure = recorder_failure()
            if failure:
                raise SystemExit(failure)
            time.sleep(0.2)
        failure = recorder_failure()
        if failure:
            raise SystemExit(failure)
        # Drain ALL FOUR recorders fully before any verdict: the stream threads
        # finish only when the pipes close, so a completed join proves every
        # captured byte is on disk. The joins are deliberately unbounded — the
        # previous join(timeout=30) was a comment-declared diagnosis aid, not a
        # measured contract — because both children have already been waited to
        # their real exits here (with error_box observed throughout those
        # waits, including the late window during them): EOF is the only
        # remaining end condition, and a recorder still alive past that point
        # means its pipe never closed, which is itself the defect to surface.
        # A failure surfacing during these final joins is refused by the
        # backstop after ``finally``. If no progression remains, the
        # finally block below still preserves the real exits, the raw channels
        # and any recorder error as the diagnosis.
        for thread in recorders:
            thread.join()
    finally:
        for process in (harness, rust):
            if process is not None and process.poll() is None:
                process.kill()
        for process in (harness, rust):
            if process is not None:
                process.wait()
        # Every path (success, fail-fast, spawn failure) drains the retained
        # recorders before leaving, so no channel is left partially captured.
        for thread in recorders:
            thread.join()
        record_exits(output / "exits.json", rust, harness, error_box)
    if error_box:
        # Backstop for a failure observed after the last loop check (e.g.
        # during the final joins): same refusal contract as the loops.
        raise SystemExit(recorder_failure())
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
        "harness_exit": harness.returncode,
        "binary_path": str(binary),
        "binary_sha256": digest_before,
        "scope": "real frozen approval command served to a real browser; one explicit "
                 "labeled-fixture empty-body consumption through the form, then the real "
                 "stale refusal; no Git effect, no provider, no second consumption",
    }, indent=2) + "\n")
    print_verdict_marker(4)

def print_verdict_marker(checks: int) -> None:
    """Print the nominative recipe verdict marker (module-level seam).

    Inputs: the verified check count. Output: the single stdout line
    ``PAEA_APPROVAL_FORM_BROWSER_VERIFIED <checks>`` flushed immediately —
    the marker the form recipe's manifest pins as required_stdout. Called
    only AFTER result.json was written and every recorder was drained, so
    observing this marker proves the whole run completed. Errors: none of
    its own; a broken stdout raises out of ``run`` like any publish step.
    The stream-drain guard substitutes this seam to observe "verdict
    returned" causally (event handshake, never a timer).
    """
    print(f"PAEA_APPROVAL_FORM_BROWSER_VERIFIED {checks}", flush=True)

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
