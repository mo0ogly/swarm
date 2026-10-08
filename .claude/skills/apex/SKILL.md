---
name: apex
description: "Analyze, plan, implement and verify a bounded Swarm change using APEX. Use when asked for APEX, a substantial feature, engine hardening or end-to-end delivery with explicit acceptance evidence. In Swarm preparation, stop at analysis and planning."
---

# APEX for Swarm

Read `tools/agent-workflows/CONTRACT.md` from the project root. It is included in
Swarm preparation contexts. Use the user's scope and requested depth; do not turn
a small fix into a large program. Arguments such as `--plan-only` and `--resume`
are requests interpreted by this skill, not executable CLI flags.

## Analyze

1. Identify the mode: planning only, authorized implementation, or resume.
   In Swarm preparation, analysis and planning are the entire permitted workflow.
2. Read the current requirement, source boundaries, public entry points, tests,
   existing plan and evidence. Establish the actual checkout/runtime relationship.
3. Define the user-visible effect, exclusions, constraints, failures and observable
   success criteria. Keep stable requirement IDs. Surface unknowns without inventing
   behavior. Inspect web, CLI and engine parity when the change crosses them.
4. For an architectural choice, compare at least two feasible approaches using
   correctness, recovery, effort and compatibility; record the decision.

## Plan

Create a small dependency-aware sequence. Each task names its requirement IDs,
owner role, scope/files, inputs, deliverable, acceptance checks and recovery route.
Include planning and independent verification responsibilities when the mission
requires them; a list of workers is not a complete coordination design.

Check for cycles, absent prerequisites, overlapping writes, missing review,
unavailable providers and incompatible budgets. Use the engine's plan format and
public validation commands; do not invent a second acceptance schema.
Preserve the canonical plan on resume. A plan-only request ends with a reviewable
plan and remaining questions, never a claim that execution has occurred.

## Execute within the assigned role

For each authorized task: observe current state, orient on evidence, decide one
bounded action, implement it, then observe its effect. First reproduce a reported
defect with a meaningful failing case where feasible. Make the smallest coherent
change; preserve unrelated work. Maintain short decision and evidence notes.

After failure, diagnose the concrete cause and change the precondition or approach.
Use `replan` for a structural obstacle. Keep original attempts and budgets; never
start a new mission simply to escape a ceiling. Delegate only when already authorized.

## Verify and close

Run checks proportionate to the change and verify the affected public behavior.
Use `verify-fix` for an actual web/CLI/API journey and `code-reviewer` for review.
Do not call a same-session self-review independent. For managed integration, bind
tests and independent review to the same candidate SHA before engine acceptance.

Deliver: result in plain language; requirements/evidence matrix; revision and dirty
state; commands and outcomes; report/artifact paths; remaining limits and next action.
A critical failed or untested criterion means incomplete delivery. Report it plainly.
