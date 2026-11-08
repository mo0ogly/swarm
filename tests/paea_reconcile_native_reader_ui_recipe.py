#!/usr/bin/env python3
"""Run the REAL native browser-witness test once on the provided managed
binary under ONE fresh owned directory, then verify its bounded artifacts.

--report-scope empty|nonempty (default empty) selects the EXACT ignored
native test, its expected witness kind and its named-check inventory, all
through ONE shared runner/qualifier pipeline (no copied code path):
- empty: reconcile_effects_native_binary_positive_reader_frontier_empty_ledger
  (rust/crates/swarm-web/tests/paea_reconcile.rs, #[ignore], opt-in browser
  witness tests/paea_reconcile_native_reader_ui.cjs) with witness kind
  paea-native-reader-ui-v1 and marker PAEA_NATIVE_READER_UI_VERIFIED 14 —
  the historically qualified contract, unchanged;
- nonempty:
  reconcile_effects_native_binary_positive_reader_frontier_nonempty_ledger
  with witness kind paea-native-reader-ui-nonempty-v1 and the DISTINCT
  marker PAEA_NATIVE_READER_UI_NONEMPTY_VERIFIED 14; its scope-specific
  named checks (real settlement finding rendered, same-page purge INCLUDING
  the previously rendered real identifiers, identical recovery re-render)
  are pinned verbatim from the conserved nonempty witness. An unknown scope
  value is refused by argparse (exit 2, stderr only) BEFORE any filesystem
  allocation, cargo invocation or child process.

Both scopes follow the existing tests/paea_effects_ui_recipe.py contract:
--binary is EXPLICIT (no build, no touch, the file's bytes are only hashed);
--output/--output-parent are mutually exclusive, the parent must already
exist; /tmp or below is refused for BOTH before any filesystem allocation; a
refused precondition exits nonzero with stderr only and leaves preexisting
bytes untouched; all writes of this invocation land only in the directory it
created itself. The cargo test command is the exact qualified one
(qualified invocation.json, lot glm-pa09-browser-final-privacy-20261103T040513Z):
cargo test --manifest-path rust/Cargo.toml --offline --locked -p swarm-web
--test paea_reconcile <scope test name> -- --ignored --exact --nocapture,
with SWARM_ENGINE_BINARY=--binary,
SWARM_PAEA_NATIVE_BROWSER_WITNESS=1,
CARGO_TARGET_DIR=<repo>/.cache/paea-verification-target,
CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0
and SWARM_PAEA_NATIVE_EVIDENCE_DIR=<wrapper>/native-browser-witness — a
path the wrapper DOES NOT create: the Rust from_env claims it with an
EXCLUSIVE create (it must not exist, paea_reconcile.rs) while the wrapper
keeps only its own private dir. timeout=None everywhere (no global
watchdog); the process environment is inherited plus the explicit overrides
below (TMPDIR pinning is the recipe runner's net when invoked through
tools/verification/recipe.py; never /tmp). stdout, stderr and the actual
returncode are conserved COMPLETELY (native.stdout/native.stderr + on
failure failure.txt) BEFORE any qualification, even on a nonzero exit — a
refusal never discards the original evidence. Exit 0 is not accepted as
success by itself — 0 tests, an ignored-only run, or a FAILED summary are
refused. Qualification parses only BOUNDED artifacts under the owned run
dir (never an arbitrary path): browser-witness.json (14 REAL named checks —
the exact strings of the requested scope, not a counter alone; kind and
report_scope bound to the requested scope; 0 unexpected diagnostics; exactly
4 observed 403 refusals all on the reconcile GET; exactly 7 reconcile GETs;
the 4 credentialed requests carry exactly the expected authorization
classifications; ledger GET-only with no Authorization off the reconcile
path), the node summary line browser-witness.stdout.log (checks 14,
unexpected 0, refusals 4, scope bound to the requested scope — coherent
with the witness body),
browser-witness.exit.json (the BROWSER child: observed, code 0, success, no
stdin IO error, exact binary path+sha256, script name, and run_dir naming
the OUTER Rust browser-witness-<time>-<suffix> dir), the top-level native
exit.json
(the RUST child: observed, exit_code 0, success) and the top-level native
stdout.log (full web-bound -> last-line web-stopped, empty drain sidecars,
browser-witness.json inside the nested JS dir) — all conserved by the Rust
RawCapture/RawOutputWitness at the evidence-dir top level, each artifact
verified separately per its true producer — and the cargo stdout summary in
the REAL format `test result: ok.
1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; ...` (exactly one
executed, 0 failed, 0 ignored, the scope's named test line ok; the 9 other
tests being filtered is normal and NOT required to be 0). On success the
scope's DISTINCT marker (empty: PAEA_NATIVE_READER_UI_VERIFIED 14, nonempty:
PAEA_NATIVE_READER_UI_NONEMPTY_VERIFIED 14) and the fresh wrapper path are
printed AFTER verify() passes; on failure the safe cause and the conserved
original evidence remain (never a PASS). These counts are this wrapper's
existing assertion inventory for one named run, never a new runtime policy.
No secret is printed: the witness ledger holds classification booleans
only, and this wrapper reports counts, names and digests, never values.
"""
import argparse, hashlib, json, os, re, subprocess, sys, tempfile, traceback
from pathlib import Path

