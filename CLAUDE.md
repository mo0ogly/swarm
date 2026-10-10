# Swarm project instructions

Read `AGENTS.md` and `tools/agent-workflows/CONTRACT.md` at the repository root.
They are the common project rules for Claude, Codex and Swarm preparation.

Project skills live in `.claude/skills/`: `/apex`, `/audit-pdca`, `/spec-builder`,
`/debug`, `/spec-audit`, `/code-reviewer`, `/verify-fix`, `/replan`, `/retex-analyzer`.
Product methods: `/product-planning`, `/product-delivery`, `/product-review`.
Creating or changing a Swarm template requires `/swarm-model-design`; read and
apply `.claude/skills/swarm-model-design/SKILL.md` before changing the model.
The `/ks-*` compatibility commands cover the product lifecycle and use these methods.
The legacy `/audit_pdca` alias is also provided.

Use the existing connection and model settings. These methods grant no additional
tools, permissions, subprocesses, budget, push or deployment authority.
For a session launched by Swarm, the assigned role, workspace and runtime contract
remain binding. A planner plans; a worker implements; an independent reviewer
reviews without modifying the candidate.

Usage: `docs/AGENT-METHODS.md` or `docs/en/AGENT-METHODS.md`.
