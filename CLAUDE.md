# Swarm project instructions

Read `AGENTS.md` and `tools/agent-workflows/CONTRACT.md` at the repository root.
They are the common project rules for Claude, Codex and Swarm preparation.

## Same quality requirements as Codex

Apply the complete **Common quality standard — Codex and Claude** in `AGENTS.md`.
It adapts Wattson's discipline to Swarm; no separate weaker Claude standard and
no automatic import of Wattson infrastructure or delivery permissions.

Before editing, recover the real state and criteria, then read the producer and
affected consumers of the contract. Never invent a symbol, field or runtime result.
Choose the smallest relevant method; use `/replan` for failures, missing evidence,
abnormal slowness and resumes. Correct or route the cause, verify the effect and
continue the authorized objective rather than waiting for another “next”.

For behavioral changes, retain a failing reproducer, fix the general contract and
run meaningful targeted checks, including rejection and recovery variants. Verify
that tests were actually collected. Do not repeatedly run the full Go suite.
Use isolated fixtures; never use a live mission database. Freeze tested sources
during a run and invalidate evidence when relevant inputs change.

Apply the function-docstring requirement in AGENTS.md to every new or modified
public/nontrivial function: contract, errors, effects and limits, in the same change.

Deliver the code, applicable FR/EN documentation and recipe proving the observable
result, then the agreed RETEX. Distinguish implementation, local verification,
independent review, native acceptance and installation. Mermaid must describe
verified behavior or explicitly labeled target behavior. Capture verified lessons
in the common guidance; repository rules alone do not update embedded role contexts.

Keep execution settings persisted and editable in the Admin modal, with one source
of truth across Go, API, CLI and UX. No hidden policy ceiling or contextual bypass.
For frontend work, apply `docs/UI-DESIGN.md`, shared semantic tokens, local assets,
FR/EN, both themes, mobile and keyboard/focus checks. A functional test does not
replace visual inspection of the changed surface.

Use supported public runtime operations and repository delivery paths. Preserve
history, consumption, unrelated work and other processes. Use applicable session
authority for commit/push/deploy/cleanup; never add `Co-Authored-By` lines or invent
authorship. These instructions grant no extra rights or paid calls.

User decision (24 October 2026): verified authorized development is built,
installed and activated locally by default, including starting/restarting only
the identified project service, without another permission request. Verify the
actual running binary/version, FR/EN notes and affected flow. Follow AGENTS.md's
local-delivery rule; a compiled candidate is not installed. This grants no commit,
push, publication, external deployment or interference with others' processes.

For the current Cursor engine evolution, the user's choice is **direct Codex-only
development** in `/home/fpizzi/Documents/ChatGPT/swarm`. Keep the native mission
paused; do not start Claude, agents or Swarm orchestration as a consequence of
reading this file. Any later change of execution mode needs user authorization.

## Permanent verification tool

Apply **Permanent verification recipes — Claude and Codex** in `AGENTS.md`.
The permanent entry point is `tools/verification/recipe.py`; retain maintained
recipes in `tools/verification/recipes/`, with test sources and fresh evidence.
Read `tools/verification/README.md` before running or verifying a recipe.
Cache-only scripts and a green exit alone do not establish a reproducible result.

## Available project methods and role boundaries

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