REPOSITORY = Path(__file__).resolve().parents[1]

MARKER = "PAEA_NATIVE_READER_UI_VERIFIED 14"
TEST_NAME = "reconcile_effects_native_binary_positive_reader_frontier_empty_ledger"
EMPTY_KIND = "paea-native-reader-ui-v1"
NONEMPTY_TEST_NAME = "reconcile_effects_native_binary_positive_reader_frontier_nonempty_ledger"
NONEMPTY_KIND = "paea-native-reader-ui-nonempty-v1"
NONEMPTY_MARKER = "PAEA_NATIVE_READER_UI_NONEMPTY_VERIFIED 14"
WRAPPED_SCRIPT = "tests/paea_reconcile_native_reader_ui.cjs"
EXIT_ARTIFACT = "browser-witness.exit.json"

# The 14 REAL named checks of the actual qualified run (receipt
# glm-pa09-browser-final-privacy-20261103T040513Z, browser-witness.json):
# every string must be present verbatim; with len==14 no other check exists.
REQUIRED_CHECKS = (
    "fr: bare reader refusal 403 paea-reader-authentication-required under a real session",
    "fr: bare refusal purges all report DOM",
    "fr: reader credential field (password, autocomplete=off) and re-read button really served",
    "fr: fixture-token re-read 200 — both journals rendered uninitialized (exact product sentence, no finding), raw JSON exactly the attributed report",
    "fr: secret field purged; reader token and session cookie absent from DOM and web storage",
    "fr: Escape clears the ephemeral credential field",
    "unknown fixture token: typed 403 authorization_required really served ON THE SAME PAGE that displayed the report (purge proven where it matters)",
    "unknown-credential refusal: report purged on the SAME page, field erased, re-read command available",
    "fr: recovery re-read 200 with the exact attributed report on the SAME page (channel intact after refusals)",
    "en: reader credential field (password, autocomplete=off) and re-read button really served",
    "en: theme toggle really switches sombre->etat dataset",
    "fr: keyboard skip link first, theme control >=44px",
    "Authorization scoped to the reconcile GET only; 4 Bearer reads (3 fixture + 1 armed unknown), 4 reconcile 403s (3 bare + 1 armed), 7 reconcile GETs, GET-only; ledger holds classification booleans only",
    "no unexpected browser diagnostic across all phases",
)

# The 11 shared check names above appear in BOTH scopes; the empty scope
# uses exactly these 14. The nonempty witness additionally carries the 3
# scope-specific names below (14 = 11 shared + these 3; with len==14 and
# every string present verbatim no other check exists). Pinned verbatim
# from the conserved real nonempty witness (kind
# paea-native-reader-ui-nonempty-v1, Codex reception on pinned ELF320
# fbf41a82…, nonempty-capture browser-witness.json):
NONEMPTY_EXTRA_CHECKS = (
    "fr: fixture-token re-read 200 — settlement rendered consistent (94 chars) with the real finding line (intent+effect+verdict), git uninitialized, raw JSON exactly the attributed report",
    "unknown-credential refusal: report purged on the SAME page INCLUDING the previously rendered real identifiers, field erased, re-read command available",
    "fr: recovery re-read 200 with the exact attributed report on the SAME page — the real finding line is re-rendered identically (channel intact after refusals)",
)

