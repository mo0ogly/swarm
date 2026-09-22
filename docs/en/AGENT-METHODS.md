# Working methods for Codex, Claude and Swarm

[Français](../AGENT-METHODS.md) · [User guide](USER-GUIDE.md)

This repository includes **eight methods** for specifying, implementing, reviewing
and improving Swarm. They adapt LIA's APEX and PDCA practices: explicit criteria,
traceable decisions, observable evidence, bounded recovery and retrospectives.
They have been rewritten for this repository and require no LIA installation,
deployment scripts, hooks or permission settings.

## Start here

Open Codex or Claude **in this repository**, then invoke a method:

| Need | Codex | Claude |
| --- | --- | --- |
| Analyze, plan and deliver a bounded change | `$apex` | `/apex` |
| Audit with reproducible evidence | `$audit-pdca` | `/audit-pdca` |
| Specify a need and testable requirements | `$spec-builder` | `/spec-builder` |
| Find gaps in a plan | `$spec-audit` | `/spec-audit` |
| Review code and implementation risks | `$code-reviewer` | `/code-reviewer` |
| Verify the affected flow after a fix | `$verify-fix` | `/verify-fix` |
| Recover an existing blocked plan | `$replan` | `/replan` |
| Explain outcomes and prioritize improvements | `$retex-analyzer` | `/retex-analyzer` |

Examples to enter in the conversation:

```text
$apex --plan-only Specify quota recovery with the same web and CLI rules.
$audit-pdca Audit result acceptance; report only, without making corrections.
$audit-pdca --fix Repair confirmed defects within this scope and rerun the checks.
$verify-fix Verify that a refused launch creates neither an agent nor a reservation.
$replan Recover the existing plan from the observed failure while preserving its limits.
```

For Claude, replace `$` with `/`. `--plan-only`, `--resume`, `--fix` and
`--score-only` are instructions interpreted by the method, not additional `swarm`
binary flags. Audits are read-only by default. Methods answer in the user's language.

## In Swarm preparation

The existing **Prepare with AI** method selector uses:

| Preparation method | Material provided to the model |
| --- | --- |
| APEX | Need analysis and planning |
| KS feature | Scope, criteria and task decomposition |
| Audit PDCA | Audit scope, risks and verification plan |

Swarm sends the entire method files and shared contract to the model. Execution
phases remain prohibited during preparation: the model proposes a brief or plan;
it cannot launch or accept agents. The other five methods are available in native
Codex/Claude sessions, not as extra preparation menu buttons.

The CLI exposes the same catalogue:

```sh
swarm --root "$PWD" --json prepare methods
```

Each entry includes `available` and a `sha256` fingerprint. `audit_pdca` remains
an alias for `audit-pdca` in Swarm, and `/audit_pdca` in Claude. The legacy
`/ks-feature` and `/ks-plan` commands are also provided.

Preparation methods are read from **Swarm's configured project root**. Installing only the
binary does not copy this configuration into other projects. With Docker, the
mounted project root must contain these files, including their symbolic links.

## Automatic engine framing

For new attempts, the engine includes role-specific methods in the context sent
to the provider. This does not depend on native skill discovery in Codex or Claude.

| Role | Included methods | Boundaries |
| --- | --- | --- |
| Planner and subplanner | APEX, audit-pdca, spec-builder, spec-audit, replan | PLAN and ACT; no tools or code changes |
| Worker | APEX, audit-pdca, verify-fix, handoff and tracking templates | DO and CHECK within authorized scope; read-only for audit-only tasks |
| Independent reviewer | audit-pdca, code-reviewer | CHECK on supplied evidence; no tools or candidate changes |

The `workflow` field records version, role, method names and SHA-256 digest. Unknown
roles and oversized framing are rejected. Methods are **embedded at build time**:
a worker cannot replace engine instructions by editing its checkout. Rebuild the
binary after changing the pack to apply it to future attempts.

Preparations still read project files dynamically. Historical attempts do not gain
these metadata retroactively. The field records constructed guidance, not proven
model obedience. Executable checks remain necessary.

The [communication protocol](AGENT-COMMUNICATION.md) explains complete handoff
delivery, context receipts, linked decisions and their limitations.
Check that you are using the intended checkout, not an older copy.

