#!/usr/bin/env python3
"""Check the repository-local Claude/Codex method pack without calling a model."""

from pathlib import Path
import re
import sys


SKILLS = (
    "apex", "audit-pdca", "spec-builder", "spec-audit", "code-reviewer",
    "verify-fix", "replan", "retex-analyzer",
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
    print(f"OK: {len(SKILLS)} shared Claude/Codex skills, preparation commands and contract")