# Per-scope contract: the exact ignored native test to run, the witness
# kind it must produce, and the verbatim named-check inventory verify()
# requires. One selection table, ONE shared runner/qualifier pipeline. The
# nonempty inventory replaces EXACTLY the 3 scope-specific positions of the
# empty inventory (fixture-token re-read, unknown-credential refusal,
# recovery re-read) and keeps the 11 shared names in witness order:
SCOPE_CONTRACT = {
    "empty": {"test": TEST_NAME, "kind": EMPTY_KIND, "marker": MARKER,
              "checks": REQUIRED_CHECKS},
    "nonempty": {"test": NONEMPTY_TEST_NAME, "kind": NONEMPTY_KIND,
                 "marker": NONEMPTY_MARKER,
                 "checks": (REQUIRED_CHECKS[:3] + (NONEMPTY_EXTRA_CHECKS[0],)
                            + REQUIRED_CHECKS[4:7]
                            + (NONEMPTY_EXTRA_CHECKS[1], NONEMPTY_EXTRA_CHECKS[2])
                            + REQUIRED_CHECKS[9:])},
}


def refuse_tmp(path: Path, option: str) -> None:
    """Refuse /tmp itself or anything below it as an output location BEFORE
    any filesystem allocation or write; `path` must already be resolved
    (absolute, symlinks resolved) — no output, evidence child or tempfile
    target may fall under /tmp. Raises ValueError naming `option` and the
    offending path; writes nothing — the caller turns this into its
    stderr-only nonzero refusal.
    """
    if path == Path("/tmp") or Path("/tmp") in path.parents:
        raise ValueError(option + " must not be /tmp or under it: " + str(path))


