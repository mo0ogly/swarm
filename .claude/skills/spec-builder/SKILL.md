---
name: spec-builder
description: "Turn a Swarm feature request into a testable brief, requirements and dependency-aware task plan. Use before implementation when behavior, roles, acceptance criteria or web/CLI parity still need definition."
---

# Build a testable specification

Read `tools/agent-workflows/CONTRACT.md`. This method prepares documents; it does
not implement the feature or launch agents. In Swarm preparation, return the
requested structured proposal using only supplied evidence.

1. Establish the user, problem, expected outcome, constraints and explicit exclusions.
   Reuse already answered questions and adopted decisions. State assumptions separately.
2. Give each requirement a stable ID and a testable form: **when** this trigger occurs,
   **given** these preconditions, the system **must** produce this observable effect.
   Add relevant invalid inputs, timeouts, cancellation, retries and recovery behavior.
3. Describe state transitions and data/API/CLI contracts, including error behavior,
   persistence, idempotence, authorization and workspace isolation as applicable.
4. For orchestration, identify the planner, scoped workers, independent review and
   engine responsibilities. Define handoff inputs, evidence, dependency freshness,
   budget limits and who can act when a task stops. Do not replace enforcement with
   a visual role badge. Keep engine rules authoritative for both web and CLI.
5. Sketch only useful flows; use Mermaid for dependencies or state transitions.
   Explain what users see and do in ready, active, waiting, failed and completed states.
6. Build tasks with scope/files, requirement IDs, owner role, dependencies, deliverable,
   acceptance checks and failure handling. Avoid overlapping writes and cycles.
   Use the existing Swarm schema and public plan validation if execution tools are allowed.
7. Audit the proposal for contradictions and missing evidence with `spec-audit` when
   its size warrants it. Ask only for materially missing choices; record reasonable
   routine assumptions instead of blocking every step.

Output a brief, requirement-to-test matrix, task/dependency plan and unresolved
decisions with their impact. Distinguish proposed, adopted and engine-validated
documents. An implementation-ready specification is not an implemented result.
