#!/usr/bin/env python3
"""Run tests/paea_effects_ui.cjs once on the provided managed binary under
ONE fresh wrapper dir, then verify the real driver evidence: 19 named
checks, 0 unexpected diagnostics, 8 typed 403 refusals, matching digest.
No build, no touch, never /tmp, no reuse, no deletion; failures keep their
cause and never print PASS. THIS invocation writes only into a directory it
created itself: a refused precondition (--output already existing, missing
--output-parent) exits nonzero with stderr only and leaves preexisting
bytes — a caller failure.txt included — untouched, and /tmp or below is
refused for --output AND --output-parent before any filesystem allocation
(paths resolved first; TMPDIR is configured elsewhere).
"""
import argparse, hashlib, json, subprocess, sys, tempfile, traceback
from pathlib import Path
MARKER = "PAEA_EFFECTS_UI_BROWSER_VERIFIED 19"
BASE = (
    "bare reader refusal 403 paea-reader-authentication-required visible under a legitimate session",
    "bare refusal purges all report DOM (no report data shown)",
    "explicit preview-unavailable state, no consent control (no fake approval)",
    "unbound git-call explicit empty state",
    "reader credential field (password, autocomplete=off) and re-read button really served",
    "keyboard skip link first and theme control ≥44px")
REQUIRED = {lang + ": " + text for lang in ("fr", "en") for text in BASE} | {
    "fr: theme etat applied, no horizontal overflow at 1280px",
    "en: theme sombre applied, no horizontal overflow at 390px",
    "unknown-credential refusal: report purged, field erased, re-read command available",
    "without session: page asset and API refuse 403 web-access-refused",
    "no unexpected browser diagnostic across all phases"}
DYNAMIC = (("unknown fixture token: typed 403 identity refusal really served — ",
            " — on a fresh journal-less fixture (authorization_required would require an existing trust journal: reader_guard.rs:86,:124-126)"),
           ("Authorization header scoped to the reconcile GET only (",
            " classified 403(s), 1 credentialed re-read)"))

def refuse_tmp(path: Path, option: str) -> None:
    """Refuse /tmp itself or anything below it as an output location BEFORE
    any filesystem allocation or write; `path` must already be resolved
    (absolute, symlinks resolved) — TMPDIR is configured elsewhere, so no
    output or tempfile target may fall under /tmp. Raises ValueError naming
    `option` and the offending path; writes nothing — the caller turns this
    into its stderr-only nonzero refusal.
    """
    if path == Path("/tmp") or Path("/tmp") in path.parents:
        raise ValueError(option + " must not be /tmp or under it: " + str(path))

def verify(proof: dict, wrapper_digest: str) -> None:
    """Check the REAL driver evidence — named checks, not a counter alone.
    Asserts kind/schema, exactly 19 checks = the 17 fixed names in REQUIRED
    plus the 2 dynamic prefix+suffix checks in DYNAMIC (so no missing and,
    with length 19, no extra name), empty unexpected_diagnostics,
    expected_denials == 8, 8 observed refusals all 403, and equality of the
    driver binary digest with the wrapper's before/after digest.
    """
    checks = proof.get("checks") or []; named = set(checks)
    shape = (proof.get("kind"), proof.get("schema_version"), len(checks))
    assert shape == ("paea-effects-ui-v1", 1, 19), "unexpected driver proof shape: %r" % (shape,)
    assert REQUIRED <= named, "missing checks: %r" % (REQUIRED - named,)
    assert all(any(c.startswith(p) and c.endswith(s) for c in checks) for p, s in DYNAMIC), "missing dynamic check"
    assert proof.get("unexpected_diagnostics") == [] and proof.get("expected_denials") == 8, proof.get("unexpected_diagnostics")
    assert len(rf := proof.get("observed_refusals") or []) == 8 and all(x.get("status") == 403 for x in rf), rf
    assert proof.get("binary_sha256") == wrapper_digest, "driver binary digest differs from wrapper digest"

def main() -> None:
    """Run the unchanged driver once under a fresh wrapper dir, then verify.
    --binary required (no build, no touch); --output/--output-parent mutex,
    parent must exist; timeout=None explicit (no watchdog). Refusals precede
    ANY allocation and write nothing: existing --output (its mkdir refuses),
    missing --output-parent, and /tmp or below for either output location
    exit nonzero with stderr only, leaving preexisting directories — and a
    caller failure.txt — byte-for-byte unchanged. All wrapper writes, up to
    failure.txt on failure, land only in the directory THIS invocation
    created: --output binds the wrapper after its mkdir succeeds,
    --output-parent after the private 0700 mkdtemp. Nonzero child exit
    preserves original stderr and exits nonzero WITHOUT PASS; marker and
    fresh path print only after verify() passes.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--output", type=Path)
    group.add_argument("--output-parent", type=Path)
    args = parser.parse_args()
    wrapper = None
    try:
        binary = args.binary.resolve()
        if args.output is not None:
            output = args.output.resolve(); refuse_tmp(output, "--output")
            output.mkdir(mode=0o700)  # refuses an existing caller directory
            wrapper = output  # owned by THIS invocation only from here on
        else:
            parent = args.output_parent.resolve(); refuse_tmp(parent, "--output-parent")
            if not parent.is_dir(): raise NotADirectoryError("--output-parent must already exist: " + str(parent))
            wrapper = Path(tempfile.mkdtemp(prefix="paea-effects-ui-", dir=parent))
        before = hashlib.sha256(binary.read_bytes()).hexdigest()
        run = subprocess.run(["node", "tests/paea_effects_ui.cjs", str(binary), str(wrapper / "browser")],
                             capture_output=True, timeout=None)
        (wrapper / "browser.stdout").write_bytes(run.stdout); (wrapper / "browser.stderr").write_bytes(run.stderr)
        if run.returncode != 0:
            sys.stderr.write(run.stderr.decode(errors="replace"))
            raise SystemExit("browser run failed: exit %d" % run.returncode)
        assert hashlib.sha256(binary.read_bytes()).hexdigest() == before, "binary changed during the run"
        proof = json.loads((wrapper / "browser" / "paea-effects-ui.json").read_bytes())
        verify(proof, before)
        (wrapper / "result.json").write_text(json.dumps({
            "schema_version": 1, "status": "PASS", "checks": len(proof["checks"]),
            "expected_denials": proof["expected_denials"], "observed_refusals": len(proof["observed_refusals"]),
            "binary": str(binary), "binary_sha256": before,
            "scope": "unchanged driver on the managed binary; no build, no touch, no provider, no guarded call"}, indent=2) + "\n")
        print(MARKER, flush=True); print(wrapper, flush=True)
    except BaseException:
        if wrapper is not None and wrapper.exists():
            (wrapper / "failure.txt").write_text(traceback.format_exc())
        raise

if __name__ == "__main__": main()