def verify(witness: dict, exit_meta: dict, node_summary: dict,
           native_exit_meta: dict, native_run_stdout: bytes,
           native_stdout: bytes, native_exit: int, binary: Path,
           digest: str, test_name: str, expected_kind: str,
           requested_scope: str, required_checks: tuple) -> None:
    """Qualify the conserved REAL evidence of ONE scope — named checks, not
    a counter alone. `test_name`, `expected_kind`, `requested_scope` and
    `required_checks` come from SCOPE_CONTRACT[args.report_scope], so each
    scope qualifies through its OWN exact contract. Asserts the witness
    shape (kind bound to the requested scope — paea-native-reader-ui-v1 or
    paea-native-reader-ui-nonempty-v1 — schema_version 1, report_scope
    equal to the requested scope, EXACTLY 14 checks and every
    required-check string present verbatim), the node summary line
    (browser-witness.stdout.log: checks 14, unexpected 0, refusals 4, scope
    equal to the requested scope — coherent with the witness body),
    empty unexpected_diagnostics, exactly 4 observed refusals all GET 403 on
    the reconcile path, a request ledger that is GET-only with every
    Authorization on the reconcile path, exactly 7 reconcile GETs, the 4
    credentialed requests classified exactly as 3 readerKnown-only + 1 armed
    unknownKnown-only all Bearer with strictly bool-typed fields, the BROWSER
    child metadata (browser-witness.exit.json) carrying the exact managed
    binary path and sha256 with observed code-0 success and no stdin IO
    error, and run_dir naming the OUTER Rust browser-witness dir), the RUST
    child exit.json separately (observed, exit_code 0, success), the native
    stdout.log holding the full web-bound start and the last-line
    web-stopped stop-record, and the cargo stdout holding exactly
    one REAL-format summary `test result: ok. 1 passed; 0 failed; 0 ignored;
    ... filtered out; ...` plus the named `test <fullname> ... ok` line for
    the requested scope's test (0 tests, ignored-only or FAILED summaries
    are refused even with exit 0; the other 9 tests being filtered is
    normal). Raises AssertionError with a safe cause (counts/names/digests,
    never secret values) on any mismatch.
    """
    assert (node_summary.get("checks"), node_summary.get("unexpected"),
            node_summary.get("refusals")) == (14, 0, 4), \
        "node summary line must restate checks=14 unexpected=0 refusals=4, got %r" % (
            {k: node_summary.get(k) for k in ("checks", "unexpected", "refusals")},)
    assert node_summary.get("scope") == requested_scope, \
        "node summary scope must be %r, got %r" % (requested_scope, node_summary.get("scope"))
    assert witness.get("report_scope") == requested_scope, \
        "witness report_scope must be %r, got %r" % (requested_scope, witness.get("report_scope"))
    checks = witness.get("checks") or []
    assert (witness.get("kind"), witness.get("schema_version"), len(checks)) == \
        (expected_kind, 1, 14), \
        "unexpected witness shape: kind=%r schema=%r checks=%d" % (
            witness.get("kind"), witness.get("schema_version"), len(checks))
    named = set(checks)
    missing = [name for name in required_checks if name not in named]
    assert not missing, "missing named checks (%d): %r" % (len(missing), missing)
    assert len(named) == 14, "expected exactly 14 distinct named checks, got %d" % len(named)
    assert witness.get("unexpected_diagnostics") == [], \
        "unexpected diagnostics present: %d" % len(witness.get("unexpected_diagnostics") or [])
    refusals = witness.get("expected_refusals") or []
    assert len(refusals) == 4 and all(
        r.get("method") == "GET" and r.get("status") == 403
        and r.get("path") == "/api/v1/paea/reconcile-effects" for r in refusals), \
        "expected exactly 4 reconcile GET 403 refusals, got %d" % len(refusals)
    ledger = witness.get("request_ledger") or []
    assert ledger and all(r.get("method") == "GET" for r in ledger), \
        "every ledger request must be GET, got %d request(s)" % len(ledger)
    assert all(r.get("path") == "/api/v1/paea/reconcile-effects"
               for r in ledger if r.get("authorization")), \
        "no Authorization may leave the reconcile path"
    reconcile = [r for r in ledger if r.get("method") == "GET"
                 and r.get("path") == "/api/v1/paea/reconcile-effects"]
    assert len(reconcile) == 7, "expected 7 reconcile GETs, got %d" % len(reconcile)
    credentialed = [r for r in reconcile if r.get("authorization")]
    assert len(credentialed) == 4, "expected 4 credentialed reconcile GETs, got %d" % len(credentialed)
    assert all(
        isinstance(r.get("authorization"), dict)
        and type(r["authorization"].get("present")) is bool
        and r["authorization"].get("present") is True
        and type(r["authorization"].get("bearer")) is bool
        and r["authorization"].get("bearer") is True
        and type(r["authorization"].get("readerKnown")) is bool
        and type(r["authorization"].get("unknownKnown")) is bool
        and (int(r["authorization"]["readerKnown"]) + int(r["authorization"]["unknownKnown"])) == 1
        for r in credentialed), \
        "credentialed requests must carry exactly one strictly-bool classification each"
    reader_known = sum(1 for r in credentialed if r["authorization"].get("readerKnown") is True)
    assert reader_known == 3, "expected 3 readerKnown-only re-reads, got %d" % reader_known
    armed = [r for r in credentialed if r.get("armed")]
    assert len(armed) == 1 and armed[0]["authorization"].get("unknownKnown") is True, \
        "exactly the armed unknown re-read must be unknownKnown"
    # BROWSER child metadata (browser-witness.exit.json) — NOT the native exit.
    assert exit_meta.get("binary") == str(binary), "exit metadata binary path differs from the provided --binary"
    assert exit_meta.get("binary_sha256") == digest, "exit metadata binary digest differs from the wrapper digest"
    assert exit_meta.get("observed") is True and exit_meta.get("exit", {}).get("success") is True, \
        "browser child must be observed with a successful exit"
    assert exit_meta.get("exit", {}).get("code") == 0 and exit_meta.get("stdin_io_error") is None, \
        "browser child exit must be code 0 with no stdin IO error"
    assert exit_meta.get("script", "").endswith(WRAPPED_SCRIPT), \
        "exit metadata must name the browser witness script " + WRAPPED_SCRIPT
    # RUST child exit.json — verified separately from the browser metadata.
    assert native_exit_meta.get("observed") is True, "rust child exit must be observed (exit.json)"
    native_exit_field = native_exit_meta.get("exit") or {}
    assert native_exit_field.get("exit_code") == 0 and native_exit_field.get("success") is True, \
        "rust child exit_code must be 0 with success, got exit_code=%r" % native_exit_field.get("exit_code")
    # Native stdout.log: full web-bound start, web-stopped stop-record as the
    # LAST line (drain-error sidecars are checked by the caller: they must be
    # empty).
    native_text = native_stdout.decode(errors="replace")
    assert '"web-bound"' in native_text, "native stdout.log must hold the web-bound start record"
    last_line = native_text.strip().splitlines()[-1] if native_text.strip() else ""
    assert '"web-stopped"' in last_line, \
        "the last native stdout.log line must be the web-stopped stop-record"
    assert native_exit == 0, "native test exit must be 0, got %d" % native_exit
    # Cargo summary in the REAL format: `test result: ok. 1 passed; 0 failed;
    # 0 ignored; 0 measured; 9 filtered out; finished in ...` — endswith("ok")
    # and `; 0 filtered` never match it (9 other tests are legitimately
    # filtered); refuse 0-run/ignored-only/FAILED even with exit 0.
    check_cargo_summary(native_run_stdout.decode(errors="replace"), test_name)


