# Working on Swarm

Read `tools/agent-workflows/CONTRACT.md` before planning or changing this repository.
It defines the scope, role boundaries, evidence and recovery rules shared by Codex,
Claude and Swarm preparation. User instructions and runtime permissions take precedence.

## Methods

Project skills are in `.agents/skills/`. They link to the canonical files under
`.claude/skills/`; edit those canonical files, not a second copy.

| Need | Skill |
| --- | --- |
| Deliver a bounded change, from analysis through verification | `apex` |
| Audit a system and optionally correct confirmed findings | `audit-pdca` |
| Turn a need into requirements and a dependency-aware plan | `spec-builder` |
| Find contradictions or omissions before executing a plan | `spec-audit` |
| Review an implementation for actionable defects | `code-reviewer` |
| Reproduce a fix through the actual CLI, API or web flow | `verify-fix` |
| Recover a blocked plan without erasing history or limits | `replan` |
| Explain outcomes and prioritize evidence-based improvements | `retex-analyzer` |

Choose the smallest relevant method. Do not chain every method for a trivial edit.
See `docs/AGENT-METHODS.md` (French) or `docs/en/AGENT-METHODS.md` (English).

## Repository checks

- Go engine: targeted `go test` cases, then the relevant package suite; run
  `go test ./...` and `go vet ./...` for engine changes. Use `go test -race` for
  changed synchronization or shared-state behavior.
- Frontend: use the scripts in `package.json` and the applicable browser recipes
  under `tests/`. `go test` does not execute those browser recipes.
- Configuration: `python3 tools/agent-workflows/check.py`.
- All changes: `git diff --check` and inspect the final diff.
- Never use a running mission's database as a test fixture. Build/test against an
  isolated temporary root. A build does not prove the running server uses it.

Inspect the current checkout and installed capabilities before running commands.
Do not assume a particular absolute directory, port, provider, model or quota.
