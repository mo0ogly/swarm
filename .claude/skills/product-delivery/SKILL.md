---
name: product-delivery
description: "Evidence based research, design and implementation per story"
---

# KS product delivery for Swarm

Read `tools/agent-workflows/CONTRACT.md`. Work only on the assigned task and stage.
A planning/documentation task never authorises application code, scaffolding,
publishing, merge, extra agents or scope expansion. Return evidence to the planner.

## Research and architecture tasks
Read project instructions and inspect current manifests, source, symbols/signatures,
call sites, tests, persistence and operations. Cite paths and locations actually
opened. Classify active/dead/prototype only from callers or runtime evidence.
Record blast radius, reproducible verification commands and open questions. Research
produces verified context, not an implementation plan. Existing architecture comes
first; preserve stack/patterns and immutable ADRs. For greenfield, require supplied
PRD, stories and an explicit stack decision before any separately authorised scaffold.

## Design system and UI tasks
Require the existing component system or an explicit visual direction. Capture
colour, typography, spacing/radius, available components, interaction patterns,
empty/loading/error/success and do/don't rules. Never invent an unrelated system.
For screen design, require that system and the target story. Use only its tokens
and components. Cover exact content, fields/actions, responsive layouts, focus,
keyboard, validation feedback and the four states. List system gaps and stop on
an unresolved required component rather than freelancing a replacement.
If external design was chosen, deliver a self-contained brief with the copied system
constraints and await its returned design. Do not silently switch to agent design.
A static mockup is a reference, never production code to paste into the app.

## Implementation
Require the current adopted task/plan and supplied research/design decisions. For
UI tasks implement the screen intent using actual application components. Preserve
unrelated changes; use the assigned isolated workspace/branch and engine integration
rather than switching or deleting another task's branch. Cover frontend/backend/
persistence only when assigned; deliver the user's end-to-end slice across its tasks.
For behavioural changes, demonstrate a failing regression/acceptance check where
feasible, implement the change, then rerun it. Follow repository-specific checks.
Do not add a test that merely mirrors code or claim an unexecuted test passed.
After each step compare scope, dependencies, architecture and acceptance with the
adopted plan. If invalidated, stop and report a plan-revision requirement. Existing
critical/major review findings take priority on a corrective run, within its limits.
Record changed files, exact commands, candidate SHA/diff, outcomes and blockers.

## Verification and handoff
Exercise observable criteria, invalid inputs, failures/recovery and relevant
persistence. UI checks cover languages/themes, keyboard, loading/errors and console/
network failures. Verify the actual build/runtime, not just a source label.
Submit evidence and remaining limits; self-checks are not independent review and a
process exiting is not acceptance. Require the engine's fresh review/integration
checks for the same candidate. Never edit a verdict, state DB, history or quota.

## Delivery
Prepare a PR only when explicitly authorised, with problem/result, scope, checks,
review and limits. Check an existing PR to avoid duplicates. Respect project branch
protection and manual merge/deploy policy; green tests do not grant publication.
Confirm the actual merge/deployment before reporting delivery. Clean a branch only
when explicitly authorised and proven merged; never use force deletion to conceal
unfinished work. In Swarm, supported managed integration owns candidate application;
a markdown `Ship allowed: yes` is evidence to assess, not engine acceptance.