def check_cargo_summary(text: str, test_name: str = TEST_NAME) -> None:
    """Pure guard over the cargo summary parser used by verify(): accepts
    EXACTLY the real single-test shape (one summary line `test result: ok.
    1 passed; 0 failed; 0 ignored; 0 measured; <n> filtered out; finished in
    ...` plus the named `test <test_name> ... ok` line for the requested
    scope's test — the other 9 tests being filtered is normal) and raises
    AssertionError on the regression controls (no summary, 0 passed,
    failed/ignored > 0, FAILED marker, missing named test line, unparsable
    or duplicate summary). `test_name` defaults to the empty-scope
    TEST_NAME so existing callers and the regression controls stay
    unchanged; verify() passes the scope's own test name. This is the
    parser regression guard: the good control is the REAL conserved summary
    from the qualified run, the bad controls are its zero/FAILED/malformed
    variants, never fake production copies. Inputs: the cargo stdout text
    and the scope's test name. Outputs: none on success. Errors:
    AssertionError with a count/name-only cause (no secret, no raw stream
    dump). Effects: none.
    """
    summaries = [line for line in text.splitlines() if line.startswith("test result: ")]
    assert len(summaries) == 1, \
        "exactly one cargo summary line expected, got %d" % len(summaries)
    parsed = re.fullmatch(
        r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;"
        r" \d+ measured; (\d+) filtered out; finished in .+", summaries[0])
    assert parsed is not None, "unparsable cargo summary line: " + summaries[0]
    outcome, passed, failed, ignored, _filtered = parsed.groups()
    assert (outcome, int(passed), int(failed), int(ignored)) == ("ok", 1, 0, 0), \
        "exactly one executed passing test expected (0 failed, 0 ignored), summary: " + summaries[0]
    assert "FAILED" not in text, "FAILED present in native stdout despite exit 0"
    test_line = re.search(r"^test " + re.escape(test_name) + r" \.\.\. ok$", text, re.M)
    assert test_line is not None, "the named test must appear as `test " + test_name + " ... ok`"