## One source, two discovery paths

```text
AGENTS.md                         Codex project instructions
CLAUDE.md                         Claude project entry point
.claude/skills/<name>/SKILL.md     canonical sources for all eight methods
.agents/skills/<name>             relative link to the same method
.claude/skills/<name>/agents/     Codex selector labels and sample prompts
.claude/commands/                 three compatibility commands
tools/agent-workflows/CONTRACT.md shared rules also loaded by Swarm
```

Links stay inside the repository and travel with Git. Copies that do not preserve
symbolic links must restore them before use. Edit the canonical source, then run:

```sh
python3 tools/agent-workflows/check.py
go test ./... -run 'TestRepositoryPreparation|TestPreparationMethodCatalogue|TestPreparationDialogueLateProposalAndMethodDrift' -count=1
git diff --check
```

Changing a method or the shared contract changes its fingerprint. Existing
preparations must apply the new version and repeat validation; earlier evidence
does not become current simply because a file changed. If a native session does
not show a skill, check the root, symbolic links and same-named personal skills,
then reopen the session.

## Guarantees and limits

Methods require separate responsibilities: planners organize, workers implement,
independent reviewers assess, and the engine applies acceptance rules. Reviewing
within the implementing agent's conversation remains a self-review. The contract
prohibits bypassing attempt limits, quotas or missing evidence to report success.

These instructions guide the model; **they do not replace engine enforcement**.
Tests with provider doubles do not demonstrate a real autonomous mission. A method
does not grant additional delegation, publication or deployment authority. Model
selection and credentials remain those of the configured provider/session; this
pack imposes no model or permission mode.

Planner/worker organization is inspired by Cursor's work. Independent review and
acceptance conditions are Swarm's own contract; this pack is not Cursor certification.

## Formats and sources

- Locally adapted from LIA's APEX, audit-pdca, spec-builder, spec-audit, code-reviewer,
  verify-fix, replan and retex-analyzer methods. Machine-specific paths, product
  assumptions and automatic publication workflows have been excluded.
- [Codex skill discovery documentation](https://learn.chatgpt.com/docs/build-skills).
- [Claude project skills and commands documentation](https://code.claude.com/docs/en/skills).
- [Swarm shared contract](../../tools/agent-workflows/CONTRACT.md).

### Local budgets and progress in other scopes

When a subplanner reaches its activation ceiling, its pending input and history
are preserved. The conductor skips it and continues scopes with capacity in both
their own budget and their ancestors' budgets. A local ceiling must not create a
global planning failure. The transactional claim rechecks limits before each call.
When no scope can act, polling consumes no calls and does not mutate the mission.
An exhausted global budget, an unavailable provider and an actual planning failure
remain separate conditions; this rule does not clear them or grant extra attempts.

### Pending handoff order

The conductor selects the owner of the oldest unhandled event in durable inbox
order. A recent coordinator resume message no longer overtakes results already
received by a subplanner. Closed, leased, exhausted or integration-pending scopes
remain ineligible; their events are retained. The claim rechecks limits before
inference. Scheduling does not close scopes on the planner’s behalf or refund
previous attempts.

### Returning a closed child scope to its coordinator

The coordinator context distinguishes owned and delegated requirements.
`task_capacity_remaining` means available creation slots, not unfinished tasks.
`descendant_validation` carries descendant task identities, status, acceptance
freshness checked by the engine, and available review identity and candidate
revision. An accepted label without current evidence yields `accepted_fresh: false`.
This summary does not replace independent review. It lets the coordinator propose
closure, while the engine checks all evidence again before applying it.

### Exceptional recovery after incomplete delivery

After three consumed attempts, `planning authorize-recovery` supports one explicit
operator-confirmed recovery. For a result rejected before paid review, supply
`result_commit` (immutable result SHA) instead of `review_id`, plus `attempt_id`,
`confirm_recovery: true`, a reason and a new corrective instruction. The engine
checks Git attribution, the stopped latest attempt, actual delivery rejection and
reviewer availability. It retains all three attempts and grants at most one more.
Authorization starts no agent and grants no acceptance; checks and independent
review remain required. Repeated grants, replaced results and already-reviewed
results are refused by this route. Available through the CLI and public API; the
existing review-rejection form is unchanged.
