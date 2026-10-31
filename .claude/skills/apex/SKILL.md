---
name: apex
description: "Analyze, plan, implement and verify a bounded Swarm change using APEX. Use when asked for APEX, a substantial feature, engine hardening or end-to-end delivery with explicit acceptance evidence. In Swarm preparation, stop at analysis and planning."
---

# APEX for Swarm

Read `tools/agent-workflows/CONTRACT.md` (embedded in preparation). Respect user
scope/depth; a small fix is not a large program. `--plan-only`/`--resume` describe
requests, not CLI flags. Preparation permits analysis/planning only.

## Analyze

1. Identify planning, authorized implementation or resume. Recover requirements,
   plan, evidence, source boundaries, entrypoints/tests and checkout/runtime link.
2. Define visible effect, exclusions, constraints, failures and observable success
   with stable requirement IDs. Name unknowns; invent no behavior. Check affected
   web/CLI/engine parity.
3. For architecture, compare at least two feasible routes on correctness,
   recovery, effort and compatibility; record the choice.

## Plan

Keep the canonical plan on resume. Order tasks by prerequisites/dependencies;
each names requirement IDs, owner role, files/scope, inputs, deliverable,
acceptance checks and recovery. Include planning/independent verification when
required; workers alone are no coordination design. Check cycles, write ownership,
review, providers and budgets. Use engine formats/public validation. Planning-only
returns a reviewable plan and unresolved questions, never execution claims.

## Execute within the assigned role

Per authorized task: observe → orient on evidence → decide one bounded action →
implement → observe its effect. Reproduce defects with a meaningful failing case
where feasible. Keep changes coherent/minimal, unrelated work intact and short
decision/evidence notes.

Apply `replan` and shared contract §3 on resume, failure, blockage, missing proof
or changed assumptions: diagnose before retrying, revise structural obstacles,
preserve attempts/originals/budgets, never replace a mission to evade a ceiling.
Delegate only within existing authority. After each result reexamine engine,
handoff and assumptions; retain original proofs, verify receipt/decision, then
resume the initial need.

## Verify and close

Scale checks to risk; prove affected public behavior with `verify-fix` and review
with `code-reviewer`. Same-context review is not independent. Managed acceptance
requires tests and independent review on the same candidate SHA.

Deliver plain-language result, requirement/evidence matrix, revision/dirty state,
commands/outcomes, report/artifact paths, remaining limits and next action.
Critical failed/untested criteria mean incomplete delivery; report them plainly.

## Dashboard request ("tableau de bord")

For PAEA dashboards use the real FR/EN `docs/plans/actions-confiance/PAEA-APEX.md`
registers, not memory: État courant, I01–I17, T01–T22 including adaptive T18–T22.
Per T01→T22 give passed/partial/not exercised/human-blocked, last evidence folder
and latest run date. Show invariant coverage (n/17, %), campaigns exercised
(n/22, %), independent acceptance (n/17, %) and delta versus the previous
POINT-ETAPE date. Include open criteria count, candidate SHA/version and human/OS
blockers, each sourced. Numerators require registers/evidence; otherwise write
"non mesuré", never invent percentages.

## Go → Rust migration

For migration work, read and apply [the adopted method](references/rust-migration.md).
Tool-free roles need this reference supplied in context; report its absence.
