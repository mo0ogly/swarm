#!/usr/bin/env python3
"""Check the repository-local Claude/Codex method pack without calling a model."""

from pathlib import Path
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


if __name__ == "__main__":
    failures = check(ROOT)
    for failure in failures:
        print(f"ERROR: {failure}", file=sys.stderr)
    if failures:
        sys.exit(1)
    # Valid individual files do not prove that their combined role context fits.
    # Exercise the engine's actual embedded assembly, without a provider call or
    # a second implementation of its capacities/framing in this Python checker.
    result = subprocess.run(
        ["go", "test", "-json", "./internal/engine", "-run",
         "^TestAgentWorkflowRolesAreBounded$", "-count=1", "-timeout", "0"],
        cwd=ROOT, capture_output=True, text=True,
    )
    if result.returncode:
        print(result.stdout + result.stderr, file=sys.stderr, end="")
        sys.exit(result.returncode)
    # A zero exit code also occurs for an empty selector. Require the named
    # test's execution/completion and package completion, not just Go's exit.
    try:
        events = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    except (ValueError, TypeError):
        events = []
    expected = "TestAgentWorkflowRolesAreBounded"
    package = "swarm.local/companion/internal/engine"
    valid = bool(events) and all(isinstance(event, dict) for event in events)
    if valid:
        valid = (
            not any(event.get("Action") in ("fail", "skip") for event in events)
            and all(any(event.get("Package") == package and event.get("Action") == action
                        and event.get("Test") == expected for event in events)
                    for action in ("run", "pass"))
            and any(event.get("Package") == package and event.get("Action") == "pass"
                    and not event.get("Test") for event in events)
        )
    if not valid:
        print("ERROR: missing, incomplete or invalid named role-context test evidence", file=sys.stderr)
        print(result.stdout + result.stderr, file=sys.stderr, end="")
        sys.exit(1)
    completed = {event["Test"] for event in events if event.get("Package") == package
                 and event.get("Action") == "pass" and event.get("Test")}
    print(f"OK: {len(SKILLS)} shared Claude/Codex skills, preparation commands, contract and engine-built role contexts ({len(completed)} tests/subtests)")