def _self_check_cargo_summary() -> None:
    """At-import regression guard (cheap, pure, no process, no write):
    the REAL conserved summary of the qualified run must be ACCEPTED and each
    regression control (zero-run, ignored-only, failed, malformed, missing
    named line) must be REFUSED. Raises AssertionError on any drift, so a
    parser regression fails loudly at wrapper start instead of qualifying a
    bad run. Inputs/Outputs: none. Errors: AssertionError on drift. Effects:
    none (in-memory only).
    """
    real = ("running 1 test\ntest " + TEST_NAME + " ... ok\n\n"
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; "
            "9 filtered out; finished in 0.00s\n")
    check_cargo_summary(real)
    controls = (
        # zero-run: exit 0 but no test executed
        "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; "
        "0 measured; 10 filtered out; finished in 0.00s\n",
        # ignored-only: the requested test skipped, still exit 0
        "running 1 test\ntest " + TEST_NAME + " ... ignored\n\n"
        "test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; "
        "9 filtered out; finished in 0.00s\n",
        # failed summary despite exit 0
        "running 1 test\ntest " + TEST_NAME + " ... FAILED\n\n"
        "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; "
        "9 filtered out; finished in 0.00s\n",
        # malformed summary (old endswith("ok") parser would accept it)
        "running 1 test\ntest " + TEST_NAME + " ... ok\n\n"
        "test result: ok-ish\n",
        # named test line missing while the summary alone looks green
        "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; "
        "9 filtered out; finished in 0.00s\n",
    )
    for control in controls:
        try:
            check_cargo_summary(control)
        except AssertionError:
            continue
        raise AssertionError("cargo-summary regression control was accepted: " + control)


