#!/usr/bin/env python3
"""Run tests/authority_admin_ui.cjs once on the provided managed binaries
under ONE fresh wrapper dir, then verify the real driver evidence: the 41
named checks, 0 failures and no error entry. No build, no touch, never /tmp,
no reuse, no deletion; failures keep their cause and never print PASS. THIS
invocation writes only into a directory it created itself: a refused
precondition (--output already existing, missing --output-parent) exits
nonzero with stderr only and leaves preexisting bytes — a caller failure.txt
included — untouched, and /tmp or below is refused for --output AND
--output-parent before any filesystem allocation (paths resolved first;
TMPDIR is configured elsewhere).

ADMIN_APEX scope: the cockpit "Utilisateurs et accès" view (accounts, API
keys, habilitations matrix, audit tabs, daemon settings) over a real
authority daemon + engine pair, FR/EN × etat/sombre, mobile, keyboard.
"""
import argparse, hashlib, json, subprocess, sys, tempfile, traceback
from pathlib import Path
MARKER = "AUTHORITY_ADMIN_UI_BROWSER_VERIFIED 41"
CORE = (
    "signed-in", "cockpit-direct-link", "cockpit-menu-entry", "cockpit-jump-bar",
    "jump-opens-one", "reset-request-shown", "header-user", "account-created",
    "focus-restored", "refusal-visible", "key-listed-without-secret", "key-works",
    "admin-column-locked", "role-saved", "role-offered-for-accounts",
    "audit-connections", "audit-shows-actions", "audit-dashboard",
    "purge-needs-days", "mobile-no-overflow")
FRENCH_STRING = ("Comptes et accès", "Journal d’accès", "Habilitations",
                 "Réglages du démon", "Ajouter un compte", "Créer une clé", "Révoquer")
REQUIRED = set(CORE) | {"en-no-french:" + s for s in FRENCH_STRING} | {
    "logo-" + scheme + "-" + lang for scheme in ("light", "dark") for lang in ("fr", "en")
} | {"forgot-confirmation", "login-mobile-no-overflow"}


def refuse_tmp(path: Path, option: str) -> None:
    """Refuse /tmp or any path below it before any allocation (operator rule:
    nothing is ever written under /tmp; fixtures live in declared areas)."""
    resolved = path.resolve()
    if str(resolved).startswith("/tmp/") or resolved == Path("/tmp"):
        raise SystemExit(option + " below /tmp is refused: " + str(resolved))


def verify(proof: dict) -> None:
    """Check the driver proof shape: success, exactly the 41 expected named
    checks all ok, no error entry. A green exit alone never proves the run."""
    checks = proof.get("checks") or []
    names = [c.get("name") for c in checks]
    assert proof.get("success") is True, "driver reports failure"
    assert len(checks) == 41, "expected 41 checks, got %d" % len(checks)
    assert all(c.get("ok") for c in checks), "failed checks: %r" % [c["name"] for c in checks if not c.get("ok")]
    assert not any(n == "error" for n in names), "error entry present"
    missing = REQUIRED - set(names)
    assert not missing, "missing checks: %r" % (missing,)


def main() -> None:
    """Run the unchanged driver once under a fresh wrapper dir, then verify.
    --engine and --daemon binaries required (no build, no touch);
    --output/--output-parent mutex, parent must exist; timeout=None explicit
    (no watchdog). Refusals precede ANY allocation and write nothing: existing
    --output (its mkdir refuses), missing --output-parent, and /tmp or below
    for either output location exit nonzero with stderr only, leaving
    preexisting directories — and a caller failure.txt — byte-for-byte
    unchanged. All wrapper writes, up to failure.txt on failure, land only in
    the directory THIS invocation created: --output binds the wrapper after
    its mkdir succeeds, --output-parent after the private 0700 mkdtemp.
    Nonzero child exit preserves original stderr and exits nonzero WITHOUT
    PASS; marker and fresh path print only after verify() passes.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", required=True, type=Path)
    parser.add_argument("--daemon", required=True, type=Path)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--output", type=Path)
    group.add_argument("--output-parent", type=Path)
    args = parser.parse_args()
    wrapper = None
    try:
        engine = args.engine.resolve(); daemon = args.daemon.resolve()
        for binary in (engine, daemon):
            if not binary.is_file():
                raise SystemExit("binary missing: " + str(binary))
        if args.output is not None:
            output = args.output.resolve(); refuse_tmp(output, "--output")
            output.mkdir(mode=0o700)  # refuses an existing caller directory
            wrapper = output  # owned by THIS invocation only from here on
        else:
            parent = args.output_parent.resolve(); refuse_tmp(parent, "--output-parent")
            if not parent.is_dir(): raise NotADirectoryError("--output-parent must already exist: " + str(parent))
            wrapper = Path(tempfile.mkdtemp(prefix="authority-admin-ui-", dir=parent))
        engine_before = hashlib.sha256(engine.read_bytes()).hexdigest()
        daemon_before = hashlib.sha256(daemon.read_bytes()).hexdigest()
        run = subprocess.run(["node", "tests/authority_admin_ui.cjs", str(engine), str(daemon), str(wrapper / "browser")],
                             capture_output=True, timeout=None)
        (wrapper / "browser.stdout").write_bytes(run.stdout); (wrapper / "browser.stderr").write_bytes(run.stderr)
        if run.returncode != 0:
            sys.stderr.write(run.stderr.decode(errors="replace"))
            raise SystemExit("browser run failed: exit %d" % run.returncode)
        assert hashlib.sha256(engine.read_bytes()).hexdigest() == engine_before, "engine binary changed during the run"
        assert hashlib.sha256(daemon.read_bytes()).hexdigest() == daemon_before, "daemon binary changed during the run"
        proof = json.loads((wrapper / "browser" / "checks.json").read_bytes())
        verify(proof)
        (wrapper / "result.json").write_text(json.dumps({
            "schema_version": 1, "status": "PASS", "checks": len(proof["checks"]),
            "engine": str(engine), "engine_sha256": engine_before,
            "daemon": str(daemon), "daemon_sha256": daemon_before,
            "scope": "unchanged driver on the managed binaries; no build, no touch, no provider, no guarded call"}, indent=2) + "\n")
        print(MARKER, flush=True); print(wrapper, flush=True)
    except BaseException:
        if wrapper is not None and wrapper.exists():
            (wrapper / "failure.txt").write_text(traceback.format_exc())
        raise

if __name__ == "__main__": main()
