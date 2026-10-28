#!/usr/bin/env python3
"""Check local Claude/Codex methods and engine-built Rust role contexts, without a model."""

from pathlib import Path
import argparse
import json
import re
import subprocess
import sys


SKILLS = (
    "apex", "audit-pdca", "spec-builder", "spec-audit", "code-reviewer",
    "verify-fix", "replan", "retex-analyzer", "debug",
    "product-planning", "product-delivery", "product-review",
)
ROOT = Path(__file__).resolve().parents[2]


def check(root: Path) -> list[str]:
    errors = []
    for name in SKILLS:
        source = root / ".claude/skills" / name
        alias = root / ".agents/skills" / name
        try:
            if not source.resolve().is_relative_to(root):
                raise ValueError("canonical skill points outside the repository")
            text = (source / "SKILL.md").read_text(encoding="utf-8")
            parts = text.split("---", 2)
            if len(parts) != 3 or parts[0] != "":
                raise ValueError("missing YAML frontmatter")
            if not re.search(rf"^name: {re.escape(name)}$", parts[1], re.M):
                raise ValueError("frontmatter name does not match its directory")
            if not re.search(r"^description: .+", parts[1], re.M):
                raise ValueError("missing discovery description")
            if not alias.is_symlink() or alias.resolve(strict=True) != source.resolve():
                raise ValueError("Codex skill must link to the canonical Claude directory")
            metadata = (source / "agents/openai.yaml").read_text(encoding="utf-8")
            if f"${name}" not in metadata:
                raise ValueError("Codex prompt does not name the skill")
        except (OSError, ValueError, RuntimeError) as exc:
            errors.append(f"{name}: {exc}")

    for relative in (
        "AGENTS.md", "CLAUDE.md", "tools/agent-workflows/CONTRACT.md",
        "tools/agent-workflows/templates/HANDOFF.md",
        "tools/agent-workflows/templates/TRACKING.md",
        ".claude/commands/ks-feature.md", ".claude/commands/ks-plan.md",
        ".claude/commands/audit_pdca.md", "docs/AGENT-METHODS.md",
        "docs/en/AGENT-METHODS.md",
    ):
        path = root / relative
        try:
            if not path.resolve().is_relative_to(root) or not path.read_text(encoding="utf-8").strip():
                raise ValueError("empty file or external target")
        except (OSError, ValueError, RuntimeError) as exc:
            errors.append(f"{relative}: {exc}")
    return errors


# Named native integration contracts, not execution/retry/concurrency policies.
ROLES = ("planner", "subplanner", "worker", "reviewer")
REQUIRED_TESTS = {"frozen_role_" + role + "_contract" for role in ROLES} | {
    "frozen_workflows_bind_exact_body_and_role_without_local_override",
    "workflow_body_policy_is_exact_saved_and_recoverable",
    "workflow_project_composition_preserves_primary_and_full_prompt_bindings",
    "workflow_legacy_absence_requires_explicit_body_policy_adoption",
    "workflow_freshness_checks_full_prompt_and_current_policy",
}


def validate_native_tests(raw):
    """Require named executed contracts plus coherent native harness completion.

    Empty collection, missing roles, duplicate names, ignored/failed tests and
    mismatched counts refuse. A passing parent/summary never replaces a role.
    Text is retained as an original test channel, not authenticated execution,
    model identity, an installed production profile or independent acceptance.
    """
    runs = re.findall(r"^running ([0-9]+) tests?$", raw, re.M)
    done = re.findall(r"^test ([A-Za-z0-9_:]+) \.\.\. ok$", raw, re.M)
    summaries = re.findall(r"^test result: ok\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored; ([0-9]+) measured; ([0-9]+) filtered out; finished in .+$", raw, re.M)
    if len(runs) != 1 or len(summaries) != 1 or len(done) != len(set(done)):
        raise ValueError("native-context-harness-incomplete")
    passed, failed, ignored, measured, filtered = map(int, summaries[0])
    if passed != len(done) or int(runs[0]) != passed or failed or ignored or measured or filtered:
        raise ValueError("native-context-counts-incoherent")
    if re.search(r"^test .+ \.\.\. (?:FAILED|ignored)", raw, re.M) or not REQUIRED_TESTS <= set(done):
        raise ValueError("native-context-named-contract-missing")
    return set(done)


def native_check():
    """Run actual Rust integration contexts without Go, a shell or a watchdog.

    Cargo uses the caller's configured toolchain/cache/features environment;
    this command selects the native integration target with offline locked
    dependencies. Nonzero compiler/test status and both original channels survive.
    No model is called and no user Root or running server is opened.
    """
    command = ["cargo", "test", "--manifest-path", "rust/Cargo.toml", "--offline", "--locked",
               "-p", "swarm-engine", "--test", "agent_workflow", "--", "--nocapture"]
    result = subprocess.run(command, cwd=ROOT, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, text=True)
    if result.returncode != 0:
        print(f"ERROR: native Cargo exit status {result.returncode}", file=sys.stderr)
        print(result.stdout + (result.stderr or ""), file=sys.stderr, end="")
        # POSIX signed signal status cannot itself be a process exit code.
        # Preserve it above and return the conventional 128+signal CLI status.
        return (128 - result.returncode if result.returncode < 0 else result.returncode), set()
    try:
        completed = validate_native_tests(result.stdout)
    except ValueError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        print(result.stdout + (result.stderr or ""), file=sys.stderr, end="")
        return 1, set()
    # Preserve actual named output even in ordinary mode; no silent empty pass.
    print(result.stdout, end="")
    print(result.stderr or "", file=sys.stderr, end="")
    return 0, completed


def main(argv=None):
    """Validate methods and actual native contexts; optionally archive a full run.

    Evidence uses the permanent source-bound recorder and a fresh output folder.
    Old Go delta baselines remain historical and cannot qualify native sources;
    their obsolete flag refuses explicitly. No retry, provider or deployment.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path)
    parser.add_argument("--evidence-baseline", type=Path)
    args = parser.parse_args(argv)
    if args.evidence_baseline:
        parser.error("Go delta baselines cannot qualify native contexts; omit --evidence-baseline for a full native archive")
    failures = check(ROOT)
    for failure in failures:
        print(f"ERROR: {failure}", file=sys.stderr)
    if failures:
        return 1
    try:
        if args.evidence:
            sys.path.insert(0, str(ROOT / "tools/verification"))
            from recipe import run_recipe
            result = run_recipe(ROOT, ROOT / "tools/verification/recipes/agent-workflows-native.json", args.evidence)
            return 0 if result["status"] == "PASS" else 1
        code, completed = native_check()
        if code:
            return code
    except Exception as error:
        print(f"ERROR: native context verification refused ({type(error).__name__}: {error})", file=sys.stderr)
        return 1
    print(f"OK: {len(SKILLS)} shared Claude/Codex skills, 4 native roles, {len(completed)} named native tests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