def main() -> None:
    """Run the requested scope's unchanged native browser-witness test once
    under a fresh wrapper dir, then verify its bounded artifacts through
    that scope's own contract. --binary required (no build, no touch);
    --report-scope empty|nonempty (default empty) is refused by argparse
    (exit 2, stderr only) BEFORE any filesystem allocation, cargo
    invocation or child process — an invalid scope never creates a
    directory nor spawns cargo; --output/--output-parent mutex, parent must
    exist; timeout=None explicit (no watchdog). Refusals precede ANY
    allocation and write nothing: existing --output (its mkdir refuses),
    missing --output-parent, and /tmp or below for either output location
    exit
    nonzero with stderr only, leaving preexisting directories — and a caller
    failure.txt — byte-for-byte unchanged. All wrapper writes, up to
    failure.txt on failure, land only in the directory THIS invocation
    created. The cargo invocation is the exact qualified one (`cargo test
    --manifest-path ...`, dedicated CARGO_TARGET_DIR, debug=0,
    CARGO_INCREMENTAL=0) with the witness env, targeting the scope's own
    test name; SWARM_PAEA_NATIVE_EVIDENCE_DIR
    is NOT created by this wrapper — the Rust from_env claims it with an
    exclusive create and refuses an existing dir. stdout/stderr are written
    to native.stdout/native.stderr and the ACTUAL returncode is durably
    conserved (in result.json on success, in failure.txt on failure) BEFORE
    any qualification, so even a failed run keeps its original evidence; a
    nonzero cargo exit preserves original stderr and exits nonzero WITHOUT
    PASS; the scope's DISTINCT marker and fresh path print only after
    verify() passes.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--report-scope", choices=("empty", "nonempty"),
                        default="empty", dest="report_scope")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--output", type=Path)
    group.add_argument("--output-parent", type=Path)
    args = parser.parse_args()
    contract = SCOPE_CONTRACT[args.report_scope]
    wrapper = None
    returncode = None
    try:
        binary = args.binary.resolve()
        if args.output is not None:
            output = args.output.resolve(); refuse_tmp(output, "--output")
            output.mkdir(mode=0o700)  # refuses an existing caller directory
            wrapper = output  # owned by THIS invocation only from here on
        else:
            parent = args.output_parent.resolve(); refuse_tmp(parent, "--output-parent")
            if not parent.is_dir(): raise NotADirectoryError("--output-parent must already exist: " + str(parent))
            wrapper = Path(tempfile.mkdtemp(prefix="paea-native-reader-ui-", dir=parent))
        before = hashlib.sha256(binary.read_bytes()).hexdigest()
        # NOT created here: the Rust from_env claims the evidence dir with an
        # exclusive create (it must NOT exist) — the wrapper only owns itself.
        evidence = wrapper / "native-browser-witness"
        environment = dict(os.environ)
        environment.update({
            "SWARM_ENGINE_BINARY": str(binary),
            "SWARM_PAEA_NATIVE_BROWSER_WITNESS": "1",
            "SWARM_PAEA_NATIVE_EVIDENCE_DIR": str(evidence),
            "CARGO_TARGET_DIR": str(REPOSITORY / ".cache" / "paea-verification-target"),
            "CARGO_PROFILE_DEV_DEBUG": "0",
            "CARGO_PROFILE_TEST_DEBUG": "0",
            "CARGO_INCREMENTAL": "0",
        })
        run = subprocess.run(
            ["cargo", "test", "--manifest-path", "rust/Cargo.toml", "--offline", "--locked",
             "-p", "swarm-web", "--test", "paea_reconcile", contract["test"],
             "--", "--ignored", "--exact", "--nocapture"],
            cwd=REPOSITORY, env=environment, capture_output=True, timeout=None)
        returncode = run.returncode
        (wrapper / "native.stdout").write_bytes(run.stdout)
        (wrapper / "native.stderr").write_bytes(run.stderr)
        assert hashlib.sha256(binary.read_bytes()).hexdigest() == before, "binary changed during the run"
        if returncode != 0:
            sys.stderr.write(run.stderr.decode(errors="replace"))
            raise SystemExit("native test run failed: exit %d" % returncode)
        assert "FAILED" not in run.stdout.decode(errors="replace"), \
            "FAILED present in native stdout despite exit 0"
        run_dirs = sorted(evidence.glob("browser-witness-*"))
        assert len(run_dirs) == 1, "exactly one browser-witness run dir expected, got %d" % len(run_dirs)
        nested = sorted(run_dirs[0].glob("browser-witness-*"))
        assert len(nested) == 1, "exactly one nested witness dir expected, got %d" % len(nested)
        # Real layout (qualified run): ALL exit/stream artifacts sit at the
        # evidence-dir top level (Rust RawCapture.store); exit_meta.run_dir
        # is the OUTER Rust browser-witness-<time>-<suffix> dir; the nested
        # JS dir inside it holds browser-witness.json.
        exit_meta = json.loads((evidence / EXIT_ARTIFACT).read_bytes())
        assert exit_meta.get("run_dir") == str(run_dirs[0]), \
            "exit metadata run_dir must be the OUTER rust browser-witness run dir under the owned evidence child"
        witness = json.loads((nested[0] / "browser-witness.json").read_bytes())
        native_exit_meta = json.loads((evidence / "exit.json").read_bytes())
        native_stdout = (evidence / "stdout.log").read_bytes()
        for sidecar in ("drain-stdout.errors.log", "drain-stderr.errors.log"):
            assert (evidence / sidecar).read_bytes() == b"", \
                sidecar + " must be empty (no drain error conserved by the rust child)"
        node_summary = json.loads((evidence / "browser-witness.stdout.log").read_bytes())
        verify(witness, exit_meta, node_summary, native_exit_meta, run.stdout,
               native_stdout, returncode, binary, before, contract["test"],
               contract["kind"], args.report_scope, contract["checks"])
        (wrapper / "result.json").write_text(json.dumps({
            "schema_version": 1, "status": "PASS", "returncode": returncode,
            "report_scope": args.report_scope,
            "test": contract["test"],
            "checks": len(witness["checks"]),
            "unexpected_diagnostics": len(witness["unexpected_diagnostics"]),
            "observed_refusals": len(witness["expected_refusals"]),
            "reconcile_gets": 7, "binary": str(binary), "binary_sha256": before,
            "browser_child": EXIT_ARTIFACT, "rust_child": "exit.json",
            "scope": "one ignored-exact native browser-witness run on the managed binary; "
                     + args.report_scope + " explicit ledger scope; no build, no touch, no provider"},
            indent=2) + "\n")
        print(contract["marker"], flush=True); print(wrapper, flush=True)
    except BaseException:
        if wrapper is not None and wrapper.exists():
            (wrapper / "failure.txt").write_text(
                "returncode: %r\n" % returncode + traceback.format_exc())
        raise


if __name__ == "__main__":
    _self_check_cargo_summary()
    main()
